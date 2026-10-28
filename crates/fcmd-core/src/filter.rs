//! Masks and filters: one value type the panel display, the find tool and the copy engine
//! all take (`docs/spec/02-panels.md` §5).
//!
//! Nothing here knows about terminals, configuration files or the clock — `Filter::matches`
//! takes the entry, and the relative-date conditions take `now` so a caller can test them.

use std::collections::BTreeMap;
use std::time::SystemTime;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::entry::{Entry, EntryFlags, EntryKind};
use crate::size::format_size;

/// How many masks a list may expand to. A handful of nested groups can multiply, and the
/// mask comes from a text field, so the expansion is bounded instead of trusted.
const MAX_MASKS: usize = 1024;

/// How deep `<group>` references may nest before the list is called nonsense.
const MAX_GROUP_DEPTH: usize = 16;

/// Compiled size of one `/regex/` mask. A 10 000-character pattern is rejected here rather
/// than eating the heap.
const REGEX_SIZE_LIMIT: usize = 1 << 20;

/// The flags of `EntryFlags` a filter can ask about, with the letter `describe` uses.
/// Flags added to `EntryFlags` later are ignored until they are listed here.
const NAMED_FLAGS: [(EntryFlags, &str); 5] = [
    (EntryFlags::HIDDEN, "h"),
    (EntryFlags::READONLY, "r"),
    (EntryFlags::EXECUTABLE, "x"),
    (EntryFlags::SYSTEM, "s"),
    (EntryFlags::BROKEN_LINK, "b"),
];

/// Everything that can be wrong with the text a user typed. Every variant names the piece it
/// choked on, because these messages go straight into the dialog.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum FilterError {
    #[error("mask list: a quoted mask is never closed")]
    UnclosedQuote,

    #[error("mask `{0}`: the regular expression has no closing `/`")]
    UnclosedRegex(String),

    #[error("mask `/{pattern}/`: {message}")]
    BadRegex { pattern: String, message: String },

    #[error("mask `/{pattern}/{flags}`: `{flags}` is not a regex flag (only `i` is)")]
    BadRegexFlags { pattern: String, flags: String },

    #[error("mask list: only one `|` is allowed, with the exclusions after it")]
    SecondPipe,

    #[error("unknown mask group `<{0}>`")]
    UnknownGroup(String),

    #[error("mask group `<{0}>` refers to itself")]
    GroupCycle(String),

    #[error("mask group `<{0}>`: a group may not contain `|`")]
    PipeInGroup(String),

    #[error("mask groups nest more than {MAX_GROUP_DEPTH} deep")]
    GroupsTooDeep,

    #[error("a mask list may not expand to more than {MAX_MASKS} masks")]
    TooManyMasks,

    #[error("attributes `{0}`: expected letters from `{1}`, each one optionally after a `-`")]
    BadAttrs(String, String),

    #[error("size `{0}`: expected a number with an optional B K M G T P E suffix")]
    BadSize(String),

    #[error("size `{0}`: too large for 64 bits")]
    SizeOverflow(String),

    #[error("size range `{0}`: expected `from-to`, either end may be left out")]
    BadSizeRange(String),

    #[error("date range `{0}`: expected `<=N`, `>=N` or `N-N` days")]
    BadDateRange(String),
}

/// Named mask lists, FAR's `<arc>`/`<temp>`/`<exec>` and whatever the user adds. Names are
/// matched without regard to case; the value is itself a mask list, so groups nest.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Groups(BTreeMap<String, String>);

impl Groups {
    pub fn new() -> Self {
        Groups(BTreeMap::new())
    }

    pub fn insert(&mut self, name: &str, masks: &str) -> Option<String> {
        self.0.insert(name.to_lowercase(), masks.to_string())
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(&name.to_lowercase()).map(String::as_str)
    }

    pub fn remove(&mut self, name: &str) -> Option<String> {
        self.0.remove(&name.to_lowercase())
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.0.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

/// The three groups FAR ships with. They are data, not policy: a caller is free to replace
/// them wholesale before parsing.
pub fn default_groups() -> Groups {
    let mut groups = Groups::new();
    groups.insert(
        "arc",
        "*.zip,*.rar,*.7z,*.tar,*.gz,*.tgz,*.bz2,*.tbz,*.xz,*.txz,*.zst,*.lz4,*.lzma,\
         *.cab,*.arj,*.lzh,*.ace,*.iso,*.jar,*.deb,*.rpm",
    );
    groups.insert(
        "temp",
        "*.tmp,*.temp,*.bak,*.old,*.orig,*.rej,*.swp,*.swo,*~,.#*,*.$$$",
    );
    groups.insert(
        "exec",
        "*.exe,*.com,*.bat,*.cmd,*.ps1,*.msi,*.sh,*.bash,*.zsh,*.fish,*.py,*.pl,*.rb,\
         *.appimage,*.run",
    );
    groups
}

/// One mask: a glob, or a whole-name regular expression.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Mask {
    Glob(String),
    Regex(RegexMask),
}

impl Mask {
    fn matches(&self, name: &str, case_sensitive: bool) -> bool {
        match self {
            Mask::Glob(pattern) => glob_match(pattern, name, case_sensitive),
            Mask::Regex(mask) => mask.re.is_match(name),
        }
    }

    fn source(&self) -> String {
        match self {
            Mask::Glob(pattern) => pattern.clone(),
            Mask::Regex(mask) => {
                let flag = if mask.case_insensitive { "i" } else { "" };
                format!("/{}/{flag}", mask.pattern)
            }
        }
    }
}

/// A `/…/` or `/…/i` mask. The compiled regex is derived state, so equality is on the
/// pattern and the flag.
#[derive(Debug, Clone)]
struct RegexMask {
    pattern: String,
    case_insensitive: bool,
    re: Regex,
}

impl RegexMask {
    fn new(pattern: &str, case_insensitive: bool) -> Result<Self, FilterError> {
        let re = RegexBuilder::new(pattern)
            .case_insensitive(case_insensitive)
            .size_limit(REGEX_SIZE_LIMIT)
            .build()
            .map_err(|e| FilterError::BadRegex {
                pattern: pattern.to_string(),
                message: one_line(&e.to_string()),
            })?;
        Ok(RegexMask {
            pattern: pattern.to_string(),
            case_insensitive,
            re,
        })
    }
}

impl PartialEq for RegexMask {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern && self.case_insensitive == other.case_insensitive
    }
}

impl Eq for RegexMask {}

/// The regex crate reports parse errors over several lines with a caret; a dialog line wants
/// the last one, which is the diagnosis.
fn one_line(message: &str) -> String {
    message
        .lines()
        .map(str::trim)
        .rev()
        .find(|l| !l.is_empty())
        .unwrap_or("invalid regular expression")
        .to_string()
}

/// A parsed mask list: `*.rs,*.toml`, `<arc>|*.rar`, `/^lib.*\.so$/i`.
///
/// An empty list matches everything, which is what makes it usable as an always-present
/// condition. Matching looks at the file name only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaskSet {
    source: String,
    case_sensitive: bool,
    include: Vec<Mask>,
    exclude: Vec<Mask>,
}

impl MaskSet {
    /// Parse with the predefined groups, matching names without regard to case.
    pub fn parse(source: &str) -> Result<Self, FilterError> {
        Self::parse_with(source, &default_groups(), false)
    }

