//! **The capture the wire swallowed, and the dial that posts it** (yog's
//! `docs/REMOTE.md` §5.6 ruling 1, DESIGN §3.8).
//!
//! The whole of what is asserted here is an ORDER: on the next dial the
//! advertisement goes out, then the held capture, and only then the read. The
//! read is what releases the engine's lease, so a capture posted after it would
//! arrive at a slot already handed out again — the tool re-run, and this box
//! paying twice for work it had in hand.
//!
//! **And what is NOT asserted is a ledger.** No id is remembered, nothing is
//! compared, and a redelivered invocation this process holds no capture for is
//! run as it always was (`redial::a_disarming_is_not_remembered_across_a_redial`
//! is the sibling that proves the gap stays empty).

use super::super::held::Held;
use super::super::hold::{Ending, hold};
use super::{
    advertised, aside, echo, entry_at, flapping_at, gesture, ops, receipt, refusal, row, set, work,
};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::config::Local;
use crate::invocation::{Capture, Invocation};
use crate::test_support::{Scratch, Waits};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

/// How many times the hand-off has run. A static because [`Handoff`] is a
/// function pointer and not a closure — a foot that carried per-invocation
/// state between calls would be a foot with a world (`run`) — and it is read by
/// the one beat that writes it.
///
/// [`Handoff`]: crate::run::Handoff
static RUNS: AtomicUsize = AtomicUsize::new(0);

/// The executor of [`super::echo`], counted.
fn counted(set: &[Local], invocation: &Invocation) -> Capture {
    RUNS.fetch_add(1, Ordering::Relaxed);
    echo(set, invocation)
}

/// The channel standing at `dir`, opened the way an operator-provisioned box
/// opens one.
fn channel_at(dir: &Path) -> Channel {
    let material = read_dir(dir).expect("readable").expect("provisioned");
    Channel::open(&material).expect("opened")
}

/// **The capture is posted FIRST on the next dial, and the tool is not re-run.**
///
/// The wire swallows the completion of `i-1`; the next channel presents its set
/// and then, before it reads anything, posts the capture the first channel
/// computed. The engine's lease is therefore released by a read that arrives
/// after the answer it was holding out for, and the hand-off ran exactly once
/// for an invocation this box was handed once.
#[test]
fn a_swallowed_capture_is_posted_first_on_the_next_dial() {
    let scratch = Scratch::new();
    let engine = flapping_at(
        scratch.path(),
        vec![
            Some(advertised()),
            Some(work(vec![row("i-1", "Bash")])),
            None,
            Some(advertised()),
            Some(receipt("i-1")),
            Some(refusal(
                "client \"foot-1\" already holds a read on this engine",
            )),
            Some(refusal("stop")),
        ],
    );
    let (waits, pause) = Waits::new();
    assert_eq!(
        crate::run::redial::redial(&entry_at(scratch.path()), &set(), counted, &aside(), &pause),
        "stop"
    );
    assert_eq!(
        ops(&engine),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "complete",
            "invocations",
            "advertise"
        ],
        "the re-post is the second channel's first act after its advertisement"
    );
    assert_eq!(
        gesture(&engine, 4),
        serde_json::json!({"op": "complete", "invocation": "i-1",
            "capture": {"stdout": "Bash", "stderr": "1 tools", "exit_code": 0}}),
        "it carries the capture the first channel computed, verbatim"
    );
    assert_eq!(
        RUNS.load(Ordering::Relaxed),
        1,
        "the hand-off ran once: the lease re-ran nothing"
    );
    assert_eq!(waits.heard().len(), 2, "one drop and one predecessor");
}

/// **A refused re-post is dropped and the channel reads on.** An engine that
/// refuses it has swept the slot, restarted, or already answered the driver —
/// all ordinary — so a foot that ended the channel over a capture nobody is
/// waiting for would be turning the ordinary case into the terminal one. It is
/// posted once and never again.
#[test]
fn a_refused_re_post_is_dropped_and_the_channel_reads_on() {
    let scratch = Scratch::new();
    let engine = flapping_at(
        scratch.path(),
        vec![
            Some(advertised()),
            Some(work(vec![row("i-1", "Bash")])),
            None,
            Some(advertised()),
            Some(refusal("no invocation \"i-1\" is in flight")),
            Some(work(vec![])),
            Some(refusal(
                "client \"foot-1\" already holds a read on this engine",
            )),
            Some(refusal("stop")),
        ],
    );
    let (_waits, pause) = Waits::new();
    assert_eq!(
        crate::run::redial::redial(&entry_at(scratch.path()), &set(), echo, &aside(), &pause),
        "stop",
        "the refusal ended nothing: the channel ran on to its own ending"
    );
    assert_eq!(
        ops(&engine),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "complete",
            "invocations",
            "invocations",
            "advertise"
        ],
        "it read on, and the refused capture was not posted again"
    );
}

/// **A re-post the wire swallows AGAIN is still held**, because a wire failure
/// is not an answer: it says nothing about whether the slot is still there. So
/// the rule holds unchanged across a flapping wire — one capture, posted first
/// on every dial until something at the far end has spoken about it.
#[test]
fn a_re_post_the_wire_swallowed_again_is_still_held() {
    let scratch = Scratch::new();
    let _engine = flapping_at(
        scratch.path(),
        vec![
            Some(advertised()),
            Some(work(vec![row("i-1", "Bash")])),
            None,
            Some(advertised()),
            None,
            Some(advertised()),
            Some(receipt("i-1")),
            Some(refusal(
                "client \"foot-1\" already holds a read on this engine",
            )),
        ],
    );
    let channel = channel_at(scratch.path());
    let Ending::Again { held, .. } = hold(&channel, &set(), echo, &aside(), None) else {
        panic!("a dropped completion must be worth another dial");
    };
    let carried = held.expect("the capture the wire swallowed");
    let Ending::Again { held, .. } = hold(&channel, &set(), echo, &aside(), Some(carried)) else {
        panic!("a dropped re-post must be worth another dial");
    };
    let carried = held.expect("a wire failure is not an answer");
    assert_eq!(
        carried,
        Held {
            id: "i-1".to_owned(),
            capture: Capture {
                stdout: "Bash".to_owned(),
                stderr: "1 tools".to_owned(),
                exit_code: 0,
            },
        },
        "the same capture, unchanged by a second drop"
    );
    let Ending::Again { held, .. } = hold(&channel, &set(), echo, &aside(), Some(carried)) else {
        panic!("the refused read is this box's own predecessor");
    };
    assert!(
        held.is_none(),
        "a capture the engine has answered for is let go"
    );
}
