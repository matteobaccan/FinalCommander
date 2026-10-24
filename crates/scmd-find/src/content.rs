//! The content condition: a needle, and the chunked scan that looks for it.
//!
//! The viewer has a searcher of its own (`scmd_view::search`), but it answers a different
//! question: one code page, a seekable [`Source`](scmd_view::Source), and "where is the next
//! hit so I can scroll to it". A search over a tree gets one forward pass per file, no seek,
//! several code pages at once, and only wants to know *whether* the file qualifies. So the
//! scan lives here, and the two never have to agree on a buffer size.

use std::io::{self, Read};

use memchr::memmem;
use scmd_core::CancelToken;
use scmd_view::Codepage;

/// How much of a file is in memory at a time. Fixed, and never derived from the length the
/// file claims: a 40 GB log and a 40-byte note cost the same buffer.
pub(crate) const CHUNK: usize = 64 * 1024;

/// Everything that can be wrong with a content condition as typed. Every variant names the
/// piece it choked on, because these messages go straight into the dialog.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    #[error("the search text is empty")]
    Empty,

    #[error("`{group}` is half a byte: a hex pattern takes two digits per byte, as in `4d 5a 90`")]
    HexOddDigits { group: String },

    #[error("`{0}` is not a hex digit: a hex pattern takes 0-9 and a-f, as in `4d 5a 90`")]
    HexDigit(char),

    #[error("no code page was chosen to search in")]
    NoCodepage,

    #[error("`{text}` cannot be written in {codepage}, so no file in it could contain the text")]
    NotInCodepage {
        text: String,
        codepage: &'static str,
    },
}

/// What to look for inside a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Needle {
    Text {
        text: String,
        case_sensitive: bool,
        /// A non-word character, or the edge of the searched region, on each side.
        whole_words: bool,
        /// The code pages to encode the text in and try. FAR offers one, a chosen set, or
        /// all of them; [`Codepage::ALL`] is "all of them".
        codepages: Vec<Codepage>,
    },
    /// Raw bytes, as typed in hex. Built with [`Needle::hex`] so a bad pattern is refused
    /// where the user can still see what they typed.
    Hex(Vec<u8>),
}

impl Needle {
    /// A plain text needle: case-insensitive, whole file, UTF-8 only — the state FAR's
    /// dialog opens in. The builders below switch the rest on.
    pub fn text(text: impl Into<String>) -> Needle {
        Needle::Text {
            text: text.into(),
            case_sensitive: false,
            whole_words: false,
            codepages: vec![Codepage::Utf8],
        }
    }

    pub fn case_sensitive(self, on: bool) -> Needle {
        match self {
            Needle::Text {
                text,
                whole_words,
                codepages,
                ..
            } => Needle::Text {
                text,
                case_sensitive: on,
                whole_words,
                codepages,
            },
            hex => hex,
        }
    }

    pub fn whole_words(self, on: bool) -> Needle {
        match self {
            Needle::Text {
                text,
                case_sensitive,
                codepages,
                ..
            } => Needle::Text {
                text,
                case_sensitive,
                whole_words: on,
                codepages,
            },
            hex => hex,
        }
    }

    /// Search in exactly these code pages, in this order. `Codepage::ALL` is FAR's "all code
    /// pages"; a hit says which one it was found in.
    pub fn in_codepages(self, codepages: impl Into<Vec<Codepage>>) -> Needle {
        match self {
            Needle::Text {
                text,
                case_sensitive,
                whole_words,
                ..
            } => Needle::Text {
                text,
                case_sensitive,
                whole_words,
                codepages: codepages.into(),
            },
            hex => hex,
        }
    }

    /// Parse `4d 5a 90` or `4d5a90` into bytes.
    ///
    /// Spaces group the digits, as FAR's dialog has them, so a group is one or more whole
    /// bytes. Two digits make a byte, which is why a group with an odd count — `4d 5`, or a
    /// bare `4d5` — is a byte the user has not finished typing rather than a nibble we are
    /// free to invent a value for.
    pub fn hex(pattern: &str) -> Result<Needle, ContentError> {
        let mut out: Vec<u8> = Vec::new();
        for group in pattern.split_whitespace() {
            let digits: Vec<char> = group.chars().collect();
            if let Some(bad) = digits.iter().find(|c| !c.is_ascii_hexdigit()) {
                return Err(ContentError::HexDigit(*bad));
            }
            if !digits.len().is_multiple_of(2) {
                return Err(ContentError::HexOddDigits {
                    group: group.to_string(),
                });
            }
            for pair in digits.chunks(2) {
                let hi = pair[0].to_digit(16).expect("checked above") as u8;
                let lo = pair[1].to_digit(16).expect("checked above") as u8;
                out.push(hi << 4 | lo);
            }
        }
        if out.is_empty() {
            return Err(ContentError::Empty);
        }
        Ok(Needle::Hex(out))
    }
}

