//! **The operator verb** (DESIGN §6.3): `thrall mcp pin -- <server argv...>`
//! runs a server once, asks for its catalog, and prints every tool as a
//! complete entry for this box's document.
//!
//! **Discovery is an operator act, once.** There is no live catalog, no
//! `list_changed` receiver and no auto-advertise: what this box offers changes
//! when the operator reconfigures the box, which is REMOTE §5's rate-of-change
//! ruling. A server whose schema moved surfaces as its own error on the next
//! call, and the remedy is a re-pin.
//!
//! **It writes nothing.** The document is operator-authored (DESIGN §3.4), and
//! **the allowlist IS the paste**: the operator copies in the entries they
//! vouch for and no others. Never a server's whole catalog — an unpinned tool
//! costs nothing and a pinned one costs its schema in every prompt of every
//! role that loads it.
//!
//! **Annotations are commentary and never advertised.** A server may hint at a
//! tool's reach (`readOnlyHint`, `destructiveHint`, `idempotentHint`,
//! `openWorldHint`); those hints go to stderr, for the operator writing the
//! engine-side policy row (DESIGN §6.6), and none of them enters the entry.
//! REMOTE §5.1 admits nothing yog stores and cannot check, and a hint from an
//! untrusted server is exactly that.
//!
//! **Nothing here refuses a name.** A name that is not a single path
//! component, or that collides with an entry already in the document, is
//! refused where every such name is refused today — `config::read`, at the
//! next `thrall run` — so the operator sees one answer to that question rather
//! than two that could disagree. Any prefixing is the operator's edit.

use std::path::Path;

use serde_json::{Value, json};

use super::rpc;
use crate::invocation::Capture;
use crate::json::str_of;

/// The one line of stderr that is not a tool's: what the rest of it is for.
const COMMENTARY: &str = "thrall: what follows is each tool's own hints, as the server stated \
                          them — for the engine-side policy row, and never advertised.\n";

/// **The catalog, as entries to paste.** `exe` is the absolute path this box
/// will spawn — `current_exe`, so a pin run from a build tree prints that
/// tree's path, which is correct and visible.
pub fn entries(server: &[String], exe: &Path) -> Capture {
    match pinned(server, exe) {
        Ok(capture) => capture,
        Err(reason) => crate::exec::refused(super::FAILED, &reason),
    }
}

/// The whole act, as a value or a sentence.
fn pinned(server: &[String], exe: &Path) -> Result<Capture, String> {
    // No tool has been named yet — the operator typed the argv, so the
    // program is the name they would recognise. An argv with nothing in it has
    // no server to name and is refused by the start below.
    let subject = server.first().map_or("", String::as_str);
    let mut mcp = rpc::Server::start(subject, server)?;
    mcp.initialize()?;
    let catalog = mcp.list()?;
    let listed = catalog
        .get("tools")
        .and_then(Value::as_array)
        .ok_or("the MCP server's catalog carries no \"tools\" array")?;
    let mut document = Vec::new();
    let mut said = COMMENTARY.to_owned();
    for tool in listed {
        let o = tool
            .as_object()
            .ok_or("the MCP server's catalog holds something that is not a tool")?;
        let name = str_of(o, "name").map_err(|e| format!("the MCP server's catalog: {e}"))?;
        document.push(entry(&name, o, exe, server));
        if let Some(hints) = o.get("annotations") {
            said.push_str(&name);
            said.push(' ');
            said.push_str(&hints.to_string());
            said.push('\n');
        }
    }
    Ok(Capture {
        stdout: format!(
            "{}\n",
            serde_json::to_string_pretty(&Value::Array(document)).unwrap_or_default()
        ),
        stderr: said,
        exit_code: 0,
    })
}

/// One string the server may have left out, as the empty string when it did.
fn said(tool: &serde_json::Map<String, Value>, key: &str) -> String {
    match tool.get(key) {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// One tool as one entry: the three advertised facts the server stated, and
/// the local half this box will spawn — which is this binary, the verb, the
/// tool's own name, and the server's argv behind the `--`.
///
/// **A description or a schema the server left out is not a refusal.** Both
/// are the server's statement to a model and both are visible in the paste, so
/// a gap is the operator's to fill before they vouch for the entry; an empty
/// object is what MCP's own arguments are when a tool declares none.
fn entry(
    name: &str,
    tool: &serde_json::Map<String, Value>,
    exe: &Path,
    server: &[String],
) -> Value {
    let mut command = vec![
        Value::String(exe.display().to_string()),
        Value::String("mcp".to_owned()),
        Value::String(name.to_owned()),
        Value::String("--".to_owned()),
    ];
    command.extend(server.iter().map(|word| Value::String(word.clone())));
    json!({
        "name": name,
        "description": said(tool, "description"),
        "input_schema": tool.get("inputSchema").cloned().unwrap_or(json!({"type": "object"})),
        "command": Value::Array(command),
    })
}
