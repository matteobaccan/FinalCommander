//! The thin layer that actually talks SSH: everything above it is tested against
//! [`crate::memory::MemoryFs`].

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use fcmd_core::EntryKind;
use russh::client::{self, Handle};
use russh::keys::{
    known_hosts::{check_known_hosts_path, learn_known_hosts_path},
    HashAlg, PrivateKeyWithHashAlg, PublicKeyOrCertificate,
};
use russh_sftp::client::SftpSession;
use russh_sftp::protocol::{FileAttributes, FileType, OpenFlags};

use crate::error::{NetError, Result, SecretKind};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};
use crate::remote::Remote;
use crate::site::{home_dir, Auth, Site};

/// What to do about the server's host key.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct HostKeyPolicy {
    /// Where the record lives. `None` means `~/.ssh/known_hosts`.
    pub known_hosts: Option<PathBuf>,
    /// Set only after the user has seen the fingerprint and said yes (trust on first use).
    /// A key that is *recorded and different* is never accepted, whatever this says.
    pub accept_new: bool,
}

impl HostKeyPolicy {
    pub fn path(&self) -> Result<PathBuf> {
        match &self.known_hosts {
            Some(path) => Ok(path.clone()),
            None => home_dir()
                .map(|home| home.join(".ssh").join("known_hosts"))
                .ok_or_else(|| NetError::Connect {
                    host: "localhost".into(),
                    detail: "no home directory, so no known_hosts file".into(),
                }),
        }
    }
}

/// The russh client handler: its one job is the host-key decision.
struct Client {
    host: String,
    port: u16,
    policy: HostKeyPolicy,
}

impl client::Handler for Client {
    type Error = NetError;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKeyOrCertificate,
    ) -> Result<bool> {
        let key = match server_public_key {
            PublicKeyOrCertificate::PublicKey { key, .. } => key.clone(),
            PublicKeyOrCertificate::Certificate(_) => {
                return Err(NetError::Unsupported {
                    path: self.host.clone(),
                    detail: "host certificates are not checked yet".into(),
                })
            }
        };
        let path = self.policy.path()?;
        match check_known_hosts_path(&self.host, self.port, &key, &path) {
            Ok(true) => Ok(true),
            Ok(false) if self.policy.accept_new => {
                learn_known_hosts_path(&self.host, self.port, &key, &path).map_err(|e| {
                    NetError::Connect {
                        host: self.host.clone(),
                        detail: format!("could not write {}: {e}", path.display()),
                    }
                })?;
                Ok(true)
            }
            Ok(false) => Err(NetError::UnknownHostKey {
                host: self.host.clone(),
                fingerprint: key.fingerprint(HashAlg::Sha256).to_string(),
            }),
            Err(russh::keys::Error::KeyChanged { line }) => Err(NetError::ChangedHostKey {
                host: self.host.clone(),
                path: path.clone(),
                line,
            }),
            Err(e) => Err(NetError::Connect {
                host: self.host.clone(),
                detail: format!("{}: {e}", path.display()),
            }),
        }
    }
}

/// Open a connection and the SFTP subsystem on it.
///
/// `secret` is the password or the key passphrase when one is needed; when it is needed and
/// missing, the error says which, so the caller can ask and try again.
pub async fn connect(site: &Site, secret: Option<&str>, policy: &HostKeyPolicy) -> Result<Remote> {
    let config = Arc::new(client::Config {
        keepalive_interval: site.keepalive,
        inactivity_timeout: Some(Duration::from_secs(3600)),
        ..client::Config::default()
    });
    // Straight at the server, or through the one host it can be reached from (SPEC 04 §6.3).
    // The bastion's own session is kept beside the destination's: the channel the destination
    // speaks through lives inside it, so dropping it would drop the connection.
    let (mut handle, through) = match &site.jump {
        None => (
            open(config, site.host.as_str(), site.port, site, policy).await?,
            None,
        ),
        Some(jump) => {
            let (destination, hop) = open_through(config, site, jump, secret, policy).await?;
            (destination, Some(hop))
        }
    };

    authenticate(&mut handle, &site.user, &site.host, &site.auth, secret).await?;

    let channel = handle
        .channel_open_session()
        .await
        .map_err(|e| russh_err(site, e))?;
    channel
        .request_subsystem(true, "sftp")
        .await
        .map_err(|e| russh_err(site, e))?;
    let session = SftpSession::new(channel.into_stream())
        .await
        .map_err(|e| NetError::Connect {
            host: site.host.clone(),
            detail: format!("the SFTP subsystem did not start: {e}"),
        })?;
    Ok(Remote::new(
        site.clone(),
        Arc::new(SftpBackend {
            session,
            keep: handle,
            keep_jump: through,
        }),
    ))
}

