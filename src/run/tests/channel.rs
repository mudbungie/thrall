//! One channel: present, wait, hand off, answer — and every sentence that ends
//! it.

use super::super::hold;
use super::{advertised, aside, echo, gesture, ops, receipt, refusal, row, set, wired, work};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::test_support::engine::Engine;
use crate::test_support::{Scratch, mint};
use serde_json::json;

/// **The whole loop, once around**: present, wait, hand off, answer, present
/// again, wait again. Nothing in it is thrall speaking unprompted, and the
/// order is the evidence.
#[test]
fn the_loop_presents_waits_hands_off_and_answers() {
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        advertised(),
        refusal("the engine is going down"),
    ]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "the engine is going down",
        "the channel ends with the sentence that ended it"
    );
    assert_eq!(
        ops(&engine),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "invocations"
        ]
    );
    // The presentation is the config's projection: three keys, no argv.
    assert_eq!(
        gesture(&engine, 0)["tools"],
        json!([{"name": "Bash", "description": "run a command in a shell",
                "input_schema": {"type": "object"}}])
    );
    // The completion quotes the handle, and carries what the hand-off was
    // given: the invocation's tool, and this box's whole set.
    assert_eq!(
        gesture(&engine, 2),
        json!({"op": "complete", "invocation": "i-1",
               "capture": {"stdout": "Bash", "stderr": "1 tools", "exit_code": 0}})
    );
}

/// **The advertisement goes first, and a refusal of it ends the channel there.**
/// A foot that could not say what it offers has nothing to wait for.
#[test]
fn a_refused_advertisement_ends_the_channel_before_any_read() {
    let (_scratch, engine, channel) = wired(vec![refusal("this leaf is not registered here")]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "this leaf is not registered here"
    );
    assert_eq!(ops(&engine), ["advertise"]);
}

/// **An empty answer is ordinary.** The engine holds the read for its own bound
/// and then answers with what it has, which is usually nothing — so a foot
/// waiting for hours is a sequence of answered reads, not a hang.
///
/// It is also the proof that the re-assertion costs an idle foot **nothing**
/// (bl-2d78): no hand-off ends, so no set is presented again, and a foot that
/// is parked the whole time is the case the engine's own guard already covers.
#[test]
fn an_empty_answer_is_waited_through_and_not_an_ending() {
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![]),
        work(vec![]),
        refusal("the engine is going down"),
    ]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "the engine is going down"
    );
    assert_eq!(
        ops(&engine),
        ["advertise", "invocations", "invocations", "invocations"]
    );
}

/// Two invocations in one answer are run **serially**, in the order they came,
/// each answered before the next begins — which is what makes a busy foot
/// absent at the far end.
#[test]
fn work_in_one_answer_is_run_one_at_a_time_in_order() {
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash"), row("i-2", "Bash")]),
        receipt("i-1"),
        advertised(),
        receipt("i-2"),
        advertised(),
        refusal("stop"),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &aside()), "stop");
    assert_eq!(
        ops(&engine),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "complete",
            "advertise",
            "invocations"
        ]
    );
    assert_eq!(gesture(&engine, 2)["invocation"], json!("i-1"));
    assert_eq!(gesture(&engine, 4)["invocation"], json!("i-2"));
}

/// **The window a foot opens itself is closed at the end of every hand-off**
/// (bl-2d78, yog bl-1462). The stored set is keyed on this box's identity and
/// any connection bearing the certificate may replace it; the engine refuses a
/// replacement only while this machine holds a parked read, and a foot holds
/// none while it executes. So each hand-off ends by saying the set again — the
/// same set, in the same spelling as the first presentation.
#[test]
fn the_set_is_asserted_again_at_the_end_of_every_hand_off() {
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        advertised(),
        refusal("stop"),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &aside()), "stop");
    assert_eq!(
        gesture(&engine, 3),
        gesture(&engine, 0),
        "the re-assertion is the presentation, not a second projection"
    );
}

/// **A refused re-assertion ends the channel**, and the engine's sentence is
/// what the supervisor's log carries. The engine declines a set that would
/// replace a *serving* machine's own, so a refusal here means another
/// connection is holding this machine's read under a different set — two
/// processes claiming one name, which is exactly what this foot must not keep
/// serving through.
#[test]
fn a_refused_re_assertion_ends_the_channel() {
    let said = "another connection is holding this engine's follow-class read";
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        refusal(said),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &aside()), said);
    assert_eq!(
        ops(&engine),
        ["advertise", "invocations", "complete", "advertise"],
        "it stops at the refusal rather than reading for more work"
    );
}

/// **A refused completion ends the channel.** The two ends disagree about what
/// is in flight, and a foot that kept answering into that would be posting
/// captures nobody is waiting for.
#[test]
fn a_refused_completion_ends_the_channel() {
    let (_scratch, _engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        refusal("no invocation \"i-1\" is in flight"),
    ]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "no invocation \"i-1\" is in flight"
    );
}

/// An engine that answers the read with something other than this machine's
/// work is a disagreement worth stopping on, and the sentence names what came
/// back.
#[test]
fn an_answer_that_is_not_this_machine_s_work_ends_the_channel() {
    let (_scratch, _engine, channel) = wired(vec![advertised(), advertised()]);
    let said = hold(&channel, &set(), echo, &aside());
    assert!(said.contains("not this machine's work"), "{said}");
    assert!(said.contains("Advertised"), "{said}");
}

/// A kind no foot gesture can earn stops the channel rather than being ignored:
/// continuing on an answer to a question nobody asked would be guessing.
#[test]
fn an_answer_no_foot_gesture_can_earn_ends_the_channel() {
    let (_scratch, _engine, channel) =
        wired(vec![json!({"ok": true, "kind": "board", "rows": []})]);
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "reply: unusable kind \"board\""
    );
}

/// A stream that ends with no frame in it is not an answer, and says so.
#[test]
fn an_engine_that_ends_the_stream_without_answering_ends_the_channel() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::start(scratch.path(), crate::corpus::PROTOCOL, vec![vec![]]);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    assert_eq!(
        hold(&channel, &set(), echo, &aside()),
        "the engine ended the stream without answering"
    );
}
