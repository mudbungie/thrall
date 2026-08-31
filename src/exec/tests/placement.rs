//! **Where a tool runs, and who said so.** The operator's own `cwd` on the
//! entry (bl-3c93), and the worktree lane's — an invocation that carries the
//! subject's working directory, under the per-tool consent that gates it
//! (REMOTE §5.4, bl-36f7).

use super::{DEADLINE, NO_SUCH_TOOL, call, execute, tool};
use crate::test_support::Scratch;
use serde_json::json;

/// The operator's `cwd` is where the tool runs, and it is the only thing about
/// the child's placement that thrall decides.
#[test]
fn a_tool_runs_where_the_operator_said_it_would() {
    let scratch = Scratch::new();
    let mut local = tool("Where", "pwd");
    local.cwd = Some(scratch.path().to_path_buf());
    let got = execute(&[local], &call("Where", json!({})), DEADLINE);
    assert_eq!(got.exit_code, 0, "{got:?}");
    assert!(
        got.stdout.trim().ends_with(
            scratch
                .path()
                .file_name()
                .expect("a name")
                .to_string_lossy()
                .as_ref()
        ),
        "{got:?}"
    );
}
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

/// **A working directory that is not one is reported as what it is**
/// (bl-3c93). A fork fails for the program, for the argv, or for the directory,
/// and the operating system spells all three the same way — so a sentence built
/// from the program alone sends an operator to a file that is exactly right.
#[test]
fn a_bad_entry_working_directory_is_not_reported_as_a_missing_command() {
    let scratch = Scratch::new();
    let absent = scratch.path().join("absent");
    let mut local = tool("Where", "pwd");
    local.cwd = Some(absent.clone());
    let got = execute(&[local], &call("Where", json!({})), DEADLINE);
    assert_eq!(got.exit_code, NO_SUCH_TOOL);
    assert!(
        got.stderr.contains(&format!(
            "working directory {:?} is not a directory",
            absent.display().to_string()
        )),
        "{}",
        got.stderr
    );
    assert!(
        got.stderr.contains("/bin/sh was never reached"),
        "the command is named as unreached, never as missing: {}",
        got.stderr
    );
}

/// **A relative directory the invocation names is refused too**, and for the
/// reason the operator's own is refused at the read: resolved against nothing
/// anybody chose, it runs the tool wherever this process happened to start.
#[test]
fn a_relative_working_directory_on_the_invocation_is_refused() {
    let mut consenting = tool("Pwd", "pwd");
    consenting.tool.subject_cwd = true;
    let mut asked = call("Pwd", json!({}));
    asked.cwd = Some("relative/dir".to_owned());
    let got = execute(&[consenting], &asked, DEADLINE);
    assert_eq!(got.exit_code, NO_SUCH_TOOL);
    assert!(
        got.stderr.contains("is not an absolute path"),
        "{}",
        got.stderr
    );
}
