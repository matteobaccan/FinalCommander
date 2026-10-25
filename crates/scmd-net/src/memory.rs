//! An in-memory [`RemoteFs`] implementation.
//!
//! It exists so that the panel, the job engine and the netbox's own path handling are tested
//! without a server: the layer that talks to `russh` is thin by design, and everything above
//! it is exercised here. It is also useful from the application's tests.

use std::collections::BTreeMap;
use std::io;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};

use async_trait::async_trait;
use scmd_core::EntryKind;
use tokio::io::AsyncWrite;

use crate::error::{NetError, Result};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Dir {
        mode: u32,
        mtime: u32,
    },
    File {
        data: Vec<u8>,
        mode: u32,
        mtime: u32,
    },
    Link {
        target: String,
    },
}

/// A fake SFTP server holding its whole filesystem in memory.
#[derive(Debug, Default)]
pub struct MemoryFs {
    /// Behind its own `Arc` so that a writer can commit after the call that made it returned.
    shared: Arc<Mutex<BTreeMap<String, Node>>>,
    /// Paths that fail, with the message the "server" returns. For error-path tests.
    pub broken: Mutex<BTreeMap<String, String>>,
}

impl MemoryFs {
    /// A server with `/` and nothing else.
    pub fn new() -> Arc<MemoryFs> {
        let fs = MemoryFs::default();
        fs.shared.lock().unwrap().insert(
            "/".to_string(),
            Node::Dir {
                mode: 0o755,
                mtime: 0,
            },
        );
        Arc::new(fs)
    }

    pub fn with_dir(self: &Arc<Self>, path: &str) -> Arc<Self> {
        let mut nodes = self.shared.lock().unwrap();
        let mut so_far = String::new();
        for part in path.trim_matches('/').split('/').filter(|p| !p.is_empty()) {
            so_far.push('/');
            so_far.push_str(part);
            nodes.entry(so_far.clone()).or_insert(Node::Dir {
                mode: 0o755,
                mtime: 0,
            });
        }
        drop(nodes);
        self.clone()
    }

    pub fn with_file(self: &Arc<Self>, path: &str, data: &[u8]) -> Arc<Self> {
        if let Some(parent) = parent_of(path) {
            self.with_dir(&parent);
        }
        self.shared.lock().unwrap().insert(
            normalise(path),
            Node::File {
                data: data.to_vec(),
                mode: 0o644,
                mtime: 1_700_000_000,
            },
        );
        self.clone()
    }

    pub fn with_link(self: &Arc<Self>, path: &str, target: &str) -> Arc<Self> {
        self.shared.lock().unwrap().insert(
            normalise(path),
            Node::Link {
                target: target.to_string(),
            },
        );
        self.clone()
    }

    /// Make every operation on `path` fail, as a server would.
    pub fn break_path(self: &Arc<Self>, path: &str, message: &str) -> Arc<Self> {
        self.broken
            .lock()
            .unwrap()
            .insert(normalise(path), message.to_string());
        self.clone()
    }

    /// The bytes of a file, for assertions.
    pub fn file(&self, path: &str) -> Option<Vec<u8>> {
        match self.shared.lock().unwrap().get(&normalise(path)) {
            Some(Node::File { data, .. }) => Some(data.clone()),
            _ => None,
        }
    }

    /// Every path the fake server holds, in order. Handy in assertions.
    pub fn paths(&self) -> Vec<String> {
        self.shared.lock().unwrap().keys().cloned().collect()
    }

    pub fn mode(&self, path: &str) -> Option<u32> {
        match self.shared.lock().unwrap().get(&normalise(path)) {
            Some(Node::File { mode, .. }) | Some(Node::Dir { mode, .. }) => Some(*mode),
            _ => None,
        }
    }

    pub fn mtime(&self, path: &str) -> Option<u32> {
        match self.shared.lock().unwrap().get(&normalise(path)) {
            Some(Node::File { mtime, .. }) | Some(Node::Dir { mtime, .. }) => Some(*mtime),
            _ => None,
        }
    }

    fn check(&self, path: &str) -> Result<()> {
        if let Some(message) = self.broken.lock().unwrap().get(path) {
            return Err(NetError::remote(path, message));
        }
        Ok(())
    }

