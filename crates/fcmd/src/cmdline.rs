//! The command line under the panels — `docs/spec/06-tools.md` §7.
//!
//! All of it is pure state and parsing, so the awkward parts (where the caret lands after a
//! word jump, what `if not exist a b if defined c d` means) are tested without a terminal.
//! Running the line is the application's job; deciding *what* the line says is this module's.

use std::path::{Path, PathBuf};

/// Where a word ends, for `Ctrl+←/→` and `Ctrl+Backspace`. A path separator counts, so
/// `Ctrl+Backspace` in `/usr/local/bin` eats one component rather than the whole line.
fn is_word_break(c: char) -> bool {
    c.is_whitespace() || matches!(c, '/' | '\\' | ',' | ';' | ':' | '=' | '"' | '\'')
}

/// The line, its caret, and the history behind it.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct CmdLine {
    pub text: String,
    /// Caret position in characters, never in bytes.
    pub caret: usize,
    /// Newest last. The line being typed is not in here until it runs.
    history: Vec<String>,
    /// Where we are while walking the history; `None` means "on the line being typed".
    walking: Option<usize>,
    /// What was being typed before the walk started, so coming back is lossless.
    stash: String,
    /// `pushd`'s stack of directories.
    pub dirs: Vec<PathBuf>,
}

impl CmdLine {
    pub fn new() -> CmdLine {
        CmdLine::default()
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn len(&self) -> usize {
        self.text.chars().count()
    }

    fn byte_at(&self, caret: usize) -> usize {
        self.text
            .char_indices()
            .nth(caret)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len())
    }

    pub fn insert(&mut self, c: char) {
        let at = self.byte_at(self.caret);
        self.text.insert(at, c);
        self.caret += 1;
        self.walking = None;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            self.insert(c);
        }
    }

    pub fn backspace(&mut self) {
        if self.caret == 0 {
            return;
        }
        let at = self.byte_at(self.caret - 1);
        self.text.remove(at);
        self.caret -= 1;
        self.walking = None;
    }

    pub fn delete(&mut self) {
        if self.caret >= self.len() {
            return;
        }
        let at = self.byte_at(self.caret);
        self.text.remove(at);
        self.walking = None;
    }

    pub fn left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    pub fn right(&mut self) {
        self.caret = (self.caret + 1).min(self.len());
    }

    pub fn home(&mut self) {
        self.caret = 0;
    }

    pub fn end(&mut self) {
        self.caret = self.len();
    }

    /// The start of the word to the left of the caret.
    fn word_left_of(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let mut at = from;
        while at > 0 && is_word_break(chars[at - 1]) {
            at -= 1;
        }
        while at > 0 && !is_word_break(chars[at - 1]) {
            at -= 1;
        }
        at
    }

    /// The start of the word to the right of the caret.
    fn word_right_of(&self, from: usize) -> usize {
        let chars: Vec<char> = self.text.chars().collect();
        let len = chars.len();
        let mut at = from;
        while at < len && !is_word_break(chars[at]) {
            at += 1;
        }
        while at < len && is_word_break(chars[at]) {
            at += 1;
        }
        at
    }

    pub fn word_left(&mut self) {
        self.caret = self.word_left_of(self.caret);
    }

    pub fn word_right(&mut self) {
        self.caret = self.word_right_of(self.caret);
    }

    pub fn delete_word_left(&mut self) {
        let to = self.word_left_of(self.caret);
        let from = self.byte_at(to);
        let until = self.byte_at(self.caret);
        self.text.replace_range(from..until, "");
        self.caret = to;
        self.walking = None;
    }

    pub fn delete_word_right(&mut self) {
        let to = self.word_right_of(self.caret);
        let from = self.byte_at(self.caret);
        let until = self.byte_at(to);
        self.text.replace_range(from..until, "");
        self.walking = None;
    }

    /// `Ctrl+K`.
    pub fn kill_to_end(&mut self) {
        let at = self.byte_at(self.caret);
        self.text.truncate(at);
        self.walking = None;
    }

    /// `Ctrl+Y`, and what `Esc` does.
    pub fn clear(&mut self) {
        self.text.clear();
        self.caret = 0;
        self.walking = None;
        self.stash.clear();
    }

    pub fn set(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.caret = self.len();
        self.walking = None;
    }

    // ── history ────────────────────────────────────────────────────────────────────────────

    pub fn history(&self) -> &[String] {
        &self.history
    }

    /// Remember a line that ran. The same command twice in a row is one entry, and a repeat
    /// moves to the end rather than piling up — that is what makes a history worth walking.
    pub fn remember(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        self.history.retain(|h| h != line);
        self.history.push(line.to_string());
        self.walking = None;
        self.stash.clear();
    }

    /// Load a history read from the state file, oldest first.
    pub fn load_history(&mut self, lines: Vec<String>) {
        self.history = lines;
    }

    /// Drop the oldest entries so the stored history stays bounded.
    pub fn trim_history(&mut self, keep: usize) {
        if self.history.len() > keep {
            let drop = self.history.len() - keep;
            self.history.drain(..drop);
        }
    }

    /// `Ctrl+E`: one step back through the history.
    pub fn history_prev(&mut self) -> bool {
        if self.history.is_empty() {
            return false;
        }
        let next = match self.walking {
            None => {
                self.stash = self.text.clone();
                self.history.len() - 1
            }
            Some(0) => return false,
            Some(at) => at - 1,
        };
        self.walking = Some(next);
        self.text = self.history[next].clone();
        self.caret = self.len();
        true
    }

    /// `Ctrl+X`: forward again, and past the newest entry back to what was being typed.
    pub fn history_next(&mut self) -> bool {
        let Some(at) = self.walking else {
            return false;
        };
        if at + 1 >= self.history.len() {
            self.walking = None;
            self.text = std::mem::take(&mut self.stash);
            self.caret = self.len();
            return true;
        }
        self.walking = Some(at + 1);
        self.text = self.history[at + 1].clone();
        self.caret = self.len();
        true
    }
}

