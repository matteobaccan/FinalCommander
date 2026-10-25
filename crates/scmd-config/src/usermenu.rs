//! The user menu — `docs/spec/06-tools.md` §4.
//!
//! `F2` in every commander since Norton opens a menu the user wrote: a few lines they run ten
//! times a day, each on a letter, each with a label they chose. It is the one feature that
//! turns a file manager into *their* file manager, and it is nothing but a list of command
//! lines with labels on them.
//!
//! Three decisions are in this file.
//!
//! **A file of its own**, `scmd-menu.toml`, beside the configuration — unlike the associations,
//! which are `[[assoc]]` blocks inside `scmd.toml`. Two reasons: a menu is long where an
//! association is one line, and a *directory* can carry its own menu, which a block inside the
//! one global file cannot. The spec first called it `menu.toml`; the shipped name follows the
//! convention the other files already set (`scmd.toml`, `scmd-state.toml`, `scmd-macros.toml`),
//! and the local one is `.scmd-menu.toml` — hidden, because it sits in a project directory
//! somebody else may also be looking at.
//!
//! **A row is a command line**, the same one the command line itself takes: the substitutions
//! of SPEC 06 §5b (`{file}`, `{list}`, `{ask:…}`), the internal commands, the prefixes. So the
//! menu gains whatever the command line gains, and there is no second dialect to learn.
//!
//! **A condition is tiny on purpose.** `when = "panel.has('Cargo.toml')"` hides the build entry
//! where there is nothing to build, which is the whole of what FAR's conditions are used for in
//! practice. Four facts, `!`, `&&` and `||`, and anything else is refused by name rather than
//! guessed at — a condition that quietly evaluated to false would hide an entry and never say
//! why.

use std::path::Path;

use serde::Deserialize;

/// The menu everywhere, beside the configuration.
pub const MENU_FILE_NAME: &str = "scmd-menu.toml";
/// The menu of one directory, merged above the global one.
pub const LOCAL_MENU_FILE_NAME: &str = ".scmd-menu.toml";

/// One row of the menu: a label, the key that takes it, and either commands to run or a
/// submenu to open.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct Entry {
    /// The letter or digit that picks this row straight away. Optional: a row without one is
    /// still reachable with the arrows.
    #[serde(default)]
    pub key: Option<char>,
    pub label: String,
    /// When this row is shown at all. `None` is always.
    #[serde(default)]
    pub when: Option<String>,
    /// The command lines, run in order and stopping at the first failure.
    #[serde(default)]
    pub run: Vec<String>,
    /// A nested menu instead of commands.
    #[serde(default)]
    pub submenu: Vec<Entry>,
}

impl Entry {
    pub fn is_submenu(&self) -> bool {
        !self.submenu.is_empty()
    }
}

/// Every row, in the order the file lists them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct UserMenu {
    #[serde(default, rename = "menu")]
    pub entries: Vec<Entry>,
}

/// What the panel holds when the menu opens: everything a condition may ask about.
#[derive(Debug, Clone, Copy, Default)]
pub struct Facts<'a> {
    /// The names in the active panel's listing.
    pub names: &'a [String],
    /// The name under the cursor.
    pub current: Option<&'a str>,
    /// Is anything selected?
    pub selected: bool,
    /// Is the panel a directory on this machine? A command line runs in a shell, and a shell
    /// cannot be given a path inside an archive or on a server.
    pub local: bool,
}

impl UserMenu {
    /// Read a menu file. A file that is not there is an empty menu and not an error: the
    /// feature is invisible until somebody writes one, which is how it should be.
    pub fn load(path: &Path) -> Result<UserMenu, String> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::parse(&text).map_err(|e| format!("{}: {e}", path.display())),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(UserMenu::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }

    /// Parse a menu, and refuse one that cannot mean anything.
    pub fn parse(text: &str) -> Result<UserMenu, String> {
        let menu: UserMenu = toml::from_str(text).map_err(|e| e.message().to_string())?;
        check(&menu.entries)?;
        Ok(menu)
    }

    /// The local menu first, then the global one. A directory's own entries come above the
    /// ones that are everywhere — the way FAR and mc merge theirs — because the reason for
    /// writing a local menu is that this project's commands are the ones wanted here.
    pub fn merged(local: &UserMenu, global: &UserMenu) -> UserMenu {
        let mut entries = local.entries.clone();
        entries.extend(global.entries.iter().cloned());
        UserMenu { entries }
    }

