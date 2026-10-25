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

/// Do these two differ only in how their lines end?
///
/// FAR's own compare plugin has this as a checkbox in a dialog — "ignore differences in new
/// line characters" — and it is there because the question comes up constantly: the same tree
/// checked out on Windows and on Linux, a file edited once in an editor that rewrote every
/// line, a patch that went through a mail client. Every byte after the first line differs, and
/// none of it means anything.
///
/// Rather than a checkbox, this is asked *after* the bytes have been found to differ, so the
/// answer can be a third thing instead of a setting: the same, different, or the same but for
/// the line endings. Nobody has to know in advance which question they wanted.
///
/// `\r\n` and a lone `\r` both count as a line ending, because old Mac files exist and a
/// file half converted has both. A zero byte on either side ends it: a binary file has no
/// lines, and treating its bytes as text would make two different files look alike.
pub fn same_but_for_line_endings(
    a: &mut dyn Read,
    b: &mut dyn Read,
    cancel: &CancelToken,
) -> io::Result<bool> {
    let mut left = Lines::new(a);
    let mut right = Lines::new(b);
    loop {
        if cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        let (x, y) = (left.next_byte()?, right.next_byte()?);
        if left.binary || right.binary {
            return Ok(false);
        }
        match (x, y) {
            (None, None) => return Ok(true),
            (Some(x), Some(y)) if x == y => {}
            _ => return Ok(false),
        }
    }
}

/// A byte stream with its line endings all turned into `\n`.
struct Lines<'a> {
    source: &'a mut dyn Read,
    buffer: Vec<u8>,
    filled: usize,
    at: usize,
    /// A `\r` was the last byte of the last block: whether it is `\r\n` or a lone `\r` is
    /// not known until the next block arrives.
    held_cr: bool,
    /// A zero byte was seen, so this is not text and the comparison is over.
    binary: bool,
}

impl<'a> Lines<'a> {
    fn new(source: &'a mut dyn Read) -> Lines<'a> {
        Lines {
            source,
            buffer: vec![0u8; CHUNK],
            filled: 0,
            at: 0,
            held_cr: false,
            binary: false,
        }
    }

    fn raw(&mut self) -> io::Result<Option<u8>> {
        if self.at == self.filled {
            self.filled = fill(self.source, &mut self.buffer)?;
            self.at = 0;
            if self.filled == 0 {
                return Ok(None);
            }
        }
        let byte = self.buffer[self.at];
        self.at += 1;
        if byte == 0 {
            self.binary = true;
        }
        Ok(Some(byte))
    }

    /// The next byte, with `\r\n` and a lone `\r` both reported as `\n`.
    fn next_byte(&mut self) -> io::Result<Option<u8>> {
        loop {
            let byte = match self.raw()? {
                Some(byte) => byte,
                None => {
                    // A `\r` at the very end of the file is a line ending of its own.
                    return Ok(if std::mem::take(&mut self.held_cr) {
                        Some(b'\n')
                    } else {
                        None
                    });
                }
            };
            if self.held_cr {
                self.held_cr = byte == b'\r';
                // `\r\n` is one ending; `\r` then anything else is an ending and that byte.
                if byte == b'\n' || byte == b'\r' {
                    return Ok(Some(b'\n'));
                }
                self.at -= 1;
                return Ok(Some(b'\n'));
            }
            if byte == b'\r' {
                self.held_cr = true;
                continue;
            }
            return Ok(Some(byte));
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

    fn only_eol(a: &[u8], b: &[u8]) -> bool {
        same_but_for_line_endings(
            &mut io::Cursor::new(a.to_vec()),
            &mut io::Cursor::new(b.to_vec()),
            &CancelToken::new(),
        )
        .unwrap()
    }

    /// The same file checked out twice, on two systems. Every line differs and none of it
    /// means anything.
    #[test]
    fn the_same_text_with_other_line_endings() {
        assert!(only_eol(b"one\ntwo\n", b"one\r\ntwo\r\n"));
        assert!(only_eol(b"one\r\ntwo\r\n", b"one\ntwo\n"));
        // An old Mac file, and one that is half converted.
        assert!(only_eol(b"one\rtwo\r", b"one\ntwo\n"));
        assert!(only_eol(b"one\r\ntwo\nthree\r", b"one\ntwo\nthree\n"));
        // A `\r` at the very end is a line ending of its own and not a lost byte.
        assert!(only_eol(b"x\r", b"x\n"));
        assert!(only_eol(b"", b""));
        // The same bytes are also "only" the line endings, trivially.
        assert!(only_eol(b"same\n", b"same\n"));
        // Across a block boundary, where the `\r` and its `\n` are read separately.
        let long: Vec<u8> = b"line\r\n"
            .iter()
            .copied()
            .cycle()
            .take(CHUNK * 2 + 6)
            .collect();
        let unix: Vec<u8> = long.iter().copied().filter(|b| *b != b'\r').collect();
        assert!(only_eol(&long, &unix));
    }

    #[test]
    fn a_real_difference_is_still_a_difference() {
        assert!(!only_eol(b"one\ntwo\n", b"one\nTWO\n"));
        assert!(!only_eol(b"one\n", b"one\ntwo\n"));
        assert!(!only_eol(b"one two\n", b"one  two\n"));
        // Whitespace that is not a line ending is a difference: FAR has a second checkbox for
        // that one, and a file manager that quietly called two different files the same would
        // be worse than one that asked.
        assert!(!only_eol(b"a\tb\n", b"a b\n"));
    }

    /// A binary file has no lines, and two different binaries must not look alike because one
    /// of them happens to hold a `\r`.
    #[test]
    fn bytes_that_are_not_text_are_compared_as_bytes() {
        assert!(!only_eol(b"\x00\r\n", b"\x00\n"));
        assert!(!only_eol(b"head\x00\r\ntail", b"head\x00\ntail"));
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
