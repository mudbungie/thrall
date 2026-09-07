//! **The version preface** (yog's `docs/REMOTE.md` §3): *"each end writes one
//! frame, `{"protocol": <integer>}`, before it reads the peer's."* Both write
//! before either reads, so neither waits on the other and there is no ordering
//! rule to remember.
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

/// The protocol this build speaks.
///
/// **A new verb is not a bump.** A `Query`, an `Action` or a reply kind the
/// peer has not heard of already refuses in band, naming it (REMOTE §3's strict
/// decode) — the boundary correcting itself, not two protocols meeting. The
/// integer moves when the *existing* shape changes meaning: the framing, the
/// envelope, or what a spelling already in use is taken to say.
///
/// **The number is the engine's and there is nothing here to version.** It must
/// read whatever yog's own `src/wire/hello/version.rs` reads, so it is stated a second
/// time — as a literal copied from that file — in `crate::corpus`, which is
/// what the suite's stand-in engine dials at. The two agreeing is a test
/// (`corpus::tests`) rather than a tautology, and that is the whole of the
/// defence: while the stand-in wrote its preface from *this* constant, both
/// ends of every test agreed by construction and the pin sat five versions
/// behind a live engine with the suite green (bl-e0f0).
///
/// **Most bumps are not a foot's business, and it can still not dial across a
/// single one of them.** Of everything past 2, exactly one touched this crate's
/// own surface — 8 (yog bl-66d4), the required `wrote` on `reply/advertised`.
/// 3 through 7 and 9 through 18 moved seat-facing shapes this crate never
/// decodes, 10 moved no field at all (`attention` became follow-class), and 17
/// and 18 each moved a VALUE rather than a field — a new `signals` word, then a
/// fourth `framing` word — that the shape ledger cannot see. The
/// preface is one integer compared for equality, so the version states the
/// engine's *build* and never which frames this end happens to read — which is
/// why this constant has now drifted seven times (bl-e0f0, bl-f88f, bl-dc5f,
/// bl-605f, bl-44b4, bl-272d, bl-dc8a) with the suite
/// green: nothing on this side of the socket can see the far end move, and a
/// real dial is the only thing that meets the skew. bl-dc8a is the first moved
/// while the engine's number was still UNPUBLISHED, which is the order yog
/// `docs/REMOTE.md` §3 asks for: a consumer's `main` carries the number, then
/// the engine publishes, then the consumer does.
pub const PROTOCOL: u32 = 18;

/// The preface's one key, and the whole of its shape.
const KEY: &str = "protocol";

/// What a peer that stated no version is called in the sentence. An unversioned
/// build, a frame that is not an object, a frame without the key and a peer
/// that hung up mid-preface are one case on purpose: none of them can be
/// served, and four sentences for one outcome is four sentences.
const UNSTATED: &str = "no version";

/// Write this build's preface. Called before this end reads, which is what
/// makes the exchange deadlock-free without an ordering rule.
pub fn state(w: &mut dyn Write) -> io::Result<()> {
    frame::write_value(w, &json!({ KEY: PROTOCOL }))
}

/// Read the engine's preface and refuse a mismatch — as the one `Err(String)`
/// every other thing that can go wrong with this transport already arrives as,
/// so nothing above here carries a case for it.
pub fn confirm(r: &mut dyn Read) -> Result<(), Failure> {
    let Some(preface) = arrived(r) else {
        return Err(Failure::Wire(mismatch(None)));
    };
    let peer = preface.get(KEY).and_then(Value::as_u64);
    if peer == Some(u64::from(PROTOCOL)) {
        return Ok(());
    }
    Err(Failure::Skew(mismatch(peer)))
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
