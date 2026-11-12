//! **The channel**: thrall's end of one wire to one server (yog's
//! `docs/REMOTE.md` §3, §5.4, §15.3; DESIGN §3.1, §3.11).
//!
//! **A foot dials and is never dialled.** Every leg is a reply to something
//! this end asked for, so there is no inbound direction to secure because there
//! is no inbound direction. That is not a property of this file — it is the
//! whole shape of it: there is a [`Channel::ask`] and there is nothing else.
//!
//! **One connection per ask, held only while waiting.** A foot's life is one
//! follow-class read (`invocations`, bl-a2ea) whose answer takes as long as it
//! takes, and a connection is held for exactly that. Between asks, and for the
//! whole time it is executing something, it is *absent* — which is why the
//! server does not treat presence as the routing predicate (REMOTE §5's
//! amendment: the mailbox queue is). Nothing is held across asks: the server
//! listens on a public port (REMOTE §15), so a dial costs milliseconds and
//! there is no connection dear enough to keep.
//!
//! **The entry states a list of endpoints and the dial tries them in order**
//! (REMOTE §15.3; [`ladder`]): the one that answered is kept for the rest of
//! the run, in RAM and never on disk, and is the first tried next time.
//!
//! **It knows nothing about being dialled again** (DESIGN §3.8). A channel
//! that fails answers the sentence that failed it, and what happens next is
//! `run::redial`'s: a dropped wire is taken up again after a wait, through the
//! same list, and a foot that cannot be a foot at all still exits.
//!
//! **The server's name comes from the endpoint dialled and from nowhere
//! else.** A dotted quad or a bracketed v6 literal is verified as an IP address
//! — the server's leaf must carry the matching `IP:` subject alternative name
//! — and anything else is a DNS name: there is nothing to configure and nothing
//! that can disagree with what was dialled.

