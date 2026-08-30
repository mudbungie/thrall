//! What a box holds, and what one broken entry costs the others.

use super::{ENTRIES, WIRE, dir, read_dir};
use crate::channel::material::{ADDRESS, ANCHORS, CHAIN, KEY};
use crate::test_support::{Scratch, mint};
use std::path::Path;

/// Provision one entry named `leaf` under an entries directory.
fn provision(entries: &Path, leaf: &str, address: &str) {
    let dir = entries.join(leaf);
    std::fs::create_dir_all(&dir).expect("the entry");
    for file in [ANCHORS, CHAIN, KEY] {
        std::fs::write(dir.join(file), b"pem bytes stand in here").expect("write");
    }
    std::fs::write(dir.join(ADDRESS), address).expect("write");
}

/// The path is REMOTE §8.2's, so a pair the operator filed for a client box is
/// filed the same way whichever program reads it.
#[test]
fn entries_live_where_the_protocol_says_they_do() {
    let root = Path::new("/somewhere/thrall");
    assert_eq!(dir(root), root.join(WIRE).join(ENTRIES));
}

/// A box that has been installed and not yet provisioned holds no channel, and
/// that is an answer rather than a refusal — absent, unreadable and empty are
/// one fact.
#[test]
fn an_unprovisioned_box_holds_no_channel() {
    let scratch = Scratch::new();
    assert_eq!(read_dir(&scratch.join("never-made")), Vec::new());
    assert_eq!(read_dir(scratch.path()), Vec::new());
}

/// Entries come back sorted by their own name, so a box serving several reads
/// the same way twice.
#[test]
fn entries_are_answered_in_one_order() {
    let scratch = Scratch::new();
    provision(scratch.path(), "second", "b.example:9000");
    provision(scratch.path(), "first", "a.example:9000");
    let held = read_dir(scratch.path());
    let names: Vec<&str> = held.iter().map(|e| e.leaf.as_str()).collect();
    assert_eq!(names, ["first", "second"]);
    assert_eq!(
        held[0].channel.as_ref().expect("provisioned").address,
        "a.example:9000"
    );
}

/// A stray file beside the entries names no intent and is not an entry with a
/// problem.
#[test]
fn a_file_beside_the_entries_is_not_an_entry() {
    let scratch = Scratch::new();
    provision(scratch.path(), "one", "a.example:9000");
    std::fs::write(scratch.join("notes.txt"), b"left here by hand").expect("write");
    assert_eq!(read_dir(scratch.path()).len(), 1);
}

/// **A refusal is one entry's, never the set's.** A box serving three engines
/// does not lose the two that are fine because the third is half-provisioned.
#[test]
fn one_broken_entry_does_not_cost_the_others() {
    let scratch = Scratch::new();
    provision(scratch.path(), "good", "a.example:9000");
    provision(scratch.path(), "half", "b.example:9000");
    std::fs::remove_file(scratch.join("half").join(KEY)).expect("rm");
    std::fs::create_dir_all(scratch.join("empty")).expect("mkdir");

    let held = read_dir(scratch.path());
    let by_name: Vec<(&str, bool)> = held
        .iter()
        .map(|e| (e.leaf.as_str(), e.channel.is_ok()))
        .collect();
    assert_eq!(by_name, [("empty", false), ("good", true), ("half", false)]);
    let half = held[2].channel.as_ref().expect_err("refused");
    assert!(half.contains("half-provisioned"), "{half}");
    let empty = held[0].channel.as_ref().expect_err("refused");
    assert!(empty.contains("is an empty entry"), "{empty}");
    assert!(empty.contains("thrall mints nothing"), "{empty}");
}

/// Opening an entry is opening its channel, and an entry with no material
/// refuses with its own sentence rather than with a transport error.
#[test]
fn opening_an_entry_is_opening_its_channel() {
    let scratch = Scratch::new();
    let entry = scratch.join("engine");
    mint::provisioned(&entry, "engine.example:9000");
    let held = read_dir(scratch.path());
    assert_eq!(held.len(), 1);
    let channel = held[0].open().expect("opened");
    assert_eq!(channel.client(), mint::FOOT_NAME);
    assert_eq!(channel.address(), "engine.example:9000");

    std::fs::remove_file(entry.join(ANCHORS)).expect("rm");
    let broken = read_dir(scratch.path());
    let refusal = broken[0].open().expect_err("refused");
    assert!(refusal.contains("half-provisioned"), "{refusal}");
}
