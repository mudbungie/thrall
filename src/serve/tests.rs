//! What `thrall run` reads, in what order it refuses — and the whole foot,
//! once around, against an engine that really speaks the protocol.

use super::serve;
use crate::channel::entries;
use crate::config;
use crate::test_support::engine::Engine;
use crate::test_support::{Scratch, mint};
use serde_json::{Value, json};
use std::path::Path;

/// Write this box's tool document, offering one tool that echoes its input.
fn offering(root: &Path) {
    std::fs::create_dir_all(root).expect("the root");
    let document = json!([{"name": "Echo",
                           "description": "echo the invocation's input",
                           "input_schema": {"type": "object"},
                           "command": ["/bin/sh", "-c", "cat"]}]);
    std::fs::write(config::path(root), document.to_string()).expect("write");
}

/// File one channel under `root`, with an engine standing at it that answers
/// the n-th dial with the n-th entry of `script`.
fn channel(root: &Path, leaf: &str, script: Vec<Value>) -> Engine {
    let dir = entries::dir(root).join(leaf);
    mint::material(&dir);
    Engine::start(
        &dir,
        crate::channel::hello::PROTOCOL,
        script.into_iter().map(|v| vec![v]).collect(),
    )
}

fn advertised() -> Value {
    json!({"ok": true, "kind": "advertised"})
}

fn refusal(said: &str) -> Value {
    json!({"ok": false, "error": said})
}

/// **The config is read first**, because a box with nothing to offer has
/// nothing to do and no reason to dial anything.
#[test]
fn a_box_with_no_tool_document_refuses_before_it_dials() {
    let scratch = Scratch::new();
    let said = serve(scratch.path());
    assert_eq!(said.code, 1);
    assert!(said.text.contains("tools.json"), "{}", said.text);
    assert!(said.text.contains("no tool config"), "{}", said.text);
    assert!(!said.text.contains("usage:"), "a failure carries no usage");
}

/// **Then the channels.** A box that can offer something and has nowhere to
/// offer it is told the shape of what is missing — never a command, because
/// there is none: the material is minted where the CA is and carried here by
/// hand.
#[test]
fn a_box_with_no_channel_is_told_the_shape_of_one() {
    let scratch = Scratch::new();
    offering(scratch.path());
    let said = serve(scratch.path());
    assert_eq!(said.code, 1);
    assert!(said.text.contains("holds no channel"), "{}", said.text);
    assert!(said.text.contains("ca.pem"), "{}", said.text);
    assert!(said.text.contains("client.key"), "{}", said.text);
    assert!(said.text.contains("thrall mints nothing"), "{}", said.text);
    assert!(
        said.text
            .contains(&entries::dir(scratch.path()).display().to_string()),
        "{}",
        said.text
    );
}

/// Every channel's sentence comes back under its own name, and serving is never
/// a success: a foot is either serving or stopped.
#[test]
fn every_channel_that_stopped_answers_under_its_own_name() {
    let scratch = Scratch::new();
    offering(scratch.path());
    let _north = channel(scratch.path(), "north", vec![refusal("north stopped")]);
    let _south = channel(scratch.path(), "south", vec![refusal("south stopped")]);
    let said = serve(scratch.path());
    assert_eq!(said.code, 1);
    assert_eq!(
        said.text,
        "thrall: north: north stopped\nsouth: south stopped"
    );
}

/// **The whole foot, once around.** An engine hands this box an invocation for
/// a tool the operator's document enables; the tool runs here; and what comes
/// back over the wire is what the command actually printed. Every leg is real:
/// a real mTLS channel, a real version preface, a real child process.
#[test]
fn a_foot_advertises_is_invoked_and_answers_with_what_ran() {
    let scratch = Scratch::new();
    offering(scratch.path());
    let engine = channel(
        scratch.path(),
        "engine",
        vec![
            advertised(),
            json!({"ok": true, "kind": "invocations",
                   "rows": [{"invocation": "i-1", "tool": "Echo",
                             "input": {"command": "echo hi"}}]}),
            json!({"ok": true, "kind": "routed", "invocation": "i-1",
                   "capture": {"stdout": "", "stderr": "", "exit_code": 0}}),
            refusal("the engine is going down"),
        ],
    );
    let said = serve(scratch.path());
    assert_eq!(said.text, "thrall: engine: the engine is going down");

    let gestures: Vec<Value> = engine
        .heard()
        .into_iter()
        .filter(|v| v.get("op").is_some())
        .collect();
    let ops: Vec<&str> = gestures
        .iter()
        .filter_map(|v| v.get("op")?.as_str())
        .collect();
    assert_eq!(ops, ["advertise", "invocations", "complete", "invocations"]);

    // What it presented is the document's projection: three keys, no argv.
    assert_eq!(
        gestures[0]["tools"],
        json!([{"name": "Echo",
                "description": "echo the invocation's input",
                "input_schema": {"type": "object"}}])
    );
    // And what it answered is what the command printed: the input, echoed.
    assert_eq!(
        gestures[2],
        json!({"op": "complete", "invocation": "i-1",
               "capture": {"stdout": "{\"command\":\"echo hi\"}",
                           "stderr": "", "exit_code": 0}})
    );
}
