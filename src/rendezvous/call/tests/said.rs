//! **What the act says** (bl-3958): each way the presence read ends, the
//! call write refused, and a punch that expires — families and counts, the
//! DHT's own reasons withheld.

use super::super::*;
use super::{dead_port, roving};
use crate::dht::tests::fake::Mood;
use crate::rendezvous::say::{self, Say};
use crate::test_support::Notices;
use crate::test_support::roving::{commons, commons_as, engine, pairing, presence};

/// A roving half speaking into a recorder.
fn heard(tuning: Tuning) -> (Roving, Notices) {
    let (notices, sink) = Notices::new();
    let mut r = roving(tuning);
    r.say = Say::to(&sink);
    (r, notices)
}

#[test]
fn a_presence_that_is_not_there_is_said_unread() {
    let (_node, tuning) = commons(vec![]);
    let (mut r, notices) = heard(tuning);
    r.rendezvous().expect_err("refused");
    assert_eq!(notices.heard(), [say::presence_unread(say::NONE)]);
}

#[test]
fn a_walk_that_fails_is_said_without_its_reason() {
    let (_node, tuning) = commons_as(Mood::Silent, vec![]);
    let (mut r, notices) = heard(tuning);
    let why = r.rendezvous().expect_err("refused");
    assert_eq!(notices.heard(), [say::presence_unread(say::WALK)]);
    assert!(!notices.heard().concat().contains(&why));
}

#[test]
fn a_presence_under_another_pairing_is_said_unopened() {
    let p = pairing();
    let sealed = crate::rendezvous::item::Presence {
        endpoints: vec!["127.0.0.1:1".parse().expect("addr")],
    }
    .seal(&[7u8; 32])
    .expect("seal");
    let item = engine().sign(p.presence_salt(), 1, sealed).expect("signed");
    let (_node, tuning) = commons(vec![item]);
    let (mut r, notices) = heard(tuning);
    r.rendezvous().expect_err("refused");
    assert_eq!(notices.heard(), [say::presence_unread(say::SEALED)]);
}

#[test]
fn a_presence_naming_nothing_is_said_read_with_none() {
    let p = pairing();
    let sealed = crate::rendezvous::item::Presence { endpoints: vec![] }
        .seal(&p.seal_key())
        .expect("seal");
    let item = engine().sign(p.presence_salt(), 4, sealed).expect("signed");
    let (_node, tuning) = commons(vec![item]);
    let (mut r, notices) = heard(tuning);
    r.rendezvous().expect_err("refused");
    assert_eq!(
        notices.heard(),
        ["rendezvous: presence read — seq 4, 0 endpoint(s) (none)"]
    );
}

#[test]
fn a_call_the_commons_refuses_is_said_unwritten() {
    let p = pairing();
    let future = p
        .inbox_keypair()
        .expect("keypair")
        .sign(p.inbox_salt(), i64::MAX, vec![])
        .expect("signed");
    let (_node, tuning) = commons(vec![presence(dead_port(), 1), future]);
    let (mut r, notices) = heard(tuning);
    r.rendezvous().expect_err("refused");
    let said = notices.heard();
    assert_eq!(said.len(), 2, "{said:?}");
    assert!(
        said.last()
            .is_some_and(|l| l.starts_with("rendezvous: call nonce ")
                && l.ends_with(&format!(" not written — {}", say::WALK))),
        "{said:?}"
    );
}

/// A punch nobody answers is said started and expired; the re-punch at the
/// cache says itself, and its expiry — the same outcome again — once.
#[test]
fn a_punch_nobody_answers_is_said_expired_once() {
    let (_node, tuning) = commons(vec![presence(dead_port(), 1)]);
    let (mut r, notices) = heard(tuning);
    r.rendezvous().expect_err("nobody there");
    assert!(r.repunch().is_none());
    let said = notices.heard();
    let tail: Vec<String> = said.iter().skip(3).cloned().collect();
    assert_eq!(
        tail,
        [
            "rendezvous: punch expired after 800ms with no stream",
            "rendezvous: re-punch at 1 cached endpoint(s) (1 v4) — window 800ms",
        ],
        "{said:?}"
    );
}
