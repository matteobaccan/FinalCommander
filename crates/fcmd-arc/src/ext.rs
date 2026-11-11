//! Driving an archiver that is already on the machine.
//!
//! Every other module here reads a format this program implements. This one reads a *table*:
//! a list of command lines, one per operation, for a program somebody else wrote. It is a
//! port of FAR's MultiArc, read from FAR's own repository and written down in
//! `docs/05-multiarc.md`, which is the specification for this file and should be read beside
//! it.
//!
//! The reason to have it: `docs/02-features.md` row 4.1 lists formats we do not read — rar,
//! cab, iso, rpm, cpio, lha, arj — and each one is thousands of lines of Rust if we write the
//! format, or four lines of table if we ask a program that already has it. MultiArc is thirty
//! years of experience in doing the second thing, and the shape it arrived at is a template
//! language with typed holes.
//!
//! What is taken, and what is deliberately not:
//!
//! - **The keys.** Fifteen named operations ([`Op`]), each one a command line. An operation
//!   this tool cannot do **has no line**, and that is how the table doubles as the list of
//!   what the tool can do: there is no separate declaration to keep in step with it.
//! - **The holes.** `%A` the archive, `%L` a file holding the names to work on, `%P` the
//!   password, `%R` the folder inside the archive, `%W` a temporary folder, `%S` the extra
//!   switches somebody typed, `%E<n>` the highest exit code that still counts as success.
//! - **`{ }`.** A part of the line in braces is kept only if a hole inside it came out with
//!   something. `{-p%P}` disappears when there is no password rather than being passed empty,
//!   which is what lets one line cover every combination (`docs/05-multiarc.md` §4).
//! - **Not `%F`/`%f`.** MultiArc re-runs the archiver until the names are used up, with a
//!   command-line length limit, because DOS had one. A list file has no limit and no name a
//!   program could mistake for a switch, so `%L` is the only way names are passed here.
//! - **Not the positional column masks.** MultiArc has two listing grammars; the older one
//!   claims columns by letter and fails silently when a name overflows its column. Only the
//!   newer one is here: a regular expression with named captures.
//! - **No shell, ever.** MultiArc writes command *lines* because Windows passes a line. Here
//!   a line is a template for an **argv array**: it is split on spaces once, while filling it
//!   in, and the values that come out of holes are never split again. A name with a space in
//!   it is one argument, and nothing quotes anything. This is the one place in the program
//!   where a quoting bug would stop being cosmetic, so there is no quoting.
//!
//! The regular expressions are Rust's `regex`, not PCRE: no backreferences and no lookaround.
//! A pattern copied out of a MultiArc `custom.ini` that uses either will fail to compile, and
//! say so, rather than quietly matching nothing.

use std::collections::BTreeMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use fcmd_core::CancelToken;
use regex::Regex;

use crate::archive::days_from_civil;

/// What an external archiver can be asked to do, named as MultiArc names it
/// (`plugins/multiarc/global.cpp:10`, plus `List` from `custom.ini`).
///
/// Fourteen of these are MultiArc's commands and `List` is its listing command; its other two
/// keys, `AllFilesMask` and `DefExt`, are settings rather than operations and live on
/// [`Tool`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Op {
    /// Print the contents, for [`Listing`] to read.
    List,
    Extract,
    ExtractWithoutPath,
    Test,
    Delete,
    Add,
    AddRecurse,
    Move,
    MoveRecurse,
    Comment,
    CommentFiles,
    /// Turn the archive into a program that unpacks itself.
    Sfx,
    /// Refuse further changes to the archive.
    Lock,
    /// Add recovery records.
    Protect,
    /// Use them.
    Recover,
}

impl Op {
    /// The name this operation has in a table, which is MultiArc's own spelling so a table can
    /// be copied across with the keys left alone.
    pub fn key(self) -> &'static str {
        match self {
            Op::List => "List",
            Op::Extract => "Extract",
            Op::ExtractWithoutPath => "ExtractWithoutPath",
            Op::Test => "Test",
            Op::Delete => "Delete",
            Op::Add => "Add",
            Op::AddRecurse => "AddRecurse",
            Op::Move => "Move",
            Op::MoveRecurse => "MoveRecurse",
            Op::Comment => "Comment",
            Op::CommentFiles => "CommentFiles",
            Op::Sfx => "SFX",
            Op::Lock => "Lock",
            Op::Protect => "Protect",
            Op::Recover => "Recover",
        }
    }

    pub const ALL: [Op; 15] = [
        Op::List,
        Op::Extract,
        Op::ExtractWithoutPath,
        Op::Test,
        Op::Delete,
        Op::Add,
        Op::AddRecurse,
        Op::Move,
        Op::MoveRecurse,
        Op::Comment,
        Op::CommentFiles,
        Op::Sfx,
        Op::Lock,
        Op::Protect,
        Op::Recover,
    ];

    /// The operation named by a key in a table, whatever case it was written in.
    pub fn of_key(key: &str) -> Option<Op> {
        Op::ALL
            .into_iter()
            .find(|op| op.key().eq_ignore_ascii_case(key))
    }

    /// Does this operation change the archive?
    ///
    /// Extracting writes files, but beside the archive and not into it, so it is on the
    /// reading side of the line — which is the line `docs/05-multiarc.md` §9 draws: a program
    /// driven from a table may do the four that answer no here, and never the others.
    pub fn writes(self) -> bool {
        !matches!(
            self,
            Op::List | Op::Test | Op::Extract | Op::ExtractWithoutPath
        )
    }
}

/// The bytes at the front of a file that prove which format it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Magic {
    pub bytes: Vec<u8>,
    /// Where to look. MultiArc's `IDPos`; `None` means at the very start.
    pub at: Option<usize>,
}

impl Magic {
    fn matches(&self, head: &[u8]) -> bool {
        match self.at {
            Some(at) => head
                .get(at..at + self.bytes.len())
                .is_some_and(|window| window == self.bytes),
            None => head.starts_with(&self.bytes),
        }
    }
}

/// Where a listing's rows begin or end: a line of dashes, a header, whatever the tool prints.
///
/// MultiArc's `Start` and `End`. A leading `^` anchors the text to the start of a line;
/// without it the text is looked for anywhere in the line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Anchor {
    text: String,
    at_line_start: bool,
}

impl Anchor {
    pub fn new(pattern: &str) -> Anchor {
        match pattern.strip_prefix('^') {
            Some(rest) => Anchor {
                text: rest.to_string(),
                at_line_start: true,
            },
            None => Anchor {
                text: pattern.to_string(),
                at_line_start: false,
            },
        }
    }

    fn matches(&self, line: &str) -> bool {
        if self.at_line_start {
            line.starts_with(&self.text)
        } else {
            line.contains(&self.text)
        }
    }
}

/// How to read what the tool printed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Listing {
    /// Rows begin after the line this matches. Without it they begin at the first line.
    pub start: Option<Anchor>,
    /// And stop at the line this matches, which is not read as a row.
    pub end: Option<Anchor>,
    /// Lines to drop outright, wherever they are. MultiArc's `IgnoreString0..N`.
    pub skip: Vec<Anchor>,
    /// Tried in order; the first that matches a line wins. The fields are named captures:
    /// `name` (required), `size`, `packedSize`, `attr`, `crc`, and `mYear` `mMonth` `mDay`
    /// `mHour` `mMin` `mSec` for the modification time.
    pub rows: Vec<String>,
    /// The letter that means "this row is a directory" when it is in the `attr` capture.
    pub dir_attr: Option<char>,
}

/// One row of a listing, as the table's patterns read it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Row {
    pub name: String,
    pub size: Option<u64>,
    pub packed: Option<u64>,
    pub attr: Option<String>,
    pub crc: Option<String>,
    pub mtime: Option<SystemTime>,
    pub is_dir: bool,
}

/// A table with its patterns compiled: what a listing is read with.
#[derive(Debug)]
pub struct Reader {
    rows: Vec<Regex>,
    start: Option<Anchor>,
    end: Option<Anchor>,
    skip: Vec<Anchor>,
    dir_attr: Option<char>,
}

