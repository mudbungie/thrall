//! **The dial ladder** (REMOTE §13.4; DESIGN §3.11): the rungs a dial runs
//! before there is a connection to speak on, in the order that costs least.
//!
//! ```text
//! the live held connection      — `Channel::ask`, before this file is reached
//! the entry's direct address    — one bounded connect
//! a re-call from cached presence — one DHT walk, a punch window
//! the full rendezvous           — presence, call, punch
//! ```
//!
//! **An entry with no rendezvous material has a one-rung ladder**, and it is
//! today's dial byte for byte: the connect either lands or its sentence is
//! the failure. The lower rungs exist only where the operator carried the
//! pairing, and they are climbed only when the direct address did not answer
//! — a LAN, a stable client and loopback never pay for the machinery.
//!
//! **The third rung is a call, never a bare re-punch** (yog REMOTE §13.3,
//! ruling bl-278f): a held connection's pings are the only thing keeping a
//! NAT mapping alive, so once the line has dropped the engine's side holds
//! none, and a punch no call asked the engine to answer is one-sided. The
//! re-call skips the presence read and nothing else; one it cannot land
//! clears the cache, and the full rendezvous under it reads presence afresh.
//!
//! **What comes back says which kind of connection it is**, because the
//! channel holds a punched one and not a dialled one: a punch costs seconds
//! and is reused across asks (REMOTE §13.4); a direct connect costs
//! milliseconds and stays one per ask, which is what keeps a busy foot
//! absent at the far end (DESIGN §3.7).

use std::io::ErrorKind;
use std::net::{IpAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::rendezvous::call::Roving;
use crate::rendezvous::say::{self, Kind};

/// How long one direct connect may take before the next rung is tried. A
/// NAT that drops the SYN answers nothing, and the kernel's own patience is
/// minutes; a rung has to give up sooner than that to be a rung.
const DIRECT: Duration = Duration::from_secs(10);

/// The direct rung refusing: the sentence, and what the ladder says of it —
/// the families it tried and whether the last one timed out, never the
/// addresses.
struct Missed {
    sentence: String,
    tried: Vec<IpAddr>,
    timed_out: bool,
}

/// A connection to the engine and whether it was punched. Only a roving
/// entry's ladder says its rungs (`rendezvous::say`): a one-rung ladder's
/// only outcome already reaches the operator as the channel's sentence.
pub(super) fn climb(
    address: &str,
    roving: Option<&mut Roving>,
) -> Result<(TcpStream, bool), String> {
    let direct = direct(address);
    let Some(roving) = roving else {
        return direct.map(|tcp| (tcp, false)).map_err(|m| m.sentence);
    };
    let refusal = match direct {
        Ok(tcp) => {
            roving.say.say(Kind::Direct, say::direct_connected());
            return Ok((tcp, false));
        }
        Err(missed) => {
            let line = say::direct_refused(&missed.tried, missed.timed_out);
            roving.say.say(Kind::Direct, line);
            missed.sentence
        }
    };
    if let Some(tcp) = roving.recall() {
        return Ok((tcp, true));
    }
    match roving.rendezvous() {
        Ok(tcp) => Ok((tcp, true)),
        Err(why) => Err(format!("{refusal}; rendezvous: {why}")),
    }
}

/// The direct rung: every address the name resolves to, each under one
/// bound, and the sentence of the last that refused.
fn direct(address: &str) -> Result<TcpStream, Missed> {
    let resolved = address.to_socket_addrs().map_err(|e| Missed {
        sentence: format!("connect {address}: {e}"),
        tried: Vec::new(),
        timed_out: false,
    })?;
    let mut last = None;
    let mut tried = Vec::new();
    for at in resolved {
        tried.push(at.ip());
        match TcpStream::connect_timeout(&at, DIRECT) {
            Ok(tcp) => return Ok(tcp),
            Err(e) => last = Some(e),
        }
    }
    let timed_out = last
        .as_ref()
        .is_some_and(|e| e.kind() == ErrorKind::TimedOut);
    Err(Missed {
        sentence: format!(
            "connect {address}: {}",
            last.map_or_else(|| "no address resolved".to_owned(), |e| e.to_string())
        ),
        tried,
        timed_out,
    })
}
