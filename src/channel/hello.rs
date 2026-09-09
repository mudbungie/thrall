//! **The version preface** (yog's `docs/REMOTE.md` §3, §3.2): *"each end
//! writes one frame ... before it reads the peer's."* Both write before either
//! reads, so neither waits on the other and there is no ordering rule to
//! remember. The frame carries two keys — `protocol`, the major, compared for
//! equality; and `edition`, what the writer can spell within it, discovered
//! and never compared.
//!
//! **This is why thrall exists as a separate program and it is not decoration.**
//! Until the four-component split one crate shipped both ends of every
//! connection and the wire could not skew. A foot is installed on a box the
//! engine's operator may not administer, upgraded on that box's own schedule —
//! so the day the two disagree about what a frame means is a day that will
//! arrive, and it must arrive as a sentence rather than as a gesture answered
//! wrongly.
//!
//! **thrall writes and confirms; it never admits.** A foot dials and is never
//! dialled, so there is exactly one half of the exchange here. The engine's
//! half — refusing a peer in band on the connection it opened — is the
//! server's, and a foot that carried it would be a foot that listens.
//!
//! **A mismatch is fail-closed and the refusal names both versions**, which is
//! REMOTE §3's requirement rather than a nicety: the sentence *is* the upgrade
//! prompt, so it must name a number an operator can act on. There is no
//! negotiation, no version list and no compat shim — negotiation is the
//! mechanism that makes every later version carry every earlier one's shape
//! forever, and the operator who installed both ends can upgrade the older one.
//!
//! **And a preface that ARRIVED is told apart from one that never did**
//! (bl-5d62). REMOTE §3 collapses them on the *engine's* side, and rightly: a
//! peer of the wrong version and a peer that hung up mid-preface are both peers
//! it cannot serve, so one sentence answers both. A foot is deciding a
//! different question — whether dialling again could help — and there the two
//! part. A frame this end read and could not agree with is a fact about the two
//! *binaries*: it will say the same thing on every dial until somebody installs
//! a new one, so it is [`Failure::Skew`] and it ends the channel. A preface that
//! never came is an engine restarting, a socket dying, a box waking up — the
//! wire, [`Failure::Wire`], and dialled again. The SENTENCE stays one sentence
//! either way; what differs is only what this foot does next.
//!
//! Under the old reading a foot that met a version bump wrote a line into its
//! operator's journal every minute forever, and one that met a restarting
//! engine was indistinguishable from it.

use std::io::{self, Read, Write};

use serde_json::{Value, json};

use super::{Failure, frame};

/// The engine's three numbers, vendored.
mod version;

pub use version::{EDITION, FLOOR, PROTOCOL};

/// The preface's first key: the major, compared for equality.
const KEY: &str = "protocol";

/// Its second key (REMOTE §3.2): what the peer can spell within that major.
/// Unlike [`KEY`] it is never compared — an edition is discovered, not agreed.
const EDITION_KEY: &str = "edition";

/// What a peer that stated no version is called in the sentence. An unversioned
/// build, a frame that is not an object, a frame without the key and a peer
/// that hung up mid-preface are one case on purpose: none of them can be
/// served, and four sentences for one outcome is four sentences.
const UNSTATED: &str = "no version";

/// Write this build's preface: the major, and the edition beside it. Called
/// before this end reads, which is what makes the exchange deadlock-free
/// without an ordering rule.
///
/// **Both ends write the edition** (REMOTE §3.2), so the key is stated even
/// though nothing at the far end has to read it — a peer that ignores it is
/// exercising rule 1, which is the same rule this end keeps on the way back.
pub fn state(w: &mut dyn Write) -> io::Result<()> {
    frame::write_value(w, &json!({ KEY: PROTOCOL, EDITION_KEY: EDITION }))
}

/// Read the engine's preface and refuse a mismatch — as the one `Err(String)`
/// every other thing that can go wrong with this transport already arrives as,
/// so nothing above here carries a case for it.
///
/// **What it answers is the engine's EDITION** (REMOTE §3.2): the fact that
/// says which post-floor fields that engine can spell. A foot reads no
/// post-floor field today — every path in its vendored ledger is at or under
/// the floor — so nothing above gates on the answer yet; it is decoded here
/// rather than ignored because the defaulting is the rule, and a rule nothing
/// returns is a rule nothing can test.
pub fn confirm(r: &mut dyn Read) -> Result<u32, Failure> {
    let Some(preface) = arrived(r) else {
        return Err(Failure::Wire(mismatch(None)));
    };
    let peer = preface.get(KEY).and_then(Value::as_u64);
    if peer == Some(u64::from(PROTOCOL)) {
        return Ok(edition_of(&preface));
    }
    Err(Failure::Skew(mismatch(peer)))
}

/// **The peer's edition, defaulted at the [`FLOOR`]** — for an absent key, and
/// for a value that is not an edition.
///
/// The absent case is REMOTE §3.2's rule; the unreadable case takes the same
/// answer rather than a refusal, because the fail-closed equality is the
/// MAJOR's and this key is not part of it. Reading a garbled edition as the
/// floor credits the peer with the least it can be spelling, which is the
/// direction that cannot invent a capability.
fn edition_of(preface: &Value) -> u32 {
    preface
        .get(EDITION_KEY)
        .and_then(Value::as_u64)
        .and_then(|stated| u32::try_from(stated).ok())
        .unwrap_or(FLOOR)
}

/// **The peer's preface frame, if one came at all.** `None` is the wire: a read
/// that failed, or a stream that ended where a frame belongs. Anything this
/// end managed to read is a frame the peer deliberately wrote, whatever it
/// says.
fn arrived(r: &mut dyn Read) -> Option<Value> {
    frame::read_value(r).ok().flatten()
}

/// The refusal: both versions, and what to do about it.
fn mismatch(peer: Option<u64>) -> String {
    let peer = peer.map_or_else(|| UNSTATED.to_owned(), |v| v.to_string());
    format!(
        "wire protocol mismatch: this foot speaks version {PROTOCOL}, \
         the engine speaks {peer}. There is no negotiation — \
         upgrade the older component until both speak one version."
    )
}

#[cfg(test)]
mod tests;
