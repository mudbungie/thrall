//! **Reaching the server for a run** (yog's `docs/REMOTE.md` §5.7): the one
//! dial a post may try twice, and the real executor under a real refusal.
//!
//! Split from [`stream`](super::stream) at the line cap, on the seam between
//! what the loop does with an answer and whether there was a server to give
//! one.

use super::super::hold::{Ending, hold};
use super::stream::{chatty, handed, partial, posted, stop, streaming};
use super::{aside, gesture, idle, ops, receipt, refusal, said, set};
use crate::config::Local;
use crate::test_support::server::Dial;
use crate::tools::Tool;
use serde_json::json;
use std::time::{Duration, Instant};

#[test]
fn a_server_that_could_not_be_reached_for_a_run_s_first_post_is_dialled_once_more() {
    let run = Dial::run(vec![posted("i-1"), posted("i-1"), receipt("i-1")]);
    let script = [handed(), vec![Dial::Unreachable, run], stop()].concat();
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
        ops(&server).len(),
        7,
        "the run was posted whole on the second dial"
    );
}

#[test]
fn a_server_unreachable_twice_ends_the_run_and_the_channel_is_dialled_again() {
    let script = [handed(), vec![Dial::Unreachable, Dial::Unreachable]].concat();
    let (_scratch, server, mut channel) = streaming(script);
    let Ending::Again { held, .. } =
        hold(&mut channel, &set(), chatty, &aside(), None, &mut idle())
    else {
        panic!("a server that cannot be reached is worth another dial");
    };
    assert_eq!(held, None);
    assert_eq!(
        ops(&server),
        ["advertise", "invocations"],
        "no post ever landed"
    );
}

/// **The two halves joined**: the real executor, a real child that would run
/// for half a minute, and a server that refuses its first post. The child is
/// stopped, and the channel is back at its read long before the child would
/// have finished.
#[test]
fn a_refused_post_stops_a_real_child_and_the_channel_reads_on() {
    let slow = Local {
        tool: Tool {
            name: "Bash".to_owned(),
            description: "a tool that takes its time".to_owned(),
            input_schema: json!({"type": "object"}),
            subject_cwd: false,
        },
        command: ["/bin/sh", "-c", "echo started; sleep 30"]
            .map(str::to_owned)
            .to_vec(),
        cwd: None,
    };
    let refused = Dial::one(refusal("nobody is listening for \"i-1\""));
    let (_scratch, server, mut channel) = streaming([handed(), vec![refused], stop()].concat());
    let asked = Instant::now();
    let ending = hold(
        &mut channel,
        &[slow],
        crate::exec::handoff,
        &aside(),
        None,
        &mut idle(),
    );
    assert_eq!(said(ending), "stop");
    assert!(
        asked.elapsed() < Duration::from_secs(20),
        "the child was waited out"
    );
    assert_eq!(gesture(&server, 2), partial("i-1", "started\n"));
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
}
