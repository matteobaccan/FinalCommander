//! An FTP server, in process, for the tests of the client beside it.
//!
//! It speaks enough of RFC 959 and RFC 3659 to be worth testing against: a login, `FEAT`,
//! `PWD`/`CWD`, passive data connections through both `PASV` and `EPSV`, `MLSD` and `LIST`,
//! `RETR`, `STOR`, `MKD`, `RMD`, `DELE`, `RNFR`/`RNTO`, `SIZE`, `MLST` and `MFMT`. It is not a
//! general FTP server and is not meant to be: it is the other half of the conversation, so
//! that the client's own code — the listing parsers, the transfer connections, the error
//! messages — is exercised over a real socket rather than mocked away.
//!
//! The tree it serves lives in memory: a map from an absolute path to its bytes, or to nothing
//! at all for a directory.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

/// A file's bytes, or `None` for a directory.
type Node = Option<Vec<u8>>;
type Tree = Arc<Mutex<BTreeMap<String, Node>>>;

/// A running server. Dropping it stops answering new connections.
pub struct FakeFtp {
    addr: SocketAddr,
    tree: Tree,
    /// Every `MFMT` the server was asked for, so a test can see what the client sent.
    pub stamps: Arc<Mutex<Vec<(String, String)>>>,
    /// Answer `FEAT` with `MLSD`/`MFMT`, or with nothing at all.
    modern: bool,
}

impl FakeFtp {
    /// Start one, with a root directory and nothing in it.
    pub async fn start(modern: bool) -> FakeFtp {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let addr = listener.local_addr().expect("the address");
        let mut nodes = BTreeMap::new();
        nodes.insert("/".to_string(), None);
        let server = FakeFtp {
            addr,
            tree: Arc::new(Mutex::new(nodes)),
            stamps: Arc::new(Mutex::new(Vec::new())),
            modern,
        };
        let tree = server.tree.clone();
        let stamps = server.stamps.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let session = Session {
                    tree: tree.clone(),
                    stamps: stamps.clone(),
                    cwd: "/".to_string(),
                    data: None,
                    rename_from: None,
                    modern,
                };
                tokio::spawn(session.run(socket));
            }
        });
        server
    }

    pub fn url(&self) -> String {
        format!("ftp://mb@{}/", self.addr)
    }

    pub fn with_dir(self, path: &str) -> FakeFtp {
        self.tree.lock().unwrap().insert(path.to_string(), None);
        self
    }

    pub fn with_file(self, path: &str, body: &[u8]) -> FakeFtp {
        self.tree
            .lock()
            .unwrap()
            .insert(path.to_string(), Some(body.to_vec()));
        self
    }

    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        self.tree.lock().unwrap().get(path).cloned().flatten()
    }

    pub fn has(&self, path: &str) -> bool {
        self.tree.lock().unwrap().contains_key(path)
    }

    pub fn is_modern(&self) -> bool {
        self.modern
    }
}

struct Session {
    tree: Tree,
    stamps: Arc<Mutex<Vec<(String, String)>>>,
    cwd: String,
    /// The listener a `PASV` or `EPSV` opened, waiting for the transfer command.
    data: Option<TcpListener>,
    rename_from: Option<String>,
    modern: bool,
}

impl Session {
    async fn run(mut self, socket: TcpStream) {
        let (read, mut write) = socket.into_split();
        let mut lines = BufReader::new(read).lines();
        if write.write_all(b"220 fake ftp\r\n").await.is_err() {
            return;
        }
        while let Ok(Some(line)) = lines.next_line().await {
            let (verb, rest) = match line.trim_end().split_once(' ') {
                Some((verb, rest)) => (verb.to_ascii_uppercase(), rest.trim().to_string()),
                None => (line.trim_end().to_ascii_uppercase(), String::new()),
            };
            let quit = verb == "QUIT";
            if self.answer(&verb, &rest, &mut write).await.is_err() || quit {
                return;
            }
        }
    }

