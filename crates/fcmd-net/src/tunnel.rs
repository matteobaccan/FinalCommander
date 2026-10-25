//! Tunnels: `ssh -L`, through the connection the panel is already using.
//!
//! A port on this machine stands for a port the *server* can reach. Anything that connects to
//! it — a database client, a browser, the S3 provider next door pointed at a MinIO nobody
//! outside the server room can see — is carried over the SSH session and comes out the other
//! end as if it had been opened there.
//!
//! The point of having it here rather than running `ssh -L` beside the program is that there
//! is nothing else to set up: the session is open, the host key has been checked, the
//! authentication has happened. A tunnel costs one more channel on it.
//!
//! **It binds to the loopback address.** A tunnel is the server's network handed to this
//! machine, and binding it to every interface would hand it to the rest of the office too.
//! `Spec::everywhere` exists for the case where that is what is meant, and nothing sets it by
//! accident.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::net::TcpListener;

use crate::error::{NetError, Result};
use crate::remote::Remote;

/// What to forward where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    /// The port to listen on here. Zero asks the system for one.
    pub local_port: u16,
    /// The host to connect to *from the server*: `localhost` means the server itself.
    pub host: String,
    pub port: u16,
    /// Listen on every interface rather than on the loopback address only.
    pub everywhere: bool,
}

impl Spec {
    /// `8080:db.internal:5432`, `db.internal:5432`, `5432`.
    ///
    /// Three shapes because the first is what `ssh -L` takes and the other two are what people
    /// mean most of the time: the same port on the server's own loopback, with the local port
    /// defaulting to the remote one.
    pub fn parse(text: &str) -> Result<Spec> {
        let text = text.trim();
        let bad = |detail: &str| NetError::Connect {
            host: text.to_string(),
            detail: detail.to_string(),
        };
        let parts: Vec<&str> = text.split(':').collect();
        let spec = match parts.as_slice() {
            [port] => {
                let port = port.parse().map_err(|_| bad("a port number is expected"))?;
                Spec {
                    local_port: port,
                    host: "localhost".into(),
                    port,
                    everywhere: false,
                }
            }
            [host, port] => {
                let port = port.parse().map_err(|_| bad("a port number is expected"))?;
                if host.is_empty() {
                    return Err(bad("the host is empty"));
                }
                Spec {
                    local_port: port,
                    host: (*host).to_string(),
                    port,
                    everywhere: false,
                }
            }
            [local, host, port] => {
                let local_port = local
                    .parse()
                    .map_err(|_| bad("the local port is not a number"))?;
                let port = port.parse().map_err(|_| bad("a port number is expected"))?;
                if host.is_empty() {
                    return Err(bad("the host is empty"));
                }
                Spec {
                    local_port,
                    host: (*host).to_string(),
                    port,
                    everywhere: false,
                }
            }
            _ => return Err(bad("expected [local:]host:port, or a port on its own")),
        };
        if spec.port == 0 {
            return Err(bad("port zero is not a port to connect to"));
        }
        Ok(spec)
    }

    /// Listen on every interface. Not a default, and not reachable by typing a shape wrong.
    pub fn everywhere(mut self) -> Spec {
        self.everywhere = true;
        self
    }

    fn bind_address(&self) -> SocketAddr {
        let ip = if self.everywhere {
            IpAddr::V4(Ipv4Addr::UNSPECIFIED)
        } else {
            IpAddr::V4(Ipv4Addr::LOCALHOST)
        };
        SocketAddr::new(ip, self.local_port)
    }
}

impl std::fmt::Display for Spec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}:{}", self.local_port, self.host, self.port)
    }
}

/// A tunnel that is open. Dropping it closes the listener and every connection through it.
#[derive(Debug)]
pub struct Tunnel {
    pub id: u64,
    /// Which connection carries it: `user@host:port`, as a path names it.
    pub authority: String,
    pub spec: Spec,
    /// What was actually bound — the port the system chose, when zero was asked for.
    pub local: SocketAddr,
    counters: Arc<Counters>,
    /// Dropping this ends the accept loop. It is never sent on.
    _stop: tokio::sync::oneshot::Sender<()>,
}

#[derive(Debug, Default)]
struct Counters {
    served: AtomicU64,
    live: AtomicU64,
    failed: AtomicU64,
}

impl Tunnel {
    /// How many connections have gone through, how many are open now, and how many could not
    /// be opened at the far end.
    pub fn counts(&self) -> (u64, u64, u64) {
        (
            self.counters.served.load(Ordering::Relaxed),
            self.counters.live.load(Ordering::Relaxed),
            self.counters.failed.load(Ordering::Relaxed),
        )
    }

