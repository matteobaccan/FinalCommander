use sc_cmd::{Command, Key, KeyCode, Keymap, Mods, Scope};
use sc_config::{Config, Paths, QuickSearch};
use sc_core::{Entry, MaskScope, Sort, SortKey, VPath, ViewMode};
use sc_i18n::I18n;
use sc_tui::Input;
use sc_ui::{Theme, ThemeSet};

use crate::panel::Panel;

/// Work the application wants done off the UI thread. The runner drains these.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    Read {
        panel: usize,
        path: VPath,
        generation: u64,
    },
    DirSize {
        panel: usize,
        path: VPath,
    },
}

/// What comes back.
#[derive(Debug)]
pub enum Event {
    Input(Input),
    Listed {
        panel: usize,
        generation: u64,
        result: Result<Vec<Entry>, String>,
    },
    DirSized {
        panel: usize,
        path: VPath,
        bytes: u64,
        files: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Goto,
    SelectMask,
    DeselectMask,
    Filter,
    /// Quick search; remembers where the cursor was so Esc can put it back.
    QuickSearch {
        origin: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Prompt(Prompt),
    Help { scroll: usize },
    Message { title: String, lines: Vec<String> },
}

/// Everything the application is started with. Grouping it keeps `App::new` honest and makes
/// the test setup read like the real startup path.
pub struct Startup {
    pub cfg: Config,
    pub paths: Paths,
    pub i18n: I18n,
    pub keymap: Keymap,
    pub themes: ThemeSet,
    pub theme: Theme,
    pub left: VPath,
    pub right: VPath,
}

pub struct App {
    pub cfg: Config,
    pub paths: Paths,
    pub i18n: I18n,
    pub keymap: Keymap,
    pub themes: ThemeSet,
    pub theme: Theme,
    pub panels: [Panel; 2],
    pub active: usize,
    pub mode: Mode,
    pub status: String,
    pub status_error: bool,
    pub quit: bool,
    pub requests: Vec<Request>,
    /// Panel geometry from the last frame, so key handling knows how far a page is.
    pub rows: usize,
    pub columns: usize,
}

impl App {
    pub fn new(start: Startup) -> Self {
        let Startup {
            cfg,
            paths,
            i18n,
            keymap,
            themes,
            theme,
            left,
            right,
        } = start;
        let sort = Sort {
            key: parse_sort(&cfg.panel.sort),
            reverse: cfg.panel.reverse,
            dirs_first: cfg.panel.dirs_first,
            selected_first: cfg.panel.selected_first,
            natural: cfg.panel.natural_sort,
            ..Sort::default()
        };
        let view = ViewMode::parse(&cfg.panel.view).unwrap_or(ViewMode::Full);
        let hidden = cfg.panel.show_hidden;
        let mut app = App {
            cfg,
            paths,
            i18n,
            keymap,
            themes,
            theme,
            panels: [
                Panel::new(left, sort, view, hidden),
                Panel::new(right, sort, view, hidden),
            ],
            active: 0,
            mode: Mode::Normal,
            status: String::new(),
            status_error: false,
            quit: false,
            requests: Vec::new(),
            rows: 20,
            columns: 1,
        };
        app.read_panel(0);
        app.read_panel(1);
        app
    }

    pub fn panel(&self) -> &Panel {
        &self.panels[self.active]
    }

    pub fn panel_mut(&mut self) -> &mut Panel {
        &mut self.panels[self.active]
    }

    pub fn other(&self) -> &Panel {
        &self.panels[1 - self.active]
    }

    fn read_panel(&mut self, index: usize) {
        let generation = self.panels[index].generation + 1;
        self.panels[index].generation = generation;
        self.panels[index].loading = true;
        let path = self.panels[index].cwd.clone();
        self.requests.push(Request::Read {
            panel: index,
            path,
            generation,
        });
    }

    pub fn take_requests(&mut self) -> Vec<Request> {
        std::mem::take(&mut self.requests)
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_error = false;
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.status = text.into();
        self.status_error = true;
    }

    /// Show a modal message (startup warnings, command output that needs reading).
    pub fn show_message(&mut self, title: impl Into<String>, lines: Vec<String>) {
        self.mode = Mode::Message {
            title: title.into(),
            lines,
        };
    }

    /// Where the configuration for this run lives (shown in the help page footer).
    pub fn config_file(&self) -> &std::path::Path {
        &self.paths.config_file
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::Input(input) => self.handle_input(input),
            Event::Listed {
                panel,
                generation,
                result,
            } => {
                // A listing from a directory we have already left is dropped.
                if self.panels[panel].generation != generation {
                    return;
                }
                match result {
                    Ok(entries) => self.panels[panel].set_listing(entries, generation),
                    Err(e) => {
                        self.panels[panel].loading = false;
                        self.panels[panel].error = Some(e.clone());
                        self.set_error(e);
                    }
                }
            }
            Event::DirSized {
                panel,
                path,
                bytes,
                files,
            } => {
                let msg = self.i18n.t_args(
                    "msg-dirsize",
                    &[
                        ("path", path.display_name().into()),
                        ("bytes", sc_core::format_size(bytes).into()),
                        ("files", (files as i64).into()),
                    ],
                );
                self.panels[panel].message = Some(msg.clone());
                self.set_status(msg);
            }
        }
    }

    pub fn handle_input(&mut self, input: Input) {
        match input {
            Input::Key(key) => self.handle_key(key),
            Input::Paste(text) => {
                if let Mode::Prompt(p) = &mut self.mode {
                    p.text.push_str(text.trim());
                    let text = p.text.clone();
                    self.prompt_changed(&text);
                }
            }
            Input::Resize(..) => {}
            Input::Eof => self.quit = true,
        }
    }

    fn handle_key(&mut self, key: Key) {
        match self.mode.clone() {
            Mode::Help { scroll } => self.help_key(key, scroll),
            Mode::Message { .. } => {
                if matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::F(10)
                ) {
                    self.mode = Mode::Normal;
                }
            }
            Mode::Prompt(prompt) => self.prompt_key(key, prompt),
            Mode::Normal => self.normal_key(key),
        }
    }

