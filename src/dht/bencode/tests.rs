//! The encoding both ways, and every refusal the strict decoder makes.

use super::*;

fn dict(pairs: &[(&str, Value)]) -> Value {
    Value::Dict(pairs.iter().map(|(k, v)| entry(k, v.clone())).collect())
}

#[test]
fn every_shape_round_trips_canonically() {
    let v = dict(&[
        ("b", bytes(b"bytes")),
        ("i", Value::Int(-42)),
        ("l", Value::List(vec![Value::Int(0), bytes(b"")])),
        ("a", dict(&[("z", Value::Int(1))])),
    ]);
    let encoded = v.encode();
    assert_eq!(encoded, b"d1:ad1:zi1ee1:b5:bytes1:ii-42e1:lli0e0:ee");
    assert_eq!(Value::decode(&encoded).unwrap(), v);
}

#[test]
fn accessors_read_their_own_shape_and_nothing_else() {
    let v = dict(&[("n", Value::Int(7)), ("s", bytes(b"x"))]);
    assert_eq!(v.get("n").and_then(Value::as_int), Some(7));
    assert_eq!(v.get("s").and_then(Value::as_bytes), Some(b"x".as_slice()));
    assert_eq!(v.get("n").and_then(Value::as_bytes), None);
    assert_eq!(v.get("s").and_then(Value::as_int), None);
    assert_eq!(v.get("s").and_then(Value::as_dict), None);
    assert_eq!(v.get("missing"), None);
    assert_eq!(Value::Int(1).get("n"), None);
    assert_eq!(v.as_dict().map(BTreeMap::len), Some(2));
}

#[test]
fn a_later_duplicate_key_wins() {
    let v = Value::decode(b"d1:ai1e1:ai2ee").unwrap();
    assert_eq!(v.get("a").and_then(Value::as_int), Some(2));
}

#[test]
fn trailing_bytes_refuse() {
    let e = Value::decode(b"i1eXY").unwrap_err();
    assert_eq!(e, "2 trailing byte(s) after the value");
}

#[test]
fn nesting_is_bounded() {
    let deep: Vec<u8> = std::iter::repeat_n(b'l', 40).collect();
    assert_eq!(Value::decode(&deep).unwrap_err(), "nested deeper than 32");
    let fine: Vec<u8> = std::iter::repeat_n(b'l', 30)
        .chain(std::iter::repeat_n(b'e', 30))
        .collect();
    assert!(Value::decode(&fine).is_ok());
}

#[test]
fn a_dictionary_key_must_be_a_string() {
    let e = Value::decode(b"di1ei2ee").unwrap_err();
    assert_eq!(e, "dictionary key at 1 is not a string");
}

#[test]
fn a_string_may_not_run_past_the_end() {
    let e = Value::decode(b"5:abc").unwrap_err();
    assert_eq!(e, "string at 0 runs past the end");
}

#[test]
fn an_unexpected_byte_and_a_short_input_refuse() {
    assert_eq!(
        Value::decode(b"x").unwrap_err(),
        "unexpected byte 0x78 at 0"
    );
    assert_eq!(
        Value::decode(b"-1:a").unwrap_err(),
        "unexpected byte 0x2d at 0"
    );
    assert_eq!(
        Value::decode(b"").unwrap_err(),
        "unexpected end of input at 0"
    );
    assert_eq!(
        Value::decode(b"l").unwrap_err(),
        "unexpected end of input at 1"
    );
}

#[test]
fn a_number_needs_its_closer_and_its_digits() {
    assert_eq!(
        Value::decode(b"i12").unwrap_err(),
        "no 'e' closes the number at 1"
    );
    assert_eq!(
        Value::decode(b"iabce").unwrap_err(),
        "\"abc\" at 1 is not a number"
    );
    assert_eq!(
        Value::decode(b"ie").unwrap_err(),
        "\"\" at 1 is not a number"
    );
    assert_eq!(
        Value::decode(b"3abc").unwrap_err(),
        "no ':' closes the number at 0"
    );
    // A sign inside a length is not a length: the parser is `usize` there.
    assert_eq!(
        Value::decode(b"1-1:a").unwrap_err(),
        "\"1-1\" at 0 is not a number"
    );
    assert_eq!(Value::decode(b"i-1e").unwrap(), Value::Int(-1));
}
