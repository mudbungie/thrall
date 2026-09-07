//! **The stdio transport, and the three requests a bridge makes** (DESIGN
//! §6.2): `initialize`, `tools/call` and `tools/list`, hand-read in the strict
//! style [`json`](crate::json) establishes.
//!
//! **Transport is stdio and only stdio.** One JSON-RPC message per line, down
//! the server's stdin and back up its stdout; its stderr is inherited, so
//! whatever it says about itself lands in the capture's stderr beside the
//! tool's own, with nothing here to plumb. A remote or HTTP server — and any
//! OAuth flow — is refused until a named deployment needs one, and when it
//! does it is still a server argv on this box: a local proxy the operator
//! installs, which does not move this seam (DESIGN §6.2, §6.8).
//!
//! **Anything that is not the answer to the request in flight is skipped**,
//! and that includes a line that is not JSON at all (widened by bl-b6ab, on
//! evidence). A server may log; it may ask this client for something it
//! declared no capability for; and its own CHILD may write to the stdout it
//! inherited, which is this transport. The last was measured rather than
//! imagined: `mcp-server-fetch` bootstraps a node helper on a box's first
//! fetch, and that helper's package-manager warnings arrive here as three
//! lines of prose in the middle of the conversation — once per box, so a
//! refusal there is a flake an operator meets exactly once and can do nothing
//! about.
//!
//! Refusing them was a special case sitting inside the general rule rather
//! than a second rule: *read until the answer arrives, and what is not it is
//! not this end's to read*. Nothing is lost by widening it, because a program
//! that speaks no MCP at all still ends without answering, and that sentence
//! is the one an operator needs. The line itself is not carried anywhere — it
//! was never a message, and a server's own words belong on the stderr it
//! inherits, which reaches the capture already.
//!
//! What ends an invocation, then, is the far end going quiet or answering with
//! an error.
//!
//! **The write's failure is not an outcome, and that is a simplification
//! rather than an omission.** A server whose stdin will not take a byte is a
//! server that has died, and a dead server's stdout is at end of file — so the
//! read that follows says the same thing with better words, at the one place
//! this file has to say it. Two error stories for one event is two sentences
//! an operator has to learn.
//!
//! **The teardown is a [`Drop`] and not a step**, so it cannot be forgotten by
//! a path that returned early: every `?` above tears the server down on its
//! way out.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

/// The MCP revision this client states at `initialize`.
const PROTOCOL: &str = "2025-06-18";

/// How long a server gets to notice its stdin closed and leave on its own.
/// An MCP server's own shutdown is end of file (the specification's stdio
/// teardown), so this is the grace for taking it.
const GRACE: Duration = Duration::from_millis(500);

/// How often a leaving server is looked at. `exec::child`'s number, for
/// `exec::child`'s reason: a latency knob on the answer, never on the run.
const POLL: Duration = Duration::from_millis(20);

/// One MCP server, for as long as one invocation lasts.
pub(super) struct Server {
    child: Child,
    /// The server's stdin. Taken at the teardown — closing it is how an MCP
    /// server is asked to leave.
    input: Option<ChildStdin>,
    output: BufReader<ChildStdout>,
    /// The first word of the argv, and the only word of it any sentence here
    /// may carry (DESIGN §6.7).
    head: String,
    /// The last id spent. Ids are this client's own and monotonic, so a reply
    /// can be told from a log line by nothing but its id.
    spent: u64,
}

