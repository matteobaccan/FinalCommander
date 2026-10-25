use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use scmd_core::VPath;

use crate::error::{NetError, Result};
use crate::remote::Remote;
use crate::site::Site;
use crate::ssh::{self, HostKeyPolicy};

/// Live connections, shared by the panels, the viewer and the job engine.
///
/// One connection per `scheme://user@host:port`: opening the same site in both panels, then
/// copying between them, uses a single session (`docs/spec/04-vfs.md` §6). The scheme is part
/// of the key because a server may offer both `ftp` and `ftps` on the same port, and those are
/// two connections.
#[derive(Debug, Default)]
pub struct Pool {
    connections: Mutex<BTreeMap<String, Remote>>,
    policy: Mutex<HostKeyPolicy>,
}

impl Pool {
    pub fn new() -> Arc<Pool> {
        Arc::new(Pool::default())
    }

    pub fn policy(&self) -> HostKeyPolicy {
        self.policy.lock().expect("pool").clone()
    }

    pub fn set_policy(&self, policy: HostKeyPolicy) {
        *self.policy.lock().expect("pool") = policy;
    }

    /// Accept this one host key on the next connection attempt (trust on first use). It is a
    /// one-shot: the next unknown host asks again.
    pub fn accept_new_host_key(&self, accept: bool) {
        self.policy.lock().expect("pool").accept_new = accept;
    }

    /// The connection a path belongs to, if it is already open.
    pub fn get(&self, path: &VPath) -> Option<Remote> {
        let key = Site::key_of(path)?;
        let remote = self.connections.lock().expect("pool").get(&key).cloned()?;
        // The key says the scheme and the authority agree; `owns` is the same question asked
        // of the connection itself, and it is what keeps a path off the wrong server.
        remote.owns(path).then_some(remote)
    }

    /// Like [`Pool::get`], but says which host is missing instead of returning nothing.
    pub fn require(&self, path: &VPath) -> Result<Remote> {
        if !path.is_remote() {
            return Err(NetError::NotRemote(path.to_string()));
        }
        self.get(path).ok_or_else(|| {
            NetError::NotConnected(Site::key_of(path).unwrap_or_else(|| path.to_string()))
        })
    }

    pub fn insert(&self, remote: Remote) {
        self.connections
            .lock()
            .expect("pool")
            .insert(remote.key(), remote);
    }

    /// Open a site, or hand back the connection that is already open for it.
    pub async fn connect(&self, site: &Site, secret: Option<&str>) -> Result<Remote> {
        if let Some(existing) = self
            .connections
            .lock()
            .expect("pool")
            .get(&site.key())
            .cloned()
        {
            return Ok(existing);
        }
        let remote = match site.scheme {
            scmd_core::Scheme::Ftp | scmd_core::Scheme::Ftps => {
                crate::ftp::connect(site, secret).await?
            }
            // SFTP, and whatever else carries an authority: the SSH client says what it
            // cannot do more clearly than a match arm here could.
            _ => ssh::connect(site, secret, &self.policy()).await?,
        };
        // The user said yes to one host key, not to every host from now on.
        self.accept_new_host_key(false);
        self.insert(remote.clone());
        Ok(remote)
    }

    /// Close one connection, by the `scheme://user@host:port` key. Returns false when there
    /// was nothing to close.
    pub async fn disconnect(&self, key: &str) -> bool {
        let removed = self.connections.lock().expect("pool").remove(key);
        match removed {
            Some(remote) => {
                let _ = remote.close().await;
                true
            }
            None => false,
        }
    }

    pub async fn disconnect_all(&self) {
        let all: Vec<Remote> = self
            .connections
            .lock()
            .expect("pool")
            .values()
            .cloned()
            .collect();
        self.connections.lock().expect("pool").clear();
        for remote in all {
            let _ = remote.close().await;
        }
    }

    /// `scheme://user@host:port` for every open connection, in order.
    pub fn connected(&self) -> Vec<String> {
        self.connections
            .lock()
            .expect("pool")
            .keys()
            .cloned()
            .collect()
    }

    pub fn is_connected(&self, key: &str) -> bool {
        self.connections.lock().expect("pool").contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;

    fn fake(authority_user: &str) -> Remote {
        let site = Site::parse_url(&format!("sftp://{authority_user}/srv")).unwrap();
        Remote::new(site, MemoryFs::new().with_dir("/srv"))
    }

    #[tokio::test]
    async fn a_connection_is_found_by_the_paths_authority() {
        let pool = Pool::new();
        let remote = fake("mb@host:22");
        pool.insert(remote.clone());
        let path = remote.site().start().join("www");
        assert!(pool.get(&path).is_some());
        assert_eq!(pool.connected(), vec!["sftp://mb@host:22".to_string()]);

        // Another host, and a local path, are not this connection.
        let elsewhere = VPath::remote(scmd_core::Scheme::Sftp, "mb@other:22", "/srv");
        assert!(pool.get(&elsewhere).is_none());
        assert!(matches!(
            pool.require(&elsewhere),
            Err(NetError::NotConnected(_))
        ));
        assert!(matches!(
            pool.require(&VPath::local("/srv")),
            Err(NetError::NotRemote(_))
        ));

        assert!(pool.disconnect("sftp://mb@host:22").await);
        assert!(!pool.disconnect("sftp://mb@host:22").await);
        assert!(pool.connected().is_empty());
    }

    #[tokio::test]
    async fn trust_on_first_use_is_a_one_shot() {
        let pool = Pool::new();
        assert!(!pool.policy().accept_new);
        pool.accept_new_host_key(true);
        assert!(pool.policy().accept_new);
        // An already-open site short-circuits the dial, and clears the one-shot on the way.
        let remote = fake("mb@host:22");
        pool.insert(remote.clone());
        let again = pool.connect(remote.site(), None).await.unwrap();
        assert_eq!(again.authority(), remote.authority());
    }
}
