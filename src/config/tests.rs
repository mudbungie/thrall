//! One document, two readings — and every way an operator's file can fail to
//! be one.

use super::{TOOLS, advertisement, path, position, read};
use crate::test_support::Scratch;
use crate::tools::Tool;
use serde_json::json;
use std::path::{Path, PathBuf};

/// The document an operator would write.
fn document() -> String {
    json!([{"name": "Bash",
            "description": "run a command in a shell",
            "input_schema": {"type": "object",
                             "properties": {"command": {"type": "string"}},
                             "required": ["command"]},
            "command": ["/usr/local/libexec/thrall-tools/bash-tool"],
            "cwd": "/srv/work"},
           {"name": "Read",
            "description": "read a file",
            "input_schema": {"type": "object"},
            "command": ["/usr/local/libexec/thrall-tools/read-tool", "--strict"]}])
    .to_string()
}

/// Write `text` as this box's document and read it back.
fn written(scratch: &Scratch, text: &str) -> Result<Vec<super::Local>, String> {
    let file = path(scratch.path());
    std::fs::write(&file, text).expect("write");
    read(&file)
}

/// The document is one file beside the channel material, never inside anything
/// generated.
#[test]
fn the_document_sits_beside_the_wire_and_not_inside_it() {
    let root = Path::new("/somewhere/thrall");
    assert_eq!(path(root), root.join(TOOLS));
}

/// The whole of the operator's file, read: the advertised three, and the local
/// two that are never presented to anyone.
#[test]
fn a_document_reads_as_the_advertised_half_and_the_local_half() {
    let scratch = Scratch::new();
    let set = written(&scratch, &document()).expect("readable");
    assert_eq!(set.len(), 2);
    assert_eq!(set[0].tool.name, "Bash");
    assert_eq!(
        set[0].command,
        ["/usr/local/libexec/thrall-tools/bash-tool"]
    );
    assert_eq!(set[0].cwd, Some(PathBuf::from("/srv/work")));
    // `cwd` is the one optional key, and leaving it out is an answer.
    assert_eq!(set[1].cwd, None);
    assert_eq!(set[1].command.len(), 2);
}

/// **One document, two readings.** The advertisement is the projection that
/// drops the local half — so what this box offers and what it can run cannot
/// drift, and `command` and `cwd` cannot reach the wire even by accident.
#[test]
fn the_advertisement_is_the_document_with_the_local_half_dropped() {
    let scratch = Scratch::new();
    let set = written(&scratch, &document()).expect("readable");
    let presented = advertisement(&set);
    assert_eq!(
        presented,
        vec![
            Tool {
                name: "Bash".to_owned(),
                description: "run a command in a shell".to_owned(),
                input_schema: json!({"type": "object",
                                     "properties": {"command": {"type": "string"}},
                                     "required": ["command"]}),
                subject_cwd: false,
            },
            Tool {
                name: "Read".to_owned(),
                description: "read a file".to_owned(),
                input_schema: json!({"type": "object"}),
                subject_cwd: false,
            },
        ]
    );
    let said = crate::tools::encode(&presented).to_string();
    assert!(
        !said.contains("command\":["),
        "the argv reached the wire: {said}"
    );
    assert!(
        !said.contains("/srv/work"),
        "the cwd reached the wire: {said}"
    );
}

/// **A tool absent from the document is a tool this box does not have.** The
/// lookup is by the advertised name and answers an index, so the caller
/// resolves it against the very list it passed in.
#[test]
fn a_name_the_document_does_not_carry_is_not_here() {
    let scratch = Scratch::new();
    let set = written(&scratch, &document()).expect("readable");
    assert_eq!(position(&set, "Bash"), Some(0));
    assert_eq!(position(&set, "Read"), Some(1));
    assert_eq!(position(&set, "Write"), None);
}