    async fn answer<W>(&mut self, verb: &str, rest: &str, out: &mut W) -> std::io::Result<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        match verb {
            "USER" => out.write_all(b"331 send the password\r\n").await,
            "PASS" => out.write_all(b"230 logged in\r\n").await,
            "FEAT" if self.modern => {
                out.write_all(
                    b"211-Features:\r\n MLSD\r\n MLST type*;size*;modify*;\r\n MFMT\r\n211 End\r\n",
                )
                .await
            }
            "FEAT" => out.write_all(b"211-Features:\r\n211 End\r\n").await,
            "TYPE" | "OPTS" | "NOOP" => out.write_all(b"200 fine\r\n").await,
            "SITE" => {
                // `SITE CHMOD 644 /srv/x`: accepted and forgotten, which is all a test needs.
                out.write_all(b"200 fine\r\n").await
            }
            "MFMT" => {
                let (stamp, path) = rest.split_once(' ').unwrap_or((rest, ""));
                self.stamps
                    .lock()
                    .unwrap()
                    .push((self.resolve(path.trim()), stamp.to_string()));
                out.write_all(b"213 time set\r\n").await
            }
            "PWD" => {
                let cwd = self.cwd.clone();
                out.write_all(format!("257 \"{cwd}\" is the directory\r\n").as_bytes())
                    .await
            }
            "CWD" => {
                let to = self.resolve(rest);
                let known = matches!(self.tree.lock().unwrap().get(&to), Some(None));
                if known {
                    self.cwd = to;
                    out.write_all(b"250 here now\r\n").await
                } else {
                    out.write_all(b"550 no such directory\r\n").await
                }
            }
            "PASV" => match self.listen().await {
                Some(port) => {
                    let (hi, lo) = (port / 256, port % 256);
                    out.write_all(
                        format!("227 Entering Passive Mode (127,0,0,1,{hi},{lo})\r\n").as_bytes(),
                    )
                    .await
                }
                None => {
                    out.write_all(b"425 cannot open a data connection\r\n")
                        .await
                }
            },
            "EPSV" => match self.listen().await {
                Some(port) => {
                    out.write_all(
                        format!("229 Entering Extended Passive Mode (|||{port}|)\r\n").as_bytes(),
                    )
                    .await
                }
                None => {
                    out.write_all(b"425 cannot open a data connection\r\n")
                        .await
                }
            },
            "MLSD" | "LIST" => {
                let dir = if rest.is_empty() {
                    self.cwd.clone()
                } else {
                    self.resolve(rest)
                };
                let body = self.listing(&dir, verb == "MLSD");
                self.send_data(out, body.as_bytes()).await
            }
            "RETR" => {
                let path = self.resolve(rest);
                let body = self.tree.lock().unwrap().get(&path).cloned();
                match body {
                    Some(Some(body)) => self.send_data(out, &body).await,
                    _ => out.write_all(b"550 no such file\r\n").await,
                }
            }
            "STOR" => {
                let path = self.resolve(rest);
                let Some(listener) = self.data.take() else {
                    return out.write_all(b"425 no data connection\r\n").await;
                };
                out.write_all(b"150 send it\r\n").await?;
                let mut body = Vec::new();
                match listener.accept().await {
                    Ok((mut socket, _)) => {
                        let _ = socket.read_to_end(&mut body).await;
                    }
                    Err(_) => return out.write_all(b"426 the transfer failed\r\n").await,
                }
                self.tree.lock().unwrap().insert(path, Some(body));
                out.write_all(b"226 stored\r\n").await
            }
            "SIZE" => {
                let path = self.resolve(rest);
                let size = self
                    .tree
                    .lock()
                    .unwrap()
                    .get(&path)
                    .and_then(|node| node.as_ref().map(|body| body.len()));
                match size {
                    Some(size) => out.write_all(format!("213 {size}\r\n").as_bytes()).await,
                    None => out.write_all(b"550 no such file\r\n").await,
                }
            }
            "MLST" if self.modern => {
                let path = self.resolve(rest);
                let facts = {
                    let tree = self.tree.lock().unwrap();
                    tree.get(&path).map(|node| fact_line(&path, node))
                };
                match facts {
                    Some(line) => {
                        out.write_all(format!("250-listing\r\n {line}\r\n250 End\r\n").as_bytes())
                            .await
                    }
                    None => out.write_all(b"550 no such file\r\n").await,
                }
            }
            "MKD" => {
                let path = self.resolve(rest);
                let made = {
                    let mut tree = self.tree.lock().unwrap();
                    match tree.contains_key(&path) {
                        true => false,
                        false => {
                            tree.insert(path.clone(), None);
                            true
                        }
                    }
                };
                if !made {
                    return out.write_all(b"550 it is already there\r\n").await;
                }
                out.write_all(format!("257 \"{path}\" created\r\n").as_bytes())
                    .await
            }
            "RMD" | "DELE" => {
                let path = self.resolve(rest);
                let gone = self.tree.lock().unwrap().remove(&path).is_some();
                if gone {
                    out.write_all(b"250 gone\r\n").await
                } else {
                    out.write_all(b"550 no such path\r\n").await
                }
            }
            "RNFR" => {
                let path = self.resolve(rest);
                if self.tree.lock().unwrap().contains_key(&path) {
                    self.rename_from = Some(path);
                    out.write_all(b"350 and the new name?\r\n").await
                } else {
                    out.write_all(b"550 no such path\r\n").await
                }
            }
            "RNTO" => {
                let to = self.resolve(rest);
                let moved = self.rename_from.take().map(|from| {
                    let mut tree = self.tree.lock().unwrap();
                    match tree.remove(&from) {
                        Some(node) => {
                            tree.insert(to, node);
                            true
                        }
                        None => false,
                    }
                });
                match moved {
                    Some(true) => out.write_all(b"250 renamed\r\n").await,
                    Some(false) => out.write_all(b"550 it went away\r\n").await,
                    None => out.write_all(b"503 RNFR first\r\n").await,
                }
            }
            "QUIT" => out.write_all(b"221 goodbye\r\n").await,
            _ => out.write_all(b"502 not here\r\n").await,
        }
    }

    /// Open the data listener a passive command promises.
    async fn listen(&mut self) -> Option<u16> {
        let listener = TcpListener::bind("127.0.0.1:0").await.ok()?;
        let port = listener.local_addr().ok()?.port();
        self.data = Some(listener);
        Some(port)
    }

    /// `150`, the bytes on the data connection, then `226`.
    async fn send_data<W>(&mut self, out: &mut W, body: &[u8]) -> std::io::Result<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let Some(listener) = self.data.take() else {
            return out.write_all(b"425 no data connection\r\n").await;
        };
        out.write_all(b"150 here it comes\r\n").await?;
        match listener.accept().await {
            Ok((mut socket, _)) => {
                let _ = socket.write_all(body).await;
                let _ = socket.shutdown().await;
            }
            Err(_) => return out.write_all(b"426 the transfer failed\r\n").await,
        }
        out.write_all(b"226 that was all of it\r\n").await
    }

    /// What is immediately inside `dir`, in `MLSD` or in `ls -l` form.
    fn listing(&self, dir: &str, mlsd: bool) -> String {
        let prefix = if dir == "/" {
            "/".to_string()
        } else {
            format!("{dir}/")
        };
        let tree = self.tree.lock().unwrap();
        let mut out = String::new();
        for (path, node) in tree.iter() {
            if path == dir || !path.starts_with(&prefix) {
                continue;
            }
            let name = &path[prefix.len()..];
            if name.contains('/') {
                continue;
            }
            if mlsd {
                out.push_str(&fact_line(path, node));
                out.push_str("\r\n");
            } else {
                let (kind, size) = match node {
                    Some(body) => ('-', body.len()),
                    None => ('d', 4096),
                };
                out.push_str(&format!(
                    "{kind}rw-r--r--   1 mb users {size:>8} Jan  1 12:00 {name}\r\n"
                ));
            }
        }
        out
    }

    /// `/a/b` against the current directory, which for this server is only ever asked of
    /// absolute paths — the client sends nothing else.
    fn resolve(&self, path: &str) -> String {
        let path = path.trim();
        let joined = if path.starts_with('/') {
            path.to_string()
        } else if self.cwd == "/" {
            format!("/{path}")
        } else {
            format!("{}/{path}", self.cwd)
        };
        let trimmed = joined.trim_end_matches('/');
        if trimmed.is_empty() {
            "/".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

fn fact_line(path: &str, node: &Node) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match node {
        Some(body) => format!(
            "type=file;size={};modify=20260101120000;UNIX.mode=0644; {name}",
            body.len()
        ),
        None => format!("type=dir;modify=20260101120000;UNIX.mode=0755; {name}"),
    }
}
