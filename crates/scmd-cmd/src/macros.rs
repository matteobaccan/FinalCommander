//! Keyboard macros (SPEC 08 §5).
//!
//! A macro is **a list of commands**, not a list of keystrokes. That distinction is the whole
//! design: every action in the program is a named command already, so a recorded macro is a
//! readable list of names in a TOML file rather than an opaque run of scan codes — one can
//! read it, edit it, move it to another machine, and later hand it to a plugin, because a
//! plugin speaks the same vocabulary.
//!
//! What a key *cannot* say as a command is kept as a key: the letters typed into a prompt, the
//! text typed into the editor, `Enter` on a dialog. A macro is therefore a list of steps, each
//! one a command or a key, and playing it is handing those back to the application in order.
//!
//! A macro carries the key it answers to, the way FAR's do: you record, then press the key you
//! want it on. Nothing else has to know a macro exists for it to work.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::key::Key;
use crate::keymap::Scope;

/// One step of a macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    /// An action with a name: what nearly every step is.
    Run(Command),
    /// A key that is not an action — a letter typed into a prompt, `Enter` on a dialog.
    Press(Key),
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Step::Run(cmd) => write!(f, "{}", cmd.id()),
            Step::Press(key) => write!(f, "key:{key}"),
        }
    }
}

impl Step {
    /// A step as `macros.toml` spells it: a command id, or `key:` and a key name.
    pub fn parse(text: &str) -> Option<Step> {
        let text = text.trim();
        match text.strip_prefix("key:") {
            Some(key) => key.trim().parse::<Key>().ok().map(Step::Press),
            None => Command::parse(text).map(Step::Run),
        }
    }
}

/// How a macro is replayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Play {
    /// Once, from where the cursor is. What a macro usually wants.
    #[default]
    Once,
    /// Once for every selected file, with the cursor put on each in turn before it runs.
    ///
    /// This is the mode that makes a macro worth recording: "do this to each of the files I
    /// picked" is the thing one does by hand twenty times otherwise.
    EachSelected,
}

impl Play {
    pub fn as_str(self) -> &'static str {
        match self {
            Play::Once => "once",
            Play::EachSelected => "each-selected",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "once" | "" => Some(Play::Once),
            "each-selected" | "each" | "selected" => Some(Play::EachSelected),
            _ => None,
        }
    }
}

/// A recorded macro.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Macro {
    /// The key that plays it.
    pub key: Key,
    /// Where that key plays it: a macro recorded in the panel does not fire in the editor.
    pub scope: Scope,
    pub steps: Vec<Step>,
    pub play: Play,
    /// What it is for, in the author's own words. The list shows this.
    pub description: String,
    /// `true` skips the question before a macro that deletes or overwrites something. It is
    /// off by default and has to be set in the file by hand: a recorded macro is never
    /// trusted by the act of recording it.
    pub trusted: bool,
}

impl Macro {
    /// Does this macro do something that cannot be undone? What decides whether playing it
    /// asks first.
    ///
    /// The test is on the commands, not on a list of names kept in step with them by hand:
    /// anything that writes, moves or removes counts.
    pub fn is_destructive(&self) -> bool {
        self.steps.iter().any(|step| match step {
            Step::Run(cmd) => {
                let id = cmd.id();
                id.starts_with("file.delete")
                    || id.starts_with("file.copy")
                    || id.starts_with("file.move")
                    || id.starts_with("file.pack")
                    || id.starts_with("file.unpack")
                    || id.starts_with("editor.save")
                    || id.starts_with("cmd.run")
                    || id.starts_with("tool.replace")
            }
            Step::Press(_) => false,
        })
    }

    /// A one-line summary for the list: the key, where it plays, and what it is for.
    pub fn summary(&self) -> String {
        let what = if self.description.is_empty() {
            format!("{} steps", self.steps.len())
        } else {
            self.description.clone()
        };
        format!(
            "{:<16} {:<8} {}",
            self.key.to_string(),
            self.scope.as_str(),
            what
        )
    }
}

/// Every macro, as loaded. Lookup is by key and scope, and the first match wins — so a macro
/// on `F5` in the panel shadows the built-in copy, which is deliberate: FAR allows it, and a
/// macro one cannot put on a useful key is a macro one does not record.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Macros {
    pub all: Vec<Macro>,
}