    /// The rows to show, with the conditions applied. The error of a condition that cannot be
    /// understood is returned rather than swallowed, and the row is kept: an entry that
    /// vanished because of a typo in its own `when` would be impossible to debug.
    pub fn visible(entries: &[Entry], facts: &Facts) -> (Vec<Entry>, Vec<String>) {
        let mut rows = Vec::new();
        let mut problems = Vec::new();
        for entry in entries {
            match entry.when.as_deref() {
                None => rows.push(entry.clone()),
                Some(expr) => match holds(expr, facts) {
                    Ok(true) => rows.push(entry.clone()),
                    Ok(false) => {}
                    Err(e) => {
                        problems.push(format!("{}: {e}", entry.label));
                        rows.push(entry.clone());
                    }
                },
            }
        }
        (rows, problems)
    }
}

/// Every entry has to do something, and cannot do both things.
fn check(entries: &[Entry]) -> Result<(), String> {
    for entry in entries {
        if entry.label.trim().is_empty() {
            return Err("a menu entry has no label".to_string());
        }
        if entry.run.is_empty() && entry.submenu.is_empty() {
            return Err(format!(
                "menu entry `{}` has neither `run` nor `submenu`",
                entry.label
            ));
        }
        if !entry.run.is_empty() && !entry.submenu.is_empty() {
            return Err(format!(
                "menu entry `{}` has both `run` and `submenu`: it can only be one of the two",
                entry.label
            ));
        }
        check(&entry.submenu)?;
    }
    Ok(())
}

/// Does a condition hold? `||` of `&&` of terms, each optionally negated with `!`. No
/// parentheses: `&&` binds tighter, which is what everybody expects, and a condition that
/// needed brackets to be read is a condition that belongs in a script.
pub fn holds(expr: &str, facts: &Facts) -> Result<bool, String> {
    let mut any = false;
    for alternative in expr.split("||") {
        let mut all = true;
        let mut empty = true;
        for term in alternative.split("&&") {
            let term = term.trim();
            if term.is_empty() {
                continue;
            }
            empty = false;
            if !term_holds(term, facts)? {
                all = false;
            }
        }
        if empty {
            return Err(format!("`{}` is not a condition", expr.trim()));
        }
        any |= all;
    }
    Ok(any)
}

fn term_holds(term: &str, facts: &Facts) -> Result<bool, String> {
    match term.strip_prefix('!') {
        Some(rest) => Ok(!term_holds(rest.trim(), facts)?),
        None => match call(term) {
            Some(("panel.has", mask)) => Ok(facts
                .names
                .iter()
                .any(|name| crate::mask_matches(mask, name))),
            Some(("file.is", mask)) => Ok(facts
                .current
                .is_some_and(|name| crate::mask_matches(mask, name))),
            Some((name, _)) => Err(format!("`{name}` is not a condition this understands")),
            None => match term {
                "panel.selected" => Ok(facts.selected),
                "panel.local" => Ok(facts.local),
                other => Err(format!("`{other}` is not a condition this understands")),
            },
        },
    }
}

