//! **Running one invocation** (yog's `docs/REMOTE.md` §5.2, §5.3; DESIGN §3.4,
//! §3.5): the far end of the routing leg, where a tool actually happens.
//!
//! **It is a local tool contract, unchanged**: the `tool_use.input` JSON on
//! stdin, bytes on stdout, the exit code the verdict. So a foot's tool
//! executable is the same kind of program the engine's own pool tools are, and
//! the capture that comes back is the same three facts. `command` is an argv,
//! spawned directly — no shell, and no interpolation of the invocation's input
//! into it, because a shell would make the declared `input_schema` advisory and
//! turn an operator's config file into a command-injection surface for anything
//! a model can type.
//!
//! **Every outcome is a capture.** A tool that ran, a tool that overran, a name
//! this box does not carry and a command that could not be spawned at all are
//! four exit codes and four sentences, never four kinds of failure — because an
//! invocation that earned no answer would be the hang the whole leg exists to
//! exclude, and the model at the far end reads a refusal exactly as it reads a
//! tool that failed, which is what it is.
//!
//! **Bytes become text here, once.** A capture ends as a model's tool result
//! and a model's message is text, so the transcode happens at the one place
//! bytes stop being bytes and nothing downstream carries an encoding case. A
//! tool whose output is not UTF-8 loses exactly the bytes no `String` can name.
//!
//! **Two deadlines, measuring different things** (REMOTE §5.3). This box's own
//! bound terminates the child and answers with a sentence; the asking side's
//! longer patience stands behind it for the case where this whole process went
//! away. Neither is a knob: an engine that has not answered is down, and a tool
//! that has not answered is working.
//!
//! **Containment honesty** (DESIGN §3.5): what thrall enforces locally is the
//! operator's config — which names may run at all, with which argv and in which
//! directory — plus this deadline. It is not a sandbox and does not pretend to
//! be one. The child runs as this process's user, with this process's
//! environment less the scrub the spawn boundary performs. **The deadline
//! reaches what the child started** (bl-a78e): the child leads a process group
//! of its own and the cascade signals that group, so a tool that backgrounds a
//! helper and then hangs does not leave the helper behind. What it does not
//! reach is a descendant that left the group under its own hand — `setsid` and
//! `setpgid` are the child's to call, and a process outside the group is
//! outside the signal. Whoever administers the box is still the party that can
//! contain a foot, and the design must not imply otherwise.

use std::io::{Read, Write};
use std::process::{Child, Stdio};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::config::{self, Local};
use crate::invocation::{Capture, Invocation};

/// **This box's own bound on one tool.** It is not a knob: a tool that has not
/// answered is working, and the asking side's longer patience is what covers
/// the case where this whole process went away.
pub const DEADLINE: Duration = Duration::from_mins(2);

/// How long a child gets between being asked to stop and being stopped. Long
/// enough for a tool to flush a file or drop a lock, short enough that the
/// engine's own patience is not spent on a corpse.
const GRACE: Duration = Duration::from_secs(2);

/// How often a running child is looked at. A latency knob on the *answer*, not
/// on the run: the child streams into its pipes regardless.
const POLL: Duration = Duration::from_millis(20);

/// The verdict a child that outran its deadline earns — the shell's own
/// convention for `timeout`, so an operator reading a transcript recognizes it.
pub const TIMED_OUT: i32 = 124;

/// The verdict a name this box cannot run earns — the shell's own convention
/// for "command not found", and REMOTE §5's *"a client refuses a tool it no
/// longer carries"* answered at the end that actually knows.
pub const NO_SUCH_TOOL: i32 = 127;

/// What a child that a signal ended, and that this foot did not signal, earns.
///
/// The shell's convention is `128 + N`, and `N` is not readable without a
/// platform extension this crate does not take — so this says *a signal ended
/// it* and does not pretend to say which. It is the honest half of a
/// convention rather than a guess at the other half.
pub const SIGNALLED: i32 = 128;

/// **The hand-off the loop takes** (`run::Handoff`): run one invocation against
/// this box's set, under this box's own deadline.
pub fn handoff(set: &[Local], invocation: &Invocation) -> Capture {
    execute(set, invocation, DEADLINE)
}

