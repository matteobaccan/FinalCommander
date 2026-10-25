//! Replacing one string with another across a list of files the caller has already chosen.
//!
//! This is a job ([`crate::JobKind::Replace`]): planned, counted, cancellable and reported like
//! a copy, because it is the same kind of long walk over files that the user sits and watches.
//! What it is not is a search — the caller hands over the list, and nothing here goes looking
//! for files.
//!
//! Two decisions shape everything below.
//!
//! **Counting and rewriting are one pass with one flag.** The dry run is what the user is shown
//! before being asked to go ahead, so a dry run that counted differently from what the write
//! then does would be worse than no dry run at all. [`ReplacePass`] is the whole of the
//! difference between them: the counting pass has nowhere to put its output, and nothing else
//! about it differs.
//!
//! **The work is on bytes, never on characters.** `fcmd-jobs` has no decoder of its own — the
//! viewer's code pages live in `fcmd-view`, which this crate does not depend on — so a
//! replacement copies every byte it did not match straight through. A CP1252 file stays CP1252,
//! a UTF-16 file keeps its code units, and no byte-order mark appears in a file that had none,
//! because nothing is ever decoded or re-encoded: there is no step at which an encoding could
//! be got wrong. The price is stated once, here. An ASCII needle matches in every code page
//! that holds ASCII, which is all of the ones a file manager meets; a needle with a character
//! outside ASCII only matches a file that spells that character the way the needle is spelled,
//! which in practice means a UTF-8 file. For everything else there is the hex pattern, which
//! says the bytes outright.

use std::io::{self, Read, Write};

/// What to look for and what to put in its place, as the user spelled it in the dialog.
///
/// Kept as text and flags rather than as something already compiled, so that a [`crate::JobSpec`]
/// stays cheap to clone and compare; [`Matcher::compile`] is what turns it into something that
/// can search.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceOptions {
    pub needle: String,
    pub replacement: String,
    pub case_sensitive: bool,
    /// Match only when neither side of the match is part of the same word.
    pub whole_words: bool,
    /// Read the needle as a regular expression, and let the replacement name its groups
    /// (`$1`, `${name}`).
    ///
    /// The expression is matched against the file's bytes with Unicode classes on, which is
    /// what a text file wants; `(?-u)` turns them off for one that is not text, where `.`
    /// should mean any byte rather than any character.
    pub pattern: bool,
    /// Read both sides as hex bytes (`de ad be ef`), which is the only way to say what a
    /// binary file holds without pretending it is text.
    pub hex: bool,
}

impl ReplaceOptions {
    /// Case-sensitive, literal, whole file: the reading that changes the fewest bytes.
    pub fn new(needle: impl Into<String>, replacement: impl Into<String>) -> Self {
        ReplaceOptions {
            needle: needle.into(),
            replacement: replacement.into(),
            case_sensitive: true,
            whole_words: false,
            pattern: false,
            hex: false,
        }
    }
}

impl Default for ReplaceOptions {
    fn default() -> Self {
        ReplaceOptions::new("", "")
    }
}

/// Every way a replacement is refused before a single file is opened. Each one is a sentence
/// the dialog can show as it stands, because that is where they end up.
#[derive(Debug, thiserror::Error)]
pub enum ReplaceError {
    #[error("there is nothing to search for")]
    EmptyNeedle,

    #[error("{needle}: {message}")]
    BadPattern { needle: String, message: String },

    #[error("{needle}: not a pattern of hex bytes")]
    BadHex { needle: String },

    #[error("a hex pattern and a regular expression are two different questions: choose one")]
    HexAndPattern,

    #[error("{needle}: this matches at every position, so it would change the whole file")]
    MatchesNothing { needle: String },
}

/// Whether a pass over a file writes its result or only counts what it found.
///
/// This is the only difference between a dry run and a replacement, and it is deliberately the
/// only one: the number the user is shown is produced by the code that then does the writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacePass {
    Count,
    Rewrite,
}

