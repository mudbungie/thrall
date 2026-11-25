//! **Pushed work** (yog's `docs/REMOTE.md` §5.8): one read held open, a frame
//! per job, every job started at once on its own thread and posted on its own
//! connection — against a stand-in that holds the read open for as long as the
//! test does, and writes each frame when the test says.
//!
//! The executors here are fn pointers, so what they share with a test is a
//! static of their own; no two tests share one.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

use super::super::hold::{Ending, hold};
use super::super::redial::redial;
use super::{advertised, aside, echo, idle, ops, receipt, row, set, work};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::config::Local;
use crate::invocation::{Capture, Invocation, Piece};
use crate::run::hold::ending::SUPERSEDED;
use crate::run::{Notice, Pause};
use crate::test_support::server::{Dial, Server};
use crate::test_support::{Notices, Scratch, mint, unwaited};

/// A channel to a server that pushes, scripted per dial in the order dialled.
pub(super) fn pushing(script: Vec<Dial>) -> (Scratch, Server, Channel) {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let server = Server::pushing(scratch.path(), script);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    (scratch, server, Channel::open(&held).expect("opened"))
}

/// A read the test holds open, and the end the test writes it from.
pub(super) fn read() -> (Sender<Option<Value>>, Dial) {
    let (frames, held) = mpsc::channel();
    (frames, Dial::Held(Arc::new(Mutex::new(held))))
}

/// Write one frame on a held read.
pub(super) fn push(read: &Sender<Option<Value>>, frame: Value) {
    read.send(Some(frame)).expect("the read is held");
}

/// End a held read the way a newer activation does (REMOTE §5.1).
pub(super) fn supersede(read: &Sender<Option<Value>>) {
    push(
        read,
        json!({"ok": true, "kind": "invocations", "rows": [], "superseded": true}),
    );
    read.send(None).expect("the read is held");
}

