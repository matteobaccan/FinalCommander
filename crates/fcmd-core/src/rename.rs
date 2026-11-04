//! Multi-rename: many names worked out from one pattern (SPEC 03 §8).
//!
//! Pure on purpose. Nothing here touches a filesystem: it takes the names a panel holds and
//! the options of the dialog, and says what each one would become and what is wrong with it.
//! That is what lets the dialog preview every keystroke, and what the tests check without a
//! directory anywhere. Doing the renames is `fcmd_jobs::rename_all`'s business.
//!
//! The language is Total Commander's, which is the one people already know:
//!
//! ```text
//! [N]        the name without its extension      [N3-5]  its characters 3 to 5
//! [N3]       its third character                 [N3-]   from the third to the end
//! [N2,4]     four characters from the second     [E]     the extension, with the same forms
//! [C]        the counter, as the dialog sets it  [C10,2,3]  start 10, step 2, three digits
//! [d] [t]    the file's date and time            [d:%Y-%m]  in any strftime spelling
//! [P]        the name of the folder it is in     [[      a literal [
//! ```
//!
//! Positions count characters, not bytes, from 1, as TC does — so `[N1-3]` of `ältere` is
//! `ält` and not a cut through the middle of a letter.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

/// One thing to be renamed, as the panel knows it.
#[derive(Debug, Clone)]
pub struct Item {
    pub name: String,
    pub is_dir: bool,
    pub mtime: Option<SystemTime>,
    /// The name of the folder it is in: what `[P]` gives.
    pub parent: String,
}

/// What happens to letters, after everything else.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Case {
    #[default]
    Unchanged,
    Lower,
    Upper,
    /// The first letter of every word upper, the rest lower: `Holiday In Rome.jpg`.
    Title,
    /// The first letter of the name upper, the rest lower: `Holiday in rome.jpg`.
    Sentence,
}

/// The dialog, as data.
#[derive(Debug, Clone)]
pub struct Options {
    /// What the part before the last dot becomes.
    pub name: String,
    /// What the extension becomes; empty takes the dot away too.
    pub ext: String,
    /// Replaced in the whole new name, after the masks.
    pub search: String,
    pub replace: String,
    /// `search` is a regular expression and `replace` may say `$1`.
    pub regex: bool,
    pub case: Case,
    pub counter_start: i64,
    pub counter_step: i64,
    pub counter_width: usize,
}

impl Default for Options {
    fn default() -> Options {
        Options {
            name: "[N]".into(),
            ext: "[E]".into(),
            search: String::new(),
            replace: String::new(),
            regex: false,
            case: Case::Unchanged,
            counter_start: 1,
            counter_step: 1,
            counter_width: 1,
        }
    }
}

/// Why a row cannot be renamed as it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Problem {
    /// Nothing is left of the name.
    Empty,
    /// A slash, a NUL, or `.`/`..`: a name no filesystem takes.
    Invalid,
    /// Another row of the same plan comes out with this name.
    Twice,
    /// A file of this name is already there and is not one of those being renamed away.
    Exists,
}

/// One line of the preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub old: String,
    pub new: String,
    pub problem: Option<Problem>,
}

impl Row {
    pub fn changes(&self) -> bool {
        self.old != self.new
    }
}

/// A mask or a search that cannot be read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternError(pub String);

impl std::fmt::Display for PatternError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A piece of a mask, read once.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    Text(String),
    Name(Range),
    Ext(Range),
    Counter {
        start: Option<i64>,
        step: Option<i64>,
        width: Option<usize>,
    },
    Date(String),
    Parent,
}

/// Which characters of a name a field takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
    All,
    /// From, 1-based, and how many — `None` to the end.
    Part {
        from: usize,
        count: Option<usize>,
    },
}

impl Range {
    fn take(self, text: &str) -> String {
        match self {
            Range::All => text.to_string(),
            Range::Part { from, count } => {
                let skip = from.saturating_sub(1);
                let chars = text.chars().skip(skip);
                match count {
                    Some(n) => chars.take(n).collect(),
                    None => chars.collect(),
                }
            }
        }
    }
}

