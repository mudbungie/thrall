//! The observed endpoint (BEP 42): the walk hears every answering node's
//! claim, and `observed` answers only what the most of them agree on (yog
//! bl-efae, ported by bl-d340).

use super::*;

/// One walk over nodes of `moods`, all asked in the round after the router's.
fn walk(moods: &[Mood]) -> Vec<SocketAddr> {
    let mut nodes: Vec<FakeNode> = (1..=u8::try_from(moods.len()).unwrap())
        .map(|i| FakeNode::bind(id(i)))
        .collect();
    for (node, mood) in nodes.iter_mut().zip(moods) {
        node.serve(vec![], *mood, vec![]);
    }
    let door = router(nodes.iter().map(FakeNode::node).collect());
    let mut dht = client(
        vec![door.addr],
        Config {
            alpha: moods.len(),
            k: moods.len(),
            ..quick()
        },
    );
    assert!(dht.observed().is_empty(), "no walk, no claim");
    dht.lookup(id(0xff)).unwrap();
    dht.observed()
}

fn at(s: &str) -> SocketAddr {
    s.parse().unwrap()
}

#[test]
fn three_agreeing_outvote_one_dissenting() {
    let (us, liar) = (at("203.0.113.7:6881"), at("198.51.100.9:6881"));
    let moods = [
        Mood::Claim(liar),
        Mood::Claim(us),
        Mood::Claim(us),
        Mood::Claim(us),
    ];
    assert_eq!(walk(&moods), vec![us]);
}

#[test]
fn a_tie_is_no_answer() {
    let (a, b) = (at("203.0.113.7:6881"), at("198.51.100.9:6881"));
    assert_eq!(walk(&[Mood::Claim(a), Mood::Claim(b)]), vec![]);
    // The same address at two ports is two claims, not one.
    let moved = at("203.0.113.7:1025");
    assert_eq!(walk(&[Mood::Claim(a), Mood::Claim(moved)]), vec![]);
}

#[test]
fn each_family_is_voted_on_its_own() {
    let (v4, v6) = (at("203.0.113.7:6881"), at("[2001:db8::7]:6881"));
    let moods = [Mood::Claim(v6), Mood::Claim(v4), Mood::Answer];
    assert_eq!(walk(&moods), vec![v4, v6]);
}

#[test]
fn a_walk_where_nobody_claims_observes_nothing() {
    assert_eq!(walk(&[Mood::Answer, Mood::Answer]), vec![]);
}

#[test]
fn a_dark_walk_forgets_what_the_last_one_heard() {
    let us = at("203.0.113.7:6881");
    let mut a = FakeNode::bind(id(1));
    a.serve(vec![], Mood::Claim(us), vec![]);
    let mut dark = FakeNode::bind(id(2));
    dark.serve(vec![], Mood::Silent, vec![]);
    let door = router(vec![a.node()]);
    let mut dht = client(vec![door.addr], quick());
    dht.lookup(id(0xff)).unwrap();
    assert_eq!(dht.observed(), vec![us]);
    dht.bootstrap = vec![dark.addr];
    assert!(dht.lookup(id(0xff)).is_err());
    assert_eq!(dht.observed(), vec![], "the last walk heard nobody");
}
