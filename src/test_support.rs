//! Scaffolding the suite shares, and nothing production reads. Compiled only
//! under `cfg(test)`.
//!
//! Two things live here that live nowhere else in the crate, and both are
//! deliberate:
//!
//! - **The fork lock** the spawn boundary takes ([`fork_lock`]). It is the
//!   `Mutex` the lock-confinement rule would otherwise send to `src/state.rs`,
//!   and the rule's own text says why it does not: a test's serialization lock
//!   is scaffolding, and folding it into the chokepoint would put test
//!   machinery in the file that inventories the program.
//! - **The certificate mint** ([`mint`]). thrall mints nothing (REMOTE §1.4,
//!   DESIGN §3.3) — the operator carries a pair to the box by hand — so the
//!   suite has to perform that act on the operator's behalf before it can test
//!   a channel at all. It is `cfg(test)`, it shells to the tool an operator
//!   would use, and **no certificate is ever committed**: a fixture key in a
//!   tree is a private key in a repository, which is the exact class
//!   `make leak-scan` refuses.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

/// The stand-in for the far end of the wire.
pub(crate) mod engine;
/// The operator's out-of-channel act, performed by the suite.
pub(crate) mod mint;

/// The binary-wide fork lock. See [`crate::spawn::output`] for the ETXTBSY
/// race it closes and why the discipline belongs at the fork rather than at the
/// write.
static FORK: Mutex<()> = Mutex::new(());

/// Take the fork lock. Poisoning is ignored on purpose: a panicking test tells
/// us about that test, and a poisoned lock that refused every later fork would
/// turn one failure into a suite.
pub(crate) fn fork_lock() -> MutexGuard<'static, ()> {
    FORK.lock().unwrap_or_else(PoisonError::into_inner)
}

/// How many scratch directories this process has minted, so two tests running
/// at once never name one directory.
static NEXT: AtomicUsize = AtomicUsize::new(0);

/// A throwaway directory, removed when it drops.
///
/// Hand-rolled rather than a crate, because the dependency set is closed
/// (`Cargo.toml`'s approval comment): a scratch directory is a `create_dir_all`
/// and a `remove_dir_all`, and a test-only crate is still a crate in the
/// lockfile, the licence audit and the supply chain.
pub(crate) struct Scratch {
    path: PathBuf,
}

impl Scratch {
    /// Make one, under the platform's temporary directory.
    pub(crate) fn new() -> Self {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("thrall-test-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&path).expect("a scratch directory");
        Self { path }
    }

    /// The directory itself.
    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    /// A path inside it.
    pub(crate) fn join(&self, leaf: &str) -> PathBuf {
        self.path.join(leaf)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

#[cfg(test)]
mod tests;