impl Listing {
    /// Compile the patterns once, so a listing of ten thousand rows does not compile them
    /// ten thousand times — and so a table with a bad pattern in it is refused before any
    /// program is run.
    pub fn reader(&self) -> Result<Reader, ExtError> {
        let mut rows = Vec::with_capacity(self.rows.len());
        for pattern in &self.rows {
            let compiled = Regex::new(pattern).map_err(|e| ExtError::Pattern {
                pattern: pattern.clone(),
                reason: e.to_string(),
            })?;
            if !compiled.capture_names().flatten().any(|n| n == "name") {
                return Err(ExtError::Pattern {
                    pattern: pattern.clone(),
                    reason: "it has no (?<name>…) capture, so no row would have a name".to_string(),
                });
            }
            rows.push(compiled);
        }
        Ok(Reader {
            rows,
            start: self.start.clone(),
            end: self.end.clone(),
            skip: self.skip.clone(),
            dir_attr: self.dir_attr,
        })
    }
}

impl Reader {
    /// Read what the tool printed into rows.
    ///
    /// A line that matches no pattern is dropped. That is on purpose: every tool prints a
    /// banner, a summary and sometimes a warning, and a listing that refused to parse because
    /// of the copyright line would be a listing that never works.
    pub fn read(&self, text: &str) -> Vec<Row> {
        let mut out = Vec::new();
        let mut started = self.start.is_none();
        for line in text.lines() {
            if !started {
                started = self.start.as_ref().is_some_and(|a| a.matches(line));
                continue;
            }
            if self.end.as_ref().is_some_and(|a| a.matches(line)) {
                break;
            }
            if self.skip.iter().any(|a| a.matches(line)) {
                continue;
            }
            if let Some(row) = self.row(line) {
                out.push(row);
            }
        }
        out
    }

    fn row(&self, line: &str) -> Option<Row> {
        let found = self
            .rows
            .iter()
            .find_map(|pattern| pattern.captures(line))?;
        let text = |name: &str| found.name(name).map(|m| m.as_str().trim().to_string());
        let number = |name: &str| {
            text(name).and_then(|t| {
                // Tools print thousands separators, and a column that is there but empty is
                // not a zero: a 7z row inside a solid block has no compressed size of its own.
                let digits: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
                (!digits.is_empty()).then(|| digits.parse().ok())?
            })
        };
        let name = text("name")?;
        if name.is_empty() {
            return None;
        }
        let attr = text("attr");
        let is_dir = match (self.dir_attr, &attr) {
            (Some(letter), Some(attr)) => attr.contains(letter),
            _ => false,
        };
        Some(Row {
            name,
            size: number("size"),
            packed: number("packedSize"),
            crc: text("crc").filter(|c| !c.is_empty()),
            mtime: self.when(&found),
            attr,
            is_dir,
        })
    }

    /// The modification time out of the six captures that carry it. All of the date is needed
    /// for any of it to mean anything; the time of day is optional and defaults to midnight.
    fn when(&self, found: &regex::Captures<'_>) -> Option<SystemTime> {
        let part = |name: &str| found.name(name)?.as_str().trim().parse::<i64>().ok();
        let (year, month, day) = (part("mYear")?, part("mMonth")?, part("mDay")?);
        let (hour, min, sec) = (
            part("mHour").unwrap_or(0),
            part("mMin").unwrap_or(0),
            part("mSec").unwrap_or(0),
        );
        if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
            return None;
        }
        let days = days_from_civil(year, month as u32, day as u32)?;
        let seconds = days * 86_400 + hour * 3_600 + min * 60 + sec;
        // Before 1970 a listing is readable and the time is not: the row keeps its name and
        // loses its date rather than the whole listing failing.
        u64::try_from(seconds)
            .ok()
            .map(|s| SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(s))
    }
}

// ── the template language ─────────────────────────────────────────────────────────────────

/// What the holes in a command line are filled from.
///
/// Everything is optional, and a hole whose value is missing comes out empty — which is what
/// makes the braces useful: `{-p%P}` with no password in hand disappears entirely.
#[derive(Debug, Default, Clone, Copy)]
pub struct Fill<'a> {
    /// `%A`: the archive the command works on.
    pub archive: Option<&'a Path>,
    /// `%L`: a file holding one name per line, which is how every name reaches the tool.
    pub list_file: Option<&'a Path>,
    /// `%P`: the password.
    pub password: Option<&'a str>,
    /// `%R`: where the panel is inside the archive, without a leading slash.
    pub inner: Option<&'a str>,
    /// `%W`: a temporary folder the tool may work in, with a trailing separator.
    pub temp: Option<&'a Path>,
    /// `%S`: extra switches somebody typed in the dialog. Split on spaces, because that is
    /// the only thing that can be done with a string of switches and no shell to read it.
    pub switches: Option<&'a str>,
}

/// A command line filled in: the arguments to pass, and how to read what comes back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub args: Vec<OsString>,
    /// `%E<n>`: the highest exit code that still counts as success. Without it, only 0 does —
    /// which is right for most tools and wrong for the ones that say "finished, with
    /// warnings" with a 1.
    pub success_up_to: i32,
}

/// Which value a hole takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hole {
    Archive,
    ListFile,
    Password,
    Inner,
    Temp,
    Switches,
}

impl Hole {
    fn of_letter(letter: char) -> Option<Hole> {
        Some(match letter {
            'A' => Hole::Archive,
            'L' => Hole::ListFile,
            'P' => Hole::Password,
            'R' => Hole::Inner,
            'W' => Hole::Temp,
            'S' => Hole::Switches,
            _ => return None,
        })
    }
}

/// Fill in a command line.
///
/// The line is split into arguments on spaces *as it is read*, so a value that contains a
/// space is still one argument: that is the whole difference between this and MultiArc, which
/// hands a line to Windows and lets it split. `%%` is a literal `%`.
pub fn fill(template: &str, values: &Fill<'_>) -> Result<Line, ExtError> {
    let mut args: Vec<OsString> = Vec::new();
    let mut word = OsString::new();
    let mut word_started = false;
    let mut success_up_to = 0i32;

    // Inside braces, the words are held back until the closing brace says whether anything
    // came out of the holes in them.
    let mut group: Option<(Vec<OsString>, bool)> = None;

    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' | '\t' => flush(&mut args, &mut group, &mut word, &mut word_started),
            '{' => {
                if group.is_some() {
                    return Err(ExtError::Template {
                        template: template.to_string(),
                        reason: "a `{` inside another one: braces do not nest".to_string(),
                    });
                }
                flush(&mut args, &mut group, &mut word, &mut word_started);
                group = Some((Vec::new(), false));
            }
            '}' => {
                flush(&mut args, &mut group, &mut word, &mut word_started);
                let Some((held, any)) = group.take() else {
                    return Err(ExtError::Template {
                        template: template.to_string(),
                        reason: "a `}` with no `{` in front of it".to_string(),
                    });
                };
                // The rule from `docs/05-multiarc.md` §4: the braces are kept only if a hole
                // inside them came out with something.
                if any {
                    args.extend(held);
                }
            }
            '%' => {
                let Some(letter) = chars.next() else {
                    return Err(ExtError::Template {
                        template: template.to_string(),
                        reason: "a `%` at the end, with nothing after it".to_string(),
                    });
                };
                if letter == '%' {
                    word.push("%");
                    word_started = true;
                    continue;
                }
                if letter == 'E' {
                    let mut digits = String::new();
                    while chars.peek().is_some_and(char::is_ascii_digit) {
                        digits.push(chars.next().unwrap_or_default());
                    }
                    success_up_to = digits.parse().map_err(|_| ExtError::Template {
                        template: template.to_string(),
                        reason: "`%E` wants the exit code after it, as in `%E1`".to_string(),
                    })?;
                    continue;
                }
                let Some(hole) = Hole::of_letter(letter) else {
                    return Err(ExtError::Template {
                        template: template.to_string(),
                        reason: format!("`%{letter}` is not a hole this program fills"),
                    });
                };
                // Modifiers follow the letter with no space between. Three of MultiArc's nine
                // still mean something here; the other six are read and ignored, so a table
                // copied out of a `custom.ini` still works — see `modifiers`.
                let mut mods = String::new();
                while chars.peek().is_some_and(|c| MODIFIERS.contains(*c)) {
                    mods.push(chars.next().unwrap_or_default());
                }
                word_started = true;
                let filled = put(hole, &mods, values, &mut word, &mut args, &mut group);
                if filled {
                    if let Some((_, any)) = &mut group {
                        *any = true;
                    }
                }
            }
            other => {
                word.push(other.to_string());
                word_started = true;
            }
        }
    }
    flush(&mut args, &mut group, &mut word, &mut word_started);
    if group.is_some() {
        return Err(ExtError::Template {
            template: template.to_string(),
            reason: "a `{` that is never closed".to_string(),
        });
    }
    Ok(Line {
        args,
        success_up_to,
    })
}

