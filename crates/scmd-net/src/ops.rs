use std::ffi::OsString;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use scmd_core::{Entry, EntryFlags, EntryId, EntryKind};
use scmd_jobs::Facts;
use tokio::io::{AsyncRead, AsyncWrite};

use crate::error::Result;

/// One remote directory entry, as the wire describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEntry {
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    /// Seconds since the epoch, which is all SFTP version 3 carries.
    pub mtime: Option<u32>,
    pub mode: Option<u32>,
    pub link_target: Option<String>,
}

impl RemoteEntry {
    pub fn file(name: impl Into<String>, size: u64) -> RemoteEntry {
        RemoteEntry {
            name: name.into(),
            kind: EntryKind::File,
            size,
            mtime: None,
            mode: Some(0o644),
            link_target: None,
        }
    }

    pub fn dir(name: impl Into<String>) -> RemoteEntry {
        RemoteEntry {
            name: name.into(),
            kind: EntryKind::Dir,
            size: 0,
            mtime: None,
            mode: Some(0o755),
            link_target: None,
        }
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.mtime
            .map(|secs| UNIX_EPOCH + Duration::from_secs(u64::from(secs)))
    }

    /// A panel entry. The id is a hash of the name, because SFTP gives us no inode.
    pub fn to_entry(&self) -> Entry {
        let mut flags = EntryFlags::empty();
        if self.name.starts_with('.') {
            flags.insert(EntryFlags::HIDDEN);
        }
        if let Some(mode) = self.mode {
            if self.kind == EntryKind::File && mode & 0o111 != 0 {
                flags.insert(EntryFlags::EXECUTABLE);
            }
            if mode & 0o200 == 0 {
                flags.insert(EntryFlags::READONLY);
            }
        }
        Entry {
            name: OsString::from(&self.name),
            kind: self.kind,
            size: if self.kind == EntryKind::Dir {
                0
            } else {
                self.size
            },
            mtime: self.modified(),
            // SFTP version 3 carries an access time beside the modification time, and no
            // creation time at all. We do not read the access time yet, and sorting by a
            // timestamp nobody filled in would put every remote file at one end of the
            // panel, so both say `None` rather than lie.
            btime: None,
            atime: None,
            flags,
            id: EntryId::Name(name_hash(&self.name)),
            link_target: self.link_target.as_ref().map(OsString::from),
        }
    }

    /// What the job engine needs to know about a path.
    pub fn to_facts(&self) -> Facts {
        Facts {
            kind: self.kind,
            size: if self.kind == EntryKind::Dir {
                0
            } else {
                self.size
            },
            mtime: self.modified(),
            mode: self.mode,
            readonly: self.mode.is_some_and(|m| m & 0o200 == 0),
        }
    }
}

/// FNV-1a: a stable id for a name, so a selection survives a re-read.
pub fn name_hash(name: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in name.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    hash
}

pub type Reader = Box<dyn AsyncRead + Unpin + Send>;
pub type Writer = Box<dyn AsyncWrite + Unpin + Send>;

/// The remote filesystem operations the netbox needs, with paths as the wire spells them.
///
/// One real implementation ([`crate::ssh::SftpBackend`]) and one in-memory implementation
/// ([`crate::memory::MemorySftp`]) so that everything above this line is tested without a
/// server.
#[async_trait]
pub trait Sftp: Send + Sync + std::fmt::Debug {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>>;
    /// `follow = false` reports a symlink as a symlink (`lstat`).
    async fn stat(&self, path: &str, follow: bool) -> Result<RemoteEntry>;
    async fn create_dir(&self, path: &str) -> Result<()>;
    async fn remove_file(&self, path: &str) -> Result<()>;
    async fn remove_dir(&self, path: &str) -> Result<()>;
    async fn rename(&self, from: &str, to: &str) -> Result<()>;
    async fn read_link(&self, path: &str) -> Result<String>;
    async fn symlink(&self, target: &str, link: &str) -> Result<()>;
    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()>;
    async fn open_read(&self, path: &str) -> Result<Reader>;
    /// Create or truncate, then write.
    async fn create(&self, path: &str) -> Result<Writer>;
    /// Resolve `.`, `..` and a relative path against the login directory.
    async fn canonicalize(&self, path: &str) -> Result<String>;
    /// Close the connection. Called by an explicit disconnect; dropping also closes.
    async fn close(&self) -> Result<()> {
        Ok(())
    }

    async fn exists(&self, path: &str) -> bool {
        self.stat(path, false).await.is_ok()
    }
}
