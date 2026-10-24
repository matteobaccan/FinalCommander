use std::sync::mpsc::Sender;

use scmd_cmd::{Command, Key, KeyCode, Keymap, Mods, Scope, Side};
use scmd_config::DecompileRule;
use scmd_config::{Config, Paths, QuickSearch};
use scmd_core::{CancelToken, Entry, MaskScope, Sort, SortKey, VPath, ViewMode};
use scmd_edit::{BlockKind, Editor, HexEdit};
use scmd_i18n::I18n;
use scmd_jobs::{Decision, JobKind, JobSpec, Outcome, Progress, Question};
use scmd_net::{SecretKind, Site};
use scmd_tui::Input;
use scmd_ui::{Theme, ThemeSet};
use scmd_view::{FileSource, Mode as ViewerMode, SearchMode, Viewer};

use crate::cmdline::{self, CmdLine, Internal, Parsed, Prefix};
use crate::expand;
use crate::form::{Form, Pick, PickRow, TextEdit};
use crate::menu::{self, Action, FormKind, Mark, PickKind};
use crate::panel::Panel;

/// Work the application wants done off the UI thread. The runner drains these.
#[derive(Debug, Clone, PartialEq)]
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
    /// Walk and match. The engine is `scmd-find`; this carries what the dialog asked for.
    Find {
        id: u64,
        spec: Box<FindSpec>,
        cancel: CancelToken,
    },
    /// Re-stat the paths a list panel is made of, and drop what is no longer there.
    TempRefresh {
        panel: usize,
        generation: u64,
        title: String,
        paths: Vec<VPath>,
    },
    /// Read one directory for the tree panel: only its subdirectories matter.
    TreeRead {
        panel: usize,
        path: VPath,
    },
    /// Build an archive. A job as far as the user is concerned — the same progress box, the
    /// same counters — but the writing is one pass through `scmd-arc` rather than a plan of
    /// file copies, because a central directory or a compressed stream is not a sequence of
    /// independent files.
    Pack {
        id: u64,
        archive: VPath,
        format: scmd_arc::Format,
        level: scmd_arc::Level,
        sources: Vec<VPath>,
        cancel: CancelToken,
    },
    /// Run a plugin's command (SPEC 08 §1). Off the interface thread, because it is somebody
    /// else's program and we do not know how long it will think about it.
    Plugin {
        id: String,
        label: String,
        program: std::path::PathBuf,
        cwd: Option<std::path::PathBuf>,
        ctx: Box<scmd_plug::host::Context>,
        timeout: Option<u64>,
    },
    /// Create a link. Not a job: it is one syscall, with nothing to report progress on.
    Link {
        target: VPath,
        link: VPath,
        kind: scmd_jobs::LinkKind,
    },
    /// Run an external decompiler and show its output in the viewer.
    Decompile {
        id: u64,
        rule: DecompileRule,
        path: VPath,
    },
    /// Run a command line through the user's shell, off the UI thread.
    Shell {
        id: u64,
        line: String,
        cwd: std::path::PathBuf,
        /// Where to put the output instead of the user screen (`run:file<command`).
        into: Option<std::path::PathBuf>,
    },
    /// Run a file operation off the UI thread.
    Job {
        id: u64,
        spec: JobSpec,
        cancel: CancelToken,
    },
    /// Open a connection (or reuse the one that is already open) and show it in `panel`.
    Connect {
        panel: usize,
        site: Box<Site>,
        /// Password or key passphrase, when the connection asked for one.
        secret: Option<String>,
        /// Set only after the user has seen the fingerprint and accepted it.
        accept_host_key: bool,
    },
    /// Close a connection.
    Disconnect {
        authority: String,
    },
    /// Copy a remote file into a temporary directory so the viewer or the editor can open it.
    Fetch {
        id: u64,
        path: VPath,
        purpose: Fetched,
    },
    /// Write a temporary file back to the server it came from (SPEC 04 §5).
    Upload {
        id: u64,
        local: std::path::PathBuf,
        remote: VPath,
        /// What the file looked like when it was fetched; a change on the server is reported
        /// rather than overwritten.
        expect: Option<RemoteStamp>,
        force: bool,
    },
}

/// Why a remote file is being fetched.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fetched {
    /// `F3`: read-only, and the user is told it is a copy.
    View,
    /// `F4`: edit here, write back on save.
    Edit,
}

/// A remote file open in the editor: the bytes live in a local copy, and a save puts them
/// back where they came from (SPEC 04 §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteEdit {
    pub remote: VPath,
    pub local: std::path::PathBuf,
    pub stamp: Option<RemoteStamp>,
}

/// Enough of a remote file to notice that someone else has changed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteStamp {
    pub size: u64,
    pub mtime: Option<u32>,
}

/// What comes back.
#[derive(Debug)]
pub enum Event {
    Input(Input),
    /// A plugin's command has finished: what it asked for, or why it could not.
    PluginDone {
        id: String,
        label: String,
        result: Result<scmd_plug::host::Reply, String>,
    },
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
    /// Free space on the device a panel's directory is on, which the read reports separately
    /// because a listing is the same work whether or not the platform can answer this.
    FreeSpace {
        panel: usize,
        bytes: Option<u64>,
    },
    /// One file a search matched.
    FindHit {
        id: u64,
        path: VPath,
        entry: Box<Entry>,
    },
    /// Where a search has got to, so the list can say it is still working.
    FindProgress {
        id: u64,
        scanning: String,
        files: u64,
        dirs: u64,
    },
    /// A search finished, was stopped, or could not start.
    FindDone {
        id: u64,
        files: u64,
        dirs: u64,
        elapsed: std::time::Duration,
        problems: Vec<String>,
        error: Option<String>,
    },
    /// A list panel's rows, with the real path of each: find results, `panelize`, a re-read.
    TempListed {
        panel: usize,
        generation: u64,
        title: String,
        items: Vec<(VPath, Entry)>,
    },
    /// A link was created, or could not be.
    LinkDone {
        link: VPath,
        error: Option<String>,
    },
    /// A shell command finished: its merged output, capped, and how it ended.
    ShellDone {
        id: u64,
        line: String,
        output: String,
        status: Option<i32>,
    },
    JobProgress {
        id: u64,
        progress: Progress,
    },
    /// One directory read for the tree panel, as the subdirectories it holds.
    TreeListed {
        panel: usize,
        path: VPath,
        dirs: Vec<VPath>,
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
    Decompiled {
        id: u64,
        tool: String,
        result: Result<String, String>,
    },
    Connected {
        panel: usize,
        site: Box<Site>,
        outcome: ConnectOutcome,
    },
    Disconnected {
        authority: String,
        /// Every host still connected, so the site list stays truthful.
        connected: Vec<String>,
    },
    Fetched {
        id: u64,
        remote: VPath,
        purpose: Fetched,
        /// The local copy and what the file looked like on the server.
        result: Result<(std::path::PathBuf, RemoteStamp), String>,
    },
    Uploaded {
        id: u64,
        remote: VPath,
        result: Result<u64, UploadError>,
    },
}

/// Why a write-back did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    /// The file changed on the server since it was fetched.
    Changed,
    Failed(String),
}

/// How a connection attempt ended. A connection that needs something says what it needs,
/// instead of failing with "authentication failed" and leaving the user to guess.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectOutcome {
    /// Connected; the panel opens here.
    Open {
        start: VPath,
        connected: Vec<String>,
    },
    NeedsSecret(SecretKind),
    /// The host is not in `known_hosts` yet; the user decides, with the fingerprint in front
    /// of them.
    UnknownHostKey {
        fingerprint: String,
    },
    Failed(String),
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
    EditorSearch,
    EditorReplaceFrom,
    EditorReplaceWith,
    EditorGoto,
    EditorSaveAs,
    /// Destination for F6.
    MoveTarget,
    /// Name for F7.
    Mkdir,
    /// `Ctrl+G`: one command line, run once per selected file (SPEC 06 §5).
    ApplyCommand,
    /// `Alt+F6`: where the new link goes. The target travels in `Prompt::sources`.
    LinkName,
    /// The byte to fill a block with, in hex.
    HexFill,
    /// The key to exclusive-or a block with, in hex.
    HexXor,
    /// Where to write the marked bytes.
    HexWrite,
    /// A file to read into the buffer at the cursor.
    HexRead,
    /// How many bytes to insert at the cursor.
    HexInsert,
    /// What a just-recorded macro is for, in the author's own words.
    MacroDescription {
        key: Key,
        scope: scmd_cmd::Scope,
        steps: Vec<scmd_cmd::Step>,
    },
    /// One instruction to assemble over the bytes at the cursor (SPEC 05 §2.8).
    Assemble,
    /// `offset length`, in hex: bytes from elsewhere in this same file, copied to the cursor.
    HexCopy,
    /// The same, but the source is removed — the bytes move rather than multiply.
    HexMove,
    /// One line of arithmetic, answered in every base at once.
    Calculator,
    /// A URL to connect to: `sftp://mb@host/srv`.
    NetConnect,
    /// A password or a key passphrase. Never echoed, never stored.
    NetSecret(SecretKind),
    /// Quick search; remembers where the cursor was so Esc can put it back.
    QuickSearch {
        origin: usize,
    },
    /// A command line that asked for a value before it could run (SPEC 06 §5b): the text with
    /// `{answer}` where the answer goes.
    CommandAnswer {
        pending: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prompt {
    pub kind: PromptKind,
    pub title: String,
    pub text: String,
    /// Render as `****`: a password is never put on the screen.
    pub secret: bool,
    /// The items the answer applies to. Empty means "whatever is selected when the answer
    /// comes in", which is what every prompt wants except the ones that deliberately work on
    /// the item under the cursor alone (`Shift+F5`, `Shift+F6`).
    pub sources: Vec<VPath>,
}

impl Prompt {
    pub fn new(kind: PromptKind, title: String, text: String) -> Prompt {
        Prompt {
            kind,
            title,
            text,
            secret: false,
            sources: Vec::new(),
        }
    }
}

/// What a confirmation will do on `Enter`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    /// Start this file operation.
    Job(JobSpec),
    /// Leave the editor, losing unsaved changes.
    DiscardEdits,
    /// The same, for the hex editor: its buffer is bytes, not lines.
    DiscardHex,
    /// Save over a file that changed on disk since it was opened.
    OverwriteChanged,
    /// Write the editor's copy over a remote file that changed on the server.
    OverwriteRemote,
    /// Leave the program with screens in the background that hold unsaved changes.
    QuitWithScreens,
    /// Play a macro that deletes or overwrites something, having said so first.
    PlayMacro(Box<scmd_cmd::Macro>),
    /// Record a host key we have never seen and connect.
    AcceptHostKey {
        panel: usize,
        site: Box<Site>,
        secret: Option<String>,
    },
}

/// A yes/no question that carries what it will do on `Enter`.
#[derive(Debug, Clone, PartialEq)]
pub struct Confirm {
    pub title: String,
    pub lines: Vec<String>,
    pub action: ConfirmAction,
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

/// What to search for (SPEC 06 §1). Plain data on purpose: the engine lives in `scmd-find`,
/// and this is what the dialog produces and the worker turns into a query, so neither the
/// dialog nor the screen depends on the engine's own types.
#[derive(Debug, Clone, PartialEq)]
pub struct FindSpec {
    /// Masks, in the grammar of `scmd-core`'s filter: a list, `*`, `?`, ranges, groups.
    pub mask: String,
    /// Text or a hex pattern the file must contain. Empty means "match on the name alone".
    pub text: String,
    pub case_sensitive: bool,
    pub whole_words: bool,
    /// The text above is a hex byte pattern rather than characters.
    pub hex: bool,
    /// Files that do *not* contain it, which FAR offers and is surprisingly useful.
    pub not_containing: bool,
    pub scope: FindScope,
    pub follow_links: bool,
    /// Directories in the results, not only files.
    pub directories: bool,
    /// Where the walk starts. Worked out from the scope when the dialog is accepted, so the
    /// worker never has to know what a panel is.
    pub roots: Vec<VPath>,
}

impl Default for FindSpec {
    fn default() -> Self {
        FindSpec {
            // Every file, which is what a user who presses Enter straight away means.
            mask: "*".to_string(),
            text: String::new(),
            case_sensitive: false,
            whole_words: false,
            hex: false,
            not_containing: false,
            scope: FindScope::Tree,
            follow_links: false,
            directories: false,
            roots: Vec::new(),
        }
    }
}

/// Where a search starts. FAR has six; these are the five that mean something on every
/// platform we run on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindScope {
    /// This panel's directory and everything below it.
    Tree,
    /// This panel's directory alone.
    Panel,
    /// The directories selected in this panel, each with everything below it.
    Selection,
    /// From the root of the drive this panel is on.
    Root,
    /// The other panel's directory and everything below it.
    Other,
}

impl FindScope {
    pub fn as_str(self) -> &'static str {
        match self {
            FindScope::Tree => "tree",
            FindScope::Panel => "panel",
            FindScope::Selection => "selection",
            FindScope::Root => "root",
            FindScope::Other => "other",
        }
    }

    pub fn parse(text: &str) -> FindScope {
        match text {
            "panel" => FindScope::Panel,
            "selection" => FindScope::Selection,
            "root" => FindScope::Root,
            "other" => FindScope::Other,
            _ => FindScope::Tree,
        }
    }
}

/// A search while it runs, and its results afterwards.
#[derive(Debug)]
pub struct FindView {
    pub id: u64,
    /// What was asked for, for the frame.
    pub label: String,
    pub rows: Vec<(VPath, Entry)>,
    pub cursor: usize,
    pub top: usize,
    /// The directory being scanned right now, while it is running.
    pub scanning: Option<String>,
    pub files: u64,
    pub dirs: u64,
    pub done: bool,
    pub stopped: bool,
    pub elapsed: std::time::Duration,
    pub cancel: CancelToken,
    /// What could not be read. Kept rather than shown one by one, because a search over a
    /// whole drive hits a dozen unreadable directories and saying so each time is noise.
    pub problems: Vec<String>,
}

impl FindView {
    pub fn move_cursor(&mut self, delta: isize, page: usize) {
        if self.rows.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.rows.len() - 1;
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, last as isize) as usize;
        // Keep the cursor on screen, which for a list that grows means following it down.
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if page > 0 && self.cursor >= self.top + page {
            self.top = self.cursor + 1 - page;
        }
    }
}

#[derive(Debug)]
pub enum Mode {
    Normal,
    Prompt(Prompt),
    Confirm(Confirm),
    Job(JobView),
    /// `F3`: the internal viewer, full screen.
    Viewer(Box<Viewer>),
    /// `F4`: the internal editor, full screen.
    Editor(Box<Editor>),
    /// `F4` on a binary: the same editor, editing bytes (SPEC 05 §2.5).
    Hex(Box<HexEdit>),
    /// `F1`: the help, a book with chapters (SPEC 00 §4). `Chapter::Index` is its table of
    /// contents, which is what the key opens.
    Help {
        chapter: crate::help::Chapter,
        scroll: usize,
        /// Where the index's cursor was, so coming back lands on the chapter one read.
        at: usize,
    },
    Message {
        title: String,
        lines: Vec<String>,
    },
    /// The connection manager: the site list plus whatever is connected.
    Sites {
        cursor: usize,
    },
    /// The menu bar (`F9`): which menu is open and which entry is under the cursor.
    Menu {
        menu: usize,
        item: usize,
    },
    /// A settings dialog.
    Form(Box<Form>),
    /// A list to choose one row from.
    Pick(Box<Pick>),
    /// `Alt+F7`: a search, running or finished, with its results.
    Find(Box<FindView>),
    /// Recording has stopped and the macro is waiting for the key it will answer to: the next
    /// key pressed is the answer, which is how FAR assigns one (SPEC 08 §5).
    MacroKey {
        steps: Vec<scmd_cmd::Step>,
    },
    /// `Ctrl+O`: the output of the commands that have run, full screen.
    UserScreen {
        scroll: usize,
    },
}

impl Mode {
    pub fn is_normal(&self) -> bool {
        matches!(self, Mode::Normal)
    }

    /// A screen is something one can step away from and come back to: a viewer, an editor,
    /// the hex editor. A dialog is not — it is a question about the screen under it, and
    /// leaving it half-answered in the background would be a way to lose work quietly.
    ///
    /// The string is what the screens list shows: the kind and the file.
    pub fn screen_title(&self) -> Option<String> {
        match self {
            Mode::Viewer(v) => Some(format!("View  {}", v.path.display_name())),
            Mode::Editor(e) => Some(format!("Edit  {}", e.name())),
            Mode::Hex(e) => Some(format!("Hex   {}", e.name())),
            _ => None,
        }
    }

    /// Does this screen hold changes nobody has written yet? The screens list says so, and
    /// quitting the program asks about it.
    pub fn screen_modified(&self) -> bool {
        match self {
            Mode::Editor(e) => e.buffer.modified(),
            Mode::Hex(e) => e.modified(),
            _ => false,
        }
    }
}

impl Mode {
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

    pub fn editor(&self) -> Option<&Editor> {
        match self {
            Mode::Editor(e) => Some(e),
            _ => None,
        }
    }

    pub fn editor_mut(&mut self) -> Option<&mut Editor> {
        match self {
            Mode::Editor(e) => Some(e),
            _ => None,
        }
    }

    pub fn hex(&self) -> Option<&HexEdit> {
        match self {
            Mode::Hex(e) => Some(e),
            _ => None,
        }
    }

    pub fn hex_mut(&mut self) -> Option<&mut HexEdit> {
        match self {
            Mode::Hex(e) => Some(e),
            _ => None,
        }
    }
}

/// Everything the application is started with. Grouping it keeps `App::new` honest and makes
/// the test setup read like the real startup path.
/// A command a plugin provides, with everything needed to run it resolved (SPEC 08 §1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginCommand {
    /// Which plugin it came from, for the list and for any complaint about it.
    pub plugin: String,
    pub id: String,
    pub label: String,
    pub scope: scmd_cmd::Scope,
    pub program: std::path::PathBuf,
    pub needs: Vec<String>,
    pub timeout: Option<u64>,
}

impl PluginCommand {
    fn wants(&self, what: &str) -> bool {
        self.needs.iter().any(|n| n == what)
    }
}

