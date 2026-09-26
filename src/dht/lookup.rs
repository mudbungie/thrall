//! The iterative lookup (BEP 5), the one walk every read and write of the
//! commons makes: ask the closest nodes you know, learn closer ones from
//! their answers, repeat until the K closest have all been asked. Rounds are
//! synchronous — α queries out, then one bounded wait for whatever answers —
//! and a node that stays silent is simply never asked again. Bounded twice
//! over against a hostile commons: a round ends at its deadline whatever is
//! still pending, and the walk ends at `max_queries` however many "closer"
//! nodes the answers keep inventing.

use super::Dht;
use super::bencode::Dict;
use super::krpc::{self, Message, Node, NodeId};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;
use std::time::Instant;

/// What a walk found: every reply, closest first, and every error a node
/// answered instead of a reply.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub(crate) replies: Vec<(Node, Dict)>,
    pub(crate) errors: Vec<String>,
}

/// The transactions one round is waiting on, by transaction id.
pub(crate) type Pending = BTreeMap<Vec<u8>, SocketAddr>;

impl Dht {
    /// Walk toward `target` asking `q` with `args` of every node on the way.
    /// `Err` is a dark commons — nobody answered at all — or the socket
    /// itself failing; a walk that drew only errors is an `Ok` with none.
    pub(crate) fn search(
        &mut self,
        target: NodeId,
        q: &str,
        args: Dict,
    ) -> Result<Outcome, String> {
        if self.bootstrap.is_empty() {
            return Err("no bootstrap node to ask".into());
        }
        let mut asked: BTreeSet<SocketAddr> = BTreeSet::new();
        let mut pool: BTreeMap<[u8; 20], Node> = BTreeMap::new();
        let mut out = Outcome::default();
        let mut sent = 0usize;
        let mut picks = self.bootstrap.clone();
        loop {
            let mut pending = Pending::new();
            for addr in picks {
                asked.insert(addr);
                sent += 1;
                self.ask(&mut pending, addr, q, args.clone());
            }
            self.collect(&mut pending, &mut |addr, message| match message {
                Message::Reply { r, .. } => {
                    let Some(id) = r
                        .get(b"id".as_slice())
                        .and_then(|v| v.as_bytes())
                        .and_then(NodeId::parse)
                    else {
                        return;
                    };
                    let node = Node { id, addr };
                    for near in krpc::nodes_of(&r).into_iter().chain([node]) {
                        pool.entry(near.id.distance(&target)).or_insert(near);
                    }
                    out.replies.push((node, r));
                }
                Message::Error { code, message, .. } => {
                    out.errors.push(format!("{addr}: {code} {message}"));
                }
            })?;
            picks = pool
                .values()
                .take(self.config.k)
                .filter(|n| !asked.contains(&n.addr))
                .take(self.config.alpha)
                .map(|n| n.addr)
                .collect();
            if picks.is_empty() || sent >= self.config.max_queries {
                break;
            }
        }
        if out.replies.is_empty() && out.errors.is_empty() {
            return Err(format!("no DHT node answered {q} for {target}"));
        }
        out.replies.sort_by_key(|(n, _)| n.id.distance(&target));
        Ok(out)
    }

    /// Send one query and, if the send itself went, remember the transaction.
    pub(crate) fn ask(&mut self, pending: &mut Pending, addr: SocketAddr, q: &str, args: Dict) {
        let tid = self.next_tid();
        let datagram = krpc::query(&tid, &self.id, q, args);
        if self.transport.send(addr, &datagram).is_ok() {
            pending.insert(tid, addr);
        }
    }

    /// One round: read datagrams until every pending transaction has answered
    /// or the round's deadline passes. Only an answer to a transaction this
    /// round sent reaches `on`; everything else on the socket is noise.
    pub(crate) fn collect(
        &mut self,
        pending: &mut Pending,
        on: &mut dyn FnMut(SocketAddr, Message),
    ) -> Result<(), String> {
        let deadline = Instant::now() + self.config.round;
        while !pending.is_empty() {
            let wait = deadline.saturating_duration_since(Instant::now());
            if wait.is_zero() {
                break;
            }
            let Some((_, bytes)) = self
                .transport
                .recv(wait)
                .map_err(|e| format!("DHT socket: {e}"))?
            else {
                break;
            };
            let Some(message) = krpc::parse(&bytes) else {
                continue;
            };
            let (Message::Reply { tid, .. } | Message::Error { tid, .. }) = &message;
            if let Some(addr) = pending.remove(tid) {
                on(addr, message);
            }
        }
        Ok(())
    }
}
