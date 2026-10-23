use serde::Deserialize;
use std::collections::BTreeMap;

use crate::buffer::Style;
use crate::color::{Color, ColorDepth};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Variant {
    #[default]
    Dark,
    Light,
}

#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("{file}: {source}")]
    Toml {
        file: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("{file}: `{key}` is not a colour: `{value}`")]
    BadColor {
        file: String,
        key: String,
        value: String,
    },
    #[error("unknown theme `{0}`")]
    Unknown(String),
}

#[derive(Debug, Deserialize)]
struct ThemeFile {
    #[serde(default)]
    id: String,
    #[serde(default)]
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    variant: Variant,
    #[serde(default)]
    inherit: String,
    #[serde(default)]
    palette: BTreeMap<String, String>,
    #[serde(default)]
    colors: BTreeMap<String, String>,
}

/// A resolved theme: a flat map of semantic keys to colours.
///
/// A missing key falls back to the built-in default for that key, so a theme written for an
/// older (or newer) version still renders — unknown keys are reported as warnings, never
/// errors. See `docs/spec/07-config-theming.md` §4.
#[derive(Debug, Clone)]
pub struct Theme {
    pub id: String,
    pub name: String,
    pub author: String,
    pub variant: Variant,
    pub source: ThemeSource,
    colors: BTreeMap<String, Color>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThemeSource {
    /// Compiled into the executable.
    BuiltIn,
    /// Declared as a `[[theme]]` block in the single configuration file.
    Config,
    /// Provided by a plugin (the only other thing that may ship themes).
    Plugin(String),
}

/// The six themes compiled into the binary. `commander` is the default.
pub const BUILTIN_THEMES: &[(&str, &str)] = &[
    ("commander", include_str!("../themes/commander.toml")),
    ("black", include_str!("../themes/black.toml")),
    ("white", include_str!("../themes/white.toml")),
    ("matrix", include_str!("../themes/matrix.toml")),
    ("tron", include_str!("../themes/tron.toml")),
    ("blade", include_str!("../themes/blade.toml")),
];

pub const DEFAULT_THEME: &str = "commander";

/// Keys the UI asks for, with the fallback used when a theme omits them.
const DEFAULTS: &[(&str, Color)] = &[
    ("panel.bg", Color::Default),
    ("panel.fg", Color::Default),
    ("panel.frame", Color::Default),
    ("panel.header.fg", Color::Default),
    ("panel.header.bg", Color::Default),
    ("panel.header.inactive.fg", Color::Default),
    ("panel.footer.fg", Color::Default),
    ("cursor.fg", Color::Ansi(0)),
    ("cursor.bg", Color::Ansi(7)),
    ("cursor.inactive.fg", Color::Default),
    ("cursor.inactive.bg", Color::Ansi(8)),
    ("selected.fg", Color::Ansi(11)),
    ("file.dir", Color::Default),
    ("file.exec", Color::Ansi(10)),
    ("file.symlink", Color::Ansi(13)),
    ("file.archive", Color::Ansi(13)),
    ("file.hidden", Color::Ansi(8)),
    ("file.broken", Color::Ansi(9)),
    ("keybar.fg", Color::Ansi(0)),
    ("keybar.bg", Color::Ansi(6)),
    ("keybar.num.fg", Color::Ansi(7)),
    ("keybar.num.bg", Color::Ansi(0)),
    ("status.fg", Color::Default),
    ("status.bg", Color::Default),
    ("cmdline.fg", Color::Default),
    ("cmdline.bg", Color::Default),
    ("dialog.fg", Color::Ansi(0)),
    ("dialog.bg", Color::Ansi(7)),
    ("dialog.frame", Color::Ansi(0)),
    ("dialog.title", Color::Ansi(4)),
    ("error.fg", Color::Ansi(9)),
    ("filter.fg", Color::Ansi(11)),
    ("search.fg", Color::Ansi(11)),
];

impl Theme {
    pub fn builtin(id: &str) -> Result<Theme, ThemeError> {
        let (_, src) = BUILTIN_THEMES
            .iter()
            .find(|(name, _)| *name == id)
            .ok_or_else(|| ThemeError::Unknown(id.to_string()))?;
        let mut t = Theme::from_toml(&format!("{id}.toml"), src)?;
        t.source = ThemeSource::BuiltIn;
        Ok(t)
    }

