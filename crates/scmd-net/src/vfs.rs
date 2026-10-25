use std::sync::Arc;

use async_trait::async_trait;
use scmd_core::{CancelToken, Entry, Error, Result as CoreResult, VPath};
use scmd_vfs::{ArcVfs, Caps, Dir, LocalVfs, Vfs};

use crate::pool::Pool;

/// The netbox as a VFS provider: remote paths, served by whichever connection owns them.
#[derive(Debug, Clone)]
pub struct NetVfs {
    pool: Arc<Pool>,
}

impl NetVfs {
    pub fn new(pool: Arc<Pool>) -> NetVfs {
        NetVfs { pool }
    }

    pub fn pool(&self) -> &Arc<Pool> {
        &self.pool
    }
}

#[async_trait]
impl Vfs for NetVfs {
    fn scheme(&self) -> &'static str {
        "sftp"
    }

    /// What the netbox can do for one path, which depends on the protocol that path speaks.
    fn caps_for(&self, path: &VPath) -> Caps {
        caps_of(path.scheme())
    }

    /// What SFTP version 3 actually gives us. No hard links, no extended attributes, and the
    /// UI asks before it offers (`docs/spec/04-vfs.md` §2).
    fn caps(&self) -> Caps {
        Caps::READ
            | Caps::WRITE
            | Caps::CREATE
            | Caps::DELETE
            | Caps::RENAME
            | Caps::SEEK
            | Caps::TRUNCATE
            | Caps::META_TIMES
            | Caps::META_MODE
            | Caps::META_OWNER
            | Caps::LINKS
            | Caps::RESUME
            | Caps::RANDOM_WRITE
            | Caps::CASE_SENSITIVE
    }

    async fn list(&self, path: &VPath, cancel: CancelToken) -> CoreResult<Vec<Entry>> {
        let remote = self.pool.require(path)?;
        Ok(remote.list(path, &cancel).await?)
    }

    async fn stat(&self, path: &VPath) -> CoreResult<Entry> {
        let remote = self.pool.require(path)?;
        Ok(remote.stat(path).await?)
    }
}

/// What each protocol the netbox serves can do.
///
/// This is the table the UI asks before it offers (`docs/spec/04-vfs.md` §2), and the
/// differences are the protocols' own:
///
/// * **SFTP** is a filesystem protocol and behaves like one.
/// * **FTP** can read, write, create, delete and rename. It has no links, no truncate and no
///   random write, and it does not say SEEK: `REST` can start a transfer at an offset, but
///   that costs a new data connection every time, which is not the cheap seek the viewer's hex
///   mode asks SEEK about. (Resuming an interrupted transfer with `REST` is worth having and
///   is in the backlog; until it is written, RESUME would be a promise nothing keeps.) Times
///   and modes are extensions — `MFMT` and `SITE CHMOD` — that most servers do have, so they
///   are offered, and a server that refuses says so in its own words.
/// * **WebDAV** is HTTP: a `PUT` replaces a whole file and there is nothing to seek with, so
///   no SEEK, no TRUNCATE, no RANDOM_WRITE and no RESUME. It says nothing about times or modes
///   either, and that is the protocol rather than the client: `getlastmodified` is a *live*
///   property in RFC 4918, which means the server owns it, and HTTP has no permission bits at
///   all. `MOVE` is one request, but it is sent with `Overwrite: F` so that a rename cannot
///   replace something without being asked to — which is not what ATOMIC_RENAME promises, so
///   that is not claimed either.
pub fn caps_of(scheme: scmd_core::Scheme) -> Caps {
    let common = Caps::READ | Caps::WRITE | Caps::CREATE | Caps::DELETE | Caps::RENAME;
    match scheme {
        scmd_core::Scheme::Ftp | scmd_core::Scheme::Ftps => {
            common | Caps::META_TIMES | Caps::META_MODE | Caps::CASE_SENSITIVE
        }
        scmd_core::Scheme::Dav | scmd_core::Scheme::Davs => common | Caps::CASE_SENSITIVE,
        // SFTP, and anything else that reaches the netbox.
        _ => {
            Caps::READ
                | Caps::WRITE
                | Caps::CREATE
                | Caps::DELETE
                | Caps::RENAME
                | Caps::SEEK
                | Caps::TRUNCATE
                | Caps::META_TIMES
                | Caps::META_MODE
                | Caps::META_OWNER
                | Caps::LINKS
                | Caps::RESUME
                | Caps::RANDOM_WRITE
                | Caps::CASE_SENSITIVE
        }
    }
}