/// `3-5`, `3`, `3-`, `2,4`, or nothing.
fn range(spec: &str, field: char) -> Result<Range, PatternError> {
    let bad = || PatternError(format!("[{field}{spec}] is not a range"));
    if spec.is_empty() {
        return Ok(Range::All);
    }
    let number = |s: &str| s.trim().parse::<usize>().ok().filter(|n| *n > 0);
    if let Some((from, count)) = spec.split_once(',') {
        let from = number(from).ok_or_else(bad)?;
        let count = number(count).ok_or_else(bad)?;
        return Ok(Range::Part {
            from,
            count: Some(count),
        });
    }
    if let Some((from, to)) = spec.split_once('-') {
        let from = number(from).ok_or_else(bad)?;
        if to.trim().is_empty() {
            return Ok(Range::Part { from, count: None });
        }
        let to = number(to).ok_or_else(bad)?;
        if to < from {
            return Err(bad());
        }
        return Ok(Range::Part {
            from,
            count: Some(to - from + 1),
        });
    }
    let at = number(spec).ok_or_else(bad)?;
    Ok(Range::Part {
        from: at,
        count: Some(1),
    })
}

fn parse(mask: &str) -> Result<Vec<Piece>, PatternError> {
    let mut out = Vec::new();
    let mut text = String::new();
    let mut chars = mask.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '[' {
            text.push(c);
            continue;
        }
        if chars.peek() == Some(&'[') {
            chars.next();
            text.push('[');
            continue;
        }
        let mut inner = String::new();
        let mut closed = false;
        for c in chars.by_ref() {
            if c == ']' {
                closed = true;
                break;
            }
            inner.push(c);
        }
        if !closed {
            return Err(PatternError(format!("[{inner} is never closed")));
        }
        if !text.is_empty() {
            out.push(Piece::Text(std::mem::take(&mut text)));
        }
        let mut letters = inner.chars();
        let field = letters.next().unwrap_or(' ');
        let rest: String = letters.collect();
        out.push(match field {
            'N' => Piece::Name(range(&rest, 'N')?),
            'E' => Piece::Ext(range(&rest, 'E')?),
            'P' if rest.is_empty() => Piece::Parent,
            'C' => {
                let parts: Vec<&str> = if rest.is_empty() {
                    Vec::new()
                } else {
                    rest.split(',').collect()
                };
                if parts.len() > 3 {
                    return Err(PatternError(format!(
                        "[C{rest}] has more than three numbers"
                    )));
                }
                let bad = || PatternError(format!("[C{rest}] is not start,step,width"));
                let int = |i: usize| -> Result<Option<i64>, PatternError> {
                    match parts.get(i).map(|s| s.trim()) {
                        None | Some("") => Ok(None),
                        Some(s) => s.parse().map(Some).map_err(|_| bad()),
                    }
                };
                Piece::Counter {
                    start: int(0)?,
                    step: int(1)?,
                    width: int(2)?
                        .map(|w| usize::try_from(w).map_err(|_| bad()))
                        .transpose()?,
                }
            }
            'd' | 't' => {
                let default = if field == 'd' { "%Y-%m-%d" } else { "%H.%M.%S" };
                let format = match rest.strip_prefix(':') {
                    Some(f) if !f.is_empty() => f.to_string(),
                    None if rest.is_empty() => default.to_string(),
                    _ => return Err(PatternError(format!("[{inner}] is not a date"))),
                };
                Piece::Date(format)
            }
            _ => return Err(PatternError(format!("[{inner}] is not a field"))),
        });
    }
    if !text.is_empty() {
        out.push(Piece::Text(text));
    }
    Ok(out)
}

/// A name in two: what is before the last dot, and what is after it. A name that starts with
/// its only dot (`.bashrc`) has no extension, and neither has a directory.
fn split(name: &str, is_dir: bool) -> (&str, &str) {
    if is_dir {
        return (name, "");
    }
    match name.rfind('.') {
        Some(0) | None => (name, ""),
        Some(at) => (&name[..at], &name[at + 1..]),
    }
}