/// Wait for `what`, and fail the test naming it after ten seconds.
pub(super) fn until(what: &str, done: impl Fn() -> bool) {
    let end = Instant::now() + Duration::from_secs(10);
    while !done() {
        assert!(Instant::now() < end, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// How many times the server has been handed `op`.
pub(super) fn heard(server: &Server, op: &str) -> usize {
    ops(server).iter().filter(|said| *said == op).count()
}

/// The `holding` every read stated, in order.
fn holdings(server: &Server) -> Vec<Value> {
    let reads = server.heard().into_iter();
    let reads = reads.filter(|said| said.get("op") == Some(&json!("invocations")));
    reads.map(|said| said["holding"].clone()).collect()
}

/// One channel's whole life, on a thread of its own, so the test can drive
/// the read it holds.
pub(super) fn serve(
    channel: Channel,
    handoff: crate::run::Handoff,
    notice: Notice,
    pause: Pause,
) -> std::thread::JoinHandle<String> {
    std::thread::spawn(move || redial(Ok(channel), &set(), handoff, &notice, &pause))
}

/// What a job answers: its own id, so two posts can be told apart.
pub(super) fn ran(invocation: &Invocation) -> Capture {
    Capture {
        stdout: invocation.id.clone(),
        stderr: String::new(),
        exit_code: 0,
    }
}

/// Jobs inside the executor right now, and the most there have ever been.
static INSIDE: AtomicUsize = AtomicUsize::new(0);
static MOST: AtomicUsize = AtomicUsize::new(0);

/// An executor that waits until a second job is running beside it.
fn together(
    _: &[Local],
    invocation: &Invocation,
    _: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    let now = INSIDE.fetch_add(1, Ordering::SeqCst) + 1;
    MOST.fetch_max(now, Ordering::SeqCst);
    until("a second job beside this one", || {
        MOST.load(Ordering::SeqCst) >= 2
    });
    INSIDE.fetch_sub(1, Ordering::SeqCst);
    ran(invocation)
}

/// **Two jobs pushed back to back run at the same time**, each posting on its
/// own connection, and the read states `holding` — empty — from the first.
#[test]
fn two_jobs_pushed_back_to_back_run_at_once() {
    let (frames, held) = read();
    let posts = |id| Dial::run(vec![receipt(id)]);
    let script = vec![Dial::one(advertised()), held, posts("i-1"), posts("i-2")];
    let (_scratch, server, channel) = pushing(script);
    let serving = serve(channel, together, aside(), unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    push(&frames, work(vec![row("i-2", "Bash")]));
    until("both posts", || heard(&server, "complete") == 2);
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(MOST.load(Ordering::SeqCst), 2);
    assert_eq!(holdings(&server), [json!([])]);
    assert_eq!(
        ops(&server),
        ["advertise", "invocations", "complete", "complete"]
    );
}

static FIRST: AtomicBool = AtomicBool::new(false);
static SECOND: AtomicBool = AtomicBool::new(false);

/// An executor whose `i-1` runs until `i-2` has started.
fn first_waits(
    _: &[Local],
    invocation: &Invocation,
    _: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    let (mine, other) = if invocation.id == "i-1" {
        (&FIRST, &SECOND)
    } else {
        (&SECOND, &FIRST)
    };
    mine.store(true, Ordering::SeqCst);
    until("the other job", || other.load(Ordering::SeqCst));
    ran(invocation)
}

/// **A job pushed while another runs starts at once** — it is not held
/// behind the one already running, and the one running is not interrupted.
#[test]
fn a_job_pushed_while_another_runs_starts_at_once() {
    let (frames, held) = read();
    let posts = || Dial::run(vec![receipt("i")]);
    let (_scratch, server, channel) =
        pushing(vec![Dial::one(advertised()), held, posts(), posts()]);
    let serving = serve(channel, first_waits, aside(), unwaited());
    push(&frames, work(vec![row("i-1", "Bash")]));
    until("the first job to start", || FIRST.load(Ordering::SeqCst));
    push(&frames, work(vec![row("i-2", "Bash")]));
    until("both posts", || heard(&server, "complete") == 2);
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
}

static RUNNING: AtomicBool = AtomicBool::new(false);
static RELEASED: AtomicBool = AtomicBool::new(false);

/// An executor that runs until the test lets it finish.
fn held_open(
    _: &[Local],
    invocation: &Invocation,
    _: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    RUNNING.store(true, Ordering::SeqCst);
    until("the test to let the job finish", || {
        RELEASED.load(Ordering::SeqCst)
    });
    ran(invocation)
}

/// **Every reconnect states what this host holds**: a job still running is
/// listed, and once its final post is answered it is not.
#[test]
fn a_reconnect_states_every_job_still_running() {
    let ((one, first), (two, second), (three, third)) = (read(), read(), read());
    let script = vec![
        Dial::one(advertised()),
        first,
        Dial::one(advertised()),
        second,
        Dial::run(vec![receipt("i-1")]),
        Dial::one(advertised()),
        third,
    ];
    let (_scratch, server, channel) = pushing(script);
    let pause: Pause = Arc::new(|_| std::thread::sleep(Duration::from_millis(300)));
    let serving = serve(channel, held_open, aside(), pause);
    push(&one, work(vec![row("i-1", "Bash")]));
    until("the job to start", || RUNNING.load(Ordering::SeqCst));
    drop(one);
    until("the second read", || heard(&server, "invocations") == 2);
    RELEASED.store(true, Ordering::SeqCst);
    until("the job's post", || heard(&server, "complete") == 1);
    drop(two);
    until("the third read", || heard(&server, "invocations") == 3);
    supersede(&three);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(holdings(&server), [json!([]), json!(["i-1"]), json!([])]);
}

/// **A heartbeat is neither work nor the end**: three empty frames, then a
/// job, all on the one read — nothing is run for them and nothing reconnects.
#[test]
fn a_heartbeat_is_read_past() {
    let (frames, held) = read();
    let script = vec![
        Dial::one(advertised()),
        held,
        Dial::run(vec![receipt("i-1")]),
    ];
    let (_scratch, server, channel) = pushing(script);
    let serving = serve(channel, echo, aside(), unwaited());
    for _ in 0..3 {
        push(&frames, work(vec![]));
    }
    push(&frames, work(vec![row("i-1", "Bash")]));
    until("the post", || heard(&server, "complete") == 1);
    supersede(&frames);
    assert_eq!(serving.join().expect("served"), SUPERSEDED);
    assert_eq!(ops(&server), ["advertise", "invocations", "complete"]);
}

/// **A read with no frame for four heartbeat intervals is a dead connection**:
/// the foot dials again, advertises, and opens the read again. The bound is
/// the channel's read bound, shortened here from its two minutes.
#[test]
fn a_quiet_read_is_dialled_again() {
    let ((quiet, first), (two, second)) = (read(), read());
    let script = vec![
        Dial::one(advertised()),
        first,
        Dial::one(advertised()),
        second,
    ];
    let (_scratch, server, channel) = pushing(script);
    let channel = channel.hushed(Duration::from_millis(300));
    let (notices, notice) = Notices::new();
    let serving = serve(channel, echo, notice, unwaited());
    until("the second read", || heard(&server, "invocations") == 2);
    supersede(&two);
    assert_eq!(serving.join().expect("served"), SUPERSEDED.to_owned());
    assert_eq!(
        ops(&server),
        ["advertise", "invocations", "advertise", "invocations"]
    );
    assert!(
        notices.heard()[0].contains("receive"),
        "{:?}",
        notices.heard()
    );
    drop(quiet);
}

/// **An edition-31 server is read serially**: no `holding`, and nothing held
/// open — the old loop, byte for byte.
#[test]
fn an_edition_31_server_gets_the_serial_read() {
    let script = vec![Dial::one(advertised()), Dial::one(super::refusal("stop"))];
    let (_scratch, server, mut channel) = super::stream::streaming(script);
    let ending = hold(&mut channel, &set(), echo, &aside(), None, &mut idle());
    assert!(matches!(ending, Ending::Again { .. }));
    assert_eq!(super::gesture(&server, 1), json!({"op": "invocations"}));
}

/// **A pushing read that cannot be dialled is the wire**, at the read's leg,
/// and the channel is dialled again.
#[test]
fn a_pushing_read_that_cannot_be_dialled_is_the_wire() {
    let (_scratch, _server, mut channel) =
        pushing(vec![Dial::one(advertised()), Dial::Unreachable]);
    let ending = hold(&mut channel, &set(), echo, &aside(), None, &mut idle());
    assert!(matches!(ending, Ending::Again { served: false, .. }));
}