/// What one pass over one file found, and what it did about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Replaced {
    pub occurrences: u64,
    /// Would the file's bytes actually come out different? A needle replaced by itself matches
    /// as often as the user likes and changes nothing, and that is nothing to do rather than a
    /// rewrite.
    pub changed: bool,
    /// Whether the file on disk was really replaced. Never true for [`ReplacePass::Count`], and
    /// not true for a [`ReplacePass::Rewrite`] whose result was the bytes already there.
    pub rewritten: bool,
}

/// How much of a file a regular expression may be given at once.
///
/// A pattern has no bounded match length, so there is no window that can be promised to hold a
/// whole match and no way to stream one without risking a match cut in half. The two honest
/// answers are "read the whole file" and, past this point, "no" — a quietly missed match in the
/// middle of a 2 GB file is exactly the kind of wrong answer this crate refuses to give.
pub const PATTERN_FILE_LIMIT: u64 = 64 * 1024 * 1024;

/// A compiled needle: what to find, and what to put there instead.
///
/// Public because [`crate::JobFs::replace_in_file`] takes one, and a provider that implements
/// that call needs to be handed something it can use. Its insides are its own.
#[derive(Debug, Clone)]
pub struct Matcher {
    kind: Kind,
    whole_words: bool,
}

#[derive(Debug, Clone)]
enum Kind {
    /// A literal run of bytes, and the bytes that take its place.
    Bytes {
        needle: Vec<u8>,
        replacement: Vec<u8>,
        /// Fold `A`–`Z` against `a`–`z` while comparing. ASCII only, and that is not a
        /// shortcut: a byte outside ASCII means whatever the file's code page says it means,
        /// and this crate does not know the code page, so folding one would be a guess.
        fold_ascii_case: bool,
    },
    /// A regular expression over bytes, and a replacement that may name its groups.
    Pattern {
        re: regex::bytes::Regex,
        replacement: Vec<u8>,
    },
}

impl Matcher {
    /// Work out what the user asked for, or say why it cannot be done.
    pub fn compile(options: &ReplaceOptions) -> Result<Matcher, ReplaceError> {
        if options.needle.is_empty() {
            return Err(ReplaceError::EmptyNeedle);
        }
        if options.hex && options.pattern {
            return Err(ReplaceError::HexAndPattern);
        }

        let kind = if options.hex {
            let needle = parse_hex(&options.needle)?;
            if needle.is_empty() {
                return Err(ReplaceError::EmptyNeedle);
            }
            Kind::Bytes {
                needle,
                replacement: parse_hex(&options.replacement)?,
                // Case has no meaning for bytes the user wrote out one by one: `41` is the
                // byte 0x41 and nothing else.
                fold_ascii_case: false,
            }
        } else if options.pattern {
            let re = regex::bytes::RegexBuilder::new(&options.needle)
                .case_insensitive(!options.case_sensitive)
                .build()
                .map_err(|e| ReplaceError::BadPattern {
                    needle: options.needle.clone(),
                    message: one_line(&e.to_string()),
                })?;
            // `a*` matches at every position, including the gaps between characters, so a
            // replacement would be inserted into every one of them. That is never what anyone
            // meant, and it is a refusal rather than a job that destroys a thousand files.
            if re.is_match(b"") {
                return Err(ReplaceError::MatchesNothing {
                    needle: options.needle.clone(),
                });
            }
            Kind::Pattern {
                re,
                replacement: options.replacement.clone().into_bytes(),
            }
        } else {
            Kind::Bytes {
                needle: options.needle.clone().into_bytes(),
                replacement: options.replacement.clone().into_bytes(),
                fold_ascii_case: !options.case_sensitive,
            }
        };

        Ok(Matcher {
            kind,
            whole_words: options.whole_words,
        })
    }

    /// The most bytes a match can cover, when that can be known at all.
    ///
    /// A literal and a hex pattern are their own length, which is what lets them be matched
    /// through a window the size of one buffer. A regular expression has no bound, and that is
    /// the whole reason it is the one kind that has to see a file all at once.
    fn max_match_len(&self) -> Option<usize> {
        match &self.kind {
            Kind::Bytes { needle, .. } => Some(needle.len()),
            Kind::Pattern { .. } => None,
        }
    }
}

