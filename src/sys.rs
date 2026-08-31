//! **The confined `unsafe` file** (`rules/unsafe-outside-sys.yml`, DESIGN §4):
//! raw process effects with no safe `std` spelling, and there is exactly one.
//!
//! **The first tenant is the child-termination cascade** (bl-4cda), which is
//! the one the rule predicted, and its subject is a process GROUP (bl-a78e). A tool that
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
//!
//! **The second tenant is the non-blocking pipe** (bl-6c14). A drain that ends
//! only when every writer has closed ends when a *stranger* lets go — a helper
//! the tool backgrounded holds the same write end — so the executor reads its
//! child's pipes without blocking and takes what is there (`exec::pipes`).
//! `std` puts `set_nonblocking` on sockets and on nothing else: there is no
//! spelling for it on a `ChildStdout`, and wrapping the descriptor in a
//! `UnixStream` to borrow the socket one would read the pipe with `recv(2)`,
//! which a pipe refuses outright. So it is `fcntl(2)`, declared here beside
//! `kill(2)` for the same reason and at the same cost: no crate, no build
//! script, no lockfile line.
//!
//! **Its soundness argument is `kill`'s, with less surface.** `fcntl` takes
//! integers and answers an integer; nothing here passes it a pointer, and the
//! one command spent takes no `struct`. The descriptor it is given is one this
//! process owns for as long as the call lasts — the caller holds the pipe — so
//! there is no id to guard the way a signal's is.

/// `SIGTERM`: ask a process to stop. The number is fixed by POSIX and is 15 on
/// every platform this runs on.
const SIGTERM: i32 = 15;

/// `SIGKILL`: stop it. 9, and it is here rather than borrowed from
/// [`Child::kill`](std::process::Child::kill) because that reaches one process
/// and the insist has to reach the group.
const SIGKILL: i32 = 9;

/// `F_SETFL`: set a file description's status flags. 4 on every Unix.
const F_SETFL: i32 = 4;

/// `O_NONBLOCK` — and it is the one number in this file that is not the same
/// everywhere: `0o4000` on Linux, `0x4` on the BSDs and macOS, which are the
/// two platforms this crate builds for (`Containerfile`, `Containerfile.mac`).
/// Two `cfg` arms rather than a dependency, because a crate carrying one
/// integer would carry a build script and a lockfile line with it.
#[cfg(target_os = "linux")]
const O_NONBLOCK: i32 = 0o4000;
#[cfg(not(target_os = "linux"))]
const O_NONBLOCK: i32 = 0x4;

unsafe extern "C" {
    /// `kill(2)`. Answers `0` when the signal was sent and `-1` otherwise.
    fn kill(pid: i32, sig: i32) -> i32;

    /// `fcntl(2)`, declared **variadic**, which is what it is. A fixed
    /// signature would be the wrong ABI on AArch64 Apple, where a variadic
    /// argument goes on the stack and a fixed one goes in a register — the
    /// class of mistake that works on the machine it was written on.
    fn fcntl(fd: i32, cmd: i32, ...) -> i32;
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

/// Put `fd` into non-blocking mode, and answer whether it took.
///
/// **`F_SETFL` outright, with no read-modify-write.** The descriptors this is
/// spent on are the pipes `Command` just minted, so the only status flag they
/// carry is the access mode — which `F_SETFL` ignores — and there is no
/// `O_APPEND` or `O_ASYNC` to preserve. A `F_GETFL` first would be a second
/// call to conserve a bit that cannot be set.
///
/// `false` is not an error to handle by policy: the caller is holding the
/// descriptor, so `EBADF` cannot happen and `F_SETFL` has no other failure for
/// a pipe. It is answered rather than swallowed so the fact is readable in a
/// test.
pub(crate) fn nonblocking(fd: i32) -> bool {
    (unsafe { fcntl(fd, F_SETFL, O_NONBLOCK) }) == 0
}

#[cfg(test)]
mod tests;
