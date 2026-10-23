use std::sync::mpsc::Sender;

use sc_cmd::{Command, Key, KeyCode, Keymap, Mods, Scope};
use sc_config::{Config, Paths, QuickSearch};
use sc_core::{CancelToken, Entry, MaskScope, Sort, SortKey, VPath, ViewMode};
use sc_i18n::I18n;
use sc_jobs::{Decision, JobKind, JobSpec, Outcome, Progress, Question};
use sc_tui::Input;
use sc_ui::{Theme, ThemeSet};
use sc_view::{FileSource, SearchMode, Viewer};

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
    /// Run a file operation off the UI thread.
    Job {
        id: u64,
        spec: JobSpec,
        cancel: CancelToken,
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
    JobProgress {
        id: u64,
        progress: Progress,
    },
    /// The engine needs an answer before it can continue; it is blocked on `reply`.
    JobQuestion {
        id: u64,
        question: Question,
        reply: Sender<Decision>,
    },
    JobDone {
        id: u64,
        kind: JobKind,
        outcome: Outcome,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptKind {
    Goto,
    SelectMask,
    DeselectMask,
    Filter,
    /// Destination for F5; pre-filled with the other panel's directory.
    CopyTarget,
    /// Query for the viewer's search.
    ViewerSearch,
    /// Offset or percentage for the viewer.
    ViewerGoto,
    /// Destination for F6.
    MoveTarget,
    /// Name for F7.
    Mkdir,
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

/// A yes/no question that carries the job it will start on `Enter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Confirm {
    pub title: String,
    pub lines: Vec<String>,
    pub spec: JobSpec,
}

/// A decision point the engine is blocked on.
#[derive(Debug)]
pub struct PendingQuestion {
    pub question: Question,
    pub buttons: Vec<(String, Decision)>,
    pub reply: Sender<Decision>,
}

#[derive(Debug)]
pub struct JobView {
    pub id: u64,
    pub kind: JobKind,
    pub progress: Progress,
    pub question: Option<PendingQuestion>,
    pub cancel: CancelToken,
    pub choice: usize,
}

#[derive(Debug)]
pub enum Mode {
    Normal,
    Prompt(Prompt),
    Confirm(Confirm),
    Job(JobView),
    /// `F3`: the internal viewer, full screen.
    Viewer(Box<Viewer>),
    Help {
        scroll: usize,
    },
    Message {
        title: String,
        lines: Vec<String>,
    },
}

impl Mode {
    pub fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal)
    }

    pub fn job(&self) -> Option<&JobView> {
        match self {
            Mode::Job(view) => Some(view),
            _ => None,
        }
    }

    pub fn viewer(&self) -> Option<&Viewer> {
        match self {
            Mode::Viewer(v) => Some(v),
            _ => None,
        }
    }

    pub fn viewer_mut(&mut self) -> Option<&mut Viewer> {
        match self {
            Mode::Viewer(v) => Some(v),
            _ => None,
        }
    }
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
    /// Size of the viewer's text area, from the last frame.
    pub viewer_size: (u16, u16),
    /// Remembered across files, like every commander does.
    pub search_mode: SearchMode,
    pub search_case_insensitive: bool,
    pub last_search: Option<String>,
    /// The viewer, parked while one of its prompts is open.
    viewer_under_prompt: Option<Box<Viewer>>,
    next_job_id: u64,
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
            viewer_size: (80, 20),
            search_mode: SearchMode::Plain,
            search_case_insensitive: true,
            last_search: None,
            viewer_under_prompt: None,
            next_job_id: 1,
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
            Event::JobProgress { id, progress } => {
                if let Mode::Job(view) = &mut self.mode {
                    if view.id == id {
                        view.progress = progress;
                    }
                }
            }
            Event::JobQuestion {
                id,
                question,
                reply,
            } => {
                let buttons = self.buttons_for(&question);
                match &mut self.mode {
                    Mode::Job(view) if view.id == id => {
                        view.choice = 0;
                        view.question = Some(PendingQuestion {
                            question,
                            buttons,
                            reply,
                        });
                    }
                    // A question for a job we are no longer showing cannot be answered by a
                    // human, so it must not block the engine for ever.
                    _ => {
                        let _ = reply.send(Decision::Cancel);
                    }
                }
            }
            Event::JobDone { id, kind, outcome } => self.job_done(id, kind, outcome),
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
        // `Mode` owns a reply channel while a job is asking, so it cannot be cloned: take it
        // out, dispatch, and put back whatever the handler did not replace.
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Help { scroll } => {
                self.mode = Mode::Help { scroll };
                self.help_key(key, scroll);
            }
            Mode::Message { title, lines } => {
                let close = matches!(
                    key.code,
                    KeyCode::Esc | KeyCode::Enter | KeyCode::Char(' ') | KeyCode::F(10)
                );
                if !close {
                    self.mode = Mode::Message { title, lines };
                }
            }
            Mode::Prompt(prompt) => {
                self.mode = Mode::Prompt(prompt.clone());
                self.prompt_key(key, prompt);
            }
            Mode::Confirm(confirm) => {
                self.mode = Mode::Confirm(confirm.clone());
                self.confirm_key(key, confirm);
            }
            Mode::Job(view) => {
                self.mode = Mode::Job(view);
                self.job_key(key);
            }
            Mode::Viewer(viewer) => {
                self.mode = Mode::Viewer(viewer);
                self.viewer_key(key);
            }
            Mode::Normal => self.normal_key(key),
        }
    }

    fn confirm_key(&mut self, key: Key, confirm: Confirm) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.mode = Mode::Normal;
                self.start_job(confirm.spec);
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::F(10) => {
                self.mode = Mode::Normal
            }
            _ => {}
        }
    }

    fn job_key(&mut self, key: Key) {
        let Mode::Job(view) = &mut self.mode else {
            return;
        };
        match &view.question {
            None => {
                // No question pending: the only thing to do is stop the job.
                if matches!(key.code, KeyCode::Esc | KeyCode::Char('c')) {
                    view.cancel.cancel();
                    let kind = view.kind;
                    let msg = self
                        .i18n
                        .t_args("job-cancelled", &[("kind", self.kind_label(kind).into())]);
                    self.set_status(msg);
                }
            }
            Some(pending) => {
                let count = pending.buttons.len();
                match key.code {
                    KeyCode::Left | KeyCode::BackTab => {
                        view.choice = (view.choice + count - 1) % count
                    }
                    KeyCode::Right | KeyCode::Tab => view.choice = (view.choice + 1) % count,
                    KeyCode::Enter => {
                        let decision = pending.buttons[view.choice].1.clone();
                        self.answer(decision);
                    }
                    KeyCode::Esc => self.answer(Decision::Cancel),
                    _ => {}
                }
            }
        }
    }

    fn viewer_key(&mut self, key: Key) {
        if let Some(cmd) = self.keymap.resolve(&[Scope::Viewer, Scope::Global], key) {
            self.run(cmd);
        }
    }

    /// Unblock the engine with a decision.
    fn answer(&mut self, decision: Decision) {
        if let Mode::Job(view) = &mut self.mode {
            if let Some(pending) = view.question.take() {
                let _ = pending.reply.send(decision);
            }
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
                self.restore_viewer();
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
            PromptKind::CopyTarget | PromptKind::MoveTarget => {
                let raw = prompt.text.trim();
                if raw.is_empty() {
                    return;
                }
                let kind = if prompt.kind == PromptKind::CopyTarget {
                    JobKind::Copy
                } else {
                    JobKind::Move
                };
                let sources = self.job_sources();
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let target = self.resolve_path(raw);
                self.start_job(JobSpec::new(kind, sources, Some(target)));
            }
            PromptKind::Mkdir => {
                let raw = prompt.text.trim();
                if raw.is_empty() {
                    return;
                }
                let target = self.resolve_path(raw);
                self.start_job(JobSpec::new(JobKind::Mkdir, vec![], Some(target)));
            }
            PromptKind::ViewerSearch => {
                let query = prompt.text.trim().to_string();
                self.restore_viewer();
                if query.is_empty() {
                    return;
                }
                self.last_search = Some(query.clone());
                let (mode, ci) = (self.search_mode, self.search_case_insensitive);
                let result = self
                    .mode
                    .viewer_mut()
                    .map(|v| v.search(&query, mode, ci, false, &CancelToken::new()));
                match result {
                    Some(Ok(true)) => {}
                    Some(Ok(false)) => {
                        let msg = self
                            .i18n
                            .t_args("viewer-not-found", &[("query", query.into())]);
                        self.set_error(msg);
                    }
                    Some(Err(e)) => self.set_error(e.to_string()),
                    None => {}
                }
            }
            PromptKind::ViewerGoto => {
                let raw = prompt.text.trim().to_string();
                self.restore_viewer();
                if raw.is_empty() {
                    return;
                }
                let (width, height) = self.viewer_size;
                let _ = (width, height);
                if let Some(viewer) = self.mode.viewer_mut() {
                    if let Some(pct) = raw.strip_suffix('%') {
                        if let Ok(p) = pct.trim().parse::<u16>() {
                            viewer.goto_percent(p);
                            return;
                        }
                    }
                    let parsed = if let Some(hex) =
                        raw.strip_prefix("0x").or_else(|| raw.strip_prefix("0X"))
                    {
                        u64::from_str_radix(hex, 16).ok()
                    } else {
                        raw.parse::<u64>().ok()
                    };
                    match parsed {
                        Some(offset) => viewer.jump(offset),
                        None => {
                            let msg = format!("{raw}: ?");
                            self.set_error(msg);
                        }
                    }
                }
            }
            PromptKind::Filter | PromptKind::QuickSearch { .. } => {}
        }
    }

    /// A path typed by the user: absolute as given, `~` expanded, otherwise relative to the
    /// active panel.
    fn resolve_path(&self, raw: &str) -> VPath {
        let expanded = expand_tilde(raw);
        if std::path::Path::new(&expanded).is_absolute() {
            VPath::local(&expanded)
        } else {
            self.panel().cwd.join(&expanded)
        }
        .normalised()
    }

    /// What an operation acts on: the selection, or the item under the cursor.
    pub fn job_sources(&self) -> Vec<VPath> {
        let panel = self.panel();
        if !panel.selection.is_empty() {
            let mut out = Vec::new();
            for row in 0..panel.listing.len() {
                let e = panel.listing.row(row).expect("row in range");
                if e.is_selectable() && panel.selection.contains(e.id) {
                    out.push(panel.cwd.join(&e.name));
                }
            }
            return out;
        }
        match panel.current() {
            Some(e) if e.is_selectable() => vec![panel.cwd.join(&e.name)],
            _ => Vec::new(),
        }
    }

    /// How the sources are described in a prompt: one name, or "N selected items".
    fn sources_label(&self, sources: &[VPath]) -> String {
        match sources.len() {
            1 => sources[0].display_name(),
            n => self
                .i18n
                .t_args("what-selected", &[("count", (n as i64).into())]),
        }
    }

    fn kind_label(&self, kind: JobKind) -> String {
        self.i18n.t(match kind {
            JobKind::Copy => "job-copy",
            JobKind::Move => "job-move",
            JobKind::Delete => "job-delete",
            JobKind::Mkdir => "job-mkdir",
        })
    }

    /// Title of the progress overlay for a job kind.
    pub fn job_title(&self, kind: JobKind) -> String {
        self.kind_label(kind)
    }

    /// Title and body of the dialog for a decision point.
    pub fn question_text(&self, question: &Question) -> (String, Vec<String>) {
        match question {
            Question::Conflict {
                src,
                dst,
                src_size,
                dst_size,
                src_newer,
            } => {
                let title = self.i18n.t("conflict-title");
                let lines = vec![
                    self.i18n.t_args(
                        "conflict-dst",
                        &[
                            ("path", dst.to_string().into()),
                            ("bytes", sc_core::format_size(*dst_size).into()),
                        ],
                    ),
                    self.i18n.t_args(
                        "conflict-src",
                        &[
                            ("path", src.to_string().into()),
                            ("bytes", sc_core::format_size(*src_size).into()),
                        ],
                    ),
                    String::new(),
                    self.i18n.t(if *src_newer {
                        "conflict-newer"
                    } else {
                        "conflict-older"
                    }),
                ];
                (title, lines)
            }
            Question::Error { path, error } => (
                self.i18n.t("dialog-error"),
                vec![path.to_string(), String::new(), error.clone()],
            ),
        }
    }

    fn buttons_for(&self, question: &Question) -> Vec<(String, Decision)> {
        match question {
            Question::Conflict { .. } => vec![
                (self.i18n.t("btn-overwrite"), Decision::Overwrite),
                (self.i18n.t("btn-overwrite-all"), Decision::OverwriteAll),
                (self.i18n.t("btn-skip"), Decision::Skip),
                (self.i18n.t("btn-skip-all"), Decision::SkipAll),
                (self.i18n.t("btn-cancel"), Decision::Cancel),
            ],
            Question::Error { .. } => vec![
                (self.i18n.t("btn-retry"), Decision::Retry),
                (self.i18n.t("btn-skip"), Decision::Skip),
                (self.i18n.t("btn-skip-all"), Decision::SkipAll),
                (self.i18n.t("btn-cancel"), Decision::Cancel),
            ],
        }
    }

    /// Hand a job to the runner and show its progress overlay.
    pub fn start_job(&mut self, spec: JobSpec) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        let kind = spec.kind;
        self.mode = Mode::Job(JobView {
            id,
            kind,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        });
        self.requests.push(Request::Job { id, spec, cancel });
    }

    fn job_done(&mut self, id: u64, kind: JobKind, outcome: Outcome) {
        if let Mode::Job(view) = &self.mode {
            if view.id != id {
                return;
            }
        }
        self.mode = Mode::Normal;

        let kind_label = self.kind_label(kind);
        let mut summary = if outcome.renamed > 0 && outcome.bytes == 0 {
            self.i18n.t_args(
                "job-done-renamed",
                &[
                    ("kind", kind_label.clone().into()),
                    ("files", (outcome.files as i64).into()),
                ],
            )
        } else {
            self.i18n.t_args(
                "job-done",
                &[
                    ("kind", kind_label.clone().into()),
                    ("files", (outcome.files as i64).into()),
                    ("bytes", sc_core::format_size(outcome.bytes).into()),
                ],
            )
        };
        if outcome.skipped > 0 {
            summary.push_str(&self.i18n.t_args(
                "job-done-skipped",
                &[("skipped", (outcome.skipped as i64).into())],
            ));
        }
        if !outcome.errors.is_empty() {
            summary.push_str(&self.i18n.t_args(
                "job-done-errors",
                &[("errors", (outcome.errors.len() as i64).into())],
            ));
        }
        if outcome.cancelled {
            summary = self
                .i18n
                .t_args("job-cancelled", &[("kind", kind_label.into())]);
        }

        if outcome.errors.is_empty() {
            self.set_status(summary);
        } else {
            self.set_error(summary);
            let title = self.i18n.t("job-errors-title");
            let lines: Vec<String> = outcome
                .errors
                .iter()
                .take(8)
                .map(|(path, error)| format!("{path}: {error}"))
                .collect();
            self.show_message(title, lines);
        }

        // Both panels may be looking at what just changed.
        self.panels[self.active].selection.clear();
        self.read_panel(0);
        self.read_panel(1);
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
        // While the viewer is open, movement and its own toggles belong to it, not the panel.
        if self.mode.viewer().is_some() && is_viewer_movement(cmd) {
            self.viewer_command(cmd);
            return;
        }
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

            Command::FileView => self.open_viewer(),

            Command::ViewerQuit => self.mode = Mode::Normal,
            Command::ViewerSearch if self.mode.viewer().is_some() => {
                let mode = self.search_mode;
                let title = self
                    .i18n
                    .t_args("viewer-prompt-search", &[("mode", mode.as_str().into())]);
                self.mode_prompt_over_viewer(
                    PromptKind::ViewerSearch,
                    title,
                    self.last_search.clone().unwrap_or_default(),
                );
            }
            Command::ViewerGoto if self.mode.viewer().is_some() => {
                let title = self.i18n.t("viewer-prompt-goto");
                self.mode_prompt_over_viewer(PromptKind::ViewerGoto, title, String::new());
            }
            Command::ViewerSearchNext | Command::ViewerSearchPrev
                if self.mode.viewer().is_some() =>
            {
                let backwards = cmd == Command::ViewerSearchPrev;
                let found = self
                    .mode
                    .viewer_mut()
                    .map(|v| v.find_again(backwards, &CancelToken::new()))
                    .unwrap_or(false);
                if !found {
                    let query = self.last_search.clone().unwrap_or_default();
                    let msg = self
                        .i18n
                        .t_args("viewer-not-found", &[("query", query.into())]);
                    self.set_error(msg);
                }
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

            Command::FileCopy | Command::FileMove => {
                let sources = self.job_sources();
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let what = self.sources_label(&sources);
                let copy = cmd == Command::FileCopy;
                let title = self.i18n.t_args(
                    if copy {
                        "prompt-copy-to"
                    } else {
                        "prompt-move-to"
                    },
                    &[("what", what.into())],
                );
                // Pre-filled with the passive panel's directory, as every commander does.
                let mut text = self.other().cwd.to_string();
                if !text.ends_with(std::path::MAIN_SEPARATOR) {
                    text.push(std::path::MAIN_SEPARATOR);
                }
                self.mode = Mode::Prompt(Prompt {
                    kind: if copy {
                        PromptKind::CopyTarget
                    } else {
                        PromptKind::MoveTarget
                    },
                    title,
                    text,
                });
            }
            Command::FileMkdir => {
                let title = self.i18n.t("prompt-mkdir");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::Mkdir,
                    title,
                    text: String::new(),
                });
            }
            Command::FileDelete => {
                let sources = self.job_sources();
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let bytes = self.selected_bytes(&sources);
                let spec = JobSpec::new(JobKind::Delete, sources.clone(), None);
                if !self.cfg.general.confirm_delete {
                    self.start_job(spec);
                    return;
                }
                let title = self.i18n.t("confirm-delete-title");
                let body = self.i18n.t_args(
                    "confirm-delete-body",
                    &[
                        ("count", (sources.len() as i64).into()),
                        ("bytes", sc_core::format_size(bytes).into()),
                    ],
                );
                let mut lines = vec![body, String::new()];
                lines.extend(
                    sources
                        .iter()
                        .take(3)
                        .map(|p| format!("  {}", p.display_name())),
                );
                if sources.len() > 3 {
                    lines.push(format!("  … {}", sources.len() - 3));
                }
                lines.push(String::new());
                lines.push(self.i18n.t("confirm-yes-no"));
                self.mode = Mode::Confirm(Confirm { title, lines, spec });
            }

            other => self.not_implemented(&other.id()),
        }
    }

    /// `F3`: open the viewer on the item under the cursor. On a directory it measures it
    /// instead, as FAR does.
    fn open_viewer(&mut self) {
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        if entry.kind.is_dirlike() {
            self.run(Command::FileDirSize);
            return;
        }
        let path = self.panel().cwd.join(&entry.name);
        match FileSource::open(path.as_path()) {
            Ok(source) => {
                let mut viewer = Viewer::new(Box::new(source), path);
                if viewer.is_empty() {
                    let m = self.i18n.t("viewer-empty");
                    self.set_status(m);
                }
                self.mode = Mode::Viewer(Box::new(viewer));
            }
            Err(e) => self.set_error(format!("{path}: {e}")),
        }
    }

    /// A prompt that keeps the viewer underneath it (search, go to).
    fn mode_prompt_over_viewer(&mut self, kind: PromptKind, title: String, text: String) {
        let Mode::Viewer(viewer) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        self.viewer_under_prompt = Some(viewer);
        self.mode = Mode::Prompt(Prompt { kind, title, text });
    }

    /// Put the viewer back when its prompt closes.
    fn restore_viewer(&mut self) {
        if let Some(viewer) = self.viewer_under_prompt.take() {
            self.mode = Mode::Viewer(viewer);
        }
    }

    fn viewer_command(&mut self, cmd: Command) {
        let (width, height) = self.viewer_size;
        let mut message: Option<String> = None;
        let mut error: Option<String> = None;
        if let Some(viewer) = self.mode.viewer_mut() {
            match cmd {
                Command::NavUp => viewer.scroll(-1, width, height),
                Command::NavDown => viewer.scroll(1, width, height),
                Command::NavPageUp => viewer.scroll(-(height as i64), width, height),
                Command::NavPageDown => viewer.scroll(height as i64, width, height),
                Command::NavTop => viewer.to_start(),
                Command::NavBottom => viewer.to_end(width, height),
                Command::ViewerLeft => viewer.scroll_h(-1),
                Command::ViewerRight => viewer.scroll_h(1),
                Command::ViewerWrap => viewer.toggle_wrap(),
                Command::ViewerHex => viewer.toggle_hex(),
                Command::ViewerModeNext => {
                    let next = match viewer.mode {
                        sc_view::Mode::Text => sc_view::Mode::Dump,
                        sc_view::Mode::Dump => sc_view::Mode::Hex,
                        sc_view::Mode::Hex => sc_view::Mode::Text,
                    };
                    viewer.set_mode(next);
                }
                Command::ViewerCodepageNext => viewer.next_codepage(),
                Command::ViewerFollow => viewer.toggle_follow(),
                Command::ViewerBytesLess => viewer.set_bytes_per_row(-8),
                Command::ViewerBytesMore => viewer.set_bytes_per_row(8),
                Command::ViewerUndoPosition if !viewer.undo_jump() => {
                    error = Some("viewer-no-undo".into());
                }
                _ => {}
            }
            // Messages that depend on the new state.
            match cmd {
                Command::ViewerCodepageNext => {
                    message = Some(format!("codepage:{}", viewer.codepage.name()))
                }
                Command::ViewerWrap => message = Some(format!("wrap:{}", viewer.wrap.as_str())),
                Command::ViewerFollow => {
                    message = Some(if viewer.follow {
                        "follow:on".into()
                    } else {
                        "follow:off".into()
                    })
                }
                _ => {}
            }
        }
        if let Some(key) = error {
            let m = self.i18n.t(&key);
            self.set_error(m);
        }
        if let Some(tag) = message {
            let text = match tag.split_once(':') {
                Some(("codepage", name)) => self
                    .i18n
                    .t_args("viewer-codepage", &[("name", name.to_string().into())]),
                Some(("wrap", mode)) => self
                    .i18n
                    .t_args("viewer-wrap-mode", &[("mode", mode.to_string().into())]),
                Some(("follow", "on")) => self.i18n.t("viewer-follow-on"),
                Some(("follow", _)) => self.i18n.t("viewer-follow-off"),
                _ => String::new(),
            };
            if !text.is_empty() {
                self.set_status(text);
            }
        }
    }

    /// Total size of what is about to be deleted, for the confirmation.
    fn selected_bytes(&self, sources: &[VPath]) -> u64 {
        let panel = self.panel();
        let mut total = 0;
        for src in sources {
            if let Some(name) = src.file_name() {
                if let Some(row) = panel.listing.row_of_name(&name.to_string_lossy()) {
                    if let Some(e) = panel.listing.row(row) {
                        total += e.size;
                    }
                }
            }
        }
        total
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

    /// Hand the viewer to the renderer (it needs `&mut` to read its windows) and say whether
    /// it was parked under a prompt, so it goes back where it came from.
    pub fn viewer_for_render(&mut self) -> Option<(Box<Viewer>, bool)> {
        if let Some(viewer) = self.viewer_under_prompt.take() {
            return Some((viewer, true));
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Viewer(viewer) => Some((viewer, false)),
            other => {
                self.mode = other;
                None
            }
        }
    }

    pub fn return_viewer(&mut self, viewer: Box<Viewer>, parked: bool) {
        if parked {
            self.viewer_under_prompt = Some(viewer);
        } else {
            self.mode = Mode::Viewer(viewer);
        }
    }

    /// The key bar while the viewer is open.
    pub fn keybar_viewer(&self) -> Vec<(u8, String)> {
        vec![
            (1, self.i18n.t("key-help")),
            (2, self.i18n.t("key-wrap")),
            (3, self.i18n.t("key-quit")),
            (4, self.i18n.t("key-hex")),
            (7, self.i18n.t("key-search")),
            (8, self.i18n.t("key-code")),
            (10, self.i18n.t("key-quit")),
        ]
    }

    /// The viewer's status line.
    pub fn viewer_status(&self, status: &sc_view::Status) -> String {
        let mut text = self.i18n.t_args(
            "viewer-status",
            &[
                ("mode", status.mode.letter().to_string().into()),
                ("codepage", status.codepage.to_string().into()),
                ("offset", (status.offset as i64).into()),
                ("len", (status.len as i64).into()),
                ("percent", (status.percent as i64).into()),
            ],
        );
        if status.wrap != sc_view::Wrap::Off {
            text.push_str(&format!("  {}", status.wrap.as_str()));
        }
        if status.follow {
            text.push_str(&format!("  {}", self.i18n.t("key-follow")));
        }
        if let Some(query) = &status.search {
            text.push_str(&format!("  /{query}"));
        }
        text
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

/// Commands the viewer handles itself when it is open.
fn is_viewer_movement(cmd: Command) -> bool {
    matches!(
        cmd,
        Command::NavUp
            | Command::NavDown
            | Command::NavPageUp
            | Command::NavPageDown
            | Command::NavTop
            | Command::NavBottom
            | Command::ViewerWrap
            | Command::ViewerHex
            | Command::ViewerModeNext
            | Command::ViewerCodepageNext
            | Command::ViewerFollow
            | Command::ViewerLeft
            | Command::ViewerRight
            | Command::ViewerUndoPosition
            | Command::ViewerBytesLess
            | Command::ViewerBytesMore
    )
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

    fn job_request(app: &mut App) -> (u64, JobSpec) {
        match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Job { .. }))
        {
            Some(Request::Job { id, spec, .. }) => (id, spec),
            other => panic!("expected a job request, got {other:?}"),
        }
    }

    #[test]
    fn f5_prompts_with_the_passive_panel_prefilled() {
        let mut app = app();
        press(&mut app, "Down"); // docs/
        press(&mut app, "F5");
        match &app.mode {
            Mode::Prompt(p) => {
                assert_eq!(p.kind, PromptKind::CopyTarget);
                assert!(p.text.starts_with("/tmp"), "{}", p.text);
                assert!(p.text.ends_with('/'), "{}", p.text);
                assert!(p.title.contains("docs"), "{}", p.title);
            }
            other => panic!("expected a prompt, got {other:?}"),
        }
    }

    #[test]
    fn f5_then_enter_starts_a_copy_job_for_the_item_under_the_cursor() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.kind, JobKind::Copy);
        assert_eq!(spec.sources, vec![VPath::local("/home/mb/a.txt")]);
        assert_eq!(spec.target, Some(VPath::local("/tmp")));
        assert!(matches!(app.mode, Mode::Job(_)), "{:?}", app.mode);
    }

    #[test]
    fn f6_moves_the_whole_selection() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Ins"); // a.txt
        press(&mut app, "Ins"); // b.rs
        press(&mut app, "F6");
        press(&mut app, "Enter");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.kind, JobKind::Move);
        assert_eq!(spec.sources.len(), 2);
        assert!(spec.sources.contains(&VPath::local("/home/mb/a.txt")));
    }

    #[test]
    fn f7_creates_a_directory_relative_to_the_panel() {
        let mut app = app();
        press(&mut app, "F7");
        for c in ["n", "e", "w"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.kind, JobKind::Mkdir);
        assert_eq!(spec.target, Some(VPath::local("/home/mb/new")));
        assert!(spec.sources.is_empty());
    }

    #[test]
    fn f8_confirms_before_deleting_and_shows_what_it_will_delete() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        press(&mut app, "F8");
        match &app.mode {
            Mode::Confirm(c) => {
                assert!(c.lines.iter().any(|l| l.contains("a.txt")), "{:?}", c.lines);
                assert_eq!(c.spec.kind, JobKind::Delete);
            }
            other => panic!("expected a confirmation, got {other:?}"),
        }
        assert!(app.take_requests().is_empty(), "deleted before confirming");
        press(&mut app, "Enter");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.sources, vec![VPath::local("/home/mb/a.txt")]);
    }

    #[test]
    fn esc_at_the_confirmation_deletes_nothing() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F8");
        press(&mut app, "Esc");
        assert!(app.mode.is_normal());
        assert!(app.take_requests().is_empty());
    }

    #[test]
    fn an_operation_with_nothing_to_act_on_says_so() {
        let mut app = app();
        app.panels[0].cursor_to(0); // the `..` entry is never a source
        press(&mut app, "F5");
        assert!(app.status_error, "{}", app.status);
        assert!(app.mode.is_normal());
    }

    #[test]
    fn a_conflict_is_answered_through_the_dialog() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);

        let (reply, answers) = std::sync::mpsc::channel();
        app.handle_event(Event::JobQuestion {
            id,
            question: Question::Conflict {
                src: VPath::local("/home/mb/docs"),
                dst: VPath::local("/tmp/docs"),
                src_size: 10,
                dst_size: 20,
                src_newer: true,
            },
            reply,
        });
        // Overwrite, All, Skip, Skip all, Cancel → pick "Skip".
        press(&mut app, "Right");
        press(&mut app, "Right");
        press(&mut app, "Enter");
        assert_eq!(answers.recv().unwrap(), Decision::Skip);
        // The dialog is gone, the job keeps running.
        assert!(app.mode.job().unwrap().question.is_none());
    }

    #[test]
    fn esc_during_a_question_cancels_the_job() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);
        let (reply, answers) = std::sync::mpsc::channel();
        app.handle_event(Event::JobQuestion {
            id,
            question: Question::Error {
                path: VPath::local("/tmp/docs"),
                error: "permission denied".into(),
            },
            reply,
        });
        press(&mut app, "Esc");
        assert_eq!(answers.recv().unwrap(), Decision::Cancel);
    }

    #[test]
    fn a_question_from_a_job_we_are_not_showing_is_never_left_hanging() {
        let mut app = app();
        let (reply, answers) = std::sync::mpsc::channel();
        app.handle_event(Event::JobQuestion {
            id: 999,
            question: Question::Error {
                path: VPath::local("/x"),
                error: "boom".into(),
            },
            reply,
        });
        assert_eq!(
            answers.recv().unwrap(),
            Decision::Cancel,
            "an unanswerable question must not block the engine for ever"
        );
    }

    #[test]
    fn esc_cancels_a_running_job() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let cancel = match app.take_requests().into_iter().next() {
            Some(Request::Job { cancel, .. }) => cancel,
            other => panic!("{other:?}"),
        };
        assert!(!cancel.is_cancelled());
        press(&mut app, "Esc");
        assert!(cancel.is_cancelled());
        assert!(app.status.contains("cancel"), "{}", app.status);
    }

    #[test]
    fn progress_only_updates_the_job_it_belongs_to() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);
        app.handle_event(Event::JobProgress {
            id,
            progress: Progress {
                files_done: 3,
                files_total: 10,
                ..Progress::default()
            },
        });
        assert_eq!(app.mode.job().unwrap().progress.files_done, 3);
        app.handle_event(Event::JobProgress {
            id: id + 100,
            progress: Progress {
                files_done: 99,
                ..Progress::default()
            },
        });
        assert_eq!(app.mode.job().unwrap().progress.files_done, 3);
    }

    #[test]
    fn finishing_a_job_rereads_both_panels_and_reports() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Ins");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);
        app.handle_event(Event::JobDone {
            id,
            kind: JobKind::Copy,
            outcome: Outcome {
                files: 2,
                bytes: 2048,
                ..Outcome::default()
            },
        });
        assert!(app.mode.is_normal());
        assert!(app.status.contains("2"), "{}", app.status);
        assert!(app.status.contains("2.00 K"), "{}", app.status);
        assert!(
            app.panel().selection.is_empty(),
            "the selection should be cleared"
        );
        let reads: Vec<usize> = app
            .take_requests()
            .iter()
            .filter_map(|r| match r {
                Request::Read { panel, .. } => Some(*panel),
                _ => None,
            })
            .collect();
        assert_eq!(reads, vec![0, 1], "both panels must be refreshed");
    }

    #[test]
    fn errors_from_a_job_are_shown_not_swallowed() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F5");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);
        app.handle_event(Event::JobDone {
            id,
            kind: JobKind::Copy,
            outcome: Outcome {
                files: 1,
                errors: vec![(VPath::local("/tmp/x"), "permission denied".into())],
                ..Outcome::default()
            },
        });
        assert!(app.status_error);
        match &app.mode {
            Mode::Message { lines, .. } => {
                assert!(lines[0].contains("permission denied"), "{lines:?}")
            }
            other => panic!("expected the error list, got {other:?}"),
        }
    }

    #[test]
    fn delete_can_be_configured_not_to_ask() {
        let mut app = app();
        app.cfg.general.confirm_delete = false;
        press(&mut app, "Down");
        press(&mut app, "F8");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.kind, JobKind::Delete);
    }

    /// A panel over a real directory, so `F3` can open a real file.
    fn app_on_disk(dir: &std::path::Path) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.join("sc.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir),
            right: VPath::local(dir),
        });
        app.requests.clear();
        let entries = sc_vfs::local::list_blocking(&VPath::local(dir), &CancelToken::new())
            .expect("the test directory");
        for panel in 0..2 {
            app.handle_event(Event::Listed {
                panel,
                generation: 1,
                result: Ok(entries.clone()),
            });
        }
        app.viewer_size = (40, 4);
        app
    }

    #[test]
    fn f3_opens_the_viewer_on_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("notes.txt"),
            b"one
