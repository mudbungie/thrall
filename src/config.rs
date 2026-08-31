//! **The operator's document** (yog's `docs/REMOTE.md` §5.2, §5.4; DESIGN
//! §3.4): what this box offers, and — by dropping half of it — what it says it
//! offers.
//!
//! ```json
//! [{"name": "Bash",
//!   "description": "run a command in a shell",
//!   "input_schema": {"type": "object",
//!                    "properties": {"command": {"type": "string"}},
//!                    "required": ["command"]},
//!   "command": ["/usr/local/libexec/thrall-tools/bash-tool"],
//!   "cwd": "/srv/work"}]
//! ```
//!
//! **One document, two readings.** The first three keys *are* REMOTE §5.1's
//! advertised element, verbatim, and [`advertisement`] is the whole of the
//! derivation: drop `command` and `cwd`. So what this box offers and what it
//! can actually run cannot drift, which is the entire reason the config is not
//! a pair of lists an operator has to keep in step.
//!
//! **Local config gates what is enabled, and this document is the gate.** A
//! tool absent from it is a tool this box does not have. Server-side
//! adjudication is unchanged, stacks on top, and fails closed — but it stacks
//! on *this*, so removing a capability from this machine is deleting an entry
//! here rather than editing anything, on the box whose operator is the one
//! entitled to decide (DESIGN §3.4).
//!
//! **`command` is an argv, spawned directly.** There is no shell and no
//! interpolation of the invocation's input into it. A shell would make the
//! declared `input_schema` advisory and turn an operator's config file into a
//! command-injection surface for anything a model can type; the input reaches
//! the command the way a local tool contract already delivers one — the JSON on
//! stdin, bytes on stdout, the exit code the verdict (bl-4cda).
//!
//! **It sits beside the channel material, not inside anything generated**
//! (`<thrall-data-root>/tools.json`, the sibling of `wire/`), for the material's
//! reason exactly: it describes *this machine*, it is written by the operator's
//! hand, and nothing thrall generates may sit where a rebuild would take it.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::json::{opt_str_of, strings_of};
use crate::tools::{self, Tool};

/// The document's leaf under the data root.
pub const TOOLS: &str = "tools.json";

/// One tool this box offers: the advertised half, and the local half that is
/// never presented to anyone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    /// The three facts this box presents (REMOTE §5.1).
    pub tool: Tool,
    /// The argv, spawned directly — never a shell line.
    pub command: Vec<String>,
    /// The working directory to run it in, when the operator named one.
    pub cwd: Option<PathBuf>,
}

/// This box's document, under a data root.
pub fn path(data_root: &Path) -> PathBuf {
    data_root.join(TOOLS)
}

/// Read it, or say why it is not one.
///
/// **An absent document is a refusal rather than the empty set.** A foot with
/// nothing to offer has nothing to do, and starting one is an explicit act that
/// deserves an explicit answer — the same posture absent channel material takes
/// (`channel::material`), and the opposite of the *presented* set, where an
/// empty array is a legitimate thing to say.
pub fn read(file: &Path) -> Result<Vec<Local>, String> {
    let text = std::fs::read_to_string(file)
        .map_err(|e| format!("{}: {e} — this box has no tool config", file.display()))?;
    let doc: Value = serde_json::from_str(&text).map_err(|e| format!("{}: {e}", file.display()))?;
    let rows = doc
        .as_array()
        .ok_or_else(|| format!("{}: not a JSON array", file.display()))?;
    let set: Vec<Local> = rows
        .iter()
        .map(one)
        .collect::<Result<_, String>>()
        .map_err(|e| format!("{}: {e}", file.display()))?;
    tools::validate(&advertisement(&set)).map_err(|e| format!("{}: {e}", file.display()))?;
    Ok(set)
}

/// One element: the advertised three read by the **same** decoder the wire
/// spends, then the local two.
///
/// **`cwd` must be absolute, and that is decided here** (bl-3c93). Whether a
/// path is absolute is a property of the document — static, knowable at the
/// read, and never changing afterwards — so it belongs with the empty argv and
/// the unusable name rather than at the first invocation. It is the refusal
/// `paths::root` already makes about the data root, in this operator's other
/// document: a relative path resolves against whatever directory the
/// supervisor happened to start this process in, which nobody wrote down and
/// which changes when the unit file does. Whether the directory *exists* is a
/// fact about the box rather than about the file, can change between the read
/// and the run, and is answered by the spawn (`exec::child`).
fn one(row: &Value) -> Result<Local, String> {
    let o = row.as_object().ok_or("tool: not a JSON object")?;
    let command = strings_of(o, "command").map_err(|e| format!("tool: {e}"))?;
    if command.is_empty() {
        return Err("tool: field \"command\" is an empty argv".to_owned());
    }
    let cwd = opt_str_of(o, "cwd").map_err(|e| format!("tool: {e}"))?;
    if let Some(cwd) = cwd.as_deref().filter(|cwd| !Path::new(cwd).is_absolute()) {
        return Err(format!(
            "tool: field \"cwd\" is {cwd:?}, which is not an absolute path — where \
             a tool runs must not depend on the directory this process was started \
             in, which is a place nobody chose and nobody can find again"
        ));
    }
    Ok(Local {
        tool: tools::of_one(row)?,
        command,
        cwd: cwd.map(PathBuf::from),
    })
}

/// **The advertisement, derived** (REMOTE §5.2): the same document with the
/// local half dropped. The one derivation, so this box cannot offer what it
/// cannot run.
pub fn advertisement(set: &[Local]) -> Vec<Tool> {
    set.iter().map(|local| local.tool.clone()).collect()
}

/// Which element an invocation names, by position — an index rather than a
/// borrow, so the caller resolves it against the very list it passed in.
pub fn position(set: &[Local], name: &str) -> Option<usize> {
    set.iter().position(|local| local.tool.name == name)
}

#[cfg(test)]
mod tests;
