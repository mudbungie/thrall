//! **The stand-in registration door** (REMOTE §15.2): an HTTPS listener that
//! takes one request with no client certificate, and answers it.
//!
//! It stands in for whatever fronts a real server with a public certificate.
//! Here that certificate is the mint's server leaf, so the suite verifies it
//! against the mint's CA — which is why `register::post` takes its roots as
//! a parameter rather than reaching for the web's.
//!
//! **It answers with whatever the test hands it**, computed from the request
//! body: a real door signs a leaf for the posted key, and a test of the
//! happy path does exactly that through [`mint::claimed`](super::mint);
//! every other test answers a fixed reply.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};

use rustls::{ServerConfig, ServerConnection, StreamOwned};

use crate::channel::tls;

/// The code every test offer carries.
pub(crate) const CODE: &str = "ABCD2345";

/// What a door answers a request body with: the reply, whole.
pub(crate) type Answer = Box<dyn FnOnce(&str) -> String + Send>;

/// A listening door, and what it was sent.
pub(crate) struct Door {
    /// The registration URL that reaches it.
    pub(crate) url: String,
    seen: Arc<Mutex<String>>,
}

impl Door {
    /// Bind loopback under the server leaf minted in `dir`, and answer the
    /// one connection that arrives with `answer`.
    pub(crate) fn start(dir: &Path, answer: Answer) -> Self {
        let pair = |ext: &str| dir.join(format!("{}.{ext}", super::mint::SERVER));
        let (chain, key) = tls::identity(&pair("pem"), &pair("key")).expect("the server pair");
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let config = ServerConfig::builder_with_provider(provider)
            .with_safe_default_protocol_versions()
            .expect("tls versions")
            .with_no_client_auth()
            .with_single_cert(chain, key)
            .expect("the door's identity");
        let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
        let url = format!(
            "https://{}/yog/register/{CODE}",
            listener.local_addr().expect("bound")
        );
        let seen = Arc::new(Mutex::new(String::new()));
        let recorded = Arc::clone(&seen);
        std::thread::spawn(move || serve(&listener, Arc::new(config), answer, &recorded));
        Self { url, seen }
    }

    /// The request it was sent, head and body.
    pub(crate) fn heard(&self) -> String {
        self.seen
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

/// A `200` carrying `body`, the way the server writes one.
pub(crate) fn ok(body: &str) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
         Connection: close\r\n\r\n{body}",
        body.len()
    )
}

/// The bare `404` every unclaimable code earns.
pub(crate) fn not_found() -> String {
    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_owned()
}

/// One connection: read the request, record it, answer, hang up. Every error
/// is ignored — a test whose client refuses the handshake leaves this end
/// reading a socket that is already gone, and that is the test's own shape.
fn serve(listener: &TcpListener, config: Arc<ServerConfig>, answer: Answer, seen: &Mutex<String>) {
    let (tcp, _) = listener.accept().expect("a dial");
    let conn = ServerConnection::new(config).expect("a server connection");
    let mut tls = StreamOwned::new(conn, tcp);
    let mut request = String::new();
    let mut chunk = [0_u8; 4096];
    while !whole(&request) {
        match tls.read(&mut chunk) {
            Ok(n) if n > 0 => request.push_str(&String::from_utf8_lossy(&chunk[..n])),
            _ => return,
        }
    }
    let body = request.split_once("\r\n\r\n").expect("a head").1.to_owned();
    seen.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .push_str(&request);
    let _ = tls.write_all(answer(&body).as_bytes());
}

/// Whether `request` is a head and the whole body its `Content-Length`
/// states.
fn whole(request: &str) -> bool {
    let Some((head, body)) = request.split_once("\r\n\r\n") else {
        return false;
    };
    let stated = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .and_then(|n| n.parse::<usize>().ok());
    stated.is_some_and(|n| body.len() >= n)
}
