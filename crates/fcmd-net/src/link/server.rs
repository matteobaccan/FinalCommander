//! The called side of the Link (SPEC 04 §9.2, §9.4, §9.7).
//!
//! Every connection: TLS with the caller's certificate demanded; each side's hello; the
//! caller's ask; then the person here is asked, through [`Gate::admit`], with the six
//! digits on the screen — and only a yes opens the share. Everything the caller sends after
//! that is resolved under the share ([`Share::resolve`]) and, in a read-only session, every
//! operation that would write is refused here, whatever the caller is.

use std::collections::HashMap;
use std::io::{self, SeekFrom};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, UNIX_EPOCH};

use async_trait::async_trait;
use fcmd_core::EntryKind;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;
use tokio_rustls::TlsAcceptor;

use super::identity::{fingerprint, sas, Identity};
use super::share::Share;
use super::wire::{self, Answer, Ask, Hello, Op, Payload, Reply, Request, WireEntry};
use super::{net, tls, ANSWER_SECONDS, VERSION};
use crate::ops::RemoteEntry;
use crate::remote::CHUNK;

/// Files a caller may hold open at once; the oldest is closed to make room.
const OPEN_FILES: usize = 64;

/// Who is calling, as the dialog shows it.
#[derive(Debug, Clone)]
pub struct Caller {
    pub id: u32,
    pub name: String,
    pub fingerprint: [u8; 32],
    /// The six digits, the same as on the caller's screen.
    pub sas: String,
    pub addr: SocketAddr,
    /// The share asked for, by name.
    pub share: String,
    /// Whether the caller asked to write.
    pub write: bool,
}

/// The person's answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    ReadOnly,
    ReadWrite,
    Refused,
}

/// What the server needs from the program around it: the shares, and the person.
#[async_trait]
pub trait Gate: Send + Sync {
    /// The folders offered, the first being the default.
    fn shares(&self) -> Vec<Share>;
    /// Ask the person. No answer within [`ANSWER_SECONDS`] counts as a refusal.
    async fn admit(&self, caller: &Caller) -> Admission;
    /// A session began (for the status line).
    fn opened(&self, _caller: &Caller, _write: bool) {}
    /// It ended.
    fn closed(&self, _caller: &Caller) {}
}

/// Accept calls until the listener fails. Connections from outside the local network are
/// closed unread.
pub async fn serve(
    listener: TcpListener,
    identity: Arc<Identity>,
    gate: Arc<dyn Gate>,
) -> io::Result<()> {
    let config = tls::server_config(&identity).map_err(io::Error::other)?;
    let acceptor = TlsAcceptor::from(config);
    loop {
        let (tcp, addr) = listener.accept().await?;
        if !net::is_local(addr.ip()) {
            drop(tcp);
            continue;
        }
        let acceptor = acceptor.clone();
        let identity = identity.clone();
        let gate = gate.clone();
        tokio::spawn(async move {
            let _ = connection(tcp, addr, acceptor, identity, gate).await;
        });
    }
}

async fn connection(
    tcp: TcpStream,
    addr: SocketAddr,
    acceptor: TlsAcceptor,
    identity: Arc<Identity>,
    gate: Arc<dyn Gate>,
) -> io::Result<()> {
    let mut tls = timeout(Duration::from_secs(10), acceptor.accept(tcp))
        .await
        .map_err(|_| io::Error::other("TLS took too long"))??;
    let their_fp = {
        let (_, conn) = tls.get_ref();
        let cert = conn
            .peer_certificates()
            .and_then(|c| c.first())
            .ok_or_else(|| io::Error::other("the caller showed no certificate"))?;
        fingerprint(cert.as_ref())
    };
    let hello: Hello = timeout(Duration::from_secs(10), wire::recv(&mut tls))
        .await
        .map_err(|_| io::Error::other("no hello"))??;
    wire::send(
        &mut tls,
        &Hello {
            version: VERSION,
            id: identity.id,
            name: identity.name.clone(),
        },
    )
    .await?;
    // The caller compares six digits before it asks: a person's pace.
    let ask: Ask = timeout(
        Duration::from_secs(ANSWER_SECONDS * 3),
        wire::recv(&mut tls),
    )
    .await
    .map_err(|_| io::Error::other("no ask"))??;

    let shares = gate.shares();
    let share = if ask.share.is_empty() {
        shares.first().cloned()
    } else {
        shares.iter().find(|s| s.name == ask.share).cloned()
    };
    let Some(share) = share else {
        let reason = if shares.is_empty() {
            "nothing is shared".to_string()
        } else {
            format!("no share called {}", ask.share)
        };
        return refuse(&mut tls, reason).await;
    };
    let caller = Caller {
        id: hello.id,
        name: hello.name.clone(),
        fingerprint: their_fp,
        sas: sas(&identity.fingerprint(), &their_fp),
        addr,
        share: share.name.clone(),
        write: ask.write,
    };
    let admission = timeout(Duration::from_secs(ANSWER_SECONDS), gate.admit(&caller))
        .await
        .unwrap_or(Admission::Refused);
    let write = admission == Admission::ReadWrite && ask.write && share.write;
    if admission == Admission::Refused {
        return refuse(&mut tls, "the call was not accepted".to_string()).await;
    }
    wire::send(
        &mut tls,
        &Answer {
            ok: true,
            write,
            share: share.name.clone(),
            reason: None,
        },
    )
    .await?;
    gate.opened(&caller, write);
    let result = session(&mut tls, &share, write).await;
    gate.closed(&caller);
    result
}

async fn refuse<S: tokio::io::AsyncWrite + Unpin>(s: &mut S, reason: String) -> io::Result<()> {
    wire::send(
        s,
        &Answer {
            ok: false,
            write: false,
            share: String::new(),
            reason: Some(reason),
        },
    )
    .await
}

