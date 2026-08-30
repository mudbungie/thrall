//! **Where this box keeps what the operator gave it.**
//!
//! One directory holds both of thrall's durable facts: the channel material
//! (`wire/workspaces/<leaf>/`) and the tool document (`tools.json`). Both are
//! **operator-provisioned and irreplaceable by anything thrall can do**, which
//! is why nothing thrall generates may ever be written beside them — a
//! regenerable subtree under the same root would make a rebuild a revocation.
//! thrall generates nothing today, and this is the note that says why it must
//! not start here.
//!
//! **Two variables, and no knob of thrall's own.** The XDG convention names the
//! directory, so a box that already places application data somewhere places
//! thrall's there too, and there is nothing to configure and nothing that can
//! disagree with it. A third variable — a `THRALL_HOME` — would be a second
//! authority for one fact, and the one thing it would buy (a scratch root for
//! tests) is already had by every function below the process edge taking the
//! root as an argument.
//!
//! **Neither variable set is a refusal, not a guess.** A relative fallback
//! would put an operator's certificates wherever the supervisor happened to
//! start the process, which is a place nobody chose and nobody can find again.

use std::ffi::OsString;
use std::path::PathBuf;

/// The directory thrall's own data lives in, under whichever root names it.
const HOME: &str = "thrall";
/// The XDG variable that names the data root outright.
const XDG: &str = "XDG_DATA_HOME";
/// The variable the convention's default is derived from.
const HOME_VAR: &str = "HOME";
/// The convention's default, relative to [`HOME_VAR`].
const DEFAULT: &str = ".local/share";

/// This box's data root, read from this process's own environment.
pub fn data_root() -> Result<PathBuf, String> {
    root_of(std::env::var_os(XDG), std::env::var_os(HOME_VAR))
}

/// [`data_root`]'s pure core — the environment as two values, so the rule can
/// be read and tested without a process to fold one into.
///
/// An empty variable is an unset one: a supervisor that exports a name with no
/// value has said nothing, and treating it as a root would name the filesystem
/// root's own `thrall` directory.
fn root_of(xdg: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, String> {
    if let Some(root) = stated(xdg) {
        return Ok(root.join(HOME));
    }
    if let Some(root) = stated(home) {
        return Ok(root.join(DEFAULT).join(HOME));
    }
    Err(format!(
        "this box's data root is not named: set {XDG}, or {HOME_VAR} so that \
         {DEFAULT}/{HOME} under it can be found. thrall will not guess — an \
         operator's certificates would land wherever this process happened to \
         be started."
    ))
}

/// One variable's value, or nothing when it said nothing.
fn stated(value: Option<OsString>) -> Option<PathBuf> {
    let held = value?;
    if held.is_empty() {
        return None;
    }
    Some(PathBuf::from(held))
}

#[cfg(test)]
mod tests;
