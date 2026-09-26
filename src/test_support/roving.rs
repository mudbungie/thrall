//! **The operator's rendezvous act, performed by the suite** (REMOTE §13.2),
//! and the commons stood in for: the two files an entry carries, a fake DHT
//! node holding the engine's presence, and the tuning that points a channel
//! at both in milliseconds.

use std::net::{Ipv4Addr, SocketAddr};
use std::path::Path;
use std::time::Duration;

use crate::dht::mutable::Mutable;
use crate::dht::tests::fake::{FakeNode, Mood};
use crate::dht::tests::{id, quick, router};
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

/// The commons as the live mainline is shaped (yog REMOTE §13.7 ruling 3):
/// a bootstrap router that answers only `find_node`, and the one node past
/// it that holds the items. Both live as long as this does.
pub(crate) struct Commons {
    holder: FakeNode,
    _door: FakeNode,
}

impl Commons {
    /// Everything the holder stores right now.
    pub(crate) fn held(&self) -> Vec<Mutable> {
        self.holder.held()
    }
}

/// A commons whose holder has `items`, and the tuning that walks it.
pub(crate) fn commons(items: Vec<Mutable>) -> (Commons, Tuning) {
    commons_as(Mood::Answer, items)
}

/// A commons whose holder answers in `mood` — a `Claim` says where it saw
/// the walk come from.
pub(crate) fn commons_as(mood: Mood, items: Vec<Mutable>) -> (Commons, Tuning) {
    let mut holder = FakeNode::bind(id(0x42));
    holder.serve(vec![], mood, items);
    let door = router(vec![holder.node()]);
    let tuning = tuned(vec![door.addr.to_string()]);
    (
        Commons {
            holder,
            _door: door,
        },
        tuning,
    )
}

/// A tuning that walks `bootstrap` quickly and punches for under a second.
pub(crate) fn tuned(bootstrap: Vec<String>) -> Tuning {
    Tuning {
        config: Config { k: 8, ..quick() },
        window: Duration::from_millis(800),
        bootstrap,
    }
}
