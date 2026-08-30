//! The grade, read off a real certificate — and the DER walk read off bytes no
//! mint would ever produce.

use super::{FOOT, common_name, foot, is_foot};
use crate::channel::material::CHAIN;
use crate::test_support::{Scratch, mint};

/// One DER type-length-value in the short length form. The suite builds
/// subjects a certificate authority would never sign, because that is where the
/// walk's refusals live; the long form is read off the real certificates below,
/// which are all longer than 127 bytes.
fn der(tag: u8, body: &[u8]) -> Vec<u8> {
    assert!(body.len() < 0x80, "the suite builds short values only");
    let mut out = vec![tag, u8::try_from(body.len()).expect("short form")];
    out.extend_from_slice(body);
    out
}

/// A `Name`: a sequence of one-attribute sets, each `SEQUENCE { type, value }`.
fn name(attributes: &[(Vec<u8>, u8, Vec<u8>)]) -> Vec<u8> {
    let mut body = Vec::new();
    for (oid, tag, value) in attributes {
        let attribute = der(0x30, &[der(0x06, oid), der(*tag, value)].concat());
        body.extend(der(0x31, &attribute));
    }
    der(0x30, &body)
}

/// A certificate whose only readable part is its subject — every other field is
/// a well-formed empty one, which is all the walk looks at. `leading` is
/// whatever precedes the serial number, so a test can put the optional version
/// field back.
fn certificate(subject: &[u8], leading: &[Vec<u8>]) -> Vec<u8> {
    let mut tbs: Vec<u8> = leading.concat();
    tbs.extend(der(0x02, &[1]));
    for _ in 0..3 {
        tbs.extend(der(0x30, &[]));
    }
    tbs.extend_from_slice(subject);
    der(0x30, &der(0x30, &tbs))
}

/// The common-name attribute type.
fn cn() -> Vec<u8> {
    vec![0x55, 0x04, 0x03]
}

/// The organizational-unit attribute type.
fn ou() -> Vec<u8> {
    vec![0x55, 0x04, 0x0b]
}

/// A real minted foot leaf answers its own common name — which is the identity
/// the engine will read off the very same bytes.
#[test]
fn a_minted_foot_leaf_answers_the_client_identity() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    assert_eq!(foot(&scratch.join(CHAIN)), Ok(mint::FOOT_NAME.to_owned()));
}

/// An operator-grade leaf is refused, and the refusal names the leaf, the word
/// the subject must carry, and where to get one. thrall fails closed where the
/// engine fails open: an operator leaf on a foot is a machine holding the whole
/// boundary in order to run commands.
#[test]
fn an_operator_grade_leaf_is_refused_and_says_how_to_fix_it() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let refusal =
        foot(&scratch.join(&format!("{}.pem", mint::OPERATOR_NAME))).expect_err("refused");
    assert!(refusal.contains(mint::OPERATOR_NAME), "{refusal}");
    assert!(refusal.contains("not foot grade"), "{refusal}");
    assert!(refusal.contains(&format!("OU={FOOT}")), "{refusal}");
    assert!(refusal.contains("box that holds the CA"), "{refusal}");
}

/// A leaf carrying the grade and no common name authenticates nobody, and is
/// refused for that rather than for its grade.
#[test]
fn a_leaf_that_names_no_client_is_refused_even_when_it_says_foot() {
    let scratch = Scratch::new();
    mint::material(scratch.path());
    let refusal = foot(&scratch.join(&format!("{}.pem", mint::NAMELESS))).expect_err("refused");
    assert!(refusal.contains("states no common name"), "{refusal}");
}

/// A chain that is not there, or holds no certificate, refuses naming the path
/// — never a channel opened on bytes nobody read.
#[test]
fn a_chain_that_holds_no_certificate_refuses() {
    let scratch = Scratch::new();
    let missing = scratch.join("nowhere.pem");
    assert!(foot(&missing).expect_err("refused").contains("nowhere.pem"));
    let empty = scratch.join("empty.pem");
    std::fs::write(&empty, b"").expect("write");
    assert!(foot(&empty).expect_err("refused").contains("empty.pem"));
}

