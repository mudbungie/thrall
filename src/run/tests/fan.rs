//! Every channel a box holds, served at once — each under its own identity,
//! and each answering under its own name.

use super::super::fan;
use super::boom;
use super::{advertised, aside, echo, engine_at, receipt, refusal, restored, row, set, work};
use crate::channel::entries;
use crate::test_support::{Notices, Scratch};

/// **Every channel this box holds is served at once**, each under its own
/// identity and its own material, and each answers the sentence that stopped it
/// under its own name.
#[test]
fn every_entry_is_served_and_each_answers_under_its_own_name() {
    let scratch = Scratch::new();
    let _one = engine_at(
        &scratch.join("north"),
        vec![advertised(), refusal("north stopped")],
    );
    let _two = engine_at(&scratch.join("south"), vec![refusal("south stopped")]);
    // A third entry that was made and never provisioned: its refusal is its
    // own, and it does not cost the other two.
    std::fs::create_dir_all(scratch.join("empty")).expect("mkdir");

    let said = fan(entries::read_dir(scratch.path()), set(), echo, &aside());
    assert_eq!(said.len(), 3);
    assert!(said[0].starts_with("empty: "), "{said:?}");
    assert!(said[0].contains("is an empty entry"), "{said:?}");
    assert_eq!(said[1], "north: north stopped");
    assert_eq!(said[2], "south: south stopped");
}

/// A box that holds no entry stops instantly with nothing to say. What to make
/// of that belongs to the caller, which is the one that knows where entries
/// would have been filed.
#[test]
fn a_box_with_no_entry_has_nothing_to_report() {
    let scratch = Scratch::new();
    assert_eq!(
        fan(entries::read_dir(scratch.path()), set(), echo, &aside()),
        Vec::<String>::new()
    );
}

/// **A notice carries the name of the channel that raised it**, the same
/// prefix that channel's ending sentence carries. A box holding two engines
/// that is told it was disarmed, and not by which one, has been told nothing it
/// can act on — the two channels have different identities and different
/// operators.
#[test]
fn a_notice_names_the_channel_that_raised_it() {
    let (notices, sink) = Notices::new();
    let scratch = Scratch::new();
    let _one = engine_at(
        &scratch.join("north"),
        vec![
            advertised(),
            work(vec![row("i-1", "Bash")]),
            receipt("i-1"),
            restored(),
            refusal("north stopped"),
        ],
    );
    let _two = engine_at(&scratch.join("south"), vec![refusal("south stopped")]);
    let said = fan(entries::read_dir(scratch.path()), set(), echo, &sink);
    assert_eq!(said, ["north: north stopped", "south: south stopped"]);
    let heard = notices.heard();
    assert_eq!(heard.len(), 1, "{heard:?}");
    assert!(heard[0].starts_with("north: "), "{heard:?}");
    assert!(heard[0].contains("was not the set in force"), "{heard:?}");
}

/// **One engine's conversation does not take the others down.** An executor
/// that dies is named as its own outcome, under the channel it killed, and the
/// channel beside it still answers.
#[test]
fn a_channel_that_dies_is_named_and_does_not_take_the_others_with_it() {
    let scratch = Scratch::new();
    let _one = engine_at(
        &scratch.join("north"),
        vec![advertised(), work(vec![row("i-1", "Bash")])],
    );
    let _two = engine_at(&scratch.join("south"), vec![refusal("south stopped")]);
    let said = fan(entries::read_dir(scratch.path()), set(), boom, &aside());
    assert_eq!(
        said,
        [
            "north: the channel ended by panicking",
            "south: south stopped"
        ]
    );
}
