//! A WebDAV server, in process, for the tests of the client beside it.
//!
//! It is a small HTTP/1.1 server that answers the six methods this client sends — `PROPFIND`,
//! `GET`, `PUT`, `MKCOL`, `DELETE` and `MOVE` — over a real socket, so the client's own code
//! (the URL building, the XML reading, the streamed body of a `PUT`, the error messages) is
//! exercised rather than mocked. Every connection is answered with `Connection: close`, which
//! is the one simplification: there is no keep-alive bookkeeping, and `reqwest` is perfectly
//! happy to open another.
//!
//! The tree it serves is the same shape as the FTP fake's: a map from an absolute path to its
//! bytes, or to nothing at all for a collection.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

type Node = Option<Vec<u8>>;
type Tree = Arc<Mutex<BTreeMap<String, Node>>>;

pub struct FakeDav {
    addr: SocketAddr,
    tree: Tree,
    /// Every request the server was given, as `METHOD path`, so a test can say what was sent.
    pub seen: Arc<Mutex<Vec<String>>>,
}

impl FakeDav {
    pub async fn start() -> FakeDav {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let addr = listener.local_addr().expect("the address");
        let mut nodes = BTreeMap::new();
        nodes.insert("/".to_string(), None);
        let server = FakeDav {
            addr,
            tree: Arc::new(Mutex::new(nodes)),
            seen: Arc::new(Mutex::new(Vec::new())),
        };
        let tree = server.tree.clone();
        let seen = server.seen.clone();
        tokio::spawn(async move {
            while let Ok((socket, _)) = listener.accept().await {
                let (tree, seen) = (tree.clone(), seen.clone());
                tokio::spawn(async move {
                    let _ = serve(socket, tree, seen).await;
                });
            }
        });
        server
    }

    /// `dav://mb@127.0.0.1:port/` — the site the client connects to.
    pub fn url(&self) -> String {
        format!("dav://mb@{}/", self.addr)
    }

    pub fn with_dir(self, path: &str) -> FakeDav {
        self.tree.lock().unwrap().insert(path.to_string(), None);
        self
    }

    pub fn with_file(self, path: &str, body: &[u8]) -> FakeDav {
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

    pub fn methods(&self) -> Vec<String> {
        self.seen.lock().unwrap().clone()
    }
}

async fn serve(
    mut socket: TcpStream,
    tree: Tree,
    seen: Arc<Mutex<Vec<String>>>,
) -> std::io::Result<()> {
    // Read until the end of the headers, then as much body as they say there is.
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    let head_end = loop {
        let read = socket.read(&mut buf).await?;
        if read == 0 {
            return Ok(());
        }
        raw.extend_from_slice(&buf[..read]);
        if let Some(at) = find(&raw, b"\r\n\r\n") {
            break at + 4;
        }
    };
    let head = String::from_utf8_lossy(&raw[..head_end]).to_string();
    let mut lines = head.lines();
    let request = lines.next().unwrap_or_default().to_string();
    let mut parts = request.split_whitespace();
    let method = parts.next().unwrap_or_default().to_ascii_uppercase();
    let target = parts.next().unwrap_or("/").to_string();
    let mut length = None;
    let mut chunked = false;
    let mut destination = None;
    let mut overwrite = None;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let (name, value) = (name.trim().to_ascii_lowercase(), value.trim().to_string());
        match name.as_str() {
            "content-length" => length = value.parse::<usize>().ok(),
            "transfer-encoding" if value.eq_ignore_ascii_case("chunked") => chunked = true,
            "destination" => destination = Some(value),
            "overwrite" => overwrite = Some(value),
            _ => {}
        }
    }
    let mut body = raw[head_end..].to_vec();
    if chunked {
        // `reqwest` sends a streamed body in chunks, so this is the path a `PUT` takes.
        loop {
            match take_chunks(&body) {
                Some(decoded) => {
                    body = decoded;
                    break;
                }
                None => {
                    let read = socket.read(&mut buf).await?;
                    if read == 0 {
                        break;
                    }
                    body.extend_from_slice(&buf[..read]);
                }
            }
        }
    } else if let Some(want) = length {
        while body.len() < want {
            let read = socket.read(&mut buf).await?;
            if read == 0 {
                break;
            }
            body.extend_from_slice(&buf[..read]);
        }
        body.truncate(want);
    }

    let path = super::decode_path(target.split(['?', '#']).next().unwrap_or("/"));
    let path = normal(&path);
    seen.lock().unwrap().push(format!("{method} {path}"));

    let (status, body) = answer(
        &method,
        &path,
        &body,
        destination.as_deref(),
        overwrite.as_deref(),
        &tree,
    );
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: application/xml; charset=utf-8\r\n\
         Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    socket.write_all(head.as_bytes()).await?;
    socket.write_all(&body).await?;
    socket.flush().await?;
    Ok(())
}

