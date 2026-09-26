//! The window a walk keeps in the air (yog bl-d9c1): each query goes out with its
//! own deadline, and the caller waits for ONE event at a time — the first
//! answer to land or the nearest deadline to pass — so a silent node costs
//! its own slot and never anyone else's wait. `lookup` refills the window on
//! every event; `items` drains it for `put`.

use super::Dht;
use super::bencode::Dict;
use super::krpc::{self, Message};
use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::time::Instant;

/// One query in the air: where it went, whether it went to the door (the
/// bootstrap), and when it stops being waited for.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Query {
    pub(crate) addr: SocketAddr,
    pub(crate) door: bool,
    deadline: Instant,
}

/// The queries in the air, by transaction id.
pub(crate) type Flight = BTreeMap<Vec<u8>, Query>;

impl Dht {
    /// Send one query and, if the send itself went, put it in the air with
    /// `Config::deadline` to answer in. Answers whether it went.
    pub(crate) fn ask(
        &mut self,
        flight: &mut Flight,
        addr: SocketAddr,
        door: bool,
        q: &str,
        args: Dict,
    ) -> bool {
        let tid = self.next_tid();
        let datagram = krpc::query(&tid, &self.id, q, args);
        let sent = self.transport.send(addr, &datagram).is_ok();
        if sent {
            let deadline = Instant::now() + self.config.deadline;
            flight.insert(
                tid,
                Query {
                    addr,
                    door,
                    deadline,
                },
            );
        }
        sent
    }

    /// Wait for the flight's next event: `Some` an answer to one of its
    /// queries, which has left the flight, or `None` — the nearest deadline
    /// passed and every expired query has left it (or it was empty). The
    /// socket's read timeout is always the nearest outstanding deadline;
    /// noise, and answers to transactions not in the air, are read past.
    pub(crate) fn land(&mut self, flight: &mut Flight) -> Result<Option<(Query, Message)>, String> {
        loop {
            let now = Instant::now();
            let before = flight.len();
            flight.retain(|_, q| q.deadline > now);
            let nearest = flight.values().map(|q| q.deadline).min();
            let Some(nearest) = nearest.filter(|_| flight.len() == before) else {
                return Ok(None);
            };
            let Some((_, bytes)) = self
                .transport
                .recv(nearest.saturating_duration_since(now))
                .map_err(|e| format!("DHT socket: {e}"))?
            else {
                continue;
            };
            let Some(message) = krpc::parse(&bytes) else {
                continue;
            };
            let (Message::Reply { tid, .. } | Message::Error { tid, .. }) = &message;
            if let Some(query) = flight.remove(tid) {
                return Ok(Some((query, message)));
            }
        }
    }
}
