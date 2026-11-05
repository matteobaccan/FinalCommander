//! IMAP, read only: the mailboxes of a server as folders and their messages as `.eml` files.
//!
//! `imap://me@host/` (port 143, plain) or `imaps://me@host/` (port 993, TLS from the first
//! byte) is a panel whose folders are the mailboxes `LIST` returns, with the server's own
//! hierarchy separator turned into `/`, and whose files are the messages of each one, named
//! `<uid> <subject>.eml` and sized as the server counts them. `F3` and `F5` fetch a message
//! whole, with `BODY.PEEK[]` so that reading one does not mark it seen; `Enter` on the copy is
//! the mail container (SPEC 04 §4d). Nothing is ever written, moved, flagged or deleted: a
//! mailbox is opened with `EXAMINE`, which the protocol defines as read-only.
//!
//! The client is the protocol's small core written out by hand — `LOGIN`, `LIST`, `EXAMINE`,
//! `UID FETCH`, `LOGOUT` — over one connection, one command at a time. A response is read up
//! to its tagged line, with literals (`{123}` then that many bytes) taken whole wherever they
//! appear, and parsed into the lists and strings the protocol is made of.

pub mod parse;

#[cfg(test)]
pub(crate) mod fake;

use std::sync::Arc;

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::error::{NetError, Result};
use crate::ops::{Duplex, Reader, RemoteEntry, RemoteFs, Writer};
use crate::remote::Remote;
use crate::site::{Auth, Site};
use crate::SecretKind;
use parse::Summary;

pub const IMAP_PORT: u16 = 143;
pub const IMAPS_PORT: u16 = 993;

/// The largest message this will fetch into memory: a mailbox is not where a disc image
/// belongs, and a server that says otherwise is answered with an error, not a full memory.
const MESSAGE_LIMIT: usize = 256 * 1024 * 1024;

/// Open a site whose scheme is `imap` or `imaps`.
pub async fn connect(site: &Site, secret: Option<&str>) -> Result<Remote> {
    if let Auth::Key { .. } = site.auth {
        return Err(NetError::Unsupported {
            path: site.url(),
            detail: "IMAP has no key authentication; use a password".into(),
        });
    }
    let Some(password) = secret else {
        return Err(NetError::NeedsSecret {
            user: site.user.clone(),
            host: site.host.clone(),
            kind: SecretKind::Password,
        });
    };
    let tls = site.scheme == fcmd_core::Scheme::Imaps;
    let mut client = Client::dial(&site.host, site.port, tls).await?;
    client.login(&site.user, password).await?;
    let delimiter = client.delimiter().await.unwrap_or('/');
    Ok(Remote::new(
        site.clone(),
        Arc::new(ImapFs {
            client: Mutex::new(client),
            delimiter,
        }),
    ))
}

/// One logged-in connection.
pub(crate) struct Client {
    stream: BufReader<Box<dyn Duplex>>,
    next: u32,
    host: String,
}

impl Client {
    pub(crate) async fn dial(host: &str, port: u16, tls: bool) -> Result<Client> {
        let connect_err = |e: std::io::Error| NetError::Connect {
            host: host.to_string(),
            detail: e.to_string(),
        };
        let tcp = TcpStream::connect((host, port))
            .await
            .map_err(connect_err)?;
        let stream: Box<dyn Duplex> = if tls {
            Box::new(tls_wrap(host, tcp).await?)
        } else {
            Box::new(tcp)
        };
        let mut client = Client {
            stream: BufReader::new(stream),
            next: 1,
            host: host.to_string(),
        };
        let greeting = client.read_line().await?;
        if !greeting.starts_with("* OK") && !greeting.starts_with("* PREAUTH") {
            return Err(NetError::Connect {
                host: host.to_string(),
                detail: format!("not an IMAP server: {}", greeting.trim_end()),
            });
        }
        Ok(client)
    }

