//! **What the door answered, and the entry it becomes** (REMOTE §15.1 step
//! 2; §8.2 for the four files).
//!
//! The reply is one JSON object: the workspace to file the entry under, the
//! name and grade the offer was minted with, the list of endpoints to dial,
//! the operator CA and the leaf just signed. With the key this box made, that
//! is a whole entry — the same four files a hand-carried pair is.
//!
//! **An entry directory is made once and never overwritten.** The directory
//! is created by the one call that fails when it already exists, so two
//! claims racing for one workspace cannot both file, and a box that already
//! holds a channel under that name keeps it.

use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path};

use rustls::pki_types::CertificateDer;
use rustls::pki_types::pem::PemObject;
use serde_json::Value;

use crate::channel::material::{ADDRESS, ANCHORS, CHAIN, KEY};
use crate::json::{str_of, strings_of};

/// The mode a private key is written with: its owner's and nobody else's.
const PRIVATE: u32 = 0o600;
/// The mode of everything else in an entry.
const PUBLIC: u32 = 0o644;

/// One answered claim.
#[derive(Debug)]
pub(super) struct Claim {
    /// What the entry is filed under on this box.
    pub(super) workspace: String,
    /// The client name the leaf was signed for.
    pub(super) name: String,
    /// The grade the offer was minted at.
    pub(super) grade: String,
    /// The endpoints to dial, in order. Never empty.
    pub(super) address: Vec<String>,
    /// The operator CA, PEM.
    ca: String,
    /// The leaf, PEM.
    cert: String,
    /// The operator CA's first certificate, DER — the second half of what
    /// the confirmation word is hashed over.
    pub(super) anchor: Vec<u8>,
}

/// Read a `200`'s body as the claim it states, refusing one that could not
/// become an entry: a field missing, no endpoint, no CA certificate, or a
/// workspace that is not one directory name.
pub(super) fn read(body: &str) -> Result<Claim, String> {
    stated(body).map_err(|why| format!("the server's answer is not a registration: {why}"))
}

/// The claim, or what is wrong with it.
fn stated(body: &str) -> Result<Claim, String> {
    let value: Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let o = value.as_object().ok_or("it is not a JSON object")?;
    let workspace = str_of(o, "workspace")?;
    let address = strings_of(o, "address")?;
    let ca = str_of(o, "ca")?;
    let anchor = CertificateDer::from_pem_slice(ca.as_bytes())
        .map_err(|e| format!("field \"ca\" holds no certificate ({e})"))?;
    let named = Path::new(&workspace).components().collect::<Vec<_>>();
    if named != [Component::Normal(workspace.as_ref())] {
        return Err(format!(
            "{workspace:?} is not a name an entry can be filed under"
        ));
    }
    if address.is_empty() {
        return Err("field \"address\" lists no endpoint".to_owned());
    }
    Ok(Claim {
        workspace,
        name: str_of(o, "name")?,
        grade: str_of(o, "grade")?,
        address,
        cert: str_of(o, "cert")?,
        anchor: anchor.to_vec(),
        ca,
    })
}

/// File the entry at `dir`: make the directory, which must not exist, and
/// write the four files into it — `private` as the key, readable by its
/// owner alone.
pub(super) fn file(dir: &Path, claim: &Claim, private: &str) -> Result<(), String> {
    fill(dir, claim, private).map_err(|e| unfiled(dir, &e))
}

/// Why the entry was not filed — and for the one cause that is this box
/// already holding a channel there, what to do about it.
fn unfiled(dir: &Path, e: &std::io::Error) -> String {
    // The kind alone is not the fact: making the directories ABOVE the entry
    // answers the same kind when one of them is a file.
    if e.kind() != std::io::ErrorKind::AlreadyExists || !dir.exists() {
        return format!("{}: the entry could not be filed: {e}", dir.display());
    }
    format!(
        "{} already exists, and an entry is never overwritten. Nothing was filed, \
         and this offer is spent: decline it from a connected seat, move the old \
         entry aside if it is to be replaced, and mint a new offer.",
        dir.display()
    )
}

/// The directory, then the four files.
fn fill(dir: &Path, claim: &Claim, private: &str) -> std::io::Result<()> {
    std::fs::create_dir_all(dir.parent().unwrap_or(dir))?;
    std::fs::create_dir(dir)?;
    let address = format!("{}\n", claim.address.join("\n"));
    write(dir, KEY, PRIVATE, private)?;
    write(dir, ANCHORS, PUBLIC, &claim.ca)?;
    write(dir, CHAIN, PUBLIC, &claim.cert)?;
    write(dir, ADDRESS, PUBLIC, &address)
}

/// One new file, at the mode stated from the moment it exists.
fn write(dir: &Path, leaf: &str, mode: u32, text: &str) -> std::io::Result<()> {
    let mut options = std::fs::OpenOptions::new();
    let mut file = options
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(dir.join(leaf))?;
    file.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests;
