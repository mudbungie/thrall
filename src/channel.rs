//! **The channel**: thrall's end of one wire to one engine (yog's
//! `docs/REMOTE.md` §3, §5.4, §13.4; DESIGN §3.1, §3.11).
//!
//! **A foot dials and is never dialled.** Every leg is a reply to something
//! this end asked for, so there is no inbound direction to secure because there
//! is no inbound direction. That is not a property of this file — it is the
//! whole shape of it: there is a [`Channel::ask`] and there is nothing else.
//!
//! **One connection per ask, held only while waiting — where a dial costs
//! milliseconds.** A foot's life is one follow-class read (`invocations`,
//! bl-a2ea) whose answer takes as long as it takes, and a dialled connection is
//! held for exactly that. Between asks, and for the whole time it is executing
//! something, it is *absent* — which is why the engine does not treat presence
//! as the routing predicate (REMOTE §5's amendment: the mailbox queue is).
//!
//! **A punched connection is held across asks** (REMOTE §13.4), because it
//! cost seconds — a presence read, an inbox write, a punch window — and §10's
//! criterion for a held connection is met in its own terms. What is held is
//! exactly what was punched: the ladder ([`ladder`]) says which kind of
//! connection it climbed to, a dialled one is spent with its ask, and a punched
//! one is kept for the next. The engine pings a held connection through its
//! silence and the ping is discarded wherever a reply is read; two minutes of
//! nothing is the hangup, and it ends the conversation like any other wire
//! failure — which is to say it is dialled again.
//!
//! **It knows nothing about being dialled again** (DESIGN §3.8). A channel
//! that fails answers the sentence that failed it, and what happens next is
//! `run::redial`'s: a dropped wire is taken up again after a wait, through the
//! same ladder, and a foot that cannot be a foot at all still exits.
//!
//! **The engine's name comes from the address and from nowhere else.** A dotted
//! quad or a bracketed v6 literal is verified as an IP address — the engine's
//! leaf must carry the matching `IP:` subject alternative name — and anything
//! else is a DNS name. Whichever rung answered, that is the name the inner
//! mTLS verifies (REMOTE §13.3): there is nothing to configure and nothing
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
/// The dial ladder: how a connection is obtained, rung by rung.
mod ladder;
/// The foot grade, read off this box's own leaf.
pub mod leaf;
/// What the operator carried to this box.
pub mod material;
/// The mTLS configuration.
pub mod tls;

use crate::rendezvous::call::{Roving, Tuning};
pub use failure::Failure;
use material::Material;

/// How long one read may wait before the channel is judged gone.
///
/// It is a bound on the **transport**, not on the wait: the engine parks a
/// follow-class read for its mailbox's own hold and then answers — with no work
/// if there was none — so a foot waiting for hours is a *sequence* of answered
/// reads, never one read held for hours. This has to sit comfortably above that
/// hold, because a read timeout below it would turn the engine's ordinary empty
/// answer into a dead channel. **On a held connection it is the hangup** the
/// engine states from its end too (REMOTE §13.4): the engine pings every
/// twenty-five seconds of silence, so a live connection never reaches it, and
/// one that does is gone. It is a socket bound and not a clock, because rustls
/// has no clean resume from a half-read record — a timeout is "the connection
/// is gone", never a retry.
const READ_TIMEOUT: Duration = Duration::from_mins(2);

/// One connection's TLS stream.
type Tls = StreamOwned<ClientConnection, TcpStream>;

/// A foot's end of one wire.
#[derive(Debug)]
pub struct Channel {
    config: Arc<ClientConfig>,
    address: String,
    name: ServerName<'static>,
    client: String,
    /// The roving half, where the entry carries rendezvous material.
    roving: Option<Roving>,
    /// The punched connection being held between asks, if there is one.
    held: Option<Tls>,
    quiet: Duration,
}

