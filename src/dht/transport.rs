//! The datagram seam the client stands on: one trait, and the std UDP socket
//! that fills it in production. The trait exists so the whole client is
//! tested two ways at once — against a fake DHT the suite runs on loopback
//! UDP through the real [`Udp`], and against a stand-in transport for the
//! failure arms no loopback socket will produce on demand (REMOTE §13.5).
//! Synchronous `std::net`, timeouts on the socket, no tokio.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

/// The most a datagram may carry; a longer one is truncated and, being no
/// longer well-formed, ignored by the reader.
const DATAGRAM: usize = 8192;

/// One outbound-only datagram endpoint.
pub(crate) trait Transport: Send {
    /// Send one datagram. A refusal here is per destination — an address
    /// family this socket cannot reach, say — and the client skips that node.
    fn send(&self, to: SocketAddr, bytes: &[u8]) -> io::Result<()>;

    /// Wait up to `wait` for one datagram: `Some` a datagram, `None` the wait
    /// elapsed. Anything else is the socket itself failing.
    fn recv(&self, wait: Duration) -> io::Result<Option<(SocketAddr, Vec<u8>)>>;
}

/// The production transport: a std UDP socket, ephemeral port, no listener
/// semantics — it reads only what answers something it sent.
pub(crate) struct Udp(UdpSocket);

impl Udp {
    /// Bind to `addr` — `0.0.0.0:0` for an ephemeral v4 port, `[::]:0` for v6.
    pub(crate) fn bind(addr: SocketAddr) -> io::Result<Udp> {
        UdpSocket::bind(addr).map(Udp)
    }

    /// The address the kernel gave this socket.
    #[cfg(test)]
    pub(crate) fn local_addr(&self) -> io::Result<SocketAddr> {
        self.0.local_addr()
    }
}

impl Transport for Udp {
    fn send(&self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        self.0.send_to(bytes, to).map(|_| ())
    }

    fn recv(&self, wait: Duration) -> io::Result<Option<(SocketAddr, Vec<u8>)>> {
        // A zero timeout is an error to std, and would mean "block forever";
        // the shortest wait a caller can mean is one tick.
        self.0
            .set_read_timeout(Some(wait.max(Duration::from_millis(1))))?;
        let mut buf = vec![0u8; DATAGRAM];
        match self.0.recv_from(&mut buf) {
            Ok((n, from)) => {
                buf.truncate(n);
                Ok(Some((from, buf)))
            }
            Err(e) => elapsed(&e).then_some(None).ok_or(e),
        }
    }
}

/// Did the wait run out, as opposed to the socket failing?
fn elapsed(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}

#[cfg(test)]
mod tests;
