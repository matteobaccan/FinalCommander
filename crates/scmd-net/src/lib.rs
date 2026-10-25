//! netbox: the network panel's filesystems (SFTP, FTP, FTPS and WebDAV) and the site list.
//!
//! See `docs/spec/04-vfs.md` for the provider contract and `docs/spec/08-plugins.md` §8 for
//! why this is one of the three components that matter first.

pub mod clock;
pub mod dav;
pub mod error;
pub mod ftp;
pub mod jobs;
pub mod memory;
pub mod ops;
pub mod pool;
pub mod remote;
pub mod site;
pub mod ssh;
pub mod tunnel;
pub mod vfs;

pub use error::{NetError, Result, SecretKind};
pub use jobs::RoutedFs;
pub use memory::MemoryFs;
pub use ops::{RemoteEntry, RemoteFs};
pub use pool::Pool;
pub use remote::Remote;
pub use site::{Auth, Site};
pub use ssh::{connect, HostKeyPolicy};
pub use tunnel::{Spec as TunnelSpec, Tunnel};
pub use vfs::{NetVfs, Router};
