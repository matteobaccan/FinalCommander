//! Substitutions for the command line, file associations, the user menu and apply-command —
//! `docs/spec/06-tools.md` §5b.
//!
//! One table serves all four, in two spellings: ours, which is named (`{file}`, `{other.dir}`),
//! and FAR's, which is punctuation (`!.!`, `!\`). A migrating user's `FarMenu.ini` is written in
//! the second, so both are accepted and both mean exactly the same thing.
//!
//! Nothing here touches the filesystem or runs anything. Expansion can also *ask* — FAR's
//! `!?title?init!` prompts before the command runs — so the result says what it still needs
//! rather than making something up.

use std::path::{Path, PathBuf};

/// What the panels hold, at the moment a line is about to run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Context {
    pub active_dir: PathBuf,
    pub active_file: Option<String>,
    pub passive_dir: PathBuf,
    pub passive_file: Option<String>,
    pub left_dir: PathBuf,
    pub right_dir: PathBuf,
    /// The selected names on the active panel, or the item under the cursor when nothing is
    /// selected — which is what every commander means by "the selection".
    pub selection: Vec<String>,
}

/// Which panel the following symbols refer to. FAR's `!`, `!^`, `![`, `!]` prefixes switch it,
/// and they are toggles: the effect lasts to the next prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Active,
    Passive,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expanded {
    /// Ready to run.
    Line(String),
    /// A `{ask:…}` or `!?…?…!` needs an answer first. Once the caller has it, expansion runs
    /// again with the answer in place.
    Ask {
        title: String,
        initial: String,
        /// The named history to offer, if the symbol asked for one.
        history: Option<String>,
        /// The text with this one question replaced by `{answer}`, so the caller can put the
        /// answer in and expand again.
        pending: String,
    },
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExpandError {
    #[error("`{0}` needs a file under the cursor")]
    NoFile(&'static str),
    #[error("`{0}` needs something selected")]
    NoSelection(&'static str),
    /// A list substitution has to be written to a temporary file; the caller does that, so it
    /// says where. Expansion itself writes nothing.
    #[error("`{0}` needs a list file, which the caller must provide")]
    NeedsListFile(&'static str),
}

/// What a list symbol asked for, so the caller can write the file and expand again.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// `{list}` / `!@!`: a file holding one name per line.
    File,
    /// `{selection}` / `!&`: the names inline on the command line.
    Inline,
}

/// Quote a path the way the platform's shell needs it, and only when it needs it.
pub fn quote(text: &str) -> String {
    let needs = text.is_empty()
        || text
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '"' | '\'' | '$' | '`' | '\\' | '*' | '?'));
    if !needs {
        return text.to_string();
    }
    if cfg!(windows) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        format!("'{}'", text.replace('\'', r"'\''"))
    }
}

fn stem(name: &str) -> String {
    Path::new(name)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| name.to_string())
}

