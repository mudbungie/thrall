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
//! **A rung is the connect AND the handshake, each under a bound** (bl-45e9).
//! A socket that opens proves only that something is there: a private
//! address dialled from somebody else's network may well be answered, and so
//! may any address behind a captive network. Whatever answers must present
//! the operator's CA, and what cannot is passed over exactly as a socket that
//! would not open is. **Nothing of the gesture has crossed by then** — the
//! handshake is driven to its end here, before this end's preface or its
//! request is written — so the next rung repeats nothing.
//!
//! **What answered is kept, and only in RAM.** The next dial tries it first,
//! which is what keeps a foot at home from paying the public name's bound on
//! every ask; when it stops answering the rest are tried in the entry's own
//! order. Nothing is written back: the file is the operator's statement and
//! the run's finding is not.

use std::net::{TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use super::{Endpoint, Tls};
use rustls::{ClientConfig, ClientConnection, StreamOwned};

/// How long one connect, and one read of the handshake after it, may take
/// before the next rung is tried. A NAT that drops the SYN answers nothing,
/// and so does a listener that takes the socket and never speaks; the
/// kernel's own patience is minutes, and a rung has to give up sooner than
/// that to be a rung.
const BOUND: Duration = Duration::from_secs(5);

/// The rung that answered.
pub(super) struct Kept {
    /// Its place in the entry's list — what the channel tries first next time.
    pub(super) at: usize,
    /// The endpoint as the entry states it.
    pub(super) address: String,
}

/// A verified connection to the server and the rung that gave it: `kept`
/// first, then the rest in the entry's order. No rung answering is every
/// rung's sentence.
pub(super) fn climb(
    config: &Arc<ClientConfig>,
    endpoints: &[Endpoint],
    kept: usize,
) -> Result<(Tls, Kept), String> {
    let first = endpoints.iter().enumerate().skip(kept).take(1);
    let rest = endpoints.iter().enumerate().filter(|(at, _)| *at != kept);
    let mut refused = Vec::new();
    for (at, endpoint) in first.chain(rest) {
        match rung(config, endpoint) {
            Ok(tls) => {
                let address = endpoint.0.clone();
                return Ok((tls, Kept { at, address }));
            }
            Err(sentence) => refused.push(sentence),
        }
    }
    Err(refused.join("; "))
}

/// One rung: the socket, then the mTLS over it verifying the name read off
/// that same endpoint, driven until both ends have finished — so what comes
/// back has been proved to be the server and has been told nothing.
///
/// The bound is left on the socket's reads; the channel sets its own before
/// it says anything.
fn rung(config: &Arc<ClientConfig>, (address, name): &Endpoint) -> Result<Tls, String> {
    let refused = |e| format!("handshake {address}: {e}");
    let tcp = connect(address)?;
    tcp.set_read_timeout(Some(BOUND)).map_err(refused)?;
    let conn = ClientConnection::new(Arc::clone(config), name.clone())
        .map_err(|e| format!("tls {address}: {e}"))?;
    let mut tls = StreamOwned::new(conn, tcp);
    while tls.conn.is_handshaking() {
        tls.conn.complete_io(&mut tls.sock).map_err(refused)?;
    }
    Ok(tls)
}

/// One socket: every address the name resolves to, each under one bound, and
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
