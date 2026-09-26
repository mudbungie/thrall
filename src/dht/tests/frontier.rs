//! The walk's frontier (yog bl-d00f): a silent node leaves it, a node the socket
//! cannot reach never spends a query, and a dry frontier re-asks the door
//! while the door names anyone at all, until the query cap.

use super::*;

fn node(fill: u8, mood: Mood, items: Vec<Mutable>) -> FakeNode {
    let mut n = FakeNode::bind(id(fill));
    n.serve(vec![], mood, items);
    n
}

/// The live failure's shape: the door names five nodes, the three closest to
/// the item are silent, the two farther ones answer and hold it. The pool's
/// three closest were all asked and silent, so a walk that ranked the pool
/// ran out of picks at round two although two live nodes sat past them — the
/// dark-commons `Err` (measured against the selection before bl-d00f).
#[test]
fn silent_nodes_leave_the_frontier_for_the_live_ones_past_them() {
    let kp = Keypair::from_seed([9u8; 32]).unwrap();
    let item = kp.sign(b"s".to_vec(), 1, b"held".to_vec()).unwrap();
    let t = item.target().0;
    // Ids at rising distance from the target: flip one bit, higher each time.
    let near = |bit: usize| {
        let mut x = t;
        x[bit / 8] ^= 0x80 >> (bit % 8);
        NodeId(x)
    };
    let mut ranked: Vec<FakeNode> = (0..5).map(|i| FakeNode::bind(near(40 - i * 8))).collect();
    for (i, n) in ranked.iter_mut().enumerate() {
        let (mood, items) = if i < 3 {
            (Mood::Silent, vec![])
        } else {
            (Mood::Answer, vec![item.clone()])
        };
        n.serve(vec![], mood, items);
    }
    let door = router(ranked.iter().map(FakeNode::node).collect());
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(
        dht.get(kp.public(), b"s".to_vec()).unwrap(),
        Some(item.clone())
    );
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(
        dht.lookup(item.target()).unwrap(),
        vec![ranked[3].node(), ranked[4].node()]
    );
}

/// A v6 node is the closest the door names, and a v4 socket cannot send to
/// it. With two queries to spend — the door's and one more — the walk skips
/// it without spending the second, and asks the live node behind it.
#[test]
fn a_node_the_socket_cannot_reach_spends_no_query() {
    let a = node(0x42, Mood::Answer, vec![]);
    let unreachable = Node {
        id: id(0xfe),
        addr: "[::1]:9".parse().unwrap(),
    };
    let door = router(vec![unreachable, a.node()]);
    let config = Config {
        alpha: 1,
        max_queries: 2,
        ..quick()
    };
    let mut dht = client(vec![unreachable.addr, door.addr], config);
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![a.node()]);
}

/// The measured router: one node per answer, a new one each query. Its first
/// two seeds are silent; the walk asks it again and the third seed answers.
#[test]
fn a_dry_frontier_asks_the_door_again_for_fresh_seeds() {
    let silent = [
        node(0x41, Mood::Silent, vec![]),
        node(0x42, Mood::Silent, vec![]),
    ];
    let live = node(0x43, Mood::Answer, vec![]);
    let mut door = FakeNode::bind(id(0x00));
    door.serve(
        vec![silent[0].node(), silent[1].node(), live.node()],
        Mood::Rotor,
        vec![],
    );
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![live.node()]);
}

/// A door that only ever names a dead node is asked again until the query
/// cap, and the walk is the dark-commons `Err`: the dead node is asked once
/// and waited out once, and every later door ask names only it, so the cap
/// of six ends the knocking in one deadline and a few loopback answers.
#[test]
fn a_door_naming_only_the_dead_is_asked_until_the_cap() {
    let silent = node(0x41, Mood::Silent, vec![]);
    let door = router(vec![silent.node()]);
    let config = Config {
        max_queries: 6,
        deadline: Duration::from_millis(100),
        ..quick()
    };
    let mut dht = client(vec![door.addr], config);
    let started = std::time::Instant::now();
    assert!(dht.lookup(id(0xff)).is_err());
    assert!(started.elapsed() < Duration::from_secs(1));
}

/// A node that answers the walk's verb with an error — live, a `get` refused
/// as an unknown query — names nobody and holds no token: it leaves the
/// frontier like a silent one, and the door is asked again past it.
#[test]
fn a_refusal_is_no_reply_and_the_door_is_asked_again() {
    let refusing = node(0x41, Mood::Refuse, vec![]);
    let live = node(0x43, Mood::Answer, vec![]);
    let mut door = FakeNode::bind(id(0x00));
    door.serve(vec![refusing.node(), live.node()], Mood::Rotor, vec![]);
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(dht.lookup(id(0xff)).unwrap(), vec![live.node()]);
}

/// Starved rather than dark: the first seed replies but names nobody, so the
/// frontier is dry with one reply of the three the walk converges on. The
/// door is asked again and its next seed joins the result.
#[test]
fn a_walk_short_of_k_replies_asks_the_door_again() {
    let first = node(0x41, Mood::Answer, vec![]);
    let second = node(0x42, Mood::Answer, vec![]);
    let mut door = FakeNode::bind(id(0x00));
    door.serve(vec![first.node(), second.node()], Mood::Rotor, vec![]);
    let mut dht = client(vec![door.addr], quick());
    assert_eq!(
        dht.lookup(id(0xff)).unwrap(),
        vec![second.node(), first.node()]
    );
}
