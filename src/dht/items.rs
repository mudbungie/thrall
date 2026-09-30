//! BEP 44 traffic: `get` reads the newest verified item under a key and salt,
//! `put` stores one at the nodes closest to its target — as `holders` and
//! `store`, the walk and the flight. Both verbs are one walk
//! (`lookup`) asking `get` — the walk that finds the closest nodes is the
//! walk that collects their write tokens — and `put` then spends those tokens
//! in one more flight, every holder at once, each with its own deadline. Nothing here trusts the commons: an item is a value
//! only once its signature verifies under the key the caller asked for.

use super::Dht;
use super::flight::Flight;
use super::krpc::{Message, Node, NodeId};
use super::mutable::{Mutable, target_of};

impl Dht {
    /// The newest item signed under `key` for `salt`, or nothing: no node
    /// held one, or none held one that verifies.
    pub(crate) fn get(&mut self, key: [u8; 32], salt: Vec<u8>) -> Result<Option<Mutable>, String> {
        let target = target_of(&key, &salt);
        let out = self.search(target, "get")?;
        Ok(out
            .replies
            .iter()
            .filter_map(|(_, r)| Mutable::from_reply(r, key, &salt))
            .max_by_key(|m| m.seq))
    }

    /// Store `item` at the K closest nodes that offered a write token; answers
    /// how many acknowledged — [`Dht::holders`] then [`Dht::store`], the
    /// suite's view of the two as one verb.
    #[cfg(test)]
    pub(crate) fn put(&mut self, item: Mutable) -> Result<usize, String> {
        let holders = self.holders(item.target())?;
        self.store(&item, holders)
    }

    /// **The walk a write needs**: the K nodes nearest `target` that offered a
    /// write token, each with its token. It is split from [`Dht::store`]
    /// because an item's target is its key and salt alone, so the walk can run
    /// before the value exists — and the walk's `ip` claims are then
    /// [`Dht::observed`] for a value that has to name where this client is
    /// seen (a call, thrall bl-536d). A walk that reached no token holder at
    /// all (every node near the target silent or refusing `get`) is its own
    /// error, sent nothing, and is not a silent `put` (yog bl-f519: the two
    /// read as one, and were mistaken for each other).
    pub(crate) fn holders(&mut self, target: NodeId) -> Result<Vec<(Node, Vec<u8>)>, String> {
        let out = self.search(target, "get")?;
        let holders: Vec<(Node, Vec<u8>)> = out
            .replies
            .iter()
            .filter_map(|(n, r)| Some((*n, r.get(b"token".as_slice())?.as_bytes()?.to_vec())))
            .take(self.config.k)
            .collect();
        if holders.is_empty() {
            return Err(format!("no DHT node near {target} offered a write token"));
        }
        Ok(holders)
    }

    /// Spend `holders`' tokens on `item` in one flight, every holder at once,
    /// each with its own deadline; answers how many acknowledged. Zero is an
    /// error naming the first refusal — a stale sequence number, say — or the
    /// silence.
    pub(crate) fn store(
        &mut self,
        item: &Mutable,
        holders: Vec<(Node, Vec<u8>)>,
    ) -> Result<usize, String> {
        let target = item.target();
        let mut flight = Flight::new();
        for (node, token) in holders {
            self.ask(&mut flight, node.addr, false, "put", item.put_args(&token));
        }
        let mut acks = 0usize;
        let mut refusals = Vec::new();
        while !flight.is_empty() {
            match self.land(&mut flight)? {
                Some((_, Message::Reply { .. })) => acks += 1,
                Some((query, Message::Error { code, message, .. })) => {
                    refusals.push(format!("{}: {code} {message}", query.addr));
                }
                None => {}
            }
        }
        if acks == 0 {
            return Err(refusals
                .into_iter()
                .next()
                .unwrap_or_else(|| format!("no DHT node stored the item at {target}")));
        }
        Ok(acks)
    }
}