    fn entry_at(&self, path: &str) -> Result<RemoteEntry> {
        let nodes = self.shared.lock().unwrap();
        let node = nodes
            .get(path)
            .ok_or_else(|| NetError::remote(path, "no such file or directory"))?;
        Ok(entry_from(name_of(path), node))
    }
}

fn entry_from(name: String, node: &Node) -> RemoteEntry {
    match node {
        Node::Dir { mode, mtime } => RemoteEntry {
            name,
            kind: EntryKind::Dir,
            size: 0,
            mtime: Some(*mtime),
            mode: Some(*mode),
            link_target: None,
        },
        Node::File { data, mode, mtime } => RemoteEntry {
            name,
            kind: EntryKind::File,
            size: data.len() as u64,
            mtime: Some(*mtime),
            mode: Some(*mode),
            link_target: None,
        },
        Node::Link { target } => RemoteEntry {
            name,
            kind: EntryKind::Symlink,
            size: 0,
            mtime: Some(0),
            mode: Some(0o777),
            link_target: Some(target.clone()),
        },
    }
}

#[async_trait]
impl RemoteFs for MemoryFs {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        let path = normalise(path);
        self.check(&path)?;
        match self.entry_at(&path)?.kind {
            EntryKind::Dir => {}
            _ => return Err(NetError::remote(&path, "not a directory")),
        }
        let prefix = if path == "/" {
            "/".to_string()
        } else {
            format!("{path}/")
        };
        let nodes = self.shared.lock().unwrap();
        Ok(nodes
            .iter()
            .filter(|(key, _)| {
                key.starts_with(&prefix)
                    && key.len() > prefix.len()
                    && !key[prefix.len()..].contains('/')
            })
            .map(|(key, node)| entry_from(name_of(key), node))
            .collect())
    }

    async fn stat(&self, path: &str, follow: bool) -> Result<RemoteEntry> {
        let path = normalise(path);
        self.check(&path)?;
        let entry = self.entry_at(&path)?;
        if follow {
            if let Some(target) = &entry.link_target {
                let resolved = if target.starts_with('/') {
                    normalise(target)
                } else {
                    join(&parent_of(&path).unwrap_or_else(|| "/".into()), target)
                };
                return self.entry_at(&resolved);
            }
        }
        Ok(entry)
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        let path = normalise(path);
        self.check(&path)?;
        let mut nodes = self.shared.lock().unwrap();
        if nodes.contains_key(&path) {
            return Err(NetError::remote(&path, "file already exists"));
        }
        if let Some(parent) = parent_of(&path) {
            if !nodes.contains_key(&parent) {
                return Err(NetError::remote(&path, "no such file or directory"));
            }
        }
        nodes.insert(
            path,
            Node::Dir {
                mode: 0o755,
                mtime: 0,
            },
        );
        Ok(())
    }

    async fn remove_file(&self, path: &str) -> Result<()> {
        let path = normalise(path);
        self.check(&path)?;
        let mut nodes = self.shared.lock().unwrap();
        match nodes.get(&path) {
            Some(Node::Dir { .. }) => Err(NetError::remote(&path, "is a directory")),
            Some(_) => {
                nodes.remove(&path);
                Ok(())
            }
            None => Err(NetError::remote(&path, "no such file or directory")),
        }
    }

    async fn remove_dir(&self, path: &str) -> Result<()> {
        let path = normalise(path);
        self.check(&path)?;
        let mut nodes = self.shared.lock().unwrap();
        match nodes.get(&path) {
            Some(Node::Dir { .. }) => {
                let prefix = format!("{path}/");
                if nodes.keys().any(|k| k.starts_with(&prefix)) {
                    return Err(NetError::remote(&path, "directory not empty"));
                }
                nodes.remove(&path);
                Ok(())
            }
            Some(_) => Err(NetError::remote(&path, "not a directory")),
            None => Err(NetError::remote(&path, "no such file or directory")),
        }
    }

    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        let (from, to) = (normalise(from), normalise(to));
        self.check(&from)?;
        self.check(&to)?;
        let mut nodes = self.shared.lock().unwrap();
        if nodes.contains_key(&to) {
            return Err(NetError::remote(&to, "file already exists"));
        }
        // Move the subtree, as a server would.
        let moved: Vec<String> = nodes
            .keys()
            .filter(|k| **k == from || k.starts_with(&format!("{from}/")))
            .cloned()
            .collect();
        if moved.is_empty() {
            return Err(NetError::remote(&from, "no such file or directory"));
        }
        for key in moved {
            let node = nodes.remove(&key).expect("just listed");
            let suffix = &key[from.len()..];
            nodes.insert(format!("{to}{suffix}"), node);
        }
        Ok(())
    }

    async fn read_link(&self, path: &str) -> Result<String> {
        let path = normalise(path);
        match self.shared.lock().unwrap().get(&path) {
            Some(Node::Link { target }) => Ok(target.clone()),
            Some(_) => Err(NetError::remote(&path, "not a symlink")),
            None => Err(NetError::remote(&path, "no such file or directory")),
        }
    }

    async fn symlink(&self, target: &str, link: &str) -> Result<()> {
        let link = normalise(link);
        self.check(&link)?;
        self.shared.lock().unwrap().insert(
            link,
            Node::Link {
                target: target.to_string(),
            },
        );
        Ok(())
    }

    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()> {
        let path = normalise(path);
        self.check(&path)?;
        let mut nodes = self.shared.lock().unwrap();
        match nodes.get_mut(&path) {
            Some(Node::File {
                mode: m, mtime: t, ..
            })
            | Some(Node::Dir { mode: m, mtime: t }) => {
                if let Some(mode) = mode {
                    *m = mode;
                }
                if let Some(mtime) = mtime {
                    *t = mtime;
                }
                Ok(())
            }
            Some(Node::Link { .. }) => Ok(()),
            None => Err(NetError::remote(&path, "no such file or directory")),
        }
    }

    async fn open_read(&self, path: &str) -> Result<Reader> {
        let path = normalise(path);
        self.check(&path)?;
        match self.shared.lock().unwrap().get(&path) {
            Some(Node::File { data, .. }) => Ok(Box::new(std::io::Cursor::new(data.clone()))),
            Some(_) => Err(NetError::remote(&path, "is a directory")),
            None => Err(NetError::remote(&path, "no such file or directory")),
        }
    }

    async fn create(&self, path: &str) -> Result<Writer> {
        let path = normalise(path);
        self.check(&path)?;
        {
            let nodes = self.shared.lock().unwrap();
            if let Some(parent) = parent_of(&path) {
                if !nodes.contains_key(&parent) {
                    return Err(NetError::remote(&path, "no such file or directory"));
                }
            }
            if matches!(nodes.get(&path), Some(Node::Dir { .. })) {
                return Err(NetError::remote(&path, "is a directory"));
            }
        }
        Ok(Box::new(MemoryWriter {
            path,
            buffer: Vec::new(),
            target: self.shared.clone(),
        }))
    }

    async fn canonicalize(&self, path: &str) -> Result<String> {
        Ok(normalise(path))
    }
}

/// Writes land in the map when the writer is flushed or shut down, so a half-written file is
/// never visible — the same guarantee the real provider gives through `.scmd-part`.
struct MemoryWriter {
    path: String,
    buffer: Vec<u8>,
    target: Arc<Mutex<BTreeMap<String, Node>>>,
}

impl AsyncWrite for MemoryWriter {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.buffer.extend_from_slice(buf);
        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let me = self.get_mut();
        me.target.lock().unwrap().insert(
            me.path.clone(),
            Node::File {
                data: me.buffer.clone(),
                mode: 0o644,
                mtime: 0,
            },
        );
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}

fn normalise(path: &str) -> String {
    let trimmed = path.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn parent_of(path: &str) -> Option<String> {
    let path = normalise(path);
    if path == "/" {
        return None;
    }
    let cut = path.rfind('/')?;
    Some(if cut == 0 {
        "/".to_string()
    } else {
        path[..cut].to_string()
    })
}

fn name_of(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

fn join(base: &str, name: &str) -> String {
    if base.ends_with('/') {
        format!("{base}{name}")
    } else {
        format!("{base}/{name}")
    }
}
