//! The query shape, the two datagrams read back, and compact node parsing.

use super::*;

fn id(fill: u8) -> NodeId {
    NodeId([fill; 20])
}

#[test]
fn a_query_carries_its_id_beside_the_arguments() {
    let bytes_out = query(b"aa", &id(1), "ping", Dict::new());
    assert_eq!(
        bytes_out,
        [
            b"d1:ad2:id20:".as_slice(),
            &[1u8; 20],
            b"e1:q4:ping1:t2:aa1:y1:qe"
        ]
        .concat()
    );
    let with = query(
        b"b",
        &id(2),
        "find_node",
        Dict::from([entry("target", bytes(&[3; 20]))]),
    );
    let v = Value::decode(&with).unwrap();
    let a = v.get("a").unwrap();
    assert_eq!(
        a.get("id").and_then(Value::as_bytes),
        Some([2u8; 20].as_slice())
    );
    assert_eq!(
        a.get("target").and_then(Value::as_bytes),
        Some([3u8; 20].as_slice())
    );
}

#[test]
fn a_reply_and_an_error_parse_and_everything_else_is_noise() {
    let reply = parse(b"d1:rd2:id20:aaaaaaaaaaaaaaaaaaaae1:t2:xy1:y1:re").unwrap();
    assert_eq!(
        reply,
        Message::Reply {
            tid: b"xy".to_vec(),
            r: Dict::from([entry("id", bytes(b"aaaaaaaaaaaaaaaaaaaa"))]),
        }
    );
    let error = parse(b"d1:eli203e12:bad argumente1:t1:z1:y1:ee").unwrap();
    assert_eq!(
        error,
        Message::Error {
            tid: b"z".to_vec(),
            code: 203,
            message: "bad argument".into()
        }
    );
    // An error list with nothing in it still names a transaction.
    assert_eq!(
        parse(b"d1:ele1:t1:z1:y1:ee").unwrap(),
        Message::Error {
            tid: b"z".to_vec(),
            code: -1,
            message: String::new()
        }
    );
    for noise in [
        b"not bencode".as_slice(),
        b"d1:y1:re",             // no transaction id
        b"d1:t1:ze",             // no kind
        b"d1:t1:z1:y1:qe",       // a query: this end is no node
        b"d1:t1:z1:y1:re",       // a reply with no `r`
        b"d1:r1:x1:t1:z1:y1:re", // `r` that is not a dictionary
        b"d1:e1:x1:t1:z1:y1:ee", // `e` that is not a list
        b"d1:t1:z1:y1:ee",       // an error with no `e`
    ] {
        assert_eq!(parse(noise), None, "{}", String::from_utf8_lossy(noise));
    }
}

#[test]
fn compact_nodes_parse_in_both_families_and_drop_a_ragged_tail() {
    let mut v4 = vec![7u8; 20];
    v4.extend_from_slice(&[127, 0, 0, 1, 0x1f, 0x90]);
    let mut v6 = vec![8u8; 20];
    v6.extend_from_slice(&[0; 15]);
    v6.extend_from_slice(&[1, 0x00, 0x50]);
    let mut ragged = v4.clone();
    ragged.extend_from_slice(b"tail");
    let r = Dict::from([entry("nodes", bytes(&ragged)), entry("nodes6", bytes(&v6))]);
    assert_eq!(
        nodes_of(&r),
        vec![
            Node {
                id: id(7),
                addr: "127.0.0.1:8080".parse().unwrap()
            },
            Node {
                id: id(8),
                addr: "[::1]:80".parse().unwrap()
            },
        ]
    );
    assert_eq!(nodes_of(&Dict::new()), vec![]);
    assert_eq!(
        nodes_of(&Dict::from([entry("nodes", Value::Int(1))])),
        vec![]
    );
}

#[test]
fn distance_is_xor_and_an_id_reads_as_hex() {
    assert_eq!(id(0xff).distance(&id(0x0f)), [0xf0; 20]);
    assert_eq!(NodeId::parse(&[1; 19]), None);
    assert_eq!(id(0xab).to_string(), "ab".repeat(20));
}
