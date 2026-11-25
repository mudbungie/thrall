//! **A pushed job's own life** (yog's `docs/REMOTE.md` §5.6, §5.8): started
//! once, posted on its own connection, carried while the wire swallows its
//! capture, and waited for when the channel is over.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::push::{heard, push, pushing, ran, read, serve, supersede, until};
use super::stream::chatty;
use super::{advertised, aside, echo, ops, receipt, row, work};
use crate::config::Local;
use crate::invocation::{Capture, Invocation, Piece};
use crate::run::Pause;
use crate::run::hold::ending::SUPERSEDED;
use crate::test_support::server::Dial;
use crate::test_support::{Notices, unwaited};

static STARTS: AtomicUsize = AtomicUsize::new(0);
static LET_GO: AtomicBool = AtomicBool::new(false);

/// An executor that counts its starts and runs until the test lets it go.
fn counted(
    _: &[Local],
    invocation: &Invocation,
    _: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    STARTS.fetch_add(1, Ordering::SeqCst);
    until("the test to let the job go", || {
        LET_GO.load(Ordering::SeqCst)
    });
    ran(invocation)
}

/// **A pushed id already held is not started twice** (REMOTE §5.8).
#[test]
fn a_job_pushed_twice_is_started_once() {
    let (frames, held) = read();
    let script = vec![
        Dial::one(advertised()),
        held,
        Dial::run(vec![receipt("i-1")]),
    ];
    let (_scratch, server, channel) = pushing(script);
    let serving = serve(channel, counted, aside(), unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    push(&frames, work(vec![row("i-1", "Bash")]));
    push(&frames, work(vec![]));
    until("the job to start", || STARTS.load(Ordering::SeqCst) == 1);
    LET_GO.store(true, Ordering::SeqCst);
    until("the post", || heard(&server, "complete") == 1);
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(STARTS.load(Ordering::SeqCst), 1);
}

static STOPPED: AtomicBool = AtomicBool::new(false);
static ON: AtomicBool = AtomicBool::new(false);

/// An executor that runs until the test has superseded the read.
fn outlives(
    _: &[Local],
    invocation: &Invocation,
    _: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    ON.store(true, Ordering::SeqCst);
    until("the read to be superseded", || {
        STOPPED.load(Ordering::SeqCst)
    });
    std::thread::sleep(Duration::from_millis(200));
    ran(invocation)
}

/// **A channel that is over waits for the jobs it started**: superseded while
/// a job runs, it returns only after that job's capture was posted — so a
/// process whose channels have all stopped never exits with a job unposted.
#[test]
fn a_superseded_channel_waits_for_its_jobs() {
    let (frames, held) = read();
    let script = vec![
        Dial::one(advertised()),
        held,
        Dial::run(vec![receipt("i-1")]),
    ];
    let (_scratch, server, channel) = pushing(script);
    let serving = serve(channel, outlives, aside(), unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    until("the job to start", || ON.load(Ordering::SeqCst));
    supersede(&frames);
    STOPPED.store(true, Ordering::SeqCst);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(heard(&server, "complete"), 1);
}

/// **A capture the wire swallows is posted again until it is answered**, on a
/// connection of the job's own, and said once (REMOTE §5.6 ruling 1's note: a
/// pushing host keeps the id in `holding`, so the read never releases it).
#[test]
fn a_swallowed_capture_is_posted_again_until_answered() {
    let (frames, held) = read();
    let script = vec![
        Dial::one(advertised()),
        held,
        Dial::Vanishes,
        Dial::Vanishes,
        Dial::run(vec![receipt("i-1")]),
    ];
    let (_scratch, server, channel) = pushing(script);
    let (notices, notice) = Notices::new();
    let serving = serve(channel, echo, notice, unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    until("the third post", || heard(&server, "complete") == 3);
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    let said = notices.heard();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("held and posted again"), "{said:?}");
}

/// **A channel that is over stops posting a swallowed capture**, as the
/// serial loop drops one on an `Over`: the job's wait ends after the channel
/// was superseded, and nothing more is posted.
#[test]
fn an_over_channel_stops_posting_a_swallowed_capture() {
    let (frames, held) = read();
    let script = vec![Dial::one(advertised()), held, Dial::Vanishes];
    let (_scratch, server, channel) = pushing(script);
    let read = Arc::new(Mutex::new(frames.clone()));
    let pause: Pause = Arc::new(move |_| {
        supersede(&read.lock().expect("the read"));
        std::thread::sleep(Duration::from_millis(300));
    });
    let serving = serve(channel, echo, aside(), pause);
    push(&frames, work(vec![row("i-1", "Bash")]));
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(ops(&server), ["advertise", "invocations", "complete"]);
}

/// **A `partial` the wire swallows stops the run and drops it**, exactly as
/// the serial loop does — the executor stops the child, nothing more is
/// posted for it, and this host says so. The read is not touched.
#[test]
fn a_swallowed_partial_drops_the_run() {
    let (frames, held) = read();
    let script = vec![Dial::one(advertised()), held, Dial::Vanishes];
    let (_scratch, server, channel) = pushing(script);
    let (notices, notice) = Notices::new();
    let serving = serve(channel, chatty, notice, unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    until("the run to be dropped", || !notices.heard().is_empty());
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert!(notices.heard()[0].contains("stopped and nothing more"));
    assert_eq!(ops(&server), ["advertise", "invocations", "complete"]);
}