/// The word that has been built so far becomes an argument — unless nothing was put into it,
/// in which case there is no argument, which is how `{-p%P}` with no password leaves nothing
/// behind. Inside braces the argument is held back until the closing brace decides.
fn flush(
    args: &mut Vec<OsString>,
    group: &mut Option<(Vec<OsString>, bool)>,
    word: &mut OsString,
    started: &mut bool,
) {
    if *started && !word.is_empty() {
        match group {
            Some((held, _)) => held.push(std::mem::take(word)),
            None => args.push(std::mem::take(word)),
        }
    } else {
        word.clear();
    }
    *started = false;
}

/// Put one hole's value into the word being built. Answers whether anything went in, which is
/// what the braces are decided on.
fn put(
    hole: Hole,
    mods: &str,
    values: &Fill<'_>,
    word: &mut OsString,
    args: &mut Vec<OsString>,
    group: &mut Option<(Vec<OsString>, bool)>,
) -> bool {
    match hole {
        Hole::Password => match values.password {
            Some(p) if !p.is_empty() => {
                word.push(p);
                true
            }
            _ => false,
        },
        Hole::Inner => match values.inner {
            Some(r) if !r.is_empty() => {
                word.push(slashed(r, mods));
                true
            }
            _ => false,
        },
        Hole::Archive => match values.archive {
            Some(path) => path_into(path, mods, word),
            None => false,
        },
        Hole::ListFile => match values.list_file {
            Some(path) => path_into(path, mods, word),
            None => false,
        },
        Hole::Temp => match values.temp {
            Some(path) => {
                // With the trailing separator, as MultiArc has it: the hole is written glued
                // to a switch (`-w%W`) and the tool wants a folder, not a name.
                let mut with_sep = path.as_os_str().to_os_string();
                with_sep.push(std::path::MAIN_SEPARATOR_STR);
                let text = with_sep.to_string_lossy().into_owned();
                word.push(slashed(&text, mods));
                true
            }
            None => false,
        },
        Hole::Switches => {
            // Several switches in one string, and no shell to split them: they are split here
            // on spaces, each becoming its own argument. A switch whose value has a space in
            // it cannot be written this way, which is a limit of the field and not of the
            // tool — the table is the place for anything that complicated.
            let Some(text) = values.switches.filter(|s| !s.trim().is_empty()) else {
                return false;
            };
            let mut pieces = text.split_whitespace();
            let Some(first) = pieces.next() else {
                return false;
            };
            word.push(first);
            for piece in pieces {
                let finished = std::mem::replace(word, OsString::from(piece));
                match group {
                    Some((held, _)) => held.push(finished),
                    None => args.push(finished),
                }
            }
            true
        }
    }
}

/// A path into a word, with the modifiers that change which part of it goes in.
fn path_into(path: &Path, mods: &str, word: &mut OsString) -> bool {
    let piece: &OsStr = if mods.contains('W') {
        // The name only, without the path it is in.
        path.file_name().unwrap_or(path.as_os_str())
    } else if mods.contains('P') {
        // The path only, without the name.
        path.parent().map(Path::as_os_str).unwrap_or(OsStr::new(""))
    } else {
        path.as_os_str()
    };
    if piece.is_empty() {
        return false;
    }
    match piece.to_str() {
        // `S` asks for `/` where the system writes `\`, which is a question only Windows
        // raises. A path that is not valid Unicode cannot be rewritten without deciding what
        // its bytes mean, so it goes in as it is: on the system where `S` matters, paths are
        // always valid Unicode.
        Some(text) => word.push(slashed(text, mods)),
        None => word.push(piece),
    }
    true
}

/// `S`: forward slashes instead of backslashes.
fn slashed(text: &str, mods: &str) -> String {
    if mods.contains('S') {
        text.replace('\\', "/")
    } else {
        text.to_string()
    }
}

/// The modifiers MultiArc defines, and what each one does here.
///
/// | | MultiArc | here |
/// | --- | --- | --- |
/// | `S` | `/` instead of `\` | the same |
/// | `W` | the name without its path | the same |
/// | `P` | the path without the name | the same |
/// | `Q` `q` | quote names with spaces, or all of them | **nothing**: an argv array needs no quoting, and a quote added here would become part of the name |
/// | `M` `N` | pass a folder as `folder\*.*`, or as `folder` | **nothing**: they shape the names on the command line, and no name is ever on the command line here — they are in the list file |
/// | `*` | override `AllFilesMask` for this command | **nothing**: it applies to `%F`, which is not here |
/// | `A` | encode in ANSI rather than OEM | **nothing**: everything written here is UTF-8, which is what the list file says it is |
///
/// The six that do nothing are read and discarded rather than refused, so a table copied out
/// of a MultiArc `custom.ini` works without being edited first.
pub const MODIFIERS: &str = "QqSMN*WPA";

// ── the table ─────────────────────────────────────────────────────────────────────────────

/// One external archiver: the program, what it claims to handle, what it can do, and how to
/// read what it prints.
#[derive(Debug, Clone)]
pub struct Tool {
    /// What this table is called. A table the user writes under the same name replaces the
    /// one shipped here, which is MultiArc's `custom_user.ini` rule: what is built in is a
    /// default, not a fixed list.
    pub name: String,
    /// The program to run, found on the `PATH`. Never a shell line.
    pub program: String,
    /// The extensions it claims, lowercase and without the dot.
    pub extensions: Vec<String>,
    /// The bytes that prove it, when the format has any.
    pub magic: Option<Magic>,
    /// MultiArc's `IDOnly`: trust the bytes and pay no attention to the name.
    pub magic_only: bool,
    /// A line per operation. An operation with no line is one this tool cannot do, and that
    /// is the only declaration of what it can: see [`Tool::can`].
    pub commands: BTreeMap<Op, String>,
    /// MultiArc's `AllFilesMask`: what this tool spells "everything".
    pub all_files: Option<String>,
    /// MultiArc's `DefExt`: the extension a new archive of this tool's format gets.
    pub default_extension: Option<String>,
    pub listing: Listing,
}

impl Tool {
    /// Can this tool do that? Answered by whether the table has a line for it.
    pub fn can(&self, op: Op) -> bool {
        self.commands
            .get(&op)
            .is_some_and(|line| !line.trim().is_empty())
    }

    /// The arguments for one operation, filled in.
    pub fn line(&self, op: Op, values: &Fill<'_>) -> Result<Line, ExtError> {
        let template = self
            .commands
            .get(&op)
            .filter(|line| !line.trim().is_empty())
            .ok_or(ExtError::Cannot {
                tool: self.name.clone(),
                op,
            })?;
        fill(template, values)
    }

    /// Does this tool claim a file called this, starting with these bytes?
    ///
    /// The bytes decide when there are any and `magic_only` says to trust them; otherwise the
    /// name has to agree as well, so a `.zip` is not handed to the rar table because both
    /// happen to be read by the same program.
    pub fn claims(&self, name: &str, head: &[u8]) -> bool {
        let by_magic = self.magic.as_ref().map(|m| m.matches(head));
        if self.magic_only {
            return by_magic.unwrap_or(false);
        }
        let lower = name.to_ascii_lowercase();
        let by_name = self
            .extensions
            .iter()
            .any(|ext| lower.ends_with(&format!(".{ext}")));
        match by_magic {
            Some(magic) => by_name && magic,
            None => by_name,
        }
    }

    /// Is the program there? A table for a tool nobody installed is not an error, it is a
    /// format this machine cannot open — which is a different message and a different answer
    /// in a dialog.
    pub fn found(&self) -> Option<PathBuf> {
        which(&self.program)
    }
}

