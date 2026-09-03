//! **What a failure means for the channel's lifetime** (bl-916d).
//!
//! The whole of the redial's correctness is here, because the loop above it is
//! arithmetic: everything turns on telling apart the three parties that can end
//! a conversation — the wire, the engine refusing, and the engine answering
//! something no foot gesture can earn — and on which leg was in flight when it
//! happened. **Two refusals, two answers, and they must never be collapsed**:
//! the engine declining this box's READ is REMOTE §5.1's one-reader guard,
//! which after a drop names this very machine's dying predecessor; the engine
//! declining what this box OFFERS is another connection serving under its name
//! (bl-2d78), and that ends the channel.
//!
//! The classification is structural — who failed, at which leg — and never
//! textual. A foot that decided its own lifetime by reading the engine's prose
//! would be a foot the far end could rewrite by rewording.

use super::super::hold::{Ending, hold};
use super::{advertised, aside, echo, ops, refusal, row, set, wired, work};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::test_support::engine::Engine;
use crate::test_support::{Scratch, mint};

/// **The one-reader refusal is retryable, and it names this box's own
/// predecessor.** A read parked when its connection died does not leave until
/// the engine tries to answer it, so a redial inside that window is refused
/// naming this very machine — and a foot that took the sentence as final would
/// make the first network blip permanent, which is the whole defect this ball
/// exists to end (REMOTE §5.1, §5.3).
#[test]
fn a_refused_read_is_this_box_s_own_predecessor_and_is_dialled_again() {
    let refused = "client \"foot-1\" already holds a read on this engine";
    let (_scratch, engine, channel) = wired(vec![advertised(), refusal(refused)]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        Ending::Again {
            said: refused.to_owned(),
            predecessor: true,
            served: false,
        }
    );
    assert_eq!(ops(&engine), ["advertise", "invocations"]);
}

/// **And the ADVERTISE refusal is not collapsed into it.** The engine declines
/// a set that would replace a serving machine's own, so a refusal there means
/// another connection is holding this machine's read with a different set in
/// force — a rival rather than a predecessor, and dialling again would hand it
/// the box by pretending otherwise.
#[test]
fn a_refused_advertisement_is_over_and_not_a_predecessor() {
    let refused = "this box's set is held by a serving connection";
    let (_scratch, _engine, channel) = wired(vec![refusal(refused)]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        Ending::Over(refused.to_owned())
    );
}

/// **A refused completion is over too.** The two ends disagree about what is in
/// flight, and asking again asks the same question.
#[test]
fn a_refused_completion_is_over() {
    let refused = "no invocation \"i-1\" is in flight";
    let (_scratch, _engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        refusal(refused),
    ]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        Ending::Over(refused.to_owned())
    );
}

/// **An answer no foot gesture can earn is over.** It is the engine speaking,
/// not the wire, so another dial earns the same unusable answer.
#[test]
fn an_unusable_answer_is_over() {
    let (_scratch, _engine, channel) = wired(vec![advertised(), advertised()]);
    let Ending::Over(said) = hold(&channel, &set(), echo, &aside()) else {
        panic!("an unusable answer must end the channel");
    };
    assert!(said.contains("not this machine's work"), "{said}");
}

/// **A wire that goes away is dialled again, and it is not a predecessor.** It
/// carries no opinion of this foot at all — it is the sleeping laptop, the
/// changed network, the relay switch — which is the case REMOTE §5.3's reversal
/// was written for.
#[test]
fn a_wire_that_goes_away_is_dialled_again() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::vanishes(scratch.path());
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    let Ending::Again {
        said,
        predecessor,
        served,
    } = hold(&channel, &set(), echo, &aside())
    else {
        panic!("a dropped wire must be worth another dial");
    };
    assert!(!predecessor, "the wire is nobody's refusal");
    assert!(!served, "it never got as far as an answered read");
    assert!(said.contains("the channel to the engine failed"), "{said}");
}

/// **An answered read is what marks a channel as having served**, and that is
/// the evidence the backoff resets on: the engine parked this foot for its own
/// hold and answered, which a loop hammering a dead port cannot manufacture.
#[test]
fn an_answered_read_marks_the_channel_as_having_served() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::flapping(
        scratch.path(),
        vec![Some(vec![advertised()]), Some(vec![work(vec![])]), None],
    );
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    let Ending::Again { served, .. } = hold(&channel, &set(), echo, &aside()) else {
        panic!("a dropped wire must be worth another dial");
    };
    assert!(served, "the engine answered a read before the wire dropped");
}

/// A stream terminated with no frame in it is the ENGINE speaking — a
/// zero-length frame is the terminator it deliberately wrote — where a peer
/// that went away is a read error instead. So it is over rather than retried.
#[test]
fn a_stream_that_answered_nothing_is_over() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::start(scratch.path(), crate::corpus::PROTOCOL, vec![vec![]]);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        Ending::Over("the engine ended the stream without answering".to_owned())
    );
}

/// A completion the WIRE swallowed is a dropped wire like any other, and the
/// invocation is not lost: the engine's mark is a lease its own next read
/// releases, so the redial's fresh channel is handed the same id again
/// (REMOTE §5.3, yog bl-e658). Nothing in this crate has to remember it.
#[test]
fn a_completion_the_wire_swallowed_is_dialled_again() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::flapping(
        scratch.path(),
        vec![
            Some(vec![advertised()]),
            Some(vec![work(vec![row("i-1", "Bash")])]),
            None,
        ],
    );
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    let Ending::Again { predecessor, .. } = hold(&channel, &set(), echo, &aside()) else {
        panic!("a dropped completion must be worth another dial");
    };
    assert!(!predecessor, "the wire is nobody's refusal");
}
