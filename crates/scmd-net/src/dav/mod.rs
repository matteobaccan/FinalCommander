//! WebDAV, as the netbox's third protocol.
//!
//! Like the FTP client beside it, this implements [`RemoteFs`], so the panel, the viewer, the
//! editor, the job engine and the synchroniser work on a `dav://` path without knowing what it
//! is. The client is hand-written over `reqwest`, in the same shape as the object store's:
//! WebDAV is six HTTP methods and one XML document, and a crate for it would bring a date
//! library and a serialiser to do what [`xml`] does in one file that can be tested.
//!
//! What the protocol is, in one paragraph: a directory listing is a `PROPFIND` with
//! `Depth: 1`, which answers `207` and a `multistatus` of one `response` per entry; a file is a
//! `GET`; writing one is a `PUT` of the whole thing; a directory is a `MKCOL`; removing either
//! is a `DELETE`; and moving one is a `MOVE` with the new place in a header. There is nothing
//! to seek with and nothing to truncate — a `PUT` replaces a file entire — which is why
//! [`crate::vfs::caps_of`] says so rather than letting a caller find out.

#[cfg(test)]
pub mod fake;
pub mod xml;

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use reqwest::header::{HeaderMap, HeaderValue, CONTENT_TYPE};
use reqwest::{Client, Method, Response, StatusCode};
use scmd_core::{EntryKind, Scheme};

use crate::error::{NetError, Result, SecretKind};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};
use crate::remote::Remote;
use crate::site::{Auth, Site};

/// How long one request may take. A listing of a large collection over a slow link is the
/// worst case; a transfer is not bounded by this, because its body is streamed.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

/// The body of every `PROPFIND`: the four properties a panel needs, asked for by name.
///
/// `allprop` would be shorter to write and worse to use — it makes a server send every
/// property it has, which on some is a page of locks and quotas per entry.
const PROPFIND_BODY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<propfind xmlns="DAV:"><prop>
<resourcetype/><getcontentlength/><getlastmodified/><displayname/>
</prop></propfind>"#;

/// Open a WebDAV connection.
///
/// There is nothing to open, in truth: HTTP has no session, so this checks that the server is
/// there and that the credentials work by asking for the start directory, and keeps the client
/// (and its connection pool) for the requests that follow. A site that cannot be reached, or a
/// password that is refused, is found out here rather than on the first listing.
pub async fn connect(site: &Site, secret: Option<&str>) -> Result<Remote> {
    if let Auth::Key { .. } = site.auth {
        return Err(NetError::Unsupported {
            path: site.url(),
            detail: "WebDAV has no key authentication; use a password".into(),
        });
    }
    // An anonymous WebDAV server is a real thing (a public share), so an empty user means no
    // credentials at all rather than a prompt.
    let credentials = match (site.user.is_empty(), secret) {
        (true, _) => None,
        (false, Some(secret)) => Some((site.user.clone(), secret.to_string())),
        (false, None) => {
            return Err(NetError::NeedsSecret {
                user: site.user.clone(),
                host: site.host.clone(),
                kind: SecretKind::Password,
            })
        }
    };
    let client = Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("scmd/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| NetError::Connect {
            host: site.host.clone(),
            detail: e.to_string(),
        })?;
    let backend = DavBackend {
        site: site.clone(),
        client,
        credentials,
    };
    // One `PROPFIND` of depth 0 is the cheapest thing that proves all three of host, path and
    // password at once.
    backend.propfind(&site.start_dir, 0).await?;
    Ok(Remote::new(site.clone(), Arc::new(backend)))
}

struct DavBackend {
    site: Site,
    client: Client,
    /// The user and password, held for as long as the connection is open — HTTP sends them on
    /// every request, so there is nothing to be gained by forgetting them. Never written down.
    credentials: Option<(String, String)>,
}

impl std::fmt::Debug for DavBackend {
    /// By hand, so that no derive can ever print the password.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DavBackend")
            .field("site", &self.site.url())
            .field("credentials", &self.credentials.is_some())
            .finish()
    }
}

impl DavBackend {
    /// `https://host:port/a/b` for the wire path `/a/b`, with each segment percent-encoded.
    fn url(&self, path: &str) -> String {
        let scheme = if self.site.scheme == Scheme::Davs {
            "https"
        } else {
            "http"
        };
        let host = if self.site.host.contains(':') {
            // An IPv6 literal has to go back in its brackets.
            format!("[{}]", self.site.host)
        } else {
            self.site.host.clone()
        };
        format!("{scheme}://{host}:{}{}", self.site.port, encode_path(path))
    }

