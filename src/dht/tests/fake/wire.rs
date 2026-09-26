//! The fake node's datagrams: a reply, an error, and compact routing.

use super::super::super::bencode::{Dict, Value, bytes, entry};
use super::super::super::krpc::Node;
use std::net::IpAddr;

/// `nodes` and `nodes6` in compact form, from the peers this node advertises.
pub(super) fn routing(peers: &[Node]) -> Dict {
    let (mut v4, mut v6) = (Vec::new(), Vec::new());
    for n in peers {
        let out = if n.addr.is_ipv4() { &mut v4 } else { &mut v6 };
        out.extend_from_slice(&n.id.0);
        match n.addr.ip() {
            IpAddr::V4(ip) => out.extend_from_slice(&ip.octets()),
            IpAddr::V6(ip) => out.extend_from_slice(&ip.octets()),
        }
        out.extend_from_slice(&n.addr.port().to_be_bytes());
    }
    Dict::from([entry("nodes", bytes(&v4)), entry("nodes6", bytes(&v6))])
}

pub(super) fn reply(tid: &[u8], r: Dict) -> Vec<u8> {
    Value::Dict(Dict::from([
        entry("t", bytes(tid)),
        entry("y", bytes(b"r")),
        entry("r", Value::Dict(r)),
    ]))
    .encode()
}

pub(super) fn error(tid: &[u8], code: i64, message: &str) -> Vec<u8> {
    Value::Dict(Dict::from([
        entry("t", bytes(tid)),
        entry("y", bytes(b"e")),
        entry(
            "e",
            Value::List(vec![Value::Int(code), bytes(message.as_bytes())]),
        ),
    ]))
    .encode()
}
