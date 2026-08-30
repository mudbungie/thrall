//! Running a tool for real: what comes back, what does not, and what happens
//! to a child that will not stop.

use super::{DEADLINE, NO_SUCH_TOOL, SIGNALLED, TIMED_OUT, execute, handoff};
use crate::config::Local;
use crate::invocation::{Capture, Invocation};
use crate::test_support::Scratch;
use crate::tools::Tool;
use serde_json::{Value, json};
use std::time::Duration;

/// One tool this box offers, running `script` under `/bin/sh`.
fn tool(name: &str, script: &str) -> Local {
    Local {
        tool: Tool {
            name: name.to_owned(),
            description: "a tool".to_owned(),
            input_schema: json!({"type": "object"}),
        },
        command: vec!["/bin/sh".to_owned(), "-c".to_owned(), script.to_owned()],
        cwd: None,
    }
}

/// A call on `name`, carrying `input`.
fn call(name: &str, input: Value) -> Invocation {
    Invocation {
        id: "i-1".to_owned(),
        tool: name.to_owned(),
        input,
    }
}

/// A deadline short enough that the suite can watch the cascade.
fn brief() -> Duration {
    Duration::from_millis(150)
}

/// **The tool contract, one for one**: the invocation's input JSON on stdin,
/// bytes on stdout, the exit code the verdict.
#[test]
fn the_input_reaches_the_command_on_stdin_and_its_output_comes_back() {
    let set = [tool("Echo", "cat")];
    let input = json!({"command": "echo hi", "n": 7});
    let got = execute(&set, &call("Echo", input.clone()), DEADLINE);
    assert_eq!(got.stdout, input.to_string());
    assert_eq!(got.stderr, "");
    assert_eq!(got.exit_code, 0);
}

/// The three facts are all carried, including a verdict that is not success.
#[test]
fn stderr_and_a_failing_exit_code_are_carried_as_they_are() {
    let set = [tool("Fails", "printf out; printf err >&2; exit 3")];
    let got = execute(&set, &call("Fails", json!({})), DEADLINE);
    assert_eq!(
        got,
        Capture {
            stdout: "out".to_owned(),
            stderr: "err".to_owned(),
            exit_code: 3,
        }
    );
}

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

/// **Bytes become text here, once.** A tool whose output is not UTF-8 loses
/// exactly the bytes no string can name, and nothing downstream carries an
/// encoding case.
#[test]
fn output_that_is_not_utf8_is_transcoded_once_and_lossily() {
    // Octal, because that is the escape POSIX `printf` actually has.
    let set = [tool("Binary", "printf '\\377\\376'")];
    let got = execute(&set, &call("Binary", json!({})), DEADLINE);
    assert_eq!(got.stdout, "\u{fffd}\u{fffd}");
    assert_eq!(got.exit_code, 0);
}

/// **A tool absent from this box's set is refused in band**, which is the
/// staleness correction the far end relies on for a definition it froze at load
/// time — and it is a capture like any other, so the model reads it as a tool
/// that failed.
#[test]
fn a_name_this_box_does_not_carry_is_refused_as_a_capture() {
    let set = [tool("Echo", "cat")];
    let got = execute(&set, &call("Write", json!({})), DEADLINE);
    assert_eq!(got.exit_code, NO_SUCH_TOOL);
    assert!(got.stdout.is_empty());
    assert!(
        got.stderr
            .contains("does not carry a tool called \"Write\""),
        "{got:?}"
    );
}

/// A command that cannot be started at all is the one outcome that is not the
/// child's own verdict, and it still comes back as a capture naming the program.
#[test]
fn a_command_that_cannot_be_started_is_still_a_capture() {
    let mut local = tool("Missing", "");
    local.command = vec!["/nonexistent/thrall-test-tool".to_owned()];
    let got = execute(&[local], &call("Missing", json!({})), DEADLINE);
    assert_eq!(got.exit_code, NO_SUCH_TOOL);
    assert!(
        got.stderr.contains("/nonexistent/thrall-test-tool"),
        "{got:?}"
    );
}

