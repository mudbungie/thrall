//! **The one HTTPS request this crate makes** (REMOTE §15.1, §15.2): the
//! claim, posted to the registration door with no client certificate.
//!
//! **Hand-rolled, and it is one request.** The door answers exactly one
//! thing — `POST /yog/register/<code>` with a `Content-Length` — and closes;
//! an HTTP library would be a dependency linked for a request line, four
//! headers and a body. So this writes them and reads what comes back, under
//! three bounds: a connect that gives up, a socket that will not wait
//! forever, and a reply that cannot outgrow [`CAP`].
//!
//! **The roots are a parameter.** The door's certificate is the public web's
//! — held by whatever fronts the server — so production verifies it against
//! the web's roots (`register::web_roots`) and the suite against a root of
//! its own. The wire's mTLS configuration (`channel::tls`) is a different
//! trust store for a different question and nothing here touches it.
//!
//! **Whatever is not a `200` is one answer.** The door says the same bare
//! `404` for a code that is unknown, expired or already claimed, and which of
//! the three it was is not said to a stranger — so there is nothing here to
//! tell apart, and a reply this cannot read as a status line is the same
//! answer.

use std::io::{Read, Write};
use std::sync::Arc;
use std::time::Duration;

use rustls::{ClientConfig, ClientConnection, RootCertStore, StreamOwned};

use super::url::Door;

/// How long the socket may sit silent, in either direction, before the door
/// is judged gone.
const BOUND: Duration = Duration::from_secs(10);
/// The most a reply may weigh: a head, two certificates and a short list fit
/// several times over, and nothing a door legitimately says is larger.
const CAP: u64 = 64 * 1024;

/// Post `body` to `door` and answer the reply's body where the status is
/// `200`, `None` where it is anything else. `Err` is a door that could not be
/// reached or spoken to at all, in a sentence naming it.
pub(super) fn post(
    door: &Door,
    body: &str,
    roots: RootCertStore,
) -> Result<Option<String>, String> {
    let at = &door.address;
    let unreached = |what: String| format!("{at}: the registration could not be posted: {what}");
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| unreached(e.to_string()))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = crate::channel::server_name(at)?;
    let tcp = crate::channel::connect(at)?;
    let bounded = tcp
        .set_read_timeout(Some(BOUND))
        .and_then(|()| tcp.set_write_timeout(Some(BOUND)));
    bounded.map_err(|e| unreached(e.to_string()))?;
    let conn =
        ClientConnection::new(Arc::new(config), name).map_err(|e| unreached(e.to_string()))?;
    let mut tls = StreamOwned::new(conn, tcp);
    // The handshake happens inside this write, so a certificate the roots do
    // not cover is refused here and nothing of the claim is sent.
    tls.write_all(request(door, body).as_bytes())
        .map_err(|e| unreached(e.to_string()))?;
    let mut reply = Vec::new();
    // How the read ended is not consulted: a door that hangs up without a
    // TLS close, or never hangs up at all, has still said whatever it said,
    // and what was said is judged on its own — a body cut short is not JSON.
    let _ = tls.take(CAP).read_to_end(&mut reply);
    Ok(accepted(&String::from_utf8_lossy(&reply)))
}

/// The request, whole: the line, four headers and the body.
fn request(door: &Door, body: &str) -> String {
    format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        door.path,
        door.host,
        body.len()
    )
}

/// A reply's body, where its status line says `200`.
fn accepted(reply: &str) -> Option<String> {
    let (head, body) = reply.split_once("\r\n\r\n")?;
    let status = head.split_whitespace().nth(1)?;
    (status == "200").then(|| body.to_owned())
}

#[cfg(test)]
mod tests;
