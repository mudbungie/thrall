//! **The dial ladder** (REMOTE §15.3; DESIGN §3.11): the entry's endpoints,
//! tried in order until one connects.
//!
//! ```text
//! the endpoint that answered last   — kept for the run, in RAM
//! every other endpoint the entry states, in the order it states them
//! ```
//!
//! **An entry with one endpoint has a one-rung ladder**: the connect either
//! lands or its sentence is the failure. A list exists for the box that
//! cannot reach the server's public name from where it stands — a router that
//! will not hairpin — so the operator states the LAN address second and a
//! foot at home falls through to it.
//!
//! **A rung is a TCP connect under a bound, and nothing more.** Whatever
//! answers at an endpoint still has to present the operator's CA in the
//! handshake that follows, so dialling a private address from somebody else's
//! network is harmless — and a handshake that fails there is the channel's
//! failure, not a reason to say the request again somewhere else.
//!
//! **What answered is kept, and only in RAM.** The next dial tries it first,
//! which is what keeps a foot at home from paying the public name's bound on
//! every ask; when it stops answering the rest are tried in the entry's own
//! order. Nothing is written back: the file is the operator's statement and
//! the run's finding is not.

use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

use super::Endpoint;
use rustls::pki_types::ServerName;

/// How long one connect may take before the next rung is tried. A NAT that
/// drops the SYN answers nothing, and the kernel's own patience is minutes; a
/// rung has to give up sooner than that to be a rung.
const BOUND: Duration = Duration::from_secs(10);

/// The rung that answered.
pub(super) struct Kept {
    /// Its place in the entry's list — what the channel tries first next time.
    pub(super) at: usize,
    /// The endpoint as the entry states it.
    pub(super) address: String,
    /// The name its certificate is verified against.
    pub(super) name: ServerName<'static>,
}

/// A connection to the server and the rung that gave it: `kept` first, then
/// the rest in the entry's order. No rung answering is every rung's sentence.
pub(super) fn climb(endpoints: &[Endpoint], kept: usize) -> Result<(TcpStream, Kept), String> {
    let first = endpoints.iter().enumerate().skip(kept).take(1);
    let rest = endpoints.iter().enumerate().filter(|(at, _)| *at != kept);
    let mut refused = Vec::new();
    for (at, (address, name)) in first.chain(rest) {
        match connect(address) {
            Ok(tcp) => {
                let (address, name) = (address.clone(), name.clone());
                return Ok((tcp, Kept { at, address, name }));
            }
            Err(sentence) => refused.push(sentence),
        }
    }
    Err(refused.join("; "))
}

/// One rung: every address the name resolves to, each under one bound, and
/// the sentence of the last that refused. The registration door is reached
/// through the same connect (`crate::register`).
pub(crate) fn connect(address: &str) -> Result<TcpStream, String> {
    let resolved = address
        .to_socket_addrs()
        .map_err(|e| format!("connect {address}: {e}"))?;
    let mut last = "no address resolved".to_owned();
    for at in resolved {
        match TcpStream::connect_timeout(&at, BOUND) {
            Ok(tcp) => return Ok(tcp),
            Err(e) => last = e.to_string(),
        }
    }
    Err(format!("connect {address}: {last}"))
}
