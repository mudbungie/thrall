//! **The stand-in server**: the far end of the wire, so the suite can test a
//! channel against something that speaks the protocol rather than against a
//! mock of thrall's own beliefs about it.
//!
//! It listens, which is the one thing thrall must never do (DESIGN §2) — and
//! that is precisely why it is here and not in the crate proper. What it is
//! standing in for is yog: a real listener, a real mTLS handshake requiring a
//! client certificate the operator CA issued, a real version preface, and
//! answers framed the way REMOTE §3 frames them.
//!
//! **It is scripted, one entry per connection** ([`Dial`]), because a foot
//! dials per ask and holds a connection only while it is waiting — or, for a
//! run it posts in pieces, for that run (REMOTE §5.7). So a test says what the
//! server does on the first dial, the second, and so on: answer the one
//! request, answer each request of a run in turn, take the foot's frames and
//! go AWAY with no answer and no TLS `close_notify` — the failure a roaming
//! box actually hits, which a script of answers cannot produce because every
//! answer ends in a terminator — or hang up before the handshake, which is a
//! server that could not be reached at all.
//!
//! **It states the FLOOR as its edition unless a test says otherwise** (REMOTE
//! §3.2). A server speaking this major is at the floor or above it by
//! definition, so the floor is the least a real one can say;
//! [`Server::streaming`] states the edition that reads a capture in pieces.
//!
//! **It states the server's protocol number, not this crate's** (bl-e0f0).
//! Every fixture here dials at [`crate::corpus::PROTOCOL`], a literal copied
//! from yog's repo-root `PROTOCOL` file. While it wrote its preface from
//! `channel::hello::PROTOCOL` the two ends of every test were one constant
//! wearing two names, so they agreed at any value — and the pin sat five
//! versions behind a live server, unable to open a single real channel, with
//! the whole suite green. A stand-in built out of the thing it stands in for
//! cannot fail the one way that matters.
//!
//! **It records every frame it is handed, in order** — the version preface and
//! then the request, per connection. That is how a test asserts what thrall
//! *said* rather than only what it did with the reply, and it is the only
//! witness that the preface goes out in the same breath as the request (REMOTE
//! §3: both ends write before either reads).

use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex, PoisonError};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};

use crate::channel::{frame, material, tls};
use crate::corpus::ledger::FLOOR;

/// **What the server does with one connection.**
#[derive(Clone)]
pub(crate) enum Dial {
    /// Answer each request on it in turn — one request's frames per entry,
    /// then the terminator — and hang up after the last. One entry is the
    /// ordinary ask; several are the posts of one run.
    Answers(Vec<Vec<Value>>),
    /// Take the foot's preface and its request, then drop the stream where it
    /// stands.
    Vanishes,
    /// Hang up before the handshake: a server that cannot be reached.
    Unreachable,
    /// **A read held open** (REMOTE §5.8): take the request, then write what
    /// the test sends, as it sends it — `Some` a frame, `None` the terminator
    /// — and go away without one when the test drops its end.
    Held(Arc<Mutex<Receiver<Option<Value>>>>),
}

impl Dial {
    /// One request, answered in one frame.
    pub(crate) fn one(answer: Value) -> Self {
        Self::Answers(vec![vec![answer]])
    }

    /// The posts of one run, each answered in one frame on one connection.
    pub(crate) fn run(answers: Vec<Value>) -> Self {
        Self::Answers(answers.into_iter().map(|answer| vec![answer]).collect())
    }
}

/// A listener standing in for yog, and what it was told.
///
/// It answers no address of its own: the address it bound is written into the
/// scratch directory where [`material`](crate::channel::material) reads it, so
/// a test opens a channel exactly the way an operator-provisioned box does.
pub(crate) struct Server {
    seen: Arc<Mutex<Vec<Value>>>,
}

impl Server {
    /// Bind loopback, write the bound address into `dir` where
    /// [`material`](crate::channel::material) reads it, and serve one
    /// connection per entry in `script` — answering the n-th dial with the
    /// n-th entry's frames, then the terminator.
    ///
    /// `protocol` is what the server states as its version, so a test can make
    /// the two ends disagree without a second code path.
    pub(crate) fn start(dir: &Path, protocol: u32, script: Vec<Vec<Value>>) -> Self {
        let script = script.into_iter().map(|frames| Dial::Answers(vec![frames]));
        Self::scripted(dir, protocol, FLOOR, script.collect())
    }

    /// A server that states its version, takes what the foot says — and then
    /// **goes away mid-conversation**. One connection, and then the port is
    /// gone too.
    pub(crate) fn vanishes(dir: &Path) -> Self {
        Self::scripted(dir, crate::corpus::PROTOCOL, FLOOR, vec![Dial::Vanishes])
    }

    /// **A server that drops some dials and answers others**: `None` is the
    /// connection it disappears on, `Some(frames)` the connection it answers.
    /// A foot dials per ask, so this is how a test writes a wire that flaps
    /// under a conversation and comes back.
    pub(crate) fn flapping(dir: &Path, script: Vec<Option<Vec<Value>>>) -> Self {
        let dial =
            |answer: Option<Vec<Value>>| answer.map_or(Dial::Vanishes, |f| Dial::Answers(vec![f]));
        let script = script.into_iter().map(dial).collect();
        Self::scripted(dir, crate::corpus::PROTOCOL, FLOOR, script)
    }

