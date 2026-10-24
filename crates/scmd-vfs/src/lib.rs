//! The virtual filesystem layer: one trait, many providers.
//!
//! The local filesystem, and archives read through it. The trait and the capability flags
//! exist so that no caller may assume a local filesystem. See `docs/spec/04-vfs.md`.

pub mod arc;
mod caps;
pub mod local;

pub use arc::ArcVfs;
pub use caps::Caps;
pub use local::LocalVfs;
/// Re-exported so callers need only one import; the type lives in `scmd-core`.
pub use scmd_core::CancelToken;

use async_trait::async_trait;
use scmd_core::{Entry, Result, VPath};

/// A directory as a provider read it: the rows, and whether that is all of them.
///
/// A local directory is always read whole, and so is an archive's. Object storage is not: a
/// bucket can hold millions of keys, a listing of it is a thousand requests, and a panel that
/// pretends to show all of them is lying about something the user cares about. So a listing
/// may be cut short, and when it is, it says so and says why — and the panel shows the note
/// rather than a number nobody can tell is partial.
#[derive(Debug, Clone, Default)]
pub struct Dir {
    pub entries: Vec<Entry>,
    /// False when the provider stopped before the end.
    pub complete: bool,
    /// One line about the listing, when there is something to say.
    pub note: Option<String>,
}

impl Dir {
    /// Everything that is there.
    pub fn whole(entries: Vec<Entry>) -> Dir {
        Dir {
            entries,
            complete: true,
            note: None,
        }
    }

    /// As much as was read, and why it stopped.
    pub fn partial(entries: Vec<Entry>, note: impl Into<String>) -> Dir {
        Dir {
            entries,
            complete: false,
            note: Some(note.into()),
        }
    }
}

#[async_trait]
pub trait Vfs: Send + Sync {
    fn scheme(&self) -> &'static str;

    fn caps(&self) -> Caps;

    /// Read a directory. Must be cancellable and must not block the caller's thread.
    async fn list(&self, path: &VPath, cancel: CancelToken) -> Result<Vec<Entry>>;

    /// Metadata of a single path.
    async fn stat(&self, path: &VPath) -> Result<Entry>;

    /// Read a directory, with room to say that what came back is not all of it.
    ///
    /// The panel reads through this one; everything else — the job engine, the finder, a
    /// dialog filling a list — uses [`Vfs::list`] and gets the rows alone. A provider that
    /// always reads a directory whole need not implement it.
    async fn list_dir(&self, path: &VPath, cancel: CancelToken) -> Result<Dir> {
        Ok(Dir::whole(self.list(path, cancel).await?))
    }

    /// Can we seek cheaply? The viewer needs this for hex mode and for huge files.
    fn random_access(&self) -> bool {
        self.caps().contains(Caps::SEEK)
    }
}
