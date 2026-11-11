use std::ffi::OsString;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::time::SystemTime;

use fcmd_core::{EntryKind, VPath};

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

    /// Give `dst` the extended attributes `src` carries: the tags, labels and marks a desktop
    /// keeps beside a file's bytes, which a copy of the bytes alone leaves behind.
    ///
    /// A courtesy like [`JobFs::apply_meta`], and asked for separately only because it needs
    /// both ends where that needs the destination and the source's [`Facts`]. An error says
    /// that something could not be carried — a destination with nowhere to keep attributes
    /// is the usual one — and the engine does not lose the copy over it.
    ///
    /// The default carries nothing and says so without complaint: a server, a bucket and an
    /// archive have no such thing, and that is not a failure of the copy.
    fn copy_xattrs(&self, src: &VPath, dst: &VPath) -> io::Result<()> {
        let _ = (src, dst);
        Ok(())
    }

    /// Change what the attributes dialog asked to change.
    ///
    /// This is [`JobFs::apply_meta`]'s other half, and the difference is who asked. A copy
    /// applies the source's metadata as a courtesy and swallows a failure rather than losing
    /// the copy over it. Here somebody ticked a box and pressed Enter, so a permission that
    /// could not be set is returned and shown — a dialog that reported success it did not have
    /// would be worse than one that cannot do it at all.
    ///
    /// The default is that refusal: a provider that has nothing to set says so.
    fn set_attrs(&self, path: &VPath, change: &crate::attrs::Change) -> io::Result<()> {
        let _ = change;
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{path}: this filesystem cannot change attributes"),
        ))
    }

    /// Hand the path to the system's own wastebasket, with everything under it.
    ///
    /// It is one operation on the whole tree, not a walk: a wastebasket is told what went
    /// into it and where it came from, so a directory arrives in it as a directory and comes
    /// back as one. That is also why this cannot be built out of `rename` and `remove_file` —
    /// the record of where it came from is the part that makes it a wastebasket.
    ///
    /// The default is a refusal. A server, an archive and a bucket have no wastebasket of
    /// their own, and inventing one — a hidden directory we made up — would be a promise the
    /// rest of the system knows nothing about.
    fn trash(&self, path: &VPath) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{path}: there is no wastebasket here"),
        ))
    }

    /// The files a job is about to ask [`JobFs::copy_file`] for, said once before the first
    /// of them.
    ///
    /// A filesystem that pays per request rather than per byte can use it to do the whole
    /// set at one go: an archive read through a program on the machine starts that program
    /// once for everything, where a file at a time would start it once per file and, in a
    /// solid archive, unpack everything in front of each one again. It promises nothing —
    /// a file named here may never be asked for (skipped, cancelled), and one that was not
    /// named still may be. The default has nothing to prepare.
    fn foresee(&self, files: &mut dyn Iterator<Item = &VPath>) {
        let _ = files;
    }

    /// Copy `src` to `dst`, calling `on_bytes` as it goes. Returning `false` from `on_bytes`
    /// cancels the copy, and the partial file is removed.
    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        how: CopyHow,
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

    /// Read the file in chunks, handing each one to `on_chunk`.
    ///
    /// It is the reading half of a copy with the writing half left out, which is what a
    /// checksum needs: the bytes go through a hash and nowhere else. Returning `false` from
    /// `on_chunk` abandons the read, and the caller gets `Interrupted`.
    ///
    /// The default is a refusal. Every provider that can copy a file out of itself could
    /// implement this, and when one does, checksums work there with nothing else changed —
    /// but a default that guessed would hash the archive instead of the entry inside it.
    fn read_chunks(
        &self,
        path: &VPath,
        buffer: usize,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> io::Result<u64> {
        let _ = (buffer, on_chunk);
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            format!("{path}: this filesystem cannot be read a chunk at a time"),
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
/// Is this a directory? Asked of the path rather than of a handle, because the one caller
/// needs to know *before* it opens anything.
fn path_is_dir(path: &std::path::Path) -> bool {
    fs::metadata(path).map(|m| m.is_dir()).unwrap_or(false)
}

/// Write the attribute word back. The standard library reads `file_attributes` and has no way
/// to set it, so this is the one place a Windows API call is made directly.
#[cfg(windows)]
fn set_windows_attributes(path: &std::path::Path, attributes: u32) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    unsafe extern "system" {
        fn SetFileAttributesW(name: *const u16, attributes: u32) -> i32;
    }
    let wide: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: `wide` is a NUL-terminated UTF-16 string that outlives the call, which is the
    // whole of what this function requires of its caller.
    let ok = unsafe { SetFileAttributesW(wide.as_ptr(), attributes) };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn unsupported(message: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, message)
}

/// The local filesystem.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalFs;

/// How to copy one file: how much at a time, and whether the system's own routine may do it.
///
/// It travels with the copy rather than with the filesystem because it is the *job's*
/// question — a user preference and a buffer size the dialog chose — and because a copy that
/// crosses between a provider and the local disk has to hand the same answer to whichever
/// half ends up reading the bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyHow {
    /// How many bytes to move before reporting progress and asking whether to carry on.
    pub buffer: usize,
    /// May the kernel make the copy (`copy_file_range`, `fcopyfile`, `CopyFileExW`)? Off by
    /// default, and only ever true for a local file: a provider moves its own bytes.
    pub system: bool,
    /// May a copy that was interrupted go on from where it stopped? A `.fcmd-part` file left
    /// at the destination by a crash or a failed copy is continued rather than written
    /// again, when its last bytes are the source's at the same place (SPEC 03 §3c).
    pub resume: bool,
}

