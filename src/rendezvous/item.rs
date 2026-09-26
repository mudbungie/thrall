//! The two items the rendezvous carries on the commons (REMOTE §13.2), and
//! the seal that makes them opaque bytes there — the client's halves.
//!
//! **Presence** is the engine's item and this end *opens* it: the endpoints
//! the engine punches from. **The call** is this end's inbox item and this
//! end *seals* it: the endpoints this box punches from, and a nonce the
//! engine remembers so one call draws one punch. Both are sealed under the
//! pairing's AEAD key before they are signed and filed, so a DHT node — or
//! anyone walking the keyspace — learns neither that this is an engine nor
//! where either end lives.
//!
//! **The wire format is the engine's, mirrored** (yog
//! `src/wire/rendezvous/item.rs`): a sealed item is `nonce(12) ‖
//! ChaCha20-Poly1305(plain) ‖ tag(16)`; the presence plaintext is one
//! endpoint list; a call's is an 8-byte big-endian nonce and then one; an
//! endpoint list is a count byte, then per endpoint a family byte (`4` or
//! `6`), the address's own bytes and a big-endian port. Every field is
//! fixed-width so a hostile item is bounded before it is read, and
//! `item::tests` opens bytes the engine's own code sealed.

use ring::aead::{Aad, CHACHA20_POLY1305, LessSafeKey, Nonce, UnboundKey};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

/// The engine's published half: where it can be punched.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Presence {
    pub(crate) endpoints: Vec<SocketAddr>,
}

/// This box's inbox item: *call me at these endpoints*, once per nonce.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Call {
    pub(crate) nonce: u64,
    pub(crate) endpoints: Vec<SocketAddr>,
}

impl Presence {
    /// Open a sealed presence; anything that is not one under `key` is nothing.
    pub(crate) fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Presence> {
        let (endpoints, rest) = decode(&open(key, sealed)?)?;
        rest.is_empty().then_some(Presence { endpoints })
    }

    /// The engine's half — kept as the suite's mirror of the format.
    #[cfg(test)]
    pub(crate) fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
        seal(key, &encode(&self.endpoints))
    }
}

impl Call {
    /// Seal this call under `key` with a fresh random nonce.
    pub(crate) fn seal(&self, key: &[u8; 32]) -> Result<Vec<u8>, String> {
        let mut plain = self.nonce.to_be_bytes().to_vec();
        plain.extend(encode(&self.endpoints));
        seal(key, &plain)
    }

    /// The engine's half — kept as the suite's mirror of the format.
    #[cfg(test)]
    pub(crate) fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Call> {
        let plain = open(key, sealed)?;
        let (head, tail) = plain.split_at_checked(8)?;
        let nonce = u64::from_be_bytes(<[u8; 8]>::try_from(head).ok()?);
        let (endpoints, rest) = decode(tail)?;
        rest.is_empty().then_some(Call { nonce, endpoints })
    }
}

/// An endpoint list, as the module doc spells it.
fn encode(endpoints: &[SocketAddr]) -> Vec<u8> {
    let mut out = vec![u8::try_from(endpoints.len()).unwrap_or(u8::MAX)];
    for addr in endpoints.iter().take(255) {
        match addr.ip() {
            IpAddr::V4(ip) => {
                out.push(4);
                out.extend_from_slice(&ip.octets());
            }
            IpAddr::V6(ip) => {
                out.push(6);
                out.extend_from_slice(&ip.octets());
            }
        }
        out.extend_from_slice(&addr.port().to_be_bytes());
    }
    out
}

/// The inverse of [`encode`], handing back what followed the list.
fn decode(bytes: &[u8]) -> Option<(Vec<SocketAddr>, Vec<u8>)> {
    let (count, mut rest) = bytes.split_first()?;
    let mut endpoints = Vec::new();
    for _ in 0..*count {
        let (family, tail) = rest.split_first()?;
        let width = match family {
            4 => 4,
            6 => 16,
            _ => return None,
        };
        let (ip, tail) = tail.split_at_checked(width)?;
        let (port, tail) = tail.split_at_checked(2)?;
        let ip = match family {
            4 => IpAddr::V4(Ipv4Addr::from(<[u8; 4]>::try_from(ip).ok()?)),
            _ => IpAddr::V6(Ipv6Addr::from(<[u8; 16]>::try_from(ip).ok()?)),
        };
        let port = u16::from_be_bytes(<[u8; 2]>::try_from(port).ok()?);
        endpoints.push(SocketAddr::new(ip, port));
        rest = tail;
    }
    Some((endpoints, rest.to_vec()))
}

/// Seal `plain` under `key` with a fresh random nonce, nonce first.
fn seal(key: &[u8; 32], plain: &[u8]) -> Result<Vec<u8>, String> {
    let mut nonce = [0u8; 12];
    crate::dht::random(&mut nonce)?;
    let key = aead(key)?;
    let mut body = plain.to_vec();
    key.seal_in_place_append_tag(Nonce::assume_unique_for_key(nonce), Aad::empty(), &mut body)
        .map_err(|_| "the seal refused".to_owned())?;
    let mut out = nonce.to_vec();
    out.extend(body);
    Ok(out)
}

/// Open a sealed item; anything that does not verify under `key` is nothing.
fn open(key: &[u8; 32], sealed: &[u8]) -> Option<Vec<u8>> {
    let (nonce, body) = sealed.split_at_checked(12)?;
    let nonce = Nonce::try_assume_unique_for_key(nonce).ok()?;
    let mut body = body.to_vec();
    let plain = aead(key)
        .ok()?
        .open_in_place(nonce, Aad::empty(), &mut body)
        .ok()?;
    Some(plain.to_vec())
}

fn aead(key: &[u8; 32]) -> Result<LessSafeKey, String> {
    UnboundKey::new(&CHACHA20_POLY1305, key)
        .map(LessSafeKey::new)
        .map_err(|_| "the seal key is not 32 bytes".to_owned())
}

#[cfg(test)]
mod tests;
