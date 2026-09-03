//! Scaffolding the suite shares, and nothing production reads. Compiled only
//! under `cfg(test)`.
//!
//! Three things live here that live nowhere else in the crate, and all three
//! are deliberate:
//!
//! - **The notice sink** ([`Notices`]) and **the pause sink** ([`Waits`]). A
//!   serving foot says things without ending anything — that it was disarmed
//!   while a tool ran (DESIGN §3.7), that a channel dropped and is about to be
//!   dialled again (DESIGN §3.8) — and it sleeps between those dials. Writing
//!   to stderr and sleeping are both effects no test can read back, so both
//!   live in `src/main.rs` and the suite hands channels recorders instead. The
//!   pause recorder is also the only way to assert that a loop waited the
//!   *right* amount rather than merely that it waited.
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
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

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

/// **The notice sink, recording.** A serving foot writes what it has to say
/// mid-channel to stderr (`src/main.rs`), which no test can read back — so the
/// suite hands it this instead and reads the sentences as values.
///
/// One sink for the whole suite rather than one per test module, because an
/// empty closure written in five files is five code locations the coverage
/// floor then owes a caller apiece.
#[derive(Default)]
pub(crate) struct Notices(Arc<Mutex<Vec<String>>>);

impl Notices {
    /// A recorder, and the sink to hand a channel.
    ///
    /// The sink's body is [`record`] rather than a block, so the closure is one
    /// expression: a braced body puts a region on its closing line that
    /// llvm-cov reports uncovered however many times the closure runs.
    pub(crate) fn new() -> (Self, crate::run::Notice) {
        let said = Self::default();
        let into = Arc::clone(&said.0);
        (said, Arc::new(move |line: &str| record(&into, line)))
    }

    /// Everything it was told, in order.
    pub(crate) fn heard(&self) -> Vec<String> {
        self.0
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// One notice, kept.
///
/// The line is owned before the lock is taken rather than inside the `push`,
/// which is the shape [`engine`]'s recorder already uses: a temporary built
/// inside the locked call earns a drop region llvm-cov reports uncovered on
/// however many times the line runs.
fn record(said: &Arc<Mutex<Vec<String>>>, line: &str) {
    let line = line.to_owned();
    said.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(line);
}

/// **The pause sink, recording.** A foot's wait between dials is a real sleep
/// (`src/main.rs`), so the suite hands it this instead and reads back the
/// durations the redial *decided* on — which is the assertion that matters,
/// and the reason the whole suite proves a minute-long backoff in no time at
/// all.
///
/// **It is a channel where [`Notices`] is a `Mutex`, and that is a coverage
/// fact rather than a taste.** A `MutexGuard` dropped at the end of a
/// statement earns a cleanup region llvm-cov reports uncovered however often
/// the function runs, unless the enclosing function has some other value to
/// drop at its closing brace for the region to land on. [`record`] has one —
/// the `String` it owns first — and a duration recorder cannot, `Duration`
/// being `Copy`. Measured twice, at 831/832. A `Sender` has nothing to drop
/// per send, so the hazard is dissolved rather than dodged.
pub(crate) struct Waits(Receiver<Duration>);

impl Waits {
    /// A recorder, and the pause to hand a channel.
    pub(crate) fn new() -> (Self, crate::run::Pause) {
        let (into, waited) = mpsc::channel();
        (
            Self(waited),
            Arc::new(move |wait: Duration| keep(&into, wait)),
        )
    }

    /// Every wait it was asked for since the last time it was asked, in order.
    /// It **drains**, which no caller here needs twice and every caller here
    /// makes after the channel it was watching has stopped.
    pub(crate) fn heard(&self) -> Vec<Duration> {
        self.0.try_iter().collect()
    }
}

/// One wait, kept. A free function for [`record`]'s reason: a braced closure
/// body earns a region llvm-cov reports uncovered however often it runs.
///
/// The send's failure is ignored because [`unwaited`] is exactly the case that
/// produces it — the recorder is dropped and only the pause is kept — and a
/// test that kept no recorder is a test asking for no waits.
fn keep(into: &Sender<Duration>, wait: Duration) {
    let _ = into.send(wait);
}

/// A pause for a test whose subject is not how long the foot waited. It is
/// [`aside`]'s twin, and never sleeps.
pub(crate) fn unwaited() -> crate::run::Pause {
    Waits::new().1
}

/// A sink for a test whose subject is not what the channel said. It records
/// into a recorder nobody reads, which is cheaper than a second empty closure:
/// an unread `Notices` is one code location the coverage floor already owes a
/// caller, and five hand-written `|_| {}` sinks would be five more.
pub(crate) fn aside() -> crate::run::Notice {
    Notices::new().1
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
