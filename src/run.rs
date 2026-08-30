//! **The loop** (yog's `docs/REMOTE.md` §5.3, §5.4): the whole of what a foot
//! does, for as long as it does anything.
//!
//! ```text
//! advertise → { invocations → hand off → complete } forever
//! ```
//!
//! **Nothing in it is thrall speaking unprompted.** Every leg is a reply to
//! something this end asked for: the advertisement is an act it sends, the work
//! arrives as the answer to a read it issued, and the capture goes back as
//! another act. The engine never speaks first (REMOTE §3), and the shape of
//! this file is that invariant rather than a consequence of it.
//!
//! **The advertisement is presented once per channel, not once per dial.** A
//! foot dials per ask (`channel`), so "on every connect" in the protocol's own
//! words means each time it takes up a channel: the far end writes the set only
//! when it differs from what is stored, so re-presenting costs nothing, while
//! re-presenting before *every* read would double the traffic to say a thing
//! that had not changed.
//!
//! **The loop is serial, and that is what makes a busy foot absent** (REMOTE
//! §5's presence amendment). It runs one invocation at a time and holds a
//! connection only while it is waiting, so the far end sees it vanish for as
//! long as a tool takes — which is why the engine's routing predicate is the
//! mailbox queue and never presence.
//!
//! **The execution itself is a hand-off, and it is not here** (bl-4cda). What
//! this file owns is the conversation; what runs a command owns the deadlines,
//! the containment and the transcode. Keeping the seam a parameter is what lets
//! the whole conversation be tested against an engine that is real and an
//! executor that is a line long.
//!
//! **It never reconnects.** A channel that fails is the sentence that failed
//! it, handed back. Restart policy belongs to the supervision the operator's
//! machine already has (DESIGN §2).

use serde_json::Value;

use crate::channel::Channel;
use crate::channel::entries::Entry;
use crate::config::{self, Local};
use crate::gestures::{self, Reply};
use crate::invocation::{Capture, Invocation};

/// **The hand-off**: what runs one invocation against this box's set.
///
/// A plain function pointer rather than a closure or a trait object, because
/// the thing being injected is *code*, never state — a foot that carried
/// per-invocation state between calls would be a foot with a world. It is
/// `Send` and `Copy` for free, which is what lets [`fan`] hand the same
/// executor to every channel without a lock or a clone.
pub type Handoff = fn(&[Local], &Invocation) -> Capture;

/// **One channel, served** — until it fails, and then the sentence that failed
/// it.
///
/// There is no success return and none is spelled: the only way out is a
/// gesture that did not land, so an `Ok` arm would be one no state of the world
/// can reach.
pub fn hold(channel: &Channel, set: &[Local], handoff: Handoff) -> String {
    let presenting = gestures::advertise(&config::advertisement(set));
    if let Err(reason) = tell(channel, &presenting) {
        return reason;
    }
    loop {
        let work = match waited(channel) {
            Ok(work) => work,
            Err(reason) => return reason,
        };
        for invocation in work {
            let capture = handoff(set, &invocation);
            if let Err(reason) = answer(channel, &invocation, &capture) {
                return reason;
            }
        }
    }
}

/// **Every channel this box holds, served at once** — one thread each, and the
/// sentence that stopped each one, in the order they were filed.
///
/// Serial stays serial **per channel**: two engines are two conversations, and
/// a foot that queued one behind the other would be absent from one of them for
/// reasons the other's operator cannot see. Channels share nothing here — not a
/// connection, not an identity, not a lock — exactly as their material shares
/// nothing (`channel::entries`).
///
/// An empty set of entries stops instantly with nothing to say, which is
/// correct and is not this function's sentence to write: the caller is the one
/// that knows where entries would have been filed.
pub fn fan(entries: Vec<Entry>, set: Vec<Local>, handoff: Handoff) -> Vec<String> {
    let running: Vec<(String, std::thread::JoinHandle<String>)> = entries
        .into_iter()
        .map(|entry| {
            let set = set.clone();
            let leaf = entry.leaf.clone();
            let running = std::thread::spawn(move || match entry.open() {
                Ok(channel) => hold(&channel, &set, handoff),
                Err(reason) => reason,
            });
            (leaf, running)
        })
        .collect();
    running
        .into_iter()
        .map(|(leaf, thread)| match thread.join() {
            Ok(said) => format!("{leaf}: {said}"),
            // Every path through `hold` answers a sentence, so a panic here is
            // this program breaking rather than a channel ending. It is named
            // as its own outcome instead of being swallowed into a transport
            // sentence, and it is never re-raised into the other channels: one
            // engine's conversation must not take the others down.
            Err(_) => format!("{leaf}: the channel ended by panicking"),
        })
        .collect()
}

/// **The follow-class read**: this machine's next work, or the empty answer of
/// a hold that ended quietly. Both are ordinary; only a channel failure is not.
fn waited(channel: &Channel) -> Result<Vec<Invocation>, String> {
    match tell(channel, &gestures::invocations())? {
        Reply::Invocations(rows) => Ok(rows),
        other => Err(format!(
            "the engine answered {other:?}, not this machine's work"
        )),
    }
}

/// Post one capture back.
///
/// **The receipt is read rather than discarded.** An engine that refused the
/// completion — an expired handle, a slot addressed to another machine — is
/// saying that this foot and that engine disagree about what is in flight, and
/// a foot that kept answering into that would be posting captures nobody is
/// waiting for.
fn answer(channel: &Channel, invocation: &Invocation, capture: &Capture) -> Result<(), String> {
    tell(channel, &gestures::complete(&invocation.id, capture)).map(|_| ())
}

/// One gesture over the channel, read back with the one decoder — so this foot
/// speaks exactly what the boundary speaks and can add nothing to it.
///
/// **The last frame is the answer.** Today every one of the three gestures is
/// answered in one frame; when a read's answer is several, the newest is the
/// one that stands (REMOTE §3: the streaming form is not a second form).
fn tell(channel: &Channel, request: &Value) -> Result<Reply, String> {
    let stream = channel.ask(request)?;
    let last = stream
        .last()
        .ok_or_else(|| "the engine ended the stream without answering".to_owned())?;
    gestures::decode(last).unwrap_or_else(Err)
}

#[cfg(test)]
mod tests;
