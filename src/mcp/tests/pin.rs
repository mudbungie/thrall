//! **Discovery, once, printed to paste** (DESIGN §6.3): what `thrall mcp pin`
//! prints, what it says on the side, and what it refuses.
//!
//! The binding that matters most is the last test here: what pin prints is
//! read back by `config::read`, the very function `thrall run` spends on the
//! operator's own file. An entry that would be refused on a real box is
//! refused here first, so the paste cannot be a document this foot will not
//! read.

use super::{conversing, result};
use crate::config::read;
use crate::invocation::Capture;
use crate::mcp::pin::entries;
use crate::test_support::Scratch;
use serde_json::{Value, json};

/// Where a pinned entry says this binary lives. A literal, so the assertions
/// are about the shape rather than about whoever ran the suite.
const EXE: &str = "/usr/local/bin/thrall";

/// A catalog of two tools: one plain, one carrying the hints a server may
/// state about a tool's reach — and no description or schema at all, which is
/// the other thing a server may leave out.
fn catalog() -> Value {
    json!({"tools": [
        {"name": "fetch",
         "description": "Fetches a URL and extracts its contents as markdown.",
         "inputSchema": {"type": "object", "properties": {"url": {"type": "string"}},
                         "required": ["url"]}},
        {"name": "wipe",
         "annotations": {"readOnlyHint": false, "destructiveHint": true}},
    ]})
}

/// One pin against a server answering `body`, and the argv it was given.
fn pinned(body: Value) -> (Capture, Vec<String>) {
    let scratch = Scratch::new();
    let argv = conversing(&[result(body)], &scratch.join("list.json"));
    let capture = entries(&argv, std::path::Path::new(EXE));
    (capture, argv)
}

/// The printed array, as JSON.
fn printed(body: Value) -> Value {
    serde_json::from_str(&pinned(body).0.stdout).expect("pin printed JSON")
}

/// **Every tool of the catalog becomes a complete entry**: the three advertised
/// facts as the server stated them, and a `command` that runs this binary's
/// bridge against this very server.
#[test]
fn every_tool_becomes_an_entry_naming_this_binary_and_this_server() {
    let (capture, argv) = pinned(catalog());
    let document: Vec<Value> = serde_json::from_str(&capture.stdout).expect("an array");
    assert_eq!(document.len(), 2);
    assert_eq!(document[0]["name"], "fetch");
    assert_eq!(
        document[0]["description"],
        "Fetches a URL and extracts its contents as markdown."
    );
    assert_eq!(document[0]["input_schema"]["required"][0], "url");
    let mut expected = vec![json!(EXE), json!("mcp"), json!("fetch"), json!("--")];
    expected.extend(argv.iter().map(|word| json!(word)));
    assert_eq!(document[0]["command"], Value::Array(expected));
    assert_eq!(capture.exit_code, 0);
}

/// **A description or a schema the server left out is not a refusal.** Both
/// are visible in the paste, so a gap is the operator's to fill; an empty
/// object is what MCP's arguments are when a tool declares none.
#[test]
fn a_tool_the_server_described_thinly_still_prints_a_readable_entry() {
    let document = printed(catalog());
    assert_eq!(document[1]["name"], "wipe");
    assert_eq!(document[1]["description"], "");
    assert_eq!(document[1]["input_schema"], json!({"type": "object"}));
}

/// **Annotations are commentary and never advertised.** They name the tool on
/// stderr, for the operator's policy row on the engine side, and no entry
/// carries one — REMOTE §5.1 admits nothing yog stores and cannot check.
#[test]
fn annotations_go_to_stderr_and_never_into_an_entry() {
    let (capture, _) = pinned(catalog());
    assert!(capture.stderr.contains("never advertised"), "{capture:?}");
    assert!(capture.stderr.contains("wipe"), "{capture:?}");
    assert!(capture.stderr.contains("destructiveHint"), "{capture:?}");
    assert!(
        !capture.stderr.contains("fetch"),
        "a tool the server said nothing about earned a line anyway: {capture:?}"
    );
    let document = printed(catalog());
    let keys: Vec<String> = document[1]
        .as_object()
        .map(|o| o.keys().cloned().collect())
        .unwrap_or_default();
    assert_eq!(
        keys,
        vec!["command", "description", "input_schema", "name"],
        "an entry carries a key the document has no meaning for"
    );
}

/// **It writes nothing**: the document is operator-authored, and the allowlist
/// is the paste.
#[test]
fn nothing_is_written_where_the_document_lives() {
    let scratch = Scratch::new();
    let argv = conversing(&[result(catalog())], &scratch.join("list.json"));
    entries(&argv, std::path::Path::new(EXE));
    assert!(
        !crate::config::path(scratch.path()).exists(),
        "pin wrote a document"
    );
}

/// **What pin prints is a document this foot reads**, by the reader itself:
/// the name check, the schema and the empty-argv refusal all bite here exactly
/// as they would on a real box.
#[test]
fn the_printed_array_is_a_document_config_read_accepts() {
    let scratch = Scratch::new();
    let file = scratch.join("tools.json");
    std::fs::write(&file, pinned(catalog()).0.stdout).expect("a document");
    let set = read(&file).expect("pin printed a document thrall reads");
    assert_eq!(set.len(), 2);
    assert_eq!(set[0].tool.name, "fetch");
    assert_eq!(set[0].command[0], EXE);
    assert!(
        !set[0].tool.subject_cwd,
        "a bridged tool consents to no working directory it was not given one for"
    );
}

/// A catalog of the wrong shape is refused, in one sentence naming what was
/// wrong with it.
#[test]
fn a_catalog_of_the_wrong_shape_is_refused() {
    for (body, said) in [
        (json!({}), "carries no \"tools\" array"),
        (json!({"tools": [7]}), "something that is not a tool"),
        (json!({"tools": [{}]}), "missing field \"name\""),
    ] {
        let (capture, _) = pinned(body.clone());
        assert_eq!(capture.exit_code, 1, "{body}");
        assert!(capture.stderr.contains(said), "{body}: {capture:?}");
        assert_eq!(capture.stdout, "", "{body}");
    }
}