    /// Parse against a caller-supplied group table.
    ///
    /// `case_sensitive` applies to the globs; a regex mask takes its case folding from its
    /// own `i` flag, which is the only reason the flag exists.
    pub fn parse_with(
        source: &str,
        groups: &Groups,
        case_sensitive: bool,
    ) -> Result<Self, FilterError> {
        let mut pieces = Vec::new();
        flatten(source, groups, &mut Vec::new(), &mut pieces, true)?;

        let mut include = Vec::new();
        let mut exclude = Vec::new();
        let mut excluding = false;
        for piece in &pieces {
            match piece {
                Piece::Pipe => {
                    if excluding {
                        return Err(FilterError::SecondPipe);
                    }
                    excluding = true;
                }
                Piece::Mask(text) => {
                    let mask = compile_mask(text)?;
                    if excluding {
                        exclude.push(mask);
                    } else {
                        include.push(mask);
                    }
                }
            }
        }

        Ok(MaskSet {
            source: source.to_string(),
            case_sensitive,
            include,
            exclude,
        })
    }

    /// The text the list was parsed from, for the dialog and the status line.
    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn case_sensitive(&self) -> bool {
        self.case_sensitive
    }

    /// No mask at all: matches every name.
    pub fn is_empty(&self) -> bool {
        self.include.is_empty() && self.exclude.is_empty()
    }

    pub fn matches_name(&self, name: &str) -> bool {
        if self
            .exclude
            .iter()
            .any(|m| m.matches(name, self.case_sensitive))
        {
            return false;
        }
        self.include.is_empty()
            || self
                .include
                .iter()
                .any(|m| m.matches(name, self.case_sensitive))
    }

    pub fn matches(&self, entry: &Entry) -> bool {
        self.matches_name(&entry.name_str())
    }

    /// The masks after group expansion, as text. The UI shows this when a group is involved
    /// and the user asks what it stands for.
    pub fn expanded(&self) -> Vec<String> {
        let mut out: Vec<String> = self.include.iter().map(Mask::source).collect();
        if !self.exclude.is_empty() {
            out.push("|".to_string());
            out.extend(self.exclude.iter().map(Mask::source));
        }
        out
    }
}

/// Stored as the text that was typed, never as the expanded masks: a `filters.toml` stays
/// readable and a group whose contents change keeps working. The case-insensitive form —
/// almost every one — is a plain string.
#[derive(Serialize)]
struct MaskSetRef<'a> {
    source: &'a str,
    case_sensitive: bool,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum MaskSetRepr {
    Text(String),
    Table {
        source: String,
        #[serde(default)]
        case_sensitive: bool,
    },
}

impl Serialize for MaskSet {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.case_sensitive {
            MaskSetRef {
                source: &self.source,
                case_sensitive: true,
            }
            .serialize(serializer)
        } else {
            serializer.serialize_str(&self.source)
        }
    }
}

impl<'de> Deserialize<'de> for MaskSet {
    /// Re-parses, so a hand-edited file cannot hold a mask list that says one thing and
    /// matches another. Only the predefined groups are in reach here; a filter built on a
    /// user group is parsed by its owner with `parse_with`.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (source, case_sensitive) = match MaskSetRepr::deserialize(deserializer)? {
            MaskSetRepr::Text(source) => (source, false),
            MaskSetRepr::Table {
                source,
                case_sensitive,
            } => (source, case_sensitive),
        };
        MaskSet::parse_with(&source, &default_groups(), case_sensitive)
            .map_err(serde::de::Error::custom)
    }
}

/// What a scan of a mask list yields: the masks, and the one `|` that splits include from
/// exclude.
enum Piece {
    Mask(String),
    Pipe,
}

/// Split a mask list on `,` and `;`, keeping both literal inside `"quotes"`, inside a
/// `/regex/`, inside a `[class]` and inside a `<group>` reference.
fn scan(source: &str) -> Result<Vec<Piece>, FilterError> {
    let mut pieces = Vec::new();
    let mut token = String::new();
    let mut quoted_token = false;
    let mut quoting = false;
    let mut in_regex = false;
    let mut class = 0usize;
    let mut angle = 0usize;
    let mut chars = source.chars();

    let flush = |token: &mut String, quoted: &mut bool, pieces: &mut Vec<Piece>| {
        let text = if *quoted {
            std::mem::take(token)
        } else {
            token.trim().to_string()
        };
        token.clear();
        *quoted = false;
        if !text.is_empty() {
            pieces.push(Piece::Mask(text));
        }
    };

    while let Some(c) = chars.next() {
        if quoting {
            if c == '"' {
                quoting = false;
            } else {
                token.push(c);
            }
            continue;
        }
        if in_regex {
            token.push(c);
            match c {
                '\\' => {
                    if let Some(escaped) = chars.next() {
                        token.push(escaped);
                    }
                }
                '/' => in_regex = false,
                _ => {}
            }
            continue;
        }
        match c {
            '"' => {
                quoting = true;
                quoted_token = true;
            }
            '/' if token.trim().is_empty() && class == 0 && angle == 0 => {
                in_regex = true;
                token.push(c);
            }
            '[' => {
                class += 1;
                token.push(c);
            }
            ']' => {
                class = class.saturating_sub(1);
                token.push(c);
            }
            '<' => {
                angle += 1;
                token.push(c);
            }
            '>' => {
                angle = angle.saturating_sub(1);
                token.push(c);
            }
            ',' | ';' if class == 0 && angle == 0 => {
                flush(&mut token, &mut quoted_token, &mut pieces);
            }
            '|' if class == 0 && angle == 0 => {
                flush(&mut token, &mut quoted_token, &mut pieces);
                pieces.push(Piece::Pipe);
            }
            _ => token.push(c),
        }
    }

    if quoting {
        return Err(FilterError::UnclosedQuote);
    }
    if in_regex {
        return Err(FilterError::UnclosedRegex(token));
    }
    flush(&mut token, &mut quoted_token, &mut pieces);
    Ok(pieces)
}

/// `<name>` as a whole token, and nothing else, is a group reference.
fn group_ref(text: &str) -> Option<&str> {
    let inner = text.strip_prefix('<')?.strip_suffix('>')?;
    if inner.is_empty() || inner.contains(['<', '>']) {
        return None;
    }
    Some(inner)
}

/// Scan `source` into `out`, replacing every `<group>` by its own masks. `stack` holds the
/// groups being expanded, which is what turns `<a>` → `<a>` into an error instead of a hang.
fn flatten(
    source: &str,
    groups: &Groups,
    stack: &mut Vec<String>,
    out: &mut Vec<Piece>,
    top: bool,
) -> Result<(), FilterError> {
    for piece in scan(source)? {
        match piece {
            Piece::Pipe => {
                if !top {
                    let name = stack.last().cloned().unwrap_or_default();
                    return Err(FilterError::PipeInGroup(name));
                }
                out.push(Piece::Pipe);
            }
            Piece::Mask(text) => {
                let Some(name) = group_ref(&text) else {
                    if out.len() >= MAX_MASKS {
                        return Err(FilterError::TooManyMasks);
                    }
                    out.push(Piece::Mask(text));
                    continue;
                };
                let key = name.to_lowercase();
                if stack.contains(&key) {
                    return Err(FilterError::GroupCycle(name.to_string()));
                }
                if stack.len() >= MAX_GROUP_DEPTH {
                    return Err(FilterError::GroupsTooDeep);
                }
                let body = groups
                    .get(&key)
                    .ok_or_else(|| FilterError::UnknownGroup(name.to_string()))?;
                stack.push(key);
                flatten(body, groups, stack, out, false)?;
                stack.pop();
            }
        }
    }
    Ok(())
}

