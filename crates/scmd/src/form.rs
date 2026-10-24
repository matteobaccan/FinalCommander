//! The settings dialogs and the choose-one lists behind the menu (SPEC 10 §3, §4).
//!
//! A form row carries the configuration key it edits, so applying a form is the same code
//! path as `--set key=value` on the command line: one place decides what a setting means.

use scmd_config::Config;
use scmd_i18n::I18n;
use scmd_ui::widgets::FormRow;
use unicode_width::UnicodeWidthStr;

use crate::menu::{FormKind, PickKind};

/// One row of a settings dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// `[x] Label`, bound to a boolean configuration key.
    Toggle {
        key: &'static str,
        label: String,
        on: bool,
    },
    /// `Label: value`, cycling through the values of a configuration key.
    Choice {
        key: &'static str,
        label: String,
        /// (value written to the config, label shown)
        options: Vec<(String, String)>,
        chosen: usize,
    },
    /// A line of text. `Label: value` with a cursor while the row has the focus.
    Text {
        key: &'static str,
        label: String,
        value: String,
        /// Where the caret is, in characters.
        caret: usize,
        /// Shown instead of the characters, for a secret.
        secret: bool,
    },
    /// In the dialog for completeness; not settable yet.
    Dim(String),
    Separator,
}

impl Field {
    fn toggle(key: &'static str, label: impl Into<String>, on: bool) -> Field {
        Field::Toggle {
            key,
            label: label.into(),
            on,
        }
    }

    fn text(key: &'static str, label: impl Into<String>, value: impl Into<String>) -> Field {
        let value: String = value.into();
        Field::Text {
            key,
            label: label.into(),
            caret: value.chars().count(),
            value,
            secret: false,
        }
    }

    /// Can the cursor stop here?
    fn selectable(&self) -> bool {
        matches!(
            self,
            Field::Toggle { .. } | Field::Choice { .. } | Field::Text { .. }
        )
    }
}

/// What a key does inside a text row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextEdit {
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
}

/// A settings dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub title: String,
    pub fields: Vec<Field>,
    pub cursor: usize,
    /// For the session editor: which `[[site]]` this edits, or `None` for a new one.
    pub site: Option<usize>,
    /// `Some(0)` = OK, `Some(1)` = Cancel; `None` while the cursor is in the rows.
    pub button: Option<usize>,
}

fn choice(key: &'static str, label: String, options: &[(&str, String)], current: &str) -> Field {
    let options: Vec<(String, String)> = options
        .iter()
        .map(|(v, l)| ((*v).to_string(), l.clone()))
        .collect();
    let chosen = options
        .iter()
        .position(|(v, _)| v == current)
        .unwrap_or_default();
    Field::Choice {
        key,
        label,
        options,
        chosen,
    }
}

