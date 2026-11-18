//! **The paced read** (yog's `docs/REMOTE.md` §5.7): a run posted in pieces is
//! read no faster than it is posted.
//!
//! It is tested at the sink because the fact is about one buffer, and a real
//! child cannot be made to fill one on demand: whether a pipe yields a
//! megabyte before it runs dry is the scheduler's to decide.

use super::super::CAPTURE_LIMIT;
use super::sink;

/// A source that never runs dry and never blocks: the tool that writes faster
/// than anything reads.
fn firehose() -> Box<dyn std::io::Read> {
    Box::new(std::io::repeat(b'x'))
}

#[test]
fn a_paced_sink_stops_reading_at_the_limit_and_drops_nothing() {
    let mut paced = sink("stdout", Some(firehose()), true);
    assert!(paced.pump(), "it read");
    assert_eq!(paced.bytes.len(), CAPTURE_LIMIT);
    assert!(!paced.pump(), "a full buffer is not read into");
    assert_eq!(paced.dropped, 0);
    assert_eq!(paced.elided(), "");
}

#[test]
fn a_cut_empties_the_buffer_and_the_next_read_fills_it_again() {
    let mut paced = sink("stdout", Some(firehose()), true);
    paced.pump();
    assert_eq!(paced.cut().len(), CAPTURE_LIMIT);
    assert!(paced.bytes.is_empty());
    assert!(paced.pump());
    assert_eq!(paced.bytes.len(), CAPTURE_LIMIT);
}
