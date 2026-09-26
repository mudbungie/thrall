//! The iterative walk: it hops, it converges closest-first, it is bounded,
//! and every way a node misbehaves leaves it standing.

use super::*;

#[test]
fn a_walk_hops_toward_the_target_and_answers_closest_first() {
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], quick());
    let found = dht.lookup(id(0xff)).unwrap();
    assert_eq!(
        found,
        vec![nodes[3].node(), nodes[2].node(), nodes[1].node()]
    );
}

#[test]
fn a_walk_stops_at_its_query_cap() {
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(
        vec![nodes[0].addr],
        Config {
            max_queries: 3,
            ..quick()
        },
    );
    // The bootstrap and two more — `C` and `B`, never a fourth: `D`, two hops
    // out and learned from `B`, is never asked, because the window counts
    // every query against the cap as it sends it.
    assert_eq!(
        dht.lookup(id(0xff)).unwrap(),
        vec![nodes[2].node(), nodes[1].node()]
    );
}

#[test]
fn no_bootstrap_is_an_error_before_any_datagram() {
    let mut dht = client(vec![], quick());
    assert_eq!(dht.lookup(id(1)).unwrap_err(), "no bootstrap node to ask");
}

#[test]
fn a_silent_commons_is_an_error() {
    let mut a = FakeNode::bind(id(0));
    a.serve(vec![], Mood::Silent, vec![]);
    let mut dht = client(vec![a.addr], quick());
    let e = dht.lookup(id(0xff)).unwrap_err();
    assert_eq!(
        e,
        format!("no DHT node answered find_node for {}", "ff".repeat(20))
    );
}

#[test]
fn a_zero_deadline_never_waits() {
    let mut a = FakeNode::bind(id(0));
    a.serve(vec![], Mood::Answer, vec![]);
    let mut dht = client(
        vec![a.addr],
        Config {
            deadline: Duration::ZERO,
            ..quick()
        },
    );
    assert!(
        dht.lookup(id(0xff))
            .unwrap_err()
            .starts_with("no DHT node answered")
    );
}

#[test]
fn a_node_that_refuses_is_heard_but_is_no_result() {
    let mut a = FakeNode::bind(id(1));
    a.serve(vec![], Mood::Refuse, vec![]);
    let door = router(vec![a.node()]);
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![]);
}

#[test]
fn noise_on_the_socket_is_not_an_answer() {
    let mut garbage = FakeNode::bind(id(1));
    garbage.serve(vec![], Mood::Garbage, vec![]);
    let mut anonymous = FakeNode::bind(id(2));
    anonymous.serve(vec![], Mood::Anonymous, vec![]);
    let mut stray = FakeNode::bind(id(3));
    stray.serve(vec![], Mood::Stray, vec![]);
    let mut a = FakeNode::bind(id(4));
    a.serve(vec![], Mood::Answer, vec![]);
    let noisy = [&garbage, &anonymous, &stray, &a];
    let door = router(noisy.iter().map(|n| n.node()).collect());
    let mut dht = client(
        vec![door.addr],
        Config {
            alpha: 4,
            ..quick()
        },
    );
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![a.node()]);
}