impl Server {
    /// Spawn it through the spawn boundary — its own process group, the git
    /// environment scrubbed — with stdin and stdout piped and stderr
    /// inherited.
    pub(super) fn start(argv: &[String]) -> Result<Self, String> {
        let (head, rest) = argv.split_first().ok_or("the MCP server argv is empty")?;
        let mut cmd = crate::spawn::command(head);
        cmd.args(rest)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());
        let mut child = crate::spawn::spawn(&mut cmd)
            .map_err(|e| format!("the MCP server {head:?} would not start ({e})"))?;
        let input = child
            .stdin
            .take()
            .ok_or("the MCP server was given no stdin")?;
        let output = child
            .stdout
            .take()
            .ok_or("the MCP server was given no stdout")?;
        Ok(Self {
            child,
            input: Some(input),
            output: BufReader::new(output),
            head: head.clone(),
            spent: 0,
        })
    }

    /// The handshake: state the protocol and who is asking, then say the
    /// conversation has begun.
    pub(super) fn initialize(&mut self) -> Result<(), String> {
        let hello = json!({"protocolVersion": PROTOCOL, "capabilities": {},
                "clientInfo": {"name": "thrall", "version": env!("CARGO_PKG_VERSION")}});
        self.request("initialize", &hello)?;
        self.send(&json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        Ok(())
    }

    /// The catalog, as the server's own `tools` array — the pin's one request
    /// (DESIGN §6.3), and the only one this file makes that no invocation
    /// does.
    pub(super) fn list(&mut self) -> Result<Value, String> {
        self.request("tools/list", &json!({}))
    }

    /// **The one call an invocation makes.** The arguments are the input off
    /// stdin, verbatim: this end narrows nothing, because the schema the model
    /// was shown is the server's own.
    pub(super) fn call(&mut self, tool: &str, arguments: &Value) -> Result<Value, String> {
        self.request("tools/call", &json!({"name": tool, "arguments": arguments}))
    }

    /// One request, and the result it earns.
    fn request(&mut self, method: &str, params: &Value) -> Result<Value, String> {
        self.spent += 1;
        let id = self.spent;
        self.send(&json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        self.reply(id, method)
    }

    /// One message down. See the module comment for why nothing is returned:
    /// the read that follows is where a server that stopped listening is
    /// noticed, and it says so better.
    fn send(&mut self, message: &Value) {
        if let Some(pipe) = self.input.as_mut() {
            let _ = writeln!(pipe, "{message}");
            let _ = pipe.flush();
        }
    }

    /// Read until the answer to `id` arrives, skipping everything that is not
    /// it.
    fn reply(&mut self, id: u64, stage: &str) -> Result<Value, String> {
        loop {
            let mut line = String::new();
            if self.output.read_line(&mut line).unwrap_or(0) == 0 {
                return Err(self.said(stage, "ended before answering"));
            }
            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            if message.get("id").and_then(Value::as_u64) != Some(id) {
                continue;
            }
            return match message.get("result") {
                Some(result) => Ok(result.clone()),
                None => Err(self.refusal(stage, &message)),
            };
        }
    }

    /// A sentence about the server, naming the stage it was at.
    fn said(&self, stage: &str, what: &str) -> String {
        format!("the MCP server {:?} {what} {stage}", self.head)
    }

    /// The server's own refusal, in the server's own words when it sent any.
    fn refusal(&self, stage: &str, message: &Value) -> String {
        let said = message
            .pointer("/error/message")
            .and_then(Value::as_str)
            .unwrap_or("it sent neither a result nor an error");
        format!("{}: {said}", self.said(stage, "refused"))
    }
}

impl Drop for Server {
    /// **Ask, wait, insist** — `exec::child`'s cascade, one register down, and
    /// the subject is the group because the spawn boundary made the server the
    /// leader of one. Closing stdin is the ask an MCP server understands; the
    /// kill is unconditional so a helper the server started cannot outlive the
    /// invocation that started it, which is the whole of DESIGN §6.4.
    fn drop(&mut self) {
        self.input = None;
        // Read before the wait below can reap it away.
        let group = self.child.id();
        let until = Instant::now() + GRACE;
        while Instant::now() < until && !matches!(self.child.try_wait(), Ok(Some(_))) {
            std::thread::sleep(POLL);
        }
        crate::sys::kill_group(group);
        let _ = self.child.wait();
    }
}
