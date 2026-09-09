//! **The conformance replay** (yog's `docs/REMOTE.md` §3.2): one vendored
//! record, replayed at every edition an engine can be at, and read under a word
//! no build has heard of.
//!
//! §3.2 replaced the bump for the additive class, and what a consumer owes in
//! exchange is that its readers **grow**: an unknown key is ignored, an absent
//! post-floor key reads as its default, an unknown word maps to a named
//! catch-all, and only the reply `kind` stays strict. Three of the four are
//! statements about frames that do not exist yet — a field yog has not added, a
//! word it has not coined — so the way to hold them is to MAKE those frames out
//! of the frames there are:
//!
//! - **[`project`]** deletes from a frame every key stamped above an edition,
//!   which is exactly the frame an engine of that edition would have written.
//!   The stamps are the history, so one record replays to any point of it and
//!   no archive of old fixtures is kept (or goes stale).
//! - **[`reword`]** replaces one string with a token no build has heard of,
//!   which is exactly the frame an engine that coined a word would send. Free
//!   text passes trivially; a vocabulary passes only through its catch-all.
//!
//! **What the replay can prove today, and what it cannot.** Every path this
//! foot reads is stamped at or below the floor (`ledger::tests` holds that),
//! so the projection deletes nothing and the replay is an identity — which
//! would be a beat that never fails. Its other direction is the one that bites:
//! [`tests`] projects a FIXTURE signature in which `wrote` arrived after the
//! floor, and asserts the key is gone and the decoder refuses it — the red the
//! day yog adds a post-floor field to a foot shape, and the reason this is a
//! replay rather than a re-read.

use serde_json::Value;

use super::ledger::{self, FLOOR};
use super::{ADVERTISED, QUEUED, REFUSAL, ROUTED};

/// **The four answers a foot can earn**, each with the frames vendored for it.
/// The three requests are round-tripped rather than replayed (`corpus::tests`):
/// a request is what this end WRITES, and an edition cannot delete a key this
/// end put there.
pub(crate) const REPLIES: [(&str, &[&str]); 4] = [
    ("reply/advertised", &ADVERTISED),
    ("reply/invocations", &QUEUED),
    ("reply/routed", &ROUTED),
    ("reply/refusal", &[REFUSAL]),
];

/// **The editions an engine of this major can be at**: the floor, up to the
/// newest this build has vendored. Never empty — the floor is always in it,
/// because §3.2 defines the floor as the edition the major was cut at and no
/// engine speaking this major is below it.
///
/// Below the floor is not an older engine, it is another major, and the preface
/// refuses that by equality before a frame is decoded.
pub(crate) fn editions() -> Vec<u32> {
    (FLOOR..=FLOOR.max(ledger::edition())).collect()
}

/// **One frame as an engine of `edition` would have written it**: every key
/// stamped above that edition deleted, wherever the path occurs.
pub(crate) fn project(frame: &Value, signature: &[(&'static str, u32)], edition: u32) -> Value {
    let mut projected = frame.clone();
    for &(entry, since) in signature {
        if since > edition {
            drop_at(&mut projected, &segments(ledger::path_of(entry)));
        }
    }
    projected
}

/// **One frame with the string at `path` replaced**, wherever the path occurs.
/// `None` is a path no frame of this shape carries — which the caller reports
/// as a stale vendor rather than passing over, because a mutation that mutated
/// nothing is a beat that proves nothing.
pub(crate) fn reword(frame: &Value, path: &str, word: &str) -> Option<Value> {
    let mut reworded = frame.clone();
    set_at(&mut reworded, &segments(path), word).then_some(reworded)
}

/// A ledger path as the steps of a walk: `/rows/[]/tool` is three steps, and
/// the frame itself is none.
fn segments(path: &str) -> Vec<&str> {
    path.split('/').filter(|step| !step.is_empty()).collect()
}

/// Delete the key at `steps`, in every array element the path passes through.
fn drop_at(v: &mut Value, steps: &[&str]) {
    let Some((head, rest)) = steps.split_first() else {
        return;
    };
    if *head == "[]" {
        if let Value::Array(rows) = v {
            for row in rows {
                drop_at(row, rest);
            }
        }
        return;
    }
    let Value::Object(o) = v else {
        return;
    };
    if rest.is_empty() {
        o.remove(*head);
        return;
    }
    if let Some(next) = o.get_mut(*head) {
        drop_at(next, rest);
    }
}

/// Replace the string at `steps`; `true` if it was set anywhere.
fn set_at(v: &mut Value, steps: &[&str], word: &str) -> bool {
    let Some((head, rest)) = steps.split_first() else {
        return false;
    };
    if *head == "[]" {
        let Value::Array(rows) = v else {
            return false;
        };
        // Every element, never the first one that answers: `any` would stop
        // at the first row it set, and a row left unmutated is a frame the
        // replay would then judge as if it had been.
        let mut set = false;
        for row in rows {
            set |= set_at(row, rest, word);
        }
        return set;
    }
    let Value::Object(o) = v else {
        return false;
    };
    if rest.is_empty() {
        return match o.get_mut(*head) {
            Some(slot) if slot.is_string() => {
                *slot = Value::String(word.to_owned());
                true
            }
            _ => false,
        };
    }
    o.get_mut(*head)
        .is_some_and(|next| set_at(next, rest, word))
}

mod tests;
