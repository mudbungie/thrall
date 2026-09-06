//! **The shipped example, held to the reader that will read it** (bl-102b).
//!
//! `docs/tools.example.json` is the answer to a defect that was not a missing
//! feature: the file `thrall run` demands could not be written from the shipped
//! documentation at all. Its shape had to be assembled from three sources in
//! two repositories — this crate's DESIGN §3.4 in prose, yog's REMOTE §5.1 for
//! the only place the three advertised keys are ever *named*, and
//! `src/exec.rs`'s module comment for the stdin/stdout contract, which no
//! document stated. A reader who did not open the Rust would assume argv
//! interpolation, write an entry that receives nothing, and advertise a tool
//! that answers emptiness.
//!
//! **An example nothing reads is an example that rots**, which is the whole
//! reason this file exists rather than a paragraph. Three bindings, and each
//! closes one way the example could quietly stop being true:
//!
//! - it is parsed by [`config::read`](crate::config::read), the very function
//!   `thrall run` spends on the operator's own file, so an example that would
//!   be refused on a real box is refused here first;
//! - the README carries it **byte for byte**, so the block a reader copies and
//!   the file a reader downloads cannot drift into two documents;
//! - the README states the contract the example cannot show — stdin, stdout,
//!   exit code — because an operator who gets that wrong writes a file that
//!   parses, advertises and returns nothing.

use crate::config::{Local, TOOLS, read};
use std::path::{Path, PathBuf};

/// The repository root, from the manifest directory rather than from the
/// working directory, so the answer does not depend on who started the test.
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The example, as the bytes that ship.
fn example() -> String {
    std::fs::read_to_string(root().join("docs").join("tools.example.json"))
        .expect("the shipped example")
}

/// The README, as the bytes that ship.
fn readme() -> String {
    std::fs::read_to_string(root().join("README.md")).expect("the README")
}

/// **It is a document this box would accept**, read by the reader itself: the
/// name check, the schema, the absolute `cwd` and the empty-argv refusal all
/// bite here exactly as they would on a real box.
#[test]
fn the_shipped_example_is_a_document_this_foot_reads() {
    let set: Vec<Local> = read(Path::new(&root().join("docs").join("tools.example.json")))
        .expect("the example is a document thrall reads");
    assert_eq!(set.len(), 2, "the ball asked for two entries");
    assert!(!set[0].tool.subject_cwd, "the first entry is the plain one");
    assert!(
        set[1].tool.subject_cwd,
        "the second entry carries the worktree lane's consent"
    );
    assert!(
        set[1].cwd.is_some(),
        "and the local half beside it, so the two halves are both shown"
    );
}

/// **The README carries it byte for byte.** Two copies of one document drift
/// within a week, and the copy a reader trusts is the one they can see.
#[test]
fn the_readme_carries_the_example_verbatim() {
    assert!(
        readme().contains(example().trim_end()),
        "the README's `{TOOLS}` block is no longer the shipped example — one of \
         the two moved. docs/tools.example.json is the document; the README \
         block is its rendering, and they are one fact."
    );
}

/// **And it states the contract the example cannot show.** The keys are visible
/// in the file; how the input reaches the command is not, and that is the
/// sentence the difference between a working foot and a silent one hangs on.
#[test]
fn the_readme_states_the_stdin_stdout_contract() {
    let readme = readme();
    for said in [
        "standard input",
        "standard output",
        "exit code",
        "argv, spawned directly",
    ] {
        assert!(
            readme.contains(said),
            "the tool contract no longer says {said:?} — it is stated nowhere \
             else a reader will look, and a source comment is not a document"
        );
    }
}