fn extension(name: &str) -> String {
    Path::new(name)
        .extension()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

impl Context {
    fn dir(&self, side: Side) -> &Path {
        match side {
            Side::Active => &self.active_dir,
            Side::Passive => &self.passive_dir,
            Side::Left => &self.left_dir,
            Side::Right => &self.right_dir,
        }
    }

    fn file(&self, side: Side) -> Option<&String> {
        match side {
            Side::Active | Side::Left | Side::Right => self.active_file.as_ref(),
            Side::Passive => self.passive_file.as_ref(),
        }
        // Left and Right carry the active panel's file unless the caller distinguishes them;
        // the panel the symbol names is the one whose *path* is taken, which is how FAR's
        // `![!\!^!.!` ("the left panel's folder plus the active file's name") works.
    }
}

/// Expand `text`. `list_file` is the path of a file holding the selection, when the caller has
/// already written one; without it, a `{list}` is an error that says so.
pub fn expand(
    text: &str,
    ctx: &Context,
    list_file: Option<&Path>,
) -> Result<Expanded, ExpandError> {
    let mut out = String::with_capacity(text.len());
    let mut side = Side::Active;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0usize;

    while i < chars.len() {
        let c = chars[i];

        // ── our own named form: {file}, {other.dir}, {ask:…} ────────────────────────────────
        if c == '{' {
            if chars.get(i + 1) == Some(&'{') {
                out.push('{');
                i += 2;
                continue;
            }
            if let Some(end) = chars[i..].iter().position(|c| *c == '}') {
                let name: String = chars[i + 1..i + end].iter().collect();
                i += end + 1;
                if let Some(rest) = name.strip_prefix("ask:") {
                    let (title, initial, history) = parse_ask(rest);
                    let pending =
                        format!("{out}{{answer}}{}", chars[i..].iter().collect::<String>());
                    return Ok(Expanded::Ask {
                        title,
                        initial,
                        history,
                        pending,
                    });
                }
                match named(&name, ctx, list_file) {
                    Some(value) => out.push_str(&value?),
                    // Not one of ours: put it back exactly as it was. An `awk '{print $1}'`
                    // on the command line is far more common than a mistyped substitution,
                    // and the shell gives a better error for the rest than we would.
                    None => {
                        out.push('{');
                        out.push_str(&name);
                        out.push('}');
                    }
                }
                continue;
            }
            // An unclosed brace is just a brace.
            out.push(c);
            i += 1;
            continue;
        }

        if c == '}' && chars.get(i + 1) == Some(&'}') {
            out.push('}');
            i += 2;
            continue;
        }

        // ── FAR's punctuation form ─────────────────────────────────────────────────────────
        if c == '!' {
            let next = chars.get(i + 1).copied();
            match next {
                Some('!') => {
                    // `!!` is a literal `!`, unless it is `!!.!` — the passive panel's file.
                    if chars.get(i + 2) == Some(&'.') && chars.get(i + 3) == Some(&'!') {
                        side = Side::Passive;
                        out.push_str(&quote(&full_file(ctx, side)?));
                        i += 4;
                        continue;
                    }
                    out.push('!');
                    i += 2;
                    continue;
                }
                Some('^') => {
                    side = Side::Active;
                    i += 2;
                    continue;
                }
                Some('[') => {
                    side = Side::Left;
                    i += 2;
                    continue;
                }
                Some(']') => {
                    side = Side::Right;
                    i += 2;
                    continue;
                }
                Some('.') if chars.get(i + 2) == Some(&'!') => {
                    out.push_str(&quote(&name_of(ctx, side)?));
                    i += 3;
                    continue;
                }
                Some('?') => {
                    // `!?title?initial!`, with an optional `$history$` in front of the title.
                    let rest: String = chars[i + 2..].iter().collect();
                    if let Some(end) = rest.find('!') {
                        let body = &rest[..end];
                        let (title, initial, history) = parse_far_ask(body);
                        let after: String = chars[i + 2 + end + 1..].iter().collect();
                        let pending = format!("{out}{{answer}}{after}");
                        return Ok(Expanded::Ask {
                            title,
                            initial,
                            history,
                            pending,
                        });
                    }
                    out.push('!');
                    i += 1;
                    continue;
                }
                Some('@') if chars.get(i + 2) == Some(&'!') => {
                    out.push_str(&list(ctx, list_file, ListKind::File, "!@!")?);
                    i += 3;
                    continue;
                }
                Some('&') => {
                    out.push_str(&list(ctx, list_file, ListKind::Inline, "!&")?);
                    i += 2;
                    continue;
                }
                Some('\\') => {
                    out.push_str(&quote(&ctx.dir(side).to_string_lossy()));
                    i += 2;
                    continue;
                }
                Some('`') => {
                    out.push_str(&extension(&name_of(ctx, side)?));
                    i += 2;
                    continue;
                }
                // A bare `!` followed by anything else: the name without its extension, which
                // is FAR's `!`.
                _ => {
                    out.push_str(&quote(&stem(&name_of(ctx, side)?)));
                    i += 1;
                    continue;
                }
            }
        }

        out.push(c);
        i += 1;
    }

    Ok(Expanded::Line(out))
}

/// `{ask:title=…,init=…,history=…}` — or just `{ask:Search for}`.
fn parse_ask(rest: &str) -> (String, String, Option<String>) {
    let mut title = String::new();
    let mut initial = String::new();
    let mut history = None;
    let mut any_key = false;
    for part in rest.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("title=") {
            title = v.to_string();
            any_key = true;
        } else if let Some(v) = part.strip_prefix("init=") {
            initial = v.to_string();
            any_key = true;
        } else if let Some(v) = part.strip_prefix("history=") {
            history = Some(v.to_string());
            any_key = true;
        }
    }
    if !any_key {
        title = rest.trim().to_string();
    }
    (title, initial, history)
}

/// FAR's `[$history$]title?initial`.
fn parse_far_ask(body: &str) -> (String, String, Option<String>) {
    let (history, rest) = if let Some(rest) = body.strip_prefix('$') {
        match rest.split_once('$') {
            Some((name, rest)) => (
                (!name.is_empty()).then(|| name.to_string()),
                rest.to_string(),
            ),
            None => (None, rest.to_string()),
        }
    } else {
        (None, body.to_string())
    };
    let (title, initial) = match rest.split_once('?') {
        Some((title, initial)) => (title.to_string(), initial.to_string()),
        None => (rest, String::new()),
    };
    (title, initial, history)
}

