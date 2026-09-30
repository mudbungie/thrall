//! **The third rung, a re-call** (ruling yog bl-278f): a call written from
//! the cached presence with no presence read, and the cache that a call
//! nobody answers clears.

use super::super::*;
use super::roving;
use crate::dht::tests::fake::Mood;
use crate::rendezvous::punch::Punch;
use crate::test_support::roving::{commons, commons_as, pairing, presence, tuned};

/// A far punch port that answers `n` punches, one after another.
pub(super) fn answering(n: usize) -> (u16, std::thread::JoinHandle<usize>) {
    let far = Punch::bind(0).expect("bind");
    let port = far.port();
    let served = std::thread::spawn(move || {
        (0..n)
            .filter(|_| far.punch(vec![], Duration::from_secs(5)).is_some())
            .count()
    });
    (port, served)
}

/// This box's calls on `node`, newest sequence first.
fn calls(node: &crate::test_support::roving::Commons) -> Vec<i64> {
    let inbox = pairing().inbox_keypair().expect("keypair").public();
    let mut seqs: Vec<i64> = node
        .held()
        .into_iter()
        .filter(|i| i.key == inbox)
        .map(|i| i.seq)
        .collect();
    seqs.sort_unstable_by(|a, b| b.cmp(a));
    seqs
}

/// **The third rung** (ruling yog bl-278f): once a call has landed, the
/// next one is written from the cached presence and lands with no presence
/// read — the second commons holds no presence at all, so a read would
/// refuse — and it is one call, under a rising sequence.
#[test]
fn a_landed_call_is_re_called_from_the_cache_without_reading_presence() {
    let (port, served) = answering(2);
    let (node, tuning) = commons(vec![presence(port, 1)]);
    let mut r = roving(tuning);
    assert!(r.recall().is_none(), "nothing cached yet");
    let stream = r.rendezvous().expect("landed");
    assert_eq!(stream.peer_addr().expect("peer").port(), port);
    drop(stream);
    let first = r.last_seq;
    let (empty, tuning) = commons(vec![]);
    r.tuning = tuning;
    let again = r.recall().expect("re-called");
    assert_eq!(again.peer_addr().expect("peer").port(), port);
    drop(again);
    assert_eq!(calls(&empty).len(), 1, "exactly one call was written");
    assert!(calls(&empty).first().is_some_and(|s| *s > first));
    assert_eq!(calls(&node), [first], "and not to the first commons");
    assert!(r.cached.is_some(), "a landed re-call keeps the cache");
    assert_eq!(served.join().expect("served"), 2);
}

/// **A re-call nobody answers clears the cache**, so the next climb reads
/// presence again; a rendezvous nobody answers never filled it.
#[test]
fn a_re_call_nobody_answers_clears_the_cache() {
    let (port, served) = answering(1);
    let (node, tuning) = commons(vec![presence(port, 1)]);
    let mut r = roving(tuning);
    drop(r.rendezvous().expect("landed"));
    assert_eq!(served.join().expect("served"), 1);
    assert!(r.recall().is_none(), "the engine answered no second punch");
    assert!(r.cached.is_none());
    assert_eq!(calls(&node).len(), 1, "the re-call was written");
    assert!(r.recall().is_none(), "and nothing is left to re-call from");
    let (_node, tuning) = commons(vec![presence(super::dead_port(), 1)]);
    let mut unanswered = roving(tuning);
    unanswered.rendezvous().expect_err("nobody there");
    assert!(
        unanswered.cached.is_none(),
        "an unanswered call caches nothing"
    );
}

/// A re-call that cannot be written — no walk can start, or the walk finds
/// no token holder — lands nothing and clears the cache too.
#[test]
fn a_re_call_that_cannot_be_written_clears_the_cache() {
    let (_routers, walled) = commons_as(Mood::Router, vec![]);
    for fallen in [tuned(vec![]), walled] {
        let (port, served) = answering(1);
        let (_node, tuning) = commons(vec![presence(port, 1)]);
        let mut r = roving(tuning);
        drop(r.rendezvous().expect("landed"));
        served.join().expect("served");
        r.tuning = fallen;
        assert!(r.recall().is_none());
        assert!(r.cached.is_none());
    }
}
