//! BEP 44 over the walk: the newest verified item wins a `get`, a `put` lands
//! at the closest token holders and reads back, and a node's refusal is the
//! caller's error.

use super::*;

fn keypair() -> Keypair {
    Keypair::from_seed([9u8; 32]).unwrap()
}

/// An item's target is a hash, not `0xff`, so the topology's closest three
/// are whichever the hash says: `k = 8` asks every node there is, which is
/// what puts a known holder inside the walk.
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

#[test]
fn put_lands_at_the_closest_token_holders_and_reads_back() {
    let kp = keypair();
    let item = kp.sign(vec![], 1, b"presence".to_vec()).unwrap();
    let mut nodes = topology();
    serve(&mut nodes, vec![], vec![]);
    let mut dht = client(vec![nodes[0].addr], wide());
    assert_eq!(dht.put(item.clone()).unwrap(), 4);
    assert_eq!(dht.get(kp.public(), vec![]).unwrap(), Some(item.clone()));
    assert_eq!(nodes[3].held(), vec![item.clone()], "the store is readable");

    // A sequence number the nodes already hold is refused by every one.
    let e = dht.put(item).unwrap_err();
    assert!(
        e.ends_with(": 302 sequence number less than current"),
        "{e}"
    );
    // A newer one supersedes it.
    let next = kp.sign(vec![], 2, b"moved".to_vec()).unwrap();
    assert_eq!(dht.put(next.clone()).unwrap(), 4);
    assert_eq!(dht.get(kp.public(), vec![]).unwrap(), Some(next));
}

#[test]
fn put_with_no_token_holder_is_an_error() {
    let mut a = FakeNode::bind(id(0));
    a.serve(vec![], Mood::Refuse, vec![]);
    let mut dht = client(vec![a.addr], quick());
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
