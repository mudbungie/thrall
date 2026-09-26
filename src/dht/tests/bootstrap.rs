//! The bootstrap is a door, never a result (yog bl-9408): the measured router
//! that names one node eight times, a walk whose learned nodes are all silent,
//! and a node being one `(id, address)`.

use super::*;

/// The router as measured from the deployed engine box: its `find_node`
/// answer is one node repeated eight times.
fn eightfold(node: &FakeNode) -> Vec<Node> {
    vec![node.node(); 8]
}

#[test]
fn a_node_named_eight_times_is_learned_asked_and_answered_once() {
    let mut a = FakeNode::bind(id(0x42));
    a.serve(vec![], Mood::Answer, vec![]);
    let door = router(eightfold(&a));
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![a.node()]);
}

#[test]
fn a_walk_whose_learned_nodes_are_all_silent_is_dark_not_a_success() {
    let mut a = FakeNode::bind(id(0x42));
    a.serve(vec![], Mood::Silent, vec![]);
    let mut b = FakeNode::bind(id(0x43));
    b.serve(vec![], Mood::Silent, vec![]);
    let mut nodes = eightfold(&a);
    nodes.push(b.node());
    let door = router(nodes);
    // The door is re-asked while it names anyone (yog bl-d00f); the cap ends it.
    let config = Config {
        max_queries: 9,
        ..quick()
    };
    let mut dht = client(vec![door.addr], config);
    let dark = format!("no DHT node answered find_node for {}", "ff".repeat(20));
    assert_eq!(dht.lookup(id(0xff)).unwrap_err(), dark);
    // The BEP 44 verbs inherit it: nothing near the target, nothing to read
    // or write, and the caller is told so rather than handed an empty read.
    let kp = Keypair::from_seed([9u8; 32]).unwrap();
    assert!(
        dht.get(kp.public(), vec![])
            .unwrap_err()
            .starts_with("no DHT node answered get")
    );
}

#[test]
fn a_bootstrap_that_answers_and_names_nobody_is_dark_too() {
    let door = router(vec![]);
    let mut dht = client(vec![door.addr], quick());
    assert!(
        dht.lookup(id(0xff)).is_err(),
        "the router is not near anything"
    );
    let mut refusing = FakeNode::bind(id(0));
    refusing.serve(vec![], Mood::Refuse, vec![]);
    let mut dht = client(vec![refusing.addr], quick());
    assert!(dht.lookup(id(0xff)).is_err(), "nor is its refusal a result");
}

#[test]
fn one_id_at_two_addresses_is_two_nodes() {
    let mut a = FakeNode::bind(id(0x42));
    a.serve(vec![], Mood::Answer, vec![]);
    let mut twin = FakeNode::bind(id(0x42));
    twin.serve(vec![], Mood::Answer, vec![]);
    let door = router(vec![a.node(), twin.node(), a.node(), twin.node()]);
    let mut dht = client(vec![door.addr], quick());
    let mut found = dht.lookup(id(0xff)).unwrap();
    found.sort_by_key(|n| n.addr);
    let mut want = vec![a.node(), twin.node()];
    want.sort_by_key(|n| n.addr);
    assert_eq!(found, want);
}

#[test]
fn the_topology_walk_never_answers_the_bootstrap() {
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], Config { k: 8, ..quick() });
    let found = dht.lookup(id(0xff)).unwrap();
    assert_eq!(
        found,
        vec![nodes[3].node(), nodes[2].node(), nodes[1].node()]
    );
}