impl CopyHow {
    /// The ordinary copy: a buffer this big, read and written by this program.
    pub fn buffered(buffer: usize) -> CopyHow {
        CopyHow {
            buffer,
            system: false,
            resume: false,
        }
    }
}

/// Is this extended attribute one a copy takes with it?
///
/// On Linux only the `user.` namespace, which is where a desktop writes (`user.xdg.tags`,
/// `user.xdg.origin.url`, `user.baloo.rating`) and the only one that is the user's to write.
/// `security.` holds SELinux labels and file capabilities, `system.` the ACLs, `trusted.`
/// whatever root keeps there: each is a statement about what a file is *allowed*, made by
/// the system where the file was, and a file manager that carried them to another place
/// would be granting things nobody granted — which is why `cp` leaves them behind too
/// unless it is told `--preserve=all`.
///
/// On macOS there are no namespaces and everything goes, which is what the Finder does: the
/// tags and the label, the *where from*, the resource fork, and the quarantine mark with
/// them. That last one is deliberate. A file that came from the internet is still a file
/// that came from the internet after it has been copied, and a file manager that quietly
/// washed the mark off would be a way round Gatekeeper.
fn carries_xattr(name: &std::ffi::OsStr) -> bool {
    !cfg!(target_os = "linux") || name.as_encoded_bytes().starts_with(b"user.")
}

/// The path itself, or what it points at when it is a link. A link that leads nowhere stays
/// as it is, and whoever uses it finds that out.
fn behind_link(path: &std::path::Path) -> PathBuf {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_symlink() => {
            fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
        }
        _ => path.to_path_buf(),
    }
}

/// Suffix of the temporary file a copy writes into. A destination never appears under its final
/// name until it is complete.
pub const PART_SUFFIX: &str = ".fcmd-part";

