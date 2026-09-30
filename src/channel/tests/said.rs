//! **What the ladder says as it climbs** (bl-3958): every rung, the punch and
//! the held line, read back off a captured sink — and a repeated outcome
//! said once.

use super::super::material::read_dir;
use super::super::{Channel, READ_TIMEOUT};
use super::advertised;
use super::ladder::roving_entry;
use crate::rendezvous::say;
use crate::test_support::engine::Engine;
use crate::test_support::engine::punched::{Punched, Turn};
use crate::test_support::roving::{commons, presence, tuned};
use crate::test_support::{Notices, Scratch, mint, roving};
use serde_json::json;
use std::time::Duration;

/// The lines a call's nonce and seq make unpredictable, with both digits
/// runs folded to `N` so a whole ladder can be asserted as one list.
fn folded(lines: Vec<String>) -> Vec<String> {
    lines
        .into_iter()
        .map(|line| {
            let mut out = String::new();
            let mut run = false;
            for c in line.chars() {
                if c.is_ascii_digit() {
                    if !run {
                        out.push('N');
                    }
                    run = true;
                } else {
                    out.push(c);
                    run = false;
                }
            }
            out
        })
        .collect()
}

/// A roving entry whose address answers says the direct rung once, however
/// many asks it dials.
#[test]
fn a_direct_rung_that_answers_is_said_once() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    roving::carried(scratch.path());
    let _engine = Engine::start(
        scratch.path(),
        crate::corpus::PROTOCOL,
        vec![vec![advertised()], vec![advertised()]],
    );
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuned(vec![]));
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    for _ in 0..2 {
        assert!(channel.ask(&json!({"op": "advertise"})).is_ok());
    }
    assert_eq!(notices.heard(), [say::direct_connected()]);
}

/// An entry that does not rove climbs one rung and says nothing of it: its
/// only outcome already reaches the operator as the channel's sentence.
#[test]
fn an_entry_that_does_not_rove_says_nothing() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let _engine = Engine::start(
        scratch.path(),
        crate::corpus::PROTOCOL,
        vec![vec![advertised()]],
    );
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let mut channel = Channel::open(&held).expect("opened");
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    assert!(channel.ask(&json!({"op": "advertise"})).is_ok());
    assert!(notices.heard().is_empty());
}

/// **The whole ladder, said**: the direct rung refused, the presence read,
/// the call written, the punch started and landed, the line kept — and a
/// second ask over the held line, and the pings discarded off it, add one
/// ping line and nothing else.
#[test]
fn the_rendezvous_says_every_step_and_the_held_line_its_first_ping() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let pinged = vec![json!({"ping": true}), json!({"ping": true}), advertised()];
    let engine = Punched::start(
        scratch.path(),
        vec![
            Turn::Answer(vec![advertised()]),
            Turn::Answer(pinged.clone()),
            Turn::Answer(pinged),
        ],
    );
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuning);
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    for _ in 0..3 {
        assert_eq!(
            channel.ask(&json!({"op": "advertise"})),
            Ok(vec![advertised()])
        );
    }
    let heard = notices.heard();
    let call = heard.get(2).cloned().unwrap_or_default();
    assert!(call.ends_with(", 1 ack(s)"), "{call}");
    assert_eq!(
        folded(heard),
        folded(vec![
            say::direct_refused(&["127.0.0.1".parse().expect("ip")], false),
            "rendezvous: presence read — seq 1, 1 endpoint(s) (1 v4)".to_owned(),
            call,
            "rendezvous: punch for call nonce 1 at 1 endpoint(s) (1 v4) — window 800ms".to_owned(),
            "rendezvous: punch landed (1 v4)".to_owned(),
            say::held_kept(),
            say::ping(),
        ])
    );
}

