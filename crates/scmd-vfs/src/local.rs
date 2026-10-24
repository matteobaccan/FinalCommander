use std::ffi::OsString;
use std::fs::{self, Metadata};
use std::path::Path;

use async_trait::async_trait;
use scmd_core::{Entry, EntryFlags, EntryId, EntryKind, Error, Result, VPath};

use crate::{CancelToken, Caps, Vfs};

/// The local filesystem. Blocking `std::fs` calls are moved off the runtime threads, because
/// a `stat` on a dead network mount can block for 30 seconds and must never freeze the UI.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalVfs;

#[async_trait]
impl Vfs for LocalVfs {
    fn scheme(&self) -> &'static str {
        "file"
    }

    fn caps(&self) -> Caps {
        let base = Caps::READ
            | Caps::WRITE
            | Caps::CREATE
            | Caps::DELETE
            | Caps::RENAME
            | Caps::SEEK
            | Caps::TRUNCATE
            | Caps::META_TIMES
            | Caps::LINKS
            | Caps::HARDLINKS
            | Caps::WATCH
            | Caps::RESUME
            | Caps::RANDOM_WRITE
            | Caps::ATOMIC_RENAME;
        if cfg!(unix) {
            base | Caps::META_MODE | Caps::META_OWNER | Caps::XATTR | Caps::CASE_SENSITIVE
        } else {
            base | Caps::STREAMS
        }
    }

    async fn list(&self, path: &VPath, cancel: CancelToken) -> Result<Vec<Entry>> {
        let owned = path.clone();
        tokio::task::spawn_blocking(move || list_blocking(&owned, &cancel))
            .await
            .map_err(|e| Error::Other(format!("listing task failed: {e}")))?
    }

    async fn stat(&self, path: &VPath) -> Result<Entry> {
        let owned = path.clone();
        tokio::task::spawn_blocking(move || {
            let meta = fs::symlink_metadata(owned.as_path()).map_err(|e| Error::io(&owned, e))?;
            let name = owned
                .file_name()
                .map(OsString::from)
                .unwrap_or_else(|| OsString::from(owned.to_string()));
            Ok(entry_from_meta(name, owned.as_path(), &meta))
        })
        .await
        .map_err(|e| Error::Other(format!("stat task failed: {e}")))?
    }
}

/// The path as the filesystem itself spells it.
///
/// Symbolic links are resolved — `/tmp` on macOS is really `/private/tmp` — and on Windows a
/// short `RUNNER~1` comes back as the long name. Anything that walks a directory and then
/// tries to find a given path among the rows needs this, because the rows carry the spelling
/// the filesystem gave them and nothing else will match.
///
/// A path we cannot resolve is returned as it is: a server, an archive, or something that has
/// been removed since. This is never used to *show* a path — a panel shows what the user
/// typed — only to look one up.
pub fn real_path(path: &VPath) -> VPath {
    if !path.is_local() || path.in_container() {
        return path.clone();
    }
    match fs::canonicalize(path.as_path()) {
        Ok(real) => VPath::local(strip_verbatim(real)),
        Err(_) => path.clone(),
    }
}

/// Windows' `canonicalize` hands back a verbatim path (`\\?\C:\x`), which no listing of ours
/// ever produces and which the user would not recognise. Everything else is already plain.
fn strip_verbatim(path: std::path::PathBuf) -> std::path::PathBuf {
    if !cfg!(windows) {
        return path;
    }
    let text = path.to_string_lossy().into_owned();
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{rest}"));
    }
    match text.strip_prefix(r"\\?\") {
        Some(rest) => std::path::PathBuf::from(rest),
        None => path,
    }
}

