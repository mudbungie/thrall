//! **One channel's conversation**, and how it ended (yog's `docs/REMOTE.md`
//! §5.1, §5.3).
//!
//! ```text
//! advertise → { invocations → hand off → complete → advertise } until something fails
//! ```
//!
//! **Nothing in it is thrall speaking unprompted.** Every leg is a reply to
//! something this end asked for: the advertisement is an act it sends, the work
//! arrives as the answer to a read it issued, and the capture goes back as
//! another act. The engine never speaks first (REMOTE §3), and the shape of
//! this file is that invariant rather than a consequence of it.
//!
//! **The advertisement is presented once per channel and again after every
//! hand-off** (bl-2d78, yog bl-1462). A foot dials per ask (`channel`), so "on
//! every connect" in the protocol's own words means each time it takes up a
//! channel — and the far end writes the set only when it differs from what is
//! stored, so a re-presentation of an unchanged set costs a comparison and no
//! write.
//!
//! The re-assertion is there because the stored set is keyed on this box's
//! *identity*, not on this connection, and any connection bearing the
//! certificate may replace it. The engine refuses a replacement while this
//! machine holds a parked read (REMOTE §5.1), which covers the whole of an idle
//! foot's life — so the one window left open is the one this foot opens itself:
//! it is **absent** while it executes, and a set blanked in that window would
//! stand until the process restarted, with every later invocation refused for a
//! tool that plainly exists. Re-asserting at the end of each hand-off bounds
//! that window to one tool's runtime instead of forever.
//!
//! **And since PROTOCOL 8 it also buys knowing** (yog bl-66d4). The receipt
//! carries `wrote`: false when the engine compared, true when it changed the
//! document. So a re-assertion that WROTE is this machine being told it was
//! disarmed while it was absent — the set it presented was not the set in
//! force — and that is said out loud rather than healed in silence.
//!
//! **A true on the FIRST presentation of a channel says nothing** and is not
//! reported: every fresh channel presents into whatever the engine holds, and
//! the ordinary first one writes. Only a re-assertion — a presentation this
//! foot made after a hand-off it just performed — can distinguish a rival from
//! a beginning. **A redial makes a fresh channel** (`redial`), so that silence
//! holds across one too, and DESIGN §3.8 says why nothing is remembered over
//! the gap.
//!
//! **The loop is serial, and that is what makes a busy foot absent** (REMOTE
//! §5's presence amendment). It runs one invocation at a time and holds a
//! connection only while it is waiting, so the far end sees it vanish for as
//! long as a tool takes — which is why the engine's routing predicate is the
//! mailbox queue and never presence.
//!
//! **The execution itself is a hand-off, and it is not here** (bl-4cda). What
//! this file owns is the conversation; what runs a command owns the deadlines,
//! the containment and the transcode.
//!
//! **What this file does NOT own is the channel's lifetime.** It answers one
//! [`Ending`] and stops; whether that ending is dialled again is `redial`'s,
//! and the two are separate files because they are separate questions — what
//! happened, and what to do about it.

use serde_json::Value;

use super::{Handoff, Notice};
use crate::channel::Channel;
use crate::config::{self, Local};
use crate::gestures::{self, Reply};
use crate::invocation::{Capture, Invocation};

/// **What this machine says when a re-assertion actually wrote.**
///
/// The engine writes only a set that differs, so a `true` here means the set in
/// force was not the set this foot presents — something replaced it while this
/// box was executing and could not be told apart from a machine that had gone
/// away (REMOTE §5.1). The restoration is automatic; being told is not, and it
/// is the telling this whole chain was for.
///
/// It names both readings an operator can act on, because the foot cannot tell
/// them apart and guessing would be worse than saying so.
const DISARMED: &str = "this box's advertised set was not the set in force and has just been \
    restored: it was replaced while a tool was running. Either another connection is bearing \
    this box's identity, or the engine lost the set it was holding.";

