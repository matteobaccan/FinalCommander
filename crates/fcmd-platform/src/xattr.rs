//! Extended attributes: the named values a filesystem keeps beside a file's bytes.
//!
//! It is where a desktop puts what it knows about a file and the file itself cannot say: the
//! tags Dolphin writes as `user.xdg.tags`, the Finder's labels and its "this came from the
//! internet" mark, the address a browser downloaded the file from. A copy that carries the
//! bytes and not these comes out of a file manager untagged, which is why the job engine
//! asks here after it has written the data.
//!
//! Linux and macOS have the same idea behind two sets of calls that differ in their
//! arguments: Linux has a call of its own for "the link itself" (`lgetxattr`), macOS takes an
//! option and a position that only a resource fork ever uses. Everything here acts on **the
//! path itself and never on what a link points at**, as [`crate::set_owner`] does — a caller
//! that wants the target resolves the link first, in the open.
//!
//! Everywhere else there is nothing of the kind to read. NTFS has alternate data streams,
//! which are another model and deliberately not this one (`docs/BACKLOG.md`, Settled), so on
//! Windows a file has no attributes to list, and asking for one or setting one says it
//! cannot be done rather than reporting a success nothing performed.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;

/// The names of the attributes a file carries, in the order the filesystem gives them.
///
/// Empty for a file that has none, and on a platform that has no such thing. A filesystem
/// that keeps none (FAT, an old NFS) answers with the error the kernel gave, which a caller
/// copying on a best-effort basis treats the same as empty.
pub fn list(path: &Path) -> io::Result<Vec<OsString>> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        sys::list(path)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = path;
        Ok(Vec::new())
    }
}

/// The value of one attribute, whole. Values are bytes: a tag list is text, an icon is not.
pub fn get(path: &Path, name: &OsStr) -> io::Result<Vec<u8>> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        sys::get(path, name)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (path, name);
        Err(unsupported())
    }
}

