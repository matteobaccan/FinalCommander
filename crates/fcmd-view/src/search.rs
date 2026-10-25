use memchr::memmem;
use regex::bytes::{Regex, RegexBuilder};

use crate::decode::Codepage;
use crate::source::Source;

/// How the query is interpreted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    #[default]
    Plain,
    Regex,
    /// `4D 5A ?? 00` — bytes, with `??` matching anything.
    Hex,
}

impl SearchMode {
    pub fn as_str(self) -> &'static str {
        match self {
            SearchMode::Plain => "text",
            SearchMode::Regex => "regex",
            SearchMode::Hex => "hex",
        }
    }

    pub fn next(self) -> SearchMode {
        match self {
            SearchMode::Plain => SearchMode::Regex,
            SearchMode::Regex => SearchMode::Hex,
            SearchMode::Hex => SearchMode::Plain,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("empty search")]
    Empty,
    #[error("invalid regular expression: {0}")]
    Regex(String),
    #[error("invalid hex pattern: {0}")]
    Hex(String),
}

/// A compiled query. Searching happens on **bytes**, so match offsets are exact whatever the
/// file's encoding is.
#[derive(Debug, Clone)]
pub enum Pattern {
    Literal(Vec<u8>),
    /// Bytes with holes: `None` matches any byte.
    Masked(Vec<Option<u8>>),
    Regex(Regex),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub offset: u64,
    pub len: usize,
}

/// Compile a query. `codepage` is used so that searching for `café` finds it in a CP1252 file.
pub fn compile(
    query: &str,
    mode: SearchMode,
    case_insensitive: bool,
    codepage: Codepage,
) -> Result<Pattern, SearchError> {
    compile_with(query, mode, case_insensitive, false, codepage)
}

/// The same, with FAR's *whole words* as well: a hit has to have a word boundary on both
/// sides, which is what makes searching for `set` in a file full of `offset` bearable.
///
/// It means nothing for a hex pattern — bytes have no words — so there it is ignored.
pub fn compile_with(
    query: &str,
    mode: SearchMode,
    case_insensitive: bool,
    whole_words: bool,
    codepage: Codepage,
) -> Result<Pattern, SearchError> {
    if query.trim().is_empty() {
        return Err(SearchError::Empty);
    }
    match mode {
        SearchMode::Hex => {
            let mut out = Vec::new();
            for token in query.split_whitespace() {
                for pair in token.as_bytes().chunks(2) {
                    let text =
                        std::str::from_utf8(pair).map_err(|e| SearchError::Hex(e.to_string()))?;
                    if text == "??" {
                        out.push(None);
                        continue;
                    }
                    let byte = u8::from_str_radix(text, 16)
                        .map_err(|_| SearchError::Hex(format!("`{text}` is not a hex byte")))?;
                    out.push(Some(byte));
                }
            }
            if out.is_empty() {
                return Err(SearchError::Empty);
            }
            if out.iter().all(|b| b.is_some()) {
                Ok(Pattern::Literal(
                    out.into_iter().map(|b| b.expect("checked")).collect(),
                ))
            } else {
                Ok(Pattern::Masked(out))
            }
        }
        SearchMode::Regex => {
            let pattern = if whole_words {
                format!(r"\b(?:{query})\b")
            } else {
                query.to_string()
            };
            let re = RegexBuilder::new(&pattern)
                .case_insensitive(case_insensitive)
                .build()
                .map_err(|e| SearchError::Regex(e.to_string()))?;
            Ok(Pattern::Regex(re))
        }
        SearchMode::Plain if case_insensitive || whole_words => {
            let escaped = regex::escape(query);
            let pattern = if whole_words {
                format!(r"\b{escaped}\b")
            } else {
                escaped
            };
            let re = RegexBuilder::new(&pattern)
                .case_insensitive(case_insensitive)
                .build()
                .map_err(|e| SearchError::Regex(e.to_string()))?;
            Ok(Pattern::Regex(re))
        }
        SearchMode::Plain => Ok(Pattern::Literal(codepage.encode(query))),
    }
}

impl Pattern {
    /// Longest possible match, used to size the overlap between scan windows.
    fn span(&self) -> usize {
        match self {
            Pattern::Literal(bytes) => bytes.len(),
            Pattern::Masked(mask) => mask.len(),
            // A regex can match any length; this is the overlap we are willing to pay for.
            Pattern::Regex(_) => 4096,
        }
    }

    /// First match at or after `from` inside `window`.
    fn find_in(&self, window: &[u8], from: usize) -> Option<Match> {
        let hay = &window[from.min(window.len())..];
        match self {
            Pattern::Literal(needle) => memmem::find(hay, needle).map(|pos| Match {
                offset: (from + pos) as u64,
                len: needle.len(),
            }),
            Pattern::Masked(mask) => find_masked(hay, mask).map(|pos| Match {
                offset: (from + pos) as u64,
                len: mask.len(),
            }),
            Pattern::Regex(re) => re.find(hay).map(|m| Match {
                offset: (from + m.start()) as u64,
                len: m.len(),
            }),
        }
    }

