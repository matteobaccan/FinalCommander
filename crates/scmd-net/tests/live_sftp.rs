//! The netbox against a real SSH server.
//!
//! Everything above `crate::ssh` is tested with the in-memory fake; this test is the other
//! half: an SSH server with an SFTP subsystem, in process, on a loopback port, so the real
//! `russh` client path — key exchange, host-key check, authentication, subsystem, and every
//! SFTP call the provider makes — runs end to end against a temporary directory.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use russh::server::{Auth, ChannelOpenHandle, Msg, Server as _, Session};
use russh::{Channel, ChannelId};
use russh_sftp::protocol::{
    Attrs, Data, File, FileAttributes, Handle, Name, OpenFlags, Status, StatusCode, Version,
};
use scmd_core::{CancelToken, VPath};
use scmd_net::site::Auth as SiteAuth;
use scmd_net::{HostKeyPolicy, Site};
use tokio::sync::Mutex;

const PASSWORD: &str = "correct horse";

// ── the server ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone)]
struct Server {
    root: PathBuf,
}

impl russh::server::Server for Server {
    type Handler = SshSession;

    fn new_client(&mut self, _: Option<std::net::SocketAddr>) -> SshSession {
        SshSession {
            root: self.root.clone(),
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

struct SshSession {
    root: PathBuf,
    channels: Arc<Mutex<HashMap<ChannelId, Channel<Msg>>>>,
}

impl russh::server::Handler for SshSession {
    type Error = russh::Error;

    async fn auth_password(&mut self, _user: &str, password: &str) -> Result<Auth, Self::Error> {
        Ok(if password == PASSWORD {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.lock().await.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    /// The server end of a tunnel: connect where the client asked and pump the bytes.
    ///
    /// This is what an SSH server does for `ssh -L`, in about as many lines as it takes to
    /// say it — which is the point of testing the client against it rather than against a
    /// mock: the channel, the window, the half-close and the copying are all real.
    async fn channel_open_direct_tcpip(
        &mut self,
        channel: Channel<Msg>,
        host_to_connect: &str,
        port_to_connect: u32,
        _originator_address: &str,
        _originator_port: u32,
        reply: ChannelOpenHandle,
        _session: &mut Session,
    ) -> Result<(), Self::Error> {
        let target = format!("{host_to_connect}:{port_to_connect}");
        match tokio::net::TcpStream::connect(&target).await {
            Ok(mut socket) => {
                reply.accept().await;
                tokio::spawn(async move {
                    let mut stream = channel.into_stream();
                    let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
                });
            }
            // Nothing listening there: the channel is refused, which is what the client has
            // to survive without losing the session.
            Err(_) => drop(reply),
        }
        Ok(())
    }

    async fn subsystem_request(
        &mut self,
        channel_id: ChannelId,
        name: &str,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        if name != "sftp" {
            session.channel_failure(channel_id)?;
            return Ok(());
        }
        let channel = self
            .channels
            .lock()
            .await
            .remove(&channel_id)
            .expect("the channel was opened");
        session.channel_success(channel_id)?;
        let sftp = SftpServer {
            root: self.root.clone(),
            dirs: HashMap::new(),
            files: HashMap::new(),
            next: 0,
        };
        russh_sftp::server::run(channel.into_stream(), sftp).await;
        Ok(())
    }
}

/// An SFTP server over a real directory. Small, but it answers every call the provider makes.
struct SftpServer {
    root: PathBuf,
    /// Open directory handles: the names still to send, so the second `readdir` can say EOF.
    dirs: HashMap<String, Vec<File>>,
    /// Open file handles: the real path, and whether it was opened for writing.
    files: HashMap<String, (PathBuf, bool)>,
    next: u64,
}

impl SftpServer {
    /// Map a client path onto the temporary directory, refusing anything that climbs out.
    fn real(&self, path: &str) -> Result<PathBuf, StatusCode> {
        let mut out = self.root.clone();
        for part in path.split('/') {
            match part {
                "" | "." => {}
                ".." => return Err(StatusCode::PermissionDenied),
                other => out.push(other),
            }
        }
        Ok(out)
    }

    fn handle(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("{prefix}{}", self.next)
    }

    fn ok(id: u32) -> Status {
        Status {
            id,
            status_code: StatusCode::Ok,
            error_message: "Ok".into(),
            language_tag: "en-US".into(),
        }
    }
}

fn attrs_of(meta: &std::fs::Metadata) -> FileAttributes {
    let mut attrs = FileAttributes {
        size: Some(meta.len()),
        ..FileAttributes::default()
    };
    if meta.is_dir() {
        attrs.set_dir(true);
        attrs.permissions = Some(attrs.permissions.unwrap_or(0) | 0o755);
    } else {
        attrs.set_regular(true);
        attrs.permissions = Some(attrs.permissions.unwrap_or(0) | 0o644);
    }
    attrs.mtime = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as u32);
    attrs
}

fn io_status(e: &std::io::Error) -> StatusCode {
    match e.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NoSuchFile,
        std::io::ErrorKind::PermissionDenied => StatusCode::PermissionDenied,
        _ => StatusCode::Failure,
    }
}

impl russh_sftp::server::Handler for SftpServer {
    type Error = StatusCode;

    fn unimplemented(&self) -> StatusCode {
        StatusCode::OpUnsupported
    }

    async fn init(
        &mut self,
        _version: u32,
        _extensions: HashMap<String, String>,
    ) -> Result<Version, StatusCode> {
        Ok(Version::new())
    }

    async fn realpath(&mut self, id: u32, path: String) -> Result<Name, StatusCode> {
        let cleaned = if path.trim().is_empty() || path == "." {
            "/".to_string()
        } else {
            path
        };
        Ok(Name {
            id,
            files: vec![File::dummy(cleaned)],
        })
    }

    async fn stat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
        let real = self.real(&path)?;
        let meta = std::fs::metadata(&real).map_err(|e| io_status(&e))?;
        Ok(Attrs {
            id,
            attrs: attrs_of(&meta),
        })
    }

    async fn lstat(&mut self, id: u32, path: String) -> Result<Attrs, StatusCode> {
        let real = self.real(&path)?;
        let meta = std::fs::symlink_metadata(&real).map_err(|e| io_status(&e))?;
        Ok(Attrs {
            id,
            attrs: attrs_of(&meta),
        })
    }

    async fn opendir(&mut self, id: u32, path: String) -> Result<Handle, StatusCode> {
        let real = self.real(&path)?;
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&real).map_err(|e| io_status(&e))? {
            let entry = entry.map_err(|e| io_status(&e))?;
            let meta = entry.metadata().map_err(|e| io_status(&e))?;
            files.push(File {
                filename: entry.file_name().to_string_lossy().into_owned(),
                longname: entry.file_name().to_string_lossy().into_owned(),
                attrs: attrs_of(&meta),
            });
        }
        let handle = self.handle("dir");
        self.dirs.insert(handle.clone(), files);
        Ok(Handle { id, handle })
    }

    async fn readdir(&mut self, id: u32, handle: String) -> Result<Name, StatusCode> {
        match self.dirs.get_mut(&handle) {
            // The whole directory in one packet, then EOF, which is what the protocol expects.
            Some(files) if !files.is_empty() => Ok(Name {
                id,
                files: std::mem::take(files),
            }),
            Some(_) => Err(StatusCode::Eof),
            None => Err(StatusCode::NoSuchFile),
        }
    }

    async fn open(
        &mut self,
        id: u32,
        filename: String,
        pflags: OpenFlags,
        _attrs: FileAttributes,
    ) -> Result<Handle, StatusCode> {
        let real = self.real(&filename)?;
        let writing = pflags.contains(OpenFlags::WRITE);
        if writing {
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(pflags.contains(OpenFlags::TRUNCATE))
                .open(&real)
                .map_err(|e| io_status(&e))?;
        } else if !real.is_file() {
            return Err(StatusCode::NoSuchFile);
        }
        let handle = self.handle("file");
        self.files.insert(handle.clone(), (real, writing));
        Ok(Handle { id, handle })
    }

    async fn read(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        len: u32,
    ) -> Result<Data, StatusCode> {
        let (path, _) = self.files.get(&handle).ok_or(StatusCode::NoSuchFile)?;
        let bytes = std::fs::read(path).map_err(|e| io_status(&e))?;
        let from = offset.min(bytes.len() as u64) as usize;
        let to = (from + len as usize).min(bytes.len());
        if from >= bytes.len() {
            return Err(StatusCode::Eof);
        }
        Ok(Data {
            id,
            data: bytes[from..to].to_vec(),
        })
    }

    async fn write(
        &mut self,
        id: u32,
        handle: String,
        offset: u64,
        data: Vec<u8>,
    ) -> Result<Status, StatusCode> {
        use std::io::{Seek, SeekFrom, Write};
        let (path, _) = self.files.get(&handle).ok_or(StatusCode::NoSuchFile)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .map_err(|e| io_status(&e))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| io_status(&e))?;
        file.write_all(&data).map_err(|e| io_status(&e))?;
        Ok(SftpServer::ok(id))
    }

    async fn close(&mut self, id: u32, handle: String) -> Result<Status, StatusCode> {
        self.dirs.remove(&handle);
        self.files.remove(&handle);
        Ok(SftpServer::ok(id))
    }

    async fn mkdir(
        &mut self,
        id: u32,
        path: String,
        _attrs: FileAttributes,
    ) -> Result<Status, StatusCode> {
        let real = self.real(&path)?;
        std::fs::create_dir(&real).map_err(|e| io_status(&e))?;
        Ok(SftpServer::ok(id))
    }

    async fn rmdir(&mut self, id: u32, path: String) -> Result<Status, StatusCode> {
        let real = self.real(&path)?;
        std::fs::remove_dir(&real).map_err(|e| io_status(&e))?;
        Ok(SftpServer::ok(id))
    }

    async fn remove(&mut self, id: u32, filename: String) -> Result<Status, StatusCode> {
        let real = self.real(&filename)?;
        std::fs::remove_file(&real).map_err(|e| io_status(&e))?;
        Ok(SftpServer::ok(id))
    }

    async fn rename(
        &mut self,
        id: u32,
        oldpath: String,
        newpath: String,
    ) -> Result<Status, StatusCode> {
        let (from, to) = (self.real(&oldpath)?, self.real(&newpath)?);
        std::fs::rename(&from, &to).map_err(|e| io_status(&e))?;
        Ok(SftpServer::ok(id))
    }

    async fn setstat(
        &mut self,
        id: u32,
        path: String,
        attrs: FileAttributes,
    ) -> Result<Status, StatusCode> {
        let real = self.real(&path)?;
        #[cfg(unix)]
        if let Some(mode) = attrs.permissions {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&real, std::fs::Permissions::from_mode(mode & 0o7777))
                .map_err(|e| io_status(&e))?;
        }
        let _ = (&real, attrs);
        Ok(SftpServer::ok(id))
    }
}

