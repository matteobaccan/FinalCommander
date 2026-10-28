//! The handful of answers the standard library does not have.
//!
//! How much room is left on a device, how big its clusters are, how much memory the machine
//! has: a file manager has to show all three and `std` exposes none of them. Each platform
//! has its own call — `statvfs`, `GetDiskFreeSpaceEx`, `sysctl`, `/proc/meminfo` — and this
//! crate is the one place that makes them, so that the rest of the workspace can stay free of
//! `unsafe` and of `#[cfg]`.
//!
//! Everything here answers `Option`: a device that will not say, a platform that has no such
//! call, a path that has gone away. A file manager that guesses at these numbers is worse than
//! one that leaves the line blank, because a wrong free-space figure is the one a copy is
//! planned against.

pub mod process;
mod socket;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

pub use process::{details, processes, Details, Process};
pub use socket::{sockets, Socket};

use std::path::Path;

/// What a device has, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Disk {
    /// Everything on it, used and free.
    pub total: u64,
    /// What an ordinary user may still write. Not the same as "unused": a Unix keeps a
    /// reserve only root can touch, and it is not room anybody else can plan a copy against.
    pub free: u64,
    /// The allocation unit: the smallest number of bytes a file can take. Zero when the
    /// platform will not say.
    pub cluster: u64,
}

/// What the machine has, in bytes. `available` is what could be given to a program now,
/// which is not the same as unused — a page cache that can be dropped counts as available —
/// and is `None` where the platform does not offer the figure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Memory {
    pub total: u64,
    pub available: Option<u64>,
}

/// The device a local path is on. `None` for a path that is not local, has gone away, or sits
/// on something that does not answer.
pub fn disk(path: &Path) -> Option<Disk> {
    #[cfg(unix)]
    {
        unix::disk(path)
    }
    #[cfg(windows)]
    {
        windows::disk(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// Who a file belongs to, as names rather than numbers — what a panel row has room for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Owner {
    pub user: String,
    pub group: String,
}

/// What a run of `owner` calls learns on the way, so that it is learned once.
///
/// A directory listing is nearly always one or two accounts repeated, and turning an id or a
/// SID into a name can mean a directory server on the other side of a network. The cache is
/// handed in rather than kept in a static: the caller — one worker reading one directory —
/// owns it, which keeps this crate free of state and of locks.
#[derive(Debug, Default)]
pub struct OwnerCache {
    /// Unix: user ids to names.
    pub users: std::collections::HashMap<u32, String>,
    /// Unix: group ids to names.
    pub groups: std::collections::HashMap<u32, String>,
    /// Windows: SIDs, as text, to `DOMAIN\name`.
    pub sids: std::collections::HashMap<String, String>,
}

/// Who a local file belongs to. `None` for a path that has gone away, a filesystem that
/// carries no ownership at all, or a platform with no way to ask.
pub fn owner(path: &Path, cache: &mut OwnerCache) -> Option<Owner> {
    #[cfg(unix)]
    {
        unix::owner(path, cache)
    }
    #[cfg(windows)]
    {
        windows::owner(path, cache)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, cache);
        None
    }
}

/// How many names a file has: its hard-link count (FAR's links view, `Ctrl+9`).
///
/// One for an ordinary file, two or more for one that is in two directories at once, and a
/// number that nobody can read as "wasted space" — a hundred copies of a file that is really
/// one file take the room of one. `None` for a path that has gone away or a platform that
/// does not keep the count.
///
/// The link of a symbolic link is counted, not what it points at: the row on the screen is
/// the link, as it is everywhere else in a listing.
pub fn links(path: &Path) -> Option<u64> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        std::fs::symlink_metadata(path).ok().map(|m| m.nlink())
    }
    #[cfg(windows)]
    {
        windows::links(path)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        None
    }
}

/// What the machine has. `None` where there is no way to ask.
pub fn memory() -> Option<Memory> {
    #[cfg(unix)]
    {
        unix::memory()
    }
    #[cfg(windows)]
    {
        windows::memory()
    }
    #[cfg(not(any(unix, windows)))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The owner of a file this test has just written is whoever is running the test, and the
    /// cache has learned one name by the time the second file is asked about.
    #[test]
    fn a_file_belongs_to_somebody_and_the_name_is_learned_once() {
        let dir = std::env::temp_dir().join(format!(
            "fcmd-owner-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let one = dir.join("one");
        let two = dir.join("two");
        std::fs::write(&one, b"x").expect("write");
        std::fs::write(&two, b"y").expect("write");

        let mut cache = OwnerCache::default();
        let first = owner(&one, &mut cache).expect("a file has an owner");
        assert!(!first.user.is_empty(), "{first:?}");
        let second = owner(&two, &mut cache).expect("a file has an owner");
        assert_eq!(first, second, "two files written by one process");
        #[cfg(unix)]
        assert_eq!(cache.users.len(), 1, "the name should be learned once");

        // A path that is not there has no owner, rather than an empty one.
        assert_eq!(owner(&dir.join("gone"), &mut cache), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_device_this_test_runs_on_answers() {
        let here = std::env::temp_dir();
        let Some(d) = disk(&here) else {
            // A platform with no call of its own is allowed to say nothing; the three we
            // build for all have one, and CI runs this on each of them.
            if cfg!(any(unix, windows)) {
                panic!("a platform we support said nothing about {here:?}");
            }
            return;
        };
        assert!(d.total > 0, "a device with no size: {d:?}");
        assert!(d.free <= d.total, "{d:?}");
        assert!(d.cluster > 0, "the allocation unit: {d:?}");
        assert!(
            d.cluster.is_power_of_two(),
            "clusters come in powers of two: {d:?}"
        );
    }

    #[test]
    fn a_path_that_is_not_there_says_nothing_rather_than_zero() {
        assert_eq!(disk(Path::new("/no/such/place/at/all/here")), None);
    }

    #[test]
    fn the_machine_says_how_much_memory_it_has() {
        let Some(m) = memory() else {
            if cfg!(any(target_os = "linux", target_os = "macos", windows)) {
                panic!("a platform we support said nothing about its memory");
            }
            return;
        };
        assert!(m.total > 0, "{m:?}");
        if let Some(available) = m.available {
            assert!(available <= m.total, "{m:?}");
        }
    }
}