    /// `127.0.0.1:8080 → db.internal:5432 on mb@jump:22`, for a list the user reads.
    pub fn describe(&self) -> String {
        format!(
            "{} → {}:{} on {}",
            self.local, self.spec.host, self.spec.port, self.authority
        )
    }
}

/// Open a tunnel over `remote`.
///
/// Binding happens here, so a port already in use is an error the caller can show *before* the
/// tunnel appears in any list. Everything after that happens in a task: each connection to the
/// local port opens a channel on the session and the bytes are pumped both ways until either
/// end stops.
pub async fn open(remote: Remote, id: u64, spec: Spec) -> Result<Tunnel> {
    let listener = TcpListener::bind(spec.bind_address())
        .await
        .map_err(|e| NetError::Connect {
            host: spec.bind_address().to_string(),
            detail: e.to_string(),
        })?;
    let local = listener.local_addr().map_err(|e| NetError::Connect {
        host: spec.bind_address().to_string(),
        detail: e.to_string(),
    })?;

    let authority = remote.authority();
    let counters = Arc::new(Counters::default());
    let (stop, mut stopped) = tokio::sync::oneshot::channel();
    let task_counters = counters.clone();
    let task_spec = spec.clone();
    tokio::spawn(async move {
        loop {
            let accepted = tokio::select! {
                // Dropping the `Tunnel` closes the sender, which is how a tunnel is closed:
                // the loop ends, the listener is dropped and the port is free again.
                _ = &mut stopped => return,
                accepted = listener.accept() => accepted,
            };
            let Ok((socket, _)) = accepted else { return };
            let remote = remote.clone();
            let counters = task_counters.clone();
            let spec = task_spec.clone();
            tokio::spawn(async move {
                let channel = match remote.sftp().forward(&spec.host, spec.port).await {
                    Ok(channel) => channel,
                    Err(_) => {
                        // The far end refused: the connection that arrived is dropped, which
                        // is what a client sees when nothing is listening there either.
                        counters.failed.fetch_add(1, Ordering::Relaxed);
                        return;
                    }
                };
                counters.served.fetch_add(1, Ordering::Relaxed);
                counters.live.fetch_add(1, Ordering::Relaxed);
                let mut socket = socket;
                let mut channel = channel;
                let _ = tokio::io::copy_bidirectional(&mut socket, &mut channel).await;
                counters.live.fetch_sub(1, Ordering::Relaxed);
            });
        }
    });

    Ok(Tunnel {
        id,
        authority,
        spec,
        local,
        counters,
        _stop: stop,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_shapes_people_write() {
        assert_eq!(
            Spec::parse("8080:db.internal:5432").unwrap(),
            Spec {
                local_port: 8080,
                host: "db.internal".into(),
                port: 5432,
                everywhere: false
            }
        );
        // Without a local port, the same number is used on both sides.
        assert_eq!(
            Spec::parse("db.internal:5432").unwrap(),
            Spec {
                local_port: 5432,
                host: "db.internal".into(),
                port: 5432,
                everywhere: false
            }
        );
        // A port on its own is the server's own loopback.
        assert_eq!(
            Spec::parse("9000").unwrap(),
            Spec {
                local_port: 9000,
                host: "localhost".into(),
                port: 9000,
                everywhere: false
            }
        );
        assert_eq!(Spec::parse("  5432  ").unwrap().port, 5432);
        // Zero is a local port the system chooses, and never a remote one.
        assert_eq!(Spec::parse("0:db:5432").unwrap().local_port, 0);
        assert!(Spec::parse("0").is_err());
    }

    #[test]
    fn what_is_not_a_tunnel() {
        for bad in [
            "",
            "db.internal",
            "db.internal:",
            ":5432",
            "8080::5432",
            "eight:db:5432",
            "8080:db:5432:extra",
            "8080:db:port",
        ] {
            assert!(Spec::parse(bad).is_err(), "{bad:?} should not parse");
        }
    }

    #[test]
    fn a_tunnel_binds_the_loopback_unless_told_otherwise() {
        let spec = Spec::parse("8080:db:5432").unwrap();
        assert_eq!(spec.bind_address().ip().to_string(), "127.0.0.1");
        assert_eq!(spec.to_string(), "8080:db:5432");
        assert_eq!(spec.everywhere().bind_address().ip().to_string(), "0.0.0.0");
    }
}