    async fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        let n = self
            .stream
            .read_line(&mut line)
            .await
            .map_err(|e| self.lost(e))?;
        if n == 0 {
            return Err(self.lost(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)));
        }
        Ok(line)
    }

    fn lost(&self, e: std::io::Error) -> NetError {
        NetError::Connect {
            host: self.host.clone(),
            detail: e.to_string(),
        }
    }

    /// Send one command and read every line of its answer up to the tagged one, literals
    /// included. Returns the untagged responses as raw bytes, one per response.
    pub(crate) async fn command(&mut self, text: &str) -> Result<Vec<Vec<u8>>> {
        let tag = format!("a{}", self.next);
        self.next += 1;
        let line = format!("{tag} {text}\r\n");
        let stream = self.stream.get_mut();
        stream
            .write_all(line.as_bytes())
            .await
            .map_err(|e| NetError::Connect {
                host: self.host.clone(),
                detail: e.to_string(),
            })?;
        stream.flush().await.map_err(|e| NetError::Connect {
            host: self.host.clone(),
            detail: e.to_string(),
        })?;
        let mut untagged = Vec::new();
        loop {
            let mut response = Vec::new();
            loop {
                let line = self.read_line().await?;
                response.extend_from_slice(line.as_bytes());
                // A literal: `{n}` at the end of the line, then exactly n bytes, then the
                // rest of the same response on further lines.
                match parse::literal_length(line.trim_end_matches(['\r', '\n'])) {
                    Some(n) if n <= MESSAGE_LIMIT => {
                        let mut bytes = vec![0u8; n];
                        self.stream
                            .read_exact(&mut bytes)
                            .await
                            .map_err(|e| self.lost(e))?;
                        response.extend_from_slice(&bytes);
                    }
                    Some(_) => {
                        return Err(NetError::Remote {
                            path: self.host.clone(),
                            detail: "the server sent a message larger than 256 MiB".into(),
                        })
                    }
                    None => break,
                }
            }
            if response.starts_with(tag.as_bytes()) {
                let status = String::from_utf8_lossy(&response[tag.len()..])
                    .trim()
                    .to_string();
                if status.starts_with("OK") {
                    return Ok(untagged);
                }
                return Err(NetError::Remote {
                    path: self.host.clone(),
                    detail: status,
                });
            }
            if response.starts_with(b"+") {
                // A continuation request this client never asks for: answer nothing.
                continue;
            }
            untagged.push(response);
        }
    }

    pub(crate) async fn login(&mut self, user: &str, password: &str) -> Result<()> {
        let line = format!("LOGIN {} {}", parse::quote(user), parse::quote(password));
        self.command(&line).await.map(|_| ()).map_err(|e| match e {
            NetError::Remote { detail, .. } => NetError::Auth {
                user: user.to_string(),
                host: self.host.clone(),
                detail,
            },
            other => other,
        })
    }

    /// The hierarchy separator, from `LIST "" ""`.
    pub(crate) async fn delimiter(&mut self) -> Result<char> {
        let lines = self.command("LIST \"\" \"\"").await?;
        Ok(lines
            .iter()
            .find_map(|l| parse::list_line(l))
            .and_then(|m| m.delimiter)
            .unwrap_or('/'))
    }

    /// Every mailbox, as the server names them.
    pub(crate) async fn mailboxes(&mut self) -> Result<Vec<parse::Mailbox>> {
        let lines = self.command("LIST \"\" \"*\"").await?;
        Ok(lines.iter().filter_map(|l| parse::list_line(l)).collect())
    }

    /// The messages of one mailbox: uid, size, arrival time and subject.
    pub(crate) async fn summaries(&mut self, mailbox: &str) -> Result<Vec<Summary>> {
        let examined = self
            .command(&format!("EXAMINE {}", parse::quote(mailbox)))
            .await?;
        let empty = examined
            .iter()
            .any(|l| String::from_utf8_lossy(l).trim_end().ends_with(" 0 EXISTS"));
        if empty {
            return Ok(Vec::new());
        }
        let lines = self
            .command(
                "UID FETCH 1:* (UID RFC822.SIZE INTERNALDATE BODY.PEEK[HEADER.FIELDS (SUBJECT)])",
            )
            .await?;
        Ok(lines.iter().filter_map(|l| parse::summary(l)).collect())
    }

    /// One message whole, without marking it seen.
    pub(crate) async fn message(&mut self, mailbox: &str, uid: u32) -> Result<Vec<u8>> {
        self.command(&format!("EXAMINE {}", parse::quote(mailbox)))
            .await?;
        let lines = self
            .command(&format!("UID FETCH {uid} (BODY.PEEK[])"))
            .await?;
        lines
            .iter()
            .find_map(|l| parse::body(l))
            .ok_or_else(|| NetError::Remote {
                path: format!("{mailbox}/{uid}"),
                detail: "the server has no such message".into(),
            })
    }

    pub(crate) async fn logout(&mut self) {
        let _ = self.command("LOGOUT").await;
    }
}