pub struct Startup {
    pub cfg: Config,
    pub paths: Paths,
    pub i18n: I18n,
    pub keymap: Keymap,
    pub themes: ThemeSet,
    pub theme: Theme,
    pub left: VPath,
    pub right: VPath,
    /// The netbox's site list, from `[[site]]` blocks.
    pub sites: Vec<Site>,
    /// The commands the loaded plugins provide.
    pub plugin_commands: Vec<PluginCommand>,
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
    /// The editor, parked while one of its prompts or confirmations is open.
    editor_parked: Option<Box<Editor>>,
    /// The replacement text waiting for its second prompt.
    replace_from: Option<String>,
    /// Size of the editor's text area, from the last frame.
    pub editor_size: (u16, u16),
    /// The decompilation we are waiting for, if any.
    decompile_id: Option<u64>,
    /// The netbox's site list and what is connected right now.
    pub sites: Vec<Site>,
    pub connected: Vec<String>,
    /// The connection we are in the middle of opening, kept so a password prompt or a
    /// host-key confirmation can resume it.
    pending_connect: Option<(usize, Box<Site>, Option<String>)>,
    /// The fetch we are waiting for, if any.
    fetch_id: Option<u64>,
    /// The command line under the panels (SPEC 06 §7).
    pub cmd: CmdLine,
    /// What the commands that have run printed, for `Ctrl+O`. Capped.
    pub user_screen: Vec<String>,
    /// Which panel is hidden, if either: `Ctrl+F1` and `Ctrl+F2` give the screen to one.
    pub hidden: Option<usize>,
    /// What was searched for last, so `Alt+F7` opens on it rather than on an empty dialog.
    pub last_find: FindSpec,
    /// The tree panel's rows, per panel, and the directory cache behind them. The cache
    /// outlives the panel, so leaving the tree and coming back costs no reads.
    pub trees: [Option<Box<crate::tree::Tree>>; 2],
    pub tree_cache: crate::tree::Cache,
    /// The hex editor, while a dialog of its own is open over it.
    hex_parked: Option<Box<HexEdit>>,
    /// The quick view's own reader, and what it is of: opened when the cursor lands on a file
    /// and kept while it stays there, so moving down a directory does not reopen a file.
    pub quick: Option<Box<Viewer>>,
    quick_of: Option<VPath>,
    /// Why there is nothing to show, when there is not: an empty frame looks broken.
    pub quick_why: Option<String>,
    /// The quick view's drawing area, so a page scroll in it is a page of what is on screen.
    pub quick_size: (u16, u16),
    /// The last sum, so the calculator opens on it.
    pub last_calc: String,
    /// What the plugins provide (SPEC 08 §1): `F11` lists these.
    pub plugin_commands: Vec<PluginCommand>,
    /// Recorded macros (SPEC 08 §5), as `scmd-macros.toml` holds them.
    pub macros: scmd_cmd::Macros,
    /// The steps gathered so far, while `Ctrl+.` is recording.
    pub recording: Option<Vec<scmd_cmd::Step>>,
    /// The key being dispatched, while recording. `run` takes it to mean "this key became a
    /// command"; whatever is left after dispatch was consumed as a keystroke instead.
    record_key: Option<Key>,
    /// How deep a macro is playing. A macro that plays a macro is allowed; one that plays
    /// itself is not, and this is what says so.
    playing: usize,
    /// The screens one has stepped away from: a viewer or an editor left open while the
    /// panels, or another file, are worked on (SPEC 02 §7). The panels are not in this list —
    /// they are what is underneath, always there, and never closed.
    pub background: Vec<Mode>,
    /// The last instruction assembled, so `F9` opens on it: patching is repetitive work.
    pub last_asm: Option<String>,
    /// The archive format the last pack used, so the next one opens on it.
    pub last_pack: scmd_arc::Format,
    /// The same for the replace dialog.
    pub last_replace: scmd_jobs::ReplaceOptions,
    /// A replacement runs twice: once to count, and then, if the user says so, for real. This
    /// is the real one, waiting for the count to come back.
    pending_replace: Option<JobSpec>,
    /// Which group gives a file its colour. Compiled once: it parses mask lists.
    pub highlight: scmd_ui::Highlight,
    /// What `Ctrl+G` has left to run, one command per file. They go one at a time: a hundred
    /// copies of a converter started at once is not what anybody asked for.
    apply_queue: Vec<String>,
    /// How many commands that run started with, for counting through it.
    apply_total: usize,
    /// The shell command we are waiting for, if any, and the line that started it.
    shell: Option<(u64, String)>,
    /// Where each panel was before the last `cd`, so `cd -` has somewhere to go.
    prev_dir: [Option<VPath>; 2],
    /// The file the editor is editing on behalf of a server: where it came from, the local
    /// copy, and what it looked like when it was fetched.
    remote_edit: Option<RemoteEdit>,
    /// The write-back we are waiting for, if any.
    upload_id: Option<u64>,
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
            sites,
            plugin_commands,
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
        // The macros, read before the struct is built because `paths` moves into it. A file
        // that does not parse is said out loud rather than silently ignored: a macro half
        // read is a key that does something unexpected.
        let mut macro_error: Option<String> = None;
        let macros = match std::fs::read_to_string(&paths.macros_file) {
            Ok(text) => match scmd_cmd::Macros::parse(&text) {
                Ok(macros) => macros,
                Err(e) => {
                    macro_error = Some(e.to_string());
                    scmd_cmd::Macros::default()
                }
            },
            Err(_) => scmd_cmd::Macros::default(),
        };
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
            editor_parked: None,
            replace_from: None,
            editor_size: (80, 20),
            decompile_id: None,
            sites,
            connected: Vec::new(),
            pending_connect: None,
            fetch_id: None,
            cmd: CmdLine::new(),
            user_screen: Vec::new(),
            hidden: None,
            last_find: FindSpec::default(),
            hex_parked: None,
            trees: [None, None],
            tree_cache: crate::tree::Cache::default(),
            quick: None,
            quick_of: None,
            quick_why: None,
            quick_size: (40, 20),
            last_calc: String::new(),
            plugin_commands,
            macros,
            recording: None,
            record_key: None,
            playing: 0,
            background: Vec::new(),
            last_asm: None,
            last_pack: scmd_arc::Format::Zip,
            last_replace: scmd_jobs::ReplaceOptions::default(),
            pending_replace: None,
            highlight: scmd_ui::Highlight::far_default(),
            apply_queue: Vec::new(),
            apply_total: 0,
            shell: None,
            prev_dir: [None, None],
            remote_edit: None,
            upload_id: None,
            next_job_id: 1,
        };
        app.read_panel(0);
        app.read_panel(1);
        if let Some(e) = macro_error {
            app.set_error(e);
        }
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
        // A list panel is not a directory: re-reading it means asking again about the files it
        // holds, which is what keeps results honest after a delete or a move.
        if let Some(title) = self.panels[index].temp.as_ref().map(|t| t.title.clone()) {
            let paths = self.panels[index].temp_paths();
            self.requests.push(Request::TempRefresh {
                panel: index,
                generation,
                title,
                paths,
            });
            return;
        }
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
            Event::PluginDone { id, label, result } => match result {
                Ok(reply) => self.apply_plugin_reply(&label, reply),
                Err(e) => {
                    let m = self
                        .i18n
                        .t_args("plugin-failed", &[("id", id.into()), ("why", e.into())]);
                    self.set_error(m);
                }
            },
            Event::Input(input) => self.handle_input(input),
            Event::FindHit { id, path, entry } => {
                if let Some(view) = self.find_mut() {
                    if view.id == id {
                        view.rows.push((path, *entry));
                    }
                }
            }
            Event::FindProgress {
                id,
                scanning,
                files,
                dirs,
            } => {
                if let Some(view) = self.find_mut() {
                    if view.id == id {
                        view.scanning = Some(scanning);
                        view.files = files;
                        view.dirs = dirs;
                    }
                }
            }
            Event::FindDone {
                id,
                files,
                dirs,
                elapsed,
                problems,
                error,
            } => {
                let mine = self.find_mut().map(|v| v.id) == Some(id);
                if let (true, Some(view)) = (mine, self.find_mut()) {
                    view.done = true;
                    view.scanning = None;
                    view.files = files;
                    view.dirs = dirs;
                    view.elapsed = elapsed;
                    view.problems = problems;
                }
                if let Some(error) = error {
                    // A search that could not even start says why, where the dialog was.
                    if mine {
                        self.mode = Mode::Normal;
                    }
                    self.set_error(error);
                }
            }
            Event::TempListed {
                panel,
                generation,
                title,
                items,
            } => {
                if let Some(p) = self.panels.get_mut(panel) {
                    // A listing that arrived after the panel moved on is not thrown away
                    // silently: the generation is what tells them apart.
                    if generation >= p.generation {
                        p.generation = generation;
                        let count = items.len();
                        p.set_temp(title, items, generation);
                        let msg = self
                            .i18n
                            .t_args("panel-list-count", &[("count", (count as i64).into())]);
                        self.set_status(msg);
                    }
                }
            }
            Event::LinkDone { link, error } => match error {
                Some(e) => self.set_error(e),
                None => {
                    let msg = self
                        .i18n
                        .t_args("link-created", &[("name", link.display_name().into())]);
                    self.set_status(msg);
                    self.read_panel(0);
                    self.read_panel(1);
                }
            },
            Event::FreeSpace { panel, bytes } => {
                if let Some(p) = self.panels.get_mut(panel) {
                    p.free = bytes;
                }
            }
            Event::Connected {
                panel,
                site,
                outcome,
            } => self.connection_result(panel, site, outcome),
            Event::Disconnected {
                authority,
                connected,
            } => {
                self.connected = connected;
                let msg = self
                    .i18n
                    .t_args("net-disconnected", &[("host", authority.clone().into())]);
                self.set_status(msg);
                // A panel left on a closed connection goes home instead of showing a corpse.
                for panel in 0..2 {
                    if self.panels[panel].cwd.authority() == Some(authority.as_str()) {
                        self.panels[panel].go(local_home(), true);
                        self.read_panel(panel);
                    }
                }
            }
            Event::Fetched {
                id,
                remote,
                purpose,
                result,
            } => {
                if self.fetch_id != Some(id) {
                    return; // the user moved on
                }
                self.fetch_id = None;
                match (result, purpose) {
                    (Ok((local, _)), Fetched::View) => self.open_viewer_on(&local, remote),
                    (Ok((local, stamp)), Fetched::Edit) => {
                        self.open_remote_editor(remote, local, Some(stamp))
                    }
                    (Err(e), _) => self.set_error(e),
                }
            }
            Event::Uploaded { id, remote, result } => {
                if self.upload_id != Some(id) {
                    return;
                }
                self.upload_id = None;
                match result {
                    Ok(bytes) => {
                        if let Some(edit) = &mut self.remote_edit {
                            // What we just wrote is now what the server has.
                            edit.stamp = Some(RemoteStamp {
                                size: bytes,
                                mtime: None,
                            });
                        }
                        let msg = self.i18n.t_args(
                            "net-written-back",
                            &[
                                ("name", remote.to_string().into()),
                                ("bytes", (bytes as i64).into()),
                            ],
                        );
                        self.set_status(msg);
                        // The listing's size and date are stale now.
                        for panel in 0..2 {
                            if self.panels[panel].cwd.same_provider(&remote) {
                                self.read_panel(panel);
                            }
                        }
                    }
                    Err(UploadError::Changed) => {
                        let title = self.i18n.t("net-changed-title");
                        let body = self
                            .i18n
                            .t_args("net-changed-body", &[("name", remote.to_string().into())]);
                        self.park_editor();
                        self.mode = Mode::Confirm(Confirm {
                            title,
                            lines: vec![body],
                            action: ConfirmAction::OverwriteRemote,
                        });
                    }
                    Err(UploadError::Failed(detail)) => {
                        // The edits are not lost: they are in the local copy, and we say where.
                        let local = self
                            .remote_edit
                            .as_ref()
                            .map(|e| e.local.display().to_string())
                            .unwrap_or_default();
                        let msg = self.i18n.t_args(
                            "net-write-back-failed",
                            &[("error", detail.into()), ("local", local.into())],
                        );
                        self.set_error(msg);
                    }
                }
            }
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
            Event::ShellDone {
                id,
                line,
                output,
                status,
            } => {
                if self.shell.as_ref().map(|(running, _)| *running) != Some(id) {
                    return; // a command we already stopped waiting for
                }
                self.shell = None;
                self.push_user_screen(&line, &output);
                match status {
                    Some(0) | None => {
                        let msg = self
                            .i18n
                            .t_args("cmd-done", &[("command", line.clone().into())]);
                        self.set_status(msg);
                    }
                    Some(code) => {
                        let msg = self.i18n.t_args(
                            "cmd-failed",
                            &[
                                ("command", line.clone().into()),
                                ("code", (code as i64).into()),
                            ],
                        );
                        self.set_error(msg);
                    }
                }
                // A command may have changed what is on disk, so both panels are re-read.
                self.read_panel(0);
                self.read_panel(1);
                // A `Ctrl+G` run carries on where the last command left off, and stops at the
                // first failure rather than running ninety-nine more over a broken one.
                if !self.apply_queue.is_empty() {
                    if matches!(status, Some(code) if code != 0) {
                        let done = self.apply_total - self.apply_queue.len();
                        let total = self.apply_total;
                        self.apply_queue.clear();
                        self.apply_total = 0;
                        let msg = self.i18n.t_args(
                            "cmd-apply-stopped",
                            &[
                                ("command", line.into()),
                                ("done", (done as i64).into()),
                                ("total", (total as i64).into()),
                            ],
                        );
                        self.set_error(msg);
                    } else {
                        self.run_next_apply();
                    }
                }
            }
            Event::TreeListed { panel, path, dirs } => {
                self.tree_cache.insert(path.clone(), dirs);
                // The read came back: finish the expansion that was waiting for it. Only the
                // panel that asked is touched, and only if it is still a tree.
                if self.panels[panel].content != crate::panel::Content::Tree {
                    return;
                }
                let page = self.rows.max(1);
                if let Some(mut tree) = self.trees[panel].take() {
                    let was = tree.cursor;
                    if let Some(at) = tree.rows.iter().position(|r| r.path == path) {
                        tree.cursor = at;
                        tree.expand(&self.tree_cache);
                        tree.cursor = was;
                        tree.move_cursor(0, page);
                    }
                    self.trees[panel] = Some(tree);
                }
                // Still on the way somewhere: open the next level.
                self.tree_reveal(panel);
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
            Event::Decompiled { id, tool, result } => {
                if self.decompile_id != Some(id) {
                    return; // the viewer moved on
                }
                self.decompile_id = None;
                match result {
                    Ok(text) if self.mode.viewer().is_some() => {
                        let name = self
                            .mode
                            .viewer()
                            .map(|v| v.path.display_name())
                            .unwrap_or_default();
                        if let Some(viewer) = self.mode.viewer_mut() {
                            viewer.show_derived(&tool, text);
                        }
                        let msg = self.i18n.t_args(
                            "viewer-derived",
                            &[("producer", tool.into()), ("name", name.into())],
                        );
                        self.set_status(msg);
                    }
                    Ok(_) => {}
                    Err(e) => {
                        let msg = self.i18n.t_args(
                            "viewer-decompile-failed",
                            &[("tool", tool.into()), ("error", e.into())],
                        );
                        self.set_error(msg);
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
                        ("bytes", scmd_core::format_size(bytes).into()),
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
        // A macro answers the key before anything else does, which is what lets one put a
        // macro on `F5` — FAR allows that, and a macro one cannot put on a useful key is a
        // macro one does not record. Not while recording, or a macro would record itself.
        if self.recording.is_none() && self.playing == 0 {
            if let Some(found) = self.macros.find(key, self.scope()).cloned() {
                self.play_macro(&found);
                return;
            }
        }
        // While recording, `run` turns this into a command step; anything it does not take
        // was consumed by a mode as a keystroke, and is recorded as one.
        if self.recording.is_some() {
            self.record_key = Some(key);
        }
        self.dispatch_key(key);
        if let (Some(key), Some(steps)) = (self.record_key.take(), self.recording.as_mut()) {
            steps.push(scmd_cmd::Step::Press(key));
        }
    }

    fn dispatch_key(&mut self, key: Key) {
        // `Mode` owns a reply channel while a job is asking, so it cannot be cloned: take it
        // out, dispatch, and put back whatever the handler did not replace.
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Help {
                chapter,
                scroll,
                at,
            } => {
                self.mode = Mode::Help {
                    chapter,
                    scroll,
                    at,
                };
                self.help_key(key, chapter, scroll, at);
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
            Mode::MacroKey { steps } => self.macro_key(key, steps),
            Mode::UserScreen { scroll } => {
                self.mode = Mode::UserScreen { scroll };
                self.user_screen_key(key, scroll);
            }
            Mode::Find(view) => {
                self.mode = Mode::Find(view);
                self.find_key(key);
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
            Mode::Editor(editor) => {
                self.mode = Mode::Editor(editor);
                self.editor_key(key);
            }
            Mode::Hex(editor) => {
                self.mode = Mode::Hex(editor);
                self.hex_key(key);
            }
            Mode::Sites { cursor } => {
                self.mode = Mode::Sites { cursor };
                self.sites_key(key, cursor);
            }
            Mode::Menu { menu, item } => {
                self.mode = Mode::Menu { menu, item };
                self.menu_key(key, menu, item);
            }
            Mode::Form(form) => {
                self.mode = Mode::Form(form);
                self.form_key(key);
            }
            Mode::Pick(pick) => {
                self.mode = Mode::Pick(pick);
                self.pick_key(key);
            }
            Mode::Normal => self.normal_key(key),
        }
    }

    /// The connection manager's keys: `Enter` connects, `n` types a URL, `d` disconnects,
    /// `Ins`/`F7` stores a new session and `F4` edits the one under the cursor.
    fn sites_key(&mut self, key: Key, cursor: usize) {
        let rows = self.site_rows();
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => self.mode = Mode::Normal,
            KeyCode::Up => {
                self.mode = Mode::Sites {
                    cursor: cursor.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.mode = Mode::Sites {
                    cursor: (cursor + 1).min(rows.len().saturating_sub(1)),
                }
            }
            KeyCode::Home => self.mode = Mode::Sites { cursor: 0 },
            KeyCode::End => {
                self.mode = Mode::Sites {
                    cursor: rows.len().saturating_sub(1),
                }
            }
            // `n` is FAR's "type a URL"; `Ins` and `F7` make a stored session, `F4` edits
            // the one under the cursor (SPEC 10 §3.5).
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.mode = Mode::Normal;
                self.run(Command::NetConnect);
            }
            KeyCode::Insert | KeyCode::F(7) => self.open_site_form(None),
            KeyCode::F(4) => {
                let stored = rows
                    .get(cursor)
                    .and_then(|site| self.cfg.sites.iter().position(|c| c.id == site.id));
                self.open_site_form(stored);
            }
            KeyCode::Char('d') | KeyCode::Char('D') | KeyCode::Delete => {
                if let Some(site) = rows.get(cursor) {
                    let authority = site.authority();
                    if self.connected.contains(&authority) {
                        self.mode = Mode::Normal;
                        self.requests.push(Request::Disconnect { authority });
                    } else {
                        let msg = self
                            .i18n
                            .t_args("net-not-connected", &[("host", authority.into())]);
                        self.set_status(msg);
                    }
                }
            }
            KeyCode::Enter => {
                if let Some(site) = rows.get(cursor).cloned() {
                    self.mode = Mode::Normal;
                    self.connect_to(site, None, false);
                } else {
                    self.mode = Mode::Normal;
                    self.run(Command::NetConnect);
                }
            }
            _ => {}
        }
    }

    /// The site list: the configured sites, plus any host connected by hand this session.
    pub fn site_rows(&self) -> Vec<Site> {
        let mut rows = self.sites.clone();
        for authority in &self.connected {
            if rows.iter().any(|s| &s.authority() == authority) {
                continue;
            }
            if let Ok(site) = Site::parse_url(&format!("sftp://{authority}/")) {
                rows.push(site);
            }
        }
        rows
    }

    fn confirm_key(&mut self, key: Key, confirm: Confirm) {
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.mode = Mode::Normal;
                match confirm.action {
                    ConfirmAction::Job(spec) => self.start_job(spec),
                    ConfirmAction::DiscardEdits => {
                        self.editor_parked = None;
                        self.remote_edit = None;
                    }
                    ConfirmAction::QuitWithScreens => self.quit = true,
                    ConfirmAction::PlayMacro(which) => self.run_macro(&which),
                    ConfirmAction::DiscardHex => {
                        // The mode is already Normal: the bytes go with it.
                        self.read_panel(0);
                        self.read_panel(1);
                    }
                    ConfirmAction::OverwriteChanged => {
                        self.restore_editor();
                        self.save_editor(true);
                    }
                    ConfirmAction::OverwriteRemote => {
                        self.restore_editor();
                        self.write_back(true);
                    }
                    ConfirmAction::AcceptHostKey {
                        panel,
                        site,
                        secret,
                    } => {
                        self.requests.push(Request::Connect {
                            panel,
                            site,
                            secret,
                            accept_host_key: true,
                        });
                    }
                }
            }
            KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::F(10) => {
                self.mode = Mode::Normal;
                // A refused confirmation gives the editor back, not a lost buffer.
                self.restore_editor();
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

    /// The editor consumes movement and typing keys itself — binding thirty arrow-plus-modifier
    /// combinations as commands would buy nothing — and the keymap drives its commands.
    /// The editor commands that mean something when the buffer is bytes.
    fn hex_command(&mut self, cmd: Command) {
        match cmd {
            Command::EditorQuit => self.close_hex(),
            Command::EditorHexToggle => self.toggle_hex(),
            Command::EditorSave => {
                let result = self.mode.hex_mut().map(|e| e.save(None));
                match result {
                    Some(Ok(bytes)) => {
                        let name = self.mode.hex().map(|e| e.name()).unwrap_or_default();
                        let msg = self.i18n.t_args(
                            "editor-saved",
                            &[("name", name.into()), ("lines", (bytes as i64).into())],
                        );
                        self.set_status(msg);
                    }
                    Some(Err(e)) => self.set_error(e.to_string()),
                    None => {}
                }
            }
            Command::EditorUndo => {
                if let Some(editor) = self.mode.hex_mut() {
                    if !editor.undo() {
                        let m = self.i18n.t("editor-nothing-to-undo");
                        self.set_status(m);
                    }
                }
            }
            Command::EditorRedo => {
                if let Some(editor) = self.mode.hex_mut() {
                    if !editor.redo() {
                        let m = self.i18n.t("editor-nothing-to-redo");
                        self.set_status(m);
                    }
                }
            }
            Command::EditorBlockMark => {
                if let Some(editor) = self.mode.hex_mut() {
                    editor.toggle_mark();
                }
            }
            Command::EditorBlockOps => self.open_block_menu(),
            Command::EditorAssemble => self.open_assembler(),
            Command::EditorToggleReadOnly => {
                if let Some(editor) = self.mode.hex_mut() {
                    editor.read_only = !editor.read_only;
                }
            }
            other => self.not_implemented(&format!("{} (in hex)", other.id())),
        }
    }

    /// `Esc` or `F10` in the hex editor: the same question the text editor asks.
    fn close_hex(&mut self) {
        let dirty = self.mode.hex().map(|e| e.modified()).unwrap_or(false);
        if !dirty {
            self.mode = Mode::Normal;
            self.read_panel(0);
            self.read_panel(1);
            return;
        }
        let name = self.mode.hex().map(|e| e.name()).unwrap_or_default();
        let title = self.i18n.t("editor-discard-title");
        let body = self
            .i18n
            .t_args("editor-discard-body", &[("name", name.into())]);
        self.mode = Mode::Confirm(Confirm {
            title,
            lines: vec![body, String::new(), self.i18n.t("confirm-yes-no")],
            action: ConfirmAction::DiscardHex,
        });
    }

    /// Keys in the hex editor (SPEC 05 §2.5). Movement is in bytes and rows, typing goes to
    /// whichever pane has the cursor, and everything else falls through to the keymap — so
    /// `F2`, `Esc` and undo are the same keys as in the text editor, because they are the
    /// same commands.
    fn hex_key(&mut self, key: Key) {
        let rows = self.editor_size.1.max(1) as usize;
        let handled = {
            let Some(editor) = self.mode.hex_mut() else {
                return;
            };
            let per = editor.bytes_per_row as i64;
            let handled = match key.code {
                KeyCode::Up => {
                    editor.move_cursor(-per);
                    true
                }
                KeyCode::Down => {
                    editor.move_cursor(per);
                    true
                }
                KeyCode::Left => {
                    editor.move_cursor(-1);
                    true
                }
                KeyCode::Right => {
                    editor.move_cursor(1);
                    true
                }
                KeyCode::PageUp => {
                    editor.move_cursor(-per * rows as i64);
                    true
                }
                KeyCode::PageDown => {
                    editor.move_cursor(per * rows as i64);
                    true
                }
                KeyCode::Home if key.mods.contains(Mods::CTRL) => {
                    editor.move_cursor(-(editor.len() as i64));
                    true
                }
                KeyCode::End if key.mods.contains(Mods::CTRL) => {
                    editor.move_cursor(editor.len() as i64);
                    true
                }
                // Home and End are the row's own ends, as they are in a text editor.
                KeyCode::Home => {
                    let back = (editor.cursor % editor.bytes_per_row as u64) as i64;
                    editor.move_cursor(-back);
                    true
                }
                KeyCode::End => {
                    let forward = per - 1 - (editor.cursor % editor.bytes_per_row as u64) as i64;
                    editor.move_cursor(forward);
                    true
                }
                KeyCode::Tab => {
                    editor.pane = match editor.pane {
                        scmd_edit::Pane::Hex => scmd_edit::Pane::Text,
                        scmd_edit::Pane::Text => scmd_edit::Pane::Hex,
                    };
                    editor.half = None;
                    true
                }
                KeyCode::Insert => {
                    editor.insert = !editor.insert;
                    editor.half = None;
                    true
                }
                KeyCode::Backspace if editor.cursor > 0 => {
                    // In insert mode it takes the byte before the cursor out; in overwrite it
                    // steps back, because removing a byte there would shift the file.
                    if editor.insert {
                        let at = editor.cursor - 1;
                        editor.delete(at, 1);
                        editor.cursor = at;
                    } else {
                        editor.move_cursor(-1);
                    }
                    true
                }
                KeyCode::Delete if editor.insert && editor.cursor < editor.len() => {
                    let at = editor.cursor;
                    editor.delete(at, 1);
                    true
                }
                KeyCode::Char(c) if key.mods == Mods::NONE || key.mods == Mods::SHIFT => {
                    match editor.pane {
                        scmd_edit::Pane::Hex => editor.type_hex(c),
                        scmd_edit::Pane::Text => editor.type_char(c),
                    }
                }
                _ => false,
            };
            editor.scroll_into_view(rows);
            handled
        };
        if handled {
            return;
        }
        if let Some(cmd) = self.keymap.resolve(&[Scope::Editor, Scope::Global], key) {
            self.run(cmd);
        }
    }

    fn editor_key(&mut self, key: Key) {
        let (width, height) = self.editor_size;
        let shift = key.mods.contains(Mods::SHIFT);
        let ctrl = key.mods.contains(Mods::CTRL);
        let alt = key.mods.contains(Mods::ALT);
        // Alt+Shift+arrows select a column block, as in FAR.
        let kind = if alt && shift {
            BlockKind::Column
        } else {
            BlockKind::Stream
        };
        let extend = shift;

        let handled = {
            let Some(editor) = self.mode.editor_mut() else {
                return;
            };
            let result: Result<bool, scmd_edit::editor::EditorError> = (|| {
                Ok(match key.code {
                    // Alt alone belongs to the keymap (move line); Alt+Shift selects a column.
                    KeyCode::Up if !alt || shift => {
                        editor.move_by(-1, 0, extend, kind);
                        true
                    }
                    KeyCode::Down if !alt || shift => {
                        editor.move_by(1, 0, extend, kind);
                        true
                    }
                    KeyCode::Left => {
                        if ctrl {
                            editor.move_by(0, -5, extend, kind);
                        } else {
                            editor.move_by(0, -1, extend, kind);
                        }
                        true
                    }
                    KeyCode::Right => {
                        if ctrl {
                            editor.move_by(0, 5, extend, kind);
                        } else {
                            editor.move_by(0, 1, extend, kind);
                        }
                        true
                    }
                    KeyCode::Home => {
                        if ctrl {
                            editor.move_file_start(extend);
                        } else {
                            editor.move_line_start(extend);
                        }
                        true
                    }
                    KeyCode::End => {
                        if ctrl {
                            editor.move_file_end(extend);
                        } else {
                            editor.move_line_end(extend);
                        }
                        true
                    }
                    KeyCode::PageUp => {
                        editor.page(false, height as usize, extend);
                        true
                    }
                    KeyCode::PageDown => {
                        editor.page(true, height as usize, extend);
                        true
                    }
                    KeyCode::Enter => {
                        editor.insert_newline()?;
                        true
                    }
                    KeyCode::Backspace if !ctrl => {
                        editor.backspace()?;
                        true
                    }
                    KeyCode::Delete if !shift => {
                        editor.delete_forward()?;
                        true
                    }
                    KeyCode::Tab => {
                        editor.indent()?;
                        true
                    }
                    KeyCode::BackTab => {
                        editor.outdent()?;
                        true
                    }
                    KeyCode::Char(ch) if !ctrl && !alt => {
                        editor.insert_char(ch)?;
                        true
                    }
                    _ => false,
                })
            })();
            match result {
                Ok(handled) => {
                    editor.scroll_into_view(height as usize, width as usize);
                    Ok(handled)
                }
                Err(e) => Err(e.to_string()),
            }
        };

        match handled {
            Ok(true) => {}
            Ok(false) => {
                if let Some(cmd) = self.keymap.resolve(&[Scope::Editor, Scope::Global], key) {
                    self.run(cmd);
                }
            }
            Err(error) => {
                let name = self.mode.editor().map(|e| e.name()).unwrap_or_default();
                let msg = self.i18n.t_args(
                    "editor-failed",
                    &[("name", name.into()), ("error", error.into())],
                );
                self.set_error(msg);
            }
        }
    }

    fn viewer_key(&mut self, key: Key) {
        // While the structure pane has the focus, the arrows move its cursor rather than the
        // file's position, and `Enter` follows the row. Everything else still goes to the
        // keymap, so `F3` still closes the viewer from inside the pane.
        if self.structure_pane_key(key) {
            return;
        }
        if let Some(cmd) = self.keymap.resolve(&[Scope::Viewer, Scope::Global], key) {
            self.run(cmd);
        }
    }

    /// Returns true when the key belonged to the structure pane.
    fn structure_pane_key(&mut self, key: Key) -> bool {
        let height = self.viewer_size.1;
        let mut target = None;
        let handled = match self.mode.viewer_mut() {
            Some(viewer) if viewer.show_structure && viewer.struct_focus => match key.code {
                KeyCode::Up => {
                    viewer.structure_move(-1, height);
                    true
                }
                KeyCode::Down => {
                    viewer.structure_move(1, height);
                    true
                }
                KeyCode::PageUp => {
                    viewer.structure_move(-(height as isize), height);
                    true
                }
                KeyCode::PageDown => {
                    viewer.structure_move(height as isize, height);
                    true
                }
                KeyCode::Home => {
                    viewer.structure_home();
                    true
                }
                KeyCode::End => {
                    viewer.structure_end(height);
                    true
                }
                KeyCode::Enter => {
                    target = Some(viewer.structure_target());
                    true
                }
                _ => false,
            },
            _ => false,
        };
        // `Enter` on a row that points nowhere says so instead of moving the file silently.
        if let Some(target) = target {
            match target {
                Some(offset) => {
                    let (width, height) = self.viewer_size;
                    if let Some(viewer) = self.mode.viewer_mut() {
                        viewer.jump(offset);
                        if viewer.mode == scmd_view::Mode::Text {
                            viewer.set_mode(scmd_view::Mode::Hex);
                        }
                        let _ = (width, height);
                    }
                    let msg = self.i18n.t_args(
                        "viewer-went-to",
                        &[("offset", format!("{offset:#X}").into())],
                    );
                    self.set_status(msg);
                }
                None => {
                    let msg = self.i18n.t("viewer-no-target");
                    self.set_status(msg);
                }
            }
        }
        handled
    }

    /// Unblock the engine with a decision.
    fn answer(&mut self, decision: Decision) {
        if let Mode::Job(view) = &mut self.mode {
            if let Some(pending) = view.question.take() {
                let _ = pending.reply.send(decision);
            }
        }
    }

    /// The help's keys. The index is a list one chooses from; a chapter is a page one
    /// scrolls, and `Esc` or `Left` goes back to the index rather than out of the help, which
    /// is what makes a book a book.
    fn help_key(&mut self, key: Key, chapter: crate::help::Chapter, scroll: usize, at: usize) {
        use crate::help::Chapter;
        let rows = self.rows.max(1);
        let index = chapter == Chapter::Index;
        let count = crate::help::chapters().len();
        let lines = self.help_lines(chapter, self.help_width()).len();
        let last = lines.saturating_sub(rows);
        let go = |app: &mut App, chapter, scroll, at| {
            app.mode = Mode::Help {
                chapter,
                scroll,
                at,
            }
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') if index => self.mode = Mode::Normal,
            // Out of a chapter is back to the index; out of the index is out of the help.
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('q') => {
                go(self, Chapter::Index, 0, at)
            }
            KeyCode::F(1) | KeyCode::F(10) => self.mode = Mode::Normal,
            KeyCode::Enter | KeyCode::Right if index => {
                let chapters = crate::help::chapters();
                if let Some((chapter, _)) = chapters.get(at) {
                    go(self, *chapter, 0, at);
                }
            }
            KeyCode::Up if index => go(self, chapter, scroll, at.saturating_sub(1)),
            KeyCode::Down if index => go(self, chapter, scroll, (at + 1).min(count - 1)),
            KeyCode::Home if index => go(self, chapter, scroll, 0),
            KeyCode::End if index => go(self, chapter, scroll, count - 1),
            KeyCode::PageUp if index => go(self, chapter, scroll, at.saturating_sub(rows)),
            KeyCode::PageDown if index => go(self, chapter, scroll, (at + rows).min(count - 1)),
            KeyCode::Up => go(self, chapter, scroll.saturating_sub(1), at),
            KeyCode::Down => go(self, chapter, (scroll + 1).min(last), at),
            KeyCode::PageUp => go(self, chapter, scroll.saturating_sub(rows), at),
            KeyCode::PageDown => go(self, chapter, (scroll + rows).min(last), at),
            KeyCode::Home => go(self, chapter, 0, at),
            KeyCode::End => go(self, chapter, last, at),
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
                self.restore_editor();
                // A question asked over the hex editor goes back to it when it is dismissed:
                // changing one's mind about a fill byte must not drop unsaved patches.
                self.restore_hex();
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
                // The filter bar takes the whole mask grammar — `*.rs,*.toml`, an exclusion
                // after `|`, `<arc>`, `/regex/i` — and a bare word still means "contains".
                // A mask that does not parse yet (`*.rs|`, a half-typed group) leaves the
                // previous filter alone instead of flickering the panel empty.
                let text = text.trim().to_string();
                if text.is_empty() {
                    let panel = self.panel_mut();
                    panel.filter = None;
                    panel.filter_set = None;
                    panel.rebuild();
                    return;
                }
                if let Ok(masks) = scmd_core::filter::MaskSet::parse(&text) {
                    let panel = self.panel_mut();
                    panel.filter = Some(text);
                    panel.filter_set = Some(masks);
                    panel.rebuild();
                }
            }
            _ => {}
        }
    }

    fn prompt_accept(&mut self, prompt: Prompt) {
        match prompt.kind {
            PromptKind::Calculator => {
                let line = prompt.text.trim().to_string();
                match scmd_core::calc::evaluate(&line) {
                    Ok(answer) => {
                        let [dec, hex, bin, oct, chars] = answer.lines();
                        let title = self.i18n.t("calc-title");
                        self.last_calc = line.clone();
                        self.mode = Mode::Message {
                            title,
                            lines: vec![
                                format!("{line} ="),
                                String::new(),
                                format!("  {:<12} {dec}", self.i18n.t("calc-decimal")),
                                format!("  {:<12} {hex}", self.i18n.t("calc-hex")),
                                format!("  {:<12} {oct}", self.i18n.t("calc-octal")),
                                format!("  {:<12} {bin}", self.i18n.t("calc-binary")),
                                format!("  {:<12} {chars}", self.i18n.t("calc-characters")),
                            ],
                        };
                    }
                    Err(e) => {
                        self.set_error(e.to_string());
                        // The line stays in the box, so a typo is fixed rather than retyped.
                        let title = self.i18n.t("calc-title");
                        self.mode = Mode::Prompt(Prompt::new(PromptKind::Calculator, title, line));
                    }
                }
            }
            PromptKind::MacroDescription { key, scope, steps } => {
                let description = prompt.text.trim().to_string();
                self.mode = Mode::Normal;
                self.store_macro(scmd_cmd::Macro {
                    key,
                    scope,
                    steps,
                    play: scmd_cmd::Play::Once,
                    description,
                    // Never by recording: trusting a macro is a line one writes in the file.
                    trusted: false,
                });
            }
            PromptKind::Assemble => {
                let text = prompt.text.trim().to_string();
                self.restore_hex();
                self.assemble_at_cursor(&text);
            }
            PromptKind::HexFill
            | PromptKind::HexXor
            | PromptKind::HexWrite
            | PromptKind::HexRead
            | PromptKind::HexInsert
            | PromptKind::HexCopy
            | PromptKind::HexMove => {
                let kind = prompt.kind.clone();
                let text = prompt.text.trim().to_string();
                self.restore_hex();
                self.hex_operation(kind, &text);
            }
            PromptKind::CommandAnswer { ref pending } => {
                // The answer goes in and the line runs; an empty answer still counts, because
                // "search for nothing" is a decision the user is allowed to make.
                let line = pending.replace("{answer}", prompt.text.trim());
                self.run_line(&line);
            }
            PromptKind::NetConnect => {
                let raw = prompt.text.trim().to_string();
                if raw.is_empty() {
                    return;
                }
                match Site::parse_url(&raw) {
                    Ok(site) => self.connect_to(site, None, false),
                    Err(e) => self.set_error(e.to_string()),
                }
            }
            PromptKind::NetSecret(_) => {
                let secret = prompt.text.clone();
                match self.pending_connect.take() {
                    Some((panel, site, _)) => self.requests.push(Request::Connect {
                        panel,
                        site,
                        secret: Some(secret),
                        accept_host_key: false,
                    }),
                    None => {
                        let m = self.i18n.t("net-nothing-pending");
                        self.set_status(m);
                    }
                }
            }
            PromptKind::Goto => {
                let raw = prompt.text.trim();
                if raw.is_empty() {
                    return;
                }
                // `sftp://mb@host/srv` typed into `Ctrl+L` connects, as FAR's NetBox does.
                if let Ok(site) = Site::parse_url(raw) {
                    if raw.contains("://") {
                        if let Some(remote) = self.connected_site_path(&site) {
                            self.goto(remote);
                        } else {
                            self.connect_to(site, None, false);
                        }
                        return;
                    }
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
                let sources = if prompt.sources.is_empty() {
                    self.job_sources()
                } else {
                    prompt.sources.clone()
                };
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let target = self.resolve_path(raw);
                self.start_job(JobSpec::new(kind, sources, Some(target)));
            }
            PromptKind::LinkName => {
                let raw = prompt.text.trim();
                let Some(target) = prompt.sources.first().cloned() else {
                    return;
                };
                if raw.is_empty() {
                    return;
                }
                let link = self.resolve_path(raw);
                // A relative symbolic link, which is the one that survives the whole tree
                // being moved or copied elsewhere. The dialog that offers a hard link and an
                // absolute one is a backlog item; `link:` on the command line will carry the
                // choice when it lands.
                self.requests.push(Request::Link {
                    target,
                    link,
                    kind: scmd_jobs::LinkKind::SymlinkRelative,
                });
            }
            PromptKind::ApplyCommand => {
                let template = prompt.text.trim().to_string();
                if template.is_empty() {
                    return;
                }
                let base = self.expand_context();
                let mut lines = Vec::with_capacity(prompt.sources.len());
                for path in &prompt.sources {
                    // Each file takes its turn as "the current file", which is what makes
                    // `!.!` and the rest mean one file at a time rather than the whole list.
                    let name = path.display_name();
                    let ctx = expand::Context {
                        active_file: Some(name.clone()),
                        selection: vec![name],
                        ..base.clone()
                    };
                    match expand::expand(&template, &ctx, None) {
                        Ok(expand::Expanded::Line(line)) => lines.push(line),
                        Ok(expand::Expanded::Ask { .. }) => {
                            // One answer for a hundred files would be a different feature,
                            // and guessing which it is would be worse than saying so.
                            let m = self.i18n.t("cmd-apply-no-ask");
                            self.set_error(m);
                            return;
                        }
                        Err(e) => {
                            self.set_error(e.to_string());
                            return;
                        }
                    }
                }
                self.cmd.remember(&template);
                self.apply_total = lines.len();
                self.apply_queue = lines;
                self.run_next_apply();
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
                // In disassembly mode the query is an instruction pattern, not bytes: looking
                // for the four letters of `call` in the file would find nothing that matters.
                let result = self.mode.viewer_mut().map(|v| {
                    if v.mode == ViewerMode::Disasm {
                        Ok(v.search_instructions(&query, false, &CancelToken::new()))
                    } else {
                        v.search(&query, mode, ci, false, &CancelToken::new())
                    }
                });
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
            PromptKind::EditorSearch => {
                let query = prompt.text.trim().to_string();
                self.restore_editor();
                if query.is_empty() {
                    return;
                }
                let found = self
                    .mode
                    .editor_mut()
                    .map(|e| e.find(&query, false))
                    .unwrap_or(Ok(false));
                match found {
                    Ok(true) => {}
                    Ok(false) => {
                        let msg = self
                            .i18n
                            .t_args("editor-not-found", &[("query", query.into())]);
                        self.set_error(msg);
                    }
                    Err(e) => self.set_error(e.to_string()),
                }
            }
            PromptKind::EditorReplaceFrom => {
                let from = prompt.text.trim().to_string();
                if from.is_empty() {
                    self.restore_editor();
                    return;
                }
                self.replace_from = Some(from);
                let title = self.i18n.t("editor-prompt-replace-with");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::EditorReplaceWith,
                    title,
                    text: String::new(),
                    secret: false,
                    sources: Vec::new(),
                });
            }
            PromptKind::EditorReplaceWith => {
                let with = prompt.text.clone();
                let from = self.replace_from.take().unwrap_or_default();
                self.restore_editor();
                let result = self.mode.editor_mut().map(|e| e.replace_all(&from, &with));
                match result {
                    Some(Ok(count)) => {
                        let msg = self
                            .i18n
                            .t_args("editor-replaced", &[("count", (count as i64).into())]);
                        self.set_status(msg);
                    }
                    Some(Err(e)) => self.set_error(e.to_string()),
                    None => {}
                }
            }
            PromptKind::EditorGoto => {
                let text = prompt.text.clone();
                self.restore_editor();
                let (width, height) = self.editor_size;
                if let Some(editor) = self.mode.editor_mut() {
                    if !editor.goto(&text) {
                        let msg = format!("{}: ?", text.trim());
                        self.set_error(msg);
                    } else {
                        editor.scroll_into_view(height as usize, width as usize);
                    }
                }
            }
            PromptKind::EditorSaveAs => {
                let raw = prompt.text.trim().to_string();
                self.restore_editor();
                if raw.is_empty() {
                    return;
                }
                let target = self.resolve_path(&raw);
                if target.is_remote() {
                    self.save_as_remote(target);
                    return;
                }
                // Saving a fetched copy under a local name makes it a local file again.
                self.remote_edit = None;
                let result = self
                    .mode
                    .editor_mut()
                    .map(|e| (e.name(), e.save_as(target.as_path())));
                match result {
                    Some((name, Ok(scmd_edit::SaveOutcome::Saved { bytes, .. }))) => {
                        let msg = self.i18n.t_args(
                            "editor-saved",
                            &[("name", name.into()), ("bytes", (bytes as i64).into())],
                        );
                        self.set_status(msg);
                        self.read_panel(0);
                        self.read_panel(1);
                    }
                    Some((name, Err(e))) => {
                        let msg = self.i18n.t_args(
                            "editor-failed",
                            &[("name", name.into()), ("error", e.to_string().into())],
                        );
                        self.set_error(msg);
                    }
                    _ => {}
                }
            }
            PromptKind::Filter | PromptKind::QuickSearch { .. } => {}
        }
    }

    /// A path typed by the user: absolute as given, `~` expanded, otherwise relative to the
    /// active panel.
    fn resolve_path(&self, raw: &str) -> VPath {
        // A URL is a path too: a copy target, a save-as name or a directory can be remote.
        if raw.contains("://") {
            let parsed = VPath::parse(raw);
            if parsed.is_remote() {
                return parsed.normalised();
            }
        }
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
                    out.push(panel.path_of(e));
                }
            }
            return out;
        }
        match panel.current() {
            Some(e) if e.is_selectable() => vec![panel.path_of(e)],
            _ => Vec::new(),
        }
    }

    /// Only the item under the cursor, which is what `Shift+F5`, `Shift+F6` and `Shift+F8`
    /// mean whatever is selected.
    fn cursor_source(&self) -> Vec<VPath> {
        let panel = self.panel();
        match panel.current() {
            Some(e) if e.is_selectable() => vec![panel.path_of(e)],
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
            JobKind::Wipe => "job-wipe",
            JobKind::Replace => "job-replace",
            JobKind::Pack => "job-pack",
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
                            ("bytes", scmd_core::format_size(*dst_size).into()),
                        ],
                    ),
                    self.i18n.t_args(
                        "conflict-src",
                        &[
                            ("path", src.to_string().into()),
                            ("bytes", scmd_core::format_size(*src_size).into()),
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

    /// The same progress box as a job, with the archive writer behind it.
    fn start_pack(
        &mut self,
        archive: VPath,
        format: scmd_arc::Format,
        level: scmd_arc::Level,
        sources: Vec<VPath>,
    ) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Job(JobView {
            id,
            kind: JobKind::Pack,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        });
        self.requests.push(Request::Pack {
            id,
            archive,
            format,
            level,
            sources,
            cancel,
        });
    }

    fn job_done(&mut self, id: u64, kind: JobKind, outcome: Outcome) {
        if let Mode::Job(view) = &self.mode {
            if view.id != id {
                return;
            }
        }
        self.mode = Mode::Normal;

        // A replacement runs twice. The first run only counted, and `pending_replace` holds
        // the one that would write: so this is the count, and it asks rather than reports.
        if kind == JobKind::Replace && self.pending_replace.is_some() {
            self.replace_counted(&outcome);
            return;
        }
        if kind == JobKind::Replace {
            let msg = self.i18n.t_args(
                "replace-done",
                &[
                    ("count", (outcome.occurrences() as i64).into()),
                    ("files", (outcome.replacements.len() as i64).into()),
                ],
            );
            self.set_status(msg);
            self.read_panel(0);
            self.read_panel(1);
            return;
        }

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
                    ("bytes", scmd_core::format_size(outcome.bytes).into()),
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
        // The command line has no focus of its own: it takes the shared keys only while it has
        // something on it, which is FAR's rule and the reason typing never feels modal.
        if self.command_line_key(key) {
            return;
        }
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
        // With quick search on Alt (the default), a plain letter belongs to the command line.
        if typed.is_none() && self.cfg.ui.command_line {
            if let Some(c) = key.as_typed_char() {
                if !matches!(self.cfg.panel.quick_search, QuickSearch::Plain) {
                    self.cmd.insert(c);
                    return;
                }
            }
        }
        if let Some(c) = typed {
            let origin = self.panel().cursor;
            let title = self.i18n.t("prompt-search");
            self.mode = Mode::Prompt(Prompt {
                kind: PromptKind::QuickSearch { origin },
                title,
                text: c.to_string(),
                secret: false,
                sources: Vec::new(),
            });
            let text = c.to_string();
            self.prompt_changed(&text);
        }
    }

    pub fn run(&mut self, cmd: Command) {
        // The key that got here became a command, so that is what the macro remembers. The
        // recording keys themselves are not part of the recording.
        if self.record_key.take().is_some() {
            if let Some(steps) = self.recording.as_mut() {
                if cmd != Command::MacroRecord {
                    steps.push(scmd_cmd::Step::Run(cmd));
                }
            }
        }
        // While the viewer is open, movement and its own toggles belong to it, not the panel.
        if self.mode.viewer().is_some() && is_viewer_movement(cmd) {
            self.viewer_command(cmd);
            return;
        }
        // Crossing to the viewer leaves the editor behind, so it is not one of the editor's
        // own commands even though it is named like one.
        if cmd == Command::EditorToViewer {
            if self.mode.editor().is_some() {
                self.editor_to_viewer();
            }
            return;
        }
        if is_editor_command(cmd) {
            if self.mode.editor().is_some()
                || self.editor_parked.is_some()
                || self.mode.hex().is_some()
            {
                self.editor_command(cmd);
                return;
            }
            // An editor command with no editor open is a keymap mistake, not a crash.
            self.not_implemented(&cmd.id());
            return;
        }
        self.panel_mut().message = None;
        // A special panel has no rows: movement in it scrolls what it is showing instead.
        if self.special_panel_key(cmd) {
            return;
        }
        match cmd {
            Command::AppQuit => self.ask_before_quitting(),
            Command::AppHelp => {
                self.mode = Mode::Help {
                    chapter: crate::help::Chapter::Index,
                    scroll: 0,
                    at: 0,
                }
            }
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
                self.panel_mut().view = view;
                self.panel_mut().top = 0;
                self.scroll();
                let msg = self
                    .i18n
                    .t_args("msg-view", &[("mode", view.as_str().into())]);
                self.set_status(msg);
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
                    secret: false,
                    sources: Vec::new(),
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
            Command::NavLeft | Command::NavRight => {
                // A column, in the views that have more than one of them. In a single-column
                // view there is nothing to the left or the right, and FAR does nothing there.
                if matches!(self.panel().view, ViewMode::Brief | ViewMode::Wide) {
                    let step = self.rows.max(1) as isize;
                    self.move_cursor(if cmd == Command::NavLeft { -step } else { step });
                }
            }
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
                // Not a directory: it may still be an archive, which `Enter` goes into.
                None => match self.archive_target() {
                    Some(path) => self.goto(path),
                    None => self.not_implemented("nav.enter (running files)"),
                },
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
                    secret: false,
                    sources: Vec::new(),
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
                    secret: false,
                    sources: Vec::new(),
                });
            }
            Command::SelRestore => {
                self.panel_mut().selection.restore_previous();
                self.panel_mut().rebuild();
            }
            Command::SelExtAdd
            | Command::SelExtRemove
            | Command::SelNameAdd
            | Command::SelNameRemove => {
                let Some(current) = self.panel().current().cloned() else {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                };
                let by_extension = matches!(cmd, Command::SelExtAdd | Command::SelExtRemove);
                let on = matches!(cmd, Command::SelExtAdd | Command::SelNameAdd);
                // A file with no extension groups with the other files with no extension,
                // and a name is compared without its extension — both the way FAR reads the
                // current file to decide what "the same" means.
                let fold = |s: Option<&std::ffi::OsStr>| {
                    s.map(|s| s.to_string_lossy().to_lowercase())
                        .unwrap_or_default()
                };
                let here = std::path::Path::new(&current.name);
                let wanted = if by_extension {
                    fold(here.extension())
                } else {
                    fold(here.file_stem())
                };
                let listing = self.panel().listing.clone();
                let changed = self.panel_mut().selection.apply_where(
                    &listing,
                    MaskScope::FilesOnly,
                    on,
                    |e| {
                        let p = std::path::Path::new(&e.name);
                        let theirs = if by_extension {
                            fold(p.extension())
                        } else {
                            fold(p.file_stem())
                        };
                        theirs == wanted
                    },
                );
                self.panel_mut().rebuild();
                let msg = self.i18n.t_args(
                    if on { "msg-selected" } else { "msg-deselected" },
                    &[("count", (changed as i64).into())],
                );
                self.set_status(msg);
            }
            Command::FileWipe => {
                let sources = self.job_sources();
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let bytes = self.selected_bytes(&sources);
                let spec = JobSpec::new(JobKind::Wipe, sources.clone(), None);
                let title = self.i18n.t("confirm-wipe-title");
                let body = self.i18n.t_args(
                    "confirm-wipe-body",
                    &[
                        ("count", (sources.len() as i64).into()),
                        ("bytes", scmd_core::format_size(bytes).into()),
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
                // Overwriting is not a promise on modern hardware, and the dialog says so
                // rather than letting a user believe something the filesystem cannot deliver.
                lines.push(self.i18n.t("confirm-wipe-honest"));
                lines.push(self.i18n.t("confirm-yes-no"));
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines,
                    action: ConfirmAction::Job(spec),
                });
            }
            Command::FileLink => {
                let Some(entry) = self.panel().current().cloned() else {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                };
                if entry.kind == scmd_core::EntryKind::Parent {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let target = self.panel().path_of(&entry);
                // The link goes in the other panel, as a copy would.
                let mut text = self.other().cwd.to_string();
                if !text.ends_with(std::path::MAIN_SEPARATOR) {
                    text.push(std::path::MAIN_SEPARATOR);
                }
                text.push_str(&entry.name.to_string_lossy());
                let title = self
                    .i18n
                    .t_args("prompt-link", &[("name", target.display_name().into())]);
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::LinkName,
                    title,
                    text,
                    secret: false,
                    sources: vec![target],
                });
            }
            Command::PanelQuickView(side) => {
                self.set_content(side, crate::panel::Content::QuickView)
            }
            Command::PanelInfo(side) => self.set_content(side, crate::panel::Content::Info),
            Command::PanelTree(side) => self.set_content(side, crate::panel::Content::Tree),
            Command::FilePack => self.open_pack_form(),
            Command::FileUnpack => self.open_unpack_form(),
            Command::ToolCompareDirs => self.compare_dirs(),
            Command::ToolCalculator => {
                // It opens on the last sum: a calculator is used in runs.
                let title = self.i18n.t("calc-title");
                let last = self.last_calc.clone();
                self.mode = Mode::Prompt(Prompt::new(PromptKind::Calculator, title, last));
            }
            Command::ToolFind => self.open_find_form(),
            Command::ToolReplace => {
                let files = self.job_sources().len();
                if files == 0 {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                self.mode = Mode::Form(Box::new(Form::replace(
                    &self.last_replace,
                    files,
                    &self.i18n,
                )));
            }
            Command::FileApply => {
                let sources = self.job_sources();
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let title = self
                    .i18n
                    .t_args("prompt-apply", &[("count", (sources.len() as i64).into())]);
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::ApplyCommand,
                    title,
                    text: String::new(),
                    secret: false,
                    sources,
                });
            }
            Command::PanelToggle(side) => {
                let which = match side {
                    Side::Active => self.active,
                    Side::Left => 0,
                    Side::Right => 1,
                };
                self.hidden = if self.hidden == Some(which) {
                    None
                } else {
                    Some(which)
                };
                // Keeping the focus on a panel nobody can see would send every key into the
                // dark, so it moves across with the screen.
                if self.hidden == Some(self.active) {
                    self.active = 1 - self.active;
                }
            }
            Command::PanelScrollNames(delta) => {
                let panel = self.panel_mut();
                panel.name_scroll = if delta < 0 {
                    panel.name_scroll.saturating_sub(1)
                } else {
                    panel.name_scroll + 1
                };
            }
            Command::SelInvertAll => {
                let listing = self.panel().listing.clone();
                self.panel_mut()
                    .selection
                    .invert(&listing, MaskScope::FilesAndDirs);
                self.panel_mut().rebuild();
            }

            Command::FileView => self.open_viewer(),
            Command::FileEdit => self.open_editor(),
            Command::FileEditNew => self.open_new_editor(),
            Command::AppMenu => self.open_menu(),
            Command::CmdRun => self.run_command_line(),
            Command::CmdClear => self.cmd.clear(),
            Command::CmdHistoryPrev => {
                if !self.cmd.history_prev() {
                    let msg = self.i18n.t("cmd-no-history");
                    self.set_status(msg);
                }
            }
            Command::CmdHistoryNext => {
                self.cmd.history_next();
            }
            Command::PluginCommands => self.open_plugin_commands(),
            Command::MacroRecord => self.toggle_recording(),
            Command::MacroList => self.open_macros(),
            Command::AppScreens => self.open_screens(),
            Command::AppScreenNext => self.cycle_screen(false),
            Command::AppScreenPrev => self.cycle_screen(true),
            Command::CmdHistory => self.open_pick(PickKind::CommandHistory, self.active),
            Command::CmdInsertName => self.insert_into_line(false, false),
            Command::CmdInsertNameOther => self.insert_into_line(false, true),
            Command::CmdInsertPath => self.insert_into_line(true, false),
            Command::CmdInsertPathOther => self.insert_into_line(true, true),
            Command::ShellOutputShow => self.mode = Mode::UserScreen { scroll: 0 },
            Command::AppThemes => self.open_pick(PickKind::Themes, self.active),
            Command::AppLanguages => self.open_pick(PickKind::Languages, self.active),
            Command::AppSetupSave => self.save_setup(),
            Command::OptSystem => self.open_form(FormKind::System),
            Command::OptPanel => self.open_form(FormKind::Panel),
            Command::OptInterface => self.open_form(FormKind::Interface),
            Command::OptConfirmations => self.open_form(FormKind::Confirmations),
            Command::PanelSortMenu => self.open_pick(PickKind::SortModes, self.active),
            Command::PanelHistory => self.open_pick(PickKind::FoldersHistory, self.active),
            Command::PanelDrive(side) => {
                let side = match side {
                    Side::Left => 0,
                    Side::Right => 1,
                    Side::Active => self.active,
                };
                self.open_pick(PickKind::Drives, side)
            }
            Command::NetSites => self.mode = Mode::Sites { cursor: 0 },
            Command::NetConnect => {
                let title = self.i18n.t("net-prompt-url");
                self.mode = Mode::Prompt(Prompt::new(
                    PromptKind::NetConnect,
                    title,
                    "sftp://".to_string(),
                ));
            }
            Command::NetDisconnect => self.disconnect_active(),

            Command::ViewerDisasm if self.mode.viewer().is_some() => {
                let name = self
                    .mode
                    .viewer()
                    .map(|v| v.path.display_name())
                    .unwrap_or_default();
                match self.mode.viewer_mut().map(|v| v.start_disasm()) {
                    Some(Err(e)) => {
                        let msg = self.i18n.t_args(
                            "viewer-disasm-failed",
                            &[("name", name.into()), ("error", e.to_string().into())],
                        );
                        self.set_error(msg);
                    }
                    // A bytecode listing is generated content: say who produced it.
                    Some(Ok(scmd_view::Started::Derived(producer))) => {
                        let msg = self.i18n.t_args(
                            "viewer-derived",
                            &[("producer", producer.into()), ("name", name.into())],
                        );
                        self.set_status(msg);
                    }
                    _ => {}
                }
            }
            Command::ViewerDecompile if self.mode.viewer().is_some() => self.decompile(),

            Command::ViewerQuit => self.mode = Mode::Normal,
            Command::ViewerToPanel if self.mode.viewer().is_some() => {
                // `Ctrl+F10`: the panel catches up with what is being viewed, which is the
                // point once `Gray +`/`Gray -` have walked away from where F3 was pressed.
                let path = self.mode.viewer().map(|v| v.path.clone());
                if let Some(path) = path {
                    let name = path.display_name();
                    let row = self.panel().listing.row_of_name(&name);
                    match row {
                        Some(row) => {
                            self.panel_mut().cursor_to(row);
                            let msg = self
                                .i18n
                                .t_args("viewer-in-panel", &[("name", name.into())]);
                            self.set_status(msg);
                        }
                        None => {
                            let m = self.i18n.t("viewer-not-in-panel");
                            self.set_error(m);
                        }
                    }
                }
            }
            Command::ViewerNextFile | Command::ViewerPrevFile if self.mode.viewer().is_some() => {
                self.viewer_step_file(if cmd == Command::ViewerNextFile {
                    1
                } else {
                    -1
                });
            }
            Command::ViewerToEditor if self.mode.viewer().is_some() => {
                self.viewer_to_editor();
            }
            Command::ViewerSearch if self.mode.viewer().is_some() => {
                // The prompt says what will be searched. Over a disassembly that is not text,
                // hex or a regular expression but an instruction pattern, so say so.
                let what = match self.mode.viewer().map(|v| v.mode) {
                    Some(ViewerMode::Disasm) => "instructions",
                    _ => self.search_mode.as_str(),
                };
                let title = self
                    .i18n
                    .t_args("viewer-prompt-search", &[("mode", what.into())]);
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
                    Some(e) if e.kind.is_dirlike() => (self.active, self.panel().path_of(e)),
                    _ => {
                        self.not_implemented("file.dirsize (only directories for now)");
                        return;
                    }
                };
                self.requests.push(Request::DirSize { panel, path });
                let m = self.i18n.t("panel-loading");
                self.set_status(m);
            }

            Command::FileCopy
            | Command::FileMove
            | Command::FileCopyCurrent
            | Command::FileMoveCurrent => {
                // `Shift+F5` and `Shift+F6` ignore the selection on purpose: they exist for
                // the moment when something is selected and the file under the cursor is not
                // part of what you meant to touch.
                let only_current =
                    matches!(cmd, Command::FileCopyCurrent | Command::FileMoveCurrent);
                let sources = if only_current {
                    self.cursor_source()
                } else {
                    self.job_sources()
                };
                if sources.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                }
                let what = self.sources_label(&sources);
                let copy = matches!(cmd, Command::FileCopy | Command::FileCopyCurrent);
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
                    secret: false,
                    // The cursor-only keys carry their list with them, so that selecting
                    // something else while the prompt is open cannot change what moves.
                    sources: if only_current { sources } else { Vec::new() },
                });
            }
            Command::FileMkdir => {
                let title = self.i18n.t("prompt-mkdir");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::Mkdir,
                    title,
                    text: String::new(),
                    secret: false,
                    sources: Vec::new(),
                });
            }
            Command::FileDelete | Command::FileDeleteCurrent => {
                let sources = if cmd == Command::FileDeleteCurrent {
                    self.cursor_source()
                } else {
                    self.job_sources()
                };
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
                        ("bytes", scmd_core::format_size(bytes).into()),
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
                // There is no trash yet, so the dialog says what the key really does.
                lines.push(self.i18n.t("confirm-delete-permanent"));
                lines.push(self.i18n.t("confirm-yes-no"));
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines,
                    action: ConfirmAction::Job(spec),
                });
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
        let path = self.panel().path_of(&entry);
        if path.is_remote() || path.in_container() {
            // The viewer reads a real file, so the bytes are brought out first — off a server
            // or out of an archive, the shape is the same.
            self.fetch_remote(path, entry.size, Fetched::View);
            return;
        }
        let local = path.as_path().to_path_buf();
        self.open_viewer_on(&local, path);
    }

    /// `Gray +` / `Gray -`: the next or previous file of the panel, without leaving the
    /// viewer. Directories are skipped — stepping into one would mean measuring it, which is
    /// not what a reader walking through a directory of logs asked for.
    fn viewer_step_file(&mut self, delta: isize) {
        let mode = self.mode.viewer().map(|v| v.mode);
        let len = self.panel().listing.len();
        let mut row = self.panel().cursor as isize;
        let mut found = None;
        for _ in 0..len {
            row += delta;
            if row < 0 || row as usize >= len {
                break;
            }
            let entry = self.panel().listing.row(row as usize).cloned();
            match entry {
                Some(e) if !e.kind.is_dirlike() => {
                    found = Some((row as usize, e));
                    break;
                }
                Some(_) => continue,
                None => break,
            }
        }
        let Some((row, entry)) = found else {
            let m = self.i18n.t("viewer-no-more-files");
            self.set_status(m);
            return;
        };
        self.panel_mut().cursor_to(row);
        let path = self.panel().path_of(&entry);
        if path.is_remote() || path.in_container() {
            self.fetch_remote(path, entry.size, Fetched::View);
            return;
        }
        let local = path.as_path().to_path_buf();
        self.open_viewer_on(&local, path);
        // The reader was in hex or disassembly for a reason; the next file opens the same way.
        if let (Some(mode), Some(viewer)) = (mode, self.mode.viewer_mut()) {
            viewer.set_mode(mode);
        }
    }

    /// `F6` in the viewer: the same file in the editor, on the line the bytes were showing.
    fn viewer_to_editor(&mut self) {
        let Some((path, offset)) = self
            .mode
            .viewer()
            .map(|v| (v.path.clone(), v.top))
            .filter(|(p, _)| !p.is_remote())
        else {
            let m = self.i18n.t("cmd-not-remote");
            self.set_error(m);
            return;
        };
        match Editor::open(path.as_path()) {
            Ok(mut editor) => {
                let line = line_at_offset(&editor, offset);
                editor.goto(&line.to_string());
                if editor.buffer.read_only {
                    let msg = self
                        .i18n
                        .t_args("editor-read-only", &[("name", editor.name().into())]);
                    self.set_status(msg);
                }
                self.mode = Mode::Editor(Box::new(editor));
            }
            Err(e) => self.set_error(format!("{path}: {e}")),
        }
    }

    /// `F6` in the editor: the same file in the viewer, at the byte the cursor is on. Unsaved
    /// changes stay in the editor — the viewer reads the file on disk, and saying so is better
    /// than showing a line that is not there any more.
    fn editor_to_viewer(&mut self) {
        let Some(editor) = self.mode.editor() else {
            return;
        };
        if editor.buffer.modified() {
            let m = self.i18n.t("editor-unsaved-for-viewer");
            self.set_error(m);
            return;
        }
        let Some(path) = editor.path.clone() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let offset = offset_at_line(editor, editor.cursor.line);
        if path.is_remote() {
            let m = self.i18n.t("cmd-not-remote");
            self.set_error(m);
            return;
        }
        let local = path.as_path().to_path_buf();
        self.open_viewer_on(&local, path);
        if let Some(viewer) = self.mode.viewer_mut() {
            viewer.goto(offset);
        }
    }

    /// Ask for a remote file to be copied down. The size limit keeps `F3` on a 4 GB image
    /// from quietly filling the temporary directory.
    fn fetch_remote(&mut self, path: VPath, size: u64, purpose: Fetched) {
        let limit = self.cfg.net.fetch_limit_mb.saturating_mul(1024 * 1024);
        if limit > 0 && size > limit {
            let msg = self.i18n.t_args(
                "net-too-big",
                &[
                    ("name", path.display_name().into()),
                    (
                        "limit",
                        format!("{} MB", self.cfg.net.fetch_limit_mb).into(),
                    ),
                ],
            );
            self.set_error(msg);
            return;
        }
        let id = self.next_job_id;
        self.next_job_id += 1;
        self.fetch_id = Some(id);
        let msg = self
            .i18n
            .t_args("net-fetching", &[("name", path.display_name().into())]);
        self.set_status(msg);
        self.requests.push(Request::Fetch { id, path, purpose });
    }

    /// Open the viewer on a local file, showing `shown` as its name — which is the remote
    /// path when the bytes came down from a server.
    fn open_viewer_on(&mut self, local: &std::path::Path, shown: VPath) {
        match FileSource::open(local) {
            Ok(source) => {
                let remote = shown.is_remote();
                let mut viewer = Viewer::new(Box::new(source), shown.clone());
                if viewer.is_empty() {
                    let m = self.i18n.t("viewer-empty");
                    self.set_status(m);
                } else if remote {
                    let msg = self
                        .i18n
                        .t_args("net-viewing-copy", &[("name", shown.display_name().into())]);
                    self.set_status(msg);
                }
                self.mode = Mode::Viewer(Box::new(viewer));
            }
            Err(e) => self.set_error(format!("{shown}: {e}")),
        }
    }

    // ── netbox ─────────────────────────────────────────────────────────────────────────────

    /// Start (or reuse) a connection and show it in the active panel.
    fn connect_to(&mut self, site: Site, secret: Option<String>, accept_host_key: bool) {
        let panel = self.active;
        let msg = self
            .i18n
            .t_args("net-connecting", &[("host", site.authority().into())]);
        self.set_status(msg);
        self.requests.push(Request::Connect {
            panel,
            site: Box::new(site),
            secret,
            accept_host_key,
        });
    }

    /// The path a site's start directory has, if that host is already connected.
    fn connected_site_path(&self, site: &Site) -> Option<VPath> {
        self.connected
            .contains(&site.authority())
            .then(|| site.start())
    }

    /// What came back from a connection attempt.
    fn connection_result(&mut self, panel: usize, site: Box<Site>, outcome: ConnectOutcome) {
        match outcome {
            ConnectOutcome::Open { start, connected } => {
                self.connected = connected;
                self.pending_connect = None;
                let msg = self
                    .i18n
                    .t_args("net-connected", &[("host", site.authority().into())]);
                self.set_status(msg);
                self.panels[panel].go(start, true);
                self.read_panel(panel);
            }
            ConnectOutcome::NeedsSecret(kind) => {
                let title = self.i18n.t_args(
                    match kind {
                        SecretKind::Password => "net-prompt-password",
                        SecretKind::Passphrase => "net-prompt-passphrase",
                    },
                    &[
                        ("user", site.user.clone().into()),
                        ("host", site.host.clone().into()),
                    ],
                );
                self.pending_connect = Some((panel, site, None));
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::NetSecret(kind),
                    title,
                    text: String::new(),
                    secret: true,
                    sources: Vec::new(),
                });
            }
            ConnectOutcome::UnknownHostKey { fingerprint } => {
                let title = self.i18n.t("net-hostkey-title");
                let lines = vec![
                    self.i18n
                        .t_args("net-hostkey-body", &[("host", site.host.clone().into())]),
                    fingerprint,
                    self.i18n.t("net-hostkey-question"),
                ];
                let secret = self
                    .pending_connect
                    .take()
                    .and_then(|(_, _, secret)| secret);
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines,
                    action: ConfirmAction::AcceptHostKey {
                        panel,
                        site,
                        secret,
                    },
                });
            }
            ConnectOutcome::Failed(detail) => {
                self.pending_connect = None;
                self.set_error(detail);
            }
        }
    }

    /// `Alt+Shift+N`: close the connection the active panel is on.
    fn disconnect_active(&mut self) {
        match self.panel().cwd.authority() {
            Some(authority) => {
                let authority = authority.to_string();
                self.requests.push(Request::Disconnect { authority });
            }
            None => {
                let m = self.i18n.t("net-local-panel");
                self.set_status(m);
            }
        }
    }

    // ── the command line (SPEC 06 §7) ──────────────────────────────────────────────────────

    /// Keys the command line owns while it has something on it. Returns true when it took one.
    fn command_line_key(&mut self, key: Key) -> bool {
        if !self.cfg.ui.command_line {
            return false;
        }
        let ctrl = key.mods.contains(Mods::CTRL);
        let empty = self.cmd.is_empty();
        // A line with something on it takes every printable character, `+`, `-`, `*` and
        // Space included: those keys belong to the panel only while the line is empty, which
        // is FAR's rule and the reason the command line needs no focus of its own.
        if !empty {
            if let Some(c) = key.as_typed_char() {
                self.cmd.insert(c);
                return true;
            }
        }
        match key.code {
            KeyCode::Enter if !empty && !ctrl => {
                self.run_command_line();
                true
            }
            KeyCode::Esc if !empty => {
                self.cmd.clear();
                true
            }
            KeyCode::Backspace if !empty && ctrl => {
                self.cmd.delete_word_left();
                true
            }
            KeyCode::Backspace if !empty => {
                self.cmd.backspace();
                true
            }
            KeyCode::Delete if !empty && ctrl => {
                self.cmd.delete_word_right();
                true
            }
            KeyCode::Delete if !empty => {
                self.cmd.delete();
                true
            }
            KeyCode::Left if !empty && ctrl => {
                self.cmd.word_left();
                true
            }
            KeyCode::Right if !empty && ctrl => {
                self.cmd.word_right();
                true
            }
            KeyCode::Left if !empty => {
                self.cmd.left();
                true
            }
            KeyCode::Right if !empty => {
                self.cmd.right();
                true
            }
            KeyCode::Home if !empty => {
                self.cmd.home();
                true
            }
            KeyCode::End if !empty => {
                self.cmd.end();
                true
            }
            KeyCode::Char('k') if !empty && ctrl => {
                self.cmd.kill_to_end();
                true
            }
            _ => false,
        }
    }

    /// `Ctrl+J` and friends: put a name or a path from one of the panels on the line, quoted
    /// when it needs to be, with a trailing space so the next word can be typed straight away.
    fn insert_into_line(&mut self, path_only: bool, other: bool) {
        let panel = if other {
            &self.panels[1 - self.active]
        } else {
            self.panel()
        };
        let text = if path_only {
            panel.cwd.to_string()
        } else {
            match panel
                .current()
                .filter(|e| e.kind != scmd_core::EntryKind::Parent)
            {
                Some(entry) => entry.name.to_string_lossy().into_owned(),
                None => panel.cwd.to_string(),
            }
        };
        let quoted = expand::quote(&text);
        self.cmd.insert_str(&quoted);
        self.cmd.insert(' ');
    }

    /// What the panels hold, for the substitutions (SPEC 06 §5b).
    fn expand_context(&self) -> expand::Context {
        let active = self.panel();
        let passive = &self.panels[1 - self.active];
        let name_of = |p: &Panel| {
            p.current()
                .filter(|e| e.kind != scmd_core::EntryKind::Parent)
                .map(|e| e.name.to_string_lossy().into_owned())
        };
        // On a list panel the rows live in other directories, so "the directory" is the one
        // the file under the cursor is in rather than the panel's own.
        let dir_of = |p: &Panel| match p.temp.is_some() {
            true => p
                .current_path()
                .and_then(|path| path.parent())
                .map(|path| path.as_path().to_path_buf())
                .unwrap_or_else(|| p.cwd.as_path().to_path_buf()),
            false => p.cwd.as_path().to_path_buf(),
        };
        // The selection, or the item under the cursor when nothing is marked — which is what
        // every commander means by "the selection".
        let selection: Vec<String> = {
            let mut marked: Vec<String> = Vec::new();
            for row in 0..active.listing.len() {
                if let Some(e) = active.listing.row(row) {
                    if e.kind != scmd_core::EntryKind::Parent && active.selection.contains(e.id) {
                        marked.push(e.name.to_string_lossy().into_owned());
                    }
                }
            }
            if marked.is_empty() {
                name_of(active).into_iter().collect()
            } else {
                marked
            }
        };
        expand::Context {
            active_dir: dir_of(active),
            active_file: name_of(active),
            passive_dir: dir_of(passive),
            passive_file: name_of(passive),
            left_dir: self.panels[0].cwd.as_path().to_path_buf(),
            right_dir: self.panels[1].cwd.as_path().to_path_buf(),
            selection,
        }
    }

    /// `Enter` on a non-empty command line.
    fn run_command_line(&mut self) {
        let line = self.cmd.text.trim().to_string();
        if line.is_empty() {
            return;
        }
        self.cmd.remember(&line);
        self.cmd.trim_history(self.cfg.ui.command_history);
        self.cmd.clear();
        self.run_line(&line);
    }

    /// Run a line that has already been remembered: substitutions, then conditions, then
    /// whatever kind of command it turns out to be.
    fn run_line(&mut self, line: &str) {
        let ctx = self.expand_context();
        let list_file = if expand::wants_list_file(line) {
            match self.write_list_file(&ctx.selection) {
                Ok(path) => Some(path),
                Err(e) => {
                    self.set_error(e);
                    return;
                }
            }
        } else {
            None
        };
        let expanded = match expand::expand(line, &ctx, list_file.as_deref()) {
            Ok(expand::Expanded::Line(line)) => line,
            Ok(expand::Expanded::Ask {
                title,
                initial,
                pending,
                ..
            }) => {
                // A line that asks is put back as a prompt; the answer goes in and it runs.
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::CommandAnswer { pending },
                    title,
                    text: initial,
                    secret: false,
                    sources: Vec::new(),
                });
                return;
            }
            Err(e) => {
                self.set_error(e.to_string());
                return;
            }
        };

        let cwd = self.panel().cwd.clone();
        let local = cwd.as_path().to_path_buf();
        let kept = expanded.clone();
        let resolved = cmdline::resolve_conditions(
            &expanded,
            &|path: &str| {
                let candidate = std::path::Path::new(path);
                if candidate.is_absolute() {
                    candidate.exists()
                } else {
                    local.join(candidate).exists()
                }
            },
            &|name: &str| std::env::var_os(name).is_some(),
        );
        let Some(to_run) = resolved else {
            let msg = self
                .i18n
                .t_args("cmd-condition-false", &[("command", kept.into())]);
            self.set_status(msg);
            return;
        };

        match cmdline::parse(&to_run) {
            Parsed::Empty => {}
            Parsed::Internal(internal) => self.run_internal(internal),
            Parsed::Prefix(prefix) => self.run_prefix(prefix),
            Parsed::Shell(line) => self.run_shell(line, None),
        }
    }

    /// The selection as a file, one name per line, for `{list}`.
    fn write_list_file(&self, names: &[String]) -> Result<std::path::PathBuf, String> {
        if names.is_empty() {
            return Err(self.i18n.t("cmd-no-selection"));
        }
        let dir = std::env::temp_dir();
        let path = dir.join(format!("scmd-list-{}.txt", std::process::id()));
        let body = names.join("\n") + "\n";
        std::fs::write(&path, body).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    fn run_internal(&mut self, internal: Internal) {
        match internal {
            Internal::Cd { path, local_only } => self.run_cd(&path, local_only),
            Internal::Cls => {
                self.user_screen.clear();
                let msg = self.i18n.t("cmd-screen-cleared");
                self.set_status(msg);
            }
            Internal::Exit => self.quit = true,
            Internal::Set { name, value } => self.run_set(name, value),
            Internal::PushD(path) => {
                let here = self.panel().cwd.as_path().to_path_buf();
                self.cmd.dirs.push(here);
                match path {
                    Some(path) => self.run_cd(&path, false),
                    None => {
                        let msg = self.i18n.t_args(
                            "cmd-pushed",
                            &[("count", (self.cmd.dirs.len() as i64).into())],
                        );
                        self.set_status(msg);
                    }
                }
            }
            Internal::PopD => match self.cmd.dirs.pop() {
                Some(path) => self.goto(VPath::local(path)),
                None => {
                    let msg = self.i18n.t("cmd-stack-empty");
                    self.set_error(msg);
                }
            },
            Internal::ClrD => {
                self.cmd.dirs.clear();
                let msg = self.i18n.t("cmd-stack-cleared");
                self.set_status(msg);
            }
            Internal::Title(_) => self.not_implemented("title"),
        }
    }

    /// `cd`, `chdir`, and a bare drive letter on Windows. A remote panel takes `cd` inside the
    /// provider; `chdir` always means the local filesystem.
    fn run_cd(&mut self, argument: &str, local_only: bool) {
        let panel = self.panel();
        let here = panel.cwd.clone();
        let remote = here.is_remote() && !local_only;
        let argument = argument.trim();

        if remote {
            // On a remote panel the path is the provider's, so it is resolved as a string.
            let target = match argument {
                "" | "~" => here.root(),
                "-" => match self.prev_dir[self.active].clone() {
                    Some(previous) => previous,
                    None => {
                        let msg = self.i18n.t("cmd-no-previous");
                        self.set_error(msg);
                        return;
                    }
                },
                // An absolute remote path is parsed as a whole, so the authority travels with
                // it; anything else hangs off where we are.
                _ if argument.starts_with('/') => VPath::parse(&format!(
                    "{}://{}{}",
                    here.scheme().as_str(),
                    here.authority().unwrap_or(""),
                    argument
                )),
                ".." => here.parent().unwrap_or_else(|| here.clone()),
                _ => here.join(argument),
            };
            self.prev_dir[self.active] = Some(here);
            self.goto(target);
            return;
        }

        let home = dirs_home().map(std::path::PathBuf::from);
        let previous = self.prev_dir[self.active]
            .clone()
            .map(|p| p.as_path().to_path_buf());
        let Some(target) = cmdline::resolve_cd(
            argument,
            here.as_path(),
            home.as_deref(),
            previous.as_deref(),
        ) else {
            let msg = self.i18n.t("cmd-no-previous");
            self.set_error(msg);
            return;
        };
        if !target.is_dir() {
            let msg = self.i18n.t_args(
                "cmd-no-such-dir",
                &[("path", target.display().to_string().into())],
            );
            self.set_error(msg);
            return;
        }
        self.prev_dir[self.active] = Some(here);
        self.goto(VPath::local(target).normalised());
    }

    /// `set`, which only ever affects the children we start — never the shell that started us.
    fn run_set(&mut self, name: Option<String>, value: Option<String>) {
        match (name, value) {
            (None, _) => {
                let mut lines: Vec<String> =
                    std::env::vars().map(|(k, v)| format!("{k}={v}")).collect();
                lines.sort();
                let title = self.i18n.t("cmd-environment");
                self.push_user_screen(&title, &lines.join("\n"));
                self.mode = Mode::UserScreen { scroll: 0 };
            }
            (Some(name), None) => match std::env::var(&name) {
                Ok(value) => {
                    let msg = format!("{name}={value}");
                    self.set_status(msg);
                }
                Err(_) => {
                    let msg = self
                        .i18n
                        .t_args("cmd-not-set", &[("name", name.clone().into())]);
                    self.set_status(msg);
                }
            },
            (Some(name), Some(value)) => {
                if value.is_empty() {
                    std::env::remove_var(&name);
                } else {
                    std::env::set_var(&name, &value);
                }
                let msg = self
                    .i18n
                    .t_args("cmd-set", &[("name", name.into()), ("value", value.into())]);
                self.set_status(msg);
            }
        }
    }

    fn run_prefix(&mut self, prefix: Prefix) {
        match prefix {
            Prefix::View(arg) => match self.resolve_argument(&arg) {
                Some(path) if path.as_path().is_file() => {
                    let local = path.as_path().to_path_buf();
                    self.open_viewer_on(&local, path);
                }
                _ => self.argument_missing(&arg),
            },
            Prefix::Edit(arg) => match self.resolve_argument(&arg) {
                // A name that does not exist yet is a new file, which is what `edit:` is for.
                Some(path) => match Editor::open(path.as_path()) {
                    Ok(editor) => self.mode = Mode::Editor(Box::new(editor)),
                    Err(e) => self.set_error(format!("{path}: {e}")),
                },
                None => self.argument_missing(&arg),
            },
            Prefix::Goto(arg) | Prefix::WhereIs(arg) => {
                let Some(path) = self.resolve_argument(&arg) else {
                    self.argument_missing(&arg);
                    return;
                };
                // A directory is where we go; a file is where we go *and* what the cursor
                // lands on, which is the whole point of `goto:`.
                let as_path = path.as_path().to_path_buf();
                if as_path.is_dir() {
                    self.goto(path);
                } else if let Some(parent) = path.parent() {
                    let name = as_path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.goto(parent);
                    self.panels[self.active].want_cursor = Some(name);
                } else {
                    self.argument_missing(&arg);
                }
            }
            Prefix::Run { into, command } => {
                let into = into
                    .and_then(|f| self.resolve_argument(&f))
                    .map(|p| p.as_path().to_path_buf());
                self.run_shell(command, into);
            }
        }
    }

    /// A prefix argument: absolute, or relative to the active panel.
    fn resolve_argument(&self, argument: &str) -> Option<VPath> {
        let argument = argument.trim().trim_matches('"');
        if argument.is_empty() {
            return None;
        }
        let path = std::path::Path::new(argument);
        if path.is_absolute() {
            return Some(VPath::local(path).normalised());
        }
        Some(self.panel().cwd.join(argument))
    }

    fn argument_missing(&mut self, argument: &str) {
        let msg = self
            .i18n
            .t_args("cmd-no-such-path", &[("path", argument.to_string().into())]);
        self.set_error(msg);
    }

    /// Hand a line to the user's shell, off the UI thread.
    fn run_shell(&mut self, line: String, into: Option<std::path::PathBuf>) {
        if self.shell.is_some() {
            let msg = self.i18n.t("cmd-busy");
            self.set_error(msg);
            return;
        }
        let cwd = self.panel().cwd.clone();
        if cwd.is_remote() {
            let msg = self.i18n.t("cmd-not-remote");
            self.set_error(msg);
            return;
        }
        let id = self.next_job_id;
        self.next_job_id += 1;
        self.shell = Some((id, line.clone()));
        self.requests.push(Request::Shell {
            id,
            line: line.clone(),
            cwd: cwd.as_path().to_path_buf(),
            into,
        });
        let msg = self.i18n.t_args("cmd-running", &[("command", line.into())]);
        self.set_status(msg);
    }

    /// Start the next command `Ctrl+G` has queued, counting through the list in the status
    /// line so a long run says where it is.
    fn run_next_apply(&mut self) {
        if self.apply_queue.is_empty() {
            self.apply_total = 0;
            return;
        }
        let line = self.apply_queue.remove(0);
        let done = self.apply_total - self.apply_queue.len();
        let total = self.apply_total;
        self.run_shell(line.clone(), None);
        if self.shell.is_some() {
            let msg = self.i18n.t_args(
                "cmd-apply-progress",
                &[
                    ("command", line.into()),
                    ("done", (done as i64).into()),
                    ("total", (total as i64).into()),
                ],
            );
            self.set_status(msg);
        }
    }

    /// OK in the replace dialog: count first, always. The dry run and the write share the
    /// engine's one code path, so what the confirmation says is what the write will do.
    /// `Shift+F1`: the dialog for a new archive out of what is selected.
    fn open_pack_form(&mut self) {
        let sources = self.job_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        // FAR's default: one item takes its own name, several take the directory's, and the
        // archive is offered in the other panel — which is where a user looking at two panels
        // means to put it.
        let stem = if sources.len() == 1 {
            sources[0].display_name()
        } else {
            self.panel().cwd.display_name()
        };
        let stem = stem.trim_end_matches('/').to_string();
        let name = self
            .other()
            .cwd
            .join(format!("{stem}.{}", self.last_pack.extension()))
            .to_string();
        self.mode = Mode::Form(Box::new(Form::pack(
            name,
            self.last_pack,
            sources.len(),
            &self.i18n,
        )));
    }

    /// OK in the archive dialog: work out the format, and start writing.
    fn apply_pack_form(&mut self, form: &Form) {
        let name = form.text_value("pack.name");
        if name.is_empty() {
            let m = self.i18n.t("pack-needs-name");
            self.set_error(m);
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let sources = self.job_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let archive = self.resolve_path(&name);
        // The name wins over the choice: a user who typed `.tar.gz` means that, whatever the
        // row below says, and a name with no format in it takes the one that is chosen.
        let chosen = scmd_arc::Format::ALL
            .iter()
            .copied()
            .find(|f| f.extension() == form.text_value("pack.format"))
            .unwrap_or(scmd_arc::Format::Zip);
        let format = scmd_arc::Format::of_name(&archive.display_name()).unwrap_or(chosen);
        let level = match form.text_value("pack.level").as_str() {
            "fast" => scmd_arc::Level::Fast,
            "best" => scmd_arc::Level::Best,
            _ => scmd_arc::Level::Normal,
        };
        self.last_pack = format;
        self.start_pack(archive, format, level, sources);
    }

    /// `Shift+F2`: the dialog for taking an archive apart.
    fn open_unpack_form(&mut self) {
        // Inside an archive this is a copy of what is selected, which `F5` already does well.
        if self.panel().cwd.in_container() {
            self.run(Command::FileCopy);
            return;
        }
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let path = self.panel().path_of(&entry);
        let name = path.display_name();
        if scmd_arc::Format::of_name(&name).is_none() {
            let m = self.i18n.t("unpack-not-an-archive");
            self.set_error(m);
            return;
        }
        // A directory of its own, named after the archive without its extension: unpacking a
        // hundred files straight into a panel is nobody's intention.
        let stem = name
            .strip_suffix(&format!(
                ".{}",
                scmd_arc::Format::of_name(&name)
                    .map(|f| f.extension())
                    .unwrap_or("")
            ))
            .unwrap_or(&name)
            .to_string();
        let to = self.other().cwd.join(stem).to_string();
        self.mode = Mode::Form(Box::new(Form::unpack(to, name, &self.i18n)));
    }

    /// OK in the extract dialog: an ordinary copy job, out of the archive.
    fn apply_unpack_form(&mut self, form: &Form) {
        let to = form.text_value("unpack.to");
        if to.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let Some(entry) = self.panel().current().cloned() else {
            return;
        };
        let archive = self.panel().path_of(&entry);
        // The archive's contents, not a directory named after it.
        let options = scmd_jobs::JobOptions {
            contents: true,
            ..Default::default()
        };
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![archive.enter("/")],
            Some(self.resolve_path(&to)),
        )
        .with_options(options);
        self.start_job(spec);
    }

    fn apply_replace_form(&mut self, form: &Form) {
        let needle = form.text_value("replace.needle");
        if needle.is_empty() {
            let m = self.i18n.t("replace-needs-needle");
            self.set_error(m);
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let options = scmd_jobs::ReplaceOptions {
            needle,
            replacement: form.text_value("replace.with"),
            case_sensitive: form.toggle_value("replace.case"),
            whole_words: form.toggle_value("replace.words"),
            pattern: form.toggle_value("replace.pattern"),
            hex: form.toggle_value("replace.hex"),
        };
        let sources = self.job_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        self.last_replace = options.clone();

        // A rewritten file should look modified, so its old modification time is not kept.
        let mut real_options = scmd_jobs::JobOptions::default();
        real_options.preserve.mtime = false;
        let real = JobSpec::new(JobKind::Replace, sources.clone(), None)
            .with_replace(options.clone())
            .with_options(real_options);
        self.pending_replace = Some(real);

        let mut counting = real_options;
        counting.dry_run = true;
        let dry = JobSpec::new(JobKind::Replace, sources, None)
            .with_replace(options)
            .with_options(counting);
        self.start_job(dry);
    }

    /// The count came back: say what would change and ask, or say that nothing would.
    fn replace_counted(&mut self, outcome: &Outcome) {
        let Some(real) = self.pending_replace.take() else {
            return;
        };
        let occurrences = outcome.occurrences();
        if occurrences == 0 || outcome.replacements.is_empty() {
            let m = self.i18n.t("replace-nothing");
            self.set_status(m);
            return;
        }
        let title = self.i18n.t("replace-confirm-title");
        let body = self.i18n.t_args(
            "replace-confirm-body",
            &[
                ("count", (occurrences as i64).into()),
                ("files", (outcome.replacements.len() as i64).into()),
            ],
        );
        let mut lines = vec![body, String::new()];
        lines.extend(
            outcome
                .replacements
                .iter()
                .take(5)
                .map(|(path, n)| format!("  {}  ({n})", path.display_name())),
        );
        if outcome.replacements.len() > 5 {
            lines.push(format!("  … {}", outcome.replacements.len() - 5));
        }
        lines.push(String::new());
        lines.push(self.i18n.t("confirm-yes-no"));
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action: ConfirmAction::Job(real),
        });
    }

    /// `Alt+F7`: the dialog, filled in with the last search.
    fn open_find_form(&mut self) {
        self.mode = Mode::Form(Box::new(Form::find(&self.last_find, &self.i18n)));
    }

    /// OK in the find dialog: work out where to search, and start.
    fn apply_find_form(&mut self, form: &Form) {
        let mask = form.text_value("find.mask");
        if mask.is_empty() {
            let m = self.i18n.t("find-needs-mask");
            self.set_error(m);
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let scope = FindScope::parse(&form.choice_value("find.scope"));
        let mut spec = FindSpec {
            mask,
            text: form.text_value("find.text"),
            case_sensitive: form.toggle_value("find.case"),
            whole_words: form.toggle_value("find.words"),
            hex: form.toggle_value("find.hex"),
            not_containing: form.toggle_value("find.not"),
            scope,
            follow_links: form.toggle_value("find.links"),
            directories: form.toggle_value("find.dirs"),
            roots: Vec::new(),
        };
        spec.roots = self.find_roots(scope);
        if spec.roots.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        self.last_find = spec.clone();
        self.start_find(spec);
    }

    /// Where a scope starts. Worked out here, where the panels are, so that neither the
    /// engine nor the worker has to know what a panel is.
    fn find_roots(&self, scope: FindScope) -> Vec<VPath> {
        let panel = self.panel();
        match scope {
            FindScope::Tree | FindScope::Panel => vec![panel.cwd.clone()],
            FindScope::Other => vec![self.other().cwd.clone()],
            FindScope::Root => vec![panel.cwd.root()],
            FindScope::Selection => {
                // The directories that are selected; with none selected, the one under the
                // cursor if it is a directory, and otherwise nothing rather than silently
                // searching the whole tree.
                let mut out = Vec::new();
                for row in 0..panel.listing.len() {
                    let Some(e) = panel.listing.row(row) else {
                        continue;
                    };
                    if e.kind.is_dirlike()
                        && e.kind != scmd_core::EntryKind::Parent
                        && panel.selection.contains(e.id)
                    {
                        out.push(panel.path_of(e));
                    }
                }
                if out.is_empty() {
                    if let Some(e) = panel.current() {
                        if e.kind.is_dirlike() && e.kind != scmd_core::EntryKind::Parent {
                            out.push(panel.path_of(e));
                        }
                    }
                }
                out
            }
        }
    }

    fn start_find(&mut self, spec: FindSpec) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        let label = self.find_label(&spec);
        self.requests.push(Request::Find {
            id,
            spec: Box::new(spec),
            cancel: cancel.clone(),
        });
        self.mode = Mode::Find(Box::new(FindView {
            id,
            label,
            rows: Vec::new(),
            cursor: 0,
            top: 0,
            scanning: None,
            files: 0,
            dirs: 0,
            done: false,
            stopped: false,
            elapsed: std::time::Duration::ZERO,
            cancel,
            problems: Vec::new(),
        }));
    }

    /// What the results frame says: the mask, the text, and where it looked.
    fn find_label(&self, spec: &FindSpec) -> String {
        let mut label = spec.mask.clone();
        if !spec.text.is_empty() {
            label.push_str(" · ");
            label.push_str(&spec.text);
        }
        let root = spec
            .roots
            .first()
            .map(|p| p.to_string())
            .unwrap_or_default();
        let more = spec.roots.len().saturating_sub(1);
        if more > 0 {
            format!("{label} · {root} +{more}")
        } else {
            format!("{label} · {root}")
        }
    }

    /// Keys in the results of a search. The list grows under the cursor while the search runs,
    /// which is the point: a result is useful before the search has finished.
    fn find_key(&mut self, key: Key) {
        let page = self.rows.max(1) as isize;
        let (rows, done, cancel) = {
            let Some(view) = self.find_mut() else { return };
            (view.rows.len(), view.done, view.cancel.clone())
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                if done {
                    self.mode = Mode::Normal;
                } else {
                    // The first Esc stops the search and keeps what it found; the second
                    // closes the list. Nothing is thrown away by accident.
                    cancel.cancel();
                    if let Some(view) = self.find_mut() {
                        view.stopped = true;
                    }
                }
            }
            KeyCode::Up => self.find_move(-1),
            KeyCode::Down => self.find_move(1),
            KeyCode::PageUp => self.find_move(-page),
            KeyCode::PageDown => self.find_move(page),
            KeyCode::Home => self.find_move(-(rows as isize)),
            KeyCode::End => self.find_move(rows as isize),
            KeyCode::Enter => self.find_go_to(),
            KeyCode::F(3) => self.find_view_file(),
            KeyCode::F(5) => self.find_panelise(),
            _ => {}
        }
    }

    fn find_mut(&mut self) -> Option<&mut FindView> {
        match &mut self.mode {
            Mode::Find(view) => Some(view),
            _ => None,
        }
    }

    fn find_move(&mut self, delta: isize) {
        let page = self.rows.max(1);
        if let Some(view) = self.find_mut() {
            view.move_cursor(delta, page);
        }
    }

    /// `Enter`: go to the file — the panel changes directory and the cursor lands on it.
    fn find_go_to(&mut self) {
        let chosen = match &self.mode {
            Mode::Find(view) => view.rows.get(view.cursor).cloned(),
            _ => None,
        };
        let Some((path, entry)) = chosen else { return };
        self.mode = Mode::Normal;
        let dir = if entry.kind.is_dirlike() {
            path.clone()
        } else {
            match path.parent() {
                Some(dir) => dir.normalised(),
                None => path.clone(),
            }
        };
        let index = self.active;
        self.panels[index].want_cursor = Some(path.display_name());
        self.panels[index].go(dir, true);
        self.read_panel(index);
    }

    /// `F3`: read the file without leaving the results.
    fn find_view_file(&mut self) {
        let chosen = match &self.mode {
            Mode::Find(view) => view.rows.get(view.cursor).cloned(),
            _ => None,
        };
        let Some((path, entry)) = chosen else { return };
        if entry.kind.is_dirlike() {
            return;
        }
        if path.is_remote() || path.in_container() {
            self.fetch_remote(path, entry.size, Fetched::View);
            return;
        }
        let local = path.as_path().to_path_buf();
        self.open_viewer_on(&local, path);
    }

    /// `F5`: put the results in the panel, which is what makes them worth something — every
    /// key that works on a panel then works on them.
    fn find_panelise(&mut self) {
        let (rows, label) = match &mut self.mode {
            Mode::Find(view) => (std::mem::take(&mut view.rows), view.label.clone()),
            _ => return,
        };
        if rows.is_empty() {
            let m = self.i18n.t("find-none");
            self.set_error(m);
            return;
        }
        let count = rows.len();
        self.mode = Mode::Normal;
        let index = self.active;
        let generation = self.panels[index].generation + 1;
        self.panels[index].generation = generation;
        self.panels[index].set_temp(label, rows, generation);
        let msg = self
            .i18n
            .t_args("find-panelised", &[("count", (count as i64).into())]);
        self.set_status(msg);
    }

    /// Keep what a command printed, capped, with the command itself as its heading.
    fn push_user_screen(&mut self, line: &str, output: &str) {
        const MAX_LINES: usize = 10_000;
        if !self.user_screen.is_empty() {
            self.user_screen.push(String::new());
        }
        self.user_screen.push(format!("$ {line}"));
        for row in output.lines() {
            self.user_screen.push(row.to_string());
        }
        if self.user_screen.len() > MAX_LINES {
            let drop = self.user_screen.len() - MAX_LINES;
            self.user_screen.drain(..drop);
        }
    }

    fn user_screen_key(&mut self, key: Key, scroll: usize) {
        let step = self.rows.max(1);
        let last = self.user_screen.len().saturating_sub(1);
        let set = |app: &mut App, at: usize| {
            app.mode = Mode::UserScreen {
                scroll: at.min(last),
            }
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') | KeyCode::Char('o') => {
                self.mode = Mode::Normal
            }
            KeyCode::Up => set(self, scroll.saturating_sub(1)),
            KeyCode::Down => set(self, scroll + 1),
            KeyCode::PageUp => set(self, scroll.saturating_sub(step)),
            KeyCode::PageDown => set(self, scroll + step),
            KeyCode::Home => set(self, 0),
            KeyCode::End => set(self, last),
            _ => {}
        }
    }

    // ── the menu bar and its dialogs (SPEC 10) ─────────────────────────────────────────────

    /// `F9`: open the menu bar.
    fn open_menu(&mut self) {
        let menu = if self.active == 1 { 4 } else { 0 };
        self.mode = Mode::Menu {
            menu,
            item: self.first_item(menu),
        };
    }

    fn first_item(&self, menu: usize) -> usize {
        menu::menus()
            .get(menu)
            .and_then(|m| {
                m.items
                    .iter()
                    .position(|e| matches!(e, menu::Entry::Item(_)))
            })
            .unwrap_or(0)
    }

    /// Walk to the next or previous entry, stepping over separators.
    fn menu_step(&self, menu: usize, item: usize, down: bool) -> usize {
        let menus = menu::menus();
        let Some(m) = menus.get(menu) else {
            return item;
        };
        let len = m.items.len();
        if len == 0 {
            return item;
        }
        let mut i = item;
        for _ in 0..len {
            i = if down {
                (i + 1) % len
            } else {
                (i + len - 1) % len
            };
            if matches!(m.items.get(i), Some(menu::Entry::Item(_))) {
                return i;
            }
        }
        item
    }

    fn menu_key(&mut self, key: Key, menu: usize, item: usize) {
        let menus = menu::menus();
        let count = menus.len();
        match key.code {
            KeyCode::Esc | KeyCode::F(9) | KeyCode::F(10) => self.mode = Mode::Normal,
            KeyCode::Left => {
                let menu = (menu + count - 1) % count;
                self.mode = Mode::Menu {
                    menu,
                    item: self.first_item(menu),
                };
            }
            KeyCode::Right => {
                let menu = (menu + 1) % count;
                self.mode = Mode::Menu {
                    menu,
                    item: self.first_item(menu),
                };
            }
            KeyCode::Up => {
                self.mode = Mode::Menu {
                    menu,
                    item: self.menu_step(menu, item, false),
                }
            }
            KeyCode::Down => {
                self.mode = Mode::Menu {
                    menu,
                    item: self.menu_step(menu, item, true),
                }
            }
            KeyCode::Home => {
                self.mode = Mode::Menu {
                    menu,
                    item: self.first_item(menu),
                }
            }
            KeyCode::End => {
                self.mode = Mode::Menu {
                    menu,
                    item: self.menu_step(menu, 0, false),
                }
            }
            KeyCode::Enter => self.run_menu_item(menu, item),
            // A letter picks the first entry whose label starts with it, as FAR does.
            KeyCode::Char(c) if c.is_alphanumeric() => {
                let wanted = c.to_lowercase().next().unwrap_or(c);
                let found = menus.get(menu).and_then(|m| {
                    m.items.iter().position(|e| match e {
                        menu::Entry::Item(i) => self
                            .i18n
                            .t(i.label)
                            .chars()
                            .next()
                            .map(|f| f.to_lowercase().next().unwrap_or(f) == wanted)
                            .unwrap_or(false),
                        menu::Entry::Separator => false,
                    })
                });
                if let Some(index) = found {
                    self.run_menu_item(menu, index);
                }
            }
            _ => {}
        }
    }

    /// Do what the entry says. Left and Right aim at their own panel, as FAR does.
    fn run_menu_item(&mut self, menu_index: usize, item: usize) {
        let menus = menu::menus();
        let Some(m) = menus.get(menu_index) else {
            self.mode = Mode::Normal;
            return;
        };
        let side = m.side;
        let Some(menu::Entry::Item(entry)) = m.items.get(item) else {
            return;
        };
        let action = entry.action.clone();
        self.mode = Mode::Normal;
        if let Some(side) = side {
            self.active = side;
        }
        match action {
            Action::Run(cmd) => self.run(cmd),
            Action::Todo(what) => self.not_implemented(what),
        }
    }

    /// Is the `√` on? The left and right menus mark the state of their own panel.
    pub fn menu_mark(&self, mark: &Mark, side: Option<usize>) -> bool {
        let panel = &self.panels[side.unwrap_or(self.active)];
        match mark {
            Mark::None => false,
            Mark::View(mode) => panel.view == *mode,
            Mark::Hidden => panel.show_hidden,
        }
    }

    pub fn open_form(&mut self, kind: FormKind) {
        self.mode = Mode::Form(Box::new(Form::build(kind, &self.cfg, &self.i18n)));
    }

    fn form_key(&mut self, key: Key) {
        let Mode::Form(form) = &mut self.mode else {
            return;
        };
        // A row that takes text takes the editing keys too, so a path with spaces or a minus
        // in it can be typed.
        let text = form.on_text() && form.button.is_none();
        match key.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Up => {
                if form.button.is_some() {
                    form.button = None;
                } else {
                    form.move_cursor(false)
                }
            }
            KeyCode::Down => form.move_cursor(true),
            KeyCode::Char(c) if text && !key.mods.contains(Mods::CTRL) => {
                form.type_char(c);
            }
            KeyCode::Backspace if text => {
                form.edit_text(TextEdit::Backspace);
            }
            KeyCode::Delete if text => {
                form.edit_text(TextEdit::Delete);
            }
            KeyCode::Home if text => {
                form.edit_text(TextEdit::Home);
            }
            KeyCode::End if text => {
                form.edit_text(TextEdit::End);
            }
            KeyCode::Left if text => {
                form.edit_text(TextEdit::Left);
            }
            KeyCode::Right if text => {
                form.edit_text(TextEdit::Right);
            }
            KeyCode::Char(' ') => form.toggle(true),
            KeyCode::Left => form.toggle(false),
            KeyCode::Right => form.toggle(true),
            KeyCode::Tab => {
                form.button = Some(match form.button {
                    None => 0,
                    Some(0) => 1,
                    Some(_) => 0,
                })
            }
            KeyCode::Enter => {
                // Enter on a row toggles it; on the buttons it answers the dialog. In a text
                // row there is nothing to toggle, so Enter means OK.
                match form.button {
                    Some(1) => self.mode = Mode::Normal,
                    Some(_) => self.apply_form(),
                    None if text => self.apply_form(),
                    None => form.toggle(true),
                }
            }
            KeyCode::F(10) => self.apply_form(),
            _ => {}
        }
    }

    /// OK: write the dialog into the configuration and act on what changed at once.
    fn apply_form(&mut self) {
        let Mode::Form(form) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        if form.kind == FormKind::Site {
            self.apply_site_form(&form);
            return;
        }
        if form.kind == FormKind::Find {
            self.apply_find_form(&form);
            return;
        }
        if form.kind == FormKind::Replace {
            self.apply_replace_form(&form);
            return;
        }
        if form.kind == FormKind::Pack {
            self.apply_pack_form(&form);
            return;
        }
        if form.kind == FormKind::Unpack {
            self.apply_unpack_form(&form);
            return;
        }
        let problems = form.apply(&mut self.cfg);
        if !problems.is_empty() {
            self.set_error(problems.join("; "));
            return;
        }
        self.apply_config(&form.keys());
        let msg = self.i18n.t("opt-applied");
        self.set_status(msg);
    }

    /// Push the configuration into the running panels: a dialog is useless if the change only
    /// shows up after a restart. Only the keys the dialog owns are pushed, so a panel's own
    /// sort order — the one chosen with `Ctrl+F3..F7` — survives an unrelated dialog.
    pub fn apply_config(&mut self, keys: &[&str]) {
        let mut touched = false;
        for panel in &mut self.panels {
            for key in keys {
                match *key {
                    "panel.sort" => panel.sort.key = parse_sort(&self.cfg.panel.sort),
                    "panel.reverse" => panel.sort.reverse = self.cfg.panel.reverse,
                    "panel.dirs_first" => panel.sort.dirs_first = self.cfg.panel.dirs_first,
                    "panel.selected_first" => {
                        panel.sort.selected_first = self.cfg.panel.selected_first
                    }
                    "panel.natural_sort" => panel.sort.natural = self.cfg.panel.natural_sort,
                    "panel.show_hidden" => panel.show_hidden = self.cfg.panel.show_hidden,
                    "panel.view" => {
                        panel.view = ViewMode::parse(&self.cfg.panel.view).unwrap_or(panel.view)
                    }
                    _ => continue,
                }
                touched = true;
            }
            if touched {
                panel.rebuild();
            }
        }
    }

    /// Write the configuration file, so the dialogs are not lost on exit.
    fn save_setup(&mut self) {
        let path = self.paths.config_file.clone();
        if let Some(parent) = path.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                self.set_error(format!("{}: {e}", parent.display()));
                return;
            }
        }
        let mut text = self.cfg.dump();
        for theme in self.themes.iter() {
            if theme.source == scmd_ui::ThemeSource::Config {
                text.push('\n');
                text.push_str(&theme.to_toml());
            }
        }
        match std::fs::write(&path, text) {
            Ok(()) => {
                let msg = self
                    .i18n
                    .t_args("opt-saved", &[("path", path.display().to_string().into())]);
                self.set_status(msg);
            }
            Err(e) => self.set_error(format!("{}: {e}", path.display())),
        }
    }

    /// `Alt+N` → `F4`/`Ins`: the session editor (SPEC 10 §3.5).
    pub fn open_site_form(&mut self, index: Option<usize>) {
        let site = index.and_then(|i| self.cfg.sites.get(i).map(|cfg| (i, cfg)));
        if index.is_some() && site.is_none() {
            // A row of the list that is a live connection rather than a stored site: there is
            // nothing to edit, so offer a new one filled in from it.
            self.mode = Mode::Form(Box::new(Form::site(None, &self.i18n)));
            return;
        }
        self.mode = Mode::Form(Box::new(Form::site(site, &self.i18n)));
    }

    /// OK in the session editor: validate, store, and keep the site list in step.
    fn apply_site_form(&mut self, form: &Form) {
        let host = form.text_value("site.host");
        if host.is_empty() {
            self.set_error(self.i18n.t("site-needs-host"));
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let port = form.text_value("site.port");
        let port = match port.as_str() {
            "" => None,
            other => match other.parse::<u16>() {
                Ok(0) | Err(_) => {
                    self.set_error(
                        self.i18n
                            .t_args("site-bad-port", &[("port", other.to_string().into())]),
                    );
                    self.mode = Mode::Form(Box::new(form.clone()));
                    return;
                }
                Ok(n) => Some(n),
            },
        };
        let keepalive = form.text_value("site.keepalive_secs");
        let keepalive = match keepalive.as_str() {
            "" => None,
            other => match other.parse::<u64>() {
                Ok(n) => Some(n),
                Err(_) => {
                    self.set_error(
                        self.i18n
                            .t_args("site-bad-port", &[("port", other.to_string().into())]),
                    );
                    self.mode = Mode::Form(Box::new(form.clone()));
                    return;
                }
            },
        };
        let auth = match form.text_value("site.auth").as_str() {
            "key" => {
                let path = form.text_value("site.key");
                if path.is_empty() {
                    self.set_error(self.i18n.t("site-needs-key"));
                    self.mode = Mode::Form(Box::new(form.clone()));
                    return;
                }
                format!("key:{path}")
            }
            "ask" => "ask".to_string(),
            _ => "agent".to_string(),
        };
        let id = match form.text_value("site.id") {
            empty if empty.is_empty() => host.clone(),
            given => given,
        };
        let empty_to_none = |s: String| if s.is_empty() { None } else { Some(s) };
        let cfg = scmd_config::SiteCfg {
            id,
            scheme: "sftp".to_string(),
            host,
            port,
            user: empty_to_none(form.text_value("site.user")),
            auth: Some(auth),
            start_dir: empty_to_none(form.text_value("site.start_dir")),
            group: empty_to_none(form.text_value("site.group")),
            keepalive_secs: keepalive,
            compress: Some(form.text_value("site.compress") == "true"),
        };

        // What the editor produced has to be a site the netbox can actually use; if it is not,
        // say so and keep the dialog open rather than storing something unusable.
        if let Err(e) = Site::from_config(&cfg) {
            self.set_error(e.to_string());
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let name = cfg.id.clone();
        match form.site {
            Some(index) if index < self.cfg.sites.len() => self.cfg.sites[index] = cfg,
            _ => self.cfg.sites.push(cfg),
        }
        let (sites, problems) = Site::list_from_config(&self.cfg);
        self.sites = sites;
        if let Some(first) = problems.first() {
            self.set_error(first.clone());
            return;
        }
        self.mode = Mode::Sites {
            cursor: self.sites.iter().position(|s| s.id == name).unwrap_or(0),
        };
        let msg = self.i18n.t_args("site-stored", &[("name", name.into())]);
        self.set_status(msg);
    }

    /// `F12` in the hex editor: what to do with the bytes that are marked (SPEC 05 §2.6).
    ///
    /// Every operation says what it will do to the length before it does it, because the
    /// failure of a hex editor is a file that is quietly one byte wrong.
    fn open_block_menu(&mut self) {
        let Some(editor) = self.mode.hex_mut() else {
            return;
        };
        let block = editor.block;
        let len = editor.len();
        let cursor = editor.cursor;
        let t = |app: &App, id: &str| app.i18n.t(id);
        let span = block.map(|(a, b)| b - a).unwrap_or(0);

        let mut rows = vec![
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-read", &[("at", format!("{cursor:08X}").into())]),
                value: "read".to_string(),
                current: false,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-insert", &[("at", format!("{cursor:08X}").into())]),
                value: "insert".to_string(),
                current: false,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-copy", &[("at", format!("{cursor:08X}").into())]),
                value: "copy".to_string(),
                current: false,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-move", &[("at", format!("{cursor:08X}").into())]),
                value: "move".to_string(),
                current: false,
            },
        ];
        if span > 0 {
            // The operations that need a block, with the size they would act on.
            let size = scmd_core::format_size(span);
            rows.insert(
                0,
                PickRow {
                    label: self
                        .i18n
                        .t_args("hex-op-write", &[("size", size.clone().into())]),
                    value: "write".to_string(),
                    current: true,
                },
            );
            rows.push(PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-fill", &[("size", size.clone().into())]),
                value: "fill".to_string(),
                current: false,
            });
            rows.push(PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-xor", &[("size", size.clone().into())]),
                value: "xor".to_string(),
                current: false,
            });
            rows.push(PickRow {
                label: self.i18n.t_args(
                    "hex-op-delete",
                    &[
                        ("size", size.into()),
                        ("was", scmd_core::format_size(len).into()),
                        ("becomes", scmd_core::format_size(len - span).into()),
                    ],
                ),
                value: "delete".to_string(),
                current: false,
            });
        }
        let title = t(self, "hex-block-title");
        self.park_hex();
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::HexBlock, title, rows, 0)));
    }

    /// `F10` from the panels. A viewer or an editor left in the background can hold changes
    /// nobody has written, and the panels know nothing about them — so the one place that
    /// can ask is here, by name, before the program ends.
    fn ask_before_quitting(&mut self) {
        let dirty: Vec<String> = self
            .background
            .iter()
            .filter(|s| s.screen_modified())
            .filter_map(|s| s.screen_title())
            .collect();
        if dirty.is_empty() {
            self.quit = true;
            return;
        }
        let title = self.i18n.t("screens-quit-title");
        let mut lines = vec![self.i18n.t("screens-quit-body"), String::new()];
        lines.extend(dirty);
        lines.push(String::new());
        lines.push(self.i18n.t("confirm-yes-no"));
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action: ConfirmAction::QuitWithScreens,
        });
    }

    // ── plugin commands (SPEC 08 §1) ──────────────────────────────────────────────────────

    /// `F11`: what the loaded plugins provide, in the scope we are in.
    fn open_plugin_commands(&mut self) {
        let scope = self.scope();
        let rows: Vec<PickRow> = self
            .plugin_commands
            .iter()
            .enumerate()
            .filter(|(_, c)| c.scope == scope || c.scope == scmd_cmd::Scope::Global)
            .map(|(i, c)| PickRow {
                label: format!("{:<40} {}", c.label, c.plugin),
                value: i.to_string(),
                current: false,
            })
            .collect();
        if rows.is_empty() {
            // Said rather than a list with nothing in it, and said in a way that answers the
            // next question: there is nowhere to get plugins from yet.
            let m = self.i18n.t_args(
                "plugin-none",
                &[("dir", self.paths.plugins_dir.display().to_string().into())],
            );
            self.set_status(m);
            return;
        }
        let title = self.i18n.t("plugin-list-title");
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Plugins, title, rows, 0)));
    }

    /// Ask for a plugin command to be run, with the context it asked for and nothing else.
    fn run_plugin_command(&mut self, which: usize) {
        let Some(cmd) = self.plugin_commands.get(which).cloned() else {
            return;
        };
        let mut ctx = scmd_plug::host::Context::new(&cmd.id);
        let view = |path: &VPath| scmd_plug::host::PanelView {
            cwd: path.to_string(),
            // A path inside an archive or on a server has no directory to run a program in,
            // and the field is absent rather than holding something that cannot be opened.
            dir: (!path.is_remote() && !path.in_container())
                .then(|| path.as_path().display().to_string()),
        };
        let here = self.panel().cwd.clone();
        if cmd.wants("panel") {
            ctx.panel = Some(view(&here));
        }
        if cmd.wants("other") {
            ctx.other = Some(view(&self.panels[1 - self.active].cwd.clone()));
        }
        if cmd.wants("cursor") {
            ctx.cursor = self.panel().current().map(|e| e.name_str().into_owned());
        }
        if cmd.wants("selection") {
            let panel = self.panel();
            ctx.selection = (0..panel.listing.len())
                .filter_map(|row| panel.listing.row(row))
                .filter(|e| panel.selection.contains(e.id))
                .map(|e| e.name_str().into_owned())
                .collect();
        }
        let cwd = (!here.is_remote() && !here.in_container()).then(|| here.as_path().to_path_buf());
        let msg = self
            .i18n
            .t_args("plugin-running", &[("label", cmd.label.clone().into())]);
        self.set_status(msg);
        self.requests.push(Request::Plugin {
            id: cmd.id.clone(),
            label: cmd.label.clone(),
            program: cmd.program.clone(),
            cwd,
            ctx: Box::new(ctx),
            timeout: cmd.timeout,
        });
    }

    /// What a plugin asked for, carried out.
    ///
    /// Every action goes through the same door a keystroke would: `run` resolves against the
    /// command vocabulary and is refused if it names nothing, so a plugin cannot reach past
    /// what a key could have done.
    fn apply_plugin_reply(&mut self, label: &str, reply: scmd_plug::host::Reply) {
        use scmd_plug::host::Action;
        let mut said = false;
        for action in reply.actions {
            match action {
                Action::Status(text) => {
                    self.set_status(text);
                    said = true;
                }
                Action::Error(text) => {
                    self.set_error(text);
                    said = true;
                }
                Action::Run(id) => match Command::parse(&id) {
                    Some(cmd) => self.run(cmd),
                    None => {
                        let m = self
                            .i18n
                            .t_args("plugin-unknown-command", &[("id", id.into())]);
                        self.set_error(m);
                        said = true;
                    }
                },
                Action::Chdir { path, panel } => {
                    let side = match panel.as_deref() {
                        Some("other") => 1 - self.active,
                        Some("left") => 0,
                        Some("right") => 1,
                        _ => self.active,
                    };
                    let target = self.resolve_path(&path);
                    self.panels[side].go(target, true);
                    self.read_panel(side);
                }
                Action::Message { title, lines } => {
                    self.mode = Mode::Message { title, lines };
                    said = true;
                }
                Action::Cursor(name) => {
                    if let Some(row) = self.panel().listing.row_of_name(&name) {
                        self.panel_mut().cursor_to(row);
                    }
                }
                Action::Select(names) => {
                    let ids: Vec<_> = {
                        let panel = self.panel();
                        (0..panel.listing.len())
                            .filter_map(|row| panel.listing.row(row))
                            .filter(|e| names.iter().any(|n| n.as_str() == e.name_str()))
                            .map(|e| e.id)
                            .collect()
                    };
                    let panel = self.panel_mut();
                    panel.selection.clear();
                    for id in ids {
                        panel.selection.set(id, true);
                    }
                }
            }
        }
        // Whatever the plugin wrote for a human, rather than for us, is shown — but only if
        // it did not already say something of its own.
        let mut leftover = reply.text;
        leftover.extend(reply.complaints);
        if !leftover.is_empty() && !said {
            self.mode = Mode::Message {
                title: label.to_string(),
                lines: leftover,
            };
        }
    }

    // ── macros (SPEC 08 §5) ───────────────────────────────────────────────────────────────

    /// Which scope the keymap — and a macro — is looked up in right now.
    pub fn scope(&self) -> scmd_cmd::Scope {
        use scmd_cmd::Scope;
        match &self.mode {
            Mode::Normal => Scope::Panel,
            Mode::Viewer(_) => Scope::Viewer,
            Mode::Editor(_) | Mode::Hex(_) => Scope::Editor,
            // Everything else is a dialog of some kind: a question, a list, a form.
            _ => Scope::Dialog,
        }
    }

    /// `Ctrl+.`: start recording, or stop and ask which key the macro goes on.
    fn toggle_recording(&mut self) {
        match self.recording.take() {
            None => {
                self.recording = Some(Vec::new());
                let m = self.i18n.t("macro-recording");
                self.set_status(m);
            }
            Some(steps) if steps.is_empty() => {
                let m = self.i18n.t("macro-empty");
                self.set_status(m);
            }
            Some(steps) => {
                // The key is pressed rather than typed, which is how one knows it is free.
                self.mode = Mode::MacroKey { steps };
            }
        }
    }

    /// The key a just-recorded macro will answer to. `Esc` throws the recording away.
    fn macro_key(&mut self, key: Key, steps: Vec<scmd_cmd::Step>) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Normal;
            let m = self.i18n.t("macro-dropped");
            self.set_status(m);
            return;
        }
        // The scope it was recorded in is the scope it plays in: a macro of editor keys
        // would do something else entirely in the panel.
        let scope = scmd_cmd::Scope::Panel;
        let title = self
            .i18n
            .t_args("macro-describe", &[("key", key.to_string().into())]);
        self.mode = Mode::Prompt(Prompt::new(
            PromptKind::MacroDescription { key, scope, steps },
            title,
            String::new(),
        ));
    }

    /// Store a macro and write the file. A recording that cannot be saved is still in memory
    /// for this session, and says so rather than pretending it was written.
    fn store_macro(&mut self, new: scmd_cmd::Macro) {
        let key = new.key.to_string();
        let steps = new.steps.len();
        self.macros.set(new);
        match self.save_macros() {
            Ok(()) => {
                let msg = self.i18n.t_args(
                    "macro-saved",
                    &[("key", key.into()), ("steps", (steps as i64).into())],
                );
                self.set_status(msg);
            }
            Err(e) => self.set_error(e),
        }
    }

    fn save_macros(&self) -> Result<(), String> {
        let path = &self.paths.macros_file;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        std::fs::write(path, self.macros.to_toml()).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// `Ctrl+Shift+.`: the macros, to play or delete one.
    fn open_macros(&mut self) {
        if self.macros.is_empty() {
            let m = self.i18n.t("macro-none");
            self.set_status(m);
            return;
        }
        let rows: Vec<PickRow> = self
            .macros
            .all
            .iter()
            .enumerate()
            .map(|(i, m)| PickRow {
                label: m.summary(),
                value: i.to_string(),
                current: false,
            })
            .collect();
        let title = self.i18n.t("macro-list-title");
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Macros, title, rows, 0)));
    }

    /// Play one, asking first when it does something that cannot be undone.
    fn play_macro(&mut self, which: &scmd_cmd::Macro) {
        if which.is_destructive() && !which.trusted {
            let title = self.i18n.t("macro-sure-title");
            let body = self
                .i18n
                .t_args("macro-sure-body", &[("key", which.key.to_string().into())]);
            self.mode = Mode::Confirm(Confirm {
                title,
                lines: vec![
                    body,
                    String::new(),
                    which
                        .steps
                        .iter()
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                        .join("  "),
                    String::new(),
                    self.i18n.t("confirm-yes-no"),
                ],
                action: ConfirmAction::PlayMacro(Box::new(which.clone())),
            });
            return;
        }
        self.run_macro(which);
    }

    /// Hand the steps back to the application, in order.
    ///
    /// A macro is a list of commands, so playing one is running them — there is no second
    /// interpreter and no way for a macro to do something a key could not.
    fn run_macro(&mut self, which: &scmd_cmd::Macro) {
        // Thirty-two deep is far more than anything sane and still finite, which is what
        // matters: a macro that plays itself must stop rather than fill the stack.
        if self.playing > 32 {
            let m = self.i18n.t("macro-too-deep");
            self.set_error(m);
            return;
        }
        self.playing += 1;
        match which.play {
            scmd_cmd::Play::Once => self.run_steps(&which.steps),
            scmd_cmd::Play::EachSelected => {
                // The rows selected now, by path: the listing can change under a step, and a
                // row number would then mean a different file.
                let chosen: Vec<VPath> = {
                    let panel = self.panel();
                    (0..panel.listing.len())
                        .filter_map(|row| panel.listing.row(row))
                        .filter(|e| panel.selection.contains(e.id))
                        .map(|e| panel.path_of(e))
                        .collect()
                };
                if chosen.is_empty() {
                    // Nothing selected is the row under the cursor, which is the rule
                    // everywhere else in the program.
                    self.run_steps(&which.steps);
                } else {
                    for path in chosen {
                        let name = path.display_name();
                        let row = self.panel().listing.row_of_name(&name);
                        if let Some(row) = row {
                            self.panel_mut().cursor_to(row);
                            self.run_steps(&which.steps);
                        }
                    }
                }
            }
        }
        self.playing -= 1;
    }

    fn run_steps(&mut self, steps: &[scmd_cmd::Step]) {
        for step in steps {
            match step {
                scmd_cmd::Step::Run(cmd) => self.run(*cmd),
                scmd_cmd::Step::Press(key) => self.dispatch_key(*key),
            }
        }
    }

    // ── the screens (SPEC 02 §7) ──────────────────────────────────────────────────────────

    /// `F12`: the viewers and editors left open, with the panels at the top.
    ///
    /// The list is always offered, even with nothing in the background, because the answer
    /// "only the panels" is an answer — and because a key that sometimes does nothing is
    /// worse than a list with one row in it.
    fn open_screens(&mut self) {
        let mut rows = vec![PickRow {
            label: self.i18n.t("screens-panels"),
            value: "panels".to_string(),
            current: self.mode.screen_title().is_none(),
        }];
        // The one in front first, then the ones behind it, newest first: the order one
        // stepped away from them, which is the order one comes back through.
        if let Some(title) = self.mode.screen_title() {
            rows.push(PickRow {
                label: self.screen_label(&title, self.mode.screen_modified()),
                value: "front".to_string(),
                current: true,
            });
        }
        for (i, screen) in self.background.iter().enumerate().rev() {
            let Some(title) = screen.screen_title() else {
                continue;
            };
            rows.push(PickRow {
                label: self.screen_label(&title, screen.screen_modified()),
                value: i.to_string(),
                current: false,
            });
        }
        let at = rows.iter().position(|r| r.current).unwrap_or(0);
        let title = self.i18n.t("screens-title");
        self.park_screen_dialogs();
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Screens, title, rows, at)));
    }

    /// A row of the screens list: the title, and a star if it holds unsaved changes.
    fn screen_label(&self, title: &str, modified: bool) -> String {
        if modified {
            format!("* {title}")
        } else {
            format!("  {title}")
        }
    }

    /// Put the screen that is in front into the background and bring `which` forward.
    ///
    /// Nothing is dropped and nothing is closed: a screen only leaves this world through its
    /// own `F10`, which is what asks about unsaved changes.
    fn show_screen(&mut self, which: Option<usize>) {
        let front = std::mem::replace(&mut self.mode, Mode::Normal);
        let coming = match which {
            Some(i) if i < self.background.len() => Some(self.background.remove(i)),
            _ => None,
        };
        if front.screen_title().is_some() {
            self.background.push(front);
        }
        if let Some(screen) = coming {
            self.mode = screen;
        }
        // A screen that comes forward may be looking at a file the panels have changed.
        if self.mode.is_normal() {
            self.read_panel(0);
            self.read_panel(1);
        }
    }

    /// `Ctrl+Tab` / `Ctrl+Shift+Tab`: round the screens without the list.
    ///
    /// The panels are screen zero of the ring and the one in front is its last member, so
    /// `Ctrl+Tab` from a screen steps out to the panels and `Ctrl+Tab` from the panels steps
    /// in to the next screen behind them: the panels are never more than one key away, and
    /// every screen is reached by pressing it again. `Ctrl+Shift+Tab` walks the other way,
    /// which from the panels is the screen one most recently stepped away from.
    ///
    /// Going straight from one screen to another, without the panels in between, is `F12` —
    /// and that is what the list is for. A ring that walks screen to screen would have to own
    /// the mode rather than shadow it; `docs/BACKLOG.md` carries that.
    fn cycle_screen(&mut self, backwards: bool) {
        self.park_screen_dialogs();
        let behind = self.background.len();
        let in_front = self.mode.screen_title().is_some();
        // The ring: the panels, then the background oldest first, then the one in front.
        let ring = behind + 1 + usize::from(in_front);
        if ring < 2 {
            return; // only the panels: nowhere to go
        }
        let at = if in_front { ring - 1 } else { 0 };
        let next = if backwards {
            (at + ring - 1) % ring
        } else {
            (at + 1) % ring
        };
        // Index 0 is the panels; 1..=behind are the background screens, oldest first.
        if next == 0 {
            self.show_screen(None);
        } else {
            self.show_screen(Some(next - 1));
        }
    }

    /// A dialog is not a screen, so one open over a viewer is put back on its screen before
    /// the screen is moved. Without this, stepping away from a prompt would leave the viewer
    /// behind it in the background and the prompt in front of the panels.
    fn park_screen_dialogs(&mut self) {
        if self.mode.screen_title().is_none() {
            self.restore_viewer();
            self.restore_editor();
            self.restore_hex();
        }
    }

    /// `F9`: ask for one instruction, with the address it will sit at in the title.
    ///
    /// The dialog is deliberately the plainest one in the program: the hard part is not
    /// choosing the instruction but seeing what it costs, and that is answered after it is
    /// assembled, before a byte is written.
    fn open_assembler(&mut self) {
        let Some(cursor) = self.mode.hex().map(|e| e.cursor) else {
            return;
        };
        let arch = self.hex_arch();
        if !arch.disassemblable() {
            let msg = self
                .i18n
                .t_args("asm-no-arch", &[("arch", arch.as_str().into())]);
            self.set_error(msg);
            return;
        }
        let title = self.i18n.t_args(
            "asm-title",
            &[
                ("at", format!("{cursor:08X}").into()),
                ("arch", arch.as_str().into()),
            ],
        );
        let last = self.last_asm.clone().unwrap_or_default();
        self.park_hex();
        self.mode = Mode::Prompt(Prompt::new(PromptKind::Assemble, title, last));
    }

    /// Which processor the bytes under the cursor are for.
    ///
    /// The file's own header answers when it has one — a 32-bit PE is assembled as 32-bit
    /// code even on a 64-bit machine, which is the whole reason to ask. Failing that it is
    /// the machine's own width, and the dialog's title names whatever was assumed.
    fn hex_arch(&mut self) -> scmd_view::Arch {
        let head = self
            .mode
            .hex_mut()
            .map(|e| e.read(0, 4096))
            .unwrap_or_default();
        // The header alone, not the sections: this is one field, read from the front.
        if let Ok(binary) = scmd_view::analyze(&head, false) {
            if binary.arch.disassemblable() {
                return binary.arch;
            }
        }
        let host = scmd_view::disasm::host_arch();
        if host.disassemblable() {
            host
        } else {
            scmd_view::Arch::X86_64
        }
    }

    /// Assemble what was typed and write it over the bytes at the cursor.
    fn assemble_at_cursor(&mut self, line: &str) {
        if line.trim().is_empty() {
            return;
        }
        self.last_asm = Some(line.to_string());
        let arch = self.hex_arch();
        let Some(editor) = self.mode.hex_mut() else {
            return;
        };
        let at = editor.cursor;
        // The address, when the file is a binary we understand, is the one the instruction
        // will have when it runs — which is what a relative jump is written against.
        let address = at;
        match scmd_view::assemble(line, arch, address) {
            Ok(done) => {
                let old = editor.read(at, done.bytes.len());
                // Overwrite: the length of a binary is not ours to change behind the user's
                // back, and insert mode is the way to say otherwise.
                let remove = if editor.insert {
                    0
                } else {
                    (done.bytes.len() as u64).min(editor.len().saturating_sub(at))
                };
                editor.splice(at, remove, &done.bytes);
                let shown: Vec<String> = done.bytes.iter().map(|b| format!("{b:02X}")).collect();
                let grew = done.bytes.len() as i64 - old.len() as i64;
                let msg = self.i18n.t_args(
                    if grew == 0 {
                        "asm-written"
                    } else {
                        "asm-written-longer"
                    },
                    &[
                        ("text", done.text.into()),
                        ("bytes", shown.join(" ").into()),
                        ("grew", grew.into()),
                    ],
                );
                self.set_status(msg);
            }
            Err(e) => self.set_error(e.to_string()),
        }
    }

    /// Carry out one block operation with the value the user typed.
    fn hex_operation(&mut self, kind: PromptKind, text: &str) {
        // Whatever needs the rest of the application — a path, a message — is worked out
        // before the editor is borrowed, because the two cannot be held at once.
        let path = matches!(kind, PromptKind::HexWrite | PromptKind::HexRead)
            .then(|| self.resolve_path(text));
        let Some(editor) = self.mode.hex_mut() else {
            return;
        };
        let block = editor.block;
        let cursor = editor.cursor;
        match kind {
            PromptKind::HexFill => {
                let Some(bytes) = App::hex_bytes(text) else {
                    let m = self.i18n.t("hex-not-hex");
                    self.set_error(m);
                    return;
                };
                let Some((a, b)) = block else { return };
                let span = (b - a) as usize;
                // The value repeats over the block: one byte fills it, four bytes pattern it.
                let filled: Vec<u8> = bytes.iter().copied().cycle().take(span).collect();
                editor.splice(a, b - a, &filled);
                let msg = self.i18n.t_args(
                    "hex-filled",
                    &[("size", scmd_core::format_size(b - a).into())],
                );
                self.set_status(msg);
            }
            PromptKind::HexXor => {
                let Some(key) = App::hex_bytes(text) else {
                    let m = self.i18n.t("hex-not-hex");
                    self.set_error(m);
                    return;
                };
                let Some((a, b)) = block else { return };
                let mut bytes = editor.read(a, (b - a) as usize);
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte ^= key[i % key.len()];
                }
                editor.splice(a, b - a, &bytes);
                let msg = self.i18n.t_args(
                    "hex-xored",
                    &[("size", scmd_core::format_size(b - a).into())],
                );
                self.set_status(msg);
            }
            PromptKind::HexWrite => {
                let Some((a, b)) = block else { return };
                let bytes = editor.read(a, (b - a) as usize);
                let Some(path) = path else { return };
                match std::fs::write(path.as_path(), &bytes) {
                    Ok(()) => {
                        let msg = self.i18n.t_args(
                            "hex-written",
                            &[
                                ("size", scmd_core::format_size(bytes.len() as u64).into()),
                                ("name", path.display_name().into()),
                            ],
                        );
                        self.set_status(msg);
                        self.read_panel(0);
                        self.read_panel(1);
                    }
                    Err(e) => self.set_error(format!("{path}: {e}")),
                }
            }
            PromptKind::HexRead => {
                let Some(path) = path else { return };
                match std::fs::read(path.as_path()) {
                    Ok(bytes) => {
                        // Overwrite where the cursor is, or insert when insert mode is on:
                        // the mode the user can see decides, not a second question.
                        let remove = if editor.insert {
                            0
                        } else {
                            (bytes.len() as u64).min(editor.len() - cursor.min(editor.len()))
                        };
                        editor.splice(cursor, remove, &bytes);
                        let msg = self.i18n.t_args(
                            "hex-read-done",
                            &[
                                ("size", scmd_core::format_size(bytes.len() as u64).into()),
                                ("name", path.display_name().into()),
                            ],
                        );
                        self.set_status(msg);
                    }
                    Err(e) => self.set_error(format!("{path}: {e}")),
                }
            }
            PromptKind::HexInsert => {
                let count: u64 = match text.trim().parse() {
                    Ok(n) if n > 0 => n,
                    _ => {
                        let m = self.i18n.t("hex-not-a-count");
                        self.set_error(m);
                        return;
                    }
                };
                // A quarter of a gigabyte of zeros is a mistake, not a plan.
                let count = count.min(256 << 20);
                editor.insert_bytes(cursor, &vec![0u8; count as usize]);
                let msg = self.i18n.t_args(
                    "hex-inserted",
                    &[("size", scmd_core::format_size(count).into())],
                );
                self.set_status(msg);
            }
            PromptKind::HexCopy | PromptKind::HexMove => {
                // `offset length`, both in hex, because that is what the rest of the pane
                // speaks: the source is somewhere else in this same file.
                let Some((from, span)) = App::hex_range(text) else {
                    let m = self.i18n.t("hex-not-a-range");
                    self.set_error(m);
                    return;
                };
                let len = editor.len();
                if span == 0 || from >= len || from + span > len {
                    let m = self.i18n.t_args(
                        "hex-range-outside",
                        &[("size", scmd_core::format_size(len).into())],
                    );
                    self.set_error(m);
                    return;
                }
                let moving = kind == PromptKind::HexMove;
                if moving && cursor > from && cursor < from + span {
                    // Moving a block onto itself would eat its own source.
                    let m = self.i18n.t("hex-move-into-itself");
                    self.set_error(m);
                    return;
                }
                let bytes = editor.read(from, span as usize);
                // Insert first, then remove the source, so the offsets are the ones the user
                // typed: taking the source out first would shift the destination.
                editor.insert_bytes(cursor, &bytes);
                if moving {
                    let source = if from >= cursor { from + span } else { from };
                    editor.delete(source, span);
                    if cursor > from {
                        editor.cursor = cursor - span;
                    }
                }
                let msg = self.i18n.t_args(
                    if moving { "hex-moved" } else { "hex-copied" },
                    &[
                        ("size", scmd_core::format_size(span).into()),
                        ("from", format!("{from:08X}").into()),
                    ],
                );
                self.set_status(msg);
            }
            _ => {}
        }
    }

    /// `offset length`, both hex, separated by a space or a comma. Anything else is nothing.
    fn hex_range(text: &str) -> Option<(u64, u64)> {
        let mut parts = text
            .split([' ', ',', '\t', '+'])
            .filter(|p| !p.trim().is_empty());
        let from = u64::from_str_radix(parts.next()?.trim().trim_start_matches("0x"), 16).ok()?;
        let span = u64::from_str_radix(parts.next()?.trim().trim_start_matches("0x"), 16).ok()?;
        if parts.next().is_some() {
            return None;
        }
        Some((from, span))
    }

    fn park_hex(&mut self) {
        if let Mode::Hex(editor) = std::mem::replace(&mut self.mode, Mode::Normal) {
            self.hex_parked = Some(editor);
        }
    }

    fn restore_hex(&mut self) {
        if let Some(editor) = self.hex_parked.take() {
            self.mode = Mode::Hex(editor);
        }
    }

    /// Ask for the one value an operation needs, with the hex editor waiting behind it.
    fn hex_prompt(&mut self, kind: PromptKind, title_key: &str) {
        self.park_hex();
        let title = self.i18n.t(title_key);
        self.mode = Mode::Prompt(Prompt::new(kind, title, String::new()));
    }

    /// Hex digits, with spaces anywhere: `de ad be ef` and `deadbeef` are the same key.
    fn hex_bytes(text: &str) -> Option<Vec<u8>> {
        let digits: String = text.chars().filter(|c| !c.is_whitespace()).collect();
        if digits.is_empty() || !digits.len().is_multiple_of(2) {
            return None;
        }
        (0..digits.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&digits[i..i + 2], 16).ok())
            .collect()
    }

    pub fn open_pick(&mut self, kind: PickKind, side: usize) {
        let pick = match kind {
            // Built by `open_block_menu`, which knows what is marked; nothing opens it here.
            PickKind::HexBlock => return,
            // The same: `open_screens` knows what is open, and this does not.
            PickKind::Screens => return,
            // And `open_macros`, which knows what has been recorded.
            PickKind::Macros => return,
            // And `open_plugin_commands`, which knows the scope.
            PickKind::Plugins => return,
            PickKind::Languages => Pick::new(
                kind,
                self.i18n.t("menu-languages"),
                scmd_i18n::LANGUAGES
                    .iter()
                    .map(|(tag, name)| PickRow {
                        label: format!("{name}  ({tag})"),
                        value: (*tag).to_string(),
                        current: *tag == self.i18n.tag(),
                    })
                    .collect(),
                side,
            ),
            PickKind::Themes => Pick::new(
                kind,
                self.i18n.t("menu-themes"),
                self.themes
                    .iter()
                    .map(|theme| PickRow {
                        label: theme.name.clone(),
                        value: theme.id.clone(),
                        current: theme.id == self.theme.id,
                    })
                    .collect(),
                side,
            ),
            PickKind::Drives => {
                let mut rows: Vec<PickRow> = local_roots()
                    .into_iter()
                    .map(|path| PickRow {
                        label: path.clone(),
                        value: path,
                        current: false,
                    })
                    .collect();
                for authority in &self.connected {
                    rows.push(PickRow {
                        label: format!("sftp://{authority}"),
                        value: format!("sftp://{authority}/"),
                        current: self.panels[side].cwd.authority() == Some(authority.as_str()),
                    });
                }
                Pick::new(kind, self.i18n.t("menu-change-drive"), rows, side)
            }
            PickKind::FoldersHistory => {
                let panel = &self.panels[side];
                let current = panel.cwd.to_string();
                // Newest first, and a folder visited three times is one row, not three —
                // `dedup` would only catch the repeats that happen to be adjacent.
                let mut seen = std::collections::HashSet::new();
                let rows: Vec<PickRow> = panel
                    .history
                    .iter()
                    .rev()
                    .map(|p| p.to_string())
                    .filter(|path| seen.insert(path.clone()))
                    .map(|path| PickRow {
                        current: path == current,
                        label: path.clone(),
                        value: path,
                    })
                    .collect();
                Pick::new(kind, self.i18n.t("menu-folders-history"), rows, side)
            }
            PickKind::CommandHistory => {
                // Newest first, which is the order someone reaching for a past command wants.
                let rows: Vec<PickRow> = self
                    .cmd
                    .history()
                    .iter()
                    .rev()
                    .map(|line| PickRow {
                        label: line.clone(),
                        value: line.clone(),
                        current: false,
                    })
                    .collect();
                Pick::new(kind, self.i18n.t("menu-history"), rows, side)
            }
            PickKind::SortModes => {
                let panel = &self.panels[side];
                let keys = [
                    (SortKey::Name, "sort-name"),
                    (SortKey::Ext, "sort-ext"),
                    (SortKey::Size, "sort-size"),
                    (SortKey::MTime, "sort-mtime"),
                    (SortKey::BTime, "sort-btime"),
                    (SortKey::ATime, "sort-atime"),
                    (SortKey::Unsorted, "sort-none"),
                ];
                Pick::new(
                    kind,
                    self.i18n.t("menu-sort-modes"),
                    keys.iter()
                        .map(|(key, id)| PickRow {
                            label: self.i18n.t(id),
                            value: format!("{key:?}").to_lowercase(),
                            current: panel.sort.key == *key,
                        })
                        .collect(),
                    side,
                )
            }
        };
        self.mode = Mode::Pick(Box::new(pick));
    }

    fn pick_key(&mut self, key: Key) {
        let Mode::Pick(pick) = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                self.mode = Mode::Normal;
                // The block menu was opened over the hex editor; closing it goes back there.
                self.restore_hex();
            }
            KeyCode::Up => pick.move_cursor(-1),
            KeyCode::Down => pick.move_cursor(1),
            KeyCode::PageUp => pick.move_cursor(-10),
            KeyCode::PageDown => pick.move_cursor(10),
            KeyCode::Home => pick.cursor = 0,
            KeyCode::End => pick.cursor = pick.rows.len().saturating_sub(1),
            KeyCode::Enter => self.apply_pick(),
            _ => {}
        }
    }

    fn apply_pick(&mut self) {
        let Mode::Pick(pick) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        let Some(row) = pick.chosen().cloned() else {
            return;
        };
        match pick.kind {
            PickKind::Plugins => {
                if let Ok(i) = row.value.parse::<usize>() {
                    self.run_plugin_command(i);
                }
            }
            PickKind::Macros => {
                if let Ok(i) = row.value.parse::<usize>() {
                    if let Some(which) = self.macros.all.get(i).cloned() {
                        self.play_macro(&which);
                    }
                }
            }
            PickKind::Screens => match row.value.as_str() {
                "panels" => self.show_screen(None),
                "front" => {} // already there
                other => {
                    if let Ok(i) = other.parse::<usize>() {
                        self.show_screen(Some(i));
                    }
                }
            },
            PickKind::HexBlock => {
                self.restore_hex();
                match row.value.as_str() {
                    "write" => self.hex_prompt(PromptKind::HexWrite, "hex-write-title"),
                    "read" => self.hex_prompt(PromptKind::HexRead, "hex-read-title"),
                    "fill" => self.hex_prompt(PromptKind::HexFill, "hex-fill-title"),
                    "xor" => self.hex_prompt(PromptKind::HexXor, "hex-xor-title"),
                    "insert" => self.hex_prompt(PromptKind::HexInsert, "hex-insert-title"),
                    "copy" => self.hex_prompt(PromptKind::HexCopy, "hex-copy-title"),
                    "move" => self.hex_prompt(PromptKind::HexMove, "hex-move-title"),
                    "delete" => {
                        let done = self.mode.hex_mut().and_then(|editor| {
                            let (a, b) = editor.block?;
                            editor.delete(a, b - a);
                            editor.cursor = a;
                            editor.clear_block();
                            Some(b - a)
                        });
                        if let Some(bytes) = done {
                            let msg = self.i18n.t_args(
                                "hex-deleted",
                                &[("size", scmd_core::format_size(bytes).into())],
                            );
                            self.set_status(msg);
                        }
                    }
                    _ => {}
                }
            }
            PickKind::Languages => {
                self.i18n = I18n::new(&row.value);
                self.cfg.ui.language = row.value.clone();
                let msg = self
                    .i18n
                    .t_args("opt-language-set", &[("name", row.label.clone().into())]);
                self.set_status(msg);
            }
            PickKind::Themes => {
                if let Some(theme) = self.themes.get(&row.value).cloned() {
                    self.cfg.ui.theme = theme.id.clone();
                    let name = theme.name.clone();
                    self.theme = theme;
                    let msg = self.i18n.t_args("msg-theme", &[("name", name.into())]);
                    self.set_status(msg);
                }
            }
            PickKind::Drives | PickKind::FoldersHistory => {
                self.active = pick.side;
                let target = VPath::parse(&row.value);
                if target.is_remote() {
                    match scmd_net::Site::parse_url(&row.value) {
                        Ok(site) => match self.connected_site_path(&site) {
                            Some(path) => self.goto(path),
                            None => self.connect_to(site, None, false),
                        },
                        Err(e) => self.set_error(e.to_string()),
                    }
                } else {
                    self.goto(target.normalised());
                }
            }
            PickKind::CommandHistory => {
                // Chosen from the list, the line goes back on the command line rather than
                // running at once: a command from history is usually a command to edit.
                self.cmd.set(row.value.clone());
            }
            PickKind::SortModes => {
                let key = match row.value.as_str() {
                    "ext" => SortKey::Ext,
                    "size" => SortKey::Size,
                    "mtime" => SortKey::MTime,
                    "btime" => SortKey::BTime,
                    "atime" => SortKey::ATime,
                    "unsorted" | "none" => SortKey::Unsorted,
                    _ => SortKey::Name,
                };
                let panel = &mut self.panels[pick.side];
                // Choosing the order that is already in force turns it round, the way FAR
                // does it — unless the setting says reverse sorting is not wanted.
                if panel.sort.key == key && self.cfg.panel.allow_reverse_sort {
                    panel.sort.reverse = !panel.sort.reverse;
                } else {
                    panel.sort.key = key;
                }
                let reversed = panel.sort.reverse;
                panel.rebuild();
                let mut name = row.label.clone();
                if reversed {
                    name.push_str(" ↓");
                }
                let msg = self.i18n.t_args("sort-set", &[("name", name.into())]);
                self.set_status(msg);
            }
        }
    }

    /// `F4`: open the editor on the item under the cursor.
    fn open_editor(&mut self) {
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        if entry.kind.is_dirlike() {
            self.not_implemented("file.edit (attributes dialog for directories)");
            return;
        }
        let path = self.panel().path_of(&entry);
        if path.in_container() {
            // Editing the copy would be easy and useless: there is nowhere to save it until
            // writing into a container exists, so `F3` is the honest answer today.
            let m = self.i18n.t("arc-read-only");
            self.set_error(m);
            return;
        }
        if path.is_remote() {
            // Fetch, edit the copy, and write it back on save (SPEC 04 §5).
            self.fetch_remote(path, entry.size, Fetched::Edit);
            return;
        }
        // A raw device is bytes by definition and never lines: it opens in hex, read-only,
        // without the text editor being given a chance to try (SPEC 05 §1.10).
        if std::fs::metadata(path.as_path())
            .map(|m| scmd_core::is_device(&m))
            .unwrap_or(false)
        {
            self.open_hex(path.as_path());
            return;
        }
        // A file a text view would mangle opens in hex instead (SPEC 05 §2.5). The reader's
        // own test decides, so `F3` and `F4` agree about what a binary is.
        if looks_binary_file(path.as_path()) {
            self.open_hex(path.as_path());
            return;
        }
        match Editor::open(path.as_path()) {
            Ok(editor) => {
                if editor.buffer.read_only {
                    let msg = self
                        .i18n
                        .t_args("editor-read-only", &[("name", editor.name().into())]);
                    self.set_status(msg);
                }
                self.mode = Mode::Editor(Box::new(editor));
            }
            Err(e) => self.set_error(format!("{path}: {e}")),
        }
    }

    /// Open the editor on bytes rather than lines.
    pub(crate) fn open_hex(&mut self, path: &std::path::Path) {
        match HexEdit::open(path) {
            Ok(mut editor) => {
                if editor.device {
                    // A device says what it is and how long it measured, because a size that
                    // came from a seek rather than from `stat` is worth stating once.
                    let msg = self.i18n.t_args(
                        "hex-device",
                        &[
                            ("name", editor.name().into()),
                            ("size", scmd_core::format_size(editor.len()).into()),
                        ],
                    );
                    self.set_status(msg);
                } else if editor.read_only {
                    let msg = self
                        .i18n
                        .t_args("editor-read-only", &[("name", editor.name().into())]);
                    self.set_status(msg);
                }
                editor.bytes_per_row = 16;
                self.mode = Mode::Hex(Box::new(editor));
            }
            Err(e) => self.set_error(e.to_string()),
        }
    }

    /// `Alt+H`: the same file the other way round — bytes instead of lines, or back again.
    ///
    /// It reopens from disk, so a buffer with unsaved changes is asked about first rather
    /// than quietly thrown away: the two models cannot hold each other's edits.
    fn toggle_hex(&mut self) {
        let (path, dirty) = match &self.mode {
            Mode::Editor(editor) => (editor.path.clone(), editor.buffer.modified()),
            Mode::Hex(editor) => (editor.path.clone(), editor.modified()),
            _ => return,
        };
        let Some(path) = path.filter(|p| p.is_local() && !p.in_container()) else {
            let m = self.i18n.t("hex-needs-a-file");
            self.set_error(m);
            return;
        };
        if dirty {
            let m = self.i18n.t("hex-save-first");
            self.set_error(m);
            return;
        }
        if self.mode.hex().is_some() {
            match Editor::open(path.as_path()) {
                Ok(editor) => self.mode = Mode::Editor(Box::new(editor)),
                Err(e) => self.set_error(format!("{path}: {e}")),
            }
            return;
        }
        self.open_hex(path.as_path());
    }

    /// `Shift+F4`: a new file in the active panel's directory.
    pub(crate) fn open_new_editor(&mut self) {
        self.mode = Mode::Editor(Box::new(Editor::empty(None)));
    }

    /// Open the editor on a copy fetched from a server. The buffer is the local file; the
    /// title, the status line and the write-back all speak in terms of the remote path.
    fn open_remote_editor(
        &mut self,
        remote: VPath,
        local: std::path::PathBuf,
        stamp: Option<RemoteStamp>,
    ) {
        match Editor::open(&local) {
            Ok(editor) => {
                let msg = self
                    .i18n
                    .t_args("net-editing", &[("name", remote.to_string().into())]);
                self.remote_edit = Some(RemoteEdit {
                    remote,
                    local,
                    stamp,
                });
                self.mode = Mode::Editor(Box::new(editor));
                self.set_status(msg);
            }
            Err(e) => self.set_error(format!("{}: {e}", local.display())),
        }
    }

    /// `Shift+F2` onto a remote path: write the buffer to a local copy, then send it up.
    fn save_as_remote(&mut self, target: VPath) {
        let dir = std::env::temp_dir().join(format!("scmd-{}", std::process::id()));
        if let Err(e) = std::fs::create_dir_all(&dir) {
            self.set_error(format!("{}: {e}", dir.display()));
            return;
        }
        let name = target.display_name();
        let local = dir.join(if name.is_empty() {
            "untitled".to_string()
        } else {
            name
        });
        let result = self
            .mode
            .editor_mut()
            .map(|e| (e.name(), e.save_as(&local)));
        match result {
            Some((_, Ok(_))) => {
                self.remote_edit = Some(RemoteEdit {
                    remote: target,
                    local,
                    // Nothing was fetched, so there is nothing to compare: an existing file
                    // on the server is overwritten, which is what "save as" means.
                    stamp: None,
                });
                self.write_back(true);
            }
            Some((name, Err(e))) => {
                let msg = self.i18n.t_args(
                    "editor-failed",
                    &[("name", name.into()), ("error", e.to_string().into())],
                );
                self.set_error(msg);
            }
            None => {}
        }
    }

    /// What the editor is editing on behalf of a server, if anything.
    pub fn remote_edit(&self) -> Option<&RemoteEdit> {
        self.remote_edit.as_ref()
    }

    #[cfg(test)]
    fn open_remote_editor_for_test(&mut self, remote: VPath, local: std::path::PathBuf) {
        self.open_remote_editor(
            remote,
            local,
            Some(RemoteStamp {
                size: 5,
                mtime: Some(1),
            }),
        );
    }

    /// Pretend a write-back is in flight, so a test can answer it twice.
    #[cfg(test)]
    fn upload_id_for_test(&mut self, id: u64) {
        self.upload_id = Some(id);
    }

    /// Send the editor's local copy back to the server it came from.
    fn write_back(&mut self, force: bool) {
        let Some(edit) = self.remote_edit.clone() else {
            return;
        };
        let id = self.next_job_id;
        self.next_job_id += 1;
        self.upload_id = Some(id);
        let msg = self.i18n.t_args(
            "net-writing-back",
            &[("name", edit.remote.to_string().into())],
        );
        self.set_status(msg);
        self.requests.push(Request::Upload {
            id,
            local: edit.local,
            remote: edit.remote,
            expect: edit.stamp,
            force,
        });
    }

    /// Park the editor under a prompt or confirmation, so it comes back afterwards.
    fn park_editor(&mut self) {
        if let Mode::Editor(editor) = std::mem::replace(&mut self.mode, Mode::Normal) {
            self.editor_parked = Some(editor);
        }
    }

    fn restore_editor(&mut self) {
        if let Some(editor) = self.editor_parked.take() {
            self.mode = Mode::Editor(editor);
        }
    }

    fn editor_prompt(&mut self, kind: PromptKind, title_key: &str, text: String) {
        self.park_editor();
        let title = self.i18n.t(title_key);
        self.mode = Mode::Prompt(Prompt::new(kind, title, text));
    }

    /// Save, turning the engine's answers into messages and dialogs.
    fn save_editor(&mut self, force: bool) {
        let Some(editor) = self.mode.editor_mut() else {
            return;
        };
        let name = editor.name();
        match editor.save(force) {
            Ok(scmd_edit::SaveOutcome::Saved { bytes, .. }) => {
                if self.remote_edit.is_some() {
                    // The local copy is written; now put it back on the server.
                    self.write_back(false);
                } else {
                    let msg = self.i18n.t_args(
                        "editor-saved",
                        &[("name", name.into()), ("bytes", (bytes as i64).into())],
                    );
                    self.set_status(msg);
                }
            }
            Ok(scmd_edit::SaveOutcome::NothingToDo) => {
                let msg = self
                    .i18n
                    .t_args("editor-nothing-to-save", &[("name", name.into())]);
                self.set_status(msg);
            }
            Err(scmd_edit::editor::EditorError::NoPath) => self.editor_prompt(
                PromptKind::EditorSaveAs,
                "editor-prompt-save-as",
                String::new(),
            ),
            Err(scmd_edit::editor::EditorError::ChangedOnDisk(_)) => {
                // Never overwrite someone else's work without being told to.
                let title = self.i18n.t("editor-changed-title");
                let body = self
                    .i18n
                    .t_args("editor-changed-body", &[("name", name.into())]);
                self.park_editor();
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines: vec![body],
                    action: ConfirmAction::OverwriteChanged,
                });
            }
            Err(e) => {
                let msg = self.i18n.t_args(
                    "editor-failed",
                    &[("name", name.into()), ("error", e.to_string().into())],
                );
                self.set_error(msg);
            }
        }
    }

    /// Leaving the editor: unsaved changes are confirmed, never dropped silently.
    fn close_editor(&mut self) {
        let dirty = self
            .mode
            .editor()
            .map(|e| e.buffer.modified())
            .unwrap_or(false);
        if !dirty {
            self.mode = Mode::Normal;
            self.editor_parked = None;
            self.remote_edit = None;
            // The file may have been saved: refresh both panels.
            self.read_panel(0);
            self.read_panel(1);
            return;
        }
        let name = self.mode.editor().map(|e| e.name()).unwrap_or_default();
        let title = self.i18n.t("editor-discard-title");
        let body = self
            .i18n
            .t_args("editor-discard-body", &[("name", name.into())]);
        self.park_editor();
        self.mode = Mode::Confirm(Confirm {
            title,
            lines: vec![body],
            action: ConfirmAction::DiscardEdits,
        });
    }

    fn editor_command(&mut self, cmd: Command) {
        // The hex editor shares the editor's keys because it shares its commands; the few
        // that mean something there are handled on their own, and the rest say so.
        if self.mode.hex().is_some() {
            self.hex_command(cmd);
            return;
        }
        // `F9` is the assembler in bytes and the menu bar in lines: there is nothing to
        // assemble over a line of text, so the key keeps the meaning it has everywhere else.
        if cmd == Command::EditorAssemble {
            self.run(Command::AppMenu);
            return;
        }
        let (width, height) = self.editor_size;
        let mut message: Option<String> = None;
        let mut error: Option<String> = None;

        match cmd {
            Command::EditorQuit => {
                self.close_editor();
                return;
            }
            Command::EditorSave => {
                self.save_editor(false);
                return;
            }
            Command::EditorSaveAs => {
                let text = self
                    .mode
                    .editor()
                    .and_then(|e| e.path.as_ref().map(|p| p.to_string()))
                    .unwrap_or_default();
                self.editor_prompt(PromptKind::EditorSaveAs, "editor-prompt-save-as", text);
                return;
            }
            Command::EditorSearch => {
                let text = self
                    .mode
                    .editor()
                    .and_then(|e| e.last_search.clone())
                    .unwrap_or_default();
                self.editor_prompt(PromptKind::EditorSearch, "editor-prompt-search", text);
                return;
            }
            Command::EditorReplace => {
                self.editor_prompt(
                    PromptKind::EditorReplaceFrom,
                    "editor-prompt-replace-from",
                    String::new(),
                );
                return;
            }
            Command::EditorGoto => {
                self.editor_prompt(PromptKind::EditorGoto, "editor-prompt-goto", String::new());
                return;
            }
            _ => {}
        }

        let Some(editor) = self.mode.editor_mut() else {
            return;
        };
        let name = editor.name();
        let result: Result<(), String> = (|| {
            match cmd {
                Command::EditorUndo => {
                    if !editor.undo() {
                        return Err("editor-no-undo".into());
                    }
                }
                Command::EditorRedo => {
                    if !editor.redo() {
                        return Err("editor-no-redo".into());
                    }
                }
                Command::EditorSelectAll => editor.select_all(),
                Command::EditorCopy => {
                    editor.copy_block();
                }
                Command::EditorCut => {
                    editor.cut_block().map_err(|e| e.to_string())?;
                }
                Command::EditorPaste => {
                    editor.paste().map_err(|e| e.to_string())?;
                }
                Command::EditorDeleteLine => editor.delete_line().map_err(|e| e.to_string())?,
                Command::EditorDuplicateLine => {
                    editor.duplicate_line().map_err(|e| e.to_string())?
                }
                Command::EditorMoveLineUp => editor.move_line(false).map_err(|e| e.to_string())?,
                Command::EditorMoveLineDown => editor.move_line(true).map_err(|e| e.to_string())?,
                Command::EditorIndent => editor.indent().map_err(|e| e.to_string())?,
                Command::EditorOutdent => editor.outdent().map_err(|e| e.to_string())?,
                Command::EditorDeleteToEnd => {
                    editor.delete_to_line_end().map_err(|e| e.to_string())?
                }
                Command::EditorDeleteWordLeft => {
                    editor.delete_word_left().map_err(|e| e.to_string())?
                }
                Command::EditorToggleReadOnly => {
                    editor.buffer.read_only = !editor.buffer.read_only;
                    return Err(if editor.buffer.read_only {
                        "editor-readonly-on".into()
                    } else {
                        "editor-readonly-off".into()
                    });
                }
                Command::EditorReload => {
                    editor.reload().map_err(|e| e.to_string())?;
                    return Err("editor-reloaded".into());
                }
                Command::EditorSearchNext | Command::EditorSearchPrev => {
                    let query = editor.last_search.clone().unwrap_or_default();
                    if query.is_empty() {
                        return Ok(());
                    }
                    let backwards = cmd == Command::EditorSearchPrev;
                    if !editor.find(&query, backwards).map_err(|e| e.to_string())? {
                        return Err(format!("not-found:{query}"));
                    }
                }
                _ => {}
            }
            editor.scroll_into_view(height as usize, width as usize);
            Ok(())
        })();

        match result {
            Ok(()) => {}
            // Message keys the editor hands back, resolved here where the catalogue lives.
            Err(key) if key.starts_with("not-found:") => {
                let query = key.trim_start_matches("not-found:").to_string();
                message = Some(
                    self.i18n
                        .t_args("editor-not-found", &[("query", query.into())]),
                );
            }
            Err(key) if key.starts_with("editor-") && !key.contains(' ') => {
                message = Some(self.i18n.t(&key));
            }
            Err(other) => {
                error = Some(self.i18n.t_args(
                    "editor-failed",
                    &[("name", name.into()), ("error", other.into())],
                ));
            }
        }
        if let Some(text) = message {
            self.set_status(text);
        }
        if let Some(text) = error {
            self.set_error(text);
        }
    }

    /// `F6`: produce readable text from a compiled file.
    ///
    /// WebAssembly is printed in process; everything else goes to an external tool, because a
    /// decompiler is a research project and we would rather call a good one than ship a bad
    /// one. See `[[decompile]]` in the config.
    fn decompile(&mut self) {
        let Some(viewer) = self.mode.viewer_mut() else {
            return;
        };
        let name = viewer.path.display_name();
        let path = viewer.path.clone();
        let format = viewer.format();

        if format == scmd_view::Format::Wasm {
            match viewer.wasm_text() {
                Ok(text) => {
                    viewer.show_derived("WAT", text);
                    let msg = self.i18n.t_args(
                        "viewer-derived",
                        &[("producer", "WAT".into()), ("name", name.into())],
                    );
                    self.set_status(msg);
                }
                Err(e) => {
                    let msg = self.i18n.t_args(
                        "viewer-disasm-failed",
                        &[("name", name.into()), ("error", e.to_string().into())],
                    );
                    self.set_error(msg);
                }
            }
            return;
        }

        match self.cfg.decompiler_for(&name) {
            Some(rule) => {
                let id = self.next_job_id;
                self.next_job_id += 1;
                self.decompile_id = Some(id);
                let msg = self.i18n.t_args(
                    "viewer-decompile-running",
                    &[("tool", rule.name.clone().into()), ("name", name.into())],
                );
                self.set_status(msg);
                self.requests.push(Request::Decompile { id, rule, path });
            }
            None => {
                let msg = self.i18n.t_args(
                    "viewer-decompile-none",
                    &[("format", format.as_str().into())],
                );
                self.set_error(msg);
            }
        }
    }

    /// A prompt that keeps the viewer underneath it (search, go to).
    fn mode_prompt_over_viewer(&mut self, kind: PromptKind, title: String, text: String) {
        let Mode::Viewer(viewer) = std::mem::replace(&mut self.mode, Mode::Normal) else {
            return;
        };
        self.viewer_under_prompt = Some(viewer);
        self.mode = Mode::Prompt(Prompt::new(kind, title, text));
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
                    // text → dump → hex → text. Disassembly is F5, not part of the cycle,
                    // because it only applies to some files.
                    let next = match viewer.mode {
                        scmd_view::Mode::Text => scmd_view::Mode::Dump,
                        scmd_view::Mode::Dump => scmd_view::Mode::Hex,
                        scmd_view::Mode::Hex | scmd_view::Mode::Disasm => scmd_view::Mode::Text,
                    };
                    viewer.set_mode(next);
                }
                Command::ViewerFollowBranch => {
                    if viewer.follow_branch().is_none() {
                        error = Some("viewer-no-branch".into());
                    }
                }
                Command::ViewerBack => {
                    if !viewer.unfollow_branch() {
                        error = Some("viewer-no-back".into());
                    }
                }
                Command::ViewerStructure => {
                    if !viewer.toggle_structure() {
                        error = Some("viewer-no-structure".into());
                    }
                }
                Command::ViewerPaneNext => viewer.toggle_struct_focus(),
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

    /// `Enter` on an archive goes into it (SPEC 04 §4).
    ///
    /// Decided by the name, like FAR: opening every file under the cursor to look at its first
    /// bytes is not a thing to do on a key the user presses to walk a tree. A file that lies
    /// about its name says so when the listing fails, which is the right place for it.
    fn archive_target(&self) -> Option<VPath> {
        let panel = self.panel();
        let entry = panel.current()?;
        if entry.kind != scmd_core::EntryKind::File {
            return None;
        }
        let name = entry.name_str();
        if !scmd_vfs::ArcVfs::looks_like_archive(std::path::Path::new(&*name)) {
            return None;
        }
        Some(panel.path_of(entry).enter("/"))
    }

    /// Compare the two panels and select what differs (SPEC 02 §9, FAR's own rule).
    ///
    /// Names, sizes and times — no subdirectories and no contents, which is what makes it
    /// instant and what the deeper comparison in the backlog is for. In each panel it selects
    /// the files the other panel does not have, and the ones it has in an older version; a
    /// file that is the same on both sides is selected in neither, which is the whole point:
    /// what stays selected is what a copy would have to carry.
    fn compare_dirs(&mut self) {
        /// A file's name, with the comparison's own notion of "the same name": case is
        /// ignored, because the two panels may be on filesystems that disagree about it, and
        /// a pair that differs only in case is a difference the user wants to see rather than
        /// a pair we quietly match up.
        fn key(entry: &Entry) -> String {
            entry.name_str().to_lowercase()
        }

        let mut facts: [std::collections::HashMap<String, (u64, Option<std::time::SystemTime>)>;
            2] = [Default::default(), Default::default()];
        for (side, side_facts) in facts.iter_mut().enumerate() {
            let listing = &self.panels[side].listing;
            for row in 0..listing.len() {
                let entry = listing.row(row).expect("row in range");
                // Directories are not compared: FAR's version does not look inside them, and
                // selecting one would mean copying a tree the user has not been shown.
                if entry.kind.is_dirlike() {
                    continue;
                }
                side_facts.insert(key(entry), (entry.size, entry.mtime));
            }
        }

        let mut marked = [0u64; 2];
        for (side, count) in marked.iter_mut().enumerate() {
            let other = 1 - side;
            let listing = self.panels[side].listing.clone();
            let mut wanted: Vec<scmd_core::EntryId> = Vec::new();
            for row in 0..listing.len() {
                let entry = listing.row(row).expect("row in range");
                if entry.kind.is_dirlike() {
                    continue;
                }
                let mine = (entry.size, entry.mtime);
                let theirs = facts[other].get(&key(entry));
                let differs = match theirs {
                    // Not over there at all: this copy is the only one.
                    None => true,
                    // There, and the same size at the same time: nothing to do.
                    Some(other) if *other == mine => false,
                    // There and older, or there with no time to compare and a different
                    // size. A file with no timestamp on either side falls back to the size,
                    // because that is all there is to go on.
                    Some((other_size, other_time)) => match (mine.1, other_time) {
                        (Some(a), Some(b)) => a > *b,
                        _ => mine.0 != *other_size,
                    },
                };
                if differs {
                    wanted.push(entry.id);
                }
            }
            *count = wanted.len() as u64;
            self.panels[side].selection.clear();
            for id in wanted {
                self.panels[side].selection.set(id, true);
            }
        }

        let msg = self.i18n.t_args(
            "compare-done",
            &[
                ("left", (marked[0] as i64).into()),
                ("right", (marked[1] as i64).into()),
            ],
        );
        self.set_status(msg);
    }

    /// `Ctrl+Q` and `Ctrl+L`: turn a special panel kind on, or off again (SPEC 02 §7).
    ///
    /// FAR's rule, and ours: the kind goes in the *passive* panel, so `Side::Active` means
    /// "the other one" — pressing the key twice puts the files back.
    fn set_content(&mut self, side: Side, want: crate::panel::Content) {
        use crate::panel::Content;
        let index = match side {
            Side::Left => 0,
            Side::Right => 1,
            // The special kinds go in the passive panel — except when the active panel is
            // already one of them, which only the tree can be: the key that turned it on has
            // to turn it off, and by then the focus is in it.
            Side::Active if self.panels[self.active].content != Content::Files => self.active,
            Side::Active => 1 - self.active,
        };
        let now = self.panels[index].content;
        let next = if now == want { Content::Files } else { want };
        self.panels[index].content = next;
        if next == Content::Files {
            // Back to a directory: it may have missed changes while it was something else.
            // The tree's rows and the preview's reader go with it; the directory cache they
            // came from stays, so turning the panel back into a tree costs no reads.
            self.trees[index] = None;
            self.quick = None;
            self.quick_of = None;
            self.read_panel(index);
            return;
        }
        if next == Content::Tree {
            // The tree is the one special kind that takes the focus: moving its cursor is how
            // it changes the other panel's directory, so the keys have to reach it.
            //
            // It is rooted where the other panel's path starts — the drive, or the archive, or
            // the server — and opens its way down to where that panel is, so it comes up with
            // the cursor on the directory the user is already looking at.
            // The spelling the filesystem uses, not the one the panel shows: the tree can only
            // put its cursor on a row a listing produced, and a listing of `/tmp` on macOS
            // says `/private/tmp`, while Windows may hand us `RUNNER~1` for a long name.
            let here = scmd_vfs::local::real_path(&self.panels[1 - index].cwd);
            let mut tree = crate::tree::Tree::new(here.root());
            tree.want = Some(here);
            self.active = index;
            self.trees[index] = Some(Box::new(tree));
            self.tree_reveal(index);
            return;
        }
        self.trees[index] = None;
        // Nothing in the other two can be selected or copied, so the keys stay with the
        // files: the focus moves to the other panel rather than onto a frame with no rows.
        self.active = 1 - index;
        if next == Content::QuickView {
            // Opened now rather than on the next frame, so the panel is never blank once.
            self.quick_view_for(self.active);
        }
    }

    /// Open the tree's way towards the directory it wants to show, asking for the one read it
    /// needs. The loop and the reason it cannot spin are in [`crate::tree::Tree::reveal`].
    fn tree_reveal(&mut self, index: usize) {
        let page = self.rows.max(1);
        let Some(mut tree) = self.trees[index].take() else {
            return;
        };
        let wanted = tree.reveal(&self.tree_cache, page);
        self.trees[index] = Some(tree);
        if let Some(path) = wanted {
            self.requests.push(Request::TreeRead { panel: index, path });
        }
    }

    /// Keys inside the tree panel. Movement walks the rows and takes the other panel with it;
    /// `Enter`/`→` expands, `←` collapses or climbs, `*` expands everything under the cursor.
    fn tree_key(&mut self, cmd: Command) -> bool {
        let index = self.active;
        let page = self.rows.max(1);
        let Some(mut tree) = self.trees[index].take() else {
            return false;
        };
        let mut wanted: Vec<VPath> = Vec::new();
        let handled = match cmd {
            Command::NavUp => {
                tree.move_cursor(-1, page);
                true
            }
            Command::NavDown => {
                tree.move_cursor(1, page);
                true
            }
            Command::NavPageUp => {
                tree.move_cursor(-(page as isize), page);
                true
            }
            Command::NavPageDown => {
                tree.move_cursor(page as isize, page);
                true
            }
            Command::NavTop => {
                tree.move_cursor(-(tree.rows.len() as isize), page);
                true
            }
            Command::NavBottom => {
                tree.move_cursor(tree.rows.len() as isize, page);
                true
            }
            Command::NavEnter | Command::NavRight => {
                if !tree.expand(&self.tree_cache) {
                    if let Some(path) = tree.path() {
                        wanted.push(path);
                    }
                }
                true
            }
            Command::NavLeft => {
                if tree.is_expanded() {
                    tree.collapse();
                } else {
                    // Not expanded: `←` climbs to the parent row, which is where the eye is.
                    let depth = tree.current().map(|r| r.depth).unwrap_or(0);
                    if depth > 0 {
                        let up = tree.rows[..tree.cursor]
                            .iter()
                            .rposition(|r| r.depth < depth)
                            .unwrap_or(0);
                        let delta = up as isize - tree.cursor as isize;
                        tree.move_cursor(delta, page);
                    }
                }
                true
            }
            // `*` has nothing to invert in a tree, and FAR's tree panel uses it to expand
            // everything under the cursor — bounded, so `*` on `/` cannot hang the panel.
            Command::SelInvert => {
                wanted = tree.expand_all(&self.tree_cache);
                true
            }
            Command::PanelReread => {
                if let Some(path) = tree.path() {
                    self.tree_cache.forget(&path);
                    tree.collapse();
                    wanted.push(path);
                }
                true
            }
            _ => false,
        };
        let chosen = tree.path();
        self.trees[index] = Some(tree);
        for path in wanted {
            self.requests.push(Request::TreeRead { panel: index, path });
        }
        if handled {
            // The other panel follows the cursor: that is the whole point of the tree.
            if let Some(path) = chosen {
                let other = 1 - index;
                if self.panels[other].cwd != path {
                    self.panels[other].go(path, true);
                    self.read_panel(other);
                }
            }
        }
        handled
    }

    /// Keys inside a special panel. The quick view scrolls; the info panel has nothing to
    /// scroll. Returns true when the key was ours, so the panel's own row movement — which
    /// would move a cursor that is not there — never runs.
    fn special_panel_key(&mut self, cmd: Command) -> bool {
        use crate::panel::Content;
        if self.panels[self.active].content == Content::Files {
            return false;
        }
        if self.panels[self.active].content == Content::Tree {
            return self.tree_key(cmd);
        }
        let rows = self.rows.max(1) as i64;
        let delta = match cmd {
            Command::NavUp => -1,
            Command::NavDown => 1,
            Command::NavPageUp => -rows,
            Command::NavPageDown => rows,
            Command::NavTop => i64::MIN,
            Command::NavBottom => i64::MAX,
            // Everything else belongs to whatever the command normally does; a panel with no
            // rows simply has nothing for most of them to act on.
            _ => return false,
        };
        let (w, h) = (self.quick_size.0.max(1), self.quick_size.1.max(1));
        if let Some(viewer) = self.quick.as_mut() {
            match delta {
                i64::MIN => viewer.goto(0),
                i64::MAX => viewer.goto_percent(100),
                d => viewer.scroll(d, w, h),
            }
        }
        true
    }

    /// Point the quick view at whatever `panel` has under its cursor, opening a reader when
    /// that has changed and leaving it alone when it has not.
    pub fn quick_view_for(&mut self, panel: usize) {
        let Some(entry) = self.panels[panel].current().cloned() else {
            self.quick = None;
            self.quick_of = None;
            self.quick_why = Some(self.i18n.t("msg-nothing-to-do"));
            return;
        };
        if entry.kind.is_dirlike() {
            // A directory has no bytes to show. FAR puts its size here; measuring it is
            // `F3`'s job and takes a walk, so this says what it is and leaves it at that.
            self.quick = None;
            self.quick_of = None;
            self.quick_why = Some(self.i18n.t_args(
                "quick-view-directory",
                &[("name", entry.name_str().into_owned().into())],
            ));
            return;
        }
        let path = self.panels[panel].path_of(&entry);
        if self.quick_of.as_ref() == Some(&path) {
            return;
        }
        if !path.is_local() || path.in_container() {
            // The bytes would have to be fetched, and a preview that copies a file down on
            // every cursor movement is a trap. `F3` fetches it when the user asks.
            self.quick = None;
            self.quick_of = None;
            self.quick_why = Some(self.i18n.t("quick-view-not-here"));
            return;
        }
        match scmd_view::FileSource::open(path.as_path()) {
            Ok(source) => {
                self.quick = Some(Box::new(Viewer::new(Box::new(source), path.clone())));
                self.quick_of = Some(path);
                self.quick_why = None;
            }
            Err(e) => {
                self.quick = None;
                self.quick_of = None;
                self.quick_why = Some(format!("{}: {e}", path.display_name()));
            }
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

    /// Is an editor parked under a prompt? The renderer needs to know before it takes it.
    pub fn editor_parked_for_render(&self) -> bool {
        self.editor_parked.is_some()
    }

    /// Hand the editor to the renderer (it needs `&mut` to scroll), and say whether it was
    /// parked under a prompt so it goes back where it came from.
    pub fn editor_for_render(&mut self) -> Option<(Box<Editor>, bool)> {
        if let Some(editor) = self.editor_parked.take() {
            return Some((editor, true));
        }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Editor(editor) => Some((editor, false)),
            other => {
                self.mode = other;
                None
            }
        }
    }

    pub fn return_editor(&mut self, editor: Box<Editor>, parked: bool) {
        if parked {
            self.editor_parked = Some(editor);
        } else {
            self.mode = Mode::Editor(editor);
        }
    }

    /// The key bar while the editor is open.
    pub fn keybar_editor(&self) -> Vec<(u8, String)> {
        vec![
            (1, self.i18n.t("key-help")),
            (2, self.i18n.t("key-save")),
            (4, self.i18n.t("key-quit")),
            (7, self.i18n.t("key-search")),
            (8, self.i18n.t("key-replace")),
            (10, self.i18n.t("key-quit")),
        ]
    }

    /// The key bar while the hex editor is open.
    pub fn hex_keybar(&self) -> Vec<(u8, String)> {
        vec![
            (1, self.i18n.t("key-help")),
            (2, self.i18n.t("key-save")),
            (3, self.i18n.t("key-mark")),
            (9, self.i18n.t("key-asm")),
            (10, self.i18n.t("key-quit")),
            (12, self.i18n.t("key-block")),
        ]
    }

    /// The key bar while the viewer is open.
    pub fn keybar_viewer(&self) -> Vec<(u8, String)> {
        vec![
            (1, self.i18n.t("key-help")),
            (2, self.i18n.t("key-wrap")),
            (3, self.i18n.t("key-quit")),
            (4, self.i18n.t("key-hex")),
            (5, self.i18n.t("key-disasm")),
            (6, self.i18n.t("key-edit")),
            (7, self.i18n.t("key-search")),
            (8, self.i18n.t("key-code")),
            (10, self.i18n.t("key-quit")),
            (11, self.i18n.t("key-struct")),
        ]
    }

    /// The viewer's status line.
    pub fn viewer_status(&self, status: &scmd_view::Status) -> String {
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
        if status.wrap != scmd_view::Wrap::Off {
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
            (12, "key-screens"),
        ]
        .iter()
        .map(|(n, id)| (*n, self.i18n.t(id)))
        .collect()
    }

    /// The help page, generated from the live keymap so it can never drift.
    /// The lines of one chapter, ready to draw. The written ones are rendered from the text
    /// compiled into the binary; the rest are built here, from what the program is running.
    pub fn help_lines(&self, chapter: crate::help::Chapter, width: usize) -> Vec<String> {
        use crate::help::Chapter;
        match chapter {
            Chapter::Index => crate::help::chapters()
                .into_iter()
                .map(|(_, title)| title)
                .collect(),
            Chapter::Prose(i) => match crate::help::PROSE.get(i) {
                Some(page) => crate::help::render(page.text, width),
                None => Vec::new(),
            },
            Chapter::Keys => self.help_keys(),
            Chapter::Commands => self.help_commands(),
            Chapter::Options => self.help_options(),
        }
    }

    /// The width the help renders prose at: the dialog is the screen less its frame, and the
    /// panel's own width is the nearest thing the application is told about. It only has to
    /// agree with the renderer closely enough for the scroll limit to be right.
    pub fn help_width(&self) -> usize {
        (self.viewer_size.0 as usize)
            .saturating_sub(4)
            .clamp(20, 100)
    }

    /// Every binding of the keymap as loaded, by scope. Generated rather than written, so it
    /// shows the profile in force and any binding of the reader's own.
    fn help_keys(&self) -> Vec<String> {
        let mut lines: Vec<String> = vec![
            format!(
                "The {} profile, plus your own bindings.",
                self.cfg.ui.keymap
            ),
            String::new(),
        ];
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

    /// Every command in the vocabulary with the keys that reach it — including the ones that
    /// reach none, which is the list someone binding a key of their own needs.
    fn help_commands(&self) -> Vec<String> {
        let mut lines = vec![
            "Every command, and the keys bound to it. One with no key is still a command:".into(),
            "put it in keymap.toml, or type it on the command line.".into(),
            String::new(),
        ];
        let mut unbound = 0usize;
        for cmd in Command::all() {
            let keys: Vec<String> = self
                .keymap
                .bindings()
                .filter(|(_, _, c)| *c == cmd)
                .map(|(_, k, _)| k.to_string())
                .collect();
            if keys.is_empty() {
                unbound += 1;
            }
            lines.push(format!("  {:<28} {}", cmd.id(), keys.join(", ")));
        }
        lines.push(String::new());
        lines.push(format!(
            "{} commands, {unbound} of them with no key.",
            Command::all().len()
        ));
        lines
    }

    /// The settings as they are now, in the same shape `--dump-config` prints them, because
    /// that is the file one edits.
    fn help_options(&self) -> Vec<String> {
        let mut lines = vec![
            "The settings this session is running with, as config.toml spells them.".into(),
            "`scmd --dump-config` prints the same thing to a terminal.".into(),
            String::new(),
        ];
        lines.extend(self.cfg.dump().lines().map(|l| l.to_string()));
        lines
    }
}

/// Commands that only make sense inside the editor.
/// Would a text view mangle this file? The viewer's own test, on the first few kilobytes.
fn looks_binary_file(path: &std::path::Path) -> bool {
    use std::io::Read;
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut head = [0u8; 4096];
    match file.read(&mut head) {
        Ok(read) => scmd_view::looks_binary(&head[..read]),
        Err(_) => false,
    }
}

fn is_editor_command(cmd: Command) -> bool {
    cmd.id().starts_with("editor.")
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
            | Command::ViewerFollowBranch
            | Command::ViewerBack
            | Command::ViewerStructure
            | Command::ViewerPaneNext
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

/// The one-based line a byte offset falls on. The sum is over encoded bytes, so a file in a
/// code page other than UTF-8 still lands on the right line.
fn line_at_offset(editor: &Editor, offset: u64) -> usize {
    let mut at = 0u64;
    let eol = editor.buffer.eol.bytes().len() as u64;
    for (i, line) in editor.buffer.lines().iter().enumerate() {
        let bytes = editor.buffer.codepage.encode(line).len() as u64 + eol;
        if at + bytes > offset {
            return i + 1;
        }
        at += bytes;
    }
    editor.buffer.len().max(1)
}

/// The byte offset a line starts at: the other direction, for `F6` out of the editor.
fn offset_at_line(editor: &Editor, line: usize) -> u64 {
    let eol = editor.buffer.eol.bytes().len() as u64;
    editor
        .buffer
        .lines()
        .iter()
        .take(line)
        .map(|l| editor.buffer.codepage.encode(l).len() as u64 + eol)
        .sum()
}

fn parse_sort(s: &str) -> SortKey {
    match s.trim().to_ascii_lowercase().as_str() {
        "ext" | "extension" => SortKey::Ext,
        "size" => SortKey::Size,
        "mtime" | "time" | "date" => SortKey::MTime,
        "btime" | "created" => SortKey::BTime,
        "atime" | "accessed" => SortKey::ATime,
        "none" | "unsorted" => SortKey::Unsorted,
        _ => SortKey::Name,
    }
}

/// The places a panel can jump to: `/` and the home directory on Unix, the drive letters that
/// answer on Windows. No platform crate yet, so this is what `std` can tell us.
pub fn local_roots() -> Vec<String> {
    let mut roots = Vec::new();
    if cfg!(windows) {
        for letter in b'A'..=b'Z' {
            let path = format!("{}:\\", letter as char);
            if std::path::Path::new(&path).exists() {
                roots.push(path);
            }
        }
    } else {
        roots.push("/".to_string());
    }
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = home.to_string_lossy().into_owned();
        if !home.is_empty() && !roots.contains(&home) {
            roots.push(home);
        }
    }
    roots
}

/// Where a panel goes when the connection it was on is closed.
pub fn local_home() -> VPath {
    VPath::local(
        std::env::var_os("HOME")
            .or_else(|| std::env::var_os("USERPROFILE"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("/")),
    )
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
    use scmd_core::{EntryFlags, EntryId, EntryKind};
    use std::ffi::OsString;

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn app() -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(std::path::Path::new("/tmp/scmd-test/scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local("/home/mb"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
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

    // ── the menu bar and the dialogs (SPEC 10) ──────────────────────────────────────────────

    /// Where in the open menu is the entry with this label?
    fn menu_row(menu: usize, label: &str) -> usize {
        menu::menus()[menu]
            .items
            .iter()
            .position(|e| matches!(e, menu::Entry::Item(i) if i.label == label))
            .unwrap_or_else(|| panic!("no `{label}` in menu {menu}"))
    }

    fn open(app: &mut App, menu: usize, label: &str) {
        press(app, "F9");
        let item = menu_row(menu, label);
        app.mode = Mode::Menu { menu, item };
        press(app, "Enter");
    }

    fn type_text(app: &mut App, text: &str) {
        for c in text.chars() {
            let key = if c == ' ' {
                "Space".to_string()
            } else {
                c.to_string()
            };
            press(app, &key);
        }
    }

    #[test]
    fn the_session_editor_stores_a_connection_that_the_netbox_can_use() {
        let mut app = app();
        press(&mut app, "Alt+N");
        press(&mut app, "Ins");
        let Mode::Form(form) = &app.mode else {
            panic!("the session editor did not open")
        };
        assert_eq!(form.kind, FormKind::Site);
        assert_eq!(form.site, None, "Ins makes a new one, it does not edit");

        type_text(&mut app, "jump");
        press(&mut app, "Down");
        type_text(&mut app, "jump.example.net");
        press(&mut app, "Down");
        type_text(&mut app, "2222");
        press(&mut app, "Down");
        type_text(&mut app, "mb");
        press(&mut app, "F10");

        assert_eq!(app.cfg.sites.len(), 1);
        let stored = &app.cfg.sites[0];
        assert_eq!(stored.id, "jump");
        assert_eq!(stored.host, "jump.example.net");
        assert_eq!(stored.port, Some(2222));
        assert_eq!(stored.user.as_deref(), Some("mb"));
        assert_eq!(stored.auth.as_deref(), Some("agent"));
        // The list the dialog came from shows it straight away.
        assert!(app.sites.iter().any(|s| s.id == "jump"));
        assert!(matches!(app.mode, Mode::Sites { .. }));
    }

    #[test]
    fn the_session_editor_keeps_the_dialog_open_on_something_it_cannot_store() {
        let mut app = app();
        app.open_site_form(None);
        press(&mut app, "F10"); // no host at all
        assert!(
            matches!(app.mode, Mode::Form(_)),
            "a dialog that cannot be applied must stay open"
        );
        assert!(app.status_error, "{}", app.status);
        assert!(app.cfg.sites.is_empty());

        // A port that is not a number is refused too, and nothing is stored.
        press(&mut app, "Down");
        type_text(&mut app, "host.example");
        press(&mut app, "Down");
        type_text(&mut app, "no");
        press(&mut app, "F10");
        assert!(matches!(app.mode, Mode::Form(_)));
        assert!(app.status.contains("no"), "{}", app.status);
        assert!(app.cfg.sites.is_empty());
    }

    #[test]
    fn f4_edits_the_connection_under_the_cursor_instead_of_adding_one() {
        let mut app = app();
        app.cfg.sites.push(scmd_config::SiteCfg {
            id: "box".into(),
            host: "old.example.net".into(),
            ..Default::default()
        });
        let (sites, problems) = Site::list_from_config(&app.cfg);
        assert!(problems.is_empty(), "{problems:?}");
        app.sites = sites;

        press(&mut app, "Alt+N");
        press(&mut app, "F4");
        let Mode::Form(form) = &app.mode else {
            panic!("the session editor did not open")
        };
        assert_eq!(form.site, Some(0));
        assert_eq!(form.text_value("site.host"), "old.example.net");

        // Clear the host and type another one: Backspace works where the caret is.
        press(&mut app, "Down");
        for _ in 0..40 {
            press(&mut app, "Backspace");
        }
        type_text(&mut app, "new.example.net");
        press(&mut app, "F10");
        assert_eq!(app.cfg.sites.len(), 1, "F4 must edit, not add");
        assert_eq!(app.cfg.sites[0].host, "new.example.net");
        assert_eq!(app.cfg.sites[0].id, "box");
    }

    #[test]
    fn a_key_file_is_asked_for_when_the_key_authentication_is_chosen() {
        let mut app = app();
        app.open_site_form(None);
        press(&mut app, "Down");
        type_text(&mut app, "host.example");
        // Down to the authentication choice, and pick the key file.
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Down");
        let Mode::Form(form) = &app.mode else {
            panic!("the dialog closed")
        };
        assert!(
            matches!(
                form.fields.get(form.cursor),
                Some(crate::form::Field::Choice { .. })
            ),
            "expected to be on the authentication row"
        );
        press(&mut app, "Right"); // agent -> key
        press(&mut app, "F10");
        assert!(matches!(app.mode, Mode::Form(_)), "{}", app.status);
        assert!(!app.status.is_empty());

        press(&mut app, "Down"); // the key file row
        type_text(&mut app, "~/.ssh/id_ed25519");
        press(&mut app, "F10");
        assert_eq!(
            app.cfg.sites.first().and_then(|s| s.auth.clone()),
            Some("key:~/.ssh/id_ed25519".to_string())
        );
    }

    #[test]
    fn alt_del_asks_first_and_says_what_overwriting_cannot_promise() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        app.run(Command::FileWipe);
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        let text = confirm.lines.join(" ");
        assert!(text.contains("a.txt"), "{text}");
        // The dialog has to be honest about SSDs and snapshots, or it promises something the
        // filesystem cannot deliver.
        assert!(text.contains("SSD"), "{text}");
        match &confirm.action {
            ConfirmAction::Job(spec) => {
                assert_eq!(spec.kind, scmd_jobs::JobKind::Wipe);
                assert_eq!(spec.sources.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn alt_f6_puts_a_relative_symlink_in_the_other_panel() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        app.run(Command::FileLink);
        let Mode::Prompt(prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("the link prompt did not open")
        };
        assert_eq!(prompt.kind, PromptKind::LinkName);
        // Pre-filled with the passive panel's directory, as a copy would be — joined with
        // whatever separator this platform uses.
        let expected = std::path::PathBuf::from("/tmp").join("a.txt");
        assert_eq!(prompt.text, expected.to_string_lossy());
        assert_eq!(
            prompt.sources.len(),
            1,
            "the target travels with the prompt"
        );
        app.prompt_accept(prompt);
        match app.take_requests().into_iter().next() {
            Some(Request::Link { target, link, kind }) => {
                assert!(target.to_string().ends_with("a.txt"), "{target}");
                assert_eq!(link.as_path(), expected);
                assert_eq!(kind, scmd_jobs::LinkKind::SymlinkRelative);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_viewer_walks_to_the_next_file_without_going_back_to_the_panel() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"first\n").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"second\n").unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let mut app = app_on_disk(dir.path());
        // `sub` sorts before the files, so two steps to reach one of them.
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "F3");
        let first = app.mode.viewer().expect("open").path.display_name();
        press(&mut app, "+");
        let second = app.mode.viewer().expect("still open").path.display_name();
        assert_ne!(first, second, "it did not move");
        // A directory is skipped rather than measured.
        assert!(second.ends_with(".txt"), "{second}");
        // And the panel's cursor came along, so Esc leaves it on what was last read.
        assert_eq!(app.panel().current().unwrap().name_str(), second);
        press(&mut app, "-");
        assert_eq!(
            app.mode.viewer().expect("still open").path.display_name(),
            first
        );
    }

    #[test]
    fn f6_crosses_between_the_viewer_and_the_editor_at_the_same_place() {
        let dir = tempfile::tempdir().unwrap();
        // Ten lines of six bytes each, so an offset maps to an obvious line.
        let text: String = (0..10).map(|i| format!("line{i}\n")).collect();
        std::fs::write(dir.path().join("notes.txt"), &text).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        // Down three lines, then across to the editor.
        for _ in 0..3 {
            press(&mut app, "Down");
        }
        let offset = app.mode.viewer().expect("open").top;
        assert_eq!(offset, 18, "three lines of six bytes");
        press(&mut app, "F6");
        let editor = app.mode.editor().expect("the editor did not open");
        assert_eq!(editor.cursor.line, 3, "line 4, one-based");
        // And back again, landing on the same byte.
        press(&mut app, "F6");
        assert_eq!(app.mode.viewer().expect("open").top, 18);
    }

    #[test]
    fn ctrl_f10_brings_the_panel_to_what_is_being_viewed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"first\n").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"second\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "+");
        let viewed = app.mode.viewer().expect("open").path.display_name();
        // Move the panel away, then ask it to catch up.
        app.panel_mut().cursor_to(0);
        app.run(Command::ViewerToPanel);
        assert_eq!(app.panel().current().unwrap().name_str(), viewed);
        assert!(app.status.contains(&viewed), "{}", app.status);
    }

    // ── replace across files ────────────────────────────────────────────────────────────────

    #[test]
    fn the_replace_dialog_counts_before_it_writes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"alpha beta alpha\n").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"nothing here\n").unwrap();
        let mut app = app_on_disk(dir.path());

        // Select both files, then open the dialog.
        let listing = app.panel().listing.clone();
        app.panel_mut()
            .selection
            .select_all(&listing, scmd_core::MaskScope::FilesOnly);
        app.run(Command::ToolReplace);
        let Mode::Form(mut form) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("the replace dialog did not open")
        };
        assert_eq!(form.kind, crate::menu::FormKind::Replace);
        for field in &mut form.fields {
            if let crate::form::Field::Text {
                key, value, caret, ..
            } = field
            {
                match *key {
                    "replace.needle" => *value = "alpha".to_string(),
                    "replace.with" => *value = "omega".to_string(),
                    _ => {}
                }
                *caret = value.chars().count();
            }
        }
        app.mode = Mode::Form(form);
        press(&mut app, "F10");

        // The first job out is the count, and it changes nothing.
        let (id, spec) = match app.take_requests().into_iter().next() {
            Some(Request::Job { id, spec, .. }) => (id, spec),
            other => panic!("{other:?}"),
        };
        assert_eq!(spec.kind, scmd_jobs::JobKind::Replace);
        assert!(spec.options.dry_run, "the first run must not write");
        assert_eq!(spec.sources.len(), 2);

        // What the count found comes back, and the program asks before writing.
        let counted = scmd_jobs::Outcome {
            files: 1,
            replacements: vec![(scmd_core::VPath::local(dir.path().join("one.txt")), 2)],
            ..Default::default()
        };
        app.handle_event(Event::JobDone {
            id,
            kind: scmd_jobs::JobKind::Replace,
            outcome: counted,
        });
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        let text = confirm.lines.join(" ");
        assert!(text.contains('2'), "{text}");
        assert!(text.contains("one.txt"), "{text}");
        // And saying yes runs the same thing for real.
        let real = match &confirm.action {
            ConfirmAction::Job(spec) => spec.clone(),
            other => panic!("{other:?}"),
        };
        assert!(!real.options.dry_run, "the second run has to write");
        assert_eq!(real.replace.as_ref().unwrap().needle, "alpha");
        assert_eq!(real.replace.as_ref().unwrap().replacement, "omega");
        // A rewritten file should look modified.
        assert!(!real.options.preserve.mtime);
    }

    #[test]
    fn a_count_that_finds_nothing_asks_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"nothing here\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        app.run(Command::ToolReplace);
        let Mode::Form(mut form) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("no dialog")
        };
        for field in &mut form.fields {
            if let crate::form::Field::Text {
                key, value, caret, ..
            } = field
            {
                if *key == "replace.needle" {
                    *value = "alpha".to_string();
                    *caret = value.chars().count();
                }
            }
        }
        app.mode = Mode::Form(form);
        press(&mut app, "F10");
        let id = match app.take_requests().into_iter().next() {
            Some(Request::Job { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::JobDone {
            id,
            kind: scmd_jobs::JobKind::Replace,
            outcome: scmd_jobs::Outcome::default(),
        });
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert!(!app.status.is_empty());
    }

    #[test]
    fn a_replacement_with_nothing_to_look_for_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        app.run(Command::ToolReplace);
        press(&mut app, "F10");
        assert!(app.status_error, "{}", app.status);
        // The dialog stays open, with what was typed still in it.
        assert!(matches!(app.mode, Mode::Form(_)));
    }

    // ── find file (SPEC 06 §1) ──────────────────────────────────────────────────────────────

    #[test]
    fn the_find_dialog_opens_on_the_last_search() {
        let mut app = app();
        app.run(Command::ToolFind);
        let Mode::Form(form) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.kind, crate::menu::FormKind::Find);
        // Every file, which is what pressing Enter straight away should mean.
        assert_eq!(form.text_value("find.mask"), "*");

        // What was searched for last comes back next time.
        app.last_find = FindSpec {
            mask: "*.rs".into(),
            text: "TODO".into(),
            case_sensitive: true,
            scope: FindScope::Panel,
            ..FindSpec::default()
        };
        app.run(Command::ToolFind);
        let Mode::Form(form) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("no dialog")
        };
        assert_eq!(form.text_value("find.mask"), "*.rs");
        assert_eq!(form.text_value("find.text"), "TODO");
        assert!(form.toggle_value("find.case"));
        assert_eq!(form.choice_value("find.scope"), "panel");
    }

    #[test]
    fn the_scope_decides_where_the_walk_starts() {
        let mut app = app();
        // The panel is on /home/mb, the other on /tmp.
        assert_eq!(
            app.find_roots(FindScope::Tree),
            vec![VPath::local("/home/mb")]
        );
        assert_eq!(
            app.find_roots(FindScope::Panel),
            vec![VPath::local("/home/mb")]
        );
        assert_eq!(app.find_roots(FindScope::Other), vec![VPath::local("/tmp")]);
        assert_eq!(
            app.find_roots(FindScope::Root),
            vec![app.panel().cwd.root()]
        );

        // With nothing selected, the folders scope takes the directory under the cursor.
        press(&mut app, "Down"); // docs
        assert_eq!(app.panel().current().unwrap().name_str(), "docs");
        assert_eq!(
            app.find_roots(FindScope::Selection),
            vec![VPath::local("/home/mb/docs")]
        );
        // And a file under the cursor is not a place to search, so it says nothing rather
        // than quietly searching the whole tree.
        press(&mut app, "Down"); // a.txt
        assert!(app.find_roots(FindScope::Selection).is_empty());
    }

    #[test]
    fn a_search_starts_from_the_dialog_and_can_be_stopped_without_losing_what_it_found() {
        let mut app = app();
        app.run(Command::ToolFind);
        let Mode::Form(mut form) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("no dialog")
        };
        // Type a mask and a text, then OK.
        for field in &mut form.fields {
            if let crate::form::Field::Text {
                key, value, caret, ..
            } = field
            {
                if *key == "find.text" {
                    *value = "needle".to_string();
                    *caret = value.chars().count();
                }
            }
        }
        app.mode = Mode::Form(form);
        press(&mut app, "F10");

        let (id, cancel) = match &app.mode {
            Mode::Find(view) => {
                assert!(view.label.contains("needle"), "{}", view.label);
                (view.id, view.cancel.clone())
            }
            other => panic!("{other:?}"),
        };
        match app.take_requests().into_iter().next() {
            Some(Request::Find { spec, .. }) => {
                assert_eq!(spec.text, "needle");
                assert_eq!(spec.mask, "*");
                assert_eq!(spec.roots, vec![VPath::local("/home/mb")]);
            }
            other => panic!("{other:?}"),
        }

        // Two hits arrive while it runs, and the list shows them.
        for name in ["one.rs", "two.rs"] {
            app.handle_event(Event::FindHit {
                id,
                path: VPath::local(format!("/home/mb/{name}")),
                entry: Box::new(entry(name, scmd_core::EntryKind::File, 10)),
            });
        }
        assert!(matches!(&app.mode, Mode::Find(v) if v.rows.len() == 2));

        // Esc stops the search and keeps the results; a second Esc closes the list.
        press(&mut app, "Esc");
        assert!(cancel.is_cancelled(), "the search was not stopped");
        assert!(matches!(&app.mode, Mode::Find(v) if v.rows.len() == 2 && v.stopped));
        app.handle_event(Event::FindDone {
            id,
            files: 7,
            dirs: 2,
            elapsed: std::time::Duration::from_millis(120),
            problems: vec![],
            error: None,
        });
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Normal));
    }

    #[test]
    fn f5_turns_the_results_into_the_panel() {
        let mut app = app();
        app.run(Command::ToolFind);
        press(&mut app, "F10");
        let id = match &app.mode {
            Mode::Find(view) => view.id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::FindHit {
            id,
            path: VPath::local("/home/mb/deep/one.rs"),
            entry: Box::new(entry("one.rs", scmd_core::EntryKind::File, 10)),
        });
        app.handle_event(Event::FindDone {
            id,
            files: 1,
            dirs: 1,
            elapsed: std::time::Duration::from_millis(5),
            problems: vec![],
            error: None,
        });
        press(&mut app, "F5");
        assert!(matches!(app.mode, Mode::Normal));
        // The panel is now the results, and the row knows where it really lives.
        assert!(app.panel().temp.is_some(), "the panel is not a list");
        assert_eq!(app.panel().listing.len(), 1);
        assert_eq!(
            app.panel().current_path().unwrap(),
            VPath::local("/home/mb/deep/one.rs")
        );
    }

    #[test]
    fn enter_on_a_result_goes_to_the_file() {
        let mut app = app();
        app.run(Command::ToolFind);
        press(&mut app, "F10");
        let id = match &app.mode {
            Mode::Find(view) => view.id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::FindHit {
            id,
            path: VPath::local("/home/mb/deep/one.rs"),
            entry: Box::new(entry("one.rs", scmd_core::EntryKind::File, 10)),
        });
        press(&mut app, "Enter");
        assert!(matches!(app.mode, Mode::Normal));
        assert_eq!(app.panel().cwd, VPath::local("/home/mb/deep"));
        assert_eq!(app.panel().want_cursor.as_deref(), Some("one.rs"));
    }

    // ── a panel that is a list rather than a directory ──────────────────────────────────────

    #[test]
    fn a_list_panel_knows_where_its_rows_really_live() {
        let dir = tempfile::tempdir().unwrap();
        let one = dir.path().join("a/one.txt");
        let two = dir.path().join("b/two.txt");
        std::fs::create_dir_all(one.parent().unwrap()).unwrap();
        std::fs::create_dir_all(two.parent().unwrap()).unwrap();
        std::fs::write(&one, b"1").unwrap();
        std::fs::write(&two, b"22").unwrap();

        let mut app = app();
        let items = vec![
            (
                VPath::local(&two),
                entry("two.txt", scmd_core::EntryKind::File, 2),
            ),
            (
                VPath::local(&one),
                entry("one.txt", scmd_core::EntryKind::File, 1),
            ),
        ];
        app.handle_event(Event::TempListed {
            panel: 0,
            generation: 2,
            title: "found 2".into(),
            items,
        });

        // Sorting reorders the rows, and the paths must follow them rather than their position.
        assert_eq!(app.panel().listing.len(), 2);
        // Sorting by size reorders the two rows (choosing the order already in force would
        // reverse it instead, which is FAR's behaviour and not what this test is about).
        app.run(Command::PanelSortBy(scmd_core::SortKey::Size));
        app.handle_input(Input::Key("Home".parse().unwrap()));
        assert_eq!(app.panel().current().unwrap().name_str(), "one.txt");
        assert_eq!(app.panel().current_path().unwrap().as_path(), one);
        app.handle_input(Input::Key("Down".parse().unwrap()));
        assert_eq!(app.panel().current_path().unwrap().as_path(), two);

        // An operation works on the real file, not on the panel's own directory joined to a name.
        app.run(Command::FileCopyCurrent);
        let Mode::Prompt(prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("no prompt")
        };
        assert_eq!(prompt.sources, vec![VPath::local(&two)]);

        // `Enter` on a result goes to the directory it lives in.
        let target = app.panel().enter_target().expect("somewhere to go");
        assert_eq!(target.as_path(), two.parent().unwrap());
    }

    #[test]
    fn re_reading_a_list_panel_drops_what_is_no_longer_there() {
        let dir = tempfile::tempdir().unwrap();
        let gone = dir.path().join("gone.txt");
        let stays = dir.path().join("stays.txt");
        std::fs::write(&gone, b"x").unwrap();
        std::fs::write(&stays, b"y").unwrap();

        let mut app = app();
        app.handle_event(Event::TempListed {
            panel: 0,
            generation: 2,
            title: "found 2".into(),
            items: vec![
                (
                    VPath::local(&gone),
                    entry("gone.txt", scmd_core::EntryKind::File, 1),
                ),
                (
                    VPath::local(&stays),
                    entry("stays.txt", scmd_core::EntryKind::File, 1),
                ),
            ],
        });
        assert_eq!(app.panel().listing.len(), 2);

        std::fs::remove_file(&gone).unwrap();
        app.requests.clear();
        app.run(Command::PanelReread);
        // A list panel re-reads by asking about its own paths, not by listing a directory.
        match app.take_requests().into_iter().next() {
            Some(Request::TempRefresh { paths, title, .. }) => {
                assert_eq!(paths.len(), 2);
                assert_eq!(title, "found 2");
            }
            other => panic!("{other:?}"),
        }
    }

    // ── selection and the keys that ignore it ───────────────────────────────────────────────

    #[test]
    fn the_modifier_picks_the_criterion_not_just_the_action() {
        let mut app = app();
        // The cursor on `b.rs`, so "the same extension" means `.rs`.
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "b.rs");
        app.run(Command::SelExtAdd);
        assert_eq!(app.panel().selection.len(), 2, "both .rs files");
        // The name criterion, from a file whose stem nothing else shares.
        app.run(Command::SelNone);
        app.run(Command::SelNameAdd);
        assert_eq!(app.panel().selection.len(), 1);
        // Inverting without folders leaves `docs` alone; the Alt variant takes it too.
        app.run(Command::SelNone);
        app.run(Command::SelInvert);
        assert_eq!(app.panel().selection.len(), 3, "the three files");
        app.run(Command::SelNone);
        app.run(Command::SelInvertAll);
        assert_eq!(app.panel().selection.len(), 4, "the folder as well");
        // And one restore undoes a whole sweep.
        app.run(Command::SelRestore);
        assert!(app.panel().selection.is_empty());
    }

    #[test]
    fn shift_f8_deletes_the_file_under_the_cursor_and_not_the_selection() {
        let mut app = app();
        // Select `a.txt` by its extension, then move the cursor onto `b.rs`.
        press(&mut app, "Down");
        press(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "a.txt");
        app.run(Command::SelExtAdd);
        assert_eq!(app.panel().selection.len(), 1, "a.txt is selected");
        press(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "b.rs");

        app.run(Command::FileDeleteCurrent);
        let spec = match &app.mode {
            Mode::Confirm(c) => match &c.action {
                ConfirmAction::Job(spec) => spec.clone(),
                other => panic!("{other:?}"),
            },
            other => panic!("{other:?}"),
        };
        assert_eq!(spec.sources.len(), 1, "{:?}", spec.sources);
        assert!(
            spec.sources[0].to_string().ends_with("b.rs"),
            "{:?}",
            spec.sources
        );
    }

    #[test]
    fn shift_f5_carries_its_own_list_so_a_later_selection_cannot_change_it() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Down"); // b.rs
        app.run(Command::FileCopyCurrent);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(prompt.kind, PromptKind::CopyTarget);
        assert_eq!(prompt.sources.len(), 1);
        // Selecting everything while the prompt is open must not widen the copy.
        let listing = app.panel().listing.clone();
        app.panel_mut()
            .selection
            .select_all(&listing, scmd_core::MaskScope::FilesAndDirs);
        let Mode::Prompt(prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            unreachable!()
        };
        app.prompt_accept(prompt);
        let spec = match app.take_requests().into_iter().next() {
            Some(Request::Job { spec, .. }) => spec,
            other => panic!("{other:?}"),
        };
        assert_eq!(spec.sources.len(), 1, "{:?}", spec.sources);
    }

    #[test]
    fn ctrl_g_runs_one_command_per_file_in_turn() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Down");
        app.run(Command::SelExtAdd);
        assert_eq!(app.panel().selection.len(), 2);

        app.run(Command::FileApply);
        let Mode::Prompt(mut prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("the apply prompt did not open")
        };
        assert_eq!(prompt.kind, PromptKind::ApplyCommand);
        prompt.text = "wc -l !.!".to_string();
        app.prompt_accept(prompt);

        // The first command is out; the second waits for it.
        let first = shell_requests(&app);
        assert_eq!(first.len(), 1, "{first:?}");
        assert!(first[0].starts_with("wc -l "), "{first:?}");
        assert_eq!(app.apply_queue.len(), 1);

        let (id, line) = app.shell.clone().expect("a command is running");
        app.handle_event(Event::ShellDone {
            id,
            line,
            output: "12 b.rs\n".into(),
            status: Some(0),
        });
        let all = shell_requests(&app);
        assert_eq!(all.len(), 2, "the second did not follow: {all:?}");
        assert_ne!(all[0], all[1], "the same file twice");
        assert!(app.apply_queue.is_empty());
    }

    #[test]
    fn a_failure_stops_the_rest_of_an_apply_run() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "Down");
        app.run(Command::SelExtAdd);
        app.run(Command::FileApply);
        let Mode::Prompt(mut prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("the apply prompt did not open")
        };
        prompt.text = "false !.!".to_string();
        app.prompt_accept(prompt);
        let (id, line) = app.shell.clone().expect("running");
        app.handle_event(Event::ShellDone {
            id,
            line,
            output: String::new(),
            status: Some(1),
        });
        assert!(app.apply_queue.is_empty(), "it carried on after a failure");
        assert!(app.status_error, "{}", app.status);
        assert_eq!(shell_requests(&app).len(), 1);
    }

    // ── the command line (SPEC 06 §7) ──────────────────────────────────────────────────────

    /// Type text the way a terminal delivers it, rather than through the key parser, which
    /// lowercases a bare letter because that is what a *binding* means.
    fn typed(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_input(Input::Key(Key::new(KeyCode::Char(c), Mods::NONE)));
        }
    }

    fn shell_requests(app: &App) -> Vec<String> {
        app.requests
            .iter()
            .filter_map(|r| match r {
                Request::Shell { line, .. } => Some(line.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn typing_lands_on_the_command_line_and_enter_runs_it() {
        let mut app = app();
        typed(&mut app, "echo hi");
        assert_eq!(app.cmd.text, "echo hi");
        // A letter must not have moved the cursor or started a quick search.
        assert!(app.mode.is_normal());
        press(&mut app, "Enter");
        assert_eq!(shell_requests(&app), ["echo hi"]);
        assert!(app.cmd.is_empty(), "the line is cleared once it runs");
        assert_eq!(app.cmd.history(), ["echo hi"], "and remembered");
    }

    #[test]
    fn the_shared_keys_belong_to_the_line_only_while_it_has_something_on_it() {
        let mut app = app();
        let before = app.panel().cwd.clone();
        // Empty line: Backspace is the parent directory, as it always was.
        press(&mut app, "Backspace");
        assert_ne!(app.panel().cwd, before, "Backspace went to the parent");

        typed(&mut app, "ls");
        press(&mut app, "Backspace");
        assert_eq!(app.cmd.text, "l", "now Backspace edits the line");
        press(&mut app, "Esc");
        assert!(app.cmd.is_empty(), "Esc clears the line");
        // And Enter on an empty line is the panel's again.
        let cwd = app.panel().cwd.clone();
        press(&mut app, "Enter");
        assert!(
            app.panel().cwd != cwd || app.mode.is_normal(),
            "Enter with an empty line is the panel's"
        );
    }

    #[test]
    fn cd_moves_the_panel_and_never_reaches_a_shell() {
        let dir = std::env::temp_dir();
        let mut app = app();
        typed(&mut app, &format!("cd {}", dir.display()));
        press(&mut app, "Enter");
        assert!(
            shell_requests(&app).is_empty(),
            "cd is ours, not the shell's"
        );
        assert_eq!(
            app.panel().cwd.as_path(),
            dir.canonicalize()
                .unwrap_or(dir.clone())
                .as_path()
                .to_path_buf()
                .as_path()
                .parent()
                .map(|_| dir.as_path())
                .unwrap_or(dir.as_path())
        );
    }

    #[test]
    fn cd_to_nowhere_says_so_instead_of_moving() {
        let mut app = app();
        let before = app.panel().cwd.clone();
        typed(&mut app, "cd /no/such/place/at/all");
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
        assert_eq!(app.panel().cwd, before);
    }

    #[test]
    fn a_condition_that_does_not_hold_runs_nothing() {
        let mut app = app();
        typed(&mut app, "if exist no-such-file make");
        press(&mut app, "Enter");
        assert!(shell_requests(&app).is_empty());
        assert!(app.status.contains("make"), "{}", app.status);
    }

    #[test]
    fn the_history_is_walked_and_the_half_typed_line_comes_back() {
        let mut app = app();
        typed(&mut app, "make");
        press(&mut app, "Enter");
        typed(&mut app, "half");
        press(&mut app, "Ctrl+E");
        assert_eq!(app.cmd.text, "make");
        press(&mut app, "Ctrl+X");
        assert_eq!(app.cmd.text, "half");
        press(&mut app, "Ctrl+Y");
        assert!(app.cmd.is_empty());
    }

    #[test]
    fn the_history_list_puts_a_line_back_to_be_edited_rather_than_running_it() {
        let mut app = app();
        typed(&mut app, "cargo test -p scmd");
        press(&mut app, "Enter");
        app.requests.clear();
        press(&mut app, "Alt+F8");
        let Mode::Pick(pick) = &app.mode else {
            panic!("no history list")
        };
        assert_eq!(pick.rows[0].value, "cargo test -p scmd");
        press(&mut app, "Enter");
        assert_eq!(app.cmd.text, "cargo test -p scmd");
        assert!(shell_requests(&app).is_empty(), "it must not run by itself");
    }

    #[test]
    fn ctrl_j_and_ctrl_bracket_put_names_and_paths_on_the_line() {
        let mut app = app();
        press(&mut app, "Down"); // onto `docs`
        press(&mut app, "Ctrl+Enter");
        assert_eq!(app.cmd.text, "docs ");
        app.cmd.clear();
        press(&mut app, "Ctrl+[");
        assert!(app.cmd.text.starts_with("/home/mb"), "{}", app.cmd.text);
        app.cmd.clear();
        press(&mut app, "Ctrl+]");
        assert!(app.cmd.text.starts_with("/tmp"), "{}", app.cmd.text);
    }

    #[test]
    fn a_substitution_is_expanded_before_the_shell_sees_it() {
        let mut app = app();
        press(&mut app, "Down"); // `docs`
        typed(&mut app, "wc -l {name}");
        press(&mut app, "Enter");
        assert_eq!(shell_requests(&app), ["wc -l docs"]);
    }

    #[test]
    fn a_line_that_asks_opens_a_prompt_and_runs_with_the_answer() {
        let mut app = app();
        typed(&mut app, "grep {ask:Search for} .");
        press(&mut app, "Enter");
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("no prompt: {:?}", app.mode)
        };
        assert_eq!(prompt.title, "Search for");
        assert!(
            shell_requests(&app).is_empty(),
            "nothing runs before the answer"
        );
        typed(&mut app, "TODO");
        press(&mut app, "Enter");
        assert_eq!(shell_requests(&app), ["grep TODO ."]);
    }

    #[test]
    fn the_view_prefix_opens_the_viewer_on_a_file() {
        let dir = std::env::temp_dir().join(format!("scmd-cmd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("hello.txt");
        std::fs::write(&file, b"hello\n").unwrap();
        let mut app = app();
        typed(&mut app, &format!("view:{}", file.display()));
        press(&mut app, "Enter");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn ctrl_o_shows_what_a_command_printed_and_the_exit_code_is_not_hidden() {
        let mut app = app();
        typed(&mut app, "false");
        press(&mut app, "Enter");
        let id = match app.requests.iter().find_map(|r| match r {
            Request::Shell { id, .. } => Some(*id),
            _ => None,
        }) {
            Some(id) => id,
            None => panic!("no shell request"),
        };
        app.handle_event(Event::ShellDone {
            id,
            line: "false".into(),
            output: "nothing to say\n".into(),
            status: Some(1),
        });
        assert!(
            app.status_error,
            "a non-zero exit is not hidden: {}",
            app.status
        );
        press(&mut app, "Ctrl+O");
        assert!(matches!(app.mode, Mode::UserScreen { .. }));
        assert!(app.user_screen.iter().any(|l| l == "$ false"));
        assert!(app.user_screen.iter().any(|l| l == "nothing to say"));
        press(&mut app, "Esc");
        assert!(app.mode.is_normal());
    }

    #[test]
    fn two_commands_at_once_are_refused_rather_than_queued_silently() {
        let mut app = app();
        typed(&mut app, "sleep 10");
        press(&mut app, "Enter");
        typed(&mut app, "echo second");
        press(&mut app, "Enter");
        assert_eq!(shell_requests(&app), ["sleep 10"]);
        assert!(app.status_error, "{}", app.status);
    }

    #[test]
    fn f9_opens_the_menu_on_the_side_that_has_the_focus() {
        let mut app = app();
        press(&mut app, "F9");
        assert!(matches!(app.mode, Mode::Menu { menu: 0, .. }));
        press(&mut app, "Esc");
        assert!(app.mode.is_normal());

        app.active = 1;
        press(&mut app, "F9");
        assert!(matches!(app.mode, Mode::Menu { menu: 4, .. }));
    }

    #[test]
    fn the_menu_walks_sideways_and_skips_separators() {
        let mut app = app();
        press(&mut app, "F9");
        press(&mut app, "Right");
        let Mode::Menu { menu, item } = app.mode else {
            panic!("the menu closed")
        };
        assert_eq!(menu, 1);
        assert_eq!(item, 0);
        press(&mut app, "Left");
        press(&mut app, "Left");
        assert!(matches!(app.mode, Mode::Menu { menu: 4, .. }));

        // Down from the last entry before a separator must land on an entry, never on the line.
        press(&mut app, "Esc");
        press(&mut app, "F9");
        for _ in 0..40 {
            press(&mut app, "Down");
            let Mode::Menu { menu, item } = app.mode else {
                panic!("the menu closed")
            };
            assert!(
                matches!(menu::menus()[menu].items[item], menu::Entry::Item(_)),
                "the cursor stopped on a separator"
            );
        }
    }

    #[test]
    fn a_menu_entry_runs_its_command_on_its_own_panel() {
        let mut app = app();
        app.active = 0;
        // The Right menu acts on the right panel even when the left one has the focus.
        open(&mut app, 4, "menu-brief");
        assert_eq!(app.panels[1].view, ViewMode::Brief);
        assert_eq!(app.panels[0].view, ViewMode::Full);
        assert_eq!(app.active, 1);
    }

    #[test]
    fn an_entry_with_no_feature_behind_it_says_so_instead_of_pretending() {
        let mut app = app();
        // The archive commands menu is the one archive entry still to come.
        open(&mut app, 1, "menu-archive-commands");
        assert!(
            app.status.contains("archive.commands"),
            "a missing feature must say so instead of doing nothing quietly: `{}`",
            app.status
        );
        assert!(app.mode.is_normal());
    }

    #[test]
    fn the_system_dialog_writes_through_to_the_configuration() {
        let mut app = app();
        app.cfg.panel.natural_sort = false;
        open(&mut app, 3, "menu-system-settings");
        let Mode::Form(form) = &app.mode else {
            panic!("the dialog did not open")
        };
        assert_eq!(form.kind, FormKind::System);

        press(&mut app, "Space"); // the first row: natural number order
        press(&mut app, "Tab"); // to OK
        press(&mut app, "Enter");
        assert!(app.mode.is_normal());
        assert!(app.cfg.panel.natural_sort, "the dialog was not applied");

        // Esc throws the change away.
        open(&mut app, 3, "menu-system-settings");
        press(&mut app, "Space");
        press(&mut app, "Esc");
        assert!(app.cfg.panel.natural_sort);
    }

    #[test]
    fn the_panel_dialog_takes_effect_at_once() {
        let mut app = app();
        assert!(!app.panels[0].show_hidden);
        open(&mut app, 3, "menu-panel-settings");
        press(&mut app, "Space"); // show hidden files
        press(&mut app, "F10"); // the shortcut for OK
        assert!(app.cfg.panel.show_hidden);
        assert!(
            app.panels[0].show_hidden,
            "the panel did not pick up the new setting"
        );
    }

    #[test]
    fn an_unrelated_dialog_leaves_a_panels_own_sort_order_alone() {
        let mut app = app();
        press(&mut app, "Ctrl+F6"); // sort this panel by size, which the config does not say
        assert_eq!(app.panels[0].sort.key, SortKey::Size);
        open(&mut app, 3, "menu-system-settings");
        press(&mut app, "F10");
        assert_eq!(
            app.panels[0].sort.key,
            SortKey::Size,
            "a dialog that says nothing about sorting must not reset it"
        );
        // The panel dialog does own the sort order, so there it is applied.
        open(&mut app, 3, "menu-panel-settings");
        press(&mut app, "F10");
        assert_eq!(app.panels[0].sort.key, SortKey::Name);
    }

    #[test]
    fn the_language_list_switches_the_interface() {
        let mut app = app();
        open(&mut app, 3, "menu-languages");
        let Mode::Pick(pick) = &app.mode else {
            panic!("the list did not open")
        };
        let row = pick
            .rows
            .iter()
            .position(|r| r.value == "it")
            .expect("Italian is shipped");
        let Mode::Pick(pick) = &mut app.mode else {
            unreachable!()
        };
        pick.cursor = row;
        press(&mut app, "Enter");
        assert_eq!(app.i18n.tag(), "it");
        assert_eq!(app.cfg.ui.language, "it");
        assert_eq!(menu::menus()[1].title, "menu-files");
        assert_eq!(app.i18n.t("menu-files"), "File");
    }

    #[test]
    fn the_sort_list_turns_the_order_round_when_the_same_order_is_chosen_again() {
        let mut app = app();
        press(&mut app, "Ctrl+F12");
        let Mode::Pick(pick) = &app.mode else {
            panic!("the sort list did not open")
        };
        assert_eq!(pick.kind, PickKind::SortModes);
        assert!(
            pick.rows[pick.cursor].current,
            "the cursor starts on the order in force"
        );
        assert!(!app.panels[0].sort.reverse);
        press(&mut app, "Enter");
        assert!(
            app.panels[0].sort.reverse,
            "choosing it again did not reverse it"
        );

        // With reverse sorting switched off in the settings, it stays as it is.
        app.cfg.panel.allow_reverse_sort = false;
        press(&mut app, "Ctrl+F12");
        press(&mut app, "Enter");
        assert!(app.panels[0].sort.reverse);
    }

    #[test]
    fn the_folders_list_goes_back_to_a_folder_this_panel_has_been_in() {
        let mut app = app();
        // Compare whole paths, not substrings: a path is written with this platform's
        // separator, so `/home/mb` is not a string that has to appear on Windows.
        let came_from = app.panels[0].cwd.clone();
        app.goto(came_from.join("docs"));
        press(&mut app, "Alt+F12");
        let Mode::Pick(pick) = &app.mode else {
            panic!("the history did not open")
        };
        assert_eq!(pick.kind, PickKind::FoldersHistory);
        assert!(
            pick.rows
                .iter()
                .any(|r| VPath::parse(&r.value).normalised() == came_from.normalised()),
            "the folder we came from is not in the list: {:?}",
            pick.rows
        );

        // Walking back and forth must not fill the list with the same two folders.
        for _ in 0..3 {
            app.goto(came_from.join("docs"));
            app.goto(came_from.clone());
        }
        press(&mut app, "Esc");
        press(&mut app, "Alt+F12");
        let Mode::Pick(pick) = &app.mode else {
            panic!("the history did not open")
        };
        let mut paths: Vec<&String> = pick.rows.iter().map(|r| &r.value).collect();
        let before = paths.len();
        paths.sort();
        paths.dedup();
        assert_eq!(before, paths.len(), "the history repeats itself: {paths:?}");
    }

    #[test]
    fn save_setup_writes_a_configuration_file_that_can_be_read_back() {
        let dir = std::env::temp_dir().join(format!("scmd-setup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("scmd.toml");
        let mut app = app();
        app.paths.config_file = file.clone();
        app.cfg.panel.show_hidden = true;
        press(&mut app, "Shift+F9");
        assert!(!app.status_error, "{}", app.status);
        let text = std::fs::read_to_string(&file).unwrap();
        let (back, problems) =
            Config::parse("scmd.toml", &text).expect("what we write, we can read");
        assert!(problems.is_empty(), "{problems:?}");
        assert!(back.panel.show_hidden);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn every_menu_entry_either_runs_something_or_admits_it_cannot() {
        let mut app = app();
        for (index, m) in menu::menus().iter().enumerate() {
            for entry in &m.items {
                let menu::Entry::Item(item) = entry else {
                    continue;
                };
                if matches!(item.action, Action::Run(Command::AppQuit)) {
                    continue;
                }
                app.mode = Mode::Normal;
                app.status.clear();
                app.status_error = false;
                let row = menu_row(index, item.label);
                app.mode = Mode::Menu {
                    menu: index,
                    item: row,
                };
                press(&mut app, "Enter");
                if let Action::Todo(what) = item.action {
                    assert!(
                        app.status.contains(what),
                        "{}: expected a `not here yet` for {what}, got `{}`",
                        item.label,
                        app.status
                    );
                }
                assert!(!app.quit, "{} quit the program", item.label);
            }
        }
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
    fn the_calculator_answers_in_every_base_and_keeps_the_last_sum() {
        let mut app = app();
        app.run(Command::ToolCalculator);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(prompt.kind, PromptKind::Calculator);

        for c in "0x400+0x1c".chars() {
            press(&mut app, &c.to_string());
        }
        press(&mut app, "Enter");
        let Mode::Message { lines, .. } = &app.mode else {
            panic!("{:?}", app.mode)
        };
        let text = lines.join("\n");
        assert!(text.contains("1052"), "{text}");
        assert!(text.contains("0x41C"), "{text}");
        assert!(text.contains("0b10000011100"), "{text}");
        assert!(text.contains("0o2034"), "{text}");

        // It opens again on the sum just worked out.
        press(&mut app, "Esc");
        app.run(Command::ToolCalculator);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(prompt.text, "0x400+0x1c");

        // Something that is not arithmetic says so and keeps what was typed.
        press(&mut app, "End");
        for c in "/0".chars() {
            press(&mut app, &c.to_string());
        }
        press(&mut app, "Enter");
        assert!(app.status.contains("divide by zero"), "{}", app.status);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(prompt.text.contains("/0"), "the line should still be there");
    }

    /// A macro is recorded in a temporary configuration directory, so the test writes a real
    /// `scmd-macros.toml` and reads it back — which is the half of the feature that a unit
    /// test of the type cannot reach.
    fn app_with_macros(dir: &std::path::Path) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local("/home/mb"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
        });
        app.requests.clear();
        let entries = vec![
            Entry::parent_entry(),
            entry("a.txt", EntryKind::File, 100),
            entry("b.rs", EntryKind::File, 200),
            entry("c.rs", EntryKind::File, 300),
        ];
        for side in [0, 1] {
            app.handle_event(Event::Listed {
                panel: side,
                generation: app.panels[side].generation,
                result: Ok(entries.clone()),
            });
        }
        app.requests.clear();
        app
    }

    #[test]
    fn a_macro_is_recorded_as_commands_assigned_to_a_key_and_played_back() {
        use scmd_cmd::{Play, Step};
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with_macros(dir.path());

        // Ctrl+. starts recording, and says so.
        press(&mut app, "Ctrl+.");
        assert!(app.recording.is_some());
        assert!(app.status.contains("Recording"), "{}", app.status);

        // Three keys: two that are commands, and one the panel consumed as a keystroke.
        press(&mut app, "Ctrl+H"); // panel.hidden.toggle
        press(&mut app, "Down"); // nav.down
        let steps = app.recording.clone().unwrap();
        assert_eq!(steps.len(), 2, "{steps:?}");
        assert!(
            matches!(steps[0], Step::Run(cmd) if cmd.id() == "panel.hidden.toggle"),
            "{steps:?}"
        );

        // Ctrl+. again asks for the key, and the recording keys are not in the recording.
        press(&mut app, "Ctrl+.");
        let Mode::MacroKey { steps } = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(steps.len(), 2);

        // The key is pressed rather than typed; then a description, which may be empty.
        press(&mut app, "Alt+9");
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(matches!(prompt.kind, PromptKind::MacroDescription { .. }));
        for c in "hide-and-step".chars() {
            press(&mut app, &c.to_string());
        }
        press(&mut app, "Enter");
        assert!(app.recording.is_none());
        assert_eq!(app.macros.all.len(), 1);
        let saved = &app.macros.all[0];
        assert_eq!(saved.key.to_string(), "Alt+9");
        assert_eq!(saved.description, "hide-and-step");
        assert_eq!(saved.play, Play::Once);
        assert!(!saved.trusted, "recording never trusts a macro");

        // It is on disk, and in the shape the file documents.
        let written = std::fs::read_to_string(dir.path().join("scmd-macros.toml")).unwrap();
        assert!(written.contains("key = \"Alt+9\""), "{written}");
        assert!(written.contains("panel.hidden.toggle"), "{written}");

        // Playing it: the key now does what was recorded, twice over.
        let hidden_before = app.panels[app.active].show_hidden;
        let cursor_before = app.panels[app.active].cursor;
        press(&mut app, "Alt+9");
        assert_ne!(app.panels[app.active].show_hidden, hidden_before);
        assert_eq!(app.panels[app.active].cursor, cursor_before + 1);

        // And a fresh application finds it again.
        let again = app_with_macros(dir.path());
        assert_eq!(again.macros.all.len(), 1);
        assert_eq!(again.macros.all[0].description, "hide-and-step");
    }

    #[test]
    fn a_macro_that_deletes_asks_before_it_plays_unless_the_file_trusts_it() {
        use scmd_cmd::{Macro, Play, Scope, Step};
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with_macros(dir.path());
        let key = "Alt+8".parse::<Key>().unwrap();
        let deleting = Macro {
            key,
            scope: Scope::Panel,
            steps: vec![Step::Run(Command::parse("file.delete").unwrap())],
            play: Play::Once,
            description: "the-one-that-bites".into(),
            trusted: false,
        };
        app.macros.set(deleting.clone());

        // The key does not delete: it asks, and names what it would run.
        press(&mut app, "Alt+8");
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(matches!(confirm.action, ConfirmAction::PlayMacro(_)));
        assert!(
            confirm.lines.iter().any(|l| l.contains("file.delete")),
            "{:?}",
            confirm.lines
        );
        press(&mut app, "Esc");
        assert!(app.mode.is_normal());

        // A macro the file trusts plays without the question.
        app.macros.set(Macro {
            trusted: true,
            ..deleting
        });
        press(&mut app, "Alt+8");
        assert!(
            !matches!(app.mode, Mode::Confirm(_)),
            "a trusted macro must not ask: {:?}",
            app.mode
        );
    }

    #[test]
    fn screens_keep_a_viewer_and_an_editor_open_behind_the_panels() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("notes.txt");
        let bin = dir.path().join("prog.bin");
        std::fs::write(&text, "one\ntwo\n").unwrap();
        std::fs::write(&bin, [0x55, 0x48, 0x89, 0xE5]).unwrap();
        let mut app = app();

        // Nothing open: the list still answers, with the panels alone.
        app.run(Command::AppScreens);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.rows.len(), 1);
        assert_eq!(pick.rows[0].value, "panels");
        press(&mut app, "Esc");

        // Open the viewer, then step back to the panels: it stays open behind them. The
        // panels are screen zero of the ring, so Ctrl+Tab from the only screen lands there.
        app.open_viewer_on(&text, VPath::local(&text));
        assert!(app.mode.viewer().is_some());
        app.run(Command::AppScreenNext);
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        assert_eq!(app.background.len(), 1);

        // And the hex editor on top of that: Ctrl+Tab steps out to the panels again, and
        // both screens are now behind them.
        app.open_hex(&bin);
        app.run(Command::AppScreenNext);
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        assert_eq!(app.background.len(), 2, "both are behind the panels now");

        // The list shows both, the panels first and the newest screen last.
        app.run(Command::AppScreens);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        let labels: Vec<&str> = pick.rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels.len(), 3, "{labels:?}");
        assert!(labels[1].contains("prog.bin"), "{labels:?}");
        assert!(labels[2].contains("notes.txt"), "{labels:?}");

        // Choose the viewer: it comes forward and the panels are no longer in front.
        let at = pick
            .rows
            .iter()
            .position(|r| r.label.contains("notes"))
            .unwrap();
        let Mode::Pick(pick) = &mut app.mode else {
            unreachable!()
        };
        pick.cursor = at;
        press(&mut app, "Enter");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
        assert_eq!(app.background.len(), 1, "the hex editor is still behind");

        // Ctrl+Tab from a screen steps out to the panels, and again steps in to the next
        // one behind them: every screen is reachable, and the panels are one key away.
        app.run(Command::AppScreenNext);
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        app.run(Command::AppScreenNext);
        assert!(app.mode.hex().is_some(), "{:?}", app.mode);
        // Backwards from the panels is the screen one stepped away from last.
        app.run(Command::AppScreenNext);
        assert!(app.mode.is_normal());
        app.run(Command::AppScreenPrev);
        assert!(app.mode.hex().is_some(), "{:?}", app.mode);
    }

    #[test]
    fn quitting_asks_about_a_screen_left_with_unsaved_changes() {
        let dir = tempfile::tempdir().unwrap();
        let bin = dir.path().join("prog.bin");
        std::fs::write(&bin, [0x55, 0x48, 0x89, 0xE5]).unwrap();
        let mut app = app();

        // Change a byte, then step away from the hex editor without saving.
        app.open_hex(&bin);
        app.mode.hex_mut().unwrap().set_byte(0, 0xC3);
        app.run(Command::AppScreenNext);
        assert_eq!(app.background.len(), 1);
        assert!(app.background[0].screen_modified());

        // F10 from the panels does not end the program silently: it names the screen.
        app.run(Command::AppQuit);
        assert!(!app.quit, "quitting must ask first");
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(confirm.action, ConfirmAction::QuitWithScreens);
        assert!(
            confirm.lines.iter().any(|l| l.contains("prog.bin")),
            "{:?}",
            confirm.lines
        );

        // No leaves it alone; yes ends it.
        press(&mut app, "Esc");
        assert!(!app.quit);
        assert_eq!(app.background.len(), 1, "the screen is still there");
        app.run(Command::AppQuit);
        press(&mut app, "Enter");
        assert!(app.quit);
    }

    #[test]
    fn f9_assembles_one_instruction_over_the_bytes_at_the_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("patch.bin");
        // push rbp / mov rbp,rsp / xor eax,eax / ret, and room after it.
        std::fs::write(&path, [0x55, 0x48, 0x89, 0xE5, 0x31, 0xC0, 0xC3, 0x90]).unwrap();
        let mut app = app();
        app.open_hex(&path);

        // The dialog says where it is about to write, and for which processor.
        app.run(Command::EditorAssemble);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(prompt.kind, PromptKind::Assemble);
        assert!(prompt.title.contains("00000000"), "{}", prompt.title);

        // Make the function return straight away: one byte over the first one.
        press(&mut app, "Esc");
        app.assemble_at_cursor("ret");
        let editor = app.mode.hex_mut().expect("still in hex");
        assert_eq!(editor.read(0, 2), [0xC3, 0x48]);
        assert!(
            editor.is_changed(0),
            "the patched byte is marked as changed"
        );
        assert!(app.status.contains("C3"), "{}", app.status);

        // Undo puts it back, which is the whole point of the patch list.
        let editor = app.mode.hex_mut().unwrap();
        assert!(editor.undo());
        assert_eq!(editor.read(0, 1), [0x55]);

        // A jump is written as the address to arrive at, measured from the cursor.
        let editor = app.mode.hex_mut().unwrap();
        editor.cursor = 4;
        app.assemble_at_cursor("jmp 6");
        let editor = app.mode.hex_mut().unwrap();
        assert_eq!(editor.read(4, 2), [0xEB, 0x00]);

        // Something out of the subset leaves the file alone and says why.
        let before = app.mode.hex_mut().unwrap().read(0, 8);
        app.assemble_at_cursor("mov eax, [rbx]");
        assert!(
            app.status.to_lowercase().contains("memory"),
            "{}",
            app.status
        );
        assert_eq!(app.mode.hex_mut().unwrap().read(0, 8), before);

        // And F9 opens again on what was typed last.
        app.run(Command::EditorAssemble);
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(prompt.text, "mov eax, [rbx]");
    }

    #[test]
    fn a_range_for_a_block_operation_is_an_offset_and_a_length_in_hex() {
        assert_eq!(App::hex_range("10 4"), Some((0x10, 4)));
        assert_eq!(App::hex_range(" 0x1F,0x10 "), Some((0x1f, 0x10)));
        assert_eq!(App::hex_range("1f+10"), Some((0x1f, 0x10)));
        // One number is not a range, three are not either, and letters are not hex.
        assert_eq!(App::hex_range("10"), None);
        assert_eq!(App::hex_range("10 4 2"), None);
        assert_eq!(App::hex_range("ten four"), None);
        assert_eq!(App::hex_range(""), None);
    }

    #[test]
    fn a_block_is_copied_and_moved_from_elsewhere_in_the_same_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bytes.bin");
        std::fs::write(&path, b"ABCDEFGH").unwrap();
        let mut app = app();

        // Copy the two bytes at 6 (`GH`) to the start: the file grows by two.
        app.open_hex(&path);
        app.run(Command::EditorBlockOps);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(
            pick.rows.iter().any(|r| r.value == "copy")
                && pick.rows.iter().any(|r| r.value == "move"),
            "the block menu must offer both"
        );
        press(&mut app, "Esc"); // the menu puts the editor back where it was
        app.hex_operation(PromptKind::HexCopy, "6 2");
        let editor = app.mode.hex_mut().expect("still in hex");
        assert_eq!(editor.read(0, 10), b"GHABCDEFGH");

        // A range that is not in the file is refused, and nothing is written.
        app.hex_operation(PromptKind::HexCopy, "FF 2");
        assert!(
            app.status.to_lowercase().contains("outside"),
            "{}",
            app.status
        );
        assert_eq!(app.mode.hex_mut().unwrap().read(0, 10), b"GHABCDEFGH");

        // Move the two bytes back: the file is the length it started at, in the old order.
        let editor = app.mode.hex_mut().unwrap();
        editor.cursor = 10;
        editor.clear_block();
        app.hex_operation(PromptKind::HexMove, "0 2");
        let editor = app.mode.hex_mut().unwrap();
        assert_eq!(editor.len(), 10);
        assert_eq!(editor.read(0, 10), b"ABCDEFGHGH");

        // A move whose destination is inside its own source is refused, not half done.
        let editor = app.mode.hex_mut().unwrap();
        editor.cursor = 2;
        app.hex_operation(PromptKind::HexMove, "0 6");
        assert_eq!(app.mode.hex_mut().unwrap().read(0, 10), b"ABCDEFGHGH");
    }

    #[test]
    fn comparing_the_panels_selects_what_a_copy_would_have_to_carry() {
        use std::time::{Duration, SystemTime};
        let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let new = old + Duration::from_secs(3_600);
        let mut app = app();

        let stamped = |name: &str, size: u64, when: SystemTime| {
            let mut e = entry(name, EntryKind::File, size);
            e.mtime = Some(when);
            e
        };

        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir, 0),
                stamped("same.txt", 10, old),
                stamped("newer.txt", 10, new),
                stamped("older.txt", 10, old),
                stamped("only-here.txt", 10, old),
            ]),
        });
        app.handle_event(Event::Listed {
            panel: 1,
            generation: app.panels[1].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir, 0),
                stamped("same.txt", 10, old),
                stamped("newer.txt", 10, old),
                stamped("older.txt", 10, new),
                stamped("only-there.txt", 10, old),
            ]),
        });

        app.run(Command::ToolCompareDirs);

        let selected = |app: &App, side: usize| -> Vec<String> {
            let panel = &app.panels[side];
            let mut out = Vec::new();
            for row in 0..panel.listing.len() {
                let e = panel.listing.row(row).unwrap();
                if panel.selection.contains(e.id) {
                    out.push(e.name_str().into_owned());
                }
            }
            out.sort();
            out
        };
        // Each side keeps what the other does not have, and the version it has newer. The
        // file that is the same on both sides is selected in neither, and no directory is.
        assert_eq!(selected(&app, 0), ["newer.txt", "only-here.txt"]);
        assert_eq!(selected(&app, 1), ["older.txt", "only-there.txt"]);
        assert!(app.status.contains('2'), "{}", app.status);

        // Run again and nothing piles up: the comparison replaces the selection.
        app.run(Command::ToolCompareDirs);
        assert_eq!(selected(&app, 0), ["newer.txt", "only-here.txt"]);
    }

    #[test]
    fn left_and_right_move_by_a_column_where_there_is_one() {
        let mut app = app();
        app.rows = 2;
        app.panels[0].view = ViewMode::Brief;
        // Four rows, two to a column: `→` crosses to the second column.
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir, 0),
                entry("a.txt", EntryKind::File, 1),
                entry("b.txt", EntryKind::File, 2),
            ]),
        });
        app.panels[0].cursor_to(0);
        press(&mut app, "Right");
        assert_eq!(app.panels[0].cursor, 2);
        press(&mut app, "Left");
        assert_eq!(app.panels[0].cursor, 0);

        // In a one-column view they do nothing, as in FAR.
        app.panels[0].view = ViewMode::Full;
        press(&mut app, "Right");
        assert_eq!(app.panels[0].cursor, 0);
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
        press(&mut app, "Ctrl+Shift+L");
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
        assert_eq!(app.theme.id, "far");
        assert!(app.status.contains("FAR"), "{}", app.status);
    }

    #[test]
    fn the_help_is_a_book_whose_reference_chapters_are_generated() {
        use crate::help::Chapter;
        let mut app = app();

        // F1 opens the table of contents, not a wall of keys.
        press(&mut app, "F1");
        let Mode::Help { chapter, at, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Index);
        assert_eq!(at, 0);
        let index = app.help_lines(Chapter::Index, 76);
        assert_eq!(index.len(), crate::help::chapters().len());
        assert!(index[0].contains("What this is"), "{index:?}");

        // Down to the keys chapter and into it.
        let keys_at = crate::help::chapters()
            .iter()
            .position(|(c, _)| *c == Chapter::Keys)
            .unwrap();
        for _ in 0..keys_at {
            press(&mut app, "Down");
        }
        press(&mut app, "Enter");
        let Mode::Help { chapter, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Keys);
        let lines = app.help_lines(Chapter::Keys, 76);
        assert!(lines
            .iter()
            .any(|l| l.contains("F5") && l.contains("file.copy")));
        assert!(lines.iter().any(|l| l.contains("[global]")));

        // Esc out of a chapter is back to the index, on the chapter one was reading; Esc
        // again leaves the help.
        press(&mut app, "Esc");
        let Mode::Help { chapter, at, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Index);
        assert_eq!(at, keys_at, "the index remembers where one was");
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Normal));

        // The command chapter names every command, bound or not, and the settings chapter is
        // the configuration as this session has it.
        let commands = app.help_lines(Chapter::Commands, 76);
        assert!(commands.iter().any(|l| l.contains("file.copy")));
        assert!(
            commands.iter().any(|l| l.contains("editor.assemble")),
            "a command added later must appear without anyone writing it down"
        );
        let options = app.help_lines(Chapter::Options, 76);
        assert!(options.iter().any(|l| l.contains("keymap")));

        // And a written chapter is wrapped prose, with no Markdown left in it.
        let prose = app.help_lines(Chapter::Prose(0), 76);
        assert!(!prose.is_empty());
        assert!(
            !prose.iter().any(|l| l.contains('`') || l.contains("**")),
            "the marks must be stripped: {prose:?}"
        );
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
                // The prefill ends with the platform's separator, so typing a name appends.
                assert!(p.text.ends_with(std::path::MAIN_SEPARATOR), "{}", p.text);
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
                match &c.action {
                    ConfirmAction::Job(spec) => assert_eq!(spec.kind, JobKind::Delete),
                    other => panic!("expected a job confirmation, got {other:?}"),
                }
            }
            other => panic!("expected a confirmation, got {other:?}"),
        }
        assert!(app.take_requests().is_empty(), "deleted before confirming");
        press(&mut app, "Enter");
        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.sources, vec![VPath::local("/home/mb/a.txt")]);
    }

    #[test]
    fn the_delete_confirmation_says_it_cannot_be_undone() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "F8");
        match &app.mode {
            Mode::Confirm(c) => assert!(
                c.lines
                    .iter()
                    .any(|l| l.to_lowercase().contains("permanent")),
                "the user must be told there is no trash: {:?}",
                c.lines
            ),
            other => panic!("expected a confirmation, got {other:?}"),
        }
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
            paths: Paths::discover(Some(&dir.join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir),
            right: VPath::local(dir),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
        });
        app.requests.clear();
        let entries = scmd_vfs::local::list_blocking(&VPath::local(dir), &CancelToken::new())
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
        assert_eq!(viewer.mode, scmd_view::Mode::Text);

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
        assert_eq!(app.mode.viewer().unwrap().mode, scmd_view::Mode::Hex);
        press(&mut app, "F4");
        assert_eq!(app.mode.viewer().unwrap().mode, scmd_view::Mode::Text);

        press(&mut app, "F2");
        assert_eq!(app.mode.viewer().unwrap().wrap, scmd_view::Wrap::Char);
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
    fn f5_disassembles_a_real_executable_in_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        let exe = std::env::current_exe().unwrap();
        std::fs::copy(&exe, dir.path().join("program")).unwrap();
        let mut app = app_on_disk(dir.path());

        press(&mut app, "Down"); // program
        press(&mut app, "F3");
        assert_eq!(app.mode.viewer().unwrap().mode, scmd_view::Mode::Dump);

        press(&mut app, "F5");
        match app.mode.viewer().unwrap().mode {
            scmd_view::Mode::Disasm => {
                let viewer = app.mode.viewer_mut().unwrap();
                let rows = viewer.rows(100, 6);
                assert!(
                    rows.iter().any(|r| r.kind == scmd_view::RowKind::Code),
                    "{rows:?}"
                );
                // Either the file was parsed, or it was over the parse limit (a debug build
                // of this very test is ~100 MB) and we are sweeping raw bytes — the status
                // line says which, and never nothing.
                let summary = viewer.status().binary.expect("a summary either way");
                assert!(
                    viewer.binary.is_some() || summary.contains("raw"),
                    "{summary}"
                );
            }
            _ => {
                // A non-x86 runner: the message must say an external tool is needed.
                assert!(app.status_error, "{}", app.status);
                assert!(app.status.contains("external tool"), "{}", app.status);
            }
        }
    }

    #[test]
    fn f5_on_a_class_file_lists_its_bytecode_in_process() {
        let dir = tempfile::tempdir().unwrap();
        // A minimal class: `public class A` with no members.
        let mut class: Vec<u8> = Vec::new();
        class.extend_from_slice(&0xCAFE_BABEu32.to_be_bytes());
        class.extend_from_slice(&0u16.to_be_bytes()); // minor
        class.extend_from_slice(&52u16.to_be_bytes()); // Java 8
        class.extend_from_slice(&5u16.to_be_bytes()); // pool count (4 entries)
        class.push(1); // Utf8 "A"
        class.extend_from_slice(&1u16.to_be_bytes());
        class.push(b'A');
        class.push(7); // Class #1
        class.extend_from_slice(&1u16.to_be_bytes());
        class.push(1); // Utf8 "java/lang/Object"
        class.extend_from_slice(&16u16.to_be_bytes());
        class.extend_from_slice(b"java/lang/Object");
        class.push(7); // Class #3
        class.extend_from_slice(&3u16.to_be_bytes());
        class.extend_from_slice(&0x0021u16.to_be_bytes()); // public super
        class.extend_from_slice(&2u16.to_be_bytes()); // this_class
        class.extend_from_slice(&4u16.to_be_bytes()); // super_class
        class.extend_from_slice(&0u16.to_be_bytes()); // interfaces
        class.extend_from_slice(&0u16.to_be_bytes()); // fields
        class.extend_from_slice(&0u16.to_be_bytes()); // methods
        class.extend_from_slice(&0u16.to_be_bytes()); // attributes
        std::fs::write(dir.path().join("A.class"), &class).unwrap();

        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "F5");

        let viewer = app.mode.viewer_mut().expect("still open");
        assert_eq!(viewer.derived.as_deref(), Some("bytecode"));
        let text = viewer
            .rows(80, 6)
            .into_iter()
            .map(|r| r.text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Java 8"), "{text}");
        assert!(text.contains("public super class A"), "{text}");
        assert!(app.status.contains("bytecode"), "{}", app.status);
        assert!(
            app.take_requests().is_empty(),
            "no external tool was needed"
        );
    }

    #[test]
    fn f5_on_a_text_file_explains_itself_and_changes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"not a binary\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "F5");
        assert_eq!(app.mode.viewer().unwrap().mode, scmd_view::Mode::Text);
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("notes.txt"), "{}", app.status);
    }

    #[test]
    fn shift_f6_prints_a_wasm_module_as_text_without_an_external_tool() {
        let dir = tempfile::tempdir().unwrap();
        let wasm: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
        ];
        std::fs::write(dir.path().join("mod.wasm"), wasm).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "Shift+F6");

        let viewer = app.mode.viewer_mut().expect("still open");
        assert_eq!(viewer.derived.as_deref(), Some("WAT"));
        let rows: Vec<String> = viewer.rows(80, 6).into_iter().map(|r| r.text).collect();
        assert!(rows.iter().any(|r| r.contains("i32.const 42")), "{rows:?}");
        assert!(app.status.contains("WAT"), "{}", app.status);
        assert!(
            app.take_requests().is_empty(),
            "no external tool was needed"
        );
    }

    #[test]
    fn f6_asks_the_runner_to_run_a_configured_decompiler() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Main.class"),
            b"\xca\xfe\xba\xbe\x00\x00\x00\x34",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        // A tool that certainly exists, so the test does not depend on javap being installed.
        app.cfg.decompile = vec![DecompileRule {
            name: "cat".into(),
            mask: "*.class".into(),
            cmd: "cat {file}".into(),
        }];
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "Shift+F6");

        if !scmd_config::command_exists("cat") {
            return; // Windows runner
        }
        let request = app.take_requests().into_iter().next().expect("a request");
        let id = match request {
            Request::Decompile { id, rule, path } => {
                assert_eq!(rule.name, "cat");
                assert!(path.to_string().ends_with("Main.class"));
                id
            }
            other => panic!("expected a decompile request, got {other:?}"),
        };
        assert!(app.status.contains("cat"), "{}", app.status);

        app.handle_event(Event::Decompiled {
            id,
            tool: "cat".into(),
            result: Ok("public class Main {}\n".into()),
        });
        let viewer = app.mode.viewer_mut().expect("still open");
        assert_eq!(viewer.derived.as_deref(), Some("cat"));
        assert_eq!(
            viewer.rows(40, 1).first().map(|r| r.text.clone()),
            Some("public class Main {}".to_string())
        );
    }

    #[test]
    fn shift_f6_without_any_tool_says_what_to_install() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("thing.unknownext"), b"\x00\x01\x02\x03").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "Shift+F6");
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("decompile"), "{}", app.status);
        assert!(app.take_requests().is_empty());
    }

    #[test]
    fn a_decompiler_failure_is_reported_and_the_viewer_stays() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Main.class"), b"\xca\xfe\xba\xbe").unwrap();
        let mut app = app_on_disk(dir.path());
        app.cfg.decompile = vec![DecompileRule {
            name: "broken".into(),
            mask: "*.class".into(),
            cmd: "cat {file}".into(),
        }];
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "Shift+F6");
        if !scmd_config::command_exists("cat") {
            return;
        }
        let id = match app.take_requests().into_iter().next().unwrap() {
            Request::Decompile { id, .. } => id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::Decompiled {
            id,
            tool: "broken".into(),
            result: Err("exited with 2".into()),
        });
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("exited with 2"), "{}", app.status);
        assert!(
            app.mode.viewer().is_some(),
            "the viewer should still be open"
        );
    }

    #[test]
    fn a_stale_decompilation_result_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"text\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        app.handle_event(Event::Decompiled {
            id: 4242,
            tool: "ghost".into(),
            result: Ok("whatever".into()),
        });
        assert!(app.mode.viewer().unwrap().derived.is_none());
    }

    #[test]
    fn localisation_follows_the_configured_language() {
        let mut app = app();
        app.i18n = I18n::new("it");
        press(&mut app, "Ctrl+H");
        assert!(app.status.contains("nascosti"), "{}", app.status);
        assert_eq!(app.keybar()[4].1, "Copia");
    }

    /// A panel over a real directory, sized for the editor.
    fn editing_app(dir: &std::path::Path) -> App {
        let mut app = app_on_disk(dir);
        app.editor_size = (40, 6);
        app
    }

    #[test]
    fn f4_opens_the_editor_on_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"alpha\nbeta\n").unwrap();
        let mut app = editing_app(dir.path());

        press(&mut app, "Down"); // notes.txt
        press(&mut app, "F4");
        let editor = app.mode.editor().expect("the editor should be open");
        assert_eq!(editor.name(), "notes.txt");
        assert_eq!(editor.buffer.lines(), ["alpha", "beta"]);

        // Typing goes to the editor, not to the panel.
        let cursor_before = app.panel().cursor;
        press(&mut app, "End");
        for c in ["!", "!"] {
            press(&mut app, c);
        }
        assert_eq!(app.panel().cursor, cursor_before, "the panel moved instead");
        assert_eq!(app.mode.editor().unwrap().buffer.line(0), "alpha!!");
        assert!(app.mode.editor().unwrap().buffer.modified());
    }

    #[test]
    fn f2_writes_the_file_and_reports_what_it_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"alpha\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "End");
        press(&mut app, "x");
        press(&mut app, "F2");

        assert_eq!(std::fs::read_to_string(&path).unwrap(), "alphax\n");
        assert!(!app.mode.editor().unwrap().buffer.modified());
        assert!(app.status.contains("notes.txt"), "{}", app.status);
        // Nothing more to save is said, not silently ignored.
        press(&mut app, "F2");
        assert!(app.status.contains("notes.txt"), "{}", app.status);
    }

    #[test]
    fn quitting_with_unsaved_changes_asks_first() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"alpha\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "End");
        press(&mut app, "z");

        press(&mut app, "Esc");
        match &app.mode {
            Mode::Confirm(c) => assert_eq!(c.action, ConfirmAction::DiscardEdits),
            other => panic!("expected the discard confirmation, got {other:?}"),
        }
        // Saying no keeps the editor and the edit.
        press(&mut app, "Esc");
        assert_eq!(app.mode.editor().unwrap().buffer.line(0), "alphaz");
        // Saying yes throws the edit away and leaves the file alone.
        press(&mut app, "Esc");
        press(&mut app, "Enter");
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "alpha\n");
    }

    #[test]
    fn quitting_a_clean_editor_refreshes_the_panels() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"alpha\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        app.requests.clear();
        press(&mut app, "F10");
        assert!(app.mode.is_normal());
        assert!(!app.quit, "F10 in the editor closes the editor, not scmd");
        let reads: Vec<usize> = app
            .take_requests()
            .iter()
            .filter_map(|r| match r {
                Request::Read { panel, .. } => Some(*panel),
                _ => None,
            })
            .collect();
        assert_eq!(reads, vec![0, 1]);
    }

    #[test]
    fn shift_f4_edits_a_new_file_in_the_panel_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("keep.txt"), b"x\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Shift+F4");
        assert!(app.mode.editor().unwrap().path.is_none());
        for c in ["h", "i"] {
            press(&mut app, c);
        }

        // With no name yet, F2 has to ask for one.
        press(&mut app, "F2");
        match &app.mode {
            Mode::Prompt(p) => assert_eq!(p.kind, PromptKind::EditorSaveAs),
            other => panic!("expected the save-as prompt, got {other:?}"),
        }
        for c in ["n", "e", "w", ".", "t", "x", "t"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let written = std::fs::read_to_string(dir.path().join("new.txt")).unwrap();
        // A new file gets this platform's line ending, so compare against that.
        assert_eq!(
            written,
            format!("hi{}", scmd_edit::Eol::detect("").0.bytes())
        );
        assert_eq!(app.mode.editor().unwrap().name(), "new.txt");
    }

    #[test]
    fn a_file_changed_underneath_us_is_never_overwritten_silently() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.txt");
        std::fs::write(&path, b"mine\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "End");
        press(&mut app, "!");
        // Someone else saves while we are editing.
        std::thread::sleep(std::time::Duration::from_millis(20));
        std::fs::write(&path, b"theirs\n").unwrap();

        press(&mut app, "F2");
        match &app.mode {
            Mode::Confirm(c) => assert_eq!(c.action, ConfirmAction::OverwriteChanged),
            other => panic!("expected the overwrite confirmation, got {other:?}"),
        }
        press(&mut app, "Esc");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");
        assert!(app.mode.editor().is_some(), "the editor must stay open");

        press(&mut app, "F2");
        press(&mut app, "Enter"); // yes, overwrite
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "mine!\n");
    }

    #[test]
    fn searching_and_replacing_runs_through_the_prompts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("code.rs").as_path(),
            b"let a = 1;\nlet b = 2;\n",
        )
        .unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");

        press(&mut app, "F7");
        for c in ["l", "e", "t", "Space", "b"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let editor = app.mode.editor().expect("the editor should be back");
        assert_eq!(editor.cursor.line, 1);
        assert_eq!(editor.last_search.as_deref(), Some("let b"));

        // A query that is not there says so and keeps the editor open.
        press(&mut app, "F7");
        for c in ["z", "z"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
        assert!(app.mode.editor().is_some());

        // Replace all: "let" → "const".
        press(&mut app, "Ctrl+F7");
        for c in ["l", "e", "t"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        for c in ["c", "o", "n", "s", "t"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(
            app.mode.editor().unwrap().buffer.lines(),
            ["const a = 1;", "const b = 2;"]
        );
        assert!(app.status.contains('2'), "{}", app.status);
    }

    #[test]
    fn undo_goto_and_line_commands_are_reachable_from_the_keymap() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"one\ntwo\nthree\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");

        press(&mut app, "Ctrl+Y"); // delete the line, as in FAR
        assert_eq!(app.mode.editor().unwrap().buffer.lines(), ["two", "three"]);
        press(&mut app, "Ctrl+Z");
        assert_eq!(
            app.mode.editor().unwrap().buffer.lines(),
            ["one", "two", "three"]
        );

        press(&mut app, "Alt+F8"); // go to line
        press(&mut app, "3");
        press(&mut app, "Enter");
        assert_eq!(app.mode.editor().unwrap().cursor.line, 2);
    }

    #[test]
    fn a_column_block_is_cut_and_pasted_as_a_rectangle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"abcd\nefgh\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");

        // Alt+Shift+arrows mark a column block: two rows, two columns.
        press(&mut app, "Alt+Shift+Right");
        press(&mut app, "Alt+Shift+Right");
        press(&mut app, "Alt+Shift+Down");
        let editor = app.mode.editor().unwrap();
        assert_eq!(editor.block.unwrap().kind, BlockKind::Column);
        assert_eq!(editor.block_text().as_deref(), Some("ab\nef"));

        press(&mut app, "Ctrl+X");
        assert_eq!(app.mode.editor().unwrap().buffer.lines(), ["cd", "gh"]);
        press(&mut app, "Ctrl+V");
        assert_eq!(app.mode.editor().unwrap().buffer.lines(), ["abcd", "efgh"]);
    }

    #[test]
    fn a_read_only_file_is_opened_and_said_to_be_read_only() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("ro.txt");
        std::fs::write(&path, b"locked\n").unwrap();
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(&path, perms).unwrap();

        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        assert!(app.mode.editor().unwrap().buffer.read_only);
        assert!(
            app.status.to_lowercase().contains("read-only"),
            "{}",
            app.status
        );
        press(&mut app, "x");
        assert_eq!(app.mode.editor().unwrap().buffer.line(0), "locked");
        assert!(app.status_error, "{}", app.status);
    }

    // ── netbox ─────────────────────────────────────────────────────────────────────────────

    fn site() -> Site {
        let mut site = Site::parse_url("sftp://mb@host.example:22/srv").unwrap();
        site.id = "work".into();
        site
    }

    /// An app with one site in the list, nothing connected yet.
    fn net_app() -> App {
        let mut app = app();
        app.sites = vec![site()];
        app
    }

    /// An app whose left panel is already on the remote host.
    fn connected_app() -> App {
        let mut app = net_app();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site()),
            outcome: ConnectOutcome::Open {
                start: site().start(),
                connected: vec![site().authority()],
            },
        });
        app.requests.clear();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("www", EntryKind::Dir, 0),
                entry("notes.txt", EntryKind::File, 120),
            ]),
        });
        app
    }

    fn connect_request(app: &mut App) -> (usize, Site, Option<String>, bool) {
        match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Connect { .. }))
        {
            Some(Request::Connect {
                panel,
                site,
                secret,
                accept_host_key,
            }) => (panel, *site, secret, accept_host_key),
            other => panic!("expected a connect request, got {other:?}"),
        }
    }

    #[test]
    fn the_site_list_connects_on_enter() {
        let mut app = net_app();
        press(&mut app, "Alt+N");
        assert!(
            matches!(app.mode, Mode::Sites { cursor: 0 }),
            "{:?}",
            app.mode
        );
        press(&mut app, "Enter");
        let (panel, site, secret, accept) = connect_request(&mut app);
        assert_eq!(panel, 0);
        assert_eq!(site.authority(), "mb@host.example:22");
        assert_eq!(secret, None);
        assert!(
            !accept,
            "a first attempt never accepts a host key by itself"
        );
        assert!(app.status.contains("host.example"), "{}", app.status);

        // The answer puts the panel on the server and asks for a listing.
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site.clone()),
            outcome: ConnectOutcome::Open {
                start: site.start(),
                connected: vec![site.authority()],
            },
        });
        assert_eq!(app.panels[0].cwd, site.start());
        assert_eq!(
            app.panels[0].cwd.to_string(),
            "sftp://mb@host.example:22/srv"
        );
        assert!(app.connected.contains(&site.authority()));
        assert!(app
            .take_requests()
            .iter()
            .any(|r| matches!(r, Request::Read { panel: 0, .. })));
    }

    #[test]
    fn a_password_is_asked_for_and_never_echoed() {
        let mut app = net_app();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site()),
            outcome: ConnectOutcome::NeedsSecret(scmd_net::SecretKind::Password),
        });
        match &app.mode {
            Mode::Prompt(p) => {
                assert_eq!(
                    p.kind,
                    PromptKind::NetSecret(scmd_net::SecretKind::Password)
                );
                assert!(p.secret, "a password prompt must be masked");
                assert!(p.title.contains("mb"), "{}", p.title);
            }
            other => panic!("expected the password prompt, got {other:?}"),
        }
        for c in ["h", "u", "n", "t", "e", "r"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let (_, site, secret, _) = connect_request(&mut app);
        assert_eq!(secret.as_deref(), Some("hunter"));
        assert_eq!(site.authority(), "mb@host.example:22");
    }

    #[test]
    fn an_unknown_host_key_is_a_decision_with_the_fingerprint_in_view() {
        let mut app = net_app();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site()),
            outcome: ConnectOutcome::UnknownHostKey {
                fingerprint: "SHA256:abc123".into(),
            },
        });
        match &app.mode {
            Mode::Confirm(c) => {
                assert!(
                    c.lines.iter().any(|l| l.contains("SHA256:abc123")),
                    "{:?}",
                    c.lines
                );
                assert!(matches!(c.action, ConfirmAction::AcceptHostKey { .. }));
            }
            other => panic!("expected the host-key confirmation, got {other:?}"),
        }
        // Saying no connects to nothing.
        press(&mut app, "Esc");
        assert!(app.take_requests().is_empty());

        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site()),
            outcome: ConnectOutcome::UnknownHostKey {
                fingerprint: "SHA256:abc123".into(),
            },
        });
        press(&mut app, "Enter");
        let (_, _, _, accept) = connect_request(&mut app);
        assert!(accept, "the retry must carry the user's decision");
    }

    #[test]
    fn a_refused_connection_says_why_and_changes_nothing() {
        let mut app = net_app();
        let before = app.panels[0].cwd.clone();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site()),
            outcome: ConnectOutcome::Failed("mb@host.example: authentication failed".into()),
        });
        assert!(app.status_error, "{}", app.status);
        assert!(
            app.status.contains("authentication failed"),
            "{}",
            app.status
        );
        assert_eq!(app.panels[0].cwd, before);
    }

    #[test]
    fn disconnecting_takes_the_panel_home() {
        let mut app = connected_app();
        press(&mut app, "Alt+Shift+N");
        match app.take_requests().into_iter().next() {
            Some(Request::Disconnect { authority }) => {
                assert_eq!(authority, "mb@host.example:22")
            }
            other => panic!("expected a disconnect, got {other:?}"),
        }
        app.handle_event(Event::Disconnected {
            authority: "mb@host.example:22".into(),
            connected: Vec::new(),
        });
        assert!(app.panels[0].cwd.is_local(), "{}", app.panels[0].cwd);
        assert!(app.connected.is_empty());
        assert!(app.status.contains("host.example"), "{}", app.status);

        // A local panel is told plainly that there is nothing to disconnect.
        app.take_requests(); // the read of the home directory
        press(&mut app, "Alt+Shift+N");
        assert!(app.take_requests().is_empty());
        assert!(!app.status.is_empty());
    }

    #[test]
    fn a_url_typed_into_the_goto_prompt_connects() {
        let mut app = net_app();
        press(&mut app, "Ctrl+Shift+L");
        for c in [
            "s", "f", "t", "p", ":", "/", "/", "m", "b", "@", "h", "o", "s", "t", "/", "s", "r",
            "v",
        ] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        let (_, site, _, _) = connect_request(&mut app);
        assert_eq!(site.host, "host");
        assert_eq!(site.user, "mb");
        assert_eq!(site.start_dir, "/srv");
        // A local path still changes directory, it does not dial anything.
        press(&mut app, "Ctrl+Shift+L");
        for c in ["/", "t", "m", "p"] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        assert_eq!(app.panel().cwd, VPath::local("/tmp"));
    }

    #[test]
    fn a_remote_panel_navigates_and_feeds_jobs_remote_paths() {
        let mut app = connected_app();
        assert_eq!(app.panel().listing.len(), 3);
        // Enter on a directory asks for the remote listing of it.
        press(&mut app, "Down");
        press(&mut app, "Enter");
        match app.take_requests().into_iter().next() {
            Some(Request::Read { path, .. }) => {
                assert_eq!(path.to_string(), "sftp://mb@host.example:22/srv/www")
            }
            other => panic!("{other:?}"),
        }
        press(&mut app, "Backspace");
        app.take_requests();
        assert_eq!(app.panel().cwd.to_string(), "sftp://mb@host.example:22/srv");
        // The parent's listing comes back, as it would from the server.
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("www", EntryKind::Dir, 0),
                entry("notes.txt", EntryKind::File, 120),
            ]),
        });

        // F5 from the remote panel to the local one: the source is a remote path.
        // The passive panel gets a real local directory, so the prefilled destination is a
        // path this platform calls absolute (on Windows, `/tmp` is not).
        app.panels[1].go(VPath::local(std::env::temp_dir()), true);
        app.panels[0].cursor_to(2); // notes.txt
        press(&mut app, "F5");
        press(&mut app, "Enter");
        match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Job { .. }))
        {
            Some(Request::Job { spec, .. }) => {
                assert_eq!(spec.kind, JobKind::Copy);
                assert_eq!(
                    spec.sources[0].to_string(),
                    "sftp://mb@host.example:22/srv/notes.txt"
                );
                assert!(spec.target.unwrap().is_local());
            }
            other => panic!("expected a copy job, got {other:?}"),
        }
    }

    #[test]
    fn f3_on_a_remote_file_fetches_it_then_views_the_copy() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(
            &local,
            b"from the server
",
        )
        .unwrap();
        let mut app = connected_app();
        app.panels[0].cursor_to(2); // notes.txt
        press(&mut app, "F3");
        let id = match app.take_requests().into_iter().next() {
            Some(Request::Fetch { id, path, purpose }) => {
                assert_eq!(path.to_string(), "sftp://mb@host.example:22/srv/notes.txt");
                assert_eq!(purpose, Fetched::View);
                id
            }
            other => panic!("expected a fetch, got {other:?}"),
        };
        assert!(app.mode.is_normal(), "the viewer must wait for the bytes");

        app.handle_event(Event::Fetched {
            id,
            remote: app.panels[0].cwd.join("notes.txt"),
            purpose: Fetched::View,
            result: Ok((
                local.clone(),
                RemoteStamp {
                    size: 16,
                    mtime: None,
                },
            )),
        });
        let viewer = app.mode.viewer().expect("the viewer should be open");
        assert_eq!(
            viewer.path.to_string(),
            "sftp://mb@host.example:22/srv/notes.txt"
        );
        assert!(app.status.contains("copy"), "{}", app.status);

        // A fetch we are no longer waiting for is ignored.
        app.handle_event(Event::Fetched {
            id: id + 99,
            remote: app.panels[0].cwd.join("other.txt"),
            purpose: Fetched::View,
            result: Ok((
                local,
                RemoteStamp {
                    size: 16,
                    mtime: None,
                },
            )),
        });
        assert_eq!(app.mode.viewer().unwrap().path.display_name(), "notes.txt");
    }

    #[test]
    fn a_remote_file_too_big_to_fetch_says_so_with_the_limit() {
        let mut app = connected_app();
        app.cfg.net.fetch_limit_mb = 1;
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("huge.iso", EntryKind::File, 50 * 1024 * 1024),
            ]),
        });
        app.panels[0].cursor_to(1);
        press(&mut app, "F3");
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("1 MB"), "{}", app.status);
        assert!(app.take_requests().is_empty(), "it fetched it anyway");
    }

    #[test]
    fn the_site_list_shows_hosts_connected_by_hand_too() {
        let mut app = net_app();
        app.connected = vec!["root@other:2222".into()];
        let rows = app.site_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].authority(), "mb@host.example:22");
        assert_eq!(rows[1].authority(), "root@other:2222");

        // `d` on a site that is not connected does not pretend to disconnect it.
        press(&mut app, "Alt+N");
        press(&mut app, "d");
        assert!(app.take_requests().is_empty());
        assert!(matches!(app.mode, Mode::Sites { .. }));
        // On the connected one it does.
        press(&mut app, "Down");
        press(&mut app, "d");
        match app.take_requests().into_iter().next() {
            Some(Request::Disconnect { authority }) => assert_eq!(authority, "root@other:2222"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn f4_on_a_remote_file_edits_a_copy_and_writes_it_back() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(&local, b"server side\n").unwrap();
        let mut app = connected_app();
        app.panels[0].cursor_to(2); // notes.txt
        press(&mut app, "F4");

        let (id, purpose) = match app.take_requests().into_iter().next() {
            Some(Request::Fetch { id, path, purpose }) => {
                assert_eq!(path.to_string(), "sftp://mb@host.example:22/srv/notes.txt");
                (id, purpose)
            }
            other => panic!("expected a fetch, got {other:?}"),
        };
        assert_eq!(purpose, Fetched::Edit);

        let remote = app.panels[0].cwd.join("notes.txt");
        app.handle_event(Event::Fetched {
            id,
            remote: remote.clone(),
            purpose: Fetched::Edit,
            result: Ok((
                local.clone(),
                RemoteStamp {
                    size: 12,
                    mtime: Some(1_700_000_000),
                },
            )),
        });
        assert!(app.mode.editor().is_some(), "the editor should be open");
        assert_eq!(
            app.remote_edit().map(|e| e.remote.to_string()),
            Some("sftp://mb@host.example:22/srv/notes.txt".to_string())
        );
        assert!(app.status.contains("copy"), "{}", app.status);

        // Type, save: the local copy is written and the upload is asked for.
        press(&mut app, "End");
        press(&mut app, "!");
        press(&mut app, "F2");
        assert_eq!(std::fs::read_to_string(&local).unwrap(), "server side!\n");
        let (upload_id, expect, force) = match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Upload { .. }))
        {
            Some(Request::Upload {
                id,
                local: sent,
                remote: target,
                expect,
                force,
            }) => {
                assert_eq!(sent, local);
                assert_eq!(target, remote);
                (id, expect, force)
            }
            other => panic!("expected an upload, got {other:?}"),
        };
        assert!(!force, "a first write-back never forces");
        assert_eq!(
            expect.map(|s| s.size),
            Some(12),
            "the stamp must be carried"
        );

        app.handle_event(Event::Uploaded {
            id: upload_id,
            remote: remote.clone(),
            result: Ok(13),
        });
        assert!(app.status.contains("written back"), "{}", app.status);
        assert!(!app.status_error);
        // The panels are refreshed, because the size and date just changed.
        assert!(app
            .take_requests()
            .iter()
            .any(|r| matches!(r, Request::Read { .. })));
    }

    #[test]
    fn a_file_changed_on_the_server_is_not_overwritten_without_a_yes() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(&local, b"mine\n").unwrap();
        let mut app = connected_app();
        let remote = app.panels[0].cwd.join("notes.txt");
        app.open_remote_editor_for_test(remote.clone(), local.clone());
        press(&mut app, "End");
        press(&mut app, "x");
        press(&mut app, "F2");
        let id = match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Upload { .. }))
        {
            Some(Request::Upload { id, .. }) => id,
            other => panic!("{other:?}"),
        };

        app.handle_event(Event::Uploaded {
            id,
            remote: remote.clone(),
            result: Err(UploadError::Changed),
        });
        match &app.mode {
            Mode::Confirm(c) => {
                assert_eq!(c.action, ConfirmAction::OverwriteRemote);
                assert!(c.lines[0].contains("notes.txt"), "{:?}", c.lines);
            }
            other => panic!("expected the overwrite confirmation, got {other:?}"),
        }
        // No: the editor comes back and the server keeps its file.
        press(&mut app, "Esc");
        assert!(app.mode.editor().is_some());
        assert!(app.take_requests().is_empty());

        // Yes: the write-back is repeated, this time forcing.
        app.upload_id_for_test(id);
        app.handle_event(Event::Uploaded {
            id,
            remote,
            result: Err(UploadError::Changed),
        });
        press(&mut app, "Enter");
        match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Upload { .. }))
        {
            Some(Request::Upload { force, .. }) => assert!(force, "the retry must force"),
            other => panic!("{other:?}"),
        }
        assert!(app.mode.editor().is_some(), "the editor must stay open");
    }

    #[test]
    fn a_failed_write_back_says_where_the_edits_are() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(&local, b"mine\n").unwrap();
        let mut app = connected_app();
        let remote = app.panels[0].cwd.join("notes.txt");
        app.open_remote_editor_for_test(remote.clone(), local.clone());
        press(&mut app, "End");
        press(&mut app, "x");
        press(&mut app, "F2");
        let id = match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Upload { .. }))
        {
            Some(Request::Upload { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::Uploaded {
            id,
            remote,
            result: Err(UploadError::Failed("quota exceeded".into())),
        });
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("quota exceeded"), "{}", app.status);
        assert!(
            app.status.contains(&local.display().to_string()),
            "the user must be told where their work is: {}",
            app.status
        );
        assert!(app.mode.editor().is_some());
    }

    #[test]
    fn save_as_onto_a_remote_path_writes_a_copy_and_sends_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("local.txt"), b"local file\n").unwrap();
        let mut app = app_on_disk(dir.path());
        app.panels[1].go(
            VPath::remote(scmd_core::Scheme::Sftp, "mb@host:22", "/srv"),
            true,
        );
        app.take_requests();
        press(&mut app, "Down"); // local.txt
        press(&mut app, "F4");
        assert!(app.mode.editor().is_some(), "{}", app.status);

        press(&mut app, "Shift+F2");
        match &app.mode {
            Mode::Prompt(p) => assert_eq!(p.kind, PromptKind::EditorSaveAs),
            other => panic!("expected the save-as prompt, got {other:?}"),
        }
        // Clear the prefilled path and type a remote URL.
        for _ in 0..80 {
            press(&mut app, "Backspace");
        }
        for c in [
            "s", "f", "t", "p", ":", "/", "/", "m", "b", "@", "h", "o", "s", "t", ":", "2", "2",
            "/", "s", "r", "v", "/", "u", "p", ".", "t", "x", "t",
        ] {
            press(&mut app, c);
        }
        press(&mut app, "Enter");
        match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Upload { .. }))
        {
            Some(Request::Upload {
                local,
                remote,
                force,
                ..
            }) => {
                assert_eq!(remote.to_string(), "sftp://mb@host:22/srv/up.txt");
                assert_eq!(std::fs::read_to_string(&local).unwrap(), "local file\n");
                assert!(force, "save-as means overwrite");
            }
            other => panic!("expected an upload, got {other:?}"),
        }
        assert_eq!(
            app.remote_edit().map(|e| e.remote.display_name()),
            Some("up.txt".to_string())
        );
    }

    #[test]
    fn leaving_the_editor_forgets_the_remote_file() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(&local, b"mine\n").unwrap();
        let mut app = connected_app();
        let remote = app.panels[0].cwd.join("notes.txt");
        app.open_remote_editor_for_test(remote, local);
        press(&mut app, "F10");
        assert!(app.mode.is_normal());
        assert!(app.remote_edit().is_none());
    }
}
