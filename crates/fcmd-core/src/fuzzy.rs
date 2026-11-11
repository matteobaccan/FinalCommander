//! Approximate matching of a name: the letters typed, in order, anywhere in it — and a number
//! that says how good the match is, so a list can put the likeliest first.
//!
//! `mnrs` finds `main.rs`, `rdme` finds `README.md`. That is all "fuzzy" means here: a
//! subsequence, never a guess about typing mistakes, so a match can always be explained by
//! pointing at the letters. What makes it useful is the ranking, and the ranking is a handful
//! of preferences anybody would state out loud:
//!
//! - letters next to each other beat letters far apart (`main` in `main.rs` before `main` in
//!   `my_admin`);
//! - a letter that starts a word beats one in the middle of it — the start of the name, after
//!   `_`, `-`, `.`, a space or a path separator, or a capital after a lower-case letter
//!   (`fb` finds `FooBar` and `foo_bar` before `offbeat`);
//! - of two names that match equally well, the shorter is the nearer miss.
//!
//! Several words separated by spaces must each match, in any order: `test app` finds
//! `app_tests.rs`. A pattern with nothing in it matches everything, with nothing to rank by.

/// What every matched letter is worth.
const LETTER: i32 = 16;
/// Added when the letter before it in the pattern matched the letter before it in the name.
const ADJACENT: i32 = 24;
/// Added for the first letter of the name.
const AT_START: i32 = 20;
/// Added for the first letter of a word inside the name.
const AT_WORD: i32 = 18;
/// Taken off for every letter of the name stepped over.
const SKIPPED: i32 = 1;
/// "No match", kept well away from the edge so that adding to it cannot wrap.
const NEVER: i32 = i32::MIN / 2;

/// A pattern, made ready once and asked about many names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fuzzy {
    words: Vec<Vec<char>>,
    case_sensitive: bool,
}

impl Fuzzy {
    /// Read a pattern as typed. `*` and `?` are dropped rather than looked for: the field
    /// this is typed into is a mask field the rest of the time, and `*` left in it from the
    /// last search should mean what it meant there — anything.
    pub fn new(pattern: &str, case_sensitive: bool) -> Fuzzy {
        let words = pattern
            .split_whitespace()
            .map(|word| {
                word.chars()
                    .filter(|c| !matches!(c, '*' | '?'))
                    .flat_map(|c| fold(c, case_sensitive))
                    .collect::<Vec<char>>()
            })
            .filter(|word| !word.is_empty())
            .collect();
        Fuzzy {
            words,
            case_sensitive,
        }
    }

    /// Does the pattern ask for anything at all?
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// How well `text` matches, or `None` when it does not: higher is better, and the
    /// numbers mean nothing except against each other for the same pattern.
    pub fn score(&self, text: &str) -> Option<i32> {
        if self.words.is_empty() {
            return Some(0);
        }
        // The name as written, for the word boundaries, and as compared.
        let shown: Vec<char> = text.chars().collect();
        let folded: Vec<char> = shown
            .iter()
            .map(|&c| fold(c, self.case_sensitive).next().unwrap_or(c))
            .collect();
        let mut total = 0;
        for word in &self.words {
            total += score_word(word, &shown, &folded)?;
        }
        Some(total)
    }
}

/// One call for one name, where there is only one name to ask about.
pub fn score(pattern: &str, text: &str) -> Option<i32> {
    Fuzzy::new(pattern, false).score(text)
}

fn fold(c: char, case_sensitive: bool) -> impl Iterator<Item = char> {
    // One character in, one out: a letter whose lower case is two characters keeps itself,
    // so the positions in the folded name are the positions in the name.
    let mut lower = c.to_lowercase();
    let one = match (case_sensitive, lower.next(), lower.next()) {
        (false, Some(l), None) => l,
        _ => c,
    };
    std::iter::once(one)
}

/// The bonus a letter of the name earns for where it stands.
fn place(shown: &[char], at: usize) -> i32 {
    let Some(before) = at.checked_sub(1).map(|i| shown[i]) else {
        return AT_START;
    };
    let here = shown[at];
    let starts_word = matches!(before, '_' | '-' | '.' | ' ' | '/' | '\\')
        || (before.is_lowercase() && here.is_uppercase())
        || (!before.is_ascii_digit() && here.is_ascii_digit());
    if starts_word {
        AT_WORD
    } else {
        0
    }
}