    pub fn default_theme() -> Theme {
        Theme::builtin(DEFAULT_THEME).expect("the default theme compiles")
    }

    /// A `[[theme]]` block from the configuration file (or from a plugin manifest).
    pub fn from_table(origin: &str, table: &toml::Table) -> Result<Theme, ThemeError> {
        let src = toml::to_string(table).map_err(|e| ThemeError::BadColor {
            file: origin.to_string(),
            key: "[[theme]]".to_string(),
            value: e.to_string(),
        })?;
        Theme::from_toml(origin, &src)
    }

    pub fn from_toml(file: &str, src: &str) -> Result<Theme, ThemeError> {
        let parsed: ThemeFile = toml::from_str(src).map_err(|source| ThemeError::Toml {
            file: file.to_string(),
            source,
        })?;

        let mut theme = if parsed.inherit.is_empty() {
            Theme {
                id: String::new(),
                name: String::new(),
                author: String::new(),
                variant: Variant::default(),
                source: ThemeSource::BuiltIn,
                colors: BTreeMap::new(),
                warnings: Vec::new(),
            }
        } else {
            Theme::builtin(&parsed.inherit)?
        };

        if !parsed.id.is_empty() {
            theme.id = parsed.id.clone();
        } else if theme.id.is_empty() {
            theme.id = file.trim_end_matches(".toml").to_string();
        }
        if !parsed.name.is_empty() {
            theme.name = parsed.name.clone();
        }
        if !parsed.author.is_empty() {
            theme.author = parsed.author.clone();
        }
        theme.variant = parsed.variant;

        let mut palette: BTreeMap<String, Color> = BTreeMap::new();
        for (k, v) in &parsed.palette {
            let color = Color::parse(v).ok_or_else(|| ThemeError::BadColor {
                file: file.to_string(),
                key: format!("palette.{k}"),
                value: v.clone(),
            })?;
            palette.insert(k.clone(), color);
        }

        for (key, value) in &parsed.colors {
            let color = match palette.get(value.trim()) {
                Some(c) => *c,
                None => Color::parse(value).ok_or_else(|| ThemeError::BadColor {
                    file: file.to_string(),
                    key: key.clone(),
                    value: value.clone(),
                })?,
            };
            if !DEFAULTS.iter().any(|(k, _)| k == key) {
                theme
                    .warnings
                    .push(format!("{file}: unknown theme key `{key}` (ignored)"));
            }
            theme.colors.insert(key.clone(), color);
        }

        if theme.name.is_empty() {
            theme.name = theme.id.clone();
        }
        Ok(theme)
    }

    /// Non-fatal problems found while loading (unknown keys). The UI shows them once.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    pub fn color(&self, key: &str) -> Color {
        if let Some(c) = self.colors.get(key) {
            return *c;
        }
        DEFAULTS
            .iter()
            .find(|(k, _)| *k == key)
            .map(|(_, c)| *c)
            .unwrap_or(Color::Default)
    }

    /// `fg_key` on the panel background, as a ready-to-use style.
    pub fn fg_on(&self, fg_key: &str, bg_key: &str) -> Style {
        Style::new(self.color(fg_key), self.color(bg_key))
    }

    pub fn panel(&self) -> Style {
        self.fg_on("panel.fg", "panel.bg")
    }

    pub fn keybar(&self) -> Style {
        self.fg_on("keybar.fg", "keybar.bg")
    }

    pub fn dialog(&self) -> Style {
        self.fg_on("dialog.fg", "dialog.bg")
    }

    /// Degrade every colour once, at startup, instead of on every cell.
    pub fn degraded(&self, depth: ColorDepth) -> Theme {
        let mut out = self.clone();
        for c in out.colors.values_mut() {
            *c = c.degrade(depth);
        }
        out
    }

    /// Write the theme out as a `[[theme]]` block, ready to paste into the config file
    /// (`sc --export-theme`).
    pub fn to_toml(&self) -> String {
        let mut s = String::new();
        s.push_str("[[theme]]\n");
        s.push_str(&format!("id = \"{}\"\n", self.id));
        s.push_str(&format!("name = \"{}\"\n", self.name));
        if !self.author.is_empty() {
            s.push_str(&format!("author = \"{}\"\n", self.author));
        }
        s.push_str(&format!(
            "variant = \"{}\"\n\n[theme.colors]\n",
            match self.variant {
                Variant::Dark => "dark",
                Variant::Light => "light",
            }
        ));
        for (key, _) in DEFAULTS {
            s.push_str(&format!("\"{key}\" = \"{}\"\n", self.color(key)));
        }
        s
    }
}

