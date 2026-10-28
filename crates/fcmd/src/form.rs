//! The settings dialogs and the choose-one lists behind the menu (SPEC 10 §3, §4).
//!
//! A form row carries the configuration key it edits, so applying a form is the same code
//! path as `--set key=value` on the command line: one place decides what a setting means.

use fcmd_config::Config;
use fcmd_i18n::I18n;
use fcmd_ui::widgets::FormRow;
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
    /// In the dialog for completeness; not settable yet. Drawn as an unticked box, because
    /// that is what it is: a setting FAR has and this does not.
    Dim(String),
    /// A line that says something rather than asking something — a name, what a file is now.
    /// Not a setting, so not a box: a box nobody can tick is a worse lie than no box.
    Note(String),
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

/// One page of a dialog that holds more than one screenful of a subject.
///
/// FAR's session editor has six of them along the top and so has this one: a dialog with
/// twenty rows in it is a dialog nobody reads, and the pages say what a row is *about*
/// before it is read. A page that does not apply to what is being edited — the SFTP page of
/// an FTP session — is drawn grey and skipped, rather than offering settings that would do
/// nothing.
#[derive(Debug, Clone, PartialEq)]
pub struct Page {
    pub label: String,
    /// The fields it holds, as a half-open range into [`Form::fields`].
    pub from: usize,
    pub to: usize,
    pub enabled: bool,
}

/// A settings dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub title: String,
    pub fields: Vec<Field>,
    /// The pages the fields are divided into, or empty for a dialog that is one page — which
    /// is every dialog but the session editor.
    pub pages: Vec<Page>,
    /// Which page is on screen.
    pub page: usize,
    pub cursor: usize,
    /// For the session editor: which `[[site]]` this edits, or `None` for a new one.
    pub site: Option<usize>,
    /// Which button the cursor is on; `None` while it is in the rows. The last button is
    /// always the one that closes the dialog without doing anything.
    pub button: Option<usize>,
    /// What the buttons say. Most dialogs answer OK or Cancel; a search has three answers,
    /// because *next* and *previous* are the same question asked in two directions.
    pub buttons: Vec<String>,
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

/// A message with a menu label's `&` marker taken off.
fn plain(i18n: &I18n, id: &str) -> String {
    fcmd_ui::mnemonic(&i18n.t(id)).0
}

/// What the attributes dialog is opened about: one file, or a selection of them.
#[derive(Debug, Clone, Default)]
pub struct AttrFacts {
    /// The name shown at the top, which is the file under the cursor.
    pub what: String,
    /// How many were selected; one is a name, more is a count.
    pub count: usize,
    /// The Unix mode of the file under the cursor, where there is one.
    pub mode: Option<u32>,
    pub flags: fcmd_core::EntryFlags,
    pub times: Times,
    /// Who it belongs to, as `user:group`.
    pub owner: Option<String>,
    /// Is a directory among them? Then the dialog offers to go down into them.
    pub directories: bool,
    /// Windows' own attribute word for the file under the cursor, read with the same `stat`
    /// as the times. Zero everywhere else, where there is no such word.
    pub win_attributes: u32,
    /// These are entries inside an archive. Every format here keeps a Unix mode and one
    /// time, whatever platform the program is running on, and keeps nothing else — so the
    /// dialog offers those and says what changing them costs.
    pub in_archive: bool,
}

/// The four times a file carries, as the attributes dialog shows them. Each is `None` where
/// the platform does not keep it: Linux has no creation time before ext4, and nothing but
/// Windows lets one be written.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Times {
    pub mtime: Option<std::time::SystemTime>,
    pub atime: Option<std::time::SystemTime>,
    pub btime: Option<std::time::SystemTime>,
    pub ctime: Option<std::time::SystemTime>,
}

/// Which of the three buttons under the times was pressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeFill {
    /// What the file carries now.
    Original,
    /// This moment.
    Current,
    /// Nothing, which is how a row says "leave this one alone".
    Blank,
}