/// **How a channel ended, and whether taking it up again could help.**
///
/// The distinction is drawn from **who failed and at which leg**, never from
/// the engine's prose: a foot that decided its own lifetime by reading
/// sentences would be a foot the far end could rewrite by rewording.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Ending {
    /// **Dial again** (REMOTE §5.3's reversal, bl-916d).
    Again {
        /// The sentence that ended it — said as it happens, because under a
        /// redial it is no longer a sentence anything returns.
        said: String,
        /// Whether it was the one-reader refusal: the claim a vanished
        /// predecessor of *this box* still holds. Its life is one hold's width
        /// and not the connection's (REMOTE §5.1), so it is the one wait whose
        /// length is known in advance.
        predecessor: bool,
        /// Whether the engine answered a read on this channel before it ended.
        /// One answered read is the engine having parked this foot for its own
        /// hold, which is the evidence that this channel was real — and a
        /// hammering loop cannot manufacture it.
        served: bool,
    },
    /// **This channel is over.** This box's own material, an engine refusing
    /// what this box offers or what it captured, or an answer no foot gesture
    /// can earn: dialling again would ask the same question and get the same
    /// answer.
    Over(String),
}

/// **A gesture that did not land, by who failed** — which is the whole of what
/// the ending turns on, and the one thing the engine's own sentence cannot be
/// asked for.
enum Failed {
    /// **The wire.** It carries no opinion of this foot at all, so it is worth
    /// dialling again whichever leg it struck.
    Wire(String),
    /// **The engine spoke, and what it said is no.** Whether that is worth
    /// asking again is the LEG's answer and not this one's.
    Refused(String),
    /// **The engine answered something this foot cannot use.** Asking again
    /// would ask the same question.
    Unusable(String),
}

/// Which gesture was in flight, which is the whole of what decides a refusal.
enum Leg {
    Advertisement,
    Read,
    Completion,
}

impl Failed {
    /// **The decision matrix, and it is three rows.** The wire is always worth
    /// another dial. A refusal of this box's *read* is REMOTE §5.1's one-reader
    /// guard, which after a blip names this very machine — a predecessor whose
    /// claim is already expiring, so a foot that took it as final would make
    /// the first blip permanent. Every other refusal, and every answer this
    /// foot cannot read, ends the channel: an engine declining the set this box
    /// offers is telling it another connection is serving under its name
    /// (bl-2d78), and an engine declining a capture is telling it the two ends
    /// disagree about what is in flight.
    fn at(self, leg: Leg, served: bool) -> Ending {
        match (self, leg) {
            (Self::Wire(said), _) => Ending::Again {
                said,
                predecessor: false,
                served,
            },
            (Self::Refused(said), Leg::Read) => Ending::Again {
                said,
                predecessor: true,
                served,
            },
            (Self::Refused(said) | Self::Unusable(said), _) => Ending::Over(said),
        }
    }
}

/// **One channel, served** — until something ends it, and then what ended it.
///
/// There is no success return and none is spelled: the only way out is a
/// gesture that did not land, so an `Ok` arm would be one no state of the world
/// can reach.
pub(crate) fn hold(channel: &Channel, set: &[Local], handoff: Handoff, notice: &Notice) -> Ending {
    let presenting = gestures::advertise(&config::advertisement(set));
    let mut served = false;
    // The first presentation's reading is discarded on purpose: a fresh channel
    // writes whenever the engine held something else, and there is no rival in
    // that. Only a re-assertion below can mean one.
    if let Err(failed) = present(channel, &presenting) {
        return failed.at(Leg::Advertisement, served);
    }
    loop {
        let work = match waited(channel) {
            Ok(work) => work,
            Err(failed) => return failed.at(Leg::Read, served),
        };
        served = true;
        for invocation in work {
            let capture = handoff(set, &invocation);
            if let Err(failed) = answer(channel, &invocation, &capture) {
                return failed.at(Leg::Completion, served);
            }
            match present(channel, &presenting) {
                Err(failed) => return failed.at(Leg::Advertisement, served),
                Ok(true) => notice(DISARMED),
                Ok(false) => {}
            }
        }
    }
}

