//! **The client rendezvous** (yog's `docs/REMOTE.md` §13.2–§13.4; DESIGN
//! §3.11): how a foot finds an engine that cannot be dialled, and the
//! connection it then punches.
//!
//! The engine end is yog's `src/wire/rendezvous` (bl-4263) and this is its
//! mirror, byte for byte where bytes cross: [`pairing`] derives the four
//! facts both ends compute from the pairing salt, [`item`] seals and opens
//! the two items the commons carries, [`punch`] is the TCP simultaneous open
//! from one port, and [`call`] is the act itself — read presence, write a
//! call, punch — plus the RAM cache the ladder's third rung re-punches at.
//!
//! **A foot touches the commons only at the moment it wants a connection**
//! (REMOTE §13.4). Nothing here publishes, polls or holds a thread: every
//! act is one dial's, on that channel's own thread, and an entry holding no
//! rendezvous material never constructs any of it.

pub(crate) mod call;
pub(crate) mod item;
pub(crate) mod pairing;
pub(crate) mod punch;

/// The mainline DHT's bootstrap nodes, resolved at the moment of a call —
/// a name lookup is a network act, and opening a channel dials nothing. The
/// four standard long-lived routers, yog's list: from its deployed engine box
/// only one of the first two answered at all (yog REMOTE §13.7 ruling 3,
/// bl-9408), so a silent router should cost a quarter of the roster, not half.
pub(crate) fn mainline() -> Vec<String> {
    vec![
        "router.bittorrent.com:6881".to_owned(),
        "dht.transmissionbt.com:6881".to_owned(),
        "router.utorrent.com:6881".to_owned(),
        "dht.aelitis.com:6881".to_owned(),
    ]
}