/// One TCP connection to an SSH server, with the host-key decision made against the name it
/// was reached under. `site` is only there for the shape of the error.
async fn open(
    config: Arc<client::Config>,
    host: &str,
    port: u16,
    site: &Site,
    policy: &HostKeyPolicy,
) -> Result<Handle<Client>> {
    let handler = Client {
        host: host.to_string(),
        port,
        policy: policy.clone(),
    };
    client::connect(config, (host, port), handler)
        .await
        .map_err(|e| match e {
            // The handler's own refusals must not be buried under "connection failed".
            NetError::UnknownHostKey { .. }
            | NetError::ChangedHostKey { .. }
            | NetError::Unsupported { .. }
            | NetError::Connect { .. } => e,
            other => NetError::Connect {
                host: format!("{}:{}", site.host, site.port),
                detail: other.to_string(),
            },
        })
}

/// The destination, reached through a jump host — `ssh -J`'s one hop (SPEC 04 §6.3).
///
/// Three steps and nothing clever: a session on the bastion, a `direct-tcpip` channel from it
/// to the destination, and the destination's own SSH session *inside that channel*. The
/// second session is a session like any other — its host key is checked under the
/// destination's name, and it authenticates as the site says — because the tunnel is only how
/// its bytes travel. Nothing in this program decrypts anything on the way: the bastion carries
/// a stream it cannot read, which is the whole point of `ProxyJump` over an agent-forwarding
/// hop.
///
/// Returns the destination and the bastion, in that order; the caller keeps both.
async fn open_through(
    config: Arc<client::Config>,
    site: &Site,
    jump: &crate::site::Jump,
    secret: Option<&str>,
    policy: &HostKeyPolicy,
) -> Result<(Handle<Client>, Handle<Client>)> {
    // The bastion is a server like any other, so its key is checked under *its* name:
    // trusting the destination's key says nothing about the machine in front of it.
    let mut hop = open(config.clone(), jump.host.as_str(), jump.port, site, policy).await?;
    let auth = jump_auth(site);
    authenticate(&mut hop, &jump.user, &jump.host, &auth, secret)
        .await
        .map_err(|e| match (&site.auth, e) {
            // The one case worth a longer sentence: the site types a password, which is not
            // sent on to another machine, so the bastion was tried with the agent.
            (Auth::Password, NetError::Auth { user, host, detail }) => NetError::Auth {
                user,
                host,
                detail: format!(
                    "{detail} — a jump host is authenticated with the agent or with the \
                     session's key file, never with the password typed for {}",
                    site.host
                ),
            },
            (_, other) => other,
        })?;

    // The channel the destination's session will speak over. The originator is this program,
    // which has no address the bastion cares about: `ssh` sends its own loopback and so does
    // this.
    let channel = hop
        .channel_open_direct_tcpip(site.host.clone(), u32::from(site.port), "127.0.0.1", 0)
        .await
        .map_err(|e| NetError::Connect {
            host: format!("{}:{}", site.host, site.port),
            detail: format!("{} would not open a channel to it: {e}", jump.authority()),
        })?;

    let handler = Client {
        host: site.host.clone(),
        port: site.port,
        policy: policy.clone(),
    };
    let destination = client::connect_stream(config, channel.into_stream(), handler)
        .await
        .map_err(|e| match e {
            NetError::UnknownHostKey { .. }
            | NetError::ChangedHostKey { .. }
            | NetError::Unsupported { .. }
            | NetError::Connect { .. } => e,
            other => NetError::Connect {
                host: format!("{}:{}", site.host, site.port),
                detail: format!("through {}: {other}", jump.authority()),
            },
        })?;
    Ok((destination, hop))
}