/// **Say what this box offers.** Once when the channel is taken up, and once
/// more at the end of every hand-off, closing the window this foot was absent
/// for (see this module's head).
///
/// It is the same value both times rather than a second projection of the
/// config, so "the set this foot presents" has one spelling and the
/// re-assertion cannot drift from the presentation.
///
/// **It answers the engine's reading**: whether that presentation WROTE the
/// stored set or found it identical and compared (REMOTE §5.1's `wrote`,
/// PROTOCOL 8). What that means depends on which presentation it was, so the
/// judgement is the caller's and this is only the reading.
///
/// **And it is strict about the kind.** An engine that answered the
/// advertisement with something other than the advertisement's receipt has not
/// said the set landed, and a foot that read on regardless would be waiting for
/// work under a set nobody confirmed.
fn present(channel: &Channel, presenting: &Value) -> Result<bool, Failed> {
    match tell(channel, presenting)? {
        Reply::Advertised { wrote } => Ok(wrote),
        other => Err(Failed::Unusable(format!(
            "the engine answered {other:?}, not that the advertisement landed"
        ))),
    }
}

/// **The follow-class read**: this machine's next work, or the empty answer of
/// a hold that ended quietly. Both are ordinary; only a channel failure is not.
fn waited(channel: &Channel) -> Result<Vec<Invocation>, Failed> {
    match tell(channel, &gestures::invocations())? {
        Reply::Invocations(rows) => Ok(rows),
        other => Err(Failed::Unusable(format!(
            "the engine answered {other:?}, not this machine's work"
        ))),
    }
}

/// Post one capture back.
///
/// **The receipt is read rather than discarded.** An engine that refused the
/// completion — an expired handle, a slot addressed to another machine — is
/// saying that this foot and that engine disagree about what is in flight, and
/// a foot that kept answering into that would be posting captures nobody is
/// waiting for.
///
/// **A capture the WIRE swallowed is a different thing and is not lost**: the
/// engine's mark on a handed-out invocation is a lease, released by this
/// client's own next read, so the redial's fresh channel is handed the same
/// invocation under the same id and runs it again (REMOTE §5.3's at-least-once
/// leg, yog bl-e658). Nothing here has to remember it — and nothing here may:
/// the id is offered as an idempotency key and a foot declines it, because
/// suppressing the second run without answering it would leave the engine
/// holding a slot with no capture, which is the silence that ball was about
/// (DESIGN §3.8, bl-9261).
fn answer(channel: &Channel, invocation: &Invocation, capture: &Capture) -> Result<(), Failed> {
    tell(channel, &gestures::complete(&invocation.id, capture)).map(|_| ())
}

/// One gesture over the channel, read back with the one decoder — so this foot
/// speaks exactly what the boundary speaks and can add nothing to it.
///
/// **The last frame is the answer.** Today every one of the three gestures is
/// answered in one frame; when a read's answer is several, the newest is the
/// one that stands (REMOTE §3: the streaming form is not a second form).
///
/// **The three ways it can fail are three different parties**, and keeping them
/// apart here is what lets [`Failed::at`] decide a lifetime without reading a
/// sentence: the socket, the engine's refusal, and an answer no foot gesture
/// can earn. A stream that terminated with no frame in it belongs to the last
/// of those: the terminator is a zero-length frame the engine deliberately
/// wrote, where a peer that went away is a read error instead.
fn tell(channel: &Channel, request: &Value) -> Result<Reply, Failed> {
    let stream = channel.ask(request).map_err(Failed::Wire)?;
    let last = stream.last().ok_or_else(|| {
        Failed::Unusable("the engine ended the stream without answering".to_owned())
    })?;
    match gestures::decode(last) {
        Ok(Ok(reply)) => Ok(reply),
        Ok(Err(refusal)) => Err(Failed::Refused(refusal)),
        Err(unusable) => Err(Failed::Unusable(unusable)),
    }
}
