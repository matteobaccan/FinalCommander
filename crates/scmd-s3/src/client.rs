//! The requests themselves: list a bucket, list the buckets, read an object's head, read its
//! bytes.
//!
//! Four calls out of the hundred S3 has, which is the point: a file manager browses and reads,
//! and every one of these is a plain signed `GET` or `HEAD` whose answer is a document the
//! reader in `xml` understands. One connection pool is kept per service, because a listing of
//! a large bucket is a thousand requests in a row and a fresh TLS handshake for each would
//! cost more than everything else put together.

use std::sync::Arc;
use std::time::{Duration, SystemTime};

use scmd_core::CancelToken;

use crate::endpoint::Endpoint;
use crate::error::{Result, S3Error};
use crate::sign::{self, Credentials, Signable, Signer, EMPTY_SHA256};
use crate::uri;
use crate::xml::{self, BucketRow, Page};

/// How many keys one page of a listing asks for. A thousand is the service's own maximum, and
/// asking for fewer only means more round trips.
pub const PAGE_KEYS: u32 = 1000;

/// One service, one region, one set of credentials.
#[derive(Debug, Clone)]
pub struct Client {
    http: reqwest::Client,
    endpoint: Endpoint,
    /// `None` for an anonymous client, which is how a public bucket is read.
    signer: Option<Signer>,
    timeout: Duration,
    retries: u32,
}

/// What a `HEAD` tells us about one object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectMeta {
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub etag: Option<String>,
}

