//! The client end to end against a fake DHT on loopback UDP, through the
//! real [`Udp`] transport — and against two stand-in transports for the
//! socket failures loopback will not produce on demand.

mod bootstrap;
pub(crate) mod fake;
mod frontier;
mod items;
mod walks;
mod window;

use super::mutable::Mutable;
use super::*;
use fake::{FakeNode, Mood};
use std::io;

/// A walk that converges on three nodes and waits milliseconds, not seconds.
pub(crate) fn quick() -> Config {
    Config {
        alpha: 3,
        k: 3,
        deadline: Duration::from_millis(300),
        max_queries: 64,
    }
}

fn client(bootstrap: Vec<SocketAddr>, config: Config) -> Dht {
    let udp = Udp::bind("127.0.0.1:0".parse().unwrap()).unwrap();
    Dht::new(Box::new(udp), bootstrap, config).unwrap()
}

pub(crate) fn id(fill: u8) -> NodeId {
    NodeId([fill; 20])
}

/// A mainline bootstrap router at `0x00` (yog REMOTE §13.7 ruling 3): it
/// answers `find_node` with `peers` and nothing else.
pub(crate) fn router(peers: Vec<Node>) -> FakeNode {
    let mut r = FakeNode::bind(id(0x00));
    r.serve(peers, Mood::Router, vec![]);
    r
}

/// A bootstrap node at `0x00` advertising `B` (`0x0f`) and `C` (`0x10`), `B`
/// advertising `D` (`0xf0`) — so a walk toward `0xff` must hop through `B`
/// to find the closest node, and with `k = 3` converges on `[D, C, B]`.
/// Every node answers; the caller reshapes moods and stores by hand.
fn topology() -> [FakeNode; 4] {
    let a = FakeNode::bind(id(0x00));
    let b = FakeNode::bind(id(0x0f));
    let c = FakeNode::bind(id(0x10));
    let d = FakeNode::bind(id(0xf0));
    [a, b, c, d]
}

fn serve(nodes: &mut [FakeNode; 4], items_on_c: Vec<Mutable>, items_on_d: Vec<Mutable>) {
    let (b, c, d) = (nodes[1].node(), nodes[2].node(), nodes[3].node());
    // A also advertises a v6 node the client's v4 socket cannot reach, and
    // it is the closest id of all: the send fails, the node is skipped, and
    // the walk is none the worse.
    let unreachable = Node {
        id: id(0xfe),
        addr: "[::1]:9".parse().unwrap(),
    };
    nodes[0].serve(vec![b, c, unreachable], Mood::Answer, vec![]);
    nodes[1].serve(vec![d], Mood::Answer, vec![]);
    nodes[2].serve(vec![], Mood::Answer, items_on_c);
    nodes[3].serve(vec![], Mood::Answer, items_on_d);
}

#[test]
fn the_defaults_are_the_measured_window_and_deadline() {
    let c = Config::default();
    assert_eq!(
        (c.alpha, c.k, c.deadline, c.max_queries),
        (8, 8, Duration::from_secs(1), 64)
    );
}

#[test]
fn a_client_mints_an_id_and_counts_transactions_around_the_wrap() {
    let mut dht = client(vec![], quick());
    assert_ne!(dht.id, NodeId([0; 20]));
    assert_eq!(dht.next_tid(), vec![0, 1]);
    dht.tid = u16::MAX;
    assert_eq!(dht.next_tid(), vec![0, 0]);
}

/// A stand-in for the socket failures loopback will not produce on demand:
/// every send is accepted or refused as `sends` says, and any read is the
/// socket dying — which a walk only reaches once a send has gone.
struct Stub {
    sends: bool,
}

impl Transport for Stub {
    fn send(&self, _: SocketAddr, _: &[u8]) -> io::Result<()> {
        self.sends
            .then_some(())
            .ok_or_else(|| io::Error::other("no route"))
    }

    fn recv(&self, _: Duration) -> io::Result<Option<(SocketAddr, Vec<u8>)>> {
        Err(io::Error::other("socket died"))
    }
}

#[test]
fn a_socket_that_dies_is_the_walks_error() {
    let mut dht = Dht::new(
        Box::new(Stub { sends: true }),
        vec!["127.0.0.1:9".parse().unwrap()],
        quick(),
    )
    .unwrap();
    assert_eq!(dht.lookup(id(1)).unwrap_err(), "DHT socket: socket died");
}

#[test]
fn a_transport_that_sends_nothing_hears_nothing() {
    let mut dht = Dht::new(
        Box::new(Stub { sends: false }),
        vec!["127.0.0.1:9".parse().unwrap()],
        quick(),
    )
    .unwrap();
    let e = dht.lookup(id(0xff)).unwrap_err();
    assert_eq!(
        e,
        format!("no DHT node answered find_node for {}", "ff".repeat(20))
    );
}
