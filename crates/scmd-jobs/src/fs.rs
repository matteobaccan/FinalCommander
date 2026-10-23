use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use scmd_core::{EntryKind, VPath};

use crate::spec::Preserve;

/// The facts about a path the engine needs: no more, so a remote provider can supply them
/// cheaply.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Facts {
    pub kind: EntryKind,
    pub size: u64,
    pub mtime: Option<SystemTime>,
    pub mode: Option<u32>,
    pub readonly: bool,
}

impl Facts {
    pub fn is_dir(&self) -> bool {
        self.kind == EntryKind::Dir
    }
}

/// The filesystem operations a job needs.
///
/// Today only [`LocalFs`] implements it; when a VFS provider gains `WRITE` it implements this
/// too and the engine runs unchanged (`docs/spec/04-vfs.md`).
pub trait JobFs: Send + Sync {
    /// Facts about the path itself: a symlink reports as a symlink.
    fn facts(&self, path: &VPath) -> io::Result<Facts>;
    /// Facts about what the path points at, for `follow_symlinks`.
    fn facts_following(&self, path: &VPath) -> io::Result<Facts>;
    fn exists(&self, path: &VPath) -> bool;
    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>>;
    fn create_dir(&self, path: &VPath) -> io::Result<()>;
    fn create_dir_all(&self, path: &VPath) -> io::Result<()>;
    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()>;
    fn remove_file(&self, path: &VPath) -> io::Result<()>;
    fn remove_dir(&self, path: &VPath) -> io::Result<()>;
    fn read_link(&self, path: &VPath) -> io::Result<PathBuf>;
    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()>;
    fn same_file(&self, a: &VPath, b: &VPath) -> bool;
    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()>;

    /// Copy `src` to `dst`, calling `on_bytes` as it goes. Returning `false` from `on_bytes`
    /// cancels the copy, and the partial file is removed.
    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64>;
}

/// The local filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

/// Suffix of the temporary file a copy writes into. A destination never appears under its final
/// name until it is complete.
pub const PART_SUFFIX: &str = ".scmd-part";

impl JobFs for LocalFs {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        let meta = fs::symlink_metadata(path.as_path())?;
        Ok(facts_from(&meta))
    }

    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        let meta = fs::metadata(path.as_path())?;
        Ok(facts_from(&meta))
    }

    fn exists(&self, path: &VPath) -> bool {
        fs::symlink_metadata(path.as_path()).is_ok()
    }

    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(path.as_path())? {
            let entry = entry?;
            let meta = match entry.metadata() {
                Ok(m) => m,
                Err(_) => fs::symlink_metadata(entry.path())?,
            };
            out.push((entry.file_name(), facts_from(&meta)));
        }
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out)
    }

    fn create_dir(&self, path: &VPath) -> io::Result<()> {
        match fs::create_dir(path.as_path()) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            other => other,
        }
    }

    fn create_dir_all(&self, path: &VPath) -> io::Result<()> {
        fs::create_dir_all(path.as_path())
    }

    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()> {
        fs::rename(from.as_path(), to.as_path())
    }

    fn remove_file(&self, path: &VPath) -> io::Result<()> {
        // A read-only file still has to go when the user says delete.
        if let Ok(meta) = fs::symlink_metadata(path.as_path()) {
            if meta.permissions().readonly() {
                let mut perms = meta.permissions();
                #[allow(clippy::permissions_set_readonly_false)]
                perms.set_readonly(false);
                let _ = fs::set_permissions(path.as_path(), perms);
            }
        }
        fs::remove_file(path.as_path())
    }

    fn remove_dir(&self, path: &VPath) -> io::Result<()> {
        fs::remove_dir(path.as_path())
    }

    fn read_link(&self, path: &VPath) -> io::Result<PathBuf> {
        fs::read_link(path.as_path())
    }

    #[cfg(unix)]
    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link.as_path())
    }

    #[cfg(windows)]
    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()> {
        // Without developer mode or elevation this fails; the engine turns that into a normal
        // error decision point rather than a crash.
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link.as_path())
        } else {
            std::os::windows::fs::symlink_file(target, link.as_path())
        }
    }

    fn same_file(&self, a: &VPath, b: &VPath) -> bool {
        match (fs::canonicalize(a.as_path()), fs::canonicalize(b.as_path())) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }

    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()> {
        if preserve.mtime {
            if let Some(mtime) = facts.mtime {
                // Opening for write would truncate nothing but does need write access; a
                // failure here is not worth losing the copy over.
                if let Ok(file) = File::options().write(true).open(path.as_path()) {
                    let _ = file.set_modified(mtime);
                }
            }
        }
        #[cfg(unix)]
        if preserve.mode {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = facts.mode {
                let _ = fs::set_permissions(path.as_path(), fs::Permissions::from_mode(mode));
            }
        }
        Ok(())
    }

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let part = part_path(dst);
        let mut reader = File::open(src.as_path())?;
        let mut writer = File::create(part.as_path())?;
        let mut buf = vec![0u8; buffer.clamp(16 * 1024, 8 * 1024 * 1024)];
        let mut total = 0u64;

        loop {
            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    drop(writer);
                    let _ = fs::remove_file(part.as_path());
                    return Err(e);
                }
            };
            if let Err(e) = writer.write_all(&buf[..read]) {
                drop(writer);
                let _ = fs::remove_file(part.as_path());
                return Err(e);
            }
            total += read as u64;
            if !on_bytes(read as u64) {
                drop(writer);
                let _ = fs::remove_file(part.as_path());
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
        }

        writer.flush()?;
        drop(writer);
        // Only now does the destination appear under its final name.
        if let Err(e) = fs::rename(part.as_path(), dst.as_path()) {
            let _ = fs::remove_file(part.as_path());
            return Err(e);
        }
        Ok(total)
    }
}

