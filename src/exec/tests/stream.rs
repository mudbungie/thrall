//! **A run posted while it is produced** (yog's `docs/REMOTE.md` §5.7): the
//! executor's half, watched through a post that records what it was handed.
//!
//! What the loop does with a post's answer is `run::tests::stream`'s; here the
//! post is a closure, so every piece is a value and the answer is a script.

use super::super::stream::boundary;
use super::super::{CADENCE, CAPTURE_LIMIT, STOPPED};
use super::{call, execute, handoff, tool};
use crate::invocation::{Capture, Piece};
use crate::test_support::Scratch;
use serde_json::json;
use std::time::{Duration, Instant};

/// Long enough that no test here reaches it: a post it sees was earned by text.
fn never() -> Duration {
    Duration::from_mins(10)
}

/// Run `script` posted in pieces, answering `true` to the first `listening`
/// posts and `false` to the next. What comes back is every piece handed over,
/// and the capture.
fn streamed(script: &str, every: Duration, listening: usize) -> (Vec<Piece>, Capture) {
    let set = [tool("Streams", script)];
    let mut pieces = Vec::new();
    let mut post = |piece: Piece| {
        pieces.push(piece);
        pieces.len() <= listening
    };
    let capture = execute(&set, &call("Streams", json!({})), Some(&mut post), every);
    (pieces, capture)
}

/// Every post's stdout and then the capture's: what the asker absorbs.
fn absorbed(pieces: &[Piece], capture: &Capture) -> String {
    let posted: String = pieces.iter().map(|piece| piece.stdout.as_str()).collect();
    posted + &capture.stdout
}

/// **A slow child's first line is posted before it exits** — which is the whole
/// of what the streamed form is for. The marker is written between the two
/// lines, so a first piece that arrives while it is absent arrived mid-run.
#[test]
fn a_slow_child_s_first_line_is_posted_before_it_exits() {
    let scratch = Scratch::new();
    let later = scratch.join("later");
    let set = [tool(
        "Slow",
        &format!("echo first; sleep 1; : > {}; echo second", later.display()),
    )];
    let mut early = Vec::new();
    let mut post = |piece: Piece| {
        early.push((piece.stdout, later.exists()));
        true
    };
    let got = execute(&set, &call("Slow", json!({})), Some(&mut post), never());
    assert_eq!(
        early.first(),
        Some(&("first\n".to_owned(), false)),
        "the first line was not posted while the child was still running"
    );
    let posted: String = early.iter().map(|(text, _)| text.as_str()).collect();
    assert_eq!(posted + &got.stdout, "first\nsecond\n");
    assert_eq!(got.exit_code, 0);
}

/// **A silent child is posted for anyway**, at the cadence: two empty strings
/// say *still running*, which is what keeps the server from ending the run in
/// doubt.
#[test]
fn a_silent_child_posts_an_empty_partial_at_the_cadence() {
    let (pieces, got) = streamed(
        "sleep 1; printf done",
        Duration::from_millis(100),
        usize::MAX,
    );
    let empty = pieces.iter().filter(|piece| **piece == Piece::default());
    assert!(empty.count() >= 2, "{pieces:?}");
    assert_eq!(absorbed(&pieces, &got), "done");
    assert_eq!(got.exit_code, 0);
}

/// **A post answered `false` is the stop, and it reaches the child's process
/// GROUP**: the helper the tool started does not outlive the run.
#[test]
fn a_refused_post_kills_the_child_s_process_group() {
    let scratch = Scratch::new();
    let started = scratch.join("started");
    let outlived = scratch.join("outlived");
    let script = format!(
        "( : > {started}; sleep 1; : > {outlived} ) & \
         while [ ! -e {started} ]; do :; done; echo go; sleep 30",
        started = started.display(),
        outlived = outlived.display()
    );
    let asked = Instant::now();
    let (pieces, got) = streamed(&script, never(), 0);
    assert!(
        asked.elapsed() < Duration::from_secs(20),
        "it was waited out"
    );
    assert_eq!(pieces.len(), 1, "a stopped run is posted for no further");
    assert_eq!(got.exit_code, STOPPED, "{got:?}");
    assert!(got.stderr.contains("nobody is listening"), "{got:?}");
    // Past the helper's own wait, so a survivor has had its chance to write.
    std::thread::sleep(Duration::from_millis(1_500));
    assert!(!outlived.exists(), "the helper outlived the stop");
}

