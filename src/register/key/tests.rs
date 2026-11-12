//! The word's pinned vector, and a made key read back by the libraries that
//! will read it for real.

use super::{PUBLIC, base64, make, pem, word};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, SubjectPublicKeyInfoDer};

/// yog's `src/registry/offer/tests.rs` vector, verbatim (REMOTE §15.1): every
/// implementation reproduces it.
const VECTOR_KEY: &str = "-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAENflj+nLA7lsFuxVfqaryK84n9wP4
3JqG0mf0zqZecVFyPJ6ZX6m9IVK9g0IKk4hdMBbVTWXZvaIRcR4UDpgzCg==
-----END PUBLIC KEY-----
";
const VECTOR_CA: &str = "-----BEGIN CERTIFICATE-----
MIIBdzCCAR2gAwIBAgIUS6jy88xKuMgm0widLw6ejqRjV2EwCgYIKoZIzj0EAwIw
ETEPMA0GA1UEAwwGeW9nLWNhMB4XDTI2MTExMTE5MTgyOVoXDTI5MDIxMzE5MTgy
OVowETEPMA0GA1UEAwwGeW9nLWNhMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE
a2lFtf+Bdi55ENgocMU/VFmRRnrTvKFmGo1Jc/jkpoLCcUiuY+oHUyxbLu9bjGPN
KTzzSgWlmGrNv8lTlWTamKNTMFEwHQYDVR0OBBYEFDuf4kFm3Ug1PoxXdM4G1sUD
nSeYMB8GA1UdIwQYMBaAFDuf4kFm3Ug1PoxXdM4G1sUDnSeYMA8GA1UdEwEB/wQF
MAMBAf8wCgYIKoZIzj0EAwIDSAAwRQIhAIYwai5c3FwmIffCooBMLoSYb8A+40Id
n1/Fr3EXpRUyAiB/98dHbS9USqSz9WaFPMWG6lVZgiWI3ZGtfzdB30JEmA==
-----END CERTIFICATE-----
";

/// **The pinned vector**: the digest begins `c2 8b 19 3c` and the word is
/// `2LFTUR`.
#[test]
fn the_confirmation_word_reproduces_the_server_s_vector() {
    let spki = SubjectPublicKeyInfoDer::from_pem_slice(VECTOR_KEY.as_bytes()).expect("the key");
    let ca = CertificateDer::from_pem_slice(VECTOR_CA.as_bytes()).expect("the CA");
    assert_eq!(spki.len(), 91, "a P-256 SubjectPublicKeyInfo");
    assert_eq!(word(&spki, &ca), "2LFTUR");
    assert_ne!(word(&ca, &spki), "2LFTUR", "the order is part of the word");
}

/// The vector's own PEM is what this file's encoder writes for those bytes —
/// so what is posted is the text the server's reader expects.
#[test]
fn pem_is_written_the_way_the_vector_is() {
    let spki = SubjectPublicKeyInfoDer::from_pem_slice(VECTOR_KEY.as_bytes()).expect("the key");
    assert_eq!(pem(PUBLIC, &spki), VECTOR_KEY);
    for (plain, encoded) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v")] {
        assert_eq!(base64(plain.as_bytes()), encoded);
    }
}

/// **A made key is a P-256 pair the TLS stack accepts**: the private half
/// reads back as PKCS#8 and loads as a signing key, and the public half is
/// the 91-byte `SubjectPublicKeyInfo` of that same key. Two makes are two
/// keys.
#[test]
fn a_made_key_is_one_the_wire_can_present() {
    let made = make().expect("made");
    assert_eq!(made.spki.len(), 91);
    let private = PrivateKeyDer::from_pem_slice(made.private.as_bytes()).expect("PKCS#8 PEM");
    assert!(matches!(private, PrivateKeyDer::Pkcs8(_)));
    let signer = rustls::crypto::ring::sign::any_ecdsa_type(&private).expect("a P-256 key");
    let stated = signer.public_key().expect("its public half");
    assert_eq!(stated.as_ref(), made.spki.as_slice());
    assert_ne!(make().expect("made").spki, made.spki);
}
