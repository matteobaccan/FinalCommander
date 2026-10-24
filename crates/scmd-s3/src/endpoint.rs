//! Where the requests go, and in which of the two shapes.
//!
//! S3 is an AWS service and a protocol a dozen other things speak — MinIO, Ceph, Garage,
//! Cloudflare R2, Backblaze, Wasabi, Hetzner, a LocalStack in a container. What separates them
//! is one host name, one region string and one choice: does the bucket go in front of the host
//! (`bucket.s3.eu-west-1.amazonaws.com/key`, the virtual-hosted style) or at the front of the
//! path (`minio.lan:9000/bucket/key`, the path style). Everything else about the request is
//! the same, which is why one provider can reach all of them.

/// A service that speaks S3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    /// `s3.eu-west-1.amazonaws.com`, `minio.lan:9000`, `127.0.0.1:9000`.
    pub host: String,
    /// The region the signature is scoped to. Not optional, even where the service ignores it:
    /// it is part of what is signed, and a wrong one is a refusal rather than a redirect.
    pub region: String,
    pub tls: bool,
    /// Put the bucket in the path rather than in front of the host.
    pub path_style: bool,
}

impl Endpoint {
    /// AWS itself, in one region.
    pub fn aws(region: impl Into<String>) -> Endpoint {
        let region = region.into();
        Endpoint {
            host: format!("s3.{region}.amazonaws.com"),
            region,
            tls: true,
            path_style: false,
        }
    }

    /// Anything else: `https://minio.lan:9000`, `http://127.0.0.1:9000`, `minio.lan:9000`.
    ///
    /// The path style is the default for one of these, because a host that is not AWS usually
    /// has no wildcard certificate and no DNS entry per bucket, and because an address given
    /// as a bare IP cannot have a bucket put in front of it at all.
    pub fn parse(url: &str, region: impl Into<String>) -> Endpoint {
        let (tls, rest) = match url.trim() {
            u if u.starts_with("https://") => (true, &u[8..]),
            u if u.starts_with("http://") => (false, &u[7..]),
            u => (true, u),
        };
        let host = rest.trim_end_matches('/').split('/').next().unwrap_or(rest);
        Endpoint {
            host: host.to_string(),
            region: region.into(),
            tls,
            path_style: true,
        }
    }

    pub fn virtual_hosted(mut self) -> Endpoint {
        self.path_style = false;
        self
    }

    pub fn with_path_style(mut self, path_style: bool) -> Endpoint {
        self.path_style = path_style;
        self
    }

    /// The host header for a request about `bucket`, and the path to sign and to send.
    ///
    /// A bucket whose name is not a DNS label — one with a dot in it, or an upper-case letter,
    /// or longer than sixty-three characters — goes in the path whatever the endpoint prefers:
    /// `my.bucket.s3.amazonaws.com` is a host no certificate covers, and the request would
    /// fail at the handshake rather than anywhere one could read a reason.
    pub fn target(&self, bucket: &str, key: &str) -> Target {
        let path_style = self.path_style || !is_dns_label(bucket);
        let encoded = crate::uri::encode_path(key);
        if bucket.is_empty() {
            return Target {
                host: self.host.clone(),
                path: "/".to_string(),
                tls: self.tls,
            };
        }
        if path_style {
            Target {
                host: self.host.clone(),
                path: format!("/{bucket}/{encoded}"),
                tls: self.tls,
            }
        } else {
            Target {
                host: format!("{bucket}.{}", self.host),
                path: format!("/{encoded}"),
                tls: self.tls,
            }
        }
    }
}

/// Where one request is addressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub host: String,
    pub path: String,
    pub tls: bool,
}

impl Target {
    /// The URL to fetch. `query` is the canonical query string, already encoded and sorted.
    pub fn url(&self, query: &str) -> String {
        let scheme = if self.tls { "https" } else { "http" };
        if query.is_empty() {
            format!("{scheme}://{}{}", self.host, self.path)
        } else {
            format!("{scheme}://{}{}?{query}", self.host, self.path)
        }
    }
}

/// Can this name go in front of a host? The rule is S3's own for a "DNS-compliant" bucket.
fn is_dns_label(bucket: &str) -> bool {
    !bucket.is_empty()
        && bucket.len() <= 63
        && !bucket.contains('.')
        && bucket
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && !bucket.starts_with('-')
        && !bucket.ends_with('-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aws_is_virtual_hosted_and_regional() {
        let e = Endpoint::aws("eu-west-1");
        assert_eq!(e.host, "s3.eu-west-1.amazonaws.com");
        assert!(e.tls && !e.path_style);
        let t = e.target("logs", "2024/08/access.log");
        assert_eq!(t.host, "logs.s3.eu-west-1.amazonaws.com");
        assert_eq!(t.path, "/2024/08/access.log");
        assert_eq!(
            t.url("list-type=2"),
            "https://logs.s3.eu-west-1.amazonaws.com/2024/08/access.log?list-type=2"
        );
    }

    #[test]
    fn something_that_is_not_aws_is_path_style_unless_told_otherwise() {
        let e = Endpoint::parse("http://127.0.0.1:9000", "us-east-1");
        assert_eq!(e.host, "127.0.0.1:9000");
        assert!(!e.tls && e.path_style);
        let t = e.target("mybucket", "x/y.txt");
        assert_eq!(t.host, "127.0.0.1:9000");
        assert_eq!(t.path, "/mybucket/x/y.txt");
        assert_eq!(t.url(""), "http://127.0.0.1:9000/mybucket/x/y.txt");

        // A host given without a scheme is assumed to have TLS, not assumed not to.
        assert!(Endpoint::parse("minio.lan:9000", "us-east-1").tls);
        // And a URL with a path in it keeps only the host.
        assert_eq!(
            Endpoint::parse("https://minio.lan/s3/", "x").host,
            "minio.lan"
        );

        let r2 =
            Endpoint::parse("https://account.r2.cloudflarestorage.com", "auto").virtual_hosted();
        assert_eq!(
            r2.target("media", "a.png").host,
            "media.account.r2.cloudflarestorage.com"
        );
    }

    /// The one that bites: a bucket with a dot in its name cannot be a host name.
    #[test]
    fn a_bucket_that_is_not_a_dns_label_goes_in_the_path_anyway() {
        let e = Endpoint::aws("us-east-1");
        let t = e.target("my.bucket.example", "k");
        assert_eq!(t.host, "s3.us-east-1.amazonaws.com");
        assert_eq!(t.path, "/my.bucket.example/k");

        for bad in [
            "My-Bucket",
            "-leading",
            "trailing-",
            &"x".repeat(64),
            "with.dot",
        ] {
            assert!(
                e.target(bad, "k").path.starts_with(&format!("/{bad}/")),
                "{bad}"
            );
        }
        for good in ["logs", "a-b-c", "bucket123", &"x".repeat(63)] {
            assert_eq!(e.target(good, "k").path, "/k", "{good}");
        }
    }

    #[test]
    fn no_bucket_at_all_is_the_service_itself() {
        let t = Endpoint::aws("us-east-1").target("", "");
        assert_eq!(t.host, "s3.us-east-1.amazonaws.com");
        assert_eq!(t.path, "/");
    }

    #[test]
    fn a_key_is_encoded_in_the_path_but_the_slashes_stay() {
        let t = Endpoint::aws("us-east-1").target("logs", "a b/c&d+e.txt");
        assert_eq!(t.path, "/a%20b/c%26d%2Be.txt");
    }
}