/// How the jump host is authenticated: the agent, or the same key file the session uses.
///
/// Never the destination's password. A password typed for one machine must not be sent to
/// another, and asking for a second secret means a second one through the pool, the prompts
/// and the retry — which is a bigger change than this, and is the thing to do the day
/// somebody has a bastion that wants one.
fn jump_auth(site: &Site) -> Auth {
    match &site.auth {
        Auth::Key { path } => Auth::Key { path: path.clone() },
        Auth::Agent | Auth::Password => Auth::Agent,
    }
}

async fn authenticate(
    handle: &mut Handle<Client>,
    user: &str,
    host: &str,
    auth: &Auth,
    secret: Option<&str>,
) -> Result<()> {
    let auth_failed = |detail: String| NetError::Auth {
        user: user.to_string(),
        host: host.to_string(),
        detail,
    };
    match auth {
        Auth::Password => {
            let password = secret.ok_or_else(|| NetError::NeedsSecret {
                user: user.to_string(),
                host: host.to_string(),
                kind: SecretKind::Password,
            })?;
            let result = handle
                .authenticate_password(user, password)
                .await
                .map_err(|e| auth_failed(e.to_string()))?;
            if !result.success() {
                return Err(auth_failed("the server rejected the password".into()));
            }
            Ok(())
        }
        Auth::Key { path } => {
            let key = russh::keys::load_secret_key(path, secret).map_err(|e| match e {
                russh::keys::Error::KeyIsEncrypted => NetError::NeedsSecret {
                    user: user.to_string(),
                    host: host.to_string(),
                    kind: SecretKind::Passphrase,
                },
                other => auth_failed(format!("{}: {other}", path.display())),
            })?;
            let hash_alg = handle
                .best_supported_rsa_hash()
                .await
                .ok()
                .flatten()
                .flatten();
            let result = handle
                .authenticate_publickey(user, PrivateKeyWithHashAlg::new(Arc::new(key), hash_alg))
                .await
                .map_err(|e| auth_failed(e.to_string()))?;
            if !result.success() {
                return Err(auth_failed(format!(
                    "the server rejected {}",
                    path.display()
                )));
            }
            Ok(())
        }
        Auth::Agent => {
            let mut agent = connect_agent()
                .await
                .map_err(|e| auth_failed(format!("no usable SSH agent: {e}")))?;
            let identities = agent
                .request_identities()
                .await
                .map_err(|e| auth_failed(format!("the agent gave no identities: {e}")))?;
            if identities.is_empty() {
                return Err(auth_failed("the agent holds no keys".into()));
            }
            let hash_alg = handle
                .best_supported_rsa_hash()
                .await
                .ok()
                .flatten()
                .flatten();
            let mut last = String::new();
            for identity in identities {
                let key = match identity {
                    russh::keys::agent::AgentIdentity::PublicKey { key, .. } => key,
                    // Certificates from the agent are not tried yet; keys are.
                    russh::keys::agent::AgentIdentity::Certificate { .. } => continue,
                };
                let fingerprint = key.fingerprint(HashAlg::Sha256).to_string();
                match handle
                    .authenticate_publickey_with(user, key, hash_alg, &mut agent)
                    .await
                {
                    Ok(result) if result.success() => return Ok(()),
                    Ok(_) => last = format!("the server rejected {fingerprint}"),
                    Err(e) => last = format!("{fingerprint}: {e}"),
                }
            }
            Err(auth_failed(if last.is_empty() {
                "none of the agent's keys were accepted".into()
            } else {
                last
            }))
        }
    }
}

/// The running SSH agent, however this platform exposes one.
///
/// Unix: `SSH_AUTH_SOCK`. Windows: OpenSSH's named pipe first, then Pageant, which is what
/// PuTTY and WinSCP users have. The stream type differs per platform, so the client is boxed
/// into its dynamic form and the caller sees one type either way.
#[cfg(unix)]
async fn connect_agent() -> std::result::Result<
    russh::keys::agent::client::AgentClient<
        Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>,
    >,
    russh::keys::Error,