    /// A request with the credentials and the headers already on it.
    fn request(&self, method: Method, path: &str) -> reqwest::RequestBuilder {
        let builder = self.client.request(method, self.url(path));
        match &self.credentials {
            Some((user, password)) => builder.basic_auth(user, Some(password)),
            None => builder,
        }
    }

    async fn send(&self, request: reqwest::RequestBuilder, path: &str) -> Result<Response> {
        let response = request.send().await.map_err(|e| self.failed(path, e))?;
        let status = response.status();
        if status.is_success() || status == StatusCode::MULTI_STATUS {
            return Ok(response);
        }
        Err(self.refused(path, status, response).await)
    }

    /// A listing or a single entry: `PROPFIND` with the depth the caller wants.
    async fn propfind(&self, path: &str, depth: u8) -> Result<Vec<xml::DavRow>> {
        let mut headers = HeaderMap::new();
        headers.insert("Depth", HeaderValue::from(u32::from(depth)));
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/xml"));
        let request = self
            .request(
                Method::from_bytes(b"PROPFIND").expect("a method name"),
                path,
            )
            .headers(headers)
            .body(PROPFIND_BODY);
        let response = self.send(request, path).await?;
        let body = response.text().await.map_err(|e| self.failed(path, e))?;
        Ok(xml::parse_multistatus(&body))
    }

    /// What the server said went wrong, in words rather than in a number alone.
    async fn refused(&self, path: &str, status: StatusCode, response: Response) -> NetError {
        let detail = match status {
            StatusCode::UNAUTHORIZED => "the password was refused".to_string(),
            StatusCode::FORBIDDEN => "forbidden".to_string(),
            StatusCode::NOT_FOUND => "no such file or directory".to_string(),
            StatusCode::CONFLICT => "the directory above it does not exist".to_string(),
            StatusCode::PRECONDITION_FAILED => "something is already there".to_string(),
            StatusCode::INSUFFICIENT_STORAGE => "the server is out of space".to_string(),
            StatusCode::LOCKED => "it is locked by somebody else".to_string(),
            other => {
                // A WebDAV error body is XML and usually says more than the code does; the
                // first line of it is enough for a status bar.
                let body = response.text().await.unwrap_or_default();
                let first = body
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty() && !line.starts_with("<?"))
                    .unwrap_or("");
                if first.is_empty() {
                    format!("the server answered {other}")
                } else {
                    format!("{other}: {first}")
                }
            }
        };
        if status == StatusCode::UNAUTHORIZED {
            return NetError::Auth {
                user: self.site.user.clone(),
                host: self.site.host.clone(),
                detail,
            };
        }
        NetError::Remote {
            path: path.to_string(),
            detail,
        }
    }

    fn failed(&self, path: &str, e: reqwest::Error) -> NetError {
        // `reqwest`'s own message carries the URL, and with it the password-free authority;
        // the path is said separately so a status line reads the same as it does for SFTP.
        let detail = match (e.is_timeout(), e.is_connect()) {
            (true, _) => "the server did not answer in time".to_string(),
            (_, true) => format!("cannot reach the server: {e}"),
            _ => e.to_string(),
        };
        NetError::Remote {
            path: path.to_string(),
            detail,
        }
    }

    /// One `PROPFIND` row as a panel entry, with its name taken from the `href`.
    fn row_entry(&self, row: &xml::DavRow) -> Option<RemoteEntry> {
        let path = href_path(&row.href)?;
        let name = path.rsplit('/').find(|part| !part.is_empty())?.to_string();
        Some(RemoteEntry {
            // The name comes from the `href` and not from `displayname`: the href is the one
            // the next request has to use, and a server is free to make the two differ.
            name,
            kind: if row.is_dir {
                EntryKind::Dir
            } else {
                EntryKind::File
            },
            size: row.size,
            mtime: row.mtime,
            // HTTP has no permission bits. A read-only share answers `403` to a `PUT`, which
            // is the truth arriving at the moment it matters rather than a guess in a column.
            mode: None,
            link_target: None,
        })
    }
}

