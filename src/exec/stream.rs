//! **A run posted while it is produced** (yog's `docs/REMOTE.md` §5.7): how
//! often, and where a stream may be cut.
//!
//! **A capture is N ≥ 1 posts.** To a server that reads the streamed form, the
//! executor hands over what the child has written whenever there is something
//! to hand over or [`CADENCE`] has passed, and the answer to each hand-over is
//! whether anybody is still listening. That answer **replaces the fixed
//! deadline**: a model call outlasts two minutes routinely, and the only party
//! that knows how long a run is worth is the one waiting for it. To an older
//! server nothing here is spent — one capture, under the deadline, as before.
//!
//! **The post is a parameter and its answer is a `bool`**: `true` while
//! somebody is listening, `false` when the run must stop — the server refused
//! the post, or could not be reached for it. Which of those it was is the
//! loop's to know (`run::held`); the executor only stops.

use std::time::Duration;

/// **How long a child may write nothing before this box says so anyway.** A
/// post of two empty strings says only *still running*, and the server ends a
/// run in doubt after thirty seconds without one (REMOTE §5.7) — so this is a
/// third of that, which a slow post and a lost one both fit inside.
pub const CADENCE: Duration = Duration::from_secs(10);

/// **Where `bytes` may be cut so that no character is split**: its whole
/// length, unless it ends inside a multi-byte sequence — then the start of
/// that sequence, which stays behind for the next cut.
///
/// A cut in front of a lead byte is always safe, because no sequence continues
/// through one: the text of the two halves, each transcoded on its own, is the
/// text of the whole. Bytes that are simply not UTF-8 are not waited on; they
/// transcode to the replacement character on either side of any cut.
pub(super) fn boundary(bytes: &[u8]) -> usize {
    let len = bytes.len();
    for back in 1..=3.min(len) {
        let at = len - back;
        let needs = match bytes.get(at) {
            Some(0xC0..=0xDF) => 2,
            Some(0xE0..=0xEF) => 3,
            Some(0xF0..=0xFF) => 4,
            // A continuation byte: the sequence began further back.
            Some(0x80..=0xBF) => continue,
            // ASCII ends every sequence before it.
            _ => return len,
        };
        return if needs > back { at } else { len };
    }
    len
}