/// Where a program is, by the same rule a shell uses, and nothing else: no current folder, no
/// guessing, no shell.
///
/// On Windows a name with no extension is tried against `PATHEXT`, because `7z` on the `PATH`
/// is `7z.exe` and `Command` would not find it on its own.
pub fn which(program: &str) -> Option<PathBuf> {
    let direct = Path::new(program);
    if direct.components().count() > 1 {
        return direct.is_file().then(|| direct.to_path_buf());
    }
    let extensions: Vec<String> = if cfg!(windows) {
        std::env::var("PATHEXT")
            .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_string())
            .split(';')
            .filter(|e| !e.is_empty())
            .map(str::to_string)
            .collect()
    } else {
        Vec::new()
    };
    for folder in std::env::split_paths(&std::env::var_os("PATH")?) {
        let candidate = folder.join(program);
        if candidate.is_file() {
            return Some(candidate);
        }
        for extension in &extensions {
            let with = folder.join(format!("{program}{extension}"));
            if with.is_file() {
                return Some(with);
            }
        }
    }
    None
}

/// What can go wrong driving somebody else's program.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExtError {
    #[error("`{tool}` has no command for {}", op.key())]
    Cannot { tool: String, op: Op },
    #[error("{reason} — in `{template}`")]
    Template { template: String, reason: String },
    #[error("{reason} — in `{pattern}`")]
    Pattern { pattern: String, reason: String },
    /// The table claims the format and the program is not on this machine. The name of the
    /// program is the whole message: it is the one thing the reader can act on, and
    /// [`crate::needs_program`] reads it back out to say so in the reader's own language.
    #[error(
        "{}{program}{}",
        crate::NEEDS_PROGRAM_BEFORE,
        crate::NEEDS_PROGRAM_AFTER
    )]
    NotInstalled { program: String },
    #[error("`{program}` could not be run: {reason}")]
    Io { program: String, reason: String },
    /// It ran and said no. The exit code and whatever it printed on its standard error, which
    /// is where every one of these programs puts its reason.
    #[error("`{program}` failed{}{}", match code { Some(c) => format!(" ({c})"), None => " (killed)".to_string() }, if reason.is_empty() { String::new() } else { format!(": {reason}") })]
    Failed {
        program: String,
        code: Option<i32>,
        reason: String,
    },
    #[error("cancelled")]
    Cancelled,
}

// ── running it ────────────────────────────────────────────────────────────────────────────

/// What a program printed and how it ended.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ran {
    /// `None` when it was killed rather than returning.
    pub code: Option<i32>,
    pub out: String,
    pub err: String,
}

/// How a program is run.
///
/// Behind a trait for two reasons. The parsing can then be tested against output a real
/// program really printed, without that program having to be installed wherever the tests run
/// — which is the difference between a test that checks our grammar and a test that checks
/// somebody's package manager. And spawning a child is the one thing in this crate that
/// reaches outside the process, so it is worth having exactly one place that does it.
pub trait Run: std::fmt::Debug + Send + Sync {
    /// `cwd` is where the program runs, and for an extraction it is where the files land:
    /// none of these tools takes an output folder the same way, and all of them write into
    /// the folder they were started in. It is MultiArc's answer too.
    fn run(
        &self,
        program: &Path,
        args: &[OsString],
        cwd: Option<&Path>,
        cancel: &CancelToken,
    ) -> Result<Ran, ExtError>;
}

/// The real one: a child process, no shell, output captured, `Esc` kills it.
#[derive(Debug, Clone, Copy, Default)]
pub struct Spawn;

impl Run for Spawn {
    fn run(
        &self,
        program: &Path,
        args: &[OsString],
        cwd: Option<&Path>,
        cancel: &CancelToken,
    ) -> Result<Ran, ExtError> {
        use std::io::Read as _;
        use std::process::{Command, Stdio};

        let name = program.display().to_string();
        let io = |e: std::io::Error| ExtError::Io {
            program: name.clone(),
            reason: e.to_string(),
        };
        // No shell, and nothing on the standard input: a program that decides to ask a
        // question would otherwise wait for an answer nobody can give it, for ever.
        let mut command = Command::new(program);
        if let Some(cwd) = cwd {
            command.current_dir(cwd);
        }
        let mut child = command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(io)?;

        // Both pipes are drained on threads of their own. A program that fills one while we
        // read the other would stop, waiting for a reader, and so would we: the deadlock that
        // every first attempt at this has.
        let mut stdout = child.stdout.take();
        let mut stderr = child.stderr.take();
        let out_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(pipe) = stdout.as_mut() {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });
        let err_thread = std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(pipe) = stderr.as_mut() {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        });

        let mut killed = false;
        let status = loop {
            if cancel.is_cancelled() && !killed {
                // `Esc` against a child process can only mean this. What it was writing is the
                // caller's to clean up, which is why nothing driven this way ever writes
                // *into* an archive (`docs/05-multiarc.md` §9).
                let _ = child.kill();
                killed = true;
            }
            match child.try_wait().map_err(io)? {
                Some(status) => break status,
                None => std::thread::sleep(std::time::Duration::from_millis(25)),
            }
        };
        let out = out_thread.join().unwrap_or_default();
        let err = err_thread.join().unwrap_or_default();
        if killed {
            return Err(ExtError::Cancelled);
        }
        Ok(Ran {
            code: status.code(),
            // These programs print in whatever the terminal's encoding is and a name can be
            // anything; lossy is the only honest reading, and a replacement character in one
            // row is better than a listing that failed.
            out: String::from_utf8_lossy(&out).into_owned(),
            err: String::from_utf8_lossy(&err).into_owned(),
        })
    }
}

// ── asking a tool what is in an archive ───────────────────────────────────────────────────

impl Tool {
    /// Run `List` and read what it printed into rows.
    ///
    /// The program's own exit code is judged against `%E<n>`: most of these say 0 for "fine"
    /// and something else for "I could not", but a few count 1 as "finished, with warnings",
    /// which is why the table gets to say so.
    pub fn list(
        &self,
        archive: &Path,
        runner: &dyn Run,
        cancel: &CancelToken,
    ) -> Result<Vec<Row>, ExtError> {
        let program = self.found().ok_or_else(|| ExtError::NotInstalled {
            program: self.program.clone(),
        })?;
        let reader = self.listing.reader()?;
        let line = self.line(
            Op::List,
            &Fill {
                archive: Some(archive),
                ..Fill::default()
            },
        )?;
        let ran = runner.run(&program, &line.args, None, cancel)?;
        let bad = match ran.code {
            Some(code) => code > line.success_up_to,
            // Killed, and not by us — `Spawn` reports cancellation as cancellation.
            None => true,
        };
        if bad {
            return Err(ExtError::Failed {
                program: self.program.clone(),
                code: ran.code,
                // Some of them put the reason on the standard output instead, so the one that
                // has something is the one shown, and the first line of it is enough: the
                // rest is a usage screen.
                reason: first_line(if ran.err.trim().is_empty() {
                    &ran.out
                } else {
                    &ran.err
                }),
            });
        }
        Ok(reader.read(&ran.out))
    }
}

impl Tool {
    /// Extract these entries into `into`, by running `Extract`.
    ///
    /// The names go in a file, one per line, and never on the command line: no length limit,
    /// and no name a program could mistake for a switch (`docs/05-multiarc.md` §3). The file
    /// is written in UTF-8, which is what the shipped table tells `7z` to expect.
    ///
    /// `into` is where the program runs, so it is where the files land — with the paths they
    /// have inside the archive, which is what `Extract` means as against
    /// `ExtractWithoutPath`.
    pub fn extract(
        &self,
        archive: &Path,
        names: &[String],
        into: &Path,
        runner: &dyn Run,
        cancel: &CancelToken,
    ) -> Result<(), ExtError> {
        let program = self.found().ok_or_else(|| ExtError::NotInstalled {
            program: self.program.clone(),
        })?;
        let list = self.write_names(names, into)?;
        let line = self.line(
            Op::Extract,
            &Fill {
                archive: Some(archive),
                list_file: Some(&list),
                ..Fill::default()
            },
        )?;
        let ran = runner.run(&program, &line.args, Some(into), cancel);
        // The list file is ours and goes whatever happened, so it cannot be mistaken for
        // something that came out of the archive.
        let _ = std::fs::remove_file(&list);
        let ran = ran?;
        let bad = match ran.code {
            Some(code) => code > line.success_up_to,
            None => true,
        };
        if bad {
            return Err(ExtError::Failed {
                program: self.program.clone(),
                code: ran.code,
                reason: first_line(if ran.err.trim().is_empty() {
                    &ran.out
                } else {
                    &ran.err
                }),
            });
        }
        Ok(())
    }
}

