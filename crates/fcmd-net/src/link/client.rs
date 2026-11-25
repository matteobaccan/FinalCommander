//! The calling side of the Link (SPEC 04 §9.4, §9.8).
//!
//! [`call`] opens the TLS connection and exchanges hellos; it returns a [`Pending`] call with
//! the six digits to show and the other side's fingerprint, for the program to compare with
//! the one it knows. [`Pending::ask`] then asks for a share and waits for the person there to
//! answer; a yes is a [`LinkFs`], which the panel reads like an SFTP connection.
//!
//! One request at a time on the connection: a file is read in chunks the reader asks for
//! ([`CHUNK`] each), so a long copy never holds the connection while the panel lists a
//! folder.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use futures_util::stream;
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;

use super::identity::{fingerprint, sas, Identity};
use super::wire::{self, Answer, Ask, Frame, Hello, Op, Payload, Reply, Request};
use super::{net, tls, ANSWER_SECONDS, VERSION};
use crate::error::{NetError, Result};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};
use crate::remote::CHUNK;

/// The machine on the other end.
#[derive(Debug, Clone)]
pub struct Peer {
    pub id: u32,
    pub name: String,
    pub fingerprint: [u8; 32],
    pub addr: SocketAddr,
}

/// A call answered by the machine but not yet by its person.
pub struct Pending {
    stream: TlsStream<TcpStream>,
    pub peer: Peer,
    /// The six digits to show; the same on the other screen.
    pub sas: String,
}

impl std::fmt::Debug for Pending {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pending")
            .field("peer", &self.peer)
            .field("sas", &self.sas)
            .finish_non_exhaustive()
    }
}

fn connect_error(addr: SocketAddr, detail: impl std::fmt::Display) -> NetError {
    NetError::Connect {
        host: addr.to_string(),
        detail: detail.to_string(),
    }
}

/// Call the machine at `addr`, which must be on the local network.
pub async fn call(addr: SocketAddr, identity: &Identity) -> Result<Pending> {
    if !net::is_local(addr.ip()) {
        return Err(connect_error(
            addr,
            "not on the local network: the Link stays there",
        ));
    }
    let tcp = timeout(Duration::from_secs(5), TcpStream::connect(addr))
        .await
        .map_err(|_| connect_error(addr, "no answer"))?
        .map_err(|e| connect_error(addr, e))?;
    let config = tls::client_config(identity).map_err(|e| connect_error(addr, e))?;
    let mut stream = timeout(
        Duration::from_secs(10),
        TlsConnector::from(config).connect(tls::server_name(), tcp),
    )
    .await
    .map_err(|_| connect_error(addr, "TLS took too long"))?
    .map_err(|e| connect_error(addr, e))?;
    let their_fp = {
        let (_, conn) = stream.get_ref();
        let cert = conn
            .peer_certificates()
            .and_then(|c| c.first())
            .ok_or_else(|| connect_error(addr, "no certificate"))?;
        fingerprint(cert.as_ref())
    };
    wire::send(
        &mut stream,
        &Hello {
            version: VERSION,
            id: identity.id,
            name: identity.name.clone(),
        },
    )
    .await
    .map_err(|e| connect_error(addr, e))?;
    let hello: Hello = timeout(Duration::from_secs(10), wire::recv(&mut stream))
        .await
        .map_err(|_| connect_error(addr, "no hello"))?
        .map_err(|e| connect_error(addr, e))?;
    Ok(Pending {
        stream,
        sas: sas(&identity.fingerprint(), &their_fp),
        peer: Peer {
            id: hello.id,
            name: hello.name,
            fingerprint: their_fp,
            addr,
        },
    })
}