fn compile_mask(text: &str) -> Result<Mask, FilterError> {
    if let Some(rest) = text.strip_prefix('/') {
        let end = rest
            .rfind('/')
            .ok_or_else(|| FilterError::UnclosedRegex(text.to_string()))?;
        let (pattern, flags) = (&rest[..end], &rest[end + 1..]);
        let case_insensitive = match flags {
            "" => false,
            "i" | "I" => true,
            other => {
                return Err(FilterError::BadRegexFlags {
                    pattern: pattern.to_string(),
                    flags: other.to_string(),
                })
            }
        };
        return Ok(Mask::Regex(RegexMask::new(pattern, case_insensitive)?));
    }
    let pattern = if text.contains(['*', '?', '[']) {
        text.to_string()
    } else {
        // A bare word means "contains", as in every commander and in `Selection::apply_mask`.
        format!("*{text}*")
    };
    Ok(Mask::Glob(pattern))
}

fn fold(c: char, case_sensitive: bool) -> char {
    if case_sensitive {
        c
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

fn same(a: char, b: char, case_sensitive: bool) -> bool {
    fold(a, case_sensitive) == fold(b, case_sensitive)
}

/// `*`, `?` and `[a-f]` against one file name.
///
/// Backtracking goes to the last `*` only, so a hostile mask costs O(name × mask) and can
/// never hang. An unterminated `[` is a literal bracket, as the shells have it.
fn glob_match(pattern: &str, name: &str, case_sensitive: bool) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let n: Vec<char> = name.chars().collect();
    let mut pi = 0usize;
    let mut ni = 0usize;
    let mut star: Option<(usize, usize)> = None;

    while ni < n.len() {
        let step = if pi < p.len() {
            match p[pi] {
                '*' => {
                    star = Some((pi + 1, ni));
                    pi += 1;
                    continue;
                }
                '?' => Some((pi + 1, ni + 1)),
                '[' => match char_class(&p, pi, n[ni], case_sensitive) {
                    Some((end, true)) => Some((end, ni + 1)),
                    Some((_, false)) => None,
                    None if same(p[pi], n[ni], case_sensitive) => Some((pi + 1, ni + 1)),
                    None => None,
                },
                c if same(c, n[ni], case_sensitive) => Some((pi + 1, ni + 1)),
                _ => None,
            }
        } else {
            None
        };

        match step {
            Some((next_p, next_n)) => {
                pi = next_p;
                ni = next_n;
            }
            None => match star {
                Some((resume, consumed)) => {
                    pi = resume;
                    ni = consumed + 1;
                    star = Some((resume, consumed + 1));
                }
                None => return false,
            },
        }
    }

    p[pi..].iter().all(|c| *c == '*')
}

/// Match one `[…]` at `start`. Returns the index just past the `]` and whether `c` is in the
/// class, or `None` when there is no closing `]`.
fn char_class(p: &[char], start: usize, c: char, case_sensitive: bool) -> Option<(usize, bool)> {
    let mut i = start + 1;
    let negated = matches!(p.get(i), Some('!' | '^'));
    if negated {
        i += 1;
    }
    let mut hit = false;
    let mut first = true;
    while i < p.len() {
        if p[i] == ']' && !first {
            return Some((i + 1, hit != negated));
        }
        first = false;
        if i + 2 < p.len() && p[i + 1] == '-' && p[i + 2] != ']' {
            let lo = fold(p[i], case_sensitive);
            let hi = fold(p[i + 2], case_sensitive);
            let x = fold(c, case_sensitive);
            if lo <= x && x <= hi {
                hit = true;
            }
            i += 3;
        } else {
            if same(p[i], c, case_sensitive) {
                hit = true;
            }
            i += 1;
        }
    }
    None
}

/// Which entries a condition set looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KindFilter {
    #[default]
    Both,
    Files,
    Dirs,
}

impl KindFilter {
    pub fn as_str(self) -> &'static str {
        match self {
            KindFilter::Both => "both",
            KindFilter::Files => "files",
            KindFilter::Dirs => "dirs",
        }
    }

    /// The spellings a person writes: the plural the dialog shows, the singular that reads
    /// better in a configuration file, and `folder` for the word half the world uses.
    pub fn parse(text: &str) -> Option<KindFilter> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "both" | "any" => Some(KindFilter::Both),
            "file" | "files" => Some(KindFilter::Files),
            "dir" | "dirs" | "folder" | "folders" => Some(KindFilter::Dirs),
            _ => None,
        }
    }

    fn matches(self, kind: EntryKind) -> bool {
        match self {
            KindFilter::Both => true,
            KindFilter::Files => !kind.is_dirlike(),
            KindFilter::Dirs => kind.is_dirlike(),
        }
    }
}

/// A size window in bytes, either end optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct SizeRange {
    pub min: Option<u64>,
    pub max: Option<u64>,
}

impl SizeRange {
    /// `1M-2G`, `-2G`, `1M-`; a lone `1M` is a lower bound, which is how the dialog's single
    /// "from" field behaves.
    pub fn parse(text: &str) -> Result<Self, FilterError> {
        let trimmed = text.trim();
        let range = match trimmed.split_once('-') {
            Some((from, to)) => SizeRange {
                min: optional_size(from)?,
                max: optional_size(to)?,
            },
            None => SizeRange {
                min: Some(parse_size(trimmed)?),
                max: None,
            },
        };
        if let (Some(min), Some(max)) = (range.min, range.max) {
            if min > max {
                return Err(FilterError::BadSizeRange(text.to_string()));
            }
        }
        Ok(range)
    }

    pub fn contains(&self, bytes: u64) -> bool {
        self.min.is_none_or(|min| bytes >= min) && self.max.is_none_or(|max| bytes <= max)
    }

    pub fn is_empty(&self) -> bool {
        self.min.is_none() && self.max.is_none()
    }

    fn describe(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("{}..{}", format_size(min), format_size(max)),
            (Some(min), None) => format!(">={}", format_size(min)),
            (None, Some(max)) => format!("<={}", format_size(max)),
            (None, None) => "any size".to_string(),
        }
    }
}

fn optional_size(text: &str) -> Result<Option<u64>, FilterError> {
    if text.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(parse_size(text)?))
    }
}

/// `1024`, `64K`, `3M`, `2g`: an upper-case suffix is binary, a lower-case one decimal, no
/// suffix means bytes.
pub fn parse_size(text: &str) -> Result<u64, FilterError> {
    let trimmed = text.trim();
    let bad = || FilterError::BadSize(text.to_string());
    let last = trimmed.chars().next_back().ok_or_else(bad)?;
    let (digits, unit) = if last.is_ascii_alphabetic() {
        let unit = match last {
            'B' | 'b' => 1u64,
            'K' => 1 << 10,
            'k' => 1_000,
            'M' => 1 << 20,
            'm' => 1_000_000,
            'G' => 1 << 30,
            'g' => 1_000_000_000,
            'T' => 1 << 40,
            't' => 1_000_000_000_000,
            'P' => 1 << 50,
            'p' => 1_000_000_000_000_000,
            'E' => 1 << 60,
            'e' => 1_000_000_000_000_000_000,
            _ => return Err(bad()),
        };
        (trimmed[..trimmed.len() - last.len_utf8()].trim(), unit)
    } else {
        (trimmed, 1)
    };
    let count: u64 = digits.parse().map_err(|_| bad())?;
    count
        .checked_mul(unit)
        .ok_or_else(|| FilterError::SizeOverflow(text.to_string()))
}

/// Which of an entry's timestamps a date condition looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimeField {
    #[default]
    Modified,
    Created,
    Accessed,
    Changed,
}

impl TimeField {
    /// The spellings a person writes: the column's own name, and the plain English verb a
    /// configuration file reads better with.
    pub fn parse(text: &str) -> Option<TimeField> {
        match text.trim().to_ascii_lowercase().as_str() {
            "" | "write" | "written" | "modified" | "mtime" => Some(TimeField::Modified),
            "create" | "created" | "btime" => Some(TimeField::Created),
            "access" | "accessed" | "atime" => Some(TimeField::Accessed),
            "change" | "changed" | "ctime" => Some(TimeField::Changed),
            _ => None,
        }
    }

