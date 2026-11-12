//! **`thrall register <url>`**: this box claims an operator's offer and
//! becomes a foot of their server (yog `docs/REMOTE.md` §15.1; DESIGN §3.3).
//!
//! In the order it happens:
//!
//! 1. the URL is read ([`url`]) — it carries no key and no certificate;
//! 2. a P-256 key pair is made here ([`key`]), and the private half never
//!    leaves this box;
//! 3. the public half is posted to the registration door over HTTPS verified
//!    against the roots the caller hands in ([`post`]);
//! 4. the answer is read as a claim and filed as an entry ([`claim`]) —
//!    unless it was minted at any grade but `foot`, or this box already holds
//!    an entry under that workspace's name;
//! 5. the confirmation word is said, so the operator can find this device's
//!    row on a seat;
//! 6. the wire is dialled once with the new entry. The server holds an
//!    unapproved client and refuses it everything — and that refusal is the
//!    proof the offer was waiting for: the holder of the private key has
//!    spoken, which a copy of the URL never could.
//!
//! Approval is the operator's act on a seat and nothing here waits for it.
//! `thrall run` serves the entry once it has happened.
//!
//! **A claim is single-use, so the order of refusals matters.** Everything
//! that can be judged before the post is: the URL's shape, and the key. What
//! cannot be — the grade and the workspace are the *reply's* — is judged
//! before anything is written, and says that the offer is now spent.

use std::path::Path;

use rustls::RootCertStore;
use serde_json::json;

use crate::channel::material::{ANCHORS, CHAIN, KEY, Material};
use crate::channel::{Channel, Failure, entries};
use crate::cli::Verdict;
use crate::run::Notice;

/// What the door answered, and the entry it becomes.
mod claim;
/// The key this box makes, and the confirmation word.
mod key;
/// The one HTTPS request.
mod post;
/// The registration URL.
mod url;

/// The only grade a foot opens on (REMOTE §4.2).
const FOOT: &str = "foot";

/// What a registration that reached the server ends by saying.
const WAITING: &str = "waiting for approval — the server has seen this foot. Once the row is \
     approved, `thrall run` serves it.";

/// **The public web's roots** — what the registration door's certificate is
/// verified against in production. The one thing `webpki-roots` is linked
/// for; the wire's own trust is the operator CA and never these.
pub fn web_roots() -> RootCertStore {
    RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    }
}

/// Claim the offer at `url` into the data root `root`, verifying the door
/// against `roots`, and say the confirmation word through `notice` before the
/// wire is dialled.
pub fn register(root: &Path, url: &str, roots: RootCertStore, notice: &Notice) -> Verdict {
    match claimed(root, url, roots, notice) {
        Ok(said) => Verdict::ok(said),
        Err(reason) => Verdict::failed(reason),
    }
}

/// The act, as far as it gets.
fn claimed(
    root: &Path,
    url: &str,
    roots: RootCertStore,
    notice: &Notice,
) -> Result<String, String> {
    let door = url::parse(url)?;
    let made = key::make()?;
    let body = json!({ "public_key": key::pem(key::PUBLIC, &made.spki) }).to_string();
    let Some(answer) = post::post(&door, &body, roots)? else {
        return Err(format!(
            "this offer cannot be claimed: {} did not accept it. An offer is \
             claimed once, and lasts ten minutes unclaimed — mint a new one \
             from a connected seat.",
            door.host
        ));
    };
    let claim = claim::read(&answer)?;
    if claim.grade != FOOT {
        return Err(format!(
            "this offer was minted at the wrong grade: {:?}, and a foot only \
             opens on a foot-grade certificate. Nothing was filed, and the \
             offer is spent: decline it from a connected seat and mint one \
             at grade foot.",
            claim.grade
        ));
    }
    let dir = entries::dir(root).join(&claim.workspace);
    claim::file(&dir, &claim, &made.private)?;
    let word = key::word(&made.spki, &claim.anchor);
    notice(&format!(
        "registered as {} in workspace {}; the entry is {}",
        claim.name,
        claim.workspace,
        dir.display()
    ));
    notice(&format!(
        "Confirmation word: {word}. On a connected seat, approve the row for \
         {} that shows this word.",
        claim.name
    ));
    proved(&dir, claim.address, &word)
}

/// **Dial the wire once with the new entry**, so the server sees the holder
/// of the key. Any answer at all is the proof — while the offer is unapproved
/// the answer is a refusal, and that is expected.
///
/// The request is the read a foot spends its life in (`invocations`): a foot
/// says three things and this is the one that changes nothing.
fn proved(dir: &Path, address: Vec<String>, word: &str) -> Result<String, String> {
    let (anchors, chain, key) = (dir.join(ANCHORS), dir.join(CHAIN), dir.join(KEY));
    let entry = Material {
        anchors,
        chain,
        key,
        address,
    };
    let mut channel = Channel::open(&entry)
        .map_err(|why| format!("the entry was filed and will not open: {why}"))?;
    match channel.ask(&crate::gestures::invocations()) {
        Ok(_) => Ok(WAITING.to_owned()),
        Err(Failure::Wire(said) | Failure::Skew(said)) => Err(format!(
            "the entry is filed, but the server has not seen this foot yet: \
             {said}. `thrall run` proves the offer with the first dial that \
             lands; the word to approve is still {word}."
        )),
    }
}

#[cfg(test)]
mod tests;