fn name_of(ctx: &Context, side: Side) -> Result<String, ExpandError> {
    ctx.file(side)
        .cloned()
        .ok_or(ExpandError::NoFile("the file under the cursor"))
}

fn full_file(ctx: &Context, side: Side) -> Result<String, ExpandError> {
    let name = name_of(ctx, side)?;
    Ok(ctx.dir(side).join(name).to_string_lossy().into_owned())
}

fn list(
    ctx: &Context,
    list_file: Option<&Path>,
    kind: ListKind,
    symbol: &'static str,
) -> Result<String, ExpandError> {
    if ctx.selection.is_empty() {
        return Err(ExpandError::NoSelection(symbol));
    }
    match kind {
        ListKind::Inline => Ok(ctx
            .selection
            .iter()
            .map(|n| quote(n))
            .collect::<Vec<_>>()
            .join(" ")),
        ListKind::File => match list_file {
            Some(path) => Ok(quote(&path.to_string_lossy())),
            None => Err(ExpandError::NeedsListFile(symbol)),
        },
    }
}

/// `None` means "not a substitution of ours", which the caller writes through unchanged.
fn named(
    name: &str,
    ctx: &Context,
    list_file: Option<&Path>,
) -> Option<Result<String, ExpandError>> {
    let (name, raw) = match name.strip_suffix(".raw") {
        Some(base) => (base, true),
        None => (name, false),
    };
    // These two quote each name themselves, so they never go through `quote` again.
    match name {
        "selection" => return Some(list(ctx, list_file, ListKind::Inline, "{selection}")),
        "list" => return Some(list(ctx, list_file, ListKind::File, "{list}")),
        _ => {}
    }
    let value: Result<String, ExpandError> = match name {
        "file" => full_file(ctx, Side::Active),
        "name" => name_of(ctx, Side::Active),
        "stem" => name_of(ctx, Side::Active).map(|n| stem(&n)),
        "ext" => name_of(ctx, Side::Active).map(|n| extension(&n)),
        "dir" => Ok(ctx.active_dir.to_string_lossy().into_owned()),
        "other.file" => full_file(ctx, Side::Passive),
        "other.name" => name_of(ctx, Side::Passive),
        "other.dir" => Ok(ctx.passive_dir.to_string_lossy().into_owned()),
        "left.dir" => Ok(ctx.left_dir.to_string_lossy().into_owned()),
        "right.dir" => Ok(ctx.right_dir.to_string_lossy().into_owned()),
        _ => return None,
    };
    Some(value.map(|v| if raw { v } else { quote(&v) }))
}

