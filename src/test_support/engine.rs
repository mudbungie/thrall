//! **The stand-in engine**: the far end of the wire, so the suite can test a
//! channel against something that speaks the protocol rather than against a
//! mock of thrall's own beliefs about it.
//!
//! It listens, which is the one thing thrall must never do (DESIGN §2) — and
//! that is precisely why it is here and not in the crate proper. What it is
//! standing in for is yog: a real listener, a real mTLS handshake requiring a
//! client certificate the operator CA issued, a real version preface, and
//! answers framed the way REMOTE §3 frames them.
//!
//! **It is scripted, one answer per connection**, because a foot dials per ask
//! and holds a connection only while it is waiting. So a test says what the
//! engine answers the first dial, the second, and so on.
//!
//! **It records every frame it is handed, in order** — the version preface and
//! then the request, per connection. That is how a test asserts what thrall
//! *said* rather than only what it did with the reply, and it is the only
//! witness that the preface goes out in the same breath as the request (REMOTE
//! §3: both ends write before either reads).

use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{ServerConfig, ServerConnection, StreamOwned};
use serde_json::{Value, json};

use crate::channel::{frame, material, tls};

/// How many frames a foot writes per connection: its preface, then its
/// request.
const FRAMES_IN: usize = 2;

/// A listener standing in for yog, and what it was told.
///
/// It answers no address of its own: the address it bound is written into the
/// scratch directory where [`material`](crate::channel::material) reads it, so
/// a test opens a channel exactly the way an operator-provisioned box does.
pub(crate) struct Engine {
    seen: Arc<Mutex<Vec<Value>>>,
}

impl Engine {
    /// Bind loopback, write the bound address into `dir` where
    /// [`material`](crate::channel::material) reads it, and serve one
    /// connection per entry in `script` — answering the n-th dial with the
    /// n-th entry's frames, then the terminator.
    ///
    /// `protocol` is what the engine states as its version, so a test can make
    /// the two ends disagree without a second code path.
    pub(crate) fn start(dir: &Path, protocol: u32, script: Vec<Vec<Value>>) -> Self {
        let config = server_config(dir);
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let address = listener.local_addr().expect("bound").to_string();
        std::fs::write(dir.join(material::ADDRESS), address).expect("the address file");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let recorded = Arc::clone(&seen);
        std::thread::spawn(move || {
            for answer in script {
                let (tcp, _) = listener.accept().expect("a dial");
                serve(&config, tcp, protocol, &answer, &recorded);
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

/// One connection: state a version, record the two frames the foot writes —
/// its own preface and its request — then answer and terminate.
///
/// Every write ignores its error. A test that makes thrall refuse mid-exchange
/// — a version mismatch, an untrusted anchor — leaves this end writing into a
/// socket that is already gone, and that is the *expected* shape of those
/// tests rather than a failure of the stand-in.
fn serve(
    config: &Arc<ServerConfig>,
    tcp: TcpStream,
    protocol: u32,
    answer: &[Value],
    seen: &Arc<Mutex<Vec<Value>>>,
) {
    let conn = ServerConnection::new(Arc::clone(config)).expect("a server connection");
    let mut tls = StreamOwned::new(conn, tcp);
    let _ = frame::write_value(&mut tls, &json!({ "protocol": protocol }));
    for _ in 0..FRAMES_IN {
        if let Ok(Some(said)) = frame::read_value(&mut tls) {
            seen.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(said);
        }
    }
    for value in answer {
        let _ = frame::write_value(&mut tls, value);
    }
    let _ = frame::write_end(&mut tls);
}

/// The engine's end of the mTLS: present the engine leaf, and require a client
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
        CertificateDer::pem_file_iter(dir.join(format!("{}.pem", super::mint::ENGINE)))
            .expect("the engine chain")
            .collect::<Result<_, _>>()
            .expect("the engine chain");
    let key = PrivateKeyDer::from_pem_file(dir.join(format!("{}.key", super::mint::ENGINE)))
        .expect("the engine key");
    Arc::new(
        ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("tls versions")
            .with_client_cert_verifier(verifier)
            .with_single_cert(chain, key)
            .expect("the engine identity"),
    )
}
