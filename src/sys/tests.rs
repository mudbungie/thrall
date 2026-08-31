//! The one raw effect, and the sign that would make it something else.

use super::{kill_group, terminate_group};
use crate::spawn::{command, spawn};
use std::process::Stdio;

/// A real child stops when its group is asked to, and the ask is answered.
///
/// It is also the assertion that the spawn boundary put the child at the head
/// of a group of its own: `kill(-pid, …)` names a group, so a child that was
/// still in this process's group would leave nothing of that id to signal.
#[test]
fn a_running_child_leads_a_group_that_is_asked_to_stop() {
    let mut cmd = command("/bin/sh");
    cmd.args(["-c", "sleep 30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = spawn(&mut cmd).expect("sh runs");
    assert!(terminate_group(child.id()), "the signal was sent");
    let ended = child.wait().expect("waited");
    assert!(
        ended.code().is_none(),
        "it did not exit on its own: {ended:?}"
    );
}

/// The insist half reaches the same group, and a tool that ignores the ask does
/// not get to ignore this.
#[test]
fn a_group_that_had_its_chance_is_killed() {
    let mut cmd = command("/bin/sh");
    cmd.args(["-c", "trap '' TERM; sleep 30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = spawn(&mut cmd).expect("sh runs");
    assert!(terminate_group(child.id()), "the ask was sent");
    assert!(kill_group(child.id()), "the insist was sent");
    let ended = child.wait().expect("waited");
    assert!(
        ended.code().is_none(),
        "it did not exit on its own: {ended:?}"
    );
}

/// **Zero is not a group id, it is *this* group**, and negated it would name the
/// process doing the asking. It is refused here rather than passed to a syscall
/// that would read it as "everything I am part of". Nothing about a foot's
/// deadline is worth that.
#[test]
fn a_non_positive_group_is_refused_rather_than_widened() {
    assert!(!terminate_group(0));
}

/// A group that is not there cannot be asked to stop, and that is an answer
/// rather than an error: the caller's next move is the same either way.
#[test]
fn a_group_that_is_not_there_answers_false() {
    // Above every platform's process-id ceiling, so it names nothing.
    let ceiling = u32::try_from(i32::MAX).expect("fits");
    assert!(!terminate_group(ceiling));
    assert!(!kill_group(ceiling));
}