// ── what a line means ───────────────────────────────────────────────────────────────────────

/// A command the program runs itself, because shelling out would lose the panel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Internal {
    /// `cd`, `chdir`. `chdir` always means the local filesystem, even on a remote panel.
    Cd {
        path: String,
        local_only: bool,
    },
    Cls,
    /// `set`, `set NAME`, `set NAME=value`, `set NAME=` (which removes it).
    Set {
        name: Option<String>,
        value: Option<String>,
    },
    PushD(Option<String>),
    PopD,
    ClrD,
    Title(Option<String>),
    Exit,
}

/// A prefix that hands the rest of the line to a part of the program instead of to a shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Prefix {
    View(String),
    Edit(String),
    Goto(String),
    WhereIs(String),
    /// `run:file<command` — run the command and keep its output in `file`, or on the screen
    /// when no file is given.
    Run {
        into: Option<String>,
        command: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Parsed {
    Empty,
    Internal(Internal),
    Prefix(Prefix),
    /// Anything else goes to the shell.
    Shell(String),
}

/// `if [not] exist <path> <command>` and `if [not] defined <name> <command>`, which may nest.
/// Returns what is left to run once the tests have passed, or `None` when one failed.
pub fn resolve_conditions(
    line: &str,
    exists: &dyn Fn(&str) -> bool,
    defined: &dyn Fn(&str) -> bool,
) -> Option<String> {
    let mut rest = line.trim();
    // Nesting is a loop, not recursion: `if a if b if c cmd` is the normal shape.
    loop {
        let lower = rest.to_ascii_lowercase();
        if !lower.starts_with("if ") {
            return Some(rest.to_string());
        }
        let mut words = rest[3..].trim_start();
        let negate = {
            let lower = words.to_ascii_lowercase();
            if lower.starts_with("not ") {
                words = words[4..].trim_start();
                true
            } else {
                false
            }
        };
        let lower = words.to_ascii_lowercase();
        let (test_exist, after) = if lower.starts_with("exist ") {
            (true, words[6..].trim_start())
        } else if lower.starts_with("defined ") {
            (false, words[8..].trim_start())
        } else {
            // Not a form we know: let the shell have it rather than guessing.
            return Some(rest.to_string());
        };
        let (argument, tail) = split_first_word(after);
        if argument.is_empty() {
            return Some(rest.to_string());
        }
        let holds = if test_exist {
            exists(&argument)
        } else {
            defined(&argument)
        };
        if holds == negate {
            return None;
        }
        rest = tail.trim();
        if rest.is_empty() {
            return None;
        }
    }
}

/// The first word, honouring double quotes, and what follows it.
fn split_first_word(s: &str) -> (String, &str) {
    let s = s.trim_start();
    if let Some(rest) = s.strip_prefix('"') {
        if let Some(end) = rest.find('"') {
            return (rest[..end].to_string(), &rest[end + 1..]);
        }
        return (rest.to_string(), "");
    }
    match s.find(char::is_whitespace) {
        Some(at) => (s[..at].to_string(), &s[at..]),
        None => (s.to_string(), ""),
    }
}

/// What a line is, before anything has been expanded.
pub fn parse(line: &str) -> Parsed {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Parsed::Empty;
    }

    // A prefix binds tighter than anything else, and `view:` must not be confused with a
    // Windows drive (`c:`), which is why the name has to be at least two characters.
    if let Some((head, rest)) = trimmed.split_once(':') {
        let name = head.trim().to_ascii_lowercase();
        let rest = rest.trim().to_string();
        if name.len() > 1 && name.chars().all(|c| c.is_ascii_alphabetic()) {
            match name.as_str() {
                "view" => return Parsed::Prefix(Prefix::View(rest)),
                "edit" => return Parsed::Prefix(Prefix::Edit(rest)),
                "goto" => return Parsed::Prefix(Prefix::Goto(rest)),
                "whereis" => return Parsed::Prefix(Prefix::WhereIs(rest)),
                "run" => {
                    let (into, command) = match rest.split_once('<') {
                        Some((file, cmd)) => {
                            let file = file.trim();
                            (
                                (!file.is_empty()).then(|| file.to_string()),
                                cmd.trim().to_string(),
                            )
                        }
                        None => (None, rest),
                    };
                    return Parsed::Prefix(Prefix::Run { into, command });
                }
                _ => {}
            }
        }
    }

    let (word, rest) = split_first_word(trimmed);
    let rest = rest.trim();
    match word.to_ascii_lowercase().as_str() {
        "cd" => {
            return Parsed::Internal(Internal::Cd {
                path: rest.to_string(),
                local_only: false,
            })
        }
        "chdir" => {
            return Parsed::Internal(Internal::Cd {
                path: rest.to_string(),
                local_only: true,
            })
        }
        "cls" | "clear" if rest.is_empty() => return Parsed::Internal(Internal::Cls),
        "exit" | "quit" if rest.is_empty() => return Parsed::Internal(Internal::Exit),
        "pushd" => {
            return Parsed::Internal(Internal::PushD(
                (!rest.is_empty()).then(|| rest.to_string()),
            ))
        }
        "popd" if rest.is_empty() => return Parsed::Internal(Internal::PopD),
        "clrd" if rest.is_empty() => return Parsed::Internal(Internal::ClrD),
        "title" => {
            return Parsed::Internal(Internal::Title(
                (!rest.is_empty()).then(|| rest.to_string()),
            ))
        }
        "set" => {
            if rest.is_empty() {
                return Parsed::Internal(Internal::Set {
                    name: None,
                    value: None,
                });
            }
            let (name, value) = match rest.split_once('=') {
                Some((name, value)) => (name.trim().to_string(), Some(value.trim().to_string())),
                None => (rest.to_string(), None),
            };
            return Parsed::Internal(Internal::Set {
                name: Some(name),
                value,
            });
        }
        _ => {}
    }

    // On Windows a bare `d:` changes drive. Elsewhere it is a command like any other.
    if cfg!(windows) && trimmed.len() == 2 && trimmed.ends_with(':') {
        let drive = trimmed.chars().next().unwrap_or('c');
        if drive.is_ascii_alphabetic() {
            return Parsed::Internal(Internal::Cd {
                path: format!("{drive}:\\"),
                local_only: true,
            });
        }
    }

    Parsed::Shell(trimmed.to_string())
}

