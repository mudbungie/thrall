//! **The redial is load-bearing over a punched wire** (yog bl-0247, thrall
//! bl-0a8b; DESIGN §3.11): a held connection that drops re-enters the
//! ladder after the ordinary wait, and the re-punch lands.

use super::super::redial::{FIRST, redial};
use super::{advertised, echo, entry_at, receipt, refusal, row, set, work};
use crate::channel::material::ADDRESS;
use crate::test_support::engine::punched::{Punched, Turn};
use crate::test_support::roving::{carried, commons, presence};
use crate::test_support::{Notices, Scratch, Waits, mint};
use std::time::Duration;

/// A punched engine drops the held connection under a conversation; the foot
/// says so, waits the floor, and the next dial climbs back to it — one wait,
/// two connections, a hand-off served over the second, and the channel over
/// only when the engine refused the re-presentation.
#[test]
fn a_dropped_held_connection_is_dialled_again_through_the_ladder() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    carried(scratch.path());
    std::fs::write(scratch.path().join(ADDRESS), "127.0.0.1:1").expect("address");
    let engine = Punched::start(
        scratch.path(),
        vec![
            Turn::Answer(vec![advertised()]),
            Turn::Vanish,
            Turn::Answer(vec![advertised()]),
            Turn::Answer(vec![work(vec![row("i-1", "Bash")])]),
            Turn::Answer(vec![receipt("i-1")]),
            Turn::Answer(vec![refusal("stop")]),
        ],
    );
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = entry_at(scratch.path()).open().expect("opened");
    channel.tune(Duration::from_secs(5), tuning);
    let (waits, pause) = Waits::new();
    let (notices, sink) = Notices::new();
    assert_eq!(redial(Ok(channel), &set(), echo, &sink, &pause), "stop");
    assert_eq!(waits.heard(), [FIRST]);
    let said = notices.heard();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("the channel to the engine failed"),
        "{said:?}"
    );
    assert!(said[0].contains("again in 1s"), "{said:?}");
    let prefaces = engine
        .heard()
        .iter()
        .filter(|v| v.get("protocol").is_some())
        .count();
    assert_eq!(prefaces, 2, "the second dial punched a second connection");
}