/// Start the server on a loopback port and return it.
async fn start(root: &Path) -> u16 {
    let config = russh::server::Config {
        auth_rejection_time: Duration::from_millis(1),
        inactivity_timeout: Some(Duration::from_secs(60)),
        keys: vec![russh::keys::PrivateKey::random(
            &mut rand::rng(),
            russh::keys::Algorithm::Ed25519,
        )
        .expect("a throwaway host key")],
        ..russh::server::Config::default()
    };
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("a loopback port");
    let port = listener.local_addr().unwrap().port();
    let mut server = Server {
        root: root.to_path_buf(),
    };
    tokio::spawn(async move {
        let _ = server.run_on_socket(Arc::new(config), &listener).await;
    });
    port
}

// ── the test ───────────────────────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn the_provider_works_against_a_real_ssh_server() {
    let server_root = tempfile::tempdir().unwrap();
    std::fs::create_dir(server_root.path().join("srv")).unwrap();
    std::fs::write(
        server_root.path().join("srv/hello.txt"),
        b"hello from ssh\n",
    )
    .unwrap();
    let port = start(server_root.path()).await;

    let local = tempfile::tempdir().unwrap();
    let known_hosts = local.path().join("known_hosts");

    let mut site = Site::parse_url(&format!("sftp://mb@127.0.0.1:{port}/srv")).unwrap();
    site.auth = SiteAuth::Password;

    // An unknown host key is refused until someone decides, and the fingerprint is in the
    // error so the user can check it.
    let strict = HostKeyPolicy {
        known_hosts: Some(known_hosts.clone()),
        accept_new: false,
    };
    let err = scmd_net::connect(&site, Some(PASSWORD), &strict)
        .await
        .expect_err("an unknown host key must not connect");
    assert!(
        matches!(err, scmd_net::NetError::UnknownHostKey { .. }),
        "{err}"
    );
    assert!(err.to_string().contains("SHA256:"), "{err}");

    // A wrong password fails with the user and host named.
    let tofu = HostKeyPolicy {
        known_hosts: Some(known_hosts.clone()),
        accept_new: true,
    };
    let err = scmd_net::connect(&site, Some("wrong"), &tofu)
        .await
        .expect_err("a wrong password must not connect");
    assert!(err.to_string().contains("mb@127.0.0.1"), "{err}");

    // Accepting the key records it, so the next connection is no longer "unknown".
    let remote = scmd_net::connect(&site, Some(PASSWORD), &tofu)
        .await
        .expect("the connection should be up");
    let recorded = std::fs::read_to_string(&known_hosts).unwrap();
    assert!(recorded.contains("127.0.0.1"), "{recorded}");

    let root = site.start();
    let path = |name: &str| root.join(name);

    // Listing: the `..` entry, then what the server has.
    let listing = remote.list(&root, &CancelToken::new()).await.unwrap();
    let names: Vec<String> = listing.iter().map(|e| e.name_str().into_owned()).collect();
    assert_eq!(names, ["..", "hello.txt"]);
    assert_eq!(listing[1].size, 15);

    // Download, upload, and a server-side copy.
    let mut ignore = |_: u64| true;
    let downloaded = local.path().join("hello.txt");
    let moved = remote
        .download(&path("hello.txt"), &downloaded, &mut ignore)
        .await
        .unwrap();
    assert_eq!(moved, 15);
    assert_eq!(
        std::fs::read_to_string(&downloaded).unwrap(),
        "hello from ssh\n"
    );

    let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    let big = local.path().join("big.bin");
    std::fs::write(&big, &payload).unwrap();
    let mut seen = 0u64;
    let mut count = |done: u64| {
        seen = done;
        true
    };
    remote
        .upload(&big, &path("big.bin"), &mut count)
        .await
        .unwrap();
    assert_eq!(seen, payload.len() as u64, "progress stopped early");
    assert_eq!(
        std::fs::read(server_root.path().join("srv/big.bin")).unwrap(),
        payload
    );

    remote
        .copy_within(&path("big.bin"), &path("copy.bin"), &mut ignore)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(server_root.path().join("srv/copy.bin")).unwrap(),
        payload
    );

    // Directories, renames, metadata and deletes.
    remote.create_dir_all(&path("a/b/c")).await.unwrap();
    assert!(server_root.path().join("srv/a/b/c").is_dir());
    remote
        .rename(&path("hello.txt"), &path("a/b/c/renamed.txt"))
        .await
        .unwrap();
    assert!(server_root.path().join("srv/a/b/c/renamed.txt").is_file());

    let facts = remote
        .facts(&path("a/b/c/renamed.txt"), false)
        .await
        .unwrap();
    assert_eq!(facts.size, 15);
    assert_eq!(facts.kind, scmd_core::EntryKind::File);
    let mut read_only = facts.clone();
    read_only.mode = Some(0o400);
    remote
        .apply_meta(&path("a/b/c/renamed.txt"), &read_only)
        .await
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(server_root.path().join("srv/a/b/c/renamed.txt"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o400);
    }

    remote.remove_file(&path("copy.bin")).await.unwrap();
    assert!(!server_root.path().join("srv/copy.bin").exists());
    assert!(
        remote.remove_dir(&path("a/b/c")).await.is_err(),
        "a directory with a file in it must not be removed"
    );
    remote
        .remove_file(&path("a/b/c/renamed.txt"))
        .await
        .unwrap();
    remote.remove_dir(&path("a/b/c")).await.unwrap();
    assert!(!server_root.path().join("srv/a/b/c").exists());

    // The server's own refusal arrives with the path.
    let err = remote.stat(&path("nothing-here")).await.unwrap_err();
    assert!(err.to_string().contains("nothing-here"), "{err}");

    // The write-back path `F4` uses: fetch a file, change the copy, put it back in place.
    std::fs::write(server_root.path().join("srv/edit.txt"), b"before\n").unwrap();
    let edited = remote
        .fetch_to(&path("edit.txt"), local.path(), &CancelToken::new())
        .await
        .unwrap();
    assert_eq!(std::fs::read_to_string(&edited).unwrap(), "before\n");
    std::fs::write(&edited, b"edited on the client\n").unwrap();
    remote
        .upload_in_place(&edited, &path("edit.txt"), &mut ignore)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(server_root.path().join("srv/edit.txt")).unwrap(),
        "edited on the client\n"
    );
    assert!(
        !server_root.path().join("srv/edit.txt.scmd-part").exists(),
        "the part file was left on the server"
    );

    // `fetch_to`, which is what `F3` uses.
    let fetched = remote
        .fetch_to(&path("big.bin"), local.path(), &CancelToken::new())
        .await
        .unwrap();
    assert_eq!(std::fs::read(&fetched).unwrap(), payload);

    // And a path from somewhere else is still refused, even with a live connection.
    assert!(remote.stat(&VPath::local("/etc/hosts")).await.is_err());

    remote.close().await.unwrap();
}

