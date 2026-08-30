//! **The three gestures a foot may send, and the answers it may receive**
//! (yog's `docs/REMOTE.md` §4.2, §5.1, §5.3).
//!
//! REMOTE §4.2 enumerates the foot set outright — *"`advertise`, `invocations`
//! and `complete`. No other `Query`, no other `Action`"* — and this file is
//! that enumeration in code. A foot cannot ask *about the world* and cannot act
//! *on the world*; what it may do is answer for the machine it is. **Note which
//! of the routing leg's four verbs is absent: `invoke`, the asking side's. A
//! foot is invoked; it never invokes.**
//!
//! So the enumeration is the enforcement thrall can keep. The real enforcement
//! is the engine's — it is the party that can be trusted to hold it — but a
//! foot with no spelling for a fourth verb cannot send one by accident, and
//! adding one here would be a visible act rather than a slip.
//!
//! **None of the three names a client, and that is the gesture** (REMOTE §5.1).
//! The identity a set lands under, the queue a read drains and the invocation a
//! completion answers are all the *intake's* — the connection's certificate
//! common name. A `client` field on the wire would let any connection overwrite
//! or drain any other's, which is the authorization the certificate has already
//! decided.
//!
//! **The wire adds nothing to the boundary** (REMOTE §3), so these are ordinary
//! gestures in the ordinary envelope: `op` the discriminant, every parameter a
//! named field. There is no wire-only verb, no envelope and no field here that
//! a seat could not type.

use serde_json::{Value, json};

use crate::invocation::{Capture, Invocation, capture_of, capture_value, invocation_of};
use crate::json::str_of;
use crate::tools::{self, Tool};

/// **Present what this box offers** (REMOTE §5.1): one field, `tools`, an array
/// whose element is exactly three facts.
pub fn advertise(set: &[Tool]) -> Value {
    json!({ "op": "advertise", "tools": tools::encode(set) })
}

/// **Wait for this machine's next work** (REMOTE §5.3): the follow-class read,
/// and where a foot spends nearly all of its life. It carries no field at all —
/// a connection drains its own queue.
pub fn invocations() -> Value {
    json!({ "op": "invocations" })
}

/// **Answer exactly one invocation** (REMOTE §5.3), quoting the handle it was
/// minted under and carrying the capture in its one spelling.
pub fn complete(invocation: &str, capture: &Capture) -> Value {
    json!({ "op": "complete", "invocation": invocation,
            "capture": capture_value(capture) })
}

/// What an engine can answer a foot. Three kinds, because there are three
/// gestures — plus the refusal every gesture can earn.
///
/// A refusal is not an error *here*: the engine spoke, and what it said is that
/// it will not do the thing. Whether that ends the loop is the loop's decision
/// (see [`run`](crate::run)), not the decoder's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    /// The advertisement landed.
    Advertised,
    /// This machine's work, which is ordinarily empty: the engine holds the
    /// read for its own bound and then answers with what it has.
    Invocations(Vec<Invocation>),
    /// One invocation's standing after the call — the handle, and the capture
    /// once there is one. It is a **re-read**, never an echo, which is why a
    /// foot reads its own completion's receipt rather than discarding it.
    Routed {
        invocation: String,
        capture: Option<Capture>,
    },
}

/// Read one reply frame: the outer `Err` is bytes this decoder cannot read at
/// all, and the inner `Err` is the refusal the envelope faithfully carried.
///
/// **The refusal is the envelope with no `kind`.** `ok` cannot be the
/// discriminant — a reply may faithfully report something that failed — so a
/// body carrying a `kind` is an answer, and a body carrying none must be
/// `{"ok": false, "error": …}`.
///
/// **A kind this foot has no use for is an error and not a shrug.** The three
/// above are the only answers the three gestures can earn; anything else means
/// the engine answered a question nobody here asked, and continuing on it would
/// be guessing.
pub fn decode(v: &Value) -> Result<Result<Reply, String>, String> {
    let o = v.as_object().ok_or("reply: not a JSON object")?;
    let Some(kind) = o.get("kind") else {
        return refusal_of(o).map(Err);
    };
    let kind = kind.as_str().ok_or("reply: non-string field \"kind\"")?;
    match kind {
        "advertised" => Ok(Reply::Advertised),
        "invocations" => rows(o).map(Reply::Invocations),
        "routed" => routed(o),
        other => Err(format!("reply: unusable kind {other:?}")),
    }
    .map(Ok)
}

/// The kind-less envelope: a refusal, and nothing else may wear that shape.
fn refusal_of(o: &serde_json::Map<String, Value>) -> Result<String, String> {
    match o.get("ok") {
        Some(Value::Bool(false)) => str_of(o, "error").map_err(|e| format!("reply: {e}")),
        _ => Err("reply: an answer with no kind".to_owned()),
    }
}

/// The follow-class read's rows.
fn rows(o: &serde_json::Map<String, Value>) -> Result<Vec<Invocation>, String> {
    o.get("rows")
        .and_then(Value::as_array)
        .ok_or("reply: invocations with no rows")?
        .iter()
        .map(invocation_of)
        .collect()
}

/// One invocation's standing. `capture` is **absent** rather than empty while
/// the work is still out, so a reader never has to tell "not finished" from
/// "finished saying nothing".
fn routed(o: &serde_json::Map<String, Value>) -> Result<Reply, String> {
    Ok(Reply::Routed {
        invocation: str_of(o, "invocation").map_err(|e| format!("reply: {e}"))?,
        capture: match o.get("capture") {
            None | Some(Value::Null) => None,
            Some(held) => Some(capture_of(held)?),
        },
    })
}

#[cfg(test)]
mod tests;
