//! **The two directions of the embed sweep** (bl-bb7d), and the resolution it
//! now performs: every compile-time embed under `src` must be a file the
//! published crate carries, and the sweep must be shown to find one it was not
//! told about.
//!
//! A sibling file rather than an inline module for the ordinary reason — the
//! 300-line cap counts inline tests — and because the assert carve-out
//! (`rules/no-assert-outside-tests.yml`) reaches a split file by its name.

use super::{embed_spellings, embedded, embeds_under, literals, normalized};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// **Every compile-time embed is a file the published crate carries.** Both
/// halves: the class is ruled in here, and the manifest's `include` really put
/// the path in the tarball.
#[test]
fn every_embed_ships_with_the_crate() {
    let packaged = crate::packaged_tests::packaged();
    for (by, read) in embedded() {
        assert!(
            crate::packaged_tests::is_ruled_in(&read),
            "{by} reads {read} at compile time, and no class here rules it into \
             the published crate. Add it to `include` in Cargo.toml and to \
             is_ruled_in, or the crate cannot compile for anyone who downloads it"
        );
        assert!(
            packaged.contains(&read),
            "{by} reads {read} at compile time and `cargo package --list` does \
             not carry it — the class is ruled in but `include` never named the \
             path, so this builds here and nowhere else"
        );
    }
}

/// The sweep must not have quietly stopped finding anything: the crate has one
/// embed and it is the example document, so a sweep answering with nothing is
/// broken rather than a clean tree.
#[test]
fn the_sweep_still_finds_the_one_embed_this_crate_has() {
    assert_eq!(
        embedded(),
        vec![(
            "src/config.rs".to_owned(),
            "docs/tools.example.json".to_owned()
        )],
        "the example document is the crate's one compile-time embed; a second \
         one is a deliberate act that belongs in this list"
    );
}

/// The sweep must be able to SEE an embed it was not told about, and to resolve
/// the path rather than merely notice a macro. Pointed at a scratch tree, since
/// `src` can only ever prove the embed above.
#[test]
fn the_embed_sweep_sees_and_resolves_a_planted_embed() {
    let scratch = crate::test_support::Scratch::new();
    let nested = scratch.path().join("nested");
    std::fs::create_dir(&nested).expect("a directory");
    let planted = nested.join("embeds.rs");
    let [bytes, _] = embed_spellings();
    std::fs::write(
        &planted,
        format!("const X: &[u8] = {bytes}(\"../x.bin\");\n"),
    )
    .expect("the plant is written");
    std::fs::write(nested.join("plain.rs"), "const X: u8 = 1;\n").expect("a plain file");
    assert_eq!(
        embeds_under(scratch.path(), scratch.path()),
        BTreeSet::from([(
            PathBuf::from("nested").join("embeds.rs"),
            PathBuf::from("x.bin")
        )])
    );
    // A directory that is not there is an empty answer, not a panic.
    assert!(embeds_under(scratch.path(), &scratch.path().join("absent")).is_empty());
}

/// And the reader beneath it: a file with no embed says nothing, one with two
/// says both, and a spelling with no literal after it is not an embed.
#[test]
fn the_literal_reader_reads_what_is_there() {
    let [bytes, text] = embed_spellings();
    assert!(literals("const X: u8 = 1;").is_empty());
    assert_eq!(
        literals(&format!("{bytes}(\"a.bin\") and {text}(\"b/c.json\")")),
        vec!["a.bin".to_owned(), "b/c.json".to_owned()]
    );
    assert!(literals(&bytes).is_empty());
}

/// `..` folds, `.` disappears, and a path with neither is itself.
#[test]
fn the_normaliser_folds_what_it_should() {
    assert_eq!(normalized(Path::new("a/b/../c")), PathBuf::from("a/c"));
    assert_eq!(normalized(Path::new("./a/./b")), PathBuf::from("a/b"));
    assert_eq!(normalized(Path::new("a/b")), PathBuf::from("a/b"));
    assert_eq!(normalized(Path::new("../a")), PathBuf::from("a"));
}
