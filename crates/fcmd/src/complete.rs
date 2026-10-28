//! What the command line could become: autocomplete (SPEC 06 §7.4).
//!
//! Four sources, each one a setting with three states — on, off, or only when asked for with
//! `Ctrl+Space`. The arithmetic is here and pure: given a line, a caret and what the
//! application already knows, it answers with the lines the command line could become. The
//! application does the asking, the drawing and the taking.
//!
//! Nothing here reads the filesystem. A listing is handed in, because the panel already has
//! the one for the directory it is showing and reading it again per keystroke is the kind of
//! call that has no business inside a key handler.

use fcmd_config::CompleteCfg;

/// How keen a source is to answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum When {
    Off,
    /// While one types.
    On,
    /// Only when `Ctrl+Space` asks.
    Asked,
}

impl When {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "no" | "false" => Some(When::Off),
            "on" | "yes" | "true" | "always" => Some(When::On),
            "asked" | "ask" | "on-demand" | "demand" => Some(When::Asked),
            _ => None,
        }
    }

    /// Does this source answer now? `by_hand` is a completion `Ctrl+Space` asked for.
    fn answers(self, by_hand: bool) -> bool {
        match self {
            When::Off => false,
            When::On => true,
            When::Asked => by_hand,
        }
    }
}

/// What the application knows, handed in rather than looked up.
pub struct Sources<'a> {
    /// The names in the directory the active panel is showing, and whether each is a
    /// directory. Already in memory: this is the panel's own listing.
    pub here: &'a [(String, bool)],
    /// The names in the directory a word's own path names, when the word has a separator in
    /// it and somebody read that directory for us. Empty otherwise.
    pub under: &'a [(String, bool)],
    /// The command lines that have run, newest last.
    pub history: &'a [String],
    /// The names of the executables on the `PATH`, gathered once.
    pub path: &'a [String],
    /// The environment's variable names.
    pub env: &'a [String],
    /// What the program runs itself, and its prefixes: `cd`, `cls`, `view:`, `edit:`.
    pub commands: &'a [&'a str],
}

/// One answer: what the line becomes, where the caret lands, and what the list shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub line: String,
    pub caret: usize,
    pub text: String,
}

/// What the program runs itself, and the prefixes that hand a line to a part of it — the
/// short, certain half of what a first word can be (SPEC 06 §7.1 and §7.2).
pub const OUR_OWN: &[&str] = &[
    "cd", "cls", "clear", "exit", "set", "pushd", "popd", "clrd", "title", "view:", "edit:",
    "goto:", "whereis:", "run:",
];