/// `name('argument')` split into the two, with either kind of quote.
fn call(term: &str) -> Option<(&str, &str)> {
    let (name, rest) = term.split_once('(')?;
    let inside = rest.strip_suffix(')')?.trim();
    let argument = inside
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .or_else(|| inside.strip_prefix('"').and_then(|s| s.strip_suffix('"')))
        .unwrap_or(inside);
    Some((name.trim(), argument))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
[[menu]]
key = "c"
label = "Build (cargo)"
when = "panel.has('Cargo.toml')"
run = ["cargo build --release"]

[[menu]]
key = "g"
label = "Git"
submenu = [
  { key = "s", label = "status", run = ["git status -sb"] },
  { key = "l", label = "log", run = ["git log --oneline -40"] },
]
"#;

    fn facts<'a>(names: &'a [String], current: Option<&'a str>) -> Facts<'a> {
        Facts {
            names,
            current,
            selected: false,
            local: true,
        }
    }

    #[test]
    fn a_menu_is_the_file_the_spec_shows() {
        let menu = UserMenu::parse(SAMPLE).expect("it parses");
        assert_eq!(menu.entries.len(), 2);
        let build = &menu.entries[0];
        assert_eq!(build.key, Some('c'));
        assert_eq!(build.label, "Build (cargo)");
        assert_eq!(build.run, ["cargo build --release"]);
        assert!(!build.is_submenu());
        let git = &menu.entries[1];
        assert!(git.is_submenu());
        assert_eq!(git.submenu.len(), 2);
        assert_eq!(git.submenu[0].label, "status");
    }

    #[test]
    fn a_row_that_does_nothing_is_refused_by_name() {
        let e = UserMenu::parse("[[menu]]\nlabel = \"Nothing\"\n").expect_err("refused");
        assert!(e.contains("Nothing"), "{e}");
        assert!(e.contains("neither"), "{e}");

        let both = UserMenu::parse(
            "[[menu]]\nlabel = \"Both\"\nrun = [\"ls\"]\nsubmenu = [{ label = \"x\", run = [\"ls\"] }]\n",
        )
        .expect_err("refused");
        assert!(both.contains("Both"), "{both}");

        // A broken row inside a submenu is found too, which is where one would miss it.
        let nested =
            UserMenu::parse("[[menu]]\nlabel = \"Git\"\nsubmenu = [{ label = \"nope\" }]\n")
                .expect_err("refused");
        assert!(nested.contains("nope"), "{nested}");
    }

    #[test]
    fn a_missing_file_is_an_empty_menu_and_not_a_failure() {
        let dir = std::env::temp_dir().join("scmd-usermenu-missing");
        let menu = UserMenu::load(&dir.join(MENU_FILE_NAME)).expect("no file is fine");
        assert!(menu.entries.is_empty());
    }

    #[test]
    fn the_condition_asks_about_what_the_panel_holds() {
        let names = vec!["Cargo.toml".to_string(), "src".to_string()];
        let f = facts(&names, Some("src"));
        assert_eq!(holds("panel.has('Cargo.toml')", &f), Ok(true));
        // Case does not matter, and the mask grammar is the one the associations use.
        assert_eq!(holds("panel.has('cargo.toml')", &f), Ok(true));
        assert_eq!(holds("panel.has('*.rs')", &f), Ok(false));
        assert_eq!(holds("panel.has('Makefile')", &f), Ok(false));
        assert_eq!(holds("file.is('src')", &f), Ok(true));
        assert_eq!(holds("file.is('*.rs')", &f), Ok(false));
        assert_eq!(holds("panel.local", &f), Ok(true));
        assert_eq!(holds("panel.selected", &f), Ok(false));
    }

    #[test]
    fn conditions_combine_with_not_and_and_or() {
        let names = vec!["Cargo.toml".to_string()];
        let f = facts(&names, None);
        assert_eq!(holds("!panel.selected", &f), Ok(true));
        assert_eq!(
            holds("panel.local && panel.has('Cargo.toml')", &f),
            Ok(true)
        );
        assert_eq!(holds("panel.selected && panel.local", &f), Ok(false));
        assert_eq!(holds("panel.selected || panel.local", &f), Ok(true));
        // `&&` binds tighter than `||`, so this is false-and-false, or true.
        assert_eq!(
            holds("panel.selected && panel.has('x') || panel.local", &f),
            Ok(true)
        );
        // A file.is with nothing under the cursor is false rather than an error.
        assert_eq!(holds("file.is('*.rs')", &f), Ok(false));
    }

    #[test]
    fn a_condition_nobody_wrote_on_purpose_is_refused_by_name() {
        let f = facts(&[], None);
        let e = holds("panel.hs('Cargo.toml')", &f).expect_err("refused");
        assert!(e.contains("panel.hs"), "{e}");
        assert!(holds("panel.selcted", &f).is_err());
        assert!(holds("", &f).is_err());
    }

    #[test]
    fn a_row_whose_condition_fails_is_left_out_and_a_broken_one_is_kept_with_the_reason() {
        let menu = UserMenu::parse(
            r#"
[[menu]]
label = "Build"
when = "panel.has('Cargo.toml')"
run = ["cargo build"]

[[menu]]
label = "Make"
when = "panel.has('Makefile')"
run = ["make"]

[[menu]]
label = "Broken"
when = "panel.whatever"
run = ["true"]
"#,
        )
        .expect("it parses");
        let names = vec!["Cargo.toml".to_string()];
        let (rows, problems) = UserMenu::visible(&menu.entries, &facts(&names, None));
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Build", "Broken"]);
        assert_eq!(problems.len(), 1);
        assert!(problems[0].contains("Broken"), "{:?}", problems);
    }

    #[test]
    fn the_directorys_own_menu_comes_above_the_one_that_is_everywhere() {
        let global = UserMenu::parse("[[menu]]\nlabel = \"Everywhere\"\nrun = [\"ls\"]\n").unwrap();
        let local = UserMenu::parse("[[menu]]\nlabel = \"Here\"\nrun = [\"ls\"]\n").unwrap();
        let merged = UserMenu::merged(&local, &global);
        let labels: Vec<&str> = merged.entries.iter().map(|e| e.label.as_str()).collect();
        assert_eq!(labels, ["Here", "Everywhere"]);
    }
}
