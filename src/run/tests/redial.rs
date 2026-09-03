//! **One channel's lifetime**: dropped, waited out, dialled again (bl-916d).
//!
//! Two registers, because the loop is two things. The arithmetic is tested as
//! **values** — a suite that slept a sixty-four second cap out would take a
//! minute to prove one shift — and the loop itself is driven over a **real
//! wire** that really drops, with the waits read back out of a recording pause.

use super::super::redial::{CAP, FIRST, PREDECESSOR, next, redial};
use super::{advertised, aside, echo, flapping_at, receipt, refusal, restored, row, set, work};
use crate::channel::entries::Entry;
use crate::channel::material::read_dir;
use crate::test_support::{Notices, Scratch, Waits, unwaited};
use std::path::Path;
use std::time::Duration;

/// The entry standing at `dir`, where a flapping engine has been provisioned.
fn entry_at(dir: &Path) -> Entry {
    Entry {
        leaf: "engine".to_owned(),
        channel: Ok(read_dir(dir).expect("readable").expect("provisioned")),
    }
}

/// **The series doubles and it stops**, so a box whose network is not coming
/// back for hours settles to a dial a minute rather than burning a core.
#[test]
fn the_wait_doubles_from_its_floor_and_stops_at_the_cap() {
    let mut series = FIRST;
    let mut waited = Vec::new();
    for _ in 0..9 {
        let (wait, then) = next(series, false, false);
        waited.push(wait);
        series = then;
    }
    assert_eq!(
        waited,
        [1, 2, 4, 8, 16, 32, 64, 64, 64].map(Duration::from_secs)
    );
    assert_eq!(*waited.last().expect("a wait"), CAP);
}

/// **A one-reader refusal waits past one hold's width**, because the thing
/// refusing is this box's own predecessor and REMOTE §5.1 states exactly how
/// long it has left. Asking sooner earns the same sentence and spends a
/// handshake to hear it.
#[test]
fn a_predecessor_refusal_waits_past_one_hold_s_width() {
    assert_eq!(next(FIRST, true, false).0, PREDECESSOR);
    assert!(
        PREDECESSOR >= Duration::from_secs(30),
        "REMOTE §5.1 states the claim's life as one hold's width, thirty seconds"
    );
}

/// **And the series still advances underneath it**, so a refusal that is a
/// genuine rival rather than a dying predecessor backs off to the cap like
/// anything else instead of polling at one fixed cadence forever.
#[test]
fn a_predecessor_floor_is_overtaken_by_the_series() {
    let mut series = FIRST;
    for _ in 0..8 {
        series = next(series, true, false).1;
    }
    assert_eq!(next(series, true, false).0, CAP);
}

/// **A channel that served returns the series to its floor.** One answered read
/// is the engine having parked this foot for its own hold; without the reset a
/// laptop that sleeps nightly would creep to the cap and stay there for the
/// life of the process, which is the delay this whole loop exists to shorten.
#[test]
fn a_channel_that_served_returns_the_wait_to_its_floor() {
    assert_eq!(next(CAP, false, true), (FIRST, FIRST * 2));
}

/// **The whole point: a channel that drops is dialled again.** The wire goes
/// away under the first presentation, the foot waits, and the second dial is
/// answered — where before this ball that engine's tools were silent for the
/// life of the process.
#[test]
fn a_dropped_channel_is_dialled_again() {
    let scratch = Scratch::new();
    let engine = flapping_at(scratch.path(), vec![None, Some(refusal("stop"))]);
    let (waits, pause) = Waits::new();
    let (notices, sink) = Notices::new();
    assert_eq!(
        redial(&entry_at(scratch.path()), &set(), echo, &sink, &pause),
        "stop"
    );
    assert_eq!(waits.heard(), [FIRST]);
    let ops: Vec<String> = engine
        .heard()
        .iter()
        .filter_map(|v| v.get("op")?.as_str().map(str::to_owned))
        .collect();
    assert_eq!(ops, ["advertise", "advertise"], "it presented again");
    let said = notices.heard();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(
        said[0].contains("the channel to the engine failed"),
        "{said:?}"
    );
    assert!(said[0].contains("again in 1s"), "{said:?}");
}

