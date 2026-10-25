//! How long is the thing we just opened (SPEC 05 §1.10).
//!
//! For a regular file `stat` answers, and that is the whole story. For a raw device it does
//! not: on Linux and macOS `/dev/sda` and `/dev/disk0` are files whose metadata says they are
//! zero bytes long, and a viewer that believes it shows an empty screen over a terabyte of
//! disk. The length is there to be had, it just has to be asked for differently — by seeking
//! to the end, which the kernel answers for a block device without reading anything.
//!
//! This is deliberately free of `ioctl` and of a C dependency: a seek is portable, and what
//! it cannot answer (a character device, a pipe, a Windows `\\.\PhysicalDrive0`) it says it
//! cannot answer, rather than guessing a number the rest of the program would trust.

use std::fs::{File, Metadata};
use std::io::{Seek, SeekFrom};

/// The byte length of an open file or device, and whether that length is to be trusted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Length {
    pub bytes: u64,
    /// `false` when nothing we can ask knows the size: a pipe, a character device, a raw
    /// Windows volume. The caller shows what it can read and says the end is unknown.
    pub known: bool,
}

/// How long the thing behind this handle is.
///
/// A regular file is its metadata length. Anything else — a block device above all — is
/// measured by seeking to the end and back, which leaves the handle where it found it.
pub fn length(file: &mut File, meta: &Metadata) -> Length {
    if meta.is_file() && meta.len() > 0 {
        return Length {
            bytes: meta.len(),
            known: true,
        };
    }
    // Zero-length metadata is either an empty file or a device pretending to be one. A seek
    // tells the two apart: on a device it answers the size, on an empty file it answers 0,
    // and on something unseekable it fails.
    let here = file.stream_position().unwrap_or(0);
    let end = file.seek(SeekFrom::End(0));
    let _ = file.seek(SeekFrom::Start(here));
    match end {
        Ok(bytes) => Length {
            bytes,
            // An empty regular file is known to be empty; an unseekable device is not.
            known: bytes > 0 || meta.is_file(),
        },
        // A seek that fails says nothing about a regular file — some of them, procfs's
        // among them, are not seekable at all — so there metadata stays the authority.
        Err(_) => Length {
            bytes: meta.len(),
            known: meta.is_file(),
        },
    }
}

/// Is this path something other than a regular file — a device, a pipe, a socket? What the
/// warning before a write is for, and what makes a size worth measuring rather than reading.
pub fn is_device(meta: &Metadata) -> bool {
    !meta.is_file() && !meta.is_dir() && !meta.is_symlink()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn a_regular_file_is_its_metadata_length() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        std::fs::write(&path, b"0123456789").unwrap();
        let mut file = File::open(&path).unwrap();
        let meta = file.metadata().unwrap();
        assert_eq!(
            length(&mut file, &meta),
            Length {
                bytes: 10,
                known: true
            }
        );
        assert!(!is_device(&meta));
    }

    #[test]
    fn an_empty_file_is_known_to_be_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty");
        File::create(&path).unwrap();
        let mut file = File::open(&path).unwrap();
        let meta = file.metadata().unwrap();
        assert_eq!(
            length(&mut file, &meta),
            Length {
                bytes: 0,
                known: true
            }
        );
    }

    #[test]
    fn measuring_leaves_the_handle_where_it_was() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("f");
        let mut out = File::create(&path).unwrap();
        out.write_all(&[7u8; 4096]).unwrap();
        drop(out);
        let mut file = File::open(&path).unwrap();
        file.seek(SeekFrom::Start(100)).unwrap();
        let meta = file.metadata().unwrap();
        assert_eq!(length(&mut file, &meta).bytes, 4096);
        assert_eq!(file.stream_position().unwrap(), 100);
    }

    /// A file whose metadata says zero and which cannot be seeked at all. A block device is
    /// the case this module exists for, but it needs a disk and permissions CI does not have;
    /// `/proc/self/stat` is the reachable one with the same shape, and the rule it proves is
    /// the one that matters either way: a regular file's length is never reported as unknown,
    /// whatever the seek says.
    #[test]
    #[cfg(target_os = "linux")]
    fn a_file_that_cannot_be_measured_falls_back_to_its_metadata() {
        let Ok(mut file) = File::open("/proc/self/stat") else {
            return;
        };
        let meta = file.metadata().unwrap();
        assert_eq!(meta.len(), 0, "the premise of this test");
        let measured = length(&mut file, &meta);
        assert!(
            measured.known,
            "a regular file's length is never unknown: {measured:?}"
        );
    }
}
