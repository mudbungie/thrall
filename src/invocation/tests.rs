//! What crosses the routing leg, read strictly in both directions.

use super::{Capture, Invocation, capture_of, capture_value, invocation_of, invocation_value};
use serde_json::json;

fn call() -> Invocation {
    Invocation {
        id: "i-1".to_owned(),
        tool: "Bash".to_owned(),
        input: json!({"command": "echo hi"}),
    }
}

fn ran() -> Capture {
    Capture {
        stdout: "hi\n".to_owned(),
        stderr: String::new(),
        exit_code: 0,
    }
}

/// The row is three keys, and the model's own input is carried verbatim.
#[test]
fn an_invocation_round_trips_and_carries_its_input_untouched() {
    let said = invocation_value(&call());
    assert_eq!(said["input"], call().input);
    assert_eq!(invocation_of(&said), Ok(call()));
}

/// A row missing anything refuses with the key — an invocation is an
/// instruction about what to run on this machine, and a guessed one would run
/// the wrong thing.
#[test]
fn an_invocation_missing_a_field_refuses_by_key() {
    assert_eq!(
        invocation_of(&json!(["i-1"])),
        Err("invocation: not a JSON object".to_owned())
    );
    assert_eq!(
        invocation_of(&json!({"tool": "Bash", "input": {}})),
        Err("invocation: missing field \"invocation\"".to_owned())
    );
    assert_eq!(
        invocation_of(&json!({"invocation": "i-1", "input": {}})),
        Err("invocation: missing field \"tool\"".to_owned())
    );
    assert_eq!(
        invocation_of(&json!({"invocation": "i-1", "tool": "Bash"})),
        Err("invocation: missing field \"input\"".to_owned())
    );
}

/// The capture is the three facts a local tool contract already answers with.
#[test]
fn a_capture_round_trips() {
    let said = capture_value(&ran());
    assert_eq!(
        said,
        json!({"stdout": "hi\n", "stderr": "", "exit_code": 0})
    );
    assert_eq!(capture_of(&said), Ok(ran()));
}

/// A capture that is missing a fact, or is not one at all, refuses.
#[test]
fn a_capture_missing_a_field_refuses_by_key() {
    assert_eq!(
        capture_of(&json!("hi")),
        Err("capture: not a JSON object".to_owned())
    );
    assert_eq!(
        capture_of(&json!({"stderr": "", "exit_code": 0})),
        Err("capture: missing field \"stdout\"".to_owned())
    );
    assert_eq!(
        capture_of(&json!({"stdout": "", "exit_code": 0})),
        Err("capture: missing field \"stderr\"".to_owned())
    );
    assert_eq!(
        capture_of(&json!({"stdout": "", "stderr": ""})),
        Err("capture: missing or non-integer field \"exit_code\"".to_owned())
    );
    assert_eq!(
        capture_of(&json!({"stdout": "", "stderr": "", "exit_code": "0"})),
        Err("capture: missing or non-integer field \"exit_code\"".to_owned())
    );
}

/// An exit code narrows to what a process can actually have exited with; a
/// number outside that is refused rather than truncated into a different
/// verdict.
#[test]
fn an_exit_code_outside_a_process_s_range_refuses() {
    let wide = json!({"stdout": "", "stderr": "", "exit_code": 5_000_000_000_i64});
    let refusal = capture_of(&wide).expect_err("refused");
    assert!(refusal.contains("out of range"), "{refusal}");
    // A signal-shaped negative code is inside the range and is carried.
    let negative = json!({"stdout": "", "stderr": "", "exit_code": -1});
    assert_eq!(capture_of(&negative).map(|c| c.exit_code), Ok(-1));
}

/// The default capture is what a tool that said nothing and succeeded looks
/// like, so nothing downstream has to build one by hand.
#[test]
fn the_default_capture_is_silence_and_success() {
    assert_eq!(
        capture_value(&Capture::default()),
        json!({"stdout": "", "stderr": "", "exit_code": 0})
    );
}
