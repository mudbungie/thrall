//! **The act** (REMOTE §13.2–§13.4): read the engine's presence off the
//! commons, write a sealed call into the inbox, and punch — and the RAM
//! cache that lets the ladder's third rung re-call without reading presence.
//!
//! **Every duration is a [`Tuning`] field**, stated as the defaults REMOTE
//! §13.7 ruling 3 records and a test's to shorten, so the suite drives a
//! whole rendezvous against a fake DHT and a fake punched engine on loopback
//! in milliseconds. The bootstrap names are resolved at the moment of a call
//! and never earlier: opening a channel dials nothing, and a name lookup is a
//! network act.
//!
//! **What worked stays RAM for the run and never disk** (REMOTE §13.4, the
//! runtime half of §8's `:0` discipline): the engine endpoints of the last
//! call that landed, the sequence it was written under, and the punch port
//! this end bound once. A redial after a drop writes a fresh call to those
//! endpoints before it reads presence again — one walk rather than two
//! (REMOTE §13.3, ruling yog bl-278f) — and a re-call nobody answers clears
//! them, because the presence they came from is then the suspect.

use super::item::Presence;
use super::pairing::Pairing;
use super::punch::Punch;
use super::say::{self, Kind, Say};
use crate::dht::{Config, Dht, Udp};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

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
    /// The engine endpoints of the last call that landed — the re-call's
    /// input, cleared by a re-call that does not.
    cached: Option<Vec<SocketAddr>>,
    /// The last sequence written, so two calls in one second still move
    /// forward — a node refuses a `seq` that does not.
    last_seq: i64,
    /// What the ladder says as it climbs (`say`), silent until a loop hands
    /// the channel its sink.
    pub(crate) say: Say,
}

impl Roving {
    pub(crate) fn new(pairing: Pairing, tuning: Tuning) -> Roving {
        Roving {
            pairing,
            tuning,
            punch: None,
            cached: None,
            last_seq: 0,
            say: Say::silent(),
        }
    }

    /// **The third rung, a re-call** (yog REMOTE §13.3, ruling bl-278f): a
    /// fresh call written from the cached presence and punched — one DHT
    /// walk, not two, and no presence read. `None` where nothing is cached or
    /// the call did not land; **a re-call that does not land clears the
    /// cache**, so the rung under it — and every climb after — reads presence
    /// again.
    ///
    /// It is a call and never a bare re-punch: an engine behind a NAT holds
    /// no mapping once the served stream has ended and sends no SYN outside
    /// a call's window, so a punch at cached endpoints with no call behind it
    /// is one-sided and lands on nothing (measured on yog bl-65dc).
    pub(crate) fn recall(&mut self) -> Option<TcpStream> {
        let engine = self.cached.take()?;
        let mut dht = self.dht().ok()?;
        self.called(&mut dht, engine, true).ok()?
    }

    /// **The fourth rung**: presence, call, punch — each step said as it
    /// ends (`say`), a DHT failure without its reason.
    pub(crate) fn rendezvous(&mut self) -> Result<TcpStream, String> {
        let unread = |why: &str| say::presence_unread(why);
        let mut dht = self
            .dht()
            .map_err(|e| self.missed(Kind::Presence, unread(say::NO_WALK), e))?;
        let got = dht.get(self.pairing.key, self.pairing.presence_salt());
        let item = got
            .map_err(|e| self.missed(Kind::Presence, unread(say::WALK), e))?
            .ok_or_else(|| {
                self.missed(
                    Kind::Presence,
                    unread(say::NONE),
                    "no presence is published under the engine's rendezvous key".to_owned(),
                )
            })?;
        let presence = Presence::open(&self.pairing.seal_key(), &item.value).ok_or_else(|| {
            self.missed(
                Kind::Presence,
                unread(say::SEALED),
                "the presence item does not open under this pairing".to_owned(),
            )
        })?;
        let line = say::presence_read(item.seq, &presence.endpoints);
        self.say.say(Kind::Presence, line);
        if presence.endpoints.is_empty() {
            return Err("the engine's presence names no endpoint".to_owned());
        }
        let count = presence.endpoints.len();
        self.called(&mut dht, presence.endpoints, false)?
            .ok_or_else(|| {
                format!("no SYN crossed toward {count} engine endpoint(s) inside the window")
            })
    }

    /// One punch toward `targets`, said as it starts and as it ends.
    fn punched(
        &mut self,
        nonce: u64,
        targets: Vec<SocketAddr>,
    ) -> Result<Option<TcpStream>, String> {
        let window = self.tuning.window;
        self.punch()?;
        self.say
            .say(Kind::Punch, say::punch_started(nonce, &targets, window));
        let landed = self.punch()?.punch(targets, window);
        let line = landed.as_ref().map_or_else(
            || say::expired(window),
            |tcp| say::landed(tcp.peer_addr().ok()),
        );
        self.say.say(Kind::Landed, line);
        Ok(landed)
    }

    /// Say `line` and hand back the refusal it stands for.
    fn missed(&mut self, kind: Kind, line: String, why: String) -> String {
        self.say.say(kind, line);
        why
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

mod write;

#[cfg(test)]
mod tests;