/// The requests of one session, one at a time, until the caller hangs up.
async fn session<S>(s: &mut S, share: &Share, write: bool) -> io::Result<()>
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let mut files: HashMap<u32, tokio::fs::File> = HashMap::new();
    let mut order: Vec<u32> = Vec::new();
    let mut next_handle = 1u32;
    loop {
        let req: Request = match wire::recv(s).await {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(e) => return Err(e),
        };
        if req.op.writes() {
            let why = if write {
                "writing over the Link comes in a later version"
            } else {
                "this session is read only"
            };
            reply(s, req.n, Err(why.to_string())).await?;
            continue;
        }
        match req.op {
            Op::Read {
                handle,
                offset,
                len,
            } => {
                let Some(file) = files.get_mut(&handle) else {
                    reply(s, req.n, Err("no such open file".into())).await?;
                    continue;
                };
                let len = (len as usize).min(CHUNK);
                match read_at(file, offset, len).await {
                    Ok(bytes) => wire::send_data(s, &bytes).await?,
                    Err(e) => reply(s, req.n, Err(e.to_string())).await?,
                }
            }
            Op::Open { path } => {
                let result = open(share, &path).await;
                let answer = match result {
                    Ok((file, size)) => {
                        let handle = next_handle;
                        next_handle = next_handle.wrapping_add(1).max(1);
                        files.insert(handle, file);
                        order.push(handle);
                        if order.len() > OPEN_FILES {
                            let oldest = order.remove(0);
                            files.remove(&oldest);
                        }
                        Ok(Payload::Opened { handle, size })
                    }
                    Err(e) => Err(e.to_string()),
                };
                reply(s, req.n, answer).await?;
            }
            Op::Close { handle } => {
                files.remove(&handle);
                order.retain(|h| *h != handle);
                reply(s, req.n, Ok(Payload::Done)).await?;
            }
            op => {
                let answer = answer(share, op).await.map_err(|e| e.to_string());
                reply(s, req.n, answer).await?;
            }
        }
    }
}

async fn reply<S: tokio::io::AsyncWrite + Unpin>(
    s: &mut S,
    n: u64,
    result: Result<Payload, String>,
) -> io::Result<()> {
    wire::send(s, &Reply { n, result }).await
}

async fn read_at(file: &mut tokio::fs::File, offset: u64, len: usize) -> io::Result<Vec<u8>> {
    file.seek(SeekFrom::Start(offset)).await?;
    let mut buf = vec![0u8; len];
    let mut got = 0;
    while got < len {
        let n = file.read(&mut buf[got..]).await?;
        if n == 0 {
            break;
        }
        got += n;
    }
    buf.truncate(got);
    Ok(buf)
}

async fn open(share: &Share, path: &str) -> io::Result<(tokio::fs::File, u64)> {
    let real = share.resolve(path)?;
    let meta = tokio::fs::metadata(&real).await?;
    if !meta.is_file() {
        return Err(io::Error::other(format!("{path}: not a file")));
    }
    Ok((tokio::fs::File::open(&real).await?, meta.len()))
}

/// The operations answered with one message.
async fn answer(share: &Share, op: Op) -> io::Result<Payload> {
    match op {
        Op::ReadDir { path } => {
            let real = share.resolve(&path)?;
            let mut out = Vec::new();
            let mut dir = tokio::fs::read_dir(&real).await?;
            while let Some(item) = dir.next_entry().await? {
                let name = item.file_name().to_string_lossy().into_owned();
                if let Ok(meta) = tokio::fs::symlink_metadata(item.path()).await {
                    let link = if meta.file_type().is_symlink() {
                        tokio::fs::read_link(item.path())
                            .await
                            .ok()
                            .map(|t| t.to_string_lossy().into_owned())
                    } else {
                        None
                    };
                    out.push(WireEntry::from(&entry(name, &meta, link)));
                }
            }
            Ok(Payload::Entries { entries: out })
        }
        Op::Stat { path, follow } => {
            let real = share.resolve(&path)?;
            let meta = if follow {
                tokio::fs::metadata(&real).await?
            } else {
                tokio::fs::symlink_metadata(&real).await?
            };
            let name = real
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            Ok(Payload::Entry {
                entry: WireEntry::from(&entry(name, &meta, None)),
            })
        }
        Op::ReadLink { path } => {
            let real = share.resolve(&path)?;
            let target = tokio::fs::read_link(&real).await?;
            Ok(Payload::Path {
                path: target.to_string_lossy().into_owned(),
            })
        }
        Op::Canonicalize { path } => {
            let real = share.resolve(&path)?;
            Ok(Payload::Path {
                path: wire_path(&share.root, &real),
            })
        }
        _ => Err(io::Error::other("not an operation of this kind")),
    }
}

/// `real` as the caller names it: `/` and the path under the share.
fn wire_path(root: &Path, real: &Path) -> String {
    let rel = real.strip_prefix(root).unwrap_or(Path::new(""));
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    format!("/{}", parts.join("/"))
}

fn entry(name: String, meta: &std::fs::Metadata, link: Option<String>) -> RemoteEntry {
    let kind = if meta.file_type().is_symlink() {
        EntryKind::Symlink
    } else if meta.is_dir() {
        EntryKind::Dir
    } else {
        EntryKind::File
    };
    let mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs().min(u64::from(u32::MAX)) as u32);
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        Some(meta.permissions().mode())
    };
    #[cfg(not(unix))]
    let mode = Some(if meta.permissions().readonly() {
        0o444
    } else {
        0o644
    });
    RemoteEntry {
        name,
        kind,
        size: if meta.is_dir() { 0 } else { meta.len() },
        mtime,
        mode,
        link_target: link,
    }
}
