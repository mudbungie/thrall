//! BEP 44's mutable item — the shape REMOTE §13.2's presence and inbox are
//! written in: an ed25519 public key, an optional salt, a sequence number and
//! a value of up to 1000 bencoded bytes, signed by the key's holder. `ring`
//! signs and verifies (it is the provider rustls already links; no new crate),
//! and the item's **target** — the 160-bit key the DHT stores it under — is
//! `sha1(key ‖ salt)`, which is the one legacy use of SHA-1 this crate has.
//!
//! What is signed is the canonical bencoding of `salt` (only when non-empty),
//! `seq` and `v` **without** the enclosing dictionary — `4:salt…3:seqi…e1:v…`
//! — exactly as the BEP spells it, so the bytes here are the bytes every node
//! on the commons checks. The value is always a byte string here: both of
//! §13.2's items are sealed opaque bytes, so `v` is never anything else.

use super::bencode::{Dict, Value, bytes, entry};
use super::krpc::NodeId;
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};

/// The most a mutable item's bencoded value may carry (BEP 44).
pub(crate) const MAX_VALUE: usize = 1000;

/// An ed25519 keypair. **A foot holds none of its own** (REMOTE §13.2): the
/// one it signs with is derived from the pairing salt, so the only way one
/// comes to exist here is [`Keypair::from_seed`].
pub(crate) struct Keypair(Ed25519KeyPair);

impl Keypair {
    /// The keypair a 32-byte seed determines.
    pub(crate) fn from_seed(seed: [u8; 32]) -> Result<Keypair, String> {
        Ed25519KeyPair::from_seed_unchecked(&seed)
            .map(Keypair)
            .map_err(|e| format!("ed25519 seed refused: {e}"))
    }

    /// The public half.
    pub(crate) fn public(&self) -> [u8; 32] {
        <[u8; 32]>::try_from(self.0.public_key().as_ref()).unwrap_or([0u8; 32])
    }

    /// Sign one item under this key. A value over [`MAX_VALUE`] is refused
    /// here rather than by every node it would have been sent to.
    pub(crate) fn sign(&self, salt: Vec<u8>, seq: i64, value: Vec<u8>) -> Result<Mutable, String> {
        let encoded = bytes(&value).encode().len();
        if encoded > MAX_VALUE {
            return Err(format!(
                "value bencodes to {encoded} bytes; BEP 44 allows {MAX_VALUE}"
            ));
        }
        let sig = self.0.sign(&signed_bytes(&salt, seq, &value));
        let sig = <[u8; 64]>::try_from(sig.as_ref()).unwrap_or([0u8; 64]);
        Ok(Mutable {
            key: self.public(),
            salt,
            seq,
            value,
            sig,
        })
    }
}

/// One signed mutable item, as stored and as read back.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Mutable {
    pub(crate) key: [u8; 32],
    pub(crate) salt: Vec<u8>,
    pub(crate) seq: i64,
    pub(crate) value: Vec<u8>,
    pub(crate) sig: [u8; 64],
}

impl Mutable {
    /// Where the DHT stores this item.
    pub(crate) fn target(&self) -> NodeId {
        target_of(&self.key, &self.salt)
    }

    /// Does the signature hold under the item's own key?
    pub(crate) fn verify(&self) -> bool {
        UnparsedPublicKey::new(&ED25519, self.key)
            .verify(&signed_bytes(&self.salt, self.seq, &self.value), &self.sig)
            .is_ok()
    }

    /// The item a `get` reply carries under `key` and `salt`, if the reply
    /// carries one and it verifies — an unsigned or forged item is nothing.
    pub(crate) fn from_reply(r: &Dict, key: [u8; 32], salt: &[u8]) -> Option<Mutable> {
        let sig = <[u8; 64]>::try_from(r.get(b"sig".as_slice())?.as_bytes()?).ok()?;
        let item = Mutable {
            key,
            salt: salt.to_vec(),
            seq: r.get(b"seq".as_slice())?.as_int()?,
            value: r.get(b"v".as_slice())?.as_bytes()?.to_vec(),
            sig,
        };
        item.verify().then_some(item)
    }

    /// The `put` arguments this item spells (BEP 44), under a node's token.
    pub(crate) fn put_args(&self, token: &[u8]) -> Dict {
        let mut args = Dict::from([
            entry("k", bytes(&self.key)),
            entry("seq", Value::Int(self.seq)),
            entry("sig", bytes(&self.sig)),
            entry("token", bytes(token)),
            entry("v", bytes(&self.value)),
        ]);
        if !self.salt.is_empty() {
            args.insert(b"salt".to_vec(), bytes(&self.salt));
        }
        args
    }
}

/// `sha1(key ‖ salt)` — the target a key and salt name.
pub(crate) fn target_of(key: &[u8; 32], salt: &[u8]) -> NodeId {
    let mut input = key.to_vec();
    input.extend_from_slice(salt);
    let digest = ring::digest::digest(&ring::digest::SHA1_FOR_LEGACY_USE_ONLY, &input);
    NodeId::parse(digest.as_ref()).unwrap_or(NodeId([0u8; 20]))
}

/// The bytes a signature covers; `salt` appears only when non-empty.
pub(crate) fn signed_bytes(salt: &[u8], seq: i64, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    if !salt.is_empty() {
        out.extend_from_slice(b"4:salt");
        out.extend_from_slice(&bytes(salt).encode());
    }
    out.extend_from_slice(b"3:seq");
    out.extend_from_slice(&Value::Int(seq).encode());
    out.extend_from_slice(b"1:v");
    out.extend_from_slice(&bytes(value).encode());
    out
}

#[cfg(test)]
mod tests;