/// An argv with no program in it names nothing to run. The config refuses one
/// at the read, so this is the executor holding the same line for a set built
/// any other way.
#[test]
fn an_argv_with_no_program_in_it_names_nothing_to_run() {
    let mut local = tool("Empty", "");
    local.command = Vec::new();
    let got = execute(&[local], &call("Empty", json!({})), DEADLINE);
    assert_eq!(got.exit_code, NO_SUCH_TOOL);
    assert!(got.stderr.contains("empty argv"), "{got:?}");
}

/// **The deadline terminates the child and the capture says so**, in the
/// shell's own `timeout` verdict so an operator reading a transcript recognizes
/// it. A tool that stops when asked is asked, and nothing more.
#[test]
fn a_tool_that_outruns_its_deadline_is_terminated_and_says_so() {
    let set = [tool("Slow", "sleep 30")];
    let got = execute(&set, &call("Slow", json!({})), brief());
    assert_eq!(got.exit_code, TIMED_OUT);
    assert!(got.stderr.contains("no answer within"), "{got:?}");
    assert!(got.stderr.contains("terminated"), "{got:?}");
    assert!(
        !got.stderr.contains("killed"),
        "it stopped when asked: {got:?}"
    );
}

/// **A tool that ignores the ask does not get to outlive its deadline.** The
/// grace is spent, then it is killed, and the sentence says which happened —
/// so an operator can tell a slow tool from a stuck one.
#[test]
fn a_tool_that_ignores_the_signal_is_killed_after_the_grace() {
    let set = [tool("Stubborn", "trap '' TERM; sleep 30")];
    let got = execute(&set, &call("Stubborn", json!({})), brief());
    assert_eq!(got.exit_code, TIMED_OUT);
    assert!(got.stderr.contains("none to the signal"), "{got:?}");
    assert!(got.stderr.contains("killed"), "{got:?}");
}

/// A child a signal ended, and that this foot did not signal, says *a signal
/// ended it* — and does not pretend to say which, because the number is not
/// readable without a platform extension this crate does not take.
#[test]
fn a_child_a_signal_ended_says_a_signal_ended_it() {
    let set = [tool("Dies", "kill -9 $$")];
    let got = execute(&set, &call("Dies", json!({})), DEADLINE);
    assert_eq!(got.exit_code, SIGNALLED);
}

/// A tool that never reads its input cannot wedge the foot: the input goes down
/// its own thread, so nothing here waits on a pipe the child has abandoned.
#[test]
fn a_tool_that_ignores_its_input_still_answers() {
    let set = [tool("Deaf", "printf done")];
    let big = json!({"payload": "x".repeat(200_000)});
    let got = execute(&set, &call("Deaf", big), DEADLINE);
    assert_eq!(got.stdout, "done");
    assert_eq!(got.exit_code, 0);
}

/// A tool that writes more than a pipe buffer holds cannot wedge it either:
/// both pipes are drained while the child runs.
#[test]
fn a_tool_that_writes_more_than_a_pipe_holds_still_answers() {
    let set = [tool("Loud", "yes hello | head -c 300000")];
    let got = execute(&set, &call("Loud", json!({})), DEADLINE);
    assert_eq!(got.stdout.len(), 300_000);
    assert_eq!(got.exit_code, 0);
}

/// The hand-off the loop takes is this executor under this box's own bound, so
/// the loop and the suite are running the same code.
#[test]
fn the_handoff_is_the_executor_under_this_box_s_own_bound() {
    let set = [tool("Echo", "cat")];
    assert_eq!(
        handoff(&set, &call("Echo", json!({"a": 1}))),
        execute(&set, &call("Echo", json!({"a": 1})), DEADLINE)
    );
}