#[async_trait]
impl RemoteFs for DavBackend {
    /// `PROPFIND` with `Depth: 1`, whose first row is the collection itself.
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let rows = self.propfind(path, 1).await?;
        let here = normal(path);
        let mut out = Vec::with_capacity(rows.len());
        for row in &rows {
            // The collection asked for is in its own answer; the panel adds its own `..`.
            if href_path(&row.href).map(|p| normal(&p)) == Some(here.clone()) {
                continue;
            }
            if let Some(entry) = self.row_entry(row) {
                out.push(entry);
            }
        }
        Ok(out)
    }

    async fn stat(&self, path: &str, _follow: bool) -> Result<RemoteEntry> {
        let rows = self.propfind(path, 0).await?;
        let row = rows.first().ok_or_else(|| self.not_found(path))?;
        let mut entry = self.row_entry(row).ok_or_else(|| self.not_found(path))?;
        if normal(path) == "/" {
            entry.name = "/".to_string();
        }
        Ok(entry)
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let request = self.request(Method::from_bytes(b"MKCOL").expect("a method name"), path);
        self.send(request, path).await.map(|_| ())
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        let request = self.request(Method::DELETE, path);
        self.send(request, path).await.map(|_| ())
    }

    /// The same request: `DELETE` on a collection takes its tree, which is what a recursive
    /// delete means here and is one request rather than a walk.
    async fn remove_dir(&self, path: &str) -> Result<()> {
        self.remove_file(path).await
    }

    /// `MOVE`, with `Overwrite: F` so that nothing is replaced without being asked for.
    ///
    /// The engine's own conflict handling removes a destination it has been told to overwrite
    /// before it renames onto it, which is the same order SFTP needs, so refusing here costs
    /// nothing and protects a `F6` typed onto a name that is already taken.
    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let request = self
            .request(Method::from_bytes(b"MOVE").expect("a method name"), from)
            .header("Destination", self.url(to))
            .header("Overwrite", "F");
        self.send(request, from).await.map(|_| ())
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        Err(NetError::Unsupported {
            path: path.to_string(),
            detail: "WebDAV has no links".into(),
        })
    }

    async fn symlink(&self, _target: &str, link: &str) -> Result<()> {
        Err(NetError::Unsupported {
            path: link.to_string(),
            detail: "WebDAV cannot make a link".into(),
        })
    }

    /// Neither a mode nor a time can be set: `getlastmodified` is a *live* property in RFC
    /// 4918 — the server owns it — and there are no permission bits in HTTP at all.
    ///
    /// Saying so is the point. A copy that could not carry a timestamp across should say which
    /// part it could not do, rather than reporting success and leaving the file dated now.
    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()> {
        if mode.is_none() && mtime.is_none() {
            return Ok(());
        }
        Err(NetError::Unsupported {
            path: path.to_string(),
            detail: "WebDAV owns a file's time and has no permissions to set".into(),
        })
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let response = self.send(self.request(Method::GET, path), path).await?;
        let stream = response.bytes_stream();
        // `StreamReader` turns the body into an `AsyncRead` without holding it in memory,
        // which is what makes a four-gigabyte file a transfer rather than an allocation.
        Ok(Box::new(tokio_util::io::StreamReader::new(
            futures_util::TryStreamExt::map_err(stream, std::io::Error::other),
        )))
    }

    /// A `PUT` whose body is written as it arrives.
    ///
    /// HTTP wants one request for the whole file, and the caller hands us bytes a chunk at a
    /// time, so the two are joined by a pipe: the writer's end is what the caller writes to,
    /// the request reads from the other end, and the request runs on its own task until the
    /// body is closed. Closing the writer is therefore the moment the upload is finished, and
    /// it waits for the server's answer — a `PUT` that was refused has to be reported as an
    /// error there, or a failed save looks like a successful one.
    async fn create(&self, path: &str) -> Result<Writer> {
        let (ours, theirs) = tokio::io::duplex(256 * 1024);
        let body = reqwest::Body::wrap_stream(tokio_util::io::ReaderStream::new(theirs));
        let request = self
            .request(Method::PUT, path)
            .header(CONTENT_TYPE, "application/octet-stream")
            .body(body);
        let site = self.site.clone();
        let credentials = self.credentials.is_some();
        let here = path.to_string();
        let client = self.client.clone();
        let task = tokio::spawn(async move {
            let backend = DavBackend {
                site,
                client,
                // The request is already built and already carries the credentials; this is
                // only here for the error messages.
                credentials: credentials.then(|| (String::new(), String::new())),
            };
            backend.send(request, &here).await.map(|_| ())
        });
        Ok(Box::new(upload::Upload::new(ours, task)))
    }

    /// There is nothing to resolve: a WebDAV path is a URL path and the server has no notion
    /// of a working directory or of a login directory to be relative to.
    async fn canonicalize(&self, path: &str) -> Result<String> {
        Ok(normal(path))
    }
}

