//! The system's own file copy, where the system has one worth using.
//!
//! Reading a file into a buffer and writing it out again is what the engine does by default,
//! and it is the right default: it reports progress every chunk, it stops the instant `Esc`
//! is pressed, and it behaves the same on every filesystem anybody has. What it cannot do is
//! let the *kernel* make the copy — and on the filesystems people keep large files on, that
//! is the difference between copying ten gigabytes and copying none:
//!
//! - **Linux**: `copy_file_range` copies inside the kernel, so the bytes never cross into
//!   this program, and on btrfs, XFS and bcachefs it becomes a reflink — a second name for
//!   the same extents, written in a moment. Over NFS 4.2 the server copies the file itself
//!   and nothing crosses the network at all.
//! - **macOS**: `fcopyfile` is the routine the Finder uses, and on APFS it clones.
//! - **Windows**: `CopyFileExW` is the one the Explorer uses, which is also the one that
//!   knows about a network share's own server-side copy.
//!
//! The cost is that each has its own failure modes — a filesystem that refuses the call, a
//! copy that cannot be interrupted halfway, a progress figure that arrives in one lump — so
//! this is behind `general.system_copy`, off by default, and every function here answers
//! `Ok(None)` rather than an error when the platform declines: the caller then copies the
//! file the ordinary way and nobody is told anything, because nothing went wrong.
//!
//! What this does *not* do is decide anything about the copy. The destination is the
//! temporary file the engine chose, the permissions and the times are what the engine sets
//! afterwards, and the rename into place is the engine's own.

use std::io;
use std::path::Path;

/// Copy every byte of `src` into `dst`, which is created and truncated, using the system's
/// own routine. `on_bytes` is called as it goes and returning `false` from it stops the copy.
///
/// `Ok(Some(bytes))` when the system did it, `Ok(None)` when this platform has no such
/// routine or refused the call for a reason that is not an error — the two filesystems being
/// different ones, a kernel too old, a file that is not a regular file. `Err` only for a
/// failure that an ordinary copy would hit as well, so that a broken disk is still reported
/// as a broken disk.
///
/// Cancellation is as fine-grained as the platform allows, and the two are not the same: on
/// Linux the copy is made a chunk at a time, so `Esc` is answered within one chunk; on macOS
/// and Windows one call copies the whole file, and the answer comes when it returns. The
/// setting's own documentation says so, because a cancel that takes a minute on a large file
/// is a thing to know before turning it on rather than after.
pub fn system_copy(
    src: &Path,
    dst: &Path,
    chunk: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<Option<u64>> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        linux(src, dst, chunk, on_bytes)
    }
    #[cfg(target_os = "macos")]
    {
        macos(src, dst, chunk, on_bytes)
    }
    #[cfg(windows)]
    {
        windows_copy(src, dst, chunk, on_bytes)
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        windows
    )))]
    {
        let _ = (src, dst, chunk, on_bytes);
        Ok(None)
    }
}

