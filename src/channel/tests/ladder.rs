//! **The dial ladder, rung by rung** (REMOTE §15.3; DESIGN §3.11): a list
//! whose first endpoint is dead, the endpoint that answered kept for the next
//! ask, the kept one failing and the rest tried, and no rung answering at all.

use super::super::material::{ADDRESS, read_dir};
use super::super::{Channel, Failure};
use super::{advertised, said};
use crate::test_support::server::Server;
use crate::test_support::{Scratch, mint};
use serde_json::json;

/// Port 1 is below the ephemeral range and unbindable without privilege, so
/// no other test's freed port can answer a dial there.
const DEAD: &str = "127.0.0.1:1";

/// A channel over a live server answering `dials` asks, whose entry lists
/// the endpoints `order` builds from the address the server bound.
fn listed(dials: usize, order: fn(&str) -> String) -> (Scratch, Server, Channel) {
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
