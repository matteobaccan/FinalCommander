//! The virtual filesystem layer: one trait, many providers.
//!
//! M1 ships only the local provider; the trait and the capability flags exist so that no
//! caller may assume a local filesystem. See `docs/spec/04-vfs.md`.

mod cancel;
mod caps;
pub mod local;

pub use cancel::CancelToken;
pub use caps::Caps;
pub use local::LocalVfs;

use async_trait::async_trait;
use sc_core::{Entry, Result, VPath};

#[async_trait]
pub trait Vfs: Send + Sync {
    fn scheme(&self) -> &'static str;

    fn caps(&self) -> Caps;

    /// Read a directory. Must be cancellable and must not block the caller's thread.
    async fn list(&self, path: &VPath, cancel: CancelToken) -> Result<Vec<Entry>>;

    /// Metadata of a single path.
    async fn stat(&self, path: &VPath) -> Result<Entry>;

    /// Can we seek cheaply? The viewer needs this for hex mode and for huge files.
    fn random_access(&self) -> bool {
        self.caps().contains(Caps::SEEK)
    }
}
