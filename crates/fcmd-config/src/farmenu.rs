//! FAR's `FarMenu.ini`, read as it stands — `docs/spec/06-tools.md` §4a.
//!
//! Somebody coming from FAR (or from far2l, which kept the format) has years of `F2` in a
//! file: one in the profile, and one in every project directory that has commands of its
//! own. Asking them to retype it as TOML before the key does anything is asking them to
//! start by doing work; so the file is read where it lies, into the same rows
//! `fcmd-menu.toml` makes.
//!
//! The format is FAR's own text one:
//!
//! ```text
//! B:   Build
//!     make -j8
//! G:   Git
//! {
//! S:   Status
//!     git status
//! --:  Careful
//! P:   Push
//!     git push
//! }
//! ```
//!
//! A line that starts at the margin is a row — the hot key, a colon, the label. The lines
//! indented under it are its commands. A `{` on a line of its own makes the row above a
//! submenu, up to the matching `}`; a hot key of `--` is a rule, with the label as its
//! heading.
//!
//! **The commands are rewritten once, here**, from FAR's punctuation (`!.!`, `!\`, `!&`)
//! into the named substitutions (`{name}`, `{dir}`, `{selection}`), so that what the menu
//! holds is a command line like any other. What has a counterpart is mapped; what is only
//! approximated — a short 8.3 name, a path with its links resolved — is mapped and *said*;
//! and a row that uses something with no counterpart at all (the drive letter, the passive
//! panel's selection) is kept on the menu but runs nothing, and says why when it is taken.
//! A command silently run with a hole where a path should be is the one outcome this must
//! not have.

use std::path::Path;

use crate::usermenu::{Entry, UserMenu};

/// The name FAR gives the file, and the one a case-sensitive disk may hold it under.
pub const FAR_MENU_FILE_NAMES: [&str; 2] = ["FarMenu.ini", "farmenu.ini"];

/// A FAR menu as rows of ours, and everything that was read differently from FAR on the way.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FarMenu {
    pub menu: UserMenu,
    /// One line per remark, each naming the row it is about.
    pub notes: Vec<String>,
}

/// Read a `FarMenu.ini`. FAR 3 writes it in UTF-16 with a mark at the front; FAR 2 and far2l
/// write UTF-8; both are read, and anything else is read as UTF-8 with what does not decode
/// replaced, which is said.
pub fn load(path: &Path) -> Result<FarMenu, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (text, lossy) = decode(&bytes);
    let mut far = parse(&text);
    if lossy {
        far.notes.insert(
            0,
            "the file is neither UTF-8 nor UTF-16: the letters that did not read are shown as \u{fffd}"
                .to_string(),
        );
    }
    Ok(far)
}

/// The bytes as text, and whether anything was lost.
fn decode(bytes: &[u8]) -> (String, bool) {
    let utf16 = |rest: &[u8], big: bool| {
        let (pairs, odd) = rest.as_chunks::<2>();
        let units: Vec<u16> = pairs
            .iter()
            .map(|pair| {
                if big {
                    u16::from_be_bytes(*pair)
                } else {
                    u16::from_le_bytes(*pair)
                }
            })
            .collect();
        let text = String::from_utf16_lossy(&units);
        let lossy = text.contains('\u{fffd}') || !odd.is_empty();
        (text, lossy)
    };
    match bytes {
        [0xff, 0xfe, rest @ ..] => utf16(rest, false),
        [0xfe, 0xff, rest @ ..] => utf16(rest, true),
        [0xef, 0xbb, 0xbf, rest @ ..] => lossy_utf8(rest),
        _ => lossy_utf8(bytes),
    }
}

fn lossy_utf8(bytes: &[u8]) -> (String, bool) {
    match std::str::from_utf8(bytes) {
        Ok(text) => (text.to_string(), false),
        Err(_) => (String::from_utf8_lossy(bytes).into_owned(), true),
    }
}