impl JobFs for LocalFs {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        not_mine(path)?;
        let meta = fs::symlink_metadata(path.os())?;
        Ok(facts_from(&meta))
    }

    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        not_mine(path)?;
        let meta = fs::metadata(path.os())?;
        Ok(facts_from(&meta))
    }

    fn exists(&self, path: &VPath) -> bool {
        fs::symlink_metadata(path.os()).is_ok()
    }

    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
        let mut out = Vec::new();
        for entry in fs::read_dir(path.os())? {
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
        match fs::create_dir(path.os()) {
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Ok(()),
            other => other,
        }
    }

    fn create_dir_all(&self, path: &VPath) -> io::Result<()> {
        fs::create_dir_all(path.os())
    }

    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()> {
        fs::rename(from.os(), to.os())
    }

    fn remove_file(&self, path: &VPath) -> io::Result<()> {
        // A read-only file still has to go when the user says delete.
        if let Ok(meta) = fs::symlink_metadata(path.os()) {
            clear_readonly(path, &meta);
        }
        fs::remove_file(path.os())
    }

    fn remove_dir(&self, path: &VPath) -> io::Result<()> {
        fs::remove_dir(path.os())
    }

    fn trash(&self, path: &VPath) -> io::Result<()> {
        // `trash` is the platform half: the XDG wastebasket on Linux (the file moves and a
        // `.trashinfo` beside it records where it was), `NSFileManager` on macOS and
        // `IFileOperation` on Windows, each of which is what that desktop's own file manager
        // shows and restores from.
        // Windows' shell takes `\` only: a path that reached here with `/` in it (git names
        // its files that way) is said the way the shell wants it.
        #[cfg(windows)]
        let os = PathBuf::from(path.os().to_string_lossy().replace('/', "\\"));
        #[cfg(not(windows))]
        let os = path.os().to_path_buf();
        ::trash::delete(&os).map_err(|e| match e {
            // The wastebasket cannot take it: on Linux a path on a device with no trash
            // directory, on any system one the desktop refuses. It is not a bug and it is
            // not a missing file — the caller offers to delete it outright instead.
            ::trash::Error::Unknown { description } => {
                io::Error::new(io::ErrorKind::Unsupported, description)
            }
            other => io::Error::other(other.to_string()),
        })
    }

    fn read_link(&self, path: &VPath) -> io::Result<PathBuf> {
        fs::read_link(path.os())
    }

    #[cfg(unix)]
    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()> {
        std::os::unix::fs::symlink(target, link.os())
    }

    #[cfg(windows)]
    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()> {
        // Without developer mode or elevation this fails; the engine turns that into a normal
        // error decision point rather than a crash.
        if target.is_dir() {
            std::os::windows::fs::symlink_dir(target, link.os())
        } else {
            std::os::windows::fs::symlink_file(target, link.os())
        }
    }

    fn same_file(&self, a: &VPath, b: &VPath) -> bool {
        match (fs::canonicalize(a.os()), fs::canonicalize(b.os())) {
            (Ok(x), Ok(y)) => x == y,
            _ => false,
        }
    }

    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()> {
        if preserve.mtime {
            if let Some(mtime) = facts.mtime {
                // Opening for write would truncate nothing but does need write access; a
                // failure here is not worth losing the copy over.
                if let Ok(file) = File::options().write(true).open(path.os()) {
                    let _ = file.set_modified(mtime);
                }
            }
        }
        #[cfg(unix)]
        if preserve.mode {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = facts.mode {
                let _ = fs::set_permissions(path.os(), fs::Permissions::from_mode(mode));
            }
        }
        Ok(())
    }

    /// Every attribute worth carrying is tried, whatever happened to the one before, and the
    /// first refusal is what comes back — except a destination that keeps no attributes at
    /// all, which is asked once and not once per name.
    fn copy_xattrs(&self, src: &VPath, dst: &VPath) -> io::Result<()> {
        not_mine(src)?;
        not_mine(dst)?;
        // The data was read through a link when the source is one (`follow_symlinks`), and
        // written through one when the destination already was; the attributes go the same
        // way, since the calls underneath only ever speak about the path itself.
        let (src, dst) = (behind_link(&src.os()), behind_link(&dst.os()));
        let mut refused = None;
        for name in fcmd_platform::xattr::list(&src)? {
            if !carries_xattr(&name) {
                continue;
            }
            let carried = fcmd_platform::xattr::get(&src, &name)
                .and_then(|value| fcmd_platform::xattr::set(&dst, &name, &value));
            if let Err(e) = carried {
                let nowhere = e.kind() == io::ErrorKind::Unsupported;
                refused.get_or_insert(e);
                if nowhere {
                    break;
                }
            }
        }
        refused.map_or(Ok(()), Err)
    }

    fn set_attrs(&self, path: &VPath, change: &crate::attrs::Change) -> io::Result<()> {
        not_mine(path)?;
        let target = path.os();
        // Whose it is, before the mode: a `chown` that succeeds takes the setuid and setgid
        // bits off the file, so doing it the other way round would quietly undo an answer the
        // user had just given in the twelve boxes.
        if change.touches_owner() {
            fcmd_platform::set_owner(target.as_ref(), change.uid, change.gid)?;
        }
        // A directory's own times cannot be set on Windows through the standard library: it
        // has no way to open a directory as a handle (that needs `FILE_FLAG_BACKUP_SEMANTICS`
        // on `CreateFileW`), so there is nothing to call `set_times` on. The files inside it
        // are set as asked; the folder itself keeps its own times, and the help page says so.
        let times_here = !cfg!(windows) || !path_is_dir(target.as_ref());
        if let Some(when) = change.mtime.filter(|_| times_here) {
            // The times go through a handle rather than through the path, which is the only
            // way the standard library offers and also the right one: a path that changed
            // underneath us would otherwise have its successor's times set.
            let file = File::options().write(true).open(&target).or_else(|e| {
                // A read-only file still has its times set by whoever may write the
                // directory, and a directory cannot be opened for writing at all on Unix.
                if path_is_dir(target.as_ref()) {
                    File::open(&target)
                } else {
                    Err(e)
                }
            })?;
            let mut times = fs::FileTimes::new().set_modified(when);
            if let Some(atime) = change.atime {
                times = times.set_accessed(atime);
            }
            times = with_created(times, change.btime);
            file.set_times(times)?;
        } else if let Some(atime) = change.atime.filter(|_| times_here) {
            let file = File::open(&target)?;
            let times = with_created(fs::FileTimes::new().set_accessed(atime), change.btime);
            file.set_times(times)?;
        } else if change.btime.is_some() && times_here {
            let file = File::options().write(true).open(&target)?;
            file.set_times(with_created(fs::FileTimes::new(), change.btime))?;
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let current =
                || -> io::Result<u32> { Ok(fs::metadata(&target)?.permissions().mode() & 0o7777) };
            // The file's own mode with the dialog's two masks over it: a bit nobody answered
            // keeps the value this file has, which is what makes one dialog over a hundred
            // files with different permissions safe to press `OK` on.
            let mut mode = if change.touches_mode() || change.readonly.is_some() {
                Some(change.mode_over(current()?))
            } else {
                None
            };
            // Read-only is applied after the mode, so a dialog that offers both ends with
            // whichever answer the user gave last. On Unix it is the write bits and nothing
            // else: the read and execute bits say who may read, and that is a different
            // question from whether the file may be changed.
            if let (Some(bits), Some(readonly)) = (mode.as_mut(), change.readonly) {
                if readonly {
                    *bits &= !0o222;
                } else {
                    // The owner's write bit, and only that. Clearing the write bits throws
                    // away who had them, so putting them back is a guess either way — and the
                    // guess that hands the world write access to a world-readable file is the
                    // wrong one to make on a user's behalf. It is also what `chmod +w` does
                    // under the usual umask. Somebody who wants the group's bit back has the
                    // twelve boxes above for it.
                    *bits |= 0o200;
                }
            }
            if let Some(mode) = mode {
                fs::set_permissions(&target, fs::Permissions::from_mode(mode))?;
            }
        }

        #[cfg(windows)]
        {
            // The four bits Windows has. `mode` has no meaning here and is ignored rather
            // than approximated: a dialog on Windows does not offer it.
            use std::os::windows::fs::MetadataExt;
            const READONLY: u32 = 0x1;
            const HIDDEN: u32 = 0x2;
            const SYSTEM: u32 = 0x4;
            const ARCHIVE: u32 = 0x20;
            const TEMPORARY: u32 = 0x100;
            const OFFLINE: u32 = 0x1000;
            const NOT_INDEXED: u32 = 0x2000;
            let mut attributes = fs::metadata(&target)?.file_attributes();
            for (wanted, bit) in [
                (change.readonly, READONLY),
                (change.hidden, HIDDEN),
                (change.system, SYSTEM),
                (change.archive, ARCHIVE),
                (change.temporary, TEMPORARY),
                (change.offline, OFFLINE),
                // The box is asked the other way round — "the indexer may look at it" — so
                // the bit it sets is the opposite of the answer.
                (change.indexed.map(|on| !on), NOT_INDEXED),
            ] {
                match wanted {
                    Some(true) => attributes |= bit,
                    Some(false) => attributes &= !bit,
                    None => {}
                }
            }
            set_windows_attributes(target.as_ref(), attributes)?;
        }

        Ok(())
    }

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        how: CopyHow,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let part = part_path(dst);
        // The system's own routine, when `general.system_copy` asks for it (SPEC 03 §3b). It writes the
        // same temporary file this function would, so everything after it — the permissions,
        // the times, the rename into place — is unchanged; and when the platform declines it
        // answers `None` and the loop below does the work, which is why there is no second
        // code path for "it did not work".
        // A part to go on from is this loop's job: the kernel's routine copies whole files.
        if how.system && !(how.resume && fs::metadata(part.os()).is_ok()) {
            let (from, to) = (src.os(), part.os());
            match fcmd_platform::system_copy(&from, &to, how.buffer, on_bytes) {
                Ok(Some(total)) => {
                    if let Err(e) = fs::rename(part.os(), dst.os()) {
                        let _ = fs::remove_file(part.os());
                        return Err(e);
                    }
                    return Ok(total);
                }
                Ok(None) => {}
                Err(e) => {
                    let _ = fs::remove_file(part.os());
                    return Err(e);
                }
            }
        }
        let mut reader = File::open(src.os())?;
        let mut buf = vec![0u8; how.buffer.clamp(16 * 1024, 8 * 1024 * 1024)];
        // What an earlier, interrupted copy already wrote, when it can be trusted to be the
        // start of this same source: then the copy goes on from there.
        let resumed = if how.resume {
            resumable(&mut reader, &part)?
        } else {
            0
        };
        let mut writer = if resumed > 0 {
            File::options().append(true).open(part.os())?
        } else {
            File::create(part.os())?
        };
        let mut total = resumed;
        if resumed > 0 && !on_bytes(resumed) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        // A failure that is not the user's `Esc` leaves the part where it is, so the next
        // attempt goes on from it; with resuming off it is removed as it always was.
        let give_up = |part: &VPath| {
            if !how.resume {
                let _ = fs::remove_file(part.os());
            }
        };

        loop {
            let read = match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    drop(writer);
                    give_up(&part);
                    return Err(e);
                }
            };
            if let Err(e) = writer.write_all(&buf[..read]) {
                drop(writer);
                give_up(&part);
                return Err(e);
            }
            total += read as u64;
            if !on_bytes(read as u64) {
                drop(writer);
                let _ = fs::remove_file(part.os());
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
        }

        writer.flush()?;
        drop(writer);
        // Only now does the destination appear under its final name.
        if let Err(e) = fs::rename(part.os(), dst.os()) {
            let _ = fs::remove_file(part.os());
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
        let mut reader = File::open(src.os())?;
        // `append` seeks to the end on every write, so bytes already in the destination are
        // kept whatever else writes to it meanwhile; `create` rather than `create_new`
        // because appending to a destination that is not there yet is just a copy. Neither
        // touches the permissions of a file that exists, which `File::create` would not do.
        let mut writer = File::options().append(true).create(true).open(dst.os())?;
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
        fs::hard_link(target.os(), link.os())
    }

    fn read_chunks(
        &self,
        path: &VPath,
        buffer: usize,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> io::Result<u64> {
        not_mine(path)?;
        let mut file = File::open(path.os())?;
        let mut buf = vec![0u8; buffer.clamp(8 * 1024, 8 * 1024 * 1024)];
        let mut total = 0u64;
        loop {
            let read = match file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            };
            total += read as u64;
            if !on_chunk(&buf[..read]) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
        }
        Ok(total)
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
        let meta = fs::symlink_metadata(path.os())?;
        if !meta.is_file() {
            // A symlink, a device or a socket has no bytes of its own; overwriting what one
            // of them points at would destroy something the user never selected.
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only a regular file has bytes of its own to overwrite",
            ));
        }
        clear_readonly(path, &meta);

        let mut file = File::options().write(true).open(path.os())?;
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
        let meta = fs::symlink_metadata(path.os())?;
        if !meta.is_file() {
            // A directory, a symlink, a device or a socket has no bytes of its own. Rewriting
            // what a link points at would replace a file the user never chose.
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "only a regular file has bytes of its own to replace in",
            ));
        }
        let mut reader = File::open(path.os())?;

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
        let mut writer = io::BufWriter::new(File::create(part.os())?);
        let report = match replace::pass(&mut reader, Some(&mut writer), matcher, buffer, on_bytes)
        {
            Ok(report) => report,
            Err(e) => {
                drop(writer);
                let _ = fs::remove_file(part.os());
                return Err(e);
            }
        };

        if !report.changed {
            // Nothing to do. The original keeps its inode, its permissions and its modification
            // time, rather than being replaced by a file with the same bytes and none of that.
            drop(writer);
            let _ = fs::remove_file(part.os());
            return Ok(report);
        }

        // Unlike a copy, this is the only copy: once the rename lands there is no original left
        // to go back to, so the bytes have to be on the device before the name moves.
        if let Err(e) = flush_and_sync(&mut writer) {
            drop(writer);
            let _ = fs::remove_file(part.os());
            return Err(e);
        }
        drop(writer);
        // Only now does the file the user is watching change at all.
        if let Err(e) = fs::rename(part.os(), path.os()) {
            let _ = fs::remove_file(part.os());
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

/// How many bytes of `part` — what an interrupted copy left — this copy may keep, with
/// `reader` positioned just after them. Zero means start again.
///
/// Kept only when the part is no longer than the source and its last 64 KiB (or all of it,
/// when shorter) are the source's bytes at the same offset: a part left by a different file
/// of the same name, or by an older version of this one, fails that and is written over. It
/// is a check on the end and not on every byte, which is what makes resuming cheap; the
/// copy's own verify (`JobOptions::verify`) reads the whole file when that is wanted.
fn resumable(reader: &mut File, part: &VPath) -> io::Result<u64> {
    use std::io::{Seek, SeekFrom};
    const TAIL: u64 = 64 * 1024;
    let Ok(meta) = fs::metadata(part.os()) else {
        return Ok(0);
    };
    let have = meta.len();
    let source = reader.metadata()?.len();
    if have == 0 || have > source {
        return Ok(0);
    }
    let from = have.saturating_sub(TAIL);
    let len = (have - from) as usize;
    let mut ours = vec![0u8; len];
    let mut theirs = vec![0u8; len];
    let mut written = File::open(part.os())?;
    written.seek(SeekFrom::Start(from))?;
    written.read_exact(&mut ours)?;
    reader.seek(SeekFrom::Start(from))?;
    reader.read_exact(&mut theirs)?;
    if ours != theirs {
        reader.seek(SeekFrom::Start(0))?;
        return Ok(0);
    }
    // `reader` is now at `have`, exactly where the copy goes on.
    Ok(have)
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
    let _ = fs::set_permissions(path.os(), perms);
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

/// Write `bytes` over a file that is already there, without a moment where it is half a file.
///
/// The new contents go beside the original under [`PART_SUFFIX`] and are renamed over it, so
/// an interruption — a full disk, a power cut, a `kill` — leaves the original whole rather
/// than truncated. The original's mode is carried across, because a rename does not carry one
/// and a script that was executable has to stay executable.
///
/// The one place a new file is written: a caller with nothing to replace has
/// `std::fs::write`, which is simpler and right for that.
pub fn replace_file(file: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    let part = match file.file_name() {
        Some(name) => {
            let mut name = name.to_os_string();
            name.push(PART_SUFFIX);
            file.with_file_name(name)
        }
        None => return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a file")),
    };
    fs::write(&part, bytes)?;
    #[cfg(unix)]
    if let Ok(meta) = fs::metadata(file) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&part, fs::Permissions::from_mode(meta.permissions().mode()));
    }
    match fs::rename(&part, file) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = fs::remove_file(&part);
            Err(e)
        }
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

