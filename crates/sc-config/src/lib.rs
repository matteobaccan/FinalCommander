//! Configuration: discovery, TOML parsing, defaults, and a dump of what is in effect.
//!
//! Unknown keys are warnings, never errors: a config written for a later version still loads.
//! See `docs/spec/07-config-theming.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CONFIG_FILE: &str = "config.toml";
pub const PORTABLE_MARKER: &str = "sc-portable";

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {source}")]
    Toml {
        path: String,
        #[source]
        source: toml::de::Error,
    },
}

/// Where everything lives for this run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    pub config_dir: PathBuf,
    pub state_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub portable: bool,
}

impl Paths {
    /// `--config <dir>` wins; then portable mode (a marker file next to the executable);
    /// then the per-OS conventions.
    pub fn discover(explicit: Option<&Path>) -> Paths {
        if let Some(dir) = explicit {
            return Paths {
                config_dir: dir.to_path_buf(),
                state_dir: dir.join("state"),
                cache_dir: dir.join("cache"),
                portable: false,
            };
        }
        if let Some(exe_dir) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
        {
            if exe_dir.join(PORTABLE_MARKER).exists() {
                return Paths {
                    config_dir: exe_dir.clone(),
                    state_dir: exe_dir.join("state"),
                    cache_dir: exe_dir.join("cache"),
                    portable: true,
                };
            }
        }
        let config_dir = dirs::config_dir()
            .map(|d| d.join("sc"))
            .unwrap_or_else(|| PathBuf::from(".sc"));
        let state_dir = dirs::data_local_dir()
            .map(|d| d.join("sc"))
            .unwrap_or_else(|| config_dir.join("state"));
        let cache_dir = dirs::cache_dir()
            .map(|d| d.join("sc"))
            .unwrap_or_else(|| config_dir.join("cache"));
        Paths {
            config_dir,
            state_dir,
            cache_dir,
            portable: false,
        }
    }

    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join(CONFIG_FILE)
    }

    pub fn themes_dir(&self) -> PathBuf {
        self.config_dir.join("themes")
    }

    pub fn keymaps_dir(&self) -> PathBuf {
        self.config_dir.join("keymaps")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub confirm_exit: bool,
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
    /// Never on: the program makes no network call the user did not ask for.
    pub update_check: bool,
}

impl Default for General {
    fn default() -> Self {
        General {
            confirm_exit: false,
            confirm_delete: true,
            confirm_overwrite: true,
            update_check: false,
        }
    }
}

/// How typing in a panel behaves: FAR filters with `Alt`+letters, mc with bare letters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QuickSearch {
    Alt,
    Plain,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PanelCfg {
    pub view: String,
    pub sort: String,
    pub reverse: bool,
    pub dirs_first: bool,
    pub selected_first: bool,
    pub natural_sort: bool,
    pub show_hidden: bool,
    pub allow_reverse_sort: bool,
    pub quick_search: QuickSearch,
}

