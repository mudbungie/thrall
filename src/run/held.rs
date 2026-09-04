//! **The one capture this process is holding** (yog's `docs/REMOTE.md` §5.6,
//! ruling 1).
//!
//! A `complete` that fails on the WIRE leaves this foot standing with a capture
//! nobody else has: the tool ran here, the engine's slot still holds the lease
//! it handed out, and the driver is waiting. Dropping it — which is what this
//! crate did until this file existed — lets that lease re-run the tool on the
//! next dial, so the box pays for the work twice and the asker waits for the
//! second run of something already computed.
//!
//! **So it is carried out with the ending and posted first on the next dial**,
//! after the advertisement and BEFORE that channel's first follow-class read.
//! The order is the whole mechanism: the read is what releases the lease
//! (REMOTE §5.3), so a capture posted ahead of it lands on a slot the engine is
//! still holding with no capture of its own.
//!
//! **This is not the id ledger bl-9261 refused, and that refusal still
//! stands.** Nothing here is a remembered SET, nothing is compared, nothing
//! outlives the process, and no redelivered invocation is suppressed: a foot
//! handed an invocation it has no held capture for runs it, undeduped, exactly
//! as DESIGN §3.8 says. What this holds is **one** capture, on this process's
//! own stack, for an interval with a name — until the next channel's first act
//! is answered, landed or refused. bl-9261's second reason was that the
//! interval had no name; posting rather than waiting gives it one, and that is
//! the whole of what is narrowed.
//!
//! **A refusal of the re-post is dropped and the channel reads on.** An engine
//! that refuses it has swept the slot, restarted, or already handed the driver
//! an answer — all ordinary — and a foot that ended its channel over a capture
//! nobody was waiting for would be turning the ordinary case into the terminal
//! one. Only the wire swallowing it again keeps it held, because a wire failure
//! is not an answer: it says nothing about whether the slot is still there.

use crate::channel::Channel;
use crate::gestures;
use crate::invocation::{Capture, Invocation};

use super::hold::{Ending, Failed, tell};

/// **Post one capture back**, at the moment it is computed.
///
/// **The receipt is read rather than discarded.** An engine that refused the
/// completion — an expired handle, a slot addressed to another machine — is
/// saying that this foot and that engine disagree about what is in flight, and
/// a foot that kept answering into that would be posting captures nobody is
/// waiting for.
pub(super) fn answer(
    channel: &Channel,
    invocation: &Invocation,
    capture: &Capture,
) -> Result<(), Failed> {
    tell(channel, &gestures::complete(&invocation.id, capture)).map(|_| ())
}

/// **A capture the wire swallowed**, and the handle it answers.
///
/// The id is the engine's own, quoted back exactly as an ordinary completion
/// quotes it — this is the same gesture on the same lane, sent one channel
/// late, and not a second spelling of anything.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Held {
    /// The engine's handle on the invocation this box ran.
    pub(crate) id: String,
    /// What running it produced: computed once, and posted once.
    pub(crate) capture: Capture,
}

impl Held {
    /// What the foot is left holding when a completion fails on the wire.
    pub(super) fn of(invocation: &Invocation, capture: Capture) -> Self {
        Self {
            id: invocation.id.clone(),
            capture,
        }
    }

    /// **Post it, ahead of the read that would release the lease.**
    ///
    /// `None` is the channel reading on, and it is the answer to three of the
    /// four outcomes: the capture landed, the engine refused it, or the engine
    /// answered something no foot gesture can earn. All three are the far end
    /// having spoken, and once it has spoken this capture has no further claim
    /// on anything — it is dropped here and never becomes an [`Ending::Over`].
    ///
    /// `Some` is the wire swallowing it a second time, which is not an answer
    /// at all: the ending carries the same capture to the next dial, so the
    /// rule holds unchanged across a flapping wire and this process still holds
    /// exactly one.
    pub(super) fn post(self, channel: &Channel) -> Option<Ending> {
        match tell(channel, &gestures::complete(&self.id, &self.capture)) {
            Err(Failed::Wire(said)) => Some(Ending::Again {
                said,
                predecessor: false,
                served: false,
                held: Some(self),
            }),
            Ok(_) | Err(Failed::Refused(_) | Failed::Unusable(_)) => None,
        }
    }

    /// **Which endings carry it, and which drop it.**
    ///
    /// An ending that will be dialled again carries it; an `Over` drops it,
    /// and that is REMOTE §5.6's fourth consequence falling out of the matrix
    /// rather than a second decision. The two endings that stop a channel at a
    /// completion are the engine refusing it and the engine answering
    /// something no gesture can earn, and both say the two ends disagree about
    /// what is in flight — so re-posting it would ask the same question and
    /// get the same answer.
    pub(super) fn carried_by(self, ending: Ending) -> Ending {
        match ending {
            Ending::Again {
                said,
                predecessor,
                served,
                ..
            } => Ending::Again {
                said,
                predecessor,
                served,
                held: Some(self),
            },
            over @ Ending::Over(_) => over,
        }
    }
}
