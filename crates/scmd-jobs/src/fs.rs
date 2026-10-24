use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use scmd_core::{EntryKind, VPath};

use crate::replace::{self, Matcher, ReplacePass, Replaced};
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

    /// Append `src` to the end of `dst`, creating `dst` if it is not there, and leave its
    /// permissions as they were. Returns the bytes appended.
    ///
    /// Unlike [`JobFs::copy_file`] this writes into the destination itself: there is no
    /// temporary file, because the point is the bytes that are already in it.
    fn append_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let _ = (src, dst, buffer, on_bytes);
        Err(unsupported("this filesystem cannot append to a file"))
    }

    /// Point `link` at the same bytes as `target`, as a second name for one file.
    fn hard_link(&self, target: &VPath, link: &VPath) -> io::Result<()> {
        let _ = (target, link);
        Err(unsupported("this filesystem has no hard links"))
    }

    /// Overwrite every byte of a regular file with zeros, flush them to the device, and only
    /// then unlink it. `on_bytes` reports progress as it goes; returning `false` from it
    /// stops the wipe, and what has already been overwritten stays overwritten — there is no
    /// undoing a wipe halfway through, which is the nature of the operation.
    ///
    /// **What this guarantees, exactly.** On a filesystem that rewrites a file's blocks in
    /// place, on a device that rewrites the sectors it is given, the bytes that were in those
    /// blocks are gone. That is the whole of the promise, and it is the reason the default
    /// here is a refusal rather than a best effort:
    ///
    /// - an SSD writes the zeros to different cells and erases the old ones when its
    ///   controller feels like it, if ever — wear levelling is the opposite of overwriting;
    /// - a copy-on-write filesystem (Btrfs, ZFS, APFS) never writes in place by design, and
    ///   any snapshot keeps the old extents alive and readable;
    /// - a journalling filesystem can hold a small file's data in the journal as well;
    /// - a network, archive or cloud provider has no idea what the far end does with blocks,
    ///   and no way to find out.
    ///
    /// The name is not covered either: it stays in the directory's free space, and in the
    /// journal, after the unlink. This is a defence against whoever next reads the disk, not
    /// against a laboratory, and the dialog has to say so.
    fn wipe_file(
        &self,
        path: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<()> {
        let _ = (path, buffer, on_bytes);
        Err(unsupported(
            "a wipe can only overwrite a file on a local filesystem",
        ))
    }

    /// Replace every match of `matcher` in a regular file.
    ///
    /// A [`ReplacePass::Count`] pass writes nothing and reports what a [`ReplacePass::Rewrite`]
    /// would change; a `Rewrite` pass does it. The two are the same walk over the same bytes —
    /// see [`crate::replace`] — because a dry run whose count disagreed with the write it is
    /// shown before would be worse than no dry run at all.
    ///
    /// A rewrite is atomic from the outside: the result goes into a sibling temporary file, is
    /// flushed to the device and only then renamed over the original, so the file is either the
    /// one it was or the one it is meant to become, and never half of each. Whatever goes wrong
    /// in between — a read error, a full disk, a cancellation — leaves the original as it was
    /// and takes the temporary file with it. Unlike a copy there is no second copy of the bytes
    /// to fall back on, which is why this one syncs before the rename rather than only flushing.
    ///
    /// `on_bytes` reports every chunk read, so a job can show where it is inside a large file
    /// and stop there; returning `false` from it abandons the pass and changes nothing.
    ///
    /// The file's metadata is left to the caller, exactly as a copy's is: read [`Facts`] before
    /// the call and hand them to [`JobFs::apply_meta`] after it, so one [`Preserve`] decides
    /// what a rewritten file keeps for a replacement and for a copy alike.
    ///
    /// The default is a refusal, because a provider that cannot write a sibling file and rename
    /// it over another cannot promise any of the above.
    fn replace_in_file(
        &self,
        path: &VPath,
        matcher: &Matcher,
        pass: ReplacePass,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<Replaced> {
        let _ = (path, matcher, pass, buffer, on_bytes);
        Err(unsupported(
            "a replacement can only rewrite a file on a local filesystem",
        ))
    }
}

/// A path inside a container is not this filesystem's, and `as_path` would hand it the
/// archive itself — so a copy would quietly duplicate the whole zip instead of one entry.
/// Every entry into [`LocalFs`] that could make that mistake asks this first; the archive
/// layer ([`crate::ArcFs`]) takes those paths before they get here.
fn not_mine(path: &VPath) -> io::Result<()> {
    if path.in_container() {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{path}: inside an archive, which this filesystem cannot read"),
        ));
    }
    Ok(())
}