impl Form {
    /// Build a dialog from the configuration as it stands.
    pub fn build(kind: FormKind, cfg: &Config, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = match kind {
            // The session editor is built by `Form::site`, which needs the site itself.
            FormKind::Site => Vec::new(),
            FormKind::System => vec![
                Field::toggle(
                    "panel.natural_sort",
                    t("opt-natural-sort"),
                    cfg.panel.natural_sort,
                ),
                Field::toggle(
                    "panel.allow_reverse_sort",
                    t("opt-allow-reverse"),
                    cfg.panel.allow_reverse_sort,
                ),
                Field::Separator,
                Field::Dim(t("opt-trash")),
                Field::Dim(t("opt-system-copy")),
                Field::Dim(t("opt-save-history")),
                Field::Separator,
                choice(
                    "net.fetch_limit_mb",
                    t("opt-fetch-limit"),
                    &[
                        ("64", "64 MB".into()),
                        ("256", "256 MB".into()),
                        ("512", "512 MB".into()),
                        ("2048", "2 GB".into()),
                    ],
                    &cfg.net.fetch_limit_mb.to_string(),
                ),
                choice(
                    "net.keepalive_secs",
                    t("opt-keepalive"),
                    &[
                        ("0", t("opt-off")),
                        ("15", "15 s".into()),
                        ("30", "30 s".into()),
                        ("60", "60 s".into()),
                    ],
                    &cfg.net.keepalive_secs.to_string(),
                ),
            ],
            FormKind::Panel => vec![
                Field::toggle(
                    "panel.show_hidden",
                    t("opt-show-hidden"),
                    cfg.panel.show_hidden,
                ),
                Field::toggle(
                    "panel.dirs_first",
                    t("opt-dirs-first"),
                    cfg.panel.dirs_first,
                ),
                Field::toggle(
                    "panel.selected_first",
                    t("opt-selected-first"),
                    cfg.panel.selected_first,
                ),
                Field::toggle("panel.reverse", t("opt-reverse"), cfg.panel.reverse),
                Field::Separator,
                choice(
                    "panel.view",
                    t("opt-view"),
                    &[
                        ("brief", t("menu-brief")),
                        ("full", t("menu-full")),
                        ("detail", t("menu-detailed")),
                        ("wide", t("menu-wide")),
                    ],
                    &cfg.panel.view,
                ),
                choice(
                    "panel.sort",
                    t("opt-sort"),
                    &[
                        ("name", t("sort-name")),
                        ("ext", t("sort-ext")),
                        ("size", t("sort-size")),
                        ("mtime", t("sort-mtime")),
                        ("none", t("sort-none")),
                    ],
                    &cfg.panel.sort,
                ),
                choice(
                    "panel.quick_search",
                    t("opt-quick-search"),
                    &[
                        ("alt", t("opt-quick-alt")),
                        ("plain", t("opt-quick-plain")),
                        ("off", t("opt-off")),
                    ],
                    match cfg.panel.quick_search {
                        scmd_config::QuickSearch::Alt => "alt",
                        scmd_config::QuickSearch::Plain => "plain",
                        scmd_config::QuickSearch::Off => "off",
                    },
                ),
            ],
            FormKind::Interface => vec![
                Field::toggle("ui.key_bar", t("opt-key-bar"), cfg.ui.key_bar),
                Field::toggle("ui.status_bar", t("opt-status-bar"), cfg.ui.status_bar),
                Field::Separator,
                choice(
                    "ui.keymap",
                    t("opt-keymap"),
                    &[("far", "far".into()), ("mc", "mc".into())],
                    &cfg.ui.keymap,
                ),
                Field::Separator,
                Field::Dim(t("opt-clock")),
                Field::Dim(t("opt-mouse")),
                Field::Dim(t("opt-menu-always")),
            ],
            FormKind::Confirmations => vec![
                Field::toggle(
                    "general.confirm_delete",
                    t("opt-confirm-delete"),
                    cfg.general.confirm_delete,
                ),
                Field::toggle(
                    "general.confirm_overwrite",
                    t("opt-confirm-overwrite"),
                    cfg.general.confirm_overwrite,
                ),
                Field::toggle(
                    "general.confirm_exit",
                    t("opt-confirm-exit"),
                    cfg.general.confirm_exit,
                ),
                Field::Separator,
                Field::Dim(t("opt-confirm-editor")),
            ],
        };
        let title = i18n.t(match kind {
            FormKind::Site => "site-new",
            FormKind::System => "menu-system-settings",
            FormKind::Panel => "menu-panel-settings",
            FormKind::Interface => "menu-interface-settings",
            FormKind::Confirmations => "menu-confirmations",
        });
        let cursor = fields.iter().position(Field::selectable).unwrap_or(0);
        Form {
            kind,
            title,
            fields,
            cursor,
            site: None,
            button: None,
        }
    }

