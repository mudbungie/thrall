//! **What the operator carried to this box**, and what its absence means
//! (yog's `docs/REMOTE.md` §1.4, §8.2; DESIGN §3.3).
//!
//! **thrall mints nothing.** The certificate and its key are issued by the
//! operator's own CA, on the box that holds it, and carried here by hand. So
//! this module only ever *reads*, and there is no bootstrap flow to secure: a
//! foot that could provision itself over the wire would be a foot any wire
//! could provision.
//!
//! Three answers, and they are the whole of the trust bootstrap on this side:
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
//!   key for this channel, and the one address it dials.
//!
//! **The four files are REMOTE §8.2's, unchanged.** An operator who provisioned
//! an entry for a yog client has provisioned one for a thrall; the names are
//! the wire's, not this crate's, and inventing a fifth or renaming one would
//! make the operator's act depend on which program was installed.

use std::path::{Path, PathBuf};

/// The operator CA this end verifies the engine against — one anchor set, and
/// the same one the engine verifies this end with.
pub const ANCHORS: &str = "ca.pem";
/// This box's certificate chain for this channel. Its subject common name
/// **is** this client's identity (REMOTE §2), and its organizational unit is
/// the grade ([`leaf`](super::leaf)).
pub const CHAIN: &str = "client.pem";
/// This box's private key for that chain.
pub const KEY: &str = "client.key";
/// The `host:port` this channel dials. One address per relationship and no
/// flag: two spellings of one address is the drift REMOTE §8 removed.
pub const ADDRESS: &str = "address";

/// What a refusal names as the remedy. It is an act on **another** box and by
/// another hand, which is the whole of REMOTE §1.4 said where an operator will
/// read it — never a target this binary could be asked to run.
pub const REMEDY: &str = "the pair is minted by the operator on the box that issued it and carried \
     here by hand; thrall mints nothing";

/// One channel's provisioned material.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Material {
    /// The operator CA, PEM.
    pub anchors: PathBuf,
    /// This end's certificate chain, PEM.
    pub chain: PathBuf,
    /// This end's private key, PEM.
    pub key: PathBuf,
    /// `host:port` — the engine this channel dials.
    pub address: String,
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
    // A file that will not read yields no address, and no address is the same
    // refusal an empty one earns: one branch, because "unreadable" and "empty"
    // are one fact about what this box can be told to dial.
    let address = std::fs::read_to_string(dir.join(ADDRESS))
        .unwrap_or_default()
        .trim()
        .to_owned();
    if address.is_empty() {
        return Err(format!(
            "{} names no address; it must hold one host:port — {REMEDY}",
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

#[cfg(test)]
mod tests;
