//! The loop, against an engine that really speaks the protocol: what goes out,
//! in what order, and what stops it.
//!
//! Split on the seam the code is: [`channel`] is one conversation with one
//! engine, [`ending`] is what that conversation's failure means for the
//! channel's lifetime, [`redial`] is that lifetime, and [`fan`] is every
//! channel a box holds, served at once. The fixtures they share live here.

use crate::channel::Channel;
use crate::channel::material::read_dir;
use crate::config::Local;
use crate::invocation::{Capture, Invocation};
use crate::run::hold::Ending;
use crate::test_support::engine::Engine;
use crate::test_support::{Scratch, aside, mint};
use crate::tools::Tool;
use serde_json::{Value, json};
use std::path::Path;

/// This box's set: one tool, so a hand-off can prove it was handed the set as
/// well as the invocation.
fn set() -> Vec<Local> {
    vec![Local {
        tool: Tool {
            name: "Bash".to_owned(),
            description: "run a command in a shell".to_owned(),
            input_schema: json!({"type": "object"}),
            subject_cwd: false,
        },
        command: vec!["/usr/local/libexec/thrall-tools/bash-tool".to_owned()],
        cwd: None,
    }]
}

/// The stand-in executor: it answers with what it was handed, so the engine's
/// record of the completion is the evidence that both arguments arrived.
fn echo(set: &[Local], invocation: &Invocation) -> Capture {
    Capture {
        stdout: invocation.tool.clone(),
        stderr: format!("{} tools", set.len()),
        exit_code: 0,
    }
}

/// An executor that dies. Every path through `hold` answers a sentence, so
/// this is the only way a channel's thread can end without one.
fn boom(_: &[Local], _: &Invocation) -> Capture {
    panic!("an executor that died");
}

/// The ordinary receipt: the engine compared and found the set in force
/// identical, so it wrote nothing (REMOTE §5.1's `wrote`, PROTOCOL 8).
fn advertised() -> Value {
    json!({"ok": true, "kind": "advertised", "wrote": false})
}

/// The receipt that says the engine CHANGED the document. On a re-assertion
/// that is this box learning it was disarmed while a tool ran.
fn restored() -> Value {
    json!({"ok": true, "kind": "advertised", "wrote": true})
}

fn work(rows: Vec<Value>) -> Value {
    json!({"ok": true, "kind": "invocations", "rows": rows})
}

fn row(id: &str, tool: &str) -> Value {
    json!({"invocation": id, "tool": tool, "input": {"command": "echo hi"}})
}

fn receipt(id: &str) -> Value {
    json!({"ok": true, "kind": "routed", "invocation": id,
           "capture": {"stdout": "", "stderr": "", "exit_code": 0}})
}

fn refusal(said: &str) -> Value {
    json!({"ok": false, "error": said})
}

/// Provision `dir` and stand an engine at the far end, answering the n-th dial
/// with the n-th entry of `script` — one frame each.
fn engine_at(dir: &Path, script: Vec<Value>) -> Engine {
    mint::material(dir);
    Engine::start(
        dir,
        crate::corpus::PROTOCOL,
        script.into_iter().map(|v| vec![v]).collect(),
    )
}

/// The same, where a `None` is the dial the engine **goes away on** instead of
/// answering — a wire that flaps under a conversation and comes back.
fn flapping_at(dir: &Path, script: Vec<Option<Value>>) -> Engine {
    mint::material(dir);
    Engine::flapping(
        dir,
        script.into_iter().map(|v| v.map(|v| vec![v])).collect(),
    )
}

/// The sentence an ending carries, whichever ending it is.
///
/// It is a test's helper and not a method on [`Ending`], because the crate
/// proper has no use for the sentence of an ending it has not already decided
/// what to do about — `redial` reads the sentence out of the variant it
/// matched.
fn said(ending: Ending) -> String {
    match ending {
        Ending::Again { said, .. } | Ending::Over(said) => said,
    }
}

/// A box with one channel, and the engine standing at it.
fn wired(script: Vec<Value>) -> (Scratch, Engine, Channel) {
    let scratch = Scratch::new();
    let engine = engine_at(scratch.path(), script);
    let held = read_dir(scratch.path())
        .expect("readable")
        .expect("provisioned");
    let channel = Channel::open(&held).expect("opened");
    (scratch, engine, channel)
}

/// Every `op` the engine was handed, in order.
fn ops(engine: &Engine) -> Vec<String> {
    engine
        .heard()
        .iter()
        .filter_map(|v| v.get("op")?.as_str().map(str::to_owned))
        .collect()
}

/// The n-th gesture frame the engine was handed.
fn gesture(engine: &Engine, n: usize) -> Value {
    engine
        .heard()
        .into_iter()
        .filter(|v| v.get("op").is_some())
        .nth(n)
        .expect("a gesture")
}

/// One conversation with one engine.
mod channel;
/// What a channel says while it is still serving.
mod disarm;
/// What a failure means for the channel's lifetime.
mod ending;
/// Every channel this box holds, at once.
mod fan;
/// One channel's lifetime: dropped, waited out, dialled again.
mod redial;
/// When a box says a channel has stopped.
mod report;
