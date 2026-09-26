//! **The act** (REMOTE §13.2–§13.4): read the engine's presence off the
//! commons, write a sealed call into the inbox, and punch — and the RAM
//! cache that lets the ladder's third rung re-punch without a DHT round trip.
//!
//! **Every duration is a [`Tuning`] field**, stated as the defaults REMOTE
//! §13.7 ruling 3 records and a test's to shorten, so the suite drives a
//! whole rendezvous against a fake DHT and a fake punched engine on loopback
//! in milliseconds. The bootstrap names are resolved at the moment of a call
//! and never earlier: opening a channel dials nothing, and a name lookup is a
//! network act.
//!
//! **What worked stays RAM for the run and never disk** (REMOTE §13.4, the
//! runtime half of §8's `:0` discipline): the engine endpoints the last
//! presence named, and the punch port this end bound once. A redial after a
//! drop punches those endpoints first, which is the whole of what makes a
//! cellular flap cost a punch window rather than a poll period.

use super::item::{Call, Presence};
use super::pairing::Pairing;
use super::punch::{Punch, local_ips};
use crate::dht::{Config, Dht, Udp};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::{Duration, SystemTime};

/// The engine starts an inbox read every fifteen seconds and punches for
/// twenty after the read that finds a call (yog's `Cadence`), so a client
/// window that stopped at twenty would miss the engine's whole window when
/// the poll came late. The sum, and a stated default.
///
/// **It still holds with the read's own length counted** (bl-921e). This
/// window opens when `put` returns, so the call is already stored; the worst
/// case is a read that just missed it, the next starting up to fifteen
/// seconds later and landing when its walk ends. A walk is bounded by
/// `max_queries / alpha` deadlines — 64 / 8 × 1 s, ~8 s, plus the door's
/// re-asks — and measured live at ~7.5 s for a `get` (yog REMOTE §13.7
/// ruling 3, after yog bl-d9c1). So the engine starts punching by ~24 s and
/// the two windows overlap for ~11 s. Before the window walk a `get` ran to
/// ~20 s at the query cap, which put the engine's start at the very edge of
/// this window — the margin the port bought, not a reason to shrink it.
const WINDOW: Duration = Duration::from_secs(35);

/// The knobs one rendezvous runs on.
#[derive(Clone, Debug)]
pub(crate) struct Tuning {
    /// The walk's parameters.
    pub(crate) config: Config,
    /// How long a punch keeps sending SYNs.
    pub(crate) window: Duration,
    /// The DHT's bootstrap nodes, as names or addresses.
    pub(crate) bootstrap: Vec<String>,
}

impl Default for Tuning {
    fn default() -> Tuning {
        Tuning {
            config: Config::default(),
            window: WINDOW,
            bootstrap: super::mainline(),
        }
    }
}

/// One entry's roving state: its material, its knobs, and what the run has
/// learned so far.
#[derive(Debug)]
pub(crate) struct Roving {
    pairing: Pairing,
    pub(crate) tuning: Tuning,
    /// The punch port, bound on the first rendezvous and held for the run.
    punch: Option<Punch>,
    /// The engine endpoints the last presence named — the third rung's.
    cached: Option<Vec<SocketAddr>>,
    /// The last sequence written, so two calls in one second still move
    /// forward — a node refuses a `seq` that does not.
    last_seq: i64,
}

impl Roving {
    pub(crate) fn new(pairing: Pairing, tuning: Tuning) -> Roving {
        Roving {
            pairing,
            tuning,
            punch: None,
            cached: None,
            last_seq: 0,
        }
    }

    /// **The third rung**: a punch at the endpoints the last rendezvous
    /// read, touching no DHT. `None` where there is no cache to punch at or
    /// nothing landed — either way the ladder falls to the full rendezvous.
    pub(crate) fn repunch(&mut self) -> Option<TcpStream> {
        let targets = self.cached.clone()?;
        let window = self.tuning.window;
        self.punch().ok()?.punch(targets, window)
    }

    /// **The fourth rung**: presence, call, punch.
    pub(crate) fn rendezvous(&mut self) -> Result<TcpStream, String> {
        let mut dht = self.dht()?;
        let seal_key = self.pairing.seal_key();
        let item = dht
            .get(self.pairing.key, self.pairing.presence_salt())?
            .ok_or("no presence is published under the engine's rendezvous key")?;
        let presence = Presence::open(&seal_key, &item.value)
            .ok_or("the presence item does not open under this pairing")?;
        if presence.endpoints.is_empty() {
            return Err("the engine's presence names no endpoint".to_owned());
        }
        let port = self.punch()?.port();
        let call = Call {
            nonce: nonce()?,
            endpoints: local_ips()
                .into_iter()
                .map(|ip| SocketAddr::new(ip, port))
                .collect(),
        };
        let seq = unix_now().max(self.last_seq + 1);
        let signed = self.pairing.inbox_keypair()?.sign(
            self.pairing.inbox_salt(),
            seq,
            call.seal(&seal_key)?,
        )?;
        dht.put(signed)?;
        self.last_seq = seq;
        self.cached = Some(presence.endpoints.clone());
        let window = self.tuning.window;
        let targets = presence.endpoints;
        let count = targets.len();
        self.punch()?.punch(targets, window).ok_or_else(|| {
            format!("no SYN crossed toward {count} engine endpoint(s) inside the window")
        })
    }

    /// The punch port, bound once.
    fn punch(&mut self) -> Result<&Punch, String> {
        if self.punch.is_none() {
            self.punch = Some(Punch::bind(0)?);
        }
        self.punch
            .as_ref()
            .ok_or_else(|| "no punch port".to_owned())
    }

    /// A DHT client for this call, over a fresh ephemeral UDP socket.
    fn dht(&self) -> Result<Dht, String> {
        let bootstrap: Vec<SocketAddr> = self
            .tuning
            .bootstrap
            .iter()
            .filter_map(|name| name.to_socket_addrs().ok())
            .flatten()
            .collect();
        if bootstrap.is_empty() {
            return Err("no DHT bootstrap node resolved".to_owned());
        }
        let udp = Udp::bind("0.0.0.0:0".parse().map_err(|e| format!("{e}"))?)
            .map_err(|e| format!("DHT socket: {e}"))?;
        Dht::new(Box::new(udp), bootstrap, self.tuning.config.clone())
    }
}

/// A fresh call nonce.
fn nonce() -> Result<u64, String> {
    let mut bytes = [0u8; 8];
    crate::dht::random(&mut bytes)?;
    Ok(u64::from_be_bytes(bytes))
}

/// Seconds since the epoch — the natural rising `seq`.
fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests;