/// Does `text` ask for a list file? The caller writes one before expanding when it does.
pub fn wants_list_file(text: &str) -> bool {
    text.contains("{list}") || text.contains("!@!")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> Context {
        Context {
            active_dir: PathBuf::from("/home/mb/project"),
            active_file: Some("notes file.md".to_string()),
            passive_dir: PathBuf::from("/tmp"),
            passive_file: Some("other.txt".to_string()),
            left_dir: PathBuf::from("/home/mb/project"),
            right_dir: PathBuf::from("/tmp"),
            selection: vec!["a.rs".into(), "b rs.rs".into()],
        }
    }

    /// The platform's own quoting. An expectation that spells `'…'` only passes on Unix, and
    /// the thing worth asserting is *which* values get quoted, not which character does it.
    fn q(text: &str) -> String {
        if cfg!(windows) {
            format!("\"{text}\"")
        } else {
            format!("'{text}'")
        }
    }

    /// A path joined the way this platform joins one.
    fn p(dir: &str, name: &str) -> String {
        PathBuf::from(dir).join(name).to_string_lossy().into_owned()
    }

    fn line(text: &str) -> String {
        match expand(text, &ctx(), None).unwrap() {
            Expanded::Line(l) => l,
            other => panic!("expected a line, got {other:?}"),
        }
    }

    #[test]
    fn the_named_form_quotes_what_needs_quoting() {
        let file = q(&p("/home/mb/project", "notes file.md"));
        assert_eq!(line("wc -l {file}"), format!("wc -l {file}"));
        assert_eq!(line("echo {name}"), format!("echo {}", q("notes file.md")));
        assert_eq!(line("echo {stem}"), format!("echo {}", q("notes file")));
        assert_eq!(line("echo {ext}"), "echo md");
        assert_eq!(line("ls {dir}"), "ls /home/mb/project");
        assert_eq!(line("ls {other.dir}"), "ls /tmp");
        assert_eq!(
            line("diff {file} {other.file}"),
            format!("diff {file} {}", p("/tmp", "other.txt"))
        );
    }

    #[test]
    fn raw_asks_for_the_value_without_quotes() {
        assert_eq!(
            line("X={file.raw}"),
            format!("X={}", p("/home/mb/project", "notes file.md"))
        );
    }

    #[test]
    fn a_doubled_brace_is_a_brace() {
        assert_eq!(line("awk '{{print $1}}'"), "awk '{print $1}'");
    }

    #[test]
    fn fars_punctuation_means_the_same_things() {
        assert_eq!(line("wc -l !.!"), format!("wc -l {}", q("notes file.md")));
        assert_eq!(line("echo !"), format!("echo {}", q("notes file")));
        assert_eq!(line("echo !`"), "echo md");
        assert_eq!(line("ls !\\"), "ls /home/mb/project");
        assert_eq!(line("echo !!"), "echo !");
        // The passive panel's file.
        assert_eq!(
            line("diff !.! !!.!"),
            format!("diff {} {}", q("notes file.md"), p("/tmp", "other.txt"))
        );
    }

    #[test]
    fn a_side_prefix_lasts_until_the_next_one() {
        // `![` switches to the left panel's path, `!^` back to the active one.
        assert_eq!(
            line("ls ![!\\ !^!\\"),
            "ls /home/mb/project /home/mb/project"
        );
        assert_eq!(line("ls !]!\\"), "ls /tmp");
    }

    #[test]
    fn the_selection_comes_inline_or_as_a_file() {
        let selection = format!("a.rs {}", q("b rs.rs"));
        assert_eq!(line("grep -l x !&"), format!("grep -l x {selection}"));
        assert_eq!(
            line("grep -l x {selection}"),
            format!("grep -l x {selection}")
        );

        // A list file is the caller's to write, and until it exists the symbol says so.
        assert_eq!(
            expand("tar -T {list}", &ctx(), None),
            Err(ExpandError::NeedsListFile("{list}"))
        );
        assert!(wants_list_file("tar -T {list}"));
        assert!(wants_list_file("tar -T !@!"));
        assert!(!wants_list_file("tar -T {selection}"));

        let path = PathBuf::from("/tmp/scmd-list-1");
        assert_eq!(
            expand("tar -T {list}", &ctx(), Some(&path)).unwrap(),
            Expanded::Line("tar -T /tmp/scmd-list-1".into())
        );
    }

    #[test]
    fn a_symbol_with_nothing_behind_it_is_an_error_not_a_guess() {
        let mut c = ctx();
        c.active_file = None;
        assert!(matches!(
            expand("wc -l {file}", &c, None),
            Err(ExpandError::NoFile(_))
        ));
        c.selection.clear();
        assert!(matches!(
            expand("grep x {selection}", &c, None),
            Err(ExpandError::NoSelection(_))
        ));
        // A brace that is not one of ours is text, because a command line has to be able to
        // carry `awk '{print $1}'` and `jq '{a: .b}'`.
        assert_eq!(line("echo {nonsense}"), "echo {nonsense}");
        assert_eq!(line("awk '{print $1}'"), "awk '{print $1}'");
    }

    #[test]
    fn asking_stops_expansion_and_says_what_is_left_to_do() {
        let got = expand("grep {ask:title=Search for,init=TODO} {file}", &ctx(), None).unwrap();
        let Expanded::Ask {
            title,
            initial,
            history,
            pending,
        } = got
        else {
            panic!("expected a question")
        };
        assert_eq!(title, "Search for");
        assert_eq!(initial, "TODO");
        assert_eq!(history, None);
        assert_eq!(pending, "grep {answer} {file}");

        // FAR's spelling, with a named history.
        let got = expand("grep !?$GrepHist$Search for:?TODO! !.!", &ctx(), None).unwrap();
        let Expanded::Ask {
            title,
            initial,
            history,
            pending,
        } = got
        else {
            panic!("expected a question")
        };
        assert_eq!(title, "Search for:");
        assert_eq!(initial, "TODO");
        assert_eq!(history.as_deref(), Some("GrepHist"));
        assert_eq!(pending, "grep {answer} !.!");
    }

    #[test]
    fn the_answer_goes_back_in_and_the_line_finishes() {
        let Expanded::Ask { pending, .. } = expand("grep {ask:Search} !.!", &ctx(), None).unwrap()
        else {
            panic!()
        };
        let with_answer = pending.replace("{answer}", "TODO");
        assert_eq!(
            line(&with_answer),
            format!("grep TODO {}", q("notes file.md"))
        );
    }

    #[test]
    fn quoting_follows_the_platform() {
        assert_eq!(quote("plain"), "plain");
        if cfg!(windows) {
            assert_eq!(quote("two words"), "\"two words\"");
        } else {
            assert_eq!(quote("two words"), "'two words'");
            assert_eq!(quote("it's"), r"'it'\''s'");
        }
    }
}