impl Pending {
    /// Ask for `share` (empty: the first offered), to read or to write, and wait for the
    /// person there to answer.
    pub async fn ask(mut self, share: &str, write: bool) -> Result<LinkFs> {
        let addr = self.peer.addr;
        wire::send(
            &mut self.stream,
            &Ask {
                share: share.to_string(),
                write,
            },
        )
        .await
        .map_err(|e| connect_error(addr, e))?;
        let answer: Answer = timeout(
            Duration::from_secs(ANSWER_SECONDS + 15),
            wire::recv(&mut self.stream),
        )
        .await
        .map_err(|_| connect_error(addr, "no answer from the other side"))?
        .map_err(|e| connect_error(addr, e))?;
        if !answer.ok {
            return Err(connect_error(
                addr,
                answer.reason.unwrap_or_else(|| "refused".into()),
            ));
        }
        Ok(LinkFs {
            conn: Arc::new(Mutex::new(Conn {
                stream: self.stream,
                next: 1,
            })),
            peer: self.peer,
            share: answer.share,
            write: answer.write,
        })
    }
}

struct Conn {
    stream: TlsStream<TcpStream>,
    next: u64,
}

impl Conn {
    async fn request(&mut self, op: Op) -> std::io::Result<Frame> {
        let n = self.next;
        self.next += 1;
        wire::send(&mut self.stream, &Request { n, op }).await?;
        wire::read_frame(&mut self.stream).await
    }
}

/// An accepted Link session, read like any remote filesystem.
pub struct LinkFs {
    conn: Arc<Mutex<Conn>>,
    pub peer: Peer,
    /// The share's name, as the other side calls it.
    pub share: String,
    /// Whether the other side let this session write.
    pub write: bool,
}

impl std::fmt::Debug for LinkFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LinkFs")
            .field("peer", &self.peer.id)
            .field("share", &self.share)
            .field("write", &self.write)
            .finish()
    }
}

impl LinkFs {
    async fn op(&self, path: &str, op: Op) -> Result<Payload> {
        let frame = self
            .conn
            .lock()
            .await
            .request(op)
            .await
            .map_err(|e| remote(path, e))?;
        match frame {
            Frame::Json(body) => {
                let reply: Reply = serde_json::from_slice(&body).map_err(|e| remote(path, e))?;
                reply.result.map_err(|e| NetError::Remote {
                    path: path.to_string(),
                    detail: e,
                })
            }
            Frame::Data(_) => Err(remote(path, "bytes where an answer was expected")),
        }
    }

    fn read_only(&self, path: &str) -> NetError {
        NetError::Unsupported {
            path: path.to_string(),
            detail: if self.write {
                "writing over the Link comes in a later version".into()
            } else {
                "this Link session is read only".into()
            },
        }
    }
}

fn remote(path: &str, e: impl std::fmt::Display) -> NetError {
    NetError::Remote {
        path: path.to_string(),
        detail: e.to_string(),
    }
}