impl DavBackend {
    fn not_found(&self, path: &str) -> NetError {
        NetError::Remote {
            path: path.to_string(),
            detail: "no such file or directory".into(),
        }
    }
}

/// `/a/b/` → `/a/b`, and an empty path → `/`. Collections are answered with a trailing slash
/// and asked for without one, and the two have to compare equal.
fn normal(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        trimmed.to_string()
    }
}

/// The path out of an `href`, which a server may write as a whole URL or as an absolute path,
/// and which is percent-encoded either way.
pub fn href_path(href: &str) -> Option<String> {
    let path = match href.find("://") {
        Some(start) => {
            let after = &href[start + 3..];
            match after.find('/') {
                Some(cut) => &after[cut..],
                None => "/",
            }
        }
        None => href,
    };
    Some(decode_path(path))
}

/// Percent-encode a path, leaving the separators alone.
///
/// The unreserved set of RFC 3986 plus the handful of characters every server accepts raw;
/// everything else — a space, a `#`, a `?`, an `&`, anything non-ASCII — goes out as `%XX`,
/// because an unencoded `#` in a name would turn the rest of the path into a fragment and the
/// file would not be found.
pub fn encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len() + 8);
    if !path.starts_with('/') {
        out.push('/');
    }
    for byte in path.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' => out.push(*byte as char),
            b'-' | b'_' | b'.' | b'~' | b'/' | b'!' | b'$' | b'\'' | b'(' | b')' | b'*' | b'+'
            | b',' | b'=' | b':' | b'@' => out.push(*byte as char),
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// The inverse. Invalid UTF-8 in a name is kept as the bytes it was, read lossily, rather than
/// losing the row.
pub fn decode_path(path: &str) -> String {
    if !path.contains('%') {
        return path.to_string();
    }
    let bytes = path.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

mod upload {
    //! The writing end of a `PUT`.

    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use tokio::io::{AsyncWrite, DuplexStream};
    use tokio::task::JoinHandle;

    use crate::error::NetError;

    pub struct Upload {
        pipe: Option<DuplexStream>,
        task: Option<JoinHandle<Result<(), NetError>>>,
    }

    impl Upload {
        pub fn new(pipe: DuplexStream, task: JoinHandle<Result<(), NetError>>) -> Upload {
            Upload {
                pipe: Some(pipe),
                task: Some(task),
            }
        }
    }

    impl AsyncWrite for Upload {
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            data: &[u8],
        ) -> Poll<io::Result<usize>> {
            match &mut self.get_mut().pipe {
                Some(pipe) => Pin::new(pipe).poll_write(cx, data),
                None => Poll::Ready(Err(closed())),
            }
        }

        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            match &mut self.get_mut().pipe {
                Some(pipe) => Pin::new(pipe).poll_flush(cx),
                None => Poll::Ready(Ok(())),
            }
        }

        /// Close the body, then wait for what the server said about it.
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let me = self.get_mut();
            if let Some(pipe) = &mut me.pipe {
                match Pin::new(pipe).poll_shutdown(cx) {
                    Poll::Ready(Ok(())) => me.pipe = None,
                    Poll::Ready(Err(e)) => {
                        me.pipe = None;
                        return Poll::Ready(Err(e));
                    }
                    Poll::Pending => return Poll::Pending,
                }
            }
            let Some(task) = &mut me.task else {
                return Poll::Ready(Ok(()));
            };
            match Pin::new(task).poll(cx) {
                Poll::Ready(outcome) => {
                    me.task = None;
                    Poll::Ready(match outcome {
                        Ok(Ok(())) => Ok(()),
                        Ok(Err(e)) => Err(io::Error::other(e.to_string())),
                        Err(e) => Err(io::Error::other(format!("the upload task failed: {e}"))),
                    })
                }
                Poll::Pending => Poll::Pending,
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
    fn a_path_survives_the_trip_through_a_url() {
        // A space, an ampersand and a hash all have to be encoded: a raw `#` would cut the
        // path short and the file would not be found.
        assert_eq!(encode_path("/a/b c.txt"), "/a/b%20c.txt");
        assert_eq!(encode_path("/a/b&c#d.txt"), "/a/b%26c%23d.txt");
        assert_eq!(encode_path("/a/ümlaut"), "/a/%C3%BCmlaut");
        assert_eq!(encode_path("srv"), "/srv", "a path is always absolute");
        assert_eq!(encode_path("/a/b~c-d_e.txt"), "/a/b~c-d_e.txt");

        for path in ["/a/b c.txt", "/a/b&c#d.txt", "/a/ümlaut", "/plain.txt"] {
            assert_eq!(decode_path(&encode_path(path)), path, "{path}");
        }
        // A stray `%` is not an escape and is left where it was.
        assert_eq!(decode_path("/a/100%"), "/a/100%");
    }

    #[test]
    fn an_href_is_read_whether_it_is_a_url_or_a_path() {
        assert_eq!(
            href_path("https://host/dav/a%20b.txt").as_deref(),
            Some("/dav/a b.txt")
        );
        assert_eq!(href_path("/dav/a%20b.txt").as_deref(), Some("/dav/a b.txt"));
        assert_eq!(href_path("https://host").as_deref(), Some("/"));
    }

    #[test]
    fn a_collection_compares_the_same_with_or_without_its_slash() {
        assert_eq!(normal("/dav/srv/"), "/dav/srv");
        assert_eq!(normal("/dav/srv"), "/dav/srv");
        assert_eq!(normal("/"), "/");
        assert_eq!(normal(""), "/");
    }

    #[tokio::test]
    async fn a_key_is_not_something_webdav_can_be_given() {
        let mut site = Site::parse_url("davs://mb@host/").unwrap();
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
    async fn a_named_user_with_no_password_asks_for_one() {
        let site = Site::parse_url("davs://mb@host/").unwrap();
        match connect(&site, None).await {
            Err(NetError::NeedsSecret { kind, .. }) => assert_eq!(kind, SecretKind::Password),
            other => panic!("expected a request for a password, got {other:?}"),
        }
    }
    /// Everything below talks to [`fake::FakeDav`], a WebDAV server in this process, so the
    /// URL building, the XML reading, the streamed `PUT` and the error messages are all
    /// exercised over a real socket.
    async fn connected() -> (fake::FakeDav, Remote) {
        let server = fake::FakeDav::start()
            .await
            .with_dir("/srv")
            .with_file("/srv/read me.md", b"# the readme\n")
            .with_dir("/srv/pictures");
        let site = Site::parse_url(&server.url()).expect("the url");
        let remote = connect(&site, Some("secret")).await.expect("a connection");
        (server, remote)
    }

    #[tokio::test]
    async fn a_collection_is_listed_and_the_collection_itself_is_not_one_of_its_rows() {
        let (_server, remote) = connected().await;
        let rows = remote
            .list(
                &remote.site().root().join("srv"),
                &scmd_core::CancelToken::new(),
            )
            .await
            .expect("a listing");
        let names: Vec<String> = rows.iter().map(|e| e.name_str().into_owned()).collect();
        // `..` is the panel's, `pictures` and the file are the server's, and `/srv` itself —
        // which a `PROPFIND` of depth 1 always answers with — is not a row.
        assert_eq!(names, ["..", "pictures", "read me.md"]);

        let file = rows.iter().find(|e| e.name_str() == "read me.md").unwrap();
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.size, 13);
        assert!(file.mtime.is_some(), "getlastmodified was read");
        let dir = rows.iter().find(|e| e.name_str() == "pictures").unwrap();
        assert_eq!(dir.kind, EntryKind::Dir);
        assert_eq!(dir.size, 0, "a collection has no size of its own");
    }

    #[tokio::test]
    async fn a_file_travels_both_ways_with_its_body_streamed() {
        let dir = tempfile::tempdir().unwrap();
        let (server, remote) = connected().await;
        let mut nothing = |_: u64| true;

        let local = dir.path().join("readme.md");
        let got = remote
            .download(
                &remote.site().root().join("srv").join("read me.md"),
                &local,
                &mut nothing,
            )
            .await
            .expect("the download");
        assert_eq!(got, 13);
        assert_eq!(std::fs::read(&local).unwrap(), b"# the readme\n");

        // A body big enough to arrive in several chunks, which is the path the pipe exists
        // for: the writer is handed bytes a chunk at a time and the request streams them.
        let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let source = dir.path().join("big.bin");
        std::fs::write(&source, &payload).unwrap();
        let sent = remote
            .upload(
                &source,
                &remote.site().root().join("srv").join("big.bin"),
                &mut nothing,
            )
            .await
            .expect("the upload");
        assert_eq!(sent, payload.len() as u64);
        assert_eq!(server.file("/srv/big.bin"), Some(payload));
    }

    #[tokio::test]
    async fn the_things_a_panel_does_to_a_collection_all_work() {
        let (server, remote) = connected().await;
        let srv = remote.site().root().join("srv");

        remote.create_dir(&srv.join("new")).await.expect("MKCOL");
        assert!(server.has("/srv/new"));

        remote
            .rename(&srv.join("read me.md"), &srv.join("README.txt"))
            .await
            .expect("MOVE");
        assert!(!server.has("/srv/read me.md"));
        assert_eq!(
            server.file("/srv/README.txt").as_deref(),
            Some(&b"# the readme\n"[..])
        );

        let facts = remote
            .facts(&srv.join("README.txt"), false)
            .await
            .expect("PROPFIND of depth 0");
        assert_eq!(facts.size, 13);
        assert_eq!(facts.kind, EntryKind::File);
        assert!(remote.exists(&srv.join("README.txt")).await);
        assert!(!remote.exists(&srv.join("gone.txt")).await);

        remote
            .remove_file(&srv.join("README.txt"))
            .await
            .expect("DELETE");
        assert!(!server.has("/srv/README.txt"));
        // A `DELETE` on a collection takes its tree, so one request is the whole of it.
        remote
            .remove_dir(&srv.join("pictures"))
            .await
            .expect("DELETE");
        assert!(!server.has("/srv/pictures"));

        // And the methods that were sent are the methods WebDAV has, not a filesystem's.
        let sent = server.methods();
        assert!(sent.iter().any(|m| m == "MKCOL /srv/new"), "{sent:?}");
        assert!(sent.iter().any(|m| m == "MOVE /srv/read me.md"), "{sent:?}");
        assert!(
            sent.iter().any(|m| m == "PROPFIND /srv/README.txt"),
            "{sent:?}"
        );
        assert!(
            sent.iter()
                .all(|m| !m.starts_with("RMDIR") && !m.starts_with("CHMOD")),
            "{sent:?}"
        );
    }

    #[tokio::test]
    async fn a_rename_onto_a_name_that_is_taken_is_refused_rather_than_silent() {
        let (server, remote) = connected().await;
        let srv = remote.site().root().join("srv");
        let error = remote
            .rename(&srv.join("read me.md"), &srv.join("pictures"))
            .await
            .expect_err("it must not replace a collection");
        assert!(error.to_string().contains("already there"), "{error}");
        // Both are still where they were.
        assert!(server.has("/srv/read me.md"));
        assert!(server.has("/srv/pictures"));
    }

    #[tokio::test]
    async fn a_time_is_the_servers_own_and_the_client_says_so() {
        let (_server, remote) = connected().await;
        let path = remote.site().root().join("srv").join("read me.md");
        let facts = scmd_jobs::Facts {
            kind: EntryKind::File,
            size: 13,
            mtime: Some(std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_767_268_800)),
            mode: Some(0o644),
            readonly: false,
        };
        match remote.apply_meta(&path, &facts).await {
            Err(NetError::Unsupported { detail, .. }) => {
                assert!(detail.contains("owns a file's time"), "{detail}")
            }
            other => panic!("expected a refusal, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn what_is_not_there_is_said_as_what_it_is() {
        let (_server, remote) = connected().await;
        let missing = remote.site().root().join("srv").join("nowhere.txt");
        let error = remote
            .facts(&missing, false)
            .await
            .expect_err("it is not there");
        let text = error.to_string();
        assert!(text.contains("nowhere.txt"), "{text}");
        assert!(text.contains("no such file"), "{text}");
    }
}