    fn help_key(&mut self, key: Key, scroll: usize) {
        let step = self.rows.max(1);
        match key.code {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::F(10) | KeyCode::Char('q') => {
                self.mode = Mode::Normal
            }
            KeyCode::Up => {
                self.mode = Mode::Help {
                    scroll: scroll.saturating_sub(1),
                }
            }
            KeyCode::Down => self.mode = Mode::Help { scroll: scroll + 1 },
            KeyCode::PageUp => {
                self.mode = Mode::Help {
                    scroll: scroll.saturating_sub(step),
                }
            }
            KeyCode::PageDown => {
                self.mode = Mode::Help {
                    scroll: scroll + step,
                }
            }
            KeyCode::Home => self.mode = Mode::Help { scroll: 0 },
            _ => {}
        }
    }

    fn prompt_key(&mut self, key: Key, mut prompt: Prompt) {
        match key.code {
            KeyCode::Esc => {
                if let PromptKind::QuickSearch { origin } = prompt.kind {
                    self.panel_mut().cursor_to(origin);
                }
                self.mode = Mode::Normal;
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                self.prompt_accept(prompt);
            }
            KeyCode::Backspace => {
                prompt.text.pop();
                let text = prompt.text.clone();
                self.mode = Mode::Prompt(prompt);
                self.prompt_changed(&text);
            }
            _ => {
                if let Some(c) = key.as_typed_char() {
                    prompt.text.push(c);
                    let text = prompt.text.clone();
                    self.mode = Mode::Prompt(prompt);
                    self.prompt_changed(&text);
                }
            }
        }
    }

