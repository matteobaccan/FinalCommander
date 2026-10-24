//! The signature an S3 request carries: AWS Signature Version 4.
//!
//! A request is signed by building a *canonical* form of it — method, path, query, a chosen
//! set of headers, the hash of the body — hashing that, and signing the hash with a key
//! derived from the secret, the date, the region and the service. The server repeats the
//! derivation and the comparison. Nothing about it is negotiable: a byte out of place in the
//! canonical form is a `SignatureDoesNotMatch` and no hint as to where.
//!
//! Which is why the tests here are not of my own making. They come from `botocore`, the
//! library the AWS command line itself signs with, run against a frozen clock — and one of
//! them, the ranged `GET` of `examplebucket/test.txt` on 24 May 2013, is the worked example
//! AWS publishes with the signature spelled out, so the chain is anchored at both ends.

use std::time::{SystemTime, UNIX_EPOCH};

use ring::{digest, hmac};

/// Who we are to the service. The secret is held for as long as the connection is: there is no
/// way to sign without it, and SigV4 needs it again for every request.
#[derive(Clone, PartialEq, Eq)]
pub struct Credentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    /// Set for temporary credentials (a role, `aws sts assume-role`, an instance profile).
    pub session_token: Option<String>,
}

impl Credentials {
    pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
        Credentials {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: None,
        }
    }

    pub fn with_session_token(mut self, token: impl Into<String>) -> Self {
        self.session_token = Some(token.into());
        self
    }
}

/// Deliberately not `Debug`-printable in full: a secret that reaches a log is a secret gone.
impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"<hidden>")
            .field(
                "session_token",
                &self.session_token.as_ref().map(|_| "<hidden>"),
            )
            .finish()
    }
}

/// The request, in the only shape the signature cares about.
///
/// `path` and `query` are already in the form that will go on the wire, and the signature uses
/// them exactly as given: S3 is the one service that does not normalise the path before
/// signing it, so a key with a `+` or a space in it is encoded once, here, and both the URL and
/// the canonical request carry the same bytes.
pub struct Signable<'a> {
    pub method: &'a str,
    /// `host` or `host:port`, with the port left out when it is the scheme's own.
    pub host: &'a str,
    /// The encoded path, beginning with `/`.
    pub path: &'a str,
    /// The canonical query string: sorted, encoded, without the `?`. Empty when there is none.
    pub query: &'a str,
    /// Extra headers to sign, lowercase names. `host`, `x-amz-date`, `x-amz-content-sha256`
    /// and the session token are added here and must not be repeated.
    pub headers: &'a [(&'a str, String)],
    /// Hex SHA-256 of the body — of the empty string for a `GET` or a `HEAD`.
    pub payload_sha256: &'a str,
}

/// The empty body every read request has, hashed once.
pub const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

/// Signs requests for one region and one service.
#[derive(Debug, Clone)]
pub struct Signer {
    pub credentials: Credentials,
    pub region: String,
    pub service: &'static str,
}

impl Signer {
    pub fn new(credentials: Credentials, region: impl Into<String>) -> Signer {
        Signer {
            credentials,
            region: region.into(),
            service: "s3",
        }
    }

    /// The headers to add to the request, `Authorization` among them.
    pub fn sign(&self, req: &Signable<'_>, now: SystemTime) -> Vec<(String, String)> {
        self.sign_at(req, &Stamp::at(now))
    }

    /// The same, at a time given rather than taken from the clock. A server verifying a
    /// signature needs this: the stamp to check against is the one the request carries, and
    /// recomputing from the clock would reject every request that spent a second in flight.
    pub fn sign_at(&self, req: &Signable<'_>, stamp: &Stamp) -> Vec<(String, String)> {
        let stamp = stamp.clone();
        let mut signed: Vec<(String, String)> = vec![
            ("host".into(), req.host.to_string()),
            ("x-amz-content-sha256".into(), req.payload_sha256.into()),
            ("x-amz-date".into(), stamp.datetime.clone()),
        ];
        if let Some(token) = &self.credentials.session_token {
            signed.push(("x-amz-security-token".into(), token.clone()));
        }
        for (name, value) in req.headers {
            signed.push((name.to_ascii_lowercase(), value.clone()));
        }
        signed.sort_by(|a, b| a.0.cmp(&b.0));

        let names: Vec<&str> = signed.iter().map(|(n, _)| n.as_str()).collect();
        let signed_headers = names.join(";");
        let mut canonical = String::with_capacity(256);
        canonical.push_str(req.method);
        canonical.push('\n');
        canonical.push_str(req.path);
        canonical.push('\n');
        canonical.push_str(req.query);
        canonical.push('\n');
        for (name, value) in &signed {
            canonical.push_str(name);
            canonical.push(':');
            canonical.push_str(value.trim());
            canonical.push('\n');
        }
        canonical.push('\n');
        canonical.push_str(&signed_headers);
        canonical.push('\n');
        canonical.push_str(req.payload_sha256);

        let scope = format!(
            "{}/{}/{}/aws4_request",
            stamp.date, self.region, self.service
        );
        let to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            stamp.datetime,
            scope,
            hex(digest::digest(&digest::SHA256, canonical.as_bytes()).as_ref())
        );

