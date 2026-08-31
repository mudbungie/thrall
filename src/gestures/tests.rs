//! The foot set, and only the foot set — plus every answer it can be given.

use super::{Reply, advertise, complete, decode, invocations};
use crate::invocation::{Capture, Invocation};
use crate::tools::Tool;
use serde_json::json;

fn bash() -> Tool {
    Tool {
        name: "Bash".to_owned(),
        description: "run a command in a shell".to_owned(),
        input_schema: json!({"type": "object"}),
        subject_cwd: false,
    }
}

fn ran() -> Capture {
    Capture {
        stdout: "hi\n".to_owned(),
        stderr: String::new(),
        exit_code: 0,
    }
}

/// The advertisement is one field carrying the three-fact elements, and it
/// **names no client**: the identity a set lands under is the connection's
/// certificate, and a field here would let any connection overwrite any other's.
#[test]
fn advertise_carries_the_set_and_names_no_client() {
    let said = advertise(&[bash()]);
    assert_eq!(
        said,
        json!({"op": "advertise",
               "tools": [{"name": "Bash",
                          "description": "run a command in a shell",
                          "input_schema": {"type": "object"}}]})
    );
    assert!(!said.to_string().contains("client"), "{said}");
}

/// A box that offers nothing still says so: an empty presented set is a
/// statement, and it is not the same thing as never having advertised.
#[test]
fn advertise_can_say_nothing_is_offered() {
    assert_eq!(advertise(&[]), json!({"op": "advertise", "tools": []}));
}

/// The read carries no field at all — a connection drains its own queue.
#[test]
fn invocations_asks_for_this_machine_s_work_and_nothing_else() {
    assert_eq!(invocations(), json!({"op": "invocations"}));
}

/// A completion quotes the handle it is answering and carries the capture in
/// its one spelling. It names no client either.
#[test]
fn complete_answers_one_handle_with_one_capture() {
    let said = complete("i-1", &ran());
    assert_eq!(
        said,
        json!({"op": "complete", "invocation": "i-1",
               "capture": {"stdout": "hi\n", "stderr": "", "exit_code": 0}})
    );
}

/// **Three gestures, and there is no spelling for a fourth.** The foot set is
/// enumerated rather than subtracted from, so the surface this file exposes is
/// the whole of what a thrall can say — `invoke`, the asking side's verb, has
/// no encoder here at all.
#[test]
fn the_only_ops_this_crate_can_spell_are_the_foot_set() {
    let spoken: Vec<String> = [advertise(&[]), invocations(), complete("i-1", &ran())]
        .iter()
        .map(|g| g["op"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(spoken, ["advertise", "invocations", "complete"]);
}

/// The receipt for an advertisement.
#[test]
fn an_advertised_receipt_reads_back() {
    assert_eq!(
        decode(&json!({"ok": true, "kind": "advertised"})),
        Ok(Ok(Reply::Advertised))
    );
}

/// The follow-class read's rows, and the ordinary empty answer of a hold that
/// ended with no work in it.
#[test]
fn the_read_answers_rows_and_an_empty_answer_is_ordinary() {
    let said = json!({"ok": true, "kind": "invocations",
                      "rows": [{"invocation": "i-1", "tool": "Bash", "input": {"a": 1}}]});
    assert_eq!(
        decode(&said),
        Ok(Ok(Reply::Invocations(vec![Invocation {
            id: "i-1".to_owned(),
            tool: "Bash".to_owned(),
            input: json!({"a": 1}),
            cwd: None,
        }])))
    );
    assert_eq!(
        decode(&json!({"ok": true, "kind": "invocations", "rows": []})),
        Ok(Ok(Reply::Invocations(Vec::new())))
    );
}

/// A read with no rows at all, and a row that is not one, both refuse.
#[test]
fn a_read_that_answers_no_rows_refuses() {
    let refusal = decode(&json!({"ok": true, "kind": "invocations"})).expect_err("refused");
    assert!(refusal.contains("no rows"), "{refusal}");
    let bad = decode(&json!({"ok": true, "kind": "invocations", "rows": [{"tool": "Bash"}]}))
        .expect_err("refused");
    assert!(bad.contains("missing field \"invocation\""), "{bad}");
}

/// **`capture` is absent rather than empty while the work is still out**, so a
/// reader never has to tell "not finished" from "finished saying nothing".
#[test]
fn a_routed_receipt_carries_the_capture_only_once_there_is_one() {
    assert_eq!(
        decode(&json!({"ok": true, "kind": "routed", "invocation": "i-1"})),
        Ok(Ok(Reply::Routed {
            invocation: "i-1".to_owned(),
            capture: None
        }))
    );
    assert_eq!(
        decode(&json!({"ok": true, "kind": "routed", "invocation": "i-1", "capture": null})),
        Ok(Ok(Reply::Routed {
            invocation: "i-1".to_owned(),
            capture: None
        }))
    );
    let answered = json!({"ok": true, "kind": "routed", "invocation": "i-1",
                          "capture": {"stdout": "hi\n", "stderr": "", "exit_code": 0}});
    assert_eq!(
        decode(&answered),
        Ok(Ok(Reply::Routed {
            invocation: "i-1".to_owned(),
            capture: Some(ran())
        }))
    );
    let malformed = json!({"ok": true, "kind": "routed", "invocation": "i-1", "capture": {}});
    assert!(decode(&malformed).is_err());
    let nameless = json!({"ok": true, "kind": "routed"});
    assert!(decode(&nameless).is_err());
}

/// **The refusal is the envelope with no `kind`.** `ok` cannot be the
/// discriminant, so a body carrying a kind is an answer and a body carrying
/// none must be a refusal — and it arrives as the inner `Err`, faithfully
/// carried rather than turned into a decode failure.
#[test]
fn a_refusal_is_carried_faithfully_and_is_not_a_decode_failure() {
    assert_eq!(
        decode(&json!({"ok": false, "error": "this foot is not registered here"})),
        Ok(Err("this foot is not registered here".to_owned()))
    );
}

/// A body with neither a kind nor a refusal's shape is bytes this decoder
/// cannot read, and says so.
#[test]
fn an_envelope_that_is_neither_an_answer_nor_a_refusal_refuses() {
    assert_eq!(
        decode(&json!(["rows"])),
        Err("reply: not a JSON object".to_owned())
    );
    assert_eq!(
        decode(&json!({"ok": true})),
        Err("reply: an answer with no kind".to_owned())
    );
    assert_eq!(
        decode(&json!({})),
        Err("reply: an answer with no kind".to_owned())
    );
    assert_eq!(
        decode(&json!({"ok": false})),
        Err("reply: missing field \"error\"".to_owned())
    );
    assert_eq!(
        decode(&json!({"kind": 7})),
        Err("reply: non-string field \"kind\"".to_owned())
    );
}

/// **An answer to a question nobody here asked is an error, not a shrug.** A
/// foot has three gestures and three answers; anything else means the two ends
/// disagree about what was said, and continuing on it would be guessing.
#[test]
fn a_kind_no_foot_gesture_can_earn_is_refused_by_name() {
    let refusal = decode(&json!({"ok": true, "kind": "board", "rows": []})).expect_err("refused");
    assert_eq!(refusal, "reply: unusable kind \"board\"");
}
