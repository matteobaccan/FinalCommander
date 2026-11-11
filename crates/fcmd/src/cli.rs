//! The command line: what `fcmd` accepts, what it says about it, and its manual page.
//!
//! One table, [`OPTS`], says everything about every option, and the parser, `--help`, `-h`,
//! the errors and the roff of `fcmd.1` are all written from it, so none of them can describe
//! an option the others do not know. The words and the layout are the ones the `clap` crate
//! printed when it did this job, kept so that nobody's scripts, habits or packaging notice the
//! change: a test compares them line by line with what `clap` wrote.

use std::ffi::OsString;
use std::path::PathBuf;

const NAME: &str = "fcmd";
const VERSION: &str = env!("CARGO_PKG_VERSION");

const ABOUT: &str = "FinalCommander — orthodox file manager for the terminal";

const LONG_ABOUT: &str = "FinalCommander is a two-panel file manager for the terminal that \
works the same way on Linux, macOS and Windows: one program, one set of keys, one \
configuration file.\n\n\
Two panels side by side, the function keys along the bottom saying what they do right now, \
and a command line underneath. Copying, moving and deleting run as jobs with progress and a \
way to stop them. Archives open as directories. The viewer streams a file of any size and \
disassembles a binary; the editor edits lines or bytes. SFTP panels are panels like any \
other.\n\n\
The keys are FAR's by default, with a Midnight Commander profile; every action is a named \
command and every command can be bound to a key. Press F1 inside the program for the manual, \
whose key and command chapters are generated from the keymap in force.";

/// What the run was asked for.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Cli {
    /// Directory for the left panel, and optionally for the right one.
    pub paths: Vec<PathBuf>,
    pub config: Option<PathBuf>,
    pub plugins: Option<PathBuf>,
    pub theme: Option<String>,
    pub keymap: Option<String>,
    pub lang: Option<String>,
    pub set: Vec<String>,
    pub view: Option<PathBuf>,
    pub edit: Option<PathBuf>,
    pub diff: Vec<PathBuf>,
    pub printwd: Option<PathBuf>,
    pub no_save: bool,
    pub shell_init: Option<String>,
    pub list_themes: bool,
    pub list_commands: bool,
    pub list_languages: bool,
    pub check_keymap: bool,
    pub export_theme: Option<String>,
    pub dump_config: bool,
    pub show_paths: bool,
    pub list_plugins: bool,
    pub man: bool,
}

/// One option: its names, the names of the values it takes (none for a switch), what it
/// does, and what it cannot be used with.
struct Opt {
    long: &'static str,
    short: Option<char>,
    values: &'static [&'static str],
    /// The text the help shows. Its first paragraph, without the full stop, is the summary
    /// `-h` and every list give; `--help` and the manual page give all of it.
    doc: &'static str,
    possible: &'static [&'static str],
    /// Whether it may be given more than once, each time adding to the list.
    repeat: bool,
    hidden: bool,
    conflicts: &'static [&'static str],
}

impl Opt {
    /// Whether its values name files: an empty one is no value at all.
    fn takes_paths(&self) -> bool {
        matches!(
            self.long,
            "config" | "plugins" | "view" | "edit" | "diff" | "printwd"
        )
    }
}