/// The synchronous core of [`LocalVfs::list`], kept public for tests and for the sizing walker.
pub fn list_blocking(path: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>> {
    if path.in_container() {
        // Not ours: `as_path` would name the archive file, and a listing of it would be the
        // directory the archive sits in. The archive provider owns these paths.
        return Err(Error::Unsupported { path: path.clone() });
    }
    let read = fs::read_dir(path.as_path()).map_err(|e| Error::io(path, e))?;
    let mut out = Vec::with_capacity(64);
    if !path.is_root() {
        out.push(Entry::parent_entry());
    }
    for item in read {
        if cancel.is_cancelled() {
            return Err(Error::Cancelled);
        }
        // One unreadable entry must not lose the whole directory.
        let Ok(item) = item else { continue };
        let name = item.file_name();
        let meta = match item.metadata() {
            Ok(m) => m,
            Err(_) => match fs::symlink_metadata(item.path()) {
                Ok(m) => m,
                Err(_) => continue,
            },
        };
        out.push(entry_from_meta(name, &item.path(), &meta));
    }
    Ok(out)
}

/// One path, as a panel row. A list panel is built from paths rather than from a directory,
/// and re-reading it means asking this about each of them again.
pub fn stat_blocking(path: &VPath) -> Result<Entry> {
    let real = path.as_path();
    // The link itself, not what it points at: a results panel should say a link is a link,
    // and a broken one must not vanish from the list.
    let meta = fs::symlink_metadata(real).map_err(|e| Error::io(path, e))?;
    let name = real
        .file_name()
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from(real.as_os_str()));
    Ok(entry_from_meta(name, real, &meta))
}

fn entry_from_meta(name: OsString, full: &Path, meta: &Metadata) -> Entry {
    let kind = if meta.is_dir() {
        EntryKind::Dir
    } else if meta.is_symlink() {
        EntryKind::Symlink
    } else if meta.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    };

    let mut flags = EntryFlags::empty();
    flags.set(EntryFlags::HIDDEN, is_hidden(&name, meta));
    flags.set(EntryFlags::READONLY, meta.permissions().readonly());
    flags.set(EntryFlags::EXECUTABLE, is_executable(meta));
    if kind == EntryKind::Symlink && fs::metadata(full).is_err() {
        flags.insert(EntryFlags::BROKEN_LINK);
    }

    Entry {
        name,
        kind,
        size: if kind == EntryKind::Dir {
            0
        } else {
            meta.len()
        },
        mtime: meta.modified().ok(),
        // Both are optional on purpose: `created` is an error on ext3 and on most network
        // mounts, and a filesystem mounted `noatime` keeps a stale access time at best. A
        // directory listing must not fail over either, so the field simply stays empty.
        btime: meta.created().ok(),
        atime: meta.accessed().ok(),
        flags,
        id: entry_id(full, meta),
        link_target: if kind == EntryKind::Symlink {
            fs::read_link(full).ok().map(|p| p.into_os_string())
        } else {
            None
        },
    }
}

#[cfg(unix)]
fn is_hidden(name: &OsString, _meta: &Metadata) -> bool {
    name.to_string_lossy().starts_with('.')
}

#[cfg(windows)]
fn is_hidden(name: &OsString, meta: &Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
    // Two rules, both of which a user expects on Windows: the attributes Explorer honours,
    // and the leading dot. `Ctrl+H` has to mean the same thing on all three platforms, or
    // `.git` and `.env` would be in the way here and out of the way everywhere else.
    name.to_string_lossy().starts_with('.')
        || meta.file_attributes() & (FILE_ATTRIBUTE_HIDDEN | FILE_ATTRIBUTE_SYSTEM) != 0
}

#[cfg(unix)]
fn is_executable(meta: &Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    meta.permissions().mode() & 0o111 != 0
}

#[cfg(windows)]
fn is_executable(_meta: &Metadata) -> bool {
    false
}

#[cfg(unix)]
fn entry_id(_full: &Path, meta: &Metadata) -> EntryId {
    use std::os::unix::fs::MetadataExt;
    EntryId::Fs(meta.dev(), meta.ino())
}

