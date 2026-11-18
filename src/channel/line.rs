//! **One open connection, and the requests it carries** (yog's
//! `docs/REMOTE.md` §3, §5.7).
//!
//! The server answers request after request on a connection — one frame in, a
//! stream and its terminator out, and then it reads again. A foot spends that
//! twice: an ordinary ask is a line with one request on it, dropped when the
//! answer is read, and **the posts of one run share one line** (REMOTE §5.7),
//! because a run that speaks every few seconds would otherwise pay a handshake
//! per sentence.
//!
//! It is split from [`channel`](super) on the seam between *which* server and
//! *this* connection to it: the ladder, the kept endpoint and the edition are
//! the channel's and outlive every line.

use std::time::Duration;

use serde_json::Value;

use super::{Failure, Tls, frame, hello};

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

/// One verified connection to the server, past both prefaces.
pub(crate) struct Line {
    tls: Tls,
    /// The endpoint it was dialled at, for the sentence a failure earns.
    address: String,
}

impl Line {
    /// Take up a connection the ladder climbed to: bound its reads, state
    /// this end's version, send `request` in the same breath, and confirm the
    /// server's. What comes back beside the line is the server's EDITION.
    pub(super) fn open(tls: Tls, address: String, request: &Value) -> Result<(Self, u32), Failure> {
        let mut line = Self { tls, address };
        let bounded = line.tls.sock.set_read_timeout(Some(READ_TIMEOUT));
        bounded.map_err(|e| Failure::Wire(format!("connect {}: {e}", line.address)))?;
        hello::state(&mut line.tls).map_err(|e| line.failed("send", &e))?;
        line.send(request)?;
        let edition = hello::confirm(&mut line.tls)?;
        Ok((line, edition))
    }

    /// Write one more request on this connection.
    pub(super) fn send(&mut self, request: &Value) -> Result<(), Failure> {
        frame::write_value(&mut self.tls, request).map_err(|e| self.failed("send", &e))
    }

    /// Read the answer to the request last written: every frame up to the
    /// terminator.
    pub(super) fn answer(&mut self) -> Result<Vec<Value>, Failure> {
        let mut stream = Vec::new();
        loop {
            match frame::read_value(&mut self.tls) {
                Ok(Some(chunk)) => stream.push(chunk),
                Ok(None) => return Ok(stream),
                Err(e) => return Err(self.failed("receive", &e)),
            }
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
    fn failed(&self, leg: &str, e: &std::io::Error) -> Failure {
        let address = &self.address;
        Failure::Wire(format!(
            "{leg} {address}: the channel to the server failed. What the {leg} \
             reported: {e}"
        ))
    }
}