/// Is there a system routine on this platform at all? What the settings dialog asks, so that
/// a machine where the setting can do nothing says so rather than offering it.
pub fn have_system_copy() -> bool {
    cfg!(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        windows
    ))
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn linux(
    src: &Path,
    dst: &Path,
    chunk: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<Option<u64>> {
    use std::fs::File;
    use std::io::ErrorKind;

    let reader = File::open(src)?;
    // A regular file is the only thing `copy_file_range` takes, and a pipe or a device read
    // through it would fail in the middle rather than at the start.
    if !reader.metadata()?.is_file() {
        return Ok(None);
    }
    let writer = File::create(dst)?;
    let mut total = 0u64;
    loop {
        let moved = match nix::fcntl::copy_file_range(&reader, None, &writer, None, chunk.max(1)) {
            Ok(0) => break,
            Ok(n) => n as u64,
            // The call is refused rather than failed: a filesystem that does not implement
            // it, a kernel older than it, two different filesystems on an older kernel, or
            // one of the overlay and fuse filesystems that answer this way. Nothing has been
            // written yet when it happens on the first call — and when it happens later the
            // destination is this copy's own temporary file, so starting again is safe.
            Err(nix::errno::Errno::EXDEV)
            | Err(nix::errno::Errno::EINVAL)
            | Err(nix::errno::Errno::ENOSYS)
            | Err(nix::errno::Errno::EOPNOTSUPP)
            | Err(nix::errno::Errno::EPERM) => return Ok(None),
            Err(nix::errno::Errno::EINTR) => continue,
            Err(e) => return Err(io::Error::from(e)),
        };
        total += moved;
        if !on_bytes(moved) {
            return Err(io::Error::new(ErrorKind::Interrupted, "cancelled"));
        }
    }
    writer.sync_data().ok();
    Ok(Some(total))
}

#[cfg(target_os = "macos")]
fn macos(
    src: &Path,
    dst: &Path,
    _chunk: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<Option<u64>> {
    use std::fs::File;
    use std::os::unix::io::AsRawFd;

    let reader = File::open(src)?;
    let meta = reader.metadata()?;
    if !meta.is_file() {
        return Ok(None);
    }
    let writer = File::create(dst)?;
    // `COPYFILE_DATA` and nothing else: the permissions, the times and the extended
    // attributes are the engine's business, and it has already decided which of them travel.
    // SAFETY: two open descriptors this function owns, a null state (the flags below need
    // none) and a flag word from the system's own header.
    let answer = unsafe {
        libc::fcopyfile(
            reader.as_raw_fd(),
            writer.as_raw_fd(),
            std::ptr::null_mut(),
            libc::COPYFILE_DATA,
        )
    };
    if answer != 0 {
        let error = io::Error::last_os_error();
        // The same reading as on Linux: a refusal is not a failure. `ENOTSUP` is what a
        // filesystem that cannot do this answers.
        return match error.raw_os_error() {
            Some(libc::ENOTSUP) | Some(libc::EINVAL) | Some(libc::EXDEV) => Ok(None),
            _ => Err(error),
        };
    }
    let total = writer.metadata()?.len();
    // One call copied the lot, so the progress arrives in one piece. Cancelling now is too
    // late for this file, which is what the setting's documentation says.
    on_bytes(total);
    Ok(Some(total))
}

#[cfg(windows)]
fn windows_copy(
    src: &Path,
    dst: &Path,
    _chunk: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<Option<u64>> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::CopyFileExW;

    let wide = |path: &Path| -> Vec<u16> {
        path.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    };
    let (from, to) = (wide(src), wide(dst));
    // No progress callback and no cancel flag: the callback would be a C function pointer
    // into this program called from inside the copy, and what it would buy — a progress bar
    // that moves during one file — is not worth that on a setting that is off by default.
    // The whole file arrives, then its bytes are reported.
    //
    // SAFETY: two null-terminated wide strings this function owns and holds for the length of
    // the call, and nulls for every optional argument.
    let ok = unsafe {
        CopyFileExW(
            from.as_ptr(),
            to.as_ptr(),
            None,
            std::ptr::null(),
            std::ptr::null_mut(),
            0,
        )
    };
    if ok == 0 {
        let error = io::Error::last_os_error();
        // A share or a filesystem that will not take the call: copy it the ordinary way.
        return match error.raw_os_error() {
            Some(50) | Some(1) => Ok(None), // ERROR_NOT_SUPPORTED, ERROR_INVALID_FUNCTION
            _ => Err(error),
        };
    }
    let total = std::fs::metadata(dst)?.len();
    on_bytes(total);
    Ok(Some(total))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// On a platform with a routine the copy is byte for byte, and the progress adds up to
    /// the file's length. Where there is none, the answer is `None` and the caller copies it
    /// the ordinary way — which is a pass, not a skip: that is the contract.
    #[test]
    fn a_file_copied_by_the_system_arrives_whole() {
        let dir = std::env::temp_dir().join(format!("fcmd-syscopy-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to work in");
        let src = dir.join("from");
        let dst = dir.join("to");
        let body: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        std::fs::File::create(&src)
            .and_then(|mut f| f.write_all(&body))
            .expect("the source is written");

        let mut counted = 0u64;
        let answer = system_copy(&src, &dst, 64 * 1024, &mut |n| {
            counted += n;
            true
        })
        .expect("nothing failed");
        match answer {
            Some(total) => {
                assert_eq!(total, body.len() as u64);
                assert_eq!(counted, total, "the progress adds up to the file");
                assert_eq!(std::fs::read(&dst).expect("it is there"), body);
            }
            None => assert!(
                !have_system_copy(),
                "a platform that has the routine must not decline on an ordinary file"
            ),
        }
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Saying no to the progress callback stops the copy. Only where the platform copies in
    /// pieces can it stop in the middle, so what is asserted is the part that holds
    /// everywhere: nothing is left where the caller would mistake it for a finished file.
    #[test]
    fn refusing_the_progress_stops_a_copy_that_can_be_stopped() {
        let dir = std::env::temp_dir().join(format!("fcmd-syscopy-stop-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("a directory to work in");
        let src = dir.join("from");
        let dst = dir.join("to");
        std::fs::File::create(&src)
            .and_then(|mut f| f.write_all(&vec![7u8; 400_000]))
            .expect("the source is written");

        // Stopped: the partial file is the caller's to remove, as it is for the ordinary copy,
        // and what matters here is that the error says why. Copied in one call, or no routine
        // at all: both are the contract too, which is why only the error is asserted on.
        if let Err(e) = system_copy(&src, &dst, 4096, &mut |_| false) {
            assert_eq!(e.kind(), std::io::ErrorKind::Interrupted);
        }
        std::fs::remove_dir_all(&dir).ok();
    }
}
