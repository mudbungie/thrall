//! **The dial ladder, rung by rung** (REMOTE §15.3; DESIGN §3.11): a list
//! whose first endpoint is dead, the endpoint that answered kept for the next
//! ask, the kept one failing and the rest tried, what answers without being
//! the server passed over (bl-45e9), and no rung answering at all.

use std::net::TcpListener;
use std::sync::mpsc;

use super::super::material::{ADDRESS, read_dir};
use super::super::{Channel, Failure};
use super::{advertised, said};
use crate::test_support::server::Server;
use crate::test_support::{Scratch, mint};
use serde_json::{Value, json};

/// Port 1 is below the ephemeral range and unbindable without privilege, so
/// no other test's freed port can answer a dial there.
const DEAD: &str = "127.0.0.1:1";

/// A channel over a live server answering `dials` asks, whose entry lists
/// the endpoints `order` builds from the address the server bound.
fn listed(dials: usize, order: impl Fn(&str) -> String) -> (Scratch, Server, Channel) {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let script = vec![vec![advertised()]; dials];
    let server = Server::start(scratch.path(), crate::corpus::PROTOCOL, script);
    let file = scratch.join(ADDRESS);
    let bound = std::fs::read_to_string(&file).expect("the bound address");
    std::fs::write(&file, order(&bound)).expect("the list");
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    (scratch, server, channel)
}

/// **A dead first endpoint falls through to the second, and the second is
/// kept**: the next ask dials it first, and the list on disk is untouched.
#[test]
fn the_endpoint_that_answered_is_kept_for_the_run_and_never_written() {
    let (scratch, server, mut channel) = listed(2, |bound| format!("{DEAD}\n{bound}\n"));
    let before = std::fs::read_to_string(scratch.join(ADDRESS)).expect("the list");
    assert_eq!(channel.kept, 0, "nothing has answered yet");
    for _ in 0..2 {
        assert_eq!(
            channel.ask(&json!({"op": "advertise"})),
            Ok(vec![advertised()])
        );
        assert_eq!(channel.kept, 1, "the second endpoint answered");
    }
    assert_eq!(server.heard().len(), 4, "two connections, two prefaces");
    let after = std::fs::read_to_string(scratch.join(ADDRESS)).expect("the list");
    assert_eq!(before, after, "what worked is RAM, never disk");
}

/// **A kept endpoint that stops answering costs nothing but its own bound**:
/// the rest of the list is tried, in the entry's order, and the one that
/// answers is kept in its place.
#[test]
fn a_kept_endpoint_that_stops_answering_falls_back_to_the_rest() {
    let (_scratch, _server, mut channel) = listed(1, |bound| format!("{bound}\n{DEAD}"));
    channel.kept = 1;
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
    assert_eq!(channel.kept, 0);
}

/// A loopback listener that takes each connection, says so, and hangs up
/// without a byte: something that answers and is not the server.
fn stranger() -> (String, mpsc::Receiver<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let at = listener.local_addr().expect("bound").to_string();
    let (took, taken) = mpsc::channel();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if took.send(()).is_err() {
                return;
            }
            drop(stream);
        }
    });
    (at, taken)
}

/// **What answers and is not the server is passed over, and the endpoint
/// that proved itself is kept**: the stranger at the head of the list is
/// dialled once and never again.
#[test]
fn an_endpoint_that_answers_and_is_not_the_server_is_passed_over() {
    let (at, taken) = stranger();
    let (_scratch, server, mut channel) = listed(2, |bound| format!("{at}\n{bound}\n"));
    let ask = json!({"op": "advertise"});
    assert_eq!(channel.ask(&ask), Ok(vec![advertised()]));
    assert_eq!(taken.try_recv(), Ok(()), "the head of the list was tried");
    assert_eq!(channel.kept, 1, "and the server's own endpoint is kept");
    assert_eq!(channel.ask(&ask), Ok(vec![advertised()]));
    assert!(
        taken.try_recv().is_err(),
        "the stranger is not dialled again"
    );
    assert_eq!(server.heard().len(), 4, "each ask said once, to the server");
}

/// **A server under another CA is not the server**: its handshake does not
/// verify, nothing is said to it, and the list goes on.
#[test]
fn an_endpoint_that_cannot_present_the_operators_ca_is_passed_over() {
    let elsewhere = Scratch::new();
    mint::material(elsewhere.path());
    let other = Server::start(elsewhere.path(), crate::corpus::PROTOCOL, vec![vec![]]);
    let theirs = std::fs::read_to_string(elsewhere.join(ADDRESS)).expect("its address");
    let (_scratch, _server, mut channel) = listed(1, |bound| format!("{theirs}\n{bound}\n"));
    assert_eq!(
        channel.ask(&json!({"op": "advertise"})),
        Ok(vec![advertised()])
    );
    assert_eq!(channel.kept, 1);
    assert_eq!(other.heard(), Vec::<Value>::new(), "nothing was said to it");
}

/// **A handshake that failed is that rung's sentence**, named as a handshake
/// and joined with the rest like any other.
#[test]
fn a_rung_whose_handshake_failed_says_so_and_names_the_endpoint() {
    let (at, _taken) = stranger();
    let scratch = Scratch::new();
    let held = mint::provisioned(scratch.path(), &format!("{at}\n{DEAD}\n"));
    let mut channel = Channel::open(&held).expect("opened");
    let failure = channel
        .ask(&json!({"op": "advertise"}))
        .expect_err("refused");
    assert!(matches!(failure, Failure::Wire(_)), "a transport failure");
    let said = said(failure);
    let (first, second) = said.split_once("; ").expect("two sentences");
    assert!(first.starts_with(&format!("handshake {at}: ")), "{said}");
    assert!(second.starts_with("connect 127.0.0.1:1: "), "{said}");
}

/// **No rung answering is every rung's sentence**, in the order they were
/// tried — and an endpoint that is not a `host:port` at all says so itself.
#[test]
fn no_rung_answering_names_every_endpoint_it_tried() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    std::fs::write(scratch.join(ADDRESS), format!("{DEAD}\nno-port-here\n")).expect("the list");
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let mut channel = Channel::open(&held).expect("opened");
    let failure = channel
        .ask(&json!({"op": "advertise"}))
        .expect_err("refused");
    assert!(matches!(failure, Failure::Wire(_)), "a transport failure");
    let said = said(failure);
    let (first, second) = said.split_once("; ").expect("two sentences");
    assert!(first.starts_with("connect 127.0.0.1:1: "), "{said}");
    assert!(second.starts_with("connect no-port-here: "), "{said}");
}

/// An endpoint that names no server refuses when the channel opens — it is
/// this box's configuration, and no dial would improve it.
#[test]
fn an_endpoint_that_names_no_server_refuses_to_open() {
    let scratch = Scratch::new();
    let held = mint::provisioned(scratch.path(), "server.example:9000\nnot a name:9000\n");
    let refusal = Channel::open(&held).expect_err("refused");
    assert!(refusal.contains("not a server name"), "{refusal}");
}
