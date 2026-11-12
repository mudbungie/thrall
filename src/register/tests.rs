//! **The whole registration, against a door and a wire that really speak**:
//! a TLS listener standing in for the public edge, the operator CA's own act
//! performed by the mint, and the stand-in server at the address the claim
//! states.

use super::{key, register, web_roots};
use crate::channel::material::{self, ADDRESS, ANCHORS, KEY};
use crate::channel::{Channel, entries, tls};
use crate::cli::Verdict;
use crate::test_support::door::{self, Door};
use crate::test_support::server::Server;
use crate::test_support::{Notices, Scratch, mint};
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, SubjectPublicKeyInfoDer};
use serde_json::{Value, json};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// What files nothing: every refusal ahead of the entry.
mod refused;

/// The name the offer was minted under.
const NAME: &str = "shed";

/// What one registration did.
struct Run {
    scratch: Scratch,
    door: Door,
    verdict: Verdict,
    said: Vec<String>,
}

impl Run {
    /// Where the entry would be filed.
    fn entry(&self) -> PathBuf {
        entries::dir(&self.scratch.join("root")).join("home")
    }
}

/// The operator CA's directory under a scratch box.
fn ca(scratch: &Scratch) -> PathBuf {
    let dir = scratch.join("ca");
    mint::material(&dir);
    dir
}

/// The door's answer to a claim: a leaf signed for the posted key under
/// `subject`, in a reply stating `grade` and `address`.
fn signing(dir: &Path, subject: &'static str, grade: &'static str, address: Value) -> door::Answer {
    let dir = dir.to_path_buf();
    Box::new(move |body| {
        let posted: Value = serde_json::from_str(body).expect("a JSON body");
        let public = posted["public_key"].as_str().expect("the public key");
        let cert = mint::claimed(&dir, public, NAME, subject);
        let ca = std::fs::read_to_string(dir.join(ANCHORS)).expect("the CA");
        let reply = json!({"workspace": "home", "name": NAME, "grade": grade,
                           "address": address, "ca": ca, "cert": cert});
        door::ok(&reply.to_string())
    })
}

/// Register against a door answering `answer`, verified against the CA in
/// `dir`.
fn run(scratch: Scratch, dir: &Path, answer: door::Answer) -> Run {
    let door = Door::start(dir, answer);
    let roots = tls::anchors(&dir.join(ANCHORS)).expect("the root");
    let (notices, sink) = Notices::new();
    let verdict = register(&scratch.join("root"), &door.url, roots, &sink);
    let said = notices.heard();
    Run {
        scratch,
        door,
        verdict,
        said,
    }
}

/// A foot-grade claim whose reply lists a dead endpoint and then `wire`.
fn claimed(wire: Option<Vec<Value>>) -> (Run, Option<Server>) {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let mut address = vec!["127.0.0.1:1".to_owned()];
    let server = wire.map(|answer| Server::start(&dir, crate::corpus::PROTOCOL, vec![answer]));
    if server.is_some() {
        address.push(std::fs::read_to_string(dir.join(ADDRESS)).expect("the bound address"));
    }
    let answer = signing(&dir, "/OU=foot/CN=shed", "foot", json!(address));
    (run(scratch, &dir, answer), server)
}

/// **The whole act.** A key is made here and only its public half is posted;
/// the entry is filed with the key its owner's alone and the address as the
/// list; the word said is the one the server would compute; and the wire is
/// dialled once — past a dead first endpoint — where the held refusal is the
/// expected answer and the exit is success.
#[test]
fn a_claimed_offer_is_filed_shows_its_word_and_proves_itself_on_the_wire() {
    let refusal = json!({"ok": false, "error": "held: waiting for an operator's approval"});
    let (run, server) = claimed(Some(vec![refusal]));
    assert_eq!(run.verdict.code, 0, "{}", run.verdict.text);
    assert_eq!(
        run.verdict.text,
        "waiting for approval — the server has seen this foot. Once the row is \
         approved, `thrall run` serves it."
    );

    let request = run.door.heard();
    let (head, body) = request.split_once("\r\n\r\n").expect("a request");
    assert!(head.starts_with(&format!("POST /yog/register/{} HTTP/1.1\r\n", door::CODE)));
    assert!(!request.contains("PRIVATE"), "the private key never leaves");
    let posted: Value = serde_json::from_str(body).expect("JSON");
    let public = posted["public_key"].as_str().expect("the public key");
    let spki = SubjectPublicKeyInfoDer::from_pem_slice(public.as_bytes()).expect("an SPKI");

    let entry = run.entry();
    let anchor = CertificateDer::from_pem_file(entry.join(ANCHORS)).expect("the CA");
    let word = key::word(&spki, &anchor);
    assert_eq!(
        run.said,
        [
            format!(
                "registered as shed in workspace home; the entry is {}",
                entry.display()
            ),
            format!(
                "Confirmation word: {word}. On a connected seat, approve the row \
                 for shed that shows this word."
            ),
        ]
    );
    let mode = std::fs::metadata(entry.join(KEY))
        .expect("the key")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    let filed = material::read_dir(&entry)
        .expect("readable")
        .expect("an entry");
    assert_eq!(filed.address.len(), 2, "the list, as the reply stated it");
    assert_eq!(
        Channel::open(&filed).expect("a foot's entry").client(),
        NAME
    );
    let said = server.expect("the wire").heard();
    assert_eq!(said.len(), 2, "one dial: a preface and one request");
    assert_eq!(said.last(), Some(&crate::gestures::invocations()));
}

/// **A wire that cannot be reached leaves a filed entry and says so**: the
/// offer is not proved yet, the exit is a failure, and the word is repeated
/// because the operator will need it when `thrall run` gets through.
#[test]
fn a_wire_that_cannot_be_reached_leaves_the_entry_and_repeats_the_word() {
    let (run, _) = claimed(None);
    assert_eq!(run.verdict.code, 1);
    let text = &run.verdict.text;
    assert!(
        text.contains("the server has not seen this foot yet"),
        "{text}"
    );
    assert!(text.contains("connect 127.0.0.1:1"), "{text}");
    let line = run.said[1]
        .strip_prefix("Confirmation word: ")
        .expect("the word line");
    let word = line.split_once('.').expect("a sentence").0;
    assert_eq!(word.len(), 6);
    assert!(text.contains(&format!("still {word}.")), "{text}");
    assert!(run.entry().join(KEY).is_file(), "the entry stands");
}

/// A reply that says `foot` over a leaf that does not is an entry that will
/// not open — said as that, not as a wire failure.
#[test]
fn a_leaf_that_is_not_a_foot_s_is_filed_and_will_not_open() {
    let scratch = Scratch::new();
    let dir = ca(&scratch);
    let answer = signing(&dir, "/CN=shed", "foot", json!(["127.0.0.1:1"]));
    let run = run(scratch, &dir, answer);
    assert_eq!(run.verdict.code, 1);
    assert!(
        run.verdict.text.contains("will not open"),
        "{}",
        run.verdict.text
    );
    assert!(
        run.verdict.text.contains("not foot grade"),
        "{}",
        run.verdict.text
    );
}

/// Production verifies the door against the public web's roots, and there
/// are some.
#[test]
fn the_web_s_roots_are_a_store_with_roots_in_it() {
    assert!(web_roots().roots.len() > 50);
}