const fn opt(long: &'static str, values: &'static [&'static str], doc: &'static str) -> Opt {
    Opt {
        long,
        short: None,
        values,
        doc,
        possible: &[],
        repeat: false,
        hidden: false,
        conflicts: &[],
    }
}

const FILE: &[&str] = &["FILE"];

/// Every option, in the order the help lists them. `--help` and `--version` are the last two.
const OPTS: &[Opt] = &[
    opt(
        "config",
        FILE,
        "Use this configuration file instead of the default one.",
    ),
    opt(
        "plugins",
        &["DIR"],
        "Use this plugins directory (the only directory fcmd ever reads besides the config file).",
    ),
    opt(
        "theme",
        &["ID"],
        "Theme id (commander, far, black, white, matrix, tron, blade, github-dark, delphi-dark-2027, delphi-inspiration, or an external one).",
    ),
    opt(
        "keymap",
        &["NAME"],
        "Keymap profile: a built-in one (far, mc) or a [[keymap]] block from the config file.",
    ),
    opt(
        "lang",
        &["TAG"],
        "Interface language: en, it, de, fr, es, ru, zh, uk, be, pl, cs, sk, hu, lt, pt, or auto.",
    ),
    Opt {
        repeat: true,
        ..opt(
            "set",
            &["KEY=VALUE"],
            "Override a setting, e.g. --set panel.show_hidden=true",
        )
    },
    Opt {
        conflicts: &["edit", "diff"],
        ..opt(
            "view",
            FILE,
            "Open this file in the viewer; leaving the viewer ends the run.",
        )
    },
    Opt {
        conflicts: &["view", "diff"],
        ..opt(
            "edit",
            FILE,
            "Open this file in the editor; a name that is not there starts an empty file.",
        )
    },
    Opt {
        repeat: true,
        conflicts: &["view", "edit"],
        ..opt("diff", &["A", "B"], "Compare these two files side by side.")
    },
    Opt {
        short: Some('P'),
        ..opt(
            "printwd",
            FILE,
            "Write the directory the panel ended in to this file, for a shell wrapper to read.\n\n\
             A process cannot change its parent's working directory, so leaving the shell where \
             the panel was takes two halves: this, and a function in the shell. `--shell-init` \
             prints the function. Midnight Commander spells this `-P`, and so does this.",
        )
    },
    opt(
        "no-save",
        &[],
        "Read the histories and the panels' places, but write nothing back on the way out.\n\n\
         For a second copy opened for a quick job beside the main one: without it, whichever \
         quits last writes the state file, and the main one's histories lose to the side job's. \
         FAR's issue #1046.",
    ),
    Opt {
        possible: &["bash", "zsh", "fish", "powershell"],
        ..opt(
            "shell-init",
            &["SHELL"],
            "Print the shell function that makes the shell follow the panel, for this shell.\n\n\
             Add `eval \"$(fcmd --shell-init bash)\"` to the shell's startup file — one line, \
             and no file installed anywhere, which is the rule in SPEC 11 §1.",
        )
    },
    opt(
        "list-themes",
        &[],
        "List the themes found in this run and where each came from.",
    ),
    opt(
        "list-commands",
        &[],
        "List every command id (the vocabulary keys, menus and plugins bind to).",
    ),
    opt(
        "list-languages",
        &[],
        "List the interface languages compiled in.",
    ),
    opt(
        "check-keymap",
        &[],
        "Validate the keymap and print the resolved bindings.",
    ),
    opt(
        "export-theme",
        &["ID"],
        "Print a theme as TOML, to start an external theme from.",
    ),
    opt("dump-config", &[], "Print the effective configuration."),
    opt(
        "paths",
        &[],
        "Print where the config file, the state file and the plugins directory are.",
    ),
    opt(
        "list-plugins",
        &[],
        "List the plugins found in the plugins directory.",
    ),
    Opt {
        hidden: true,
        ..opt(
            "man",
            &[],
            "Print the manual page, in roff, for a packager to install as fcmd.1.\n\n\
             It is generated from these very options, so it cannot describe a flag that does \
             not exist or miss one that does. `docs/fcmd.1` in the repository is this output, \
             and a test fails if the two ever differ.",
        )
    },
    Opt {
        short: Some('h'),
        ..opt("help", &[], "Print help")
    },
    Opt {
        short: Some('V'),
        ..opt("version", &[], "Print version")
    },
];

const PATHS_DOC: &str = "Directory for the left panel, and optionally for the right one.";
const PATHS_NAME: &str = "[PATH]...";
const MAX_PATHS: usize = 2;

fn index(long: &str) -> Option<usize> {
    OPTS.iter().position(|o| o.long == long)
}

impl Opt {
    fn is(&self, long: &str) -> bool {
        self.long == long
    }

    /// `--theme <ID>`, `--diff <A> <B>`, `--paths`: how the option is named in a message.
    fn shown(&self) -> String {
        let mut s = format!("--{}", self.long);
        for v in self.values {
            s.push_str(&format!(" <{v}>"));
        }
        s
    }

    /// The first paragraph, without its full stop.
    fn summary(&self) -> String {
        if self.is("help") {
            return "Print help (see more with '--help')".to_string();
        }
        summary(self.doc)
    }

    /// All of it, when there is more than the summary.
    fn long_help(&self) -> String {
        if self.is("help") {
            return "Print help (see a summary with '-h')".to_string();
        }
        if self.doc.contains("\n\n") {
            self.doc.to_string()
        } else {
            summary(self.doc)
        }
    }
}

fn summary(doc: &str) -> String {
    let first = doc.split("\n\n").next().unwrap_or(doc);
    first.strip_suffix('.').unwrap_or(first).to_string()
}

/// What the command line comes to when it does not come to a run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Exit {
    /// `-h` (false) or `--help` (true): printed on standard output, status 0.
    Help { long: bool },
    /// `-V`, `--version`: printed on standard output, status 0.
    Version,
    /// A mistake: printed on standard error, status 2.
    Error(Mistake),
}

/// What was wrong with the command line, with what the message needs to say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mistake {
    Unknown {
        arg: String,
        similar: Option<usize>,
        usage: Usage,
    },
    MissingValue(usize),
    InvalidValue {
        opt: usize,
        value: String,
    },
    WrongNumber {
        opt: usize,
        got: usize,
    },
    TooManyValues {
        what: String,
        value: String,
        usage: Usage,
    },
    Twice(usize),
    Conflict {
        first: usize,
        second: usize,
        usage: Usage,
    },
}

/// The `Usage:` line of a message: the options already given, and whether a path was.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Usage {
    used: Vec<usize>,
    path: bool,
}

impl Usage {
    fn render(&self, out: &mut Styled) {
        out.push(Style::Header, "Usage:");
        out.text(" ");
        out.push(Style::Literal, NAME);
        out.text(" ");
        if self.used.iter().all(|&i| OPTS[i].hidden) && !self.path {
            out.text("[OPTIONS] ");
        }
        for &i in self.used.iter().filter(|&&i| !OPTS[i].hidden) {
            let o = &OPTS[i];
            out.push(Style::Literal, &format!("--{}", o.long));
            for v in o.values {
                out.text(&format!(" <{v}>"));
            }
            out.text(" ");
        }
        out.text(PATHS_NAME);
    }
}

