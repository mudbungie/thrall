//! The boundary's two obligations: a scrubbed environment, and one fork.

use super::{INHERITED, command, output};

/// The constructor removes every inherited git variable from the child's
/// environment — asserted on the builder rather than through a child, because
/// the fact under test is *what the boundary decided*, and reading it back out
/// of a subprocess would make the assertion depend on what this process
/// happened to be run with.
#[test]
fn the_constructor_removes_every_inherited_git_variable() {
    let cmd = command("/bin/sh");
    let decided: Vec<(String, bool)> = cmd
        .get_envs()
        .map(|(k, v)| (k.to_string_lossy().into_owned(), v.is_some()))
        .collect();
    for var in INHERITED {
        assert!(
            decided.contains(&((*var).to_owned(), false)),
            "{var} is not removed: {decided:?}"
        );
    }
    assert_eq!(
        decided.len(),
        INHERITED.len(),
        "the boundary decides the scrub and nothing else: {decided:?}"
    );
}

/// The fork answers what the child said and what it exited with — the three
/// facts every caller of this boundary needs, and the only ones it gets.
#[test]
fn the_fork_answers_the_child_s_own_verdict() {
    let mut cmd = command("/bin/sh");
    cmd.args(["-c", "printf out; printf err >&2; exit 3"]);
    let said = output(&mut cmd).expect("sh runs");
    assert_eq!(said.stdout, b"out");
    assert_eq!(said.stderr, b"err");
    assert_eq!(said.status.code(), Some(3));
}

/// A program that is not there fails the fork itself, and never arrives as a
/// child with a verdict — it is the one outcome that is not the child's answer.
#[test]
fn a_program_that_is_not_there_fails_the_fork() {
    let mut cmd = command("/nonexistent/thrall-test-binary");
    assert!(output(&mut cmd).is_err());
}