    /// The session editor: a stored connection, or a new one when `site` is `None`
    /// (SPEC 10 §3.5).
    pub fn site(site: Option<(usize, &scmd_config::SiteCfg)>, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let blank = scmd_config::SiteCfg::default();
        let (index, cfg) = match site {
            Some((index, cfg)) => (Some(index), cfg),
            None => (None, &blank),
        };
        let auth = cfg.auth.clone().unwrap_or_else(|| "agent".to_string());
        let (auth_kind, key_path) = match auth.split_once(':') {
            Some(("key", path)) => ("key", path.to_string()),
            _ => (auth.as_str(), String::new()),
        };
        let fields = vec![
            Field::text("site.id", t("site-name"), cfg.id.clone()),
            Field::text("site.host", t("site-host"), cfg.host.clone()),
            Field::text(
                "site.port",
                t("site-port"),
                cfg.port.map(|p| p.to_string()).unwrap_or_default(),
            ),
            Field::text(
                "site.user",
                t("site-user"),
                cfg.user.clone().unwrap_or_default(),
            ),
            Field::Separator,
            choice(
                "site.auth",
                t("site-auth"),
                &[
                    ("agent", t("site-auth-agent")),
                    ("key", t("site-auth-key")),
                    ("ask", t("site-auth-ask")),
                ],
                auth_kind,
            ),
            Field::text("site.key", t("site-key-file"), key_path),
            Field::Separator,
            Field::text(
                "site.start_dir",
                t("site-start-dir"),
                cfg.start_dir.clone().unwrap_or_default(),
            ),
            Field::text(
                "site.group",
                t("site-group"),
                cfg.group.clone().unwrap_or_default(),
            ),
            Field::text(
                "site.keepalive_secs",
                t("site-keepalive"),
                cfg.keepalive_secs
                    .map(|s| s.to_string())
                    .unwrap_or_default(),
            ),
            Field::toggle(
                "site.compress",
                t("site-compress"),
                cfg.compress == Some(true),
            ),
        ];
        Form {
            kind: FormKind::Site,
            title: i18n.t(if index.is_some() {
                "site-edit"
            } else {
                "site-new"
            }),
            fields,
            cursor: 0,
            site: index,
            button: None,
        }
    }

    /// What a text row holds now, by key.
    pub fn text_value(&self, want: &str) -> String {
        for field in &self.fields {
            match field {
                Field::Text { key, value, .. } if *key == want => return value.trim().to_string(),
                Field::Choice {
                    key,
                    options,
                    chosen,
                    ..
                } if *key == want => {
                    return options
                        .get(*chosen)
                        .map(|(v, _)| v.clone())
                        .unwrap_or_default();
                }
                Field::Toggle { key, on, .. } if *key == want => return on.to_string(),
                _ => {}
            }
        }
        String::new()
    }

    /// Typing into the row under the cursor. Returns false if the row takes no text.
    pub fn type_char(&mut self, c: char) -> bool {
        let Some(Field::Text { value, caret, .. }) = self.fields.get_mut(self.cursor) else {
            return false;
        };
        let at = value
            .char_indices()
            .nth(*caret)
            .map(|(i, _)| i)
            .unwrap_or(value.len());
        value.insert(at, c);
        *caret += 1;
        true
    }

    /// `Backspace`, `Del`, `←`, `→`, `Home`, `End` inside a text row.
    pub fn edit_text(&mut self, what: TextEdit) -> bool {
        let Some(Field::Text { value, caret, .. }) = self.fields.get_mut(self.cursor) else {
            return false;
        };
        let len = value.chars().count();
        match what {
            TextEdit::Backspace if *caret > 0 => {
                let at = value
                    .char_indices()
                    .nth(*caret - 1)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(at);
                *caret -= 1;
            }
            TextEdit::Delete if *caret < len => {
                let at = value
                    .char_indices()
                    .nth(*caret)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(at);
            }
            TextEdit::Left => *caret = caret.saturating_sub(1),
            TextEdit::Right => *caret = (*caret + 1).min(len),
            TextEdit::Home => *caret = 0,
            TextEdit::End => *caret = len,
            _ => {}
        }
        true
    }

    /// Where the caret sits in the dialog: the row, and the column inside the row's text.
    /// `None` when the cursor is not in a text row, so the terminal cursor stays hidden.
    pub fn caret(&self) -> Option<(usize, usize)> {
        if self.button.is_some() {
            return None;
        }
        match self.fields.get(self.cursor) {
            Some(Field::Text {
                label,
                value,
                caret,
                ..
            }) => {
                let before: String = value.chars().take(*caret).collect();
                Some((
                    self.cursor,
                    UnicodeWidthStr::width(label.as_str())
                        + 2
                        + UnicodeWidthStr::width(before.as_str()),
                ))
            }
            _ => None,
        }
    }

    /// Is the cursor on a row that takes typing?
    pub fn on_text(&self) -> bool {
        matches!(self.fields.get(self.cursor), Some(Field::Text { .. }))
    }

