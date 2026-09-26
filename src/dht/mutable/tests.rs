//! Signing, verifying, the target, and the exact bytes a signature covers —
//! held to fixtures the ENGINE's own code produced, so interoperation is
//! asserted rather than assumed (yog bl-0247).

use super::*;
use crate::rendezvous::pairing::unhex;

fn keypair() -> Keypair {
    Keypair::from_seed([7u8; 32]).unwrap()
}

#[test]
fn the_signed_bytes_are_the_beps_spelling() {
    assert_eq!(
        signed_bytes(b"foobar", 1, b"Hello World!"),
        b"4:salt6:foobar3:seqi1e1:v12:Hello World!"
    );
    assert_eq!(
        signed_bytes(b"", 1, b"Hello World!"),
        b"3:seqi1e1:v12:Hello World!"
    );
}

/// **The engine's bytes, verbatim.** `Keypair::from_seed([1; 32])` signing
/// `salt`/`7`/`value` on yog answers exactly this key, this signature and
/// this target; ed25519 is deterministic, so this end must answer the same.
#[test]
fn an_item_signs_byte_for_byte_as_the_engine_does() {
    let kp = Keypair::from_seed([1u8; 32]).unwrap();
    let item = kp.sign(b"salt".to_vec(), 7, b"value".to_vec()).unwrap();
    assert_eq!(
        item.key.to_vec(),
        unhex("8a88e3dd7409f195fd52db2d3cba5d72ca6709bf1d94121bf3748801b40f6f5c").unwrap()
    );
    assert_eq!(
        item.sig.to_vec(),
        unhex(
            "8624b29894bb37f90502dd5acf7b03cd1d2fd9ad2a5caadd919a9665e4c3df9a\
             2171cfcfaf46028baceef14683709b96d70cbdc470168856d930b7c5f9675702"
        )
        .unwrap()
    );
    assert_eq!(
        item.target().to_string(),
        "6c02e856b30dc29b4bcbd6aba291ce17a9cc9fa7"
    );
    assert_eq!(
        signed_bytes(b"salt", 7, b"value"),
        unhex("343a73616c74343a73616c74333a736571693765313a76353a76616c7565").unwrap()
    );
}

#[test]
fn an_item_signs_verifies_and_fails_when_touched() {
    let kp = keypair();
    let item = kp.sign(b"salt".to_vec(), 3, b"value".to_vec()).unwrap();
    assert_eq!(item.key, kp.public());
    assert!(item.verify());
    let mut forged = item.clone();
    forged.seq += 1;
    assert!(!forged.verify());
    let mut other_key = item.clone();
    other_key.key = Keypair::from_seed([8u8; 32]).unwrap().public();
    assert!(!other_key.verify());
    // The same seed is the same key, so a stored seed comes back whole.
    assert_eq!(keypair().public(), kp.public());
}

#[test]
fn a_value_over_the_beps_cap_is_refused_before_it_leaves() {
    let kp = keypair();
    assert!(kp.sign(vec![], 1, vec![0u8; 996]).is_ok());
    let e = kp.sign(vec![], 1, vec![0u8; 997]).unwrap_err();
    assert_eq!(e, "value bencodes to 1001 bytes; BEP 44 allows 1000");
}

#[test]
fn the_target_is_sha1_of_key_and_salt() {
    let key = [1u8; 32];
    let plain = target_of(&key, b"");
    let salted = target_of(&key, b"s");
    assert_ne!(plain, salted);
    let expect = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, &key);
    assert_eq!(plain.0.as_slice(), expect.as_ref());
    let item = keypair().sign(b"s".to_vec(), 1, vec![]).unwrap();
    assert_eq!(item.target(), target_of(&keypair().public(), b"s"));
}

#[test]
fn a_reply_yields_its_item_only_when_it_verifies() {
    let kp = keypair();
    let item = kp.sign(b"s".to_vec(), 5, b"v".to_vec()).unwrap();
    let mut r = Dict::from([
        entry("seq", Value::Int(5)),
        entry("sig", bytes(&item.sig)),
        entry("v", bytes(b"v")),
    ]);
    assert_eq!(
        Mutable::from_reply(&r, kp.public(), b"s"),
        Some(item.clone())
    );
    // Under another salt the same bytes verify nothing.
    assert_eq!(Mutable::from_reply(&r, kp.public(), b"t"), None);
    // A reply that spells no item, or a non-string value, is nothing.
    assert_eq!(Mutable::from_reply(&Dict::new(), kp.public(), b"s"), None);
    r.insert(b"v".to_vec(), Value::Int(1));
    assert_eq!(Mutable::from_reply(&r, kp.public(), b"s"), None);
    r.insert(b"sig".to_vec(), bytes(b"short"));
    assert_eq!(Mutable::from_reply(&r, kp.public(), b"s"), None);
}

#[test]
fn put_arguments_carry_the_salt_only_when_there_is_one() {
    let kp = keypair();
    let salted = kp.sign(b"s".to_vec(), 1, b"v".to_vec()).unwrap();
    let args = salted.put_args(b"tok");
    assert_eq!(args.get(b"salt".as_slice()), Some(&bytes(b"s")));
    assert_eq!(args.get(b"token".as_slice()), Some(&bytes(b"tok")));
    assert_eq!(args.get(b"k".as_slice()), Some(&bytes(&kp.public())));
    assert_eq!(args.get(b"seq".as_slice()), Some(&Value::Int(1)));
    assert_eq!(args.get(b"v".as_slice()), Some(&bytes(b"v")));
    assert_eq!(args.get(b"sig".as_slice()), Some(&bytes(&salted.sig)));
    let plain = kp.sign(vec![], 1, b"v".to_vec()).unwrap();
    assert_eq!(plain.put_args(b"tok").get(b"salt".as_slice()), None);
}