/// [`handoff`] with the deadline named, so the suite can watch the cascade
/// without waiting two minutes for it.
pub fn execute(set: &[Local], invocation: &Invocation, deadline: Duration) -> Capture {
    let Some(local) = config::position(set, &invocation.tool).and_then(|at| set.get(at)) else {
        return refused(
            NO_SUCH_TOOL,
            &format!(
                "this machine does not carry a tool called {:?}",
                invocation.tool
            ),
        );
    };
    match run(local, &invocation.input, deadline) {
        Ok(capture) => capture,
        Err(reason) => refused(NO_SUCH_TOOL, &reason),
    }
}

/// The spawn and the drain. The `Err` is a fork that never happened — an argv
/// with no program in it, a missing binary, an unusable working directory —
/// which is the one outcome that is not the child's own verdict.
fn run(local: &Local, input: &Value, deadline: Duration) -> Result<Capture, String> {
    let (head, args) = local
        .command
        .split_first()
        .ok_or("the command is an empty argv")?;
    let mut cmd = crate::spawn::command(head);
    cmd.args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = local.cwd.as_deref() {
        cmd.current_dir(cwd);
    }
    let mut child = crate::spawn::spawn(&mut cmd).map_err(|e| format!("{head}: {e}"))?;

    // The input goes down its own thread and the pipes are drained on theirs,
    // so no party can wedge another: a tool that never reads its input, and one
    // that writes more than a pipe buffer holds, are both ordinary here.
    let payload = input.to_string();
    let feeding = child.stdin.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let _ = pipe.write_all(payload.as_bytes());
        })
    });
    let out = child.stdout.take().map(reading);
    let err = child.stderr.take().map(reading);

    let (exit_code, note) = waited(&mut child, deadline);
    if let Some(feeding) = feeding {
        let _ = feeding.join();
    }
    let mut stderr = out_of(err);
    stderr.extend_from_slice(note.as_bytes());
    Ok(captured(&out_of(out), &stderr, exit_code))
}

/// Wait for the child, or stop it. The second half of the answer is what to say
/// about the stopping — empty when the child answered on its own, because a
/// tool that finished has nothing this foot needs to add.
fn waited(child: &mut Child, deadline: Duration) -> (i32, String) {
    let started = Instant::now();
    loop {
        if let Ok(Some(ended)) = child.try_wait() {
            return (ended.code().unwrap_or(SIGNALLED), String::new());
        }
        if started.elapsed() >= deadline {
            return (TIMED_OUT, stopped(child, deadline));
        }
        std::thread::sleep(POLL);
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
/// stragglers running, which is the whole gap this closes.
///
/// The final wait is unbounded, and what bounds it is the invariant that the
/// child is a member of the group just killed. That is the spawn boundary's to
/// hold, not this function's to re-check: a `Child::kill` here would be a second
/// mechanism for a case the boundary excludes, and one whose effect no test
/// could ever reach.
fn stopped(child: &mut Child, deadline: Duration) -> String {
    // The child leads its own group (`spawn::command`), so its id is the
    // group's — read here, before the wait below can reap it away.
    let group = child.id();
    crate::sys::terminate_group(group);
    let asked = Instant::now();
    let mut answered = false;
    while asked.elapsed() < GRACE {
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

/// Drain one pipe on its own thread.
fn reading<R: Read + Send + 'static>(mut pipe: R) -> JoinHandle<Vec<u8>> {
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        bytes
    })
}

/// What one drained pipe held.
fn out_of(reader: Option<JoinHandle<Vec<u8>>>) -> Vec<u8> {
    reader
        .and_then(|reader| reader.join().ok())
        .unwrap_or_default()
}

/// The three facts, transcoded once.
fn captured(out: &[u8], err: &[u8], exit_code: i32) -> Capture {
    Capture {
        stdout: String::from_utf8_lossy(out).into_owned(),
        stderr: String::from_utf8_lossy(err).into_owned(),
        exit_code,
    }
}

/// A refusal this box makes about itself, in the same three facts — so the far
/// end reads it exactly as it reads a tool that failed, which is what it is.
fn refused(exit_code: i32, reason: &str) -> Capture {
    Capture {
        stdout: String::new(),
        stderr: format!("thrall: {reason}\n"),
        exit_code,
    }
}

#[cfg(test)]
mod tests;
