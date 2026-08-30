//! The framing, both directions and every refusal.

use super::{MAX_FRAME, read_value, write_bytes, write_end, write_value};
use serde_json::json;

/// A request frame reads back as the value that was written.
#[test]
fn a_frame_round_trips() {
    let mut wire = Vec::new();
    write_value(&mut wire, &json!({"op": "advertise", "tools": []})).expect("write");
    let mut read = wire.as_slice();
    assert_eq!(
        read_value(&mut read).expect("read"),
        Some(json!({"op": "advertise", "tools": []}))
    );
}

/// N frames then the terminator is the whole shape of an answer: every frame
/// in order, and `None` where the stream ends.
#[test]
fn an_answer_is_n_frames_then_the_terminator() {
    let mut wire = Vec::new();
    for n in 0..3 {
        write_value(&mut wire, &json!({ "n": n })).expect("write");
    }
    write_end(&mut wire).expect("terminator");
    let mut read = wire.as_slice();
    for n in 0..3 {
        assert_eq!(
            read_value(&mut read).expect("read"),
            Some(json!({ "n": n }))
        );
    }
    assert_eq!(read_value(&mut read).expect("read"), None, "the terminator");
}

/// A length above the cap is refused **on its header**, before the allocation
/// it asks for is made. That is the property the bound exists for: a peer on
/// the open internet cannot make this reader grow to meet it.
#[test]
fn an_oversized_length_is_refused_on_its_header() {
    let header = u32::try_from(MAX_FRAME + 1).expect("fits").to_be_bytes();
    let mut read = header.as_slice();
    let e = read_value(&mut read).expect_err("refused");
    assert!(e.to_string().contains("exceeds"), "{e}");
}

/// And the same refusal in the other direction, so neither end can put a frame
/// on the wire the other would refuse to read.
#[test]
fn an_oversized_body_is_refused_before_it_is_written() {
    let body = vec![b'x'; MAX_FRAME + 1];
    let mut wire = Vec::new();
    let e = write_bytes(&mut wire, &body).expect_err("refused");
    assert!(e.to_string().contains("exceeds"), "{e}");
    assert!(wire.is_empty(), "nothing was written");
}

/// A body that is not JSON is an error naming what failed to parse — never a
/// value guessed out of the bytes.
#[test]
fn a_body_that_is_not_json_refuses() {
    let mut wire = Vec::new();
    write_bytes(&mut wire, b"not json at all").expect("write");
    let mut read = wire.as_slice();
    let e = read_value(&mut read).expect_err("refused");
    assert!(e.to_string().contains("not JSON"), "{e}");
}

/// A stream that ends mid-frame is short, not empty: the header promised bytes
/// that never came, and a truncated read must not read as a terminator.
#[test]
fn a_truncated_frame_is_an_error_and_not_a_terminator() {
    let mut wire = Vec::new();
    write_value(&mut wire, &json!({"op": "invocations"})).expect("write");
    wire.truncate(6);
    let mut read = wire.as_slice();
    assert!(read_value(&mut read).is_err(), "short read");
}

/// A header that never arrives is an error too — the same fact one byte
/// earlier.
#[test]
fn a_stream_that_ends_before_a_header_is_an_error() {
    let mut read: &[u8] = &[0u8, 0u8];
    assert!(read_value(&mut read).is_err(), "short header");
}