        let key = self.signing_key(&stamp.date);
        let signature = hex(hmac::sign(&key, to_sign.as_bytes()).as_ref());
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.credentials.access_key_id, scope, signed_headers, signature
        );

        let mut out = vec![
            ("x-amz-date".to_string(), stamp.datetime),
            (
                "x-amz-content-sha256".to_string(),
                req.payload_sha256.to_string(),
            ),
            ("authorization".to_string(), authorization),
        ];
        if let Some(token) = &self.credentials.session_token {
            out.push(("x-amz-security-token".to_string(), token.clone()));
        }
        out
    }

    /// `HMAC(HMAC(HMAC(HMAC("AWS4" + secret, date), region), service), "aws4_request")` — the
    /// chain that makes a leaked signature worth one day in one region for one service.
    fn signing_key(&self, date: &str) -> hmac::Key {
        let first = format!("AWS4{}", self.credentials.secret_access_key);
        let mut key = hmac::Key::new(hmac::HMAC_SHA256, first.as_bytes());
        for part in [date, &self.region, self.service, "aws4_request"] {
            let tag = hmac::sign(&key, part.as_bytes());
            key = hmac::Key::new(hmac::HMAC_SHA256, tag.as_ref());
        }
        key
    }
}

/// `20130524` and `20130524T000000Z`, which is the only shape of time SigV4 knows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamp {
    pub date: String,
    pub datetime: String,
}

impl Stamp {
    pub fn at(now: SystemTime) -> Stamp {
        let secs = now
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
        let (y, m, d) = crate::clock::civil_from_days(days);
        let (h, mi, s) = (rest / 3600, (rest % 3600) / 60, rest % 60);
        Stamp {
            date: format!("{y:04}{m:02}{d:02}"),
            datetime: format!("{y:04}{m:02}{d:02}T{h:02}{mi:02}{s:02}Z"),
        }
    }
}

/// Lowercase hexadecimal, which is what every field of a signature is written in.
pub fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(char::from_digit((b >> 4) as u32, 16).unwrap());
        out.push(char::from_digit((b & 0xf) as u32, 16).unwrap());
    }
    out
}

