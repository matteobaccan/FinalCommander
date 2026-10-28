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

/// A settings dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct Form {
    pub kind: FormKind,
    pub title: String,
    pub fields: Vec<Field>,
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
            | FormKind::Sync
            | FormKind::Link
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
            FormKind::Sync => "sync-title",
            FormKind::Search => "search-title",
            FormKind::Link => "link-title",
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
        let fields = vec![
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
            Field::Separator,
            Field::Dim(t("find-archives")),
        ];
        Form {
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
    pub fn pack(name: String, format: fcmd_arc::Format, files: usize, i18n: &I18n) -> Form {
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
            // FAR's `Delete files after archiving`. Off, and it stays off between dialogs:
            // an archiver that quietly removes the originals is a bad surprise once.
            Field::toggle("pack.delete", t("pack-delete-after"), false),
            Field::Separator,
            Field::Dim(i18n.t_args("pack-scope", &[("count", (files as i64).into())])),
        ];
        Form {
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
            kind: FormKind::Checksum,
            title: t("sum-title"),
            fields,
            cursor: 1,
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
    pub fn attributes(
        what: &str,
        count: usize,
        mode: Option<u32>,
        flags: fcmd_core::EntryFlags,
        mtime: Option<std::time::SystemTime>,
        directories: bool,
        i18n: &I18n,
    ) -> Form {
        let t = |id: &str| i18n.t(id);
        let bits = fcmd_jobs::Mode::of(mode.unwrap_or(0o644));
        let mut fields = vec![Field::Note(if count == 1 {
            what.to_string()
        } else {
            i18n.t_args("attr-count", &[("count", (count as i64).into())])
        })];
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
        if cfg!(unix) {
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
        // the mode.
        fields.push(Field::toggle(
            "attr.readonly",
            t("attr-readonly"),
            flags.contains(fcmd_core::EntryFlags::READONLY),
        ));
        if cfg!(windows) {
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
        }
        fields.push(Field::Separator);

        // The times. Empty means leave them; `now` is what the keyboard reaches for most, so
        // the current value is offered rather than a format to copy from the manual.
        fields.push(Field::text(
            "attr.mtime",
            t("attr-mtime"),
            mtime.map(fcmd_ui::format_stamp).unwrap_or_default(),
        ));
        fields.push(Field::text("attr.atime", t("attr-atime"), String::new()));
        if directories {
            fields.push(Field::Separator);
            fields.push(Field::toggle("attr.recursive", t("attr-recursive"), false));
        }

        Form {
            kind: FormKind::Attributes,
            title: t("attr-title"),
            fields,
            cursor: 0,
            site: None,
            button: None,
            buttons: vec![i18n.t("btn-ok"), i18n.t("btn-cancel")],
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
    /// A key that jumps straight to this row and takes it, shown at the start of the line.
    /// FAR numbers the plugin panels in the drive dialog this way, and the number is how
    /// anyone who uses it twice a day actually picks one.
    pub hotkey: Option<char>,
}

impl PickRow {
    /// A row with no key of its own.
    pub fn plain(label: impl Into<String>, value: impl Into<String>, current: bool) -> PickRow {
        PickRow {
            label: label.into(),
            value: value.into(),
            current,
            hotkey: None,
        }
    }

    pub fn with_key(mut self, key: char) -> PickRow {
        self.hotkey = Some(key);
        self
    }
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
        // The hotkey column exists only if some row has one, so every other list looks
        // exactly as it did.
        let keyed = self.rows.iter().any(|r| r.hotkey.is_some());
        self.rows
            .iter()
            .map(|r| {
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
