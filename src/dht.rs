//! **The DHT rendezvous client** (yog's `docs/REMOTE.md` §13.2; DESIGN
//! §3.11) — a pure client of the BitTorrent mainline DHT: outbound-UDP
//! iterative lookups and BEP 44 signed mutable get/put, and never a node. No
//! listener, no routing answers, no storage offered — nothing for the commons
//! to dial back, which is REMOTE §13's ruling 2 applied to the rendezvous
//! itself and DESIGN §2's *it never listens* kept at the DHT.
//!
//! **Reimplemented here rather than shared** (REMOTE §13.7 ruling 2): the
//! engine's `src/dht` is the original and this is its mirror, byte for byte
//! on the wire, because a DHT client is substrate and not protocol — nothing
//! yog-shaped crosses its interface — and a crate of its own would cost more
//! in release machinery than the copy costs.
//!
//! The shape the punched wire consumes is this module root: a caller hands
//! [`Dht::new`] a [`Transport`] (the std UDP socket [`Udp`], or a stand-in),
//! the bootstrap addresses and a [`Config`], then asks [`Dht::get`] for the
//! newest item under a key and salt or [`Dht::put`] to store one a
//! [`Keypair`] signed. Everything the commons answers is untrusted until it
//! verifies: an item without a good signature under the asked-for key is not
//! an item.
//!
//! Four files under this root, one concern each: `bencode` the encoding,
//! `krpc` the datagram shapes, `mutable` the signed item, `transport` the
//! socket seam; `lookup` is the walk and `items` the two BEP 44 verbs over
//! it. Synchronous throughout — `std::net` with socket timeouts, no tokio —
//! and every duration is a [`Config`] field a test can shorten, so the fake
//! DHT the suite runs on loopback UDP answers in milliseconds where the
//! commons answers in seconds.

pub(crate) mod bencode;
mod items;
pub(crate) mod krpc;
mod lookup;
pub(crate) mod mutable;
pub(crate) mod transport;

#[cfg(test)]
pub(crate) use krpc::Node;
pub(crate) use krpc::NodeId;
pub(crate) use mutable::Keypair;
pub(crate) use transport::{Transport, Udp};

#[cfg(test)]
use bencode::{Dict, bytes, entry};
use ring::rand::SecureRandom;
use std::net::SocketAddr;
use std::time::Duration;

/// The walk's parameters — stated so a test can shrink them and a caller
/// can widen them; the defaults are BEP 5's.
#[derive(Clone, Debug)]
pub(crate) struct Config {
    /// Queries in flight per round.
    pub(crate) alpha: usize,
    /// How many closest nodes a walk converges on and a `put` writes to.
    pub(crate) k: usize,
    /// How long one round waits for its answers.
    pub(crate) round: Duration,
    /// The most queries one walk may send, however the commons answers.
    pub(crate) max_queries: usize,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            alpha: 3,
            k: 8,
            round: Duration::from_secs(2),
            max_queries: 64,
        }
    }
}

/// One client: a transport, the nodes it starts from, and the id it queries
/// as. The id is random per client — a client is not a node and nobody
/// routes by it, so nothing is lost by minting a fresh one every run.
pub(crate) struct Dht {
    transport: Box<dyn Transport>,
    bootstrap: Vec<SocketAddr>,
    config: Config,
    id: NodeId,
    tid: u16,
}

impl Dht {
    /// A client over `transport`, starting every walk from `bootstrap`.
    pub(crate) fn new(
        transport: Box<dyn Transport>,
        bootstrap: Vec<SocketAddr>,
        config: Config,
    ) -> Result<Dht, String> {
        let mut id = [0u8; 20];
        random(&mut id)?;
        Ok(Dht {
            transport,
            bootstrap,
            config,
            id: NodeId(id),
            tid: 0,
        })
    }

    /// The nodes nearest `target`, closest first — BEP 5's `find_node` walk.
    /// The two BEP 44 verbs walk with `get` instead, so this is the suite's
    /// view of the walk alone.
    #[cfg(test)]
    pub(crate) fn lookup(&mut self, target: NodeId) -> Result<Vec<Node>, String> {
        let out = self.search(
            target,
            "find_node",
            Dict::from([entry("target", bytes(&target.0))]),
        )?;
        Ok(out
            .replies
            .into_iter()
            .map(|(n, _)| n)
            .take(self.config.k)
            .collect())
    }

    /// The next transaction id: two bytes, wrapping, never reused within a
    /// walk (a walk sends at most `max_queries`, far under 65 536).
    pub(crate) fn next_tid(&mut self) -> Vec<u8> {
        self.tid = self.tid.wrapping_add(1);
        self.tid.to_be_bytes().to_vec()
    }
}

/// Fill `buf` from the system's randomness.
pub(crate) fn random(buf: &mut [u8]) -> Result<(), String> {
    ring::rand::SystemRandom::new()
        .fill(buf)
        .map_err(|_| "the system offered no randomness".to_string())
}

#[cfg(test)]
pub(crate) mod tests;