/// The one informative line of a compile error.
///
/// The `regex` crate's own message is a four-line caret diagram, which reads well in a terminal
/// and not at all in the one field of a dialog. The line that says what is actually wrong is the
/// one that starts with `error:`; if the shape of that message ever changes, the first line is a
/// worse answer but still an answer.
fn one_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("error:"))
        .map(|line| line.trim().to_string())
        .unwrap_or_else(|| {
            message
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .unwrap_or(message)
                .to_string()
        })
}

/// `de ad be ef`, `DEADBEEF`, `de-ad:be,ef` — hex digit pairs with whatever the user put
/// between them. An odd digit left over is an error, not a guessed nibble.
fn parse_hex(text: &str) -> Result<Vec<u8>, ReplaceError> {
    let digits: Vec<u8> = text
        .bytes()
        .filter(|b| !b.is_ascii_whitespace() && !matches!(b, b'-' | b',' | b':' | b'_'))
        .collect();
    if !digits.len().is_multiple_of(2) || digits.iter().any(|b| !b.is_ascii_hexdigit()) {
        return Err(ReplaceError::BadHex {
            needle: text.to_string(),
        });
    }
    Ok(digits
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).unwrap_or(0) as u8;
            let lo = (pair[1] as char).to_digit(16).unwrap_or(0) as u8;
            (hi << 4) | lo
        })
        .collect())
}

/// Where the result of a pass goes, and what the pass found on the way.
///
/// The `None` writer is the dry run, and it is the only thing a dry run changes.
struct Emit<'a> {
    out: Option<&'a mut dyn Write>,
    occurrences: u64,
    changed: bool,
}

impl<'a> Emit<'a> {
    fn new(out: Option<&'a mut dyn Write>) -> Self {
        Emit {
            out,
            occurrences: 0,
            changed: false,
        }
    }

    /// Bytes that are staying exactly as they are.
    fn copy(&mut self, bytes: &[u8]) -> io::Result<()> {
        if bytes.is_empty() {
            return Ok(());
        }
        match &mut self.out {
            Some(out) => out.write_all(bytes),
            None => Ok(()),
        }
    }

    /// A match, and what takes its place.
    fn replace(&mut self, found: &[u8], with: &[u8]) -> io::Result<()> {
        self.occurrences += 1;
        // A match whose replacement is the bytes already there is still a match — it is
        // reported as one — but it is not a change, and a file of nothing but those is a file
        // with nothing to do.
        if found != with {
            self.changed = true;
        }
        match &mut self.out {
            Some(out) => out.write_all(with),
            None => Ok(()),
        }
    }

    fn finish(self) -> Replaced {
        Replaced {
            occurrences: self.occurrences,
            changed: self.changed,
            rewritten: false,
        }
    }
}

/// Is this byte part of a word, for the whole-words option?
///
/// Letters, digits and `_`, as every editor's `\b` has it — and every byte from 0x80 up as
/// well. In any code page a file manager meets, those spell a letter of somebody's alphabet,
/// so treating them as word bytes is what stops `caf` from matching inside `café`.
fn is_word_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b >= 0x80
}

/// Is a match at `start` of `len` bytes bounded by non-word bytes on both sides? `prev` is the
/// byte before `hay[0]`, which a chunk in the middle of a file has and the start of a file does
/// not.
fn stands_alone(hay: &[u8], prev: Option<u8>, start: usize, len: usize) -> bool {
    let before = if start == 0 {
        prev
    } else {
        Some(hay[start - 1])
    };
    let after = hay.get(start + len).copied();
    !before.is_some_and(is_word_byte) && !after.is_some_and(is_word_byte)
}