impl Cli {
    /// The command line of this process.
    pub fn parse() -> Self {
        match Self::try_parse_from(std::env::args_os()) {
            Ok(cli) => cli,
            Err(exit) => {
                use std::io::Write;
                let (code, text) = match &exit {
                    Exit::Error(_) => (2, render(&exit, color(Stream::Err))),
                    _ => (0, render(&exit, color(Stream::Out))),
                };
                if code == 0 {
                    let mut out = std::io::stdout();
                    let _ = out.write_all(text.as_bytes());
                    let _ = out.flush();
                } else {
                    let mut err = std::io::stderr();
                    let _ = err.write_all(text.as_bytes());
                    let _ = err.flush();
                }
                std::process::exit(code);
            }
        }
    }

    /// Read `args`, the program's own name first, the way the shell hands them over.
    pub fn try_parse_from<I, T>(args: I) -> Result<Self, Exit>
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        let args: Vec<OsString> = args.into_iter().skip(1).map(Into::into).collect();
        Parser::default().run(&args)
    }

    /// Like [`Cli::try_parse_from`], for a test that knows its line is right.
    #[cfg(test)]
    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<OsString>,
    {
        Self::try_parse_from(args).unwrap_or_else(|e| panic!("{}", render(&e, false)))
    }
}

#[derive(Default)]
struct Parser {
    values: Vec<Vec<OsString>>,
    seen: Vec<bool>,
    used: Vec<usize>,
    paths: Vec<OsString>,
    /// How many paths in a row, since the last option.
    run: usize,
    /// An option still waiting for values, with the ones it has so far.
    pending: Option<(usize, Vec<OsString>)>,
}

fn flag_like(arg: &OsString) -> bool {
    let s = arg.to_string_lossy();
    s.starts_with('-') && s.len() > 1
}

/// A value that has to be one of a list and is not.
fn check_value(i: usize, value: &str) -> Result<(), Exit> {
    let o = &OPTS[i];
    let listed = !o.possible.is_empty() && !o.possible.contains(&value);
    if listed || (o.takes_paths() && value.is_empty()) {
        return Err(Exit::Error(Mistake::InvalidValue {
            opt: i,
            value: value.to_string(),
        }));
    }
    Ok(())
}

impl Parser {
    /// The arguments left to right. An option whose values are in the arguments after it is
    /// held until the next argument comes, and only then checked and taken — the order in
    /// which `clap` found mistakes, which decides which of two a message is about.
    fn run(mut self, args: &[OsString]) -> Result<Cli, Exit> {
        self.values = vec![Vec::new(); OPTS.len()];
        self.seen = vec![false; OPTS.len()];
        let mut trailing = false;
        for arg in args {
            let text = arg.to_string_lossy();
            if trailing || !flag_like(arg) {
                match &mut self.pending {
                    Some((i, got)) if !trailing && got.len() < OPTS[*i].values.len() => {
                        got.push(arg.clone());
                    }
                    _ => {
                        self.resolve()?;
                        self.paths.push(arg.clone());
                        self.run += 1;
                    }
                }
                continue;
            }
            if text == "--" {
                self.resolve()?;
                trailing = true;
                continue;
            }
            if let Some(long) = text.strip_prefix("--") {
                let (name, inline) = match long.split_once('=') {
                    Some((n, v)) => (n, Some(v.to_string())),
                    None => (long, None),
                };
                let Some(i) = index(name) else {
                    return Err(self.unknown(format!("--{name}"), Some(name)));
                };
                if OPTS[i].values.is_empty() {
                    if let Some(value) = inline {
                        return Err(Exit::Error(Mistake::TooManyValues {
                            what: format!("--{name}"),
                            value,
                            usage: Usage {
                                used: vec![i],
                                path: false,
                            },
                        }));
                    }
                }
                self.check_paths()?;
                self.resolve()?;
                self.option(i, inline, true)?;
                continue;
            }
            // A cluster of short options: `-hV`, or one that takes the rest as its value,
            // `-Pfile`, `-P=file`.
            let shorts: Vec<char> = text[1..].chars().collect();
            for (k, c) in shorts.iter().enumerate() {
                let Some(i) = OPTS.iter().position(|o| o.short == Some(*c)) else {
                    return Err(Exit::Error(Mistake::Unknown {
                        arg: format!("-{c}"),
                        similar: None,
                        usage: Usage::default(),
                    }));
                };
                self.check_paths()?;
                self.resolve()?;
                if OPTS[i].values.is_empty() {
                    self.option(i, None, false)?;
                    continue;
                }
                let rest: String = shorts[k + 1..].iter().collect();
                if rest == "=" {
                    if self.seen[i] && !OPTS[i].repeat {
                        return Err(Exit::Error(Mistake::Twice(i)));
                    }
                    return Err(Exit::Error(Mistake::MissingValue(i)));
                }
                let rest = rest.strip_prefix('=').unwrap_or(&rest).to_string();
                self.option(i, (!rest.is_empty()).then_some(rest), false)?;
                break;
            }
        }
        self.resolve()?;
        self.check_paths()?;
        self.check_conflicts()?;
        Ok(self.into_cli())
    }

    /// More paths in a row than the panels take, said when the row ends, naming its last:
    /// what `clap` did, which counted the paths between two options, not all of them.
    fn check_paths(&mut self) -> Result<(), Exit> {
        let run = std::mem::take(&mut self.run);
        if run > MAX_PATHS {
            let last = self.paths.last().expect("a row of paths");
            return Err(Exit::Error(Mistake::TooManyValues {
                what: PATHS_NAME.to_string(),
                value: last.to_string_lossy().into_owned(),
                usage: Usage::default(),
            }));
        }
        Ok(())
    }