/// Hex SHA-256 of a body.
pub fn sha256_hex(body: &[u8]) -> String {
    hex(digest::digest(&digest::SHA256, body).as_ref())
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "AKIAIOSFODNN7EXAMPLE";
    const SECRET: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    /// 24 May 2013, 00:00:00 UTC — the date of the worked example in the AWS documentation.
    const WHEN: u64 = 1_369_353_600;

    fn signer(region: &str) -> Signer {
        Signer::new(Credentials::new(KEY, SECRET), region)
    }

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + std::time::Duration::from_secs(secs)
    }

    fn signature_of(headers: &[(String, String)]) -> String {
        let auth = headers
            .iter()
            .find(|(n, _)| n == "authorization")
            .expect("an Authorization header");
        auth.1
            .split("Signature=")
            .nth(1)
            .expect("a signature in it")
            .to_string()
    }

    /// Every one of these is `botocore` signing the same request against a frozen clock, and
    /// the second is additionally the example AWS publishes the signature of.
    #[test]
    fn the_signature_is_the_one_the_aws_tooling_produces() {
        let empty: &[(&str, String)] = &[];

        let plain = signer("us-east-1").sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/test.txt",
                query: "",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&plain),
            "df548e2ce037944d03f3e68682813b093763996d597cf890ca3d9037fd231eb4"
        );

        // The documented example: GET examplebucket/test.txt with a Range header.
        let ranged = signer("us-east-1").sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/test.txt",
                query: "",
                headers: &[("range", "bytes=0-9".to_string())],
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&ranged),
            "f0e8bdb87c964420e857bd35b5d6ed310bd44f0170aba48dd91039c6036bdb41"
        );

        let listing = signer("eu-west-1").sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/",
                query:
                    "delimiter=%2F&encoding-type=url&list-type=2&max-keys=1000&prefix=2024%2F08%2F",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&listing),
            "c71ca83ad6424799a9092f81d6378ae4bd93a58135824b92a2e758ccce67adb2"
        );

        let next_page = signer("eu-central-1").sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/",
                query: "continuation-token=1ueGcxLPRx1Tr%2FXYExHnhbYLgveDs2J%2Fwm36Hy4vbOwM%3D&encoding-type=url&list-type=2",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&next_page),
            "eb0fcd2159e0a43f0cca06106068b76dcfe98b43306ed8f25df96f79f39a9570"
        );

        // A key with a space, an ampersand, a plus and two letters outside ASCII in it: the
        // path is encoded once and signed as it stands.
        let odd = signer("us-west-2").sign(
            &Signable {
                method: "HEAD",
                host: "examplebucket.s3.amazonaws.com",
                path: "/folder%20name/%C3%BCn%C3%AFcode%20%26%20special%2Bchars.txt",
                query: "",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&odd),
            "d8c27c57d8d25aa41e2b79548d066a63646cf8b296da7d1e3ef041470c48a956"
        );

        // Path style against something that is not AWS at all, port and all.
        let minio = signer("us-east-1").sign(
            &Signable {
                method: "GET",
                host: "127.0.0.1:9000",
                path: "/mybucket/",
                query: "encoding-type=url&list-type=2&prefix=x",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&minio),
            "059a47c772d4830078ba27ad57d675a9654e96850430d43a894ff14cecc012b6"
        );

        // Temporary credentials: the token is signed, not merely sent.
        let temporary = Signer::new(
            Credentials::new(KEY, SECRET).with_session_token("SESSIONTOKEN/with+chars=="),
            "us-east-1",
        )
        .sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/",
                query: "",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&temporary),
            "0ed20005ed2d6009169d6c2339548aecea5b5e470740d9a36a9071d08beac767"
        );
        assert!(temporary
            .iter()
            .any(|(n, v)| n == "x-amz-security-token" && v == "SESSIONTOKEN/with+chars=="));
        assert!(signature_of(&temporary)
            .chars()
            .all(|c| c.is_ascii_hexdigit()));

        let buckets = signer("us-east-1").sign(
            &Signable {
                method: "GET",
                host: "s3.us-east-1.amazonaws.com",
                path: "/",
                query: "",
                headers: empty,
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        assert_eq!(
            signature_of(&buckets),
            "8745d16e49fb5550634d56c2c4bb6841e42d7595f8529cf9ea14d05d51935b20"
        );
    }

    #[test]
    fn the_signed_headers_are_named_in_order() {
        let headers = signer("us-east-1").sign(
            &Signable {
                method: "GET",
                host: "examplebucket.s3.amazonaws.com",
                path: "/test.txt",
                query: "",
                headers: &[("range", "bytes=0-9".to_string())],
                payload_sha256: EMPTY_SHA256,
            },
            at(WHEN),
        );
        let auth = &headers
            .iter()
            .find(|(n, _)| n == "authorization")
            .unwrap()
            .1;
        assert!(
            auth.contains("SignedHeaders=host;range;x-amz-content-sha256;x-amz-date,"),
            "{auth}"
        );
        assert!(auth.starts_with("AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20130524/us-east-1/s3/aws4_request, "), "{auth}");
    }

    #[test]
    fn the_clock_is_written_the_way_the_signature_wants_it() {
        assert_eq!(
            Stamp::at(at(WHEN)),
            Stamp {
                date: "20130524".into(),
                datetime: "20130524T000000Z".into()
            }
        );
        assert_eq!(Stamp::at(at(0)).datetime, "19700101T000000Z");
        // A leap day, the day after it, and the end of a century that is not a leap year.
        assert_eq!(Stamp::at(at(1_709_164_800)).date, "20240229");
        assert_eq!(Stamp::at(at(1_709_251_199)).datetime, "20240229T235959Z");
        assert_eq!(Stamp::at(at(951_782_400)).date, "20000229");
        assert_eq!(Stamp::at(at(4_107_542_400)).date, "21000301");
        // Before the epoch nothing panics and nothing wraps to a wild year.
        assert_eq!(
            Stamp::at(UNIX_EPOCH - std::time::Duration::from_secs(1)).date,
            "19700101"
        );
    }

    #[test]
    fn the_empty_body_hash_is_the_hash_of_nothing() {
        assert_eq!(sha256_hex(b""), EMPTY_SHA256);
        assert_eq!(
            sha256_hex(b"hello"),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    /// The secret must not be printable, by accident or by `{:?}`.
    #[test]
    fn a_debug_of_the_credentials_does_not_carry_the_secret() {
        let text = format!(
            "{:?}",
            Credentials::new(KEY, SECRET).with_session_token("FwoGZXIvYXdzEExample")
        );
        // The key identifies the connection and belongs in a diagnostic; the other two are
        // the things that must never reach a log, and the field *names* stay, which is why
        // the token here does not read like the word `token`.
        assert!(text.contains(KEY), "{text}");
        assert!(!text.contains(SECRET), "{text}");
        assert!(!text.contains("FwoGZXIvYXdz"), "{text}");
    }
}
