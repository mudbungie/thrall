//! **A capture is N ≥ 1 posts** (yog's `docs/REMOTE.md` §5.7): the loop's half
//! of the streamed form, against a server that states the edition which reads
//! it — and against one that does not.
//!
//! The executor here is a script that posts twice and returns the rest, so
//! every frame below is literal; what a real child does under the same post is
//! `exec::tests::stream`'s, and the last test joins the two.

use super::super::held::Held;
use super::super::hold::{Ending, hold};
use super::{
    advertised, aside, gesture, idle, ops, receipt, refusal, row, said, server_at, set, work,
};
use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::config::Local;
use crate::exec::STOPPED;
use crate::invocation::{Capture, Invocation, Piece};
use crate::test_support::server::{Dial, Server};
use crate::test_support::{Scratch, mint};
use serde_json::{Value, json};

/// What a run is worth when nobody reads it.
fn ended(stdout: &str, exit_code: i32) -> Capture {
    Capture {
        stdout: stdout.to_owned(),
        stderr: String::new(),
        exit_code,
    }
}

/// An executor that says `one`, then `two`, and answers `rest` — or all three
/// at once, to a loop that hands it nowhere to post.
pub(super) fn chatty(
    _: &[Local],
    _: &Invocation,
    post: Option<&mut dyn FnMut(Piece) -> bool>,
) -> Capture {
    let piece = |stdout: &str| Piece {
        stdout: stdout.to_owned(),
        stderr: String::new(),
    };
    let Some(post) = post else {
        return ended("onetworest", 0);
    };
    if post(piece("one")) && post(piece("two")) {
        ended("rest", 0)
    } else {
        ended("", STOPPED)
    }
}

/// The receipt of a `partial` post: the lease fact, and no capture.
pub(super) fn posted(id: &str) -> Value {
    json!({"ok": true, "kind": "routed", "invocation": id,
           "partial": {"stdout": "", "stderr": ""}})
}

/// A `partial` post, as this foot writes it.
pub(super) fn partial(id: &str, stdout: &str) -> Value {
    json!({"op": "complete", "invocation": id,
           "partial": {"stdout": stdout, "stderr": ""}})
}

/// A channel to a server that reads a capture in pieces, scripted per dial.
pub(super) fn streaming(script: Vec<Dial>) -> (Scratch, Server, Channel) {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let server = Server::streaming(scratch.path(), script);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    (scratch, server, channel)
}

/// The two dials every channel opens with: its advertisement, and the read
/// that hands it `i-1`.
pub(super) fn handed() -> Vec<Dial> {
    vec![
        Dial::one(advertised()),
        Dial::one(work(vec![row("i-1", "Bash")])),
    ]
}

/// The two dials that follow a hand-off, and stop the channel at its next
/// read.
pub(super) fn stop() -> Vec<Dial> {
    vec![Dial::one(advertised()), Dial::one(refusal("stop"))]
}

#[test]
fn a_run_is_posted_in_pieces_over_one_connection() {
    let run = Dial::run(vec![posted("i-1"), posted("i-1"), receipt("i-1")]);
    let script = [handed(), vec![run], stop()].concat();
    let (_scratch, server, mut channel) = streaming(script);
    assert_eq!(
        said(hold(
            &mut channel,
            &set(),
            chatty,
            &aside(),
            None,
            &mut idle()
        )),
        "stop"
    );
    assert_eq!(
        ops(&server),
        [
            "advertise",
            "invocations",
            "complete",
            "complete",
            "complete",
            "advertise",
            "invocations"
        ]
    );
    assert_eq!(gesture(&server, 2), partial("i-1", "one"));
    assert_eq!(gesture(&server, 3), partial("i-1", "two"));
    assert_eq!(
        gesture(&server, 4),
        json!({"op": "complete", "invocation": "i-1",
               "capture": {"stdout": "rest", "stderr": "", "exit_code": 0}}),
        "the last post carries the verdict and only the text not yet posted"
    );
    let dials = server
        .heard()
        .iter()
        .filter(|said| said.get("protocol").is_some())
        .count();
    assert_eq!(
        dials, 5,
        "three posts, one connection: five dials and not seven"
    );
}