    /// The rows as the renderer wants them.
    pub fn rows(&self) -> Vec<FormRow> {
        self.fields
            .iter()
            .map(|f| match f {
                Field::Separator => FormRow::separator(),
                Field::Dim(label) => FormRow {
                    text: format!("[ ] {label}"),
                    dim: true,
                    ..FormRow::default()
                },
                Field::Toggle { label, on, .. } => {
                    FormRow::new(format!("[{}] {label}", if *on { 'x' } else { ' ' }))
                }
                Field::Text {
                    label,
                    value,
                    secret,
                    ..
                } => FormRow::new(format!(
                    "{label}: {}",
                    if *secret {
                        "*".repeat(value.chars().count())
                    } else {
                        value.clone()
                    }
                )),
                Field::Choice {
                    label,
                    options,
                    chosen,
                    ..
                } => FormRow::new(format!(
                    "{label}: {} ↓",
                    options.get(*chosen).map(|(_, l)| l.as_str()).unwrap_or("")
                )),
            })
            .collect()
    }

    pub fn move_cursor(&mut self, down: bool) {
        let len = self.fields.len();
        if len == 0 {
            return;
        }
        let mut i = self.cursor;
        for _ in 0..len {
            i = if down {
                (i + 1) % len
            } else {
                (i + len - 1) % len
            };
            if self.fields[i].selectable() {
                self.cursor = i;
                return;
            }
        }
    }

    /// `Space` on a checkbox, or `Space`/`→` on a choice.
    pub fn toggle(&mut self, forward: bool) {
        match self.fields.get_mut(self.cursor) {
            Some(Field::Toggle { on, .. }) => *on = !*on,
            Some(Field::Choice {
                options, chosen, ..
            }) => {
                if options.is_empty() {
                    return;
                }
                *chosen = if forward {
                    (*chosen + 1) % options.len()
                } else {
                    (*chosen + options.len() - 1) % options.len()
                };
            }
            _ => {}
        }
    }

    /// The configuration keys this dialog owns, so applying it touches nothing else.
    pub fn keys(&self) -> Vec<&'static str> {
        self.fields
            .iter()
            .filter_map(|f| match f {
                Field::Toggle { key, .. } | Field::Choice { key, .. } | Field::Text { key, .. } => {
                    Some(*key)
                }
                _ => None,
            })
            .collect()
    }

    /// Write every row into the configuration. Returns what could not be applied.
    pub fn apply(&self, cfg: &mut Config) -> Vec<String> {
        let mut problems = Vec::new();
        for field in &self.fields {
            let result = match field {
                Field::Toggle { key, on, .. } => cfg.set(key, if *on { "true" } else { "false" }),
                Field::Choice {
                    key,
                    options,
                    chosen,
                    ..
                } => match options.get(*chosen) {
                    Some((value, _)) => cfg.set(key, value),
                    None => Ok(()),
                },
                _ => Ok(()),
            };
            if let Err(e) = result {
                problems.push(e);
            }
        }
        problems
    }
}

/// A list where one row is chosen.
#[derive(Debug, Clone, PartialEq)]
pub struct Pick {
    pub kind: PickKind,
    pub title: String,
    pub rows: Vec<PickRow>,
    pub cursor: usize,
    /// Which panel the choice applies to, for the panel-side lists.
    pub side: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PickRow {
    /// What the user reads.
    pub label: String,
    /// What the choice means: a language tag, a theme id, a path, a sort key.
    pub value: String,
    /// The row that is in force now.
    pub current: bool,
}

impl Pick {
    pub fn new(kind: PickKind, title: String, rows: Vec<PickRow>, side: usize) -> Pick {
        let cursor = rows.iter().position(|r| r.current).unwrap_or(0);
        Pick {
            kind,
            title,
            rows,
            cursor,
            side,
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.rows.is_empty() {
            return;
        }
        let len = self.rows.len() as isize;
        self.cursor = (self.cursor as isize + delta).rem_euclid(len) as usize;
    }

    pub fn chosen(&self) -> Option<&PickRow> {
        self.rows.get(self.cursor)
    }

    pub fn lines(&self) -> Vec<String> {
        self.rows
            .iter()
            .map(|r| format!("{} {}", if r.current { '√' } else { ' ' }, r.label))
            .collect()
    }
}
