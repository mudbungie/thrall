//! A reply read as a claim, and a claim filed as an entry.

use super::{Claim, file, fill, read};
use crate::channel::material::{self, ADDRESS, ANCHORS, CHAIN, KEY};
use crate::test_support::{Scratch, mint};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// A reply as the server writes one, under the CA minted in `dir`.
fn reply(dir: &Path) -> Value {
    mint::material(dir);
    let ca = std::fs::read_to_string(dir.join(ANCHORS)).expect("the CA");
    json!({"workspace": "home", "name": "shed", "grade": "foot",
           "address": ["engine.example:7737", "192.0.2.7:7737"],
           "ca": ca, "cert": "the leaf, PEM\n"})
}

/// `reply`, read.
fn claim(dir: &Path) -> Claim {
    read(&reply(dir).to_string()).expect("a claim")
}

/// Every field arrives as stated, and the anchor is the CA's own DER.
#[test]
fn a_reply_reads_as_the_claim_it_states() {
    let scratch = Scratch::new();
    let claim = claim(scratch.path());
    assert_eq!(claim.workspace, "home");
    assert_eq!(claim.name, "shed");
    assert_eq!(claim.grade, "foot");
    assert_eq!(claim.address, ["engine.example:7737", "192.0.2.7:7737"]);
    assert_eq!(claim.anchor.first(), Some(&0x30), "a DER certificate");
}

/// **A reply that could not become an entry refuses, naming what is wrong**
/// — and a workspace that is anything but one directory name is among them,
/// because that field becomes a path on this box.
#[test]
fn a_reply_that_could_not_be_an_entry_refuses() {
    let scratch = Scratch::new();
    let good = reply(scratch.path());
    let without = |field: &str| {
        let mut o = good.clone();
        o.as_object_mut().expect("an object").remove(field);
        o.to_string()
    };
    let with = |field: &str, value: Value| {
        let mut o = good.clone();
        o[field] = value;
        o.to_string()
    };
    let mut cases = vec![
        ("not json".to_owned(), "expected"),
        ("[]".to_owned(), "not a JSON object"),
        (with("address", json!([])), "lists no endpoint"),
        (
            with("address", json!("engine.example:7737")),
            "not an array",
        ),
        (with("ca", json!("no pem here")), "holds no certificate"),
    ];
    for field in ["workspace", "name", "grade", "address", "ca", "cert"] {
        cases.push((without(field), "missing field"));
    }
    for name in ["", ".", "..", "a/b", "/abs", "a/"] {
        cases.push((with("workspace", json!(name)), "not a name an entry"));
    }
    for (body, why) in cases {
        let refusal = read(&body).expect_err(&body);
        assert!(refusal.starts_with("the server's answer is not a registration: "));
        assert!(refusal.contains(why), "{why}: {refusal}");
    }
}

/// **A filed claim is an entry**: the four files, the key its owner's alone,
/// the address one endpoint per line — and `material` reads it back.
#[test]
fn a_filed_claim_is_an_entry_with_a_private_key_only_its_owner_reads() {
    let scratch = Scratch::new();
    let claim = claim(&scratch.join("ca"));
    let dir = scratch.join("wire").join("home");
    file(&dir, &claim, "the key, PEM\n").expect("filed");
    let text = |leaf: &str| std::fs::read_to_string(dir.join(leaf)).expect(leaf);
    assert_eq!(text(KEY), "the key, PEM\n");
    assert_eq!(text(CHAIN), "the leaf, PEM\n");
    assert_eq!(text(ADDRESS), "engine.example:7737\n192.0.2.7:7737\n");
    assert!(text(ANCHORS).starts_with("-----BEGIN CERTIFICATE-----"));
    let mode = |leaf: &str| {
        std::fs::metadata(dir.join(leaf))
            .expect(leaf)
            .permissions()
            .mode()
    };
    assert_eq!(mode(KEY) & 0o777, 0o600);
    let held = material::read_dir(&dir)
        .expect("readable")
        .expect("an entry");
    assert_eq!(held.address, claim.address);
}

/// **An entry directory that exists is never overwritten** — empty or not —
/// and the refusal says the offer is spent. Any other failure is its own
/// sentence, and nothing half-written is called an entry.
#[test]
fn an_existing_entry_is_refused_plainly_and_left_alone() {
    let scratch = Scratch::new();
    let claim = claim(&scratch.join("ca"));
    let dir = scratch.join("home");
    std::fs::create_dir(&dir).expect("the old entry");
    std::fs::write(dir.join("kept"), b"as it was").expect("write");
    let refusal = file(&dir, &claim, "key").expect_err("refused");
    assert!(refusal.contains("already exists"), "{refusal}");
    assert!(refusal.contains("never overwritten"), "{refusal}");
    assert!(refusal.contains("this offer is spent"), "{refusal}");
    let left: Vec<_> = std::fs::read_dir(&dir).expect("ls").flatten().collect();
    assert_eq!(left.len(), 1, "nothing was added");

    let under = scratch.join("kept-file");
    std::fs::write(&under, b"a file where a directory would go").expect("write");
    let refusal = file(&under.join("home"), &claim, "key").expect_err("refused");
    assert!(refusal.contains("could not be filed"), "{refusal}");
    assert!(
        fill(&dir, &claim, "key").is_err(),
        "the same refusal, unworded"
    );
}