    /// The column spelling, so a filter and a view mode call the same thing by one name.
    pub fn as_str(self) -> &'static str {
        match self {
            TimeField::Modified => "mtime",
            TimeField::Created => "btime",
            TimeField::Accessed => "atime",
            TimeField::Changed => "ctime",
        }
    }
}

/// A date window, absolute or counted back from now.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DateRange {
    /// Seconds since the Unix epoch, either end optional. Core has no calendar code, so the
    /// bounds arrive as instants from the dialog's date picker, not as text.
    Absolute { from: Option<i64>, to: Option<i64> },

    /// The entry's age in whole days, either end optional: `max_days: 0` is today,
    /// `max_days: 30` the last thirty days, `min_days: 30` thirty days old and older.
    ///
    /// This is the form that survives being saved, which is the whole point of having it.
    /// (`docs/spec/02-panels.md` §5 spells its second example `>= 30` for "thirty days ago
    /// or newer", which contradicts its first; the age reading is the one that makes both
    /// `<= 0` = today and a single comparison direction work, so that is what is used.)
    Age {
        min_days: Option<u32>,
        max_days: Option<u32>,
    },
}

impl Default for DateRange {
    fn default() -> Self {
        DateRange::Age {
            min_days: None,
            max_days: None,
        }
    }
}

impl DateRange {
    /// `<=30`, `>=7`, `7-30`: a window on the age in days.
    pub fn parse_age(text: &str) -> Result<Self, FilterError> {
        let trimmed = text.trim();
        let bad = || FilterError::BadDateRange(text.to_string());
        let days = |s: &str| -> Result<u32, FilterError> { s.trim().parse().map_err(|_| bad()) };

        if let Some(rest) = trimmed.strip_prefix("<=") {
            return Ok(DateRange::Age {
                min_days: None,
                max_days: Some(days(rest)?),
            });
        }
        if let Some(rest) = trimmed.strip_prefix(">=") {
            return Ok(DateRange::Age {
                min_days: Some(days(rest)?),
                max_days: None,
            });
        }
        match trimmed.split_once('-') {
            Some((from, to)) => {
                let (min, max) = (days(from)?, days(to)?);
                if min > max {
                    return Err(bad());
                }
                Ok(DateRange::Age {
                    min_days: Some(min),
                    max_days: Some(max),
                })
            }
            None => Ok(DateRange::Age {
                min_days: None,
                max_days: Some(days(trimmed)?),
            }),
        }
    }

    fn matches(&self, ts: SystemTime, now: SystemTime) -> bool {
        match *self {
            DateRange::Absolute { from, to } => {
                let secs = epoch_secs(ts);
                from.is_none_or(|f| secs >= f) && to.is_none_or(|t| secs <= t)
            }
            DateRange::Age { min_days, max_days } => {
                let age = age_days(ts, now);
                min_days.is_none_or(|d| age >= u64::from(d))
                    && max_days.is_none_or(|d| age <= u64::from(d))
            }
        }
    }

    fn is_empty(&self) -> bool {
        match *self {
            DateRange::Absolute { from, to } => from.is_none() && to.is_none(),
            DateRange::Age { min_days, max_days } => min_days.is_none() && max_days.is_none(),
        }
    }
}

/// A date condition: which timestamp, and the window on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct DateCondition {
    pub field: TimeField,
    pub range: DateRange,
}

impl DateCondition {
    pub fn new(field: TimeField, range: DateRange) -> Self {
        DateCondition { field, range }
    }

    /// An entry whose chosen timestamp is not known yet passes: the second metadata pass
    /// (SPEC 02 §2.4) fills it in, and a filter must not hide files over a missing stat.
    fn matches(&self, entry: &Entry, now: SystemTime) -> bool {
        match timestamp(entry, self.field) {
            Some(ts) => self.range.matches(ts, now),
            None => true,
        }
    }

    fn describe(&self) -> String {
        let field = self.field.as_str();
        match self.range {
            DateRange::Absolute { from, to } => match (from, to) {
                (Some(f), Some(t)) => format!("{field} {}..{}", epoch_day(f), epoch_day(t)),
                (Some(f), None) => format!("{field}>={}", epoch_day(f)),
                (None, Some(t)) => format!("{field}<={}", epoch_day(t)),
                (None, None) => field.to_string(),
            },
            DateRange::Age { min_days, max_days } => match (min_days, max_days) {
                (Some(a), Some(b)) => format!("{field} {a}..{b}d"),
                (Some(a), None) => format!("{field}>={a}d"),
                (None, Some(b)) => format!("{field}<={b}d"),
                (None, None) => field.to_string(),
            },
        }
    }
}

/// `Entry` carries the write time only; the other three arrive with the lazy `ExtMeta` pass
/// and are unknown until then.
fn timestamp(entry: &Entry, field: TimeField) -> Option<SystemTime> {
    match field {
        TimeField::Modified => entry.mtime,
        TimeField::Created => entry.btime,
        TimeField::Accessed => entry.atime,
        // No entry carries a change time yet, and a missing timestamp lets the entry through
        // rather than hiding it (see `DateCondition::matches`).
        TimeField::Changed => None,
    }
}

fn epoch_secs(ts: SystemTime) -> i64 {
    match ts.duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => d.as_secs() as i64,
        Err(e) => -(e.duration().as_secs() as i64),
    }
}

/// Whole days between `ts` and `now`. A timestamp in the future is age zero rather than an
/// error: clock skew and bad archives are not the user's problem.
fn age_days(ts: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(ts)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0)
}

/// `YYYY-MM-DD` from epoch seconds, for `describe`. Hinnant's civil-from-days, so that a
/// one-line description needs no calendar crate.
fn epoch_day(secs: i64) -> String {
    let days = secs.div_euclid(86_400) + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let doe = (days - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { year + 1 } else { year };
    format!("{year:04}-{month:02}-{day:02}")
}

/// One attribute's three states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttrState {
    #[default]
    DontCare,
    MustHave,
    MustNotHave,
}

/// The attribute conditions, as two `EntryFlags` masks. Flags that `EntryFlags` does not
/// have cannot be asked about, and bits it gains later are dropped when the filter is saved
/// until they are listed in `NAMED_FLAGS`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(from = "AttrRepr", into = "AttrRepr")]
pub struct AttrConditions {
    must_have: EntryFlags,
    must_not_have: EntryFlags,
}

impl AttrConditions {
    /// The state of one flag. `flag` is a single `EntryFlags` constant; a combination reads
    /// as "must have" only when every bit of it must be there.
    pub fn state(&self, flag: EntryFlags) -> AttrState {
        if self.must_have.contains(flag) {
            AttrState::MustHave
        } else if self.must_not_have.contains(flag) {
            AttrState::MustNotHave
        } else {
            AttrState::DontCare
        }
    }

