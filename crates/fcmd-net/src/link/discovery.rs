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

/// Ask who is there (`want`: one ID, or everyone), sending to `target` — [`broadcast`] on a
/// real network — and gathering answers for `wait`.
pub async fn find(target: SocketAddr, want: Option<u32>, wait: Duration) -> io::Result<Vec<Found>> {
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
    socket.send_to(ask.as_bytes(), target).await?;
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
