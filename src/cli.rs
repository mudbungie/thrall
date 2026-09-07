//! The command line, as a pure function.
//!
//! `run` takes the arguments and hands back a [`Decided`] — either a
//! [`Verdict`] to say, or the one thing this binary does that cannot be a
//! sentence. It touches no process state: no argv, no environment, no streams,
//! no exit. That is the whole reason `src/main.rs` can be the one file excluded
//! from the coverage floor (`tarpaulin.toml`) without excluding any decision:
//! every decision is here, and every decision is a value a test can read back.
//!
//! **Serving is an outcome and not a `Verdict`, and that is what keeps this
//! file pure.** A verdict is text and an exit code; serving is dialling
//! engines, spawning children and blocking until every channel has stopped. So
//! the decision *"this argv means serve"* is made here and tested here, and the
//! doing of it is the entry point's — which is what that file is excluded for.

/// What one invocation decided: an exit code and the text that explains it.
///
/// The text goes to stdout on success and to stderr otherwise — the split is
/// the caller's, because the code already says which one it is and storing the
/// stream beside it would be the same fact twice.
pub struct Verdict {
    /// The process exit code. `0` is the only success.
    pub code: u8,
    /// Everything this run has to say, without a trailing newline.
    pub text: String,
}

/// The exit code for every refusal. One code, because thrall's refusals are
/// all the same kind of event — "that is not something this binary does" — and
/// a taxonomy of exit codes would be a promise to keep them stable.
const REFUSED: u8 = 2;

/// The exit code for a run that was understood and did not finish: no config,
/// no channel, or every channel stopped. One code for the same reason.
const FAILED: u8 = 1;

impl Verdict {
    /// A successful run and what it printed.
    pub fn ok(text: String) -> Self {
        Self { code: 0, text }
    }

    /// A refusal, from the sentence naming what was refused.
    ///
    /// The prefix and the usage are appended HERE rather than at each call
    /// site, so "a refusal always says what it refused *and* what the caller
    /// could have typed instead" is structural rather than remembered: a
    /// refusal added later cannot forget it. A bare non-zero exit teaches
    /// nobody anything.
    pub fn refused(what: String) -> Self {
        Self {
            code: REFUSED,
            text: format!("thrall: {what}\n\n{}", usage()),
        }
    }

    /// A run that did what it was asked and could not finish it.
    ///
    /// It carries **no usage**, and that is the difference from a refusal: a
    /// refusal is about what the caller typed, so the alternatives are the
    /// useful thing to say next; a failure is about this box or the far end,
    /// where a usage line is noise in front of the sentence that matters.
    pub fn failed(what: String) -> Self {
        Self {
            code: FAILED,
            text: format!("thrall: {what}"),
        }
    }
}

/// What one invocation decided to do.
pub enum Decided {
    /// Say this, and exit. Every flag and every refusal is one of these.
    Say(Verdict),
    /// Serve every channel this box holds, until they have all stopped. The
    /// entry point performs it, because it is the only thing here that is not
    /// a value.
    Serve,
    /// **Bridge one call to an MCP server on this box** (DESIGN §6.2): run
    /// `tool` against the server this argv names. The input arrives on this
    /// process's stdin, which is why the entry point performs it — reading a
    /// stream is not a value this file can hand back.
    Bridge {
        /// The tool to call, as the server names it.
        tool: String,
        /// The server's argv, whole.
        server: Vec<String>,
    },
}

/// The crate's name and version, as the `--version` line.
pub fn version() -> String {
    format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"))
}

/// The usage text. It states what thrall is before it states what to type,
/// because a foot arrives on a machine whose operator did not necessarily
/// choose to install it.
pub fn usage() -> String {
    format!(
        "{}

thrall is the foot: a tool-execution client for a yog server. It dials in,
advertises what this box offers, waits for work, and posts the captures back.
It never listens and it never speaks first.

usage: thrall run
       thrall mcp <tool> -- <server argv...>
       thrall [--version | --help]

  run             serve every channel this box is provisioned for: present
                  what it offers, wait for work, run it, post the captures
                  back. It does not return while a channel is up. A channel
                  that drops is dialled again, with a backoff that settles;
                  a channel that cannot be served at all is an exit naming
                  it, and restarting the process belongs to this machine's
                  own supervision.
  mcp <tool>      call one tool on an MCP server this box can spawn, as an
                  ordinary tool command: the invocation's input JSON on stdin,
                  the tool's content on stdout, the exit code the verdict. The
                  server runs for this one call and is torn down after it. It
                  is not a verb to type by hand — it is what a `command` in
                  tools.json names, so a bridged tool is a plain entry and this
                  box's document gains no key.
  -V, --version   print the name and version
  -h, --help      print this

What it reads, both provisioned by hand and never by thrall: the tool document
at <data root>/tools.json, and one channel per directory under <data
root>/wire/workspaces/. The data root is $XDG_DATA_HOME/thrall, or
$HOME/.local/share/thrall.

See docs/DESIGN.md for the role and the module map, and yog's docs/REMOTE.md
for the protocol thrall implements against.",
        version()
    )
}

/// Decide what one invocation does. `args` is argv **without** the program
/// name.
pub fn run(args: Vec<String>) -> Decided {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["run"] => Decided::Serve,
        ["mcp", tool, "--", server @ ..] if !server.is_empty() => Decided::Bridge {
            tool: (*tool).to_owned(),
            server: server.iter().map(|w| (*w).to_owned()).collect(),
        },
        ["mcp", ..] => Decided::Say(Verdict::refused(
            "mcp takes a tool name, then `--`, then the argv of a server this \
             box can spawn: thrall mcp fetch -- uvx mcp-server-fetch"
                .to_string(),
        )),
        ["--version" | "-V"] => Decided::Say(Verdict::ok(version())),
        ["--help" | "-h"] => Decided::Say(Verdict::ok(usage())),
        [] => Decided::Say(Verdict::refused(
            "nothing to do — `thrall run` is the verb".to_string(),
        )),
        other => Decided::Say(Verdict::refused(format!(
            "unrecognised argument: {}",
            other.join(" ")
        ))),
    }
}

#[cfg(test)]
mod tests;