    /// Live feedback while typing: quick search moves the cursor, the filter re-filters.
    fn prompt_changed(&mut self, text: &str) {
        let kind = match &self.mode {
            Mode::Prompt(p) => p.kind.clone(),
            _ => return,
        };
        match kind {
            PromptKind::QuickSearch { origin } => {
                if let Some(row) = self.panel().find_row(text, origin) {
                    self.panel_mut().cursor_to(row);
                }
            }
            PromptKind::Filter => {
                let lower = text.to_lowercase();
                let panel = self.panel_mut();
                panel.filter = if lower.is_empty() { None } else { Some(lower) };
                panel.rebuild();
            }
            _ => {}
        }
    }

    fn prompt_accept(&mut self, prompt: Prompt) {
        match prompt.kind {
            PromptKind::Goto => {
                let raw = prompt.text.trim();
                if raw.is_empty() {
                    return;
                }
                let expanded = expand_tilde(raw);
                let target = if std::path::Path::new(&expanded).is_absolute() {
                    VPath::local(&expanded)
                } else {
                    self.panel().cwd.join(&expanded)
                }
                .normalised();
                self.goto(target);
            }
            PromptKind::SelectMask | PromptKind::DeselectMask => {
                let on = prompt.kind == PromptKind::SelectMask;
                let listing = self.panel().listing.clone();
                let result = self.panel_mut().selection.apply_mask(
                    &listing,
                    &prompt.text,
                    on,
                    MaskScope::FilesOnly,
                );
                match result {
                    Ok(changed) => {
                        self.panel_mut().rebuild();
                        let msg = self.i18n.t_args(
                            if on { "msg-selected" } else { "msg-deselected" },
                            &[("count", (changed as i64).into())],
                        );
                        self.set_status(msg);
                    }
                    Err(e) => {
                        let msg = self
                            .i18n
                            .t_args("msg-mask-invalid", &[("error", e.to_string().into())]);
                        self.set_error(msg);
                    }
                }
            }
            PromptKind::Filter | PromptKind::QuickSearch { .. } => {}
        }
    }

    fn normal_key(&mut self, key: Key) {
        if let Some(cmd) = self.keymap.resolve(&[Scope::Panel, Scope::Global], key) {
            self.run(cmd);
            return;
        }
        // No binding: maybe it starts a quick search.
        let typed = match self.cfg.panel.quick_search {
            QuickSearch::Off => None,
            QuickSearch::Plain => key.as_typed_char(),
            QuickSearch::Alt => {
                if key.mods.contains(Mods::ALT) && !key.mods.contains(Mods::CTRL) {
                    match key.code {
                        KeyCode::Char(c) => Some(c),
                        _ => None,
                    }
                } else {
                    None
                }
            }
        };
        if let Some(c) = typed {
            let origin = self.panel().cursor;
            let title = self.i18n.t("prompt-search");
            self.mode = Mode::Prompt(Prompt {
                kind: PromptKind::QuickSearch { origin },
                title,
                text: c.to_string(),
            });
            let text = c.to_string();
            self.prompt_changed(&text);
        }
    }

