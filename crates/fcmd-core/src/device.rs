//! How long is the thing we just opened (SPEC 05 §1.10).
//!
//! For a regular file `stat` answers, and that is the whole story. For a raw device it does
//! not: on Linux and macOS `/dev/sda` and `/dev/disk0` are files whose metadata says they are
//! zero bytes long, and a viewer that believes it shows an empty screen over a terabyte of
//! disk. The length is there to be had, it just has to be asked for differently — by seeking
//! to the end, which the kernel answers for a block device without reading anything.
//!
//! This is deliberately free of `ioctl` and of a C dependency: a seek is portable, and what
//! it cannot answer — a character device, a pipe, a Windows `\\.\PhysicalDrive0`, where
//! seeking to the end answers nothing — it says it cannot answer, rather than guessing a
//! number the rest of the program would trust. For the Windows case `fcmd-platform` asks
//! the device itself, and hands the answer back here.
//!
//! What *is* here, beside the measuring, is the reading: a raw device on Windows will not
//! be read from an odd offset or in a part of a sector, and [`read_at`] is the one place
//! that knows it, so the viewer and the hex editor cannot disagree about it.

use std::fs::{File, Metadata};
use std::io::{Read, Seek, SeekFrom};

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

/// Where a read has to start and how long it has to be, for a device that will only be read
/// in whole blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// The offset to seek to: the one asked for, rounded down to a block boundary.
    pub start: u64,
    /// How many bytes to read: enough whole blocks to cover what was asked for.
    pub size: usize,
    /// How far into what was read the bytes asked for begin.
    pub skip: usize,
}

/// The block-aligned read that holds `offset .. offset + len`.
///
/// A raw device on Windows refuses a read that does not start on a sector boundary and is not
/// a whole number of sectors long — `ERROR_INVALID_PARAMETER`, for a call that would have
/// worked on any file. So the window is widened to the sectors around it, and the bytes asked
/// for are taken out of the middle.
pub fn aligned_window(offset: u64, len: usize, sector: u32) -> Window {
    let sector = sector.max(1) as u64;
    let start = offset - (offset % sector);
    let skip = (offset - start) as usize;
    // The tail is rounded up as well: the last sector is read whole or not at all.
    let span = skip as u64 + len as u64;
    Window {
        start,
        size: (span.div_ceil(sector) * sector) as usize,
        skip,
    }
}

/// Read `buf.len()` bytes from `offset`, through a handle that may be a raw device.
///
/// `sector` is `None` for a file, and for the devices that read like files — everything on
/// Linux and macOS — and `Some` only for a Windows raw device, which is read in whole
/// sectors. Returns how many bytes were actually put in `buf`: a short answer means the end,
/// which for a device is the end of the disk.
pub fn read_at(
    file: &mut File,
    offset: u64,
    buf: &mut [u8],
    sector: Option<u32>,
) -> std::io::Result<usize> {
    let Some(sector) = sector.filter(|s| *s > 1) else {
        file.seek(SeekFrom::Start(offset))?;
        return fill(file, buf);
    };
    let window = aligned_window(offset, buf.len(), sector);
    let mut whole = vec![0u8; window.size];
    file.seek(SeekFrom::Start(window.start))?;
    let read = fill(file, &mut whole)?;
    let from = read.min(window.skip);
    let available = &whole[from..read];
    let n = available.len().min(buf.len());
    buf[..n].copy_from_slice(&available[..n]);
    Ok(n)
}

/// Read until the buffer is full or the handle says there is no more.
fn fill(file: &mut File, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut read = 0;
    while read < buf.len() {
        match file.read(&mut buf[read..]) {
            Ok(0) => break,
            Ok(n) => read += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(read)
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

    /// The aligned read is the one half of this that can be proved without a disk: over an
    /// ordinary file, reading through the sector arithmetic has to give exactly the bytes
    /// that reading without it gives — at every offset and every length.
    #[test]
    fn an_aligned_read_gives_the_same_bytes_as_a_plain_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("disk.img");
        let body: Vec<u8> = (0..5000u32).map(|i| (i % 251) as u8).collect();
        std::fs::write(&path, &body).unwrap();
        let mut file = File::open(&path).unwrap();

        for sector in [512u32, 4096] {
            for offset in [0u64, 1, 7, 511, 512, 513, 4095, 4096, 4999] {
                for len in [1usize, 16, 100, 512, 1000, 4096] {
                    let mut aligned = vec![0u8; len];
                    let n = read_at(&mut file, offset, &mut aligned, Some(sector)).unwrap();
                    let start = (offset as usize).min(body.len());
                    let want = &body[start..(start + len).min(body.len())];
                    assert_eq!(n, want.len(), "sector {sector}, {offset}+{len}");
                    assert_eq!(&aligned[..n], want, "sector {sector}, {offset}+{len}");
                }
            }
        }
        // Past the end there is nothing, rather than a wrong number of bytes.
        let mut buf = [0u8; 64];
        assert_eq!(read_at(&mut file, 9000, &mut buf, Some(512)).unwrap(), 0);
        // And with no sector to honour it is the plain read, which must agree too.
        let mut plain = [0u8; 100];
        let n = read_at(&mut file, 600, &mut plain, None).unwrap();
        assert_eq!(&plain[..n], &body[600..700]);
    }

    #[test]
    fn a_window_covers_what_was_asked_for_in_whole_blocks() {
        assert_eq!(
            aligned_window(0, 16, 512),
            Window {
                start: 0,
                size: 512,
                skip: 0
            }
        );
        assert_eq!(
            aligned_window(600, 16, 512),
            Window {
                start: 512,
                size: 512,
                skip: 88
            }
        );
        // A read that straddles a boundary takes both blocks.
        assert_eq!(
            aligned_window(500, 100, 512),
            Window {
                start: 0,
                size: 1024,
                skip: 500
            }
        );
        // A sector of one is arithmetic nobody should have to think about.
        assert_eq!(
            aligned_window(7, 3, 1),
            Window {
                start: 7,
                size: 3,
                skip: 0
            }
        );
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