    /// Last match in `window` ending at or before `until`.
    fn rfind_in(&self, window: &[u8], until: usize) -> Option<Match> {
        let hay = &window[..until.min(window.len())];
        match self {
            Pattern::Literal(needle) => memmem::rfind(hay, needle).map(|pos| Match {
                offset: pos as u64,
                len: needle.len(),
            }),
            Pattern::Masked(mask) => {
                let mut last = None;
                let mut at = 0;
                while let Some(pos) = find_masked(&hay[at..], mask) {
                    last = Some(at + pos);
                    at += pos + 1;
                    if at >= hay.len() {
                        break;
                    }
                }
                last.map(|pos| Match {
                    offset: pos as u64,
                    len: mask.len(),
                })
            }
            Pattern::Regex(re) => re.find_iter(hay).last().map(|m| Match {
                offset: m.start() as u64,
                len: m.len(),
            }),
        }
    }

    /// Does this pattern match inside a single rendered row? Used to highlight hits.
    pub fn matches_in(&self, bytes: &[u8]) -> Vec<Match> {
        let mut out = Vec::new();
        let mut at = 0usize;
        while at <= bytes.len() {
            match self.find_in(bytes, at) {
                Some(m) => {
                    let next = m.offset as usize + m.len.max(1);
                    out.push(m);
                    at = next;
                }
                None => break,
            }
        }
        out
    }
}

fn find_masked(hay: &[u8], mask: &[Option<u8>]) -> Option<usize> {
    if mask.len() > hay.len() {
        return None;
    }
    'outer: for start in 0..=hay.len() - mask.len() {
        for (i, expected) in mask.iter().enumerate() {
            if let Some(byte) = expected {
                if hay[start + i] != *byte {
                    continue 'outer;
                }
            }
        }
        return Some(start);
    }
    None
}

/// The scan window: big enough to amortise reads, small enough to stay responsive.
const WINDOW: usize = 256 * 1024;