/// The whole content condition: the needle, FAR's "not containing" inversion, and FAR's
/// advanced-page limit of "look no further than this far into each file".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Content {
    pub needle: Needle,
    /// A file is a hit when the needle is **absent**. Deciding that costs the whole file:
    /// there is no early answer, which is why the limit below exists.
    pub not_containing: bool,
    /// Only the first N bytes of each file are searched. `None` reads to the end.
    pub limit: Option<u64>,
}

impl Content {
    /// Containing this needle, whole file.
    pub fn containing(needle: Needle) -> Content {
        Content {
            needle,
            not_containing: false,
            limit: None,
        }
    }

    /// Not containing this needle, whole file.
    pub fn not_containing(needle: Needle) -> Content {
        Content {
            needle,
            not_containing: true,
            limit: None,
        }
    }

    pub fn within(mut self, bytes: u64) -> Content {
        self.limit = Some(bytes);
        self
    }
}

/// How a scan ended. Not an `Option`, because "the user pressed `Esc` halfway through" is a
/// third answer and must not read as "this file does not contain it".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scanned {
    Found {
        offset: u64,
        codepage: Option<Codepage>,
    },
    Absent,
    Cancelled,
}

/// One compiled needle: the bytes as the file would hold them, and how to compare them.
#[derive(Debug, Clone)]
struct Pattern {
    /// The needle, already folded when `fold` is set, so the comparison is a plain memcmp.
    bytes: Vec<u8>,
    /// Which code page these bytes are the needle in; `None` for a hex pattern.
    codepage: Option<Codepage>,
    fold: Option<Fold>,
    whole_words: bool,
    /// Bytes per code unit: 2 for UTF-16, 1 for everything else.
    unit: usize,
}

/// Case folding, done **in the code page being searched** rather than by decoding to UTF-8
/// first: `É` is one byte in CP1252 and two in UTF-8, so a byte-for-byte search of a CP1252
/// file has to fold CP1252's own `0xC9` to its own `0xE9`.
#[derive(Clone)]
enum Fold {
    /// One byte in, one byte out: the code page's own lower-casing table.
    Bytes(Box<[u8; 256]>),
    /// UTF-16 code units, in the endianness of the code page. A table again, and not a call
    /// into the Unicode tables per code unit: that call is two bytes of a file's worth of
    /// work, and a search reads gigabytes.
    Units {
        table: Box<[u16; 0x10000]>,
        big_endian: bool,
    },
}

impl Fold {
    fn for_codepage(cp: Codepage) -> Fold {
        match cp {
            Codepage::Utf16Le => Fold::Units {
                table: unit_table(),
                big_endian: false,
            },
            Codepage::Utf16Be => Fold::Units {
                table: unit_table(),
                big_endian: true,
            },
            other => Fold::Bytes(byte_table(other)),
        }
    }

    fn apply(&self, src: &[u8], dst: &mut Vec<u8>) {
        dst.clear();
        match self {
            Fold::Bytes(table) => dst.extend(src.iter().map(|b| table[*b as usize])),
            Fold::Units { table, big_endian } => {
                for pair in src.chunks(2) {
                    if pair.len() < 2 {
                        // A half unit at the very end of the file: nothing to fold it as.
                        dst.push(pair[0]);
                        continue;
                    }
                    let raw = if *big_endian {
                        u16::from_be_bytes([pair[0], pair[1]])
                    } else {
                        u16::from_le_bytes([pair[0], pair[1]])
                    };
                    let folded = table[raw as usize];
                    dst.extend_from_slice(&if *big_endian {
                        folded.to_be_bytes()
                    } else {
                        folded.to_le_bytes()
                    });
                }
            }
        }
    }