/// Parse the text of a `FarMenu.ini`.
///
/// Nothing here is an error. FAR itself reads this file line by line and skips what it does
/// not understand, and a menu that refused to open over one stray line would be stricter
/// than the program the file was written for; what is skipped is a note instead.
pub fn parse(text: &str) -> FarMenu {
    let mut notes = Vec::new();
    let mut lines = text.lines().enumerate().peekable();
    let entries = level(&mut lines, &mut notes, 0);
    FarMenu {
        menu: UserMenu { entries },
        notes,
    }
}

type Lines<'a> = std::iter::Peekable<std::iter::Enumerate<std::str::Lines<'a>>>;

/// One level of the menu: up to the `}` that closes it, or the end of the file.
fn level(lines: &mut Lines, notes: &mut Vec<String>, depth: usize) -> Vec<Entry> {
    let mut entries: Vec<Entry> = Vec::new();
    // The commands of the row being read, as FAR wrote them: translated when the row ends,
    // so that the remarks about one row come out together.
    let mut raw: Vec<String> = Vec::new();
    // Whether the last row can still take commands or a `{`: a rule cannot, and neither can
    // a row whose submenu has just closed.
    let mut open = false;

    while let Some((n, line)) = lines.next() {
        let line = line.trim_end();
        if line.trim().is_empty() {
            continue;
        }
        let indented = line.starts_with([' ', '\t']);
        let body = line.trim_start();
        if !indented && body.starts_with('{') {
            match entries.last_mut().filter(|_| open) {
                Some(row) => {
                    if !raw.is_empty() {
                        notes.push(format!(
                            "`{}`: it has commands and a submenu; FAR opens the submenu, and \
                             so does this",
                            row.label
                        ));
                        raw.clear();
                    }
                    row.submenu = level(lines, notes, depth + 1);
                    if row.submenu.is_empty() {
                        row.says = vec!["this submenu is empty".to_string()];
                    }
                }
                None => {
                    notes.push(format!(
                        "line {}: a `{{` with no row above it — what it holds is skipped",
                        n + 1
                    ));
                    level(lines, &mut Vec::new(), depth + 1);
                }
            }
            open = false;
            continue;
        }
        if !indented && body.starts_with('}') {
            if depth == 0 {
                notes.push(format!("line {}: a `}}` that closes nothing", n + 1));
                continue;
            }
            finish(&mut entries, &mut raw, open, notes);
            return entries;
        }
        if indented {
            if open {
                raw.push(body.to_string());
            } else {
                notes.push(format!(
                    "line {}: `{body}` is a command with no row to belong to",
                    n + 1
                ));
            }
            continue;
        }
        // A row: the hot key, a colon, the label. FAR allows the colon itself as a hot key,
        // which is written `::`.
        finish(&mut entries, &mut raw, open, notes);
        let Some(mut colon) = line.find(':') else {
            notes.push(format!(
                "line {}: `{line}` has no `:` after a hot key, and FAR skips such a line too",
                n + 1
            ));
            open = false;
            continue;
        };
        if colon == 0 && line[1..].starts_with(':') {
            colon = 1;
        }
        let hot = line[..colon].trim();
        let label = line[colon + 1..].trim();
        if hot == "--" {
            entries.push(Entry {
                label: label.to_string(),
                separator: true,
                ..Entry::default()
            });
            open = false;
            continue;
        }
        let (label, marked) = unmark(label);
        let mut hot_chars = hot.chars();
        let key = match (hot_chars.next(), hot_chars.next()) {
            (None, _) => marked,
            (Some(c), None) => Some(c),
            _ => {
                notes.push(format!(
                    "`{label}`: the hot key `{hot}` is not one letter or digit, so the row has \
                     none — the arrows still reach it"
                ));
                marked
            }
        };
        let label = if label.is_empty() {
            // FAR draws a row with no label as a blank line one can still take.
            hot.to_string()
        } else {
            label
        };
        entries.push(Entry {
            key,
            label,
            ..Entry::default()
        });
        open = true;
    }
    finish(&mut entries, &mut raw, open, notes);
    if depth > 0 {
        notes.push("a `{` is never closed: the submenu runs to the end of the file".to_string());
    }
    entries
}

