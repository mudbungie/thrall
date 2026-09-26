//! **The dial ladder** (REMOTE §13.4; DESIGN §3.11): the rungs a dial runs
//! before there is a connection to speak on, in the order that costs least.
//!
//! ```text
//! the live held connection      — `Channel::ask`, before this file is reached
//! the entry's direct address    — one bounded connect
//! a re-punch at cached endpoints — a punch window, no DHT round trip
//! the full rendezvous           — presence, call, punch
//! ```
//!
//! **An entry with no rendezvous material has a one-rung ladder**, and it is
//! today's dial byte for byte: the connect either lands or its sentence is
//! the failure. The lower rungs exist only where the operator carried the
//! pairing, and they are climbed only when the direct address did not answer
//! — a LAN, a stable client and loopback never pay for the machinery.
//!
//! **What comes back says which kind of connection it is**, because the
//! channel holds a punched one and not a dialled one: a punch costs seconds
//! and is reused across asks (REMOTE §13.4); a direct connect costs
//! milliseconds and stays one per ask, which is what keeps a busy foot
//! absent at the far end (DESIGN §3.7).

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use crate::rendezvous::call::Roving;

/// How long one direct connect may take before the next rung is tried. A
/// NAT that drops the SYN answers nothing, and the kernel's own patience is
/// minutes; a rung has to give up sooner than that to be a rung.
const DIRECT: Duration = Duration::from_secs(10);

/// A connection to the engine and whether it was punched.
pub(super) fn climb(
    address: &str,
    roving: Option<&mut Roving>,
) -> Result<(TcpStream, bool), String> {
    let direct = direct(address);
    let Some(roving) = roving else {
        return direct.map(|tcp| (tcp, false));
    };
    let refusal = match direct {
        Ok(tcp) => return Ok((tcp, false)),
        Err(refusal) => refusal,
    };
    if let Some(tcp) = roving.repunch() {
        return Ok((tcp, true));
    }
    match roving.rendezvous() {
        Ok(tcp) => Ok((tcp, true)),
        Err(why) => Err(format!("{refusal}; rendezvous: {why}")),
    }
}

/// The direct rung: every address the name resolves to, each under one
/// bound, and the sentence of the last that refused.
fn direct(address: &str) -> Result<TcpStream, String> {
    let resolved = address
        .to_socket_addrs()
        .map_err(|e| format!("connect {address}: {e}"))?;
    let mut last = None;
    for at in resolved {
        match TcpStream::connect_timeout(&at, DIRECT) {
            Ok(tcp) => return Ok(tcp),
            Err(e) => last = Some(e),
        }
    }
    Err(format!(
        "connect {address}: {}",
        last.map_or_else(|| "no address resolved".to_owned(), |e| e.to_string())
    ))
}