    fn folded(&self, src: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(src.len());
        self.apply(src, &mut out);
        out
    }
}

/// Printed by name: the tables are 256 bytes and 128 KiB, and neither belongs in a panic.
impl std::fmt::Debug for Fold {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Fold::Bytes(_) => f.write_str("Fold::Bytes"),
            Fold::Units { big_endian, .. } => {
                write!(f, "Fold::Units(big_endian: {big_endian})")
            }
        }
    }
}

/// The lower-casing table for UTF-16 code units: 128 KiB, built once per search and shared
/// by every file in it.
fn unit_table() -> Box<[u16; 0x10000]> {
    let mut table = Box::new([0u16; 0x10000]);
    for (unit, slot) in table.iter_mut().enumerate() {
        *slot = fold_unit(unit as u16);
    }
    table
}

/// Lower-case one UTF-16 code unit, when that is a single unit again. A surrogate half, and
/// anything whose lower case is longer than one unit, is left as it is.
fn fold_unit(raw: u16) -> u16 {
    let Some(c) = char::from_u32(raw as u32) else {
        return raw;
    };
    let mut lower = c.to_lowercase();
    let Some(first) = lower.next() else {
        return raw;
    };
    if lower.next().is_some() {
        return raw;
    }
    u16::try_from(first as u32).unwrap_or(raw)
}

/// A code page's lower-casing table, built from the code page itself.
///
/// Only a byte that survives a decode-and-encode round trip has a meaning in this code page
/// worth folding: CP437 decodes `0x00` to a space, and NUL must not come out equal to a
/// space. For UTF-8 the round trip holds for ASCII and fails for every high byte, which is
/// exactly right — a high byte there is half a character, and [`compile`] covers the
/// non-ASCII case by also trying the text's own lower- and upper-case spellings.
fn byte_table(cp: Codepage) -> Box<[u8; 256]> {
    let mut table = [0u8; 256];
    for (index, slot) in table.iter_mut().enumerate() {
        let byte = index as u8;
        *slot = byte;
        let decoded = cp.decode(&[byte]);
        if cp.encode(&decoded) != [byte] {
            continue;
        }
        let mut chars = decoded.chars();
        let (Some(c), None) = (chars.next(), chars.next()) else {
            continue;
        };
        let mut lower = c.to_lowercase();
        let (Some(l), None) = (lower.next(), lower.next()) else {
            continue;
        };
        let encoded = cp.encode(&l.to_string());
        if let [single] = encoded[..] {
            *slot = single;
        }
    }
    Box::new(table)
}

/// A compiled content condition, ready to be run over one file after another.
#[derive(Debug, Clone)]
pub(crate) struct Scanner {
    patterns: Vec<Pattern>,
    not_containing: bool,
    limit: Option<u64>,
    /// Bytes carried from one window into the next. See [`Scanner::scan`].
    carry: usize,
    chunk: usize,
}

impl Scanner {
    pub(crate) fn compile(content: &Content) -> Result<Scanner, ContentError> {
        let patterns = match &content.needle {
            Needle::Hex(bytes) => {
                if bytes.is_empty() {
                    return Err(ContentError::Empty);
                }
                vec![Pattern {
                    bytes: bytes.clone(),
                    codepage: None,
                    fold: None,
                    whole_words: false,
                    unit: 1,
                }]
            }
            Needle::Text {
                text,
                case_sensitive,
                whole_words,
                codepages,
            } => text_patterns(text, *case_sensitive, *whole_words, codepages)?,
        };

        // The window carries the longest needle plus one byte: one byte so that a match
        // which ended exactly at the old window's edge has its right-hand neighbour here,
        // and the needle's length so the match itself is whole. See `scan`.
        let carry = patterns
            .iter()
            .map(|p| p.bytes.len())
            .max()
            .unwrap_or(1)
            .saturating_add(1);
        // Rounded even, so a UTF-16 code unit is never split across two windows and the
        // offset of the new data stays a multiple of the unit.
        let carry = carry + carry % 2;
        let chunk = CHUNK.max(carry);

        Ok(Scanner {
            patterns,
            not_containing: content.not_containing,
            limit: content.limit,
            carry,
            chunk: chunk + chunk % 2,
        })
    }

    /// How many bytes are in memory at once: one chunk plus the overlap.
    pub(crate) fn window(&self) -> usize {
        self.chunk + self.carry
    }

