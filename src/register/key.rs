//! **The key this box makes, and the word both ends show** (REMOTE §15.1).
//!
//! The private key is generated here and never leaves: what is posted is the
//! public half, and what comes back is a leaf the operator's CA signed for
//! it. That is the whole difference from the carried pair this replaces —
//! a copy of the registration URL is not the device.
//!
//! **`ring` and nothing else.** It is the provider `rustls` already links:
//! it generates the P-256 key as PKCS#8, which is the form `rustls` reads a
//! client key in, and hashes the word. What `ring` does not do is wrap a
//! public key as `SubjectPublicKeyInfo` or write PEM, and both are small and
//! fixed enough to state here — a 26-byte prefix, and base64 in 64-column
//! lines — rather than link a crate for.

use ring::rand::SystemRandom;
use ring::signature::{ECDSA_P256_SHA256_ASN1_SIGNING, EcdsaKeyPair, KeyPair};

/// The PEM label of a `SubjectPublicKeyInfo`.
pub(super) const PUBLIC: &str = "PUBLIC KEY";
/// The PEM label of a PKCS#8 private key.
const PRIVATE: &str = "PRIVATE KEY";

/// What a `SubjectPublicKeyInfo` for a P-256 key says before the point
/// itself: an `id-ecPublicKey` on `prime256v1`, then a 66-byte bit string
/// holding the 65-byte uncompressed point. 91 bytes in all, always.
const SPKI_P256: [u8; 26] = [
    0x30, 0x59, 0x30, 0x13, 0x06, 0x07, 0x2a, 0x86, 0x48, 0xce, 0x3d, 0x02, 0x01, 0x06, 0x08, 0x2a,
    0x86, 0x48, 0xce, 0x3d, 0x03, 0x01, 0x07, 0x03, 0x42, 0x00,
];

/// The alphabet a code and a word are written in: capitals and digits
/// without `I`, `O`, `0` and `1`, because a person reads them.
const WORD: &str = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
/// How many symbols a confirmation word has — thirty bits, five at a time.
const SYMBOLS: usize = 6;

/// The base64 alphabet (RFC 4648 §4).
const BASE64: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
/// How wide a PEM body line is (RFC 7468).
const COLUMNS: usize = 64;

/// A key pair made on this box.
pub(super) struct Made {
    /// The private key, PKCS#8, PEM — what the entry's `client.key` holds.
    pub(super) private: String,
    /// The public key, DER `SubjectPublicKeyInfo` — what is posted, and the
    /// first half of what the word is hashed over.
    pub(super) spki: Vec<u8>,
}

/// Make a P-256 key pair from the system's random source.
pub(super) fn make() -> Result<Made, String> {
    let rng = SystemRandom::new();
    let alg = &ECDSA_P256_SHA256_ASN1_SIGNING;
    let unmade = |what: &str| format!("this box could not make a key: {what}");
    let pkcs8 = EcdsaKeyPair::generate_pkcs8(alg, &rng).map_err(|e| unmade(&e.to_string()))?;
    let pair =
        EcdsaKeyPair::from_pkcs8(alg, pkcs8.as_ref(), &rng).map_err(|e| unmade(&e.to_string()))?;
    Ok(Made {
        private: pem(PRIVATE, pkcs8.as_ref()),
        spki: [&SPKI_P256, pair.public_key().as_ref()].concat(),
    })
}

/// **The confirmation word**: SHA-256 over the device's public key (DER
/// `SubjectPublicKeyInfo`) followed by the CA certificate (DER), its first
/// thirty bits read five at a time, most significant first, each an index
/// into [`WORD`]. The server computes the same from what it signed and what
/// it holds; an impostor between the two cannot make them agree.
pub(super) fn word(spki: &[u8], ca: &[u8]) -> String {
    let digest = ring::digest::digest(&ring::digest::SHA256, &[spki, ca].concat());
    let head = digest.as_ref().iter().take(4);
    let bits = head.fold(0_usize, |bits, byte| (bits << 8) | usize::from(*byte));
    let symbol = |at: usize| WORD.chars().nth((bits >> (27 - 5 * at)) & 31);
    (0..SYMBOLS).filter_map(symbol).collect()
}

/// `der` as one PEM block under `label`.
pub(super) fn pem(label: &str, der: &[u8]) -> String {
    let symbols: Vec<char> = base64(der).chars().collect();
    let lines: Vec<String> = symbols
        .chunks(COLUMNS)
        .map(|l| l.iter().collect())
        .collect();
    format!(
        "-----BEGIN {label}-----\n{}\n-----END {label}-----\n",
        lines.join("\n")
    )
}

/// Base64, padded.
fn base64(bytes: &[u8]) -> String {
    let mut said = String::new();
    for group in bytes.chunks(3) {
        let short = 3 - group.len();
        let held = group
            .iter()
            .fold(0_usize, |held, byte| (held << 8) | usize::from(*byte));
        let bits = held << (8 * short);
        let symbol = |at: usize| BASE64.chars().nth((bits >> (18 - 6 * at)) & 63);
        said.extend((0..4 - short).filter_map(symbol));
        said.push_str(&"=".repeat(short));
    }
    said
}

#[cfg(test)]
mod tests;
