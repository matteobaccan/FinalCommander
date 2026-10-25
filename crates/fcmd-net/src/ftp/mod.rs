//! FTP and FTPS, as the netbox's second protocol.
//!
//! It implements the same [`RemoteFs`] trait as SFTP does, so everything above this line —
//! the panel, the viewer, the editor, the job engine, the synchroniser — works on an `ftp://`
//! path without knowing that it is one. What is different is what the protocol can do, and
//! that is said once, in [`crate::vfs::caps_of`], rather than discovered halfway through an
//! operation.
//!
//! Three decisions are worth reading before the code.
//!
//! **One connection for commands, a new one for each transfer.** FTP sends the bytes of a file
//! over a second socket, and a control connection can only have one of those open at a time.
//! Holding the control connection for the length of a download would mean a listing has to
//! wait for it — and, worse, a copy from one FTP directory to another on the same server would
//! deadlock against itself, because that reads and writes at the same time. So a transfer
//! dials its own connection and logs in again, which is what every graphical FTP client does,
//! and the pooled connection stays free to answer commands.
//!
//! **The password is kept in memory while the connection is open.** It has to be: a transfer
//! logs in again, and there is nobody to ask at that moment. It is never written anywhere, and
//! it goes when the connection is closed.
//!
//! **`MLSD` first, `LIST` second.** See [`list`] for why the difference matters so much.

#[cfg(test)]
pub mod fake;
pub mod list;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fcmd_core::{EntryKind, Scheme};
use suppaftp::tokio::{AsyncRustlsConnector, AsyncRustlsFtpStream};
use suppaftp::types::FileType;
use suppaftp::{FtpError, Status};
use tokio::sync::Mutex;

use crate::error::{NetError, Result, SecretKind};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};
use crate::remote::Remote;
use crate::site::{Auth, Site};

/// The client type: the TLS-capable one, used for plain FTP as well. A stream only becomes
/// encrypted when `AUTH TLS` is asked for, so one type serves both schemes and there is no
/// second code path to keep in step.
type Client = AsyncRustlsFtpStream;

/// How long to wait for the greeting before giving up on a host.
const DIAL_TIMEOUT: Duration = Duration::from_secs(20);

/// Open an FTP or FTPS connection and log in.
///
/// `secret` is the password. A login as `anonymous` or `ftp` does not need one — the
/// convention is an e-mail address and every server accepts anything — so those two names
/// connect with no prompt; any other user without a password asks for one rather than sending
/// an empty string and being refused.
pub async fn connect(site: &Site, secret: Option<&str>) -> Result<Remote> {
    if let Auth::Key { .. } = site.auth {
        return Err(NetError::Unsupported {
            path: site.url(),
            detail: "FTP has no key authentication; use a password or log in anonymously".into(),
        });
    }
    let password = match (anonymous(&site.user), secret) {
        (true, None) => "anonymous@".to_string(),
        (_, Some(secret)) => secret.to_string(),
        (false, None) => {
            return Err(NetError::NeedsSecret {
                user: site.user.clone(),
                host: site.host.clone(),
                kind: SecretKind::Password,
            })
        }
    };
    let mut client = dial(site, &password).await?;
    // What the server can do beyond RFC 959, asked once: `MLSD` for listings that carry a real
    // time, `MFMT` for setting one. A server that does not answer `FEAT` at all gets the
    // oldest behaviour, which is what it is telling us it wants.
    let features = match client.feat().await {
        Ok(features) => Features {
            mlsd: features.contains_key("MLSD"),
            mfmt: features.contains_key("MFMT"),
        },
        Err(_) => Features::default(),
    };
    let _ = client.noop().await;
    Ok(Remote::new(
        site.clone(),
        Arc::new(FtpBackend {
            site: site.clone(),
            password,
            features,
            control: Mutex::new(client),
        }),
    ))
}

fn anonymous(user: &str) -> bool {
    user.eq_ignore_ascii_case("anonymous") || user.eq_ignore_ascii_case("ftp")
}