two
three
four
five
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());

        press(&mut app, "Down"); // notes.txt
        press(&mut app, "F3");
        let viewer = app.mode.viewer().expect("the viewer should be open");
        assert_eq!(viewer.path.display_name(), "notes.txt");
        assert_eq!(viewer.mode, sc_view::Mode::Text);

        // Movement goes to the viewer, not to the panel.
        let cursor_before = app.panel().cursor;
        press(&mut app, "Down");
        press(&mut app, "Down");
        assert_eq!(app.panel().cursor, cursor_before, "the panel moved instead");
        let rows = app
            .mode
            .viewer_mut()
            .unwrap()
            .rows(40, 2)
            .into_iter()
            .map(|r| r.text)
            .collect::<Vec<_>>();
        assert_eq!(rows, vec!["three", "four"]);

        press(&mut app, "Esc");
        assert!(app.mode.is_normal(), "Esc should close the viewer");
    }

    #[test]
    fn the_viewer_toggles_wrap_hex_and_codepage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.txt"),
            b"hello hex
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");

        press(&mut app, "F4");
        assert_eq!(app.mode.viewer().unwrap().mode, sc_view::Mode::Hex);
        press(&mut app, "F4");
        assert_eq!(app.mode.viewer().unwrap().mode, sc_view::Mode::Text);

        press(&mut app, "F2");
        assert_eq!(app.mode.viewer().unwrap().wrap, sc_view::Wrap::Char);
        assert!(app.status.contains("wrap"), "{}", app.status);

        press(&mut app, "F8");
        assert_eq!(app.mode.viewer().unwrap().codepage.name(), "UTF-16LE");
        assert!(app.status.contains("codepage"), "{}", app.status);
    }

    #[test]
    fn searching_in_the_viewer_finds_and_reports() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("log.txt"),
            b"starting
