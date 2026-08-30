//! **The wire's framing** — yog's `docs/REMOTE.md` §3 is the authority and
//! this is thrall's end of it: *"a big-endian `u32` byte length, then that many
//! bytes of JSON. A request is one frame; an answer is N ≥ 1 reply frames
//! followed by a zero-length frame, which is the terminator."*
//!
//! The protocol document states why it is length-delimited rather than
//! newline-delimited and this file does not restate it. What matters on this
//! side is the consequence: a reader never scans, the allocation is bounded
//! before it is made, and a zero-length frame is not a JSON value, so nothing a
//! payload can say collides with the terminator.
//!
//! **The streaming form is not a second form.** Every answer is a stream, so
//! the follow-class read a foot spends its life on (`invocations`, bl-a2ea) is
//! the general path with more than one frame in it. There is no flag, no
//! version and no second reader here, and there is nothing to add when that
//! read lands.

use std::io::{self, Read, Write};

use serde_json::Value;

/// The largest frame either end will write or read: 16 MiB. It is a fact about
/// the wire and not about this end, so it is the number REMOTE §3's
/// implementation fixed rather than one a foot chooses — a reader that accepted
/// more would accept a frame the peer will never send, and one that accepted
/// less would refuse a frame the peer may.
pub const MAX_FRAME: usize = 16 * 1024 * 1024;

/// The frame header's width — a big-endian `u32`.
const HEADER: usize = 4;

/// Write one JSON frame.
///
/// The body is [`Value::to_string`] rather than a fallible serialization: a
/// `Value` is JSON already, so the error arm of `to_string(v)?` cannot be
/// reached from any input, and an unreachable branch is an untested one.
pub fn write_value(w: &mut dyn Write, v: &Value) -> io::Result<()> {
    write_bytes(w, v.to_string().as_bytes())
}

/// Write the end-of-stream terminator: a zero-length frame.
pub fn write_end(w: &mut dyn Write) -> io::Result<()> {
    write_bytes(w, &[])
}

/// Read one JSON frame: `Some(value)` a frame, `None` the terminator. An
/// oversized length, a short stream and a body that is not JSON are all errors
/// — the strict-decode discipline the boundary keeps, held at the framing so
/// nothing above it has to.
pub fn read_value(r: &mut dyn Read) -> io::Result<Option<Value>> {
    let Some(body) = read_bytes(r)? else {
        return Ok(None);
    };
    serde_json::from_slice(&body).map(Some).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("frame is not JSON: {e}"),
        )
    })
}

/// The length-prefixed write both spellings above share.
///
/// The bound and the header's own width are **one decision**, so they are one
/// match: a body larger than a `u32` can count and a body larger than the wire
/// permits are the same refusal, and splitting them would leave a conversion
/// arm no input on a 64-bit machine can reach.
pub(crate) fn write_bytes(w: &mut dyn Write, body: &[u8]) -> io::Result<()> {
    let len = match u32::try_from(body.len()) {
        Ok(len) if body.len() <= MAX_FRAME => len,
        _ => return Err(oversize(body.len())),
    };
    w.write_all(&len.to_be_bytes())?;
    w.write_all(body)?;
    w.flush()
}

/// The length-prefixed read: `None` for the zero-length terminator.
fn read_bytes(r: &mut dyn Read) -> io::Result<Option<Vec<u8>>> {
    let mut header = [0u8; HEADER];
    r.read_exact(&mut header)?;
    let len = u32::from_be_bytes(header) as usize;
    if len == 0 {
        return Ok(None);
    }
    if len > MAX_FRAME {
        return Err(oversize(len));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(Some(body))
}

/// The one refusal a length can earn, said the same way in both directions.
fn oversize(len: usize) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("frame of {len} bytes exceeds the {MAX_FRAME}-byte limit"),
    )
}

#[cfg(test)]
mod tests;
