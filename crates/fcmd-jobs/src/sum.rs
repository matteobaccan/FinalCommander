//! Checksums: writing a list of digests, and checking files against one.
//!
//! The format written is the one `md5sum`, `sha1sum` and `sha256sum` write and read —
//! `<hex>  <name>`, two spaces — because the point of a checksum file is that somebody else
//! can check it with the tool they already have. The formats *read* are wider than that: the
//! same line with ` *name` (what those tools call binary mode), and BSD's
//! `SHA256 (name) = <hex>`, which is what macOS and FreeBSD produce.
//!
//! Which hash a list was made with is worked out from the digests in it rather than from the
//! file's name. A list is called `SHA256SUMS` or `sums.txt` or nothing in particular, and the
//! length of a hex digest says what made it: 32 characters is MD5, 40 is SHA-1, 64 is SHA-256.
//! A name can lie; a length cannot.

use std::collections::BTreeSet;

use fcmd_core::{CancelToken, VPath};

use crate::fs::JobFs;
use crate::run::Outcome;

/// The hashes offered. Three, and all three on purpose: SHA-256 is what to choose, SHA-1 and
/// MD5 are what the lists one is handed were made with.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Algorithm {
    Md5,
    Sha1,
    #[default]
    Sha256,
}

impl Algorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            Algorithm::Md5 => "md5",
            Algorithm::Sha1 => "sha1",
            Algorithm::Sha256 => "sha256",
        }
    }

    /// What a list made with it is called, by the convention those tools use.
    pub fn extension(self) -> &'static str {
        self.as_str()
    }

    pub fn parse(text: &str) -> Option<Algorithm> {
        match text
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], "")
            .as_str()
        {
            "md5" => Some(Algorithm::Md5),
            "sha1" => Some(Algorithm::Sha1),
            "sha256" => Some(Algorithm::Sha256),
            _ => None,
        }
    }

    /// The hash whose digest is this many hex characters, if exactly one is.
    pub fn of_digest_len(len: usize) -> Option<Algorithm> {
        match len {
            32 => Some(Algorithm::Md5),
            40 => Some(Algorithm::Sha1),
            64 => Some(Algorithm::Sha256),
            _ => None,
        }
    }
}

/// One line of a list: a digest, and the name it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub digest: String,
    pub name: String,
}

/// What checking one line found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The bytes hash to what the list says.
    Matches,
    /// They do not.
    Differs,
    /// There is no such file beside the list.
    Missing,
    /// It is there and could not be read.
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verdict {
    pub name: String,
    pub state: State,
}

/// The hash of one file, as lower-case hex.
///
/// `on_bytes` is called with each chunk's length, so a job can show where it is inside a file
/// of forty gigabytes; returning `false` from it stops the read and the whole call.
pub fn digest(
    fs: &dyn JobFs,
    path: &VPath,
    algorithm: Algorithm,
    buffer: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> std::io::Result<String> {
    // Three hashes, three crates, one `update` each — and all three were already in the tree
    // under the SSH client, so this costs nothing to build.
    let mut md5 = md5::Context::new();
    let mut sha1 = <sha1::Sha1 as sha1::Digest>::new();
    let mut sha256 = <sha2::Sha256 as sha2::Digest>::new();
    fs.read_chunks(path, buffer, &mut |chunk| {
        match algorithm {
            Algorithm::Md5 => md5.consume(chunk),
            Algorithm::Sha1 => sha1::Digest::update(&mut sha1, chunk),
            Algorithm::Sha256 => sha2::Digest::update(&mut sha256, chunk),
        }
        on_bytes(chunk.len() as u64)
    })?;
    Ok(match algorithm {
        Algorithm::Md5 => hex(md5.finalize().as_ref()),
        Algorithm::Sha1 => hex(sha1::Digest::finalize(sha1).as_ref()),
        Algorithm::Sha256 => hex(sha2::Digest::finalize(sha256).as_ref()),
    })
}

fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Hash every path, in the order given, and report each one as it is finished.
///
/// A file that cannot be read is collected with its reason and the rest go on, as a delete
/// over a tree does: one unreadable file in a hundred should not cost the other ninety-nine.
/// The names in the lines are the paths' last parts, because a list lives beside the files it
/// is about — the whole reason those tools write names and not paths.
pub fn create(
    fs: &dyn JobFs,
    paths: &[VPath],
    algorithm: Algorithm,
    buffer: usize,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&VPath, u64),
) -> (Vec<Line>, Outcome) {
    let mut lines = Vec::with_capacity(paths.len());
    let mut outcome = Outcome::default();
    for (n, path) in paths.iter().enumerate() {
        if cancel.is_cancelled() {
            outcome.cancelled = true;
            break;
        }
        progress(path, n as u64);
        let mut bytes = 0u64;
        let mut tick = |n: u64| {
            bytes += n;
            !cancel.is_cancelled()
        };
        match digest(fs, path, algorithm, buffer, &mut tick) {
            Ok(digest) => {
                lines.push(Line {
                    digest,
                    name: path.display_name(),
                });
                outcome.files += 1;
                outcome.bytes += bytes;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                outcome.cancelled = true;
                break;
            }
            Err(e) => outcome.errors.push((path.clone(), e.to_string())),
        }
    }
    (lines, outcome)
}

