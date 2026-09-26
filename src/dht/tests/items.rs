//! BEP 44 over the walk: the newest verified item wins a `get`, a `put` lands
//! at the closest token holders and reads back, and a node's refusal is the
//! caller's error.

use super::*;

fn keypair() -> Keypair {
    Keypair::from_seed([9u8; 32]).unwrap()
}

/// An item's target is a hash, not `0xff`, so the topology's closest three
/// are whichever the hash says: `k = 8` asks every node there is, which is
/// what puts a known holder inside the walk. The bootstrap `A` is asked
/// `find_node`, never `get`, so it holds no token and stores nothing: a `put`
/// lands at the three nodes past it.
fn wide() -> Config {
    Config { k: 8, ..quick() }
}

#[test]
fn get_reads_the_newest_verified_item_and_ignores_a_forged_one() {
    let kp = keypair();
    let older = kp.sign(b"s".to_vec(), 2, b"old".to_vec()).unwrap();
    let newer = kp.sign(b"s".to_vec(), 5, b"new".to_vec()).unwrap();
    let mut nodes = topology();
    serve(&mut nodes, vec![newer.clone()], vec![older.clone()]);
    let mut dht = client(vec![nodes[0].addr], wide());
    assert_eq!(
        dht.get(kp.public(), b"s".to_vec()).unwrap(),
        Some(newer.clone())
    );

    let mut forged = newer;
    forged.value = b"forged".to_vec();
    let mut nodes = topology();
    serve(&mut nodes, vec![forged], vec![older.clone()]);
    let mut dht = client(vec![nodes[0].addr], wide());
    assert_eq!(dht.get(kp.public(), b"s".to_vec()).unwrap(), Some(older));
}

#[test]
fn get_is_none_when_nobody_holds_one() {
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], wide());
    assert_eq!(dht.get(keypair().public(), vec![]).unwrap(), None);
}

/// The live mainline's shape (yog REMOTE §13.7 ruling 3, yog bl-f6e1): the bootstrap
/// is a router that answers `find_node` and never `get`, and the item's home
/// is a node past it. Asking the router `get` — the old walk's first and only
/// round — hears nothing; the walk asks it `find_node` instead, and `get` and
/// `put` of the node it opens onto.
#[test]
fn bep44_walks_past_a_router_that_never_answers_get() {
    let kp = keypair();
    let item = kp.sign(vec![], 1, b"presence".to_vec()).unwrap();
    let mut holder = FakeNode::bind(id(0x42));
    holder.serve(vec![], Mood::Answer, vec![]);
    let mut router = FakeNode::bind(id(0));
    router.serve(vec![holder.node()], Mood::Router, vec![]);
    let mut dht = client(vec![router.addr], quick());

    let mut flight = flight::Flight::new();
    let target = bencode::Dict::from([bencode::entry("target", bencode::bytes(&item.target().0))]);
    assert!(dht.ask(&mut flight, router.addr, false, "get", target));
    assert!(
        dht.land(&mut flight).unwrap().is_none(),
        "the router is silent to get"
    );
    assert!(flight.is_empty());

    assert_eq!(dht.put(item.clone()).unwrap(), 1);
    assert_eq!(dht.get(kp.public(), vec![]).unwrap(), Some(item));
}

#[test]
fn put_lands_at_the_closest_token_holders_and_reads_back() {
    let kp = keypair();
    let item = kp.sign(vec![], 1, b"presence".to_vec()).unwrap();
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], wide());
    assert_eq!(dht.put(item.clone()).unwrap(), 3);
    assert_eq!(dht.get(kp.public(), vec![]).unwrap(), Some(item.clone()));
    assert_eq!(nodes[3].held(), vec![item.clone()], "the store is readable");
    assert_eq!(nodes[0].held(), vec![], "the door stores nothing");

    // A sequence number the nodes already hold is refused by every one.
    let e = dht.put(item).unwrap_err();
    assert!(
        e.ends_with(": 302 sequence number less than current"),
        "{e}"
    );
    // A newer one supersedes it.
    let next = kp.sign(vec![], 2, b"moved".to_vec()).unwrap();
    assert_eq!(dht.put(next.clone()).unwrap(), 3);
    assert_eq!(dht.get(kp.public(), vec![]).unwrap(), Some(next));
}

#[test]
fn put_with_no_token_holder_is_an_error() {
    let mut a = FakeNode::bind(id(1));
    a.serve(vec![], Mood::Refuse, vec![]);
    let door = router(vec![a.node()]);
    let mut dht = client(vec![door.addr], quick());
    let item = keypair().sign(vec![], 1, b"x".to_vec()).unwrap();
    let e = dht.put(item.clone()).unwrap_err();
    assert_eq!(
        e,
        format!("no DHT node near {} offered a write token", item.target())
    );
}

/// A holder that offers a token and never answers `put` is the other zero:
/// the item went out and nobody stored it (yog bl-f519 told the two apart).
#[test]
fn put_to_holders_that_never_answer_is_an_error() {
    let mut mute = FakeNode::bind(id(1));
    mute.serve(vec![], Mood::Mute, vec![]);
    let door = router(vec![mute.node()]);
    let mut dht = client(vec![door.addr], quick());
    let item = keypair().sign(vec![], 1, b"x".to_vec()).unwrap();
    let e = dht.put(item.clone()).unwrap_err();
    assert_eq!(
        e,
        format!("no DHT node stored the item at {}", item.target())
    );
}

#[test]
fn a_forged_item_is_refused_by_the_node_not_the_client() {
    let mut item = keypair().sign(vec![], 1, b"x".to_vec()).unwrap();
    item.seq = 7;
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], wide());
    let e = dht.put(item).unwrap_err();
    assert!(e.ends_with(": 206 invalid signature"), "{e}");
}
