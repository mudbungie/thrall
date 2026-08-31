//! **The publication guard** (bl-d25a, prep for bl-006e): what `cargo publish`
//! would upload is read off the real `cargo package --list`, and every path in
//! it must belong to a class that was ruled in.
//!
//! **Why a test and not a checklist line.** A publication is the one act in
//! this repo with no undo — a yanked version stays downloadable — so the check
//! has to fire before the act, on every gate run, at no one's discretion. And
//! an `include` allowlist without a test is a comment: nothing else notices
//! when a later edit widens it, and the notice would otherwise arrive after
//! the version is public. Upstream shipped the operator's home paths and three
//! transcripts exactly that way, because its manifest declared no list at all.
//!
//! **The classes below are a SECOND statement of the manifest's policy**, which
//! is deliberate. A check that derived its allowlist from the `include` key
//! would widen with it and stay green through the exact edit it exists to
//! catch.
//!
//! **It lives in `src` and not in a `tests/` crate**, unlike the upstream guard
//! it mirrors, because it forks a child: `cargo package --list` is a
//! subprocess, the spawn boundary is `pub(crate)` (DESIGN §4), and an
//! integration crate could only reach a bare `Command::new` — which the
//! confinement rules refuse, with no test carve-out. The boundary is also what
//! makes this safe to run under a commit hook: `git` exports `GIT_DIR` into its
//! children and it OUTRANKS a `current_dir`, so an unscrubbed `cargo` here
//! would read whichever repository invoked the suite.
//!
//! **Both directions, because a shape guard dies by matching nothing.**
//! [`the_list_is_not_vacuous`] fails a spawn that answered with a short list,
//! and [`the_allowlist_sees_its_own_violations`] fails an [`is_ruled_in`] that
//! has quietly become true of everything.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// The real answer to *"what would `cargo publish` upload?"*, one path per line.
///
/// `--offline` keeps the guard hermetic — the lockfile is committed and every
/// dependency is resolved long before a test binary runs. `--allow-dirty` is
/// required because `cargo package` refuses a worktree with uncommitted changes
/// outright, and a claim worktree mid-edit is the normal case for the author
/// this test is addressed to.
fn packaged() -> Vec<String> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let mut cmd = crate::spawn::command(&cargo);
    cmd.current_dir(root())
        .args(["package", "--list", "--offline", "--allow-dirty"]);
    let out = crate::spawn::output(&mut cmd).expect("cargo runs");
    assert!(
        out.status.success(),
        "cargo package --list did not answer: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The crate root, off this file's own location rather than the working
/// directory, so the answer does not depend on who started the test binary.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The classes ruled into the published crate: the crate's own Rust source, and
/// the two files crates.io renders. `Cargo.toml.orig` and `.cargo_vcs_info.json`
/// are minted by cargo into the tarball and are not tree files at all.
fn is_ruled_in(path: &str) -> bool {
    let named = matches!(
        path,
        "Cargo.toml"
            | "Cargo.lock"
            | "Cargo.toml.orig"
            | ".cargo_vcs_info.json"
            | "README.md"
            | "LICENSE"
    );
    named
        || path
            .strip_prefix("src/")
            .is_some_and(|rest| Path::new(rest).extension().is_some_and(|ext| ext == "rs"))
}

/// The defect: the design document, the gate apparatus, the agent guide and a
/// corpus of fabricated secrets shipping to a registry with the binary. Stated
/// as an allowlist so the next file class added to the tree is red here instead
/// of public there.
#[test]
fn no_commentary_or_apparatus_ships() {
    let strays: Vec<String> = packaged().into_iter().filter(|p| !is_ruled_in(p)).collect();
    assert!(
        strays.is_empty(),
        "paths `cargo publish` would upload that no class rules in. A yanked \
         version stays downloadable, so widen `include` in Cargo.toml only with \
         a reason, and add the class beside it here:\n{}",
        strays.join("\n")
    );
}

/// The other side of a fail-closed list: the crate must still be a crate.
#[test]
fn the_files_a_registry_needs_ship() {
    let list = packaged();
    for needed in [
        "Cargo.toml",
        "Cargo.lock",
        "README.md",
        "LICENSE",
        "src/lib.rs",
        "src/main.rs",
    ] {
        assert!(
            list.iter().any(|p| p == needed),
            "{needed} is not in the packaged list — `include` dropped a file the \
             registry or the build needs"
        );
    }
}

/// A guard that measured nothing must not read as a pass: a failed spawn or an
/// empty stdout would otherwise be an empty stray list, which is a green.
#[test]
fn the_list_is_not_vacuous() {
    let list = packaged();
    let sources = list.iter().filter(|p| p.starts_with("src/")).count();
    assert!(
        sources > 30,
        "the packaged list carries {sources} src paths over {} entries — the \
         spawn is broken, not the tree",
        list.len()
    );
}

/// **The fail-closed list's one cost, paid.** `include` names `.rs` files under
/// `src` and nothing else, so a compile-time embed of anything else would build
/// here and fail to build for everyone who downloaded the crate. The sweep is
/// over the tree rather than over a list, so it covers embeds that do not exist
/// yet — and today the right answer is that there are none.
#[test]
fn nothing_the_build_reads_is_left_out_of_the_package() {
    let embedding = sources_that_embed(&root().join("src"));
    assert!(
        embedding.is_empty(),
        "these files read a build input at compile time. Every such input must \
         be ruled into `include` in Cargo.toml and into is_ruled_in above, or \
         the published crate cannot compile:\n{}",
        embedding
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The two macro spellings that read a file at compile time, **assembled rather
/// than written**: no pattern may match its own text, and a sweep that flags the
/// file doing the sweeping is a sweep that can never be green. It is the same
/// discipline `scripts/leak-rules.sh` holds for the disclosure rules, and the
/// reason this file names neither macro literally anywhere.
fn embed_spellings() -> [String; 2] {
    ["bytes", "str"].map(|kind| format!("include_{kind}!"))
}

/// Every file under `dir` that names a compile-time embed, in one order.
fn sources_that_embed(dir: &Path) -> BTreeSet<PathBuf> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(sources_that_embed(&path));
        } else if std::fs::read_to_string(&path).is_ok_and(|text| {
            embed_spellings()
                .iter()
                .any(|spelling| text.contains(spelling))
        }) {
            found.insert(path);
        }
    }
    found
}