/// The subject is read most-specific-last: two common names in one subject, and
/// the leaf's own is the final one.
#[test]
fn the_last_common_name_in_a_subject_wins() {
    let bytes = certificate(
        &name(&[
            (cn(), 0x0c, b"an intermediate".to_vec()),
            (cn(), 0x0c, b"the leaf".to_vec()),
        ]),
        &[],
    );
    assert_eq!(common_name(&bytes).as_deref(), Some("the leaf"));
}

/// The grade is one organizational unit anywhere in the subject, and any other
/// unit is simply not it.
#[test]
fn the_grade_is_the_organizational_unit_and_nothing_else() {
    let is = certificate(
        &name(&[
            (ou(), 0x0c, FOOT.as_bytes().to_vec()),
            (cn(), 0x0c, b"box".to_vec()),
        ]),
        &[],
    );
    assert!(is_foot(&is));
    assert_eq!(common_name(&is).as_deref(), Some("box"));
    let other = certificate(
        &name(&[
            (ou(), 0x0c, b"operations".to_vec()),
            (cn(), 0x0c, b"box".to_vec()),
        ]),
        &[],
    );
    assert!(!is_foot(&other));
}

/// The subject is located relative to the serial number, so the optional
/// version field before it moves nothing: a version-1 certificate and a
/// version-3 one take one path.
#[test]
fn an_optional_version_field_does_not_move_the_subject() {
    let versioned = certificate(&name(&[(cn(), 0x0c, b"box".to_vec())]), &[der(0xa0, &[])]);
    assert_eq!(common_name(&versioned).as_deref(), Some("box"));
}

/// A subject attribute this walk cannot read is skipped, never mis-read: a
/// value that is not UTF-8, an attribute with no value at all, and an attribute
/// whose type is not an object identifier.
#[test]
fn an_unreadable_attribute_is_skipped_and_not_guessed() {
    let utf16 = certificate(&name(&[(cn(), 0x1e, vec![0xff, 0xfe])]), &[]);
    assert_eq!(common_name(&utf16), None);

    let valueless = der(0x30, &der(0x31, &der(0x30, &der(0x06, &cn()))));
    assert_eq!(common_name(&certificate(&valueless, &[])), None);

    let mistyped = der(
        0x30,
        &der(
            0x31,
            &der(0x30, &[der(0x02, &cn()), der(0x0c, b"box")].concat()),
        ),
    );
    assert_eq!(common_name(&certificate(&mistyped, &[])), None);
}

/// Bytes that are not a certificate are not a foot, and name nobody. Every
/// malformed length DER forbids takes the same road: the indefinite form, a
/// width this walk will not serve, a length whose own bytes are missing, a
/// value shorter than its header promised, and a header that ends after its
/// tag.
#[test]
fn bytes_that_are_not_a_certificate_name_nobody() {
    let malformed: [&[u8]; 6] = [
        &[0x30, 0x80, 0x01],
        &[0x30, 0x85, 0x01, 0x02, 0x03, 0x04, 0x05],
        &[0x30, 0x82, 0x01],
        &[0x30, 0x05, 0x01],
        &[0x30],
        b"not DER at all",
    ];
    for bytes in malformed {
        assert_eq!(common_name(bytes), None, "{bytes:?}");
        assert!(!is_foot(bytes), "{bytes:?}");
    }
}

/// A certificate whose fields run out before the subject does not read one off
/// whatever happened to be there, and neither does one with no serial number to
/// count from.
#[test]
fn a_certificate_with_no_subject_to_reach_answers_nothing() {
    let short = der(
        0x30,
        &der(0x30, &[der(0x02, &[1]), der(0x30, &[])].concat()),
    );
    assert_eq!(common_name(&short), None);
    let serialless = der(0x30, &der(0x30, &der(0x30, &[])));
    assert_eq!(common_name(&serialless), None);
}
