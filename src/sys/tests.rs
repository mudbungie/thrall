//! The one raw effect, and the sign that would make it something else.

use super::terminate;
use crate::spawn::{command, spawn};
use std::process::Stdio;

/// A real child stops when it is asked to, and the ask is answered.
#[test]
fn a_running_child_is_asked_to_stop() {
    let mut cmd = command("/bin/sh");
    cmd.args(["-c", "sleep 30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = spawn(&mut cmd).expect("sh runs");
    assert!(terminate(child.id()), "the signal was sent");
    let ended = child.wait().expect("waited");
    assert!(
        ended.code().is_none(),
        "it did not exit on its own: {ended:?}"
    );
}

/// **Zero is a process group, never a process**, and it is refused here rather
/// than passed to a syscall that would read it as "every process in this
/// group". Nothing about a foot's deadline is worth that.
#[test]
fn a_non_positive_process_is_refused_rather_than_widened() {
    assert!(!terminate(0));
}

/// A process that is not there cannot be asked to stop, and that is an answer
/// rather than an error: the caller's next move is the same either way.
#[test]
fn a_process_that_is_not_there_answers_false() {
    // Above every platform's process-id ceiling, so it names nothing.
    let ceiling = u32::try_from(i32::MAX).expect("fits");
    assert!(!terminate(ceiling));
}