/// The one refusal every provider that cannot keep a promise returns, so the engine's error
/// text reads the same wherever it came from.
fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}

/// The local filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

/// Suffix of the temporary file a copy writes into. A destination never appears under its final
/// name until it is complete.
pub const PART_SUFFIX: &str = ".scmd-part";

impl JobFs for LocalFs {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        not_mine(path)?;
        let meta = fs::symlink_metadata(path.as_path())?;
        Ok(facts_from(&meta))
    }

    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        not_mine(path)?;
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
            clear_readonly(path, &meta);
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

    fn append_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let mut reader = File::open(src.as_path())?;
        // `append` seeks to the end on every write, so bytes already in the destination are
        // kept whatever else writes to it meanwhile; `create` rather than `create_new`
        // because appending to a destination that is not there yet is just a copy. Neither
        // touches the permissions of a file that exists, which `File::create` would not do.
        let mut writer = File::options()
            .append(true)
            .create(true)
            .open(dst.as_path())?;
        let mut buf = vec![0u8; buffer.clamp(16 * 1024, 8 * 1024 * 1024)];
        let mut total = 0u64;

        loop {
            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            writer.write_all(&buf[..read])?;
            total += read as u64;
            if !on_bytes(read as u64) {
                // Half an append cannot be taken back the way a half-written copy can: the
                // bytes are in the user's own file. They are flushed and left there.
                writer.flush()?;
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
        }

        writer.flush()?;
        Ok(total)
    }

    fn hard_link(&self, target: &VPath, link: &VPath) -> io::Result<()> {
        fs::hard_link(target.as_path(), link.as_path())
    }

    fn wipe_file(
        &self,
        path: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<()> {
        if !path.is_local() {
            return Err(unsupported(
                "a wipe can only overwrite a file on a local filesystem",
            ));
        }
        let meta = fs::symlink_metadata(path.as_path())?;
        if !meta.is_file() {
            // A symlink, a device or a socket has no bytes of its own; overwriting what one
            // of them points at would destroy something the user never selected.
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only a regular file has bytes of its own to overwrite",
            ));
        }
        clear_readonly(path, &meta);

        let mut file = File::options().write(true).open(path.as_path())?;
        let zeros = vec![0u8; buffer.clamp(16 * 1024, 8 * 1024 * 1024)];
        let mut left = meta.len();
        while left > 0 {
            let chunk = left.min(zeros.len() as u64) as usize;
            file.write_all(&zeros[..chunk])?;
            left -= chunk as u64;
            if !on_bytes(chunk as u64) {
                let _ = file.sync_all();
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
        }

        // The zeros have to be on the device before the name goes: a crash in between would
        // otherwise leave the directory entry removed and the old blocks untouched, which is
        // the one outcome a wipe must not have.
        file.sync_all()?;
        drop(file);
        self.remove_file(path)
    }

    fn replace_in_file(
        &self,
        path: &VPath,
        matcher: &Matcher,
        pass: ReplacePass,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<Replaced> {
        if !path.is_local() {
            return Err(unsupported(
                "a replacement can only rewrite a file on a local filesystem",
            ));
        }
        let meta = fs::symlink_metadata(path.as_path())?;
        if !meta.is_file() {
            // A directory, a symlink, a device or a socket has no bytes of its own. Rewriting
            // what a link points at would replace a file the user never chose.
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only a regular file has bytes of its own to replace in",
            ));
        }
        let mut reader = File::open(path.as_path())?;

        if pass == ReplacePass::Count {
            return replace::pass(&mut reader, None, matcher, buffer, on_bytes);
        }

        // The result is written as it is found, because whether a single byte differs is only
        // known once the last one has been read. A file nothing matched in therefore costs a
        // temporary file that is then removed — one sequential write, against reading every
        // file twice, which is the other way to learn the same thing.
        let part = part_path(path);
        // Buffered, because the result arrives as a long run of bytes that did not change, a
        // short replacement, another long run: a file with a thousand matches is not worth a
        // thousand extra syscalls.
        let mut writer = io::BufWriter::new(File::create(part.as_path())?);
        let report = match replace::pass(&mut reader, Some(&mut writer), matcher, buffer, on_bytes)
        {
            Ok(report) => report,
            Err(e) => {
                drop(writer);
                let _ = fs::remove_file(part.as_path());
                return Err(e);
            }
        };

        if !report.changed {
            // Nothing to do. The original keeps its inode, its permissions and its modification
            // time, rather than being replaced by a file with the same bytes and none of that.
            drop(writer);
            let _ = fs::remove_file(part.as_path());
            return Ok(report);
        }

        // Unlike a copy, this is the only copy: once the rename lands there is no original left
        // to go back to, so the bytes have to be on the device before the name moves.
        if let Err(e) = flush_and_sync(&mut writer) {
            drop(writer);
            let _ = fs::remove_file(part.as_path());
            return Err(e);
        }
        drop(writer);
        // Only now does the file the user is watching change at all.
        if let Err(e) = fs::rename(part.as_path(), path.as_path()) {
            let _ = fs::remove_file(part.as_path());
            return Err(e);
        }
        Ok(Replaced {
            rewritten: true,
            ..report
        })
    }
}