/// Check each line against the file of that name in `dir`.
pub fn verify(
    fs: &dyn JobFs,
    dir: &VPath,
    lines: &[Line],
    algorithm: Algorithm,
    buffer: usize,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> (Vec<Verdict>, Outcome) {
    let mut verdicts = Vec::with_capacity(lines.len());
    let mut outcome = Outcome::default();
    for (n, line) in lines.iter().enumerate() {
        if cancel.is_cancelled() {
            outcome.cancelled = true;
            break;
        }
        progress(&line.name, n as u64);
        let path = dir.join(std::ffi::OsStr::new(&line.name));
        if !fs.exists(&path) {
            verdicts.push(Verdict {
                name: line.name.clone(),
                state: State::Missing,
            });
            continue;
        }
        let mut bytes = 0u64;
        let mut tick = |n: u64| {
            bytes += n;
            !cancel.is_cancelled()
        };
        match digest(fs, &path, algorithm, buffer, &mut tick) {
            Ok(found) => {
                let state = if found.eq_ignore_ascii_case(&line.digest) {
                    State::Matches
                } else {
                    State::Differs
                };
                verdicts.push(Verdict {
                    name: line.name.clone(),
                    state,
                });
                outcome.files += 1;
                outcome.bytes += bytes;
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                outcome.cancelled = true;
                break;
            }
            Err(e) => {
                outcome.errors.push((path.clone(), e.to_string()));
                verdicts.push(Verdict {
                    name: line.name.clone(),
                    state: State::Unreadable,
                });
            }
        }
    }
    (verdicts, outcome)
}

/// A list as those tools write it: `<hex>  <name>`, one per line, in the order given.
pub fn write_list(lines: &[Line]) -> String {
    let mut out = String::new();
    for line in lines {
        out.push_str(&line.digest);
        out.push_str("  ");
        out.push_str(&line.name);
        out.push('\n');
    }
    out
}

/// Read a list, and say which hash made it.
///
/// `None` for the algorithm means the lines do not agree on one — two digest lengths in one
/// file, or none that is a hash's length — which is a list this cannot check rather than one
/// to guess about. Blank lines and `#` comments are skipped, as those tools skip them.
pub fn parse_list(text: &str) -> (Vec<Line>, Option<Algorithm>) {
    let mut lines = Vec::new();
    let mut lengths = BTreeSet::new();
    for raw in text.lines() {
        let raw = raw.trim();
        if raw.is_empty() || raw.starts_with('#') || raw.starts_with(';') {
            continue;
        }
        let parsed = parse_bsd(raw).or_else(|| parse_coreutils(raw));
        if let Some(line) = parsed {
            lengths.insert(line.digest.len());
            lines.push(line);
        }
    }
    let algorithm = (lengths.len() == 1)
        .then(|| lengths.iter().next().copied())
        .flatten()
        .and_then(Algorithm::of_digest_len);
    (lines, algorithm)
}

/// `<hex>  <name>`, or `<hex> *<name>` for what those tools call binary mode.
fn parse_coreutils(raw: &str) -> Option<Line> {
    let (digest, rest) = raw.split_once(char::is_whitespace)?;
    if !is_hex(digest) {
        return None;
    }
    let name = rest.trim_start().trim_start_matches('*').trim();
    (!name.is_empty()).then(|| Line {
        digest: digest.to_ascii_lowercase(),
        name: name.to_string(),
    })
}

/// `SHA256 (name) = <hex>`, which is what macOS and FreeBSD write.
fn parse_bsd(raw: &str) -> Option<Line> {
    let (head, digest) = raw.rsplit_once('=')?;
    let digest = digest.trim();
    if !is_hex(digest) {
        return None;
    }
    let open = head.find('(')?;
    let close = head.rfind(')')?;
    if close <= open + 1 {
        return None;
    }
    Algorithm::parse(&head[..open])?;
    Some(Line {
        digest: digest.to_ascii_lowercase(),
        name: head[open + 1..close].to_string(),
    })
}

fn is_hex(text: &str) -> bool {
    !text.is_empty() && text.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;

    fn file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> VPath {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        VPath::local(path)
    }

    fn sum(path: &VPath, algorithm: Algorithm) -> String {
        let mut ignore = |_: u64| true;
        digest(&LocalFs, path, algorithm, 64 * 1024, &mut ignore).expect("a readable file")
    }

    /// The published vectors. A hash either agrees with the rest of the world or it is not
    /// that hash, and these are the three lines that say which.
    #[test]
    fn the_hashes_are_the_hashes_everybody_else_computes() {
        let dir = tempfile::tempdir().unwrap();
        let abc = file(dir.path(), "abc", b"abc");
        assert_eq!(
            sum(&abc, Algorithm::Md5),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            sum(&abc, Algorithm::Sha1),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            sum(&abc, Algorithm::Sha256),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        // And the empty file, which is the other vector everybody publishes.
        let empty = file(dir.path(), "empty", b"");
        assert_eq!(
            sum(&empty, Algorithm::Md5),
            "d41d8cd98f00b204e9800998ecf8427e"
        );
        assert_eq!(
            sum(&empty, Algorithm::Sha256),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn a_file_larger_than_the_buffer_hashes_the_same_as_one_pass_of_it() {
        let dir = tempfile::tempdir().unwrap();
        let bytes: Vec<u8> = (0..200_000u32).map(|n| (n % 251) as u8).collect();
        let big = file(dir.path(), "big.bin", &bytes);
        let mut ignore = |_: u64| true;
        let chunked = digest(&LocalFs, &big, Algorithm::Sha256, 4 * 1024, &mut ignore).unwrap();
        let whole = digest(&LocalFs, &big, Algorithm::Sha256, 1 << 20, &mut ignore).unwrap();
        assert_eq!(chunked, whole, "the chunk size must not change the answer");
    }

    #[test]
    fn a_list_is_written_the_way_the_tools_read_it() {
        let dir = tempfile::tempdir().unwrap();
        let a = file(dir.path(), "a.txt", b"abc");
        let b = file(dir.path(), "b.txt", b"");
        let mut nothing = |_: &VPath, _: u64| {};
        let (lines, outcome) = create(
            &LocalFs,
            &[a, b],
            Algorithm::Md5,
            64 * 1024,
            &CancelToken::new(),
            &mut nothing,
        );
        assert_eq!(outcome.files, 2);
        assert!(outcome.errors.is_empty());
        assert_eq!(
            write_list(&lines),
            "900150983cd24fb0d6963f7d28e17f72  a.txt\n\
             d41d8cd98f00b204e9800998ecf8427e  b.txt\n"
        );
    }

    #[test]
    fn both_shapes_of_list_are_read_and_the_hash_comes_from_the_digests() {
        let (lines, algorithm) = parse_list(
            "# written by somebody else\n\
             \n\
             900150983cd24fb0d6963f7d28e17f72  a.txt\n\
             D41D8CD98F00B204E9800998ECF8427E *b.bin\n",
        );
        assert_eq!(algorithm, Some(Algorithm::Md5));
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[1].name, "b.bin");
        assert_eq!(lines[1].digest, "d41d8cd98f00b204e9800998ecf8427e");

        let (lines, algorithm) =
            parse_list("SHA256 (notes.txt) = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad\n");
        assert_eq!(algorithm, Some(Algorithm::Sha256));
        assert_eq!(lines[0].name, "notes.txt");
    }

    #[test]
    fn a_list_that_does_not_agree_on_one_hash_is_not_guessed_at() {
        let (lines, algorithm) = parse_list(
            "900150983cd24fb0d6963f7d28e17f72  a.txt\n\
             a9993e364706816aba3e25717850c26c9cd0d89d  b.txt\n",
        );
        assert_eq!(lines.len(), 2, "both lines are read");
        assert_eq!(
            algorithm, None,
            "but there is no one hash to check them with"
        );
    }

    #[test]
    fn verifying_says_which_files_match_which_differ_and_which_are_not_there() {
        let dir = tempfile::tempdir().unwrap();
        file(dir.path(), "same.txt", b"abc");
        file(dir.path(), "changed.txt", b"not what it was");
        let lines = vec![
            Line {
                digest: "900150983cd24fb0d6963f7d28e17f72".into(),
                name: "same.txt".into(),
            },
            Line {
                digest: "900150983cd24fb0d6963f7d28e17f72".into(),
                name: "changed.txt".into(),
            },
            Line {
                digest: "900150983cd24fb0d6963f7d28e17f72".into(),
                name: "gone.txt".into(),
            },
        ];
        let mut nothing = |_: &str, _: u64| {};
        let (verdicts, outcome) = verify(
            &LocalFs,
            &VPath::local(dir.path()),
            &lines,
            Algorithm::Md5,
            64 * 1024,
            &CancelToken::new(),
            &mut nothing,
        );
        assert_eq!(verdicts[0].state, State::Matches);
        assert_eq!(verdicts[1].state, State::Differs);
        assert_eq!(verdicts[2].state, State::Missing);
        assert_eq!(outcome.files, 2, "the one that is not there was not read");
    }

    #[test]
    fn a_cancelled_run_stops_where_it_was_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let a = file(dir.path(), "a.txt", b"abc");
        let b = file(dir.path(), "b.txt", b"def");
        let cancel = CancelToken::new();
        cancel.cancel();
        let mut nothing = |_: &VPath, _: u64| {};
        let (lines, outcome) = create(
            &LocalFs,
            &[a, b],
            Algorithm::Sha256,
            64 * 1024,
            &cancel,
            &mut nothing,
        );
        assert!(lines.is_empty());
        assert!(outcome.cancelled);
    }

    #[test]
    fn a_name_can_be_read_back_into_the_hash_it_names() {
        for (text, want) in [
            ("md5", Algorithm::Md5),
            ("SHA-1", Algorithm::Sha1),
            ("sha_256", Algorithm::Sha256),
        ] {
            assert_eq!(Algorithm::parse(text), Some(want), "{text}");
        }
        assert_eq!(Algorithm::parse("crc32"), None);
    }
}