/// The row that was being read is complete: its commands become ours.
fn finish(entries: &mut [Entry], raw: &mut Vec<String>, open: bool, notes: &mut Vec<String>) {
    let commands = std::mem::take(raw);
    let Some(row) = entries.last_mut().filter(|_| open) else {
        return;
    };
    if row.is_submenu() || row.separator {
        return;
    }
    let mut refused = Vec::new();
    for command in &commands {
        let translated = command_line(command);
        for remark in translated.approximated {
            let note = format!("`{}`: {remark}", row.label);
            if !notes.contains(&note) {
                notes.push(note);
            }
        }
        for remark in translated.unsupported {
            if !refused.contains(&remark) {
                refused.push(remark);
            }
        }
        if !translated.line.trim().is_empty() {
            row.run.push(translated.line);
        }
    }
    if !refused.is_empty() {
        for remark in &refused {
            notes.push(format!("`{}`: {remark} — the row runs nothing", row.label));
        }
        row.run.clear();
        row.says = refused;
    } else if row.run.is_empty() {
        // FAR keeps a row with no commands, and so is it kept here: a menu whose letters
        // moved because a placeholder vanished is not the menu its owner knows.
        row.says = vec!["this row has no commands".to_string()];
    }
}

/// A label without FAR's `&`, and the letter the `&` marked. `&&` is an ampersand.
fn unmark(label: &str) -> (String, Option<char>) {
    let mut out = String::with_capacity(label.len());
    let mut marked = None;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            out.push(c);
            continue;
        }
        match chars.peek().copied() {
            Some('&') => {
                chars.next();
                out.push('&');
            }
            Some(next) if !next.is_whitespace() => {
                if marked.is_none() {
                    marked = Some(next);
                }
            }
            _ => out.push('&'),
        }
    }
    (out, marked)
}

/// One of FAR's command lines as one of ours.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Translated {
    pub line: String,
    /// What was mapped onto something close: said, and the row still runs.
    pub approximated: Vec<String>,
    /// What has no counterpart: the row is not run with a hole in it.
    pub unsupported: Vec<String>,
}

/// Which panel FAR's symbols are about, switched by `!#`, `!^`, `![` and `!]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Active,
    Passive,
    Left,
    Right,
}

impl Side {
    fn words(self) -> &'static str {
        match self {
            Side::Active => "the active panel",
            Side::Passive => "the other panel (`!#`)",
            Side::Left => "the left panel (`![`)",
            Side::Right => "the right panel (`!]`)",
        }
    }
}

/// What a symbol asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum What {
    Name,
    Stem,
    Ext,
    Dir,
    File,
    List,
    Selection,
}

impl What {
    fn words(self) -> &'static str {
        match self {
            What::Name => "file name",
            What::Stem => "file name without its extension",
            What::Ext => "extension",
            What::Dir => "path",
            What::File => "full file name",
            What::List => "list file of the selection",
            What::Selection => "selected names",
        }
    }
}

/// The named substitution for a symbol about a panel, where there is one.
fn named(side: Side, what: What) -> Option<&'static str> {
    Some(match (side, what) {
        (Side::Active, What::Name) => "{name}",
        (Side::Active, What::Stem) => "{stem}",
        (Side::Active, What::Ext) => "{ext}",
        (Side::Active, What::Dir) => "{dir}",
        (Side::Active, What::File) => "{file}",
        (Side::Active, What::List) => "{list}",
        (Side::Active, What::Selection) => "{selection}",
        (Side::Passive, What::Name) => "{other.name}",
        (Side::Passive, What::Dir) => "{other.dir}",
        (Side::Passive, What::File) => "{other.file}",
        (Side::Left, What::Dir) => "{left.dir}",
        (Side::Right, What::Dir) => "{right.dir}",
        _ => return None,
    })
}

