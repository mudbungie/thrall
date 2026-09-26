//! **The dial ladder, rung by rung** (DESIGN §3.11): a direct address that
//! answers, one that does not and the rendezvous under it, the held
//! connection that comes back, the ping discarded off it, the silence that
//! hangs it up, and the re-punch that needs no commons.

use super::super::material::read_dir;
use super::super::{Channel, Failure, READ_TIMEOUT};
use crate::test_support::engine::Engine;
use crate::test_support::engine::punched::{Punched, Turn};
use crate::test_support::roving::{commons, presence, tuned};
use crate::test_support::{Scratch, mint, roving};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Duration;

fn advertised() -> Value {
    json!({"ok": true, "kind": "advertised", "wrote": false})
}

/// A roving entry: the operator carried the pairing beside the four files.
fn roving_entry(dir: &Path, address: &str) -> crate::channel::material::Material {
    mint::material(dir);
    roving::carried(dir);
    std::fs::write(dir.join(super::super::material::ADDRESS), address).expect("address");
    read_dir(dir).expect("readable").expect("provisioned")
}

/// The number of prefaces a punched engine heard — one per connection.
fn connections(engine: &Punched) -> usize {
    engine
        .heard()
        .iter()
        .filter(|v| v.get("protocol").is_some())
        .count()
}

/// **The second rung**: a roving entry whose address answers is dialled
/// directly, per ask, and the commons is never consulted — the bootstrap
/// here resolves to nothing, so a walk would refuse.
#[test]
fn a_direct_address_that_answers_is_taken_and_stays_one_per_ask() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    roving::carried(scratch.path());
    let engine = Engine::start(
        scratch.path(),
        crate::corpus::PROTOCOL,
        vec![vec![advertised()], vec![advertised()]],
    );
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuned(vec![]));
    for _ in 0..2 {
        assert_eq!(
            channel.ask(&json!({"op": "advertise"})),
            Ok(vec![advertised()])
        );
    }
    assert_eq!(engine.heard().len(), 4, "two connections, two prefaces");
    assert!(channel.held.is_none(), "a dialled connection is not held");
}

/// **The fourth rung, and the connection is held.** The direct address
/// refuses at once, nothing is cached, so the channel reads presence off the
/// commons, files its call, punches — and the second ask rides the same
/// connection with no second preface.
#[test]
fn a_dead_address_falls_through_to_the_rendezvous_and_the_connection_is_held() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(
        scratch.path(),
        vec![
            Turn::Answer(vec![advertised()]),
            Turn::Answer(vec![advertised()]),
        ],
    );
    let (node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuning);
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
    assert!(channel.held.is_some(), "a punched connection is held");
    assert_eq!(
        channel.ask(&json!({"op": "invocations"})),
        Ok(vec![advertised()])
    );
    assert_eq!(connections(&engine), 1, "one connection carried both asks");
    assert_eq!(engine.heard().len(), 3, "a preface and two requests");
    assert_eq!(
        node.held().len(),
        2,
        "the presence, and this box's call beside it"
    );
}

/// **A ping is discarded wherever a reply is read** (REMOTE §13.4): the
/// engine writes `{"ping":true}` into a held connection's silence, and it is
/// never the start of a reply stream.
#[test]
fn a_ping_in_the_reply_stream_is_discarded() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(
        scratch.path(),
        vec![Turn::Answer(vec![
            json!({"ping": true}),
            json!({"ping": true}),
            advertised(),
        ])],
    );
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuning);
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
}

/// **Two minutes of silence is the hangup**, and it is the wire — dialled
/// again by the loop above. The bound is shortened here; its production
/// value is asserted beside it.
#[test]
fn silence_past_the_bound_hangs_up_as_the_wire() {
    assert_eq!(READ_TIMEOUT, Duration::from_mins(2));
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(scratch.path(), vec![Turn::Silence]);
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(Duration::from_millis(300), tuning);
    let Err(Failure::Wire(said)) = channel.ask(&json!({"op": "advertise"})) else {
        panic!("silence is the wire");
    };
    assert!(said.starts_with("receive 127.0.0.1:1"), "{said}");
    assert!(channel.held.is_none(), "a hung-up connection is not held");
}

/// **The third rung**: a held connection that drops is re-punched at the
/// endpoints the run cached, and the commons is not consulted — the fake node
/// is gone by then, so a walk would refuse.
#[test]
fn a_dropped_held_connection_is_re_punched_without_the_commons() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(
        scratch.path(),
        vec![
            Turn::Answer(vec![advertised()]),
            Turn::Vanish,
            Turn::Answer(vec![advertised()]),
        ],
    );
    let (node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuning);
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
    drop(node);
    let Err(Failure::Wire(said)) = channel.ask(&json!({"op": "invocations"})) else {
        panic!("the engine vanished");
    };
    assert!(said.starts_with("receive 127.0.0.1:1"), "{said}");
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
    assert_eq!(connections(&engine), 2, "a second connection landed");
}

/// **Every rung refusing is one sentence naming them all**, so an operator
/// reads which engine, which address, and why the commons did not help.
#[test]
fn no_rung_answering_names_the_address_and_the_rendezvous() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuned(vec![]));
    let Err(Failure::Wire(said)) = channel.ask(&json!({"op": "advertise"})) else {
        panic!("nothing answered");
    };
    assert!(said.starts_with("connect 127.0.0.1:1"), "{said}");
    assert!(
        said.ends_with("; rendezvous: no DHT bootstrap node resolved"),
        "{said}"
    );
}

/// A held connection the engine has already closed fails on the send or the
/// receive as the wire, and is dropped.
#[test]
fn a_held_connection_the_engine_closed_is_the_wire() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(scratch.path(), vec![Turn::Answer(vec![advertised()])]);
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(Duration::from_secs(3), tuning);
    assert!(channel.ask(&json!({"op": "advertise"})).is_ok());
    // The script is spent: the engine drops the connection on the next read.
    assert!(matches!(
        channel.ask(&json!({"op": "advertise"})),
        Err(Failure::Wire(_))
    ));
    assert!(channel.held.is_none());
}
