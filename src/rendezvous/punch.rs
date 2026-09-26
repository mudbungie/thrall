//! The punch (REMOTE §13.3): TCP simultaneous open from one fixed port, and
//! the data path the wire then rides directly.
//!
//! **Listen and connect from one port.** A [`Punch`] binds its port twice
//! over — a listener per address family — and a punch toward a peer binds a
//! *third* socket to the same port per target and connects from it. Whichever
//! SYN lands first is the connection: the peer's on our listener, ours on
//! theirs, or both crossing in the middle, which is the simultaneous open a
//! NAT pair needs (REMOTE §13.8: a "one side just listens" shortcut has no
//! chance). That takes `SO_REUSEADDR` and, on Linux, `SO_REUSEPORT` — the
//! calls `socket2` exists to make, and the reason it is a dependency (REMOTE
//! §13.7 ruling 1, operator 2026-09-23).
//!
//! **The first stream that lands is the connection** — the client's half of
//! REMOTE §13.3's rule. Two hosts that can both reach each other's listener
//! form two connections; the engine serves every one that lands and lets the
//! one nobody speaks on die in its handshake, so this end keeps the first and
//! drops the rest, and no negotiation over which to keep is needed.
//!
//! **This is the one socket a foot ever listens on, and DESIGN §2 says so.**
//! It exists only on an entry holding rendezvous material, it is bound once
//! per run, and it accepts only inside a window this end opened by calling
//! the engine. A SYN arriving outside a window sits in the backlog: the next
//! punch takes it if the engine is still serving it, and a handshake that
//! fails is the ordinary wire failure the redial already answers.
//!
//! **v6 first** where both ends published it (REMOTE §13.3): a stateful v6
//! firewall has no ports to rewrite. Targets are punched in parallel
//! regardless — the ordering is which stream this end sees first.

use socket2::{Domain, Protocol, SockAddr, Socket, Type};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// How often the listeners are looked at while a punch waits.
const ACCEPT_POLL: Duration = Duration::from_millis(20);
/// The longest one SYN is given before the next is sent.
const ATTEMPT: Duration = Duration::from_secs(2);

/// One fixed punch port: its listeners, held for the run.
#[derive(Debug)]
pub(crate) struct Punch {
    port: u16,
    listeners: Vec<TcpListener>,
}

impl Punch {
    /// Bind `port` on both families — `0` lets the kernel choose once, and
    /// the v6 listener then takes the same number. A family the box lacks is
    /// simply not listened on; a port nothing can bind is a refusal.
    pub(crate) fn bind(port: u16) -> Result<Punch, String> {
        let v4 = listener(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), port))?;
        let port = v4.local_addr().map_err(|e| e.to_string())?.port();
        let mut listeners = vec![v4];
        if let Ok(v6) = listener(SocketAddr::new(IpAddr::V6(Ipv6Addr::UNSPECIFIED), port)) {
            listeners.push(v6);
        }
        Ok(Punch { port, listeners })
    }

    /// The port every SYN leaves from and every listener sits on.
    pub(crate) fn port(&self) -> u16 {
        self.port
    }

    /// Simultaneous-open toward every target for up to `window`: the first
    /// stream that lands — connected or accepted — or none.
    pub(crate) fn punch(&self, targets: Vec<SocketAddr>, window: Duration) -> Option<TcpStream> {
        let (tx, rx) = mpsc::channel();
        let done = Arc::new(AtomicBool::new(false));
        for target in ordered(targets) {
            let (tx, done, port) = (tx.clone(), Arc::clone(&done), self.port);
            std::thread::spawn(move || connect(port, target, window, &done, &tx));
        }
        drop(tx);
        let started = Instant::now();
        let landed = loop {
            if started.elapsed() >= window {
                break None;
            }
            if let Some(stream) = self.accepted() {
                break Some(stream);
            }
            if let Ok(stream) = rx.try_recv() {
                break Some(stream);
            }
            std::thread::sleep(ACCEPT_POLL);
        };
        done.store(true, Ordering::Relaxed);
        landed
    }

    /// One look at every listener.
    fn accepted(&self) -> Option<TcpStream> {
        self.listeners.iter().find_map(|listener| {
            let (stream, _) = listener.accept().ok()?;
            let _ = stream.set_nonblocking(false);
            Some(stream)
        })
    }
}

/// v6 endpoints ahead of v4, each family in the order given.
pub(crate) fn ordered(targets: Vec<SocketAddr>) -> Vec<SocketAddr> {
    let (v6, v4): (Vec<_>, Vec<_>) = targets.into_iter().partition(SocketAddr::is_ipv6);
    v6.into_iter().chain(v4).collect()
}

/// One target's SYNs: from `port`, one every [`ATTEMPT`] at most, until one
/// lands, the window closes, or the caller has what it needs.
fn connect(
    port: u16,
    target: SocketAddr,
    window: Duration,
    done: &AtomicBool,
    tx: &mpsc::Sender<TcpStream>,
) {
    let started = Instant::now();
    while !done.load(Ordering::Relaxed) && started.elapsed() < window {
        let attempt = ATTEMPT.min(window.saturating_sub(started.elapsed()));
        if let Ok(socket) = reusable(target.is_ipv6(), port)
            && socket
                .connect_timeout(&SockAddr::from(target), attempt)
                .is_ok()
        {
            let _ = tx.send(TcpStream::from(socket));
            return;
        }
        std::thread::sleep(ACCEPT_POLL);
    }
}

/// A listener on `at`, with the port reusable by the connectors beside it.
fn listener(at: SocketAddr) -> Result<TcpListener, String> {
    let socket = reusable(at.is_ipv6(), at.port()).map_err(|e| format!("punch {at}: {e}"))?;
    socket.listen(8).map_err(|e| format!("punch {at}: {e}"))?;
    socket
        .set_nonblocking(true)
        .map_err(|e| format!("punch {at}: {e}"))?;
    Ok(TcpListener::from(socket))
}

/// A TCP socket of one family, bound to `port` on every address of it, with
/// the two reuse options set before the bind — the whole of what `socket2`
/// is here for.
fn reusable(v6: bool, port: u16) -> std::io::Result<Socket> {
    let domain = if v6 { Domain::IPV6 } else { Domain::IPV4 };
    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.set_reuse_port(true)?;
    let ip = if v6 {
        IpAddr::V6(Ipv6Addr::UNSPECIFIED)
    } else {
        IpAddr::V4(Ipv4Addr::UNSPECIFIED)
    };
    if v6 {
        socket.set_only_v6(true)?;
    }
    socket.bind(&SockAddr::from(SocketAddr::new(ip, port)))?;
    Ok(socket)
}

/// The addresses this box would send from, one per family it has a route
/// on: a UDP socket "connected" to a global address sends nothing and reads
/// back the local end the route would use — so the address only has to
/// select the default route, and the documentation ranges (RFC 5737, RFC
/// 3849) do that as well as any real host would. Loopback never appears — a
/// box with no route has no address to publish, and says so with an empty
/// list, which is a call the engine answers by listening alone.
pub(crate) fn local_ips() -> Vec<IpAddr> {
    ["[2001:db8::1]:53", "192.0.2.1:53"]
        .iter()
        .filter_map(|probe| {
            let bind = if probe.starts_with('[') {
                "[::]:0"
            } else {
                "0.0.0.0:0"
            };
            let socket = UdpSocket::bind(bind).ok()?;
            socket.connect(probe).ok()?;
            let ip = socket.local_addr().ok()?.ip();
            (!ip.is_loopback()).then_some(ip)
        })
        .collect()
}

#[cfg(test)]
mod tests;