/// `cd ~`, `cd -`, `cd ..`, and an absolute or relative path, resolved against where we are.
/// Returns the path to go to, or `None` when the line asks for something we cannot answer
/// (`cd -` with no previous directory).
pub fn resolve_cd(
    argument: &str,
    cwd: &Path,
    home: Option<&Path>,
    previous: Option<&Path>,
) -> Option<PathBuf> {
    let argument = argument.trim();
    match argument {
        "" => home.map(Path::to_path_buf),
        "~" => home.map(Path::to_path_buf),
        "-" => previous.map(Path::to_path_buf),
        _ => {
            if let Some(rest) = argument.strip_prefix("~/").or(argument.strip_prefix("~\\")) {
                return home.map(|h| h.join(rest));
            }
            let path = Path::new(argument);
            if path.is_absolute() {
                Some(path.to_path_buf())
            } else {
                Some(cwd.join(path))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(text: &str) -> CmdLine {
        let mut l = CmdLine::new();
        l.set(text);
        l
    }

    #[test]
    fn typing_and_deleting_track_the_caret_in_characters() {
        let mut l = CmdLine::new();
        l.insert_str("città");
        assert_eq!(l.caret, 5, "five characters, not six bytes");
        l.left();
        l.backspace();
        assert_eq!(
            l.text, "cità",
            "Backspace takes the character before the caret"
        );
        assert_eq!(l.caret, 3);
        l.insert('t');
        assert_eq!(l.text, "città", "and typing puts one back, by characters");
        assert_eq!(l.caret, 4);
    }

    #[test]
    fn a_word_jump_stops_at_a_path_separator() {
        let mut l = line("cp /usr/local/bin/fcmd .");
        l.end();
        l.word_left(); // past "."
        l.word_left(); // to "fcmd"
        assert_eq!(
            l.text.chars().nth(l.caret),
            Some('f'),
            "caret at {}",
            l.caret
        );
        // Ctrl+Backspace eats the component *before* the caret, not the one it sits on.
        l.delete_word_left();
        assert_eq!(l.text, "cp /usr/local/fcmd .");
    }

    #[test]
    fn delete_word_right_eats_one_component() {
        let mut l = line("/usr/local/bin");
        l.home();
        l.delete_word_right();
        assert_eq!(l.text, "usr/local/bin");
    }

    #[test]
    fn kill_to_end_and_clear() {
        let mut l = line("make -j8 release");
        l.home();
        l.word_right();
        l.kill_to_end();
        assert_eq!(l.text, "make ");
        l.clear();
        assert!(l.is_empty());
        assert_eq!(l.caret, 0);
    }

    #[test]
    fn walking_the_history_keeps_what_was_being_typed() {
        let mut l = CmdLine::new();
        l.remember("make");
        l.remember("cargo test");
        l.insert_str("half typ");
        assert!(l.history_prev());
        assert_eq!(l.text, "cargo test");
        assert!(l.history_prev());
        assert_eq!(l.text, "make");
        assert!(!l.history_prev(), "the oldest entry is the end of the walk");
        l.history_next();
        assert_eq!(l.text, "cargo test");
        l.history_next();
        assert_eq!(l.text, "half typ", "coming back must be lossless");
        assert_eq!(l.caret, 8);
    }

    #[test]
    fn a_repeated_command_moves_to_the_end_instead_of_piling_up() {
        let mut l = CmdLine::new();
        l.remember("make");
        l.remember("cargo test");
        l.remember("make");
        assert_eq!(l.history(), ["cargo test", "make"]);
        l.remember("   ");
        assert_eq!(l.history().len(), 2, "blank lines are not history");
    }

    #[test]
    fn the_history_is_trimmed_from_the_oldest_end() {
        let mut l = CmdLine::new();
        for i in 0..10 {
            l.remember(&format!("command {i}"));
        }
        l.trim_history(3);
        assert_eq!(l.history(), ["command 7", "command 8", "command 9"]);
    }

    #[test]
    fn internal_commands_are_recognised_with_their_arguments() {
        assert_eq!(parse(""), Parsed::Empty);
        assert_eq!(parse("   "), Parsed::Empty);
        assert_eq!(
            parse("cd /tmp"),
            Parsed::Internal(Internal::Cd {
                path: "/tmp".into(),
                local_only: false
            })
        );
        assert_eq!(
            parse("CHDIR ~"),
            Parsed::Internal(Internal::Cd {
                path: "~".into(),
                local_only: true
            })
        );
        assert_eq!(parse("cls"), Parsed::Internal(Internal::Cls));
        assert_eq!(parse("exit"), Parsed::Internal(Internal::Exit));
        assert_eq!(
            parse("pushd /var/log"),
            Parsed::Internal(Internal::PushD(Some("/var/log".into())))
        );
        assert_eq!(parse("popd"), Parsed::Internal(Internal::PopD));
        assert_eq!(
            parse("set EDITOR=vim"),
            Parsed::Internal(Internal::Set {
                name: Some("EDITOR".into()),
                value: Some("vim".into())
            })
        );
        assert_eq!(
            parse("set PATH"),
            Parsed::Internal(Internal::Set {
                name: Some("PATH".into()),
                value: None
            })
        );
        assert_eq!(
            parse("set EDITOR="),
            Parsed::Internal(Internal::Set {
                name: Some("EDITOR".into()),
                value: Some(String::new())
            }),
            "an empty value removes the variable"
        );
    }

    #[test]
    fn a_command_that_looks_internal_but_has_arguments_goes_to_the_shell() {
        // `cls` takes nothing; `cls -x` is somebody's program.
        assert_eq!(parse("cls -x"), Parsed::Shell("cls -x".into()));
        assert_eq!(parse("exit 3"), Parsed::Shell("exit 3".into()));
    }

    #[test]
    fn prefixes_hand_the_line_to_the_program_itself() {
        assert_eq!(
            parse("view:/etc/hosts"),
            Parsed::Prefix(Prefix::View("/etc/hosts".into()))
        );
        assert_eq!(
            parse("edit: notes.md"),
            Parsed::Prefix(Prefix::Edit("notes.md".into()))
        );
        assert_eq!(
            parse("goto:/usr/lib"),
            Parsed::Prefix(Prefix::Goto("/usr/lib".into()))
        );
        assert_eq!(
            parse("run:out.txt<make -j8"),
            Parsed::Prefix(Prefix::Run {
                into: Some("out.txt".into()),
                command: "make -j8".into()
            })
        );
        assert_eq!(
            parse("run:make"),
            Parsed::Prefix(Prefix::Run {
                into: None,
                command: "make".into()
            })
        );
    }

    #[test]
    fn a_url_or_a_drive_is_not_a_prefix() {
        // One letter before the colon is a drive, not a prefix; and an unknown name is a
        // command, because `scp host:path .` must still work.
        assert!(matches!(parse("scp host:path ."), Parsed::Shell(_)));
        assert!(matches!(
            parse("curl https://example.com"),
            Parsed::Shell(_)
        ));
    }

    #[test]
    fn conditions_are_resolved_before_anything_runs() {
        let exists = |p: &str| p == "Cargo.toml";
        let defined = |n: &str| n == "HOME";

        assert_eq!(
            resolve_conditions("if exist Cargo.toml cargo build", &exists, &defined).as_deref(),
            Some("cargo build")
        );
        assert_eq!(
            resolve_conditions("if exist nope cargo build", &exists, &defined),
            None
        );
        assert_eq!(
            resolve_conditions("if not exist nope touch nope", &exists, &defined).as_deref(),
            Some("touch nope")
        );
        // Nested, which is the form that makes conditions worth having.
        assert_eq!(
            resolve_conditions(
                "if exist Cargo.toml if defined HOME if not exist nope make",
                &exists,
                &defined
            )
            .as_deref(),
            Some("make")
        );
        assert_eq!(
            resolve_conditions("if defined NOPE make", &exists, &defined),
            None
        );
        // A quoted path with a space in it.
        assert_eq!(
            resolve_conditions("if not exist \"my file\" echo gone", &exists, &defined).as_deref(),
            Some("echo gone")
        );
        // Something that is not one of our forms stays a shell line.
        assert_eq!(
            resolve_conditions("if [ -f x ]; then make; fi", &exists, &defined).as_deref(),
            Some("if [ -f x ]; then make; fi")
        );
        // A test with nothing after it does nothing rather than running an empty line.
        assert_eq!(
            resolve_conditions("if exist Cargo.toml", &exists, &defined),
            None
        );
    }

    #[test]
    fn cd_resolves_the_shapes_people_type() {
        let cwd = Path::new("/home/mb/project");
        let home = Path::new("/home/mb");
        let previous = Path::new("/tmp");
        assert_eq!(
            resolve_cd("~", cwd, Some(home), None).as_deref(),
            Some(home)
        );
        assert_eq!(resolve_cd("", cwd, Some(home), None).as_deref(), Some(home));
        assert_eq!(
            resolve_cd("-", cwd, Some(home), Some(previous)).as_deref(),
            Some(previous)
        );
        assert_eq!(
            resolve_cd("-", cwd, Some(home), None),
            None,
            "nowhere to go back to"
        );
        assert_eq!(
            resolve_cd("~/src", cwd, Some(home), None).unwrap(),
            PathBuf::from("/home/mb/src")
        );
        assert_eq!(
            resolve_cd("src", cwd, Some(home), None).unwrap(),
            PathBuf::from("/home/mb/project/src")
        );
        assert_eq!(
            resolve_cd("/etc", cwd, Some(home), None).unwrap(),
            PathBuf::from("/etc")
        );
    }
}