/// Sends each path to the provider that owns it. This is what the application holds, so that
/// nothing above it has to know whether a panel is local or remote.
#[derive(Clone)]
pub struct Router {
    local: LocalVfs,
    net: NetVfs,
    /// Archives, with the one container cache behind them.
    arc: Arc<ArcVfs>,
    /// Object storage, when any is configured. Held as a trait object rather than as the S3
    /// provider itself: the netbox knows how to route a scheme, and nothing more about it.
    objects: Arc<dyn Vfs>,
}

impl std::fmt::Debug for Router {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Router")
            .field("net", &self.net)
            .field("objects", &self.objects.scheme())
            .finish()
    }
}

impl Router {
    pub fn new(pool: Arc<Pool>) -> Router {
        Router::with_limits(pool, scmd_arc::Limits::default())
    }

    /// The same, with the archive guards of SPEC 04 §4 set from the configuration.
    pub fn with_limits(pool: Arc<Pool>, limits: scmd_arc::Limits) -> Router {
        Router {
            local: LocalVfs,
            net: NetVfs::new(pool),
            arc: Arc::new(ArcVfs::new(limits)),
            objects: Arc::new(Unconfigured),
        }
    }

    /// Hand the router the object storage provider. Without this, an `s3://` path is refused
    /// with the reason rather than with a failure that reads like a network problem.
    pub fn with_objects(mut self, objects: Arc<dyn Vfs>) -> Router {
        self.objects = objects;
        self
    }

    pub fn pool(&self) -> &Arc<Pool> {
        self.net.pool()
    }

    /// The archive provider, for the reads that do not go through the panel.
    pub fn archives(&self) -> &ArcVfs {
        &self.arc
    }

    /// The provider that owns `path`. A path inside a container belongs to the archive
    /// provider whatever holds the archive itself, which is what the `!` in it says.
    pub fn provider(&self, path: &VPath) -> &dyn Vfs {
        if path.in_container() {
            &*self.arc
        } else if path.is_local() {
            &self.local
        } else if path.scheme() == scmd_core::Scheme::S3 {
            &*self.objects
        } else {
            &self.net
        }
    }

    /// What the path's provider can do, for a UI that asks before it offers.
    pub fn caps_for(&self, path: &VPath) -> Caps {
        self.provider(path).caps_for(path)
    }
}

#[async_trait]
impl Vfs for Router {
    fn scheme(&self) -> &'static str {
        "*"
    }

    fn caps(&self) -> Caps {
        self.local.caps()
    }

    async fn list(&self, path: &VPath, cancel: CancelToken) -> CoreResult<Vec<Entry>> {
        self.provider(path).list(path, cancel).await
    }

    async fn stat(&self, path: &VPath) -> CoreResult<Entry> {
        self.provider(path).stat(path).await
    }

    async fn list_dir(&self, path: &VPath, cancel: CancelToken) -> CoreResult<Dir> {
        self.provider(path).list_dir(path, cancel).await
    }

    fn random_access(&self) -> bool {
        true
    }
}

/// What an `s3://` path meets when no object storage is configured. It is a provider so that
/// the routing has one answer for every scheme, and its answer is the sentence that says what
/// to do about it.
#[derive(Debug, Clone, Copy)]
struct Unconfigured;

#[async_trait]
impl Vfs for Unconfigured {
    fn scheme(&self) -> &'static str {
        "s3 (not configured)"
    }

    fn caps(&self) -> Caps {
        Caps::empty()
    }

    async fn list(&self, path: &VPath, _cancel: CancelToken) -> CoreResult<Vec<Entry>> {
        Err(self.refusal(path))
    }

    async fn stat(&self, path: &VPath) -> CoreResult<Entry> {
        Err(self.refusal(path))
    }
}

impl Unconfigured {
    fn refusal(&self, path: &VPath) -> Error {
        Error::Other(format!(
            "{path}: no object storage is configured — add an `[[s3]]` block to scmd.toml"
        ))
    }
}