impl Tool {
    /// The list file: one name per line, in UTF-8, in `folder`.
    fn write_names(&self, names: &[String], folder: &Path) -> Result<PathBuf, ExtError> {
        let list = folder.join(".fcmd-names");
        let mut text = String::new();
        for name in names {
            // Inside an archive the separator is `/` and a leading one is ours, not the
            // archive's: what goes in the file is the name the tool printed.
            text.push_str(name.trim_start_matches('/'));
            text.push('\n');
        }
        std::fs::write(&list, text).map_err(|e| ExtError::Io {
            program: self.program.clone(),
            reason: format!("{}: {e}", list.display()),
        })?;
        Ok(list)
    }

    /// Run `Test` over these entries and say which of them the program complained about.
    ///
    /// An empty answer is an archive that tested whole: the program ended with a code the
    /// table counts as success. Otherwise each line it wrote on its standard error — where
    /// every one of these puts a complaint; its standard output, where it wrote nothing
    /// there — is looked through for one of the names it was given, and comes back beside
    /// that name; a line that names nothing comes back beside an empty name, which is a
    /// complaint about the archive. A program that fails and says nothing at all is one
    /// such line, with its exit code.
    ///
    /// `work` is a folder of the caller's for the list file, and where the program runs.
    pub fn test(
        &self,
        archive: &Path,
        names: &[String],
        work: &Path,
        runner: &dyn Run,
        cancel: &CancelToken,
    ) -> Result<Vec<(String, String)>, ExtError> {
        let program = self.found().ok_or_else(|| ExtError::NotInstalled {
            program: self.program.clone(),
        })?;
        let list = self.write_names(names, work)?;
        let line = self.line(
            Op::Test,
            &Fill {
                archive: Some(archive),
                list_file: Some(&list),
                ..Fill::default()
            },
        )?;
        let ran = runner.run(&program, &line.args, Some(work), cancel);
        let _ = std::fs::remove_file(&list);
        let ran = ran?;
        if ran.code.is_some_and(|code| code <= line.success_up_to) {
            return Ok(Vec::new());
        }
        let known: std::collections::HashSet<&str> = names
            .iter()
            .map(|name| name.trim_start_matches('/'))
            .collect();
        let complaints: Vec<&str> = if ran.err.trim().is_empty() {
            // On the standard output a complaint sits among the banner and the totals, so
            // only the lines that call themselves one are taken.
            ran.out
                .lines()
                .map(str::trim)
                .filter(|line| {
                    let lower = line.to_ascii_lowercase();
                    lower.contains("error") || lower.contains("failed") || lower.contains("corrupt")
                })
                .collect()
        } else {
            ran.err
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .collect()
        };
        let mut bad: Vec<(String, String)> = Vec::new();
        for complaint in complaints {
            let name = named_in(complaint, &known).unwrap_or_default().to_string();
            // One line per entry, and one per distinct thing said about the archive.
            let said = bad
                .iter()
                .any(|(n, why)| *n == name && (!name.is_empty() || why == complaint));
            if !said {
                bad.push((name, complaint.to_string()));
            }
        }
        if bad.is_empty() {
            bad.push((
                String::new(),
                match ran.code {
                    Some(code) => format!("`{}` ended with code {code}", self.program),
                    None => format!("`{}` was killed", self.program),
                },
            ));
        }
        Ok(bad)
    }
}

/// The entry a line of a program's complaint is about: the longest of the known names
/// that stands in it as whole words.
///
/// No two programs word a complaint the same way — `ERROR: CRC Failed : src/a.bin`,
/// `src/a.bin - checksum error`, `Checksum error in src/a.bin. The file is corrupt` — so
/// the line is not parsed at all: every run of its words is looked up among the names the
/// program was given, which finds a name with spaces in it and does not care what is
/// written around it. A full stop, a colon or a quote stuck to the end of the name is
/// tried without.
fn named_in<'a>(line: &str, known: &std::collections::HashSet<&'a str>) -> Option<&'a str> {
    // Where a word starts, and where one ends.
    let mut starts = Vec::new();
    let mut ends = Vec::new();
    let mut inside = false;
    for (at, c) in line.char_indices() {
        if c.is_whitespace() {
            if inside {
                ends.push(at);
            }
            inside = false;
        } else if !inside {
            starts.push(at);
            inside = true;
        }
    }
    if inside {
        ends.push(line.len());
    }
    let mut best: Option<&'a str> = None;
    for &start in &starts {
        for &end in ends.iter().filter(|&&end| end > start) {
            let words = &line[start..end];
            let bare = words
                .trim_end_matches(['.', ':', ',', ';'])
                .trim_matches(|c| matches!(c, '"' | '\'' | '`'))
                .trim_end_matches(['.', ':', ',', ';']);
            // On Windows the programs write the separator their own way: `src\a.bin` is
            // the entry `src/a.bin`.
            let (words, bare) = (words.replace('\\', "/"), bare.replace('\\', "/"));
            for candidate in [words.as_str(), bare.as_str()] {
                if let Some(name) = known.get(candidate) {
                    if best.is_none_or(|found| name.len() > found.len()) {
                        best = Some(name);
                    }
                }
            }
        }
    }
    best
}

/// The first line with anything on it, trimmed — a program's reason, without its banner.
fn first_line(text: &str) -> String {
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_string()
}

/// Which table claims a file called this, starting with these bytes.
///
/// First match wins, and the shipped tables come last, so a table the user wrote under the
/// same name is the one that answers — MultiArc's `custom_user.ini` rule (§6).
pub fn tool_for<'a>(tools: &'a [Tool], name: &str, head: &[u8]) -> Option<&'a Tool> {
    tools.iter().find(|tool| tool.claims(name, head))
}

/// The program a table needs for this file and has not got.
///
/// Answered separately from opening, because the message is different in kind: "install `7z`"
/// is something a reader can act on, and it has to be able to say so before anything has
/// failed. `None` means either that no table claims the file or that the program is there.
pub fn missing_program(tools: &[Tool], name: &str, head: &[u8]) -> Option<String> {
    let tool = tool_for(tools, name, head)?;
    tool.found().is_none().then(|| tool.program.clone())
}

// ── what is shipped ───────────────────────────────────────────────────────────────────────

/// The shipped tables, built once — for the questions asked often enough that building the
/// list every time would be silly.
fn shipped() -> &'static [Tool] {
    static ONCE: std::sync::OnceLock<Vec<Tool>> = std::sync::OnceLock::new();
    ONCE.get_or_init(builtin_tools)
}

/// The tables this run knows, once the configuration has been read.
static INSTALLED: std::sync::OnceLock<Vec<Tool>> = std::sync::OnceLock::new();

/// Put the configuration's `[[archiver]]` tables in front of the shipped ones, for the rest
/// of the run.
///
/// Called once, at startup, after the configuration is read and before anything opens an
/// archive — every [`crate::Cache`] built after this sees the same tables, and so does the
/// decision whether `Enter` goes into a file. There is one truth for the run, rather than a
/// cache that knows a user's table and a panel that does not.
///
/// A second call is refused rather than half-applied, and answers `false`: the tables a run
/// knows are fixed for that run.
pub fn install_tools(user: Vec<Tool>) -> bool {
    INSTALLED.set(merge(user, builtin_tools())).is_ok()
}

/// The configuration's tables, then the shipped ones that are not replaced by one of them.
///
/// MultiArc's `custom_user.ini` rule (`docs/05-multiarc.md` §6): what ships is a default, not
/// a fixed list. A table named like a shipped one *replaces* it — so `[[archiver]] name =
/// "7z"` is how somebody changes our switches — and any other table goes first, so it is
/// asked first. Kept apart from [`install_tools`] so it can be tested without process-wide
/// state.
pub fn merge(user: Vec<Tool>, shipped: Vec<Tool>) -> Vec<Tool> {
    let mut out = user;
    for tool in shipped {
        if !out.iter().any(|t| t.name.eq_ignore_ascii_case(&tool.name)) {
            out.push(tool);
        }
    }
    out
}

