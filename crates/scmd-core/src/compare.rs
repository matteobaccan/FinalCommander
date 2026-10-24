//! Are these two files the same file?
//!
//! Not "do they have the same name, size and date", which is what comparing two folders
//! answers and what every file manager means by *compare*. This is the other question, the one
//! nobody can answer by looking: the same bytes, read from both and checked against each
//! other.
//!
//! No hashing. A hash of both sides reads every byte of both files and then compares 32 bytes
//! — this stops at the first byte that differs, which for two files that are *not* the same is
//! usually in the first block. Hashing wins only when one side is compared against many, which
//! is not what a folder comparison does. And a hash can collide, while a comparison of the
//! bytes cannot be wrong.

use std::io::{self, Read};

use crate::CancelToken;

/// How much is read from each side at a time. Large enough that the system call is not the
/// cost, small enough that two of them do not matter.
const CHUNK: usize = 64 * 1024;

/// Read both streams to the end, or to the first difference.
///
/// `on_bytes` is called with the bytes read *from one side* as the comparison goes, for a
/// progress bar; returning `false` from it stops the comparison, which then reports
/// [`io::ErrorKind::Interrupted`] rather than an answer it did not reach.
///
/// Two streams of different lengths are different as soon as one ends, which is also how a
/// short read in the middle is handled: the buffers are filled, not merely read once.
pub fn same_contents(
    a: &mut dyn Read,
    b: &mut dyn Read,
    cancel: &CancelToken,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<bool> {
    let mut left = vec![0u8; CHUNK];
    let mut right = vec![0u8; CHUNK];
    loop {
        if cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let got_a = fill(a, &mut left)?;
        let got_b = fill(b, &mut right)?;
        if got_a != got_b {
            // One of them ended first: whatever is in front of that, they are not the same
            // file. (A short read that is not the end cannot happen here — `fill` only stops
            // short at end of stream.)
            return Ok(false);
        }
        if got_a == 0 {
            return Ok(true);
        }
        if left[..got_a] != right[..got_b] {
            return Ok(false);
        }
        if !on_bytes(got_a as u64) {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
    }
}

/// Read until the buffer is full or the stream ends, which is not what one `read` promises.
fn fill(source: &mut dyn Read, buffer: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buffer.len() {
        match source.read(&mut buffer[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compare(a: &[u8], b: &[u8]) -> io::Result<bool> {
        same_contents(
            &mut io::Cursor::new(a.to_vec()),
            &mut io::Cursor::new(b.to_vec()),
            &CancelToken::new(),
            &mut |_| true,
        )
    }

    #[test]
    fn the_same_bytes_are_the_same_file() {
        assert!(compare(b"", b"").unwrap());
        assert!(compare(b"hello", b"hello").unwrap());
        let long = vec![7u8; CHUNK * 3 + 11];
        assert!(compare(&long, &long).unwrap());
    }

    #[test]
    fn one_byte_is_enough_to_differ() {
        assert!(!compare(b"hello", b"hellp").unwrap());
        assert!(!compare(b"", b"x").unwrap());
        assert!(!compare(b"x", b"").unwrap());
        // A difference past the first chunk is found as surely as one in it.
        let mut a = vec![0u8; CHUNK * 2 + 5];
        let mut b = a.clone();
        b[CHUNK * 2 + 4] = 1;
        assert!(!compare(&a, &b).unwrap());
        // And a length that differs only after a whole number of chunks.
        a.truncate(CHUNK * 2);
        assert!(!compare(&a, &b).unwrap());
    }

    /// A stream that hands over a few bytes at a time — a socket, a decompressor — must not
    /// be read as if a short read meant the end.
    #[test]
    fn a_dribbling_stream_is_read_to_the_end() {
        struct Dribble(Vec<u8>, usize);
        impl Read for Dribble {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                if self.1 >= self.0.len() {
                    return Ok(0);
                }
                let n = out.len().min(3).min(self.0.len() - self.1);
                out[..n].copy_from_slice(&self.0[self.1..self.1 + n]);
                self.1 += n;
                Ok(n)
            }
        }
        let bytes: Vec<u8> = (0..=255u8).cycle().take(CHUNK + 100).collect();
        let mut a = Dribble(bytes.clone(), 0);
        let mut b = io::Cursor::new(bytes.clone());
        assert!(same_contents(&mut a, &mut b, &CancelToken::new(), &mut |_| true).unwrap());

        let mut shorter = bytes.clone();
        shorter.pop();
        let mut a = Dribble(bytes, 0);
        let mut b = io::Cursor::new(shorter);
        assert!(!same_contents(&mut a, &mut b, &CancelToken::new(), &mut |_| true).unwrap());
    }

    #[test]
    fn the_bytes_read_are_reported_and_the_reader_can_stop() {
        let bytes = vec![3u8; CHUNK * 2];
        let mut seen = 0u64;
        let same = same_contents(
            &mut io::Cursor::new(bytes.clone()),
            &mut io::Cursor::new(bytes.clone()),
            &CancelToken::new(),
            &mut |n| {
                seen += n;
                true
            },
        )
        .unwrap();
        assert!(same);
        assert_eq!(seen, bytes.len() as u64);

        // Saying no stops it, and the answer is not an answer.
        let err = same_contents(
            &mut io::Cursor::new(bytes.clone()),
            &mut io::Cursor::new(bytes.clone()),
            &CancelToken::new(),
            &mut |_| false,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);

        // So does the token, and before anything is read.
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = same_contents(
            &mut io::Cursor::new(bytes.clone()),
            &mut io::Cursor::new(bytes),
            &cancel,
            &mut |_| true,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
    }

    #[test]
    fn a_read_that_fails_is_not_an_answer() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "no"))
            }
        }
        let err = same_contents(
            &mut Broken,
            &mut io::Cursor::new(vec![0u8; 10]),
            &CancelToken::new(),
            &mut |_| true,
        )
        .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }
}