    /// An option met: a switch is taken now, as is a value written into it (`--theme=x`);
    /// values in the arguments after it wait for [`Parser::resolve`].
    fn option(&mut self, i: usize, inline: Option<String>, long: bool) -> Result<(), Exit> {
        let o = &OPTS[i];
        if o.is("help") {
            return Err(Exit::Help { long });
        }
        if o.is("version") {
            return Err(Exit::Version);
        }
        match inline {
            _ if o.values.is_empty() => self.done(i, Vec::new()),
            Some(v) => {
                if self.seen[i] && !o.repeat {
                    return Err(Exit::Error(Mistake::Twice(i)));
                }
                check_value(i, &v)?;
                if o.values.len() > 1 {
                    // `--diff=a` is the one value it says, never the start of two.
                    return Err(Exit::Error(Mistake::WrongNumber { opt: i, got: 1 }));
                }
                self.done(i, vec![v.into()])
            }
            None => {
                self.pending = Some((i, Vec::new()));
                Ok(())
            }
        }
    }

    /// The option held for its values, once the next argument has come: short of them, a
    /// value that is not one of its list, or taken.
    fn resolve(&mut self) -> Result<(), Exit> {
        let Some((i, got)) = self.pending.take() else {
            return Ok(());
        };
        if got.is_empty() {
            return Err(Exit::Error(Mistake::MissingValue(i)));
        }
        if got.len() < OPTS[i].values.len() {
            return Err(Exit::Error(Mistake::WrongNumber {
                opt: i,
                got: got.len(),
            }));
        }
        if self.seen[i] && !OPTS[i].repeat {
            return Err(Exit::Error(Mistake::Twice(i)));
        }
        for v in &got {
            check_value(i, &v.to_string_lossy())?;
        }
        self.done(i, got)
    }

    /// An option with all its values: the second time is a mistake, unless it adds to a list.
    fn done(&mut self, i: usize, got: Vec<OsString>) -> Result<(), Exit> {
        if self.seen[i] && !OPTS[i].repeat {
            return Err(Exit::Error(Mistake::Twice(i)));
        }
        self.values[i].extend(got);
        self.seen[i] = true;
        if !self.used.contains(&i) {
            self.used.push(i);
        }
        Ok(())
    }

    fn unknown(&self, arg: String, long: Option<&str>) -> Exit {
        let similar = long.and_then(|name| {
            let names: Vec<&str> = OPTS.iter().map(|o| o.long).collect();
            similar(name, &names).and_then(index)
        });
        let mut used = self.used.clone();
        // An option held for the check of its values counts as given once it has them all —
        // unless it was given before and is about to be refused, which `clap` left out.
        if let Some((i, got)) = &self.pending {
            if got.len() == OPTS[*i].values.len() {
                if self.seen[*i] && !OPTS[*i].repeat {
                    used.retain(|u| u != i);
                } else if !used.contains(i) {
                    used.push(*i);
                }
            }
        }
        if let Some(s) = similar {
            if !used.contains(&s) {
                used.push(s);
            }
        }
        Exit::Error(Mistake::Unknown {
            arg,
            similar,
            usage: Usage {
                used,
                // Not while the paths in a row are more than there are panels.
                path: !self.paths.is_empty() && self.run <= MAX_PATHS,
            },
        })
    }

    fn check_conflicts(&self) -> Result<(), Exit> {
        for (n, &a) in self.used.iter().enumerate() {
            for &b in &self.used[n + 1..] {
                let clash = OPTS[a].conflicts.contains(&OPTS[b].long)
                    || OPTS[b].conflicts.contains(&OPTS[a].long);
                if clash {
                    return Err(Exit::Error(Mistake::Conflict {
                        first: a,
                        second: b,
                        usage: Usage {
                            used: self.used.iter().copied().filter(|&u| u != b).collect(),
                            path: !self.paths.is_empty(),
                        },
                    }));
                }
            }
        }
        Ok(())
    }

    fn into_cli(self) -> Cli {
        let one = |long: &str| -> Option<OsString> {
            index(long).and_then(|i| self.values[i].first().cloned())
        };
        let all = |long: &str| -> Vec<OsString> {
            index(long)
                .map(|i| self.values[i].clone())
                .unwrap_or_default()
        };
        let on = |long: &str| index(long).is_some_and(|i| self.seen[i]);
        let text = |v: OsString| v.to_string_lossy().into_owned();
        Cli {
            paths: self.paths.iter().map(PathBuf::from).collect(),
            config: one("config").map(PathBuf::from),
            plugins: one("plugins").map(PathBuf::from),
            theme: one("theme").map(text),
            keymap: one("keymap").map(text),
            lang: one("lang").map(text),
            set: all("set").into_iter().map(text).collect(),
            view: one("view").map(PathBuf::from),
            edit: one("edit").map(PathBuf::from),
            diff: all("diff").into_iter().map(PathBuf::from).collect(),
            printwd: one("printwd").map(PathBuf::from),
            no_save: on("no-save"),
            shell_init: one("shell-init").map(text),
            list_themes: on("list-themes"),
            list_commands: on("list-commands"),
            list_languages: on("list-languages"),
            check_keymap: on("check-keymap"),
            export_theme: one("export-theme").map(text),
            dump_config: on("dump-config"),
            show_paths: on("paths"),
            list_plugins: on("list-plugins"),
            man: on("man"),
        }
    }
}

