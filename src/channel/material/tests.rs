//! The three answers a directory can give.

use super::{ADDRESS, ANCHORS, CHAIN, KEY, read_dir};
use crate::test_support::Scratch;
use std::path::Path;

/// Write every file a provisioned channel holds, with `address` as stated.
fn provision(dir: &Path, address: &str) {
    for file in [ANCHORS, CHAIN, KEY] {
        std::fs::write(dir.join(file), b"pem bytes stand in here").expect("write");
    }
    std::fs::write(dir.join(ADDRESS), address).expect("write");
}

/// An empty directory holds no channel, and that is an answer. Removing the
/// material deletes config, not code.
#[test]
fn nothing_provisioned_is_an_answer_and_not_an_error() {
    let tmp = Scratch::new();
    assert_eq!(read_dir(tmp.path()), Ok(None));
}

/// A directory that does not exist reads exactly as an empty one: absent and
/// empty are one fact about what this box holds.
#[test]
fn a_directory_that_is_not_there_reads_as_nothing_provisioned() {
    let tmp = Scratch::new();
    assert_eq!(read_dir(&tmp.path().join("never-made")), Ok(None));
}

/// Everything present is the material, with the address trimmed.
#[test]
fn a_provisioned_directory_answers_its_material() {
    let tmp = Scratch::new();
    provision(tmp.path(), "  engine.example:9000\n");
    let held = read_dir(tmp.path())
        .expect("readable")
        .expect("provisioned");
    assert_eq!(held.address, "engine.example:9000");
    assert_eq!(held.anchors, tmp.path().join(ANCHORS));
    assert_eq!(held.chain, tmp.path().join(CHAIN));
    assert_eq!(held.key, tmp.path().join(KEY));
}

/// Half a trust store refuses, and names EVERY gap at once — a remedy that
/// reveals one missing file per run is a remedy run four times.
#[test]
fn a_half_provisioned_directory_names_every_gap_at_once() {
    let tmp = Scratch::new();
    provision(tmp.path(), "engine.example:9000");
    std::fs::remove_file(tmp.path().join(KEY)).expect("rm");
    std::fs::remove_file(tmp.path().join(ADDRESS)).expect("rm");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains(KEY), "{refusal}");
    assert!(refusal.contains(ADDRESS), "{refusal}");
    assert!(refusal.contains("half-provisioned"), "{refusal}");
}

/// The remedy is an act on another box by another hand, and the refusal says
/// so. thrall must never look like something that could mint its own way in.
#[test]
fn every_refusal_says_the_material_arrives_by_hand() {
    let tmp = Scratch::new();
    provision(tmp.path(), "engine.example:9000");
    std::fs::remove_file(tmp.path().join(ANCHORS)).expect("rm");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains("thrall mints nothing"), "{refusal}");
    assert!(refusal.contains("carried here by hand"), "{refusal}");
}

/// An address file that is empty, whitespace, or unreadable is one refusal:
/// they are one fact about what this box can be told to dial.
#[test]
fn an_address_that_says_nothing_refuses_however_it_says_it() {
    for stated in ["", "   \n"] {
        let tmp = Scratch::new();
        provision(tmp.path(), stated);
        let refusal = read_dir(tmp.path()).expect_err("refused");
        assert!(refusal.contains("names no address"), "{refusal}");
    }
    // Unreadable takes the same branch: a directory where the file goes.
    let tmp = Scratch::new();
    provision(tmp.path(), "engine.example:9000");
    std::fs::remove_file(tmp.path().join(ADDRESS)).expect("rm");
    std::fs::create_dir(tmp.path().join(ADDRESS)).expect("mkdir");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains("half-provisioned"), "{refusal}");
}
