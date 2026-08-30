//! The mTLS configuration, and every way provisioned bytes can fail to be one.

use super::{anchors, client_config, identity};
use crate::channel::material::{ANCHORS, CHAIN, KEY};
use crate::test_support::{Scratch, mint};

/// PEM armour around `body`, so a test can hand the reader something that is
/// shaped like a certificate and is not one.
fn armoured(body: &str) -> String {
    format!("-----BEGIN CERTIFICATE-----\n{body}\n-----END CERTIFICATE-----\n")
}

/// Provisioned material builds a client configuration — the ordinary case, and
/// the one every test below is a refusal of.
#[test]
fn provisioned_material_builds_a_client_configuration() {
    let scratch = Scratch::new();
    let held = mint::provisioned(scratch.path(), "engine.example:9000");
    assert!(client_config(&held).is_ok());
}

/// An anchor file that is not there names itself in the refusal.
#[test]
fn anchors_that_are_not_there_refuse_by_name() {
    let scratch = Scratch::new();
    let refusal = anchors(&scratch.join("nowhere.pem")).expect_err("refused");
    assert!(refusal.contains("nowhere.pem"), "{refusal}");
}

/// A file holding no certificate is not an empty trust store — it is a
/// misconfiguration, and a trust store that silently trusted nothing would
/// refuse every engine with a handshake error instead of a sentence.
#[test]
fn an_anchor_file_with_no_certificate_in_it_refuses() {
    let scratch = Scratch::new();
    let empty = scratch.join("empty.pem");
    std::fs::write(&empty, b"# nothing here\n").expect("write");
    let refusal = anchors(&empty).expect_err("refused");
    assert!(refusal.contains("no certificate in it"), "{refusal}");
}

/// Armour around bytes that are not base64 fails the read, and armour around
/// base64 that is not a certificate fails the store — two different refusals of
/// the same operator mistake, and both name the file.
#[test]
fn anchor_bytes_that_are_not_a_certificate_refuse() {
    let scratch = Scratch::new();
    let unreadable = scratch.join("unreadable.pem");
    std::fs::write(&unreadable, armoured("not base64 !!!")).expect("write");
    assert!(
        anchors(&unreadable)
            .expect_err("refused")
            .contains("unreadable.pem")
    );

    let undecodable = scratch.join("undecodable.pem");
    // Valid base64, and nothing a certificate parser can do with it.
    std::fs::write(&undecodable, armoured("bm90IGEgY2VydGlmaWNhdGU=")).expect("write");
    assert!(
        anchors(&undecodable)
            .expect_err("refused")
            .contains("undecodable.pem")
    );
}

/// The same three failures on this end's own identity: a chain that is not
/// there, a chain holding no certificate, unreadable armour, and a key that is
/// not there.
#[test]
fn an_identity_that_cannot_be_read_refuses_by_file() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let chain = scratch.join(CHAIN);
    let key = scratch.join(KEY);

    let missing = scratch.join("nowhere.pem");
    assert!(
        identity(&missing, &key)
            .expect_err("refused")
            .contains("nowhere.pem")
    );

    let empty = scratch.join("empty.pem");
    std::fs::write(&empty, b"# nothing here\n").expect("write");
    assert!(
        identity(&empty, &key)
            .expect_err("refused")
            .contains("no certificate in it")
    );

    let unreadable = scratch.join("unreadable.pem");
    std::fs::write(&unreadable, armoured("not base64 !!!")).expect("write");
    assert!(
        identity(&unreadable, &key)
            .expect_err("refused")
            .contains("unreadable.pem")
    );

    let keyless = scratch.join("nowhere.key");
    assert!(
        identity(&chain, &keyless)
            .expect_err("refused")
            .contains("nowhere.key")
    );
}

/// A chain and a key that are not each other's refuse at the configuration
/// rather than at the handshake: a foot must learn it cannot authenticate
/// before it dials, not from an engine.
#[test]
fn a_chain_and_a_key_that_do_not_match_refuse_before_anything_is_dialled() {
    let scratch = Scratch::new();
    let mut held = mint::provisioned(scratch.path(), "engine.example:9000");
    held.key = scratch.join(&format!("{}.key", mint::OPERATOR_NAME));
    let refusal = client_config(&held).expect_err("refused");
    assert!(refusal.contains("client identity"), "{refusal}");
}

/// Anchors that will not read refuse the whole configuration, and say which
/// file — an engine nobody can verify is not an engine to try dialling.
#[test]
fn a_configuration_with_unreadable_anchors_refuses() {
    let scratch = Scratch::new();
    let mut held = mint::provisioned(scratch.path(), "engine.example:9000");
    held.anchors = scratch.join("nowhere.pem");
    let refusal = client_config(&held).expect_err("refused");
    assert!(refusal.contains("nowhere.pem"), "{refusal}");
    assert!(
        scratch.join(ANCHORS).is_file(),
        "the operator's real anchors are untouched"
    );
}