/// Everything a rewritten file has to have on the device before its name is allowed to change:
/// out of the buffer, out of the kernel, onto the disk.
fn flush_and_sync(writer: &mut io::BufWriter<File>) -> io::Result<()> {
    writer.flush()?;
    writer.get_ref().sync_all()
}

/// Clear the read-only bit, best effort. A file the user told us to delete or wipe has to go
/// whatever its permissions say, and a failure here is reported by the write that follows.
fn clear_readonly(path: &VPath, meta: &fs::Metadata) {
    if !meta.permissions().readonly() {
        return;
    }
    let mut perms = meta.permissions();
    #[allow(clippy::permissions_set_readonly_false)]
    perms.set_readonly(false);
    let _ = fs::set_permissions(path.as_path(), perms);
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
    use scmd_core::Scheme;

    /// A provider that implements only the calls the trait requires, so the operations a
    /// filesystem may not be able to do can be checked to refuse rather than guess. Every
    /// required call is a refusal too: none of them is reached.
    struct Barest;

    impl JobFs for Barest {
        fn facts(&self, _path: &VPath) -> io::Result<Facts> {
            Err(unsupported("nothing here"))
        }

        fn facts_following(&self, _path: &VPath) -> io::Result<Facts> {
            Err(unsupported("nothing here"))
        }

        fn exists(&self, _path: &VPath) -> bool {
            false
        }

        fn read_dir(&self, _path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
            Err(unsupported("nothing here"))
        }

        fn create_dir(&self, _path: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn create_dir_all(&self, _path: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn rename(&self, _from: &VPath, _to: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn remove_file(&self, _path: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn remove_dir(&self, _path: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn read_link(&self, _path: &VPath) -> io::Result<PathBuf> {
            Err(unsupported("nothing here"))
        }

        fn symlink(&self, _target: &std::path::Path, _link: &VPath) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn same_file(&self, _a: &VPath, _b: &VPath) -> bool {
            false
        }

        fn apply_meta(&self, _p: &VPath, _f: &Facts, _preserve: Preserve) -> io::Result<()> {
            Err(unsupported("nothing here"))
        }

        fn copy_file(
            &self,
            _src: &VPath,
            _dst: &VPath,
            _buffer: usize,
            _on_bytes: &mut dyn FnMut(u64) -> bool,
        ) -> io::Result<u64> {
            Err(unsupported("nothing here"))
        }
    }

    fn remote() -> VPath {
        VPath::remote(Scheme::Sftp, "host", "/srv/x")
    }

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
    fn a_wipe_overwrites_the_bytes_before_it_unlinks_the_name() {
        let dir = tempfile::tempdir().unwrap();
        let secret = VPath::local(dir.path().join("secret"));
        let witness = VPath::local(dir.path().join("witness"));
        fs::write(secret.as_path(), vec![0xABu8; 70_000]).unwrap();
        // A second name for the same inode outlives the unlink, which is the only way to
        // read back what a wipe left behind.
        LocalFs.hard_link(&secret, &witness).unwrap();

        let mut seen = 0u64;
        LocalFs
            .wipe_file(&secret, 16 * 1024, &mut |n| {
                seen += n;
                true
            })
            .unwrap();

        assert_eq!(seen, 70_000, "progress must cover every byte");
        assert!(!LocalFs.exists(&secret), "the name should be gone");
        let left = fs::read(witness.as_path()).unwrap();
        assert_eq!(left.len(), 70_000);
        assert!(left.iter().all(|b| *b == 0), "a byte survived the wipe");
    }

    #[test]
    fn a_cancelled_wipe_leaves_the_file_and_what_it_already_destroyed() {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("half"));
        fs::write(path.as_path(), vec![0xABu8; 200_000]).unwrap();

        let err = LocalFs
            .wipe_file(&path, 16 * 1024, &mut |_| false)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);

        // There is no undoing an overwrite, so the honest outcome is a file that is still
        // there with its first chunk already gone.
        let left = fs::read(path.as_path()).unwrap();
        assert_eq!(left.len(), 200_000);
        assert!(left[..16 * 1024].iter().all(|b| *b == 0));
        assert!(left[16 * 1024..].iter().all(|b| *b == 0xAB));
    }

    #[test]
    fn a_wipe_refuses_everything_it_cannot_make_a_promise_about() {
        let dir = tempfile::tempdir().unwrap();
        let real = VPath::local(dir.path().join("real"));
        let link = VPath::local(dir.path().join("link"));
        fs::write(real.as_path(), b"keep me").unwrap();
        LocalFs
            .symlink(std::path::Path::new("real"), &link)
            .unwrap();

        let err = LocalFs.wipe_file(&remote(), 0, &mut |_| true).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);

        for path in [&link, &VPath::local(dir.path())] {
            let err = LocalFs.wipe_file(path, 0, &mut |_| true).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{path}");
        }
        assert_eq!(
            fs::read(real.as_path()).unwrap(),
            b"keep me",
            "wiping a link must not touch what it points at"
        );
    }

    #[test]
    fn appending_lands_after_the_bytes_already_there() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("part2"));
        let dst = VPath::local(dir.path().join("whole"));
        fs::write(src.as_path(), b"second").unwrap();
        fs::write(dst.as_path(), b"first ").unwrap();

        let mut seen = 0u64;
        let written = LocalFs
            .append_file(&src, &dst, 16 * 1024, &mut |n| {
                seen += n;
                true
            })
            .unwrap();
        assert_eq!((written, seen), (6, 6));
        assert_eq!(fs::read(dst.as_path()).unwrap(), b"first second");

        // A destination that is not there yet is simply a copy.
        let fresh = VPath::local(dir.path().join("fresh"));
        LocalFs
            .append_file(&src, &fresh, 16 * 1024, &mut |_| true)
            .unwrap();
        assert_eq!(fs::read(fresh.as_path()).unwrap(), b"second");
    }

    #[test]
    fn a_provider_that_implements_none_of_them_refuses_instead_of_trying() {
        let path = remote();
        let matcher =
            Matcher::compile(&crate::replace::ReplaceOptions::new("alpha", "omega")).unwrap();
        for kind in [
            Barest
                .append_file(&path, &path, 0, &mut |_| true)
                .unwrap_err()
                .kind(),
            Barest.hard_link(&path, &path).unwrap_err().kind(),
            Barest
                .wipe_file(&path, 0, &mut |_| true)
                .unwrap_err()
                .kind(),
            Barest
                .replace_in_file(&path, &matcher, ReplacePass::Count, 0, &mut |_| true)
                .unwrap_err()
                .kind(),
        ] {
            assert_eq!(kind, io::ErrorKind::Unsupported);
        }
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