/// `Error::Unsupported` with the reason spelled out, for a capability a provider lacks.
pub fn unsupported(path: &VPath, detail: &str) -> Error {
    Error::Other(format!("{path}: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;
    use crate::remote::Remote;
    use crate::site::Site;

    #[tokio::test]
    async fn the_router_sends_each_path_to_its_own_provider() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("local.txt"), b"x").unwrap();
        let pool = Pool::new();
        let site = Site::parse_url("sftp://mb@host:22/srv").unwrap();
        pool.insert(Remote::new(
            site.clone(),
            MemoryFs::new().with_file("/srv/remote.txt", b"yy"),
        ));
        let router = Router::new(pool);

        let local = router
            .list(&VPath::local(dir.path()), CancelToken::new())
            .await
            .unwrap();
        assert!(local.iter().any(|e| e.name_str() == "local.txt"));

        let remote = router
            .list(&site.start(), CancelToken::new())
            .await
            .unwrap();
        assert!(remote.iter().any(|e| e.name_str() == "remote.txt"));

        // And a path with a `!` in it goes to the archive provider, wherever the archive is.
        let zip = dir.path().join("tree.zip");
        let file = std::fs::File::create(&zip).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("inside.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, b"zz").unwrap();
        writer.finish().unwrap();
        let inside = router
            .list(&VPath::local(&zip).enter("/"), CancelToken::new())
            .await
            .unwrap();
        assert!(inside.iter().any(|e| e.name_str() == "inside.txt"));
        assert_eq!(
            router.caps_for(&VPath::local(&zip).enter("/")),
            Caps::READ | Caps::SEEK
        );
        assert_eq!(
            router
                .stat(&site.start().join("remote.txt"))
                .await
                .unwrap()
                .size,
            2
        );

        // Capabilities follow the path: SFTP has no hard links.
        assert!(router.caps_for(&site.start()).contains(Caps::RENAME));
        assert!(!router.caps_for(&site.start()).contains(Caps::HARDLINKS));
    }

    /// An `s3://` path goes to the object provider the router was handed, and the netbox is
    /// not consulted about it — which is the whole point of handing it over as a trait object.
    #[tokio::test]
    async fn an_object_path_goes_to_the_object_provider() {
        /// Not the real one: the S3 provider is tested against a service in its own crate,
        /// and what this has to prove is the routing.
        #[derive(Debug)]
        struct Stub;

        #[async_trait]
        impl Vfs for Stub {
            fn scheme(&self) -> &'static str {
                "s3"
            }

            fn caps(&self) -> Caps {
                Caps::READ | Caps::SEEK
            }

            async fn list(&self, _path: &VPath, _cancel: CancelToken) -> CoreResult<Vec<Entry>> {
                Ok(vec![Entry::parent_entry()])
            }

            async fn stat(&self, _path: &VPath) -> CoreResult<Entry> {
                Ok(Entry::parent_entry())
            }

            async fn list_dir(&self, _path: &VPath, _cancel: CancelToken) -> CoreResult<Dir> {
                Ok(Dir::partial(
                    vec![Entry::parent_entry()],
                    "the first 1 of many",
                ))
            }
        }

        let router = Router::new(Pool::new()).with_objects(Arc::new(Stub));
        let path = VPath::parse("s3://logs/2024/08/");
        assert_eq!(path.scheme(), scmd_core::Scheme::S3);
        assert_eq!(
            router.list(&path, CancelToken::new()).await.unwrap().len(),
            1
        );
        // And the listing's own account of itself survives the routing.
        let dir = router.list_dir(&path, CancelToken::new()).await.unwrap();
        assert!(!dir.complete);
        assert_eq!(dir.note.as_deref(), Some("the first 1 of many"));
        // Capabilities follow the path: object storage cannot rename.
        assert!(router.caps_for(&path).contains(Caps::READ));
        assert!(!router.caps_for(&path).contains(Caps::RENAME));
        // A local path is unaffected by any of this.
        assert!(router.caps_for(&VPath::local("/tmp")).contains(Caps::WRITE));
    }

    /// With nothing configured, an `s3://` path must not read like a network failure.
    #[tokio::test]
    async fn an_object_path_with_no_service_configured_says_what_to_do() {
        let router = Router::new(Pool::new());
        let err = router
            .list(&VPath::parse("s3://logs/2024/"), CancelToken::new())
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("s3://logs/2024"), "{text}");
        assert!(text.contains("[[s3]]"), "{text}");
    }

    #[tokio::test]
    async fn a_path_whose_host_is_not_connected_says_so() {
        let router = Router::new(Pool::new());
        let path = VPath::remote(scmd_core::Scheme::Sftp, "mb@host:22", "/srv");
        let err = router.list(&path, CancelToken::new()).await.unwrap_err();
        assert!(err.to_string().contains("mb@host:22"), "{err}");
    }
}
