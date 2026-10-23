use std::sync::Arc;

use async_trait::async_trait;
use scmd_core::{CancelToken, Entry, Error, Result as CoreResult, VPath};
use scmd_vfs::{Caps, LocalVfs, Vfs};

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

/// Sends each path to the provider that owns it. This is what the application holds, so that
/// nothing above it has to know whether a panel is local or remote.
#[derive(Debug, Clone)]
pub struct Router {
    local: LocalVfs,
    net: NetVfs,
}

impl Router {
    pub fn new(pool: Arc<Pool>) -> Router {
        Router {
            local: LocalVfs,
            net: NetVfs::new(pool),
        }
    }

    pub fn pool(&self) -> &Arc<Pool> {
        self.net.pool()
    }

    /// The provider that owns `path`.
    pub fn provider(&self, path: &VPath) -> &dyn Vfs {
        if path.is_local() {
            &self.local
        } else {
            &self.net
        }
    }

    /// What the path's provider can do, for a UI that asks before it offers.
    pub fn caps_for(&self, path: &VPath) -> Caps {
        self.provider(path).caps()
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

    fn random_access(&self) -> bool {
        true
    }
}

/// `Error::Unsupported` with the reason spelled out, for a capability a provider lacks.
pub fn unsupported(path: &VPath, detail: &str) -> Error {
    Error::Other(format!("{path}: {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemorySftp;
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
            MemorySftp::new().with_file("/srv/remote.txt", b"yy"),
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

    #[tokio::test]
    async fn a_path_whose_host_is_not_connected_says_so() {
        let router = Router::new(Pool::new());
        let path = VPath::remote(scmd_core::Scheme::Sftp, "mb@host:22", "/srv");
        let err = router.list(&path, CancelToken::new()).await.unwrap_err();
        assert!(err.to_string().contains("mb@host:22"), "{err}");
    }
}
