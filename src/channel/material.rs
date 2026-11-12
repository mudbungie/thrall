//! **What an entry holds**, and what its absence means (yog's
//! `docs/REMOTE.md` §8.2, §15; DESIGN §3.3).
//!
//! **This module only ever reads.** An entry arrives one of two ways and
//! neither is here: `thrall register` claims an operator's offer with a key
//! this box makes and files what the server answers (`crate::register`), or
//! the operator mints the pair on the box that holds the CA and carries it
//! over by hand (`yog wire-certs WIRE_LEAF`, for a box with no route to the
//! registration door). Either way the result is the same four files.
//!
//! Three answers:
//!
//! - **Nothing provisioned** — `Ok(None)`. This directory holds no channel.
//!   Removing it deletes config, not code, which is why absence is an answer
//!   and not an error.
//! - **Partly provisioned** — `Err`, naming every missing file at once. Half a
//!   trust store is a misconfiguration, and one that silently degraded to *no
//!   encryption* is the failure mode mTLS exists to exclude. Every missing file
//!   at once because a remedy that reveals one gap per run is a remedy run four
//!   times.
//! - **Provisioned** — `Ok(Some(Material))`: the anchors, this box's leaf and
//!   key for this channel, and the endpoints it dials.
//!
//! **The four files are REMOTE §8.2's, unchanged.** An operator who provisioned
//! an entry for a yog client has provisioned one for a thrall; the names are
//! the wire's, not this crate's, and inventing a fifth or renaming one would
//! make the operator's act depend on which program was installed. **Nothing
//! else in the directory is read**: an entry filed before REMOTE §15 may still
//! hold `rendezvous.pub` and `pairing.salt`, and it works with them ignored.

use std::path::{Path, PathBuf};

/// The operator CA this end verifies the server against — one anchor set, and
/// the same one the server verifies this end with.
pub const ANCHORS: &str = "ca.pem";
/// This box's certificate chain for this channel. Its subject common name
/// **is** this client's identity (REMOTE §2), and its organizational unit is
/// the grade ([`leaf`](super::leaf)).
pub const CHAIN: &str = "client.pem";
/// This box's private key for that chain.
pub const KEY: &str = "client.key";
/// The endpoints this channel dials: one `host:port` per line, in dial order
/// (REMOTE §15.3). A file of one line is a list of one. No flag restates it:
/// two spellings of one address is the drift REMOTE §8 removed. **It is also
/// the server NAME's one home**: the mTLS verifies the host of whichever
/// endpoint answered.
pub const ADDRESS: &str = "address";

/// What a refusal names as the remedy: the two ways an entry is made, neither
/// of which is repairing one by hand.
pub const REMEDY: &str = "an entry is filed whole, by `thrall register <url>` from an operator's offer, \
     or minted by the operator on the box that holds the CA and carried here";

/// One channel's provisioned material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    /// The operator CA, PEM.
    pub anchors: PathBuf,
    /// This end's certificate chain, PEM.
    pub chain: PathBuf,
    /// This end's private key, PEM.
    pub key: PathBuf,
    /// The endpoints this channel dials, `host:port` each, in dial order.
    /// Never empty.
    pub address: Vec<String>,
}

/// Read one directory as the channel it claims to be. See the module doc for
/// the three answers.
pub fn read_dir(dir: &Path) -> Result<Option<Material>, String> {
    let wanted = [ANCHORS, CHAIN, KEY, ADDRESS];
    let missing: Vec<&str> = wanted
        .iter()
        .copied()
        .filter(|f| !dir.join(f).is_file())
        .collect();
    if missing.len() == wanted.len() {
        return Ok(None);
    }
    if !missing.is_empty() {
        return Err(format!(
            "{} is half-provisioned: missing {} — {REMEDY}",
            dir.display(),
            missing.join(", ")
        ));
    }
    // A file that will not read yields no endpoint, and no endpoint is the
    // same refusal an empty one earns: one branch, because "unreadable" and
    // "empty" are one fact about what this box can be told to dial.
    let stated = std::fs::read_to_string(dir.join(ADDRESS)).unwrap_or_default();
    let address = endpoints(&stated);
    if address.is_empty() {
        return Err(format!(
            "{} names no address; it must hold one host:port per line — {REMEDY}",
            dir.join(ADDRESS).display()
        ));
    }
    Ok(Some(Material {
        anchors: dir.join(ANCHORS),
        chain: dir.join(CHAIN),
        key: dir.join(KEY),
        address,
    }))
}

/// The endpoints a file's text states: every line that says something,
/// trimmed, in order.
fn endpoints(stated: &str) -> Vec<String> {
    let said = stated.lines().map(str::trim).filter(|l| !l.is_empty());
    said.map(str::to_owned).collect()
}

#[cfg(test)]
mod tests;
