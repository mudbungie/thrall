//! **What the build reads, and whether it ships** (split from
//! `packaged_tests.rs` by bl-bb7d).
//!
//! The `include` allowlist fails closed, and this file is the one bill that
//! posture sends: a compile-time embed of a file the list does not name builds
//! here — the tree has the file — and fails to build for everyone who
//! downloaded the crate, where it does not. The failure is invisible on the
//! author's box by construction, which is exactly the class a test has to
//! carry.
//!
//! **It is a resolution and no longer a prohibition.** The first version
//! asserted that `src` contained no embed at all, which was true when it was
//! written and stopped being true the moment one was wanted:
//! [`config::EXAMPLE`](crate::config::EXAMPLE) embeds
//! `docs/tools.example.json` so a registry install can print its own example
//! document (bl-bb7d). So the sweep reads each embed's path, resolves it
//! against the file that names it, and holds it to both halves of the policy —
//! the classes [`is_ruled_in`](super::is_ruled_in) admits, and the real
//! `cargo package --list`. The two are asked separately on purpose: the first
//! says the class was thought about, the second says the manifest actually
//! carries the entry, and a green on one beside a red on the other is the
//! honest description of the two ways this breaks.
//!
//! **Both directions**, like every guard here: the sweep must also be shown to
//! SEE an embed, since `src` proving it is exactly what this crate stopped
//! being able to do. This file is the sweep; `embeds/tests.rs` beside it is
//! what asks it both questions.

use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

/// The two macro spellings that read a file at compile time, **assembled rather
/// than written**: no pattern may match its own text, and a sweep that flags the
/// file doing the sweeping is a sweep that can never be green. It is the same
/// discipline `scripts/leak-rules.sh` holds for the disclosure rules, and the
/// reason this file names neither macro literally anywhere.
fn embed_spellings() -> [String; 2] {
    ["bytes", "str"].map(|kind| format!("include_{kind}!"))
}

/// The path literal of every embed in one file's text.
///
/// Split rather than sliced: the text after a spelling begins at the open
/// parenthesis, so what lies between its first and second quote is the literal,
/// and `split` reaches it without an index this crate's lints refuse.
fn literals(text: &str) -> Vec<String> {
    let mut found = Vec::new();
    for spelling in embed_spellings() {
        for after in text.split(&spelling).skip(1) {
            if let Some(literal) = after.split('"').nth(1) {
                found.push(literal.to_owned());
            }
        }
    }
    found
}

/// `a/b/../c` as `a/c`, by folding components rather than by asking the
/// filesystem: a test that canonicalized would only answer for paths that
/// already exist, which is the one case that never fails.
fn normalized(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// Every embed under `dir`, as (the file that names it, the file it reads),
/// both stated relative to `root`.
///
/// An embed that resolves OUTSIDE `root` keeps its absolute path rather than
/// being dropped: it is ruled in by no class and packaged by no list, so it
/// reddens — which is the answer that case deserves.
fn embeds_under(root: &Path, dir: &Path) -> BTreeSet<(PathBuf, PathBuf)> {
    let mut found = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(embeds_under(root, &path));
            continue;
        }
        let (Ok(text), Some(parent)) = (std::fs::read_to_string(&path), path.parent()) else {
            continue;
        };
        for literal in literals(&text) {
            let read = normalized(&parent.join(literal));
            found.insert((
                path.strip_prefix(root).unwrap_or(&path).to_owned(),
                read.strip_prefix(root).unwrap_or(&read).to_owned(),
            ));
        }
    }
    found
}

/// The embeds this crate actually has, as strings, in one order.
fn embedded() -> Vec<(String, String)> {
    let root = super::root();
    embeds_under(&root, &root.join("src"))
        .into_iter()
        .map(|(by, read)| (by.display().to_string(), read.display().to_string()))
        .collect()
}

#[cfg(test)]
mod tests;