/// **A refused read is the predecessor, and the wait says so.** Under a loop
/// the sentence that ended a connection is no longer returned by anything, so
/// it has to reach the operator as it happens — and "refused, and dialling
/// again" reads as a foot ignoring a refusal unless it says whose refusal it
/// expects.
#[test]
fn a_refused_read_is_waited_out_and_named() {
    let scratch = Scratch::new();
    let refused = "client \"foot-1\" already holds a read on this engine";
    let _engine = flapping_at(
        scratch.path(),
        vec![
            Some(advertised()),
            Some(refusal(refused)),
            Some(refusal("stop")),
        ],
    );
    let (waits, pause) = Waits::new();
    let (notices, sink) = Notices::new();
    assert_eq!(
        redial(&entry_at(scratch.path()), &set(), echo, &sink, &pause),
        "stop"
    );
    assert_eq!(waits.heard(), [PREDECESSOR]);
    let said = notices.heard();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].starts_with(refused), "{said:?}");
    assert!(said[0].contains("own connection still dying"), "{said:?}");
    assert!(said[0].contains("again in 32s"), "{said:?}");
}

/// **An ending that is over stops the channel and waits for nothing.** A
/// refused advertisement is another connection serving under this box's name,
/// and dialling again would hand it the box by pretending otherwise.
#[test]
fn an_ending_that_is_over_stops_the_channel_with_no_wait() {
    let scratch = Scratch::new();
    let _engine = flapping_at(
        scratch.path(),
        vec![Some(refusal("a rival holds this set"))],
    );
    let (waits, pause) = Waits::new();
    let (notices, sink) = Notices::new();
    assert_eq!(
        redial(&entry_at(scratch.path()), &set(), echo, &sink, &pause),
        "a rival holds this set"
    );
    assert_eq!(waits.heard(), Vec::<Duration>::new());
    assert_eq!(notices.heard(), Vec::<String>::new());
}

/// **A channel this box cannot open is over before anything is dialled.**
/// Opening reads no file and dials nothing — it is a fact about this box's own
/// material — so asking again would ask the same question.
#[test]
fn an_entry_that_cannot_be_opened_is_over_before_any_dial() {
    let (waits, pause) = Waits::new();
    let entry = Entry {
        leaf: "engine".to_owned(),
        channel: Err("this entry is empty".to_owned()),
    };
    assert_eq!(
        redial(&entry, &set(), echo, &aside(), &pause),
        "this entry is empty"
    );
    assert_eq!(waits.heard(), Vec::<Duration>::new());
}

/// **Two drops with nothing served between them back off**, and the same pair
/// with an answered read between them does not. It is the reset proved over a
/// real wire rather than as arithmetic: the second wait is the whole assertion.
#[test]
fn a_second_drop_backs_off_unless_the_channel_served() {
    let backed_off = waits_over(vec![
        Some(advertised()),
        None,
        Some(advertised()),
        None,
        Some(refusal("stop")),
    ]);
    assert_eq!(backed_off, [FIRST, FIRST * 2]);

    let reset = waits_over(vec![
        Some(advertised()),
        None,
        Some(advertised()),
        Some(work(vec![])),
        None,
        Some(refusal("stop")),
    ]);
    assert_eq!(reset, [FIRST, FIRST]);
}

/// Run one channel against `script` and answer every wait it took.
fn waits_over(script: Vec<Option<serde_json::Value>>) -> Vec<Duration> {
    let scratch = Scratch::new();
    let _engine = flapping_at(scratch.path(), script);
    let (waits, pause) = Waits::new();
    assert_eq!(
        redial(&entry_at(scratch.path()), &set(), echo, &aside(), &pause),
        "stop"
    );
    waits.heard()
}

/// **Nothing crosses a redial, and the disarming is the case that proves it**
/// (DESIGN §3.8). A redial makes a NEW channel, whose first presentation writes
/// into whatever the engine happens to hold and says nothing by design (REMOTE
/// §5.1). A foot that remembered having been disarmed and read the next
/// channel's first `wrote` as a rival would cry rival on the most ordinary
/// redial there is — the engine restarted, or this box's own dying predecessor
/// wrote last — which is the noise that gets the real one ignored.
#[test]
fn a_disarming_is_not_remembered_across_a_redial() {
    let scratch = Scratch::new();
    let _engine = flapping_at(
        scratch.path(),
        vec![
            Some(restored()),
            None,
            Some(restored()),
            Some(work(vec![row("i-1", "Bash")])),
            Some(receipt("i-1")),
            Some(refusal("stop")),
        ],
    );
    let (notices, sink) = Notices::new();
    assert_eq!(
        redial(&entry_at(scratch.path()), &set(), echo, &sink, &unwaited()),
        "stop"
    );
    let said = notices.heard();
    assert_eq!(said.len(), 1, "only the redial spoke: {said:?}");
    assert!(said[0].contains("Dialling this engine again"), "{said:?}");
    assert!(
        !said[0].contains("was not the set in force"),
        "a fresh channel's first presentation says nothing: {said:?}"
    );
}
