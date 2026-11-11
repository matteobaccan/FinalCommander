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
    /// How a key's text is drawn besides its colour: `"bold"`, `"italic"`, `"underline"`, or
    /// several of them in one string. The keys are the colour keys.
    #[serde(default)]
    styles: BTreeMap<String, String>,
}

/// What a theme asks of a key's text besides its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Attrs {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
}

impl Attrs {
    fn parse(text: &str) -> Option<Attrs> {
        let mut out = Attrs::default();
        for word in text.split(|c: char| c == ',' || c.is_whitespace()) {
            match word.trim().to_ascii_lowercase().as_str() {
                "" | "normal" => {}
                "bold" => out.bold = true,
                "italic" => out.italic = true,
                "underline" => out.underline = true,
                _ => return None,
            }
        }
        Some(out)
    }

    fn words(self) -> String {
        let mut words = Vec::new();
        if self.bold {
            words.push("bold");
        }
        if self.italic {
            words.push("italic");
        }
        if self.underline {
            words.push("underline");
        }
        words.join(" ")
    }

    /// The same style with these attributes added.
    pub fn on(self, mut style: Style) -> Style {
        style.bold |= self.bold;
        style.italic |= self.italic;
        style.underline |= self.underline;
        style
    }
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
    styles: BTreeMap<String, Attrs>,
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

/// The nine themes compiled into the binary. `commander` is the default.
pub const BUILTIN_THEMES: &[(&str, &str)] = &[
    ("commander", include_str!("../themes/commander.toml")),
    ("far", include_str!("../themes/far.toml")),
    ("black", include_str!("../themes/black.toml")),
    ("white", include_str!("../themes/white.toml")),
    ("matrix", include_str!("../themes/matrix.toml")),
    ("tron", include_str!("../themes/tron.toml")),
    ("blade", include_str!("../themes/blade.toml")),
    ("github-dark", include_str!("../themes/github-dark.toml")),
    (
        "delphi-dark-2027",
        include_str!("../themes/delphi-dark-2027.toml"),
    ),
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
    // FAR's Panel.Info.Selected: the totals turn yellow on cyan while something is selected.
    ("panel.info.selected.fg", Color::Ansi(11)),
    ("panel.info.selected.bg", Color::Ansi(6)),
    // The active panel's path is drawn inverted, as FAR does it.
    ("panel.title.active.fg", Color::Ansi(0)),
    ("panel.title.active.bg", Color::Ansi(6)),
    // The tab row, which a side draws only when it has more than one tab. The tab that is
    // on screen is drawn as the path is: inverted, and in the header's inactive colour on
    // the panel that does not have the focus.
    ("panel.tab.fg", Color::Default),
    ("panel.tab.bg", Color::Default),
    ("panel.tab.active.fg", Color::Ansi(0)),
    ("panel.tab.active.bg", Color::Ansi(6)),
    // Column titles, the sort-mode letter and the filter mark.
    ("panel.column.fg", Color::Ansi(11)),
    ("panel.scrollbar.fg", Color::Default),
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
    // FAR's `<temp>` group: backups, swap files and the rest of the litter.
    ("file.temp", Color::Ansi(3)),
    ("file.broken", Color::Ansi(9)),
    ("keybar.fg", Color::Ansi(0)),
    ("keybar.bg", Color::Ansi(6)),
    ("keybar.num.fg", Color::Ansi(7)),
    ("keybar.num.bg", Color::Ansi(0)),
    // FAR's Keybar.Background: the row beyond the labels, which is not the label colour.
    ("keybar.background.fg", Color::Ansi(7)),
    ("keybar.background.bg", Color::Ansi(0)),
    ("status.fg", Color::Default),
    ("status.bg", Color::Default),
    // Viewer.Text, Viewer.Status, Viewer.Text.Selected — and the editor's three. FAR keeps
    // these apart from the panel's and from the status bar's on purpose: the two views are
    // full-screen, their status line is the key bar's colour (black on cyan in its own
    // palette), and a search hit or a marked block is that colour too. Borrowing
    // `panel.header.fg` over `status.bg` for the title, which is what this did before, mixed
    // two roles that belong to other things and came out wrong in every theme where they
    // happen not to match.
    ("viewer.fg", Color::Default),
    ("viewer.bg", Color::Default),
    ("viewer.status.fg", Color::Ansi(0)),
    ("viewer.status.bg", Color::Ansi(6)),
    ("viewer.selected.fg", Color::Ansi(0)),
    ("viewer.selected.bg", Color::Ansi(6)),
    ("editor.fg", Color::Default),
    ("editor.bg", Color::Default),
    ("editor.status.fg", Color::Ansi(0)),
    ("editor.status.bg", Color::Ansi(6)),
    ("editor.selected.fg", Color::Ansi(0)),
    ("editor.selected.bg", Color::Ansi(6)),
    // Syntax. FAR's own editor has no colouring — its users install Colorer — so these are
    // not FAR's palette but the one every terminal editor has converged on, and they are
    // foregrounds only: the background stays the editor's, so nothing flickers between a
    // coloured run and plain text.
    ("syntax.keyword", Color::Ansi(15)),
    ("syntax.type", Color::Ansi(14)),
    ("syntax.string", Color::Ansi(10)),
    ("syntax.number", Color::Ansi(13)),
    ("syntax.comment", Color::Ansi(8)),
    ("syntax.preproc", Color::Ansi(11)),
    ("syntax.tag", Color::Ansi(14)),
    ("syntax.attr", Color::Ansi(11)),
    ("syntax.entity", Color::Ansi(13)),
    ("syntax.function", Color::Ansi(12)),
    // A diff's two kinds of line: what it adds in green and what it takes away in red, as
    // `git diff --color` and every review tool draw them.
    ("syntax.added", Color::Ansi(10)),
    ("syntax.removed", Color::Ansi(9)),
    // The side-by-side diff paints the whole of a line that differs, not only its sign: red
    // under what went, green under what came, with ink that reads on both.
    ("diff.line.fg", Color::Ansi(15)),
    ("diff.removed.bg", Color::Ansi(1)),
    ("diff.added.bg", Color::Ansi(2)),
    ("syntax.text", Color::Default),
    // The line down a Pascal block, from the word that opens it to its `end`: a colour that
    // is seen without being read.
    ("syntax.guide", Color::Ansi(8)),
    ("cmdline.fg", Color::Default),
    ("cmdline.bg", Color::Default),
    // What the completion would add, drawn after the caret and not in the line (SPEC 06
    // §7.4): dim on purpose, because it is an offer and not text somebody typed.
    ("cmdline.hint.fg", Color::Ansi(8)),
    // The menu bar (horizontal) and a pulled-down menu or list.
    ("hmenu.fg", Color::Ansi(0)),
    ("hmenu.bg", Color::Ansi(6)),
    ("hmenu.selected.fg", Color::Ansi(15)),
    ("hmenu.selected.bg", Color::Ansi(0)),
    ("hmenu.hotkey.fg", Color::Ansi(11)),
    ("menu.fg", Color::Ansi(15)),
    ("menu.bg", Color::Ansi(6)),
    ("menu.frame", Color::Ansi(15)),
    ("menu.title.fg", Color::Ansi(15)),
    ("menu.selected.fg", Color::Ansi(15)),
    ("menu.selected.bg", Color::Ansi(0)),
    ("menu.hotkey.fg", Color::Ansi(11)),
    ("menu.dim.fg", Color::Ansi(8)),
    ("dialog.fg", Color::Ansi(0)),
    ("dialog.bg", Color::Ansi(7)),
    ("dialog.frame", Color::Ansi(0)),
    ("dialog.title", Color::Ansi(4)),
    ("dialog.edit.fg", Color::Ansi(0)),
    ("dialog.edit.bg", Color::Ansi(6)),
    ("dialog.button.fg", Color::Ansi(0)),
    ("dialog.button.bg", Color::Ansi(7)),
    ("dialog.button.selected.fg", Color::Ansi(0)),
    ("dialog.button.selected.bg", Color::Ansi(6)),
    // The help is Markdown drawn on a dialog: its runs need inks chosen for that background
    // rather than the syntax theme's, which is written for a panel.
    ("help.heading.fg", Color::Ansi(4)),
    ("help.code.fg", Color::Ansi(1)),
    ("help.marker.fg", Color::Ansi(4)),
    ("help.link.fg", Color::Ansi(5)),
    ("help.frame.fg", Color::Ansi(8)),
    ("warn.fg", Color::Ansi(15)),
    ("warn.bg", Color::Ansi(1)),
    ("warn.frame", Color::Ansi(15)),
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
                styles: BTreeMap::new(),
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

        for (key, value) in &parsed.styles {
            let attrs = Attrs::parse(value).ok_or_else(|| ThemeError::BadColor {
                file: file.to_string(),
                key: format!("styles.{key}"),
                value: value.clone(),
            })?;
            if !DEFAULTS.iter().any(|(k, _)| k == key) {
                theme.warnings.push(format!(
                    "{file}: unknown theme key `styles.{key}` (ignored)"
                ));
            }
            theme.styles.insert(key.clone(), attrs);
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

    /// What the theme asks of a key's text besides its colour: bold, italic, underlined.
    pub fn attrs(&self, key: &str) -> Attrs {
        self.styles.get(key).copied().unwrap_or_default()
    }

    /// `base` with the key's colour as its foreground and the key's attributes on top: how
    /// a syntax class or a file kind is drawn.
    pub fn styled(&self, key: &str, base: Style) -> Style {
        self.attrs(key).on(base.with_fg(self.color(key)))
    }

    /// `fg_key` on the panel background, as a ready-to-use style.
    pub fn fg_on(&self, fg_key: &str, bg_key: &str) -> Style {
        self.attrs(fg_key)
            .on(Style::new(self.color(fg_key), self.color(bg_key)))
    }

    pub fn panel(&self) -> Style {
        self.fg_on("panel.fg", "panel.bg")
    }

    pub fn keybar(&self) -> Style {
        self.fg_on("keybar.fg", "keybar.bg")
    }

    /// The viewer's body: its own text colours, not the panel's.
    pub fn viewer(&self) -> Style {
        Style::new(self.color("viewer.fg"), self.color("viewer.bg"))
    }

    /// The viewer's status line — `Viewer.Status` in FAR's palette.
    pub fn viewer_status(&self) -> Style {
        Style::new(
            self.color("viewer.status.fg"),
            self.color("viewer.status.bg"),
        )
    }

    pub fn editor(&self) -> Style {
        Style::new(self.color("editor.fg"), self.color("editor.bg"))
    }

    pub fn editor_status(&self) -> Style {
        Style::new(
            self.color("editor.status.fg"),
            self.color("editor.status.bg"),
        )
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

    /// Every key the interface asks for, in the order they are declared — which groups them
    /// by what they colour (the panel, then the dialogs, then the viewer, and so on), and is
    /// therefore the order to show a person editing them.
    pub fn keys() -> impl Iterator<Item = &'static str> {
        DEFAULTS.iter().map(|(key, _)| *key)
    }

    /// Change one colour. A key the interface does not ask for is kept all the same: a theme
    /// may carry colours for a plugin that is not loaded in this run.
    pub fn set_color(&mut self, key: &str, color: Color) {
        self.colors.insert(key.to_string(), color);
    }

    /// The theme as a `[[theme]]` block's table, for writing into the configuration file.
    ///
    /// Every key is written out, not only the ones this theme set: a block that leaves a key
    /// out falls back to the built-in default, so writing the lot is what makes the file say
    /// what is on the screen.
    pub fn to_table(&self) -> toml::Table {
        let mut colors = toml::Table::new();
        for key in Theme::keys() {
            colors.insert(
                key.to_string(),
                toml::Value::String(self.color(key).to_string()),
            );
        }
        let mut table = toml::Table::new();
        table.insert("id".into(), toml::Value::String(self.id.clone()));
        table.insert("name".into(), toml::Value::String(self.name.clone()));
        if !self.author.is_empty() {
            table.insert("author".into(), toml::Value::String(self.author.clone()));
        }
        table.insert(
            "variant".into(),
            toml::Value::String(
                match self.variant {
                    Variant::Dark => "dark",
                    Variant::Light => "light",
                }
                .to_string(),
            ),
        );
        table.insert("colors".into(), toml::Value::Table(colors));
        if !self.styles.is_empty() {
            let styles: toml::Table = self
                .styles
                .iter()
                .map(|(k, a)| (k.clone(), toml::Value::String(a.words())))
                .collect();
            table.insert("styles".into(), toml::Value::Table(styles));
        }
        table
    }

    /// Write the theme out as a `[[theme]]` block, ready to paste into the config file
    /// (`fcmd --export-theme`).
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
        if !self.styles.is_empty() {
            s.push_str("\n[theme.styles]\n");
            for (key, attrs) in &self.styles {
                s.push_str(&format!("\"{key}\" = \"{}\"\n", attrs.words()));
            }
        }
        s
    }
}

/// Every theme available in this run: the nine built-ins plus whatever was found on disk.
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
    fn every_builtin_theme_loads_and_defines_every_key() {
        let set = ThemeSet::builtin_only();
        assert_eq!(set.len(), 9);
        assert_eq!(
            set.ids(),
            vec![
                "commander",
                "far",
                "black",
                "white",
                "matrix",
                "tron",
                "blade",
                "github-dark",
                "delphi-dark-2027"
            ]
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
        for id in [
            "commander",
            "far",
            "black",
            "matrix",
            "tron",
            "blade",
            "github-dark",
            "delphi-dark-2027",
        ] {
            assert_eq!(set.get(id).unwrap().variant, Variant::Dark, "{id}");
        }
    }

    /// WCAG's contrast ratio between two colours given as RGB.
    fn contrast(a: Color, b: Color) -> f64 {
        fn lum(c: Color) -> f64 {
            let Color::Rgb(r, g, b) = c else {
                panic!("{c:?} is not an RGB colour")
            };
            let ch = |v: u8| {
                let v = v as f64 / 255.0;
                if v <= 0.03928 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * ch(r) + 0.7152 * ch(g) + 0.0722 * ch(b)
        }
        let (x, y) = (lum(a), lum(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    /// The black theme paints no ground but black, and everything on it is light enough to
    /// read: 7:1 for the text, 4.5:1 for what is only glanced at.
    #[test]
    fn the_black_theme_is_black_behind_everything_and_light_on_top() {
        let t = Theme::builtin("black").unwrap();
        let black = Color::Rgb(0, 0, 0);
        for key in Theme::keys() {
            let c = t.color(key);
            if key.ends_with(".bg") || key == "panel.bg" {
                assert_eq!(c, black, "{key} is a ground and is not black");
            } else if [
                "panel.fg",
                "viewer.fg",
                "editor.fg",
                "cmdline.fg",
                "dialog.fg",
            ]
            .contains(&key)
            {
                assert!(contrast(c, black) >= 7.0, "{key}: {}", contrast(c, black));
            } else if !key.ends_with("frame") && key != "panel.scrollbar.fg" {
                assert!(contrast(c, black) >= 4.5, "{key}: {}", contrast(c, black));
            }
        }
        // The cursor has no bar to stand out with: its text does, bold and underlined.
        let cursor = t.fg_on("cursor.fg", "cursor.bg");
        assert!(cursor.bold && cursor.underline);
        assert!(t.attrs("syntax.comment").italic);
    }

    /// `[styles]` adds bold, italic and underline to a key, and survives an export.
    #[test]
    fn a_theme_says_how_a_key_is_drawn_besides_its_colour() {
        let t = Theme::from_toml(
            "t.toml",
            r##"
            id = "t"
            [colors]
            "syntax.keyword" = "#ffffff"
            [styles]
            "syntax.keyword" = "bold italic"
            "cursor.fg" = "underline"
            "##,
        )
        .unwrap();
        let k = t.styled("syntax.keyword", Style::default());
        assert!(k.bold && k.italic && !k.underline);
        assert!(t.fg_on("cursor.fg", "cursor.bg").underline);
        assert!(!t.attrs("panel.fg").bold);
        let again = Theme::from_table("fcmd.toml", &t.to_table()).unwrap();
        assert_eq!(again.attrs("syntax.keyword"), t.attrs("syntax.keyword"));
        assert!(
            Theme::from_toml("b.toml", "id = \"b\"\n[styles]\n\"panel.fg\" = \"blink\"\n").is_err()
        );
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
            "fcmd.toml",
            &[table(
                r##"
                id = "commander"
                name = "Mine"
                [colors]
                "panel.bg" = "#010203"
                "##,
            )],
        );
        assert_eq!(set.len(), 9, "the config theme replaced the built-in one");
        let t = set.get("commander").unwrap();
        assert_eq!(t.name, "Mine");
        assert_eq!(t.color("panel.bg"), Color::Rgb(1, 2, 3));
        assert_eq!(t.source, ThemeSource::Config);
    }

    #[test]
    fn a_broken_config_theme_is_reported_not_fatal() {
        let set = ThemeSet::builtin_only().with_tables(
            "fcmd.toml",
            &[table(
                r##"
                id = "oops"
                [colors]
                "panel.fg" = "chartreuse"
                "##,
            )],
        );
        assert_eq!(set.len(), 9, "the broken theme was not added");
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
        assert_eq!(set.next_after("commander").id, "far");
        assert_eq!(set.next_after("blade").id, "github-dark");
        assert_eq!(set.next_after("delphi-dark-2027").id, "commander");
        assert_eq!(set.next_after("nonexistent").id, "far");
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
        let reparsed = Theme::from_table("fcmd.toml", &block).unwrap();
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
