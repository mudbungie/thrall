//! The two files, their three states, and the derivations both ends must
//! agree on — held to the bytes the ENGINE's own code derives.

use super::*;
use crate::test_support::Scratch;

fn pairing() -> Pairing {
    Pairing {
        key: [1u8; 32],
        salt: [2u8; 32],
    }
}

fn write(dir: &Path, name: &str, text: &str) {
    std::fs::write(dir.join(name), text).expect("write");
}

/// **The engine's bytes, verbatim** (yog `src/wire/rendezvous/material.rs`
/// over `salt = [2; 32]`). A label that drifted by one character would derive
/// a salt nobody files under and a key nothing opens with, and this is where
/// that would show.
#[test]
fn the_four_derivations_are_the_engines_byte_for_byte() {
    let p = pairing();
    assert_eq!(
        hex(&p.presence_salt()),
        "832100dd41d9219596850cbb12bb86caabe0d7ebcac788f3dc829b7a2093e55e"
    );
    assert_eq!(
        hex(&p.inbox_salt()),
        "5fc17259eeb89d3f93aa22916cecff7a171da3f998f836fb2aaa993b83654b38"
    );
    assert_eq!(
        hex(&p.seal_key()),
        "c3481ff89e54257cc554538285910e3ab328eccd206cad9a2ad0ed7ddf8b748d"
    );
    assert_eq!(
        hex(&p.inbox_keypair().expect("keypair").public()),
        "9571b09392c75e738b1829605d29401eaa38adfb63071c7e53259217c7564e58"
    );
}

#[test]
fn another_salt_derives_other_facts() {
    let other = Pairing {
        key: [1u8; 32],
        salt: [3u8; 32],
    };
    assert_ne!(other.seal_key(), pairing().seal_key());
    assert_ne!(other.presence_salt(), pairing().presence_salt());
}

#[test]
fn nothing_carried_is_none_and_both_files_are_a_pairing() {
    let scratch = Scratch::new();
    assert_eq!(read_dir(scratch.path()).expect("read"), None);
    write(scratch.path(), KEY, &format!("{}\n", hex(&[1u8; 32])));
    write(scratch.path(), SALT, &hex(&[2u8; 32]));
    assert_eq!(read_dir(scratch.path()).expect("read"), Some(pairing()));
}

#[test]
fn half_the_material_is_a_refusal_naming_both_files() {
    let scratch = Scratch::new();
    write(scratch.path(), SALT, &hex(&[2u8; 32]));
    let refusal = read_dir(scratch.path()).expect_err("half");
    assert!(refusal.contains(KEY) && refusal.contains(SALT), "{refusal}");
    assert!(
        refusal.contains("half-provisioned for rendezvous"),
        "{refusal}"
    );
}

#[test]
fn a_file_that_is_not_32_bytes_of_hex_is_a_refusal() {
    let scratch = Scratch::new();
    write(scratch.path(), KEY, "zz\n");
    write(scratch.path(), SALT, &hex(&[2u8; 32]));
    let refusal = read_dir(scratch.path()).expect_err("not hex");
    assert!(refusal.contains("does not read"), "{refusal}");
    write(scratch.path(), KEY, &hex(&[1u8; 31]));
    assert!(read_dir(scratch.path()).is_err(), "31 bytes");
}

#[test]
fn hex_round_trips_and_refuses_what_is_not_hex() {
    assert_eq!(hex(&[0, 15, 255]), "000fff");
    assert_eq!(unhex("000fff"), Some(vec![0, 15, 255]));
    assert_eq!(unhex("0"), None, "odd length");
    assert_eq!(unhex("0g"), None, "not a digit");
    assert_eq!(unhex("é0"), None, "not even ASCII");
}