pub fn part_path(dst: &VPath) -> VPath {
    let name = dst
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    match dst.parent() {
        Some(parent) => parent.join(format!("{name}{PART_SUFFIX}")),
        None => VPath::local(format!("{name}{PART_SUFFIX}")),
    }
}

fn facts_from(meta: &fs::Metadata) -> Facts {
    let kind = if meta.is_dir() {
        EntryKind::Dir
    } else if meta.is_symlink() {
        EntryKind::Symlink
    } else if meta.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };
    Facts {
        kind,
        size: if kind == EntryKind::Dir {
            0
        } else {
            meta.len()
        },
        mtime: meta.modified().ok(),
        mode: mode_of(meta),
        readonly: meta.permissions().readonly(),
    }
}

#[cfg(unix)]
fn mode_of(meta: &fs::Metadata) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    Some(meta.permissions().mode())
}

#[cfg(not(unix))]
fn mode_of(_meta: &fs::Metadata) -> Option<u32> {
    None
}

/// `name (2).txt` — the first free variant next to `dst`.
pub fn free_name(fs: &dyn JobFs, dst: &VPath) -> VPath {
    if !fs.exists(dst) {
        return dst.clone();
    }
    let parent = match dst.parent() {
        Some(p) => p,
        None => return dst.clone(),
    };
    let full = dst
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (stem, ext) = match full.rfind('.') {
        Some(i) if i > 0 => (full[..i].to_string(), full[i..].to_string()),
        _ => (full.clone(), String::new()),
    };
    for n in 2..10_000u32 {
        let candidate = parent.join(format!("{stem} ({n}){ext}"));
        if !fs.exists(&candidate) {
            return candidate;
        }
    }
    dst.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_reports_progress_and_never_leaves_a_partial_destination() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        fs::write(src.as_path(), vec![7u8; 100_000]).unwrap();

        let mut seen = 0u64;
        let copied = LocalFs
            .copy_file(&src, &dst, 16 * 1024, &mut |n| {
                seen += n;
                true
            })
            .unwrap();
        assert_eq!(copied, 100_000);
        assert_eq!(seen, 100_000);
        assert_eq!(fs::read(dst.as_path()).unwrap().len(), 100_000);
        assert!(
            !LocalFs.exists(&part_path(&dst)),
            "the .scmd-part file stayed behind"
        );
    }

    #[test]
    fn a_cancelled_copy_removes_the_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        fs::write(src.as_path(), vec![0u8; 200_000]).unwrap();

        let err = LocalFs
            .copy_file(&src, &dst, 16 * 1024, &mut |_| false)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(!LocalFs.exists(&dst), "a cancelled copy left a destination");
        assert!(!LocalFs.exists(&part_path(&dst)));
    }

    #[test]
    fn metadata_is_carried_over() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("a"));
        let dst = VPath::local(dir.path().join("b"));
        fs::write(src.as_path(), b"hello").unwrap();
        let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000);
        File::options()
            .write(true)
            .open(src.as_path())
            .unwrap()
            .set_modified(when)
            .unwrap();

        let facts = LocalFs.facts(&src).unwrap();
        LocalFs
            .copy_file(&src, &dst, 64 * 1024, &mut |_| true)
            .unwrap();
        LocalFs
            .apply_meta(&dst, &facts, Preserve::default())
            .unwrap();
        assert_eq!(LocalFs.facts(&dst).unwrap().mtime, Some(when));
        #[cfg(unix)]
        assert_eq!(LocalFs.facts(&dst).unwrap().mode, facts.mode);
    }

    #[test]
    fn free_name_walks_around_what_exists() {
        let dir = tempfile::tempdir().unwrap();
        let target = VPath::local(dir.path().join("notes.txt"));
        assert_eq!(free_name(&LocalFs, &target), target);
        fs::write(target.as_path(), b"x").unwrap();
        let second = free_name(&LocalFs, &target);
        assert_eq!(
            second.file_name().unwrap().to_string_lossy(),
            "notes (2).txt"
        );
        fs::write(second.as_path(), b"x").unwrap();
        assert_eq!(
            free_name(&LocalFs, &target)
                .file_name()
                .unwrap()
                .to_string_lossy(),
            "notes (3).txt"
        );
    }

    #[test]
    fn read_only_files_can_still_be_deleted() {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("locked"));
        fs::write(path.as_path(), b"x").unwrap();
        let mut perms = fs::metadata(path.as_path()).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(path.as_path(), perms).unwrap();
        LocalFs.remove_file(&path).unwrap();
        assert!(!LocalFs.exists(&path));
    }

    #[test]
    fn same_file_sees_through_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let real = VPath::local(dir.path().join("real"));
        let link = VPath::local(dir.path().join("link"));
        fs::write(real.as_path(), b"x").unwrap();
        LocalFs
            .symlink(std::path::Path::new("real"), &link)
            .unwrap();
        assert!(LocalFs.same_file(&real, &link));
        assert!(!LocalFs.same_file(&real, &VPath::local(dir.path().join("nope"))));
    }
}
