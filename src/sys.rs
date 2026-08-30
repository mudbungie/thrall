//! **The confined `unsafe` file** (`rules/unsafe-outside-sys.yml`, DESIGN §4):
//! raw process effects with no safe `std` spelling, and there is exactly one.
//!
//! **The tenant is the child-termination cascade** (bl-4cda), which is the one
//! the rule predicted. A tool that outruns the foot's deadline is sent
//! `SIGTERM` and, if it is still there after a grace, `SIGKILL` — and `std`
//! spells only the second half: [`Child::kill`](std::process::Child::kill) is
//! `SIGKILL` and there is no `Child::terminate`. A tool that traps `SIGTERM` to
//! flush a file or drop a lock is a tool worth giving the chance, so the first
//! half is worth this file existing for.
//!
//! **The declaration rather than a dependency.** `kill(2)` lives in libc, which
//! `std` already links, so declaring the one symbol costs no crate, no build
//! script and no lockfile line — where a C-toolchain crate would cost all three
//! and break the single-binary story that is most of why a foot is easy to
//! install on a box nobody administers for you.
//!
//! **The soundness argument.** `kill` is an ordinary syscall wrapper with no
//! memory contract at all: its arguments are two integers and its answer is an
//! integer. What makes it `unsafe` in Rust is that it is `extern`, not that it
//! can corrupt anything. The one real hazard is the argument's *sign* — a
//! non-positive first argument means a process **group** or *every process this
//! user may signal*, never one process — and that is refused below rather than
//! documented.

/// `SIGTERM`: ask a process to stop. The number is fixed by POSIX and is 15 on
/// every platform this runs on.
const SIGTERM: i32 = 15;

unsafe extern "C" {
    /// `kill(2)`. Answers `0` when the signal was sent and `-1` otherwise.
    fn kill(pid: i32, sig: i32) -> i32;
}

/// Ask the process `pid` to stop, and answer whether the signal was sent.
///
/// **Only ever one process.** A `pid` that is not strictly positive means a
/// group, or the whole session, to `kill(2)` — so it is refused here rather
/// than passed through. thrall terminates the child it spawned and nothing
/// else; what that child started is its own to clean up (DESIGN §3.5).
///
/// `false` is not an error to handle: a process that is already gone cannot be
/// asked to stop, and the caller's next move — wait briefly, then `SIGKILL` —
/// is the same either way.
pub(crate) fn terminate(pid: u32) -> bool {
    match i32::try_from(pid) {
        Ok(pid) if pid > 0 => (unsafe { kill(pid, SIGTERM) }) == 0,
        _ => false,
    }
}

#[cfg(test)]
mod tests;