/// **Posts concatenate to exactly the child's output across a multi-byte
/// boundary.** The child stops mid-character twice, and the cadence fires
/// while it does — so a cut that split a character would post a replacement
/// character, and one that waited would post nothing and then the whole.
#[test]
fn posts_concatenate_to_the_child_s_output_across_a_multi_byte_boundary() {
    let script = "printf 'a\\303'; sleep 1; printf '\\251\\342\\202'; sleep 1; printf '\\254z'";
    let (pieces, got) = streamed(script, Duration::from_millis(100), usize::MAX);
    assert_eq!(absorbed(&pieces, &got), "a\u{e9}\u{20ac}z");
    assert!(
        pieces.len() > 2,
        "the cadence fired mid-character: {pieces:?}"
    );
    assert_eq!(got.exit_code, 0);
}

/// **The limit bounds a post and not the run**: three times the limit comes
/// back whole, in posts none of which is over it, and nothing is dropped.
#[test]
fn a_run_past_the_limit_loses_nothing_when_it_is_posted_in_pieces() {
    let whole = CAPTURE_LIMIT * 3;
    let (pieces, got) = streamed(&format!("yes hello | head -c {whole}"), never(), usize::MAX);
    assert_eq!(absorbed(&pieces, &got).len(), whole);
    assert!(
        pieces
            .iter()
            .all(|piece| piece.stdout.len() <= CAPTURE_LIMIT)
    );
    assert!(!got.stderr.contains("exceeded"), "{}", got.stderr);
}

/// **Stderr streams too** — a sign-in prints its device code there (REMOTE
/// §5.7) — and the capture's own stderr is only what was not yet posted.
#[test]
fn stderr_is_posted_in_pieces_beside_stdout() {
    let (pieces, got) = streamed(
        "echo code >&2; sleep 1; echo late >&2; exit 3",
        never(),
        usize::MAX,
    );
    let posted: String = pieces.iter().map(|piece| piece.stderr.as_str()).collect();
    assert_eq!(
        pieces.first().map(|piece| piece.stderr.as_str()),
        Some("code\n")
    );
    assert_eq!(posted + &got.stderr, "code\nlate\n");
    assert_eq!(got.exit_code, 3);
}

/// The hand-off with somewhere to post is the executor at this box's cadence —
/// a third of the server's thirty seconds of patience for a silent run.
#[test]
fn the_handoff_posts_at_this_box_s_own_cadence() {
    assert_eq!(CADENCE, Duration::from_secs(10));
    let set = [tool("Echo", "cat")];
    let mut pieces = Vec::new();
    let mut post = |piece: Piece| {
        pieces.push(piece);
        true
    };
    let got = handoff(&set, &call("Echo", json!({"a": 1})), Some(&mut post));
    assert_eq!(absorbed(&pieces, &got), r#"{"a":1}"#);
}

/// **Where a stream may be cut**: everywhere but inside a character the child
/// has not finished writing.
#[test]
fn a_cut_never_splits_a_character_and_never_waits_on_bytes_that_are_not_one() {
    assert_eq!(boundary(b""), 0);
    assert_eq!(boundary(b"plain"), 5);
    assert_eq!(boundary("caf\u{e9}".as_bytes()), 5, "a finished character");
    assert_eq!(boundary(b"caf\xC3"), 3, "the head of two bytes");
    assert_eq!(boundary(b"\xE2\x82"), 0, "two of three");
    assert_eq!(boundary("\u{20ac}".as_bytes()), 3, "three of three");
    assert_eq!(boundary(b"x\xF0\x9F\x98"), 1, "three of four");
    assert_eq!(boundary("\u{1f600}".as_bytes()), 4, "four of four");
    assert_eq!(boundary(b"\x80\x80\x80\x80"), 4, "no lead byte to wait for");
    assert_eq!(boundary(b"\x80"), 1, "a stray continuation");
}
