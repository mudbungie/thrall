//! BEP 44 traffic: `get` reads the newest verified item under a key and salt,
//! `put` stores one at the nodes closest to its target. Both are one walk
//! (`lookup`) asking `get` — the walk that finds the closest nodes is the
//! walk that collects their write tokens — and `put` then spends those tokens
//! in one more round. Nothing here trusts the commons: an item is a value
//! only once its signature verifies under the key the caller asked for.

use super::Dht;
use super::bencode::{Dict, bytes, entry};
use super::krpc::{Message, Node};
use super::lookup::Pending;
use super::mutable::{Mutable, target_of};

impl Dht {
    /// The newest item signed under `key` for `salt`, or nothing: no node
    /// held one, or none held one that verifies.
    pub(crate) fn get(&mut self, key: [u8; 32], salt: Vec<u8>) -> Result<Option<Mutable>, String> {
        let target = target_of(&key, &salt);
        let out = self.search(
            target,
            "get",
            Dict::from([entry("target", bytes(&target.0))]),
        )?;
        Ok(out
            .replies
            .iter()
            .filter_map(|(_, r)| Mutable::from_reply(r, key, &salt))
            .max_by_key(|m| m.seq))
    }

    /// Store `item` at the K closest nodes that offered a write token; answers
    /// how many acknowledged. Zero is an error naming the first refusal —
    /// a stale sequence number, say — or the silence.
    pub(crate) fn put(&mut self, item: Mutable) -> Result<usize, String> {
        let target = item.target();
        let out = self.search(
            target,
            "get",
            Dict::from([entry("target", bytes(&target.0))]),
        )?;
        let holders: Vec<(Node, Vec<u8>)> = out
            .replies
            .iter()
            .filter_map(|(n, r)| Some((*n, r.get(b"token".as_slice())?.as_bytes()?.to_vec())))
            .take(self.config.k)
            .collect();
        let mut pending = Pending::new();
        for (node, token) in holders {
            self.ask(&mut pending, node.addr, "put", item.put_args(&token));
        }
        let mut acks = 0usize;
        let mut refusals = Vec::new();
        self.collect(&mut pending, &mut |addr, message| match message {
            Message::Reply { .. } => acks += 1,
            Message::Error { code, message, .. } => {
                refusals.push(format!("{addr}: {code} {message}"));
            }
        })?;
        if acks == 0 {
            return Err(refusals
                .into_iter()
                .next()
                .unwrap_or_else(|| format!("no DHT node stored the item at {target}")));
        }
        Ok(acks)
    }
}
