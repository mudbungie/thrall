//! Strict reads: every refusal names the key the operator typed.

use super::{bool_of, opt_str_of, str_of, strings_of};
use serde_json::{Map, Value, json};

/// The object a test reads fields out of.
fn object(v: Value) -> Map<String, Value> {
    v.as_object().expect("an object").clone()
}

#[test]
fn a_string_field_reads_back_and_a_missing_one_names_itself() {
    let o = object(json!({"name": "Bash"}));
    assert_eq!(str_of(&o, "name"), Ok("Bash".to_owned()));
    assert_eq!(
        str_of(&o, "description"),
        Err("missing field \"description\"".to_owned())
    );
}

#[test]
fn a_mistyped_string_field_refuses_rather_than_stringifying_it() {
    let o = object(json!({"name": 7}));
    assert_eq!(
        str_of(&o, "name"),
        Err("field \"name\" is not a string".to_owned())
    );
}

/// **A required boolean has three answers and only one of them is a value.**
/// Absent refuses rather than reading false, because the fields wearing this
/// shape are readings off the far end (`wrote`), and a reading defaulted to
/// false is the reassuring answer invented for an engine that said nothing.
#[test]
fn a_required_boolean_refuses_absence_rather_than_reading_false() {
    let o = object(json!({"wrote": true}));
    assert_eq!(bool_of(&o, "wrote"), Ok(true));
    assert_eq!(
        bool_of(&object(json!({"wrote": false})), "wrote"),
        Ok(false)
    );
    assert_eq!(
        bool_of(&object(json!({})), "wrote"),
        Err("missing field \"wrote\"".to_owned())
    );
    assert_eq!(
        bool_of(&object(json!({"wrote": "true"})), "wrote"),
        Err("field \"wrote\" is not a boolean".to_owned())
    );
}

/// Absent and null are one answer — the operator left it out — and a value of
/// the wrong type is still a refusal.
#[test]
fn an_optional_field_is_absent_null_or_wrong() {
    let held = object(json!({"cwd": "/srv/work"}));
    assert_eq!(opt_str_of(&held, "cwd"), Ok(Some("/srv/work".to_owned())));
    let nulled = object(json!({"cwd": null}));
    assert_eq!(opt_str_of(&nulled, "cwd"), Ok(None));
    assert_eq!(opt_str_of(&object(json!({})), "cwd"), Ok(None));
    let mistyped = object(json!({"cwd": ["/srv/work"]}));
    assert_eq!(
        opt_str_of(&mistyped, "cwd"),
        Err("field \"cwd\" is not a string".to_owned())
    );
}

/// An argv reads back whole. An element that is not a string refuses the whole
/// field: an argv with a number in it is not an argv with a gap.
#[test]
fn an_array_of_strings_refuses_whole() {
    let o = object(json!({"command": ["/bin/tool", "--flag"]}));
    assert_eq!(
        strings_of(&o, "command"),
        Ok(vec!["/bin/tool".to_owned(), "--flag".to_owned()])
    );
    assert_eq!(
        strings_of(&object(json!({})), "command"),
        Err("missing field \"command\"".to_owned())
    );
    assert_eq!(
        strings_of(&object(json!({"command": "/bin/tool"})), "command"),
        Err("field \"command\" is not an array".to_owned())
    );
    assert_eq!(
        strings_of(&object(json!({"command": ["/bin/tool", 7]})), "command"),
        Err("field \"command\" holds something that is not a string".to_owned())
    );
}
