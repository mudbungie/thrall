//! **The advertised element** (yog's `docs/REMOTE.md` §5.1): the three facts a
//! tool host presents about one tool, and the one spelling of them.
//!
//! ```json
//! {"name": "Bash", "description": "run a command",
//!  "input_schema": {"type": "object",
//!                   "properties": {"command": {"type": "string"}}}}
//! ```
//!
//! - **`name` is a single path component.** It is the handle the far end's load
//!   act addresses one tool by, and a name carrying a separator is a name that
//!   addresses a filesystem.
//! - **`description` is one string**, this host's own words.
//! - **`input_schema` is the JSON Schema, verbatim.** Neither validated nor
//!   rewritten — it is this box's statement to a model, and narrowing it here
//!   would be thrall inventing a contract it does not own. It is also why the
//!   config document is JSON: any other syntax would make the operator
//!   transcribe a schema.
//!
//! **Nothing else.** No version, no enable flag, no per-workspace list — each
//! would be a fact stored on one side and checkable on neither.
//!
//! **One spelling, spent in both directions.** [`encode`] is what the
//! advertisement puts on the wire and [`of_one`] is what reads an element back;
//! the operator's document ([`config`](crate::config)) is read by the very same
//! decoder, because a presented element and a configured one spelled twice
//! drift within a week.

use serde_json::{Value, json};

use crate::json::str_of;

/// One advertised tool: the three facts, the schema as the value it
/// arrived as, and the optional fourth fact (REMOTE §5.4's worktree lane,
/// yog bl-77be / thrall bl-36f7): whether this box consents to run the
/// tool at a working directory the invocation names.
///
/// **[`Eq`] is written rather than derived**, because [`Value`] is not `Eq` —
/// it holds `f64`, whose `NaN` is the one value equality is not reflexive over.
/// A schema that came through a JSON decoder cannot hold one: the grammar has
/// no `NaN` literal and `serde_json` will not emit one. So equality here is
/// reflexive by construction, and saying so is what lets the whole surface keep
/// the `Eq` its round-trip is asserted with.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    /// The tool's name, a single path component.
    pub name: String,
    /// What it does, in this host's own words.
    pub description: String,
    /// Its JSON Schema, verbatim.
    pub input_schema: Value,
    /// Whether the operator consents to this tool executing at a working
    /// directory the invocation carries (`"subject_cwd": true` in the
    /// document; absent reads false). Advertised, because the engine
    /// routes the worktree lane on it; severable, because deleting the
    /// key deletes the capability.
    pub subject_cwd: bool,
}

impl Eq for Tool {}

/// A set as JSON — the array the advertisement carries.
pub fn encode(tools: &[Tool]) -> Value {
    Value::Array(tools.iter().map(one).collect())
}

/// One element, spelled once. `subject_cwd` rides only when true: absence
/// is the default and the wire stays minimal for the ordinary entry.
pub fn one(t: &Tool) -> Value {
    let mut o = json!({ "name": t.name, "description": t.description,
            "input_schema": t.input_schema });
    if let (true, Some(map)) = (t.subject_cwd, o.as_object_mut()) {
        map.insert("subject_cwd".to_owned(), Value::Bool(true));
    }
    o
}

/// One element, read back — [`one`]'s inverse, and the decoder the operator's
/// document spends on its own first three keys.
pub fn of_one(row: &Value) -> Result<Tool, String> {
    let o = row.as_object().ok_or("tool: not a JSON object")?;
    Ok(Tool {
        name: str_of(o, "name").map_err(|e| format!("tool: {e}"))?,
        description: str_of(o, "description").map_err(|e| format!("tool: {e}"))?,
        input_schema: o
            .get("input_schema")
            .cloned()
            .ok_or("tool: missing field \"input_schema\"")?,
        subject_cwd: match o.get("subject_cwd") {
            None => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return Err("tool: field \"subject_cwd\" is not a boolean".to_owned()),
        },
    })
}

/// Refuse a set that cannot be addressed (REMOTE §5.1): a name that is not a
/// single path component, or two elements wearing one name. Both name the
/// offending token — a decline an operator can act on.
///
/// **A collision across hosts is legal and ordinary** — two boxes both offering
/// `Bash` — and disambiguating them belongs to the act at the far end that
/// loads one. What cannot stand is a collision *inside* this box's set, because
/// then no name addresses one tool here.
pub fn validate(tools: &[Tool]) -> Result<(), String> {
    let mut seen = std::collections::BTreeSet::new();
    for tool in tools {
        if !is_component(&tool.name) {
            return Err(format!("unusable tool name {:?}", tool.name));
        }
        if !seen.insert(tool.name.clone()) {
            return Err(format!("duplicate tool name {:?}", tool.name));
        }
    }
    Ok(())
}

/// Whether `name` is a single path component: something that names one thing
/// and cannot walk anywhere.
fn is_component(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && !name.contains('\0')
}

#[cfg(test)]
mod tests;