    /// **A server that reads a capture posted in pieces** (REMOTE §5.7) and
    /// does not push: it states 31, the last edition before `holding`, and the
    /// script says what each connection does.
    pub(crate) fn streaming(dir: &Path, script: Vec<Dial>) -> Self {
        let edition = crate::channel::hello::PUSHES - 1;
        Self::scripted(dir, crate::corpus::PROTOCOL, edition, script)
    }

    /// **A server that pushes** (REMOTE §5.8): it states the edition this
    /// vendored ledger is at, which is past `holding`'s stamp.
    pub(crate) fn pushing(dir: &Path, script: Vec<Dial>) -> Self {
        let edition = crate::corpus::ledger::edition();
        Self::scripted(dir, crate::corpus::PROTOCOL, edition, script)
    }

    /// The one listener: serve one connection per script entry, in the order
    /// they are dialled — each on a thread of its own, because a pushing foot
    /// holds its read open while its jobs dial to post.
    fn scripted(dir: &Path, protocol: u32, edition: u32, script: Vec<Dial>) -> Self {
        let config = server_config(dir);
        let (listener, seen) = bound(dir);
        let recorded = Arc::clone(&seen);
        let preface = json!({ "protocol": protocol, "edition": edition });
        std::thread::spawn(move || {
            for dial in script {
                let (tcp, _) = listener.accept().expect("a dial");
                let (config, preface, seen) =
                    (Arc::clone(&config), preface.clone(), Arc::clone(&recorded));
                std::thread::spawn(move || serve(&config, tcp, &preface, dial, &seen));
            }
        });
        Self { seen }
    }

    /// Every frame it has been handed, in order and across connections.
    pub(crate) fn heard(&self) -> Vec<Value> {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// Bind loopback and write the bound address into `dir`, where
/// [`material`](crate::channel::material) reads it — so a test opens a channel
/// exactly the way an operator-provisioned box does.
fn bound(dir: &Path) -> (TcpListener, Arc<Mutex<Vec<Value>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    let address = listener.local_addr().expect("bound").to_string();
    std::fs::write(dir.join(material::ADDRESS), address).expect("the address file");
    (listener, Arc::new(Mutex::new(Vec::new())))
}

/// One connection: state a version, record the foot's preface and every
/// request it writes, and answer each as the script says.
///
/// Every write ignores its error. A test that makes thrall refuse mid-exchange
/// — a version mismatch, an untrusted anchor — leaves this end writing into a
/// socket that is already gone, and that is the *expected* shape of those
/// tests rather than a failure of the stand-in.
fn serve(
    config: &Arc<ServerConfig>,
    tcp: TcpStream,
    preface: &Value,
    dial: Dial,
    seen: &Arc<Mutex<Vec<Value>>>,
) {
    if matches!(dial, Dial::Unreachable) {
        return;
    }
    let conn = ServerConnection::new(Arc::clone(config)).expect("a server connection");
    let mut tls = StreamOwned::new(conn, tcp);
    let _ = frame::write_value(&mut tls, preface);
    hear(&mut tls, seen);
    let answers = match dial {
        Dial::Answers(answers) => answers,
        Dial::Held(said) => return held(&mut tls, &said, seen),
        Dial::Vanishes | Dial::Unreachable => return hear(&mut tls, seen),
    };
    for answer in answers {
        hear(&mut tls, seen);
        for value in &answer {
            let _ = frame::write_value(&mut tls, value);
        }
        let _ = frame::write_end(&mut tls);
    }
}

/// A read held open: hear its request, then write what the test sends until
/// the test lets go — and then go away, as a server whose socket died does.
fn held(
    tls: &mut StreamOwned<ServerConnection, TcpStream>,
    said: &Mutex<Receiver<Option<Value>>>,
    seen: &Arc<Mutex<Vec<Value>>>,
) {
    hear(tls, seen);
    let said = said.lock().unwrap_or_else(PoisonError::into_inner);
    while let Ok(next) = said.recv() {
        let _ = match next {
            Some(value) => frame::write_value(tls, &value),
            None => frame::write_end(tls),
        };
    }
}

/// Read one frame the foot wrote — its preface or a request — and record it.
fn hear(tls: &mut StreamOwned<ServerConnection, TcpStream>, seen: &Arc<Mutex<Vec<Value>>>) {
    if let Ok(Some(said)) = frame::read_value(tls) {
        seen.lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(said);
    }
}

/// The server's end of the mTLS: present the server leaf, and require a client
/// certificate the operator CA issued. Requiring one is the point — a stand-in
/// that accepted an anonymous connection would prove nothing about the channel
/// thrall actually opens.
fn server_config(dir: &Path) -> Arc<ServerConfig> {
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let anchors = tls::anchors(&dir.join(material::ANCHORS)).expect("the operator CA");
    let verifier =
        WebPkiClientVerifier::builder_with_provider(Arc::new(anchors), Arc::clone(&provider))
            .build()
            .expect("a client verifier");
    let chain: Vec<CertificateDer<'static>> =
        CertificateDer::pem_file_iter(dir.join(format!("{}.pem", super::mint::SERVER)))
            .expect("the server chain")
            .collect::<Result<_, _>>()
            .expect("the server chain");
    let key = PrivateKeyDer::from_pem_file(dir.join(format!("{}.key", super::mint::SERVER)))
        .expect("the server key");
    Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("tls versions")
            .with_client_cert_verifier(verifier)
            .with_single_cert(chain, key)
            .expect("the server identity"),
    )
}
