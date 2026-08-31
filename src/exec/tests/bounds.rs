//! **What bounds a capture.** Two things do, and neither is the child's own
//! exit: the deadline, which a pipe a stranger holds must not extend
//! (bl-6c14), and this box's own limit on how much output it will carry back
//! (bl-6028).

use super::super::CAPTURE_LIMIT;
use super::{DEADLINE, call, execute, tool};
use serde_json::json;
use std::time::Duration;

/// **A tool that backgrounds a helper and exits still earns its capture**
/// (bl-6c14). The helper inherits the child's stdout and stderr write ends, so
/// a drain that ends only when every writer has closed is a drain waiting on a
/// stranger — and the serial loop behind it waits with it, forever. The
/// deadline bounds the whole capture, not the child's own exit.
///
/// Both halves in one test, because a bounded drain that dropped the bytes
/// would satisfy the first half alone: the tool's own output is delivered, and
/// it is delivered without waiting on the helper.
#[test]
fn a_tool_that_leaves_a_helper_holding_the_pipes_still_answers() {
    let set = [tool("Daemon", "sleep 5 & printf started")];
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(execute(&set, &call("Daemon", json!({})), DEADLINE));
    });
    let got = rx
        .recv_timeout(Duration::from_secs(2))
        .expect("the capture, long before the helper lets the pipes go");
    assert_eq!(got.stdout, "started", "{got:?}");
    assert_eq!(got.exit_code, 0, "{got:?}");
}

/// **A capture too big to carry is truncated in band, and the capture says so**
/// (bl-6028). The bound is this box's own — a local fact about how much output
/// it will answer with — and it is stated per stream, in the same `thrall:`
/// sentence a deadline earns, so the far end reads a bounded answer rather than
/// waiting out a slot for a foot that died.
///
/// Both streams, because the sentence has to name which one overran: a note
/// that said only "output" would leave an operator with the wrong file open.
#[test]
fn output_past_this_box_s_limit_is_truncated_and_the_capture_names_the_drop() {
    let over = CAPTURE_LIMIT + 4096;
    let set = [tool(
        "Loud",
        &format!("yes hello | head -c {over}; yes oops | head -c {over} >&2"),
    )];
    let got = execute(&set, &call("Loud", json!({})), DEADLINE);
    assert_eq!(got.exit_code, 0, "{}", got.stderr);
    assert_eq!(got.stdout.len(), CAPTURE_LIMIT);
    assert!(got.stderr.starts_with("oops"), "the tool's own bytes first");
    assert!(
        got.stderr
            .contains(&format!("stdout exceeded this box's {CAPTURE_LIMIT}-byte")),
        "no note for stdout"
    );
    assert!(
        got.stderr
            .contains(&format!("stderr exceeded this box's {CAPTURE_LIMIT}-byte")),
        "no note for stderr"
    );
    assert_eq!(got.stderr.matches("4096 further bytes").count(), 2);
}
