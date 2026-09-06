//! **How a channel ended, and who ended it** (yog's `docs/REMOTE.md` §3, §5.1;
//! DESIGN §3.8) — the classification, kept apart from the conversation that
//! produces it because they are two questions: what happened, and what to do
//! about it.
//!
//! **The decision is drawn from who failed and at which leg, never from the
//! engine's prose.** A foot that decided its own lifetime by reading sentences
//! would be a foot the far end could rewrite by rewording.
//!
//! **Three parties can fail a gesture and the split between them is the whole
//! of this file.** The wire has no opinion of either binary. The engine can
//! say no, and whether that is worth asking again is the LEG's answer. And the
//! two binaries themselves can disagree about the protocol, which is the one
//! failure that is not about this conversation at all: REMOTE §3 states that a
//! mismatch has no negotiation and names the remedy — *"upgrade the older
//! component"* — so a foot that dialled again would spend a handshake a minute
//! to be told the same thing until somebody installed a new binary (bl-5d62).

use super::held::Held;
use crate::channel::Failure;

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
        /// **The capture the wire swallowed on the way out**, when this ending
        /// is carrying one: the next dial posts it first, ahead of the read
        /// that releases the engine's lease (REMOTE §5.6 ruling 1, [`Held`]).
        /// `None` is the ordinary ending, which has nothing in flight.
        held: Option<Held>,
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
pub(crate) enum Failed {
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
pub(super) enum Leg {
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
    pub(super) fn at(self, leg: Leg, served: bool) -> Ending {
        match (self, leg) {
            (Self::Wire(said), _) => Ending::Again {
                said,
                predecessor: false,
                served,
                held: None,
            },
            (Self::Refused(said), Leg::Read) => Ending::Again {
                said,
                predecessor: true,
                served,
                held: None,
            },
            (Self::Refused(said) | Self::Unusable(said), _) => Ending::Over(said),
        }
    }
}

impl From<Failure> for Failed {
    /// **A channel that could not carry the gesture, as the party that failed
    /// it.** The transport is the engine's absence and is dialled again; a
    /// version the two ends do not share is neither end's *answer* and is the
    /// one thing this foot cannot use however it asks, so it ends the channel
    /// wherever it is met.
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Wire(said) => Self::Wire(said),
            Failure::Skew(said) => Self::Unusable(said),
        }
    }
}