impl Form {
    /// Build a dialog from the configuration as it stands.
    pub fn build(kind: FormKind, cfg: &Config, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = match kind {
            // The session editor is built by `Form::site`, the find dialog by `Form::find`,
            // the replace dialog by `Form::replace` and the two archive dialogs by
            // `Form::pack` and `Form::unpack`: each needs more than the configuration.
            FormKind::Site
            | FormKind::Find
            | FormKind::Replace
            | FormKind::Pack
            | FormKind::Unpack
            | FormKind::Attributes
            | FormKind::Copy
            | FormKind::Checksum
            | FormKind::Compare
            | FormKind::Drives
            | FormKind::Sync
            | FormKind::Link
            | FormKind::Highlight
            | FormKind::SortGroup
            | FormKind::Search => Vec::new(),
            FormKind::EditorSettings => vec![
                choice(
                    "editor.tabs",
                    t("opt-editor-tabs"),
                    &[
                        ("keep", t("opt-editor-tabs-keep")),
                        ("typed", t("opt-editor-tabs-typed")),
                        ("all", t("opt-editor-tabs-all")),
                    ],
                    &cfg.editor.tabs,
                ),
                choice(
                    "editor.tab_width",
                    t("opt-editor-tab-width"),
                    &[("2", "2".into()), ("4", "4".into()), ("8", "8".into())],
                    &cfg.editor.tab_width.to_string(),
                ),
                Field::toggle(
                    "editor.auto_indent",
                    t("opt-editor-auto-indent"),
                    cfg.editor.auto_indent,
                ),
                Field::Separator,
                Field::toggle(
                    "editor.show_whitespace",
                    t("opt-editor-whitespace"),
                    cfg.editor.show_whitespace,
                ),
                Field::toggle(
                    "editor.cursor_after_match",
                    t("opt-editor-cursor-after"),
                    cfg.editor.cursor_after_match,
                ),
                Field::Separator,
                // In the dialog for completeness: the setting does not exist yet, and a row
                // that pretended it did would be a row that lies.
                Field::Dim(t("opt-editor-beyond-eol")),
            ],
            FormKind::ViewerSettings => vec![
                // The three rows of chrome, in the order they are drawn: the top line, the
                // bar down the side, the key bar at the foot. Each is also a key inside the
                // viewer, and that key changes nothing here: the setting is what a file opens
                // with, the key is what this reader wants of the file in front of them.
                Field::toggle(
                    "viewer.status_line",
                    t("opt-viewer-status-line"),
                    cfg.viewer.status_line,
                ),
                Field::toggle(
                    "viewer.scrollbar",
                    t("opt-viewer-scrollbar"),
                    cfg.viewer.scrollbar,
                ),
                Field::toggle(
                    "viewer.key_bar",
                    t("opt-viewer-key-bar"),
                    cfg.viewer.key_bar,
                ),
                Field::Separator,
                choice(
                    "viewer.wrap",
                    t("opt-viewer-wrap"),
                    &[
                        ("off", t("opt-viewer-wrap-off")),
                        ("char", t("opt-viewer-wrap-char")),
                        ("word", t("opt-viewer-wrap-word")),
                    ],
                    cfg.viewer.wrap_kind(),
                ),
                choice(
                    "viewer.tab_width",
                    t("opt-viewer-tab-width"),
                    &[("2", "2".into()), ("4", "4".into()), ("8", "8".into())],
                    &cfg.viewer.tab_width.to_string(),
                ),
                Field::Separator,
                // In the dialog because FAR has it and somebody will look for it: remembering
                // a file's position wants the store the histories want, and a row that
                // pretended otherwise would be a row that lies.
                Field::Dim(t("opt-viewer-remember")),
            ],
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
                Field::toggle("general.use_trash", t("opt-trash"), cfg.general.use_trash),
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
                Field::toggle(
                    "panel.column_titles",
                    t("opt-column-titles"),
                    cfg.panel.column_titles,
                ),
                Field::toggle(
                    "panel.status_line",
                    t("opt-status-line"),
                    cfg.panel.status_line,
                ),
                Field::toggle("panel.scrollbar", t("opt-scrollbar"), cfg.panel.scrollbar),
                Field::toggle(
                    "panel.free_space",
                    t("opt-free-space"),
                    cfg.panel.free_space,
                ),
                Field::toggle(
                    "panel.auto_refresh",
                    t("opt-auto-refresh"),
                    cfg.panel.auto_refresh,
                ),
                Field::Separator,
                choice(
                    "panel.view",
                    t("opt-view"),
                    &[
                        // The same words as the menu rows, without the `&` that marks a
                        // menu row's key: a dialog row is not a menu row.
                        ("brief", plain(i18n, "menu-brief")),
                        ("full", plain(i18n, "menu-full")),
                        ("detail", plain(i18n, "menu-detailed")),
                        ("wide", plain(i18n, "menu-wide")),
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
                        fcmd_config::QuickSearch::Alt => "alt",
                        fcmd_config::QuickSearch::Plain => "plain",
                        fcmd_config::QuickSearch::Off => "off",
                    },
                ),
            ],
            FormKind::Interface => vec![
                Field::toggle("ui.key_bar", t("opt-key-bar"), cfg.ui.key_bar),
                Field::toggle("ui.status_bar", t("opt-status-bar"), cfg.ui.status_bar),
                Field::toggle("ui.syntax", t("opt-syntax"), cfg.ui.syntax),
                Field::toggle("ui.markdown", t("opt-markdown"), cfg.ui.markdown),
                Field::toggle("ui.table", t("opt-table"), cfg.ui.table),
                Field::Separator,
                choice(
                    "ui.keymap",
                    t("opt-keymap"),
                    &[("far", "far".into()), ("mc", "mc".into())],
                    &cfg.ui.keymap,
                ),
                Field::toggle("ui.mouse", t("opt-mouse"), cfg.ui.mouse),
                Field::toggle("ui.clipboard", t("opt-clipboard"), cfg.ui.clipboard),
                Field::toggle(
                    "ui.command_screen",
                    t("opt-command-screen"),
                    cfg.ui.command_screen,
                ),
                Field::Separator,
                Field::Dim(t("opt-clock")),
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
        // Five of these titles are menu labels, and a menu label carries the `&` that marks
        // its key: a dialog's frame is not a menu row, so the marker comes off here.
        let title = fcmd_ui::mnemonic(&i18n.t(match kind {
            FormKind::Site => "site-new",
            FormKind::Find => "find-title",
            FormKind::Replace => "replace-title",
            FormKind::Pack => "pack-title",
            FormKind::Unpack => "unpack-title",
            FormKind::Attributes => "attr-title",
            FormKind::Copy => "copy-title",
            FormKind::Checksum => "sum-title",
            FormKind::Compare => "cmp-title",
            FormKind::Drives => "drives-title",
            FormKind::Sync => "sync-title",
            FormKind::Search => "search-title",
            FormKind::Link => "link-title",
            FormKind::Highlight => "hl-new",
            FormKind::SortGroup => "sg-title",
            FormKind::EditorSettings => "menu-editor-settings",
            FormKind::ViewerSettings => "menu-viewer-settings",
            FormKind::System => "menu-system-settings",
            FormKind::Panel => "menu-panel-settings",
            FormKind::Interface => "menu-interface-settings",
            FormKind::Confirmations => "menu-confirmations",
        }))
        .0;
        let cursor = fields.iter().position(Field::selectable).unwrap_or(0);
        Form {
            pages: Vec::new(),
            page: 0,
            kind,
            title,
            fields,
            cursor,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// The session editor: a stored connection, or a new one when `site` is `None`
    /// (SPEC 10 §3.5).
    pub fn site(site: Option<(usize, &fcmd_config::SiteCfg)>, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let blank = fcmd_config::SiteCfg::default();
        let (index, cfg) = match site {
            Some((index, cfg)) => (Some(index), cfg),
            None => (None, &blank),
        };
        let auth = cfg.auth.clone().unwrap_or_else(|| "agent".to_string());
        let (auth_kind, key_path) = match auth.split_once(':') {
            Some(("key", path)) => ("key", path.to_string()),
            _ => (auth.as_str(), String::new()),
        };
        let scheme = match fcmd_core::Scheme::parse(&cfg.scheme) {
            Some(known) if known.is_remote() => known.as_str().to_string(),
            // A `[[site]]` with a scheme this build does not know is shown as what it is
            // closest to rather than silently turned into something else; the dialog's own
            // validation then says so when OK is pressed.
            _ => "sftp".to_string(),
        };
        // Four pages, in FAR's own order and under FAR's own names: the session itself, the
        // environment it runs in, the directories it starts in, and the page for the
        // protocol. The protocol page is the one that changes with the protocol, so the one
        // that does not apply is drawn grey — an SFTP key file on an FTP session is a
        // setting that would do nothing.
        let sftp = scheme == "sftp";
        let fields = vec![
            // Session
            Field::text("site.id", t("site-name"), cfg.id.clone()),
            choice(
                "site.scheme",
                t("site-protocol"),
                &[
                    ("sftp", t("site-protocol-sftp")),
                    ("ftp", t("site-protocol-ftp")),
                    ("ftps", t("site-protocol-ftps")),
                    ("dav", t("site-protocol-dav")),
                    ("davs", t("site-protocol-davs")),
                ],
                &scheme,
            ),
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
                // FTP has no agent and no keys: it reads `ask`, and a user called
                // `anonymous` or `ftp` logs in without being asked at all.
                &[
                    ("agent", t("site-auth-agent")),
                    ("key", t("site-auth-key")),
                    ("ask", t("site-auth-ask")),
                ],
                auth_kind,
            ),
            // No password row, and that is deliberate: a password in a configuration file is
            // a password in everybody's backups. `ask` is answered at the moment of
            // connecting and kept for as long as the connection is.
            Field::Note(t("site-password-note")),
            // Environment
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
            Field::Dim(t("site-env-eol")),
            Field::Dim(t("site-env-utf8")),
            // Directories
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
            // The protocol's own page.
            Field::text("site.key", t("site-key-file"), key_path),
            Field::Dim(t("site-sftp-ciphers")),
        ];
        let pages = vec![
            Page {
                label: t("site-page-session"),
                from: 0,
                to: 8,
                enabled: true,
            },
            Page {
                label: t("site-page-environment"),
                from: 8,
                to: 12,
                enabled: true,
            },
            Page {
                label: t("site-page-directories"),
                from: 12,
                to: 14,
                enabled: true,
            },
            Page {
                label: t("site-page-sftp"),
                from: 14,
                to: 16,
                enabled: sftp,
            },
        ];
        Form {
            pages,
            page: 0,
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
            // Three answers, as FAR's own session editor has: keep it, keep it and go there
            // now, or leave it alone.
            buttons: vec![
                i18n.t("btn-ok"),
                i18n.t("btn-connect"),
                i18n.t("btn-cancel"),
            ],
        }
    }

    /// One colour group (SPEC 10 §3.11), or a new one when `group` is `None`.
    ///
    /// Every row is the same word the configuration file takes, because it is the same
    /// condition set: somebody who learned the dialog can write the file, and the other way
    /// round. The two inks take a colour or the name of a theme colour, which is why they are
    /// text rows rather than a palette.
    /// One `[[sort_group]]` band: what to call it, what falls into it, and which side of the
    /// ungrouped files it sits on (SPEC 02 §4).
    pub fn sort_group(group: Option<&fcmd_config::SortGroupCfg>, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let blank = fcmd_config::SortGroupCfg::default();
        let g = group.unwrap_or(&blank);
        let fields = vec![
            Field::text("sg.name", t("sg-name"), g.name.clone()),
            Field::Separator,
            Field::text("sg.mask", t("sg-mask"), g.mask.clone()),
            choice(
                "sg.place",
                t("sg-place"),
                &[("front", t("sg-place-front")), ("back", t("sg-place-back"))],
                if g.place == "back" { "back" } else { "front" },
            ),
            Field::Separator,
            Field::Dim(t("sg-hint")),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::SortGroup,
            title: t("sg-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    pub fn highlight(group: Option<&fcmd_config::HighlightCfg>, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let blank = fcmd_config::HighlightCfg::default();
        let g = group.unwrap_or(&blank);
        let fields = vec![
            Field::text("hl.name", t("hl-name"), g.name.clone()),
            Field::Separator,
            Field::text("hl.mask", t("hl-mask"), g.mask.clone()),
            choice(
                "hl.kind",
                t("hl-kind"),
                &[
                    ("", t("hl-kind-both")),
                    ("file", t("hl-kind-file")),
                    ("dir", t("hl-kind-dir")),
                ],
                &g.kind,
            ),
            Field::text("hl.size", t("hl-size"), g.size.clone()),
            Field::text("hl.age", t("hl-age"), g.age.clone()),
            choice(
                "hl.date_field",
                t("hl-date-field"),
                &[
                    ("write", t("col-modified")),
                    ("create", t("col-created")),
                    ("access", t("col-accessed")),
                ],
                if g.date_field.is_empty() {
                    "write"
                } else {
                    &g.date_field
                },
            ),
            Field::text("hl.attrs", t("hl-attrs"), g.attrs.clone()),
            Field::Separator,
            Field::text("hl.fg", t("hl-fg"), g.fg.clone()),
            Field::text("hl.bg", t("hl-bg"), g.bg.clone()),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Highlight,
            title: t(if group.is_some() { "hl-edit" } else { "hl-new" }),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// Find file (SPEC 06 §1), filled in with what was searched for last — a search is very
    /// often the same one again with one thing changed.
    pub fn find(last: &crate::app::FindSpec, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = vec![
            Field::text("find.mask", t("find-mask"), last.mask.clone()),
            Field::text("find.size", t("find-size"), last.size.clone()),
            Field::text("find.text", t("find-text"), last.text.clone()),
            Field::toggle("find.case", t("find-case"), last.case_sensitive),
            Field::toggle("find.words", t("find-words"), last.whole_words),
            Field::toggle("find.hex", t("find-hex"), last.hex),
            Field::toggle("find.not", t("find-not"), last.not_containing),
            Field::Separator,
            choice(
                "find.scope",
                t("find-scope"),
                &[
                    ("tree", t("find-scope-tree")),
                    ("panel", t("find-scope-panel")),
                    ("selection", t("find-scope-selection")),
                    ("root", t("find-scope-root")),
                    ("other", t("find-scope-other")),
                ],
                last.scope.as_str(),
            ),
            Field::toggle("find.links", t("find-links"), last.follow_links),
            Field::toggle("find.dirs", t("find-dirs"), last.directories),
            // Off unless asked for: opening every container of a whole drive is a search of a
            // different size, and one nobody should start by accident.
            Field::toggle("find.archives", t("find-archives"), last.in_archives),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Find,
            title: t("find-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// Whether a toggle row is on, by key. `false` when there is no such row, which is what a
    /// dialog that does not offer the option should mean.
    /// Is the cursor on the row with this key?
    pub fn on_key(&self, want: &str) -> bool {
        matches!(
            self.fields.get(self.cursor),
            Some(Field::Toggle { key, .. } | Field::Text { key, .. } | Field::Choice { key, .. })
                if *key == want
        )
    }

    pub fn toggle_value(&self, want: &str) -> bool {
        self.fields
            .iter()
            .any(|f| matches!(f, Field::Toggle { key, on, .. } if *key == want && *on))
    }

    /// The value a choice row is on, by key.
    pub fn choice_value(&self, want: &str) -> String {
        for field in &self.fields {
            if let Field::Choice {
                key,
                options,
                chosen,
                ..
            } = field
            {
                if *key == want {
                    return options
                        .get(*chosen)
                        .map(|(v, _)| v.clone())
                        .unwrap_or_default();
                }
            }
        }
        String::new()
    }

    /// Replace across files. `files` is how many are selected, which the dialog says out loud
    /// because the one thing a user must not get wrong here is *what* it will rewrite.
    pub fn replace(last: &fcmd_jobs::ReplaceOptions, files: usize, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = vec![
            Field::text("replace.needle", t("replace-needle"), last.needle.clone()),
            Field::text("replace.with", t("replace-with"), last.replacement.clone()),
            Field::toggle("replace.case", t("replace-case"), last.case_sensitive),
            Field::toggle("replace.words", t("replace-words"), last.whole_words),
            Field::toggle("replace.pattern", t("replace-pattern"), last.pattern),
            Field::toggle("replace.hex", t("replace-hex"), last.hex),
            Field::Separator,
            Field::Dim(i18n.t_args("replace-scope", &[("count", (files as i64).into())])),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Replace,
            title: t("replace-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// `Shift+F1`: the archive to build, out of what is selected.
    pub fn pack(
        name: String,
        format: fcmd_arc::Format,
        solid: bool,
        files: usize,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let formats: Vec<(&str, String)> = fcmd_arc::Format::ALL
            .iter()
            .map(|f| (f.extension(), f.label().to_string()))
            .collect();
        let levels = [
            ("normal", t("pack-level-normal")),
            ("fast", t("pack-level-fast")),
            ("best", t("pack-level-best")),
        ];
        let fields = vec![
            Field::text("pack.name", t("pack-name"), name),
            choice(
                "pack.format",
                t("pack-format"),
                &formats,
                format.extension(),
            ),
            choice("pack.level", t("pack-level"), &levels, "normal"),
            // 7-Zip's *solid*, and only a 7z does anything with it: the files share one
            // compressed stream, which is smaller and makes reading one of them decode
            // everything in front of it. A tar is solid anyway and a zip cannot be.
            Field::toggle("pack.solid", t("pack-solid"), solid),
            // FAR's `Delete files after archiving`. Off, and it stays off between dialogs:
            // an archiver that quietly removes the originals is a bad surprise once.
            Field::toggle("pack.delete", t("pack-delete-after"), false),
            Field::Separator,
            Field::Dim(i18n.t_args("pack-scope", &[("count", (files as i64).into())])),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Pack,
            title: t("pack-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// `F5` and `F6`: where it goes, and under what rules.
    ///
    /// It replaces a one-line prompt, and every row below the first is something the engine
    /// could already do and the prompt had no way to ask for. The defaults are the prompt's
    /// behaviour exactly, so somebody who types a path and presses Enter gets what they got
    /// before and need not read the rest.
    #[allow(clippy::too_many_arguments)]
    pub fn copy(
        target: String,
        moving: bool,
        what: String,
        conflict: &str,
        only_newer: bool,
        follow_links: bool,
        filter: &str,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let policies = [
            ("ask", t("copy-conflict-ask")),
            ("overwrite", t("copy-conflict-overwrite")),
            ("newer", t("copy-conflict-newer")),
            ("skip", t("copy-conflict-skip")),
            ("rename", t("copy-conflict-rename")),
            ("append", t("copy-conflict-append")),
        ];
        let fields = vec![
            Field::Note(what),
            Field::text("copy.target", t("copy-target"), target),
            Field::Separator,
            choice("copy.conflict", t("copy-conflict"), &policies, conflict),
            // FAR has this as a box of its own as well as a policy, and the two are not the
            // same: the box holds even when the policy says to overwrite, or when the user
            // has already answered "overwrite all" to a question.
            Field::toggle("copy.only_newer", t("copy-only-newer"), only_newer),
            Field::Separator,
            Field::text("copy.filter", t("copy-filter"), filter.to_string()),
            Field::Separator,
            // Off, and FAR's default too: a copy that silently turned links into copies of
            // what they point at would duplicate a tree the first time somebody tried it.
            Field::toggle("copy.follow_links", t("copy-follow-links"), follow_links),
            Field::toggle("copy.times", t("copy-keep-times"), true),
            Field::toggle("copy.mode", t("copy-keep-mode"), true),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Copy,
            title: t(if moving {
                "copy-title-move"
            } else {
                "copy-title"
            }),
            fields,
            cursor: 1,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// Which way to synchronise two directories, and what counts as the same (SPEC 03 §10).
    ///
    /// Every row here changes what the *table* will say, not what will happen: the table is
    /// the question, and this is only how it is worked out. So the defaults are the two that
    /// cannot lose anything — walk the trees, leave extras alone — and the two that can are
    /// off until somebody ticks them.
    pub fn sync(
        left: String,
        right: String,
        options: &fcmd_jobs::sync::Options,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = vec![
            Field::Note(i18n.t_args(
                "sync-between",
                &[("left", left.into()), ("right", right.into())],
            )),
            Field::toggle("sync.recursive", t("sync-recursive"), options.recursive),
            Field::Separator,
            // Off: one way, the left panel being the source, which is what a backup is and
            // what somebody who has not thought about it expects.
            Field::toggle("sync.both_ways", t("sync-both-ways"), options.both_ways),
            Field::toggle(
                "sync.delete_extra",
                t("sync-delete-extra"),
                options.delete_extra,
            ),
            Field::Separator,
            // Off: the size and the time answer it for almost nothing, and reading every byte
            // of both trees to be sure is a choice somebody should make deliberately.
            Field::toggle("sync.by_content", t("sync-by-content"), options.by_content),
            Field::text(
                "sync.filter",
                t("sync-filter"),
                options
                    .filter
                    .as_ref()
                    .map(|f| f.source().to_string())
                    .unwrap_or_default(),
            ),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Sync,
            title: t("sync-title"),
            fields,
            cursor: 1,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// What to hash, with what, and where the list goes (SPEC 03 §9).
    ///
    /// The name is offered rather than fixed because the two conventions disagree and both are
    /// right: a commander writes `project.sha256` beside the directory it is about, and a
    /// Linux release writes `SHA256SUMS`. Either is read by `sha256sum -c`.
    pub fn checksum(
        what: String,
        algorithm: fcmd_jobs::Algorithm,
        name: String,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let algorithms = [
            ("sha256", "SHA-256".to_string()),
            ("sha1", "SHA-1".to_string()),
            ("md5", "MD5".to_string()),
        ];
        let fields = vec![
            Field::Note(what),
            choice(
                "sum.algorithm",
                t("sum-algorithm"),
                &algorithms,
                algorithm.as_str(),
            ),
            Field::Separator,
            // On: one list for the lot is what `sha256sum * > SHA256SUMS` gives and what the
            // tool that checks it expects to be handed.
            Field::toggle("sum.one_file", t("sum-one-file"), true),
            Field::text("sum.name", t("sum-name"), name),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Checksum,
            title: t("sum-title"),
            fields,
            cursor: 1,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// What counts as the same file, before the two panels are compared (SPEC 03 §11).
    ///
    /// FAR has this as a plugin with a dialog of its own, and the dialog is the point: the
    /// answer to "are these two folders the same" depends entirely on what one means by the
    /// same, and that changes with the job. A tree copied by something that dropped the
    /// timestamps has to be compared by its bytes; two checkouts of one repository differ in
    /// every line ending and in nothing else; a memory stick keeps the time to the nearest
    /// two seconds. Every row here is one of those.
    pub fn compare(cfg: &fcmd_config::CompareCfg, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = vec![
            Field::toggle("compare.case", t("cmp-case"), cfg.case),
            Field::Separator,
            Field::toggle("compare.size", t("cmp-size"), cfg.size),
            Field::toggle("compare.time", t("cmp-time"), cfg.time),
            Field::text("compare.seconds", t("cmp-seconds"), cfg.seconds.to_string()),
            Field::Separator,
            Field::toggle("compare.contents", t("cmp-contents"), cfg.contents),
            Field::toggle(
                "compare.when_time_differs",
                t("cmp-when-time"),
                cfg.when_time_differs,
            ),
            Field::text(
                "compare.fragment_kb",
                t("cmp-fragment"),
                cfg.fragment_kb.to_string(),
            ),
            Field::toggle("compare.ignore_eol", t("cmp-ignore-eol"), cfg.ignore_eol),
            Field::Separator,
            Field::toggle("compare.subdirs", t("cmp-subdirs"), cfg.subdirs),
            Field::text("compare.depth", t("cmp-depth"), cfg.depth.to_string()),
            Field::Separator,
            Field::toggle(
                "compare.selected_only",
                t("cmp-selected"),
                cfg.selected_only,
            ),
            Field::toggle("compare.stop_on_first", t("cmp-stop"), cfg.stop_on_first),
            Field::toggle("compare.newer_only", t("cmp-newer"), cfg.newer_only),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Compare,
            title: t("cmp-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// `F9` inside the change-drive dialog: the same switches its `Ctrl`-and-a-digit keys
    /// work, as a dialog of boxes — which is what FAR's own `F9` there opens.
    ///
    /// The keys are faster for somebody who knows them and this is how anybody finds out
    /// they exist, so each row says which key it is.
    pub fn drives(cfg: &fcmd_config::DrivesCfg, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        // The digit each row answers to, in front of its label: the dialog is also the list
        // of the keys.
        let row = |key: &'static str, digit: char, id: &str, on: bool| -> Field {
            Field::toggle(key, format!("Ctrl+{digit}  {}", t(id)), on)
        };
        let fields = vec![
            row("drives.show_type", '1', "drives-col-type", cfg.show_type),
            row(
                "drives.show_device",
                '2',
                "drives-col-device",
                cfg.show_device,
            ),
            row("drives.show_label", '3', "drives-col-label", cfg.show_label),
            row("drives.show_fs", '4', "drives-col-fs", cfg.show_fs),
            row("drives.show_size", '5', "drives-col-size", cfg.show_size),
            Field::Separator,
            row(
                "drives.show_removable",
                '6',
                "drives-col-removable",
                cfg.show_removable,
            ),
            row(
                "drives.show_panels",
                '7',
                "drives-col-panels",
                cfg.show_panels,
            ),
            row(
                "drives.show_optical",
                '8',
                "drives-col-optical",
                cfg.show_optical,
            ),
            row(
                "drives.show_network",
                '9',
                "drives-col-network",
                cfg.show_network,
            ),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Drives,
            title: t("drives-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// `Alt+F6`: a link to the file under the cursor.
    ///
    /// Three kinds, and which one to use is a real choice rather than a detail. A **relative**
    /// symbolic link survives the whole tree being moved, copied or mounted somewhere else,
    /// because it says "the file next to me"; an **absolute** one survives the link alone
    /// being moved to another directory, because it says where the file is; a **hard** link is
    /// not a pointer at all but a second name for the same bytes, which survives the original
    /// name being deleted and cannot cross a filesystem or name a directory.
    pub fn link(what: String, name: String, kind: fcmd_jobs::LinkKind, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let kinds = [
            ("symlink-relative", t("link-kind-relative")),
            ("symlink-absolute", t("link-kind-absolute")),
            ("hard", t("link-kind-hard")),
        ];
        let fields = vec![
            Field::Note(what),
            Field::text("link.name", t("link-name"), name),
            Field::Separator,
            choice("link.kind", t("link-kind"), &kinds, kind.as_str()),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Link,
            title: t("link-title"),
            fields,
            // On the name: it is the one field anybody edits, and the kind is remembered from
            // the last time.
            cursor: 1,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// `Ctrl+A`: what a file *is*, rather than what is in it.
    ///
    /// The rows are what the platform has. Unix has a mode and two timestamps; Windows has
    /// four attribute bits and the same timestamps; a file on a server has a mode and a
    /// modification time. They overlap in one place — read-only, which on Unix is the write
    /// bits — so that is offered everywhere and the rest where it exists.
    ///
    /// A row left alone changes nothing. That is why the mode's boxes start from what the
    /// file has and the times start empty: a blank time field means "leave it", and a time
    /// typed in means that time, for every path the dialog was opened on.
    pub fn attributes(facts: &AttrFacts, i18n: &I18n) -> Form {
        let AttrFacts {
            what,
            count,
            mode,
            flags,
            times,
            owner,
            directories,
            win_attributes,
            in_archive,
        } = facts;
        let (count, mode, flags, directories) = (*count, *mode, *flags, *directories);
        let in_archive = *in_archive;
        let win_attributes = *win_attributes;
        let owner = owner.as_deref();
        let t = |id: &str| i18n.t(id);
        let bits = fcmd_jobs::Mode::of(mode.unwrap_or(0o644));
        let mut fields = vec![Field::Note(if count == 1 {
            what.to_string()
        } else {
            i18n.t_args("attr-count", &[("count", (count as i64).into())])
        })];
        // Whose it is. A row rather than a box: a dialog that offered to change the owner
        // would have to ask for a password it has no business taking.
        if let Some(owner) = owner {
            fields.push(Field::Note(
                i18n.t_args("attr-owner-is", &[("who", owner.to_string().into())]),
            ));
        }
        if let Some(mode) = mode {
            fields.push(Field::Note(i18n.t_args(
                "attr-now",
                &[
                    ("text", fcmd_jobs::Mode::of(mode).text().into()),
                    ("octal", format!("{:04o}", mode & 0o7777).into()),
                ],
            )));
        }
        fields.push(Field::Separator);

        // Unix's twelve boxes. On Windows there is no mode to set, so they are not offered:
        // a dialog that showed them would be asking about something nothing would read.
        // Inside an archive they are offered everywhere, because that is what the formats
        // hold: a zip, a tar and a 7z all record a Unix mode and nothing of Windows'.
        if cfg!(unix) || in_archive {
            for (key, label, on) in [
                ("attr.ur", "attr-owner-read", bits.owner_read),
                ("attr.uw", "attr-owner-write", bits.owner_write),
                ("attr.ux", "attr-owner-exec", bits.owner_exec),
                ("attr.gr", "attr-group-read", bits.group_read),
                ("attr.gw", "attr-group-write", bits.group_write),
                ("attr.gx", "attr-group-exec", bits.group_exec),
                ("attr.or", "attr-other-read", bits.other_read),
                ("attr.ow", "attr-other-write", bits.other_write),
                ("attr.ox", "attr-other-exec", bits.other_exec),
            ] {
                fields.push(Field::toggle(key, t(label), on));
            }
            fields.push(Field::Separator);
            for (key, label, on) in [
                ("attr.setuid", "attr-setuid", bits.setuid),
                ("attr.setgid", "attr-setgid", bits.setgid),
                ("attr.sticky", "attr-sticky", bits.sticky),
            ] {
                fields.push(Field::toggle(key, t(label), on));
            }
            fields.push(Field::Separator);
        }

        // Read-only is the one row both platforms have, and the only one a server has beyond
        // the mode. An archive entry has no such bit of its own — its mode is the whole of
        // what it carries — so inside one the row is not there to be written by mistake.
        if !in_archive {
            fields.push(Field::toggle(
                "attr.readonly",
                t("attr-readonly"),
                flags.contains(fcmd_core::EntryFlags::READONLY),
            ));
        }
        if cfg!(windows) && !in_archive {
            // Seeded from what the file carries, not from `false`. A box that opened empty on
            // a hidden file would not merely look wrong: `OK` writes every box, so it would
            // take the bit away from a file nobody meant to change.
            for (key, label, on) in [
                (
                    "attr.hidden",
                    "attr-hidden",
                    flags.contains(fcmd_core::EntryFlags::HIDDEN),
                ),
                (
                    "attr.system",
                    "attr-system",
                    flags.contains(fcmd_core::EntryFlags::SYSTEM),
                ),
                (
                    "attr.archive",
                    "attr-archive",
                    flags.contains(fcmd_core::EntryFlags::ARCHIVE),
                ),
            ] {
                fields.push(Field::toggle(key, t(label), on));
            }
            // The three more that can be written. They are not in the listing's own flags —
            // no column shows them — so they come off the attribute word read beside the
            // times, and a box starts where the file is rather than at `false`.
            const TEMPORARY: u32 = 0x100;
            const OFFLINE: u32 = 0x1000;
            const NOT_INDEXED: u32 = 0x2000;
            fields.push(Field::toggle(
                "attr.temporary",
                t("attr-temporary"),
                win_attributes & TEMPORARY != 0,
            ));
            fields.push(Field::toggle(
                "attr.offline",
                t("attr-offline"),
                win_attributes & OFFLINE != 0,
            ));
            // Asked the way round a reader thinks of it, which is the opposite of the bit.
            fields.push(Field::toggle(
                "attr.indexed",
                t("attr-indexed"),
                win_attributes & NOT_INDEXED == 0,
            ));
            // And the ones that are set by doing the thing rather than by writing a bit:
            // shown, because they answer "why is this file green", and never offered.
            const COMPRESSED: u32 = 0x800;
            const ENCRYPTED: u32 = 0x4000;
            const SPARSE: u32 = 0x200;
            const REPARSE: u32 = 0x400;
            let also: Vec<String> = [
                (COMPRESSED, "attr-compressed"),
                (ENCRYPTED, "attr-encrypted"),
                (SPARSE, "attr-sparse"),
                (REPARSE, "attr-reparse"),
            ]
            .iter()
            .filter(|(bit, _)| win_attributes & bit != 0)
            .map(|(_, id)| t(id))
            .collect();
            if !also.is_empty() {
                fields.push(Field::Dim(
                    i18n.t_args("attr-also", &[("what", also.join(", ").into())]),
                ));
            }
        }
        fields.push(Field::Separator);

        // The clock the four rows below are read and written in. The file's times are kept in
        // UTC whatever this says; the box only decides which clock they are shown in, which
        // is the question anybody comparing a file with a build server's copy of it has.
        fields.push(Field::toggle("attr.utc", t("attr-utc"), false));
        // The four times FAR shows. Empty means leave that one alone; each starts at what the
        // file carries, so the format is in front of the reader rather than in the manual.
        // An archive keeps one time per entry, so inside one there is one row.
        let rows: &[(&str, &str, Option<std::time::SystemTime>)] = if in_archive {
            &[("attr.mtime", "attr-mtime", times.mtime)]
        } else {
            &[
                ("attr.mtime", "attr-mtime", times.mtime),
                ("attr.atime", "attr-atime", times.atime),
                ("attr.btime", "attr-btime", times.btime),
            ]
        };
        for (key, label, when) in rows.iter().copied() {
            fields.push(Field::text(
                key,
                t(label),
                when.map(fcmd_ui::format_stamp).unwrap_or_default(),
            ));
        }
        // The fourth is read-only everywhere: no system has a call that sets the moment a
        // file's own record last changed — it is set *by* changing it.
        if let Some(ctime) = times.ctime {
            fields.push(Field::Dim(format!(
                "{}  {}",
                t("attr-ctime"),
                fcmd_ui::format_stamp(ctime)
            )));
        }
        // And on a platform that cannot set the creation time, the row says so rather than
        // taking a time and dropping it.
        if !cfg!(windows) && !in_archive {
            fields.push(Field::Dim(t("attr-btime-readonly")));
        }
        // What `OK` costs in there, said before it is pressed.
        if in_archive {
            fields.push(Field::Dim(t("attr-in-archive")));
        }
        if directories && !in_archive {
            fields.push(Field::Separator);
            fields.push(Field::toggle("attr.recursive", t("attr-recursive"), false));
        }

        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Attributes,
            title: t("attr-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            // Three buttons that do not answer the dialog: they fill the time rows in, the
            // way FAR's own do, and the dialog stays on screen for the next one. They come
            // first so that `OK` and `Cancel` keep their places at the end.
            buttons: vec![
                i18n.t("btn-attr-original"),
                i18n.t("btn-attr-current"),
                i18n.t("btn-attr-blank"),
                i18n.t("btn-ok"),
                i18n.t("btn-cancel"),
            ],
        }
    }

    /// Fill the three time rows from one of the buttons: what the file carries, now, or
    /// nothing at all. `utc` is the clock the rows are written in.
    pub fn fill_times(&mut self, which: TimeFill, times: &Times, utc: bool) {
        let now = std::time::SystemTime::now();
        for (key, when) in [
            ("attr.mtime", times.mtime),
            ("attr.atime", times.atime),
            ("attr.btime", times.btime),
        ] {
            let text = match which {
                TimeFill::Original => when
                    .map(|t| fcmd_ui::format_stamp_in(t, utc))
                    .unwrap_or_default(),
                TimeFill::Current => fcmd_ui::format_stamp_in(now, utc),
                TimeFill::Blank => String::new(),
            };
            self.set_text(key, text);
        }
    }

    /// `F7`: what to look for, and how. FAR asks all of it in one dialog rather than
    /// hiding the how behind keys you have to know, and it offers both directions as
    /// buttons, because *next* and *previous* are the same question asked twice.
    pub fn search(
        what: &str,
        mode: &str,
        case_sensitive: bool,
        whole_words: bool,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let modes = [
            ("text", t("search-mode-text")),
            ("hex", t("search-mode-hex")),
            ("regex", t("search-mode-regex")),
        ];
        let fields = vec![
            Field::text("search.what", t("search-for"), what.to_string()),
            choice("search.mode", t("search-mode"), &modes, mode),
            Field::toggle("search.case", t("search-case"), case_sensitive),
            Field::toggle("search.words", t("search-words"), whole_words),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Search,
            title: t("search-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![t("btn-find-next"), t("btn-find-prev"), t("btn-cancel")],
        }
    }

    /// `Shift+F2`: where to put what is inside an archive.
    pub fn unpack(to: String, archive: String, i18n: &I18n) -> Form {
        let t = |id: &str| i18n.t(id);
        let fields = vec![
            Field::text("unpack.to", t("unpack-to"), to),
            Field::Separator,
            Field::Dim(i18n.t_args("unpack-scope", &[("name", archive.into())])),
        ];
        Form {
            pages: Vec::new(),
            page: 0,
            kind: FormKind::Unpack,
            title: t("unpack-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
        }
    }

    /// What a text row holds now, by key.
    /// Write a text row's value, leaving the caret at the end of it.
    pub fn set_text(&mut self, want: &str, text: String) {
        for field in &mut self.fields {
            if let Field::Text {
                key, value, caret, ..
            } = field
            {
                if *key == want {
                    *caret = text.chars().count();
                    *value = text;
                    return;
                }
            }
        }
    }

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

    /// Pasted text into the row under the cursor. A dialog row is one line, so only the first
    /// line of the paste goes in — a path copied from somewhere with its newline still on it
    /// is the common case, and it should land as the path, not refuse to.
    ///
    /// Returns what was dropped, so the caller can say that there was more.
    pub fn insert_str(&mut self, text: &str) -> usize {
        let Some(Field::Text { value, caret, .. }) = self.fields.get_mut(self.cursor) else {
            return 0;
        };
        let first = text.lines().next().unwrap_or_default();
        let dropped = text
            .lines()
            .skip(1)
            .filter(|l| !l.trim().is_empty())
            .count();
        if !first.is_empty() {
            let at = value
                .char_indices()
                .nth(*caret)
                .map(|(i, _)| i)
                .unwrap_or(value.len());
            value.insert_str(at, first);
            *caret += first.chars().count();
        }
        dropped
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
                    // Among the rows drawn, which on a dialog with pages is not where the
                    // field sits in the list of all of them.
                    self.cursor_row(),
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
    /// The pages along the top, for the renderer. Empty for a dialog that is one page.
    pub fn tabs(&self) -> Vec<fcmd_ui::widgets::FormTab> {
        self.pages
            .iter()
            .map(|p| fcmd_ui::widgets::FormTab {
                label: p.label.clone(),
                enabled: p.enabled,
            })
            .collect()
    }

    /// Which fields are on screen: the page's, or all of them for a one-page dialog.
    pub fn visible(&self) -> std::ops::Range<usize> {
        match self.pages.get(self.page) {
            Some(page) => page.from..page.to.min(self.fields.len()),
            None => 0..self.fields.len(),
        }
    }

    /// Where the cursor is among the rows that are drawn, which is not where it is among the
    /// fields once a dialog has pages.
    pub fn cursor_row(&self) -> usize {
        self.cursor.saturating_sub(self.visible().start)
    }

    /// Turn to the next page that has anything on it, or the one before. The cursor goes to
    /// the first field of the page it lands on, because a cursor left behind on another page
    /// is a cursor nobody can see.
    pub fn turn_page(&mut self, forward: bool) {
        if self.pages.len() < 2 {
            return;
        }
        let n = self.pages.len();
        let mut at = self.page;
        for _ in 0..n {
            at = if forward {
                (at + 1) % n
            } else {
                (at + n - 1) % n
            };
            if self.pages[at].enabled {
                self.page = at;
                self.button = None;
                let range = self.visible();
                self.cursor = range.start;
                if !self.fields[self.cursor].selectable() {
                    self.move_cursor(true);
                }
                return;
            }
        }
    }

    pub fn rows(&self) -> Vec<FormRow> {
        let range = self.visible();
        self.fields[range]
            .iter()
            .map(|f| match f {
                Field::Separator => FormRow::separator(),
                Field::Dim(label) => FormRow {
                    text: format!("[ ] {label}"),
                    dim: true,
                    ..FormRow::default()
                },
                Field::Note(text) => FormRow {
                    text: text.clone(),
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

    /// The cursor walks the page it is on and wraps inside it: a dialog with pages is read
    /// one page at a time, and an arrow that slid onto the next page would move the cursor
    /// somewhere nobody can see.
    pub fn move_cursor(&mut self, down: bool) {
        let range = self.visible();
        let len = range.len();
        if len == 0 {
            return;
        }
        let mut i = self.cursor.max(range.start) - range.start;
        for _ in 0..len {
            i = if down {
                (i + 1) % len
            } else {
                (i + len - 1) % len
            };
            if self.fields[range.start + i].selectable() {
                self.cursor = range.start + i;
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
    /// A key that jumps straight to this row and takes it, shown at the start of the line.
    /// FAR numbers the plugin panels in the drive dialog this way, and the number is how
    /// anyone who uses it twice a day actually picks one.
    pub hotkey: Option<char>,
    /// A rule across the list rather than a row to choose: what FAR draws in the drive
    /// dialog between the drives and the panels that are not drives. The cursor steps over
    /// it and nothing happens if a click lands on it.
    pub separator: bool,
}

impl PickRow {
    /// A row with no key of its own.
    pub fn plain(label: impl Into<String>, value: impl Into<String>, current: bool) -> PickRow {
        PickRow {
            label: label.into(),
            value: value.into(),
            current,
            hotkey: None,
            separator: false,
        }
    }

    /// The rule between two groups of rows.
    pub fn rule() -> PickRow {
        PickRow {
            label: String::new(),
            value: String::new(),
            current: false,
            hotkey: None,
            separator: true,
        }
    }

    pub fn with_key(mut self, key: char) -> PickRow {
        self.hotkey = Some(key);
        self
    }
}

impl Pick {
    pub fn new(kind: PickKind, title: String, rows: Vec<PickRow>, side: usize) -> Pick {
        let cursor = rows
            .iter()
            .position(|r| r.current)
            .or_else(|| rows.iter().position(|r| !r.separator))
            .unwrap_or(0);
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
        // A rule is not a row: the cursor carries on the way it was going. One step is
        // enough — two rules never sit together.
        if self.rows[self.cursor].separator {
            let step = if delta < 0 { -1 } else { 1 };
            self.cursor = (self.cursor as isize + step).rem_euclid(len) as usize;
        }
    }

    pub fn chosen(&self) -> Option<&PickRow> {
        self.rows.get(self.cursor).filter(|r| !r.separator)
    }

    pub fn lines(&self) -> Vec<String> {
        // The hotkey column exists only if some row has one, so every other list looks
        // exactly as it did.
        let keyed = self.rows.iter().any(|r| r.hotkey.is_some());
        self.rows
            .iter()
            .map(|r| {
                if r.separator {
                    // Left blank here and drawn by the renderer, which is the only one that
                    // knows where the frame is: a rule that stops two columns short of it
                    // is a rule that looks like a mistake.
                    return String::new();
                }
                let mark = if r.current { '√' } else { ' ' };
                match (keyed, r.hotkey) {
                    (true, Some(k)) => format!("{mark} {k}  {}", r.label),
                    (true, None) => format!("{mark}    {}", r.label),
                    _ => format!("{mark} {}", r.label),
                }
            })
            .collect()
    }

    /// The row a typed character goes to, if any.
    pub fn by_hotkey(&self, typed: char) -> Option<usize> {
        let typed = typed.to_ascii_lowercase();
        self.rows
            .iter()
            .position(|r| r.hotkey.map(|k| k.to_ascii_lowercase()) == Some(typed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::FormKind;

    /// Every row of a settings dialog has to name a setting the configuration knows about.
    /// A row that names one it does not looks like it works and then reports "unknown
    /// setting" when the dialog is accepted — which is what `ui.mouse` did until this test
    /// was written. The five dialogs built from the configuration are all of them; the rest
    /// are built by hand from something other than the configuration.
    #[test]
    fn every_settings_row_names_a_setting_the_configuration_knows() {
        let i18n = I18n::new("en");
        let base = Config::default();
        for kind in [
            FormKind::System,
            FormKind::Panel,
            FormKind::Interface,
            FormKind::Confirmations,
            FormKind::EditorSettings,
            FormKind::ViewerSettings,
        ] {
            let form = Form::build(kind, &base, &i18n);
            assert!(!form.fields.is_empty(), "{kind:?} has no rows");
            for field in &form.fields {
                let (key, value) = match field {
                    Field::Toggle { key, on, .. } => (*key, if *on { "true" } else { "false" }),
                    Field::Choice {
                        key,
                        options,
                        chosen,
                        ..
                    } => (
                        *key,
                        options
                            .get(*chosen)
                            .map(|(v, _)| v.as_str())
                            .unwrap_or_default(),
                    ),
                    Field::Text { key, value, .. } => (*key, value.as_str()),
                    _ => continue,
                };
                let mut cfg = base.clone();
                assert_eq!(cfg.set(key, value), Ok(()), "{kind:?} row `{key}`");
            }
        }
    }
}