/// Connect, secure the connection if the scheme asks for it, log in, and go binary.
async fn dial(site: &Site, password: &str) -> Result<Client> {
    let address = format!("{}:{}", site.host, site.port);
    let connect = Client::connect(address.as_str());
    let mut client = match tokio::time::timeout(DIAL_TIMEOUT, connect).await {
        Ok(Ok(client)) => client,
        Ok(Err(e)) => return Err(connect_error(site, e)),
        Err(_) => {
            return Err(NetError::Connect {
                host: address,
                detail: format!("no answer in {} seconds", DIAL_TIMEOUT.as_secs()),
            })
        }
    };
    if site.scheme == Scheme::Ftps {
        client = client
            .into_secure(tls_connector()?, &site.host)
            .await
            .map_err(|e| NetError::Connect {
                host: address.clone(),
                detail: format!("TLS: {e}"),
            })?;
    }
    client
        .login(site.user.as_str(), password)
        .await
        .map_err(|e| NetError::Auth {
            user: site.user.clone(),
            host: site.host.clone(),
            detail: reason(e),
        })?;
    // Without this, every `\n` in an uploaded file becomes `\r\n`: RFC 959's default transfer
    // type is ASCII, and a file manager moves files, not text.
    client
        .transfer_type(FileType::Binary)
        .await
        .map_err(|e| connect_error(site, e))?;
    Ok(client)
}

/// TLS for FTPS: rustls with the machine's own trust anchors.
///
/// The same reasoning as the object-storage client's: an FTPS server inside a company is very
/// often behind that company's certificate authority, and a bundled root list cannot be told
/// about it, while the machine already has been.
fn tls_connector() -> Result<AsyncRustlsConnector> {
    use suppaftp::tokio_rustls::rustls::{ClientConfig, RootCertStore};
    let mut roots = RootCertStore::empty();
    let found = rustls_native_certs::load_native_certs();
    for cert in found.certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err(NetError::Connect {
            host: "tls".into(),
            detail: "this machine has no certificate authorities to check a server against".into(),
        });
    }
    let provider = Arc::new(suppaftp::tokio_rustls::rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| NetError::Connect {
            host: "tls".into(),
            detail: e.to_string(),
        })?
        .with_root_certificates(roots)
        .with_no_client_auth();
    Ok(AsyncRustlsConnector::from(
        suppaftp::tokio_rustls::TlsConnector::from(Arc::new(config)),
    ))
}

/// What `FEAT` said the server can do beyond the 1985 protocol.
#[derive(Debug, Clone, Copy, Default)]
struct Features {
    mlsd: bool,
    mfmt: bool,
}

/// One live FTP connection: the control socket, and what it takes to open another.
struct FtpBackend {
    site: Site,
    /// Held so a transfer can log in again. Never written anywhere; gone when this is dropped.
    password: String,
    features: Features,
    control: Mutex<Client>,
}

impl std::fmt::Debug for FtpBackend {
    /// By hand, so that no derive can ever print the password.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FtpBackend")
            .field("site", &self.site.url())
            .field("features", &self.features)
            .finish_non_exhaustive()
    }
}

impl FtpBackend {
    /// A connection of its own for one transfer. See the module's note on why.
    async fn transfer_connection(&self) -> Result<Client> {
        dial(&self.site, &self.password).await
    }

    /// Every entry of a directory, by whichever listing command the server has.
    async fn entries(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let mut client = self.control.lock().await;
        if self.features.mlsd {
            let lines = client
                .mlsd(Some(path))
                .await
                .map_err(|e| remote(path, &e))?;
            return Ok(lines.iter().filter_map(|l| list::mlsd_line(l)).collect());
        }
        let lines = client
            .list(Some(path))
            .await
            .map_err(|e| remote(path, &e))?;
        Ok(lines.iter().filter_map(|l| list::list_line(l)).collect())
    }
}