/// What `macros.toml` holds. A step is a string so the file stays readable and hand-editable.
#[derive(Debug, Default, Deserialize, Serialize)]
struct File {
    #[serde(default, rename = "macro")]
    macros: Vec<Entry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Entry {
    key: String,
    #[serde(default)]
    scope: Option<String>,
    steps: Vec<String>,
    #[serde(default)]
    play: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    trusted: Option<bool>,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MacroError {
    #[error("macros.toml: {0}")]
    Syntax(String),
    #[error("macros.toml: `{0}` is not a key")]
    Key(String),
    #[error("macros.toml: `{0}` is not a scope")]
    Scope(String),
    #[error("macros.toml: `{0}` is neither a command nor a key")]
    Step(String),
    #[error("macros.toml: `{0}` is not a way to play a macro")]
    Play(String),
}

impl Macros {
    /// Read the file. A macro that does not parse is an error rather than a warning: a macro
    /// that half runs is worse than one that does not run at all.
    pub fn parse(text: &str) -> Result<Macros, MacroError> {
        let file: File =
            toml::from_str(text).map_err(|e| MacroError::Syntax(e.message().to_string()))?;
        let mut all = Vec::with_capacity(file.macros.len());
        for entry in file.macros {
            let key = entry
                .key
                .parse::<Key>()
                .map_err(|_| MacroError::Key(entry.key.clone()))?;
            let scope = match entry.scope.as_deref() {
                None => Scope::Panel,
                Some(s) => Scope::parse(s).ok_or_else(|| MacroError::Scope(s.to_string()))?,
            };
            let mut steps = Vec::with_capacity(entry.steps.len());
            for raw in &entry.steps {
                steps.push(Step::parse(raw).ok_or_else(|| MacroError::Step(raw.clone()))?);
            }
            let play = match entry.play.as_deref() {
                None => Play::Once,
                Some(p) => Play::parse(p).ok_or_else(|| MacroError::Play(p.to_string()))?,
            };
            all.push(Macro {
                key,
                scope,
                steps,
                play,
                description: entry.description.unwrap_or_default(),
                trusted: entry.trusted.unwrap_or(false),
            });
        }
        Ok(Macros { all })
    }

    /// The file to write back. Generated rather than patched, so a recorded macro and a
    /// hand-written one come out the same shape.
    pub fn to_toml(&self) -> String {
        let mut out = String::from(
            "# Keyboard macros (SPEC 08 §5). A macro is a list of commands, which is why this\n\
             # file is readable: `scmd --list-commands` prints every name a step can hold, and\n\
             # `key:` in front of a key name is a keystroke that is not a command — a letter\n\
             # typed into a prompt, or Enter on a dialog.\n\
             #\n\
             # `trusted = true` lets a macro that deletes or overwrites play without asking\n\
             # first. Recording never sets it: that is a decision one makes in this file.\n",
        );
        for m in &self.all {
            out.push_str("\n[[macro]]\n");
            out.push_str(&format!("key = \"{}\"\n", m.key));
            out.push_str(&format!("scope = \"{}\"\n", m.scope.as_str()));
            if !m.description.is_empty() {
                out.push_str(&format!(
                    "description = \"{}\"\n",
                    m.description.replace('\\', "\\\\").replace('"', "\\\"")
                ));
            }
            if m.play != Play::Once {
                out.push_str(&format!("play = \"{}\"\n", m.play.as_str()));
            }
            if m.trusted {
                out.push_str("trusted = true\n");
            }
            let steps: Vec<String> = m.steps.iter().map(|s| format!("\"{s}\"")).collect();
            out.push_str(&format!("steps = [{}]\n", steps.join(", ")));
        }
        out
    }

    /// The macro a key plays in this scope, if any. `Global` macros play everywhere.
    pub fn find(&self, key: Key, scope: Scope) -> Option<&Macro> {
        self.all
            .iter()
            .find(|m| m.key == key && (m.scope == scope || m.scope == Scope::Global))
    }

    /// Add one, replacing whatever was on that key in that scope — recording over a macro is
    /// how one fixes a macro, and two macros on one key would mean the file decides which.
    pub fn set(&mut self, new: Macro) {
        self.all
            .retain(|m| !(m.key == new.key && m.scope == new.scope));
        self.all.push(new);
    }

    pub fn remove(&mut self, key: Key, scope: Scope) -> bool {
        let before = self.all.len();
        self.all.retain(|m| !(m.key == key && m.scope == scope));
        self.all.len() != before
    }

    pub fn is_empty(&self) -> bool {
        self.all.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_macro_file_round_trips_through_its_own_writer() {
        let text = r#"
[[macro]]
key = "Ctrl+Shift+D"
scope = "panel"
description = "the backups, gone"
trusted = true
steps = ["sel.mask.add", "key:*", "key:.", "key:b", "key:a", "key:k", "key:Enter", "file.delete"]

[[macro]]
key = "Alt+1"
play = "each-selected"
steps = ["file.view"]

[[macro]]
key = "Alt+2"
steps = ["file.view"]
"#;
        let macros = Macros::parse(text).expect("this file is sound");
        assert_eq!(macros.all.len(), 3);

        let first = &macros.all[0];
        assert_eq!(first.scope, Scope::Panel);
        assert_eq!(first.steps.len(), 8);
        assert_eq!(first.steps[6], Step::Press("Enter".parse::<Key>().unwrap()));
        assert!(first.trusted);
        assert!(first.is_destructive());

        // A scope that is not given is the panel. The play mode is read when it is given,
        // and is `once` when it is not.
        let second = &macros.all[1];
        assert_eq!(second.scope, Scope::Panel);
        assert_eq!(second.play, Play::EachSelected);
        assert_eq!(macros.all[2].play, Play::Once);
        assert!(!second.trusted);
        assert!(!second.is_destructive(), "viewing deletes nothing");

        // Written out and read back, it is the same set.
        let again = Macros::parse(&macros.to_toml()).expect("what we write, we can read");
        assert_eq!(again, macros);
    }

    #[test]
    fn a_macro_that_does_not_parse_is_refused_by_name() {
        let bad = |text: &str| Macros::parse(text).unwrap_err();
        assert!(matches!(
            bad("[[macro]]\nkey = \"Ctrl+Shift+Nope\"\nsteps = []\n"),
            MacroError::Key(_)
        ));
        assert!(matches!(
            bad("[[macro]]\nkey = \"F5\"\nscope = \"everywhere\"\nsteps = []\n"),
            MacroError::Scope(_)
        ));
        assert!(matches!(
            bad("[[macro]]\nkey = \"F5\"\nsteps = [\"file.teleport\"]\n"),
            MacroError::Step(_)
        ));
        assert!(matches!(
            bad("[[macro]]\nkey = \"F5\"\nplay = \"twice\"\nsteps = []\n"),
            MacroError::Play(_)
        ));
        assert!(matches!(bad("this is not toml"), MacroError::Syntax(_)));
    }

    #[test]
    fn a_key_finds_its_macro_in_its_own_scope_and_a_global_one_anywhere() {
        let macros = Macros::parse(
            "[[macro]]\nkey = \"F5\"\nscope = \"panel\"\nsteps = [\"file.view\"]\n\
             [[macro]]\nkey = \"F6\"\nscope = \"global\"\nsteps = [\"app.help\"]\n",
        )
        .unwrap();
        let f5 = "F5".parse::<Key>().unwrap();
        let f6 = "F6".parse::<Key>().unwrap();
        assert!(macros.find(f5, Scope::Panel).is_some());
        assert!(macros.find(f5, Scope::Editor).is_none(), "panel only");
        assert!(
            macros.find(f6, Scope::Editor).is_some(),
            "global plays anywhere"
        );
    }

    #[test]
    fn recording_over_a_macro_replaces_it_rather_than_adding_a_second() {
        let key = "Ctrl+Shift+X".parse::<Key>().unwrap();
        let one = Macro {
            key,
            scope: Scope::Panel,
            steps: vec![Step::Run(Command::parse("file.view").unwrap())],
            play: Play::Once,
            description: String::new(),
            trusted: false,
        };
        let mut macros = Macros::default();
        macros.set(one.clone());
        macros.set(Macro {
            steps: vec![Step::Run(Command::parse("app.help").unwrap())],
            ..one.clone()
        });
        assert_eq!(macros.all.len(), 1);
        assert_eq!(macros.all[0].steps.len(), 1);
        assert_eq!(
            macros.all[0].steps[0],
            Step::Run(Command::parse("app.help").unwrap())
        );
        // The same key in another scope is a different macro.
        macros.set(Macro {
            scope: Scope::Editor,
            ..one
        });
        assert_eq!(macros.all.len(), 2);
        assert!(macros.remove(key, Scope::Editor));
        assert!(!macros.remove(key, Scope::Viewer));
        assert_eq!(macros.all.len(), 1);
    }
}
