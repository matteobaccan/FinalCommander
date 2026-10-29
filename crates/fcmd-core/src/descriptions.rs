//! `descript.ion`: the line of text a directory keeps about each of its files (SPEC 02 §9).
//!
//! It is a 4DOS habit that FAR kept and that nothing else has: a plain file in the directory,
//! one line per name, holding what a file *is* where the name has no room to say it. The whole
//! format is a name and the rest of the line:
//!
//! ```text
//! readme.md          what to read first
//! "two words.txt"    a name with a space in it is quoted
//! ```
//!
//! What makes it worth having is that it travels: the description belongs to the directory and
//! not to a database somewhere, so a folder copied to a memory stick takes its descriptions
//! with it — as long as whatever copied it knew to carry the file, which is why the job engine
//! has to (SPEC 03 §7).

use std::collections::HashMap;

/// The names a directory's descriptions may be under, in the order they are looked for.
///
/// `descript.ion` is 4DOS's and FAR's; `files.bbs` is the older bulletin-board spelling of the
/// same thing, which FAR also reads, and which still turns up in files downloaded from one.
pub const DEFAULT_NAMES: [&str; 2] = ["descript.ion", "files.bbs"];

/// How a directory's descriptions are looked for and written back (SPEC 02 §9).
///
/// Both of these are FAR's own settings. The names because a directory may keep its
/// descriptions under either spelling and somebody with a tree full of `files.bbs` does not
/// want a second file appearing beside each one; the code page because a `descript.ion` is
/// often shared with the DOS-era tools that wrote it, and those cannot read UTF-8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rules {
    /// The names tried, in order; the first is what a new file is written under. An empty
    /// list is the two defaults, because a panel that looked for nothing would quietly stop
    /// showing descriptions at all.
    pub names: Vec<String>,
    /// Write the file in UTF-8. Off, it is written back in the code page it was read in —
    /// there is nothing else to go on, and guessing the machine's own would turn a Russian
    /// `descript.ion` into a Latin one on a Latin machine. A file that does not exist yet,
    /// and one that was already UTF-8, is written in UTF-8 either way.
    pub utf8: bool,
}

impl Default for Rules {
    fn default() -> Rules {
        Rules {
            names: DEFAULT_NAMES.iter().map(|n| n.to_string()).collect(),
            utf8: true,
        }
    }
}

impl Rules {
    /// The rules a configuration asks for, with an empty list of names meaning the defaults.
    pub fn new(names: &[String], utf8: bool) -> Rules {
        let names: Vec<String> = names
            .iter()
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty())
            .collect();
        match names.is_empty() {
            true => Rules {
                utf8,
                ..Rules::default()
            },
            false => Rules { names, utf8 },
        }
    }

    /// Where a new file goes: the first name, which is `descript.ion` unless somebody said
    /// otherwise.
    pub fn write_name(&self) -> &str {
        self.names
            .first()
            .map(String::as_str)
            .unwrap_or(DEFAULT_NAMES[0])
    }
}

/// How FAR writes a description that has more than one line in it: the two bytes stand for the
/// break, because the format itself is one line per file and cannot hold a real newline.
const BREAK: &str = "\u{4}\u{c2}";

/// The descriptions of one directory.
///
/// Names are matched without regard to case, because the file is written by hand as often as
/// by a program and `Readme.md` in it is meant to be the `README.md` on disk. The order of the
/// file is kept, so writing it back does not shuffle lines somebody arranged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Descriptions {
    /// Lowercased name → where it sits in `rows`.
    index: HashMap<String, usize>,
    /// The name as it was written, and its description.
    rows: Vec<(String, String)>,
}

impl Descriptions {
    /// Read a `descript.ion` from its text.
    ///
    /// A line that is only a name describes nothing and is dropped: an empty description is
    /// the same as no description, and keeping it would write back a line that says nothing.
    pub fn parse(text: &str) -> Self {
        let mut out = Descriptions::default();
        for line in text.lines() {
            let line = line.trim_end_matches(['\r', '\u{1a}']);
            if line.trim().is_empty() {
                continue;
            }
            let (name, rest) = split_name(line);
            let description = rest.trim();
            if name.is_empty() || description.is_empty() {
                continue;
            }
            out.set(name, description);
        }
        out
    }

    /// What a name is described as, if anything.
    pub fn get(&self, name: &str) -> Option<&str> {
        let at = *self.index.get(&fold(name))?;
        Some(self.rows[at].1.as_str())
    }

    /// Describe a name. An empty description takes the line away, because a line with nothing
    /// after the name is what "no description" looks like in the file.
    pub fn set(&mut self, name: &str, description: &str) {
        let description = description.trim();
        if description.is_empty() {
            self.remove(name);
            return;
        }
        match self.index.get(&fold(name)) {
            Some(&at) => self.rows[at].1 = description.to_string(),
            None => {
                self.index.insert(fold(name), self.rows.len());
                self.rows.push((name.to_string(), description.to_string()));
            }
        }
    }

    /// Forget a name — what a delete has to do, so a description does not outlive its file and
    /// land on the next file to be given that name.
    pub fn remove(&mut self, name: &str) -> Option<String> {
        let at = self.index.remove(&fold(name))?;
        let (_, text) = self.rows.remove(at);
        // Everything after the hole moved down one.
        for seat in self.index.values_mut() {
            if *seat > at {
                *seat -= 1;
            }
        }
        Some(text)
    }

