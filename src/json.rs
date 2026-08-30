//! **Strict field reads**, shared by everything that decodes JSON here.
//!
//! The wire and the operator's document are both **instructions**, never
//! observations: a missing field, a mistyped value and a value of the wrong
//! shape each refuse with the offending key rather than defaulting. That is
//! yog's `docs/REMOTE.md` §3 discipline — *"an unknown `op`, a missing field, a
//! mistyped value each refuse with a reason"* — held at the one place fields
//! are read, so no caller carries a forgiving branch.
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
