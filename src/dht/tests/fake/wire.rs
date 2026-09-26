//! The fake node's datagrams: a reply, an error, compact routing, and the
//! `ip` a claiming node splices into a reply it has already built.

use super::super::super::bencode::{Dict, Value, bytes, entry};
use super::super::super::krpc::Node;
use std::net::{IpAddr, SocketAddr};

/// `nodes` and `nodes6` in compact form, from the peers this node advertises.
pub(super) fn routing(peers: &[Node]) -> Dict {
    let (mut v4, mut v6) = (Vec::new(), Vec::new());
    for n in peers {
        let out = if n.addr.is_ipv4() { &mut v4 } else { &mut v6 };
        out.extend_from_slice(&n.id.0);
        out.extend(compact(n.addr));
    }
    Dict::from([entry("nodes", bytes(&v4)), entry("nodes6", bytes(&v6))])
}

/// An address in compact form: its own bytes, then a big-endian port.
fn compact(addr: SocketAddr) -> Vec<u8> {
    let mut out = match addr.ip() {
        IpAddr::V4(ip) => ip.octets().to_vec(),
        IpAddr::V6(ip) => ip.octets().to_vec(),
    };
    out.extend_from_slice(&addr.port().to_be_bytes());
    out
}

/// `datagram` with a top-level `ip` naming `addr`.
pub(super) fn claim(datagram: Vec<u8>, addr: SocketAddr) -> Vec<u8> {
    let mut d = Value::decode(&datagram).unwrap().as_dict().unwrap().clone();
    d.insert(b"ip".to_vec(), bytes(&compact(addr)));
    Value::Dict(d).encode()
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
