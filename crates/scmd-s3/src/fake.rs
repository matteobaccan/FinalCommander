//! A service that speaks S3, in this process, for the tests.
//!
//! There is no AWS account in a test run and there must not be one, so the only way to test
//! what the client actually puts on the wire is to answer it. This is a small HTTP server with
//! a map of buckets behind it, and it is deliberately strict in two ways the real thing is:
//!
//! It verifies the signature, recomputed from the request line it received rather than from
//! anything the client passed in. That catches the whole class of bug where the URL and the
//! canonical request disagree — a key encoded twice, a query sorted differently, a port in the
//! host header — which is otherwise a `SignatureDoesNotMatch` from a service far away with no
//! indication of which byte was wrong.
//!
//! And it pages. `max-keys` is honoured and can be set to three, so a listing that forgets to
//! follow the continuation token fails here rather than in a bucket with a million objects in
//! it.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::sign::{Credentials, Signable, Signer, Stamp};
use crate::uri;

/// What the fake service holds and how it is meant to behave.
#[derive(Debug, Default)]
pub struct Store {
    /// bucket → key → bytes.
    pub buckets: BTreeMap<String, BTreeMap<String, Vec<u8>>>,
    /// The largest page the service will answer with, whatever is asked for.
    pub page_limit: usize,
    /// Fail the next `n` requests with a `503 SlowDown`, to exercise the retry.
    pub throttle: usize,
    /// Requests served, by method and path, for a test that wants to count round trips.
    pub seen: Vec<String>,
    /// Refuse everything with this code, if set.
    pub refuse: Option<(u16, String, String)>,
    /// Serve unsigned requests, the way a bucket with a public read policy does. Off by
    /// default, because a private bucket is the case that has to work.
    pub public: bool,
}

/// A running fake service.
pub struct FakeS3 {
    pub store: Arc<Mutex<Store>>,
    pub address: String,
    pub credentials: Credentials,
}

impl FakeS3 {
    /// Start one. The address is `127.0.0.1:<port>`, with the port the system chose.
    pub async fn start(buckets: BTreeMap<String, BTreeMap<String, Vec<u8>>>) -> FakeS3 {
        let credentials = Credentials::new("AKIAIOSFODNN7EXAMPLE", "secretsecretsecret");
        let store = Arc::new(Mutex::new(Store {
            buckets,
            page_limit: 1000,
            ..Store::default()
        }));
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("a port");
        let address = listener.local_addr().expect("an address").to_string();
        let server = Server {
            store: store.clone(),
            signer: Signer::new(credentials.clone(), "us-east-1"),
        };
        tokio::spawn(async move {
            loop {
                let Ok((socket, _)) = listener.accept().await else {
                    return;
                };
                let server = server.clone();
                tokio::spawn(async move { server.serve(socket).await });
            }
        });
        FakeS3 {
            store,
            address,
            credentials,
        }
    }

    /// A bucket with `count` objects under `prefix`, named with a counter.
    pub fn many(prefix: &str, count: usize) -> BTreeMap<String, Vec<u8>> {
        (0..count)
            .map(|i| {
                (
                    format!("{prefix}{i:06}.txt"),
                    format!("row {i}").into_bytes(),
                )
            })
            .collect()
    }

    pub fn endpoint(&self) -> crate::endpoint::Endpoint {
        crate::endpoint::Endpoint::parse(&format!("http://{}", self.address), "us-east-1")
    }

    pub fn page_limit(&self, keys: usize) {
        self.store.lock().expect("the store").page_limit = keys;
    }

    pub fn throttle(&self, requests: usize) {
        self.store.lock().expect("the store").throttle = requests;
    }

    /// Let anonymous requests through, as a bucket with a public read policy does.
    pub fn public(&self) {
        self.store.lock().expect("the store").public = true;
    }

    pub fn refuse(&self, status: u16, code: &str, message: &str) {
        self.store.lock().expect("the store").refuse =
            Some((status, code.to_string(), message.to_string()));
    }

