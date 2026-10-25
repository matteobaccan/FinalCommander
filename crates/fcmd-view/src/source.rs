use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

/// Random-access bytes. The viewer never loads a file: it asks for the window it is showing.
///
/// A provider that cannot seek (a stream inside a solid archive) will be spooled to a
/// temporary file first; see `docs/spec/05-viewer-editor.md` §1.2.
pub trait Source: Send {
    /// Current length. It can grow between calls — that is how follow mode works.
    fn len(&mut self) -> u64;

    fn is_empty(&mut self) -> bool {
        self.len() == 0
    }

    /// Read into `buf` starting at `offset`, returning how many bytes were read (0 at EOF).
    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize>;

    /// Convenience: read up to `len` bytes as a vector.
    fn read_vec(&mut self, offset: u64, len: usize) -> io::Result<Vec<u8>> {
        let mut buf = vec![0u8; len];
        let read = self.read_at(offset, &mut buf)?;
        buf.truncate(read);
        Ok(buf)
    }
}

/// A file on disk — or a raw device, which on Linux and macOS is the same thing (SPEC 05
/// §1.10). Only the length has to be learned differently, and `fcmd_core::device` does that.
pub struct FileSource {
    file: File,
    len: u64,
    /// A device, a pipe, a socket: anything that is not a regular file. The length of one is
    /// measured rather than read from its metadata, and it is never re-stat'd.
    device: bool,
}

impl FileSource {
    pub fn open(path: &Path) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let meta = file.metadata()?;
        let device = fcmd_core::is_device(&meta);
        let len = fcmd_core::device::length(&mut file, &meta).bytes;
        Ok(FileSource { file, len, device })
    }

    /// Is this a device rather than a file? The viewer says so in the status line, and the
    /// editor refuses to write to one.
    pub fn is_device(&self) -> bool {
        self.device
    }
}

impl Source for FileSource {
    fn len(&mut self) -> u64 {
        // Re-stat: the file may be growing under us (a log being written). A device does not
        // grow, and its metadata would say zero, so the measured length stands.
        if !self.device {
            if let Ok(meta) = self.file.metadata() {
                self.len = meta.len();
            }
        }
        self.len
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        self.file.seek(SeekFrom::Start(offset))?;
        let mut read = 0;
        while read < buf.len() {
            match self.file.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(n) => read += n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
        Ok(read)
    }
}

/// Bytes in memory — used by the tests and by previews of generated content.
pub struct BytesSource {
    bytes: Vec<u8>,
}

impl BytesSource {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Self {
        BytesSource {
            bytes: bytes.into(),
        }
    }

    /// Append, so a test can exercise follow mode.
    pub fn append(&mut self, more: &[u8]) {
        self.bytes.extend_from_slice(more);
    }
}

impl Source for BytesSource {
    fn len(&mut self) -> u64 {
        self.bytes.len() as u64
    }

    fn read_at(&mut self, offset: u64, buf: &mut [u8]) -> io::Result<usize> {
        let start = offset.min(self.bytes.len() as u64) as usize;
        let available = &self.bytes[start..];
        let n = available.len().min(buf.len());
        buf[..n].copy_from_slice(&available[..n]);
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bytes_source_reads_windows() {
        let mut s = BytesSource::new(b"0123456789".to_vec());
        assert_eq!(s.len(), 10);
        assert_eq!(s.read_vec(3, 4).unwrap(), b"3456");
        assert_eq!(s.read_vec(8, 100).unwrap(), b"89");
        assert!(s.read_vec(99, 4).unwrap().is_empty());
    }

    #[test]
    fn file_source_sees_the_file_grow() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("log");
        std::fs::write(&path, b"first\n").unwrap();
        let mut s = FileSource::open(&path).unwrap();
        assert_eq!(s.len(), 6);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap()
            .write_all(b"second\n")
            .unwrap();
        assert_eq!(s.len(), 13, "follow mode needs the length to be re-read");
        assert_eq!(s.read_vec(6, 6).unwrap(), b"second");
    }

    use std::io::Write;
}