#[cfg(windows)]
fn entry_id(full: &Path, _meta: &Metadata) -> EntryId {
    // A stable file id needs GetFileInformationByHandle; until `scmd-platform` provides it,
    // hash the full path, which is stable for as long as the name is.
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    full.hash(&mut h);
    EntryId::Name(h.finish())
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_path_comes_back_the_way_the_filesystem_spells_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("real")).unwrap();
        let here = VPath::local(dir.path().join("real"));

        // Whatever the platform does to a temporary path — `/private` on macOS, a short name
        // on Windows — the answer is a path that a listing of its parent would produce.
        let real = real_path(&here);
        assert!(real.as_path().is_dir(), "{real}");
        let parent = real.parent().expect("a parent");
        let names = list_blocking(&parent, &CancelToken::new()).expect("the listing");
        assert!(
            names.iter().any(|e| e.name_str() == real.display_name()),
            "{} is not among {:?}",
            real,
            names
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect::<Vec<_>>()
        );

        // A link resolves to what it points at, which is the whole reason this exists.
        #[cfg(unix)]
        {
            let link = dir.path().join("link");
            std::os::unix::fs::symlink(dir.path().join("real"), &link).unwrap();
            assert_eq!(real_path(&VPath::local(&link)), real);
        }

        // What cannot be resolved comes back untouched rather than empty.
        let gone = VPath::local(dir.path().join("not-here"));
        assert_eq!(real_path(&gone), gone);
        let remote = VPath::remote(scmd_core::Scheme::Sftp, "host", "/srv");
        assert_eq!(real_path(&remote), remote);
    }
    use super::*;

    #[test]
    fn lists_a_directory_with_a_parent_entry() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        fs::create_dir(dir.path().join("sub")).unwrap();

        let path = VPath::local(dir.path());
        let entries = list_blocking(&path, &CancelToken::new()).unwrap();

        assert!(entries.iter().any(|e| e.kind == EntryKind::Parent));
        let file = entries.iter().find(|e| e.name_str() == "a.txt").unwrap();
        assert_eq!(file.kind, EntryKind::File);
        assert_eq!(file.size, 5);
        let sub = entries.iter().find(|e| e.name_str() == "sub").unwrap();
        assert_eq!(sub.kind, EntryKind::Dir);
    }

    #[test]
    fn hidden_files_are_flagged() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(".secret"), b"x").unwrap();
        let entries = list_blocking(&VPath::local(dir.path()), &CancelToken::new()).unwrap();
        let hidden = entries.iter().find(|e| e.name_str() == ".secret").unwrap();
        // A dotfile counts as hidden on every platform, Windows included.
        assert!(hidden.flags.contains(EntryFlags::HIDDEN));
    }

    #[test]
    fn creation_and_access_times_are_read_when_the_filesystem_has_them() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.txt"), b"hello").unwrap();
        let entries = list_blocking(&VPath::local(dir.path()), &CancelToken::new()).unwrap();
        let file = entries.iter().find(|e| e.name_str() == "a.txt").unwrap();

        // Whether the platform has a creation time is not ours to decide; that it never costs
        // us the listing is. A time we did get must at least be no newer than now.
        let now = std::time::SystemTime::now();
        for stamp in [file.mtime, file.btime, file.atime].into_iter().flatten() {
            assert!(
                stamp <= now + std::time::Duration::from_secs(1),
                "{stamp:?}"
            );
        }
        assert!(
            file.atime.is_some(),
            "every supported platform has an atime"
        );
    }

    #[test]
    fn cancellation_is_reported_not_swallowed() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..50 {
            fs::write(dir.path().join(format!("f{i}")), b"x").unwrap();
        }
        let token = CancelToken::new();
        token.cancel();
        let err = list_blocking(&VPath::local(dir.path()), &token).unwrap_err();
        assert!(matches!(err, Error::Cancelled));
    }

    #[test]
    fn missing_directory_yields_an_io_error_with_the_path() {
        let path = VPath::local("/definitely/not/here");
        let err = list_blocking(&path, &CancelToken::new()).unwrap_err();
        assert!(err.to_string().contains("/definitely/not/here"));
        assert!(err.os_code().is_some());
    }

    #[tokio::test]
    async fn async_list_and_stat_agree() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("x.bin"), vec![0u8; 1234]).unwrap();
        let vfs = LocalVfs;
        let entries = vfs
            .list(&VPath::local(dir.path()), CancelToken::new())
            .await
            .unwrap();
        assert!(entries.iter().any(|e| e.name_str() == "x.bin"));
        let stat = vfs
            .stat(&VPath::local(dir.path().join("x.bin")))
            .await
            .unwrap();
        assert_eq!(stat.size, 1234);
        assert!(vfs.caps().contains(Caps::READ));
        assert!(vfs.random_access());
    }
}