    pub fn run(&mut self, cmd: Command) {
        self.panel_mut().message = None;
        match cmd {
            Command::AppQuit => self.quit = true,
            Command::AppHelp => self.mode = Mode::Help { scroll: 0 },
            Command::AppThemeNext => {
                let next = self.themes.next_after(&self.theme.id).clone();
                let msg = self
                    .i18n
                    .t_args("msg-theme", &[("name", next.name.clone().into())]);
                self.theme = next;
                self.set_status(msg);
            }

            Command::PanelFocusToggle => self.active = 1 - self.active,
            Command::PanelSwap => {
                self.panels.swap(0, 1);
                self.active = 1 - self.active;
            }
            Command::PanelReread => {
                let a = self.active;
                self.read_panel(a);
            }
            Command::PanelHiddenToggle => {
                let show = !self.panel().show_hidden;
                for p in &mut self.panels {
                    p.show_hidden = show;
                    p.rebuild();
                }
                let msg = self.i18n.t(if show {
                    "msg-hidden-on"
                } else {
                    "msg-hidden-off"
                });
                self.set_status(msg);
            }
            Command::PanelViewSet(view) => {
                if view.is_implemented() {
                    self.panel_mut().view = view;
                    self.panel_mut().top = 0;
                    let msg = self
                        .i18n
                        .t_args("msg-view", &[("mode", view.as_str().into())]);
                    self.set_status(msg);
                } else {
                    self.not_implemented(&format!("panel.view.set({})", view.as_str()));
                }
            }
            Command::PanelSortBy(key) => {
                let allow = self.cfg.panel.allow_reverse_sort;
                let panel = self.panel_mut();
                panel.sort.toggle(key, allow);
                let reverse = panel.sort.reverse;
                panel.rebuild();
                let msg = self.i18n.t_args(
                    "msg-sort",
                    &[
                        ("key", key.as_str().into()),
                        ("reverse", if reverse { " ↓".into() } else { "".into() }),
                    ],
                );
                self.set_status(msg);
            }
            Command::PanelSortReverse => {
                let panel = self.panel_mut();
                panel.sort.reverse = !panel.sort.reverse;
                panel.rebuild();
            }
            Command::PanelFilterQuick => {
                let title = self.i18n.t("prompt-filter");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::Filter,
                    title,
                    text: self.panel().filter.clone().unwrap_or_default(),
                });
            }
            Command::PanelOtherSync => {
                let path = self.panel().cwd.clone();
                if self.other().cwd == path {
                    return;
                }
                let other = 1 - self.active;
                self.panels[other].go(path, true);
                self.read_panel(other);
            }

