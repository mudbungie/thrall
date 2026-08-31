//! One channel against something that speaks the protocol: a real listener, a
//! real mTLS handshake, a real version preface, real frames.

use super::hello::PROTOCOL;
use super::material::{CHAIN, KEY, Material, read_dir};
use super::{Channel, server_name};
use crate::test_support::engine::Engine;
use crate::test_support::{Scratch, mint};
use serde_json::{Value, json};

/// A scratch box with an engine standing at the far end, answering the n-th
/// dial with the n-th entry of `script`.
fn wired(protocol: u32, script: Vec<Vec<Value>>) -> (Scratch, Engine, Material) {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let engine = Engine::start(scratch.path(), protocol, script);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    (scratch, engine, held)
}

/// An answer of one frame: the ordinary shape.
fn advertised() -> Value {
    json!({"ok": true, "kind": "advertised"})
}

/// **Nothing is dialled when a channel opens**, and what it answers about
/// itself is read off the leaf and the address rather than off any engine.
#[test]
fn opening_a_channel_dials_nothing_and_names_this_box() {
    let scratch = Scratch::new();
    let held = mint::provisioned(scratch.path(), "engine.example:9000");
    let channel = Channel::open(&held).expect("opened");
    assert_eq!(channel.client(), mint::FOOT_NAME);
    assert_eq!(channel.address(), "engine.example:9000");
}

/// **A foot refuses to be configured as anything else** (DESIGN §3.2). The
/// grade is read before the transport, so the refusal is about this box's own
/// configuration and never arrives as a connection failure.
#[test]
fn a_channel_refuses_to_open_on_a_leaf_that_is_not_a_foot() {
    let scratch = Scratch::new();
    let mut held = mint::provisioned(scratch.path(), "engine.example:9000");
    held.chain = scratch.join(&format!("{}.pem", mint::OPERATOR_NAME));
    held.key = scratch.join(&format!("{}.key", mint::OPERATOR_NAME));
    let refusal = Channel::open(&held).expect_err("refused");
    assert!(refusal.contains("not foot grade"), "{refusal}");
}

/// One ask: this end's preface and its request go out in the same breath, the
/// engine is handed exactly those two frames, and the answer comes back as the
/// value it sent.
#[test]
fn one_ask_states_a_version_carries_the_request_and_answers_the_reply() {
    let (_scratch, engine, held) = wired(PROTOCOL, vec![vec![advertised()]]);
    let channel = Channel::open(&held).expect("opened");
    let request = json!({"op": "advertise", "tools": []});
    assert_eq!(channel.ask(&request), Ok(vec![advertised()]));
    assert_eq!(
        engine.heard(),
        vec![json!({ "protocol": PROTOCOL }), request],
        "the preface rides beside the gesture, never inside it"
    );
}

/// **The streaming form is not a second form.** An answer of several frames
/// reads back through the same call, in order, and the terminator ends it.
#[test]
fn an_answer_of_many_frames_reads_back_through_the_same_ask() {
    let rows = vec![json!({"n": 1}), json!({"n": 2}), json!({"n": 3})];
    let (_scratch, _engine, held) = wired(PROTOCOL, vec![rows.clone()]);
    let channel = Channel::open(&held).expect("opened");
    assert_eq!(channel.ask(&json!({"op": "invocations"})), Ok(rows));
}

/// **One connection per ask.** Two asks are two dials, and the engine sees both
/// requests — which is what makes a busy foot absent at the far end.
#[test]
fn each_ask_is_its_own_connection() {
    let (_scratch, engine, held) = wired(PROTOCOL, vec![vec![advertised()], vec![advertised()]]);
    let channel = Channel::open(&held).expect("opened");
    for op in ["advertise", "invocations"] {
        assert!(channel.ask(&json!({ "op": op })).is_ok());
    }
    let ops: Vec<String> = engine
        .heard()
        .iter()
        .filter_map(|v| v.get("op")?.as_str().map(str::to_owned))
        .collect();
    assert_eq!(ops, ["advertise", "invocations"]);
}

/// An engine that ends the stream without answering is an empty answer here,
/// and not an error: what to make of a stream with nothing in it belongs to the
/// gesture above, which is the only layer that knows what it asked for.
#[test]
fn an_engine_that_answers_nothing_ends_the_stream() {
    let (_scratch, _engine, held) = wired(PROTOCOL, vec![vec![]]);
    let channel = Channel::open(&held).expect("opened");
    assert_eq!(channel.ask(&json!({"op": "invocations"})), Ok(Vec::new()));
}

