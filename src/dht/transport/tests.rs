//! The std UDP transport on loopback: a datagram each way, a wait that
//! elapses, and a destination this socket cannot reach.

use super::*;

fn loopback() -> Udp {
    Udp::bind("127.0.0.1:0".parse().unwrap()).unwrap()
}

#[test]
fn a_datagram_crosses_loopback_and_names_its_sender() {
    let a = loopback();
    let b = loopback();
    a.send(b.local_addr().unwrap(), b"hello").unwrap();
    let (from, bytes) = b.recv(Duration::from_secs(2)).unwrap().unwrap();
    assert_eq!(from, a.local_addr().unwrap());
    assert_eq!(bytes, b"hello");
}

#[test]
fn a_wait_that_elapses_is_none_even_at_zero() {
    let a = loopback();
    assert_eq!(a.recv(Duration::from_millis(5)).unwrap(), None);
    assert_eq!(a.recv(Duration::ZERO).unwrap(), None);
}

#[test]
fn a_family_this_socket_cannot_reach_refuses_the_send() {
    let a = loopback();
    assert!(a.send("[::1]:9".parse().unwrap(), b"x").is_err());
}

#[test]
fn a_datagram_over_the_buffer_is_truncated() {
    let a = loopback();
    let b = loopback();
    a.send(b.local_addr().unwrap(), &vec![1u8; DATAGRAM + 100])
        .unwrap();
    let (_, bytes) = b.recv(Duration::from_secs(2)).unwrap().unwrap();
    assert_eq!(bytes.len(), DATAGRAM);
}
