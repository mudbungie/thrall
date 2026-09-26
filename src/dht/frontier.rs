//! A walk's state and the frontier read off it (yog bl-d00f, bl-d9c1): every node
//! heard of, which were asked, which replied — and, against the flight, the
//! next node to ask and whether anything on the frontier is still in the air.
//! `lookup` is the loop that drives it.

use super::bencode::Dict;
use super::flight::{Flight, Query};
use super::krpc::{self, Message, Node, NodeId};
use std::collections::{BTreeMap, BTreeSet};
use std::net::SocketAddr;

/// What a walk found: every reply, closest first, and every error a node
/// answered instead of a reply.
#[derive(Debug, Default)]
pub(crate) struct Outcome {
    pub(crate) replies: Vec<(Node, Dict)>,
    pub(crate) errors: Vec<String>,
}

/// The frontier as it stands between two events of the walk.
pub(crate) struct Frontier {
    /// The closest frontier node not yet asked.
    pub(crate) next: Option<SocketAddr>,
    /// A frontier node is in the air: its answer may still move the frontier.
    pub(crate) waiting: bool,
    /// Walk queries in the air — the door's are not, and hold no α slot.
    pub(crate) walking: usize,
    /// A door query is in the air: its answer may still seed the pool.
    pub(crate) door: bool,
}

pub(crate) struct Walk {
    target: NodeId,
    k: usize,
    pub(crate) asked: BTreeSet<SocketAddr>,
    replied: BTreeSet<SocketAddr>,
    /// Keyed `(distance, address)`: a repeated entry is learned once, one id
    /// at two addresses is two nodes, and iteration is closest first.
    pool: BTreeMap<([u8; 20], SocketAddr), Node>,
    pub(crate) out: Outcome,
}

impl Walk {
    pub(crate) fn new(target: NodeId, k: usize) -> Walk {
        Walk {
            target,
            k,
            asked: BTreeSet::new(),
            replied: BTreeSet::new(),
            pool: BTreeMap::new(),
            out: Outcome::default(),
        }
    }

    /// The frontier: the K closest nodes that replied, are not yet asked, or
    /// are still in the air. A node asked and silent past its deadline, one
    /// that answered only an error, and one the socket refused have left it,
    /// so none holds a slot a live node past it would take.
    pub(crate) fn frontier(&self, flight: &Flight) -> Frontier {
        let airborne: BTreeSet<SocketAddr> = flight
            .values()
            .filter(|q| !q.door)
            .map(|q| q.addr)
            .collect();
        let front: Vec<SocketAddr> = self
            .pool
            .values()
            .map(|n| n.addr)
            .filter(|a| self.replied.contains(a) || !self.asked.contains(a) || airborne.contains(a))
            .take(self.k)
            .collect();
        Frontier {
            next: front.iter().copied().find(|a| !self.asked.contains(a)),
            waiting: front.iter().any(|a| airborne.contains(a)),
            walking: flight.values().filter(|q| !q.door).count(),
            door: flight.values().any(|q| q.door),
        }
    }

    /// Whether the door should be asked again: it has named someone, and
    /// fewer than K nodes past it have replied.
    pub(crate) fn dry(&self) -> bool {
        !self.pool.is_empty() && self.replied.len() < self.k
    }

    /// Take one answer in. The door's reply seeds the pool, but the door is never a result; a walk node's reply joins the
    /// pool, the replied set and the outcome, and its error the outcome's
    /// errors. A reply that names no id is heard and is nothing.
    pub(crate) fn heard(&mut self, query: Query, message: Message) {
        match message {
            Message::Reply { r, .. } => {
                let Some(id) = r
                    .get(b"id".as_slice())
                    .and_then(|v| v.as_bytes())
                    .and_then(NodeId::parse)
                else {
                    return;
                };
                let node = Node {
                    id,
                    addr: query.addr,
                };
                let this = (!query.door).then_some(node);
                for near in krpc::nodes_of(&r).into_iter().chain(this) {
                    self.pool
                        .insert((near.id.distance(&self.target), near.addr), near);
                }
                if !query.door {
                    self.replied.insert(query.addr);
                    self.out.replies.push((node, r));
                }
            }
            Message::Error { code, message, .. } if !query.door => {
                self.out
                    .errors
                    .push(format!("{}: {code} {message}", query.addr));
            }
            Message::Error { .. } => {}
        }
    }
}
