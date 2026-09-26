//! A fake DHT node on loopback UDP — the far end every walk in this corpus
//! talks to (REMOTE §13.5: designed for from the first line). One thread per
//! node, a scripted routing answer, a BEP 44 store that checks what a real
//! node checks (token, signature, sequence), and a `Mood` for each way a
//! node on the commons misbehaves.
//!
//! **It listens**, which is the one thing thrall must never do (DESIGN §2) —
//! and that is precisely why it is scaffolding and not the crate proper: what
//! it stands in for is the commons.

use super::super::bencode::{Dict, Value, bytes, entry};
use super::super::krpc::{Node, NodeId};
use super::super::mutable::Mutable;
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::Duration;
use wire::{claim, error, reply, routing};

mod wire;

pub(crate) const TOKEN: &[u8] = b"tok";

#[derive(Clone, Copy)]
pub(crate) enum Mood {
    Answer,
    Silent,
    Garbage,
    Refuse,
    /// Replies without stating an id.
    Anonymous,
    /// Replies under a transaction nobody opened.
    Stray,
    /// Answers, and says (BEP 42's `ip`) the query came from this address —
    /// true or not, which is the point: one node's word is only a claim.
    Claim(SocketAddr),
    /// A mainline bootstrap router as yog measured it (REMOTE §13.7 ruling
    /// 3): answers `find_node`, silent to BEP 44's `get` and `put`.
    Router,
    /// The one router that answered from yog's deployed engine box (yog
    /// bl-d00f): a `Router` whose every `find_node` answer is ONE of its
    /// peers repeated eight times, the next peer on the next query.
    Rotor,
    /// Answers everything but `put`: offers a token and never spends it.
    Mute,
}

/// The node's BEP 44 store, shared with the test so it can read back what a
/// client put there.
pub(crate) type Store = Arc<Mutex<Vec<Mutable>>>;

pub(crate) struct FakeNode {
    socket: Option<UdpSocket>,
    pub(crate) addr: SocketAddr,
    pub(crate) id: NodeId,
    pub(crate) items: Store,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl FakeNode {
    pub(crate) fn bind(id: NodeId) -> FakeNode {
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        socket
            .set_read_timeout(Some(Duration::from_millis(20)))
            .unwrap();
        let addr = socket.local_addr().unwrap();
        FakeNode {
            socket: Some(socket),
            addr,
            id,
            items: Store::default(),
            stop: Arc::new(AtomicBool::new(false)),
            thread: None,
        }
    }

    pub(crate) fn node(&self) -> Node {
        Node {
            id: self.id,
            addr: self.addr,
        }
    }

    pub(crate) fn serve(&mut self, peers: Vec<Node>, mood: Mood, items: Vec<Mutable>) {
        let socket = self.socket.take().unwrap();
        *self.items.lock().unwrap_or_else(PoisonError::into_inner) = items;
        let (id, stop, store) = (self.id, Arc::clone(&self.stop), Arc::clone(&self.items));
        self.thread = Some(std::thread::spawn(move || {
            run(&socket, id, &peers, mood, &store, &stop);
        }));
    }

    /// Everything the node holds right now.
    pub(crate) fn held(&self) -> Vec<Mutable> {
        self.items
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Drop for FakeNode {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            t.join().unwrap();
        }
    }
}

fn run(
    socket: &UdpSocket,
    id: NodeId,
    peers: &[Node],
    mood: Mood,
    items: &Store,
    stop: &AtomicBool,
) {
    let mut buf = vec![0u8; 8192];
    let mut turn = 0usize;
    while !stop.load(Ordering::Relaxed) {
        let Ok((n, from)) = socket.recv_from(&mut buf) else {
            continue;
        };
        let q = Value::decode(&buf[..n]).unwrap();
        let tid = q.get("t").unwrap().as_bytes().unwrap().to_vec();
        let verb = q.get("q").unwrap().as_bytes().unwrap().to_vec();
        let mut held = items.lock().unwrap_or_else(PoisonError::into_inner);
        let datagram = match mood {
            Mood::Silent => continue,
            Mood::Router | Mood::Rotor if verb != b"find_node" => continue,
            Mood::Mute if verb == b"put" => continue,
            Mood::Rotor => {
                turn += 1;
                let one = vec![peers[(turn - 1) % peers.len()]; 8];
                answer(&tid, id, &one, &mut held, &q)
            }
            Mood::Garbage => b"not bencode".to_vec(),
            Mood::Refuse => error(&tid, 201, "refused"),
            Mood::Anonymous => reply(&tid, Dict::new()),
            Mood::Stray => reply(b"stray", Dict::from([entry("id", bytes(&id.0))])),
            Mood::Answer | Mood::Router | Mood::Mute => answer(&tid, id, peers, &mut held, &q),
            Mood::Claim(ip) => claim(answer(&tid, id, peers, &mut held, &q), ip),
        };
        drop(held);
        socket.send_to(&datagram, from).unwrap();
    }
}

fn answer(tid: &[u8], id: NodeId, peers: &[Node], items: &mut Vec<Mutable>, q: &Value) -> Vec<u8> {
    let a = q.get("a").unwrap();
    let mut r = Dict::from([entry("id", bytes(&id.0))]);
    match q.get("q").unwrap().as_bytes().unwrap() {
        b"find_node" => {
            r.extend(routing(peers));
            reply(tid, r)
        }
        b"get" => {
            r.extend(routing(peers));
            r.insert(b"token".to_vec(), bytes(TOKEN));
            let target = NodeId::parse(a.get("target").unwrap().as_bytes().unwrap()).unwrap();
            if let Some(item) = items.iter().find(|i| i.target() == target) {
                r.insert(b"k".to_vec(), bytes(&item.key));
                r.insert(b"seq".to_vec(), Value::Int(item.seq));
                r.insert(b"sig".to_vec(), bytes(&item.sig));
                r.insert(b"v".to_vec(), bytes(&item.value));
            }
            reply(tid, r)
        }
        b"put" => {
            if a.get("token").unwrap().as_bytes().unwrap() != TOKEN {
                return error(tid, 203, "bad token");
            }
            let item = Mutable {
                key: a.get("k").unwrap().as_bytes().unwrap().try_into().unwrap(),
                salt: a
                    .get("salt")
                    .map(|s| s.as_bytes().unwrap().to_vec())
                    .unwrap_or_default(),
                seq: a.get("seq").unwrap().as_int().unwrap(),
                value: a.get("v").unwrap().as_bytes().unwrap().to_vec(),
                sig: a
                    .get("sig")
                    .unwrap()
                    .as_bytes()
                    .unwrap()
                    .try_into()
                    .unwrap(),
            };
            if !item.verify() {
                return error(tid, 206, "invalid signature");
            }
            let target = item.target();
            if items
                .iter()
                .any(|i| i.target() == target && i.seq >= item.seq)
            {
                return error(tid, 302, "sequence number less than current");
            }
            items.retain(|i| i.target() != target);
            items.push(item);
            reply(tid, r)
        }
        _ => error(tid, 204, "unknown method"),
    }
}