/// **An absent document is a refusal, not the empty set.** A foot with nothing
/// to offer has nothing to do, and starting one is an explicit act that
/// deserves an explicit answer.
#[test]
fn a_box_with_no_document_refuses_and_says_which_file() {
    let scratch = Scratch::new();
    let refusal = read(&path(scratch.path())).expect_err("refused");
    assert!(refusal.contains(TOOLS), "{refusal}");
    assert!(refusal.contains("no tool config"), "{refusal}");
}

/// A file that is not the document refuses naming the file: bytes that are not
/// JSON, JSON that is not an array, and an element that is not an object.
#[test]
fn a_file_that_is_not_the_document_refuses_naming_it() {
    let scratch = Scratch::new();
    for (text, said) in [
        ("not json at all", "expected"),
        (r#"{"Bash": []}"#, "not a JSON array"),
        (r#"["Bash"]"#, "not a JSON object"),
    ] {
        let refusal = written(&scratch, text).expect_err("refused");
        assert!(refusal.contains(TOOLS), "{refusal}");
        assert!(refusal.contains(said), "{refusal}");
    }
}

/// An entry missing either half refuses, and the refusal names the key — an
/// operator editing JSON by hand needs the key they typed.
#[test]
fn an_entry_missing_a_field_refuses_by_key() {
    let scratch = Scratch::new();
    for (row, said) in [
        (
            json!({"description": "d", "input_schema": {}, "command": ["/bin/t"]}),
            "missing field \"name\"",
        ),
        (
            json!({"name": "T", "description": "d", "input_schema": {}}),
            "missing field \"command\"",
        ),
        (
            json!({"name": "T", "description": "d", "input_schema": {}, "command": "/bin/t"}),
            "field \"command\" is not an array",
        ),
        (
            json!({"name": "T", "description": "d", "input_schema": {},
                   "command": ["/bin/t"], "cwd": 7}),
            "field \"cwd\" is not a string",
        ),
    ] {
        let refusal = written(&scratch, &json!([row]).to_string()).expect_err("refused");
        assert!(refusal.contains(said), "{refusal}");
    }
}

/// **An empty argv is not a command.** A tool that names no program to run is
/// an entry with nothing behind it, and it is refused at the read rather than
/// discovered at the first invocation.
#[test]
fn an_empty_argv_is_refused_at_the_read() {
    let scratch = Scratch::new();
    let row = json!([{"name": "T", "description": "d", "input_schema": {}, "command": []}]);
    let refusal = written(&scratch, &row.to_string()).expect_err("refused");
    assert!(refusal.contains("empty argv"), "{refusal}");
}

/// The set has to be addressable, and the document is where that is decided:
/// an unusable name and a name used twice both refuse the whole file.
#[test]
fn a_set_that_cannot_be_addressed_refuses_the_whole_document() {
    let scratch = Scratch::new();
    let walking = json!([{"name": "tools/T", "description": "d",
                          "input_schema": {}, "command": ["/bin/t"]}]);
    let refusal = written(&scratch, &walking.to_string()).expect_err("refused");
    assert!(refusal.contains("unusable tool name"), "{refusal}");

    let twice = json!([{"name": "T", "description": "d", "input_schema": {},
                        "command": ["/bin/t"]},
                       {"name": "T", "description": "other", "input_schema": {},
                        "command": ["/bin/other"]}]);
    let refusal = written(&scratch, &twice.to_string()).expect_err("refused");
    assert!(refusal.contains("duplicate tool name \"T\""), "{refusal}");
}

/// A document that offers nothing reads as the empty set. It is a statement —
/// this box enables no tool — rather than a mistake, and it is the shape a box
/// takes while an operator is switching everything off.
#[test]
fn a_document_that_offers_nothing_is_an_empty_set() {
    let scratch = Scratch::new();
    assert_eq!(written(&scratch, "[]"), Ok(Vec::new()));
    assert_eq!(advertisement(&[]), Vec::new());
}
