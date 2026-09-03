//! **When a box says a channel has stopped** (bl-e834, DESIGN §3.9).
//!
//! Its own module and not `fan`'s, on the seam the complaint was filed on: what
//! a box SERVES is one question, and when the operator hears about it is
//! another. The test here is the second one, and the whole of it is timing —
//! the sentence's words are `hold`'s and `redial`'s, and the exit's summary is
//! `serve`'s.

use super::super::fan;
use super::{echo, engine_at, flapping_at, refusal, set};
use crate::channel::entries;
use crate::run::{Notice, Pause};
use crate::test_support::{Notices, Scratch};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, SyncSender};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

/// The sentence this test's release is keyed on: the ending of the channel that
/// stops while its sibling is still serving.
const ENDED: &str = "south: south stopped";

/// How long the sibling's wait may stand before this test calls the report
/// buffered. It is a bound on a FAILURE and never a wait a green run pays: when
/// the sentence is said at the moment the channel stops, the release is already
/// in the channel by the time the sibling asks for it.
const BOUND: Duration = Duration::from_secs(10);

/// **A channel's terminal sentence reaches the operator when THAT channel
/// stops, not when the box does** (bl-e834).
///
/// The sibling is the whole proof, and its two halves are the two ways this
/// could be wrong:
///
/// - **`north` is still serving when `south` ends.** Its wire flaps, and the
///   pause before its next dial does not return until the sink has heard
///   `south`'s ending. A `fan` that collected sentences and answered once every
///   channel had stopped never says that line at all, so the wait stands to
///   [`BOUND`] and the flag is false.
/// - **`north` is FIRST in filing order.** These threads are joined in that
///   order, so a report that said each sentence as it was *joined* — the near
///   miss — would still be stuck behind `north`, which cannot finish until
///   `south`'s sentence releases it. Only saying it in the channel's own thread
///   passes.
///
/// The vector is asserted too: the emission is beside the exit's summary and
/// never instead of it.
#[test]
fn a_terminal_sentence_is_said_while_a_sibling_is_still_serving() {
    let scratch = Scratch::new();
    let _north = flapping_at(
        &scratch.join("north"),
        vec![None, Some(refusal("north stopped"))],
    );
    let _south = engine_at(&scratch.join("south"), vec![refusal("south stopped")]);
    let (notices, into) = Notices::new();
    let (release, awaited) = std::sync::mpsc::sync_channel(1);
    let sink: Notice = Arc::new(move |line: &str| watch(&into, &release, line));
    let released = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&released);
    let held = Mutex::new(awaited);
    let pause: Pause = Arc::new(move |_| wait_out(&held, &flag));

    let said = fan(
        entries::read_dir(scratch.path()),
        set(),
        echo,
        &sink,
        &pause,
    );

    assert!(
        released.load(Ordering::Relaxed),
        "north waited out {BOUND:?} for a sentence south had already earned: {:?}",
        notices.heard()
    );
    assert_eq!(said, ["north: north stopped", "south: south stopped"]);
    assert!(notices.heard().contains(&ENDED.to_owned()), "{said:?}");
}

/// The sink under test: keep every line, and release the sibling's wait on the
/// one line this test is about. `try_send` because the release is a signal and
/// not a queue — a second identical sentence would have nothing to add.
fn watch(into: &Notice, release: &SyncSender<String>, line: &str) {
    into(line);
    if line == ENDED {
        let _ = release.try_send(line.to_owned());
    }
}

/// The sibling's wait: it ends when the sentence arrives, or at [`BOUND`], and
/// records which.
///
/// The received value is held to the closing brace on purpose — it is the
/// non-`Copy` value the `MutexGuard`'s cleanup region lands on, the same
/// coverage hazard `test_support`'s own recorders are written around.
fn wait_out(held: &Mutex<Receiver<String>>, flag: &AtomicBool) {
    let got = held
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .recv_timeout(BOUND);
    flag.store(got.is_ok(), Ordering::Relaxed);
}
