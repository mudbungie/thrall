//! **The entries this box holds** (REMOTE §2, §8.2; DESIGN §3.3).
//!
//! An **entry** is a directory carrying the channel facts that reach one
//! engine: the host's anchors, this box's leaf and key for it, and the address.
//! It is the client's half of the pair a server-side registration is the other
//! half of — **possession, where registration is permission** — exactly as a
//! channel needs both a certificate and its issuer's trust.
//!
//! ```text
//! <thrall-data-root>/wire/workspaces/<leaf>/{ca.pem, client.pem, client.key, address}
//! ```
//!
//! **The path is REMOTE §8.2's, unchanged**, so a pair the operator minted for
//! a client box is filed the same way whichever program reads it. The directory
//! name is this box's own label for the channel and crosses no wire: nothing a
//! foot may say names a workspace, so there is nothing here to reconcile with
//! what the engine calls anything.
//!
//! **There is no flat root, and its absence is a simplification rather than an
//! omission.** Upstream a client box also holds material directly under `wire/`,
//! because that same directory is where a *server* keeps the address it binds.
//! A foot never binds anything (DESIGN §2), so that second meaning does not
//! exist here and one shape covers every case: every channel is an entry, and a
//! box with one engine has one entry.
//!
//! **Entries share nothing.** Not anchors — two engines are two operators'
//! trust roots — not leaves, since one certificate is one client identity, and
//! not addresses. So there is no inheritance and no path by which one entry can
//! be read through another; the structure below is a `readdir` and a read per
//! directory, and separation is the absence of a mechanism.
//!
//! **A refusal is one entry's, never the set's.** [`Entry::channel`] carries
//! its own `Result`, so a half-provisioned entry says so while every other
//! entry stands — a box serving three engines does not lose the two that are
//! fine.

use std::path::{Path, PathBuf};

use super::material::{self, Material, REMEDY};

/// The material directory's leaf under the data root.
pub const WIRE: &str = "wire";
/// The entries directory's leaf under [`WIRE`].
pub const ENTRIES: &str = "workspaces";

/// One channel this box holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The directory name — this box's own label for the channel.
    pub leaf: String,
    /// Its material, or the sentence saying why it has none.
    pub channel: Result<Material, String>,
}

impl Entry {
    /// **This entry, opened** — the one place an entry becomes a client of its
    /// engine. Its `Err` is the entry's own sentence where the material would
    /// not read, and the channel's where it read but will not open; both are
    /// one fact to every caller — *this channel cannot be dialled, and here is
    /// why* — so there is one function and not one per cause.
    pub fn open(&self) -> Result<super::Channel, String> {
        super::Channel::open(&self.channel.clone()?)
    }
}

/// Where entries live under a data root.
pub fn dir(data_root: &Path) -> PathBuf {
    data_root.join(WIRE).join(ENTRIES)
}

/// Every entry in `dir`, sorted by [`leaf`](Entry::leaf).
///
/// **A directory that will not read is zero entries, not a refusal.** Absent,
/// unreadable and empty are one fact — this box holds no channel — and that is
/// the shape of a foot that has been installed and not yet provisioned.
pub fn read_dir(dir: &Path) -> Vec<Entry> {
    let Ok(listing) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut held: Vec<Entry> = listing
        .flatten()
        .map(|found| found.path())
        // An entry *is* a directory. A stray file beside them names no intent
        // and is not an entry with a problem.
        .filter(|path| path.is_dir())
        .map(|path| entry(&path))
        .collect();
    held.sort_by(|a, b| a.leaf.cmp(&b.leaf));
    held
}

/// One directory read as the entry it claims to be.
fn entry(dir: &Path) -> Entry {
    let leaf = dir
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    let channel = match material::read_dir(dir) {
        Ok(Some(held)) => Ok(held),
        // Nothing provisioned is silence at the entries directory, where
        // absence means this box holds no channel. Here it is a refusal: a
        // directory somebody made names an intent, and an intent with no
        // material behind it is the half-provisioned failure one step earlier.
        Ok(None) => Err(format!("{} is an empty entry: {REMEDY}", dir.display())),
        Err(refusal) => Err(refusal),
    };
    Entry { leaf, channel }
}

#[cfg(test)]
mod tests;