/// A tunnel over the same session the panel uses: `ssh -L`, without the ssh.
///
/// There is a real TCP server at the far end, a real SSH server in the middle and a real
/// client socket at this end, so what is tested is the whole path — the channel opening, the
/// copying both ways, the counters, and what happens when the tunnel is closed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tunnel_carries_a_connection_to_the_other_side() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    // The service the tunnel reaches: it greets, then echoes in upper case, so the test can
    // tell which direction a byte went.
    let service = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let service_port = service.local_addr().unwrap().port();
    tokio::spawn(async move {
        while let Ok((mut socket, _)) = service.accept().await {
            tokio::spawn(async move {
                let _ = socket.write_all(b"hello from the far side\n").await;
                let mut buf = vec![0u8; 256];
                while let Ok(n) = socket.read(&mut buf).await {
                    if n == 0 {
                        return;
                    }
                    let shouted = buf[..n].to_ascii_uppercase();
                    if socket.write_all(&shouted).await.is_err() {
                        return;
                    }
                }
            });
        }
    });

    let server_root = tempfile::tempdir().unwrap();
    let port = start(server_root.path()).await;
    let local = tempfile::tempdir().unwrap();
    let mut site = Site::parse_url(&format!("sftp://mb@127.0.0.1:{port}/")).unwrap();
    site.auth = SiteAuth::Password;
    let policy = HostKeyPolicy {
        known_hosts: Some(local.path().join("known_hosts")),
        accept_new: true,
    };
    let remote = scmd_net::connect(&site, Some(PASSWORD), &policy)
        .await
        .expect("the connection");

    let spec = scmd_net::TunnelSpec::parse(&format!("0:127.0.0.1:{service_port}")).unwrap();
    let tunnel = scmd_net::tunnel::open(remote, 1, spec)
        .await
        .expect("the tunnel opens");
    assert_eq!(tunnel.counts(), (0, 0, 0));
    assert!(tunnel.describe().contains(&service_port.to_string()));
    // Port zero asked the system for one, and it is not zero any more.
    assert_ne!(tunnel.local.port(), 0);
    assert_eq!(tunnel.local.ip().to_string(), "127.0.0.1");

    let mut client = tokio::net::TcpStream::connect(tunnel.local).await.unwrap();
    let mut greeting = vec![0u8; 24];
    client.read_exact(&mut greeting).await.unwrap();
    assert_eq!(&greeting, b"hello from the far side\n");
    client.write_all(b"ping\n").await.unwrap();
    let mut answer = vec![0u8; 5];
    client.read_exact(&mut answer).await.unwrap();
    assert_eq!(&answer, b"PING\n");

    let (served, live, failed) = tunnel.counts();
    assert_eq!((served, failed), (1, 0));
    assert_eq!(live, 1, "the connection is still open");

    // A second connection goes through the same tunnel.
    let mut other = tokio::net::TcpStream::connect(tunnel.local).await.unwrap();
    let mut greeting = vec![0u8; 24];
    other.read_exact(&mut greeting).await.unwrap();
    assert_eq!(tunnel.counts().0, 2);

    // Closing the tunnel frees the port: the listener is dropped with it.
    let address = tunnel.local;
    drop(tunnel);
    drop(client);
    drop(other);
    let mut freed = false;
    for _ in 0..50 {
        tokio::time::sleep(Duration::from_millis(20)).await;
        if tokio::net::TcpListener::bind(address).await.is_ok() {
            freed = true;
            break;
        }
    }
    assert!(freed, "the port should be free again");
}

