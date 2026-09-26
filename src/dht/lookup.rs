//! The iterative lookup (BEP 5), the one walk every read and write of the
//! commons makes: ask the closest nodes you know, learn closer ones from
//! their answers, repeat until the K closest have all been asked. The walk
//! is a sliding window, not lockstep rounds (yog bl-d9c1): up to α queries in the
//! air, each with its own deadline, and the next node asked the moment any
//! of them answers or times out — so a silent node costs its own slot for
//! one deadline and never delays an answer beside it. A node that stays
//! silent is never asked again and leaves the frontier. Bounded twice over
//! against a hostile commons: a query ends at its deadline, and the walk at
//! `max_queries` however many "closer" nodes the answers keep inventing.
//! yog's walk also keeps each answering node's BEP 42 `ip` claim for its
//! observed-address vote; a foot publishes no presence, so it has no vote
//! and reads no claim.

use super::Dht;
use super::bencode::{Dict, bytes, entry};
use super::flight::Flight;
pub(crate) use super::frontier::Outcome;
use super::frontier::Walk;
use super::krpc::NodeId;

impl Dht {
    /// Walk toward `target` asking `q` of every node on the way — except the
    /// bootstrap, which is asked `find_node` whatever `q` is. The mainline's
    /// routers answer `find_node` and never BEP 44's `get` (REMOTE §13.7
    /// ruling 3, yog bl-f6e1), so a walk that asked them `q` was dark in one
    /// round; the bootstrap is a door into the keyspace, and every node it
    /// opens onto is asked `q`. One walk, not a lookup and then a second one.
    ///
    /// The bootstrap is a door and never a result (yog bl-9408): its answers seed
    /// the pool, but it is not a node near the target
    /// and nothing it says is in the [`Outcome`]. So a walk whose learned
    /// nodes were all silent — measured one walk in five from the deployed
    /// engine box, the one answering router naming a single node eight times —
    /// is the same `Err` as a silent bootstrap: a dark commons. Not an empty
    /// `Ok`, because an empty `Ok` already means *nodes near the target
    /// answered and held nothing*, and a caller seeding on it would be dark
    /// without being told. `Err` is that, or the socket itself failing; a
    /// walk whose learned nodes drew only errors is an `Ok` with none.
    ///
    /// A node is one `(id, address)`: a repeated entry is learned once, and
    /// one id at two addresses is two nodes to ask.
    ///
    /// The walk converges on a frontier, not on the pool (yog bl-d00f): the K
    /// closest nodes that replied, are not yet asked, or are still in the
    /// air. A node silent past its deadline, or that answered only an error
    /// (live, a `get` refused as an unknown query), leaves it, so the walk
    /// ends when the K closest *responsive* nodes have all been asked — or at
    /// `max_queries` — never because dead nodes sat on the slots a live one
    /// past them needed. A node this socket cannot send to (a v6 address from
    /// a v4 socket) leaves it the same way, before a query is spent: a
    /// refused send is not counted and holds no slot. The walk ends without
    /// waiting out a query that is in the air but no longer on the frontier.
    ///
    /// The door is asked again whenever the frontier runs dry before K nodes
    /// past it have replied — none, the dark case, being the one measured —
    /// as long as it has ever named anyone; `max_queries` bounds it. The
    /// door's queries hold no α slot. Measured from the deployed engine box
    /// (REMOTE §13.7 ruling 3), the one router that answers names a single
    /// random node eight times per query — often the same one several
    /// queries running — so its two seeds are both silent about one walk in
    /// three, and re-asking draws fresh ones. Asking only while the door
    /// named someone NEW was tried and measured: its repeats ended a walk
    /// dark that the next ask would have opened. A door that is silent or
    /// names nobody leaves the pool empty and is not re-asked, so a dark
    /// commons costs one deadline.
    pub(crate) fn search(&mut self, target: NodeId, q: &str) -> Result<Outcome, String> {
        if self.bootstrap.is_empty() {
            return Err("no bootstrap node to ask".into());
        }
        let args = Dict::from([entry("target", bytes(&target.0))]);
        let mut walk = Walk::new(target, self.config.k);
        let mut flight = Flight::new();
        let mut sent = self.door(&mut walk, &mut flight, &args);
        loop {
            let f = walk.frontier(&flight);
            let under = sent < self.config.max_queries;
            if let Some(addr) = f.next.filter(|_| under && f.walking < self.config.alpha) {
                walk.asked.insert(addr);
                sent += usize::from(self.ask(&mut flight, addr, false, q, args.clone()));
            } else if f.waiting || f.door || (f.next.is_some() && under && f.walking > 0) {
                if let Some((query, message)) = self.land(&mut flight)? {
                    walk.heard(query, message);
                }
            } else if f.next.is_none() && walk.dry() && under {
                sent += self.door(&mut walk, &mut flight, &args);
            } else {
                break;
            }
        }
        let mut out = walk.out;
        if out.replies.is_empty() && out.errors.is_empty() {
            return Err(format!("no DHT node answered {q} for {target}"));
        }
        out.replies.sort_by_key(|(n, _)| n.id.distance(&target));
        Ok(out)
    }

    /// Ask every bootstrap address `find_node`. Answers the queries it
    /// spent, and never less than one: a door whose every send was refused
    /// still spends a query of the cap, so a walk cannot knock at it forever.
    fn door(&mut self, walk: &mut Walk, flight: &mut Flight, args: &Dict) -> usize {
        let mut sent = 0usize;
        for addr in self.bootstrap.clone() {
            walk.asked.insert(addr);
            sent += usize::from(self.ask(flight, addr, true, "find_node", args.clone()));
        }
        sent.max(1)
    }
}