/// The negative direction for the restated policy: every excluded class, and
/// the unanchored-pattern trap the manifest's comment records, must be seen as
/// a violation — and the classes that ship must not.
#[test]
fn the_allowlist_sees_its_own_violations() {
    for stray in [
        "docs/DESIGN.md",
        "AGENTS.md",
        "CLAUDE.md",
        "Makefile",
        "Containerfile",
        "deny.toml",
        "clippy.toml",
        "tarpaulin.toml",
        "rules/no-bare-command.yml",
        "rules/fixtures/violations.rs",
        ".githooks/pre-commit",
        ".github/workflows/release-plz.yml",
        "scripts/leak-scan.sh",
        // the unanchored-pattern sighting: a bare `README.md` include pattern
        // ships this, and no `scripts` class rules it in
        "scripts/leak-fixtures/README.md",
        "scripts/leak-fixtures/private-key.txt",
    ] {
        assert!(!is_ruled_in(stray), "{stray} must not be ruled in");
    }
    for shipped in ["src/main.rs", "src/channel/tls.rs", "LICENSE"] {
        assert!(is_ruled_in(shipped), "{shipped} must be ruled in");
    }
}

/// The sweep must be able to SEE an embed, or its silence means nothing. It is
/// pointed at this crate's own scratch directory, since `src` can no longer
/// prove it.
#[test]
fn the_embed_sweep_sees_an_embed() {
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
        sources_that_embed(scratch.path()),
        BTreeSet::from([planted])
    );
    // A directory that is not there is an empty answer, not a panic.
    assert!(sources_that_embed(&scratch.path().join("absent")).is_empty());
}
