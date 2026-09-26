//! Bencode (BEP 3), the encoding every KRPC datagram is spelled in — in-house,
//! because the whole of it is one enum, one encoder and one strict decoder
//! (REMOTE §13.2: zero new crates).
//!
//! The decoder is strict where the open internet makes strictness load-bearing:
//! the whole input must be one value (trailing bytes refuse), nesting is
//! bounded so a hostile `llll…` cannot recurse the stack away, and a length
//! that runs past the datagram refuses rather than reading what is not there.
//! The encoder is canonical — `BTreeMap` keys already sort, which is what BEP
//! 44's signature over `salt`/`seq`/`v` relies on.

use std::collections::BTreeMap;

/// A dictionary's storage: byte keys, already in the order the wire wants.
pub(crate) type Dict = BTreeMap<Vec<u8>, Value>;

/// The deepest a value may nest before the decoder refuses it.
const MAX_DEPTH: usize = 32;

/// One bencoded value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Value {
    Bytes(Vec<u8>),
    Int(i64),
    List(Vec<Value>),
    Dict(Dict),
}

impl Value {
    /// The canonical encoding.
    pub(crate) fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.write(&mut out);
        out
    }

    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Value::Bytes(b) => {
                out.extend_from_slice(b.len().to_string().as_bytes());
                out.push(b':');
                out.extend_from_slice(b);
            }
            Value::Int(n) => {
                out.push(b'i');
                out.extend_from_slice(n.to_string().as_bytes());
                out.push(b'e');
            }
            Value::List(items) => {
                out.push(b'l');
                for v in items {
                    v.write(out);
                }
                out.push(b'e');
            }
            Value::Dict(d) => {
                out.push(b'd');
                for (k, v) in d {
                    Value::Bytes(k.clone()).write(out);
                    v.write(out);
                }
                out.push(b'e');
            }
        }
    }

    /// Decode exactly one value spanning the whole input.
    pub(crate) fn decode(bytes: &[u8]) -> Result<Value, String> {
        let (value, end) = parse(bytes, 0, 0)?;
        if end != bytes.len() {
            return Err(format!(
                "{} trailing byte(s) after the value",
                bytes.len() - end
            ));
        }
        Ok(value)
    }

    /// A string entry of this dictionary, by key.
    pub(crate) fn get(&self, key: &str) -> Option<&Value> {
        self.as_dict()?.get(key.as_bytes())
    }

    pub(crate) fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Value::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub(crate) fn as_int(&self) -> Option<i64> {
        match self {
            Value::Int(n) => Some(*n),
            _ => None,
        }
    }

    pub(crate) fn as_dict(&self) -> Option<&Dict> {
        match self {
            Value::Dict(d) => Some(d),
            _ => None,
        }
    }
}

/// A dictionary entry's spelling — `(key, value)` from a `&str` key.
pub(crate) fn entry(key: &str, value: Value) -> (Vec<u8>, Value) {
    (key.as_bytes().to_vec(), value)
}

/// `Value::Bytes` from a slice.
pub(crate) fn bytes(b: &[u8]) -> Value {
    Value::Bytes(b.to_vec())
}

/// Parse one value at `at`; answers it and the index just past it.
fn parse(b: &[u8], at: usize, depth: usize) -> Result<(Value, usize), String> {
    if depth > MAX_DEPTH {
        return Err(format!("nested deeper than {MAX_DEPTH}"));
    }
    match b.get(at) {
        Some(b'i') => {
            let (n, end) = number::<i64>(b, at + 1, b'e')?;
            Ok((Value::Int(n), end))
        }
        Some(b'l') => {
            let mut items = Vec::new();
            let mut i = at + 1;
            while b.get(i) != Some(&b'e') {
                let (v, end) = parse(b, i, depth + 1)?;
                items.push(v);
                i = end;
            }
            Ok((Value::List(items), i + 1))
        }
        Some(b'd') => {
            let mut dict = Dict::new();
            let mut i = at + 1;
            while b.get(i) != Some(&b'e') {
                let (key, end) = parse(b, i, depth + 1)?;
                let Value::Bytes(key) = key else {
                    return Err(format!("dictionary key at {i} is not a string"));
                };
                let (v, end) = parse(b, end, depth + 1)?;
                dict.insert(key, v);
                i = end;
            }
            Ok((Value::Dict(dict), i + 1))
        }
        Some(c) if c.is_ascii_digit() => {
            let (len, start) = number::<usize>(b, at, b':')?;
            let end = start + len;
            let s = b
                .get(start..end)
                .ok_or_else(|| format!("string at {at} runs past the end"))?;
            Ok((Value::Bytes(s.to_vec()), end))
        }
        Some(c) => Err(format!("unexpected byte {c:#04x} at {at}")),
        None => Err(format!("unexpected end of input at {at}")),
    }
}

/// The decimal number running from `at` to the `stop` byte; answers it and
/// the index just past `stop`. Lengths parse as `usize`, values as `i64`, so
/// a sign where a length belongs is "not a length" and nothing else.
fn number<T: std::str::FromStr>(b: &[u8], at: usize, stop: u8) -> Result<(T, usize), String> {
    let len = b
        .get(at..)
        .and_then(|rest| rest.iter().position(|c| *c == stop))
        .ok_or_else(|| format!("no {:?} closes the number at {at}", char::from(stop)))?;
    let text = String::from_utf8_lossy(b.get(at..at + len).unwrap_or_default());
    let n = text
        .parse::<T>()
        .map_err(|_| format!("{text:?} at {at} is not a number"))?;
    Ok((n, at + len + 1))
}

#[cfg(test)]
mod tests;
