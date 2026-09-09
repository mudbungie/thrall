//! **Strict field reads**, shared by everything that decodes JSON here.
//!
//! The wire and the operator's document are both **instructions**, never
//! observations: a missing field, a mistyped value and a value of the wrong
//! shape each refuse with the offending key rather than defaulting. That is
//! yog's `docs/REMOTE.md` §3 discipline — *"an unknown `op`, a missing field, a
//! mistyped value each refuse with a reason"* — held at the one place fields
//! are read, so no caller carries a forgiving branch.
//!
//! **And strict is not the same as frozen: readers here GROW** (yog's
//! `docs/REMOTE.md` §3.2). Since bl-e598 the wire's integer is a *major* and an
//! addition ships without one, stamped an edition in the corpus ledger instead
//! — so a reader must take a frame from an engine of another edition without
//! refusing it. Four rules, and this file is where the next field meets them:
//!
//! 1. **An unknown key is ignored.** Structural here and nothing to keep: every
//!    read below indexes by key and nothing enumerates an object.
//! 2. **A key stamped ABOVE the ledger's floor reads as its default when
//!    absent, and the default is the fact before the field existed.** No such
//!    key exists in a shape a foot decodes — every path it vendors is at or
//!    under the floor, which `corpus::ledger::tests` holds — so [`bool_of`] and
//!    [`str_of`] are required for the reason stated on them and stay that way.
//!    The FIRST post-floor field a foot reads is the one that must not use
//!    them: it gets a reader of its own, named for the fact it defaults to,
//!    and the projection replay (`corpus::replay`) is what goes red until it
//!    does.
//! 3. **An unknown word maps to a named catch-all**, carrying the word and
//!    rendered honestly — never to the nearest known word, and never to a
//!    refusal of the row it sits in. Nothing here decodes a vocabulary today:
//!    the wire's only closed word set a foot reads is the reply `kind`, which
//!    is rule 4's exception. A field that arrives spelling one is decoded to an
//!    enum with an `Unknown(String)` arm, not matched to a `_ => Err(…)`.
//! 4. **The reply `kind` stays strict** (`gestures::decode`): a reader asks
//!    only what it paints, so a kind it has never heard of means an ask started
//!    being answered differently — which is a major, not an addition.
//!
//! **Why a hand codec and not a derive.** `serde_json` is linked for the
//! grammar; `serde`'s derive is not on the approved dependency set, and the
//! surface here is small and closed enough that the refusals are worth writing
//! by hand: a derive's error text names a Rust field, and an operator editing a
//! JSON document by hand needs the key they typed.

use serde_json::{Map, Value};

/// One string field.
pub fn str_of(o: &Map<String, Value>, key: &str) -> Result<String, String> {
    match o.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(format!("field {key:?} is not a string")),
        None => Err(format!("missing field {key:?}")),
    }
}

/// One string field the operator may leave out. Absent is an answer; present
/// and mistyped is not.
pub fn opt_str_of(o: &Map<String, Value>, key: &str) -> Result<Option<String>, String> {
    match o.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("field {key:?} is not a string")),
    }
}

/// One boolean field, required.
///
/// Required rather than defaulted, because the fields that wear this shape are
/// *readings* — `wrote` on the advertisement's receipt says whether the far end
/// changed anything (REMOTE §5.1) — and a reading defaulted to `false` is a
/// foot inventing the reassuring answer for an engine that said nothing. An
/// optional flag the operator may leave out is a different thing and is read
/// where it is declared (`tools::of_one`'s `subject_cwd`).
pub fn bool_of(o: &Map<String, Value>, key: &str) -> Result<bool, String> {
    match o.get(key) {
        Some(Value::Bool(b)) => Ok(*b),
        Some(_) => Err(format!("field {key:?} is not a boolean")),
        None => Err(format!("missing field {key:?}")),
    }
}

/// One array-of-strings field. An element that is not a string refuses the
/// whole field: an argv with a number in it is not an argv with a gap.
pub fn strings_of(o: &Map<String, Value>, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = o.get(key) else {
        return Err(format!("missing field {key:?}"));
    };
    let rows = value
        .as_array()
        .ok_or_else(|| format!("field {key:?} is not an array"))?;
    rows.iter()
        .map(|row| match row {
            Value::String(s) => Ok(s.clone()),
            _ => Err(format!(
                "field {key:?} holds something that is not a string"
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests;
