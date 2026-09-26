//! **The rendezvous material an entry holds** (REMOTE §13.2; DESIGN §3.11),
//! and everything both ends derive from it.
//!
//! Two files beside `ca.pem`, both hex, both carried here by the operator's
//! hand exactly as the anchors are (REMOTE §1.4): the engine's ed25519
//! **public** key, which presence is filed under, and the **pairing salt**
//! the two ends share. **A foot holds no keypair of its own.** The inbox is
//! signed under a keypair *derived* from the salt — `inbox key` below is an
//! ed25519 seed — so a client needs nothing minted to write it and the
//! engine holds no client key to poll it; that is what makes the material two
//! files rather than three.
//!
//! **One salt, four derivations, all HKDF-SHA256** with the salt `yog
//! rendezvous` and one info label each, 32 bytes out: the DHT salts the two
//! items are filed under, the AEAD key both are sealed with, and the inbox
//! seed. The labels are the engine's (`src/wire/rendezvous/material.rs`), and
//! `pairing::tests` holds the bytes the engine's own code derives from a
//! known salt, so a label that drifted by one character is a red test here
//! rather than a call nobody answers.

use crate::dht::Keypair;
use ring::hkdf::{HKDF_SHA256, Salt};
use std::path::Path;

/// The engine's rendezvous public key, hex — where presence lives.
pub(crate) const KEY: &str = "rendezvous.pub";
/// The pairing salt, hex — the secret this box and the engine share.
pub(crate) const SALT: &str = "pairing.salt";

/// What the two files hold, decoded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Pairing {
    /// The engine's public key.
    pub(crate) key: [u8; 32],
    pub(crate) salt: [u8; 32],
}

impl Pairing {
    /// The keypair the inbox is signed with, which both ends derive.
    pub(crate) fn inbox_keypair(&self) -> Result<Keypair, String> {
        Keypair::from_seed(self.derive(b"inbox key"))
    }

    /// The DHT salt the engine's presence is filed under.
    pub(crate) fn presence_salt(&self) -> Vec<u8> {
        self.derive(b"presence salt").to_vec()
    }

    /// The DHT salt the inbox is filed under.
    pub(crate) fn inbox_salt(&self) -> Vec<u8> {
        self.derive(b"inbox salt").to_vec()
    }

    /// The AEAD key both items are sealed under.
    pub(crate) fn seal_key(&self) -> [u8; 32] {
        self.derive(b"seal key")
    }

    /// HKDF-SHA256 over the pairing salt, one label per derived fact.
    fn derive(&self, label: &[u8]) -> [u8; 32] {
        let mut out = [0u8; 32];
        // HKDF-SHA256 expands to exactly 32 bytes for a 32-byte buffer, so
        // neither step can refuse; the fallback is unreachable and named.
        if let Ok(okm) = Salt::new(HKDF_SHA256, b"yog rendezvous")
            .extract(&self.salt)
            .expand(&[label], HKDF_SHA256)
        {
            let _ = okm.fill(&mut out);
        }
        out
    }
}

/// Read the pairing out of `dir`: `Ok(None)` is no rendezvous material —
/// today's entry, which roves nowhere and is not a failure — and `Err` is
/// half of it, the same misconfiguration a half-provisioned entry is.
pub(crate) fn read_dir(dir: &Path) -> Result<Option<Pairing>, String> {
    let (key, salt) = (dir.join(KEY), dir.join(SALT));
    match (key.is_file(), salt.is_file()) {
        (false, false) => Ok(None),
        (true, true) => match (hex_file(&key), hex_file(&salt)) {
            (Some(key), Some(salt)) => Ok(Some(Pairing { key, salt })),
            _ => Err(format!(
                "{} holds rendezvous material that does not read: {KEY} and {SALT} are \
                 each exactly 32 bytes of hex — {}",
                dir.display(),
                crate::channel::material::REMEDY
            )),
        },
        _ => Err(format!(
            "{} is half-provisioned for rendezvous: {KEY} and {SALT} are carried \
             together — {}",
            dir.display(),
            crate::channel::material::REMEDY
        )),
    }
}

/// Exactly 32 bytes of hex in `path`, or nothing.
fn hex_file(path: &Path) -> Option<[u8; 32]> {
    let text = std::fs::read_to_string(path).ok()?;
    let bytes = unhex(text.trim())?;
    <[u8; 32]>::try_from(bytes).ok()
}

/// Lowercase hex — the spelling both files are written in, and the suite's
/// way of writing them.
#[cfg(test)]
pub(crate) fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

/// The inverse of [`hex`]; `None` on any byte that is not one.
pub(crate) fn unhex(text: &str) -> Option<Vec<u8>> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect()
}

#[cfg(test)]
mod tests;