    /// Follow a rename, keeping the line where it was in the file.
    pub fn rename(&mut self, from: &str, to: &str) {
        let Some(&at) = self.index.get(&fold(from)) else {
            return;
        };
        // A rename onto a name that is already described replaces that one, as the file does.
        if fold(from) != fold(to) {
            if let Some(other) = self.index.get(&fold(to)).copied() {
                if other != at {
                    let text = self.rows[at].1.clone();
                    self.remove(from);
                    self.set(to, &text);
                    return;
                }
            }
            self.index.remove(&fold(from));
            self.index.insert(fold(to), at);
        }
        self.rows[at].0 = to.to_string();
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Every name and its description, in the order the file holds them.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.rows.iter().map(|(n, d)| (n.as_str(), d.as_str()))
    }

    /// The file, as it goes back to disk. Empty when there is nothing to say, which is the
    /// caller's cue to take the file away rather than leave an empty one behind.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        for (name, description) in &self.rows {
            out.push_str(&quoted(name));
            out.push(' ');
            out.push_str(&description.replace('\n', BREAK));
            out.push('\n');
        }
        out
    }
}

/// A description as one line of a panel: the breaks FAR writes inside one are spaces here,
/// because a row of a listing has one line to give.
pub fn one_line(description: &str) -> String {
    description
        .replace(BREAK, " ")
        .replace(['\n', '\r', '\t'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// The key a name is matched by: its own case folded away.
fn fold(name: &str) -> String {
    name.trim().to_lowercase()
}

/// The name at the head of a line, and what is left of the line.
fn split_name(line: &str) -> (&str, &str) {
    let line = line.trim_start();
    if let Some(rest) = line.strip_prefix('"') {
        match rest.find('"') {
            Some(end) => return (&rest[..end], &rest[end + 1..]),
            // An opening quote with no closing one: the whole of it is the name, which is
            // what a half-written line most likely meant.
            None => return (rest, ""),
        }
    }
    match line.find(char::is_whitespace) {
        Some(at) => (&line[..at], &line[at..]),
        None => (line, ""),
    }
}

/// A name as the file holds it: quoted when it has a space in it, because the format splits
/// on the first one.
fn quoted(name: &str) -> String {
    if name.contains(char::is_whitespace) || name.starts_with('"') {
        format!("\"{}\"", name.replace('"', "'"))
    } else {
        name.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_file_is_read_and_written_back_as_it_was() {
        let text = "readme.md what to read first\n\
                    \"two words.txt\" a name with a space\n\
                    nothing.txt\n\
                    \n\
                    build.sh   runs the build\n";
        let d = Descriptions::parse(text);
        assert_eq!(d.len(), 3, "the line with no description is not one");
        assert_eq!(d.get("readme.md"), Some("what to read first"));
        assert_eq!(d.get("two words.txt"), Some("a name with a space"));
        assert_eq!(d.get("build.sh"), Some("runs the build"));
        assert_eq!(d.get("nothing.txt"), None);
        // The case of the name is not the name: a file written by hand says `Readme.md`.
        assert_eq!(d.get("README.MD"), Some("what to read first"));

        // Back to the file, in the order it had, with the space-bearing name quoted again.
        assert_eq!(
            d.to_text(),
            "readme.md what to read first\n\
             \"two words.txt\" a name with a space\n\
             build.sh runs the build\n"
        );
    }

    #[test]
    fn setting_an_empty_description_takes_the_line_away() {
        let mut d = Descriptions::parse("a.txt one\nb.txt two\nc.txt three\n");
        d.set("b.txt", "  ");
        assert_eq!(d.get("b.txt"), None);
        assert_eq!(d.to_text(), "a.txt one\nc.txt three\n");
        // And the rows after the hole are still found, which is what the index has to keep.
        assert_eq!(d.get("c.txt"), Some("three"));
        d.set("c.txt", "third");
        assert_eq!(d.get("c.txt"), Some("third"));
    }

    #[test]
    fn a_rename_takes_the_description_with_it() {
        let mut d = Descriptions::parse("a.txt one\nb.txt two\n");
        d.rename("a.txt", "z.txt");
        assert_eq!(d.get("a.txt"), None);
        assert_eq!(d.get("z.txt"), Some("one"));
        // The line stays where it was: a rename is not a reason to shuffle the file.
        assert_eq!(d.to_text(), "z.txt one\nb.txt two\n");
        // Renaming onto a described name replaces it, as the rename itself replaces the file.
        d.rename("z.txt", "b.txt");
        assert_eq!(d.len(), 1);
        assert_eq!(d.get("b.txt"), Some("one"));
        // And renaming something nobody described describes nothing.
        d.rename("gone.txt", "also-gone.txt");
        assert_eq!(d.len(), 1);
    }

    #[test]
    fn a_description_of_more_than_one_line_survives_the_round_trip() {
        let mut d = Descriptions::default();
        d.set("notes.txt", "first line\nsecond line");
        let text = d.to_text();
        assert!(text.lines().count() == 1, "one line per file: {text:?}");
        let again = Descriptions::parse(&text);
        assert_eq!(
            again.get("notes.txt"),
            Some("first line\u{4}\u{c2}second line")
        );
        // And a panel shows it on the one line it has.
        assert_eq!(
            one_line(again.get("notes.txt").unwrap()),
            "first line second line"
        );
    }
}