/// Search forwards (or backwards) from `from`. Returns the match's absolute offset.
///
/// Windows overlap by the pattern's span so a match straddling a window boundary is still
/// found. Cancellation is checked between windows.
pub fn find(
    source: &mut dyn Source,
    pattern: &Pattern,
    from: u64,
    backwards: bool,
    cancel: &fcmd_core::CancelToken,
) -> Option<Match> {
    let len = source.len();
    if len == 0 {
        return None;
    }
    let overlap = pattern.span() as u64;

    if !backwards {
        let mut at = from.min(len);
        while at < len {
            if cancel.is_cancelled() {
                return None;
            }
            let window = source.read_vec(at, WINDOW).ok()?;
            if window.is_empty() {
                return None;
            }
            if let Some(m) = pattern.find_in(&window, 0) {
                return Some(Match {
                    offset: at + m.offset,
                    len: m.len,
                });
            }
            if at + window.len() as u64 >= len {
                return None;
            }
            at += window.len() as u64 - overlap.min(window.len() as u64 - 1);
        }
        None
    } else {
        let mut end = from.min(len);
        while end > 0 {
            if cancel.is_cancelled() {
                return None;
            }
            let size = (WINDOW as u64).min(end);
            let start = end - size;
            let window = source.read_vec(start, size as usize).ok()?;
            if let Some(m) = pattern.rfind_in(&window, window.len()) {
                return Some(Match {
                    offset: start + m.offset,
                    len: m.len,
                });
            }
            if start == 0 {
                return None;
            }
            end = start + overlap.min(size - 1);
            if end >= from.min(len) {
                return None;
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whole words is the difference between finding `set` and finding `offset`.
    #[test]
    fn whole_words_wants_a_boundary_on_both_sides() {
        let hay = b"offset = set + settle";
        let loose = compile("set", SearchMode::Plain, false, Codepage::Utf8).unwrap();
        let tight = compile_with("set", SearchMode::Plain, false, true, Codepage::Utf8).unwrap();
        assert_eq!(loose.find_in(hay, 0).map(|m| m.offset), Some(3));
        assert_eq!(tight.find_in(hay, 0).map(|m| m.offset), Some(9));
        // And it applies to a regular expression too, around the whole of it.
        let re = compile_with("se.", SearchMode::Regex, false, true, Codepage::Utf8).unwrap();
        assert_eq!(re.find_in(hay, 0).map(|m| m.offset), Some(9));
    }
    use crate::source::BytesSource;
    use fcmd_core::CancelToken;

    fn cp() -> Codepage {
        Codepage::Utf8
    }

    #[test]
    fn plain_search_finds_the_first_and_then_the_next() {
        let mut s = BytesSource::new(b"alpha beta alpha gamma".to_vec());
        let p = compile("alpha", SearchMode::Plain, false, cp()).unwrap();
        let first = find(&mut s, &p, 0, false, &CancelToken::new()).unwrap();
        assert_eq!(first, Match { offset: 0, len: 5 });
        let next = find(&mut s, &p, 1, false, &CancelToken::new()).unwrap();
        assert_eq!(next.offset, 11);
        assert!(find(&mut s, &p, 12, false, &CancelToken::new()).is_none());
    }

    #[test]
    fn search_backwards_finds_the_previous_hit() {
        let mut s = BytesSource::new(b"alpha beta alpha gamma".to_vec());
        let p = compile("alpha", SearchMode::Plain, false, cp()).unwrap();
        let back = find(&mut s, &p, 22, true, &CancelToken::new()).unwrap();
        assert_eq!(back.offset, 11);
        let back = find(&mut s, &p, 11, true, &CancelToken::new()).unwrap();
        assert_eq!(back.offset, 0);
        assert!(find(&mut s, &p, 0, true, &CancelToken::new()).is_none());
    }

    #[test]
    fn case_insensitive_and_regex_searches_work_on_bytes() {
        let mut s = BytesSource::new(b"Hello World".to_vec());
        let ci = compile("hello", SearchMode::Plain, true, cp()).unwrap();
        assert_eq!(
            find(&mut s, &ci, 0, false, &CancelToken::new())
                .unwrap()
                .offset,
            0
        );
        let re = compile(r"W\w+d", SearchMode::Regex, false, cp()).unwrap();
        let m = find(&mut s, &re, 0, false, &CancelToken::new()).unwrap();
        assert_eq!((m.offset, m.len), (6, 5));
    }

    #[test]
    fn a_plain_query_is_encoded_in_the_files_codepage() {
        let mut s = BytesSource::new(b"caf\xe9 noir".to_vec());
        let p = compile("café", SearchMode::Plain, false, Codepage::Cp1252).unwrap();
        assert_eq!(
            find(&mut s, &p, 0, false, &CancelToken::new())
                .unwrap()
                .offset,
            0
        );
        // The same query as UTF-8 bytes is simply not in this file.
        let utf8 = compile("café", SearchMode::Plain, false, Codepage::Utf8).unwrap();
        assert!(find(&mut s, &utf8, 0, false, &CancelToken::new()).is_none());
    }

    #[test]
    fn hex_patterns_support_wildcards() {
        let mut s = BytesSource::new(vec![0x00, 0x4d, 0x5a, 0x90, 0x00, 0x03]);
        let exact = compile("4D 5A", SearchMode::Hex, false, cp()).unwrap();
        assert_eq!(
            find(&mut s, &exact, 0, false, &CancelToken::new())
                .unwrap()
                .offset,
            1
        );
        let masked = compile("4d ?? 90", SearchMode::Hex, false, cp()).unwrap();
        assert_eq!(
            find(&mut s, &masked, 0, false, &CancelToken::new())
                .unwrap()
                .offset,
            1
        );
        assert!(compile("zz", SearchMode::Hex, false, cp()).is_err());
        assert!(compile("", SearchMode::Hex, false, cp()).is_err());
    }

    #[test]
    fn a_match_straddling_two_windows_is_still_found() {
        let needle = b"NEEDLE";
        let mut bytes = vec![b'.'; WINDOW - 3];
        bytes.extend_from_slice(needle);
        bytes.extend(std::iter::repeat_n(b'.', 100));
        let mut s = BytesSource::new(bytes);
        let p = compile("NEEDLE", SearchMode::Plain, false, cp()).unwrap();
        let m = find(&mut s, &p, 0, false, &CancelToken::new()).unwrap();
        assert_eq!(m.offset, (WINDOW - 3) as u64);
    }

    #[test]
    fn a_cancelled_search_gives_up() {
        let mut s = BytesSource::new(vec![b'.'; WINDOW * 4]);
        let p = compile("zzz", SearchMode::Plain, false, cp()).unwrap();
        let token = CancelToken::new();
        token.cancel();
        assert!(find(&mut s, &p, 0, false, &token).is_none());
    }

    #[test]
    fn row_highlighting_reports_every_hit() {
        let p = compile("ab", SearchMode::Plain, false, cp()).unwrap();
        let hits = p.matches_in(b"abxab");
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].offset, 0);
        assert_eq!(hits[1].offset, 3);
    }

    #[test]
    fn invalid_regexes_are_reported() {
        let err = compile("(unclosed", SearchMode::Regex, false, cp()).unwrap_err();
        assert!(
            err.to_string().contains("invalid regular expression"),
            "{err}"
        );
    }
}
