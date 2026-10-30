//! Containers that are not files on this disk (SPEC 04 §4).
//!
//! Every decoder here reads a *file*: a zip's index is at its end, a tar has to be walked, and
//! both want seeks. So an archive that is not a file on this disk — one inside another archive,
//! one on a server — is put on this disk first, once, and read from there.
//!
//! The copy is the honest price of the feature and it is paid out loud: it is counted against
//! a limit of its own (`[arc] max_spill_mb`), it is made only as deep as `[arc] max_depth`
//! allows, and it is thrown away when the program ends. Nothing above this layer knows it
//! happened — a path stays `/srv/outer.zip!/inner.zip!/readme.md`, which is where the file
//! really is, rather than becoming the name of a temporary file nobody asked for.
//!
//! A copy can be written to, but writing it is not the whole job: the archive it came out of
//! still holds the bytes it had. So the rebuild of SPEC 04 §4 puts the rebuilt copy back where
//! it came from and goes on outwards until the file on this disk, which is the one that really
//! changes. The container on a server is the one that cannot be written into: its copy would
//! change and nothing would carry it back.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use fcmd_core::{CancelToken, VPath};

use crate::{ArcError, Result};

/// Something that can put the bytes of a path on this disk: a server, a bucket — whatever the
/// application routes that scheme to. This crate knows none of them, only that something above
/// it does, so the archive code stays a reader of files.
pub trait Fetch: Send + Sync {
    /// Copy what `path` names into `to`, which does not exist yet.
    fn fetch(&self, path: &VPath, to: &Path, cancel: &CancelToken) -> Result<()>;

    /// How large the thing is, where the provider will say. A copy too large to make is
    /// refused before it starts rather than after it has filled the disk.
    fn size(&self, path: &VPath) -> Option<u64>;

    /// A word that changes when the thing does — its size and time, however the provider
    /// spells them. `None` where the provider will not say, and then a copy already made is
    /// used for the rest of the session rather than fetched again on every listing.
    fn stamp(&self, path: &VPath) -> Option<String>;
}

/// One copy that was made, and what it was made from.
#[derive(Debug)]
struct Made {
    /// The container's own path, with the stamp it had when it was copied.
    key: String,
    file: PathBuf,
}

/// Where the next copy's name comes from. Process-wide on purpose: two caches in one program
/// — a panel's and a test's — would otherwise both start at zero and name their first copy the
/// same thing, which on Windows is a file in use and everywhere else a file quietly shared.
static NEXT: AtomicU64 = AtomicU64::new(0);

/// The copies made for this session, and the fetcher that makes the remote ones.
///
/// Small on purpose: a handful of containers at a time is what a person opens, and each copy
/// is a whole archive on the disk.
#[derive(Default)]
pub struct Nest {
    made: Mutex<Vec<Made>>,
    fetch: Mutex<Option<Arc<dyn Fetch>>>,
    capacity: usize,
}

impl std::fmt::Debug for Nest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let made = self.made.lock().map(|m| m.len()).unwrap_or(0);
        f.debug_struct("Nest").field("made", &made).finish()
    }
}

impl Nest {
    /// A cache holding `capacity` copies at once. A rebuild of a container nested inside
    /// others holds one copy of every level alive while it walks back out to the disk, so the
    /// cache has to be at least as deep as the nesting limit allows.
    pub fn with_capacity(capacity: usize) -> Nest {
        Nest {
            made: Mutex::new(Vec::new()),
            fetch: Mutex::new(None),
            capacity,
        }
    }

    /// Hand it the way to reach what is not on this disk. Without one, a container on a server
    /// is refused with the reason rather than opened from thin air.
    pub fn set_fetch(&self, fetch: Arc<dyn Fetch>) {
        if let Ok(mut slot) = self.fetch.lock() {
            *slot = Some(fetch);
        }
    }

    pub(crate) fn fetcher(&self) -> Option<Arc<dyn Fetch>> {
        self.fetch.lock().ok().and_then(|f| f.clone())
    }