impl Client {
    pub fn new(endpoint: Endpoint, credentials: Option<Credentials>) -> Result<Client> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("scmd/", env!("CARGO_PKG_VERSION")))
            // A pool large enough that the pages of one listing reuse a connection, and an
            // idle timeout that lets it go when the panel moves on.
            .pool_max_idle_per_host(4)
            .pool_idle_timeout(Duration::from_secs(60))
            .build()
            .map_err(|e| S3Error::Transport {
                host: endpoint.host.clone(),
                detail: e.to_string(),
            })?;
        let signer = credentials.map(|c| Signer::new(c, endpoint.region.clone()));
        Ok(Client {
            http,
            endpoint,
            signer,
            timeout: Duration::from_secs(30),
            retries: 3,
        })
    }

    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// The same client against another region, for when the service says the bucket is
    /// elsewhere and names where.
    pub fn in_region(&self, region: &str) -> Result<Client> {
        let mut endpoint = self.endpoint.clone();
        // An AWS host carries its region in its name, and the two must agree.
        if let Some(rest) = endpoint
            .host
            .strip_prefix(&format!("s3.{}.", endpoint.region))
        {
            endpoint.host = format!("s3.{region}.{rest}");
        }
        endpoint.region = region.to_string();
        Client::new(
            endpoint,
            self.signer.as_ref().map(|s| s.credentials.clone()),
        )
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Client {
        self.timeout = timeout;
        self
    }

    /// One page of a listing.
    ///
    /// `delimiter` is what turns a flat namespace into a tree: with `/` the service answers
    /// with the objects directly under `prefix` and, separately, the set of prefixes one level
    /// below it — which is exactly a directory's files and subdirectories. Without it the
    /// answer is every key under the prefix, however deep, which is how a bucket with a
    /// million objects in it is read if someone really asks for that.
    pub async fn list_page(
        &self,
        bucket: &str,
        prefix: &str,
        delimiter: Option<&str>,
        token: Option<&str>,
        cancel: &CancelToken,
    ) -> Result<Page> {
        let mut query: Vec<(&str, String)> = vec![
            ("list-type", "2".into()),
            ("encoding-type", "url".into()),
            ("max-keys", PAGE_KEYS.to_string()),
        ];
        if !prefix.is_empty() {
            query.push(("prefix", prefix.to_string()));
        }
        if let Some(delimiter) = delimiter {
            query.push(("delimiter", delimiter.to_string()));
        }
        if let Some(token) = token {
            query.push(("continuation-token", token.to_string()));
        }
        let body = self.get_text("GET", bucket, "", &query, cancel).await?;
        Ok(xml::parse_page(&body))
    }

    /// Every bucket the credentials can see. Anonymous access cannot do this, and says so.
    pub async fn list_buckets(&self, cancel: &CancelToken) -> Result<Vec<BucketRow>> {
        if self.signer.is_none() {
            return Err(S3Error::NoCredentials(
                "listing buckets needs credentials; an anonymous client can only read a bucket it is told the name of".into(),
            ));
        }
        let body = self.get_text("GET", "", "", &[], cancel).await?;
        Ok(xml::parse_buckets(&body))
    }

    /// What one object is, without reading it.
    pub async fn head(&self, bucket: &str, key: &str, cancel: &CancelToken) -> Result<ObjectMeta> {
        let response = self
            .send("HEAD", bucket, key, &[], &[], Payload::Empty, cancel)
            .await?;
        let header = |name: &str| {
            response
                .headers()
                .get(name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        };
        Ok(ObjectMeta {
            size: header("content-length")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            modified: header("last-modified").as_deref().and_then(http_date),
            etag: header("etag").map(|e| e.trim_matches('"').to_string()),
        })
    }

    /// Write an object from bytes already in memory.
    ///
    /// This is the small write: the editor's save, and the empty object that stands for a
    /// directory. It returns what the service said the object is, so a caller can show the
    /// new stamp without asking again.
    pub async fn put(
        &self,
        bucket: &str,
        key: &str,
        body: Vec<u8>,
        cancel: &CancelToken,
    ) -> Result<ObjectMeta> {
        let len = body.len() as u64;
        let sha = sign::sha256_hex(&body);
        let response = self
            .send(
                "PUT",
                bucket,
                key,
                &[],
                &[],
                Payload::Bytes(Arc::new(body), sha),
                cancel,
            )
            .await?;
        Ok(ObjectMeta {
            size: len,
            modified: None,
            etag: response
                .headers()
                .get("etag")
                .and_then(|v| v.to_str().ok())
                .map(|e| e.trim_matches('"').to_string()),
        })
    }

    /// Write an object from a file, streamed rather than held in memory.
    ///
    /// The body goes up as it is read, so a file larger than this process would like to hold
    /// is not a problem — but it is read twice: once to hash it for the signature, once to
    /// send it (see [`sign::sha256_file`]).
    ///
    /// One `PUT` is one operation to the service: it either arrives whole or leaves nothing
    /// behind, so there is no half-written object and no partial progress worth reporting
    /// after a failure. `Esc` is answered before the body starts and by the request timeout
    /// after it. An object above five gigabytes needs a multipart upload, which this is not,
    /// and the service's own refusal is what says so.
    pub async fn put_file(
        &self,
        bucket: &str,
        key: &str,
        source: &std::path::Path,
        cancel: &CancelToken,
    ) -> Result<u64> {
        let len = tokio::fs::metadata(source)
            .await
            .map_err(|e| local_error(source, e))?
            .len();
        let path = source.to_path_buf();
        let sha = tokio::task::spawn_blocking(move || sign::sha256_file(&path))
            .await
            .map_err(|e| S3Error::Local(e.to_string()))?
            .map_err(|e| local_error(source, e))?;
        self.send(
            "PUT",
            bucket,
            key,
            &[],
            &[],
            Payload::File {
                path: source.to_path_buf(),
                len,
                sha256: sha,
            },
            cancel,
        )
        .await?;
        Ok(len)
    }

    /// Copy an object inside the service, without the bytes coming through here.
    ///
    /// `from` is `bucket/key`, which is the form the header takes. The service does the work,
    /// which is what makes a rename on object storage affordable: a rename is a copy and a
    /// delete, and this is the copy.
    pub async fn copy_object(
        &self,
        bucket: &str,
        key: &str,
        from_bucket: &str,
        from_key: &str,
        cancel: &CancelToken,
    ) -> Result<()> {
        // The source is a path, so it is encoded as one — a key with a space or a `+` in it
        // is a key, and a header that said otherwise would copy the wrong object.
        let source = format!("/{from_bucket}/{}", uri::encode_path(from_key));
        self.send(
            "PUT",
            bucket,
            key,
            &[],
            &[("x-amz-copy-source", source)],
            Payload::Empty,
            cancel,
        )
        .await?;
        Ok(())
    }

    /// Remove one object. A key that is not there is not an error: the end state asked for is
    /// the end state, and the service reports it the same way.
    pub async fn delete_object(&self, bucket: &str, key: &str, cancel: &CancelToken) -> Result<()> {
        self.send("DELETE", bucket, key, &[], &[], Payload::Empty, cancel)
            .await?;
        Ok(())
    }

    /// Read an object, or a window of one. The viewer asks for windows; a download asks for
    /// the whole thing.
    pub async fn get(
        &self,
        bucket: &str,
        key: &str,
        range: Option<(u64, Option<u64>)>,
        cancel: &CancelToken,
    ) -> Result<Vec<u8>> {
        let range = range.map(|(from, len)| match len {
            Some(len) if len > 0 => format!("bytes={}-{}", from, from + len - 1),
            Some(_) => format!("bytes={from}-{from}"),
            None => format!("bytes={from}-"),
        });
        let range: Vec<(&str, String)> = match range {
            Some(range) => vec![("range", range)],
            None => Vec::new(),
        };
        let response = self
            .send("GET", bucket, key, &[], &range, Payload::Empty, cancel)
            .await?;
        let host = self.endpoint.host.clone();
        let bytes = response.bytes().await.map_err(|e| S3Error::Transport {
            host,
            detail: e.to_string(),
        })?;
        Ok(bytes.to_vec())
    }

    /// Read an object straight into a file, a chunk at a time.
    ///
    /// Unlike [`Client::get`] the bytes never all exist here at once, which is what makes a
    /// download of something larger than this process's memory possible. `on_bytes` is called
    /// with the running total as each chunk lands and stops the download by returning
    /// `false`, in which case the partial file is removed — a half-written file under the
    /// name of a whole one is worse than no file.
    pub async fn get_to_file(
        &self,
        bucket: &str,
        key: &str,
        target: &std::path::Path,
        on_bytes: &mut dyn FnMut(u64) -> bool,
        cancel: &CancelToken,
    ) -> Result<u64> {
        use tokio::io::AsyncWriteExt;

        let mut response = self
            .send("GET", bucket, key, &[], &[], Payload::Empty, cancel)
            .await?;
        let host = self.endpoint.host.clone();
        let mut file = tokio::fs::File::create(target)
            .await
            .map_err(|e| local_error(target, e))?;
        let mut done = 0u64;
        loop {
            let chunk = response.chunk().await.map_err(|e| S3Error::Transport {
                host: host.clone(),
                detail: e.to_string(),
            });
            let chunk = match chunk {
                Ok(Some(chunk)) => chunk,
                Ok(None) => break,
                Err(e) => {
                    drop(file);
                    let _ = tokio::fs::remove_file(target).await;
                    return Err(e);
                }
            };
            if let Err(e) = file.write_all(&chunk).await {
                drop(file);
                let _ = tokio::fs::remove_file(target).await;
                return Err(local_error(target, e));
            }
            done += chunk.len() as u64;
            if !on_bytes(done) || cancel.is_cancelled() {
                drop(file);
                let _ = tokio::fs::remove_file(target).await;
                return Err(S3Error::Cancelled);
            }
        }
        file.flush().await.map_err(|e| local_error(target, e))?;
        Ok(done)
    }

    async fn get_text(
        &self,
        method: &str,
        bucket: &str,
        key: &str,
        query: &[(&str, String)],
        cancel: &CancelToken,
    ) -> Result<String> {
        let response = self
            .send(method, bucket, key, query, &[], Payload::Empty, cancel)
            .await?;
        let host = self.endpoint.host.clone();
        response.text().await.map_err(|e| S3Error::Transport {
            host,
            detail: e.to_string(),
        })
    }

    /// Sign, send, and retry what is worth retrying.
    #[allow(clippy::too_many_arguments)]
    async fn send(
        &self,
        method: &str,
        bucket: &str,
        key: &str,
        query: &[(&str, String)],
        headers: &[(&str, String)],
        payload: Payload,
        cancel: &CancelToken,
    ) -> Result<reqwest::Response> {
        let mut attempt = 0;
        loop {
            if cancel.is_cancelled() {
                return Err(S3Error::Cancelled);
            }
            match self
                .send_once(method, bucket, key, query, headers, payload.clone())
                .await
            {
                Ok(response) => return Ok(response),
                Err(error) => {
                    attempt += 1;
                    if attempt > self.retries || !error.retryable() || cancel.is_cancelled() {
                        return Err(error);
                    }
                    // A throttle wants to be left alone for a moment, and each wait is twice
                    // the last: a quarter of a second, then half, then one.
                    let wait = Duration::from_millis(250 * (1 << (attempt - 1)));
                    tokio::time::sleep(wait).await;
                }
            }
        }
    }

    async fn send_once(
        &self,
        method: &str,
        bucket: &str,
        key: &str,
        query: &[(&str, String)],
        headers: &[(&str, String)],
        payload: Payload,
    ) -> Result<reqwest::Response> {
        let target = self.endpoint.target(bucket, key);
        let canonical_query = uri::query_string(query);
        let url = target.url(&canonical_query);

        // Two lists: what is signed, and what is also sent. `content-length` belongs to the
        // body rather than to the request's meaning, and signing it would make every retry a
        // different request to a service that computes the signature from the headers it
        // chose to include.
        let signed: Vec<(&str, String)> = headers.iter().map(|(n, v)| (*n, v.clone())).collect();
        let extra: Vec<(&str, String)> = signed
            .iter()
            .map(|(n, v)| (*n, v.clone()))
            .chain(payload.header())
            .collect();
        let mut request = self
            .http
            .request(
                reqwest::Method::from_bytes(method.as_bytes()).expect("a known method"),
                &url,
            )
            .timeout(self.timeout);
        for (name, value) in &extra {
            request = request.header(*name, value);
        }
        if let Some(signer) = &self.signer {
            let signable = Signable {
                method,
                host: &target.host,
                path: &target.path,
                query: &canonical_query,
                headers: &signed,
                payload_sha256: payload.sha256(),
            };
            for (name, value) in signer.sign(&signable, SystemTime::now()) {
                request = request.header(name, value);
            }
        }
        let request = payload.attach(request, &target.host).await?;

        let response = request.send().await.map_err(|e| S3Error::Transport {
            host: target.host.clone(),
            detail: transport_detail(&e),
        })?;
        let status = response.status();
        if status.is_success() {
            return Ok(response);
        }
        let status_code = status.as_u16();
        let body = response.text().await.unwrap_or_default();
        Err(match xml::parse_error(&body) {
            Some(error) => {
                if status_code == 404 && (error.code == "NoSuchKey" || error.code == "NotFound") {
                    S3Error::NotFound(if key.is_empty() {
                        bucket.to_string()
                    } else {
                        format!("{bucket}/{key}")
                    })
                } else {
                    S3Error::Service {
                        status: status_code,
                        code: error.code,
                        message: error.message,
                        region: error.region,
                    }
                }
            }
            // A `HEAD` has no body to explain itself with, and a proxy in the way may answer
            // with HTML. The status is then all there is, so it is what gets shown.
            None => match status_code {
                404 => S3Error::NotFound(if key.is_empty() {
                    bucket.to_string()
                } else {
                    format!("{bucket}/{key}")
                }),
                401 | 403 => S3Error::Service {
                    status: status_code,
                    code: "AccessDenied".into(),
                    message: if self.signer.is_none() {
                        "no credentials, and the bucket is not public".into()
                    } else {
                        "the credentials are not allowed to do this".into()
                    },
                    region: None,
                },
                _ => S3Error::Service {
                    status: status_code,
                    code: format!("HTTP {status_code}"),
                    message: status
                        .canonical_reason()
                        .unwrap_or("the service refused")
                        .to_string(),
                    region: None,
                },
            },
        })
    }
}

/// `Tue, 13 Aug 2024 10:11:12 GMT` — what a `HEAD` puts in `Last-Modified`, which is not the
/// form a listing uses for the same fact.
fn http_date(text: &str) -> Option<SystemTime> {
    let text = text.trim();
    let (_, rest) = text.split_once(", ")?;
    let mut parts = rest.split(' ');
    let day: u32 = parts.next()?.parse().ok()?;
    let month = match parts.next()? {
        "Jan" => 1,
        "Feb" => 2,
        "Mar" => 3,
        "Apr" => 4,
        "May" => 5,
        "Jun" => 6,
        "Jul" => 7,
        "Aug" => 8,
        "Sep" => 9,
        "Oct" => 10,
        "Nov" => 11,
        "Dec" => 12,
        _ => return None,
    };
    let year: i64 = parts.next()?.parse().ok()?;
    let mut time = parts.next()?.split(':');
    let (h, m, s): (i64, i64, i64) = (
        time.next()?.parse().ok()?,
        time.next()?.parse().ok()?,
        time.next()?.parse().ok()?,
    );
    let secs = crate::clock::days_from_civil(year, month, day) * 86_400 + h * 3600 + m * 60 + s;
    u64::try_from(secs)
        .ok()
        .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

/// What actually happened, rather than the chain of wrappers `reqwest` prints.
/// What a request carries, and what its signature says about it.
///
/// It is cloneable because [`Client::send`] retries, and a retry has to send the same body
/// again: bytes are shared rather than copied, and a file is re-read from its path. The hash
/// travels with it for the same reason — computing it once is enough, and for a file that
/// matters, because computing it means reading the file.
#[derive(Clone)]
enum Payload {
    /// No body at all: every read, a delete, a server-side copy.
    Empty,
    /// A body held in memory, signed over its bytes.
    Bytes(Arc<Vec<u8>>, String),
    /// A body streamed from a file, signed over the hash of what is in it.
    File {
        path: std::path::PathBuf,
        len: u64,
        sha256: String,
    },
}

impl Payload {
    /// What `x-amz-content-sha256` is to say. A signed request always says something here,
    /// which for an empty body is the hash of nothing.
    fn sha256(&self) -> &str {
        match self {
            Payload::Empty => EMPTY_SHA256,
            Payload::Bytes(_, sha) => sha,
            Payload::File { sha256, .. } => sha256,
        }
    }

    /// The header the body itself asks for, which is its length where there is one. It is
    /// *not* signed as an extra header — `content-length` is not in the signed set — but the
    /// service wants it, and a streamed body has no length of its own to offer.
    fn header(&self) -> Option<(&'static str, String)> {
        match self {
            Payload::Empty => None,
            Payload::Bytes(body, _) => Some(("content-length", body.len().to_string())),
            Payload::File { len, .. } => Some(("content-length", len.to_string())),
        }
    }

    /// Put the body on the request.
    async fn attach(
        self,
        request: reqwest::RequestBuilder,
        host: &str,
    ) -> Result<reqwest::RequestBuilder> {
        match self {
            Payload::Empty => Ok(request),
            // The bytes are shared, and `reqwest` wants its own: one copy per attempt, which
            // is the price of a retry that can repeat the request.
            Payload::Bytes(body, _) => Ok(request.body(body.as_ref().clone())),
            Payload::File { path, .. } => {
                let file = tokio::fs::File::open(&path)
                    .await
                    .map_err(|e| local_error(&path, e))?;
                let stream = tokio_util::io::ReaderStream::new(file);
                let _ = host;
                Ok(request.body(reqwest::Body::wrap_stream(stream)))
            }
        }
    }
}

/// A file we could not read is not the service's fault, and the message should not blame it.
fn local_error(path: &std::path::Path, error: std::io::Error) -> S3Error {
    S3Error::Local(format!("{}: {error}", path.display()))
}

fn transport_detail(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        return "timed out".to_string();
    }
    if error.is_connect() {
        return "could not connect".to_string();
    }
    let mut source: Option<&(dyn std::error::Error + 'static)> = Some(error);
    let mut last = error.to_string();
    while let Some(e) = source {
        last = e.to_string();
        source = e.source();
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fake::FakeS3;
    use std::collections::BTreeMap;

    async fn service() -> (FakeS3, Client) {
        let mut buckets = BTreeMap::new();
        let mut logs = BTreeMap::new();
        for key in [
            "2024/08/13/access.log",
            "2024/08/13/error.log",
            "2024/08/14/access.log",
            "2024/09/01/access.log",
            "readme.txt",
            "an odd & key +name.txt",
        ] {
            logs.insert(key.to_string(), format!("contents of {key}").into_bytes());
        }
        buckets.insert("logs".to_string(), logs);
        buckets.insert("empty".to_string(), BTreeMap::new());
        let fake = FakeS3::start(buckets).await;
        let client = Client::new(fake.endpoint(), Some(fake.credentials.clone())).unwrap();
        (fake, client)
    }

    /// Writing an object, deleting it, and copying one inside the service — each of them a
    /// signed request whose signature covers a body, which is the part a read never exercises.
    #[tokio::test]
    async fn an_object_can_be_written_copied_and_deleted() {
        let (fake, client) = service().await;
        let cancel = CancelToken::new();

        // Write, then read it back: the bytes that arrived are the bytes that were sent, and
        // the signature covered them — a service that disagreed would have answered 403.
        let meta = client
            .put("logs", "notes/new.txt", b"hello there".to_vec(), &cancel)
            .await
            .expect("the write is accepted");
        assert_eq!(meta.size, 11);
        let back = client
            .get("logs", "notes/new.txt", None, &cancel)
            .await
            .expect("it reads back");
        assert_eq!(back, b"hello there");

        // A key with the characters that break naive encoding, written and read.
        client
            .put("logs", "an odd & key +2.txt", b"x".to_vec(), &cancel)
            .await
            .expect("an awkward key is still a key");
        assert_eq!(
            client
                .get("logs", "an odd & key +2.txt", None, &cancel)
                .await
                .expect("it reads back"),
            b"x"
        );

        // Copy inside the service: the bytes do not come through here.
        client
            .copy_object("logs", "notes/copy.txt", "logs", "notes/new.txt", &cancel)
            .await
            .expect("the copy is accepted");
        assert_eq!(
            client
                .get("logs", "notes/copy.txt", None, &cancel)
                .await
                .expect("the copy is there"),
            b"hello there"
        );
        assert!(
            fake.requests().iter().any(|r| r.contains("notes/copy.txt")),
            "the copy went to the service: {:?}",
            fake.requests()
        );

        // Delete, and it is gone; deleting it again is still fine.
        client
            .delete_object("logs", "notes/new.txt", &cancel)
            .await
            .expect("the delete is accepted");
        assert!(matches!(
            client.get("logs", "notes/new.txt", None, &cancel).await,
            Err(S3Error::NotFound(_))
        ));
        client
            .delete_object("logs", "notes/new.txt", &cancel)
            .await
            .expect("deleting what is not there is not an error");
    }

    /// A file goes up as a stream rather than as a buffer, and the signature is over a hash
    /// computed by reading it — so what the service verifies is what is on the disk.
    #[tokio::test]
    async fn a_file_is_uploaded_as_a_stream() {
        let (_fake, client) = service().await;
        let cancel = CancelToken::new();
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("payload.bin");
        // Larger than one read, so the streaming path is actually a stream.
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&file, &body).unwrap();

        let sent = client
            .put_file("logs", "big/payload.bin", &file, &cancel)
            .await
            .expect("the upload is accepted");
        assert_eq!(sent, body.len() as u64);
        let back = client
            .get("logs", "big/payload.bin", None, &cancel)
            .await
            .expect("it reads back");
        assert_eq!(back.len(), body.len());
        assert_eq!(back, body, "every byte arrived as it was");

        // An empty file is a file: zero bytes still hash to something, and the object exists.
        let empty = dir.path().join("empty");
        std::fs::write(&empty, b"").unwrap();
        assert_eq!(
            client
                .put_file("logs", "big/empty", &empty, &cancel)
                .await
                .expect("an empty object is allowed"),
            0
        );
        assert!(client
            .head("logs", "big/empty", &cancel)
            .await
            .is_ok_and(|m| m.size == 0));
    }

    /// A write to a bucket that is not there says which bucket, and a write with no
    /// credentials is refused by the service rather than by a guess here.
    #[tokio::test]
    async fn a_write_that_cannot_work_says_why() {
        let (fake, client) = service().await;
        let cancel = CancelToken::new();
        let error = client
            .put("nope", "k", b"x".to_vec(), &cancel)
            .await
            .expect_err("there is no such bucket");
        assert!(error.to_string().contains("NoSuchBucket"), "{error}");

        let anonymous = Client::new(fake.endpoint(), None).unwrap();
        let error = anonymous
            .put("logs", "k", b"x".to_vec(), &cancel)
            .await
            .expect_err("an anonymous write is refused");
        assert!(error.to_string().contains("AccessDenied"), "{error}");
    }

    /// The whole round trip, against a service that recomputes the signature from the request
    /// line it received: if the URL and the canonical request disagreed about a single byte
    /// of encoding, this is a 403 rather than a listing.
    #[tokio::test]
    async fn a_prefix_reads_as_a_directory() {
        let (_fake, client) = service().await;
        let cancel = CancelToken::new();
        let page = client
            .list_page("logs", "", Some("/"), None, &cancel)
            .await
            .unwrap();
        assert_eq!(page.prefixes, vec!["2024/"]);
        let mut names: Vec<&str> = page.objects.iter().map(|o| o.key.as_str()).collect();
        names.sort_unstable();
        assert_eq!(names, vec!["an odd & key +name.txt", "readme.txt"]);
        assert!(!page.truncated);

        let deeper = client
            .list_page("logs", "2024/08/", Some("/"), None, &cancel)
            .await
            .unwrap();
        assert_eq!(deeper.prefixes, vec!["2024/08/13/", "2024/08/14/"]);
        assert!(deeper.objects.is_empty());
        assert_eq!(deeper.prefix, "2024/08/");
    }

    /// Without a delimiter the answer is every key under the prefix, however deep. This is the
    /// shape a bucket with a million objects is read in, so it had better be right.
    #[tokio::test]
    async fn no_delimiter_means_everything_underneath() {
        let (_fake, client) = service().await;
        let page = client
            .list_page("logs", "2024/", None, None, &CancelToken::new())
            .await
            .unwrap();
        assert_eq!(page.objects.len(), 4);
        assert!(page.prefixes.is_empty());
    }

    /// A listing is pages, and the client must follow them. The service here hands out three
    /// keys at a time, so forgetting the token loses eleven of fourteen rows.
    #[tokio::test]
    async fn the_pages_are_followed_to_the_end() {
        let mut buckets = BTreeMap::new();
        buckets.insert("big".to_string(), FakeS3::many("data/", 14));
        let fake = FakeS3::start(buckets).await;
        fake.page_limit(3);
        let client = Client::new(fake.endpoint(), Some(fake.credentials.clone())).unwrap();
        let cancel = CancelToken::new();

        let mut keys = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let page = client
                .list_page("big", "data/", None, token.as_deref(), &cancel)
                .await
                .unwrap();
            keys.extend(page.objects.iter().map(|o| o.key.clone()));
            match page.next_token {
                Some(next) if page.truncated => token = Some(next),
                _ => break,
            }
        }
        assert_eq!(keys.len(), 14, "{keys:?}");
        assert_eq!(keys[0], "data/000000.txt");
        assert_eq!(keys[13], "data/000013.txt");
        // Five pages of three, four full and one short.
        assert_eq!(
            fake.requests()
                .iter()
                .filter(|r| r.contains("list-type=2"))
                .count(),
            5
        );
    }

    #[tokio::test]
    async fn an_object_is_read_whole_or_by_the_window() {
        let (_fake, client) = service().await;
        let cancel = CancelToken::new();
        let all = client
            .get("logs", "readme.txt", None, &cancel)
            .await
            .unwrap();
        assert_eq!(all, b"contents of readme.txt");

        // The window the viewer asks for.
        let window = client
            .get("logs", "readme.txt", Some((9, Some(2))), &cancel)
            .await
            .unwrap();
        assert_eq!(window, b"of");

        // From an offset to the end.
        let tail = client
            .get("logs", "readme.txt", Some((12, None)), &cancel)
            .await
            .unwrap();
        assert_eq!(tail, b"readme.txt");

        // A key with a space, an ampersand and a plus in it: encoded once, signed as sent.
        let odd = client
            .get("logs", "an odd & key +name.txt", None, &cancel)
            .await
            .unwrap();
        assert_eq!(odd, b"contents of an odd & key +name.txt");
    }

    #[tokio::test]
    async fn a_head_says_what_an_object_is_without_reading_it() {
        let (_fake, client) = service().await;
        let meta = client
            .head("logs", "readme.txt", &CancelToken::new())
            .await
            .unwrap();
        assert_eq!(meta.size, 22);
        assert_eq!(
            meta.modified,
            crate::clock::parse_iso8601("2024-08-13T10:11:12Z")
        );
        assert_eq!(
            meta.etag.as_deref(),
            Some("00000000000000000000000000000016")
        );
    }

    #[tokio::test]
    async fn the_buckets_can_be_listed_and_not_anonymously() {
        let (fake, client) = service().await;
        let names: Vec<String> = client
            .list_buckets(&CancelToken::new())
            .await
            .unwrap()
            .into_iter()
            .map(|b| b.name)
            .collect();
        assert_eq!(names, vec!["empty", "logs"]);

        let anonymous = Client::new(fake.endpoint(), None).unwrap();
        let err = anonymous
            .list_buckets(&CancelToken::new())
            .await
            .unwrap_err();
        assert!(matches!(err, S3Error::NoCredentials(_)), "{err:?}");
    }

    #[tokio::test]
    async fn what_is_missing_says_so_and_what_is_refused_says_why() {
        let (fake, client) = service().await;
        let cancel = CancelToken::new();
        let missing = client
            .get("logs", "nope.txt", None, &cancel)
            .await
            .unwrap_err();
        assert!(matches!(missing, S3Error::NotFound(_)), "{missing:?}");

        let no_bucket = client
            .list_page("nosuch", "", Some("/"), None, &cancel)
            .await
            .unwrap_err();
        match no_bucket {
            S3Error::Service { code, status, .. } => {
                assert_eq!(code, "NoSuchBucket");
                assert_eq!(status, 404);
            }
            other => panic!("{other:?}"),
        }

        // An anonymous request to a bucket that is not public.
        let anonymous = Client::new(fake.endpoint(), None).unwrap();
        let err = anonymous
            .get("logs", "readme.txt", None, &cancel)
            .await
            .unwrap_err();
        match err {
            S3Error::Service { code, status, .. } => {
                assert_eq!(code, "AccessDenied");
                assert_eq!(status, 403);
            }
            other => panic!("{other:?}"),
        }

        // And the credentials being wrong is a different sentence from the key being absent.
        let wrong = Client::new(
            fake.endpoint(),
            Some(Credentials::new("AKIAIOSFODNN7EXAMPLE", "not-the-secret")),
        )
        .unwrap();
        let err = wrong
            .get("logs", "readme.txt", None, &cancel)
            .await
            .unwrap_err();
        assert!(
            matches!(&err, S3Error::Service { code, .. } if code == "SignatureDoesNotMatch"),
            "{err:?}"
        );
    }

    /// A bucket with a public read policy is read without credentials at all, which is how
    /// an open data set is browsed.
    #[tokio::test]
    async fn a_public_bucket_needs_no_credentials() {
        let (fake, _signed) = service().await;
        fake.public();
        let anonymous = Client::new(fake.endpoint(), None).unwrap();
        let cancel = CancelToken::new();
        let page = anonymous
            .list_page("logs", "", Some("/"), None, &cancel)
            .await
            .unwrap();
        assert_eq!(page.prefixes, vec!["2024/"]);
        assert_eq!(
            anonymous
                .get("logs", "readme.txt", None, &cancel)
                .await
                .unwrap(),
            b"contents of readme.txt"
        );
        // No Authorization header was sent at all.
        assert!(
            !fake.requests().is_empty(),
            "the requests should have been seen"
        );
    }

    /// A throttle is waited out, not reported.
    #[tokio::test]
    async fn a_slow_down_is_retried() {
        let (fake, client) = service().await;
        fake.throttle(2);
        let page = client
            .list_page("logs", "", Some("/"), None, &CancelToken::new())
            .await
            .unwrap();
        assert_eq!(page.prefixes, vec!["2024/"]);
        assert_eq!(fake.requests().len(), 3);

        // But not for ever: the fourth refusal in a row is the answer.
        fake.throttle(9);
        let err = client
            .list_page("logs", "", Some("/"), None, &CancelToken::new())
            .await
            .unwrap_err();
        assert!(
            matches!(&err, S3Error::Service { code, .. } if code == "SlowDown"),
            "{err:?}"
        );
    }

    #[tokio::test]
    async fn a_cancelled_listing_stops_where_it_is() {
        let (fake, client) = service().await;
        fake.throttle(3);
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = client
            .list_page("logs", "", Some("/"), None, &cancel)
            .await
            .unwrap_err();
        assert!(matches!(err, S3Error::Cancelled), "{err:?}");
        assert!(fake.requests().is_empty(), "nothing should have been sent");
    }

    #[test]
    fn a_last_modified_header_is_read() {
        let when = http_date("Tue, 13 Aug 2024 10:11:12 GMT").unwrap();
        assert_eq!(
            when,
            crate::clock::parse_iso8601("2024-08-13T10:11:12Z").unwrap()
        );
        assert!(http_date("").is_none());
        assert!(http_date("not a date").is_none());
        assert!(http_date("Tue, 13 Rot 2024 10:11:12 GMT").is_none());
    }

    #[test]
    fn a_region_is_followed_by_name_not_by_guess() {
        let client = Client::new(Endpoint::aws("us-east-1"), None).unwrap();
        let moved = client.in_region("eu-west-1").unwrap();
        assert_eq!(moved.endpoint().host, "s3.eu-west-1.amazonaws.com");
        assert_eq!(moved.endpoint().region, "eu-west-1");

        // Somewhere that is not AWS keeps its host and only changes what is signed.
        let other = Client::new(Endpoint::parse("http://minio.lan:9000", "us-east-1"), None)
            .unwrap()
            .in_region("local");
        let other = other.unwrap();
        assert_eq!(other.endpoint().host, "minio.lan:9000");
        assert_eq!(other.endpoint().region, "local");
    }

    #[test]
    fn what_is_worth_trying_again() {
        let throttled = S3Error::Service {
            status: 503,
            code: "SlowDown".into(),
            message: String::new(),
            region: None,
        };
        assert!(throttled.retryable());
        assert!(S3Error::Transport {
            host: "h".into(),
            detail: "timed out".into()
        }
        .retryable());
        assert!(!S3Error::Service {
            status: 403,
            code: "SignatureDoesNotMatch".into(),
            message: String::new(),
            region: None,
        }
        .retryable());
        assert!(!S3Error::NotFound("b/k".into()).retryable());
        assert!(!S3Error::Cancelled.retryable());
    }
}