            Command::NavUp => self.move_cursor(-1),
            Command::NavDown => self.move_cursor(1),
            Command::NavPageUp => {
                let step = self.page_step();
                self.move_cursor(-step)
            }
            Command::NavPageDown => {
                let step = self.page_step();
                self.move_cursor(step)
            }
            Command::NavTop => {
                self.panel_mut().cursor_to(0);
                self.scroll();
            }
            Command::NavBottom => {
                let last = self.panel().listing.len().saturating_sub(1);
                self.panel_mut().cursor_to(last);
                self.scroll();
            }
            Command::NavEnter => match self.panel().enter_target() {
                Some(path) => self.goto(path),
                None => self.not_implemented("nav.enter (running files)"),
            },
            Command::NavParent => {
                if let Some(parent) = self.panel().cwd.parent() {
                    let child = self.panel().cwd.display_name();
                    self.goto(parent.normalised());
                    // Put the cursor back on the directory we came out of, like FAR does.
                    self.panel_mut().message = Some(child);
                }
            }
            Command::NavRoot => {
                let root = self.panel().cwd.root();
                self.goto(root);
            }
            Command::NavBack => match self.panel_mut().history_back() {
                Some(path) => {
                    let a = self.active;
                    self.panels[a].go(path, false);
                    self.read_panel(a);
                }
                None => {
                    let m = self.i18n.t("msg-no-history");
                    self.set_status(m)
                }
            },
            Command::NavForward => match self.panel_mut().history_forward() {
                Some(path) => {
                    let a = self.active;
                    self.panels[a].go(path, false);
                    self.read_panel(a);
                }
                None => {
                    let m = self.i18n.t("msg-no-history");
                    self.set_status(m)
                }
            },
            Command::NavGotoPath => {
                let title = self.i18n.t("prompt-goto");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::Goto,
                    title,
                    text: String::new(),
                });
            }

            Command::SelToggle => {
                if let Some(e) = self.panel().current() {
                    if e.is_selectable() {
                        let id = e.id;
                        self.panel_mut().selection.toggle(id);
                        self.panel_mut().rebuild();
                        self.move_cursor(1);
                    }
                }
            }
            Command::SelAll => {
                let listing = self.panel().listing.clone();
                self.panel_mut()
                    .selection
                    .select_all(&listing, MaskScope::FilesAndDirs);
                self.panel_mut().rebuild();
            }
            Command::SelNone => {
                self.panel_mut().selection.clear();
                self.panel_mut().rebuild();
            }
            Command::SelInvert => {
                let listing = self.panel().listing.clone();
                self.panel_mut()
                    .selection
                    .invert(&listing, MaskScope::FilesOnly);
                self.panel_mut().rebuild();
            }
            Command::SelMaskAdd | Command::SelMaskRemove => {
                let add = cmd == Command::SelMaskAdd;
                let title = self.i18n.t(if add {
                    "prompt-select-mask"
                } else {
                    "prompt-deselect-mask"
                });
                self.mode = Mode::Prompt(Prompt {
                    kind: if add {
                        PromptKind::SelectMask
                    } else {
                        PromptKind::DeselectMask
                    },
                    title,
                    text: "*".to_string(),
                });
            }
            Command::SelRestore => {
                self.panel_mut().selection.restore_previous();
                self.panel_mut().rebuild();
            }

            Command::FileDirSize => {
                let (panel, path) = match self.panel().current() {
                    Some(e) if e.kind.is_dirlike() => (self.active, self.panel().cwd.join(&e.name)),
                    _ => {
                        self.not_implemented("file.dirsize (only directories for now)");
                        return;
                    }
                };
                self.requests.push(Request::DirSize { panel, path });
                let m = self.i18n.t("panel-loading");
                self.set_status(m);
            }

            other => self.not_implemented(&other.id()),
        }
    }

    fn not_implemented(&mut self, what: &str) {
        let msg = self.i18n.t_args(
            "msg-not-implemented",
            &[("command", what.to_string().into())],
        );
        self.set_status(msg);
    }

    fn page_step(&self) -> isize {
        let panel = self.panel();
        panel.capacity(self.rows, self.columns) as isize
    }

    fn move_cursor(&mut self, delta: isize) {
        self.panel_mut().move_cursor(delta);
        self.scroll();
    }

    fn scroll(&mut self) {
        let (rows, cols) = (self.rows, self.columns);
        self.panel_mut().scroll_into_view(rows, cols);
    }

    pub fn goto(&mut self, path: VPath) {
        let index = self.active;
        self.panels[index].go(path, true);
        self.read_panel(index);
    }

    /// The `F1..F10` labels for the key bar, localised.
    pub fn keybar(&self) -> Vec<(u8, String)> {
        [
            (1u8, "key-help"),
            (2, "key-menu"),
            (3, "key-view"),
            (4, "key-edit"),
            (5, "key-copy"),
            (6, "key-move"),
            (7, "key-mkdir"),
            (8, "key-delete"),
            (9, "key-pulldown"),
            (10, "key-quit"),
        ]
        .iter()
        .map(|(n, id)| (*n, self.i18n.t(id)))
        .collect()
    }

    /// The help page, generated from the live keymap so it can never drift.
    pub fn help_lines(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();
        let mut last_scope = None;
        for (scope, key, cmd) in self.keymap.bindings() {
            if last_scope != Some(scope) {
                if last_scope.is_some() {
                    lines.push(String::new());
                }
                lines.push(format!("[{}]", scope.as_str()));
                last_scope = Some(scope);
            }
            lines.push(format!("  {:<16} {}", key.to_string(), cmd.id()));
        }
        lines
    }
}

fn parse_sort(s: &str) -> SortKey {
    match s.trim().to_ascii_lowercase().as_str() {
        "ext" | "extension" => SortKey::Ext,
        "size" => SortKey::Size,
        "mtime" | "time" | "date" => SortKey::MTime,
        "none" | "unsorted" => SortKey::Unsorted,
        _ => SortKey::Name,
    }
}