/// Rewrite FAR's metasymbols as the named substitutions (SPEC 06 §5b).
pub fn command_line(far: &str) -> Translated {
    let chars: Vec<char> = far.chars().collect();
    let mut out = Translated::default();
    let mut side = Side::Active;
    let mut i = 0;
    // A leading `@` is FAR's "do not echo this command": there is nothing here it would
    // switch off, and a shell would take it for part of the program's name.
    if chars.first() == Some(&'@') {
        i = 1;
    }
    let at = |i: usize| chars.get(i).copied();
    let short = "short (8.3) names do not exist here: the long name is used";
    // FAR puts a name into the line as it is, so a careful menu writes `"!.!"`. Our
    // substitutions quote themselves, which inside somebody's own quotes would be quotes
    // within quotes — so there the bare value is asked for (`{name.raw}`), and the line
    // means what its author wrote.
    let mut quoted = false;

    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            quoted = !quoted;
        }
        // A brace of the command's own must not be read as one of our substitutions.
        if c == '{' || c == '}' {
            out.line.push(c);
            out.line.push(c);
            i += 1;
            continue;
        }
        if c != '!' {
            out.line.push(c);
            i += 1;
            continue;
        }
        // What the symbol is, how long it is, and whether it was the 8.3 spelling.
        let (what, len, is_short): (What, usize, bool) = match (at(i + 1), at(i + 2), at(i + 3)) {
            (Some('!'), ..) => {
                // A literal `!`, which is spelled the same way on our side.
                out.line.push_str("!!");
                i += 2;
                continue;
            }
            (Some('#'), ..) => {
                side = Side::Passive;
                i += 2;
                continue;
            }
            (Some('^'), ..) => {
                side = Side::Active;
                i += 2;
                continue;
            }
            (Some('['), ..) => {
                side = Side::Left;
                i += 2;
                continue;
            }
            (Some(']'), ..) => {
                side = Side::Right;
                i += 2;
                continue;
            }
            (Some('?'), ..) => {
                // `!?title?initial!`: a question, which the command line already asks in
                // FAR's own spelling — passed through whole, up to the `!` that ends it.
                match chars[i + 2..].iter().position(|c| *c == '!') {
                    Some(end) => {
                        out.line.extend(&chars[i..=i + 2 + end]);
                        i += end + 3;
                    }
                    None => {
                        out.unsupported
                            .push("a `!?` question that is never closed by a `!`".to_string());
                        i = chars.len();
                    }
                }
                continue;
            }
            (Some(':'), ..) => {
                out.unsupported
                    .push("`!:` (the drive letter) has no counterpart here".to_string());
                i += 2;
                continue;
            }
            (Some('.'), Some('!'), _) => (What::Name, 3, false),
            (Some('-' | '+'), Some('!'), _) => (What::Name, 3, true),
            (Some('`'), Some('~'), _) => (What::Ext, 3, true),
            (Some('`'), ..) => (What::Ext, 2, false),
            (Some('&'), Some('~'), _) => (What::Selection, 3, true),
            (Some('&'), ..) => (What::Selection, 2, false),
            (Some('='), Some('\\' | '/'), _) => {
                let remark = "`!=\\` is read as `!\\`: links in the path are not resolved";
                if !out.approximated.iter().any(|r| r == remark) {
                    out.approximated.push(remark.to_string());
                }
                (What::Dir, 3, at(i + 2) == Some('/'))
            }
            (Some('\\'), ..) => (What::Dir, 2, false),
            (Some('/'), ..) => (What::Dir, 2, true),
            (Some(list @ ('@' | '$')), ..) => {
                // `!@!`, or `!@QSF!` with FAR's modifiers between the two.
                let mods: String = chars[i + 2..]
                    .iter()
                    .take_while(|c| "QSFAUWqsfauw".contains(**c))
                    .collect();
                if at(i + 2 + mods.len()) == Some('!') {
                    if !mods.is_empty() {
                        out.approximated.push(format!(
                            "the list file modifiers `{mods}` are not applied: the list is \
                             plain names, one per line, in UTF-8"
                        ));
                    }
                    (What::List, 3 + mods.len(), list == '$')
                } else {
                    // Not a list after all: FAR reads the `!` alone, as the name.
                    (What::Stem, 1, false)
                }
            }
            (Some('~'), ..) => (What::Stem, 2, true),
            _ => (What::Stem, 1, false),
        };
        i += len;
        if is_short && !out.approximated.iter().any(|r| r == short) {
            out.approximated.push(short.to_string());
        }
        // FAR's path ends in a separator, so `!\!.!` is the whole file name — which is one
        // substitution of ours, and the only way to quote the two as one word.
        let mut what = what;
        if what == What::Dir {
            let name_follows = at(i) == Some('!')
                && matches!(at(i + 1), Some('.' | '-' | '+'))
                && at(i + 2) == Some('!');
            if name_follows {
                if at(i + 1) != Some('.') && !out.approximated.iter().any(|r| r == short) {
                    out.approximated.push(short.to_string());
                }
                what = What::File;
                i += 3;
            }
        }
        match named(side, what) {
            // The two lists quote each name themselves and are never one word.
            Some(name) if quoted && !matches!(what, What::List | What::Selection) => {
                out.line.push_str(name.trim_end_matches('}'));
                out.line.push_str(".raw}");
                if what == What::Dir {
                    out.line.push(std::path::MAIN_SEPARATOR);
                }
            }
            Some(name) => {
                out.line.push_str(name);
                if what == What::Dir {
                    out.line.push(std::path::MAIN_SEPARATOR);
                }
            }
            None => {
                let remark = format!(
                    "the {} of {} has no counterpart here",
                    what.words(),
                    side.words()
                );
                if !out.unsupported.contains(&remark) {
                    out.unsupported.push(remark);
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape of the file FAR ships as an example, with a submenu in a submenu.
    const SAMPLE: &str = "\
V:   View the file under the cursor
    view:!.!
E:   &Edit FarMenu.ini here
    edit:FarMenu.ini
--:
C:   Compress
{
Z:   to &zip, the selected files
    zip -r !.zip !&
T:   to tar.gz, from a list
    tar czf \"!\\!.tar.gz\" -T !@!
--:  Elsewhere
O:   into the other panel
{
1:   one archive
    zip \"!#!\\!^!.zip\" !&
}
}
F5:  Rebuild
    @make clean
    make -j8
:    No hot key at all
    echo done
";

    fn labels(entries: &[Entry]) -> Vec<String> {
        entries
            .iter()
            .map(|e| {
                if e.separator {
                    format!("-- {}", e.label)
                } else if e.is_submenu() {
                    format!("{} >", e.label)
                } else {
                    e.label.clone()
                }
            })
            .collect()
    }

    #[test]
    fn a_far_menu_becomes_rows_with_keys_commands_rules_and_submenus() {
        let far = parse(SAMPLE);
        let top = &far.menu.entries;
        assert_eq!(
            labels(top),
            [
                "View the file under the cursor",
                "Edit FarMenu.ini here",
                "-- ",
                "Compress >",
                "Rebuild",
                "No hot key at all",
            ]
        );
        assert_eq!(top[0].key, Some('V'));
        assert_eq!(top[0].run, ["view:{name}"]);
        // The hot key before the colon wins over the letter an `&` marks, and the `&` is
        // FAR's way of underlining a letter, not part of the label.
        assert_eq!(top[1].key, Some('E'));
        assert_eq!(top[1].run, ["edit:FarMenu.ini"]);

        let compress = &top[3].submenu;
        assert_eq!(
            labels(compress),
            [
                "to zip, the selected files",
                "to tar.gz, from a list",
                "-- Elsewhere",
                "into the other panel >"
            ]
        );
        assert_eq!(compress[0].run, ["zip -r {stem}.zip {selection}"]);
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            compress[1].run,
            [format!(
                "tar czf \"{{dir.raw}}{sep}{{stem.raw}}.tar.gz\" -T {{list}}"
            )]
        );
        // The other panel's folder, then back to this one for the name.
        assert_eq!(
            compress[3].submenu[0].run,
            [format!(
                "zip \"{{other.dir.raw}}{sep}{{stem.raw}}.zip\" {{selection}}"
            )]
        );

        // A function key is not a letter: the row is there, without one, and it is said.
        assert_eq!(top[4].key, None);
        assert_eq!(top[4].run, ["make clean", "make -j8"]);
        assert_eq!(top[5].key, None);
        assert_eq!(far.notes.len(), 1, "{:?}", far.notes);
        assert!(far.notes[0].contains("F5"), "{:?}", far.notes);
        // And every row of it is one the TOML reader would have accepted.
        assert!(top.iter().all(|e| e.says.is_empty()));
    }

    #[test]
    fn the_metasymbols_that_have_a_counterpart_are_mapped_onto_it() {
        let sep = std::path::MAIN_SEPARATOR;
        let line = |far: &str| {
            let t = command_line(far);
            assert!(t.approximated.is_empty(), "{far}: {:?}", t.approximated);
            assert!(t.unsupported.is_empty(), "{far}: {:?}", t.unsupported);
            t.line
        };
        assert_eq!(line("type !.!"), "type {name}");
        assert_eq!(line("ren !.! !.bak"), "ren {name} {stem}.bak");
        assert_eq!(line("echo !`"), "echo {ext}");
        assert_eq!(line("cd !\\"), format!("cd {{dir}}{sep}"));
        assert_eq!(line("copy !\\!.! d:"), "copy {file} d:");
        // Inside the author's own quotes the value goes in bare, as FAR puts it.
        assert_eq!(
            line("copy \"!\\!.!\" \"!#!\\\" !^!&"),
            format!("copy \"{{file.raw}}\" \"{{other.dir.raw}}{sep}\" {{selection}}")
        );
        assert_eq!(line("pack !@!"), "pack {list}");
        assert_eq!(line("pack !&"), "pack {selection}");
        // The other panel, and back; the left and the right by name.
        assert_eq!(
            line("fc !.! !#!\\!.! !^!.!"),
            "fc {name} {other.file} {name}"
        );
        assert_eq!(line("cmp !#!.!"), "cmp {other.name}");
        assert_eq!(
            line("diff -r ![!\\ !]!\\"),
            format!("diff -r {{left.dir}}{sep} {{right.dir}}{sep}")
        );
        // A literal `!` stays one, a question is asked in FAR's own words, and a brace of
        // the command's own is not taken for a substitution.
        assert_eq!(line("echo done!!"), "echo done!!");
        assert_eq!(
            line("grep !?$find$Search for?TODO! !.!"),
            "grep !?$find$Search for?TODO! {name}"
        );
        assert_eq!(line("awk '{print $1}' !.!"), "awk '{{print $1}}' {name}");
        assert_eq!(line("@echo off"), "echo off");
    }

    #[test]
    fn what_is_only_close_is_mapped_and_said_and_what_is_missing_stops_the_row() {
        let t = command_line("dir !~ !-! !`~ !/ !$! !&~");
        let sep = std::path::MAIN_SEPARATOR;
        assert_eq!(
            t.line,
            format!("dir {{stem}} {{name}} {{ext}} {{dir}}{sep} {{list}} {{selection}}")
        );
        assert_eq!(t.approximated.len(), 1, "said once: {:?}", t.approximated);
        assert!(t.approximated[0].contains("8.3"));
        assert!(t.unsupported.is_empty());

        let t = command_line("cd !=\\ && pack !@QF!");
        assert_eq!(t.line, format!("cd {{dir}}{sep} && pack {{list}}"));
        assert_eq!(t.approximated.len(), 2, "{:?}", t.approximated);
        assert!(t.approximated[1].contains("QF"), "{:?}", t.approximated);

        // The drive, the other panel's selection and an unfinished question have nothing
        // to stand for them.
        for (far, word) in [
            ("format !:", "drive"),
            ("del !#!&", "selected names"),
            ("ren !#! x", "without its extension"),
            ("type ![!.!", "left panel"),
            ("echo !?never closed", "never closed"),
        ] {
            let t = command_line(far);
            assert_eq!(t.unsupported.len(), 1, "{far}: {:?}", t.unsupported);
            assert!(
                t.unsupported[0].contains(word),
                "{far}: {:?}",
                t.unsupported
            );
        }

        let far = parse("D:  Free space\n    dir !:\n    echo !.!\nX:  Placeholder\n");
        let rows = &far.menu.entries;
        // The row is on the menu and holds nothing to run; taking it says why.
        assert!(rows[0].run.is_empty());
        assert!(rows[0].says[0].contains("drive"), "{:?}", rows[0].says);
        assert!(rows[1].run.is_empty() && !rows[1].says.is_empty());
        assert_eq!(far.notes.len(), 1, "{:?}", far.notes);
        assert!(far.notes[0].contains("Free space"), "{:?}", far.notes);
    }

    #[test]
    fn a_file_that_is_not_quite_right_is_read_as_far_as_it_goes() {
        let far = parse(
            "    orphan command\n\
             {\n\
             X:  lost\n    never\n\
             }\n\
             no colon here\n\
             A:  First\n    one\n\
             }\n\
             B:  Both\n    two\n\
             {\n\
             C:  Inner\n    three\n",
        );
        assert_eq!(labels(&far.menu.entries), ["First", "Both >"]);
        assert_eq!(far.menu.entries[1].submenu[0].run, ["three"]);
        let said = far.notes.join("\n");
        for word in [
            "orphan command",
            "no row above",
            "no `:`",
            "closes nothing",
            "commands and a submenu",
            "never closed",
        ] {
            assert!(said.contains(word), "{word} in\n{said}");
        }
        assert_eq!(far.notes.len(), 6, "{said}");

        // `::` is a row whose hot key is the colon.
        let far = parse("::  Colon\n    true\n");
        assert_eq!(far.menu.entries[0].key, Some(':'));
        assert_eq!(far.menu.entries[0].label, "Colon");
    }

    #[test]
    fn far_three_writes_utf_16_and_far_two_utf_8_and_both_are_read() {
        let text = "Ф:  Файлы\r\n    echo привет\r\n";
        let read = |bytes: &[u8]| {
            let (text, lossy) = decode(bytes);
            (parse(&text), lossy)
        };

        let mut utf16 = vec![0xff, 0xfe];
        utf16.extend(text.encode_utf16().flat_map(u16::to_le_bytes));
        let (far, lossy) = read(&utf16);
        assert!(!lossy);
        assert_eq!(far.menu.entries[0].key, Some('Ф'));
        assert_eq!(far.menu.entries[0].label, "Файлы");
        assert_eq!(far.menu.entries[0].run, ["echo привет"]);
        assert!(far.notes.is_empty(), "{:?}", far.notes);

        let mut big = vec![0xfe, 0xff];
        big.extend(text.encode_utf16().flat_map(u16::to_be_bytes));
        assert_eq!(read(&big), (far.clone(), false));
        let mut utf8 = vec![0xef, 0xbb, 0xbf];
        utf8.extend(text.as_bytes());
        assert_eq!(read(&utf8), (far.clone(), false));
        assert_eq!(read(text.as_bytes()), (far, false));

        // An old OEM file: read, with the letters that are not UTF-8 replaced — which the
        // caller says — and its commands still there.
        let (far, lossy) = read(b"A:  caf\xe9\n    echo ok\n");
        assert!(lossy);
        assert_eq!(far.menu.entries[0].run, ["echo ok"]);
    }
}