WARNING: disk full
done
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");

        press(&mut app, "F7");
        assert!(matches!(app.mode, Mode::Prompt(_)), "{:?}", app.mode);
        for c in ["d", "i", "s", "k"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let viewer = app.mode.viewer().expect("the viewer should be back");
        assert_eq!(viewer.query(), Some("disk"));
        assert!(viewer.last_match.is_some(), "nothing was found");

        // A query that is not there says so and keeps the viewer open.
        press(&mut app, "F7");
        for c in ["z", "z", "z"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("zzz"), "{}", app.status);
        assert!(app.mode.viewer().is_some());
    }

    #[test]
    fn esc_in_the_search_prompt_returns_to_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.txt"),
            b"text
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "F7");
        press(&mut app, "Esc");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
    }

    #[test]
    fn goto_accepts_offsets_hex_and_percentages() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("a.txt"),
            b"0123456789
abcdefghij
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");

        press(&mut app, "Alt+F8");
        for c in ["1", "1"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(app.mode.viewer().unwrap().top, 11);

        press(&mut app, "Alt+F8");
        for c in ["0", "%"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(app.mode.viewer().unwrap().top, 0);

        press(&mut app, "Alt+F8");
        for c in ["n", "o", "p", "e"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert!(app.status_error, "a bad offset should be reported");
    }

    #[test]
    fn f3_on_a_directory_measures_it_instead_of_opening_a_viewer() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down"); // sub/
        press(&mut app, "F3");
        assert!(app.mode.is_normal());
        assert!(matches!(
            app.take_requests().first(),
            Some(Request::DirSize { .. })
        ));
    }

    #[test]
    fn opening_a_file_that_vanished_reports_the_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gone.txt"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        std::fs::remove_file(dir.path().join("gone.txt")).unwrap();
        press(&mut app, "F3");
        assert!(app.status_error, "{}", app.status);
        assert!(app.mode.is_normal());
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