/// The creation time, where the platform has a call for it. Windows does; nothing else does,
/// so there the row is shown in the dialog and never written.
fn with_created(times: fs::FileTimes, btime: Option<SystemTime>) -> fs::FileTimes {
    #[cfg(windows)]
    {
        use std::os::windows::fs::FileTimesExt;
        if let Some(btime) = btime {
            return times.set_created(btime);
        }
    }
    let _ = btime;
    times
}

/// Put back from the system's wastebasket what was sent there from these paths: the most
/// recent item for each, and only where nothing is in its place now (SPEC 03 §14, 0.39.0).
///
/// Returns how many came back. The freedesktop trash and the Windows recycle bin can be
/// listed and restored from; the macOS one cannot be from outside the Finder, so there it is
/// an error that says so rather than a quiet nothing.
pub fn restore_from_trash(paths: &[std::path::PathBuf]) -> Result<usize, String> {
    #[cfg(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    ))]
    {
        let items = ::trash::os_limited::list().map_err(|e| e.to_string())?;
        let mut chosen: Vec<::trash::TrashItem> = Vec::new();
        for path in paths {
            if path.exists() {
                continue;
            }
            // The same name may have been thrown away more than once: the last one is the
            // one this program put there.
            if let Some(item) = items
                .iter()
                .filter(|i| i.original_path() == *path)
                .max_by_key(|i| i.time_deleted)
            {
                chosen.push(item.clone());
            }
        }
        let n = chosen.len();
        if n > 0 {
            ::trash::os_limited::restore_all(chosen).map_err(|e| e.to_string())?;
        }
        Ok(n)
    }
    #[cfg(not(any(
        target_os = "windows",
        all(
            unix,
            not(target_os = "macos"),
            not(target_os = "ios"),
            not(target_os = "android")
        )
    )))]
    {
        let _ = paths;
        Err("this system's trash can only be emptied back from its own file manager".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::Scheme;

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn the_attributes_of_a_file_are_carried_one_by_one_and_a_refusal_is_reported() {
        use fcmd_platform::xattr;
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("src.txt"), dir.path().join("dst.txt"));
        fs::write(&src, b"words").unwrap();
        fs::write(&dst, b"words").unwrap();
        if let Err(e) = xattr::set(&src, "user.xdg.tags".as_ref(), b"one,two") {
            eprintln!("skipped: no user attributes under {:?} ({e})", dir.path());
            return;
        }
        xattr::set(&src, "user.fcmd.empty".as_ref(), b"").unwrap();
        // The destination had one of its own and one of the same name: the first stays, the
        // second becomes the source's.
        xattr::set(&dst, "user.fcmd.own".as_ref(), b"mine").unwrap();
        xattr::set(&dst, "user.xdg.tags".as_ref(), b"old").unwrap();

        // What comes back is not asserted on here or below: macOS may have put an
        // attribute of its own on a file it watched being made, and will not let anybody
        // write that one. What the test put there is what has to arrive.
        let _ = LocalFs.copy_xattrs(&VPath::local(&src), &VPath::local(&dst));
        assert_eq!(
            xattr::get(&dst, "user.xdg.tags".as_ref()).unwrap(),
            b"one,two"
        );
        assert_eq!(xattr::get(&dst, "user.fcmd.empty".as_ref()).unwrap(), b"");
        assert_eq!(xattr::get(&dst, "user.fcmd.own".as_ref()).unwrap(), b"mine");

        // Through a link at either end, since that is how the bytes went.
        let (link, other) = (dir.path().join("link"), dir.path().join("other.txt"));
        std::os::unix::fs::symlink(&src, &link).unwrap();
        fs::write(&other, b"").unwrap();
        let _ = LocalFs.copy_xattrs(&VPath::local(&link), &VPath::local(&other));
        assert_eq!(
            xattr::get(&other, "user.xdg.tags".as_ref()).unwrap(),
            b"one,two"
        );

        // Nowhere to put them is an error for whoever wants to know, and a source with
        // nothing to carry is not — which only Linux can promise of a file just made.
        let gone = dir.path().join("gone.txt");
        assert!(LocalFs
            .copy_xattrs(&VPath::local(&src), &VPath::local(&gone))
            .is_err());
        let plain = dir.path().join("plain.txt");
        fs::write(&plain, b"").unwrap();
        let nothing = LocalFs.copy_xattrs(&VPath::local(&plain), &VPath::local(&gone));
        if cfg!(target_os = "linux") {
            nothing.expect("nothing to carry, so nothing to refuse");
        }
    }

    #[test]
    fn only_the_users_own_namespace_is_carried_on_linux_and_everything_elsewhere() {
        let carried = |name: &str| carries_xattr(std::ffi::OsStr::new(name));
        assert!(carried("user.xdg.tags"));
        assert!(carried("user.baloo.rating"));
        // What a system says a file may do stays where the system said it.
        for name in [
            "security.selinux",
            "security.capability",
            "system.posix_acl_access",
            "trusted.overlay.opaque",
            "username",
        ] {
            assert_eq!(carried(name), !cfg!(target_os = "linux"), "{name}");
        }
        // The Finder's own, quarantine included: copied on macOS as the Finder copies them.
        for name in [
            "com.apple.metadata:_kMDItemUserTags",
            "com.apple.quarantine",
        ] {
            assert_eq!(carried(name), !cfg!(target_os = "linux"), "{name}");
        }
    }

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
            _how: CopyHow,
            _on_bytes: &mut dyn FnMut(u64) -> bool,
        ) -> io::Result<u64> {
            Err(unsupported("nothing here"))
        }
    }

    fn remote() -> VPath {
        VPath::remote(Scheme::Sftp, "host", "/srv/x")
    }

    #[test]
    fn a_filesystem_with_no_wastebasket_says_so_rather_than_inventing_one() {
        let err = Barest
            .trash(&remote())
            .expect_err("there is no wastebasket on a server");
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);
    }

    /// Needs a desktop wastebasket, which a build machine does not have: on Linux the file has
    /// to be on a filesystem with a trash directory, and `/tmp` on a container usually is not.
    /// Run it by hand on a real machine — `cargo test -p fcmd-jobs -- --ignored` — and then
    /// look in the wastebasket.
    #[test]
    #[ignore = "needs the desktop's own wastebasket"]
    fn the_local_wastebasket_takes_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rubbish.txt");
        fs::write(&file, b"out").unwrap();
        LocalFs
            .trash(&VPath::local(&file))
            .expect("the wastebasket took it");
        assert!(!file.exists(), "it is no longer where it was");
    }

    fn resuming(buffer: usize) -> CopyHow {
        CopyHow {
            buffer,
            system: false,
            resume: true,
        }
    }

    /// A part an interrupted copy left, whose end is the source's at the same place, is gone
    /// on from: the bytes before it are not read again, and the file that arrives is whole.
    #[test]
    fn an_interrupted_copy_goes_on_from_the_part_it_left() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("big.bin"));
        let dst = VPath::local(dir.path().join("copy.bin"));
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        fs::write(src.as_path(), &body).unwrap();
        fs::write(part_path(&dst).as_path(), &body[..100_000]).unwrap();

        let mut reports = Vec::new();
        let total = LocalFs
            .copy_file(&src, &dst, resuming(16 * 1024), &mut |n| {
                reports.push(n);
                true
            })
            .unwrap();
        assert_eq!(total, 300_000);
        assert_eq!(
            reports[0], 100_000,
            "what was already there is reported at once"
        );
        assert_eq!(reports.iter().sum::<u64>(), 300_000);
        assert_eq!(fs::read(dst.as_path()).unwrap(), body);
        assert!(!LocalFs.exists(&part_path(&dst)));
    }

    /// A part that is not this source's beginning — another file of the same name, an older
    /// version — and one longer than the source are written over, not continued.
    #[test]
    fn a_part_that_is_not_this_files_start_is_written_again() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("a.bin"));
        let dst = VPath::local(dir.path().join("b.bin"));
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 249) as u8).collect();
        fs::write(src.as_path(), &body).unwrap();
        for stale in [vec![9u8; 50_000], vec![1u8; 250_000]] {
            fs::write(part_path(&dst).as_path(), &stale).unwrap();
            let total = LocalFs
                .copy_file(&src, &dst, resuming(16 * 1024), &mut |_| true)
                .unwrap();
            assert_eq!(total, 200_000);
            assert_eq!(fs::read(dst.as_path()).unwrap(), body);
            fs::remove_file(dst.as_path()).unwrap();
        }
    }

    /// `Esc` is the user's own stop, so the part goes even with resuming on.
    #[test]
    fn a_cancel_still_takes_the_part_away() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("a.bin"));
        let dst = VPath::local(dir.path().join("b.bin"));
        fs::write(src.as_path(), vec![3u8; 200_000]).unwrap();
        let err = LocalFs
            .copy_file(&src, &dst, resuming(16 * 1024), &mut |_| false)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(!LocalFs.exists(&part_path(&dst)));
    }

    #[test]
    fn copy_reports_progress_and_never_leaves_a_partial_destination() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        fs::write(src.as_path(), vec![7u8; 100_000]).unwrap();

        let mut seen = 0u64;
        let copied = LocalFs
            .copy_file(&src, &dst, CopyHow::buffered(16 * 1024), &mut |n| {
                seen += n;
                true
            })
            .unwrap();
        assert_eq!(copied, 100_000);
        assert_eq!(seen, 100_000);
        assert_eq!(fs::read(dst.as_path()).unwrap().len(), 100_000);
        assert!(
            !LocalFs.exists(&part_path(&dst)),
            "the .fcmd-part file stayed behind"
        );
    }

    /// The same copy with the kernel doing it: the file that arrives, the progress that was
    /// reported and the temporary file that is gone are the same three answers, which is the
    /// whole promise of the setting. Where a platform has no such routine the ordinary loop
    /// runs instead and the assertions hold anyway, which is why there is nothing here about
    /// *how* it was copied.
    #[test]
    fn the_system_copy_leaves_the_same_file_behind() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        let body: Vec<u8> = (0..300_000u32).map(|i| (i % 253) as u8).collect();
        fs::write(src.as_path(), &body).unwrap();

        let mut seen = 0u64;
        let how = CopyHow {
            buffer: 32 * 1024,
            system: true,
            resume: false,
        };
        let copied = LocalFs
            .copy_file(&src, &dst, how, &mut |n| {
                seen += n;
                true
            })
            .unwrap();
        assert_eq!(copied, body.len() as u64);
        assert_eq!(seen, copied, "the progress adds up to the file");
        assert_eq!(fs::read(dst.as_path()).unwrap(), body);
        assert!(!LocalFs.exists(&part_path(&dst)));
    }

    /// And it is still a copy that can be refused: whichever routine moved the bytes, a
    /// `false` from the progress callback leaves no destination and no temporary file. The
    /// one difference the platforms have is *when* the answer is given, and that is the
    /// setting's documentation rather than a thing to assert here.
    #[test]
    fn a_cancelled_system_copy_leaves_nothing_either() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        fs::write(src.as_path(), vec![3u8; 400_000]).unwrap();

        let how = CopyHow {
            buffer: 4096,
            system: true,
            resume: false,
        };
        match LocalFs.copy_file(&src, &dst, how, &mut |_| false) {
            Err(e) => {
                assert_eq!(e.kind(), io::ErrorKind::Interrupted);
                assert!(!LocalFs.exists(&dst), "a cancelled copy left a destination");
                assert!(!LocalFs.exists(&part_path(&dst)));
            }
            // A platform that copies the whole file in one call cannot be stopped inside it,
            // and the file is then there: that is the bargain, not a failure.
            Ok(n) => assert_eq!(n, 400_000),
        }
    }

    #[test]
    fn a_cancelled_copy_removes_the_partial_file() {
        let dir = tempfile::tempdir().unwrap();
        let src = VPath::local(dir.path().join("src.bin"));
        let dst = VPath::local(dir.path().join("dst.bin"));
        fs::write(src.as_path(), vec![0u8; 200_000]).unwrap();

        let err = LocalFs
            .copy_file(&src, &dst, CopyHow::buffered(16 * 1024), &mut |_| false)
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
            .copy_file(&src, &dst, CopyHow::buffered(64 * 1024), &mut |_| true)
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
    #[test]
    fn a_path_past_the_windows_limit_is_still_a_file_we_can_work_with() {
        // This is the test for `fcmd_core::longpath`, and it is here rather than there
        // because the point is not the string: it is that the operating system accepts what
        // comes out of it. On a Unix it passes because `PATH_MAX` is 4096; on Windows it
        // passes only because every call in `LocalFs` goes through `VPath::os`, and the CI
        // runner is the machine that proves it.
        let dir = tempfile::tempdir().unwrap();
        let mut deep = dir.path().to_path_buf();
        // Twelve directories of twenty-four characters: past 260 even from a short root.
        for i in 0..12 {
            deep.push(format!("a-directory-no-{i:02}-xxxxxx"));
        }
        let folder = VPath::local(&deep);
        assert!(
            fcmd_core::is_long(folder.as_path()),
            "the fixture is not long enough: {} characters",
            folder.as_path().as_os_str().len()
        );

        LocalFs.create_dir_all(&folder).expect("mkdir -p");
        assert!(LocalFs.exists(&folder));
        assert!(LocalFs.facts(&folder).expect("stat").is_dir());

        let file = VPath::local(deep.join("notes.txt"));
        {
            // Through the engine's own writer, which is what a copy uses.
            let source = VPath::local(dir.path().join("source.txt"));
            fs::write(source.as_path(), b"a long way down\n").unwrap();
            let mut nothing = |_: u64| true;
            LocalFs
                .copy_file(&source, &file, CopyHow::buffered(64 * 1024), &mut nothing)
                .expect("the copy");
        }
        let mut read = Vec::new();
        LocalFs
            .read_chunks(&file, 64 * 1024, &mut |chunk| {
                read.extend_from_slice(chunk);
                true
            })
            .expect("read it back");
        assert_eq!(read, b"a long way down\n");
        assert_eq!(LocalFs.facts(&file).expect("stat").size, 16);

        // And the whole way back out again: a rename, then a delete.
        let moved = VPath::local(deep.join("renamed.txt"));
        LocalFs.rename(&file, &moved).expect("rename");
        assert!(!LocalFs.exists(&file));
        assert!(LocalFs.exists(&moved));
        LocalFs.remove_file(&moved).expect("delete");
        LocalFs.remove_dir(&folder).expect("rmdir");
        assert!(!LocalFs.exists(&folder));
    }
}