/// Nothing listening at the far end: the connection that arrived is dropped and the session
/// carries on, which is what matters — a refused tunnel must not take the panel with it.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_tunnel_to_nothing_does_not_take_the_session_down() {
    use tokio::io::AsyncReadExt;

    let server_root = tempfile::tempdir().unwrap();
    std::fs::write(server_root.path().join("there.txt"), b"still here\n").unwrap();
    let port = start(server_root.path()).await;
    let local = tempfile::tempdir().unwrap();
    let mut site = Site::parse_url(&format!("sftp://mb@127.0.0.1:{port}/")).unwrap();
    site.auth = SiteAuth::Password;
    let policy = HostKeyPolicy {
        known_hosts: Some(local.path().join("known_hosts")),
        accept_new: true,
    };
    let remote = scmd_net::connect(&site, Some(PASSWORD), &policy)
        .await
        .expect("the connection");

    // A port nothing is on: bound and dropped, so it is free and almost certainly unused.
    let dead = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let dead_port = dead.local_addr().unwrap().port();
    drop(dead);

    let spec = scmd_net::TunnelSpec::parse(&format!("0:127.0.0.1:{dead_port}")).unwrap();
    let tunnel = scmd_net::tunnel::open(remote.clone(), 2, spec)
        .await
        .expect("the tunnel opens: nothing has been connected yet");

    let mut client = tokio::net::TcpStream::connect(tunnel.local).await.unwrap();
    let mut buf = vec![0u8; 8];
    // The far end refuses, so this end sees the connection close with nothing on it.
    let read = client.read(&mut buf).await;
    assert!(matches!(read, Ok(0) | Err(_)), "{read:?}");
    for _ in 0..50 {
        if tunnel.counts().2 > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(tunnel.counts(), (0, 0, 1), "one refusal, nothing served");

    // And the session is still good for what it was opened for.
    let entry = remote
        .stat(&VPath::remote(
            scmd_core::Scheme::Sftp,
            site.authority(),
            "/there.txt",
        ))
        .await
        .expect("the session survived");
    assert_eq!(entry.size, 11);
}