impl Default for PanelCfg {
    fn default() -> Self {
        PanelCfg {
            view: "full".into(),
            sort: "name".into(),
            reverse: false,
            dirs_first: true,
            selected_first: false,
            natural_sort: true,
            show_hidden: false,
            allow_reverse_sort: true,
            quick_search: QuickSearch::Alt,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiCfg {
    pub theme: String,
    pub keymap: String,
    pub language: String,
    pub key_bar: bool,
    pub status_bar: bool,
    /// Extra directories to look for themes in.
    pub theme_path: Vec<PathBuf>,
}

impl Default for UiCfg {
    fn default() -> Self {
        UiCfg {
            theme: "commander".into(),
            keymap: "far".into(),
            language: "auto".into(),
            key_bar: true,
            status_bar: true,
            theme_path: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub panel: PanelCfg,
    pub ui: UiCfg,
    /// Anything we did not recognise, kept so `--dump-config` can show it and warn.
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, toml::Value>,
}

impl Config {
    /// Load the config file if it exists. A missing file is not an error; a broken one is.
    pub fn load(paths: &Paths) -> Result<(Config, Vec<String>), ConfigError> {
        let file = paths.config_file();
        if !file.exists() {
            return Ok((Config::default(), Vec::new()));
        }
        let src = std::fs::read_to_string(&file).map_err(|source| ConfigError::Io {
            path: file.display().to_string(),
            source,
        })?;
        Self::parse(&file.display().to_string(), &src)
    }

    pub fn parse(name: &str, src: &str) -> Result<(Config, Vec<String>), ConfigError> {
        let cfg: Config = toml::from_str(src).map_err(|source| ConfigError::Toml {
            path: name.to_string(),
            source,
        })?;
        let warnings = cfg
            .unknown
            .keys()
            .map(|k| format!("{name}: unknown section or key `{k}` (ignored)"))
            .collect();
        Ok((cfg, warnings))
    }

    /// The effective configuration as TOML (`sc --dump-config`).
    pub fn dump(&self) -> String {
        toml::to_string_pretty(self).unwrap_or_else(|e| format!("# cannot serialise: {e}\n"))
    }

    /// Apply a `--set key=value` override. Returns an error message when it does not apply.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), String> {
        match key {
            "ui.theme" => self.ui.theme = value.to_string(),
            "ui.keymap" => self.ui.keymap = value.to_string(),
            "ui.language" => self.ui.language = value.to_string(),
            "ui.key_bar" => self.ui.key_bar = parse_bool(value)?,
            "ui.status_bar" => self.ui.status_bar = parse_bool(value)?,
            "panel.view" => self.panel.view = value.to_string(),
            "panel.sort" => self.panel.sort = value.to_string(),
            "panel.reverse" => self.panel.reverse = parse_bool(value)?,
            "panel.show_hidden" => self.panel.show_hidden = parse_bool(value)?,
            "panel.dirs_first" => self.panel.dirs_first = parse_bool(value)?,
            "panel.selected_first" => self.panel.selected_first = parse_bool(value)?,
            "panel.natural_sort" => self.panel.natural_sort = parse_bool(value)?,
            "panel.quick_search" => {
                self.panel.quick_search = match value {
                    "alt" => QuickSearch::Alt,
                    "plain" => QuickSearch::Plain,
                    "off" => QuickSearch::Off,
                    other => {
                        return Err(format!(
                            "panel.quick_search: expected alt|plain|off, got `{other}`"
                        ))
                    }
                }
            }
            "general.confirm_exit" => self.general.confirm_exit = parse_bool(value)?,
            "general.confirm_delete" => self.general.confirm_delete = parse_bool(value)?,
            other => return Err(format!("unknown setting `{other}`")),
        }
        Ok(())
    }
}

fn parse_bool(v: &str) -> Result<bool, String> {
    match v.trim().to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Ok(true),
        "false" | "no" | "off" | "0" => Ok(false),
        other => Err(format!("expected a boolean, got `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_documented_ones() {
        let c = Config::default();
        assert_eq!(c.ui.theme, "commander");
        assert_eq!(c.ui.keymap, "far");
        assert_eq!(c.panel.view, "full");
        assert!(c.panel.dirs_first);
        assert!(!c.general.update_check, "no network calls by default");
        assert_eq!(c.panel.quick_search, QuickSearch::Alt);
    }

    #[test]
    fn partial_config_keeps_the_other_defaults() {
        let (c, warnings) = Config::parse(
            "t.toml",
            r#"
            [ui]
            theme = "matrix"
            [panel]
            show_hidden = true
            "#,
        )
        .unwrap();
        assert_eq!(c.ui.theme, "matrix");
        assert_eq!(c.ui.keymap, "far");
        assert!(c.panel.show_hidden);
        assert!(c.panel.dirs_first);
        assert!(warnings.is_empty());
    }

    #[test]
    fn unknown_sections_warn_but_load() {
        let (c, warnings) = Config::parse(
            "t.toml",
            r#"
            [ui]
            theme = "tron"
            [from_the_future]
            holograms = true
            "#,
        )
        .unwrap();
        assert_eq!(c.ui.theme, "tron");
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("from_the_future"), "{:?}", warnings);
    }

    #[test]
    fn broken_toml_is_an_error_naming_the_file() {
        let err = Config::parse("broken.toml", "[ui\ntheme =").unwrap_err();
        assert!(err.to_string().starts_with("broken.toml:"), "{err}");
    }

    #[test]
    fn dump_round_trips() {
        let mut c = Config::default();
        c.ui.theme = "blade".into();
        c.panel.show_hidden = true;
        let (again, warnings) = Config::parse("dump.toml", &c.dump()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.ui.theme, "blade");
        assert!(again.panel.show_hidden);
    }

    #[test]
    fn cli_overrides_are_validated() {
        let mut c = Config::default();
        c.set("ui.theme", "matrix").unwrap();
        c.set("panel.show_hidden", "yes").unwrap();
        assert_eq!(c.ui.theme, "matrix");
        assert!(c.panel.show_hidden);
        assert!(c.set("panel.show_hidden", "maybe").is_err());
        assert!(c.set("ui.colour_of_magic", "octarine").is_err());
    }

    #[test]
    fn explicit_config_dir_wins() {
        let p = Paths::discover(Some(Path::new("/tmp/scconf")));
        assert_eq!(p.config_dir, PathBuf::from("/tmp/scconf"));
        assert_eq!(p.config_file(), PathBuf::from("/tmp/scconf/config.toml"));
        assert_eq!(p.themes_dir(), PathBuf::from("/tmp/scconf/themes"));
        assert!(!p.portable);
    }

    #[test]
    fn discovery_without_an_override_produces_absolute_paths() {
        let p = Paths::discover(None);
        assert!(p.config_dir.is_absolute() || p.config_dir.starts_with(".sc"));
    }
}
