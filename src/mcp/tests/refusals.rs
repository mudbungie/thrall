//! **Every way the bridge refuses**, and the sentence each one earns
//! (split from `tests.rs` at the pre-split band). A refusal is a capture like
//! any other — three facts and a `thrall:` sentence — because the model at the
//! far end reads one as it reads a tool that failed, which is what it is.

use super::{answered, bridge, server};
use serde_json::json;

/// **A server that would not start is a capture**, and the sentence names the
/// program and nothing else of the argv: a credential lives in that argv, and
/// a spawn failure loves to quote the command line (DESIGN §6.7).
#[test]
fn a_server_that_would_not_start_names_the_program_and_no_more_of_the_argv() {
    let argv = vec![
        "/nonexistent/mcp-server".to_owned(),
        "--the-argv-tail".to_owned(),
    ];
    let capture = bridge("fetch", &argv, "{}");
    assert_eq!(capture.exit_code, 1);
    assert_eq!(capture.stdout, "");
    assert!(
        capture.stderr.contains("/nonexistent/mcp-server"),
        "{capture:?}"
    );
    assert!(capture.stderr.contains("would not start"), "{capture:?}");
    assert!(
        !capture.stderr.contains("the-argv-tail"),
        "the sentence quoted the argv past its first word: {capture:?}"
    );
}

/// An argv with no program in it is refused before anything is spawned.
#[test]
fn an_empty_server_argv_is_refused() {
    let capture = bridge("fetch", &[], "{}");
    assert_eq!(capture.exit_code, 1);
    assert!(capture.stderr.contains("argv is empty"), "{capture:?}");
}

/// A server that leaves without answering is the wire's own silence, named at
/// the stage it fell over.
#[test]
fn a_server_that_ends_before_the_handshake_says_which_stage() {
    let capture = bridge("fetch", &server("exit 0"), "{}");
    assert_eq!(capture.exit_code, 1);
    assert!(
        capture.stderr.contains("ended before answering initialize"),
        "{capture:?}"
    );
}

/// **The diagnosis names the tool the operator typed, not the argv's first
/// word** (bl-3c8d). A bridged server is nearly always reached through a
/// launcher — `uvx`, `npx`, `docker run` — so the head of the argv is the one
/// process in the story that is not the server, and naming it sent an operator
/// to read the launcher's documentation for a server's fault. Here the head is
/// `/bin/sh`, which no sentence may claim was the MCP server.
#[test]
fn a_transport_failure_names_the_tool_and_not_the_launcher() {
    let capture = bridge("fetch", &server("exit 0"), "{}");
    assert!(
        capture
            .stderr
            .contains(r#"the MCP server "fetch" ended before answering initialize"#),
        "{capture:?}"
    );
    assert!(
        !capture.stderr.contains("/bin/sh"),
        "the sentence named the launcher rather than the tool: {capture:?}"
    );
}

/// A JSON-RPC error answers in the server's own words.
#[test]
fn an_error_on_the_call_carries_the_servers_own_sentence() {
    let capture = answered(
        &json!({"jsonrpc": "2.0", "id": 2,
                "error": {"code": -32602, "message": "unknown tool: fetch"}})
        .to_string(),
    );
    assert_eq!(capture.exit_code, 1);
    assert!(
        capture
            .stderr
            .contains("refused tools/call: unknown tool: fetch"),
        "{capture:?}"
    );
}

/// A response that is neither a result nor an error is refused in this box's
/// words, because the server offered none.
#[test]
fn a_response_carrying_neither_result_nor_error_is_refused() {
    let capture = answered(&json!({"jsonrpc": "2.0", "id": 2}).to_string());
    assert_eq!(capture.exit_code, 1);
    assert!(
        capture
            .stderr
            .contains("it sent neither a result nor an error"),
        "{capture:?}"
    );
}

/// The input is the object the model produced, whole — so anything that is not
/// an object, and anything that is not JSON at all, is refused before a server
/// is spawned.
#[test]
fn input_that_is_not_a_json_object_is_refused_before_anything_is_spawned() {
    for (input, said) in [("", "not JSON"), ("[1, 2]", "not a JSON object")] {
        let capture = bridge("fetch", &server("exit 0"), input);
        assert_eq!(capture.exit_code, 1, "{input:?}");
        assert!(capture.stderr.contains(said), "{input:?}: {capture:?}");
    }
}