/// Every theme available in this run: the six built-ins plus whatever was found on disk.
/// A file theme with the id of a built-in replaces it, so a user can retune `commander`
/// without renaming it.
#[derive(Debug, Clone)]
pub struct ThemeSet {
    themes: Vec<Theme>,
    pub load_errors: Vec<String>,
}

impl ThemeSet {
    pub fn builtin_only() -> ThemeSet {
        let themes = BUILTIN_THEMES
            .iter()
            .map(|(id, _)| Theme::builtin(id).expect("built-in themes compile"))
            .collect();
        ThemeSet {
            themes,
            load_errors: Vec::new(),
        }
    }

    /// Add the `[[theme]]` blocks declared in the configuration file.
    ///
    /// A theme with the id of a built-in replaces it, so a user can retune `commander`
    /// without inventing a new name. A broken block is reported, never fatal.
    pub fn with_tables(mut self, origin: &str, tables: &[toml::Table]) -> ThemeSet {
        for table in tables {
            match Theme::from_table(origin, table) {
                Ok(mut theme) => {
                    theme.source = ThemeSource::Config;
                    self.themes.retain(|t| t.id != theme.id);
                    self.themes.push(theme);
                }
                Err(e) => self.load_errors.push(e.to_string()),
            }
        }
        self
    }

    /// Add themes a plugin declares. Same rules, different origin in `--list-themes`.
    pub fn with_plugin_tables(mut self, plugin: &str, tables: &[toml::Table]) -> ThemeSet {
        for table in tables {
            match Theme::from_table(plugin, table) {
                Ok(mut theme) => {
                    theme.source = ThemeSource::Plugin(plugin.to_string());
                    self.themes.retain(|t| t.id != theme.id);
                    self.themes.push(theme);
                }
                Err(e) => self.load_errors.push(e.to_string()),
            }
        }
        self
    }

    pub fn ids(&self) -> Vec<&str> {
        self.themes.iter().map(|t| t.id.as_str()).collect()
    }

    pub fn len(&self) -> usize {
        self.themes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.themes.is_empty()
    }

    pub fn get(&self, id: &str) -> Option<&Theme> {
        self.themes.iter().find(|t| t.id == id)
    }

