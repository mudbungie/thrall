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
//! **It costs an idle foot nothing**, which is what makes it the right place:
//! a foot with no work sends no extra gesture, because there is no hand-off to
//! end. Re-presenting before *every* read would double the traffic of a foot
//! that is protected by the engine's guard the whole time it is waiting.
//!
//! **And since PROTOCOL 8 it also buys knowing** (yog bl-66d4). The receipt
//! carries `wrote`: false when the engine compared, true when it changed the
//! document. So a re-assertion that WROTE is this machine being told it was
//! disarmed while it was absent — the set it presented was not the set in
//! force — and that is said out loud rather than healed in silence, which was
//! the whole complaint bl-2d78 could not close from this end.
//!
//! **A true on the FIRST presentation of a channel says nothing** and is not
//! reported: every fresh channel presents into whatever the engine holds, and
//! the ordinary first one writes. Only a re-assertion — a presentation this
//! foot made after a hand-off it just performed — can distinguish a rival from
//! a beginning.
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

use std::sync::Arc;

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

/// **Where a foot says something while it is still serving.**
///
/// Every other thing this file has to say is the sentence a channel ends with,
/// returned as a value the caller renders. A disarming is the one fact that
/// must be said *without* ending anything — the foot has already healed it and
/// carries on — so it needs somewhere to go that is not a return.
///
/// It is injected for [`Handoff`]'s reason, one register down: what a running
/// foot writes to is stderr, which is an effect no test can read back, so the
/// effect lives in `src/main.rs` (the one coverage exclusion, and the file that
/// already owns serving) and every test reads the notices back as values. It is
/// `Arc<dyn Fn>` rather than a bare `fn` because a recording sink has to
/// capture where it records, and shared across [`fan`]'s threads because a box
/// with several engines has one place to say things.
pub type Notice = Arc<dyn Fn(&str) + Send + Sync>;

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

/// **One channel, served** — until it fails, and then the sentence that failed
/// it.
///
/// There is no success return and none is spelled: the only way out is a
/// gesture that did not land, so an `Ok` arm would be one no state of the world
/// can reach.
pub fn hold(channel: &Channel, set: &[Local], handoff: Handoff, notice: &Notice) -> String {
    let presenting = gestures::advertise(&config::advertisement(set));
    // The first presentation's reading is discarded on purpose: a fresh channel
    // writes whenever the engine held something else, and there is no rival in
    // that. Only a re-assertion below can mean one.
    if let Err(reason) = present(channel, &presenting) {
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
            match present(channel, &presenting) {
                Err(reason) => return reason,
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
/// **A refusal ends the channel**, on the terms every other gesture's does: the
/// engine declines a set that would replace a serving machine's own, so a
/// refusal here is another connection holding this machine's read with a
/// different set in force — two processes claiming one name, which is the
/// engine's sentence to say and this foot's to stop on.
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
fn present(channel: &Channel, presenting: &Value) -> Result<bool, String> {
    match tell(channel, presenting)? {
        Reply::Advertised { wrote } => Ok(wrote),
        other => Err(format!(
            "the engine answered {other:?}, not that the advertisement landed"
        )),
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
pub fn fan(entries: Vec<Entry>, set: Vec<Local>, handoff: Handoff, notice: &Notice) -> Vec<String> {
    let running: Vec<(String, std::thread::JoinHandle<String>)> = entries
        .into_iter()
        .map(|entry| {
            let set = set.clone();
            let leaf = entry.leaf.clone();
            let under = named(notice, &leaf);
            let running = std::thread::spawn(move || match entry.open() {
                Ok(channel) => hold(&channel, &set, handoff, &under),
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

/// One channel's notices, under the name that channel was filed as — the same
/// prefix its ending sentence carries. A box holding two engines that was told
/// it had been disarmed, and not by which one, has been told nothing it can
/// act on.
fn named(notice: &Notice, leaf: &str) -> Notice {
    let notice = Arc::clone(notice);
    let leaf = leaf.to_owned();
    Arc::new(move |line: &str| notice(&format!("{leaf}: {line}")))
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
