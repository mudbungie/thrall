//! The preface: what this end states, and every way a peer can fail to agree.

use super::{PROTOCOL, confirm, state};
use crate::channel::frame;
use serde_json::{Value, json};

/// The bytes a peer stating `protocol` would put on the wire.
fn stated(v: &Value) -> Vec<u8> {
    let mut wire = Vec::new();
    frame::write_value(&mut wire, v).expect("write");
    wire
}

/// What this end writes is one frame carrying one key.
#[test]
fn this_end_states_one_integer_in_one_frame() {
    let mut wire = Vec::new();
    state(&mut wire).expect("state");
    let mut read = wire.as_slice();
    assert_eq!(
        frame::read_value(&mut read).expect("read"),
        Some(json!({ "protocol": PROTOCOL })),
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
fn an_engine_of_this_version_is_confirmed() {
    let wire = stated(&json!({ "protocol": crate::corpus::PROTOCOL }));
    assert_eq!(confirm(&mut wire.as_slice()), Ok(()));
}

/// A mismatch names BOTH versions and the remedy. That is the requirement, not
/// a nicety: the sentence is the upgrade prompt, so a number an operator can
/// act on has to be in it.
#[test]
fn a_mismatch_names_both_versions_and_the_remedy() {
    let wire = stated(&json!({ "protocol": 99 }));
    let refusal = confirm(&mut wire.as_slice()).expect_err("refused");
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

/// Four ways to state nothing, and they are one case: an unversioned build, a
/// frame that is not an object, an object without the key, and a peer that hung
/// up before it said anything.
#[test]
fn every_way_of_stating_nothing_is_the_one_sentence() {
    let silences = [
        stated(&json!({"op": "advertise"})),
        stated(&json!(["not an object"])),
        stated(&json!({"protocol": "one"})),
        Vec::new(),
    ];
    for wire in silences {
        let refusal = confirm(&mut wire.as_slice()).expect_err("refused");
        assert!(
            refusal.contains("the engine speaks no version"),
            "{refusal}"
        );
    }
}
