//! **What crosses the routing leg** (yog's `docs/REMOTE.md` §5.3): the work a
//! foot is handed, and the answer it hands back.
//!
//! Two nouns and one spelling each. Both are the wire's rather than this
//! crate's, so they are read strictly — a missing field refuses with the key,
//! because an invocation is an *instruction* about what to run on this machine
//! and a guessed one would run the wrong thing.
//!
//! **An invocation carries no client.** The read that delivers it is answered
//! to one identity — the connection's certificate common name — so a foot being
//! told its own name would be a fact it already holds (REMOTE §5.3: *"the
//! identity is the intake's"*).
//!
//! **A capture is text, and the transcode happens once.** A capture ends as a
//! model's tool result and a model's message is text, so the executor
//! transcodes its child's bytes at the one place bytes stop being bytes
//! (bl-4cda) and nothing here carries an encoding case.

use serde_json::{Value, json};

use crate::json::str_of;

/// **What this machine is handed**: the engine's handle on one call, and the
/// two facts needed to run it.
///
/// [`Eq`] is written rather than derived for [`Value`]'s reason
/// ([`Tool`](crate::tools::Tool)): the JSON grammar has no `NaN` literal, so an
/// input that came through a decoder cannot hold the one value equality is not
/// reflexive over.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    /// The engine's handle, minted at the post and quoted by the completion.
    pub id: String,
    /// The advertised name, as this box spells it.
    pub tool: String,
    /// The `tool_use.input` JSON, verbatim.
    pub input: Value,
    /// The subject's location, when the invocation carries one (REMOTE §5's
    /// worktree lane, bl-36f7): the working directory to execute at,
    /// honoured only for an entry the operator marked `subject_cwd`.
    pub cwd: Option<String>,
}

impl Eq for Invocation {}

/// **What came back**: bytes on stdout, bytes on stderr, the exit code the
/// verdict — a local tool contract, one for one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Capture {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// One queued invocation as JSON — the follow-class read's row, in the one
/// spelling.
pub fn invocation_value(invocation: &Invocation) -> Value {
    let mut o = json!({ "invocation": invocation.id, "tool": invocation.tool,
            "input": invocation.input });
    if let (Some(cwd), Some(map)) = (&invocation.cwd, o.as_object_mut()) {
        map.insert("cwd".to_owned(), Value::String(cwd.clone()));
    }
    o
}

/// [`invocation_value`]'s inverse, strict.
pub fn invocation_of(v: &Value) -> Result<Invocation, String> {
    let o = v.as_object().ok_or("invocation: not a JSON object")?;
    Ok(Invocation {
        id: str_of(o, "invocation").map_err(|e| format!("invocation: {e}"))?,
        tool: str_of(o, "tool").map_err(|e| format!("invocation: {e}"))?,
        input: o
            .get("input")
            .cloned()
            .ok_or("invocation: missing field \"input\"")?,
        cwd: match o.get("cwd") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return Err("invocation: field \"cwd\" is not a string".to_owned()),
        },
    })
}

/// A capture as JSON — the **one** spelling, spent by the completing act and by
/// the reply that quotes it back.
pub fn capture_value(capture: &Capture) -> Value {
    json!({ "stdout": capture.stdout, "stderr": capture.stderr,
            "exit_code": capture.exit_code })
}

/// [`capture_value`]'s inverse, on the same strict terms.
pub fn capture_of(v: &Value) -> Result<Capture, String> {
    let o = v.as_object().ok_or("capture: not a JSON object")?;
    Ok(Capture {
        stdout: str_of(o, "stdout").map_err(|e| format!("capture: {e}"))?,
        stderr: str_of(o, "stderr").map_err(|e| format!("capture: {e}"))?,
        exit_code: exit_of(o.get("exit_code"))?,
    })
}

/// An exit code narrowed to what a process can actually have exited with.
fn exit_of(field: Option<&Value>) -> Result<i32, String> {
    let code = field
        .and_then(Value::as_i64)
        .ok_or("capture: missing or non-integer field \"exit_code\"")?;
    i32::try_from(code).map_err(|_| format!("capture: exit_code {code} out of range"))
}

#[cfg(test)]
mod tests;