fn fill(
    pieces: &[Piece],
    item: &Item,
    index: usize,
    opts: &Options,
    date: &dyn Fn(SystemTime, &str) -> String,
) -> String {
    let (stem, ext) = split(&item.name, item.is_dir);
    let mut out = String::new();
    for piece in pieces {
        match piece {
            Piece::Text(t) => out.push_str(t),
            Piece::Name(r) => out.push_str(&r.take(stem)),
            Piece::Ext(r) => out.push_str(&r.take(ext)),
            Piece::Parent => out.push_str(&item.parent),
            Piece::Counter { start, step, width } => {
                let start = start.unwrap_or(opts.counter_start);
                let step = step.unwrap_or(opts.counter_step);
                let width = width.unwrap_or(opts.counter_width);
                let value = start.saturating_add(step.saturating_mul(index as i64));
                let digits = format!("{}", value.unsigned_abs());
                let sign = if value < 0 { "-" } else { "" };
                out.push_str(&format!("{sign}{digits:0>width$}"));
            }
            Piece::Date(format) => {
                if let Some(t) = item.mtime {
                    out.push_str(&date(t, format));
                }
            }
        }
    }
    out
}

fn recase(name: &str, case: Case) -> String {
    match case {
        Case::Unchanged => name.to_string(),
        Case::Lower => name.to_lowercase(),
        Case::Upper => name.to_uppercase(),
        Case::Title | Case::Sentence => {
            let mut out = String::with_capacity(name.len());
            let mut start = true;
            for c in name.chars() {
                if start && c.is_alphanumeric() {
                    out.extend(c.to_uppercase());
                    start = false;
                } else {
                    out.extend(c.to_lowercase());
                    if case == Case::Title && !c.is_alphanumeric() && c != '\'' {
                        start = true;
                    }
                }
            }
            out
        }
    }
}

/// What every item would be called, and what is wrong where something is.
///
/// `existing` is every name in the folder, the items' own included: a new name that is one of
/// them is a problem unless the file holding it is itself being renamed to something else.
/// `date` spells a time in a strftime format; the caller has a clock, this module does not.
pub fn preview(
    items: &[Item],
    opts: &Options,
    existing: &HashSet<String>,
    date: &dyn Fn(SystemTime, &str) -> String,
) -> Result<Vec<Row>, PatternError> {
    let name = parse(&opts.name)?;
    let ext = parse(&opts.ext)?;
    let search = if opts.search.is_empty() {
        None
    } else if opts.regex {
        Some(
            regex::Regex::new(&opts.search)
                .map_err(|e| PatternError(format!("the search does not compile: {e}")))?,
        )
    } else {
        Some(
            regex::Regex::new(&regex::escape(&opts.search))
                .expect("an escaped string always compiles"),
        )
    };

    let mut rows: Vec<Row> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let stem = fill(&name, item, i, opts, date);
            let tail = fill(&ext, item, i, opts, date);
            let mut new = if tail.is_empty() {
                stem
            } else {
                format!("{stem}.{tail}")
            };
            if let Some(search) = &search {
                new = if opts.regex {
                    search.replace_all(&new, opts.replace.as_str()).into_owned()
                } else {
                    search
                        .replace_all(&new, regex::NoExpand(&opts.replace))
                        .into_owned()
                };
            }
            let new = recase(&new, opts.case);
            Row {
                old: item.name.clone(),
                new,
                problem: None,
            }
        })
        .collect();

    // Names leaving: a file renamed away frees its name for another row to take.
    let leaving: HashSet<String> = rows
        .iter()
        .filter(|r| r.changes())
        .map(|r| r.old.clone())
        .collect();
    let mut seen: HashMap<String, usize> = HashMap::new();
    for row in &rows {
        *seen.entry(row.new.clone()).or_default() += 1;
    }
    for row in &mut rows {
        row.problem = if row.new.is_empty() {
            Some(Problem::Empty)
        } else if row.new == "." || row.new == ".." || row.new.contains(['/', '\0']) {
            Some(Problem::Invalid)
        } else if seen.get(&row.new).copied().unwrap_or(0) > 1 {
            Some(Problem::Twice)
        } else if row.changes() && existing.contains(&row.new) && !leaving.contains(&row.new) {
            Some(Problem::Exists)
        } else {
            None
        };
    }
    Ok(rows)
}