    /// The conditions a person writes rather than clicks: the letters the panel's attribute
    /// column draws, each one meaning "must have it", and a `-` before a letter meaning
    /// "must not". `x-h` is an executable that is not hidden. Letters may be given in any
    /// order and the string may be empty, which asks nothing.
    ///
    /// The letters are `NAMED_FLAGS`' own, so what can be asked for here is exactly what a
    /// saved filter can carry, and a flag that is added to `EntryFlags` is askable the day it
    /// is named there.
    pub fn parse(text: &str) -> Result<Self, FilterError> {
        let mut attrs = AttrConditions::default();
        let mut negated = false;
        for ch in text.chars() {
            if ch.is_whitespace() || ch == ',' {
                continue;
            }
            if ch == '-' || ch == '!' {
                negated = true;
                continue;
            }
            let letter = ch.to_ascii_lowercase();
            let Some((flag, _)) = NAMED_FLAGS
                .iter()
                .find(|(_, name)| name.starts_with(letter))
            else {
                let known: String = NAMED_FLAGS.iter().map(|(_, n)| *n).collect();
                return Err(FilterError::BadAttrs(text.to_string(), known));
            };
            attrs.set(
                *flag,
                if negated {
                    AttrState::MustNotHave
                } else {
                    AttrState::MustHave
                },
            );
            negated = false;
        }
        Ok(attrs)
    }

    pub fn set(&mut self, flag: EntryFlags, state: AttrState) {
        self.must_have.set(flag, state == AttrState::MustHave);
        self.must_not_have
            .set(flag, state == AttrState::MustNotHave);
    }

    pub fn is_empty(&self) -> bool {
        self.must_have == EntryFlags::empty() && self.must_not_have == EntryFlags::empty()
    }

    fn matches(&self, flags: EntryFlags) -> bool {
        flags.contains(self.must_have) && flags.bits() & self.must_not_have.bits() == 0
    }

    fn describe_into(&self, parts: &mut Vec<String>) {
        for (flag, letter) in NAMED_FLAGS {
            match self.state(flag) {
                AttrState::MustHave => parts.push(format!("+{letter}")),
                AttrState::MustNotHave => parts.push(format!("-{letter}")),
                AttrState::DontCare => {}
            }
        }
    }
}

/// The on-disk spelling of `AttrConditions`: one named three-state per flag, so a filter
/// file reads as `hidden = "must_not_have"`.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
struct AttrRepr {
    hidden: AttrState,
    readonly: AttrState,
    executable: AttrState,
    system: AttrState,
    broken_link: AttrState,
}

impl From<AttrRepr> for AttrConditions {
    fn from(repr: AttrRepr) -> Self {
        let mut attrs = AttrConditions::default();
        attrs.set(EntryFlags::HIDDEN, repr.hidden);
        attrs.set(EntryFlags::READONLY, repr.readonly);
        attrs.set(EntryFlags::EXECUTABLE, repr.executable);
        attrs.set(EntryFlags::SYSTEM, repr.system);
        attrs.set(EntryFlags::BROKEN_LINK, repr.broken_link);
        attrs
    }
}

impl From<AttrConditions> for AttrRepr {
    fn from(attrs: AttrConditions) -> Self {
        AttrRepr {
            hidden: attrs.state(EntryFlags::HIDDEN),
            readonly: attrs.state(EntryFlags::READONLY),
            executable: attrs.state(EntryFlags::EXECUTABLE),
            system: attrs.state(EntryFlags::SYSTEM),
            broken_link: attrs.state(EntryFlags::BROKEN_LINK),
        }
    }
}

/// The hard-link condition. FAR's is a checkbox for "more than one"; the other way round
/// costs nothing and the find tool wants it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HardLinks {
    MoreThanOne,
    ExactlyOne,
}

/// `Entry` has no link count yet; it comes with the lazy `ExtMeta` pass. Until then the
/// condition is skipped rather than guessed at.
fn link_count(_entry: &Entry) -> Option<u64> {
    None
}

/// A named condition set, used by the panel display, by find and by copy.
///
/// Every condition is switchable on its own: `None` (or an all-"don't care" `AttrConditions`,
/// or `KindFilter::Both`) means the condition is off, so a saved filter can be narrowed
/// without being rewritten. An all-off filter matches everything.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub masks: Option<MaskSet>,
    pub kind: KindFilter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<SizeRange>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<DateCondition>,
    pub attrs: AttrConditions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hard_links: Option<HardLinks>,
}

impl Filter {
    pub fn named(name: &str) -> Self {
        Filter {
            name: name.to_string(),
            ..Filter::default()
        }
    }

    /// A filter on a mask list and nothing else — the quick filter bar and the mask dialogs.
    pub fn from_masks(masks: MaskSet) -> Self {
        Filter {
            masks: Some(masks),
            ..Filter::default()
        }
    }

    /// No condition is switched on, so this filter hides nothing.
    pub fn is_empty(&self) -> bool {
        self.masks.as_ref().is_none_or(MaskSet::is_empty)
            && self.kind == KindFilter::Both
            && self.size.is_none_or(|s| s.is_empty())
            && self.date.is_none_or(|d| d.range.is_empty())
            && self.attrs.is_empty()
            && self.hard_links.is_none()
    }

    pub fn matches(&self, entry: &Entry) -> bool {
        self.matches_at(entry, SystemTime::now())
    }

    /// `now` is passed in so the relative-date conditions are testable and so a whole listing
    /// is judged against one instant.
    pub fn matches_at(&self, entry: &Entry, now: SystemTime) -> bool {
        // `..` is never filtered out: a panel that loses it cannot be left with `Enter`.
        if entry.kind == EntryKind::Parent {
            return true;
        }
        if !self.kind.matches(entry.kind) {
            return false;
        }
        if let Some(masks) = &self.masks {
            if !masks.matches(entry) {
                return false;
            }
        }
        if let Some(size) = &self.size {
            if !size.contains(entry.size) {
                return false;
            }
        }
        if let Some(date) = &self.date {
            if !date.matches(entry, now) {
                return false;
            }
        }
        if !self.attrs.matches(entry.flags) {
            return false;
        }
        if let (Some(condition), Some(links)) = (self.hard_links, link_count(entry)) {
            let wanted = match condition {
                HardLinks::MoreThanOne => links > 1,
                HardLinks::ExactlyOne => links == 1,
            };
            if !wanted {
                return false;
            }
        }
        true
    }