impl Channel {
    /// Open the channel from provisioned material. **Nothing is dialled here**:
    /// a channel is a fact about what this box may say, not about whether an
    /// engine happens to be up — and nothing is bound either, the punch port
    /// being taken on the first rendezvous.
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
            roving: m
                .rendezvous
                .clone()
                .map(|pairing| Roving::new(pairing, Tuning::default())),
            held: None,
            quiet: READ_TIMEOUT,
        })
    }

    /// The suite's knobs: the silence bound, and the rendezvous tuning where
    /// the entry roves. Production runs on the defaults and has no caller.
    #[cfg(test)]
    pub(crate) fn tune(&mut self, quiet: Duration, tuning: Tuning) {
        self.quiet = quiet;
        if let Some(roving) = self.roving.as_mut() {
            roving.tuning = tuning;
        }
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
    ///
    /// **A held connection is spoken on first and dropped on any failure**: a
    /// failure there is the wire, the next dial climbs the ladder, and nothing
    /// here retries inside one ask.
    pub fn ask(&mut self, request: &Value) -> Result<Vec<Value>, Failure> {
        let (mut tls, punched) = match self.held.take() {
            Some(mut tls) => {
                frame::write_value(&mut tls, request)
                    .map_err(|e| Failure::Wire(self.failed("send", &e)))?;
                (tls, true)
            }
            None => self.dial(request)?,
        };
        let answer = self.answer(&mut tls)?;
        if punched {
            self.held = Some(tls);
        }
        Ok(answer)
    }

    /// Every frame of one answer, in order, up to the terminator — and never
    /// a ping, which the engine writes into a held connection's silence
    /// (REMOTE §13.4) and which is never the start of a reply stream.
    fn answer(&self, tls: &mut Tls) -> Result<Vec<Value>, Failure> {
        let mut stream = Vec::new();
        loop {
            match frame::read_value(tls).map_err(|e| Failure::Wire(self.failed("receive", &e)))? {
                Some(chunk) if is_ping(&chunk) => {}
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
    /// has to act. What this box will do next is `run::redial`'s to say in the
    /// same breath (bl-916d).
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

    /// Climb the ladder to a connection, handshake and send. The TLS handshake
    /// happens inside the first write, and the one frame read here is the
    /// engine's version preface — so what this hands back is a stream with a
    /// request on it and no *answer* yet read, and whether it was punched.
    ///
    /// **Both ends state a version before either reads** (REMOTE §3), and the
    /// request goes out in the same breath as this end's preface — so
    /// confirming the engine's costs no round trip, and a mismatch refuses
    /// before a frame of the answer is decoded. **The engine's EDITION comes
    /// back from that confirmation and is dropped here** (REMOTE §3.2): a foot
    /// reads no post-floor field, so nothing above consults it yet.
    fn dial(&mut self, request: &Value) -> Result<(Tls, bool), Failure> {
        let (tcp, punched) =
            ladder::climb(&self.address, self.roving.as_mut()).map_err(Failure::Wire)?;
        tcp.set_read_timeout(Some(self.quiet))
            .map_err(|e| Failure::Wire(format!("connect {}: {e}", self.address)))?;
        let conn = ClientConnection::new(Arc::clone(&self.config), self.name.clone())
            .map_err(|e| Failure::Wire(format!("tls {}: {e}", self.address)))?;
        let mut tls = StreamOwned::new(conn, tcp);
        hello::state(&mut tls).map_err(|e| Failure::Wire(self.failed("send", &e)))?;
        frame::write_value(&mut tls, request)
            .map_err(|e| Failure::Wire(self.failed("send", &e)))?;
        hello::confirm(&mut tls)?;
        Ok((tls, punched))
    }
}

/// The engine's keepalive on a held connection: `{"ping":true}` and nothing
/// else (yog `wire::server::peer`).
fn is_ping(frame: &Value) -> bool {
    frame.get("ping").and_then(Value::as_bool) == Some(true)
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
