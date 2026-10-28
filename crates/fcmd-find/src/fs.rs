use std::io::{self, Read};

use fcmd_core::{CancelToken, Entry, Result, VPath};
use fcmd_vfs::{local, LocalVfs};

/// What the walk remembers about a directory it has already read.
///
/// Device and inode where the platform gives them, because that is the only identity that
/// survives a symbolic link, a bind mount and a second name for the same directory; a
/// canonical path everywhere else, which is weaker but still breaks a loop.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum VisitId {
    Fs(u64, u64),
    Path(String),
}

/// The filesystem a search walks and reads.
///
/// `fcmd-vfs`'s own [`Vfs`](fcmd_vfs::Vfs) trait is `async` and has no "give me the bytes"
/// method, so a synchronous engine — this one, like the job engine next door — cannot be
/// written against it as it stands. This trait is the four operations a search needs, and it
/// is implemented for [`LocalVfs`] in terms of `fcmd_vfs::local`, so a caller hands over the
/// VFS object it already has and a provider that learns to read becomes one more `impl`
/// rather than a second walker.
pub trait FindFs: Send + Sync {
    /// Read a directory. The `..` entry may be in the result; the walk drops it.
    fn list(&self, dir: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>>;

    /// Open a file for sequential reading. The engine reads in fixed chunks and never asks
    /// how long the file claims to be, so a stream with no length at all is enough.
    fn open(&self, path: &VPath) -> io::Result<Box<dyn Read + Send>>;

    /// Identity of a directory, with links followed. `None` means "this provider has no
    /// answer", and the walk falls back to the path it arrived by.
    fn visit_id(&self, dir: &VPath) -> Option<VisitId> {
        let _ = dir;
        None
    }

    /// Does `path` point at a directory once links are followed? Asked only about symbolic
    /// links, and only when the query says to follow them, so a provider without links can
    /// leave it alone.
    fn is_dir_following(&self, path: &VPath) -> bool {
        let _ = path;
        false
    }

    /// Is this file something the walk may step into as if it were a directory — an archive?
    ///
    /// Asked only when the query says to search inside archives, and only about files. A
    /// provider that says yes must then list and open the paths under `path.enter("/")`.
    fn opens_as_dir(&self, path: &VPath) -> bool {
        let _ = path;
        false
    }
}

/// The local filesystem, through the VFS's own lister rather than a second `read_dir` loop.
impl FindFs for LocalVfs {
    fn list(&self, dir: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>> {
        local::list_blocking(dir, cancel)
    }

    fn open(&self, path: &VPath) -> io::Result<Box<dyn Read + Send>> {
        if !path.is_local() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "the local provider cannot read a remote path",
            ));
        }
        Ok(Box::new(std::fs::File::open(path.as_path())?))
    }

    fn visit_id(&self, dir: &VPath) -> Option<VisitId> {
        if !dir.is_local() {
            return None;
        }
        local_visit_id(dir.as_path())
    }

    fn is_dir_following(&self, path: &VPath) -> bool {
        // `metadata`, not `symlink_metadata`: the question is what the link points at.
        path.is_local() && std::fs::metadata(path.as_path()).is_ok_and(|m| m.is_dir())
    }
}

#[cfg(unix)]
fn local_visit_id(path: &std::path::Path) -> Option<VisitId> {
    use std::os::unix::fs::MetadataExt;
    let meta = std::fs::metadata(path).ok()?;
    Some(VisitId::Fs(meta.dev(), meta.ino()))
}

#[cfg(not(unix))]
fn local_visit_id(path: &std::path::Path) -> Option<VisitId> {
    // Windows has a file index, but only behind `GetFileInformationByHandle`; until
    // `fcmd-platform` offers it, the canonical path is what breaks a junction loop.
    Some(VisitId::Path(
        std::fs::canonicalize(path)
            .ok()?
            .to_string_lossy()
            .into_owned(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_local_provider_lists_and_reads_through_one_object() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        let fs: &dyn FindFs = &LocalVfs;

        let entries = fs
            .list(&VPath::local(dir.path()), &CancelToken::new())
            .unwrap();
        assert!(entries.iter().any(|e| e.name_str() == "a.txt"));

        let mut text = String::new();
        fs.open(&VPath::local(dir.path().join("a.txt")))
            .unwrap()
            .read_to_string(&mut text)
            .unwrap();
        assert_eq!(text, "hello");
    }

    #[test]
    fn two_names_for_one_directory_share_an_identity_where_the_platform_says_so() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let fs = LocalVfs;

        let direct = fs.visit_id(&VPath::local(dir.path().join("sub")));
        let round_about = fs.visit_id(&VPath::local(dir.path().join("sub").join(".")));
        assert!(direct.is_some());
        assert_eq!(direct, round_about);
    }

    #[test]
    fn a_remote_path_is_not_something_the_local_provider_will_read() {
        let path = VPath::remote(fcmd_core::Scheme::Sftp, "mb@host:22", "/srv/www/index.html");
        let Err(err) = LocalVfs.open(&path) else {
            panic!("the local provider opened an sftp path");
        };
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
        assert!(LocalVfs.visit_id(&path).is_none());
    }
}
