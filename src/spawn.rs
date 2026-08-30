//! **The spawn boundary** (`rules/no-bare-command.yml`, `rules/no-bare-fork.yml`;
//! DESIGN §4): every child process in this crate is BUILT here and FORKED here,
//! and nowhere else does either.
//!
//! thrall is the component that runs commands on somebody else's machine, so
//! *what exactly is handed to a child* is not an implementation detail — it is
//! the product, and it has to be decided in one place. Two things are that
//! decision and each is invisibly wrong at a spawn site that forgot it:
//!
//! - **The inherited git environment.** `git` exports `GIT_DIR` and
//!   `GIT_INDEX_FILE` into every process it starts, and those OUTRANK `-C
//!   <repo>` and a `current_dir` — so a child spawned while one is set forks
//!   its own `git` against the wrong repository. [`INHERITED`] is scrubbed at
//!   the constructor, where one `env_remove` clears the whole descendant tree.
//! - **The fork itself, under test.** `fs::write` on a file holds a write fd; a
//!   fork on any *other* thread copies that fd into a child that keeps it until
//!   its own exec, and an exec of that same file inside the window is ETXTBSY.
//!   The victim's own care cannot save it — the fork is the other party — so
//!   the discipline belongs at the fork, and under `cfg(test)` the boundary
//!   takes one binary-wide lock across it.
//!
//! **It was founded before it had a production tenant** (bl-a4a5, for the
//! suite's stand-in certificate mint) and gained one with the executor
//! (bl-4cda). That order is the point of DESIGN §4: a boundary rule that
//! arrives after the first spawn site is a rule that has to be argued with.

use std::io;
#[cfg(test)]
use std::process::Output;
use std::process::{Child, Command};

/// Every environment variable a child must not inherit from this process.
///
/// The list is `git`'s ambient exports. It is scrubbed unconditionally rather
/// than when a `git` is involved, because the hazard is transitive: a child
/// that never runs `git` may start a grandchild that does.
pub(crate) const INHERITED: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_PREFIX",
    "GIT_COMMON_DIR",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
];

/// A command running `program` with every [`INHERITED`] variable removed. The
/// only lawful way to build a child here.
pub(crate) fn command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    for var in INHERITED {
        cmd.env_remove(var);
    }
    cmd
}

/// Run `cmd` to completion and collect what it said.
///
/// `cfg(test)`, because its one tenant is the suite's stand-in for the
/// operator's certificate mint — thrall itself mints nothing (REMOTE §1.4) and
/// the executor cannot use this shape at all (see [`spawn`]). It stays here
/// rather than in the suite because the boundary is a location, not a
/// convenience: a fork written anywhere else would be a second inventory.
#[cfg(test)]
pub(crate) fn output(cmd: &mut Command) -> io::Result<Output> {
    #[cfg(test)]
    let _fork = crate::test_support::fork_lock();
    cmd.output()
}

/// Start `cmd` and hand back the child — **the fork the executor spends**, and
/// the only other lawful way to start a process here.
///
/// It is separate from [`output`] because a foot cannot collect and then
/// decide: a tool that outruns its deadline has to be *reachable* while it
/// runs, which is exactly what `output` does not give you.
pub(crate) fn spawn(cmd: &mut Command) -> io::Result<Child> {
    #[cfg(test)]
    let _fork = crate::test_support::fork_lock();
    cmd.spawn()
}

#[cfg(test)]
mod tests;
