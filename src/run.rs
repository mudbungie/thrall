//! **The loop** (yog's `docs/REMOTE.md` §5.3, §5.4): the whole of what a foot
//! does, for as long as it does anything.
//!
//! It is three questions, kept apart because a channel that is dialled again
//! has a lifetime that is not its conversation:
//!
//! - [`hold`] — **one channel's conversation**: present, wait, hand off,
//!   answer, present again, and the [`Ending`](hold::Ending) that stopped it.
//!   Beside it, [`held`] is the capture going back — posted as it is computed,
//!   or carried to the next dial when the wire swallowed it (REMOTE §5.6).
//! - [`redial`] — **one channel's lifetime**: an ending that is worth another
//!   dial, the wait before it, and the sentence said in the meantime.
//! - this file — **every channel this box holds**, one thread each, and the
//!   three things a serving foot does that a test cannot read back as a value.
//!
//! **Three seams are parameters, and all three for one reason** (bl-4cda,
//! bl-2d78, bl-916d). What runs a command, where a foot says something while it
//! is still serving, and where it waits are each an *effect*: a child process,
//! this process's stderr, a real minute of sleep. None can be read back by a
//! test, so each is injected, the one implementation of each lives in
//! `src/main.rs` — the single coverage exclusion — and the suite hands down a
//! one-line executor, a recording sink and a recording pause. That is what lets
//! the whole conversation be tested against an engine that is real.

use std::sync::Arc;
use std::time::Duration;

use crate::channel::entries::Entry;
use crate::config::Local;
use crate::invocation::{Capture, Invocation};

/// The capture going back: posted as it is computed, or held to the next dial.
pub(crate) mod held;
/// One channel's conversation, and how it ended.
pub(crate) mod hold;
/// One channel's lifetime: the ending that is dialled again, and the wait.
pub(crate) mod redial;

/// **The hand-off**: what runs one invocation against this box's set.
///
/// A plain function pointer rather than a closure or a trait object, because
/// the thing being injected is *code*, never state — a foot that carried
/// per-invocation state between calls would be a foot with a world. It is
/// `Send` and `Copy` for free, which is what lets [`fan`] hand the same
/// executor to every channel without a lock or a clone.
pub type Handoff = fn(&[Local], &Invocation) -> Capture;

/// **Where a foot says something while it is still serving.**
///
/// Three things reach an operator here, and none of them can wait for this
/// process to end: a disarming this foot has already healed (DESIGN §3.7), a
/// channel that dropped and is about to be dialled again (DESIGN §3.8), and —
/// since bl-e834 — the **terminal** sentence that stopped a channel for good.
/// The first two end nothing; the third ends one channel and not the box, which
/// on a multi-entry box is the same thing as far as this process's lifetime is
/// concerned. All three are facts an operator acts on the moment they are true,
/// so they need somewhere to go that is not a return value.
///
/// It is `Arc<dyn Fn>` rather than a bare `fn` because a recording sink has to
/// capture where it records, and shared across [`fan`]'s threads because a box
/// with several engines has one place to say things.
pub type Notice = Arc<dyn Fn(&str) + Send + Sync>;

/// **Where a foot waits between dials** (`redial`).
///
/// A real wait is an effect and not a value: a suite that slept the backoff
/// series out would spend a minute proving arithmetic. So the sleep is
/// `src/main.rs`'s, beside the stderr [`Notice`] it already owns, and every
/// test reads the waits back as the durations they would have been — which is
/// also the only way to assert that a loop waited the *right* amount.
pub type Pause = Arc<dyn Fn(Duration) + Send + Sync>;

/// **Every channel this box holds, served at once** — one thread each, and the
/// sentence that stopped each one, in the order they were filed.
///
/// Serial stays serial **per channel**: two engines are two conversations, and
/// a foot that queued one behind the other would be absent from one of them for
/// reasons the other's operator cannot see. Channels share nothing here — not a
/// connection, not an identity, not a lock — exactly as their material shares
/// nothing (`channel::entries`).
///
/// **Each channel is now dialled again for as long as this process lives**, so
/// what a thread here answers is the *terminal* sentence: the one ending that
/// asking again cannot improve. A box whose every channel has ended that way is
/// a foot that cannot be a foot, which is where supervision takes over.
///
/// **The returned vector is the exit's summary and never the report** (bl-e834).
/// These threads are joined in filing order, so a sentence read out of a join is
/// a sentence held until every sibling has stopped — which for a healthy sibling
/// is forever. A dead channel would then be a live-looking process serving one
/// engine and silently not serving another, and the two refusals yog bl-1462
/// added — the loudest thing an engine can say to this box — would arrive, if at
/// all, at a process that would not repeat them. So each channel says its own
/// terminal sentence through [`Notice`] the moment it stops ([`served`]), and
/// the vector below is what the exit is written from.
///
/// An empty set of entries stops instantly with nothing to say, which is
/// correct and is not this function's sentence to write: the caller is the one
/// that knows where entries would have been filed.
pub fn fan(
    entries: Vec<Entry>,
    set: Vec<Local>,
    handoff: Handoff,
    notice: &Notice,
    pause: &Pause,
) -> Vec<String> {
    let running: Vec<(String, std::thread::JoinHandle<String>)> = entries
        .into_iter()
        .map(|entry| {
            let set = set.clone();
            let leaf = entry.leaf.clone();
            let under = named(notice, &leaf);
            let pause = Arc::clone(pause);
            let running = std::thread::spawn(move || served(&entry, &set, handoff, &under, &pause));
            (leaf, running)
        })
        .collect();
    running
        .into_iter()
        .map(|(leaf, thread)| match thread.join() {
            Ok(said) => format!("{leaf}: {said}"),
            // Every path out of `redial` answers a sentence, so a panic here is
            // this program breaking rather than a channel ending. It is named
            // as its own outcome instead of being swallowed into a transport
            // sentence, and it is never re-raised into the other channels: one
            // engine's conversation must not take the others down. It is the one
            // outcome no channel says for itself, because a thread that unwound
            // past `served` had no sentence to say.
            Err(_) => format!("{leaf}: the channel ended by panicking"),
        })
        .collect()
}

/// **One channel, served until it is over — and heard the moment it is.**
///
/// The sentence is said before it is returned, which is the whole of bl-e834's
/// fix: this is the one place that holds both the channel's name and the ending
/// that stopped it *at the time it stopped*, and everything downstream of it is
/// a join that waits on the siblings. It says the same line the summary will
/// carry — `notice` is already this channel's own, under the same prefix
/// [`fan`] joins with — because an operator reading a supervisor's log and an
/// operator reading the exit must not have to reconcile two wordings of one
/// ending.
///
/// It is a function and not a closure body inside the spawn so that the
/// decision has somewhere to be written down, and so the spawn stays one
/// expression.
fn served(
    entry: &Entry,
    set: &[Local],
    handoff: Handoff,
    notice: &Notice,
    pause: &Pause,
) -> String {
    let said = redial::redial(entry, set, handoff, notice, pause);
    notice(&said);
    said
}

/// One channel's notices, under the name that channel was filed as — the same
/// prefix its ending sentence carries. A box holding two engines that was told
/// it had been disarmed, or that a channel is being dialled again, and not by
/// which one, has been told nothing it can act on.
fn named(notice: &Notice, leaf: &str) -> Notice {
    let notice = Arc::clone(notice);
    let leaf = leaf.to_owned();
    Arc::new(move |line: &str| notice(&format!("{leaf}: {line}")))
}

#[cfg(test)]
mod tests;