#[async_trait]
impl RemoteFs for FtpBackend {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        self.entries(path).await
    }

    /// FTP has no `stat`. `MLST` is one round trip and the right answer where the server has
    /// it; everywhere else the parent directory is listed and the name looked for in it, which
    /// is what every FTP client has always done.
    ///
    /// `follow` is ignored: nothing in FTP resolves a symlink on request, and a listing says
    /// what it says.
    async fn stat(&self, path: &str, _follow: bool) -> Result<RemoteEntry> {
        let path = path.trim_end_matches('/');
        if path.is_empty() {
            return Ok(RemoteEntry {
                name: "/".to_string(),
                kind: EntryKind::Dir,
                size: 0,
                mtime: None,
                mode: None,
                link_target: None,
            });
        }
        if self.features.mlsd {
            let mut client = self.control.lock().await;
            if let Ok(line) = client.mlst(Some(path)).await {
                if let Some(mut entry) = list::mlsd_line(line.trim()) {
                    // `MLST` answers with the path that was asked for, not with a bare name.
                    entry.name = leaf(path).to_string();
                    return Ok(entry);
                }
            }
        }
        let name = leaf(path);
        let parent = match path.rfind('/') {
            Some(0) | None => "/",
            Some(cut) => &path[..cut],
        };
        self.entries(parent)
            .await?
            .into_iter()
            .find(|e| e.name == name)
            .ok_or_else(|| remote(path, &"no such file or directory"))
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let mut client = self.control.lock().await;
        client.mkdir(path).await.map_err(|e| remote(path, &e))
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        let mut client = self.control.lock().await;
        client.rm(path).await.map_err(|e| remote(path, &e))
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        let mut client = self.control.lock().await;
        client.rmdir(path).await.map_err(|e| remote(path, &e))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let mut client = self.control.lock().await;
        client.rename(from, to).await.map_err(|e| remote(from, &e))
    }

    /// A listing may say what a link points at; nothing in FTP will say it on request.
    async fn read_link(&self, path: &str) -> Result<String> {
        match self.stat(path, false).await?.link_target {
            Some(target) => Ok(target),
            None => Err(NetError::Unsupported {
                path: path.to_string(),
                detail: "FTP does not say where a link points".into(),
            }),
        }
    }

    async fn symlink(&self, _target: &str, link: &str) -> Result<()> {
        Err(NetError::Unsupported {
            path: link.to_string(),
            detail: "FTP cannot make a link".into(),
        })
    }

    /// The mode through `SITE CHMOD` and the time through `MFMT`, which is what the servers
    /// that can do either of them use. A server that refuses says so in its own words, and the
    /// message carries that rather than hiding it: a copy that could not carry a timestamp
    /// across should say which of the two it was.
    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()> {
        let mut client = self.control.lock().await;
        if let Some(mode) = mode {
            client
                .site(format!("CHMOD {:03o} {path}", mode & 0o7777))
                .await
                .map_err(|e| remote(path, &format!("SITE CHMOD: {}", reason(e))))?;
        }
        if let Some(mtime) = mtime {
            if !self.features.mfmt {
                return Err(NetError::Unsupported {
                    path: path.to_string(),
                    detail: "this server cannot set a file's time (no MFMT)".into(),
                });
            }
            let stamp = crate::clock::compact_stamp(mtime);
            client
                .custom_command(
                    format!("MFMT {stamp} {path}"),
                    // 213 is what RFC 3659 says; a few servers answer 200 or 250 instead.
                    &[
                        Status::File,
                        Status::CommandOk,
                        Status::RequestedFileActionOk,
                    ],
                )
                .await
                .map_err(|e| remote(path, &format!("MFMT: {}", reason(e))))?;
        }
        Ok(())
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let mut client = self.transfer_connection().await?;
        let stream = client
            .retr_as_stream(path)
            .await
            .map_err(|e| remote(path, &e))?;
        Ok(Box::new(transfer::Download {
            stream,
            _client: client,
        }))
    }

    async fn create(&self, path: &str) -> Result<Writer> {
        let mut client = self.transfer_connection().await?;
        let stream = client
            .put_with_stream(path)
            .await
            .map_err(|e| remote(path, &e))?;
        Ok(Box::new(transfer::Upload::new(stream, client)))
    }

    /// `PWD`, after a `CWD` when a path was named. This is how a site that starts at `.` or at
    /// a relative directory finds out where that is.
    async fn canonicalize(&self, path: &str) -> Result<String> {
        let mut client = self.control.lock().await;
        let wanted = path.trim();
        if !matches!(wanted, "" | "." | "~") {
            client.cwd(wanted).await.map_err(|e| remote(path, &e))?;
        }
        let here = client.pwd().await.map_err(|e| remote(path, &e))?;
        Ok(if here.starts_with('/') {
            here
        } else {
            format!("/{here}")
        })
    }

    async fn close(&self) -> Result<()> {
        let mut client = self.control.lock().await;
        // `QUIT` is a courtesy: the connection is going either way, so a server that has
        // already hung up is not an error worth reporting.
        let _ = client.quit().await;
        Ok(())
    }
}

