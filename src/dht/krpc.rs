//! KRPC (BEP 5), the client's half only: the query shapes it sends and the two
//! kinds of datagram it reads back — a reply and an error. A query arriving
//! here is ignored, not answered: this end is never a node (REMOTE §13.2), so
//! it holds no routing table for anyone and offers nothing to store. A reply
//! may also say where its node saw the query come from (BEP 42's `ip`), which
//! is one node's claim and is read as nothing more (`Dht::observed` votes).

use super::bencode::{Dict, Value, bytes, entry};
use std::fmt;
use std::net::{IpAddr, SocketAddr};

/// A 160-bit DHT node id — the keyspace both nodes and targets live in.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct NodeId(pub(crate) [u8; 20]);

impl NodeId {
    /// The XOR metric: how far `other` sits from this id, comparable as bytes.
    pub(crate) fn distance(&self, other: &NodeId) -> [u8; 20] {
        let mut out = [0u8; 20];
        for (o, (a, b)) in out.iter_mut().zip(self.0.iter().zip(other.0.iter())) {
            *o = a ^ b;
        }
        out
    }

    /// An id read off the wire: exactly 20 bytes, or nothing.
    pub(crate) fn parse(b: &[u8]) -> Option<NodeId> {
        <[u8; 20]>::try_from(b).ok().map(NodeId)
    }
}

impl fmt::Display for NodeId {
    /// Forty hex digits — how a target reads in an error and a log.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|b| write!(f, "{b:02x}"))
    }
}

/// A node the client has heard of: where it is and what it calls itself.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) addr: SocketAddr,
}

/// What one datagram from the network says.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Message {
    Reply {
        tid: Vec<u8>,
        r: Dict,
        /// BEP 42: the address this node says the query came from.
        ip: Option<SocketAddr>,
    },
    Error {
        tid: Vec<u8>,
        code: i64,
        message: String,
    },
}

/// One query datagram: `tid` names the transaction, `q` the method, and
/// `args` its arguments beside the querying node's own `id`.
pub(crate) fn query(tid: &[u8], id: &NodeId, q: &str, mut args: Dict) -> Vec<u8> {
    args.insert(b"id".to_vec(), bytes(&id.0));
    Value::Dict(Dict::from([
        entry("t", bytes(tid)),
        entry("y", bytes(b"q")),
        entry("q", bytes(q.as_bytes())),
        entry("a", Value::Dict(args)),
    ]))
    .encode()
}

/// Read one datagram. Bytes that are not a well-formed reply or error — a
/// query, garbage, a reply with no transaction id — are `None`: noise.
pub(crate) fn parse(datagram: &[u8]) -> Option<Message> {
    let v = Value::decode(datagram).ok()?;
    let tid = v.get("t")?.as_bytes()?.to_vec();
    match v.get("y")?.as_bytes()? {
        b"r" => Some(Message::Reply {
            tid,
            r: v.get("r")?.as_dict()?.clone(),
            ip: v.get("ip").and_then(Value::as_bytes).and_then(compact_addr),
        }),
        b"e" => {
            let Value::List(e) = v.get("e")? else {
                return None;
            };
            let code = e.first().and_then(Value::as_int).unwrap_or(-1);
            let message = e
                .get(1)
                .and_then(Value::as_bytes)
                .map(|m| String::from_utf8_lossy(m).into_owned())
                .unwrap_or_default();
            Some(Message::Error { tid, code, message })
        }
        _ => None,
    }
}

/// The nodes a reply carries in compact form: `nodes` (26 bytes each, v4) and
/// `nodes6` (38 bytes each, v6). A ragged tail is dropped, never misread.
pub(crate) fn nodes_of(r: &Dict) -> Vec<Node> {
    let mut out = Vec::new();
    if let Some(Value::Bytes(b)) = r.get(b"nodes".as_slice()) {
        out.extend(b.chunks_exact(26).filter_map(compact_node));
    }
    if let Some(Value::Bytes(b)) = r.get(b"nodes6".as_slice()) {
        out.extend(b.chunks_exact(38).filter_map(compact_node));
    }
    out
}

/// A 20-byte id, then a compact address.
fn compact_node(c: &[u8]) -> Option<Node> {
    Some(Node {
        id: NodeId::parse(c.get(..20)?)?,
        addr: compact_addr(c.get(20..)?)?,
    })
}

/// A compact address: the IP's own bytes (4 or 16) and a big-endian port —
/// six bytes or eighteen, and any other length is nothing.
pub(crate) fn compact_addr(b: &[u8]) -> Option<SocketAddr> {
    let (ip, port) = b.split_at_checked(b.len().checked_sub(2)?)?;
    let ip = <[u8; 4]>::try_from(ip)
        .map(IpAddr::from)
        .or_else(|_| <[u8; 16]>::try_from(ip).map(IpAddr::from))
        .ok()?;
    Some(SocketAddr::new(
        ip,
        u16::from_be_bytes(port.try_into().ok()?),
    ))
}

#[cfg(test)]
mod tests;
