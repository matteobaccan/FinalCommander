//! Finding a FinalCommander on the local network (SPEC 04 §9.3).
//!
//! The caller broadcasts `FCMD-LINK/1 who <id>` (or `who *`); a machine that accepts calls
//! and has that ID answers to the sender alone, `FCMD-LINK/1 here <id> <port> <name>`, with the
//! TCP port its server listens on. A packet from outside the local network is dropped unread.

use std::collections::BTreeMap;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::Duration;

use tokio::net::UdpSocket;
use tokio::time::{timeout_at, Instant};

use super::net;

/// The UDP port discovery uses.
pub const PORT: u16 = 47831;

const MAGIC: &str = "FCMD-LINK/1";

/// A machine that answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub id: u32,
    pub name: String,
    /// Where its server listens.
    pub addr: SocketAddr,
}

/// Where a broadcast goes on the local network.
pub fn broadcast() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::BROADCAST), PORT)
}

/// Answer broadcasts on `socket` for this machine, until the socket fails.
pub async fn answer(socket: UdpSocket, id: u32, name: String, tcp_port: u16) -> io::Result<()> {
    let mut buf = [0u8; 512];
    loop {
        let (n, from) = socket.recv_from(&mut buf).await?;
        if !net::is_local(from.ip()) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&buf[..n]) else {
            continue;
        };
        let mut parts = text.split_whitespace();
        if parts.next() != Some(MAGIC) || parts.next() != Some("who") {
            continue;
        }
        let wanted = parts.next().unwrap_or("*");
        if wanted != "*" && wanted.parse::<u32>().ok() != Some(id) {
            continue;
        }
        let reply = format!("{MAGIC} here {id} {tcp_port} {name}");
        let _ = socket.send_to(reply.as_bytes(), from).await;
    }
}

/// Ask who is there on every network this machine is on that a broadcast can reach: the
/// all-ones broadcast, which Windows sends out of one network only when there are several
/// (WSL, VirtualBox, a VPN), and the broadcast of the network the default route goes out
/// of, taken as a /24 — what nearly every home and office network is.
pub async fn find_everywhere(want: Option<u32>, wait: Duration) -> io::Result<Vec<Found>> {
    let mut targets = vec![broadcast()];
    if let Some(IpAddr::V4(here)) = default_address() {
        let [a, b, c, _] = here.octets();
        targets.push(SocketAddr::new(
            IpAddr::V4(Ipv4Addr::new(a, b, c, 255)),
            PORT,
        ));
    }
    find_at(&targets, want, wait).await
}

/// The address this machine reaches the rest of the world from: asking the system which way
/// a packet to a public address would go sends nothing.
pub fn default_address() -> Option<IpAddr> {
    let socket = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:9").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    net::is_local(ip).then_some(ip)
}

/// Ask who is there (`want`: one ID, or everyone), sending to `target` — [`broadcast`] on a
/// real network — and gathering answers for `wait`.
pub async fn find(target: SocketAddr, want: Option<u32>, wait: Duration) -> io::Result<Vec<Found>> {
    find_at(&[target], want, wait).await
}

async fn find_at(
    targets: &[SocketAddr],
    want: Option<u32>,
    wait: Duration,
) -> io::Result<Vec<Found>> {
    let target = targets[0];
    let bind: SocketAddr = if target.is_ipv4() {
        "0.0.0.0:0".parse().expect("an address")
    } else {
        "[::]:0".parse().expect("an address")
    };
    let socket = UdpSocket::bind(bind).await?;
    socket.set_broadcast(true)?;
    let ask = match want {
        Some(id) => format!("{MAGIC} who {id}"),
        None => format!("{MAGIC} who *"),
    };
    // One that cannot be sent to (a network that is not there) is no reason not to try the
    // others; only when none could be is it an error.
    let mut sent = None;
    for target in targets {
        match socket.send_to(ask.as_bytes(), target).await {
            Ok(_) => sent = Some(Ok(())),
            Err(e) if sent.is_none() => sent = Some(Err(e)),
            Err(_) => {}
        }
    }
    sent.unwrap_or(Ok(()))?;
    let deadline = Instant::now() + wait;
    let mut found: BTreeMap<u32, Found> = BTreeMap::new();
    let mut buf = [0u8; 512];
    while let Ok(Ok((n, from))) = timeout_at(deadline, socket.recv_from(&mut buf)).await {
        if !net::is_local(from.ip()) {
            continue;
        }
        let Ok(text) = std::str::from_utf8(&buf[..n]) else {
            continue;
        };
        let mut parts = text.splitn(5, ' ');
        if parts.next() != Some(MAGIC) || parts.next() != Some("here") {
            continue;
        }
        let (Some(id), Some(port)) = (
            parts.next().and_then(|s| s.parse::<u32>().ok()),
            parts.next().and_then(|s| s.parse::<u16>().ok()),
        ) else {
            continue;
        };
        let name = parts.next().unwrap_or("").to_string();
        if want.is_some_and(|w| w != id) {
            continue;
        }
        found.insert(
            id,
            Found {
                id,
                name,
                addr: SocketAddr::new(from.ip(), port),
            },
        );
        if want.is_some() {
            break;
        }
    }
    Ok(found.into_values().collect())
}
