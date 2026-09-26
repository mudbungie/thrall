//! **The operator's rendezvous act, performed by the suite** (REMOTE §13.2),
//! and the commons stood in for: the two files an entry carries, a fake DHT
//! node holding the engine's presence, and the tuning that points a channel
//! at both in milliseconds.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use crate::dht::mutable::Mutable;
use crate::dht::tests::fake::{FakeNode, Mood};
use crate::dht::tests::{id, quick};
use crate::dht::{Config, Keypair};
use crate::rendezvous::call::Tuning;
use crate::rendezvous::item::Presence;
use crate::rendezvous::pairing::{self, Pairing, hex};

/// The engine's rendezvous seed — the half a foot never holds.
const SEED: [u8; 32] = [5u8; 32];
/// The pairing salt both ends share.
const SALT: [u8; 32] = [6u8; 32];

/// The pairing as this box reads it: the engine's PUBLIC key and the salt.
pub(crate) fn pairing() -> Pairing {
    Pairing {
        key: engine().public(),
        salt: SALT,
    }
}

/// The engine's keypair, for signing what the engine would sign.
pub(crate) fn engine() -> Keypair {
    Keypair::from_seed(SEED).expect("a keypair")
}

/// Write the two rendezvous files into `dir`.
pub(crate) fn carried(dir: &Path) {
    std::fs::write(dir.join(pairing::KEY), hex(&pairing().key)).expect("the key");
    std::fs::write(dir.join(pairing::SALT), hex(&SALT)).expect("the salt");
}

/// The engine's presence item, naming loopback at `port`, sealed and signed
/// as the engine's loop would.
pub(crate) fn presence(port: u16, seq: i64) -> Mutable {
    let p = pairing();
    let sealed = Presence {
        endpoints: vec![SocketAddr::new(Ipv4Addr::LOCALHOST.into(), port)],
    }
    .seal(&p.seal_key())
    .expect("seal");
    engine()
        .sign(p.presence_salt(), seq, sealed)
        .expect("signed")
}

/// A one-node commons holding `items`, and the tuning that walks it.
pub(crate) fn commons(items: Vec<Mutable>) -> (FakeNode, Tuning) {
    let mut node = FakeNode::bind(id(0x42));
    node.serve(vec![], Mood::Answer, items);
    let tuning = tuned(vec![node.addr.to_string()]);
    (node, tuning)
}

/// A tuning that walks `bootstrap` quickly and punches for under a second.
pub(crate) fn tuned(bootstrap: Vec<String>) -> Tuning {
    Tuning {
        config: Config { k: 8, ..quick() },
        window: Duration::from_millis(800),
        bootstrap,
    }
}
