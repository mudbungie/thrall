//! **What this foot owes the engine's corpus** (REMOTE §3): every reply frame
//! it can earn decodes into this crate's own types, and every request frame it
//! emits round-trips — decode then re-encode returning the frame exactly.
//!
//! It is the half of conformance the version preface cannot buy. The preface
//! catches an engine of the wrong *build*; these catch a frame of the wrong
//! *shape* from an engine of the right one — which is the failure that arrives
//! when the number is correct and a field was added under it.

use super::{ADVERTISE, ADVERTISED, COMPLETE, INVOCATIONS, PROTOCOL, QUEUED, REFUSAL, ROUTED};
use crate::channel::hello;
use crate::gestures::{self, Reply};
use crate::invocation::{Capture, capture_of};
use crate::json::str_of;
use crate::tools;
use serde_json::Value;

/// One corpus frame as a value. A frame that will not parse is this file's own
/// defect — the text is a copy, so a broken one is a bad copy.
fn frame(text: &str) -> Value {
    serde_json::from_str(text).expect("a corpus frame is JSON")
}

/// **The pin is the engine's number, and the two have different sources.**
/// [`hello::PROTOCOL`] is what this build states on the wire;
/// [`PROTOCOL`](super::PROTOCOL) is yog's own line, copied. A bump on the far
/// end is this test going red naming both numbers — which is the sentence the
/// wire would otherwise give an operator, one dial too late.
#[test]
fn the_version_this_build_states_is_the_version_the_engine_states() {
    assert_eq!(
        hello::PROTOCOL,
        PROTOCOL,
        "this foot pins {}, the engine's corpus states {PROTOCOL} — \
         re-vendor src/corpus.rs from yog and move the pin with it",
        hello::PROTOCOL,
    );
}

/// **The advertisement round-trips**, element for element and byte for byte —
/// including the empty set and §5.1's optional `subject_cwd`, which rides only
/// when true.
#[test]
fn every_advertisement_round_trips() {
    for text in ADVERTISE {
        let set: Vec<tools::Tool> = frame(text)["tools"]
            .as_array()
            .expect("tools")
            .iter()
            .map(|row| tools::of_one(row).expect("a corpus element"))
            .collect();
        assert_eq!(gestures::advertise(&set).to_string(), text);
    }
}

/// The follow-class read carries no field, so there is nothing to decode and
/// the whole of the round trip is the spelling.
#[test]
fn the_follow_class_read_round_trips() {
    assert_eq!(gestures::invocations().to_string(), INVOCATIONS);
}

/// **The completion round-trips**, which is the one request this foot builds
/// out of something it was handed: the handle it was minted under, and the
/// capture in the spelling both ends share.
#[test]
fn the_completion_round_trips() {
    let sent = frame(COMPLETE);
    let o = sent.as_object().expect("an object");
    let capture = capture_of(&o["capture"]).expect("a corpus capture");
    assert_eq!(
        capture,
        Capture {
            stdout: "hello\n".to_owned(),
            stderr: "warned\n".to_owned(),
            exit_code: 3,
        }
    );
    let id = str_of(o, "invocation").expect("the handle");
    assert_eq!(gestures::complete(&id, &capture).to_string(), COMPLETE);
}

/// **The receipt at PROTOCOL 8**: `wrote` is read, both ways round, and it is
/// the reading the loop acts on.
#[test]
fn the_advertised_receipt_carries_the_engine_s_reading() {
    assert_eq!(
        gestures::decode(&frame(ADVERTISED[0])),
        Ok(Ok(Reply::Advertised { wrote: false }))
    );
    assert_eq!(
        gestures::decode(&frame(ADVERTISED[1])),
        Ok(Ok(Reply::Advertised { wrote: true }))
    );
}

/// **A receipt in the shape this foot used to accept is refused now.** That is
/// the bump biting: before PROTOCOL 8 the frame below was the whole of the
/// reply, and a build that read it as *"nothing was restored"* would be
/// answering the reassuring thing on exactly the engine that cannot say.
#[test]
fn the_receipt_the_engine_sent_before_the_bump_is_refused() {
    assert_eq!(
        gestures::decode(&frame(r#"{"kind":"advertised","ok":true}"#)),
        Err("reply: missing field \"wrote\"".to_owned())
    );
}

/// Every queued row decodes — the empty answer, an ordinary row, and the one
/// carrying the worktree lane's `cwd`.
#[test]
fn every_queued_row_decodes() {
    let rows: Vec<Reply> = QUEUED
        .iter()
        .map(|text| {
            gestures::decode(&frame(text))
                .expect("readable")
                .expect("an answer")
        })
        .collect();
    assert_eq!(rows[0], Reply::Invocations(vec![]));
    let Reply::Invocations(ordinary) = &rows[1] else {
        panic!("invocations");
    };
    assert_eq!(ordinary[0].id, "inv-1");
    assert_eq!(ordinary[0].tool, "Bash");
    assert_eq!(ordinary[0].cwd, None);
    let Reply::Invocations(lane) = &rows[2] else {
        panic!("invocations");
    };
    assert_eq!(lane[0].cwd.as_deref(), Some("/w/home/agents/c-1"));
}

/// The slot as it stands: the capture **absent** while the work is still out,
/// and present once there is one — never empty for both.
#[test]
fn the_routed_receipt_tells_unfinished_from_finished_saying_nothing() {
    assert_eq!(
        gestures::decode(&frame(ROUTED[0])),
        Ok(Ok(Reply::Routed {
            invocation: "inv-1".to_owned(),
            capture: None
        }))
    );
    assert_eq!(
        gestures::decode(&frame(ROUTED[1])),
        Ok(Ok(Reply::Routed {
            invocation: "inv-2".to_owned(),
            capture: Some(Capture {
                stdout: "hello\n".to_owned(),
                stderr: "warned\n".to_owned(),
                exit_code: 3,
            })
        }))
    );
}

/// The refusal is the envelope with no `kind`, and it carries the engine's own
/// sentence rather than a class this end invented.
#[test]
fn the_refusal_is_the_envelope_with_no_kind() {
    assert_eq!(
        gestures::decode(&frame(REFUSAL)),
        Ok(Err("unknown op \"fhtagn\"".to_owned()))
    );
}