    /// The copy made for `key`, if it is still the right one and still there.
    fn hit(&self, key: &str) -> Option<PathBuf> {
        let mut made = self.made.lock().ok()?;
        let i = made.iter().position(|m| m.key == key)?;
        if !made[i].file.is_file() {
            // Somebody emptied the temporary directory under us, which is their right.
            made.remove(i);
            return None;
        }
        let hit = made.remove(i);
        let file = hit.file.clone();
        made.push(hit);
        Some(file)
    }

    /// Where the next copy goes. The name keeps the container's own, because a user who finds
    /// these files should be able to tell what they are.
    fn room_for(&self, name: &str) -> Result<PathBuf> {
        let dir = std::env::temp_dir()
            .join(format!("fcmd-{}", std::process::id()))
            .join("nested");
        std::fs::create_dir_all(&dir).map_err(|e| ArcError::Io {
            path: dir.display().to_string(),
            source: e,
        })?;
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let name = if name.is_empty() { "archive" } else { name };
        Ok(dir.join(format!("{n}-{name}")))
    }

    /// Remember a copy, and throw the oldest away once there are too many.
    fn keep(&self, key: String, file: PathBuf) {
        if let Ok(mut made) = self.made.lock() {
            made.push(Made { key, file });
            while made.len() > self.capacity.max(1) {
                let old = made.remove(0);
                let _ = std::fs::remove_file(&old.file);
            }
        }
    }

    /// A word that tells one state of `host` from another, for the copies made from it.
    ///
    /// For a container inside another, it is the file at the bottom of the chain: a rebuild
    /// writes a new archive, so its size and time change, and every copy taken out of it is
    /// stale at once. For one on a server, it is whatever the provider will say.
    fn stamp(&self, host: &VPath) -> String {
        if host.in_container() {
            let mut root = host.clone();
            while let Some(up) = root.leave() {
                root = up;
            }
            if root.is_local() {
                if let Ok(meta) = std::fs::metadata(root.as_path()) {
                    let when = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_nanos())
                        .unwrap_or(0);
                    return format!("{}:{when}", meta.len());
                }
                return "gone".into();
            }
        }
        self.fetcher()
            .and_then(|f| f.stamp(host))
            .unwrap_or_else(|| "once".into())
    }
}

impl Drop for Nest {
    /// The copies are this session's, and they go with it. A file left behind would be a whole
    /// archive in the temporary directory that nothing ever names again.
    fn drop(&mut self) {
        if let Ok(made) = self.made.get_mut() {
            for one in made.iter() {
                let _ = std::fs::remove_file(&one.file);
            }
        }
    }
}

/// What the spill limit has to say when it fires.
pub(crate) fn too_big(host: &VPath, size: u64, limit: u64) -> ArcError {
    ArcError::Refused {
        path: host.to_string(),
        reason: format!(
            "it would have to be copied to this disk first and it is {} MB, over the {} MB \
             `[arc] max_spill_mb` allows",
            size >> 20,
            limit >> 20
        ),
    }
}

/// What a container on a server meets when nothing can bring it here.
pub(crate) fn no_fetcher(host: &VPath) -> ArcError {
    ArcError::Refused {
        path: host.to_string(),
        reason: "an archive that is not on this disk has to be copied here first, and nothing \
                 in this program can reach that place"
            .into(),
    }
}

/// What the nesting limit has to say.
pub(crate) fn too_deep(host: &VPath, depth: usize) -> ArcError {
    ArcError::Refused {
        path: host.to_string(),
        reason: format!("more than {depth} containers deep (`[arc] max_depth`)"),
    }
}