async fn tls_wrap(
    host: &str,
    tcp: TcpStream,
) -> Result<suppaftp::tokio_rustls::client::TlsStream<TcpStream>> {
    use suppaftp::tokio_rustls::rustls::pki_types::ServerName;
    use suppaftp::tokio_rustls::rustls::{ClientConfig, RootCertStore};
    let fail = |detail: String| NetError::Connect {
        host: host.to_string(),
        detail,
    };
    let mut roots = RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = roots.add(cert);
    }
    if roots.is_empty() {
        return Err(fail(
            "this machine has no certificate authorities to check a server against".into(),
        ));
    }
    let provider = Arc::new(suppaftp::tokio_rustls::rustls::crypto::ring::default_provider());
    let config = ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| fail(e.to_string()))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    let name = ServerName::try_from(host.to_string()).map_err(|e| fail(e.to_string()))?;
    suppaftp::tokio_rustls::TlsConnector::from(Arc::new(config))
        .connect(name, tcp)
        .await
        .map_err(|e| fail(e.to_string()))
}

/// The panel's view of one IMAP connection.
pub(crate) struct ImapFs {
    client: Mutex<Client>,
    delimiter: char,
}

impl std::fmt::Debug for ImapFs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImapFs").finish_non_exhaustive()
    }
}

impl ImapFs {
    /// `/Work/2026` on the wire is `Work.2026` on a server whose separator is `.`.
    fn mailbox_of(&self, path: &str) -> String {
        path.trim_matches('/')
            .split('/')
            .collect::<Vec<_>>()
            .join(&self.delimiter.to_string())
    }

    fn read_only(path: &str) -> NetError {
        NetError::Unsupported {
            path: path.to_string(),
            detail: "an IMAP mailbox is read only here".into(),
        }
    }
}

