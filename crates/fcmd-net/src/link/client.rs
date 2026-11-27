//! The calling side of the Link (SPEC 04 §9.4, §9.8).
//!
//! [`call`] opens the TLS connection and exchanges hellos; it returns a [`Pending`] call with
//! the six digits to show and the other side's fingerprint, for the program to compare with
//! the one it knows. [`Pending::ask`] then asks for a share and waits for the person there to
//! answer; a yes is a [`LinkFs`], which the panel reads like an SFTP connection.
//!
//! One request at a time on the connection: a file is read in chunks the reader asks for
//! ([`READ_CHUNK`](wire::READ_CHUNK) each), so a long copy never holds the connection while the panel lists a
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
use super::pin::{self, Exchange};
use super::wire::{
    self, Answer, Ask, Frame, Hello, Op, Payload, PinMessage, PinOffer, PinProof, Reply, Request,
};
use super::{net, tls, ANSWER_SECONDS, VERSION};
use crate::error::{NetError, Result};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};

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
    /// This machine's fingerprint, which the PIN exchange is bound to.
    own: [u8; 32],
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
        own: identity.fingerprint(),
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
    pub async fn ask(self, share: &str, write: bool) -> Result<LinkFs> {
        self.ask_with(share, write, None, || {}).await
    }

    /// The same, proving `pin` when there is one (SPEC 04 §9.5): a machine that holds the
    /// same PIN lets the caller in without its person. `to_person` is called when the call
    /// goes to the person after all — no PIN given, or none set there — which is when the
    /// six digits are worth showing.
    pub async fn ask_with(
        mut self,
        share: &str,
        write: bool,
        pin: Option<&str>,
        to_person: impl FnOnce(),
    ) -> Result<LinkFs> {
        let addr = self.peer.addr;
        wire::send(
            &mut self.stream,
            &Ask {
                share: share.to_string(),
                write,
                pin: pin.is_some(),
            },
        )
        .await
        .map_err(|e| connect_error(addr, e))?;
        let proved = match pin {
            Some(pin) => self.prove(pin).await?,
            None => false,
        };
        if !proved {
            to_person();
        }
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

impl Pending {
    /// The caller's side of the PIN exchange. `false`: the machine has no PIN, and the call
    /// goes to its person.
    async fn prove(&mut self, typed: &str) -> Result<bool> {
        let addr = self.peer.addr;
        let err = |e: std::io::Error| connect_error(addr, e);
        let offer: PinOffer = timeout(Duration::from_secs(15), wire::recv(&mut self.stream))
            .await
            .map_err(|_| connect_error(addr, "no answer about the PIN"))?
            .map_err(err)?;
        if let Some(why) = offer.refused {
            return Err(connect_error(addr, why));
        }
        let Some(salt) = offer.salt.and_then(|s| pin::from_hex_vec(&s)) else {
            return Ok(false);
        };
        let typed = typed.to_string();
        // A second of arithmetic on a small machine: not on the thread that runs the rest.
        let stretched = tokio::task::spawn_blocking(move || pin::stretch(&typed, &salt))
            .await
            .map_err(|e| connect_error(addr, e))?;
        let exchange = Exchange::caller(&stretched, &self.own, &self.peer.fingerprint);
        wire::send(
            &mut self.stream,
            &PinMessage {
                message: pin::hex(&exchange.message),
            },
        )
        .await
        .map_err(err)?;
        let theirs: PinMessage = timeout(Duration::from_secs(30), wire::recv(&mut self.stream))
            .await
            .map_err(|_| connect_error(addr, "no PIN message"))?
            .map_err(err)?;
        let theirs = pin::from_hex_vec(&theirs.message)
            .ok_or_else(|| connect_error(addr, "a broken PIN exchange"))?;
        let proofs = exchange.finish(&theirs).map_err(err)?;
        wire::send(
            &mut self.stream,
            &PinProof {
                proof: pin::hex(&proofs.mine),
            },
        )
        .await
        .map_err(err)?;
        // Their proof, or the refusal a wrong PIN gets.
        let frame = timeout(Duration::from_secs(30), wire::read_frame(&mut self.stream))
            .await
            .map_err(|_| connect_error(addr, "no answer to the PIN"))?
            .map_err(err)?;
        let Frame::Json(body) = frame else {
            return Err(connect_error(addr, "bytes where an answer was expected"));
        };
        if let Ok(proof) = serde_json::from_slice::<PinProof>(&body) {
            let proof = pin::from_hex_vec(&proof.proof).unwrap_or_default();
            if !proofs.check(&proof) {
                return Err(connect_error(
                    addr,
                    "the other machine did not prove it holds the PIN — refusing to go on",
                ));
            }
            return Ok(true);
        }
        let answer: Answer = serde_json::from_slice(&body).map_err(|e| connect_error(addr, e))?;
        Err(connect_error(
            addr,
            answer.reason.unwrap_or_else(|| "refused".into()),
        ))
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
                        len: wire::READ_CHUNK as u32,
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
        let big: Vec<u8> = (0..(wire::READ_CHUNK * 3 + 1234))
            .map(|i| (i % 251) as u8)
            .collect();
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

    struct HangingGate {
        shares: Vec<Share>,
        hang: tokio::sync::watch::Sender<super::super::server::HangUp>,
        sessions: StdMutex<Vec<u64>>,
    }

    #[async_trait]
    impl Gate for HangingGate {
        fn shares(&self) -> Vec<Share> {
            self.shares.clone()
        }
        async fn admit(&self, caller: &Caller) -> Admission {
            self.sessions.lock().unwrap().push(caller.session);
            Admission::ReadOnly
        }
        fn hang_up(&self) -> Option<tokio::sync::watch::Receiver<super::super::server::HangUp>> {
            Some(self.hang.subscribe())
        }
    }

    #[tokio::test]
    async fn the_called_side_can_hang_up() {
        let dir = tree();
        let (hang, _) = tokio::sync::watch::channel(super::super::server::HangUp::default());
        let gate = Arc::new(HangingGate {
            shares: vec![Share::new("docs", &dir.path().join("shared"), false).unwrap()],
            hang,
            sessions: StdMutex::new(Vec::new()),
        });
        let them = Arc::new(Identity::generate("there").unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let g: Arc<dyn Gate> = gate.clone();
        tokio::spawn(serve(listener, them, g));
        let me = Identity::generate("here").unwrap();
        let fs = call(addr, &me).await.unwrap().ask("", false).await.unwrap();
        let other = call(addr, &me).await.unwrap().ask("", false).await.unwrap();
        assert!(fs.stat("/small.txt", true).await.is_ok());
        assert!(other.stat("/small.txt", true).await.is_ok());
        // One call ended by its number: the other goes on. Sessions are numbered in the
        // order they were let in, the first being the lower.
        let first = gate.sessions.lock().unwrap()[0];
        gate.hang.send_modify(|h| {
            h.generation += 1;
            h.only = Some(first);
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            fs.stat("/small.txt", true).await.is_err(),
            "that one is over"
        );
        assert!(
            other.stat("/small.txt", true).await.is_ok(),
            "the other is not"
        );
        // And all of them at once.
        gate.hang.send_modify(|h| {
            h.generation += 1;
            h.only = None;
        });
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(
            other.stat("/small.txt", true).await.is_err(),
            "the session is over"
        );
    }

    struct PinGate {
        shares: Vec<Share>,
        pin: Option<super::super::pin::Pin>,
        asked: StdMutex<usize>,
        refused: StdMutex<Vec<String>>,
    }

    #[async_trait]
    impl Gate for PinGate {
        fn shares(&self) -> Vec<Share> {
            self.shares.clone()
        }
        async fn admit(&self, _caller: &Caller) -> Admission {
            *self.asked.lock().unwrap() += 1;
            Admission::ReadOnly
        }
        fn pin(&self) -> Option<super::super::pin::Pin> {
            self.pin.clone()
        }
        fn refused(&self, _caller: &Caller, why: &str) {
            self.refused.lock().unwrap().push(why.to_string());
        }
    }

    async fn pin_server(
        root: &std::path::Path,
        pin: Option<&str>,
    ) -> (SocketAddr, Arc<PinGate>, Identity) {
        let gate = Arc::new(PinGate {
            shares: vec![Share::new("docs", root, false).unwrap()],
            pin: pin.map(|p| super::super::pin::Pin::new(p).unwrap()),
            asked: StdMutex::new(0),
            refused: StdMutex::new(Vec::new()),
        });
        let them = Arc::new(Identity::generate("box").unwrap());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let g: Arc<dyn Gate> = gate.clone();
        tokio::spawn(serve(listener, them, g));
        (addr, gate, Identity::generate("here").unwrap())
    }

    #[tokio::test]
    async fn the_right_pin_opens_without_the_person_and_a_wrong_one_does_not() {
        let dir = tree();
        let (addr, gate, me) = pin_server(&dir.path().join("shared"), Some("424242")).await;
        let mut person = false;
        let fs = call(addr, &me)
            .await
            .unwrap()
            .ask_with("", false, Some("424242"), || person = true)
            .await
            .unwrap();
        assert!(!person, "the PIN is the yes");
        assert_eq!(*gate.asked.lock().unwrap(), 0);
        assert!(!fs.write, "read only, as the PIN grants in this step");
        assert_eq!(fs.stat("/small.txt", true).await.unwrap().size, 5);

        let err = call(addr, &me)
            .await
            .unwrap()
            .ask_with("", false, Some("000000"), || {})
            .await
            .unwrap_err();
        assert!(err.to_string().contains("wrong PIN"), "{err}");
        assert_eq!(
            *gate.asked.lock().unwrap(),
            0,
            "a wrong PIN is not a question"
        );
        assert_eq!(gate.refused.lock().unwrap().as_slice(), ["wrong PIN"]);
    }

    #[tokio::test]
    async fn five_wrong_pins_lock_the_caller_out() {
        let dir = tree();
        let (addr, _gate, me) = pin_server(&dir.path().join("shared"), Some("424242")).await;
        for _ in 0..super::super::pin::MAX_WRONG {
            let err = call(addr, &me)
                .await
                .unwrap()
                .ask_with("", false, Some("111111"), || {})
                .await
                .unwrap_err();
            assert!(err.to_string().contains("wrong PIN"), "{err}");
        }
        let err = call(addr, &me)
            .await
            .unwrap()
            .ask_with("", false, Some("424242"), || {})
            .await
            .unwrap_err();
        assert!(err.to_string().contains("too many wrong PINs"), "{err}");
    }

    #[tokio::test]
    async fn a_pin_given_to_a_machine_without_one_goes_to_its_person() {
        let dir = tree();
        let (addr, gate, me) = pin_server(&dir.path().join("shared"), None).await;
        let mut person = false;
        call(addr, &me)
            .await
            .unwrap()
            .ask_with("", false, Some("424242"), || person = true)
            .await
            .unwrap();
        assert!(person);
        assert_eq!(*gate.asked.lock().unwrap(), 1);
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