    pub fn requests(&self) -> Vec<String> {
        self.store.lock().expect("the store").seen.clone()
    }
}

#[derive(Clone)]
struct Server {
    store: Arc<Mutex<Store>>,
    signer: Signer,
}

impl Server {
    async fn serve(&self, mut socket: tokio::net::TcpStream) {
        let mut buffer = Vec::new();
        let mut chunk = [0u8; 4096];
        // Read until the end of the headers. Nothing here has a body.
        loop {
            let read = match socket.read(&mut chunk).await {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };
            buffer.extend_from_slice(&chunk[..read]);
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let text = String::from_utf8_lossy(&buffer).into_owned();
        let mut lines = text.split("\r\n");
        let Some(request_line) = lines.next() else {
            return;
        };
        let mut parts = request_line.split(' ');
        let (Some(method), Some(target)) = (parts.next(), parts.next()) else {
            return;
        };
        let mut headers: Vec<(String, String)> = Vec::new();
        for line in lines {
            if line.is_empty() {
                break;
            }
            if let Some((name, value)) = line.split_once(':') {
                headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
            }
        }
        let header = |name: &str| {
            headers
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
        };

        let (path, query) = match target.split_once('?') {
            Some((p, q)) => (p, q),
            None => (target, ""),
        };
        self.store
            .lock()
            .expect("the store")
            .seen
            .push(format!("{method} {target}"));

        let response = self.answer(method, path, query, &headers, &header);
        let _ = socket.write_all(&response).await;
        let _ = socket.shutdown().await;
    }

    fn answer(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &[(String, String)],
        header: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<u8> {
        if header("authorization").is_none() {
            if !self.store.lock().expect("the store").public {
                return error_response(
                    403,
                    "AccessDenied",
                    "anonymous access is not allowed on this bucket",
                );
            }
        } else if let Some(bad) = self.signature_problem(method, path, query, headers, header) {
            return error_response(403, "SignatureDoesNotMatch", &bad);
        }
        {
            let mut store = self.store.lock().expect("the store");
            if store.throttle > 0 {
                store.throttle -= 1;
                return error_response(503, "SlowDown", "please reduce your request rate");
            }
            if let Some((status, code, message)) = store.refuse.clone() {
                return error_response(status, &code, &message);
            }
        }

        // Path style only: `/`, `/bucket/`, `/bucket/key`.
        let trimmed = path.trim_start_matches('/');
        let (bucket, key) = match trimmed.split_once('/') {
            Some((bucket, key)) => (bucket.to_string(), uri::decode(key)),
            None => (trimmed.to_string(), String::new()),
        };

        if bucket.is_empty() {
            return self.list_buckets();
        }
        let params: BTreeMap<String, String> = query
            .split('&')
            .filter(|p| !p.is_empty())
            .map(|pair| {
                let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
                (uri::decode(k), uri::decode(v))
            })
            .collect();
        if params.get("list-type").map(|v| v == "2").unwrap_or(false) {
            return self.list_objects(&bucket, &params);
        }
        if key.is_empty() {
            return error_response(405, "MethodNotAllowed", "nothing to do with a bucket");
        }
        self.object(method, &bucket, &key, header)
    }

    /// Recompute the signature from what arrived. `None` when it matches.
    fn signature_problem(
        &self,
        method: &str,
        path: &str,
        query: &str,
        headers: &[(String, String)],
        header: &dyn Fn(&str) -> Option<String>,
    ) -> Option<String> {
        let authorization = header("authorization")?;
        let date = header("x-amz-date")?;
        let payload = header("x-amz-content-sha256")?;
        let host = header("host")?;
        let signed_names: Vec<String> = authorization
            .split("SignedHeaders=")
            .nth(1)?
            .split(',')
            .next()?
            .split(';')
            .map(|s| s.to_string())
            .collect();
        // Everything the client said it signed, apart from the three the signer adds itself.
        let extra: Vec<(&str, String)> = signed_names
            .iter()
            .filter(|n| {
                !matches!(
                    n.as_str(),
                    "host" | "x-amz-date" | "x-amz-content-sha256" | "x-amz-security-token"
                )
            })
            .filter_map(|name| {
                headers
                    .iter()
                    .find(|(n, _)| n == name)
                    .map(|(n, v)| (n.as_str(), v.clone()))
            })
            .collect();
        // The canonical query is the pairs of the request line, sorted as they stand.
        let mut pairs: Vec<&str> = query.split('&').filter(|p| !p.is_empty()).collect();
        pairs.sort_unstable();
        let canonical_query = pairs.join("&");
        let stamp = Stamp {
            date: date.get(..8)?.to_string(),
            datetime: date.clone(),
        };
        let expected = self.signer.sign_at(
            &Signable {
                method,
                host: &host,
                path,
                query: &canonical_query,
                headers: &extra,
                payload_sha256: &payload,
            },
            &stamp,
        );
        let expected = expected
            .iter()
            .find(|(n, _)| n == "authorization")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        if expected == authorization {
            None
        } else {
            Some(format!(
                "the signature does not match\n  sent:     {authorization}\n  expected: {expected}"
            ))
        }
    }

    fn list_buckets(&self) -> Vec<u8> {
        let store = self.store.lock().expect("the store");
        let mut body = String::from(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListAllMyBucketsResult><Buckets>",
        );
        for name in store.buckets.keys() {
            body.push_str(&format!(
                "<Bucket><Name>{name}</Name><CreationDate>2020-01-02T03:04:05.000Z</CreationDate></Bucket>"
            ));
        }
        body.push_str("</Buckets></ListAllMyBucketsResult>");
        ok_response(body.into_bytes(), &[])
    }

    fn list_objects(&self, bucket: &str, params: &BTreeMap<String, String>) -> Vec<u8> {
        let store = self.store.lock().expect("the store");
        let Some(objects) = store.buckets.get(bucket) else {
            return error_response(404, "NoSuchBucket", "the specified bucket does not exist");
        };
        let prefix = params.get("prefix").cloned().unwrap_or_default();
        let delimiter = params.get("delimiter").cloned().unwrap_or_default();
        let after = params
            .get("continuation-token")
            .cloned()
            .unwrap_or_default();
        let wanted: usize = params
            .get("max-keys")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1000);
        let limit = wanted.min(store.page_limit).max(1);

        let mut keys = Vec::new();
        let mut prefixes: Vec<String> = Vec::new();
        let mut truncated = false;
        let mut next = None;
        for (key, bytes) in objects.iter() {
            // The token names the first key not yet returned, and resuming includes it. A
            // service that excluded it would drop one key per page, which is the kind of bug
            // a test double exists to not have.
            if !key.starts_with(&prefix) || (!after.is_empty() && key.as_str() < after.as_str()) {
                continue;
            }
            let rest = &key[prefix.len()..];
            if !delimiter.is_empty() {
                if let Some(cut) = rest.find(&delimiter) {
                    let common = format!("{prefix}{}{delimiter}", &rest[..cut]);
                    if !prefixes.contains(&common) {
                        if keys.len() + prefixes.len() >= limit {
                            truncated = true;
                            next = Some(key.clone());
                            break;
                        }
                        prefixes.push(common);
                    }
                    continue;
                }
            }
            if keys.len() + prefixes.len() >= limit {
                truncated = true;
                next = Some(key.clone());
                break;
            }
            keys.push((key.clone(), bytes.len()));
        }

        let mut body = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?><ListBucketResult><Name>{bucket}</Name>\
             <Prefix>{}</Prefix><EncodingType>url</EncodingType><IsTruncated>{}</IsTruncated>",
            uri::encode_query(&prefix),
            truncated
        );
        if let Some(next) = next {
            // The real service's token is opaque base64; this one is the key to carry on from,
            // which is just as opaque to the client and much easier to read in a failure. It
            // is *not* percent-encoded: `encoding-type` covers keys and prefixes, not the
            // token, and a client that re-encoded one would ask for a page that does not
            // exist — which is exactly the bug this double caught.
            body.push_str(&format!(
                "<NextContinuationToken>{}</NextContinuationToken>",
                escape(&next)
            ));
        }
        for (key, size) in keys {
            body.push_str(&format!(
                "<Contents><Key>{}</Key><LastModified>2024-08-13T10:11:12.000Z</LastModified>\
                 <ETag>&quot;{:032x}&quot;</ETag><Size>{size}</Size>\
                 <StorageClass>STANDARD</StorageClass></Contents>",
                uri::encode_query(&key),
                size
            ));
        }
        for common in prefixes {
            body.push_str(&format!(
                "<CommonPrefixes><Prefix>{}</Prefix></CommonPrefixes>",
                uri::encode_query(&common)
            ));
        }
        body.push_str("</ListBucketResult>");
        ok_response(body.into_bytes(), &[])
    }

