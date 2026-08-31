//! **The channel**: thrall's end of one wire to one engine (yog's
//! `docs/REMOTE.md` §3, §5.4; DESIGN §3.1).
//!
//! **A foot dials and is never dialled.** Every leg is a reply to something
//! this end asked for, so there is no inbound direction to secure because there
//! is no inbound direction. That is not a property of this file — it is the
//! whole shape of it: there is a [`Channel::ask`] and there is nothing else.
//!
//! **One connection per ask, held only while waiting.** A foot's life is one
//! follow-class read (`invocations`, bl-a2ea) whose answer takes as long as it
//! takes, and it holds a connection for exactly that. Between asks, and for the
//! whole time it is executing something, it is *absent* — which is why the
//! engine does not treat presence as the routing predicate (REMOTE §5's
//! amendment: the mailbox queue is).
//!
//! **It never reconnects** (DESIGN §2). A channel that fails answers the
//! sentence that failed it and the loop above ends. Restart policy belongs to
//! the supervision the operator's machine already has, and inventing one here
//! would be thrall deciding how a box it does not administer runs a program.
//!
//! **The engine's name comes from the address and from nowhere else.** A dotted
//! quad or a bracketed v6 literal is verified as an IP address — the engine's
//! leaf must carry the matching `IP:` subject alternative name — and anything
//! else is a DNS name. There is nothing to configure and nothing that can
//! disagree with what was dialled.

use std::net::{IpAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use rustls::pki_types::ServerName;
use rustls::{ClientConfig, ClientConnection, StreamOwned};
use serde_json::Value;

/// Every channel this box holds, as the operator filed them.
pub mod entries;
/// The wire's framing.
pub mod frame;
/// The version preface.
pub mod hello;
/// The foot grade, read off this box's own leaf.
pub mod leaf;
/// What the operator carried to this box.
pub mod material;
/// The mTLS configuration.
pub mod tls;

use material::Material;

/// How long one read may wait before the channel is judged gone.
///
/// It is a bound on the **transport**, not on the wait: the engine parks a
/// follow-class read for its mailbox's own hold and then answers — with no work
/// if there was none — so a foot waiting for hours is a *sequence* of answered
/// reads, never one read held for hours. This has to sit comfortably above that
/// hold, because a read timeout below it would turn the engine's ordinary empty
/// answer into a dead channel.
const READ_TIMEOUT: Duration = Duration::from_mins(2);

/// A foot's end of one wire.
#[derive(Debug)]
pub struct Channel {
    config: Arc<ClientConfig>,
    address: String,
    name: ServerName<'static>,
    client: String,
}

impl Channel {
    /// Open the channel from provisioned material. **Nothing is dialled here**:
    /// a channel is a fact about what this box may say, not about whether an
    /// engine happens to be up.
    ///
    /// The grade is read first, because a leaf that is not a foot's is a
    /// refusal about *this box's configuration* and has nothing to do with any
    /// engine — it must not arrive as a connection failure.
    pub fn open(m: &Material) -> Result<Self, String> {
        let client = leaf::foot(&m.chain)?;
        Ok(Self {
            config: tls::client_config(m)?,
            address: m.address.clone(),
            name: server_name(&m.address)?,
            client,
        })
    }

    /// The client identity this channel presents — the leaf's own common name.
    pub fn client(&self) -> String {
        self.client.clone()
    }

    /// The address it dials.
    pub fn address(&self) -> String {
        self.address.clone()
    }

    /// Send one request and read its whole answer: every frame up to the
    /// terminator. A stream of one is the ordinary answer, and a stream of
    /// several is a follow-class read — the same reader, which is REMOTE §3's
    /// *"the streaming form is not a second form"*.
    ///
    /// One `Err` for a refusal, an unreadable answer and a socket that never
    /// opened alike: all three are the same fact to the loop above — this
    /// channel is gone, and here is the sentence.
    pub fn ask(&self, request: &Value) -> Result<Vec<Value>, String> {
        let mut tls = self.dial(request)?;
        let mut stream = Vec::new();
        loop {
            match frame::read_value(&mut tls).map_err(|e| self.failed("receive", &e))? {
                Some(chunk) => stream.push(chunk),
                None => return Ok(stream),
            }
        }
    }

    /// **A channel that failed, in this box's own words** (bl-52ba).
    ///
    /// The sentence IS the product on this path: a foot never reconnects
    /// (DESIGN §2), so what a supervisor's log carries is the whole of what an
    /// operator gets — and what they need from it is which engine went away and
    /// whether anything is going to happen next. A library's own diagnosis
    /// answers neither: it names no address, and "peer closed connection
    /// without sending TLS close_notify" is a fact about TLS rather than about
    /// what to do. **It follows the sentence rather than replacing it**, because
    /// it is the right text for the one reader who wants it and the wrong text
    /// for the one who has to act.
    ///
    /// `leg` is the half that failed, so this reads like the connect refusals
    /// beside it: the act, the address, then what happened.
    fn failed(&self, leg: &str, e: &std::io::Error) -> String {
        format!(
            "{leg} {}: the channel to the engine failed, and thrall does not \
             reconnect — bringing this foot back is the supervision this machine \
             already has. What the {leg} reported: {e}",
            self.address
        )
    }

    /// Connect, handshake and send. The TLS handshake happens inside the first
    /// write, and the one frame read here is the engine's version preface — so
    /// what this hands back is a socket with a request on it and no *answer*
    /// yet read.
    ///
    /// **Both ends state a version before either reads** (REMOTE §3), and the
    /// request goes out in the same breath as this end's preface — so
    /// confirming the engine's costs no round trip, and a mismatch refuses
    /// before a frame of the answer is decoded.
    fn dial(&self, request: &Value) -> Result<StreamOwned<ClientConnection, TcpStream>, String> {
        let tcp = TcpStream::connect(&self.address)
            .and_then(|tcp| tcp.set_read_timeout(Some(READ_TIMEOUT)).map(|()| tcp))
            .map_err(|e| format!("connect {}: {e}", self.address))?;
        let conn = ClientConnection::new(Arc::clone(&self.config), self.name.clone())
            .map_err(|e| format!("tls {}: {e}", self.address))?;
        let mut tls = StreamOwned::new(conn, tcp);
        hello::state(&mut tls).map_err(|e| self.failed("send", &e))?;
        frame::write_value(&mut tls, request).map_err(|e| self.failed("send", &e))?;
        hello::confirm(&mut tls)?;
        Ok(tls)
    }
}

/// The name to verify the engine's certificate against, read off the address.
fn server_name(address: &str) -> Result<ServerName<'static>, String> {
    let host = address.rsplit_once(':').map_or(address, |(head, _)| head);
    let host = host.trim_start_matches('[').trim_end_matches(']');
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ServerName::IpAddress(ip.into()));
    }
    ServerName::try_from(host.to_owned()).map_err(|e| format!("{address}: not a server name: {e}"))
}

#[cfg(test)]
mod tests;
