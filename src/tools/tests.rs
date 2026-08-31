//! The advertised element: one spelling in both directions, and the two ways a
//! set can fail to be addressable.

use super::{Tool, encode, of_one, one, validate};
use serde_json::{Value, json};

/// A tool with a schema of its own, so the round trip is asserted over
/// something a decoder could plausibly mangle.
fn bash() -> Tool {
    Tool {
        name: "Bash".to_owned(),
        description: "run a command in a shell".to_owned(),
        input_schema: json!({"type": "object",
                             "properties": {"command": {"type": "string"}},
                             "required": ["command"]}),
        subject_cwd: false,
    }
}

/// The element is exactly three keys, and the schema is carried verbatim —
/// neither validated nor rewritten, because it is this box's statement to a
/// model.
#[test]
fn an_element_is_three_facts_and_the_schema_is_untouched() {
    let said = one(&bash());
    let o = said.as_object().expect("an object");
    assert_eq!(o.len(), 3, "nothing else rides along: {said}");
    assert_eq!(o["input_schema"], bash().input_schema);
    assert_eq!(of_one(&said), Ok(bash()));
}

/// A set is the array the advertisement carries, in order.
#[test]
fn a_set_encodes_as_its_array() {
    let other = Tool {
        name: "Read".to_owned(),
        description: "read a file".to_owned(),
        input_schema: json!({"type": "object"}),
        subject_cwd: false,
    };
    let said = encode(&[bash(), other.clone()]);
    let rows = said.as_array().expect("an array");
    assert_eq!(rows.len(), 2);
    assert_eq!(of_one(&rows[0]), Ok(bash()));
    assert_eq!(of_one(&rows[1]), Ok(other));
}

/// Reading an element back is strict: every field is required, and a refusal
/// names the key.
#[test]
fn an_element_that_is_missing_something_refuses_by_key() {
    assert_eq!(
        of_one(&json!("Bash")),
        Err("tool: not a JSON object".to_owned())
    );
    assert_eq!(
        of_one(&json!({"description": "d", "input_schema": {}})),
        Err("tool: missing field \"name\"".to_owned())
    );
    assert_eq!(
        of_one(&json!({"name": "Bash", "input_schema": {}})),
        Err("tool: missing field \"description\"".to_owned())
    );
    assert_eq!(
        of_one(&json!({"name": "Bash", "description": "d"})),
        Err("tool: missing field \"input_schema\"".to_owned())
    );
    assert_eq!(
        of_one(&json!({"name": 7, "description": "d", "input_schema": {}})),
        Err("tool: field \"name\" is not a string".to_owned())
    );
}

/// A schema that is not an object is still carried: yog neither validates nor
/// rewrites it, and thrall is not entitled to be stricter about a document it
/// hands onward unread.
#[test]
fn a_schema_is_carried_whatever_it_is() {
    let odd = json!({"name": "Bash", "description": "d", "input_schema": "not a schema"});
    assert_eq!(
        of_one(&odd).map(|t| t.input_schema),
        Ok(Value::String("not a schema".to_owned()))
    );
}

/// A name that could address a filesystem is not a name (REMOTE §5.1), and
/// every spelling of that is refused with the token in it.
#[test]
fn a_name_that_is_not_a_single_path_component_is_refused() {
    for unusable in ["", ".", "..", "tools/Bash", "tools\\Bash", "Ba\0sh"] {
        let named = Tool {
            name: unusable.to_owned(),
            ..bash()
        };
        let refusal = validate(&[named]).expect_err("refused");
        assert!(refusal.starts_with("unusable tool name"), "{refusal}");
    }
}

/// Two tools wearing one name inside this box's set cannot be addressed, so it
/// declines loudly, naming the token. A collision ACROSS boxes is legal and
/// ordinary, and is not this set's business.
#[test]
fn one_name_twice_in_one_set_declines_naming_it() {
    let refusal = validate(&[bash(), bash()]).expect_err("refused");
    assert_eq!(refusal, "duplicate tool name \"Bash\"");
}

/// The ordinary set passes, and so does an empty one: a box may say it offers
/// nothing, and that is a statement rather than a mistake.
#[test]
fn an_addressable_set_passes_and_so_does_an_empty_one() {
    assert_eq!(validate(&[bash()]), Ok(()));
    assert_eq!(validate(&[]), Ok(()));
}

/// **The optional fourth fact** (bl-36f7): consent rides only when true,
/// absence reads false, and a mistyped value refuses rather than silently
/// dropping an operator's statement.
#[test]
fn subject_cwd_rides_only_when_true_and_a_mistyped_one_refuses() {
    let mut consenting = bash();
    consenting.subject_cwd = true;
    let spelled = one(&consenting);
    assert_eq!(spelled.get("subject_cwd"), Some(&json!(true)));
    assert_eq!(of_one(&spelled), Ok(consenting.clone()));

    let plain = one(&bash());
    assert!(plain.get("subject_cwd").is_none(), "absence is the default");
    assert_eq!(of_one(&plain), Ok(bash()));

    let mut mistyped = spelled;
    mistyped["subject_cwd"] = json!("yes");
    assert_eq!(
        of_one(&mistyped),
        Err("tool: field \"subject_cwd\" is not a boolean".to_owned())
    );
}
