//! **The one capture this process is holding** (yog's `docs/REMOTE.md` §5.6,
//! ruling 1).
//!
//! A `complete` that fails on the WIRE leaves this foot standing with a capture
//! nobody else has: the tool ran here, the server's slot still holds the lease
//! it handed out, and the driver is waiting. Dropping it — which is what this
//! crate did until this file existed — lets that lease re-run the tool on the
//! next dial, so the box pays for the work twice and the asker waits for the
//! second run of something already computed.
//!
//! **So it is carried out with the ending and posted first on the next dial**,
//! after the advertisement and BEFORE that channel's first follow-class read.
//! The order is the whole mechanism: the read is what releases the lease
//! (REMOTE §5.3), so a capture posted ahead of it lands on a slot the server is
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
//! **A refusal of the re-post is dropped and the channel reads on.** A server
//! that refuses it has swept the slot, restarted, or already handed the driver
//! an answer — all ordinary — and a foot that ended its channel over a capture
//! nobody was waiting for would be turning the ordinary case into the terminal
//! one. Only the wire swallowing it again keeps it held, because a wire failure
//! is not an answer: it says nothing about whether the slot is still there.

use crate::channel::Channel;
use crate::config::Local;
use crate::gestures;
use crate::invocation::{Capture, Invocation, Piece};

use super::Handoff;
use super::hold::{Ending, Failed, tell, tell_on};

/// **Run one invocation and post what it produced** — once, or as it goes.
///
/// `Ok` is the channel reading on: the capture landed, or the run was dropped.
/// `Err` is the gesture that ended the channel, and beside it the capture this
/// box is left holding, when it finished one ([`Held`]).
///
/// **To a server that reads a capture in pieces, the run is posted while it is
/// produced** (REMOTE §5.7): the executor is handed a post, every post of the
/// run rides one connection, and the last carries the capture with only the
/// text not yet posted. Three things follow from the server's answer to a
/// post:
///
/// - **refused** — nobody is listening, or the slot is gone. The executor
///   stops the child, the run is dropped and the channel reads on. A refusal
///   here is the asker's ordinary stop and never an ending;
/// - **the wire** — the child is stopped, because a run nobody can be told
///   about has no asker, and the channel is dialled again. Unless it was the
///   capture's own post: that one is held and posted first on the next dial,
///   exactly as it always was;
/// - **an answer no gesture can earn** — the channel is over, as at any leg.
///
/// **To an older server it is one post** and the receipt is read as before: a
/// server that refused the completion is saying the two ends disagree about
/// what is in flight, and that ends the channel.
pub(super) fn run(
    channel: &mut Channel,
    set: &[Local],
    handoff: Handoff,
    invocation: &Invocation,
) -> Result<(), (Failed, Option<Held>)> {
    if !channel.streams() {
        let capture = handoff(set, invocation, None);
        return tell(channel, &gestures::complete(&invocation.id, &capture))
            .map(|_| ())
            .map_err(|failed| (failed, Some(Held::of(invocation, capture))));
    }
    let mut line = None;
    let mut stopped = None;
    let mut post = |piece: Piece| {
        let partial = gestures::partial(&invocation.id, &piece);
        stopped = tell_on(channel, &mut line, &partial).err();
        stopped.is_none()
    };
    let capture = handoff(set, invocation, Some(&mut post));
    match stopped {
        Some(Failed::Refused(_)) => return Ok(()),
        Some(failed) => return Err((failed, None)),
        None => {}
    }
    match tell_on(
        channel,
        &mut line,
        &gestures::complete(&invocation.id, &capture),
    ) {
        Ok(_) | Err(Failed::Refused(_)) => Ok(()),
        Err(failed) => Err((failed, Some(Held::of(invocation, capture)))),
    }
}

/// **A capture the wire swallowed**, and the handle it answers.
///
/// The id is the server's own, quoted back exactly as an ordinary completion
/// quotes it — this is the same gesture on the same lane, sent one channel
/// late, and not a second spelling of anything.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Held {
    /// The server's handle on the invocation this box ran.
    pub(crate) id: String,
    /// What running it produced: computed once, and posted once.
    pub(crate) capture: Capture,
}

impl Held {
    /// What the foot is left holding when a completion fails on the wire.
    fn of(invocation: &Invocation, capture: Capture) -> Self {
        Self {
            id: invocation.id.clone(),
            capture,
        }
    }

    /// **Post it, ahead of the read that would release the lease.**
    ///
    /// `None` is the channel reading on, and it is the answer to three of the
    /// four outcomes: the capture landed, the server refused it, or the server
    /// answered something no foot gesture can earn. All three are the far end
    /// having spoken, and once it has spoken this capture has no further claim
    /// on anything — it is dropped here and never becomes an [`Ending::Over`].
    ///
    /// `Some` is the wire swallowing it a second time, which is not an answer
    /// at all: the ending carries the same capture to the next dial, so the
    /// rule holds unchanged across a flapping wire and this process still holds
    /// exactly one.
    pub(super) fn post(self, channel: &mut Channel) -> Option<Ending> {
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
    /// completion are the server refusing it and the server answering
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