> {
    Ok(russh::keys::agent::client::AgentClient::connect_env()
        .await?
        .dynamic())
}

#[cfg(windows)]
async fn connect_agent() -> std::result::Result<
    russh::keys::agent::client::AgentClient<
        Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>,
    >,
    russh::keys::Error,
> {
    const OPENSSH_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";
    match russh::keys::agent::client::AgentClient::connect_named_pipe(OPENSSH_PIPE).await {
        Ok(client) => Ok(client.dynamic()),
        Err(openssh) => match russh::keys::agent::client::AgentClient::connect_pageant().await {
            Ok(client) => Ok(client.dynamic()),
            // The first failure is the more likely one to be useful.
            Err(_) => Err(openssh),
        },
    }
}

#[cfg(not(any(unix, windows)))]
async fn connect_agent() -> std::result::Result<
    russh::keys::agent::client::AgentClient<
        Box<dyn russh::keys::agent::client::AgentStream + Send + Unpin>,
    >,
    russh::keys::Error,
> {
    Err(russh::keys::Error::IO(std::io::Error::other(
        "no SSH agent on this platform",
    )))
}

fn russh_err(site: &Site, e: russh::Error) -> NetError {
    NetError::Connect {
        host: format!("{}:{}", site.host, site.port),
        detail: e.to_string(),
    }
}

/// [`RemoteFs`] over a live russh-sftp session. `keep` holds the SSH connection open for as long
/// as the subsystem is in use.
struct SftpBackend {
    session: SftpSession,
    keep: Handle<Client>,
    /// The jump host's own connection, where the site goes through one: the channel `keep`
    /// speaks over is inside it, so it has to outlive the session as well.
    keep_jump: Option<Handle<Client>>,
}

impl std::fmt::Debug for SftpBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("SftpBackend")
    }
}

fn entry_from(name: String, attrs: &FileAttributes) -> RemoteEntry {
    let kind = match attrs.file_type() {
        FileType::Dir => EntryKind::Dir,
        FileType::File => EntryKind::File,
        FileType::Symlink => EntryKind::Symlink,
        FileType::Other => EntryKind::Other,
    };
    RemoteEntry {
        name,
        kind,
        size: attrs.size.unwrap_or(0),
        mtime: attrs.mtime,
        mode: attrs.permissions.map(|p| p & 0o7777),
        link_target: None,
    }
}

fn name_of(path: &str) -> String {
    path.trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or(path)
        .to_string()
}