/// Write one attribute, replacing the value it had.
///
/// What the filesystem refuses comes back as it refused it: no room for attributes at all
/// (`ENOTSUP`), a namespace that is not the caller's to write (`EPERM` for `security.*` and
/// `trusted.*`), a value larger than the filesystem keeps.
pub fn set(path: &Path, name: &OsStr, value: &[u8]) -> io::Result<()> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        sys::set(path, name, value)
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = (path, name, value);
        Err(unsupported())
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn unsupported() -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        "this system has no extended attributes",
    )
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod sys {
    use std::ffi::{CStr, CString, OsStr, OsString};
    use std::io;
    use std::os::unix::ffi::{OsStrExt, OsStringExt};
    use std::path::Path;

    /// A path or a name as the kernel takes it. One with a zero byte in it names nothing.
    fn c_string(bytes: &[u8]) -> io::Result<CString> {
        CString::new(bytes).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))
    }

    pub fn list(path: &Path) -> io::Result<Vec<OsString>> {
        let path = c_string(path.as_os_str().as_bytes())?;
        let names = whole(|buf| raw_list(&path, buf))?;
        // One name after another, each closed by a zero byte.
        Ok(names
            .split(|b| *b == 0)
            .filter(|name| !name.is_empty())
            .map(|name| OsString::from_vec(name.to_vec()))
            .collect())
    }

    pub fn get(path: &Path, name: &OsStr) -> io::Result<Vec<u8>> {
        let path = c_string(path.as_os_str().as_bytes())?;
        let name = c_string(name.as_bytes())?;
        whole(|buf| raw_get(&path, &name, buf))
    }

    pub fn set(path: &Path, name: &OsStr, value: &[u8]) -> io::Result<()> {
        let path = c_string(path.as_os_str().as_bytes())?;
        let name = c_string(name.as_bytes())?;
        if raw_set(&path, &name, value) == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    /// Read something whose length only the kernel knows: ask how long it is, make room, ask
    /// for it.
    ///
    /// Between the two calls somebody else may have made it longer, and then the second one
    /// says the room is too small (`ERANGE`) and the question is asked again. A few times
    /// only: a value that grows every time it is looked at is not one to wait for.
    fn whole(mut call: impl FnMut(&mut [u8]) -> isize) -> io::Result<Vec<u8>> {
        let mut last = None;
        for _ in 0..8 {
            let need = call(&mut []);
            if need < 0 {
                return Err(io::Error::last_os_error());
            }
            let mut buf = vec![0u8; need as usize];
            if buf.is_empty() {
                return Ok(buf);
            }
            let got = call(&mut buf);
            if got >= 0 {
                buf.truncate(got as usize);
                return Ok(buf);
            }
            let error = io::Error::last_os_error();
            if error.raw_os_error() != Some(libc::ERANGE) {
                return Err(error);
            }
            last = Some(error);
        }
        Err(last.unwrap_or_else(|| io::Error::other("the attribute kept changing")))
    }

    #[cfg(target_os = "linux")]
    fn raw_list(path: &CStr, buf: &mut [u8]) -> isize {
        // SAFETY: `path` is a NUL-terminated string that outlives the call. The kernel writes
        // at most `buf.len()` bytes into `buf`, and with a length of zero writes nothing and
        // only reports the size, so the dangling pointer of an empty slice is never used.
        unsafe { libc::llistxattr(path.as_ptr(), buf.as_mut_ptr().cast(), buf.len()) }
    }

    #[cfg(target_os = "linux")]
    fn raw_get(path: &CStr, name: &CStr, buf: &mut [u8]) -> isize {
        // SAFETY: `path` and `name` are NUL-terminated strings that outlive the call. The
        // kernel writes at most `buf.len()` bytes into `buf`, and with a length of zero
        // writes nothing and only reports the size.
        unsafe {
            libc::lgetxattr(
                path.as_ptr(),
                name.as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
            )
        }
    }

    #[cfg(target_os = "linux")]
    fn raw_set(path: &CStr, name: &CStr, value: &[u8]) -> i32 {
        // SAFETY: `path` and `name` are NUL-terminated strings that outlive the call, and the
        // kernel reads exactly `value.len()` bytes from `value`, which is that long. Flags of
        // zero mean "create it or replace it".
        unsafe {
            libc::lsetxattr(
                path.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
            )
        }
    }

    #[cfg(target_os = "macos")]
    fn raw_list(path: &CStr, buf: &mut [u8]) -> isize {
        // SAFETY: `path` is a NUL-terminated string that outlives the call. The kernel writes
        // at most `buf.len()` bytes into `buf`, and with a length of zero writes nothing and
        // only reports the size. `XATTR_NOFOLLOW` is what makes this the link itself.
        unsafe {
            libc::listxattr(
                path.as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                libc::XATTR_NOFOLLOW,
            )
        }
    }

    #[cfg(target_os = "macos")]
    fn raw_get(path: &CStr, name: &CStr, buf: &mut [u8]) -> isize {
        // SAFETY: `path` and `name` are NUL-terminated strings that outlive the call. The
        // kernel writes at most `buf.len()` bytes into `buf`, and with a length of zero
        // writes nothing and only reports the size. The position is an offset into a
        // resource fork and must be zero for everything else, which read from the start is.
        unsafe {
            libc::getxattr(
                path.as_ptr(),
                name.as_ptr(),
                buf.as_mut_ptr().cast(),
                buf.len(),
                0,
                libc::XATTR_NOFOLLOW,
            )
        }
    }

    #[cfg(target_os = "macos")]
    fn raw_set(path: &CStr, name: &CStr, value: &[u8]) -> i32 {
        // SAFETY: `path` and `name` are NUL-terminated strings that outlive the call, and the
        // kernel reads exactly `value.len()` bytes from `value`, which is that long. Position
        // zero writes the value from its start; no `XATTR_CREATE` or `XATTR_REPLACE`, so it
        // is created or replaced.
        unsafe {
            libc::setxattr(
                path.as_ptr(),
                name.as_ptr(),
                value.as_ptr().cast(),
                value.len(),
                0,
                libc::XATTR_NOFOLLOW,
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A file in a directory of its own, and whether the filesystem under it keeps the
    /// attributes an ordinary user may write. Not every one does — a `tmpfs` without them, a
    /// FAT stick, a CI runner's overlay — and a test that failed there would be reporting on
    /// the machine rather than on the code.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    fn tagged_file() -> Option<(tempfile::TempDir, std::path::PathBuf)> {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let file = dir.path().join("report.txt");
        std::fs::write(&file, b"words").expect("a file to tag");
        match set(&file, OsStr::new("user.fcmd.probe"), b"1") {
            Ok(()) => Some((dir, file)),
            Err(e) => {
                eprintln!("skipped: no user attributes under {:?} ({e})", dir.path());
                None
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn an_attribute_written_is_listed_and_read_back_byte_for_byte() {
        let Some((_dir, file)) = tagged_file() else {
            return;
        };
        let name = OsStr::new("user.xdg.tags");
        set(&file, name, "holiday,été".as_bytes()).expect("the tag is written");
        // Bytes, not text: a zero in the middle is part of the value.
        let icon = OsStr::new("user.fcmd.icon");
        set(&file, icon, &[0, 159, 0, 255]).expect("the bytes are written");

        let names = list(&file).expect("the names");
        assert!(names.iter().any(|n| n == name), "{names:?}");
        assert!(names.iter().any(|n| n == icon), "{names:?}");
        assert_eq!(get(&file, name).expect("the tag"), "holiday,été".as_bytes());
        assert_eq!(get(&file, icon).expect("the bytes"), [0, 159, 0, 255]);

        // Writing it again replaces it, and an empty value is a value.
        set(&file, name, b"").expect("emptied");
        assert_eq!(get(&file, name).expect("still there"), b"");
        assert!(list(&file).expect("the names").iter().any(|n| n == name));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn a_large_value_comes_back_whole() {
        let Some((_dir, file)) = tagged_file() else {
            return;
        };
        // Larger than any buffer somebody might have guessed at, smaller than the block a
        // filesystem keeps attributes in.
        let value: Vec<u8> = (0..1500u32).map(|i| (i % 251) as u8).collect();
        let name = OsStr::new("user.fcmd.large");
        if let Err(e) = set(&file, name, &value) {
            eprintln!("skipped: this filesystem keeps no value of 1500 bytes ({e})");
            return;
        }
        assert_eq!(get(&file, name).expect("the value"), value);
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn what_is_not_there_is_an_error_and_not_an_empty_value() {
        let Some((dir, file)) = tagged_file() else {
            return;
        };
        assert!(get(&file, OsStr::new("user.fcmd.nobody.wrote.this")).is_err());
        let gone = dir.path().join("gone.txt");
        assert!(list(&gone).is_err());
        assert!(get(&gone, OsStr::new("user.xdg.tags")).is_err());
        assert!(set(&gone, OsStr::new("user.xdg.tags"), b"x").is_err());
        // A name the kernel cannot be handed is refused here, before it gets there.
        let broken = set(&file, OsStr::new("user.a\0b"), b"x").expect_err("a zero in a name");
        assert_eq!(broken.kind(), io::ErrorKind::InvalidInput);
    }

    /// The link itself, never its target: the reason for `l…xattr` and `XATTR_NOFOLLOW`.
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    #[test]
    fn a_link_is_asked_about_itself_and_not_about_its_target() {
        let Some((dir, file)) = tagged_file() else {
            return;
        };
        set(&file, OsStr::new("user.xdg.tags"), b"target").expect("the target is tagged");
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&file, &link).expect("a link");
        let on_link = list(&link).unwrap_or_default();
        assert!(
            !on_link.iter().any(|n| n == "user.xdg.tags"),
            "the link answered with its target's attributes: {on_link:?}"
        );
        assert!(get(&link, OsStr::new("user.xdg.tags")).is_err());
        // Linux keeps no user attributes on a link at all and refuses; macOS writes it on
        // the link. Either way the target is as it was.
        let _ = set(&link, OsStr::new("user.xdg.tags"), b"link");
        assert_eq!(
            get(&file, OsStr::new("user.xdg.tags")).expect("the target's tag"),
            b"target"
        );
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    #[test]
    fn a_system_without_them_lists_nothing_and_says_it_cannot_set_one() {
        let dir = tempfile::tempdir().expect("a temporary directory");
        let file = dir.path().join("report.txt");
        std::fs::write(&file, b"words").expect("a file");
        assert!(list(&file).expect("nothing is not an error").is_empty());
        let name = OsStr::new("user.xdg.tags");
        assert_eq!(
            set(&file, name, b"x")
                .expect_err("nowhere to put it")
                .kind(),
            io::ErrorKind::Unsupported
        );
        assert_eq!(
            get(&file, name).expect_err("nothing to read").kind(),
            io::ErrorKind::Unsupported
        );
    }
}