fn answer(
    method: &str,
    path: &str,
    body: &[u8],
    destination: Option<&str>,
    overwrite: Option<&str>,
    tree: &Tree,
) -> (&'static str, Vec<u8>) {
    match method {
        "PROPFIND" => {
            // Depth is not read: the client asks for 0 and for 1, and answering a depth-0
            // request with its children too is something a lenient server does — so the
            // client has to drop the collection's own row either way, and this makes sure it
            // is tested doing so.
            let nodes = tree.lock().unwrap();
            let Some(node) = nodes.get(path) else {
                return ("404 Not Found", Vec::new());
            };
            let mut xml = String::from(
                "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<D:multistatus xmlns:D=\"DAV:\">",
            );
            xml.push_str(&response_xml(path, node));
            if node.is_none() {
                let prefix = if path == "/" {
                    "/".to_string()
                } else {
                    format!("{path}/")
                };
                for (child, node) in nodes.iter() {
                    if child == path || !child.starts_with(&prefix) {
                        continue;
                    }
                    if child[prefix.len()..].contains('/') {
                        continue;
                    }
                    xml.push_str(&response_xml(child, node));
                }
            }
            xml.push_str("</D:multistatus>");
            ("207 Multi-Status", xml.into_bytes())
        }
        "GET" => match tree.lock().unwrap().get(path) {
            Some(Some(bytes)) => ("200 OK", bytes.clone()),
            Some(None) => ("405 Method Not Allowed", Vec::new()),
            None => ("404 Not Found", Vec::new()),
        },
        "PUT" => {
            let mut nodes = tree.lock().unwrap();
            let parent = parent_of(path);
            if !matches!(nodes.get(&parent), Some(None)) {
                return ("409 Conflict", Vec::new());
            }
            let fresh = !nodes.contains_key(path);
            nodes.insert(path.to_string(), Some(body.to_vec()));
            if fresh {
                ("201 Created", Vec::new())
            } else {
                ("204 No Content", Vec::new())
            }
        }
        "MKCOL" => {
            let mut nodes = tree.lock().unwrap();
            if nodes.contains_key(path) {
                return ("405 Method Not Allowed", Vec::new());
            }
            if !matches!(nodes.get(&parent_of(path)), Some(None)) {
                return ("409 Conflict", Vec::new());
            }
            nodes.insert(path.to_string(), None);
            ("201 Created", Vec::new())
        }
        "DELETE" => {
            let mut nodes = tree.lock().unwrap();
            if !nodes.contains_key(path) {
                return ("404 Not Found", Vec::new());
            }
            // A collection takes its tree with it, which is what RFC 4918 says it does.
            let prefix = format!("{path}/");
            let doomed: Vec<String> = nodes
                .keys()
                .filter(|k| *k == path || k.starts_with(&prefix))
                .cloned()
                .collect();
            for key in doomed {
                nodes.remove(&key);
            }
            ("204 No Content", Vec::new())
        }
        "MOVE" => {
            let Some(to) = destination.and_then(super::href_path) else {
                return ("400 Bad Request", Vec::new());
            };
            let to = normal(&to);
            let mut nodes = tree.lock().unwrap();
            if !nodes.contains_key(path) {
                return ("404 Not Found", Vec::new());
            }
            let replacing = nodes.contains_key(&to);
            if replacing
                && overwrite
                    .map(|o| o.eq_ignore_ascii_case("F"))
                    .unwrap_or(false)
            {
                return ("412 Precondition Failed", Vec::new());
            }
            let node = nodes.remove(path).expect("it was there a line ago");
            nodes.insert(to, node);
            if replacing {
                ("204 No Content", Vec::new())
            } else {
                ("201 Created", Vec::new())
            }
        }
        _ => ("501 Not Implemented", Vec::new()),
    }
}

/// One `<response>`, with the collection's own trailing slash, as a real server writes it.
fn response_xml(path: &str, node: &Node) -> String {
    let href = super::encode_path(path);
    match node {
        Some(body) => format!(
            "<D:response><D:href>{href}</D:href><D:propstat><D:prop>\
             <D:resourcetype/><D:getcontentlength>{}</D:getcontentlength>\
             <D:getlastmodified>Thu, 01 Jan 2026 12:00:00 GMT</D:getlastmodified>\
             </D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat></D:response>",
            body.len()
        ),
        None => {
            let href = if href.ends_with('/') {
                href
            } else {
                format!("{href}/")
            };
            // One response, two propstats: the properties the collection has, and the one it
            // was asked for and does not have. That is how Apache and nginx answer, and a
            // client that read the 404 one anyway would show every directory as an empty file.
            format!(
                "<D:response><D:href>{href}</D:href><D:propstat><D:prop>\
                 <D:resourcetype><D:collection/></D:resourcetype>\
                 <D:getlastmodified>Thu, 01 Jan 2026 12:00:00 GMT</D:getlastmodified>\
                 </D:prop><D:status>HTTP/1.1 200 OK</D:status></D:propstat>\
                 <D:propstat><D:prop><D:getcontentlength/></D:prop>\
                 <D:status>HTTP/1.1 404 Not Found</D:status></D:propstat></D:response>"
            )
        }
    }
}

fn parent_of(path: &str) -> String {
    match path.rfind('/') {
        Some(0) | None => "/".to_string(),
        Some(cut) => path[..cut].to_string(),
    }
}

fn normal(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Decode a chunked body, or `None` when the terminating zero chunk has not arrived yet.
fn take_chunks(raw: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    let mut rest = raw;
    loop {
        let cut = find(rest, b"\r\n")?;
        let size = usize::from_str_radix(String::from_utf8_lossy(&rest[..cut]).trim(), 16).ok()?;
        rest = &rest[cut + 2..];
        if size == 0 {
            return Some(out);
        }
        if rest.len() < size + 2 {
            return None;
        }
        out.extend_from_slice(&rest[..size]);
        rest = &rest[size + 2..];
    }
}