/// The names of the executables on the `PATH`.
///
/// One read of every directory on the `PATH`, which is why it is done once and kept: the
/// `PATH` does not change under a running program, and a completion that re-read it per
/// keystroke would be a completion nobody leaves on.
pub fn path_names() -> Vec<String> {
    let Some(path) = std::env::var_os("PATH") else {
        return Vec::new();
    };
    let mut names: Vec<String> = Vec::new();
    for dir in std::env::split_paths(&path) {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten().take(4096) {
            // A directory on the `PATH` is not a command, and on Unix a file nobody may run
            // is not one either — which is most of what makes this list short enough to read.
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                continue;
            }
            if !runnable(&entry) {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.sort_by_key(|name| name.to_lowercase());
    names
}

#[cfg(unix)]
fn runnable(entry: &std::fs::DirEntry) -> bool {
    use std::os::unix::fs::PermissionsExt;
    entry
        .metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn runnable(entry: &std::fs::DirEntry) -> bool {
    // Windows decides by extension, and `PATHEXT` is the list it decides from.
    let name = entry.file_name().to_string_lossy().to_ascii_lowercase();
    let exts = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
    exts.split(';')
        .map(|e| e.trim().to_ascii_lowercase())
        .any(|e| !e.is_empty() && name.ends_with(&e))
}

/// The word the caret is in, as character positions. Whitespace separates words, and a word
/// in quotes is one word — a path with a space in it is what quotes are for.
pub fn word_at(line: &str, caret: usize) -> (usize, usize) {
    let chars: Vec<char> = line.chars().collect();
    let caret = caret.min(chars.len());
    // Where the quoting stands at the caret, so a space inside quotes does not split.
    let mut start = 0usize;
    let mut quoted = false;
    for (i, c) in chars.iter().enumerate().take(caret) {
        match c {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => start = i + 1,
            _ => {}
        }
    }
    // The opening quote is not part of the word: what is completed is the name inside it, and
    // the quote stays where it is.
    if chars.get(start) == Some(&'"') {
        start += 1;
    }
    let mut end = caret;
    while end < chars.len() && (quoted || !chars[end].is_whitespace()) {
        if chars[end] == '"' {
            break;
        }
        end += 1;
    }
    (start, end)
}

/// Is the word at `start` the first word of the line — the one that names a command?
fn at_command(line: &str, start: usize) -> bool {
    line.chars()
        .take(start)
        .all(|c| c.is_whitespace() || c == '"')
}

/// What the line could become. Empty when nothing has anything to say.
pub fn rows(line: &str, caret: usize, by_hand: bool, cfg: &CompleteCfg, src: &Sources) -> Vec<Row> {
    let files = When::parse(&cfg.files).unwrap_or(When::On);
    let history = When::parse(&cfg.history).unwrap_or(When::On);
    let path = When::parse(&cfg.path).unwrap_or(When::Asked);
    let env = When::parse(&cfg.env).unwrap_or(When::On);

    let (start, end) = word_at(line, caret);
    let chars: Vec<char> = line.chars().collect();
    let word: String = chars[start..end.min(chars.len())].iter().collect();
    let command_position = at_command(line, start);

    let mut out: Vec<Row> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let push = |out: &mut Vec<Row>, seen: &mut Vec<String>, text: String, replace: &str| {
        if seen.iter().any(|s| s == &text) {
            return;
        }
        seen.push(text.clone());
        // What the line becomes: the word swapped for the answer, the rest left alone.
        let before: String = chars[..start].iter().collect();
        let after: String = chars[end.min(chars.len())..].iter().collect();
        let line = format!("{before}{replace}{after}");
        let caret = before.chars().count() + replace.chars().count();
        out.push(Row { line, caret, text });
    };

    // A variable: `$HOME` here, `%PATH%` on Windows. Before everything else, because a word
    // that starts with one of those is not a file and not a command.
    let sigil = word.starts_with('$') || word.starts_with('%');
    if env.answers(by_hand) && sigil {
        let bare = word.trim_start_matches(['$', '%']);
        for name in src.env {
            if !starts(name, bare) {
                continue;
            }
            let text = if word.starts_with('%') {
                format!("%{name}%")
            } else {
                format!("${name}")
            };
            push(&mut out, &mut seen, text.clone(), &text);
        }
    }

    if !sigil {
        // The whole line again, out of the history. FAR completes the line rather than the
        // word here, and that is the point of it: a line one has run is a line one may run
        // again, with the end of it already typed.
        if history.answers(by_hand) && caret == chars.len() && !line.trim().is_empty() {
            for past in src.history.iter().rev() {
                if past.as_str() != line && starts(past, line) {
                    let caret = past.chars().count();
                    let text = past.clone();
                    if !seen.iter().any(|s| s == &text) {
                        seen.push(text.clone());
                        out.push(Row {
                            line: past.clone(),
                            caret,
                            text,
                        });
                    }
                }
            }
        }

        if command_position {
            // What the program runs itself, then what is on the `PATH`: the first list is
            // short and certain, the second is long and only as good as the environment.
            for name in src.commands {
                if starts(name, &word) {
                    push(&mut out, &mut seen, (*name).to_string(), name);
                }
            }
            if path.answers(by_hand) {
                for name in src.path {
                    if starts(name, &word) {
                        push(&mut out, &mut seen, name.clone(), name);
                    }
                }
            }
        }

        if files.answers(by_hand) {
            // A word with a separator in it names a directory of its own, and then it is the
            // last part that is being completed; the application hands in that directory's
            // names as `under`.
            let (prefix, stem) = split_path(&word);
            let listing = if prefix.is_empty() {
                src.here
            } else {
                src.under
            };
            for (name, is_dir) in listing {
                if !starts(name, stem) {
                    continue;
                }
                // A directory keeps its separator, so the next keystroke carries on inside it
                // rather than starting again.
                let tail = if *is_dir { "/" } else { "" };
                let text = format!("{name}{tail}");
                let replace = format!("{prefix}{name}{tail}");
                push(&mut out, &mut seen, text, &replace);
            }
        }
    }

    // Sorted by what the list shows, case last: a list that changes order as one types is a
    // list nobody can aim at. The history keeps its own order, newest first, so it is sorted
    // separately and stays in front.
    let from_history = out
        .iter()
        .take_while(|row| row.text.contains(' ') && row.line == row.text)
        .count();
    out[from_history..].sort_by(|a, b| {
        a.text
            .to_lowercase()
            .cmp(&b.text.to_lowercase())
            .then_with(|| a.text.cmp(&b.text))
    });
    out.truncate(cfg.limit.max(1) as usize);
    out
}

/// Does `name` begin with `prefix`, ignoring case? Ignoring it is what a shell does and what
/// anybody typing two letters of a name expects.
fn starts(name: &str, prefix: &str) -> bool {
    if prefix.is_empty() {
        return true;
    }
    name.len() >= prefix.len()
        && name
            .chars()
            .zip(prefix.chars())
            .all(|(a, b)| a.eq_ignore_ascii_case(&b) || a.to_lowercase().eq(b.to_lowercase()))
}

/// A word into the directory part (with its separator) and the part being completed.
pub fn split_path(word: &str) -> (&str, &str) {
    match word.rfind(['/', '\\']) {
        Some(at) => (&word[..=at], &word[at + 1..]),
        None => ("", word),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CompleteCfg {
        CompleteCfg::default()
    }

    fn listing(names: &[(&str, bool)]) -> Vec<(String, bool)> {
        names.iter().map(|(n, d)| ((*n).to_string(), *d)).collect()
    }

    const COMMANDS: &[&str] = &["cd", "cls", "exit", "view:", "edit:"];

    fn sources<'a>(
        here: &'a [(String, bool)],
        under: &'a [(String, bool)],
        history: &'a [String],
        path: &'a [String],
        env: &'a [String],
    ) -> Sources<'a> {
        Sources {
            here,
            under,
            history,
            path,
            env,
            commands: COMMANDS,
        }
    }

    #[test]
    fn the_word_under_the_caret_is_what_gets_replaced() {
        assert_eq!(word_at("vim REA", 7), (4, 7));
        assert_eq!(word_at("vim REA", 5), (4, 7));
        assert_eq!(word_at("", 0), (0, 0));
        // A quoted path is one word, spaces and all.
        assert_eq!(word_at("cp \"my file", 11), (4, 11));
    }

    #[test]
    fn a_name_from_the_panels_own_listing_finishes_the_word() {
        let here = listing(&[("README.md", false), ("src", true), ("Cargo.toml", false)]);
        let found = rows(
            "vim REA",
            7,
            false,
            &cfg(),
            &sources(&here, &[], &[], &[], &[]),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, "vim README.md");
        assert_eq!(found[0].caret, "vim README.md".chars().count());

        // A directory keeps its separator, so the next keystroke carries on inside it.
        let found = rows(
            "vim s",
            5,
            false,
            &cfg(),
            &sources(&here, &[], &[], &[], &[]),
        );
        assert_eq!(found[0].line, "vim src/");
    }

    #[test]
    fn the_first_word_is_a_command_and_the_rest_are_files() {
        let here = listing(&[("code.rs", false)]);
        let path: Vec<String> = vec!["cargo".into(), "clang".into()];
        // In command position the program's own words and the PATH answer — the PATH only
        // when asked, which is its default.
        let found = rows("c", 1, false, &cfg(), &sources(&here, &[], &[], &path, &[]));
        let shown: Vec<&str> = found.iter().map(|r| r.text.as_str()).collect();
        assert!(shown.contains(&"cd"), "{shown:?}");
        assert!(shown.contains(&"cls"), "{shown:?}");
        assert!(!shown.contains(&"cargo"), "the PATH waits to be asked");
        assert!(
            shown.contains(&"code.rs"),
            "a file is a command too: {shown:?}"
        );

        let found = rows("c", 1, true, &cfg(), &sources(&here, &[], &[], &path, &[]));
        let shown: Vec<&str> = found.iter().map(|r| r.text.as_str()).collect();
        assert!(shown.contains(&"cargo"), "asked for by hand: {shown:?}");
    }

    #[test]
    fn the_history_finishes_the_whole_line_and_comes_first() {
        let history: Vec<String> = vec![
            "cargo build --release".into(),
            "cargo test --workspace".into(),
        ];
        let found = rows(
            "cargo t",
            7,
            false,
            &cfg(),
            &sources(&[], &[], &history, &[], &[]),
        );
        assert_eq!(found[0].line, "cargo test --workspace");
        assert_eq!(found[0].caret, "cargo test --workspace".chars().count());
        // The line as typed is not offered back to itself.
        let found = rows(
            "cargo test --workspace",
            22,
            false,
            &cfg(),
            &sources(&[], &[], &history, &[], &[]),
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn a_word_with_a_separator_is_completed_inside_its_own_directory() {
        let here = listing(&[("src", true)]);
        let under = listing(&[("main.rs", false), ("app.rs", false)]);
        let found = rows(
            "vim src/ma",
            10,
            false,
            &cfg(),
            &sources(&here, &under, &[], &[], &[]),
        );
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!(found[0].line, "vim src/main.rs");
        assert_eq!(split_path("src/ma"), ("src/", "ma"));
        assert_eq!(split_path("ma"), ("", "ma"));
    }

    #[test]
    fn a_variable_is_a_variable_and_nothing_else() {
        let here = listing(&[("HOMEWORK", false)]);
        let env: Vec<String> = vec!["HOME".into(), "HOSTNAME".into()];
        let found = rows(
            "ls $HO",
            6,
            false,
            &cfg(),
            &sources(&here, &[], &[], &[], &env),
        );
        let shown: Vec<&str> = found.iter().map(|r| r.text.as_str()).collect();
        assert_eq!(shown, vec!["$HOME", "$HOSTNAME"], "{shown:?}");
        assert!(
            !shown.contains(&"HOMEWORK"),
            "a word that starts with a sigil is not a file"
        );
    }

    #[test]
    fn a_source_that_is_off_says_nothing_even_when_asked() {
        let here = listing(&[("README.md", false)]);
        let mut cfg = cfg();
        cfg.files = "off".into();
        let found = rows(
            "vim REA",
            7,
            true,
            &cfg,
            &sources(&here, &[], &[], &[], &[]),
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn two_sources_naming_the_same_thing_are_one_row() {
        let here = listing(&[("cargo", false)]);
        let path: Vec<String> = vec!["cargo".into()];
        let found = rows(
            "car",
            3,
            true,
            &cfg(),
            &sources(&here, &[], &[], &path, &[]),
        );
        assert_eq!(found.len(), 1, "{found:?}");
    }
}