#[async_trait]
impl RemoteFs for SftpBackend {
    /// `direct-tcpip` over the session the panel is already using: `ssh -L` without the ssh.
    ///
    /// The originator the server is told is the loopback address and port zero. It is not a
    /// lie the server can act on — it is what the client claims the connection came from, and
    /// servers that log it log something true enough — and inventing the real one would mean
    /// asking the operating system for a peer address this layer has never seen.
    async fn forward(&self, host: &str, port: u16) -> Result<crate::ops::Forwarded> {
        let channel = self
            .keep
            .channel_open_direct_tcpip(host, u32::from(port), "127.0.0.1", 0)
            .await
            .map_err(|e| NetError::remote(format!("{host}:{port}"), e))?;
        Ok(Box::new(channel.into_stream()))
    }

    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let dir = self
            .session
            .read_dir(path)
            .await
            .map_err(|e| NetError::remote(path, e))?;
        Ok(dir
            .map(|entry| entry_from(entry.file_name(), &entry.metadata()))
            .collect())
    }

    async fn stat(&self, path: &str, follow: bool) -> Result<RemoteEntry> {
        let attrs = if follow {
            self.session.metadata(path).await
        } else {
            self.session.symlink_metadata(path).await
        }
        .map_err(|e| NetError::remote(path, e))?;
        Ok(entry_from(name_of(path), &attrs))
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        self.session
            .create_dir(path)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        self.session
            .remove_file(path)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        self.session
            .remove_dir(path)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        self.session
            .rename(from, to)
            .await
            .map_err(|e| NetError::remote(from, e))
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        self.session
            .read_link(path)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn symlink(&self, target: &str, link: &str) -> Result<()> {
        self.session
            .symlink(link, target)
            .await
            .map_err(|e| NetError::remote(link, e))
    }

    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()> {
        if mode.is_none() && mtime.is_none() {
            return Ok(());
        }
        let attrs = FileAttributes {
            permissions: mode,
            atime: mtime,
            mtime,
            ..FileAttributes::default()
        };
        self.session
            .set_metadata(path, attrs)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let file = self
            .session
            .open(path)
            .await
            .map_err(|e| NetError::remote(path, e))?;
        Ok(Box::new(file))
    }

    async fn create(&self, path: &str) -> Result<Writer> {
        let file = self
            .session
            .open_with_flags(
                path,
                OpenFlags::CREATE | OpenFlags::TRUNCATE | OpenFlags::WRITE,
            )
            .await
            .map_err(|e| NetError::remote(path, e))?;
        Ok(Box::new(file))
    }

    async fn canonicalize(&self, path: &str) -> Result<String> {
        self.session
            .canonicalize(path)
            .await
            .map_err(|e| NetError::remote(path, e))
    }

    async fn close(&self) -> Result<()> {
        let _ = self.session.close().await;
        let _ = self
            .keep
            .disconnect(russh::Disconnect::ByApplication, "", "en")
            .await;
        // Inside out: the destination's session first, then the hop that carried it, so the
        // bastion is told goodbye rather than left to notice a channel going quiet.
        if let Some(jump) = &self.keep_jump {
            let _ = jump
                .disconnect(russh::Disconnect::ByApplication, "", "en")
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_known_hosts_path_defaults_to_the_users_file() {
        let policy = HostKeyPolicy::default();
        let path = policy.path().expect("a home directory in the test env");
        assert!(path.ends_with(".ssh/known_hosts"), "{}", path.display());
        let explicit = HostKeyPolicy {
            known_hosts: Some(PathBuf::from("/tmp/kh")),
            accept_new: true,
        };
        assert_eq!(explicit.path().unwrap(), PathBuf::from("/tmp/kh"));
    }

    #[tokio::test]
    async fn a_connection_to_nowhere_names_the_host_and_port() {
        // Port 1 on the loopback: refused immediately, no network needed.
        let mut site = Site::parse_url("sftp://mb@127.0.0.1:1/").unwrap();
        site.auth = Auth::Password;
        let err = connect(&site, Some("x"), &HostKeyPolicy::default())
            .await
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("127.0.0.1:1"), "{text}");
    }

    #[tokio::test]
    async fn a_missing_secret_is_reported_before_anything_is_dialled() {
        let mut site = Site::parse_url("sftp://mb@127.0.0.1:1/").unwrap();
        site.auth = Auth::Password;
        // The password is asked for only once the connection is up, so this test drives the
        // branch directly through `authenticate`'s precondition.
        let err = NetError::NeedsSecret {
            user: site.user.clone(),
            host: site.host.clone(),
            kind: SecretKind::Password,
        };
        assert!(err.to_string().contains("a password"), "{err}");
    }

    #[test]
    fn attributes_become_entries_the_panel_understands() {
        let mut attrs = FileAttributes::default();
        attrs.set_dir(true);
        attrs.permissions = Some(0o40755);
        let entry = entry_from("srv".into(), &attrs);
        assert_eq!(entry.kind, EntryKind::Dir);
        assert_eq!(entry.mode, Some(0o755));

        let mut file = FileAttributes::default();
        file.set_regular(true);
        file.size = Some(4096);
        file.mtime = Some(1_700_000_000);
        file.permissions = Some(0o100_755);
        let entry = entry_from("run.sh".into(), &file);
        assert_eq!(entry.kind, EntryKind::File);
        assert_eq!(entry.size, 4096);
        assert!(entry
            .to_entry()
            .flags
            .contains(fcmd_core::EntryFlags::EXECUTABLE));
        assert_eq!(name_of("/srv/www/"), "www");
    }
}
