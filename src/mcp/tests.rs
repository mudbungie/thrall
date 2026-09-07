//! **The bridge as a tool command**: what one invocation says to a server,
//! what comes back, and what happens to a server that will not talk.
//!
//! **The fake server is an argv and not a file** (`/bin/sh -c <script>`),
//! which is the shape `exec`'s own suite already uses for a tool. Nothing is
//! written and nothing is made executable, so the ETXTBSY window a fixture
//! script opens — a write fd this process holds, copied into some other
//! thread's fork, and an exec of that same file inside the window — does not
//! exist here at all. It is dissolved rather than bracketed: `/bin/sh` is an
//! executable nobody in this process is writing.

use super::bridge;
use crate::test_support::Scratch;
use serde_json::{Value, json};

/// The rendering rows of DESIGN §6.5, one test apiece.
mod content;
/// Every way this leg refuses, and the sentence each one earns.
mod refusals;

/// A fake server, as the argv a document entry would carry.
fn server(script: &str) -> Vec<String> {
    vec!["/bin/sh".to_owned(), "-c".to_owned(), script.to_owned()]
}

/// A server that performs the whole conversation: the handshake, then every
/// line of `answers` in order. The `tools/call` request itself is written to
/// `log`, so any test may read back exactly what this box sent.
fn conversing(answers: &[String], log: &std::path::Path) -> Vec<String> {
    let mut said = String::new();
    for answer in answers {
        said.push_str("printf '%s\\n' '");
        said.push_str(answer);
        said.push_str("'; ");
    }
    server(&format!(
        "read -r line; printf '%s\\n' '{}'; read -r line; \
         read -r line; printf '%s\\n' \"$line\" > '{}'; {said}",
        handshake(),
        log.display()
    ))
}

/// The server's half of the handshake — a result, and nothing this end reads.
fn handshake() -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": {}}).to_string()
}

/// One bridged call against a server that answers with these lines, and the
/// request it received.
fn called(answers: &[String], input: &str) -> (crate::invocation::Capture, Value) {
    let scratch = Scratch::new();
    let log = scratch.join("call.json");
    let capture = bridge("fetch", &conversing(answers, &log), input);
    let sent = std::fs::read_to_string(&log).unwrap_or_default();
    (capture, serde_json::from_str(&sent).unwrap_or(Value::Null))
}

/// A bridged call whose input is the empty object — for the tests whose
/// subject is the answer rather than the ask.
fn answered(result: &str) -> crate::invocation::Capture {
    called(&[result.to_owned()], "{}").0
}

/// A `tools/call` result, as the server would send it back under this end's
/// second id.
fn result(body: Value) -> String {
    json!({"jsonrpc": "2.0", "id": 2, "result": body}).to_string()
}

/// A result carrying one text part, as a server would send it.
fn text(said: &str) -> String {
    result(json!({"content": [{"type": "text", "text": said}]}))
}

/// **The tool contract, one for one**: the invocation's input JSON becomes the
/// call's `arguments`, verbatim and unnarrowed, and the tool the argv named is
/// the tool called.
#[test]
fn the_input_on_stdin_becomes_the_calls_arguments_verbatim() {
    let input = json!({"url": "https://example.com/a", "max_length": 500});
    let (capture, sent) = called(&[text("a page")], &input.to_string());
    assert_eq!(sent["method"], "tools/call");
    assert_eq!(sent["params"]["name"], "fetch");
    assert_eq!(sent["params"]["arguments"], input);
    assert_eq!(capture.stdout, "a page\n");
    assert_eq!(capture.exit_code, 0);
}

/// The handshake states the protocol revision and who is asking, and the call
/// carries the id this end is waiting on.
#[test]
fn the_call_is_the_second_request_and_the_first_was_the_handshake() {
    let (_, sent) = called(&[text("said")], "{}");
    assert_eq!(sent["id"], 2, "the handshake spent the first id");
    assert_eq!(sent["jsonrpc"], "2.0");
}

/// **A message that is not the answer is skipped.** A server may log, and a
/// log line is not an error worth ending an invocation over.
#[test]
fn a_notification_before_the_answer_is_not_the_answer() {
    let logged = json!({"jsonrpc": "2.0", "method": "notifications/message",
            "params": {"level": "info"}})
    .to_string();
    let (capture, _) = called(&[logged, text("after the log")], "{}");
    assert_eq!(capture.stdout, "after the log\n");
    assert_eq!(capture.exit_code, 0);
}

/// **The server's lifetime is contained in the invocation's** (DESIGN §6.4).
/// A server that ignores its stdin closing is asked, waited for, and stopped —
/// so the call returns rather than becoming this box's resident state.
#[test]
fn a_server_that_ignores_end_of_file_is_stopped_anyway() {
    let scratch = Scratch::new();
    let mut argv = conversing(&[text("answered, then stayed")], &scratch.join("call.json"));
    if let Some(script) = argv.last_mut() {
        script.push_str("while :; do sleep 1; done");
    }
    let started = std::time::Instant::now();
    let capture = bridge("fetch", &argv, "{}");
    assert_eq!(capture.stdout, "answered, then stayed\n");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(10),
        "the bridge waited on a server that was never going to leave"
    );
}

/// **And what the server started does not outlive it either.** The teardown
/// signals the group, unconditionally, so a helper the server backgrounded is
/// gone when the invocation is — which is the whole of why the spawn boundary
/// gives every child a group of its own.
#[test]
fn a_helper_the_server_backgrounded_does_not_outlive_the_invocation() {
    let scratch = Scratch::new();
    let marker = scratch.join("helper-ran");
    let mut argv = conversing(&[text("done")], &scratch.join("call.json"));
    if let Some(script) = argv.last_mut() {
        script.insert_str(0, &format!("(sleep 0.6; : > '{}') & ", marker.display()));
    }
    let capture = bridge("fetch", &argv, "{}");
    assert_eq!(capture.stdout, "done\n");
    std::thread::sleep(std::time::Duration::from_millis(1200));
    assert!(
        !marker.exists(),
        "the helper outlived the invocation that started it"
    );
}
