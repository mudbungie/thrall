//! **The stand-in engine's punched end** (REMOTE §13.3–§13.4): a punch port
//! that listens, serves whatever lands on it over the same mTLS the dialled
//! stand-in serves, and — unlike that one — carries a whole script over ONE
//! connection, because a punched connection is held between asks.
//!
//! It never SYNs toward the foot: on loopback the foot's own SYN lands on
//! this listener, which is the degenerate case REMOTE §13.3 names (a
//! publicly reachable end is one whose peer's SYN simply lands). What it
//! stands in for is the engine's `punch.punch(call.endpoints, window)` and
//! the `serve` it hands each stream to.
//!
//! **Three turns, one per request read.** [`Turn::Answer`] writes frames and
//! the terminator, like a script entry of the dialled stand-in. [`Turn::Vanish`]
//! drops the connection where it stands — the flap — and the next request is
//! read off the next connection to land. [`Turn::Silence`] answers nothing
//! and waits for the foot to hang up, which is how the two-minute bound is
//! driven from this side.

use std::net::TcpStream;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};

use crate::channel::frame;
use crate::rendezvous::punch::Punch;

/// How long the stand-in keeps a punch window open waiting for a foot, and
/// how long it waits on a read before giving up on a foot that never asks
/// again. Both bound a stuck test, never a passing one.
const PATIENCE: Duration = Duration::from_secs(10);

/// What the engine does with the next request it reads.
pub(crate) enum Turn {
    /// Write these frames and the terminator.
    Answer(Vec<Value>),
    /// Drop the connection with nothing said, and take the next one to land.
    Vanish,
    /// Say nothing until the foot hangs up, then take the next connection.
    Silence,
}

/// The punched stand-in: its port, and what it was told.
pub(crate) struct Punched {
    port: u16,
    seen: Arc<Mutex<Vec<Value>>>,
}

impl Punched {
    /// Bind a punch port and serve `script` over the connections that land on
    /// it. `dir` holds the engine's own material, as [`mint`](super::super::mint)
    /// wrote it.
    pub(crate) fn start(dir: &Path, script: Vec<Turn>) -> Punched {
        let config = super::server_config(dir);
        let punch = Punch::bind(0).expect("a punch port");
        let port = punch.port();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        std::thread::spawn(move || serve(&punch, &config, script, &recorded));
        Punched { port, seen }
    }

    /// The port a presence item names for this engine.
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    /// Every frame it has been handed, in order and across connections — a
    /// preface per connection, then that connection's requests.
    pub(crate) fn heard(&self) -> Vec<Value> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Connections until the script is spent or nobody comes.
fn serve(
    punch: &Punch,
    config: &Arc<ServerConfig>,
    script: Vec<Turn>,
    seen: &Arc<Mutex<Vec<Value>>>,
) {
    let mut turns = script.into_iter();
    while let Some(tcp) = punch.punch(vec![], PATIENCE) {
        let _ = tcp.set_read_timeout(Some(PATIENCE));
        let mut tls = StreamOwned::new(
            ServerConnection::new(Arc::clone(config)).expect("a server connection"),
            tcp,
        );
        let _ = frame::write_value(
            &mut tls,
            &json!({ "protocol": crate::corpus::PROTOCOL, "edition": crate::corpus::ledger::FLOOR }),
        );
        // The foot's preface, once per connection.
        if !record(&mut tls, seen) {
            continue;
        }
        while record(&mut tls, seen) {
            match turns.next() {
                Some(Turn::Answer(frames)) => {
                    for value in &frames {
                        let _ = frame::write_value(&mut tls, value);
                    }
                    let _ = frame::write_end(&mut tls);
                }
                Some(Turn::Vanish) => break,
                Some(Turn::Silence) => {
                    let _ = frame::read_value(&mut tls);
                    break;
                }
                None => return,
            }
        }
    }
}

/// One frame off the foot, kept; `false` is the foot gone.
fn record(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
    seen: &Arc<Mutex<Vec<Value>>>,
) -> bool {
    let Ok(Some(said)) = frame::read_value(tls) else {
        return false;
    };
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push(said);
    true
}
