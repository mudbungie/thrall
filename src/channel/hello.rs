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

use std::io::{self, Read, Write};

use serde_json::json;

use super::frame;

/// The protocol this build speaks.
///
/// **A new verb is not a bump.** A `Query`, an `Action` or a reply kind the
/// peer has not heard of already refuses in band, naming it (REMOTE §3's strict
/// decode) — the boundary correcting itself, not two protocols meeting. The
/// integer moves when the *existing* shape changes meaning: the framing, the
/// envelope, or what a spelling already in use is taken to say.
pub const PROTOCOL: u32 = 2;

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
pub fn confirm(r: &mut dyn Read) -> Result<(), String> {
    let peer = stated(r);
    if peer == Some(u64::from(PROTOCOL)) {
        return Ok(());
    }
    Err(mismatch(peer))
}

/// The version the peer stated, or `None` when it stated none.
fn stated(r: &mut dyn Read) -> Option<u64> {
    frame::read_value(r).ok().flatten()?.get(KEY)?.as_u64()
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