    /// Does this file qualify? `Scanned::Cancelled` is never a yes.
    pub(crate) fn decides(&self, scanned: &Scanned) -> bool {
        match scanned {
            Scanned::Found { .. } => !self.not_containing,
            Scanned::Absent => self.not_containing,
            Scanned::Cancelled => false,
        }
    }

    /// Read `reader` in fixed windows, looking for any of the patterns.
    ///
    /// Windows overlap by [`Scanner::carry`] bytes so that a needle straddling the boundary
    /// is still whole in the next one. `bytes_read` is added to, not reset, because the
    /// caller is counting a whole tree.
    ///
    /// The scan stops at the first match, which is also the right thing for a
    /// `not_containing` condition — one match already settles it. When there is no match the
    /// whole file (or the whole limit) is read, because nothing less can prove an absence.
    pub(crate) fn scan(
        &self,
        reader: &mut dyn Read,
        cancel: &CancelToken,
        bytes_read: &mut u64,
    ) -> io::Result<Scanned> {
        let mut buf = vec![0u8; self.window()];
        let mut folded: Vec<u8> = Vec::with_capacity(buf.len());
        let mut filled = 0usize;
        // File offset of `buf[0]`, so a reported offset is an offset into the file.
        let mut base = 0u64;
        let mut taken = 0u64;

        loop {
            if cancel.is_cancelled() {
                return Ok(Scanned::Cancelled);
            }

            let mut eof = false;
            while filled < buf.len() {
                let room = match self.limit {
                    // The limit is a promise not to read further, so it caps the request
                    // itself: a file larger than the limit costs exactly the limit.
                    Some(limit) => {
                        let left = limit.saturating_sub(taken);
                        if left == 0 {
                            eof = true;
                            break;
                        }
                        (buf.len() - filled).min(left as usize)
                    }
                    None => buf.len() - filled,
                };
                match reader.read(&mut buf[filled..filled + room]) {
                    Ok(0) => {
                        eof = true;
                        break;
                    }
                    Ok(n) => {
                        filled += n;
                        taken += n as u64;
                        *bytes_read += n as u64;
                    }
                    Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                }
            }

            if let Some(found) = self.search(&buf[..filled], &mut folded, base, eof) {
                return Ok(found);
            }
            if eof {
                return Ok(Scanned::Absent);
            }

            let keep = self.carry.min(filled);
            buf.copy_within(filled - keep..filled, 0);
            base += (filled - keep) as u64;
            filled = keep;
        }
    }

    /// Look for every pattern in one window. `base` is the window's file offset and `eof`
    /// says whether the searched region ends here.
    fn search(&self, window: &[u8], folded: &mut Vec<u8>, base: u64, eof: bool) -> Option<Scanned> {
        for pattern in &self.patterns {
            let hay: &[u8] = match &pattern.fold {
                Some(fold) => {
                    fold.apply(window, folded);
                    folded.as_slice()
                }
                None => window,
            };
            for at in memmem::find_iter(hay, &pattern.bytes) {
                // A match at the very start of a carried-over window was already looked at
                // in the window before, where it was not at the start.
                if at == 0 && base > 0 {
                    continue;
                }
                if pattern.unit > 1 && !(base + at as u64).is_multiple_of(pattern.unit as u64) {
                    continue;
                }
                let end = at + pattern.bytes.len();
                if pattern.whole_words {
                    if at >= pattern.unit && word_byte(hay, at - pattern.unit, pattern) {
                        continue;
                    }
                    if end + pattern.unit <= hay.len() {
                        if word_byte(hay, end, pattern) {
                            continue;
                        }
                    } else if !eof {
                        // The neighbour on the right is in the next window; the carry brings
                        // this match back with room to judge it.
                        continue;
                    }
                }
                return Some(Scanned::Found {
                    offset: base + at as u64,
                    codepage: pattern.codepage,
                });
            }
        }
        None
    }
}