/// Walk `hay`, hand the replaced result to `emit`, and return how many of its bytes were
/// decided.
///
/// When `eof` is false the tail that could still turn out to be the start of a match — or whose
/// following byte has yet to decide a whole-word boundary — is left undecided for the next
/// chunk, which is how a needle that straddles a buffer boundary is found anyway.
fn walk(
    hay: &[u8],
    prev: Option<u8>,
    eof: bool,
    matcher: &Matcher,
    emit: &mut Emit<'_>,
) -> io::Result<usize> {
    match &matcher.kind {
        Kind::Bytes {
            needle,
            replacement,
            fold_ascii_case,
        } => {
            let n = needle.len();
            let mut i = 0;
            // Start of the run of bytes that are staying as they are, written out in one go
            // when something interrupts it.
            let mut literal = 0;

            while i + n <= hay.len() {
                // The byte after a match decides its right-hand boundary, and at the end of a
                // chunk that byte has not arrived yet.
                if matcher.whole_words && i + n == hay.len() && !eof {
                    break;
                }
                let hit = matches_at(hay, i, needle, *fold_ascii_case)
                    && (!matcher.whole_words || stands_alone(hay, prev, i, n));
                if hit {
                    emit.copy(&hay[literal..i])?;
                    emit.replace(&hay[i..i + n], replacement)?;
                    i += n;
                    literal = i;
                } else {
                    i += 1;
                }
            }

            // Everything up to `i` is decided. At the end of the file that is all of it; in the
            // middle, the last few bytes wait for the chunk that completes them.
            let decided = if eof { hay.len() } else { i };
            emit.copy(&hay[literal..decided])?;
            Ok(decided)
        }

        Kind::Pattern { re, replacement } => {
            // No bounded match length means no window that can be trusted, so a pattern decides
            // nothing until it has the whole file. The caller grows the window until it does.
            if !eof {
                return Ok(0);
            }
            let mut literal = 0;
            let mut expanded = Vec::new();
            for caps in re.captures_iter(hay) {
                let found = caps.get(0).expect("a capture always has group 0");
                // A match the whole-word option throws out is passed over whole; the engine
                // does not go looking for a shorter one inside it.
                if matcher.whole_words && !stands_alone(hay, prev, found.start(), found.len()) {
                    continue;
                }
                emit.copy(&hay[literal..found.start()])?;
                expanded.clear();
                caps.expand(replacement, &mut expanded);
                emit.replace(found.as_bytes(), &expanded)?;
                literal = found.end();
            }
            emit.copy(&hay[literal..])?;
            Ok(hay.len())
        }
    }
}

fn matches_at(hay: &[u8], at: usize, needle: &[u8], fold_ascii_case: bool) -> bool {
    let window = &hay[at..at + needle.len()];
    if fold_ascii_case {
        window
            .iter()
            .zip(needle)
            .all(|(a, b)| a.eq_ignore_ascii_case(b))
    } else {
        window == needle
    }
}

/// Replace in bytes the caller already holds: a preview, or a file small enough to have been
/// read in one piece. The streaming pass walks this same code, chunk by chunk.
pub fn replace_bytes(bytes: &[u8], matcher: &Matcher) -> (Vec<u8>, Replaced) {
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let report = {
        let mut emit = Emit::new(Some(&mut out));
        // A `Vec` never refuses bytes, so there is no error here to pass on.
        let _ = walk(bytes, None, true, matcher, &mut emit);
        emit.finish()
    };
    (out, report)
}

