//! **What the ladder says while it climbs** (bl-3958; yog REMOTE §13.4 "The
//! operator's view of the loop", the engine's half being yog bl-355c). A dial
//! that fails silently is indistinguishable from one that never ran, so each
//! rung, the punch and the held line say their outcome through the channel's
//! notice sink (`run::Notice`, stderr in production) — one line per event,
//! under the channel's own name, and nothing else is needed to tell which
//! step a call stopped at.
//!
//! **Counts, sequence numbers, nonces and address families — never an
//! address, a key, a salt or a sealed byte.** Every line is built here and
//! nowhere else, so the one rule they share is enforced by the only file that
//! could break it. It is also why a DHT failure is said without its reason:
//! the walk's refusals name the nodes it asked and the target it walked
//! toward, a derivation of the key and the salt. (The channel's own failure
//! sentence still names the engine's address, as DESIGN §2 has it say; that
//! is the address the operator filed, not one the commons revealed.)
//!
//! **A repeated outcome is said once** ([`Say`]): each family remembers the
//! last line it said and stays quiet while the next one is the same. A roving
//! entry whose direct address answers dials per ask, a held line is pinged
//! every twenty-five seconds, and a box with no network climbs the ladder
//! once a minute — none of that is news after the first time.

use std::collections::BTreeMap;
use std::io::{Error, ErrorKind};
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::run::Notice;

/// The family a line belongs to — what "the same outcome again" is judged
/// within.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Kind {
    /// The direct rung's outcome.
    Direct,
    /// The presence read.
    Presence,
    /// The call write.
    Call,
    /// A punch starting.
    Punch,
    /// A punch's outcome.
    Landed,
    /// The held line kept or dropped.
    Held,
    /// A ping discarded off the held line.
    Ping,
}

/// One channel's speaker: the sink, and the last line each family said.
pub(crate) struct Say {
    notice: Notice,
    last: BTreeMap<Kind, String>,
}

impl Say {
    /// A speaker that says into `notice`.
    pub(crate) fn to(notice: &Notice) -> Say {
        Say {
            notice: std::sync::Arc::clone(notice),
            last: BTreeMap::new(),
        }
    }

    /// A speaker nobody hears — a channel no loop has handed a sink.
    pub(crate) fn silent() -> Say {
        Say::to(&(std::sync::Arc::new(|_: &str| ()) as Notice))
    }

    /// Say `line`, unless it is what `kind` said last. A held line kept or
    /// dropped is a new line to ping, so it lets the next ping be said.
    pub(crate) fn say(&mut self, kind: Kind, line: String) {
        if self.last.get(&kind) != Some(&line) {
            (self.notice)(&line);
            self.last.insert(kind, line);
        }
        if kind == Kind::Held {
            self.last.remove(&Kind::Ping);
        }
    }
}

impl std::fmt::Debug for Say {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Say")
            .field("notice", &"a sink")
            .field("last", &self.last)
            .finish()
    }
}

const P: &str = "rendezvous:";

/// The walk's refusal, withheld: it names the nodes asked.
pub(crate) const WALK: &str = "the DHT walk failed (reason withheld: it names nodes)";
/// No walk could start.
pub(crate) const NO_WALK: &str =
    "no DHT walk could start (no bootstrap node resolved, or no socket)";
/// The commons holds nothing under the key.
pub(crate) const NONE: &str = "none is published under the engine's rendezvous key";
/// The item is not this pairing's.
pub(crate) const SEALED: &str = "the item does not open under this pairing";

pub(crate) fn direct_connected() -> String {
    format!("{P} direct rung connected — no punch needed")
}

pub(crate) fn direct_refused(tried: &[IpAddr], timed_out: bool) -> String {
    let how = if timed_out { "timed out" } else { "refused" };
    format!(
        "{P} direct rung {how} — {} address(es) tried ({})",
        tried.len(),
        families(tried)
    )
}

pub(crate) fn presence_read(seq: i64, endpoints: &[SocketAddr]) -> String {
    format!(
        "{P} presence read — seq {seq}, {}",
        counted(endpoints, "endpoint")
    )
}

pub(crate) fn presence_unread(why: &str) -> String {
    format!("{P} presence not read — {why}")
}

pub(crate) fn call_written(nonce: u64, seq: i64, endpoints: &[SocketAddr], acks: usize) -> String {
    format!(
        "{P} call nonce {nonce} written — seq {seq}, {}, {acks} ack(s)",
        counted(endpoints, "endpoint")
    )
}

pub(crate) fn call_unwritten(nonce: u64) -> String {
    format!("{P} call nonce {nonce} not written — {WALK}")
}

/// A punch starting: for a call's nonce, or a re-punch at the cache.
pub(crate) fn punch_started(
    nonce: Option<u64>,
    targets: &[SocketAddr],
    window: Duration,
) -> String {
    let at = nonce.map_or_else(
        || format!("re-punch at {}", counted(targets, "cached endpoint")),
        |n| {
            format!(
                "punch for call nonce {n} at {}",
                counted(targets, "endpoint")
            )
        },
    );
    format!("{P} {at} — window {window:?}")
}

pub(crate) fn landed(peer: Option<SocketAddr>) -> String {
    let peer: Vec<IpAddr> = peer.iter().map(SocketAddr::ip).collect();
    format!("{P} punch landed ({})", families(&peer))
}

pub(crate) fn expired(window: Duration) -> String {
    format!("{P} punch expired after {window:?} with no stream")
}

pub(crate) fn held_kept() -> String {
    format!("{P} held line kept — the punched connection carries the next ask")
}

/// The held line lost on `leg`, by the class of what the socket reported.
pub(crate) fn held_dropped(leg: &str, e: &Error) -> String {
    let class = match e.kind() {
        ErrorKind::TimedOut | ErrorKind::WouldBlock => "silent past the hangup",
        ErrorKind::UnexpectedEof
        | ErrorKind::ConnectionReset
        | ErrorKind::ConnectionAborted
        | ErrorKind::BrokenPipe => "closed by the far end",
        _ => "a transport error",
    };
    format!("{P} held line dropped — the {leg} failed ({class})")
}

pub(crate) fn ping() -> String {
    format!("{P} ping discarded off the held line")
}

/// `3 endpoint(s) (1 v6, 2 v4)`.
fn counted(endpoints: &[SocketAddr], noun: &str) -> String {
    let ips: Vec<IpAddr> = endpoints.iter().map(SocketAddr::ip).collect();
    format!("{} {noun}(s) ({})", ips.len(), families(&ips))
}

/// How many of `ips` are of each family, v6 first — `1 v6, 2 v4`, or `none`.
fn families(ips: &[IpAddr]) -> String {
    let v6 = ips.iter().filter(|ip| ip.is_ipv6()).count();
    let v4 = ips.len() - v6;
    let parts: Vec<String> = [(v6, "v6"), (v4, "v4")]
        .into_iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, family)| format!("{n} {family}"))
        .collect();
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join(", ")
    }
}

#[cfg(test)]
mod tests;
