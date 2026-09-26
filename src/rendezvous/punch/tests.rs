//! The punch on loopback: a SYN that lands on the peer's listener, two ends
//! punching each other at once, and nobody there.

use super::*;
use std::io::{Read, Write};

fn loopback(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

#[test]
fn a_punch_lands_on_the_peers_listener_and_carries_bytes() {
    let engine = Punch::bind(0).expect("bind");
    let client = Punch::bind(0).expect("bind");
    let target = loopback(engine.port());
    let served = std::thread::spawn(move || {
        let mut stream = engine
            .punch(vec![], Duration::from_secs(5))
            .expect("accepted");
        let mut byte = [0u8; 1];
        stream.read_exact(&mut byte).expect("read");
        byte[0]
    });
    let mut stream = client
        .punch(vec![target], Duration::from_secs(5))
        .expect("connected");
    assert_eq!(
        stream.local_addr().expect("addr").port(),
        client.port(),
        "the SYN left from the fixed port"
    );
    stream.write_all(b"x").expect("write");
    assert_eq!(served.join().expect("served"), b'x');
}

#[test]
fn two_ends_punching_each_other_both_hold_a_connection() {
    let a = Punch::bind(0).expect("bind");
    let b = Punch::bind(0).expect("bind");
    let (to_a, to_b) = (loopback(a.port()), loopback(b.port()));
    let from_a = std::thread::spawn(move || a.punch(vec![to_b], Duration::from_secs(5)));
    let from_b = b.punch(vec![to_a], Duration::from_secs(5));
    assert!(from_a.join().expect("a").is_some());
    assert!(from_b.is_some());
}

#[test]
fn nobody_there_is_no_stream_at_the_end_of_the_window() {
    let gone = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let target = gone.local_addr().expect("addr");
    drop(gone);
    let punch = Punch::bind(0).expect("bind");
    let started = Instant::now();
    assert!(
        punch
            .punch(vec![target], Duration::from_millis(400))
            .is_none()
    );
    assert!(
        started.elapsed() >= Duration::from_millis(400),
        "the window was waited out"
    );
}

#[test]
fn a_port_nothing_can_bind_refuses() {
    let refusal = Punch::bind(1).expect_err("privileged");
    assert!(refusal.contains("punch"), "{refusal}");
}

#[test]
fn v6_is_punched_first() {
    let v4: SocketAddr = "192.0.2.4:1".parse().expect("v4");
    let v6: SocketAddr = "[2001:db8::1]:1".parse().expect("v6");
    assert_eq!(ordered(vec![v4, v6, v4]), vec![v6, v4, v4]);
}

#[test]
fn a_v6_loopback_punch_lands_where_the_box_has_v6() {
    let engine = Punch::bind(0).expect("bind");
    let target: SocketAddr = format!("[::1]:{}", engine.port()).parse().expect("v6");
    let has_v6 = engine.listeners.len() == 2;
    let client = Punch::bind(0).expect("bind");
    let served =
        std::thread::spawn(move || engine.punch(vec![], Duration::from_millis(600)).is_some());
    let landed = client
        .punch(vec![target], Duration::from_millis(500))
        .is_some();
    assert_eq!(
        landed, has_v6,
        "a v6 SYN lands exactly where a v6 listener is"
    );
    assert_eq!(served.join().expect("served"), has_v6);
}

#[test]
fn the_box_advertises_no_loopback() {
    let ips = local_ips();
    assert!(ips.len() <= 2);
    assert!(ips.iter().all(|ip| !ip.is_loopback()));
}