/// The candidate most like `typed`, if one is alike enough: the Jaro similarity above 0.7,
/// the later of two that tie — the rule `clap` used for its "a similar argument exists".
fn similar<'a>(typed: &str, candidates: &[&'a str]) -> Option<&'a str> {
    let mut best: Option<(f64, &str)> = None;
    for c in candidates {
        let score = jaro(typed, c);
        if score > 0.7 && best.is_none_or(|(b, _)| score >= b) {
            best = Some((score, c));
        }
    }
    best.map(|(_, c)| c)
}

/// The Jaro similarity of two strings, 0 for nothing alike and 1 for the same.
fn jaro(a: &str, b: &str) -> f64 {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let range = (a.len().max(b.len()) / 2).saturating_sub(1);
    let mut a_hit = vec![false; a.len()];
    let mut b_hit = vec![false; b.len()];
    let mut matches = 0usize;
    for (i, ca) in a.iter().enumerate() {
        let lo = i.saturating_sub(range);
        let hi = (i + range + 1).min(b.len());
        for j in lo..hi {
            if !b_hit[j] && b[j] == *ca {
                a_hit[i] = true;
                b_hit[j] = true;
                matches += 1;
                break;
            }
        }
    }
    if matches == 0 {
        return 0.0;
    }
    let mut b_matched = b.iter().zip(&b_hit).filter(|(_, h)| **h).map(|(c, _)| c);
    let transpositions = a
        .iter()
        .zip(&a_hit)
        .filter(|(_, h)| **h)
        .filter(|(c, _)| b_matched.next() != Some(c))
        .count()
        / 2;
    let m = matches as f64;
    (m / a.len() as f64 + m / b.len() as f64 + (m - transpositions as f64) / m) / 3.0
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Style {
    Plain,
    /// `Usage:`, `Options:`: bold and underlined.
    Header,
    /// What is typed: `fcmd`, `--theme`. Bold.
    Literal,
    /// `error:`: bold red.
    Error,
    /// What is right: green.
    Valid,
    /// What is wrong: yellow.
    Invalid,
}

/// Text with the styles `clap` gave its parts, drawn in colour or plain.
struct Styled {
    color: bool,
    out: String,
}

impl Styled {
    fn new(color: bool) -> Self {
        Styled {
            color,
            out: String::new(),
        }
    }

    fn text(&mut self, s: &str) {
        self.out.push_str(s);
    }

    fn push(&mut self, style: Style, s: &str) {
        let on = match style {
            Style::Plain => "",
            Style::Header => "\x1b[1m\x1b[4m",
            Style::Literal => "\x1b[1m",
            Style::Error => "\x1b[1m\x1b[31m",
            Style::Valid => "\x1b[32m",
            Style::Invalid => "\x1b[33m",
        };
        if self.color && !on.is_empty() {
            self.out.push_str(on);
            self.out.push_str(s);
            self.out.push_str("\x1b[0m");
        } else {
            self.out.push_str(s);
        }
    }
}

enum Stream {
    Out,
    Err,
}

/// Whether to colour what goes to `stream`: a terminal does, a pipe and a file do not, and
/// `NO_COLOR`, `CLICOLOR=0`, `CLICOLOR_FORCE` and `TERM=dumb` say otherwise the usual way.
fn color(stream: Stream) -> bool {
    use std::io::IsTerminal;
    let var = |name: &str| std::env::var_os(name).filter(|v| !v.is_empty());
    if var("CLICOLOR_FORCE").is_some_and(|v| v != "0") {
        return true;
    }
    if var("NO_COLOR").is_some() || var("CLICOLOR").is_some_and(|v| v == "0") {
        return false;
    }
    if var("TERM").is_some_and(|v| v == "dumb") {
        return false;
    }
    // The old Windows console shows escape codes as text; Windows Terminal sets this.
    if cfg!(windows) && var("WT_SESSION").is_none() {
        return false;
    }
    match stream {
        Stream::Out => std::io::stdout().is_terminal(),
        Stream::Err => std::io::stderr().is_terminal(),
    }
}

/// What a run that ends on the command line prints.
pub fn render(exit: &Exit, color: bool) -> String {
    match exit {
        Exit::Help { long } => help(*long, color),
        Exit::Version => format!("{NAME} {VERSION}\n"),
        Exit::Error(m) => error(m, color),
    }
}

const TAB: &str = "  ";
const INDENT: &str = "          ";

/// `-h` (short) and `--help` (long).
fn help(long: bool, color: bool) -> String {
    let mut s = Styled::new(color);
    s.text(if long { LONG_ABOUT } else { ABOUT });
    s.text("\n\n");
    Usage::default().render(&mut s);
    s.text("\n\n");
    s.push(Style::Header, "Arguments:");
    s.text("\n");
    if long {
        s.text(TAB);
        s.text(PATHS_NAME);
        s.text("\n");
        s.text(INDENT);
        s.text(&summary(PATHS_DOC));
    } else {
        s.text(TAB);
        s.text(PATHS_NAME);
        s.text(TAB);
        s.text(&summary(PATHS_DOC));
    }
    s.text("\n\n");
    s.push(Style::Header, "Options:");
    s.text("\n");
    let shown: Vec<&Opt> = OPTS.iter().filter(|o| !o.hidden).collect();
    let left = |o: &Opt| -> (String, String) {
        let names = match o.short {
            Some(c) => format!("-{c}, --{}", o.long),
            None => format!("    --{}", o.long),
        };
        let values: String = o.values.iter().map(|v| format!(" <{v}>")).collect();
        (names, values)
    };
    let width = shown
        .iter()
        .map(|o| {
            let (n, v) = left(o);
            n.chars().count() + v.chars().count()
        })
        .max()
        .unwrap_or(0);
    for (k, o) in shown.iter().enumerate() {
        let (names, values) = left(o);
        s.text(TAB);
        // The names bold, the spaces in front of a long-only name not.
        let lead = names.len() - names.trim_start().len();
        s.text(&names[..lead]);
        match o.short {
            Some(c) => {
                s.push(Style::Literal, &format!("-{c}"));
                s.text(", ");
                s.push(Style::Literal, &format!("--{}", o.long));
            }
            None => s.push(Style::Literal, &format!("--{}", o.long)),
        }
        s.text(&values);
        let possible = (!o.possible.is_empty())
            .then(|| format!("[possible values: {}]", o.possible.join(", ")));
        if long {
            s.text("\n");
            let mut body = o.long_help();
            if let Some(p) = &possible {
                body.push_str("\n\n");
                body.push_str(p);
            }
            let lines: Vec<String> = body.lines().map(|l| format!("{INDENT}{l}")).collect();
            s.text(&lines.join("\n"));
            if k + 1 < shown.len() {
                s.text("\n\n");
            }
        } else {
            let used = names.chars().count() + values.chars().count();
            s.text(&" ".repeat(width - used));
            s.text(TAB);
            s.text(&o.summary());
            if let Some(p) = &possible {
                s.text(" ");
                s.text(p);
            }
            if k + 1 < shown.len() {
                s.text("\n");
            }
        }
    }
    s.text("\n");
    s.out
}

fn error(m: &Mistake, color: bool) -> String {
    let mut s = Styled::new(color);
    s.push(Style::Error, "error:");
    s.text(" ");
    let quoted = |s: &mut Styled, style: Style, text: &str| {
        s.text("'");
        s.push(style, text);
        s.text("'");
    };
    let possible = |s: &mut Styled, o: &Opt| {
        if !o.possible.is_empty() {
            s.text(&format!("\n{TAB}[possible values: "));
            for (n, v) in o.possible.iter().enumerate() {
                if n > 0 {
                    s.text(", ");
                }
                s.push(Style::Valid, v);
            }
            s.text("]");
        }
    };
    let tip = |s: &mut Styled, first: bool, parts: &[(Style, &str)]| {
        if first {
            s.text("\n");
        }
        s.text("\n");
        s.text(TAB);
        s.push(Style::Valid, "tip:");
        for (style, text) in parts {
            s.push(*style, text);
        }
    };
    let empty = Usage::default();
    let mut usage: Option<&Usage> = None;
    match m {
        Mistake::Unknown {
            arg,
            similar,
            usage: u,
        } => {
            s.text("unexpected argument ");
            quoted(&mut s, Style::Invalid, arg);
            s.text(" found");
            match similar {
                Some(i) => {
                    let name = format!("--{}", OPTS[*i].long);
                    tip(
                        &mut s,
                        true,
                        &[
                            (Style::Plain, " a similar argument exists: '"),
                            (Style::Valid, &name),
                            (Style::Plain, "'"),
                        ],
                    );
                }
                None => {
                    let pass = format!("-- {arg}");
                    tip(
                        &mut s,
                        true,
                        &[
                            (Style::Plain, " to pass '"),
                            (Style::Invalid, arg),
                            (Style::Plain, "' as a value, use '"),
                            (Style::Valid, &pass),
                            (Style::Plain, "'"),
                        ],
                    );
                }
            }
            usage = Some(u);
        }
        Mistake::MissingValue(i) => {
            s.text("a value is required for ");
            quoted(&mut s, Style::Invalid, &OPTS[*i].shown());
            s.text(" but none was supplied");
            possible(&mut s, &OPTS[*i]);
        }
        Mistake::InvalidValue { opt, value } => {
            let o = &OPTS[*opt];
            if value.is_empty() {
                s.text("a value is required for ");
                quoted(&mut s, Style::Invalid, &o.shown());
                s.text(" but none was supplied");
            } else {
                s.text("invalid value ");
                quoted(&mut s, Style::Invalid, value);
                s.text(" for ");
                quoted(&mut s, Style::Literal, &o.shown());
            }
            possible(&mut s, o);
            if let Some(like) = similar(value, o.possible) {
                tip(
                    &mut s,
                    true,
                    &[
                        (Style::Plain, " a similar value exists: '"),
                        (Style::Valid, like),
                        (Style::Plain, "'"),
                    ],
                );
            }
        }
        Mistake::WrongNumber { opt, got } => {
            let o = &OPTS[*opt];
            s.push(Style::Valid, &o.values.len().to_string());
            s.text(" values required for ");
            quoted(&mut s, Style::Literal, &o.shown());
            s.text(" but ");
            s.push(Style::Invalid, &got.to_string());
            s.text(if *got == 1 {
                " was provided"
            } else {
                " were provided"
            });
            usage = Some(&empty);
        }
        Mistake::TooManyValues {
            what,
            value,
            usage: u,
        } => {
            s.text("unexpected value ");
            quoted(&mut s, Style::Invalid, value);
            s.text(" for ");
            quoted(&mut s, Style::Literal, what);
            s.text(" found; no more were expected");
            usage = Some(u);
        }
        Mistake::Twice(i) => {
            s.text("the argument ");
            quoted(&mut s, Style::Invalid, &OPTS[*i].shown());
            s.text(" cannot be used multiple times");
            usage = Some(&empty);
        }
        Mistake::Conflict {
            first,
            second,
            usage: u,
        } => {
            s.text("the argument ");
            quoted(&mut s, Style::Invalid, &OPTS[*first].shown());
            s.text(" cannot be used with ");
            quoted(&mut s, Style::Invalid, &OPTS[*second].shown());
            usage = Some(u);
        }
    }
    if let Some(u) = usage {
        s.text("\n\n");
        u.render(&mut s);
    }
    s.text("\n\nFor more information, try ");
    quoted(&mut s, Style::Literal, "--help");
    s.text(".\n");
    s.out
}

/// The manual page's generated part, in roff: the name, the synopsis, the description, every
/// option and the version. What it does not know — the files, the keys — the caller adds.
pub fn man_page() -> String {
    let mut r = Roff::default();
    r.control(
        "TH",
        &[
            "FCMD",
            "1",
            "",
            &format!("{NAME} {VERSION}"),
            "FinalCommander",
        ],
    );
    r.control("SH", &["NAME"]);
    r.text(&[Inline::Roman(format!("{NAME} - {ABOUT}"))]);
    r.control("SH", &["SYNOPSIS"]);
    let mut line = vec![Inline::Bold(NAME.into()), Inline::Roman(" ".into())];
    for o in OPTS.iter().filter(|o| !o.hidden) {
        line.push(Inline::Roman("[".into()));
        if let Some(c) = o.short {
            line.push(Inline::Bold(format!("-{c}")));
            line.push(Inline::Roman("|".into()));
        }
        line.push(Inline::Bold(format!("--{}", o.long)));
        line.push(Inline::Roman("]".into()));
        line.push(Inline::Roman(" ".into()));
    }
    line.push(Inline::Roman("[".into()));
    line.push(Inline::Italic("PATH".into()));
    line.push(Inline::Roman("]".into()));
    line.push(Inline::Roman(" ".into()));
    r.text(&line);
    r.control("SH", &["DESCRIPTION"]);
    for l in LONG_ABOUT.lines() {
        if l.trim().is_empty() {
            r.control("PP", &[]);
        } else {
            r.text(&[Inline::Roman(l.into())]);
        }
    }
    r.control("SH", &["OPTIONS"]);
    for o in OPTS.iter().filter(|o| !o.hidden) {
        let mut header = Vec::new();
        if let Some(c) = o.short {
            header.push(Inline::Bold(format!("-{c}")));
            header.push(Inline::Roman(", ".into()));
        }
        header.push(Inline::Bold(format!("--{}", o.long)));
        if !o.values.is_empty() {
            header.push(Inline::Roman(" ".into()));
            for (n, v) in o.values.iter().enumerate() {
                if n > 0 {
                    header.push(Inline::Italic(" ".into()));
                }
                header.push(Inline::Italic(format!("<{v}>")));
            }
            header.push(Inline::Roman(String::new()));
        }
        r.control("TP", &[]);
        r.text(&header);
        r.text(&[Inline::Roman(o.long_help())]);
        if !o.possible.is_empty() {
            r.text(&[Inline::LineBreak]);
            r.text(&[Inline::LineBreak, Inline::Italic("Possible values:".into())]);
            r.control("RS", &["14"]);
            for v in o.possible {
                r.control("IP", &["\\(bu", "2"]);
                r.text(&[Inline::Roman((*v).into())]);
            }
            r.control("RE", &[]);
        }
    }
    r.control("TP", &[]);
    r.text(&[
        Inline::Roman("[".into()),
        Inline::Italic("PATH".into()),
        Inline::Roman("]".into()),
    ]);
    r.text(&[Inline::Roman(summary(PATHS_DOC))]);
    r.control("SH", &["VERSION"]);
    r.text(&[Inline::Roman(format!("v{VERSION}"))]);
    r.out
}

enum Inline {
    Roman(String),
    Italic(String),
    Bold(String),
    LineBreak,
}

/// Roff written the way man pages want it: a preamble that makes an apostrophe print as one,
/// dashes and backslashes escaped, and a line that would start with `.` or `'` kept from
/// being read as a request.
struct Roff {
    out: String,
}

impl Default for Roff {
    fn default() -> Self {
        Roff {
            out: ".ie \\n(.g .ds Aq \\(aq\n.el .ds Aq '\n".to_string(),
        }
    }
}

impl Roff {
    fn control(&mut self, name: &str, args: &[&str]) {
        self.out.push('.');
        self.out.push_str(name);
        for a in args {
            self.out.push(' ');
            if a.contains(' ') {
                self.out.push_str(&format!("\"{a}\""));
            } else {
                self.out.push_str(a);
            }
        }
        self.out.push('\n');
    }

    fn text(&mut self, parts: &[Inline]) {
        let mut start = true;
        for part in parts {
            match part {
                Inline::LineBreak => {
                    if start {
                        self.out.push_str(".br\n");
                    } else {
                        self.out.push_str("\n.br\n");
                    }
                }
                Inline::Roman(t) | Inline::Italic(t) | Inline::Bold(t) => {
                    let t = t
                        .replace('\\', "\\\\")
                        .replace('-', "\\-")
                        .replace('\'', "\\*(Aq")
                        .replace("\n.", "\n\\&.")
                        .replace("\n'", "\n\\&'");
                    match part {
                        Inline::Bold(_) => self.out.push_str(&format!("\\fB{t}\\fR")),
                        Inline::Italic(_) => self.out.push_str(&format!("\\fI{t}\\fR")),
                        _ => {
                            if start && (t.starts_with('.') || t.starts_with('\'')) {
                                self.out.push_str("\\&");
                            }
                            self.out.push_str(&t);
                        }
                    }
                }
            }
            start = false;
        }
        self.out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(line: &str) -> Result<Cli, Exit> {
        Cli::try_parse_from(std::iter::once("fcmd").chain(line.split_whitespace()))
    }

    fn says(line: &str) -> String {
        render(&run(line).expect_err("a stop"), false)
    }

    #[test]
    fn options_and_paths_are_read() {
        let cli = run("/tmp /home --theme matrix --set a=1 --set=b=2 -Pout --no-save --diff x y")
            .expect("a run");
        assert_eq!(cli.paths, [PathBuf::from("/tmp"), PathBuf::from("/home")]);
        assert_eq!(cli.theme.as_deref(), Some("matrix"));
        assert_eq!(cli.set, ["a=1", "b=2"]);
        assert_eq!(cli.printwd, Some(PathBuf::from("out")));
        assert!(cli.no_save);
        assert_eq!(cli.diff, [PathBuf::from("x"), PathBuf::from("y")]);
        assert_eq!(run("-- --paths").unwrap().paths, [PathBuf::from("--paths")]);
        assert_eq!(run("--view -").unwrap().view, Some(PathBuf::from("-")));
    }

    #[test]
    fn help_and_version_stop_where_they_are_met() {
        assert_eq!(
            run("--paths --help --bogus"),
            Err(Exit::Help { long: true })
        );
        assert_eq!(run("-hV"), Err(Exit::Help { long: false }));
        assert_eq!(run("-V"), Err(Exit::Version));
        assert_eq!(says("--version"), format!("fcmd {VERSION}\n"));
        assert!(matches!(run("--bogus --help"), Err(Exit::Error(_))));
    }

    #[test]
    fn a_mistake_is_said_the_way_it_always_was() {
        assert_eq!(
            says("--them x"),
            "error: unexpected argument '--them' found\n\n  tip: a similar argument exists: \
             '--theme'\n\nUsage: fcmd --theme <ID> [PATH]...\n\nFor more information, try \
             '--help'.\n"
        );
        assert_eq!(
            says("--shell-init tcsh"),
            "error: invalid value 'tcsh' for '--shell-init <SHELL>'\n  [possible values: bash, \
             zsh, fish, powershell]\n\n  tip: a similar value exists: 'zsh'\n\nFor more \
             information, try '--help'.\n"
        );
        assert_eq!(
            says("--paths --view a --edit b"),
            "error: the argument '--view <FILE>' cannot be used with '--edit <FILE>'\n\nUsage: \
             fcmd --paths --view <FILE> [PATH]...\n\nFor more information, try '--help'.\n"
        );
    }

    /// `-h` and `--help`, byte for byte as `clap` printed them when it was replaced.
    #[test]
    fn the_help_is_the_one_it_always_was() {
        assert_eq!(
            render(&Exit::Help { long: false }, false),
            include_str!("../tests/help-short.txt")
        );
        assert_eq!(
            render(&Exit::Help { long: true }, false),
            include_str!("../tests/help-long.txt")
        );
    }

    /// The parts of a message that a terminal colours, and nothing in a pipe.
    #[test]
    fn colour_marks_the_parts_and_only_when_asked() {
        let plain = says("--bogus");
        assert!(!plain.contains('\x1b'));
        let coloured = render(&run("--bogus").unwrap_err(), true);
        assert!(coloured.contains("\x1b[1m\x1b[31merror:\x1b[0m"));
        assert!(coloured.contains("'\x1b[33m--bogus\x1b[0m'"));
        assert!(coloured.contains("\x1b[1m\x1b[4mUsage:\x1b[0m"));
    }

    /// What `clap` counted the way it did, kept: two paths in a row at most, but a row
    /// broken by an option starts again; a value in the next argument is checked only when
    /// the argument after that comes.
    #[test]
    fn the_odd_rules_are_kept() {
        assert_eq!(run("a b --paths c d").unwrap().paths.len(), 4);
        assert!(says("a b c").contains("unexpected value 'c' for '[PATH]...'"));
        assert!(says("--shell-init tcsh --bogus").contains("unexpected argument '--bogus'"));
        assert!(says("--shell-init=tcsh --bogus").contains("invalid value 'tcsh'"));
        assert!(says("--edit=").contains("a value is required for '--edit <FILE>'"));
        assert_eq!(run("--theme=").unwrap().theme.as_deref(), Some(""));
        assert!(says("--diff=a b").contains("2 values required for '--diff <A> <B>'"));
    }

    #[test]
    fn jaro_is_the_usual_one() {
        assert_eq!(jaro("", ""), 1.0);
        assert_eq!(jaro("abc", ""), 0.0);
        assert!((jaro("martha", "marhta") - 0.944).abs() < 0.001);
        assert!((jaro("dixon", "dicksonx") - 0.767).abs() < 0.001);
    }
}
