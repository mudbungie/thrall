//! **The read, in its two forms** (yog's `docs/REMOTE.md` §5.3, §5.8): what a
//! channel is handed, as the frames it arrives in.
//!
//! Both forms answer the same item — a frame's work, `None` for the
//! supersession that ends the channel, or the failure that ends this read — so
//! the loop above has one body for both. The difference is only when the
//! connection is let go:
//!
//! - **serial** (`holding` absent): the whole answer is read to its terminator
//!   and the connection dropped *before* any of it is run, exactly as before —
//!   a serial foot is absent while it works, and an old server's busy and
//!   lease rules are built on that. The last frame is the answer, as it always
//!   was ([`tell`](super::tell)).
//! - **pushing** (`holding` stated): the connection stays open and each frame
//!   is handed up as it arrives, one job each; the read ends only when the
//!   socket does — the server going away, or four heartbeat intervals with no
//!   frame (the channel's read bound) — or at the supersession.

use super::{Failed, decoded, said};
use crate::channel::Channel;
use crate::gestures::{self, Reply};
use crate::invocation::Invocation;

/// One frame of a read: its work, possibly none — a heartbeat, or a quiet
/// serial hold — or `None`, the read a newer activation ended (REMOTE §5.1).
type Frame = Result<Option<Vec<Invocation>>, Failed>;

/// **Open the read and hand back its frames.** `holding` is the whole switch:
/// `Some` is a pushing read stating it, `None` the serial read.
pub(super) fn frames(
    channel: &mut Channel,
    holding: Option<Vec<String>>,
) -> Box<dyn Iterator<Item = Frame>> {
    let read = gestures::invocations(holding.as_deref());
    if holding.is_none() {
        return Box::new(std::iter::once(said(channel.ask(&read)).and_then(work)));
    }
    match channel.listen(&read) {
        Ok(mut line) => Box::new(
            std::iter::from_fn(move || line.frame().transpose())
                .map(|frame| decoded(&frame?).and_then(work)),
        ),
        Err(failure) => Box::new(std::iter::once(Err(failure.into()))),
    }
}

/// **What one frame of the read says.** Work — none, on a heartbeat — the
/// supersession, or an answer this read cannot earn.
fn work(reply: Reply) -> Frame {
    match reply {
        Reply::Invocations(rows) => Ok(Some(rows)),
        Reply::Superseded => Ok(None),
        other => Err(Failed::Unusable(format!(
            "the server answered {other:?}, not this machine's work"
        ))),
    }
}