    /// One line for the status line: `sources: *.rs, files, >=1.00 K, mtime<=30d, -h`.
    pub fn describe(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if let Some(masks) = &self.masks {
            if !masks.is_empty() {
                parts.push(masks.source().to_string());
            }
        }
        if self.kind != KindFilter::Both {
            parts.push(self.kind.as_str().to_string());
        }
        if let Some(size) = &self.size {
            if !size.is_empty() {
                parts.push(size.describe());
            }
        }
        if let Some(date) = &self.date {
            if !date.range.is_empty() {
                parts.push(date.describe());
            }
        }
        self.attrs.describe_into(&mut parts);
        match self.hard_links {
            Some(HardLinks::MoreThanOne) => parts.push("links>1".to_string()),
            Some(HardLinks::ExactlyOne) => parts.push("links=1".to_string()),
            None => {}
        }
        if parts.is_empty() {
            parts.push("everything".to_string());
        }
        let body = parts.join(", ");
        if self.name.is_empty() {
            body
        } else {
            format!("{}: {body}", self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entry::EntryId;
    use serde::de::value::{Error as ValueError, StrDeserializer};
    use std::ffi::OsString;
    use std::time::Duration;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size: 0,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn masks(source: &str) -> MaskSet {
        MaskSet::parse(source).expect("mask list parses")
    }

    /// Switch one condition on, the way the dialog does.
    fn filter_with(switch: impl FnOnce(&mut Filter)) -> Filter {
        let mut filter = Filter::default();
        switch(&mut filter);
        filter
    }

    #[test]
    fn a_bare_word_matches_anywhere_in_the_name() {
        let set = masks("cargo");
        assert!(set.matches_name("Cargo.toml"));
        assert!(set.matches_name("my-cargo-notes"));
        assert!(!set.matches_name("main.rs"));
    }

    #[test]
    fn a_star_mask_matches_the_extension_and_nothing_else() {
        let set = masks("*.rs");
        assert!(set.matches_name("main.rs"));
        assert!(!set.matches_name("main.rs.bak"));
        assert!(!set.matches_name("rs"));
    }

    #[test]
    fn a_question_mark_matches_exactly_one_character() {
        let set = masks("?.rs");
        assert!(set.matches_name("a.rs"));
        assert!(!set.matches_name("ab.rs"));
        assert!(!set.matches_name(".rs"));
    }

    #[test]
    fn a_character_range_matches_the_characters_in_it() {
        let set = masks("[c-f]*");
        assert!(set.matches_name("dog"));
        assert!(set.matches_name("ferret"));
        assert!(!set.matches_name("apple"));
    }

    #[test]
    fn a_negated_character_class_excludes_its_characters() {
        let set = masks("[!ad]*");
        assert!(set.matches_name("cat"));
        assert!(!set.matches_name("ant"));
        assert!(!set.matches_name("dog"));
    }

    #[test]
    fn an_unclosed_character_class_is_matched_literally() {
        let set = masks("a[b*");
        assert!(set.matches_name("a[bc"));
        assert!(!set.matches_name("abc"));
    }

    #[test]
    fn masks_are_case_insensitive_by_default() {
        assert!(masks("*.RS").matches_name("main.rs"));
        assert!(masks("[C-F]*").matches_name("dog"));
    }

    #[test]
    fn a_case_sensitive_mask_set_respects_the_case() {
        let set = MaskSet::parse_with("*.RS", &default_groups(), true).unwrap();
        assert!(!set.matches_name("main.rs"));
        assert!(set.matches_name("MAIN.RS"));
    }

    #[test]
    fn several_masks_are_separated_by_commas_or_semicolons() {
        let set = masks("*.rs;*.toml, *.md");
        assert_eq!(set.expanded().len(), 3);
        assert!(set.matches_name("main.rs"));
        assert!(set.matches_name("Cargo.toml"));
        assert!(set.matches_name("README.md"));
        assert!(!set.matches_name("a.txt"));
    }

    #[test]
    fn a_quoted_mask_may_contain_a_delimiter() {
        let set = masks("\"*,*.txt\",*.rs");
        assert_eq!(set.expanded().len(), 2);
        assert!(set.matches_name("weird,name.txt"));
        assert!(set.matches_name("main.rs"));
        assert!(!set.matches_name("plain.txt"));
    }

    #[test]
    fn an_unclosed_quote_is_reported() {
        assert_eq!(MaskSet::parse("\"*.rs"), Err(FilterError::UnclosedQuote));
    }

    #[test]
    fn a_regex_mask_matches_the_whole_name() {
        let set = masks("/^lib.*\\.so$/");
        assert!(set.matches_name("libc.so"));
        assert!(!set.matches_name("mylibc.so"));
    }

    #[test]
    fn a_regex_mask_takes_its_case_folding_from_its_own_i_flag() {
        assert!(!masks("/^MAIN/").matches_name("main.rs"));
        assert!(masks("/^MAIN/i").matches_name("main.rs"));
    }

    #[test]
    fn a_regex_mask_may_contain_a_delimiter() {
        let set = masks("/^a,b$/");
        assert_eq!(set.expanded().len(), 1);
        assert!(set.matches_name("a,b"));
    }

    #[test]
    fn an_unterminated_regex_is_reported() {
        assert_eq!(
            MaskSet::parse("/abc"),
            Err(FilterError::UnclosedRegex("/abc".to_string()))
        );
    }

    #[test]
    fn an_invalid_regex_is_reported_with_its_own_message() {
        match MaskSet::parse("/a(/") {
            Err(FilterError::BadRegex { pattern, message }) => {
                assert_eq!(pattern, "a(");
                assert!(!message.contains('\n'));
                assert!(!message.is_empty());
            }
            other => panic!("expected a regex error, got {other:?}"),
        }
    }

    #[test]
    fn an_unknown_regex_flag_is_reported() {
        assert_eq!(
            MaskSet::parse("/a/x"),
            Err(FilterError::BadRegexFlags {
                pattern: "a".to_string(),
                flags: "x".to_string(),
            })
        );
    }

    #[test]
    fn a_regex_too_big_to_compile_is_rejected_rather_than_eating_the_heap() {
        let huge = "/(?:(?:(?:a{255}){255}){255}){255}/";
        assert!(matches!(
            MaskSet::parse(huge),
            Err(FilterError::BadRegex { .. })
        ));
    }

    #[test]
    fn a_ten_thousand_character_mask_comes_back_instead_of_hanging() {
        let stars: String = "*".repeat(10_000);
        assert!(masks(&stars).matches_name("anything.txt"));
        // Either answer is acceptable for the regex; what matters is that parsing returns.
        let _ = MaskSet::parse(&format!("/{}/", "x".repeat(10_000)));
    }

    #[test]
    fn an_exclude_mask_wins_over_the_include_list() {
        let set = masks("*.rs,*.toml|*_test.rs");
        assert!(set.matches_name("main.rs"));
        assert!(set.matches_name("Cargo.toml"));
        assert!(!set.matches_name("parser_test.rs"));
    }

    #[test]
    fn a_trailing_pipe_means_no_exclusions() {
        let set = masks("*.*|");
        assert!(set.matches_name("main.rs"));
        assert!(!set.matches_name("Makefile"));
    }

    #[test]
    fn a_leading_pipe_excludes_from_everything() {
        let set = masks("|*.bak");
        assert!(set.matches_name("Makefile"));
        assert!(!set.matches_name("main.rs.bak"));
    }

    #[test]
    fn only_one_pipe_is_allowed() {
        assert_eq!(
            MaskSet::parse("*.rs|*.bak|*.old"),
            Err(FilterError::SecondPipe)
        );
    }

    #[test]
    fn an_empty_mask_list_matches_everything() {
        let set = masks("  ");
        assert!(set.is_empty());
        assert!(set.matches_name("anything at all"));
    }

    #[test]
    fn a_group_expands_to_its_mask_list() {
        let set = masks("<arc>|*.rar");
        assert!(set.matches_name("sources.zip"));
        assert!(!set.matches_name("sources.rar"));
        assert!(!set.matches_name("main.rs"));
    }

    #[test]
    fn a_group_in_the_exclude_list_excludes_all_of_its_masks() {
        let set = masks("*|<temp>");
        assert!(set.matches_name("main.rs"));
        assert!(!set.matches_name("main.rs.bak"));
        assert!(!set.matches_name("notes~"));
    }

    #[test]
    fn the_predefined_groups_cover_archives_temporaries_and_executables() {
        let groups = default_groups();
        assert_eq!(groups.len(), 3);
        assert!(masks("<arc>").matches_name("x.7z"));
        assert!(masks("<temp>").matches_name("x.tmp"));
        assert!(masks("<exec>").matches_name("install.sh"));
    }

    #[test]
    fn groups_nest() {
        let mut groups = Groups::new();
        groups.insert("inner", "*.rs");
        groups.insert("outer", "<inner>,*.toml");
        let set = MaskSet::parse_with("<outer>", &groups, false).unwrap();
        assert!(set.matches_name("main.rs"));
        assert!(set.matches_name("Cargo.toml"));
        assert!(!set.matches_name("README.md"));
    }

    #[test]
    fn a_group_name_is_matched_without_regard_to_case() {
        let mut groups = Groups::new();
        groups.insert("Sources", "*.rs");
        assert!(MaskSet::parse_with("<SOURCES>", &groups, false)
            .unwrap()
            .matches_name("main.rs"));
    }

    #[test]
    fn a_group_cycle_is_reported_rather_than_looping() {
        let mut groups = Groups::new();
        groups.insert("a", "*.x,<b>");
        groups.insert("b", "<a>");
        assert_eq!(
            MaskSet::parse_with("<a>", &groups, false),
            Err(FilterError::GroupCycle("a".to_string()))
        );
    }

    #[test]
    fn a_group_that_refers_to_itself_is_reported_rather_than_looping() {
        let mut groups = Groups::new();
        groups.insert("a", "<a>");
        assert_eq!(
            MaskSet::parse_with("<a>", &groups, false),
            Err(FilterError::GroupCycle("a".to_string()))
        );
    }

    #[test]
    fn an_unknown_group_is_reported_by_name() {
        assert_eq!(
            MaskSet::parse("<nope>"),
            Err(FilterError::UnknownGroup("nope".to_string()))
        );
    }

    #[test]
    fn a_group_may_not_contain_an_exclude_pipe() {
        let mut groups = Groups::new();
        groups.insert("a", "*.rs|*.bak");
        assert_eq!(
            MaskSet::parse_with("<a>", &groups, false),
            Err(FilterError::PipeInGroup("a".to_string()))
        );
    }

    #[test]
    fn groups_nested_too_deep_are_reported() {
        let mut groups = Groups::new();
        groups.insert("g0", "*.x");
        for i in 1..=20 {
            groups.insert(&format!("g{i}"), &format!("<g{}>", i - 1));
        }
        assert_eq!(
            MaskSet::parse_with("<g20>", &groups, false),
            Err(FilterError::GroupsTooDeep)
        );
    }

    #[test]
    fn a_group_expansion_bomb_is_reported_rather_than_expanded() {
        let mut groups = Groups::new();
        groups.insert("g0", "*.x");
        for i in 1..=12 {
            let body = format!("<g{}>,<g{}>", i - 1, i - 1);
            groups.insert(&format!("g{i}"), &body);
        }
        assert_eq!(
            MaskSet::parse_with("<g12>", &groups, false),
            Err(FilterError::TooManyMasks)
        );
    }

    #[test]
    fn a_mask_set_keeps_the_text_it_was_parsed_from() {
        let set = masks("<arc>,*.rs");
        assert_eq!(set.source(), "<arc>,*.rs");
        assert!(set.expanded().len() > 2);
    }

    #[test]
    fn a_mask_set_deserializes_from_a_plain_string() {
        let set = MaskSet::deserialize(StrDeserializer::<ValueError>::new("*.rs,*.toml")).unwrap();
        assert_eq!(set.source(), "*.rs,*.toml");
        assert!(set.matches_name("main.rs"));
    }

    #[test]
    fn a_stored_mask_list_that_no_longer_parses_is_a_deserialization_error() {
        let bad = StrDeserializer::<ValueError>::new("*.rs|*.bak|*.old");
        assert!(MaskSet::deserialize(bad).is_err());
    }

    #[test]
    fn a_filter_with_no_conditions_matches_everything() {
        let filter = Filter::default();
        assert!(filter.is_empty());
        assert!(filter.matches(&entry("main.rs", EntryKind::File)));
        assert!(filter.matches(&entry("src", EntryKind::Dir)));
        assert_eq!(filter.describe(), "everything");
    }

    #[test]
    fn the_parent_entry_is_never_filtered_out() {
        let mut filter = Filter::from_masks(masks("*.rs"));
        filter.kind = KindFilter::Files;
        assert!(filter.matches(&Entry::parent_entry()));
    }

    #[test]
    fn a_kind_condition_keeps_only_files_or_only_directories() {
        let mut filter = filter_with(|f| f.kind = KindFilter::Files);
        assert!(filter.matches(&entry("main.rs", EntryKind::File)));
        assert!(!filter.matches(&entry("src", EntryKind::Dir)));

        filter.kind = KindFilter::Dirs;
        assert!(!filter.matches(&entry("main.rs", EntryKind::File)));
        assert!(filter.matches(&entry("src", EntryKind::Dir)));
    }

    #[test]
    fn an_upper_case_size_suffix_is_binary_and_a_lower_case_one_decimal() {
        assert_eq!(parse_size("1K").unwrap(), 1024);
        assert_eq!(parse_size("1k").unwrap(), 1000);
        assert_eq!(parse_size("2M").unwrap(), 2 * 1024 * 1024);
        assert_eq!(parse_size("1E").unwrap(), 1 << 60);
        assert_eq!(parse_size(" 4 G ").unwrap(), 4 << 30);
    }

    #[test]
    fn a_size_without_a_suffix_is_bytes() {
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("7B").unwrap(), 7);
    }

    #[test]
    fn a_size_that_is_not_a_number_is_reported() {
        assert_eq!(
            parse_size("1.5M"),
            Err(FilterError::BadSize("1.5M".to_string()))
        );
        assert_eq!(parse_size("M"), Err(FilterError::BadSize("M".to_string())));
        assert_eq!(parse_size(""), Err(FilterError::BadSize(String::new())));
    }

    #[test]
    fn a_size_too_large_for_sixty_four_bits_is_reported_rather_than_wrapping() {
        assert_eq!(
            parse_size("17E"),
            Err(FilterError::SizeOverflow("17E".to_string()))
        );
    }

    #[test]
    fn a_size_range_may_be_open_at_either_end() {
        let range = SizeRange::parse("1K-2K").unwrap();
        assert!(!range.contains(1023));
        assert!(range.contains(1500));
        assert!(!range.contains(4096));

        assert_eq!(SizeRange::parse("-2K").unwrap().min, None);
        assert_eq!(SizeRange::parse("1K-").unwrap().max, None);
        assert_eq!(SizeRange::parse("1K").unwrap().min, Some(1024));
    }

    #[test]
    fn a_backwards_size_range_is_reported() {
        assert_eq!(
            SizeRange::parse("2K-1K"),
            Err(FilterError::BadSizeRange("2K-1K".to_string()))
        );
    }

    #[test]
    fn a_size_condition_keeps_only_entries_inside_the_range() {
        let filter = filter_with(|f| f.size = Some(SizeRange::parse("1K-").unwrap()));
        let mut small = entry("small", EntryKind::File);
        small.size = 10;
        let mut big = entry("big", EntryKind::File);
        big.size = 4096;
        assert!(!filter.matches(&small));
        assert!(filter.matches(&big));
    }

    #[test]
    fn a_relative_date_of_zero_days_means_today() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Modified,
                DateRange::parse_age("<=0").unwrap(),
            ))
        });

        let mut today = entry("today", EntryKind::File);
        today.mtime = Some(now - Duration::from_secs(3_600));
        let mut yesterday = entry("yesterday", EntryKind::File);
        yesterday.mtime = Some(now - Duration::from_secs(2 * 86_400));

        assert!(filter.matches_at(&today, now));
        assert!(!filter.matches_at(&yesterday, now));
    }

    #[test]
    fn a_relative_date_range_keeps_a_window_of_days() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Modified,
                DateRange::parse_age("7-30").unwrap(),
            ))
        });

        let mut fresh = entry("fresh", EntryKind::File);
        fresh.mtime = Some(now - Duration::from_secs(86_400));
        let mut inside = entry("inside", EntryKind::File);
        inside.mtime = Some(now - Duration::from_secs(10 * 86_400));
        let mut ancient = entry("ancient", EntryKind::File);
        ancient.mtime = Some(now - Duration::from_secs(60 * 86_400));

        assert!(!filter.matches_at(&fresh, now));
        assert!(filter.matches_at(&inside, now));
        assert!(!filter.matches_at(&ancient, now));
    }

    #[test]
    fn a_relative_date_in_the_future_counts_as_no_age_at_all() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Modified,
                DateRange::parse_age("<=0").unwrap(),
            ))
        });
        let mut ahead = entry("ahead", EntryKind::File);
        ahead.mtime = Some(now + Duration::from_secs(86_400));
        assert!(filter.matches_at(&ahead, now));
    }

    #[test]
    fn a_date_condition_reads_the_field_it_names() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let young = |field| {
            filter_with(|f| {
                f.date = Some(DateCondition::new(
                    field,
                    DateRange::parse_age("<=1").unwrap(),
                ))
            })
        };

        // Created yesterday, last read a month ago: young by `btime`, old by `atime`.
        let mut file = entry("restored", EntryKind::File);
        file.btime = Some(now - Duration::from_secs(86_400));
        file.atime = Some(now - Duration::from_secs(30 * 86_400));
        assert!(young(TimeField::Created).matches_at(&file, now));
        assert!(!young(TimeField::Accessed).matches_at(&file, now));

        // A field this filesystem does not keep must not hide the file.
        let bare = entry("bare", EntryKind::File);
        assert!(young(TimeField::Created).matches_at(&bare, now));
        assert!(young(TimeField::Changed).matches_at(&bare, now));
    }

    #[test]
    fn a_relative_date_that_is_not_a_window_is_reported() {
        assert_eq!(
            DateRange::parse_age("soon"),
            Err(FilterError::BadDateRange("soon".to_string()))
        );
        assert_eq!(
            DateRange::parse_age("30-7"),
            Err(FilterError::BadDateRange("30-7".to_string()))
        );
    }

    #[test]
    fn an_absolute_date_range_compares_epoch_seconds() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Modified,
                DateRange::Absolute {
                    from: Some(10 * 86_400),
                    to: Some(20 * 86_400),
                },
            ))
        });

        let mut inside = entry("inside", EntryKind::File);
        inside.mtime = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(15 * 86_400));
        let mut outside = entry("outside", EntryKind::File);
        outside.mtime = Some(SystemTime::UNIX_EPOCH + Duration::from_secs(25 * 86_400));

        assert!(filter.matches_at(&inside, now));
        assert!(!filter.matches_at(&outside, now));
    }

    #[test]
    fn a_date_condition_on_a_timestamp_the_entry_does_not_carry_is_skipped() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(100 * 86_400);
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Created,
                DateRange::parse_age("<=0").unwrap(),
            ))
        });
        let mut old = entry("old", EntryKind::File);
        old.mtime = Some(SystemTime::UNIX_EPOCH);
        assert!(filter.matches_at(&old, now));
    }

    #[test]
    fn a_three_state_attribute_requires_forbids_or_ignores_a_flag() {
        let mut hidden = entry("hidden", EntryKind::File);
        hidden.flags.insert(EntryFlags::HIDDEN);
        let plain = entry("plain", EntryKind::File);

        let mut filter = filter_with(|_| {});
        assert_eq!(filter.attrs.state(EntryFlags::HIDDEN), AttrState::DontCare);
        assert!(filter.matches(&hidden) && filter.matches(&plain));

        filter.attrs.set(EntryFlags::HIDDEN, AttrState::MustHave);
        assert_eq!(filter.attrs.state(EntryFlags::HIDDEN), AttrState::MustHave);
        assert!(filter.matches(&hidden) && !filter.matches(&plain));

        filter.attrs.set(EntryFlags::HIDDEN, AttrState::MustNotHave);
        assert!(!filter.matches(&hidden) && filter.matches(&plain));

        filter.attrs.set(EntryFlags::HIDDEN, AttrState::DontCare);
        assert!(filter.attrs.is_empty());
    }

    #[test]
    fn attribute_conditions_combine_must_have_with_must_not_have() {
        let filter = filter_with(|f| {
            f.attrs.set(EntryFlags::EXECUTABLE, AttrState::MustHave);
            f.attrs.set(EntryFlags::HIDDEN, AttrState::MustNotHave);
        });

        let mut script = entry("run.sh", EntryKind::File);
        script.flags.insert(EntryFlags::EXECUTABLE);
        let mut hidden_script = script.clone();
        hidden_script.flags.insert(EntryFlags::HIDDEN);

        assert!(filter.matches(&script));
        assert!(!filter.matches(&hidden_script));
        assert!(!filter.matches(&entry("notes.txt", EntryKind::File)));
    }

    #[test]
    fn the_hard_link_condition_is_skipped_while_entries_carry_no_link_count() {
        let filter = filter_with(|f| f.hard_links = Some(HardLinks::MoreThanOne));
        assert!(!filter.is_empty());
        assert!(filter.matches(&entry("main.rs", EntryKind::File)));
    }

    #[test]
    fn the_words_for_a_kind_and_a_timestamp_are_the_ones_people_write() {
        assert_eq!(KindFilter::parse("file"), Some(KindFilter::Files));
        assert_eq!(KindFilter::parse("Folders"), Some(KindFilter::Dirs));
        assert_eq!(KindFilter::parse(""), Some(KindFilter::Both));
        assert_eq!(KindFilter::parse("sockets"), None);
        assert_eq!(TimeField::parse("create"), Some(TimeField::Created));
        assert_eq!(TimeField::parse("atime"), Some(TimeField::Accessed));
        assert_eq!(TimeField::parse(""), Some(TimeField::Modified));
        assert_eq!(TimeField::parse("whenever"), None);
        // Whatever `as_str` writes, `parse` reads back.
        for f in [
            TimeField::Modified,
            TimeField::Created,
            TimeField::Accessed,
            TimeField::Changed,
        ] {
            assert_eq!(TimeField::parse(f.as_str()), Some(f));
        }
    }

    #[test]
    fn the_attribute_letters_read_as_three_states() {
        let a = AttrConditions::parse("x-h").expect("letters");
        assert_eq!(a.state(EntryFlags::EXECUTABLE), AttrState::MustHave);
        assert_eq!(a.state(EntryFlags::HIDDEN), AttrState::MustNotHave);
        assert_eq!(a.state(EntryFlags::SYSTEM), AttrState::DontCare);
        // Order, case, spaces and commas are all allowed; nothing asked for is nothing.
        assert_eq!(AttrConditions::parse("-H, X").unwrap(), a);
        assert!(AttrConditions::parse("").unwrap().is_empty());
        // A letter nothing is named by says which ones are.
        let Err(FilterError::BadAttrs(text, known)) = AttrConditions::parse("q") else {
            panic!("a letter that names no flag has to be refused");
        };
        assert_eq!(text, "q");
        assert!(known.contains('h') && known.contains('x'), "{known}");
    }

    #[test]
    fn describe_names_every_condition_that_is_on() {
        let mut filter = Filter::named("sources");
        filter.masks = Some(masks("*.rs,*.toml"));
        filter.kind = KindFilter::Files;
        filter.size = Some(SizeRange::parse("1K-").unwrap());
        filter.date = Some(DateCondition::new(
            TimeField::Modified,
            DateRange::parse_age("<=30").unwrap(),
        ));
        filter.attrs.set(EntryFlags::HIDDEN, AttrState::MustNotHave);
        filter.hard_links = Some(HardLinks::MoreThanOne);
        assert_eq!(
            filter.describe(),
            "sources: *.rs,*.toml, files, >=1.00 K, mtime<=30d, -h, links>1"
        );
    }

    #[test]
    fn describe_of_an_absolute_date_names_the_calendar_day() {
        let filter = filter_with(|f| {
            f.date = Some(DateCondition::new(
                TimeField::Accessed,
                DateRange::Absolute {
                    from: Some(1_700_000_000),
                    to: None,
                },
            ))
        });
        assert_eq!(filter.describe(), "atime>=2023-11-14");
        assert_eq!(epoch_day(0), "1970-01-01");
    }
}