#[async_trait]
impl RemoteFs for ImapFs {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let mut client = self.client.lock().await;
        let here = path.trim_matches('/');
        let mailboxes = client.mailboxes().await?;
        let sep = self.delimiter;
        let mut rows: Vec<RemoteEntry> = Vec::new();
        // The mailboxes one level below this one, as folders.
        for mailbox in &mailboxes {
            let as_path: String = mailbox.name.replace(sep, "/");
            let child = match here {
                "" => Some(as_path.as_str()),
                _ => as_path
                    .strip_prefix(here)
                    .and_then(|rest| rest.strip_prefix('/')),
            };
            if let Some(child) = child.filter(|c| !c.is_empty() && !c.contains('/')) {
                if !rows.iter().any(|r| r.name == child) {
                    rows.push(RemoteEntry::dir(child));
                }
            }
        }
        // And, inside a mailbox that can hold messages, the messages as files.
        let selectable = mailboxes
            .iter()
            .any(|m| m.name.replace(sep, "/") == here && m.selectable);
        if !here.is_empty() && selectable {
            for message in client.summaries(&self.mailbox_of(here)).await? {
                let mut row = RemoteEntry::file(message.file_name(), message.size);
                row.mtime = message.arrived;
                row.mode = Some(0o444);
                rows.push(row);
            }
        }
        Ok(rows)
    }

    async fn stat(&self, path: &str, _follow: bool) -> Result<RemoteEntry> {
        let trimmed = path.trim_matches('/');
        let name = trimmed.rsplit('/').next().unwrap_or("");
        if trimmed.is_empty() {
            return Ok(RemoteEntry::dir(""));
        }
        let parent = trimmed.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
        self.read_dir(parent)
            .await?
            .into_iter()
            .find(|r| r.name == name)
            .ok_or_else(|| NetError::Remote {
                path: path.to_string(),
                detail: "no such mailbox or message".into(),
            })
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        Err(Self::read_only(path))
    }
    async fn remove_file(&self, path: &str) -> Result<()> {
        Err(Self::read_only(path))
    }
    async fn remove_dir(&self, path: &str) -> Result<()> {
        Err(Self::read_only(path))
    }
    async fn rename(&self, from: &str, _to: &str) -> Result<()> {
        Err(Self::read_only(from))
    }
    async fn read_link(&self, path: &str) -> Result<String> {
        Err(Self::read_only(path))
    }
    async fn symlink(&self, _target: &str, link: &str) -> Result<()> {
        Err(Self::read_only(link))
    }
    async fn set_meta(&self, path: &str, _mode: Option<u32>, _mtime: Option<u32>) -> Result<()> {
        Err(Self::read_only(path))
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let trimmed = path.trim_matches('/');
        let (folder, name) = trimmed
            .rsplit_once('/')
            .ok_or_else(|| Self::read_only(path))?;
        let uid = parse::uid_of_name(name).ok_or_else(|| NetError::Remote {
            path: path.to_string(),
            detail: "not a message of this mailbox".into(),
        })?;
        let mailbox = self.mailbox_of(folder);
        let bytes = self.client.lock().await.message(&mailbox, uid).await?;
        Ok(Box::new(std::io::Cursor::new(bytes)))
    }

    async fn create(&self, path: &str) -> Result<Writer> {
        Err(Self::read_only(path))
    }

    async fn canonicalize(&self, path: &str) -> Result<String> {
        Ok(if path.is_empty() {
            "/".into()
        } else {
            path.to_string()
        })
    }

    async fn close(&self) -> Result<()> {
        self.client.lock().await.logout().await;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_mailboxes_are_folders_and_the_messages_files() {
        let server = fake::FakeImap::start(vec![
            (
                "INBOX",
                vec![
                    (
                        5,
                        "Subject: =?UTF-8?Q?Caff=C3=A8?= at ten\r\n\r\nSee you.\r\n",
                    ),
                    (9, "Subject: Report\r\n\r\nNumbers.\r\n"),
                ],
            ),
            ("Work/2026", vec![(1, "Subject: Plan\r\n\r\nSteps.\r\n")]),
            ("Work", vec![]),
        ])
        .await;
        let mut client = Client::dial("127.0.0.1", server.port, false).await.unwrap();
        assert!(
            client.login("me", "wrong").await.is_err(),
            "a bad password is refused"
        );
        let mut client = Client::dial("127.0.0.1", server.port, false).await.unwrap();
        client.login("me", "secret").await.unwrap();
        let fs = ImapFs {
            delimiter: client.delimiter().await.unwrap(),
            client: Mutex::new(client),
        };

        let top: Vec<String> = fs
            .read_dir("/")
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.name)
            .collect();
        assert_eq!(top, vec!["INBOX", "Work"]);
        let work: Vec<(String, bool)> = fs
            .read_dir("/Work")
            .await
            .unwrap()
            .into_iter()
            .map(|r| (r.name, r.kind == fcmd_core::EntryKind::Dir))
            .collect();
        assert_eq!(
            work,
            vec![("2026".to_string(), true)],
            "an empty mailbox has its child"
        );

        let inbox = fs.read_dir("/INBOX").await.unwrap();
        let names: Vec<&str> = inbox.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(names, vec!["5 Caffè at ten.eml", "9 Report.eml"]);
        assert!(inbox.iter().all(|r| r.size > 0 && r.mtime.is_some()));

        let mut reader = fs.open_read("/INBOX/9 Report.eml").await.unwrap();
        let mut text = String::new();
        tokio::io::AsyncReadExt::read_to_string(&mut reader, &mut text)
            .await
            .unwrap();
        assert!(text.contains("Numbers."), "{text}");
        let seen = server.seen.lock().unwrap().clone();
        assert!(
            seen.iter().any(|c| c.contains("UID FETCH 9 (BODY.PEEK[])")),
            "{seen:?}"
        );
        assert!(
            seen.iter().all(|c| !c.contains("SELECT ")),
            "only EXAMINE, never SELECT"
        );
        assert!(fs.remove_file("/INBOX/9 Report.eml").await.is_err());
    }

    /// The whole way a panel goes: a site from a URL, the pool, a path of the panel's.
    #[tokio::test]
    async fn a_mailbox_is_read_through_the_pool_like_any_server() {
        let server =
            fake::FakeImap::start(vec![("INBOX", vec![(3, "Subject: Hi\r\n\r\nBody.\r\n")])]).await;
        let site = Site::parse_url(&format!("imap://me@127.0.0.1:{}/", server.port)).unwrap();
        assert_eq!(site.scheme, fcmd_core::Scheme::Imap);
        let pool = crate::pool::Pool::new();
        assert!(
            matches!(
                pool.connect(&site, None).await,
                Err(NetError::NeedsSecret { .. })
            ),
            "the password is asked for, never assumed"
        );
        let remote = pool.connect(&site, Some("secret")).await.unwrap();
        let inbox = fcmd_core::VPath::parse(&format!("{}/INBOX", site.key()));
        let rows = remote
            .list(&inbox, &fcmd_core::CancelToken::new())
            .await
            .unwrap();
        let message = rows
            .iter()
            .find(|r| r.name_str() != "..")
            .expect("the message, beside `..`");
        assert_eq!(rows.len(), 2);
        assert_eq!(message.name_str(), "3 Hi.eml");
        assert!(message.flags.contains(fcmd_core::EntryFlags::READONLY));
        assert_eq!(
            crate::vfs::caps_of(site.scheme),
            fcmd_vfs::Caps::READ | fcmd_vfs::Caps::CASE_SENSITIVE
        );
    }
}