#[test]
fn an_older_edition_gets_one_post() {
    let scratch = Scratch::new();
    let script = vec![
        advertised(),
        work(vec![row("i-1", "Bash")]),
        receipt("i-1"),
        advertised(),
        refusal("stop"),
    ];
    let server = server_at(scratch.path(), script);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let mut channel = Channel::open(&held).expect("opened");
    assert_eq!(
        said(hold(
            &mut channel,
            &set(),
            chatty,
            &aside(),
            None,
            &mut idle()
        )),
        "stop"
    );
    assert_eq!(
        ops(&server),
        [
            "advertise",
            "invocations",
            "complete",
            "advertise",
            "invocations"
        ]
    );
    assert_eq!(
        gesture(&server, 2),
        json!({"op": "complete", "invocation": "i-1",
               "capture": {"stdout": "onetworest", "stderr": "", "exit_code": 0}}),
        "a server below the stamp is never written a partial"
    );
}

#[test]
fn a_refused_post_drops_the_run_and_the_channel_takes_its_next_invocation() {
    let script = [
        handed(),
        vec![
            Dial::one(refusal("nobody is listening for \"i-1\"")),
            Dial::one(advertised()),
            Dial::one(work(vec![row("i-2", "Bash")])),
            Dial::run(vec![posted("i-2"), posted("i-2"), receipt("i-2")]),
        ],
        stop(),
    ]
    .concat();
    let (_scratch, server, mut channel) = streaming(script);
    assert_eq!(
        said(hold(
            &mut channel,
            &set(),
            chatty,
            &aside(),
            None,
            &mut idle()
        )),
        "stop",
        "the refusal ended nothing: it is the asker's ordinary stop"
    );
    let completes: Vec<Value> = server
        .heard()
        .into_iter()
        .filter(|said| said["op"] == "complete")
        .collect();
    assert_eq!(
        completes.len(),
        4,
        "one refused post, then a whole second run"
    );
    assert_eq!(completes[0], partial("i-1", "one"));
    assert!(
        completes
            .iter()
            .all(|said| said["invocation"] != "i-1" || said.get("capture").is_none()),
        "a dropped run posts no capture"
    );
}

#[test]
fn a_refused_capture_post_is_dropped_and_the_channel_reads_on() {
    let run = Dial::run(vec![
        posted("i-1"),
        posted("i-1"),
        refusal("no invocation \"i-1\""),
    ]);
    let (_scratch, server, mut channel) = streaming([handed(), vec![run], stop()].concat());
    assert_eq!(
        said(hold(
            &mut channel,
            &set(),
            chatty,
            &aside(),
            None,
            &mut idle()
        )),
        "stop"
    );
    assert_eq!(ops(&server).len(), 7, "it presented and read again");
}

#[test]
fn a_post_the_wire_swallowed_ends_the_run_and_the_channel_is_dialled_again() {
    let (_scratch, _server, mut channel) = streaming([handed(), vec![Dial::Vanishes]].concat());
    let Ending::Again {
        held, predecessor, ..
    } = hold(&mut channel, &set(), chatty, &aside(), None, &mut idle())
    else {
        panic!("a wire that dropped is worth another dial");
    };
    assert!(!predecessor, "the wire is nobody's refusal");
    assert_eq!(
        held, None,
        "a run that never finished left no capture to hold"
    );
}

#[test]
fn the_capture_s_own_post_swallowed_is_held_with_only_what_was_not_posted() {
    let run = Dial::run(vec![posted("i-1"), posted("i-1")]);
    let (_scratch, _server, mut channel) = streaming([handed(), vec![run]].concat());
    let Ending::Again { held, .. } =
        hold(&mut channel, &set(), chatty, &aside(), None, &mut idle())
    else {
        panic!("a dropped completion must be worth another dial");
    };
    assert_eq!(
        held,
        Some(Held {
            id: "i-1".to_owned(),
            capture: ended("rest", 0),
        }),
        "the next dial posts the remainder, and the server already holds the rest"
    );
}

#[test]
fn an_answer_no_gesture_can_earn_ends_the_channel_in_the_middle_of_a_run() {
    let board = Dial::one(json!({"ok": true, "kind": "board"}));
    let (_scratch, _server, mut channel) = streaming([handed(), vec![board]].concat());
    let ending = hold(&mut channel, &set(), chatty, &aside(), None, &mut idle());
    assert_eq!(
        ending,
        Ending::Over("reply: unusable kind \"board\"".to_owned())
    );
}
