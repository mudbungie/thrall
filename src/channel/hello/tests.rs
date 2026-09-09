//! The preface: what this end states, and every way a peer can fail to agree.

use super::{EDITION, FLOOR, PROTOCOL, confirm, state};
use crate::channel::Failure;
use crate::channel::frame;
use serde_json::{Value, json};

/// The bytes a peer stating `protocol` would put on the wire.
fn stated(v: &Value) -> Vec<u8> {
    let mut wire = Vec::new();
    frame::write_value(&mut wire, v).expect("write");
    wire
}

/// What this end writes is one frame carrying the major and the edition —
/// REMOTE §3.2's two keys, and nothing else.
#[test]
fn this_end_states_the_major_and_its_edition_in_one_frame() {
    let mut wire = Vec::new();
    state(&mut wire).expect("state");
    let mut read = wire.as_slice();
    assert_eq!(
        frame::read_value(&mut read).expect("read"),
        Some(json!({ "protocol": PROTOCOL, "edition": EDITION })),
    );
    assert_eq!(
        frame::read_value(&mut read).ok(),
        None,
        "the preface is one frame and nothing follows it here"
    );
}

/// An engine speaking this version is admitted, and says nothing about it.
///
/// It states the number the way the far end states it — `corpus::PROTOCOL`, a
/// literal copied from yog — rather than the pin this file is about, so the two
/// have different sources here as they do on the wire.
#[test]
fn an_engine_of_this_version_is_confirmed_and_answers_its_edition() {
    let wire = stated(&json!({ "protocol": crate::corpus::PROTOCOL, "edition": 21 }));
    assert_eq!(confirm(&mut wire.as_slice()), Ok(21));
}

/// **An engine that states no edition is read as the floor** (REMOTE §3.2),
/// and so is one whose edition is not an edition: the fail-closed equality is
/// the MAJOR's, and this key is no part of it. The floor credits the peer with
/// the least it can be spelling, which is the direction that cannot invent a
/// capability the far end does not have.
#[test]
fn an_engine_that_states_no_readable_edition_is_read_as_the_floor() {
    for preface in [
        json!({ "protocol": crate::corpus::PROTOCOL }),
        json!({ "protocol": crate::corpus::PROTOCOL, "edition": "eight" }),
        json!({ "protocol": crate::corpus::PROTOCOL, "edition": -1 }),
        json!({ "protocol": crate::corpus::PROTOCOL, "edition": u64::from(u32::MAX) + 1 }),
    ] {
        let wire = stated(&preface);
        assert_eq!(confirm(&mut wire.as_slice()), Ok(FLOOR), "{preface}");
    }
}

/// **An unknown key in the preface is ignored** (REMOTE §3.2 rule 1). It is the
/// rule the edition key itself arrived under: a build older than this one met
/// exactly this frame and had to go on.
#[test]
fn a_key_this_end_has_not_heard_of_is_ignored() {
    let wire = stated(&json!({
        "protocol": crate::corpus::PROTOCOL, "edition": 19, "fhtagn": ["deeper"]
    }));
    assert_eq!(confirm(&mut wire.as_slice()), Ok(19));
}

/// A mismatch names BOTH versions and the remedy. That is the requirement, not
/// a nicety: the sentence is the upgrade prompt, so a number an operator can
/// act on has to be in it.
///
/// **And it is a [`Failure::Skew`]**, which is the half that decides a
/// lifetime: a version the two ends do not share is a fact about the two
/// binaries, so every later dial buys the same handshake and the same sentence
/// (bl-5d62).
#[test]
fn a_mismatch_names_both_versions_and_the_remedy() {
    let wire = stated(&json!({ "protocol": 99 }));
    let Failure::Skew(refusal) = confirm(&mut wire.as_slice()).expect_err("refused") else {
        panic!("a stated version this end does not speak is not the wire");
    };
    assert!(
        refusal.contains(&format!("version {PROTOCOL}")),
        "{refusal}"
    );
    assert!(refusal.contains("engine speaks 99"), "{refusal}");
    assert!(refusal.contains("upgrade the older component"), "{refusal}");
    assert!(
        !refusal.contains("negotiat") || refusal.contains("no negotiation"),
        "the refusal must not offer a negotiation: {refusal}"
    );
}

/// Three ways to state nothing in a frame that DID arrive — an unversioned
/// build writing a gesture envelope where a preface belongs, a frame that is
/// not an object, and the key carrying something that is not an integer. All
/// three are one sentence, and all three are skew: the peer deliberately wrote
/// what it wrote, and it will write it again next time.
#[test]
fn a_preface_that_arrived_and_states_nothing_is_skew() {
    let arrived = [
        stated(&json!({"op": "advertise"})),
        stated(&json!(["not an object"])),
        stated(&json!({"protocol": "one"})),
    ];
    for wire in arrived {
        let Failure::Skew(refusal) = confirm(&mut wire.as_slice()).expect_err("refused") else {
            panic!("a frame the peer wrote is not the wire");
        };
        assert!(
            refusal.contains("the engine speaks no version"),
            "{refusal}"
        );
    }
}

/// **A preface that never arrived is the WIRE**, and that is the one place this
/// end parts from REMOTE §3's own collapsing of the two cases (bl-5d62). The
/// engine refuses a peer of the wrong version and a peer that hung up
/// mid-preface with one sentence, because it can serve neither. A foot is
/// asking a different question — whether dialling again could help — and there
/// they part: an engine restarting under a foot's dial says nothing at all, and
/// a foot that took that as final would exit on the blip that a redial exists
/// for.
#[test]
fn a_preface_that_never_arrived_is_the_wire() {
    let Failure::Wire(refusal) = confirm(&mut [].as_slice()).expect_err("refused") else {
        panic!("a peer that hung up mid-preface is not a version this end cannot speak");
    };
    assert!(
        refusal.contains("the engine speaks no version"),
        "the sentence is the same sentence: {refusal}"
    );
}
