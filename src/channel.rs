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
//! **It knows nothing about being dialled again** (DESIGN §3.8). A channel
//! that fails answers the sentence that failed it, and what happens next is
//! `run::redial`'s: a dropped wire is taken up again after a wait, and a foot
//! that cannot be a foot at all still exits, because restarting a *process*
//! belongs to the supervision the operator's machine already has. Nothing here
//! holds a connection or a retry, which is why re-dialling costs this file no
//! state at all — [`Channel::ask`] already dials per ask.
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

/// **Why a channel could not carry a gesture** — in the two classes that
/// differ in what to do next, and in nothing else.
///
/// The split is drawn from *what failed*, never from the sentence: a foot that
/// decided its own lifetime by reading prose would be a foot the far end could
/// rewrite by rewording.
#[derive(Debug, PartialEq, Eq)]
pub enum Failure {
    /// **The transport.** It carries no opinion of either binary — a socket
    /// that would not open, an engine that went away, a peer that hung up
    /// before it had said anything — so the same ask down a fresh connection
    /// may well land.
    Wire(String),
    /// **The two ends do not speak one protocol version.** REMOTE §3 admits no
    /// negotiation and names the remedy in the sentence itself — *"upgrade the
    /// older component"* — so this is the one failure a further dial cannot
    /// improve: only a new binary on one of the two boxes can, and until one
    /// arrives every dial buys the same handshake and the same sentence.
    Skew(String),
}

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
    /// **The `Err` is [`Failure`]**, which is two classes and not a taxonomy: a
    /// socket that never opened, an engine that went away and an unreadable
    /// answer are one fact to the loop above — this channel is gone, and here
    /// is the sentence — while a version the two ends do not share is the one
    /// fact that is about the two *binaries* and survives every dial.
    pub fn ask(&self, request: &Value) -> Result<Vec<Value>, Failure> {
        let mut tls = self.dial(request)?;
        let mut stream = Vec::new();
        loop {
            match frame::read_value(&mut tls)
                .map_err(|e| Failure::Wire(self.failed("receive", &e)))?
            {
                Some(chunk) => stream.push(chunk),
                None => return Ok(stream),
            }
        }
    }

    /// **A channel that failed, in this box's own words** (bl-52ba).
    ///
    /// The sentence IS the product on this path: what a supervisor's log or an
    /// operator's terminal carries is the whole of what they get — and what
    /// they need from it is which engine went away and whether anything is
    /// going to happen next. A library's own diagnosis answers neither: it
    /// names no address, and "peer closed connection without sending TLS
    /// close_notify" is a fact about TLS rather than about what to do. **It
    /// follows the sentence rather than replacing it**, because it is the right
    /// text for the one reader who wants it and the wrong text for the one who
    /// has to act.
    ///
    /// **This half says WHICH engine and WHAT happened; the other half — what
    /// this box will do next — moved out** (bl-916d). It used to be stated
    /// here, as *"thrall does not reconnect"*, and a file that dials per ask
    /// and holds nothing cannot know that any more: a dropped wire is dialled
    /// again, and how soon is `run::redial`'s to say in the same breath as this
    /// sentence.
    ///
    /// `leg` is the half that failed, so this reads like the connect refusals
    /// beside it: the act, the address, then what happened.
    fn failed(&self, leg: &str, e: &std::io::Error) -> String {
        format!(
            "{leg} {}: the channel to the engine failed. What the {leg} \
             reported: {e}",
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
    ///
    /// **The engine's EDITION comes back from that confirmation and is dropped
    /// here** (REMOTE §3.2). It is the fact that says which post-floor fields
    /// the far end can spell, and a foot reads none: every path in its vendored
    /// ledger is at or under the floor, so `stamp <= engine edition` holds for
    /// every field it decodes on every engine of this major. Threading a value
    /// no reader consults through the loop would be mechanism with no consumer;
    /// the day a foot shape gains a post-floor field, this line is where it is
    /// picked up.
    fn dial(&self, request: &Value) -> Result<StreamOwned<ClientConnection, TcpStream>, Failure> {
        let tcp = TcpStream::connect(&self.address)
            .and_then(|tcp| tcp.set_read_timeout(Some(READ_TIMEOUT)).map(|()| tcp))
            .map_err(|e| Failure::Wire(format!("connect {}: {e}", self.address)))?;
        let conn = ClientConnection::new(Arc::clone(&self.config), self.name.clone())
            .map_err(|e| Failure::Wire(format!("tls {}: {e}", self.address)))?;
        let mut tls = StreamOwned::new(conn, tcp);
        hello::state(&mut tls).map_err(|e| Failure::Wire(self.failed("send", &e)))?;
        frame::write_value(&mut tls, request)
            .map_err(|e| Failure::Wire(self.failed("send", &e)))?;
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
