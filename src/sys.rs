//! **The confined `unsafe` file** (`rules/unsafe-outside-sys.yml`, DESIGN §4):
//! raw process effects with no safe `std` spelling, and there is exactly one.
//!
//! **The tenant is the child-termination cascade** (bl-4cda), which is the one
//! the rule predicted, and its subject is a process GROUP (bl-a78e). A tool that
//! outruns the foot's deadline is sent `SIGTERM` and, if it is still there after
//! a grace, `SIGKILL` — and `std` spells neither for a group: [`Child::kill`]
//! (std::process::Child::kill) is `SIGKILL` to one process and there is no
//! `Child::terminate` at all. A tool that traps `SIGTERM` to flush a file or
//! drop a lock is a tool worth giving the chance, and a helper that tool started
//! is a process the deadline has to reach, so both halves are worth this file
//! existing for.
//!
//! **The declaration rather than a dependency.** `kill(2)` lives in libc, which
//! `std` already links, so declaring the one symbol costs no crate, no build
//! script and no lockfile line — where a C-toolchain crate would cost all three
//! and break the single-binary story that is most of why a foot is easy to
//! install on a box nobody administers for you. The *other* half of the group
//! cascade needs no declaration at all: putting the child at the head of its own
//! group is `CommandExt::process_group`, safe `std`, and it belongs at the spawn
//! boundary beside the environment scrub rather than here.
//!
//! **The soundness argument.** `kill` is an ordinary syscall wrapper with no
//! memory contract at all: its arguments are two integers and its answer is an
//! integer. What makes it `unsafe` in Rust is that it is `extern`, not that it
//! can corrupt anything. The one real hazard is the argument's *sign* — a
//! non-positive first argument means a process **group**, or *every process this
//! user may signal* — and the guard on it did not move when the group arrived.
//! What crosses this boundary is still a strictly positive id, refused below
//! when it is not; the negation is this file's own act, spelled once, inside
//! functions whose names say `group`. A caller cannot widen a signal by passing
//! a different number, only by calling a differently-named function.

/// `SIGTERM`: ask a process to stop. The number is fixed by POSIX and is 15 on
/// every platform this runs on.
const SIGTERM: i32 = 15;

/// `SIGKILL`: stop it. 9, and it is here rather than borrowed from
/// [`Child::kill`](std::process::Child::kill) because that reaches one process
/// and the insist has to reach the group.
const SIGKILL: i32 = 9;

unsafe extern "C" {
    /// `kill(2)`. Answers `0` when the signal was sent and `-1` otherwise.
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Ask every process in the group `pgid` to stop, and answer whether the signal
/// was sent.
///
/// The group, not the process, because a tool that starts a helper and does not
/// stop is a tool whose deadline has to mean something for the helper too
/// (DESIGN §3.5). The executor's child leads a group of its own — the spawn
/// boundary puts it there — so a child's id is its group's id.
pub(crate) fn terminate_group(pgid: u32) -> bool {
    signal_group(pgid, SIGTERM)
}

/// Stop every process in the group `pgid` outright: the insist half of the
/// cascade, for a group that had its chance.
pub(crate) fn kill_group(pgid: u32) -> bool {
    signal_group(pgid, SIGKILL)
}

/// The one negation, and the guard it does not get to skip.
///
/// A `pgid` that is not strictly positive is refused rather than passed through:
/// negated, `0` would name *this* process's group and `-1` every process this
/// user may signal, so the two arguments that widen a signal beyond the group
/// asked for are the two that never reach the call.
///
/// `false` is not an error to handle: a group that is already gone cannot be
/// asked to stop, and the caller's next move is the same either way.
fn signal_group(pgid: u32, sig: i32) -> bool {
    match i32::try_from(pgid) {
        Ok(pgid) if pgid > 0 => (unsafe { kill(-pgid, sig) }) == 0,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