/// The renames a preview asks for: every row that changes, if no row has a problem.
///
/// All or nothing. A plan that renames the rows it can and leaves the others would leave a
/// folder half in the old scheme and half in the new one, which is the one outcome nobody who
/// typed a pattern wanted.
pub fn plan(rows: &[Row]) -> Option<Vec<(String, String)>> {
    if rows.iter().any(|r| r.problem.is_some()) {
        return None;
    }
    Some(
        rows.iter()
            .filter(|r| r.changes())
            .map(|r| (r.old.clone(), r.new.clone()))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(names: &[&str]) -> Vec<Item> {
        names
            .iter()
            .map(|n| Item {
                name: n.to_string(),
                is_dir: false,
                mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(86_400)),
                parent: "photos".into(),
            })
            .collect()
    }

    fn date(_: SystemTime, format: &str) -> String {
        format
            .replace("%Y", "1970")
            .replace("%m", "01")
            .replace("%d", "02")
    }

    fn run(names: &[&str], opts: &Options) -> Vec<String> {
        let existing = names.iter().map(|n| n.to_string()).collect();
        preview(&items(names), opts, &existing, &date)
            .unwrap()
            .into_iter()
            .map(|r| r.new)
            .collect()
    }

    fn with(name: &str, ext: &str) -> Options {
        Options {
            name: name.into(),
            ext: ext.into(),
            ..Options::default()
        }
    }

    #[test]
    fn the_default_masks_change_nothing() {
        let names = ["a.txt", "b.tar.gz", ".bashrc", "README"];
        assert_eq!(run(&names, &Options::default()), names);
    }

    #[test]
    fn the_fields_take_the_parts_they_name() {
        assert_eq!(
            run(&["holiday.jpeg"], &with("[N1-3]", "[E1-3]")),
            ["hol.jpe"]
        );
        assert_eq!(run(&["holiday.jpeg"], &with("[N3]", "[E]")), ["l.jpeg"]);
        assert_eq!(run(&["holiday.jpeg"], &with("[N4-]", "[E]")), ["iday.jpeg"]);
        assert_eq!(run(&["holiday.jpeg"], &with("[N2,3]", "[E]")), ["oli.jpeg"]);
        assert_eq!(
            run(&["IMG_1.jpg"], &with("[P]-[N]", "[E]")),
            ["photos-IMG_1.jpg"]
        );
        assert_eq!(
            run(&["x.jpg"], &with("[d]_[N]", "[E]")),
            ["1970-01-02_x.jpg"]
        );
        assert_eq!(run(&["x.jpg"], &with("[d:%Y%m]", "[E]")), ["197001.jpg"]);
        // Characters, not bytes.
        assert_eq!(run(&["ältere.txt"], &with("[N1-3]", "[E]")), ["ält.txt"]);
        // An empty extension mask takes the dot with it.
        assert_eq!(run(&["notes.txt"], &with("[N]", "")), ["notes"]);
        assert_eq!(run(&["a.txt"], &with("[[[N]]]", "[E]")), ["[a]].txt"]);
    }

    #[test]
    fn the_counter_counts_in_the_order_of_the_rows() {
        let names = ["c.jpg", "a.jpg", "b.jpg"];
        assert_eq!(
            run(&names, &with("pic_[C]", "[E]")),
            ["pic_1.jpg", "pic_2.jpg", "pic_3.jpg"]
        );
        assert_eq!(
            run(&names, &with("[C10,5,3]", "[E]")),
            ["010.jpg", "015.jpg", "020.jpg"]
        );
        let opts = Options {
            counter_start: 7,
            counter_width: 2,
            ..with("[C]", "[E]")
        };
        assert_eq!(run(&names, &opts), ["07.jpg", "08.jpg", "09.jpg"]);
    }

    #[test]
    fn search_and_replace_runs_on_the_new_name_plain_or_as_a_regex() {
        let plain = Options {
            search: "a.b".into(),
            replace: "$1".into(),
            ..Options::default()
        };
        // Plain means plain: the dot is a dot and `$1` is two characters.
        assert_eq!(
            run(&["xa.by.txt", "xaZby.txt"], &plain),
            ["x$1y.txt", "xaZby.txt"]
        );
        let regex = Options {
            search: r"IMG_(\d+)".into(),
            replace: "photo-$1".into(),
            regex: true,
            ..Options::default()
        };
        assert_eq!(run(&["IMG_0042.jpg"], &regex), ["photo-0042.jpg"]);
    }

    #[test]
    fn the_case_is_changed_last() {
        let case = |c| Options {
            case: c,
            ..Options::default()
        };
        assert_eq!(
            run(&["My Holiday.JPG"], &case(Case::Lower)),
            ["my holiday.jpg"]
        );
        assert_eq!(
            run(&["My Holiday.JPG"], &case(Case::Upper)),
            ["MY HOLIDAY.JPG"]
        );
        assert_eq!(
            run(&["my holiday-in rome.jpg"], &case(Case::Title)),
            ["My Holiday-In Rome.Jpg"]
        );
        assert_eq!(
            run(&["MY HOLIDAY.JPG"], &case(Case::Sentence)),
            ["My holiday.jpg"]
        );
    }

    #[test]
    fn a_mask_that_cannot_be_read_says_where() {
        let existing = HashSet::new();
        for (mask, says) in [
            ("[N", "never closed"),
            ("[Q]", "not a field"),
            ("[N5-2]", "not a range"),
            ("[C1,x]", "start,step,width"),
        ] {
            let e = preview(&items(&["a.txt"]), &with(mask, "[E]"), &existing, &date).unwrap_err();
            assert!(e.0.contains(says), "{mask}: {e}");
        }
        let bad = Options {
            search: "(".into(),
            regex: true,
            ..Options::default()
        };
        let e = preview(&items(&["a.txt"]), &bad, &existing, &date).unwrap_err();
        assert!(e.0.contains("does not compile"), "{e}");
    }

    #[test]
    fn every_row_says_what_is_wrong_with_it() {
        let rows = |names: &[&str], others: &[&str], opts: &Options| {
            let mut existing: HashSet<String> = names.iter().map(|n| n.to_string()).collect();
            existing.extend(others.iter().map(|n| n.to_string()));
            preview(&items(names), opts, &existing, &date).unwrap()
        };
        // Two rows to one name.
        let r = rows(&["a.txt", "b.txt"], &[], &with("same", "[E]"));
        assert!(r.iter().all(|r| r.problem == Some(Problem::Twice)));
        // A name a file not being renamed already has.
        let r = rows(&["a.txt"], &["b.txt"], &with("b", "[E]"));
        assert_eq!(r[0].problem, Some(Problem::Exists));
        // Nothing left, and a slash.
        let r = rows(&["a"], &[], &with("", ""));
        assert_eq!(r[0].problem, Some(Problem::Empty));
        let r = rows(&["a.txt"], &[], &with("x/[N]", "[E]"));
        assert_eq!(r[0].problem, Some(Problem::Invalid));
        assert_eq!(plan(&r), None, "a plan with a problem in it is no plan");
    }

    /// `2→1` and `1→2` is a swap, not a collision: each name is freed by the other rename.
    #[test]
    fn names_that_trade_places_are_not_a_collision() {
        let names = ["2.txt", "1.txt"];
        let existing: HashSet<String> = names.iter().map(|s| s.to_string()).collect();
        let r = preview(&items(&names), &with("[C]", "[E]"), &existing, &date).unwrap();
        assert_eq!(
            r.iter().map(|r| r.new.as_str()).collect::<Vec<_>>(),
            ["1.txt", "2.txt"]
        );
        assert!(r.iter().all(|r| r.problem.is_none()), "{r:?}");
        assert_eq!(plan(&r).unwrap().len(), 2);
    }

    #[test]
    fn a_directory_has_no_extension() {
        let dir = Item {
            name: "v1.2".into(),
            is_dir: true,
            mtime: None,
            parent: "x".into(),
        };
        let r = preview(&[dir], &with("[N]-old", "[E]"), &HashSet::new(), &date).unwrap();
        assert_eq!(r[0].new, "v1.2-old");
    }
}
