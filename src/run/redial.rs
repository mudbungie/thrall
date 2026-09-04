//! **One channel's lifetime**: taken up, served, and taken up again (yog's
//! `docs/REMOTE.md` §5.3, DESIGN §3.8, bl-916d).
//!
//! **The no-reconnect ruling is reversed at the channel and kept at the
//! process.** Its premise was sound and its conclusion did not follow:
//! supervision restarts a *process*, and the failure this exists for does not
//! kill one. REMOTE §1's canonical box sleeps, changes network and crosses a
//! relay switch; TCP drops; the channel's conversation ends with its sentence;
//! and the foot stays healthy, serving whatever other engines it holds. A
//! multi-entry box loses one channel of several and there is no exit code for a
//! supervisor to see, while the engine believes this box is gone (presence is
//! connection RAM, correctly) and the box believes it is serving. **So a foot
//! redials its own channels, and still exits when it cannot be a foot at all** —
//! which is the part supervision was always the right owner of.
//!
//! **It is not a session resume and there is nothing to resume.** A redial
//! makes a *new* channel: presence re-forms as it does for any fresh
//! connection, the advertisement rides the connection already, registration is
//! durable engine-side, and an invocation in flight when the wire died is the
//! engine's mailbox lease rather than this loop's. DESIGN §3.8 states what
//! that costs and why it is the right price.
//!
//! **One thing crosses, and it is an act rather than a memory** (REMOTE §5.6
//! ruling 1, `run::held`): a capture this box computed and the wire swallowed
//! on the way back. The next dial posts it first and is done with it — landed,
//! refused or dropped — before it reads. That is not a resumed session and not
//! a ledger: one capture, on this loop's stack, spent on the next channel's
//! first act.
//!
//! **It must not hammer, so the wait is the whole of this file's decision.**
//! Three numbers and one line of arithmetic ([`next`]), kept apart from the
//! conversation that produces the endings so that both are readable and the
//! arithmetic is testable as values.
//!
//! **And it imposes no deadline of its own.** The engine has no socket timeouts
//! (yog bl-1421, filed and unbuilt), so the first dial after a flap may be
//! answered slowly by an engine still working out that the previous connection
//! is gone. The only deadline in the path is the transport one the channel
//! already had — a bound on the *socket*, sitting well above the engine's hold
//! — and if it fires it is the wire, which is retryable. So a slow engine costs
//! one more dial rather than the channel, and nothing here waits on bl-1421.

use std::time::Duration;

use super::hold::{Ending, hold};
use super::{Handoff, Notice, Pause};
use crate::channel::entries::Entry;
use crate::config::Local;

/// **The first wait after a channel drops**, and the floor the series returns
/// to. It is short because the ordinary case is a blip that is already over by
/// the time it is noticed — a laptop's lid, a relay switch — and a foot that
/// slept a minute over that would be absent for the failure rather than for its
/// cause.
pub(super) const FIRST: Duration = Duration::from_secs(1);

/// **The longest a foot waits between dials.** The series doubles from
/// [`FIRST`] and stops here, so a box in airplane mode reaches a dial a minute
/// on its seventh attempt and stays there: a slow cadence rather than a burnt
/// core, which is what "it must not spin" has to mean for a box whose network
/// is not coming back for hours.
pub(super) const CAP: Duration = Duration::from_secs(64);

/// **How long a vanished predecessor's claim can still stand.** REMOTE §5.1
/// states it as a contract rather than an accident — *"Its life is the hold and
/// not the connection's ... `Mailbox::take` drops the claim on the way out,
/// before the caller writes the answer, so a peer that vanished without a FIN
/// frees the slot within one hold's width — thirty seconds"* — so a redial that
/// meets the one-reader refusal knows exactly how long the thing refusing it
/// has left. Two seconds over the stated width, because the predecessor's
/// window began before this end noticed the drop and the series doubles past
/// this anyway if the guess is short.
pub(super) const PREDECESSOR: Duration = Duration::from_secs(32);

/// **Serve one channel for as long as this process lives.**
///
/// The entry is opened once and not per dial: opening reads no file and dials
/// nothing — it is a fact about this box's own material — so a failure there is
/// this box's configuration and never an engine, and asking it again would ask
/// the same question. Everything after it is the engine's, and that is the part
/// that is asked again.
pub(crate) fn redial(
    entry: &Entry,
    set: &[Local],
    handoff: Handoff,
    notice: &Notice,
    pause: &Pause,
) -> String {
    let channel = match entry.open() {
        Ok(channel) => channel,
        Err(reason) => return reason,
    };
    let mut series = FIRST;
    // The one thing that crosses a redial, and it crosses forward rather than
    // being remembered: a capture the wire swallowed, posted by the next dial
    // before its first read (REMOTE §5.6 ruling 1, `run::held`). It is at most
    // one, it is this loop's stack and nowhere else, and it goes no further
    // than the next channel's first act.
    let mut held = None;
    loop {
        match hold(&channel, set, handoff, notice, held.take()) {
            Ending::Over(said) => return said,
            Ending::Again {
                said,
                predecessor,
                served,
                held: carried,
            } => {
                held = carried;
                let (wait, then) = next(series, predecessor, served);
                series = then;
                notice(&waiting(&said, predecessor, wait));
                pause(wait);
            }
        }
    }
}

/// **The wait before the next dial, and the series it leaves behind.**
///
/// Three rules, and each answers one of the ways this loop could be wrong:
///
/// - **It doubles and it caps**, so a box with no network settles instead of
///   spinning.
/// - **A one-reader refusal waits at least one hold's width**, because the
///   thing refusing is this box's own predecessor and REMOTE §5.1 says exactly
///   how long it has left. Asking sooner earns the same sentence and spends a
///   handshake to hear it. The series still advances underneath, so a refusal
///   that is a genuine rival rather than a predecessor backs off to the cap
///   like anything else.
/// - **A channel that served returns the series to its floor.** One answered
///   read is the engine having parked this foot for its own hold, which is the
///   evidence that this channel was real; a loop hammering a broken engine
///   cannot produce it without that engine handing out work, and a foot already
///   dials as fast as an engine answers reads. So the reset adds no cadence the
///   conversation did not already have — and without it a laptop that sleeps
///   nightly would creep to a minute's wait and stay there for the life of the
///   process, which is the failure this whole loop exists to shorten.
pub(super) fn next(series: Duration, predecessor: bool, served: bool) -> (Duration, Duration) {
    let series = if served { FIRST } else { series };
    let floor = if predecessor { PREDECESSOR } else { FIRST };
    (series.max(floor), (series * 2).min(CAP))
}

/// **What a redial says while it is still serving.**
///
/// Under a loop the sentence that ended a channel is no longer a sentence
/// anything returns, so it has to reach the operator as it happens or not at
/// all — and what an operator needs from it is the same two facts the ending
/// sentence always carried: what happened, and what this box will do next.
///
/// The predecessor case names itself, because "refused, and dialling again"
/// reads as a foot ignoring a refusal unless it says whose refusal it expects.
fn waiting(said: &str, predecessor: bool, wait: Duration) -> String {
    let reading = if predecessor {
        " — the engine refused this box's read, which after a drop is this box's own \
         connection still dying"
    } else {
        ""
    };
    format!(
        "{said}{reading}. Dialling this engine again in {}s.",
        wait.as_secs()
    )
}
