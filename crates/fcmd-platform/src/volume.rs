//! What a drive letter *is*: the kind of device, its label, its filesystem, and the share
//! behind it where it is a mapped one (SPEC 02 §7).
//!
//! This is a Windows question. Every Unix answers it in the mount table, which the change-drive
//! dialog already reads — there is no call to make, and `volume` says so by answering `None`.
//! Windows keeps the four answers in three calls, none of which the standard library makes,
//! and without them three of that dialog's columns are empty on the one platform where a
//! drive letter needs explaining.

use std::path::Path;

/// What sort of thing a volume is, as the system says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeKind {
    Unknown,
    Removable,
    Fixed,
    /// A mapped network drive.
    Network,
    Optical,
    /// A RAM disk.
    Memory,
}

/// One volume, as far as the system will describe it. Every field but the kind may be absent:
/// an empty CD drive has no label and no filesystem, and asking is not an error.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Volume {
    pub kind: Option<VolumeKind>,
    pub label: Option<String>,
    pub fs: Option<String>,
    /// `\\server\share` for a mapped letter.
    pub share: Option<String>,
    /// Did the system describe the volume at all? It does not for a drive with nothing in
    /// it, which is the one question worth asking before going on to ask how big it is: on
    /// Windows, asking an empty drive for its free space puts a *dialog* on the screen.
    pub ready: bool,
}

/// Describe the volume at a root path (`C:\`), or `None` where this platform has nothing to
/// add to what the mount table already said.
pub fn volume(path: &Path) -> Option<Volume> {
    #[cfg(windows)]
    {
        super::windows::volume(path)
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}