/// **Version skew refuses fail-closed and names both versions** (REMOTE §3),
/// before a frame of the answer is decoded.
#[test]
fn an_engine_of_another_protocol_refuses_and_names_both_versions() {
    let (_scratch, _engine, held) = wired(PROTOCOL + 1, vec![vec![advertised()]]);
    let channel = Channel::open(&held).expect("opened");
    let refusal = channel
        .ask(&json!({"op": "advertise"}))
        .expect_err("refused");
    assert!(
        refusal.contains(&format!("foot speaks version {PROTOCOL}")),
        "{refusal}"
    );
    assert!(
        refusal.contains(&format!("engine speaks {}", PROTOCOL + 1)),
        "{refusal}"
    );
}

/// An engine nothing is listening at is one sentence naming the address. Port 1
/// is below the ephemeral range and unbindable without privilege, so no other
/// test's freed port can answer this dial.
#[test]
fn an_engine_that_is_not_there_names_the_address_it_dialled() {
    let scratch = Scratch::new();
    let held = mint::provisioned(scratch.path(), "127.0.0.1:1");
    let channel = Channel::open(&held).expect("opened");
    let refusal = channel
        .ask(&json!({"op": "advertise"}))
        .expect_err("refused");
    assert!(refusal.contains("connect 127.0.0.1:1"), "{refusal}");
}

/// **An engine this box does not trust gets no boundary at all.** The handshake
/// fails inside rustls, so the refusal is a transport sentence and no frame of
/// the gesture is ever read by anyone.
#[test]
fn an_engine_the_anchors_do_not_cover_never_reaches_the_boundary() {
    let (_scratch, engine, mut held) = wired(PROTOCOL, vec![vec![advertised()]]);
    let elsewhere = Scratch::new();
    mint::material(elsewhere.path());
    held.anchors = elsewhere.join(super::material::ANCHORS);
    let channel = Channel::open(&held).expect("opened");
    assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    assert_eq!(engine.heard(), Vec::<Value>::new(), "nothing was said");
}

/// **An engine that will not accept this box's leaf refuses the same way.** The
/// mint's own CA issued both ends here, so what fails is the client half: a
/// chain the engine's verifier cannot chain to its anchors.
#[test]
fn a_leaf_the_engine_will_not_accept_never_reaches_the_boundary() {
    let (scratch, engine, mut held) = wired(PROTOCOL, vec![vec![advertised()]]);
    let elsewhere = Scratch::new();
    mint::material(elsewhere.path());
    held.chain = elsewhere.join(CHAIN);
    held.key = elsewhere.join(KEY);
    let channel = Channel::open(&held).expect("opened");
    assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    assert_eq!(engine.heard(), Vec::<Value>::new(), "nothing was said");
    assert!(scratch.path().is_dir());
}

/// **An engine that goes away mid-conversation is thrall's own sentence, not
/// its TLS library's** (bl-52ba). It is the failure a running foot will
/// actually hit, and the one place a supervisor's log is the only thing an
/// operator gets — so it names the address that went away and what this box
/// will and will not do about it, exactly as the connect refusals beside it do.
/// The library's account follows; it does not stand in for the sentence.
#[test]
fn an_engine_that_vanishes_names_the_address_and_says_what_happens_next() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let engine = Engine::vanishes(scratch.path());
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    let said = channel
        .ask(&json!({"op": "advertise", "tools": []}))
        .expect_err("the engine went away");
    assert!(said.starts_with("receive "), "the leg comes first: {said}");
    assert!(said.contains(&held.address), "no address: {said}");
    assert!(said.contains("does not reconnect"), "{said}");
    assert!(said.contains("supervision"), "no remedy: {said}");
    assert_eq!(engine.heard().len(), 2, "the foot did speak first");
}

/// The engine's name comes off the address and from nowhere else: an IP
/// literal is an IP identity — bracketed or not — and anything else is a DNS
/// name.
#[test]
fn the_engine_s_name_is_read_off_the_address() {
    for address in [
        "127.0.0.1:9000",
        "[::1]:9000",
        "engine.example:9000",
        "engine.example",
    ] {
        assert!(server_name(address).is_ok(), "{address}");
    }
    let refusal = server_name("not a name:9000").expect_err("refused");
    assert!(refusal.contains("not a server name"), "{refusal}");
}
