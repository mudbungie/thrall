//! **One child, from the fork to the capture**: the spawn, the poll that is
//! also the drain, and the cascade that stops a tool which will not stop
//! itself (DESIGN §3.5).
//!
//! **The deadline bounds the capture, not the child's exit** (bl-6c14). Those
//! are different things the moment a tool backgrounds anything: the helper
//! holds the same pipe write ends, so a drain that waited for the pipe to end
//! waited on a process this box never started, and the serial loop behind it
//! waited too — forever, on a tool that had already finished. The loop here
//! reads what is there rather than waiting for the end (`exec::pipes`), so
//! every path out of this file is inside the deadline plus the cascade's own
//! grace, and the bytes the tool produced come back either way.

use std::process::Stdio;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::pipes::Pipes;
use super::{SIGNALLED, TIMED_OUT, captured};
use crate::config::Local;
use crate::invocation::Capture;

/// How long a child gets between being asked to stop and being stopped. Long
/// enough for a tool to flush a file or drop a lock, short enough that the
/// engine's own patience is not spent on a corpse.
const GRACE: Duration = Duration::from_secs(2);

/// How often a running child is looked at when nothing is moving. A latency
/// knob on the *answer*, not on the run: a tick that read something reads again
/// at once, so a loud tool is drained at the pipe's speed and never at this
/// one's.
const POLL: Duration = Duration::from_millis(20);

/// The spawn and the drain. The `Err` is a fork that never happened — an argv
/// with no program in it, a missing binary, an unusable working directory —
/// which is the one outcome that is not the child's own verdict. `subject`
/// is the invocation's own working directory when the entry consents
/// (`exec::subject_cwd`); it outranks the entry's `cwd`, which stands for every
/// cwd-less invocation exactly as before.
pub(super) fn run(
    local: &Local,
    subject: Option<&std::path::Path>,
    input: &Value,
    deadline: Duration,
) -> Result<Capture, String> {
    let (head, args) = local
        .command
        .split_first()
        .ok_or("the command is an empty argv")?;
    let mut cmd = crate::spawn::command(head);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = subject.or(local.cwd.as_deref()) {
        cmd.current_dir(cwd);
    }
    let mut child = crate::spawn::spawn(&mut cmd).map_err(|e| format!("{head}: {e}"))?;

    let mut pipes = Pipes::new(&mut child, input.to_string().into_bytes());
    let (exit_code, note) = waited(&mut child, &mut pipes, deadline);
    pipes.settle(Instant::now() + GRACE);
    // The tool's own stderr first, then this box's remarks about the run —
    // what would not fit, and what would not stop.
    let mut drained = pipes.take();
    drained.err.extend_from_slice(drained.note.as_bytes());
    drained.err.extend_from_slice(note.as_bytes());
    Ok(captured(&drained.out, &drained.err, exit_code))
}

/// Wait for the child, draining it as it goes, or stop it. The second half of
/// the answer is what to say about the stopping — empty when the child answered
/// on its own, because a tool that finished has nothing this foot needs to add.
///
/// **The pump is inside the wait and not beside it.** A tool that writes more
/// than a pipe holds blocks until somebody reads, so the thing that watches the
/// clock has to be the thing that reads; and a tick that moved bytes goes round
/// again without sleeping, so the poll interval bounds how late an *answer* is
/// and never how fast output comes back.
fn waited(child: &mut std::process::Child, pipes: &mut Pipes, deadline: Duration) -> (i32, String) {
    let started = Instant::now();
    loop {
        let moved = pipes.pump();
        if let Ok(Some(ended)) = child.try_wait() {
            return (ended.code().unwrap_or(SIGNALLED), String::new());
        }
        if started.elapsed() >= deadline {
            return (TIMED_OUT, stopped(child, pipes, deadline));
        }
        if !moved {
            std::thread::sleep(POLL);
        }
    }
}

/// **The cascade**: ask, wait, insist — and its subject is the child's process
/// GROUP, so a tool that started something does not leave that something behind
/// (bl-a78e). A tool that traps `SIGTERM` to flush a file gets the grace; one
/// that ignores it does not get to outlive its deadline. The sentence says both
/// what happened and how long it was given, so an operator reading a transcript
/// can tell a slow tool from a stuck one.
///
/// **The grace is the child's, and only the child's.** The wait polls the tool
/// this box was asked to run; a helper of its that ignores the ask cannot extend
/// a deadline the invocation has already overrun. The insist is the group's
/// either way — a leader with good manners is not a reason to leave its
/// stragglers running, which is the whole gap this closes. The pipes are pumped
/// across the grace for the same reason they are pumped across the wait: a tool
/// flushing a file on its way out has somewhere to write.
///
/// The final wait is unbounded, and what bounds it is the invariant that the
/// child is a member of the group just killed. That is the spawn boundary's to
/// hold, not this function's to re-check: a `Child::kill` here would be a second
/// mechanism for a case the boundary excludes, and one whose effect no test
/// could ever reach.
fn stopped(child: &mut std::process::Child, pipes: &mut Pipes, deadline: Duration) -> String {
    // The child leads its own group (`spawn::command`), so its id is the
    // group's — read here, before the wait below can reap it away.
    let group = child.id();
    crate::sys::terminate_group(group);
    let asked = Instant::now();
    let mut answered = false;
    while asked.elapsed() < GRACE {
        pipes.pump();
        if let Ok(Some(_)) = child.try_wait() {
            answered = true;
            break;
        }
        std::thread::sleep(POLL);
    }
    crate::sys::kill_group(group);
    let _ = child.wait();
    if answered {
        return format!("\nthrall: no answer within {deadline:?}; the group was terminated\n");
    }
    format!(
        "\nthrall: no answer within {deadline:?}, and none to the signal; the group was killed\n"
    )
}