/// The best way to lay one word over the name, by the usual two tables: `ends[j]` is the best
/// score with the word's current letter matched exactly at `j`, `best[j]` the best with it
/// matched anywhere up to `j`. A row per letter of the word, and only the last row is kept.
fn score_word(word: &[char], shown: &[char], folded: &[char]) -> Option<i32> {
    let n = folded.len();
    if word.len() > n {
        return None;
    }
    let mut prev_ends = vec![NEVER; n];
    let mut prev_best = vec![NEVER; n];
    for (i, &letter) in word.iter().enumerate() {
        let mut ends = vec![NEVER; n];
        let mut best = vec![NEVER; n];
        for j in 0..n {
            if folded[j] == letter {
                ends[j] = if i == 0 {
                    // What was stepped over to get here costs a little, and no more than a
                    // word boundary earns: `rs` should still find `main.rs`.
                    LETTER + place(shown, j) - (j as i32 * SKIPPED).min(AT_WORD / 2)
                } else if j == 0 {
                    NEVER
                } else {
                    let after_gap = prev_best[j - 1].saturating_add(LETTER + place(shown, j));
                    let adjacent =
                        prev_ends[j - 1].saturating_add(LETTER + ADJACENT.max(place(shown, j)));
                    after_gap.max(adjacent)
                };
            }
            let carried = if j == 0 {
                NEVER
            } else {
                best[j - 1].saturating_sub(SKIPPED)
            };
            best[j] = ends[j].max(carried);
        }
        prev_ends = ends;
        prev_best = best;
    }
    let found = *prev_best.last()?;
    (found > NEVER / 2).then_some(found)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_letters_have_to_be_there_and_in_order() {
        assert!(score("mnrs", "main.rs").is_some());
        assert!(score("abc", "a_b_c").is_some());
        assert!(score("abc", "acb").is_none());
        assert!(score("abcd", "abc").is_none());
        assert!(score("x", "").is_none());
    }

    #[test]
    fn case_is_ignored_unless_asked_for() {
        assert!(score("README", "readme.md").is_some());
        assert!(score("readme", "README.md").is_some());
        assert!(Fuzzy::new("README", true).score("readme.md").is_none());
        assert!(Fuzzy::new("README", true).score("README.md").is_some());
        // Not only ASCII.
        assert!(score("ÉTÉ", "été.txt").is_some());
    }

    #[test]
    fn letters_together_beat_letters_apart() {
        let pattern = Fuzzy::new("main", false);
        let whole = pattern.score("main.rs").unwrap();
        let inside = pattern.score("domain.rs").unwrap();
        let scattered = pattern.score("my_admin.rs").unwrap();
        assert!(whole > inside, "{whole} {inside}");
        assert!(inside > scattered, "{inside} {scattered}");
    }

    #[test]
    fn the_start_of_a_word_beats_the_middle_of_one() {
        let pattern = Fuzzy::new("fb", false);
        let camel = pattern.score("FooBar.txt").unwrap();
        let snake = pattern.score("foo_bar.txt").unwrap();
        let middle = pattern.score("offbeat.txt").unwrap();
        assert!(camel > middle, "{camel} {middle}");
        assert!(snake > middle, "{snake} {middle}");
    }

    #[test]
    fn of_two_equal_matches_the_shorter_name_is_first() {
        let pattern = Fuzzy::new("app", false);
        let short = pattern.score("app.rs").unwrap();
        let long = pattern.score("app_with_a_long_tail.rs").unwrap();
        assert!(short > long, "{short} {long}");
    }

    #[test]
    fn every_word_has_to_match_and_the_order_of_the_words_does_not_matter() {
        assert!(score("test app", "app_tests.rs").is_some());
        assert!(score("app test", "app_tests.rs").is_some());
        assert!(score("test zzz", "app_tests.rs").is_none());
    }

    #[test]
    fn a_pattern_with_nothing_in_it_matches_everything() {
        assert_eq!(score("", "anything"), Some(0));
        assert_eq!(score("  ", "anything"), Some(0));
        // A `*` left in the field from a search by mask means what it meant there.
        assert_eq!(score("*", "anything"), Some(0));
        assert!(score("*.rs", "main.rs").is_some());
        assert!(score("*.rs", "main.c").is_none());
        assert!(Fuzzy::new("*", false).is_empty());
    }

    #[test]
    fn an_extension_alone_still_finds_the_file() {
        // The letters are at the far end of a long name, and that must not sink the match.
        assert!(score("rs", "a_very_long_name_for_a_source_file.rs").is_some());
    }
}