    fn object(
        &self,
        method: &str,
        bucket: &str,
        key: &str,
        header: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<u8> {
        let store = self.store.lock().expect("the store");
        let Some(bytes) = store.buckets.get(bucket).and_then(|b| b.get(key)) else {
            return error_response(404, "NoSuchKey", "the specified key does not exist");
        };
        let extra = vec![
            ("Last-Modified", "Tue, 13 Aug 2024 10:11:12 GMT".to_string()),
            ("ETag", format!("\"{:032x}\"", bytes.len())),
        ];
        if method == "HEAD" {
            let mut response = ok_response(Vec::new(), &extra);
            // A HEAD answers with the length it would have sent.
            let header = format!("Content-Length: {}", bytes.len());
            let text = String::from_utf8_lossy(&response).replace("Content-Length: 0", &header);
            response = text.into_bytes();
            return response;
        }
        match header("range") {
            Some(range) => {
                let spec = range.trim().trim_start_matches("bytes=");
                let (from, to) = spec.split_once('-').unwrap_or((spec, ""));
                let from: usize = from.parse().unwrap_or(0);
                let to: usize = to.parse().unwrap_or(bytes.len().saturating_sub(1));
                if from >= bytes.len() {
                    return error_response(
                        416,
                        "InvalidRange",
                        "the requested range is not satisfiable",
                    );
                }
                let to = to.min(bytes.len() - 1);
                let slice = bytes[from..=to].to_vec();
                let mut response = String::from("HTTP/1.1 206 Partial Content\r\n");
                response.push_str(&format!(
                    "Content-Range: bytes {from}-{to}/{}\r\n",
                    bytes.len()
                ));
                response.push_str(&format!("Content-Length: {}\r\n", slice.len()));
                response.push_str(
                    "Content-Type: application/octet-stream\r\nConnection: close\r\n\r\n",
                );
                let mut out = response.into_bytes();
                out.extend_from_slice(&slice);
                out
            }
            None => ok_response(bytes.clone(), &extra),
        }
    }
}

/// The three characters that cannot appear in XML content as themselves.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn ok_response(body: Vec<u8>, extra: &[(&str, String)]) -> Vec<u8> {
    let mut head = String::from("HTTP/1.1 200 OK\r\n");
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Content-Type: application/xml\r\n");
    for (name, value) in extra {
        head.push_str(&format!("{name}: {value}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    let mut out = head.into_bytes();
    out.extend_from_slice(&body);
    out
}

fn error_response(status: u16, code: &str, message: &str) -> Vec<u8> {
    let body = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?><Error><Code>{code}</Code>\
         <Message>{message}</Message><RequestId>fake</RequestId></Error>"
    );
    let reason = match status {
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        503 => "Service Unavailable",
        _ => "Error",
    };
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    head.push_str(&format!("Content-Length: {}\r\n", body.len()));
    head.push_str("Content-Type: application/xml\r\nConnection: close\r\n\r\n");
    let mut out = head.into_bytes();
    out.extend_from_slice(body.as_bytes());
    out
}