/// One pass over `reader`, handing the result to `out` when there is one.
///
/// `on_bytes` is called with every chunk read, so a job knows where it is inside a large file;
/// returning `false` from it stops the pass with [`io::ErrorKind::Interrupted`], and the caller
/// decides what to do about a half-written result. Here, nothing has been promised about it.
pub(crate) fn pass(
    reader: &mut dyn Read,
    out: Option<&mut dyn Write>,
    matcher: &Matcher,
    buffer: usize,
    on_bytes: &mut dyn FnMut(u64) -> bool,
) -> io::Result<Replaced> {
    let chunk = buffer.clamp(16 * 1024, 8 * 1024 * 1024);
    let mut scratch = vec![0u8; chunk];
    let mut window: Vec<u8> = Vec::with_capacity(chunk);
    let mut emit = Emit::new(out);
    // The byte before `window[0]`, which a whole-word boundary at the start of a chunk needs.
    let mut prev: Option<u8> = None;
    let mut eof = false;

    while !eof {
        let read = loop {
            match reader.read(&mut scratch) {
                Ok(n) => break n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        };
        if read == 0 {
            eof = true;
        } else {
            window.extend_from_slice(&scratch[..read]);
            if !on_bytes(read as u64) {
                return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
            }
            if matcher.max_match_len().is_none() && window.len() as u64 > PATTERN_FILE_LIMIT {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "a regular expression has to see the whole file at once, and this file is \
                     too large to hold",
                ));
            }
        }

        let decided = walk(&window, prev, eof, matcher, &mut emit)?;
        if decided > 0 {
            prev = Some(window[decided - 1]);
            window.drain(..decided);
        }
    }

    Ok(emit.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{part_path, JobFs, LocalFs};
    use fcmd_core::{Scheme, VPath};
    use std::fs as stdfs;

    fn matcher(options: ReplaceOptions) -> Matcher {
        Matcher::compile(&options).expect("compile")
    }

    /// The whole of what a test needs: a file, and what one pass makes of it.
    fn run(
        bytes: &[u8],
        options: ReplaceOptions,
        pass_kind: ReplacePass,
    ) -> (Vec<u8>, Replaced, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("subject"));
        stdfs::write(path.as_path(), bytes).unwrap();
        let report = LocalFs
            .replace_in_file(&path, &matcher(options), pass_kind, 64 * 1024, &mut |_| {
                true
            })
            .expect("replace");
        let after = stdfs::read(path.as_path()).unwrap();
        (after, report, dir)
    }

    #[test]
    fn counting_and_rewriting_are_the_same_walk_over_the_file() {
        let text = b"one fish two fish red fish".as_slice();
        let options = ReplaceOptions::new("fish", "bird");

        let (untouched, counted, _dir) = run(text, options.clone(), ReplacePass::Count);
        assert_eq!(counted.occurrences, 3);
        assert!(counted.changed);
        assert!(!counted.rewritten, "a count may not write");
        assert_eq!(untouched, text, "a count left the file changed");

        let (after, written, _dir) = run(text, options, ReplacePass::Rewrite);
        assert_eq!(
            (written.occurrences, written.changed, written.rewritten),
            (counted.occurrences, counted.changed, true)
        );
        assert_eq!(after, b"one bird two bird red bird");
    }

    #[test]
    fn a_match_at_the_very_start_and_at_the_very_end_is_replaced() {
        let (after, report, _dir) = run(
            b"aa middle aa",
            ReplaceOptions::new("aa", "zzz"),
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 2);
        assert_eq!(after, b"zzz middle zzz");

        // And the degenerate shapes: nothing but the needle, and a needle as long as the file.
        let (after, report, _dir) = run(b"aa", ReplaceOptions::new("aa", ""), ReplacePass::Rewrite);
        assert_eq!((report.occurrences, after.len()), (1, 0));
    }

    #[test]
    fn a_needle_that_straddles_a_chunk_boundary_is_still_found() {
        // The smallest chunk the pass will read is 16 KiB, so a needle laid across that offset
        // is one that no single read ever holds whole.
        let chunk = 16 * 1024;
        let needle = b"straddle";
        let mut text = vec![b'.'; chunk * 3];
        // One across the first boundary, one ending exactly on the second, one starting there.
        let at = [
            chunk - needle.len() / 2,
            2 * chunk - needle.len(),
            2 * chunk,
        ];
        for start in at {
            text[start..start + needle.len()].copy_from_slice(needle);
        }

        let (after, report, _dir) = run(
            &text,
            ReplaceOptions::new("straddle", "CROSSED!"),
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 3, "a boundary swallowed a match");
        assert_eq!(
            after.len(),
            text.len(),
            "the replacement is the same length"
        );
        for start in at {
            assert_eq!(&after[start..start + needle.len()], b"CROSSED!", "{start}");
        }
        assert_eq!(
            after.iter().filter(|b| **b == b'.').count(),
            text.len() - at.len() * needle.len(),
            "a byte outside the matches was touched"
        );
    }

    #[test]
    fn whole_words_leaves_the_middle_of_a_longer_word_alone() {
        let options = ReplaceOptions {
            whole_words: true,
            ..ReplaceOptions::new("cat", "dog")
        };
        let (after, report, _dir) = run(
            b"cat, catalogue, the cat_alog, a cat. concatenate (cat)",
            options,
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 3, "only the three standing alone");
        assert_eq!(
            after,
            b"dog, catalogue, the cat_alog, a dog. concatenate (dog)"
        );
    }

    #[test]
    fn whole_words_holds_across_a_chunk_boundary_as_well() {
        // The deciding byte is the one after the match, which at a boundary is in the next
        // chunk: the carry has to wait for it rather than guess that the file ended.
        let chunk = 16 * 1024;
        let mut text = vec![b' '; chunk * 2];
        text[chunk - 3..chunk].copy_from_slice(b"cat");
        // Immediately followed by a letter, so this is the middle of `cats` and must not match.
        text[chunk] = b's';
        let options = ReplaceOptions {
            whole_words: true,
            ..ReplaceOptions::new("cat", "dog")
        };
        let (after, report, _dir) = run(&text, options, ReplacePass::Rewrite);
        assert_eq!(report.occurrences, 0, "`cats` is not the word `cat`");
        assert_eq!(after, text);
    }

    #[test]
    fn a_case_insensitive_needle_matches_every_spelling_of_itself() {
        let options = ReplaceOptions {
            case_sensitive: false,
            ..ReplaceOptions::new("Hello", "goodbye")
        };
        let (after, report, _dir) = run(b"hello HELLO HeLLo hellO", options, ReplacePass::Rewrite);
        assert_eq!(report.occurrences, 4);
        assert_eq!(after, b"goodbye goodbye goodbye goodbye");

        // And the same needle, taken at its word, finds only itself.
        let (after, report, _dir) = run(
            b"hello HELLO Hello",
            ReplaceOptions::new("Hello", "goodbye"),
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 1);
        assert_eq!(after, b"hello HELLO goodbye");
    }

    #[test]
    fn a_group_in_the_pattern_can_be_put_back_by_the_replacement() {
        let options = ReplaceOptions {
            pattern: true,
            ..ReplaceOptions::new(r"(\w+)@(\w+)\.com", "$2 of $1")
        };
        let (after, report, _dir) = run(
            b"write to mb@example.com or to root@localhost",
            options,
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 1);
        assert_eq!(after, b"write to example of mb or to root@localhost");
    }

    #[test]
    fn a_pattern_can_be_case_insensitive_and_bounded_to_whole_words_too() {
        let options = ReplaceOptions {
            pattern: true,
            case_sensitive: false,
            whole_words: true,
            ..ReplaceOptions::new("colou?r", "shade")
        };
        let (after, report, _dir) = run(b"Color colour colourless", options, ReplacePass::Rewrite);
        assert_eq!(report.occurrences, 2, "`colourless` is a longer word");
        assert_eq!(after, b"shade shade colourless");
    }

    #[test]
    fn a_hex_pattern_reaches_bytes_no_text_needle_could_spell() {
        let bytes = [0x00, 0xDE, 0xAD, 0xBE, 0xEF, 0x00, 0xDE, 0xAD];
        let options = ReplaceOptions {
            hex: true,
            ..ReplaceOptions::new("de ad", "C0:FE")
        };
        let (after, report, _dir) = run(&bytes, options, ReplacePass::Rewrite);
        assert_eq!(report.occurrences, 2);
        assert_eq!(after, [0x00, 0xC0, 0xFE, 0xBE, 0xEF, 0x00, 0xC0, 0xFE]);
    }

    #[test]
    fn a_hex_replacement_may_be_shorter_or_empty() {
        let options = ReplaceOptions {
            hex: true,
            ..ReplaceOptions::new("ffd8ffe0", "")
        };
        let (after, report, _dir) = run(
            &[0xFF, 0xD8, 0xFF, 0xE0, 0x01, 0x02],
            options,
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 1);
        assert_eq!(after, [0x01, 0x02]);
    }

    #[test]
    fn a_binary_file_nothing_matches_in_comes_out_byte_identical() {
        // Every byte value, twice, so anything that decoded or re-encoded would show up.
        let bytes: Vec<u8> = (0..=255u8).chain(0..=255u8).collect();
        let (after, report, _dir) = run(
            &bytes,
            ReplaceOptions::new("no such needle", "x"),
            ReplacePass::Rewrite,
        );
        assert_eq!(
            (report.occurrences, report.changed, report.rewritten),
            (0, false, false)
        );
        assert_eq!(after, bytes, "a file nothing matched in was rewritten");
    }

    #[test]
    fn a_file_in_another_code_page_keeps_its_own_bytes_and_gains_no_byte_order_mark() {
        // `caffè` in CP1252: the è is one byte, 0xE8, which is not valid UTF-8 at all.
        let mut bytes = b"un caffe".to_vec();
        bytes.extend_from_slice(b" corretto, un caff");
        bytes.push(0xE8);
        let (after, report, _dir) = run(
            &bytes,
            ReplaceOptions::new("corretto", "lungo"),
            ReplacePass::Rewrite,
        );
        assert_eq!(report.occurrences, 1);
        assert_eq!(after.first(), Some(&b'u'), "a BOM appeared out of nowhere");
        assert_eq!(
            after.last(),
            Some(&0xE8),
            "the one byte outside ASCII was re-encoded"
        );
        assert_eq!(after, b"un caffe lungo, un caff\xE8");
    }

    #[test]
    fn a_replacement_that_changes_no_byte_is_nothing_to_do() {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("same"));
        stdfs::write(path.as_path(), b"alpha beta alpha").unwrap();
        let before = stdfs::metadata(path.as_path()).unwrap();

        let report = LocalFs
            .replace_in_file(
                &path,
                &matcher(ReplaceOptions::new("alpha", "alpha")),
                ReplacePass::Rewrite,
                64 * 1024,
                &mut |_| true,
            )
            .unwrap();

        assert_eq!(report.occurrences, 2, "the matches are still reported");
        assert!(!report.changed, "nothing about the file would differ");
        assert!(!report.rewritten, "so the file must not have been replaced");
        assert_eq!(stdfs::read(path.as_path()).unwrap(), b"alpha beta alpha");
        assert_eq!(
            stdfs::metadata(path.as_path()).unwrap().modified().unwrap(),
            before.modified().unwrap(),
            "an untouched file keeps even its modification time"
        );
        assert!(!LocalFs.exists(&part_path(&path)));
    }

    #[test]
    fn a_failure_halfway_through_leaves_the_original_whole() {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("big.txt"));
        // Large enough that the pass is several reads in before it gives up.
        let mut text = Vec::new();
        for i in 0..4000 {
            text.extend_from_slice(format!("line {i} says alpha\n").as_bytes());
        }
        stdfs::write(path.as_path(), &text).unwrap();

        let mut chunks = 0;
        let err = LocalFs
            .replace_in_file(
                &path,
                &matcher(ReplaceOptions::new("alpha", "omega")),
                ReplacePass::Rewrite,
                16 * 1024,
                &mut |_| {
                    chunks += 1;
                    chunks < 2
                },
            )
            .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(chunks > 1, "the pass gave up before it had read anything");
        assert_eq!(
            stdfs::read(path.as_path()).unwrap(),
            text,
            "the original was truncated, or half rewritten"
        );
        assert!(
            !LocalFs.exists(&part_path(&path)),
            "the temporary file stayed behind"
        );
    }

    #[test]
    fn an_empty_needle_a_broken_pattern_and_a_bad_hex_string_are_refused_with_a_reason() {
        let bad = |options: ReplaceOptions| Matcher::compile(&options).unwrap_err();

        assert!(matches!(
            bad(ReplaceOptions::new("", "x")),
            ReplaceError::EmptyNeedle
        ));
        let err = bad(ReplaceOptions {
            pattern: true,
            ..ReplaceOptions::new("(unclosed", "x")
        });
        assert!(matches!(err, ReplaceError::BadPattern { .. }), "{err}");
        assert!(
            !err.to_string().contains('\n'),
            "an error a dialog shows has to be one line: {err}"
        );
        assert!(
            err.to_string().contains("unclosed group"),
            "and it has to say what is wrong, not that something is: {err}"
        );
        assert!(matches!(
            bad(ReplaceOptions {
                hex: true,
                ..ReplaceOptions::new("de a", "00")
            }),
            ReplaceError::BadHex { .. }
        ));
        assert!(matches!(
            bad(ReplaceOptions {
                hex: true,
                ..ReplaceOptions::new("zz", "00")
            }),
            ReplaceError::BadHex { .. }
        ));
        assert!(matches!(
            bad(ReplaceOptions {
                hex: true,
                pattern: true,
                ..ReplaceOptions::new("de ad", "00")
            }),
            ReplaceError::HexAndPattern
        ));
        // A pattern that matches the empty string matches between every pair of characters.
        for needle in ["a*", "", "x?", "^"] {
            let err = bad(ReplaceOptions {
                pattern: true,
                ..ReplaceOptions::new(needle, "!")
            });
            assert!(
                matches!(
                    err,
                    ReplaceError::MatchesNothing { .. } | ReplaceError::EmptyNeedle
                ),
                "{needle}: {err}"
            );
        }
    }

    #[test]
    fn hex_is_read_with_whatever_the_user_put_between_the_pairs() {
        for text in ["deadbeef", "de ad be ef", "DE-AD:BE,EF", " dead beef "] {
            assert_eq!(
                parse_hex(text).unwrap(),
                vec![0xDE, 0xAD, 0xBE, 0xEF],
                "{text}"
            );
        }
        assert!(
            parse_hex("").unwrap().is_empty(),
            "an empty replacement is a deletion"
        );
    }

    #[test]
    fn a_replacement_refuses_everything_it_cannot_promise() {
        let dir = tempfile::tempdir().unwrap();
        let real = VPath::local(dir.path().join("real.txt"));
        let link = VPath::local(dir.path().join("link.txt"));
        stdfs::write(real.as_path(), b"alpha").unwrap();
        LocalFs
            .symlink(std::path::Path::new("real.txt"), &link)
            .unwrap();
        let m = matcher(ReplaceOptions::new("alpha", "omega"));

        let err = LocalFs
            .replace_in_file(
                &VPath::remote(Scheme::Sftp, "host", "/srv/x"),
                &m,
                ReplacePass::Count,
                0,
                &mut |_| true,
            )
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Unsupported);

        // A symlink and a directory have no bytes of their own, and writing through the link
        // would rewrite a file the user never chose.
        for path in [&link, &VPath::local(dir.path())] {
            let err = LocalFs
                .replace_in_file(path, &m, ReplacePass::Rewrite, 0, &mut |_| true)
                .unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::InvalidInput, "{path}");
        }
        assert_eq!(
            stdfs::read(real.as_path()).unwrap(),
            b"alpha",
            "replacing through a link must not touch what it points at"
        );
    }

    #[test]
    fn replacing_in_a_buffer_walks_the_same_code_as_replacing_in_a_file() {
        let text = b"a-b-c-d".as_slice();
        let m = matcher(ReplaceOptions::new("-", "+"));
        let (out, report) = replace_bytes(text, &m);
        assert_eq!(out, b"a+b+c+d");
        assert_eq!(report.occurrences, 3);

        let (written, from_file, _dir) =
            run(text, ReplaceOptions::new("-", "+"), ReplacePass::Rewrite);
        assert_eq!(written, out);
        assert_eq!(from_file.occurrences, report.occurrences);
    }

    #[test]
    fn progress_covers_every_byte_of_the_file_whichever_pass_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let path = VPath::local(dir.path().join("counted.txt"));
        let text = vec![b'x'; 100_000];
        stdfs::write(path.as_path(), &text).unwrap();
        let m = matcher(ReplaceOptions::new("xxxx", "yyyy"));

        for pass_kind in [ReplacePass::Count, ReplacePass::Rewrite] {
            let mut seen = 0u64;
            LocalFs
                .replace_in_file(&path, &m, pass_kind, 16 * 1024, &mut |n| {
                    seen += n;
                    true
                })
                .unwrap();
            assert_eq!(seen, 100_000, "{pass_kind:?}");
            // Put it back, so the second pass reads the same file as the first.
            stdfs::write(path.as_path(), &text).unwrap();
        }
    }
}