use std::net::{IpAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use serde_json::Value;

/// Every channel this box holds, as the operator filed them.
pub mod entries;
/// Why a channel could not carry a gesture, in the two classes that matter.
mod failure;
/// The wire's framing.
pub mod frame;
/// The version preface.
pub mod hello;
/// The dial ladder: the entry's endpoints, tried in order.
mod ladder;
/// The foot grade, read off this box's own leaf.
pub mod leaf;
/// What an entry holds.
pub mod material;
/// The mTLS configuration.
pub mod tls;

pub use failure::Failure;
pub(crate) use ladder::connect;
use material::Material;

/// How long one read may wait before the channel is judged gone.
///
/// It is a bound on the **transport**, not on the wait: the server parks a
/// follow-class read for its mailbox's own hold and then answers — with no work
/// if there was none — so a foot waiting for hours is a *sequence* of answered
/// reads, never one read held for hours. This has to sit comfortably above that
/// hold, because a read timeout below it would turn the server's ordinary empty
/// answer into a dead channel. It is a socket bound and not a clock, because
/// rustls has no clean resume from a half-read record — a timeout is "the
/// connection is gone", never a retry.
const READ_TIMEOUT: Duration = Duration::from_mins(2);

/// One endpoint of the entry's list: the `host:port` as stated, and the name
/// its certificate is verified against.
type Endpoint = (String, ServerName<'static>);

/// One connection's TLS stream.
type Tls = StreamOwned<ClientConnection, TcpStream>;

/// A foot's end of one wire.
#[derive(Debug)]
pub struct Channel {
    config: Arc<ClientConfig>,
    /// The endpoints the entry states, in its order, each with the name its
    /// certificate is verified against.
    endpoints: Vec<Endpoint>,
    /// The endpoint that answered last — tried first next time. RAM for the
    /// run, never disk (REMOTE §15.3).
    kept: usize,
    client: String,
}

impl Channel {
    /// Open the channel from provisioned material. **Nothing is dialled here**:
    /// a channel is a fact about what this box may say, not about whether a
    /// server happens to be up.
    ///
    /// The grade is read first, because a leaf that is not a foot's is a
    /// refusal about *this box's configuration* and has nothing to do with any
    /// server — it must not arrive as a connection failure.
    pub fn open(m: &Material) -> Result<Self, String> {
        let client = leaf::foot(&m.chain)?;
        let named = |address: &String| Ok((address.clone(), server_name(address)?));
        Ok(Self {
            config: tls::client_config(m)?,
            endpoints: m.address.iter().map(named).collect::<Result<_, String>>()?,
            kept: 0,
            client,
        })
    }

    /// The client identity this channel presents — the leaf's own common name.
    pub fn client(&self) -> String {
        self.client.clone()
    }

    /// Send one request and read its whole answer: every frame up to the
    /// terminator. A stream of one is the ordinary answer, and a stream of
    /// several is a follow-class read — the same reader, which is REMOTE §3's
    /// *"the streaming form is not a second form"*.
    ///
    /// **The `Err` is [`Failure`]**, which is two classes and not a taxonomy: a
    /// socket that never opened, a server that went away and an unreadable
    /// answer are one fact to the loop above — this channel is gone, and here
    /// is the sentence — while a version the two ends do not share is the one
    /// fact that is about the two *binaries* and survives every dial.
    ///
    /// Nothing here retries inside one ask.
    pub fn ask(&mut self, request: &Value) -> Result<Vec<Value>, Failure> {
        let (mut tls, address) = self.dial(request)?;
        let mut stream = Vec::new();
        loop {
            match frame::read_value(&mut tls) {
                Ok(Some(chunk)) => stream.push(chunk),
                Ok(None) => return Ok(stream),
                Err(e) => return Err(Failure::Wire(failed("receive", &address, &e))),
            }
        }
    }

    /// Climb the ladder to a connection, handshake and send. The TLS handshake
    /// happens inside the first write, and the one frame read here is the
    /// server's version preface — so what this hands back is a stream with a
    /// request on it and no *answer* yet read.
    ///
    /// **Both ends state a version before either reads** (REMOTE §3), and the
    /// request goes out in the same breath as this end's preface — so
    /// confirming the server's costs no round trip, and a mismatch refuses
    /// before a frame of the answer is decoded. **The server's EDITION comes
    /// back from that confirmation and is dropped here** (REMOTE §3.2): a foot
    /// reads no post-floor field, so nothing above consults it yet.
    fn dial(&mut self, request: &Value) -> Result<(Tls, String), Failure> {
        let (tcp, kept) = ladder::climb(&self.endpoints, self.kept).map_err(Failure::Wire)?;
        self.kept = kept.at;
        let address = kept.address;
        tcp.set_read_timeout(Some(READ_TIMEOUT))
            .map_err(|e| Failure::Wire(format!("connect {address}: {e}")))?;
        let conn = ClientConnection::new(Arc::clone(&self.config), kept.name)
            .map_err(|e| Failure::Wire(format!("tls {address}: {e}")))?;
        let mut tls = StreamOwned::new(conn, tcp);
        hello::state(&mut tls).map_err(|e| Failure::Wire(failed("send", &address, &e)))?;
        frame::write_value(&mut tls, request)
            .map_err(|e| Failure::Wire(failed("send", &address, &e)))?;
        hello::confirm(&mut tls)?;
        Ok((tls, address))
    }
}

/// **A channel that failed, in this box's own words** (bl-52ba).
///
/// The sentence IS the product on this path: what a supervisor's log or an
/// operator's terminal carries is the whole of what they get — and what
/// they need from it is which server went away and whether anything is
/// going to happen next. A library's own diagnosis answers neither: it
/// names no address, and "peer closed connection without sending TLS
/// close_notify" is a fact about TLS rather than about what to do. **It
/// follows the sentence rather than replacing it**, because it is the right
/// text for the one reader who wants it and the wrong text for the one who
/// has to act. What this box will do next is `run::redial`'s to say in the
/// same breath (bl-916d).
///
/// `leg` is the half that failed, so this reads like the connect refusals
/// beside it: the act, the endpoint that was dialled, then what happened.
fn failed(leg: &str, address: &str, e: &std::io::Error) -> String {
    format!(
        "{leg} {address}: the channel to the server failed. What the {leg} \
         reported: {e}"
    )
}

/// The name to verify the server's certificate against, read off the address.
pub(crate) fn server_name(address: &str) -> Result<ServerName<'static>, String> {
    let host = address.rsplit_once(':').map_or(address, |(head, _)| head);
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ServerName::IpAddress(ip.into()));
    }
    ServerName::try_from(host.to_owned()).map_err(|e| format!("{address}: not a server name: {e}"))
}

#[cfg(test)]
mod tests;
