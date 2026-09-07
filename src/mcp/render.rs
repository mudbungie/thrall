//! **Content parts to a capture** (DESIGN §6.5): a capture is text, because a
//! tool result is a model's message (REMOTE §5.3).
//!
//! Four rules, and each answers one shape MCP allows:
//!
//! - **text parts, in order**, each on its own line;
//! - **a part that is not text** — an image, an audio clip, an embedded or
//!   linked resource — is replaced by one line naming its type, its mime type
//!   and the bytes it carried. The bytes are dropped and the line says so: an
//!   image a vision model could read is a later ask with its own ball, and a
//!   capture that silently lost something is worse than one that did not carry
//!   it;
//! - **`structuredContent` as the JSON it is**, when the result carried no
//!   text part at all — a server that sent both sent the same fact twice, and
//!   the text half is the half a model was meant to read;
//! - **`isError: true` writes the content and exits 1.** The content is still
//!   the answer; the code is what says the tool failed, which is the same
//!   in-band shape every other refusal on this leg takes.
//!
//! **A result that said nothing is a refusal.** A server answering with
//! neither content nor structured content has not answered, and a capture of
//! empty bytes and a zero exit code would be the silent success README warns
//! an operator about — the one failure a tool contract must never produce.

use serde_json::Value;

use crate::invocation::Capture;

/// The verdict a tool that answered `isError` earns.
const ERRORED: i32 = 1;

/// One `tools/call` result, as the three facts.
///
/// `stderr` is empty and stays empty: the server's own stderr is inherited
/// (`rpc`), so anything it had to say is already in this process's, which is
/// the capture's at the far end.
pub(super) fn capture(result: &Value) -> Result<Capture, String> {
    let o = result
        .as_object()
        .ok_or("the MCP server's result is not a JSON object")?;
    let parts: &[Value] = match o.get("content") {
        Some(Value::Array(parts)) => parts,
        Some(_) => return Err("the MCP server's \"content\" is not an array".to_owned()),
        None => &[],
    };
    let mut stdout = String::new();
    let text = parts.iter().any(is_text);
    let errored = o.get("isError") == Some(&Value::Bool(true));
    for part in parts {
        stdout.push_str(&rendered(part));
        stdout.push('\n');
    }
    if let (false, Some(structured)) = (text, o.get("structuredContent")) {
        stdout.push_str(&structured.to_string());
        stdout.push('\n');
    }
    if stdout.is_empty() {
        return Err("the MCP server's result carried neither content nor \
                    structuredContent — it did not answer"
            .to_owned());
    }
    Ok(Capture {
        stdout,
        stderr: String::new(),
        exit_code: if errored { ERRORED } else { 0 },
    })
}

/// Whether a part is a text part — the one question that decides whether
/// `structuredContent` is the answer or a second copy of it.
fn is_text(part: &Value) -> bool {
    part.get("type").and_then(Value::as_str) == Some("text")
}

/// One part as its line: its text, or the sentence naming what was not
/// carried.
fn rendered(part: &Value) -> String {
    if is_text(part) {
        return part
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
    }
    let kind = part.get("type").and_then(Value::as_str).unwrap_or("part");
    let mime = part
        .get("mimeType")
        .or(part.pointer("/resource/mimeType"))
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    format!("[{kind} {mime}, {} bytes dropped]", carried(part))
}

/// What the part carried, in bytes: a base64 payload (`data`, or an embedded
/// resource's `blob`), an embedded resource's text as it stands, and nothing
/// at all for a part that carries only a link.
fn carried(part: &Value) -> usize {
    let data = part
        .get("data")
        .or(part.pointer("/resource/blob"))
        .and_then(Value::as_str);
    match data {
        Some(base64) => decoded(base64),
        None => part
            .pointer("/resource/text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .len(),
    }
}

/// How many bytes a base64 string stands for, without decoding it: four
/// characters are three bytes, less one for each `=` of padding. Counted
/// rather than decoded because the bytes are not wanted — only their number
/// is, and a decoder would be a dependency for a sentence.
fn decoded(base64: &str) -> usize {
    let padding = base64.chars().rev().take_while(|c| *c == '=').count();
    (base64.len() / 4 * 3).saturating_sub(padding)
}
