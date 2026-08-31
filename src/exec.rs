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
//!
//! **The deadline bounds the capture and not merely the child** (bl-6c14).
//! That distinction is the whole of `child` and `pipes` below: a pipe outlives
//! the process it was given to, so a helper the tool backgrounded used to hold
//! this end reading after the tool had exited — an invocation that earned no
//! answer, which is the one thing this file's second paragraph says cannot
//! happen.
//!
//! **This file is the dispatch.** Which entry, whose directory, and the three
//! facts that come back; running the child is `child`, and reading it without
//! blocking is `pipes`.

use std::time::Duration;

use crate::config::{self, Local};
use crate::invocation::{Capture, Invocation};

/// One child, from the fork to the capture.
mod child;
/// The child's three pipes, pumped without blocking.
mod pipes;

/// **This box's own bound on one tool.** It is not a knob: a tool that has not
/// answered is working, and the asking side's longer patience is what covers
/// the case where this whole process went away.
pub const DEADLINE: Duration = Duration::from_mins(2);

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
    let cwd = match subject_cwd(local, invocation) {
        Ok(cwd) => cwd,
        Err(capture) => return capture,
    };
    match child::run(local, cwd.as_deref(), &invocation.input, deadline) {
        Ok(capture) => capture,
        Err(reason) => refused(NO_SUCH_TOOL, &reason),
    }
}

/// **The worktree lane's gate on this box** (REMOTE §5.4, bl-36f7): an
/// invocation carrying a working directory runs there only when the
/// operator's own document says this entry may (`"subject_cwd": true`) —
/// a box must opt in to executing at a path a caller names, because the
/// directory is one of the three things thrall enforces (DESIGN §3.5) and
/// it stays the operator's unless the operator says otherwise. Both
/// refusals name the remedy in the operator's terms, and both are the
/// same in-band three facts every other refusal here is.
fn subject_cwd(
    local: &Local,
    invocation: &Invocation,
) -> Result<Option<std::path::PathBuf>, Capture> {
    let Some(cwd) = invocation.cwd.as_deref() else {
        return Ok(None);
    };
    if !local.tool.subject_cwd {
        return Err(refused(
            NO_SUCH_TOOL,
            &format!(
                "the invocation names a working directory, and this box's config \
                 does not consent: add \"subject_cwd\": true to the {:?} entry in \
                 tools.json on this machine, or call a loaded instance of the tool \
                 instead",
                invocation.tool
            ),
        ));
    }
    let path = std::path::PathBuf::from(cwd);
    if !path.is_dir() {
        return Err(refused(
            NO_SUCH_TOOL,
            &format!(
                "the invocation's working directory {cwd:?} is not a directory on \
                 this box; the consenting machine must actually hold the \
                 conversation's worktree"
            ),
        ));
    }
    Ok(Some(path))
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
