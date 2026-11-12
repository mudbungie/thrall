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

/// Everything present is the material, with the address trimmed — and a file
/// of one line is a list of one.
#[test]
fn a_provisioned_directory_answers_its_material() {
    let tmp = Scratch::new();
    provision(tmp.path(), "  server.example:9000\n");
    let held = read_dir(tmp.path())
        .expect("readable")
        .expect("provisioned");
    assert_eq!(held.address, ["server.example:9000"]);
    assert_eq!(held.anchors, tmp.path().join(ANCHORS));
    assert_eq!(held.chain, tmp.path().join(CHAIN));
    assert_eq!(held.key, tmp.path().join(KEY));
}

/// Half a trust store refuses, and names EVERY gap at once — a remedy that
/// reveals one missing file per run is a remedy run four times.
#[test]
fn a_half_provisioned_directory_names_every_gap_at_once() {
    let tmp = Scratch::new();
    provision(tmp.path(), "server.example:9000");
    std::fs::remove_file(tmp.path().join(KEY)).expect("rm");
    std::fs::remove_file(tmp.path().join(ADDRESS)).expect("rm");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains(KEY), "{refusal}");
    assert!(refusal.contains(ADDRESS), "{refusal}");
    assert!(refusal.contains("half-provisioned"), "{refusal}");
}

/// The remedy is filing an entry whole — by registering, or by the operator's
/// hand — and the refusal names both: a gap is never patched file by file.
#[test]
fn every_refusal_names_the_two_ways_an_entry_is_filed() {
    let tmp = Scratch::new();
    provision(tmp.path(), "server.example:9000");
    std::fs::remove_file(tmp.path().join(ANCHORS)).expect("rm");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains("thrall register <url>"), "{refusal}");
    assert!(refusal.contains("carried here"), "{refusal}");
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
    provision(tmp.path(), "server.example:9000");
    std::fs::remove_file(tmp.path().join(ADDRESS)).expect("rm");
    std::fs::create_dir(tmp.path().join(ADDRESS)).expect("mkdir");
    let refusal = read_dir(tmp.path()).expect_err("refused");
    assert!(refusal.contains("half-provisioned"), "{refusal}");
}

/// **The address file is a list, one endpoint per line, in dial order**
/// (REMOTE §15.3) — blank lines and stray whitespace say nothing.
#[test]
fn an_address_file_of_several_lines_is_the_list_in_its_order() {
    let tmp = Scratch::new();
    provision(
        tmp.path(),
        "server.example:7737\n\n  192.0.2.7:7737 \n[2001:db8::7]:7737\n",
    );
    let held = read_dir(tmp.path())
        .expect("readable")
        .expect("provisioned");
    assert_eq!(
        held.address,
        [
            "server.example:7737",
            "192.0.2.7:7737",
            "[2001:db8::7]:7737"
        ]
    );
}

/// **The retired rendezvous files are ignored, never required and never
/// read** (REMOTE §15.4): an entry filed before the public wire still holds
/// them, garbage or not, and is the same entry without them.
#[test]
fn an_entry_still_holding_the_retired_rendezvous_files_simply_works() {
    let tmp = Scratch::new();
    provision(tmp.path(), "server.example:9000");
    let plain = read_dir(tmp.path()).expect("readable");
    std::fs::write(tmp.path().join("rendezvous.pub"), b"not hex at all").expect("write");
    std::fs::write(tmp.path().join("pairing.salt"), b"").expect("write");
    assert_eq!(read_dir(tmp.path()).expect("readable"), plain);
    std::fs::remove_file(tmp.path().join("pairing.salt")).expect("rm");
    assert_eq!(read_dir(tmp.path()).expect("half of them, too"), plain);
}