/// The tables this run knows: the installed ones, or the shipped ones when nothing was
/// installed — which is every test, and any program that never read a configuration.
pub fn tables() -> &'static [Tool] {
    INSTALLED.get().map(Vec::as_slice).unwrap_or_else(shipped)
}

/// Does any table this run knows claim a file called this?
///
/// By the name alone, with no bytes read, because this is what deciding whether `Enter` goes
/// *into* a file has to be: cheap, and wrong only for a file somebody has renamed — the same
/// promise `Format::of_name` makes. A table that claims by magic alone (`magic_only`) is
/// therefore not asked here; it is found when the file is opened, which is when the bytes are
/// read anyway.
pub fn claimed_by_a_tool(name: &str) -> bool {
    tables().iter().any(|tool| tool.claims(name, &[]))
}

/// The tables this program comes with.
///
/// One, for now, and deliberately the one MultiArc itself reaches for: `custom.ini` sends
/// `rpm`, `cpio` and `deb` through the user's `7z` in four lines each
/// (`docs/05-multiarc.md` §6). It is also the widest single answer to row 4.1 of
/// `docs/02-features.md` — one program that reads rar, cab, iso, rpm, cpio, lha, arj and more
/// — so the first table earns the most per line of it.
///
/// Every switch in it was run against 7-Zip 23.01 before it was written down.
pub fn builtin_tools() -> Vec<Tool> {
    vec![sevenzip()]
}