#[async_trait]
impl RemoteFs for LinkFs {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        match self.op(path, Op::ReadDir { path: path.into() }).await? {
            Payload::Entries { entries } => {
                Ok(entries.into_iter().map(RemoteEntry::from).collect())
            }
            _ => Err(remote(path, "an unexpected answer")),
        }
    }

    async fn stat(&self, path: &str, follow: bool) -> Result<RemoteEntry> {
        match self
            .op(
                path,
                Op::Stat {
                    path: path.into(),
                    follow,
                },
            )
            .await?
        {
            Payload::Entry { entry } => Ok(entry.into()),
            _ => Err(remote(path, "an unexpected answer")),
        }
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        Err(self.read_only(path))
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        Err(self.read_only(path))
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        Err(self.read_only(path))
    }

    async fn rename(&self, from: &str, _to: &str) -> Result<()> {
        Err(self.read_only(from))
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        match self.op(path, Op::ReadLink { path: path.into() }).await? {
            Payload::Path { path } => Ok(path),
            _ => Err(remote(path, "an unexpected answer")),
        }
    }

    async fn symlink(&self, _target: &str, link: &str) -> Result<()> {
        Err(self.read_only(link))
    }

    async fn set_meta(&self, path: &str, _mode: Option<u32>, _mtime: Option<u32>) -> Result<()> {
        Err(self.read_only(path))
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let (handle, size) = match self.op(path, Op::Open { path: path.into() }).await? {
            Payload::Opened { handle, size } => (handle, size),
            _ => return Err(remote(path, "an unexpected answer")),
        };
        let conn = self.conn.clone();
        let name = path.to_string();
        // Each step asks for the next chunk; the last one closes the file on the other side.
        let chunks = stream::unfold(Some(0u64), move |state| {
            let conn = conn.clone();
            let name = name.clone();
            async move {
                let offset = state?;
                let mut c = conn.lock().await;
                if offset >= size {
                    let _ = c.request(Op::Close { handle }).await;
                    return None;
                }
                let frame = c
                    .request(Op::Read {
                        handle,
                        offset,
                        len: CHUNK as u32,
                    })
                    .await;
                drop(c);
                let item: std::io::Result<std::io::Cursor<Vec<u8>>> = match frame {
                    Ok(Frame::Data(bytes)) if !bytes.is_empty() => {
                        let next = offset + bytes.len() as u64;
                        return Some((Ok(std::io::Cursor::new(bytes)), Some(next)));
                    }
                    Ok(Frame::Data(_)) => Err(std::io::Error::new(
                        std::io::ErrorKind::UnexpectedEof,
                        format!("{name}: shorter than it said"),
                    )),
                    Ok(Frame::Json(body)) => {
                        let detail = serde_json::from_slice::<Reply>(&body)
                            .ok()
                            .and_then(|r| r.result.err())
                            .unwrap_or_else(|| "an unexpected answer".into());
                        Err(std::io::Error::other(format!("{name}: {detail}")))
                    }
                    Err(e) => Err(e),
                };
                Some((item, None))
            }
        });
        Ok(Box::new(tokio_util::io::StreamReader::new(Box::pin(
            chunks,
        ))))
    }

    async fn create(&self, path: &str) -> Result<Writer> {
        Err(self.read_only(path))
    }

    async fn canonicalize(&self, path: &str) -> Result<String> {
        match self
            .op(path, Op::Canonicalize { path: path.into() })
            .await?
        {
            Payload::Path { path } => Ok(path),
            _ => Err(remote(path, "an unexpected answer")),
        }
    }

    async fn close(&self) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let _ = self.conn.lock().await.stream.shutdown().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::super::server::{serve, Admission, Caller, Gate};
    use super::super::share::Share;
    use super::*;
    use std::sync::Mutex as StdMutex;
    use tokio::io::AsyncReadExt;

    struct TestGate {
        shares: Vec<Share>,
        answer: Admission,
        seen: StdMutex<Vec<Caller>>,
    }

    #[async_trait]
    impl Gate for TestGate {
        fn shares(&self) -> Vec<Share> {
            self.shares.clone()
        }
        async fn admit(&self, caller: &Caller) -> Admission {
            self.seen.lock().unwrap().push(caller.clone());
            self.answer
        }
    }

    /// A server on loopback sharing `root`, answering `answer`.
    async fn server(
        root: &std::path::Path,
        answer: Admission,
    ) -> (SocketAddr, Arc<TestGate>, Identity) {
        let gate = Arc::new(TestGate {
            shares: vec![Share::new("docs", root, false).unwrap()],
            answer,
            seen: StdMutex::new(Vec::new()),
        });
        let them = Arc::new(Identity::generate("there").unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let g: Arc<dyn Gate> = gate.clone();
        tokio::spawn(serve(listener, them.clone(), g));
        (addr, gate, Identity::generate("here").unwrap())
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("shared");
        std::fs::create_dir_all(root.join("sub")).unwrap();
        std::fs::write(root.join("small.txt"), b"hello").unwrap();
        // Three chunks and a bit, so the reader asks more than once.
        let big: Vec<u8> = (0..(CHUNK * 3 + 1234)).map(|i| (i % 251) as u8).collect();
        std::fs::write(root.join("sub/big.bin"), big).unwrap();
        std::fs::write(dir.path().join("secret.txt"), b"secret").unwrap();
        dir
    }

    #[tokio::test]
    async fn a_call_said_yes_to_reads_the_share_and_nothing_else() {
        let dir = tree();
        let (addr, gate, me) = server(&dir.path().join("shared"), Admission::ReadOnly).await;

        let pending = call(addr, &me).await.unwrap();
        assert_eq!(pending.peer.name, "there");
        let digits = pending.sas.clone();
        let fs = pending.ask("", false).await.unwrap();
        assert_eq!(fs.share, "docs");
        assert!(!fs.write);
        {
            let seen = gate.seen.lock().unwrap();
            assert_eq!(seen.len(), 1);
            assert_eq!(seen[0].id, me.id);
            assert_eq!(seen[0].name, "here");
            assert_eq!(seen[0].sas, digits, "both screens show the same six digits");
            assert_eq!(seen[0].fingerprint, me.fingerprint());
        }

        let mut names: Vec<String> = fs
            .read_dir("/")
            .await
            .unwrap()
            .into_iter()
            .map(|e| e.name)
            .collect();
        names.sort();
        assert_eq!(names, ["small.txt", "sub"]);
        assert_eq!(fs.stat("/small.txt", true).await.unwrap().size, 5);
        assert_eq!(
            fs.canonicalize("/sub/../sub/big.bin").await.ok(),
            None,
            "`..` is refused"
        );
        assert_eq!(
            fs.canonicalize("/sub/big.bin").await.unwrap(),
            "/sub/big.bin"
        );

        let mut got = Vec::new();
        fs.open_read("/sub/big.bin")
            .await
            .unwrap()
            .read_to_end(&mut got)
            .await
            .unwrap();
        let want = std::fs::read(dir.path().join("shared/sub/big.bin")).unwrap();
        assert_eq!(got.len(), want.len());
        assert!(got == want, "every byte, in several chunks");
        // The connection is still good after a whole file.
        assert_eq!(fs.stat("/small.txt", false).await.unwrap().size, 5);

        assert!(
            fs.open_read("/../secret.txt").await.is_err(),
            "outside the share"
        );
        assert!(fs.read_dir("/..").await.is_err());
        assert!(fs.create_dir("/new").await.is_err());
    }

    #[tokio::test]
    async fn a_read_only_session_is_enforced_by_the_called_side() {
        let dir = tree();
        let (addr, _gate, me) = server(&dir.path().join("shared"), Admission::ReadWrite).await;
        // The caller asks to write and the person says yes, but the share is read only.
        let fs = call(addr, &me)
            .await
            .unwrap()
            .ask("docs", true)
            .await
            .unwrap();
        assert!(!fs.write);
        // A caller that ignores that and sends a write anyway is refused there.
        let err = fs
            .op("/x", Op::CreateDir { path: "/x".into() })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("read only"), "{err}");
        assert!(!dir.path().join("shared/x").exists());
    }

    #[tokio::test]
    async fn a_refused_call_opens_nothing() {
        let dir = tree();
        let (addr, gate, me) = server(&dir.path().join("shared"), Admission::Refused).await;
        let err = call(addr, &me)
            .await
            .unwrap()
            .ask("", false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("not accepted"), "{err}");
        assert_eq!(gate.seen.lock().unwrap().len(), 1);
        let err = call(addr, &me)
            .await
            .unwrap()
            .ask("nope", false)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("no share called nope"), "{err}");
    }

    #[tokio::test]
    async fn nothing_off_the_local_network_is_called() {
        let me = Identity::generate("here").unwrap();
        let err = call("8.8.8.8:47832".parse().unwrap(), &me)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("local network"), "{err}");
    }

    #[tokio::test]
    async fn a_machine_is_found_by_its_id() {
        use super::super::discovery::{answer, find};
        let socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let at = socket.local_addr().unwrap();
        tokio::spawn(answer(socket, 123_456_789, "the other one".into(), 4242));
        let wait = Duration::from_millis(500);
        let found = find(at, Some(123_456_789), wait).await.unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "the other one");
        assert_eq!(found[0].addr, SocketAddr::new(at.ip(), 4242));
        assert_eq!(find(at, None, wait).await.unwrap().len(), 1);
        assert!(find(at, Some(987_654_321), wait).await.unwrap().is_empty());
    }
}
