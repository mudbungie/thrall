//! **What files nothing**: a door that will not accept the code, an offer
//! minted at the wrong grade, an entry this box already holds, an answer
//! that is no registration, and everything that stops a claim before it is
//! posted.

use super::{ca, run, signing};
use crate::channel::material::ANCHORS;
use crate::channel::{entries, tls};
use crate::register::register;
use crate::test_support::door::{self, Door};
use crate::test_support::{Notices, Scratch};
use serde_json::json;

/// **A code the door will not accept is one plain sentence**, and nothing is
/// filed.
#[test]
fn an_offer_the_door_refuses_cannot_be_claimed_and_files_nothing() {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let run = run(scratch, &dir, Box::new(|_| door::not_found()));
    assert_eq!(run.verdict.code, 1);
    assert!(
        run.verdict
            .text
            .starts_with("thrall: this offer cannot be claimed: 127.0.0.1:"),
        "{}",
        run.verdict.text
    );
    assert!(
        run.verdict.text.contains("mint a new one"),
        "{}",
        run.verdict.text
    );
    assert!(run.said.is_empty());
    assert!(!run.scratch.join("root").exists(), "nothing was filed");
}

/// **A foot only opens on a foot-grade leaf**: an offer minted at any other
/// grade files nothing and says which grade it was.
#[test]
fn an_offer_minted_at_another_grade_files_nothing() {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let answer = signing(&dir, "/CN=shed", "operator", json!(["127.0.0.1:1"]));
    let run = run(scratch, &dir, answer);
    assert_eq!(run.verdict.code, 1);
    assert!(
        run.verdict
            .text
            .contains("minted at the wrong grade: \"operator\"")
    );
    assert!(
        run.verdict.text.contains("Nothing was filed"),
        "{}",
        run.verdict.text
    );
    assert!(!run.scratch.join("root").exists(), "nothing was filed");
}

/// **An entry this box already holds is never overwritten**, and an answer
/// that is no registration at all refuses before anything is made.
#[test]
fn an_existing_entry_and_an_unreadable_answer_both_file_nothing() {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let home = entries::dir(&scratch.join("root")).join("home");
    std::fs::create_dir_all(&home).expect("the old entry");
    let answer = signing(&dir, "/OU=foot/CN=shed", "foot", json!(["127.0.0.1:1"]));
    let run = run(scratch, &dir, answer);
    assert_eq!(run.verdict.code, 1);
    assert!(
        run.verdict.text.contains("already exists"),
        "{}",
        run.verdict.text
    );
    assert_eq!(std::fs::read_dir(run.entry()).expect("ls").count(), 0);

    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let run = super::run(scratch, &dir, Box::new(|_| door::ok("{}")));
    assert!(
        run.verdict.text.contains("is not a registration"),
        "{}",
        run.verdict.text
    );
    assert!(!run.scratch.join("root").exists(), "nothing was filed");
}

/// **Everything judged before the post refuses before a byte is sent**: a
/// URL that is not one, a door nothing listens at, and a door whose
/// certificate the roots do not cover.
#[test]
fn what_cannot_be_posted_is_said_and_nothing_is_claimed() {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let root = scratch.join("root");
    let (_notices, sink) = Notices::new();
    let roots = || tls::anchors(&dir.join(ANCHORS)).expect("the root");
    let not_a_url = register(&root, "engine.example", roots(), &sink);
    assert!(not_a_url.text.contains("is not a registration URL"));
    let nobody = register(
        &root,
        "https://127.0.0.1:1/yog/register/ABCD2345",
        roots(),
        &sink,
    );
    assert!(
        nobody.text.contains("connect 127.0.0.1:1"),
        "{}",
        nobody.text
    );

    let door = Door::start(&dir, Box::new(|_| door::not_found()));
    let apart = Scratch::new();
    let elsewhere = ca(&apart);
    let other = tls::anchors(&elsewhere.join(ANCHORS)).expect("another root");
    let untrusted = register(&root, &door.url, other, &sink);
    assert_eq!(untrusted.code, 1);
    assert!(
        untrusted.text.contains("could not be posted"),
        "{}",
        untrusted.text
    );
    assert_eq!(door.heard(), "", "nothing of the claim was sent");
    assert!(!root.exists(), "nothing was filed");
}
