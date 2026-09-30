//! **Writing a call** (REMOTE §13.2–§13.3): the one act the ladder's two
//! lower rungs share — walk to the inbox's write-token holders, seal a call
//! naming where that walk saw this box, store it, punch — and the cache it
//! leaves behind when the punch lands.

use super::Roving;
use crate::dht::Dht;
use crate::dht::mutable::target_of;
use crate::rendezvous::item::Call;
use crate::rendezvous::punch::local_ips;
use crate::rendezvous::say::{self, Kind};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::time::SystemTime;

impl Roving {
    /// **A call to `engine`, and its punch** — the one act both lower rungs
    /// share. The write's walk runs first, so the call names where THAT walk
    /// saw this box (`listed`); a re-call says itself as one. **What lands is
    /// the cache**: the engine endpoints of the last call that was answered,
    /// and nothing else — the re-call's whole input beside `last_seq`.
    pub(super) fn called(
        &mut self,
        dht: &mut Dht,
        engine: Vec<SocketAddr>,
        again: bool,
    ) -> Result<Option<TcpStream>, String> {
        let port = self.punch()?.port();
        let keypair = self.pairing.inbox_keypair()?;
        let salt = self.pairing.inbox_salt();
        let nonce = nonce()?;
        let holders = dht
            .holders(target_of(&keypair.public(), &salt))
            .map_err(|e| self.missed(Kind::Call, say::call_unwritten(nonce), e))?;
        let call = Call {
            nonce,
            endpoints: listed(local_ips(), &dht.observed(), port),
        };
        let seq = unix_now().max(self.last_seq + 1);
        let signed = keypair.sign(salt, seq, call.seal(&self.pairing.seal_key())?)?;
        let acks = dht
            .store(&signed, holders)
            .map_err(|e| self.missed(Kind::Call, say::call_unwritten(nonce), e))?;
        let line = if again {
            say::recalled(nonce, &call.endpoints, acks)
        } else {
            say::call_written(nonce, seq, &call.endpoints, acks)
        };
        self.say.say(Kind::Call, line);
        self.last_seq = seq;
        let landed = self.punched(nonce, engine.clone())?;
        if landed.is_some() {
            self.cached = Some(engine);
        }
        Ok(landed)
    }
}

/// The call's endpoint list (yog REMOTE §13.2, which rules that a list
/// carry the OBSERVED endpoint; thrall bl-d340 after yog bl-efae): the
/// route-local addresses, then every address the call's own write walk voted
/// it saw this box at (`Dht::observed`) that is not already one of them —
/// all at the punch port. Behind a carrier or tethered NAT the route-local
/// ones are private, and the observed one is the only one the engine can
/// punch to (§13.8). The observed PORT is the DHT socket's UDP mapping, not
/// the punch port's TCP one, so only the address is taken and port
/// preservation is trusted; a carrier that rewrites the port is the case
/// this does not reach.
pub(super) fn listed(mut ips: Vec<IpAddr>, observed: &[SocketAddr], port: u16) -> Vec<SocketAddr> {
    for ip in observed.iter().map(SocketAddr::ip) {
        if !ips.contains(&ip) {
            ips.push(ip);
        }
    }
    ips.into_iter()
        .map(|ip| SocketAddr::new(ip, port))
        .collect()
}

/// A fresh call nonce.
pub(super) fn nonce() -> Result<u64, String> {
    let mut bytes = [0u8; 8];
    crate::dht::random(&mut bytes)?;
    Ok(u64::from_be_bytes(bytes))
}

/// Seconds since the epoch — the natural rising `seq`.
pub(super) fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}