fn expand_tilde(raw: &str) -> String {
    if let Some(rest) = raw.strip_prefix('~') {
        if let Some(home) = dirs_home() {
            return format!("{home}{rest}");
        }
    }
    raw.to_string()
}

fn dirs_home() -> Option<String> {
    std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::{EntryFlags, EntryId, EntryKind};
    use std::ffi::OsString;

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn app() -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(std::path::Path::new("/tmp/sc-test/sc.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local("/home/mb"),
            right: VPath::local("/tmp"),
        });
        app.requests.clear();
        let entries = vec![
            Entry::parent_entry(),
            entry("docs", EntryKind::Dir, 0),
            entry("a.txt", EntryKind::File, 100),
            entry("b.rs", EntryKind::File, 200),
            entry("c.rs", EntryKind::File, 300),
        ];
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(entries.clone()),
        });
        app.handle_event(Event::Listed {
            panel: 1,
            generation: 1,
            result: Ok(entries),
        });
        app
    }

    fn press(app: &mut App, key: &str) {
        app.handle_input(Input::Key(key.parse().unwrap()));
    }

    #[test]
    fn keys_drive_the_documented_commands() {
        let mut app = app();
        assert_eq!(app.active, 0);
        press(&mut app, "Tab");
        assert_eq!(app.active, 1);
        press(&mut app, "Ctrl+U");
        assert_eq!(app.active, 0);
        press(&mut app, "F10");
        assert!(app.quit);
    }

    #[test]
    fn navigation_moves_and_clamps_the_cursor() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        assert_eq!(app.panel().cursor, 2);
        press(&mut app, "End");
        assert_eq!(app.panel().cursor, 4);
        press(&mut app, "Home");
        assert_eq!(app.panel().cursor, 0);
    }

    #[test]
    fn enter_on_a_directory_requests_a_read_of_it() {
        let mut app = app();
        press(&mut app, "Down"); // docs/
        press(&mut app, "Enter");
        let reqs = app.take_requests();
        assert_eq!(
            reqs,
            vec![Request::Read {
                panel: 0,
                path: VPath::local("/home/mb/docs"),
                generation: 2
            }]
        );
        assert_eq!(app.panel().cwd, VPath::local("/home/mb/docs"));
    }

    #[test]
    fn enter_on_a_file_says_so_instead_of_doing_nothing() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        press(&mut app, "Enter");
        assert!(app.status.contains("not implemented"), "{}", app.status);
        assert!(app.take_requests().is_empty());
    }

    #[test]
    fn insert_selects_and_advances() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        press(&mut app, "Ins");
        assert_eq!(app.panel().selection.len(), 1);
        assert_eq!(app.panel().cursor, 3);
        assert_eq!(app.panel().listing.totals.selected_bytes, 100);
    }

    #[test]
    fn mask_selection_runs_through_the_prompt() {
        let mut app = app();
        press(&mut app, "+");
        assert!(matches!(app.mode, Mode::Prompt(_)));
        press(&mut app, "Backspace"); // clear the default "*"
        for c in ["*", ".", "r", "s"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(app.panel().selection.len(), 2, "{}", app.status);
        assert!(app.status.contains('2'), "{}", app.status);
    }

    #[test]
    fn sorting_toggles_direction_on_the_second_press() {
        let mut app = app();
        press(&mut app, "Ctrl+F6");
        assert_eq!(app.panel().sort.key, SortKey::Size);
        assert!(!app.panel().sort.reverse);
        press(&mut app, "Ctrl+F6");
        assert!(app.panel().sort.reverse);
        assert!(app.status.contains("size"), "{}", app.status);
        // biggest first now
        assert_eq!(app.panel().listing.row(2).unwrap().name_str(), "c.rs");
    }

    #[test]
    fn hidden_toggle_applies_to_both_panels() {
        let mut app = app();
        press(&mut app, "Ctrl+H");
        assert!(app.panels[0].show_hidden && app.panels[1].show_hidden);
        assert!(app.status.contains("hidden"), "{}", app.status);
    }

    #[test]
    fn quick_search_jumps_and_esc_restores_the_cursor() {
        let mut app = app();
        app.cfg.panel.quick_search = QuickSearch::Plain;
        press(&mut app, "d"); // docs
        assert_eq!(app.panel().cursor, 1);
        press(&mut app, "Esc");
        assert_eq!(app.panel().cursor, 0);
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn quick_filter_narrows_the_listing_live() {
        let mut app = app();
        press(&mut app, "Alt+I");
        for c in ["r", "s"] {
            press(&mut app, c);
        }
        // `..` is always kept, plus b.rs and c.rs
        assert_eq!(app.panel().listing.len(), 3);
        press(&mut app, "Enter");
        assert_eq!(app.panel().filter.as_deref(), Some("rs"));
    }

    #[test]
    fn goto_prompt_accepts_relative_and_absolute_paths() {
        let mut app = app();
        press(&mut app, "Ctrl+L");
        for c in ["d", "o", "c", "s"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(app.panel().cwd, VPath::local("/home/mb/docs"));
    }

    #[test]
    fn theme_cycling_changes_the_active_theme() {
        let mut app = app();
        assert_eq!(app.theme.id, "commander");
        press(&mut app, "Alt+T");
        assert_eq!(app.theme.id, "black");
        assert!(app.status.contains("Black"), "{}", app.status);
    }

    #[test]
    fn help_is_generated_from_the_keymap() {
        let mut app = app();
        press(&mut app, "F1");
        assert!(matches!(app.mode, Mode::Help { .. }));
        let lines = app.help_lines();
        assert!(lines
            .iter()
            .any(|l| l.contains("F5") && l.contains("file.copy")));
        assert!(lines.iter().any(|l| l.contains("[global]")));
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn stale_listings_are_dropped() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Enter"); // generation 2 requested
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1, // the old directory answering late
            result: Ok(vec![entry("stale", EntryKind::File, 1)]),
        });
        assert!(
            app.panel().listing.is_empty(),
            "the stale listing was applied"
        );
    }

    #[test]
    fn a_read_error_is_shown_and_remembered() {
        let mut app = app();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Err("/home/mb: permission denied".into()),
        });
        assert!(app.status_error);
        assert_eq!(
            app.panels[0].error.as_deref(),
            Some("/home/mb: permission denied")
        );
    }

    #[test]
    fn dirsize_requests_a_scan_and_reports_the_result() {
        let mut app = app();
        press(&mut app, "Down"); // docs/
        press(&mut app, "Space");
        assert_eq!(
            app.take_requests(),
            vec![Request::DirSize {
                panel: 0,
                path: VPath::local("/home/mb/docs")
            }]
        );
        app.handle_event(Event::DirSized {
            panel: 0,
            path: VPath::local("/home/mb/docs"),
            bytes: 4096,
            files: 12,
        });
        assert!(app.status.contains("4.00 K"), "{}", app.status);
        assert!(app.status.contains("12"), "{}", app.status);
    }

    #[test]
    fn sync_other_panel_sends_it_to_this_directory() {
        let mut app = app();
        press(&mut app, "Ctrl+=");
        assert_eq!(app.panels[1].cwd, VPath::local("/home/mb"));
        assert!(app
            .take_requests()
            .iter()
            .any(|r| matches!(r, Request::Read { panel: 1, .. })));
    }

    #[test]
    fn localisation_follows_the_configured_language() {
        let mut app = app();
        app.i18n = I18n::new("it");
        press(&mut app, "Ctrl+H");
        assert!(app.status.contains("nascosti"), "{}", app.status);
        assert_eq!(app.keybar()[4].1, "Copia");
    }
}
