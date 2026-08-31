//! The worktree lane on this box (REMOTE §5.4, bl-36f7): an invocation
//! that carries the subject's working directory, and the operator's
//! per-tool consent that gates it.

use super::{DEADLINE, call, execute, tool};
use crate::test_support::Scratch;
use serde_json::json;
/// **The worktree lane, granted** (REMOTE §5.4, bl-36f7): an invocation
/// carrying a working directory runs there when the operator's entry
/// consents — the subject's location outranks the entry's own `cwd`.
#[test]
fn a_consenting_entry_runs_at_the_invocations_working_directory() {
    let scratch = Scratch::new();
    let subject = scratch.path().join("worktree");
    std::fs::create_dir_all(&subject).expect("mkdir subject");
    let mut consenting = tool("Pwd", "pwd");
    consenting.tool.subject_cwd = true;
    consenting.cwd = Some(scratch.path().join("elsewhere"));
    let mut asked = call("Pwd", json!({}));
    asked.cwd = Some(subject.display().to_string());
    let got = execute(&[consenting], &asked, DEADLINE);
    assert_eq!(got.exit_code, 0, "{}", got.stderr);
    assert_eq!(
        got.stdout.trim_end(),
        std::fs::canonicalize(&subject)
            .expect("canonical subject")
            .display()
            .to_string()
    );
}

/// **Consent is the operator's, and its absence is a refusal that names the
/// remedy** — the box must opt in before it executes at a path a caller
/// names, and the sentence says which key, in which file, on which machine.
#[test]
fn without_consent_a_carried_working_directory_is_refused_naming_the_key() {
    let scratch = Scratch::new();
    let mut asked = call("Echo", json!({}));
    asked.cwd = Some(scratch.path().display().to_string());
    let got = execute(&[tool("Echo", "cat")], &asked, DEADLINE);
    assert_ne!(got.exit_code, 0);
    assert!(got.stderr.contains("does not consent"), "{}", got.stderr);
    assert!(
        got.stderr.contains("\"subject_cwd\": true"),
        "{}",
        got.stderr
    );
    assert!(got.stderr.contains("tools.json"), "{}", got.stderr);
}

/// A consenting box that does not actually hold the named directory refuses
/// in band rather than running the tool somewhere else — the far end named
/// this box as holding the worktree, and it does not.
#[test]
fn a_working_directory_this_box_does_not_hold_is_refused() {
    let scratch = Scratch::new();
    let mut consenting = tool("Pwd", "pwd");
    consenting.tool.subject_cwd = true;
    let mut asked = call("Pwd", json!({}));
    asked.cwd = Some(scratch.path().join("absent").display().to_string());
    let got = execute(&[consenting], &asked, DEADLINE);
    assert_ne!(got.exit_code, 0);
    assert!(
        got.stderr.contains("is not a directory on this box"),
        "{}",
        got.stderr
    );
}
