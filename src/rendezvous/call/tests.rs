//! The act against a fake commons and a loopback punch port: every way it
//! refuses, the call it writes, and the cache the third rung re-punches.

use super::*;
use crate::rendezvous::item::Call;
use crate::rendezvous::punch::Punch;
use crate::test_support::roving::{commons, engine, pairing, presence, tuned};

fn roving(tuning: Tuning) -> Roving {
    Roving::new(pairing(), tuning)
}

/// A port nothing listens on.
fn dead_port() -> u16 {
    let gone = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = gone.local_addr().expect("addr").port();
    drop(gone);
    port
}

#[test]
fn the_defaults_are_the_stated_ones() {
    let t = Tuning::default();
    assert_eq!(t.window, Duration::from_secs(35));
    assert_eq!(t.bootstrap, crate::rendezvous::mainline());
    assert_eq!(t.config.k, 8);
}

#[test]
fn no_bootstrap_resolving_refuses_before_any_datagram() {
    for bootstrap in [vec![], vec!["nowhere.invalid:6881".to_owned()]] {
        let mut r = roving(tuned(bootstrap));
        assert_eq!(
            r.rendezvous().expect_err("refused"),
            "no DHT bootstrap node resolved"
        );
    }
}

#[test]
fn no_presence_on_the_commons_refuses() {
    let (_node, tuning) = commons(vec![]);
    let mut r = roving(tuning);
    assert_eq!(
        r.rendezvous().expect_err("refused"),
        "no presence is published under the engine's rendezvous key"
    );
}

#[test]
fn a_presence_sealed_under_another_pairing_refuses() {
    let p = pairing();
    let sealed = crate::rendezvous::item::Presence {
        endpoints: vec!["127.0.0.1:1".parse().expect("addr")],
    }
    .seal(&[7u8; 32])
    .expect("seal");
    let item = engine().sign(p.presence_salt(), 1, sealed).expect("signed");
    let (_node, tuning) = commons(vec![item]);
    let mut r = roving(tuning);
    assert_eq!(
        r.rendezvous().expect_err("refused"),
        "the presence item does not open under this pairing"
    );
}

#[test]
fn a_presence_naming_no_endpoint_refuses() {
    let p = pairing();
    let sealed = crate::rendezvous::item::Presence { endpoints: vec![] }
        .seal(&p.seal_key())
        .expect("seal");
    let item = engine().sign(p.presence_salt(), 1, sealed).expect("signed");
    let (_node, tuning) = commons(vec![item]);
    let mut r = roving(tuning);
    assert_eq!(
        r.rendezvous().expect_err("refused"),
        "the engine's presence names no endpoint"
    );
}

/// The call is written before the punch, sealed so the commons cannot read
/// it, under a sequence that rises on every call — and a punch nobody
/// answers is the window waited out.
#[test]
fn a_call_is_filed_sealed_and_a_punch_nobody_answers_is_refused() {
    let port = dead_port();
    let (node, tuning) = commons(vec![presence(port, 1)]);
    let mut r = roving(tuning);
    let refusal = r.rendezvous().expect_err("nobody there");
    assert_eq!(
        refusal,
        "no SYN crossed toward 1 engine endpoint(s) inside the window"
    );
    let p = pairing();
    let inbox = p.inbox_keypair().expect("keypair").public();
    let held = node.held();
    let call = held
        .iter()
        .find(|i| i.key == inbox && i.salt == p.inbox_salt())
        .expect("the call was filed");
    assert!(call.verify());
    let opened = Call::open(&p.seal_key(), &call.value).expect("opens under the pairing");
    let punch = r.punch.as_ref().expect("bound").port();
    assert!(
        opened
            .endpoints
            .iter()
            .all(|e| e.port() == punch && !e.ip().is_loopback()),
        "{opened:?} names this box at its punch port"
    );
    let first = call.seq;
    r.rendezvous().expect_err("still nobody");
    let again = node
        .held()
        .into_iter()
        .find(|i| i.key == inbox)
        .expect("the call was filed again");
    assert!(again.seq > first, "the sequence rises");
    assert!(again.seq >= r.last_seq);
}

/// A commons that holds a newer call than this box can write refuses the
/// put, and the refusal is the act's.
#[test]
fn a_put_the_commons_refuses_is_the_acts_refusal() {
    let p = pairing();
    let future = p
        .inbox_keypair()
        .expect("keypair")
        .sign(p.inbox_salt(), i64::MAX, vec![])
        .expect("signed");
    let (_node, tuning) = commons(vec![presence(dead_port(), 1), future]);
    let mut r = roving(tuning);
    let refusal = r.rendezvous().expect_err("refused");
    assert!(
        refusal.ends_with("302 sequence number less than current"),
        "{refusal}"
    );
}

/// The whole act lands on a listening punch port, and the endpoints it read
/// are then the cache the third rung punches without the commons.
#[test]
fn a_rendezvous_lands_and_its_endpoints_are_re_punched_without_the_commons() {
    let far = Punch::bind(0).expect("bind");
    let port = far.port();
    let served = std::thread::spawn(move || {
        let first = far.punch(vec![], Duration::from_secs(5)).is_some();
        let second = far.punch(vec![], Duration::from_secs(5)).is_some();
        (first, second)
    });
    let (node, tuning) = commons(vec![presence(port, 1)]);
    let mut r = roving(tuning);
    assert!(r.repunch().is_none(), "nothing cached yet");
    let stream = r.rendezvous().expect("landed");
    assert_eq!(stream.peer_addr().expect("peer").port(), port);
    drop(stream);
    drop(node);
    let again = r.repunch().expect("re-punched at the cache");
    assert_eq!(again.peer_addr().expect("peer").port(), port);
    drop(again);
    assert_eq!(served.join().expect("served"), (true, true));
}

#[test]
fn a_re_punch_nobody_answers_is_none() {
    let (_node, tuning) = commons(vec![presence(dead_port(), 1)]);
    let mut r = roving(tuning);
    r.rendezvous().expect_err("nobody there");
    assert!(r.cached.is_some());
    assert!(r.repunch().is_none());
}

#[test]
fn a_nonce_is_fresh_and_the_clock_rises() {
    assert_ne!(nonce().expect("a"), nonce().expect("b"));
    assert!(unix_now() > 1_700_000_000);
}
