//! The lines themselves: each says counts, seqs, nonces and families, no line
//! built from addresses ever carries one, and a repeated outcome is said once.

use super::*;
use crate::test_support::Notices;
use std::net::{Ipv4Addr, Ipv6Addr};

fn endpoints() -> Vec<SocketAddr> {
    vec![
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 7)), 7737),
        SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 7738),
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7739),
    ]
}

#[test]
fn every_line_is_the_house_shape_and_names_no_address() {
    let ips: Vec<IpAddr> = endpoints().iter().map(SocketAddr::ip).collect();
    let window = Duration::from_secs(35);
    let lines = [
        direct_connected(),
        direct_refused(&ips, false),
        direct_refused(&ips, true),
        presence_read(9, &endpoints()),
        presence_unread(WALK),
        presence_unread(NO_WALK),
        presence_unread(NONE),
        presence_unread(SEALED),
        call_written(7, 10, &endpoints(), 2),
        call_unwritten(7),
        punch_started(Some(7), &endpoints(), window),
        punch_started(None, &endpoints(), window),
        landed(endpoints().first().copied()),
        landed(None),
        expired(window),
        held_kept(),
        held_dropped("receive", &Error::from(ErrorKind::TimedOut)),
        ping(),
    ];
    for line in &lines {
        assert!(line.starts_with("rendezvous: "), "{line}");
        for addr in ["203.0.113", "127.0.0.1", "::1", "7737", "7738"] {
            assert!(!line.contains(addr), "{line} names {addr}");
        }
    }
    assert_eq!(
        call_written(7, 10, &endpoints(), 2),
        "rendezvous: call nonce 7 written — seq 10, 3 endpoint(s) (1 v6, 2 v4), 2 ack(s)"
    );
    assert_eq!(
        punch_started(None, &endpoints(), window),
        "rendezvous: re-punch at 3 cached endpoint(s) (1 v6, 2 v4) — window 35s"
    );
    assert_eq!(
        direct_refused(&ips, true),
        "rendezvous: direct rung timed out — 3 address(es) tried (1 v6, 2 v4)"
    );
    assert_eq!(landed(None), "rendezvous: punch landed (none)");
}

#[test]
fn a_dropped_held_line_is_said_by_the_class_of_what_failed() {
    let class = |kind| held_dropped("send", &Error::from(kind));
    for (kind, said) in [
        (ErrorKind::TimedOut, "silent past the hangup"),
        (ErrorKind::WouldBlock, "silent past the hangup"),
        (ErrorKind::UnexpectedEof, "closed by the far end"),
        (ErrorKind::ConnectionReset, "closed by the far end"),
        (ErrorKind::ConnectionAborted, "closed by the far end"),
        (ErrorKind::BrokenPipe, "closed by the far end"),
        (ErrorKind::InvalidData, "a transport error"),
    ] {
        assert_eq!(
            class(kind),
            format!("rendezvous: held line dropped — the send failed ({said})")
        );
    }
}

#[test]
fn families_count_v6_first_and_say_none_for_nothing() {
    assert_eq!(families(&[]), "none");
    assert_eq!(families(&[IpAddr::V6(Ipv6Addr::LOCALHOST)]), "1 v6");
    assert_eq!(families(&[IpAddr::V4(Ipv4Addr::LOCALHOST)]), "1 v4");
}

/// Each family stays quiet while its outcome repeats, and speaks again when
/// it changes — and a held line kept or dropped lets the next ping be said.
#[test]
fn a_repeated_outcome_is_said_once() {
    let (notices, sink) = Notices::new();
    let mut say = Say::to(&sink);
    say.say(Kind::Direct, direct_connected());
    say.say(Kind::Direct, direct_connected());
    say.say(Kind::Ping, ping());
    say.say(Kind::Ping, ping());
    say.say(Kind::Direct, direct_refused(&[], false));
    say.say(Kind::Direct, direct_connected());
    say.say(Kind::Held, held_kept());
    say.say(Kind::Ping, ping());
    assert_eq!(
        notices.heard(),
        [
            direct_connected(),
            ping(),
            direct_refused(&[], false),
            direct_connected(),
            held_kept(),
            ping(),
        ]
    );
    assert!(format!("{say:?}").contains("Direct"));
}

#[test]
fn a_silent_speaker_says_nothing_anyone_hears() {
    let mut say = Say::silent();
    say.say(Kind::Ping, ping());
    assert!(format!("{say:?}").contains("Ping"));
}