    /// The theme after `id`, wrapping around (`app.theme.next`).
    pub fn next_after(&self, id: &str) -> &Theme {
        let pos = self.themes.iter().position(|t| t.id == id).unwrap_or(0);
        let next = (pos + 1) % self.themes.len().max(1);
        &self.themes[next]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Theme> {
        self.themes.iter()
    }
}

impl Default for ThemeSet {
    fn default() -> Self {
        Self::builtin_only()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_six_builtin_themes_load_and_define_every_key() {
        let set = ThemeSet::builtin_only();
        assert_eq!(set.len(), 6);
        assert_eq!(
            set.ids(),
            vec!["commander", "black", "white", "matrix", "tron", "blade"]
        );
        for theme in set.iter() {
            assert!(
                theme.warnings().is_empty(),
                "{}: {:?}",
                theme.id,
                theme.warnings()
            );
            for (key, _) in DEFAULTS {
                assert!(
                    theme.colors.contains_key(*key),
                    "theme `{}` does not define `{key}`",
                    theme.id
                );
            }
        }
    }

    #[test]
    fn white_is_light_and_the_rest_are_dark() {
        let set = ThemeSet::builtin_only();
        assert_eq!(set.get("white").unwrap().variant, Variant::Light);
        for id in ["commander", "black", "matrix", "tron", "blade"] {
            assert_eq!(set.get(id).unwrap().variant, Variant::Dark, "{id}");
        }
    }

    #[test]
    fn palette_names_resolve() {
        let t = Theme::from_toml(
            "t.toml",
            r##"
            id = "t"
            [palette]
            sky = "#3366ff"
            [colors]
            "panel.fg" = "sky"
            "panel.bg" = "#101010"
            "##,
        )
        .unwrap();
        assert_eq!(t.color("panel.fg"), Color::Rgb(0x33, 0x66, 0xff));
        assert_eq!(t.color("panel.bg"), Color::Rgb(0x10, 0x10, 0x10));
    }

    #[test]
    fn inheritance_overrides_single_keys() {
        let t = Theme::from_toml(
            "mine.toml",
            r##"
            id = "mine"
            inherit = "commander"
            [colors]
            "selected.fg" = "#ff0000"
            "##,
        )
        .unwrap();
        assert_eq!(t.color("selected.fg"), Color::Rgb(255, 0, 0));
        // inherited from commander
        assert_eq!(t.color("panel.bg"), Color::Rgb(0x00, 0x00, 0x80));
    }

    #[test]
    fn unknown_keys_warn_but_load() {
        let t = Theme::from_toml(
            "x.toml",
            r##"
            id = "x"
            [colors]
            "panel.fg" = "#ffffff"
            "future.thing" = "#ffffff"
            "##,
        )
        .unwrap();
        assert_eq!(t.warnings().len(), 1);
        assert!(t.warnings()[0].contains("future.thing"));
    }

    #[test]
    fn a_bad_colour_is_an_error_that_names_the_key() {
        let err = Theme::from_toml(
            "x.toml",
            r##"
            [colors]
            "panel.fg" = "chartreuse"
            "##,
        )
        .unwrap_err();
        assert_eq!(
            err.to_string(),
            "x.toml: `panel.fg` is not a colour: `chartreuse`"
        );
    }

    #[test]
    fn missing_keys_fall_back_to_defaults() {
        let t = Theme::from_toml("empty.toml", "id = \"empty\"").unwrap();
        assert_eq!(t.color("cursor.bg"), Color::Ansi(7));
        assert_eq!(t.color("panel.bg"), Color::Default);
    }

    fn table(src: &str) -> toml::Table {
        src.parse::<toml::Table>().unwrap()
    }

    #[test]
    fn a_config_theme_overrides_a_builtin_with_the_same_id() {
        let set = ThemeSet::builtin_only().with_tables(
            "sc.toml",
            &[table(
                r##"
                id = "commander"
                name = "Mine"
                [colors]
                "panel.bg" = "#010203"
                "##,
            )],
        );
        assert_eq!(set.len(), 6, "the config theme replaced the built-in one");
        let t = set.get("commander").unwrap();
        assert_eq!(t.name, "Mine");
        assert_eq!(t.color("panel.bg"), Color::Rgb(1, 2, 3));
        assert_eq!(t.source, ThemeSource::Config);
    }

    #[test]
    fn a_broken_config_theme_is_reported_not_fatal() {
        let set = ThemeSet::builtin_only().with_tables(
            "sc.toml",
            &[table(
                r##"
                id = "oops"
                [colors]
                "panel.fg" = "chartreuse"
                "##,
            )],
        );
        assert_eq!(set.len(), 6, "the broken theme was not added");
        assert_eq!(set.load_errors.len(), 1);
        assert!(
            set.load_errors[0].contains("chartreuse"),
            "{:?}",
            set.load_errors
        );
    }

    #[test]
    fn plugin_themes_say_where_they_came_from() {
        let set =
            ThemeSet::builtin_only().with_plugin_tables("sunset", &[table(r#"id = "sunset""#)]);
        assert_eq!(
            set.get("sunset").unwrap().source,
            ThemeSource::Plugin("sunset".into())
        );
    }

    #[test]
    fn theme_cycling_wraps() {
        let set = ThemeSet::builtin_only();
        assert_eq!(set.next_after("commander").id, "black");
        assert_eq!(set.next_after("blade").id, "commander");
        assert_eq!(set.next_after("nonexistent").id, "black");
    }

    #[test]
    fn export_round_trips_through_the_config_shape() {
        let original = Theme::builtin("tron").unwrap();
        let exported = original.to_toml();
        assert!(exported.starts_with("[[theme]]"), "{exported}");
        let as_config: toml::Table = exported.parse().unwrap();
        let block = as_config["theme"].as_array().unwrap()[0]
            .as_table()
            .unwrap()
            .clone();
        let reparsed = Theme::from_table("sc.toml", &block).unwrap();
        for (key, _) in DEFAULTS {
            assert_eq!(reparsed.color(key), original.color(key), "{key}");
        }
    }

    #[test]
    fn degrading_keeps_every_key() {
        let t = Theme::builtin("matrix")
            .unwrap()
            .degraded(ColorDepth::Ansi16);
        for (key, _) in DEFAULTS {
            assert!(matches!(
                t.color(key),
                Color::Ansi(_) | Color::Default | Color::Indexed(_)
            ));
        }
    }
}
