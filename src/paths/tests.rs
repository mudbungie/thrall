//! Which of two variables names the root, and what it means when neither does.

use super::{data_root, root_of};
use std::ffi::OsString;
use std::path::PathBuf;

fn stated(text: &str) -> OsString {
    OsString::from(text)
}

/// The XDG variable names the root outright, and wins over the convention's
/// default: a box that has already placed application data somewhere has
/// already answered this question.
#[test]
fn the_xdg_variable_names_the_root_outright() {
    assert_eq!(
        root_of(Some(stated("/var/lib/app")), Some(stated("/home/u"))),
        Ok(PathBuf::from("/var/lib/app/thrall"))
    );
}

/// Without it, the convention's default under the home directory.
#[test]
fn without_it_the_root_is_the_convention_under_the_home_directory() {
    assert_eq!(
        root_of(None, Some(stated("/home/u"))),
        Ok(PathBuf::from("/home/u/.local/share/thrall"))
    );
}

/// **An empty variable is an unset one.** A supervisor that exports a name with
/// no value has said nothing, and treating it as a root would name the
/// filesystem root's own directory.
#[test]
fn a_variable_with_no_value_has_said_nothing() {
    assert_eq!(
        root_of(Some(stated("")), Some(stated("/home/u"))),
        Ok(PathBuf::from("/home/u/.local/share/thrall"))
    );
    assert!(root_of(Some(stated("")), Some(stated(""))).is_err());
}

/// Neither set is a refusal naming both variables — never a relative guess,
/// which would put an operator's certificates wherever the supervisor happened
/// to start the process.
#[test]
fn neither_variable_set_refuses_and_names_them_both() {
    let refusal = root_of(None, None).expect_err("refused");
    assert!(refusal.contains("XDG_DATA_HOME"), "{refusal}");
    assert!(refusal.contains("HOME"), "{refusal}");
    assert!(refusal.contains("will not guess"), "{refusal}");
}

/// The process-edge read is the pure rule applied to this process's own
/// environment, and nothing else — it joins paths and touches no disk.
#[test]
fn the_edge_read_is_the_rule_applied_to_this_process() {
    assert_eq!(
        data_root(),
        root_of(std::env::var_os("XDG_DATA_HOME"), std::env::var_os("HOME"))
    );
}
