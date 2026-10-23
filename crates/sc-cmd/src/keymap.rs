use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;

use crate::command::Command;
use crate::key::Key;

/// Where a binding applies. Lookup walks the scope stack and falls back to `Global`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Scope {
    Global,
    Panel,
    Viewer,
    Editor,
    Dialog,
}

impl Scope {
    pub fn as_str(self) -> &'static str {
        match self {
            Scope::Global => "global",
            Scope::Panel => "panel",
            Scope::Viewer => "viewer",
            Scope::Editor => "editor",
            Scope::Dialog => "dialog",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "global" => Some(Scope::Global),
            "panel" => Some(Scope::Panel),
            "viewer" => Some(Scope::Viewer),
            "editor" => Some(Scope::Editor),
            "dialog" => Some(Scope::Dialog),
            _ => None,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum KeymapError {
    #[error("{file}: {source}")]
    Toml {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{file}: invalid key `{key}`: {source}")]
    Key {
        file: String,
        key: String,
        #[source]
        source: crate::key::KeyParseError,
    },
    #[error("{file}: unknown command `{cmd}`")]
    UnknownCommand { file: String, cmd: String },
    #[error("{file}: unknown scope `{scope}`")]
    UnknownScope { file: String, scope: String },
    #[error("unknown built-in keymap profile `{0}` (available: far, mc)")]
    UnknownProfile(String),
    #[error("{file}: `{key}` in scope `{scope}` is bound twice ({first} and {second})")]
    Duplicate {
        file: String,
        key: String,
        scope: String,
        first: String,
        second: String,
    },
}

#[derive(Debug, Deserialize)]
struct KeymapFile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    inherit: Option<String>,
    #[serde(default)]
    bind: Vec<BindFile>,
    #[serde(default)]
    unbind: Vec<UnbindFile>,
}

#[derive(Debug, Deserialize)]
struct BindFile {
    keys: Vec<String>,
    #[serde(default)]
    scope: Option<String>,
    cmd: String,
}

#[derive(Debug, Deserialize)]
struct UnbindFile {
    keys: Vec<String>,
    #[serde(default)]
    scope: Option<String>,
}

/// A resolved set of bindings.
#[derive(Debug, Clone, Default)]
pub struct Keymap {
    name: String,
    binds: BTreeMap<(Scope, Key), Command>,
}

const FAR_PROFILE: &str = include_str!("../keymaps/far.toml");
const MC_PROFILE: &str = include_str!("../keymaps/mc.toml");

impl Keymap {
    /// Load one of the profiles compiled into the binary.
    pub fn builtin(profile: &str) -> Result<Self, KeymapError> {
        let (file, src) = match profile.trim().to_ascii_lowercase().as_str() {
            "far" | "default" => ("far.toml", FAR_PROFILE),
            "mc" => ("mc.toml", MC_PROFILE),
            other => return Err(KeymapError::UnknownProfile(other.to_string())),
        };
        Self::from_toml(file, src)
    }

    pub fn builtin_profiles() -> &'static [&'static str] {
        &["far", "mc"]
    }

    /// Parse a keymap, resolving `inherit` against the built-in profiles.
    pub fn from_toml(file: &str, src: &str) -> Result<Self, KeymapError> {
        let parsed: KeymapFile = toml::from_str(src).map_err(|source| KeymapError::Toml {
            file: file.to_string(),
            source,
        })?;

        let mut map = match &parsed.inherit {
            Some(base) if !base.is_empty() => Keymap::builtin(base)?,
            _ => Keymap::default(),
        };
        map.name = if parsed.name.is_empty() {
            file.trim_end_matches(".toml").to_string()
        } else {
            parsed.name.clone()
        };

        // Within one file a double binding is a mistake; overriding an inherited one is not.
        let mut seen: BTreeMap<(Scope, Key), Command> = BTreeMap::new();
        for b in &parsed.bind {
            let scope = parse_scope(file, b.scope.as_deref())?;
            let cmd = Command::parse(&b.cmd).ok_or_else(|| KeymapError::UnknownCommand {
                file: file.to_string(),
                cmd: b.cmd.clone(),
            })?;
            for k in &b.keys {
                let key = k.parse::<Key>().map_err(|source| KeymapError::Key {
                    file: file.to_string(),
                    key: k.clone(),
                    source,
                })?;
                if let Some(first) = seen.get(&(scope, key)) {
                    if *first != cmd {
                        return Err(KeymapError::Duplicate {
                            file: file.to_string(),
                            key: key.to_string(),
                            scope: scope.as_str().to_string(),
                            first: first.id(),
                            second: cmd.id(),
                        });
                    }
                }
                seen.insert((scope, key), cmd);
                map.binds.insert((scope, key), cmd);
            }
        }

        for u in &parsed.unbind {
            let scope = parse_scope(file, u.scope.as_deref())?;
            for k in &u.keys {
                let key = k.parse::<Key>().map_err(|source| KeymapError::Key {
                    file: file.to_string(),
                    key: k.clone(),
                    source,
                })?;
                map.binds.remove(&(scope, key));
            }
        }

        Ok(map)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn len(&self) -> usize {
        self.binds.len()
    }

    pub fn is_empty(&self) -> bool {
        self.binds.is_empty()
    }

    /// Resolve a key press: the innermost scope wins, `Global` is the fallback.
    pub fn resolve(&self, scopes: &[Scope], key: Key) -> Option<Command> {
        let normalized = key.normalized();
        for candidate in [key, normalized] {
            for scope in scopes {
                if let Some(cmd) = self.binds.get(&(*scope, candidate)) {
                    return Some(*cmd);
                }
            }
            if let Some(cmd) = self.binds.get(&(Scope::Global, candidate)) {
                return Some(*cmd);
            }
        }
        None
    }

    /// First key bound to a command, for the key bar and the help page.
    pub fn key_for(&self, cmd: Command) -> Option<Key> {
        self.binds
            .iter()
            .find(|(_, c)| **c == cmd)
            .map(|((_, k), _)| *k)
    }

    /// Every binding, sorted — used by the help dialog and `sc --check-keymap`.
    pub fn bindings(&self) -> impl Iterator<Item = (Scope, Key, Command)> + '_ {
        self.binds.iter().map(|((s, k), c)| (*s, *k, *c))
    }
}

fn parse_scope(file: &str, raw: Option<&str>) -> Result<Scope, KeymapError> {
    match raw {
        None => Ok(Scope::Panel),
        Some(s) => Scope::parse(s).ok_or_else(|| KeymapError::UnknownScope {
            file: file.to_string(),
            scope: s.to_string(),
        }),
    }
}

impl fmt::Display for Keymap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "keymap `{}` ({} bindings)", self.name, self.binds.len())?;
        for (scope, key, cmd) in self.bindings() {
            writeln!(
                f,
                "  {:<10} {:<16} {}",
                scope.as_str(),
                key.to_string(),
                cmd.id()
            )?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::{SortKey, ViewMode};

    #[test]
    fn both_builtin_profiles_load() {
        for p in Keymap::builtin_profiles() {
            let km = Keymap::builtin(p).unwrap_or_else(|e| panic!("{p}: {e}"));
            assert!(km.len() > 30, "{p} has only {} bindings", km.len());
        }
    }

    #[test]
    fn far_profile_has_the_classic_keys() {
        let km = Keymap::builtin("far").unwrap();
        let cases = [
            ("F10", Command::AppQuit),
            ("F5", Command::FileCopy),
            ("F7", Command::FileMkdir),
            ("Tab", Command::PanelFocusToggle),
            ("Ctrl+R", Command::PanelReread),
            ("Ctrl+U", Command::PanelSwap),
            ("Ctrl+H", Command::PanelHiddenToggle),
            ("Ctrl+F6", Command::PanelSortBy(SortKey::Size)),
            ("Ctrl+1", Command::PanelViewSet(ViewMode::Brief)),
            ("Ins", Command::SelToggle),
            ("Ctrl+PgUp", Command::NavParent),
        ];
        for (k, want) in cases {
            let key: Key = k.parse().unwrap();
            assert_eq!(
                km.resolve(&[Scope::Panel], key),
                Some(want),
                "{k} should be {}",
                want.id()
            );
        }
    }

    #[test]
    fn mc_profile_inherits_far_and_overrides() {
        let mc = Keymap::builtin("mc").unwrap();
        // inherited
        assert_eq!(
            mc.resolve(&[Scope::Panel], "F5".parse().unwrap()),
            Some(Command::FileCopy)
        );
        // mc-specific: Alt+. toggles hidden files
        assert_eq!(
            mc.resolve(&[Scope::Panel], "Alt+.".parse().unwrap()),
            Some(Command::PanelHiddenToggle)
        );
        // mc-specific: `\` deselects by mask
        assert_eq!(
            mc.resolve(&[Scope::Panel], "\\".parse().unwrap()),
            Some(Command::SelMaskRemove)
        );
    }

    #[test]
    fn scope_fallback_prefers_the_inner_scope() {
        let km = Keymap::from_toml(
            "t.toml",
            r#"
            name = "t"
            [[bind]]
            keys = ["F1"]
            scope = "global"
            cmd = "app.help"
            [[bind]]
            keys = ["F1"]
            scope = "viewer"
            cmd = "app.quit"
            "#,
        )
        .unwrap();
        let f1: Key = "F1".parse().unwrap();
        assert_eq!(km.resolve(&[Scope::Viewer], f1), Some(Command::AppQuit));
        assert_eq!(km.resolve(&[Scope::Panel], f1), Some(Command::AppHelp));
    }

    #[test]
    fn duplicate_binding_in_one_file_is_an_error() {
        let err = Keymap::from_toml(
            "dup.toml",
            r#"
            [[bind]]
            keys = ["F5"]
            cmd = "file.copy"
            [[bind]]
            keys = ["F5"]
            cmd = "file.move"
            "#,
        )
        .unwrap_err();
        assert!(matches!(err, KeymapError::Duplicate { .. }), "{err}");
    }

    #[test]
    fn unknown_command_names_the_file_and_the_command() {
        let err = Keymap::from_toml(
            "bad.toml",
            r#"
            [[bind]]
            keys = ["F5"]
            cmd = "file.teleport"
            "#,
        )
        .unwrap_err();
        assert_eq!(err.to_string(), "bad.toml: unknown command `file.teleport`");
    }

    #[test]
    fn unbind_removes_an_inherited_binding() {
        let km = Keymap::from_toml(
            "u.toml",
            r#"
            inherit = "far"
            [[unbind]]
            keys = ["F10"]
            scope = "global"
            "#,
        )
        .unwrap();
        // F10 is bound globally in the base profile, so unbinding it there removes it for
        // every scope; the panel lookup falls back to global and finds nothing.
        assert_eq!(km.resolve(&[Scope::Panel], "F10".parse().unwrap()), None);
        assert_eq!(
            km.resolve(&[Scope::Panel], "F5".parse().unwrap()),
            Some(Command::FileCopy)
        );
    }

    #[test]
    fn shifted_letters_resolve_to_the_lowercase_binding() {
        let km = Keymap::builtin("far").unwrap();
        let shouty = Key::new(crate::key::KeyCode::Char('R'), crate::key::Mods::CTRL);
        assert_eq!(
            km.resolve(&[Scope::Panel], shouty),
            Some(Command::PanelReread)
        );
    }

    #[test]
    fn terminal_impossible_keys_are_not_bound() {
        // Ctrl+I is Tab, Ctrl+M is Enter, Ctrl+J is a newline: see docs/03-keymap.md §9.
        for p in Keymap::builtin_profiles() {
            let km = Keymap::builtin(p).unwrap();
            for k in ["Ctrl+I", "Ctrl+M", "Ctrl+J"] {
                let key: Key = k.parse().unwrap();
                assert_eq!(
                    km.resolve(&[Scope::Panel, Scope::Global], key),
                    None,
                    "{p} binds {k}, which terminals cannot deliver"
                );
            }
        }
    }
}