fn leaf(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

fn remote(path: &str, detail: &dyn std::fmt::Display) -> NetError {
    NetError::Remote {
        path: path.to_string(),
        detail: detail.to_string(),
    }
}

fn connect_error(site: &Site, e: FtpError) -> NetError {
    NetError::Connect {
        host: format!("{}:{}", site.host, site.port),
        detail: reason(e),
    }
}

/// An FTP error in the words the server used, where there are any. A reply code on its own
/// ("550") means nothing to the person reading the status line; the text beside it does.
fn reason(e: FtpError) -> String {
    match e {
        FtpError::UnexpectedResponse(response) => {
            let body = String::from_utf8_lossy(&response.body).trim().to_string();
            if body.is_empty() {
                format!("the server answered {}", response.status as u32)
            } else {
                body
            }
        }
        other => other.to_string(),
    }
}

mod transfer {
    //! The two ends of a transfer, each owning the connection it runs on.

    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use suppaftp::tokio::TransferStream;
    use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

    use super::Client;

    type Stream = TransferStream<suppaftp::tokio::AsyncRustlsStream>;

    /// A file coming off the server. The client is held so the connection lives as long as the
    /// reader and is closed with it; the bytes have all arrived by then, so the server's
    /// closing reply says nothing we do not already know.
    pub struct Download {
        pub stream: Stream,
        pub _client: Client,
    }

    impl AsyncRead for Download {
        fn poll_read(
            mut self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            Pin::new(&mut self.stream).poll_read(cx, buf)
        }
    }

    /// A file going to the server.
    ///
    /// Closing it is the part that matters. An upload is not done when the last byte is
    /// written: the data connection has to be shut, and the server then answers on the control
    /// connection with whether it stored the file. So `poll_shutdown` runs the whole of that —
    /// `TransferStream::finish`, which does both — and reports a refusal as the error it is.
    /// Dropping the writer without closing it would lose that answer, which is exactly how an
    /// upload comes to look successful and leave nothing on the server.
    pub struct Upload {
        state: State,
    }

    enum State {
        // Boxed because a client is much bigger than the future that replaces it, and this
        // enum is the writer the job engine holds for the length of an upload.
        Open(Stream, Box<Client>),
        Finishing(Pin<Box<dyn Future<Output = io::Result<()>> + Send>>),
        Done,
    }

    impl Upload {
        pub fn new(stream: Stream, client: Client) -> Upload {
            Upload {
                state: State::Open(stream, Box::new(client)),
            }
        }
    }

    impl AsyncWrite for Upload {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            data: &[u8],
        ) -> Poll<io::Result<usize>> {
            match &mut self.get_mut().state {
                State::Open(stream, _) => Pin::new(stream).poll_write(cx, data),
                _ => Poll::Ready(Err(closed())),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match &mut self.get_mut().state {
                State::Open(stream, _) => Pin::new(stream).poll_flush(cx),
                _ => Poll::Ready(Ok(())),
            }
        }

        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let me = self.get_mut();
            loop {
                match std::mem::replace(&mut me.state, State::Done) {
                    State::Open(stream, client) => {
                        me.state = State::Finishing(Box::pin(async move {
                            let outcome = stream.finish().await;
                            // The connection was dialled for this one transfer; it goes now,
                            // whatever the server said about the file.
                            let mut client = client;
                            let _ = client.quit().await;
                            outcome.map_err(|e| io::Error::other(super::reason(e)))
                        }));
                    }
                    State::Finishing(mut work) => match work.as_mut().poll(cx) {
                        Poll::Ready(done) => {
                            me.state = State::Done;
                            return Poll::Ready(done);
                        }
                        Poll::Pending => {
                            me.state = State::Finishing(work);
                            return Poll::Pending;
                        }
                    },
                    State::Done => return Poll::Ready(Ok(())),
                }
            }
        }
    }

    fn closed() -> io::Error {
        io::Error::new(io::ErrorKind::BrokenPipe, "the upload is already closed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_goes_out_in_the_form_mfmt_takes_and_comes_back_the_same() {
        // The two directions are each other's inverse, which is the only property that
        // matters: a time set on a file has to read back as the time that was set.
        for secs in [0u32, 1_767_268_800, 1_000_000_000, 2_000_000_000] {
            let stamp = crate::clock::compact_stamp(secs);
            assert_eq!(stamp.len(), 14, "{stamp}");
            assert_eq!(list::mlsd_time(&stamp), Some(secs), "{stamp}");
        }
        assert_eq!(crate::clock::compact_stamp(1_767_268_800), "20260101120000");
        assert_eq!(crate::clock::compact_stamp(0), "19700101000000");
    }

    #[test]
    fn only_the_two_anonymous_names_log_in_without_a_password() {
        assert!(anonymous("anonymous"));
        assert!(anonymous("ANONYMOUS"));
        assert!(anonymous("ftp"));
        assert!(!anonymous("mb"));
        assert!(!anonymous(""));
    }

    #[tokio::test]
    async fn a_key_is_not_something_ftp_can_be_given() {
        let mut site = Site::parse_url("ftp://mb@host/").unwrap();
        site.auth = Auth::Key {
            path: "/home/mb/.ssh/id_ed25519".into(),
        };
        match connect(&site, None).await {
            Err(NetError::Unsupported { detail, .. }) => {
                assert!(detail.contains("key authentication"), "{detail}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_named_user_with_no_password_asks_for_one_rather_than_trying_an_empty_string() {
        let site = Site::parse_url("ftp://mb@host/").unwrap();
        match connect(&site, None).await {
            Err(NetError::NeedsSecret { kind, user, .. }) => {
                assert_eq!(kind, SecretKind::Password);
                assert_eq!(user, "mb");
            }
            other => panic!("expected a request for a password, got {other:?}"),
        }
    }
    /// Everything below talks to [`fake::FakeFtp`], an FTP server in this process: the client
    /// code, the listing parsers and the transfer connections are all exercised over a real
    /// socket, which is the only way the awkward parts of this protocol can be tested at all.
    async fn connected(modern: bool) -> (fake::FakeFtp, Remote) {
        let server = fake::FakeFtp::start(modern)
            .await
            .with_dir("/srv")
            .with_file("/srv/readme.md", b"# the readme\n")
            .with_dir("/srv/pictures");
        let site = Site::parse_url(&server.url()).expect("the url");
        let remote = connect(&site, Some("secret")).await.expect("a connection");
        (server, remote)
    }

    #[tokio::test]
    async fn a_directory_is_listed_whichever_command_the_server_has() {
        // The same tree, read through MLSD and through LIST, has to come out the same except
        // for the time — which only MLSD carries, and which is the whole reason it is asked
        // for first.
        for modern in [true, false] {
            let (_server, remote) = connected(modern).await;
            let dir = remote.site().root().join("srv");
            let rows = remote
                .list(&dir, &fcmd_core::CancelToken::new())
                .await
                .unwrap_or_else(|e| panic!("modern={modern}: {e}"));
            let names: Vec<String> = rows.iter().map(|e| e.name_str().into_owned()).collect();
            assert_eq!(names, ["..", "pictures", "readme.md"], "modern={modern}");

            let readme = rows.iter().find(|e| e.name_str() == "readme.md").unwrap();
            assert_eq!(readme.size, 13, "modern={modern}");
            assert_eq!(readme.kind, EntryKind::File);
            let pictures = rows.iter().find(|e| e.name_str() == "pictures").unwrap();
            assert_eq!(pictures.kind, EntryKind::Dir);
            assert_eq!(
                readme.mtime.is_some(),
                modern,
                "a time comes from MLSD and from nowhere else"
            );
        }
    }

    #[tokio::test]
    async fn a_file_travels_both_ways_over_its_own_connection() {
        let dir = tempfile::tempdir().unwrap();
        let (server, remote) = connected(true).await;
        let mut nothing = |_: u64| true;

        // Down, with the bytes landing on this machine untouched.
        let local = dir.path().join("readme.md");
        let got = remote
            .download(
                &remote.site().root().join("srv").join("readme.md"),
                &local,
                &mut nothing,
            )
            .await
            .expect("the download");
        assert_eq!(got, 13);
        assert_eq!(std::fs::read(&local).unwrap(), b"# the readme\n");

        // And up. The upload is only finished when the server has said so, which is what the
        // writer's own shutdown waits for — a `\n` that came back as `\r\n` would mean the
        // transfer type was never set to binary.
        let source = dir.path().join("notes.txt");
        std::fs::write(&source, b"one\ntwo\n").unwrap();
        let sent = remote
            .upload(
                &source,
                &remote.site().root().join("srv").join("notes.txt"),
                &mut nothing,
            )
            .await
            .expect("the upload");
        assert_eq!(sent, 8);
        assert_eq!(
            server.file("/srv/notes.txt").as_deref(),
            Some(&b"one\ntwo\n"[..])
        );

        // A listing during a transfer is the case the second connection exists for: the
        // control connection was never held by the download above.
        let rows = remote
            .list(
                &remote.site().root().join("srv"),
                &fcmd_core::CancelToken::new(),
            )
            .await
            .expect("a listing");
        assert!(rows.iter().any(|e| e.name_str() == "notes.txt"));
    }

    #[tokio::test]
    async fn the_things_a_panel_does_to_a_remote_directory_all_work() {
        let (server, remote) = connected(true).await;
        let srv = remote.site().root().join("srv");

        remote.create_dir(&srv.join("new")).await.expect("mkdir");
        assert!(server.has("/srv/new"));

        remote
            .rename(&srv.join("readme.md"), &srv.join("README.txt"))
            .await
            .expect("rename");
        assert!(!server.has("/srv/readme.md"));
        assert_eq!(
            server.file("/srv/README.txt").as_deref(),
            Some(&b"# the readme\n"[..])
        );

        // `stat` goes through MLST where the server has it, and through the parent listing
        // where it does not; both have to find the same file.
        let facts = remote
            .facts(&srv.join("README.txt"), false)
            .await
            .expect("stat");
        assert_eq!(facts.size, 13);
        assert_eq!(facts.kind, EntryKind::File);
        assert!(remote.exists(&srv.join("README.txt")).await);
        assert!(!remote.exists(&srv.join("gone.txt")).await);

        remote
            .remove_file(&srv.join("README.txt"))
            .await
            .expect("delete");
        assert!(!server.has("/srv/README.txt"));
        remote.remove_dir(&srv.join("new")).await.expect("rmdir");
        assert!(!server.has("/srv/new"));

        // The start directory is resolved by the server, which is how a site that says `.`
        // finds out where it landed.
        assert_eq!(
            remote.canonicalize(&srv).await.expect("pwd").remote_path(),
            "/srv"
        );
    }

    #[tokio::test]
    async fn a_time_and_a_mode_are_carried_over_where_the_server_can_take_them() {
        let (server, remote) = connected(true).await;
        let path = remote.site().root().join("srv").join("readme.md");
        let facts = fcmd_jobs::Facts {
            kind: EntryKind::File,
            size: 13,
            mtime: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_767_268_800)),
            mode: Some(0o640),
            readonly: false,
        };
        remote
            .apply_meta(&path, &facts)
            .await
            .expect("the metadata");
        let stamps = server.stamps.lock().unwrap().clone();
        assert_eq!(
            stamps,
            vec![("/srv/readme.md".to_string(), "20260101120000".to_string())]
        );

        // A server with no MFMT says so instead of pretending the time was set.
        let (_old, old_remote) = connected(false).await;
        let path = old_remote.site().root().join("srv").join("readme.md");
        match old_remote.apply_meta(&path, &facts).await {
            Err(NetError::Unsupported { detail, .. }) => {
                assert!(detail.contains("MFMT"), "{detail}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn what_is_not_there_is_said_in_the_servers_own_words() {
        let (_server, remote) = connected(true).await;
        let missing = remote.site().root().join("srv").join("nowhere.txt");
        let error = remote
            .facts(&missing, false)
            .await
            .expect_err("it is not there");
        let text = error.to_string();
        assert!(text.contains("nowhere.txt"), "{text}");
        assert!(
            text.contains("no such file") || text.contains("550"),
            "the server's reason is missing: {text}"
        );
    }
}