fn sevenzip() -> Tool {
    let mut commands = BTreeMap::new();
    // `-bd` turns off the progress indicator, which is a carriage-returned line that would
    // otherwise be most of what we parsed. `-y` answers the questions it would stop on.
    // `-scsUTF-8` says what the list file is written in: without it 7-Zip reads it in the
    // system code page, which on Windows is not UTF-8 and would mangle every accented name.
    // `--` ends the switches, so an archive whose name begins with `-` is still a name.
    commands.insert(Op::List, "l -bd {-p%P} -- %A".to_string());
    commands.insert(
        Op::Extract,
        "x -y -bd -scsUTF-8 {-p%P} -i@%L -- %A".to_string(),
    );
    commands.insert(
        Op::ExtractWithoutPath,
        "e -y -bd -scsUTF-8 {-p%P} -i@%L -- %A".to_string(),
    );
    commands.insert(Op::Test, "t -bd -scsUTF-8 {-p%P} -i@%L -- %A".to_string());
    commands.insert(
        Op::Delete,
        "d -y -bd -scsUTF-8 {-p%P} -i@%L -- %A".to_string(),
    );
    // Adding takes the names as names, not as patterns to match inside the archive, which is
    // why this one is `@%L` and the four above are `-i@%L`.
    commands.insert(Op::Add, "a -y -bd -scsUTF-8 {-p%P} -- %A @%L".to_string());
    commands.insert(
        Op::AddRecurse,
        "a -y -bd -r -scsUTF-8 {-p%P} -- %A @%L".to_string(),
    );
    // Everything else 7-Zip does not do: no comments, no self-extracting archive from the
    // command line, no lock, no recovery records. Those keys are simply absent, and
    // `Tool::can` reads their absence as the answer.

    Tool {
        name: "7z".to_string(),
        program: "7z".to_string(),
        extensions: [
            "7z", "rar", "cab", "iso", "rpm", "cpio", "lha", "lzh", "arj", "msi", "wim", "dmg",
            "xar", "chm", "vhd", "vhdx", "udf", "squashfs",
        ]
        .iter()
        .map(|e| (*e).to_string())
        .collect(),
        // No magic: this table claims nine formats with nine different headers, and the name
        // is what chooses between them. The program itself reads the bytes properly.
        magic: None,
        magic_only: false,
        commands,
        all_files: Some("*".to_string()),
        default_extension: Some("7z".to_string()),
        listing: Listing {
            // The row of dashes under the column headers, and the identical one under the
            // last row. 7-Zip prints both, which is what makes them an anchor.
            start: Some(Anchor::new("^-------------------")),
            end: Some(Anchor::new("^-------------------")),
            skip: Vec::new(),
            // `2026-11-04 08:21:13 D....            0            0  src`
            //
            // Both numeric columns can be empty and often are: a file inside a solid block
            // has no compressed size of its own, and a directory in a `.wim` has neither. The
            // name is separated from them by the two spaces 7-Zip puts there, which is what
            // keeps a name that begins with digits — or holds spaces — from being read as a
            // number; after that it runs to the end of the line, being the last column.
            rows: vec![concat!(
                r"^(?<mYear>\d{4})-(?<mMonth>\d{2})-(?<mDay>\d{2}) ",
                r"(?<mHour>\d{2}):(?<mMin>\d{2}):(?<mSec>\d{2}) ",
                r"(?<attr>[A-Za-z.]{5}) *(?<size>\d*) *(?<packedSize>\d*) {2}(?<name>\S.*)$"
            )
            .to_string()],
            dir_attr: Some('D'),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(line: &Line) -> Vec<String> {
        line.args
            .iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    /// RAR's own table, whole, out of `Archivers/Rar20.reg` — the lines in
    /// `docs/05-multiarc.md` §6 with MultiArc's doubled `%%` written as the single `%` this
    /// program uses. Filling them in is the test that the language is the same language.
    const RAR: [(Op, &str); 14] = [
        (Op::Extract, "x {-p%P} {-ap%R} -y -c- {%S} %A @%LNM"),
        (Op::ExtractWithoutPath, "e -av- {-p%P} -y -c- {%S} %A @%LNM"),
        (Op::Test, "t -y {-p%P} -c- {%S} %A @%LNM"),
        (Op::Delete, "d -y {-w%W} {%S} %A"),
        (Op::Comment, "c -y {-w%W} {%S} %A"),
        (Op::CommentFiles, "cf -y {-w%W} {%S} %A {@%LNM}"),
        (Op::Sfx, "s -y {%S} %A"),
        (Op::Lock, "k -y {%S} %A"),
        (Op::Protect, "rr -y {%S} %A"),
        (Op::Recover, "r -y {%S} %A"),
        (Op::Add, "a -y {-p%P} {-ap%R} {-w%W} {%S} %A @%LN"),
        (Op::Move, "m -y {-p%P} {-ap%R} {-w%W} {%S} %A @%LN"),
        (
            Op::AddRecurse,
            "a -r0 -y {-p%P} {-ap%R} {-w%W} {%S} %A @%LN",
        ),
        (
            Op::MoveRecurse,
            "m -r0 -y {-p%P} {-ap%R} {-w%W} {%S} %A @%LN",
        ),
    ];

    #[test]
    fn rars_own_table_fills_in_with_nothing_in_hand() {
        let archive = PathBuf::from("/srv/backups/photos.rar");
        let list = PathBuf::from("/tmp/fcmd-7/names");
        let values = Fill {
            archive: Some(&archive),
            list_file: Some(&list),
            ..Fill::default()
        };
        // No password, no folder inside the archive, no temporary folder, no extra switches:
        // every brace in those lines disappears, and nothing is passed empty.
        let add = fill(RAR[10].1, &values).expect("it fills");
        assert_eq!(
            words(&add),
            ["a", "-y", "/srv/backups/photos.rar", "@/tmp/fcmd-7/names"]
        );
        let lock = fill(RAR[7].1, &values).expect("it fills");
        assert_eq!(words(&lock), ["k", "-y", "/srv/backups/photos.rar"]);
        for (op, template) in RAR {
            let line = fill(template, &values).unwrap_or_else(|e| panic!("{}: {e}", op.key()));
            assert!(
                !line.args.iter().any(|a| a.is_empty()),
                "{}: an empty argument",
                op.key()
            );
            assert!(
                !line.args.iter().any(|a| a.to_string_lossy().contains('%')),
                "{}: a hole left unfilled",
                op.key()
            );
        }
    }

    #[test]
    fn a_brace_is_kept_when_its_hole_came_out_with_something() {
        let archive = PathBuf::from("/a/b.rar");
        let list = PathBuf::from("/tmp/names");
        let temp = PathBuf::from("/tmp/work");
        let line = fill(
            RAR[10].1,
            &Fill {
                archive: Some(&archive),
                list_file: Some(&list),
                password: Some("hunter2"),
                inner: Some("photos/2024"),
                temp: Some(&temp),
                switches: Some("-m5 -ep1"),
            },
        )
        .expect("it fills");
        assert_eq!(
            words(&line),
            [
                "a",
                "-y",
                "-phunter2",
                "-apphotos/2024",
                &format!("-w/tmp/work{}", std::path::MAIN_SEPARATOR),
                "-m5",
                "-ep1",
                "/a/b.rar",
                "@/tmp/names",
            ]
        );
    }

    #[test]
    fn an_empty_password_is_the_same_as_no_password() {
        let archive = PathBuf::from("/a/b.rar");
        let values = Fill {
            archive: Some(&archive),
            password: Some(""),
            ..Fill::default()
        };
        let line = fill("t {-p%P} %A", &values).expect("it fills");
        assert_eq!(words(&line), ["t", "/a/b.rar"]);
    }

    #[test]
    fn a_name_with_a_space_in_it_is_one_argument() {
        let archive = PathBuf::from("/home/mb/My Documents/report final.rar");
        let line = fill(
            "l -- %A",
            &Fill {
                archive: Some(&archive),
                ..Fill::default()
            },
        )
        .expect("it fills");
        // Three arguments, not five: the line is split once, and what comes out of a hole is
        // never split again. This is the whole difference from a shell.
        assert_eq!(
            words(&line),
            ["l", "--", "/home/mb/My Documents/report final.rar"]
        );
    }

    #[test]
    fn the_highest_exit_code_that_counts_as_success_comes_out_of_the_line() {
        let line = fill("t %E1 -- %A", &Fill::default()).expect("it fills");
        assert_eq!(line.success_up_to, 1);
        // And it is not an argument: it tells us how to read what came back.
        assert_eq!(words(&line), ["t", "--"]);
        assert_eq!(
            fill("t -- %A", &Fill::default())
                .expect("it fills")
                .success_up_to,
            0,
            "without it, only a zero is success"
        );
    }

    #[test]
    fn two_percent_signs_are_one() {
        let line = fill("a -w100%% %A", &Fill::default()).expect("it fills");
        assert_eq!(words(&line), ["a", "-w100%"]);
    }

    #[test]
    fn the_modifiers_that_name_part_of_a_path_do_that() {
        let archive = PathBuf::from("/srv/old/photos.rar");
        let values = Fill {
            archive: Some(&archive),
            ..Fill::default()
        };
        assert_eq!(
            words(&fill("x %AW", &values).expect("it fills")),
            ["x", "photos.rar"],
            "W: the name only"
        );
        assert_eq!(
            words(&fill("x %AP", &values).expect("it fills")),
            ["x", "/srv/old"],
            "P: the path only"
        );
    }

    #[test]
    fn the_modifiers_that_mean_nothing_here_are_read_and_ignored() {
        let archive = PathBuf::from("/a/My Archive.rar");
        let list = PathBuf::from("/tmp/names");
        let values = Fill {
            archive: Some(&archive),
            list_file: Some(&list),
            ..Fill::default()
        };
        // `QqMN*A` are MultiArc's quoting, folder-form, mask and code-page modifiers. None of
        // them can mean anything to an argv array, and a table copied from a `custom.ini`
        // must still work — so they are swallowed, and the value goes in unchanged.
        let line = fill("x %AQ @%LQMN", &values).expect("it fills");
        assert_eq!(words(&line), ["x", "/a/My Archive.rar", "@/tmp/names"]);
        let line = fill("x %Aq%AA", &values).expect("it fills");
        assert_eq!(words(&line), ["x", "/a/My Archive.rar/a/My Archive.rar"]);
    }

    #[test]
    fn a_table_that_does_not_make_sense_says_what_is_wrong_with_it() {
        for (template, says) in [
            ("a {-p%P %A", "never closed"),
            ("a {-p{%P}} %A", "do not nest"),
            ("a -p%P} %A", "no `{`"),
            ("a %Z %A", "not a hole"),
            ("a %A %", "at the end"),
        ] {
            let why = fill(template, &Fill::default())
                .expect_err(template)
                .to_string();
            assert!(why.contains(says), "{template}: {why}");
        }
    }

    #[test]
    fn a_tool_can_only_do_what_its_table_has_a_line_for() {
        let tool = &builtin_tools()[0];
        assert!(tool.can(Op::List) && tool.can(Op::Extract) && tool.can(Op::Add));
        // 7-Zip writes no archive comment and makes no self-extracting archive from the
        // command line, so those keys are not in the table and that is the declaration.
        assert!(!tool.can(Op::Comment));
        assert!(!tool.can(Op::Sfx));
        assert!(!tool.can(Op::Lock));
        let why = tool.line(Op::Sfx, &Fill::default()).expect_err("it cannot");
        assert_eq!(
            why,
            ExtError::Cannot {
                tool: "7z".to_string(),
                op: Op::Sfx
            }
        );
    }

    #[test]
    fn every_line_of_every_shipped_table_fills_in_and_every_pattern_compiles() {
        let archive = PathBuf::from("/a/b.7z");
        let list = PathBuf::from("/tmp/names");
        let temp = PathBuf::from("/tmp/work");
        let full = Fill {
            archive: Some(&archive),
            list_file: Some(&list),
            password: Some("pw"),
            inner: Some("inner/dir"),
            temp: Some(&temp),
            switches: Some("-x!*.tmp"),
        };
        for tool in builtin_tools() {
            tool.listing
                .reader()
                .unwrap_or_else(|e| panic!("{}: {e}", tool.name));
            assert!(tool.can(Op::List), "{}: nothing can be read", tool.name);
            for op in Op::ALL.into_iter().filter(|op| tool.can(*op)) {
                for values in [&full, &Fill::default()] {
                    let line = tool.line(op, values).unwrap_or_else(|e| {
                        panic!("{} {}: {e}", tool.name, op.key());
                    });
                    assert!(
                        !line.args.is_empty(),
                        "{} {}: no arguments at all",
                        tool.name,
                        op.key()
                    );
                    assert!(
                        !line.args.iter().any(|a| a.is_empty()),
                        "{} {}: an empty argument",
                        tool.name,
                        op.key()
                    );
                }
            }
        }
    }

    /// What `7z l` printed on this machine, 7-Zip 23.01, for an archive of three files and
    /// two folders. Kept whole, banner and summary and all, because the banner and the
    /// summary are what the anchors exist to skip.
    const SEVENZIP_LISTING: &str = "\n\
7-Zip 23.01 (x64) : Copyright (c) 1999-2023 Igor Pavlov : 2023-06-20\n\
 64-bit locale=C.UTF-8 Threads:4 OPEN_MAX:20000\n\
\n\
Scanning the drive for archives:\n\
1 file, 229 bytes (1 KiB)\n\
\n\
Listing archive: test.7z\n\
\n\
--\n\
Path = test.7z\n\
Type = 7z\n\
Physical Size = 229\n\
Headers Size = 209\n\
Method = LZMA2:12\n\
Solid = +\n\
Blocks = 1\n\
\n\
   Date      Time    Attr         Size   Compressed  Name\n\
------------------- ----- ------------ ------------  ------------------------\n\
2026-11-04 08:21:13 D....            0            0  src\n\
2026-11-04 08:21:13 D....            0            0  src/deep\n\
2026-11-04 08:21:13 ....A            9           20  readme.md\n\
2026-11-04 08:21:13 ....A            1               src/deep/x.bin\n\
2026-11-04 08:21:13 ....A            6               src/main.rs\n\
------------------- ----- ------------ ------------  ------------------------\n\
2026-11-04 08:21:13                 16           20  3 files, 2 folders\n";

    #[test]
    fn the_shipped_table_reads_what_the_program_really_printed() {
        let reader = builtin_tools()[0].listing.reader().expect("it compiles");
        let rows = reader.read(SEVENZIP_LISTING);
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "src",
                "src/deep",
                "readme.md",
                "src/deep/x.bin",
                "src/main.rs"
            ],
            "the banner above and the summary below are not rows"
        );
        // The two folders are folders because the attribute column says so.
        assert_eq!(
            rows.iter().map(|r| r.is_dir).collect::<Vec<_>>(),
            [true, true, false, false, false]
        );
        assert_eq!(rows[2].size, Some(9));
        assert_eq!(rows[2].packed, Some(20), "it has a compressed size");
        assert_eq!(
            rows[3].packed, None,
            "and a file inside a solid block has none of its own"
        );
        assert_eq!(rows[4].size, Some(6));
        assert_eq!(rows[2].attr.as_deref(), Some("....A"));
        // 2026-11-04 08:21:13 UTC, as the listing says and with no time zone applied: a
        // listing says what the archive says.
        let seconds = rows[2]
            .mtime
            .expect("a time")
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("after 1970")
            .as_secs();
        assert_eq!(seconds, 1_793_780_473, "2026-11-04 08:21:13 UTC");
    }

    #[test]
    fn a_line_that_matches_nothing_is_dropped_and_the_rest_still_read() {
        let listing = Listing {
            start: None,
            end: None,
            skip: vec![Anchor::new("^WARNING")],
            rows: vec![r"^(?<size>\d+)\s+(?<name>\S+)$".to_string()],
            dir_attr: None,
        };
        let reader = listing.reader().expect("it compiles");
        let rows = reader.read("some banner\nWARNING 12 ignored.txt\n42 one.txt\n7 two.txt\n");
        assert_eq!(
            rows.iter().map(|r| r.name.as_str()).collect::<Vec<_>>(),
            ["one.txt", "two.txt"]
        );
        assert_eq!(rows[0].size, Some(42));
    }

    #[test]
    fn a_pattern_with_no_name_in_it_is_refused_before_anything_runs() {
        let listing = Listing {
            rows: vec![r"^(?<size>\d+)$".to_string()],
            ..Listing::default()
        };
        let why = listing.reader().expect_err("no name").to_string();
        assert!(why.contains("no (?<name>"), "{why}");

        let listing = Listing {
            rows: vec![r"^(?<name>(?=x))$".to_string()],
            ..Listing::default()
        };
        // Rust's regex has no lookaround, and a table that wants it is told so rather than
        // quietly matching nothing.
        assert!(listing.reader().is_err(), "lookaround is not supported");
    }

    #[test]
    fn a_table_claims_a_file_by_its_name_and_by_its_bytes() {
        let tool = &builtin_tools()[0];
        assert!(tool.claims("photos.rar", b""));
        assert!(tool.claims("PACKAGE.RPM", b""));
        assert!(!tool.claims("src.tar.gz", b""));
        // With a magic and `magic_only`, the name stops mattering at all.
        let mut only = tool.clone();
        only.magic = Some(Magic {
            bytes: vec![0xed, 0xab, 0xee, 0xdb],
            at: None,
        });
        only.magic_only = true;
        assert!(only.claims("anything-at-all", &[0xed, 0xab, 0xee, 0xdb, 0x03]));
        assert!(!only.claims("package.rpm", b"not an rpm"));
        // And with a magic but without it, both have to agree.
        only.magic_only = false;
        assert!(only.claims("package.rpm", &[0xed, 0xab, 0xee, 0xdb]));
        assert!(!only.claims("package.rpm", b"not an rpm"));
        assert!(!only.claims("notes.txt", &[0xed, 0xab, 0xee, 0xdb]));
    }

    #[test]
    fn a_magic_can_sit_somewhere_other_than_the_start() {
        let magic = Magic {
            bytes: b"ustar".to_vec(),
            at: Some(257),
        };
        let mut head = vec![0u8; 512];
        head[257..262].copy_from_slice(b"ustar");
        assert!(magic.matches(&head));
        assert!(!magic.matches(b"ustar at the front"));
        // And a file too short to reach it does not match, rather than panicking.
        assert!(!magic.matches(b"short"));
    }

    #[test]
    fn a_key_is_read_whatever_case_it_was_written_in() {
        assert_eq!(
            Op::of_key("extractwithoutpath"),
            Some(Op::ExtractWithoutPath)
        );
        assert_eq!(Op::of_key("SFX"), Some(Op::Sfx));
        assert_eq!(Op::of_key("AllFilesMask"), None, "that one is a setting");
        for op in Op::ALL {
            assert_eq!(Op::of_key(op.key()), Some(op), "{}", op.key());
        }
    }

    #[test]
    fn only_the_operations_that_change_the_archive_say_they_do() {
        for op in Op::ALL {
            assert_eq!(
                op.writes(),
                !matches!(
                    op,
                    Op::List | Op::Test | Op::Extract | Op::ExtractWithoutPath
                ),
                "{}",
                op.key()
            );
        }
        // Extracting writes files, beside the archive: it is reading, by the rule in §9.
        assert!(!Op::Extract.writes());
        assert!(Op::Add.writes() && Op::Delete.writes() && Op::Sfx.writes());
    }

    #[test]
    fn the_sentence_about_a_missing_program_can_be_read_back_out_of_itself() {
        let said = ExtError::NotInstalled {
            program: "7z".to_string(),
        }
        .to_string();
        assert_eq!(crate::needs_program(&said), Some("7z"), "{said}");
        // And it says nothing about a message that is about something else.
        assert_eq!(crate::needs_program("tree.zip: broken archive"), None);
    }

    #[test]
    fn a_users_table_goes_first_and_one_named_like_a_shipped_one_replaces_it() {
        let shipped = builtin_tools();
        let mut mine = shipped[0].clone();
        mine.name = "unace".to_string();
        mine.extensions = vec!["ace".to_string()];
        let merged = merge(vec![mine], shipped.clone());
        assert_eq!(
            merged.iter().map(|t| t.name.as_str()).collect::<Vec<_>>(),
            ["unace", "7z"],
            "the user's in front, so it is asked first"
        );

        // A table called `7z` is how somebody changes our switches: it replaces ours rather
        // than sitting beside it, so there is never a second `7z` answering differently.
        let mut own_7z = shipped[0].clone();
        own_7z.extensions = vec!["rar".to_string()];
        let merged = merge(vec![own_7z], shipped.clone());
        assert_eq!(merged.len(), 1, "{merged:?}");
        assert_eq!(merged[0].extensions, ["rar"], "the user's, not ours");

        // The name compares without regard to case, as the key names do.
        let mut shouting = shipped[0].clone();
        shouting.name = "7Z".to_string();
        assert_eq!(merge(vec![shouting], shipped).len(), 1);
    }

    #[test]
    fn a_program_is_looked_for_on_the_path_and_nowhere_else() {
        // Something every machine these tests run on has, under its own name.
        let shell = if cfg!(windows) { "cmd" } else { "sh" };
        assert!(which(shell).is_some(), "{shell} is on the PATH");
        assert!(which("fcmd-no-such-program-anywhere").is_none());
        // A name with a separator in it is a path, and is not looked for on the PATH at all.
        assert!(which("no/such/program").is_none());
    }

    /// A complaint is not parsed: the entry it is about is whichever known name stands in
    /// it, whatever the program wrote around it.
    #[test]
    fn the_entry_a_complaint_is_about_is_found_however_it_is_worded() {
        let known: std::collections::HashSet<&str> = [
            "src/a.bin",
            "src/a.bin.bak",
            "My Documents/annual report.doc",
            "src",
        ]
        .into_iter()
        .collect();
        for (line, name) in [
            // 7-Zip, unrar and RARLAB's `rar`, as each words it.
            ("ERROR: CRC Failed : src/a.bin", Some("src/a.bin")),
            ("src/a.bin - checksum error", Some("src/a.bin")),
            (
                "Checksum error in src/a.bin. The file is corrupt",
                Some("src/a.bin"),
            ),
            // The longest name wins, so a file is not taken for the one it is named after
            // or for the folder it is in.
            ("ERROR: Data Error : src/a.bin.bak", Some("src/a.bin.bak")),
            (
                "ERROR: CRC Failed : My Documents/annual report.doc",
                Some("My Documents/annual report.doc"),
            ),
            ("\"src/a.bin\": bad data", Some("src/a.bin")),
            // 7-Zip on Windows writes its own separator.
            ("ERROR: CRC Failed : src\\a.bin", Some("src/a.bin")),
            (
                "ERROR: CRC Failed : My Documents\\annual report.doc",
                Some("My Documents/annual report.doc"),
            ),
            // Part of a word is not the word.
            ("ERROR: CRC Failed : other/src/a.binary", None),
            ("Archives with Errors: 1", None),
            ("", None),
        ] {
            assert_eq!(named_in(line, &known), name, "{line}");
        }
    }
}