/// Is the code unit at `at` part of a word? Letters, digits and `_`, as every editor's
/// whole-word search has it.
fn word_byte(hay: &[u8], at: usize, pattern: &Pattern) -> bool {
    if pattern.unit == 2 {
        if at + 2 > hay.len() {
            return false;
        }
        let raw = match pattern.codepage {
            Some(Codepage::Utf16Be) => u16::from_be_bytes([hay[at], hay[at + 1]]),
            _ => u16::from_le_bytes([hay[at], hay[at + 1]]),
        };
        return char::from_u32(raw as u32).is_some_and(is_word_char);
    }
    let byte = hay[at];
    match pattern.codepage {
        // In UTF-8 every byte with the high bit set belongs to a non-ASCII character, and the
        // boundary question is about the character, not the byte it starts or continues.
        Some(Codepage::Utf8) | None => byte >= 0x80 || is_word_char(byte as char),
        Some(cp) => cp.decode(&[byte]).chars().next().is_some_and(is_word_char),
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// One pattern per code page, plus — for UTF-8 only — the text's own lower- and upper-case
/// spellings.
///
/// A single-byte code page and UTF-16 fold whole characters in one code unit, so the fold
/// table settles case there. UTF-8 cannot: `É` is two bytes and `é` is two different ones,
/// and a byte table cannot turn one into the other without changing the length. Searching
/// for the text as typed, all-lower and all-upper, with ASCII folding on top of each, covers
/// every spelling whose non-ASCII letters share one case — which is every spelling anyone
/// types.
fn text_patterns(
    text: &str,
    case_sensitive: bool,
    whole_words: bool,
    codepages: &[Codepage],
) -> Result<Vec<Pattern>, ContentError> {
    if text.is_empty() {
        return Err(ContentError::Empty);
    }
    if codepages.is_empty() {
        return Err(ContentError::NoCodepage);
    }

    let mut out: Vec<Pattern> = Vec::new();
    let mut unusable: Option<&'static str> = None;
    for cp in codepages {
        if out.iter().any(|p| p.codepage == Some(*cp)) {
            continue;
        }
        // A code page that cannot spell the text cannot hold it either, and must be dropped
        // rather than searched for the `?` that `encode` would fall back to.
        if cp.decode(&cp.encode(text)) != text {
            unusable = unusable.or(Some(cp.name()));
            continue;
        }

        let fold = (!case_sensitive).then(|| Fold::for_codepage(*cp));
        let mut spellings = vec![text.to_string()];
        if !case_sensitive && *cp == Codepage::Utf8 && !text.is_ascii() {
            for variant in [text.to_lowercase(), text.to_uppercase()] {
                if !spellings.contains(&variant) {
                    spellings.push(variant);
                }
            }
        }
        for spelling in spellings {
            let encoded = cp.encode(&spelling);
            let bytes = match &fold {
                Some(fold) => fold.folded(&encoded),
                None => encoded,
            };
            // Two code pages that spell the text in the same bytes are one pattern, not two:
            // an ASCII needle is the same bytes in UTF-8, CP1252 and CP437, and "all code
            // pages" would otherwise fold and scan every window three times over for one
            // answer. The hit then names the first of those code pages, which for an ASCII
            // needle is as true as any of them.
            if bytes.is_empty() || out.iter().any(|p| p.bytes == bytes) {
                continue;
            }
            out.push(Pattern {
                bytes,
                codepage: Some(*cp),
                fold: fold.clone(),
                whole_words,
                unit: cp.unit(),
            });
        }
    }

    if out.is_empty() {
        return Err(ContentError::NotInCodepage {
            text: text.to_string(),
            codepage: unusable.unwrap_or("any of the chosen code pages"),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(content: &Content, bytes: &[u8]) -> (Scanned, u64) {
        let scanner = Scanner::compile(content).unwrap();
        let mut read = 0u64;
        let mut cursor = bytes;
        let scanned = scanner
            .scan(&mut cursor, &CancelToken::new(), &mut read)
            .unwrap();
        (scanned, read)
    }

    fn found_at(content: &Content, bytes: &[u8]) -> Option<u64> {
        match scan(content, bytes).0 {
            Scanned::Found { offset, .. } => Some(offset),
            _ => None,
        }
    }

    #[test]
    fn a_needle_spanning_a_chunk_boundary_is_still_found() {
        let content = Content::containing(Needle::text("NEEDLE").case_sensitive(true));
        // Straddle the first window's edge by three bytes: half the needle is in one window
        // and half in the next, so only the overlap can find it.
        let edge = Scanner::compile(&content).unwrap().window();
        let mut bytes = vec![b'.'; edge - 3];
        bytes.extend_from_slice(b"NEEDLE");
        bytes.extend(std::iter::repeat_n(b'.', 4 * CHUNK));
        assert_eq!(found_at(&content, &bytes), Some((edge - 3) as u64));
    }

    #[test]
    fn a_needle_spanning_a_chunk_boundary_is_found_with_whole_words_too() {
        let content = Content::containing(Needle::text("word").whole_words(true));
        // The word ends exactly on the window's edge, so the character that decides its
        // right-hand boundary is only in the next window.
        let edge = Scanner::compile(&content).unwrap().window();
        let mut bytes = vec![b' '; edge - 4];
        bytes.extend_from_slice(b"word");
        bytes.extend_from_slice(b" tail");
        assert_eq!(found_at(&content, &bytes), Some((edge - 4) as u64));
    }

    #[test]
    fn whole_words_wants_a_non_word_character_or_an_edge_on_each_side() {
        let content = Content::containing(Needle::text("cat").whole_words(true));
        assert_eq!(found_at(&content, b"a cat sat"), Some(2));
        assert_eq!(found_at(&content, b"cat"), Some(0));
        assert_eq!(found_at(&content, b"(cat)"), Some(1));
        assert_eq!(found_at(&content, b"dog cat"), Some(4));
        assert_eq!(found_at(&content, b"concatenate"), None);
        assert_eq!(found_at(&content, b"cats"), None);
        assert_eq!(found_at(&content, b"bobcat"), None);
        assert_eq!(found_at(&content, b"cat_alogue"), None);
        assert_eq!(found_at(&content, b"cat9"), None);
        // A non-ASCII letter is a word character, whichever byte of it we land on.
        assert_eq!(found_at(&content, "ècat".as_bytes()), None);
        assert_eq!(found_at(&content, "catè".as_bytes()), None);
    }

    #[test]
    fn whole_words_off_matches_inside_a_longer_word() {
        let content = Content::containing(Needle::text("cat"));
        assert_eq!(found_at(&content, b"concatenate"), Some(3));
    }

    #[test]
    fn a_case_insensitive_search_in_a_non_utf8_code_page_compares_in_that_code_page() {
        // `caf\xe9` is CP1252 for "café"; `CAF\xc9` is the same word in upper case. Folding
        // in UTF-8 could not relate the two: there `É` is 0xC3 0x89 and `é` is 0xC3 0xA9.
        let content = Content::containing(Needle::text("café").in_codepages([Codepage::Cp1252]));
        assert_eq!(found_at(&content, b"le CAF\xc9 noir"), Some(3));
        assert_eq!(found_at(&content, b"le caf\xe9 noir"), Some(3));
        assert_eq!(found_at(&content, b"le Caf\xc9 noir"), Some(3));
        // The same text as UTF-8 bytes is simply not in a CP1252 file.
        assert_eq!(found_at(&content, "le café noir".as_bytes()), None);

        let sensitive = Content::containing(
            Needle::text("café")
                .case_sensitive(true)
                .in_codepages([Codepage::Cp1252]),
        );
        assert_eq!(found_at(&sensitive, b"le caf\xe9 noir"), Some(3));
        assert_eq!(found_at(&sensitive, b"le CAF\xc9 noir"), None);
    }

    #[test]
    fn a_case_insensitive_utf8_search_folds_non_ascii_letters_as_well() {
        let content = Content::containing(Needle::text("Café"));
        assert_eq!(found_at(&content, "le café noir".as_bytes()), Some(3));
        assert_eq!(found_at(&content, "le CAFÉ noir".as_bytes()), Some(3));
        assert_eq!(found_at(&content, "le cAfé noir".as_bytes()), Some(3));
    }

    #[test]
    fn a_utf16_search_folds_whole_code_units_and_only_matches_on_a_unit_boundary() {
        let content = Content::containing(Needle::text("Straße").in_codepages([Codepage::Utf16Le]));
        let mut bytes = Codepage::Utf16Le.encode("in der STRASSE");
        bytes.extend_from_slice(&Codepage::Utf16Le.encode(" und STRAßE hier"));
        assert_eq!(found_at(&content, &bytes), Some(2 * 19));

        let big = Content::containing(Needle::text("straße").in_codepages([Codepage::Utf16Be]));
        assert_eq!(
            found_at(&big, &Codepage::Utf16Be.encode("die STRAßE")),
            Some(8)
        );
        // Big-endian bytes are not little-endian ones.
        assert_eq!(
            found_at(&content, &Codepage::Utf16Be.encode("STRAßE")),
            None
        );
    }

    #[test]
    fn all_code_pages_reports_which_one_the_hit_was_in() {
        let content =
            Content::containing(Needle::text("café").in_codepages(Codepage::ALL.to_vec()));
        let scanner = Scanner::compile(&content).unwrap();
        for (bytes, expected) in [
            ("x café".as_bytes().to_vec(), Codepage::Utf8),
            (b"x caf\xe9".to_vec(), Codepage::Cp1252),
            (Codepage::Utf16Le.encode("x café"), Codepage::Utf16Le),
        ] {
            let mut read = 0u64;
            let mut cursor = bytes.as_slice();
            let scanned = scanner
                .scan(&mut cursor, &CancelToken::new(), &mut read)
                .unwrap();
            match scanned {
                Scanned::Found { codepage, .. } => assert_eq!(codepage, Some(expected)),
                other => panic!("{other:?} in {expected:?}"),
            }
        }
    }

    #[test]
    fn a_code_page_that_cannot_spell_the_text_is_dropped_rather_than_searched() {
        // CP437 has no `日`, and `encode` would fall back to `?`: a file full of question
        // marks must not come back as a hit.
        let content = Content::containing(
            Needle::text("日本").in_codepages([Codepage::Cp437, Codepage::Utf8]),
        );
        assert_eq!(found_at(&content, b"???? ??"), None);
        assert_eq!(found_at(&content, "a 日本 b".as_bytes()), Some(2));

        let only = Content::containing(Needle::text("日本").in_codepages([Codepage::Cp437]));
        let err = Scanner::compile(&only).unwrap_err();
        assert!(err.to_string().contains("CP437"), "{err}");
    }

    #[test]
    fn a_hex_pattern_with_an_odd_number_of_digits_is_an_error_with_a_usable_message() {
        let err = Needle::hex("4d5").unwrap_err();
        assert_eq!(
            err.to_string(),
            "`4d5` is half a byte: a hex pattern takes two digits per byte, as in `4d 5a 90`"
        );
        let grouped = Needle::hex("4d 5").unwrap_err();
        assert!(grouped.to_string().contains("`5`"), "{grouped}");
    }

    #[test]
    fn a_hex_pattern_with_a_character_that_is_not_a_hex_digit_says_which_one() {
        let err = Needle::hex("4d zz").unwrap_err();
        assert_eq!(
            err.to_string(),
            "`z` is not a hex digit: a hex pattern takes 0-9 and a-f, as in `4d 5a 90`"
        );
        assert_eq!(
            Needle::hex("4d-5a").unwrap_err(),
            ContentError::HexDigit('-')
        );
    }

    #[test]
    fn a_hex_pattern_is_grouped_by_spaces_or_not_at_all() {
        assert_eq!(
            Needle::hex("4d 5a 90").unwrap(),
            Needle::Hex(vec![0x4d, 0x5a, 0x90])
        );
        assert_eq!(
            Needle::hex("4D5A90").unwrap(),
            Needle::hex("4d 5a90").unwrap()
        );
        assert_eq!(Needle::hex("   ").unwrap_err(), ContentError::Empty);
    }

    #[test]
    fn a_hex_needle_matches_bytes_no_text_search_would_reach() {
        let content = Content::containing(Needle::hex("00 4d 5a").unwrap());
        assert_eq!(found_at(&content, &[0xff, 0x00, 0x4d, 0x5a, 0x90]), Some(1));
        assert_eq!(found_at(&content, &[0x00, 0x4d, 0x5b]), None);
    }

    #[test]
    fn an_empty_text_needle_is_refused_and_so_is_an_empty_set_of_code_pages() {
        let empty = Content::containing(Needle::text(""));
        assert_eq!(Scanner::compile(&empty).unwrap_err(), ContentError::Empty);
        let nowhere = Content::containing(Needle::text("x").in_codepages([]));
        assert_eq!(
            Scanner::compile(&nowhere).unwrap_err(),
            ContentError::NoCodepage
        );
    }

    #[test]
    fn not_containing_reads_the_whole_file_before_deciding() {
        let bytes = vec![b'.'; 5 * CHUNK + 17];
        let content = Content::not_containing(Needle::text("absent"));
        let (scanned, read) = scan(&content, &bytes);
        assert_eq!(scanned, Scanned::Absent);
        assert!(content_decides(&content, &scanned));
        assert_eq!(read, bytes.len() as u64, "an absence needs every byte");
    }

    #[test]
    fn not_containing_reads_the_whole_limit_before_deciding() {
        let bytes = vec![b'.'; 5 * CHUNK];
        let content = Content::not_containing(Needle::text("absent")).within(1000);
        let (scanned, read) = scan(&content, &bytes);
        assert_eq!(scanned, Scanned::Absent);
        assert_eq!(read, 1000);
    }

    #[test]
    fn not_containing_stops_at_the_first_match_because_one_already_settles_it() {
        let mut bytes = b"here it is".to_vec();
        bytes.extend(std::iter::repeat_n(b'.', 5 * CHUNK));
        let content = Content::not_containing(Needle::text("here"));
        let (scanned, read) = scan(&content, &bytes);
        assert!(matches!(scanned, Scanned::Found { .. }));
        assert!(!content_decides(&content, &scanned));
        assert!(read < bytes.len() as u64, "{read}");
    }

    #[test]
    fn a_file_larger_than_the_limit_reads_only_the_limit() {
        let mut bytes = vec![b'.'; 4 * CHUNK];
        bytes.extend_from_slice(b"NEEDLE");
        let content = Content::containing(Needle::text("NEEDLE")).within(2 * CHUNK as u64);
        let (scanned, read) = scan(&content, &bytes);
        assert_eq!(scanned, Scanned::Absent, "the needle is past the limit");
        assert_eq!(read, 2 * CHUNK as u64);

        // Inside the limit it is found, and not one byte more is read than the match needs.
        let generous = Content::containing(Needle::text("NEEDLE")).within(5 * CHUNK as u64);
        let (scanned, read) = scan(&generous, &bytes);
        assert!(matches!(scanned, Scanned::Found { .. }));
        assert!(read <= bytes.len() as u64, "{read}");
    }

    #[test]
    fn a_cancelled_scan_is_neither_a_hit_nor_an_absence() {
        let bytes = vec![b'.'; 4 * CHUNK];
        let content = Content::not_containing(Needle::text("absent"));
        let scanner = Scanner::compile(&content).unwrap();
        let token = CancelToken::new();
        token.cancel();
        let mut read = 0u64;
        let mut cursor = bytes.as_slice();
        let scanned = scanner.scan(&mut cursor, &token, &mut read).unwrap();
        assert_eq!(scanned, Scanned::Cancelled);
        assert!(
            !scanner.decides(&scanned),
            "a cancelled scan is never a hit"
        );
    }

    #[test]
    fn a_read_error_is_handed_back_rather_than_read_as_an_absence() {
        struct Broken;
        impl Read for Broken {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::PermissionDenied, "nope"))
            }
        }
        let content = Content::not_containing(Needle::text("absent"));
        let scanner = Scanner::compile(&content).unwrap();
        let mut read = 0u64;
        let err = scanner
            .scan(&mut Broken, &CancelToken::new(), &mut read)
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    }

    #[test]
    fn an_empty_file_contains_nothing_and_costs_no_bytes() {
        let content = Content::containing(Needle::text("x"));
        assert_eq!(scan(&content, b""), (Scanned::Absent, 0));
    }

    #[test]
    fn the_buffer_is_sized_by_the_needle_and_the_chunk_never_by_the_file() {
        let small = Scanner::compile(&Content::containing(Needle::text("x"))).unwrap();
        assert_eq!(small.chunk, CHUNK);
        assert_eq!(
            small.carry, 2,
            "one byte of needle, one of right-hand context"
        );
        assert_eq!(small.window(), CHUNK + 2);

        // A needle longer than a window grows the window; nothing else ever does.
        let long = "z".repeat(CHUNK * 2);
        let big = Scanner::compile(&Content::containing(Needle::text(&long))).unwrap();
        assert!(big.chunk >= long.len());
    }

    fn content_decides(content: &Content, scanned: &Scanned) -> bool {
        Scanner::compile(content).unwrap().decides(scanned)
    }
}
