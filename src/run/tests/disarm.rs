//! **What a channel says while it is still serving**, which is one thing: that
//! this box was disarmed while a tool ran and has just restored itself
//! (REMOTE §5.1's `wrote`, PROTOCOL 8; thrall bl-2d78, yog bl-66d4).
//!
//! Every other sentence in this crate ends a channel and is returned as a
//! value. This one must be said with the channel still up, because the foot has
//! already healed it and carries on — so the sink is injected and these tests
//! read back what a running foot would have written to stderr.

use super::super::hold;
use super::{advertised, echo, receipt, refusal, restored, row, set, wired, work};
use crate::test_support::Notices;

/// **A re-assertion that WROTE is a disarming, and it is said out loud.** The
/// engine writes only a set that differs, so a foot presenting the set it has
/// presented all along and being told the document changed is a foot learning
/// that something replaced it while it was absent.
///
/// This is the whole point of the chain: before the receipt carried `wrote`,
/// this healed silently and the rival reached no log on either side.
#[test]
fn a_re_assertion_that_wrote_says_the_box_was_disarmed() {
    let (notices, sink) = Notices::new();
    let (_scratch, _engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        restored(),
        refusal("stop"),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &sink), "stop");
    let said = notices.heard();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("was not the set in force"), "{said:?}");
    assert!(said[0].contains("has just been restored"), "{said:?}");
    assert!(
        said[0].contains("bearing this box's identity"),
        "the sentence names a reading an operator can act on: {said:?}"
    );
}

/// **And it does not end the channel.** The set is back, the tools work, and a
/// foot that exited here would hand a rival the box by leaving. Being told is
/// the remedy; stopping is not.
#[test]
fn a_disarming_is_said_and_the_channel_goes_on() {
    let (notices, sink) = Notices::new();
    let (_scratch, engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        restored(),
        work(vec![]),
        refusal("stop"),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &sink), "stop");
    assert_eq!(
        super::ops(&engine),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "invocations",
            "invocations"
        ],
        "it asks for its next work rather than stopping"
    );
    assert_eq!(notices.heard().len(), 1);
}

/// **The ordinary re-assertion says nothing.** The engine compared and wrote
/// nothing, which is what every hand-off on an undisturbed box answers — and a
/// notice on each of those would be the noise that gets the real one ignored.
#[test]
fn a_re_assertion_that_compared_is_silent() {
    let (notices, sink) = Notices::new();
    let (_scratch, _engine, channel) = wired(vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        advertised(),
        refusal("stop"),
    ]);
    assert_eq!(hold(&channel, &set(), echo, &sink), "stop");
    assert_eq!(notices.heard(), Vec::<String>::new());
}

/// **A write on the FIRST presentation says nothing either**, and that is
/// REMOTE §5.1's own reading: a fresh channel presents into whatever the engine
/// happens to hold, and the ordinary first presentation writes. Only a
/// presentation this foot made after a hand-off it just performed can tell a
/// rival from a beginning.
#[test]
fn a_write_on_the_first_presentation_is_ordinary_and_silent() {
    let (notices, sink) = Notices::new();
    let (_scratch, _engine, channel) = wired(vec![restored(), refusal("stop")]);
    assert_eq!(hold(&channel, &set(), echo, &sink), "stop");
    assert_eq!(notices.heard(), Vec::<String>::new());
}

/// **An engine that answers the advertisement with something else has not said
/// the set landed**, and the channel stops there. Reading on would be waiting
/// for work under a set nobody confirmed — and since the receipt is now where
/// the `wrote` reading comes from, a foot that shrugged at the kind would be
/// inventing the reading as well.
#[test]
fn an_answer_that_is_not_the_receipt_ends_the_channel() {
    let (_scratch, _engine, channel) = wired(vec![work(vec![])]);
    let said = hold(&channel, &set(), echo, &crate::test_support::aside());
    assert!(said.contains("not that the advertisement landed"), "{said}");
    assert!(said.contains("Invocations"), "{said}");
}