impl Nest {
    /// Make the copy: the caller has already decided there is to be one.
    pub(crate) fn spill(
        &self,
        host: &VPath,
        make: impl FnOnce(&Path) -> Result<()>,
    ) -> Result<PathBuf> {
        let key = format!("{host}|{}", self.stamp(host));
        if let Some(hit) = self.hit(&key) {
            return Ok(hit);
        }
        let file = self.room_for(&host.display_name())?;
        match make(&file) {
            Ok(()) => {
                self.keep(key, file.clone());
                Ok(file)
            }
            Err(e) => {
                // A half-written copy is worse than none: the next open would read it.
                let _ = std::fs::remove_file(&file);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Cache, Limits};
    use fcmd_core::Scheme;
    use std::io::Write;
    use std::sync::atomic::AtomicUsize;

    /// A provider that has the archive somewhere else and can bring it here — which is all the
    /// cache asks of a server or a bucket.
    struct FromThere {
        file: PathBuf,
        asked: AtomicUsize,
    }

    impl Fetch for FromThere {
        fn fetch(&self, _path: &VPath, to: &Path, _cancel: &CancelToken) -> Result<()> {
            self.asked.fetch_add(1, Ordering::Relaxed);
            std::fs::copy(&self.file, to)
                .map(|_| ())
                .map_err(|e| ArcError::Io {
                    path: to.display().to_string(),
                    source: e,
                })
        }

        fn size(&self, _path: &VPath) -> Option<u64> {
            std::fs::metadata(&self.file).ok().map(|m| m.len())
        }

        fn stamp(&self, _path: &VPath) -> Option<String> {
            Some("the same as it was".into())
        }
    }

    fn a_zip(at: &Path) {
        let file = std::fs::File::create(at).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("readme.md", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(b"# over there").unwrap();
        zip.finish().unwrap();
    }

    fn over_there() -> VPath {
        VPath::remote(Scheme::Sftp, "host", "/srv/over-there.zip")
    }

    #[test]
    fn an_archive_on_a_server_is_copied_here_once_and_then_read_from_the_copy() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("over-there.zip");
        a_zip(&zip);
        let fetch = Arc::new(FromThere {
            file: zip.clone(),
            asked: AtomicUsize::new(0),
        });
        let cache = Cache::new(Limits::default());
        cache.set_fetch(fetch.clone());

        let host = over_there();
        let archive = cache.open_host(&host, &CancelToken::new()).unwrap();
        assert_eq!(archive.list("/").len(), 1);
        assert_eq!(
            archive.read("/readme.md", &CancelToken::new()).unwrap(),
            b"# over there"
        );
        // Opening it again does not fetch it again: that is the whole point of the copy.
        cache.open_host(&host, &CancelToken::new()).unwrap();
        assert_eq!(fetch.asked.load(Ordering::Relaxed), 1);
        // And it is not a container anything may write to.
        assert!(!Cache::is_writable_host(&host));
    }

    #[test]
    fn a_container_too_large_to_copy_here_is_refused_with_the_limit() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("over-there.zip");
        a_zip(&zip);
        let cache = Cache::new(Limits {
            spill_bytes: 4,
            ..Limits::default()
        });
        cache.set_fetch(Arc::new(FromThere {
            file: zip,
            asked: AtomicUsize::new(0),
        }));
        let error = cache
            .open_host(&over_there(), &CancelToken::new())
            .unwrap_err();
        assert!(error.to_string().contains("max_spill_mb"), "{error}");
    }

    #[test]
    fn with_nothing_that_can_fetch_the_refusal_says_so() {
        let cache = Cache::new(Limits::default());
        let error = cache
            .open_host(&over_there(), &CancelToken::new())
            .unwrap_err();
        assert!(error.to_string().contains("copied here first"), "{error}");
    }

    /// The copies are this session's: when the cache goes, they go with it.
    #[test]
    fn the_copies_are_thrown_away_with_the_cache() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("over-there.zip");
        a_zip(&zip);
        let made = {
            let cache = Cache::new(Limits::default());
            cache.set_fetch(Arc::new(FromThere {
                file: zip,
                asked: AtomicUsize::new(0),
            }));
            let file = cache.host_file(&over_there(), &CancelToken::new()).unwrap();
            assert!(file.is_file());
            file
        };
        assert!(!made.exists(), "{} is still there", made.display());
    }
}