/// **Silence past the bound is said as the held line dropped**, by class.
#[test]
fn a_hangup_is_said_as_the_held_line_silent_past_it() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let engine = Punched::start(scratch.path(), vec![Turn::Silence]);
    let (_node, tuning) = commons(vec![presence(engine.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(Duration::from_millis(300), tuning);
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    assert_eq!(
        notices.heard().last().cloned(),
        Some(
            "rendezvous: held line dropped — the receive failed (silent past the hangup)"
                .to_owned()
        )
    );
}

/// **A re-call nobody answers falls to the full rendezvous** (ruling yog
/// bl-278f): the engine that closed the held line is gone, so the re-call
/// at its cached endpoints expires, the cache is cleared, and the same climb
/// reads presence afresh — which names the engine now listening.
#[test]
fn a_re_call_nobody_answers_is_said_and_presence_is_read_again() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let gone = Punched::start(scratch.path(), vec![Turn::Answer(vec![advertised()])]);
    let (_node, tuning) = commons(vec![presence(gone.port(), 1)]);
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuning);
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    assert!(channel.ask(&json!({"op": "advertise"})).is_ok());
    // The script is spent: the engine closes the line and its punch port.
    assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    let back = Punched::start(scratch.path(), vec![Turn::Answer(vec![advertised()])]);
    let (_node, tuning) = commons(vec![presence(back.port(), 2)]);
    channel.tune(READ_TIMEOUT, tuning);
    assert!(channel.ask(&json!({"op": "advertise"})).is_ok());
    let heard = notices.heard();
    let mut tail: Vec<String> = heard.iter().skip(6).cloned().collect();
    let written = |l: &str, head: &str| l.starts_with(head) && l.ends_with(", 1 ack(s)");
    let recall = tail.get(1).cloned().unwrap_or_default();
    let call = tail.get(5).cloned().unwrap_or_default();
    assert!(
        written(&recall, "rendezvous: re-call from cached presence — nonce "),
        "{heard:?}"
    );
    assert!(written(&call, "rendezvous: call nonce "), "{heard:?}");
    tail.retain(|l| *l != recall && *l != call);
    assert_eq!(
        folded(tail),
        folded(vec![
            "rendezvous: held line dropped — the receive failed (closed by the far end)".to_owned(),
            "rendezvous: punch for call nonce 1 at 1 endpoint(s) (1 v4) — window 800ms".to_owned(),
            "rendezvous: punch expired after 800ms with no stream".to_owned(),
            "rendezvous: presence read — seq 2, 1 endpoint(s) (1 v4)".to_owned(),
            "rendezvous: punch for call nonce 1 at 1 endpoint(s) (1 v4) — window 800ms".to_owned(),
            "rendezvous: punch landed (1 v4)".to_owned(),
            say::held_kept(),
        ]),
        "{heard:?}"
    );
}

/// No rung answering says the direct refusal and why the commons could not
/// be read — the families, never the address.
#[test]
fn no_rung_answering_says_the_direct_refusal_and_the_presence_unread() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "127.0.0.1:1");
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuned(vec![]));
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    for _ in 0..2 {
        assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    }
    assert_eq!(
        notices.heard(),
        [
            "rendezvous: direct rung refused — 1 address(es) tried (1 v4)".to_owned(),
            say::presence_unread(say::NO_WALK),
        ],
        "a second climb to the same outcome says nothing"
    );
}

/// An address that does not resolve tried nothing, and says so by count.
#[test]
fn an_address_that_does_not_resolve_says_nothing_was_tried() {
    let scratch = Scratch::new();
    let held = roving_entry(scratch.path(), "no-port-here");
    let mut channel = Channel::open(&held).expect("opened");
    channel.tune(READ_TIMEOUT, tuned(vec![]));
    let (notices, sink) = Notices::new();
    channel.speak_to(&sink);
    assert!(channel.ask(&json!({"op": "advertise"})).is_err());
    assert_eq!(
        notices.heard().first().cloned(),
        Some("rendezvous: direct rung refused — 0 address(es) tried (none)".to_owned())
    );
}
