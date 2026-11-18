//! **The channel**: thrall's end of one wire to one server (yog's
//! `docs/REMOTE.md` §3, §5.4, §15.3; DESIGN §3.1, §3.11).
//!
//! **A foot dials and is never dialled.** Every leg is a reply to something
//! this end asked for, so there is no inbound direction to secure because there
//! is no inbound direction. That is not a property of this file — it is the
//! whole shape of it: there is a [`Channel::ask`] and there is nothing else.
//!
//! **One connection per ask, held only while waiting — and one per RUN, for a
//! run that is posted in pieces** (REMOTE §5.7; [`Channel::post`], [`Line`]).
//! A foot's life is one
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
//! the run, in RAM and never on disk, and is the first tried next time. An
//! endpoint is passed over when its socket will not open or its handshake
//! does not verify.
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
/// One open connection, and the requests it carries.
mod line;
/// What an entry holds.
pub mod material;
/// The mTLS configuration.
pub mod tls;

pub use failure::Failure;
pub(crate) use ladder::connect;
pub(crate) use line::Line;
use material::Material;

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
    /// The edition the server stated on the last dial (REMOTE §3.2), and the
    /// [`FLOOR`](hello::FLOOR) until one has: the least a server of this major
    /// can be spelling.
    edition: u32,
}

/// How many times a post that opens a run's connection climbs the ladder
/// before the server is judged unreachable for it (REMOTE §5.7: *"a server it
/// cannot reach for a post ends the child"*). Two, with no wait between: the
/// wait belongs to `run::redial`, and a run whose asker is listening on a
/// thirty-second clock has none to spend.
const REACHES: usize = 2;

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
            edition: hello::FLOOR,
        })
    }

    /// The client identity this channel presents — the leaf's own common name.
    pub fn client(&self) -> String {
        self.client.clone()
    }

    /// **Whether this server reads a capture posted in pieces** (REMOTE §5.7):
    /// its edition, as last stated, is at or above the stamp of `partial`.
    pub(crate) fn streams(&self) -> bool {
        self.edition >= hello::STREAMS
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
        self.dial(request, 1)?.answer()
    }

    /// **One request of a run, over the connection that run holds** (REMOTE
    /// §5.7: *"holds one connection for the posts of a run"*). `line` is the
    /// run's: empty before its first post, which dials; kept open after every
    /// answered one; and dropped with the run.
    ///
    /// **Only the dial is tried again, and only [`REACHES`] times.** A post is
    /// an append and states no offset, so one that may have landed — written,
    /// and then the answer lost — cannot be sent twice without the asker
    /// reading that text twice. A ladder that could not be climbed has sent
    /// nothing, and is the one failure a second attempt cannot duplicate.
    pub(crate) fn post(
        &mut self,
        line: &mut Option<Line>,
        request: &Value,
    ) -> Result<Vec<Value>, Failure> {
        let mut open = match line.take() {
            Some(mut held) => {
                held.send(request)?;
                held
            }
            None => self.dial(request, REACHES)?,
        };
        let stream = open.answer()?;
        *line = Some(open);
        Ok(stream)
    }

    /// Climb the ladder to a verified connection and send. The TLS handshake
    /// is the ladder's — an endpoint that cannot present the operator's CA is
    /// a rung passed over, not a failure of the dial — and the one frame read
    /// here is the server's version preface, so what this hands back is a
    /// [`Line`] with a request on it and no *answer* yet read.
    ///
    /// **Both ends state a version before either reads** (REMOTE §3), and the
    /// request goes out in the same breath as this end's preface — so
    /// confirming the server's costs no round trip, and a mismatch refuses
    /// before a frame of the answer is decoded. **The server's EDITION comes
    /// back from that confirmation and is kept** (REMOTE §3.2): it is what
    /// [`Channel::streams`] answers from.
    fn dial(&mut self, request: &Value, reaches: usize) -> Result<Line, Failure> {
        let (tls, kept) = self.climb(reaches).map_err(Failure::Wire)?;
        self.kept = kept.at;
        let (line, edition) = Line::open(tls, kept.address, request)?;
        self.edition = edition;
        Ok(line)
    }

    /// The ladder, climbed until it answers or `reaches` climbs have failed;
    /// the sentence is the last climb's.
    fn climb(&self, reaches: usize) -> Result<(Tls, ladder::Kept), String> {
        let mut climbed = ladder::climb(&self.config, &self.endpoints, self.kept);
        for _ in 1..reaches {
            if climbed.is_ok() {
                break;
            }
            climbed = ladder::climb(&self.config, &self.endpoints, self.kept);
        }
        climbed
    }
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
