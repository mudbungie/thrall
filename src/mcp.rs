//! **The bridge verb** (DESIGN §6.2, bl-e104): `thrall mcp <tool> -- <server
//! argv...>`, an ordinary tool command whose program is this binary.
//!
//! MCP, for this file: a JSON-RPC protocol in which a **server** — a process
//! fronting some resource, spawned on this box — publishes a catalog of tools,
//! each a name, a description and a JSON Schema, and a **client** calls them.
//! That catalog is exactly the shape of an advertised element (REMOTE §5.1),
//! which is the whole reason the bridge is cheap. thrall is the client; yog
//! and litany learn no verb, no field and no transport (DESIGN §6.1).
//!
//! **The document gains no key.** A pinned MCP tool is a plain entry whose
//! `command` begins with this binary:
//!
//! ```json
//! {"name": "fetch", "description": "…", "input_schema": {…},
//!  "command": ["/usr/local/bin/thrall", "mcp", "fetch", "--",
//!              "uvx", "mcp-server-fetch"]}
//! ```
//!
//! so `config::read`, the advertisement projection and [`exec`](crate::exec)
//! are untouched, and deleting the entry deletes the capability (DESIGN §3.4).
//! The bridge honours the same contract every other entry does — the input
//! JSON on stdin, the capture on stdout, the exit code the verdict — because
//! it is being spawned by the same executor for the same reason.
//!
//! **Every outcome is a capture**, for [`exec`](crate::exec)'s reason exactly:
//! a server that would not start, a handshake that never came, a JSON-RPC
//! error and a tool that answered `isError` are all three facts and a
//! sentence, never four kinds of failure. The far end reads a refusal as it
//! reads a tool that failed, which is what it is.
//!
//! **A sentence never quotes the server's argv past its first word** (DESIGN
//! §6.7). A credential lives in the server's own argv or in the wrapper script
//! the operator wrote, and a spawn failure loves to quote the command line —
//! so the only word of it that ever reaches a sentence is the program's name,
//! which is the one word that identifies the server without carrying its
//! secrets.

use serde_json::Value;

use crate::invocation::Capture;

/// Content parts to capture bytes (DESIGN §6.5).
mod render;
/// The stdio transport and the requests a bridge makes.
mod rpc;

/// **One invocation of a bridged tool.** `input` is the invocation's `input`
/// JSON, verbatim off this process's stdin; the answer is the three facts the
/// executor at the far end will post back.
///
/// The server's lifetime is contained in this call's (DESIGN §6.4): it is
/// spawned here, handed exactly one `tools/call`, and torn down before this
/// function returns — no resident server, no reconnect, and nothing that
/// outlives the invocation to be state.
pub fn bridge(tool: &str, server: &[String], input: &str) -> Capture {
    match called(tool, server, input) {
        Ok(capture) => capture,
        Err(reason) => crate::exec::refused(FAILED, &reason),
    }
}

/// The verdict every bridge failure earns. One code, because they are one kind
/// of event to the model reading the capture: the tool did not answer.
const FAILED: i32 = 1;

/// The whole conversation, as a value or a sentence.
fn called(tool: &str, server: &[String], input: &str) -> Result<Capture, String> {
    let arguments: Value =
        serde_json::from_str(input).map_err(|e| format!("the input on stdin is not JSON ({e})"))?;
    if !arguments.is_object() {
        return Err(
            "the input on stdin is not a JSON object — a tool's input is \
                    the object the model produced, whole"
                .to_owned(),
        );
    }
    let mut server = rpc::Server::start(server)?;
    server.initialize()?;
    let result = server.call(tool, &arguments)?;
    render::capture(&result)
}

#[cfg(test)]
mod tests;
