use std::sync::mpsc::Sender;

use fcmd_cmd::{Command, Key, KeyCode, Keymap, Mods, Scope, Side};
use fcmd_config::DecompileRule;
use fcmd_config::{Config, Paths, QuickSearch};
use fcmd_core::{CancelToken, Entry, MaskScope, Sort, SortKey, VPath, ViewMode};
use fcmd_edit::{BlockKind, Editor, HexEdit};
use fcmd_i18n::I18n;
use fcmd_jobs::{Decision, JobKind, JobSpec, Outcome, Progress, Question};
use fcmd_net::{SecretKind, Site};
use fcmd_tui::Input;
use fcmd_ui::{Theme, ThemeSet};
use fcmd_view::{FileSource, Mode as ViewerMode, SearchMode, Viewer};

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
    /// Open a tunnel on the connection `authority` names.
    TunnelOpen {
        id: u64,
        authority: String,
        spec: String,
    },
    /// Close one, which frees the local port.
    TunnelClose {
        id: u64,
    },
    /// What is open, with the counters as they are now.
    TunnelList,
    /// Read both sides of every pair and say which differ. The pairs are already matched by
    /// name: this is the reading, which is the part that takes time.
    CompareContents {
        id: u64,
        pairs: Vec<ComparePair>,
        cancel: CancelToken,
    },
    /// Hash each path and write the list (SPEC 03 §9). `target` is the one file they all go
    /// into; `None` means one list beside each file.
    Checksums {
        id: u64,
        paths: Vec<VPath>,
        algorithm: fcmd_jobs::Algorithm,
        target: Option<VPath>,
        cancel: CancelToken,
    },
    /// Read a list of digests and check the files beside it against it.
    Verify {
        id: u64,
        list: VPath,
        cancel: CancelToken,
    },
    /// Ask the terminal for a font a step larger, a step smaller, or the one it started with.
    /// Only the loop that owns the terminal can send it, so it travels as a request.
    Font {
        by: i8,
    },
    /// Start or stop asking the terminal to report the mouse. Like the font, only the loop
    /// that owns the terminal can do it.
    Mouse {
        on: bool,
    },
    /// Ask the terminal to put text on the system clipboard. The terminal does the copying,
    /// which is why it travels as a request and not as a call (SPEC 06 §11).
    Clipboard {
        text: String,
    },
    /// Who the files of one directory belong to, for the owners view. A lookup per file, and
    /// on Windows a call into the security API, so it happens in a worker and only while that
    /// view is the one on screen (SPEC 02 §3).
    Owners {
        panel: usize,
        generation: u64,
        dir: VPath,
        names: Vec<String>,
    },
    /// Read a directory's `descript.ion` (SPEC 02 §9), for the views that show it.
    Descriptions {
        panel: usize,
        generation: u64,
        dir: VPath,
    },
    /// Write one back. The whole file at once: it is a line per described file, and a
    /// directory with a thousand of those has a file of a few kilobytes.
    WriteDescriptions {
        dir: VPath,
        descriptions: fcmd_core::descriptions::Descriptions,
    },
    /// Decide about every pair in two trees. Nothing is written: the answer is a table.
    SyncPlan {
        id: u64,
        left: VPath,
        right: VPath,
        options: Box<fcmd_jobs::sync::Options>,
        cancel: CancelToken,
    },
    /// Walk and match. The engine is `fcmd-find`; this carries what the dialog asked for.
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
    /// same counters — but the writing is one pass through `fcmd-arc` rather than a plan of
    /// file copies, because a central directory or a compressed stream is not a sequence of
    /// independent files.
    Pack {
        id: u64,
        archive: VPath,
        format: fcmd_arc::Format,
        level: fcmd_arc::Level,
        sources: Vec<VPath>,
        cancel: CancelToken,
    },
    /// Read every entry of an archive and report what does not come out whole. A job as far
    /// as the user is concerned, for the same reason `Pack` is one.
    ArcTest {
        id: u64,
        archive: VPath,
        cancel: CancelToken,
    },
    /// Write a new archive out of this one with the changes applied (SPEC 04 §4). The old one
    /// keeps its bytes until the new one is written and read back.
    ArcRebuild {
        id: u64,
        archive: VPath,
        changes: Vec<fcmd_arc::Change>,
        level: fcmd_arc::Level,
        cancel: CancelToken,
    },
    /// Run a plugin's command (SPEC 08 §1). Off the interface thread, because it is somebody
    /// else's program and we do not know how long it will think about it.
    Plugin {
        id: String,
        label: String,
        program: std::path::PathBuf,
        cwd: Option<std::path::PathBuf>,
        ctx: Box<fcmd_plug::host::Context>,
        timeout: Option<u64>,
    },
    /// Create a link. Not a job: it is one syscall, with nothing to report progress on.
    Link {
        target: VPath,
        link: VPath,
        kind: fcmd_jobs::LinkKind,
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
        /// The connection's key: `scheme://user@host:port`, which is what the pool holds it
        /// under — the authority alone cannot tell `ftp` from `ftps` on one port.
        key: String,
    },
    /// Copy a remote file into a temporary directory so the viewer or the editor can open it.
    Fetch {
        id: u64,
        path: VPath,
        purpose: Fetched,
    },
    /// Write a temporary file back to the server it came from (SPEC 04 §5).
    /// Change what these paths are: permissions and times (SPEC 10 §3.8).
    SetAttrs {
        id: u64,
        paths: Vec<VPath>,
        change: Box<fcmd_jobs::Change>,
        recursive: bool,
        cancel: CancelToken,
    },
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
    /// Something that is not a key has changed what the screen should show: a terminal's
    /// program writing output nobody asked for. The loop draws after every event, so this
    /// carries nothing and does nothing but arrive.
    Redraw,
    Input(Input),
    /// A plugin's command has finished: what it asked for, or why it could not.
    PluginDone {
        id: String,
        label: String,
        result: Result<fcmd_plug::host::Reply, String>,
    },
    Listed {
        panel: usize,
        generation: u64,
        result: Result<Vec<Entry>, String>,
    },
    /// Who the files of a directory belong to, for the owners view. Late, like the free
    /// space: the listing is on screen long before this arrives, and the two columns fill in
    /// when it does.
    Owners {
        panel: usize,
        generation: u64,
        owners: std::collections::HashMap<String, (String, String)>,
    },
    /// What a directory's own `descript.ion` says about its files (SPEC 02 §9). Late, as the
    /// owners are: the listing is on screen before the file has been read.
    Descriptions {
        panel: usize,
        generation: u64,
        descriptions: fcmd_core::descriptions::Descriptions,
    },
    /// What a provider said about the listing it just gave: it stopped before the end, and
    /// why. Sent after `Listed` and only when there is something to say, the way `FreeSpace`
    /// is sent separately — a local directory is always read whole and should not pay for the
    /// question.
    ListingCut {
        panel: usize,
        generation: u64,
        note: String,
    },
    /// A directory a panel is looking at has changed on disk, as the filesystem itself
    /// reported it (see the `watch` module). Debounced there: one of these stands for every
    /// change in the burst it arrived with.
    DirChanged {
        panel: usize,
    },
    /// The tunnels that are open, with their counters — the answer to `TunnelList`, and what
    /// is sent after one opens or closes.
    Tunnels {
        items: Vec<TunnelInfo>,
    },
    /// A tunnel could not be opened, and why.
    TunnelFailed {
        spec: String,
        error: String,
    },
    /// How far a comparison by content has got.
    CompareProgress {
        id: u64,
        done: u64,
        total: u64,
    },
    /// A comparison by content finished, or was stopped: the names whose bytes differ, and
    /// how many pairs could not be read at all.
    SyncProgress {
        id: u64,
        at: String,
        pairs: usize,
    },
    SyncPlanned {
        id: u64,
        pairs: Vec<fcmd_jobs::sync::Pair>,
        cancelled: bool,
        problem: Option<String>,
    },
    /// One file of a checksum run is finished: `done` of `total`.
    SumProgress {
        id: u64,
        name: String,
        done: u64,
        total: u64,
    },
    SumsDone {
        id: u64,
        /// The lists that were written.
        written: Vec<VPath>,
        errors: Vec<(VPath, String)>,
        cancelled: bool,
    },
    VerifyDone {
        id: u64,
        verdicts: Vec<fcmd_jobs::sum::Verdict>,
        cancelled: bool,
        /// The list could not be read, or its digests do not agree on one hash.
        problem: Option<String>,
    },
    CompareDone {
        id: u64,
        differ: Vec<String>,
        /// The pairs whose bytes differ only in how their lines end, which is a difference
        /// worth naming rather than counting with the rest.
        line_endings: Vec<String>,
        unreadable: u64,
        stopped: bool,
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
        disk: Option<fcmd_platform::Disk>,
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
    /// What a running command has printed since the last one of these: whole lines, as they
    /// were printed, so a long command can be watched rather than waited out.
    ShellOutput {
        id: u64,
        text: String,
    },
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
        /// `scheme://user@host:port`, as the request asked for.
        key: String,
        /// Every connection still open, so the site list stays truthful.
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
    /// What the directory's `descript.ion` should say about a file (FAR's `Ctrl+Z`). The
    /// name is carried along because the cursor may have moved by the time it is answered.
    Describe {
        name: String,
    },
    Goto,
    SelectMask,
    DeselectMask,
    Filter,
    /// Query for the viewer's search.
    ViewerSearch,
    /// Offset or percentage for the viewer.
    ViewerGoto,
    EditorReplaceFrom,
    EditorReplaceWith,
    EditorGoto,
    EditorSaveAs,
    /// Name for F7.
    Mkdir,
    /// What to call this tab. An empty answer takes the name off again.
    TabName,
    /// What to call a folder in the hotlist. An empty answer leaves it with its path.
    BookmarkName {
        row: usize,
    },
    /// `Ctrl+G`: one command line, run once per selected file (SPEC 06 §5).
    ApplyCommand,
    /// Where a tunnel goes: `[local:]host:port`, or a port on its own.
    Tunnel,
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
        scope: fcmd_cmd::Scope,
        steps: Vec<fcmd_cmd::Step>,
    },
    /// One instruction to assemble over the bytes at the cursor (SPEC 05 §2.8).
    Assemble,
    /// `offset length`, in hex: bytes from elsewhere in this same file, copied to the cursor.
    HexCopy,
    /// The same, but the source is removed — the bytes move rather than multiply.
    HexMove,
    /// One line of arithmetic, answered in every base at once.
    Calculator,
    /// A colour for the palette editor's current row: a name, one of the 256, or `#rrggbb`.
    Color,
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

/// Split a destination line into destinations.
///
/// `;` separates them, and a path that has one in it is quoted — which is the convention FAR
/// uses and the only one that can express a directory called `a;b`. A line with no separator
/// is one destination, quotes and all, so a user who never copies to two places at once never
/// meets any of this.
fn split_targets(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    for c in line.chars() {
        match c {
            '"' => quoted = !quoted,
            ';' if !quoted => {
                let trimmed = current.trim();
                if !trimmed.is_empty() {
                    out.push(trimmed.to_string());
                }
                current = String::new();
            }
            _ => current.push(c),
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

/// What the copy dialog was last answered with. Not the job's options: this is what the
/// *dialog* should open on next time, which is a smaller thing and outlives any one job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyChoices {
    /// The conflict policy's key, as the dialog writes it.
    pub conflict: &'static str,
    pub only_newer: bool,
    pub follow_links: bool,
    pub filter: String,
}

impl Default for CopyChoices {
    fn default() -> Self {
        // The prompt's behaviour, exactly: ask about a conflict, follow nothing, filter
        // nothing. Somebody who types a path and presses Enter gets what they always got.
        CopyChoices {
            conflict: "ask",
            only_newer: false,
            follow_links: false,
            filter: String::new(),
        }
    }
}

/// A local file's permission bits, or `None` where there are none to read.
///
/// On Windows there is no mode and the dialog does not offer one, so this answers `None`
/// there and the twelve boxes are not built.
fn mode_of(path: &VPath) -> Option<u32> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if !path.is_local() || path.in_container() {
            return None;
        }
        std::fs::metadata(path.os())
            .ok()
            .map(|m| m.permissions().mode() & 0o7777)
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        None
    }
}

/// Which of an `[[assoc]]` block's commands a key is after.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Which {
    View,
    Edit,
}

/// What a confirmation will do on `Enter`.
#[derive(Debug, Clone, PartialEq)]
pub enum ConfirmAction {
    /// Start this file operation.
    ///
    /// Boxed, and clippy is right to ask: a `JobSpec` carries the sources, the target and
    /// every option, which is most of a kilobyte, and every other answer here is a word.
    Job(Box<JobSpec>),
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
    PlayMacro(Box<fcmd_cmd::Macro>),
    /// Run a plugin's command, which said it should be asked about first.
    RunPlugin(usize),
    /// Write the archive again with these changes: what `F6` and `F8` do inside one.
    ArcRebuild {
        archive: VPath,
        changes: Vec<fcmd_arc::Change>,
    },
    /// Record a host key we have never seen and connect.
    AcceptHostKey {
        panel: usize,
        site: Box<Site>,
        secret: Option<String>,
    },
}

impl ConfirmAction {
    /// What the button that does it says. FAR names the verb — `{ Move }`, not `{ Yes }` —
    /// and the reason is that a reader who glances at the dialog and the button should know
    /// what is about to happen without reading the sentence again. Keeping the label here,
    /// next to the action, is what stops a button saying one thing and doing another.
    pub fn verb_key(&self) -> &'static str {
        match self {
            ConfirmAction::Job(spec) => match spec.kind {
                JobKind::Trash => "btn-move",
                JobKind::Delete => "btn-delete",
                JobKind::Wipe => "btn-wipe",
                JobKind::Copy => "btn-copy",
                JobKind::Move => "btn-move",
                _ => "btn-ok",
            },
            ConfirmAction::ArcRebuild { .. } => "btn-delete",
            ConfirmAction::DiscardEdits | ConfirmAction::DiscardHex => "btn-discard",
            ConfirmAction::OverwriteChanged | ConfirmAction::OverwriteRemote => "btn-overwrite",
            ConfirmAction::QuitWithScreens => "btn-quit",
            ConfirmAction::PlayMacro(_) => "btn-play",
            ConfirmAction::RunPlugin(_) => "btn-run",
            ConfirmAction::AcceptHostKey { .. } => "btn-accept",
        }
    }
}

/// One open tunnel, as the panels know about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelInfo {
    pub id: u64,
    /// `127.0.0.1:8080 → db.internal:5432 on mb@jump:22`.
    pub description: String,
    /// Connections carried, open now, and refused by the far end.
    pub served: u64,
    pub live: u64,
    pub failed: u64,
}

/// Two files with the same name, one in each panel, waiting to be read.
///
/// The sizes come from the listings that are already on screen, so a pair whose sizes differ
/// is settled without opening anything — which is most of them, and the reason a comparison
/// of two build trees finishes in the time it takes to read the few files that match.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComparePair {
    pub name: String,
    pub left: VPath,
    pub right: VPath,
    pub left_size: u64,
    pub right_size: u64,
}

/// A yes/no question that carries what it will do on `Enter`.
#[derive(Debug, Clone, PartialEq)]
pub struct Confirm {
    pub title: String,
    pub lines: Vec<String>,
    pub action: ConfirmAction,
    /// Which button is on: 0 does it, 1 cancels. It starts on the action, as FAR's does —
    /// somebody who opened this dialog meant to do the thing — and `Esc` is always the way
    /// out whichever button is lit.
    pub button: usize,
}

impl Confirm {
    /// The two buttons: what this will do, in a word, and the way out.
    pub fn buttons(&self, i18n: &I18n) -> Vec<String> {
        vec![i18n.t(self.action.verb_key()), i18n.t("btn-cancel")]
    }
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

/// What to search for (SPEC 06 §1). Plain data on purpose: the engine lives in `fcmd-find`,
/// and this is what the dialog produces and the worker turns into a query, so neither the
/// dialog nor the screen depends on the engine's own types.
#[derive(Debug, Clone, PartialEq)]
pub struct FindSpec {
    /// Masks, in the grammar of `fcmd-core`'s filter: a list, `*`, `?`, ranges, groups.
    pub mask: String,
    /// Text or a hex pattern the file must contain. Empty means "match on the name alone".
    pub text: String,
    pub case_sensitive: bool,
    pub whole_words: bool,
    /// The text above is a hex byte pattern rather than characters.
    pub hex: bool,
    /// Files that do *not* contain it, which FAR offers and is surprisingly useful.
    pub not_containing: bool,
    /// A size condition in the grammar of `fcmd-core`'s filter: `100M` is "from a hundred
    /// megabytes up", `1M-1G` a range, `-4K` an upper bound. Empty means any size. This is
    /// what finds the files worth deleting when a disk fills up.
    pub size: String,
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
            size: String::new(),
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

/// The review table of a synchronisation (SPEC 03 §10).
///
/// It is a screen rather than a dialog because it is the whole of what was decided, and the
/// point of it is that nothing has happened yet: every row can be changed, and only `F2` sets
/// any of it going.
#[derive(Debug)]
pub struct SyncView {
    pub id: u64,
    pub left: VPath,
    pub right: VPath,
    pub pairs: Vec<fcmd_jobs::sync::Pair>,
    pub cursor: usize,
    pub top: usize,
    /// Where the walk is, while it is running.
    pub scanning: Option<String>,
    pub done: bool,
    pub stopped: bool,
    pub cancel: CancelToken,
}

/// The palette editor: every colour the interface asks for, and what it is set to.
///
/// It edits the *live* theme, so every change is on the screen the moment it is made — which
/// is the only way to judge a colour. `F2` writes it into the configuration file as a
/// `[[theme]]` block under the theme's own id, which is the path the theme loader already has
/// for retuning a built-in without renaming it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteView {
    /// Every key, in the order the interface declares them — which groups them by what they
    /// colour, and so is the order to read them in.
    pub keys: Vec<String>,
    pub cursor: usize,
    pub top: usize,
    /// Whether anything has been changed, so leaving can say the file has not been written.
    pub dirty: bool,
}

impl PaletteView {
    pub fn new() -> PaletteView {
        PaletteView {
            keys: fcmd_ui::Theme::keys().map(|k| k.to_string()).collect(),
            cursor: 0,
            top: 0,
            dirty: false,
        }
    }

    pub fn move_cursor(&mut self, delta: isize) {
        if self.keys.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.keys.len() as isize - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, last) as usize;
    }

    pub fn scroll_into_view(&mut self, rows: usize) {
        let rows = rows.max(1);
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + rows {
            self.top = self.cursor + 1 - rows;
        }
    }

    pub fn key(&self) -> Option<&str> {
        self.keys.get(self.cursor).map(String::as_str)
    }
}

impl Default for PaletteView {
    fn default() -> PaletteView {
        PaletteView::new()
    }
}

impl SyncView {
    pub fn move_cursor(&mut self, delta: isize) {
        if self.pairs.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.pairs.len() as isize - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, last) as usize;
    }

    pub fn scroll_into_view(&mut self, rows: usize) {
        let rows = rows.max(1);
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + rows {
            self.top = self.cursor + 1 - rows;
        }
    }

    /// What the foot of the table says: how many rows do something, and how much they move.
    pub fn totals(&self) -> (usize, usize, u64) {
        use fcmd_jobs::sync::Action;
        let mut copies = 0;
        let mut deletes = 0;
        let mut bytes = 0;
        for pair in &self.pairs {
            match pair.action {
                Action::Right | Action::Left => {
                    copies += 1;
                    bytes += pair.bytes();
                }
                Action::DeleteRight | Action::DeleteLeft => deletes += 1,
                Action::Skip => {}
            }
        }
        (copies, deletes, bytes)
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
    /// A job's progress box. Boxed like the other big ones: `JobView` carries the whole of a
    /// job's progress and the question it may be asking, where most of `Mode` is a word.
    Job(Box<JobView>),
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
    /// The synchronisation table, before anything has happened (SPEC 03 §10).
    Sync(Box<SyncView>),
    /// The palette editor: the live theme, one colour per row (SPEC 10 §5).
    Palette(Box<PaletteView>),
    /// Recording has stopped and the macro is waiting for the key it will answer to: the next
    /// key pressed is the answer, which is how FAR assigns one (SPEC 08 §5).
    MacroKey {
        steps: Vec<fcmd_cmd::Step>,
    },
    /// `Ctrl+O`: the output of the commands that have run, full screen.
    UserScreen {
        scroll: usize,
    },
    /// `Ctrl+Alt+E`: a real terminal with a shell in it (SPEC 06 §4).
    ///
    /// It is a screen like a viewer: stepping away leaves the program running, and `F12`
    /// brings it back. What it is *not* is a mode with a key map — almost every key belongs to
    /// the program, and a key bar that offered its own would be lying.
    Term(Box<fcmd_term::Terminal>),
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
            // The whole path, not the name: two files called `README.md` open at once are two
            // screens, and a list that calls them both `README.md` is a list one cannot use.
            Mode::Viewer(v) => Some(format!("View  {}", v.path)),
            Mode::Editor(e) => Some(format!(
                "Edit  {}",
                e.path.as_ref().map(|p| p.to_string()).unwrap_or(e.name())
            )),
            Mode::Hex(e) => Some(format!(
                "Hex   {}",
                e.path.as_ref().map(|p| p.to_string()).unwrap_or(e.name())
            )),
            // What the program called itself, if it did: `ssh` sets a title and that is the
            // one thing that tells two terminals apart in a list.
            Mode::Term(t) => Some(format!(
                "Term  {}",
                t.with_screen(|s| s.title.clone()).unwrap_or(t.name.clone())
            )),
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
    pub scope: fcmd_cmd::Scope,
    pub program: std::path::PathBuf,
    pub needs: Vec<String>,
    pub timeout: Option<u64>,
    /// What to ask before running it, when the plugin says it should be asked.
    pub confirm: Option<String>,
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
    /// The file-colouring groups: the configuration's, then FAR's own (SPEC 02 §5).
    pub highlight: fcmd_ui::Highlight,
    /// The `[[sort_group]]` bands, compiled (SPEC 02 §4). One list for the whole program:
    /// each panel holds a handle to it.
    pub sort_groups: std::sync::Arc<fcmd_core::sort::SortGroups>,
}

pub struct App {
    pub cfg: Config,
    pub paths: Paths,
    pub i18n: I18n,
    pub keymap: Keymap,
    pub themes: ThemeSet,
    pub theme: Theme,
    pub panels: [Panel; 2],
    /// Each side's tabs (SPEC 02 §1.6). The tab that is on screen is `panels[side]`; the
    /// others are parked in the row, so nothing that works on a panel has to know that tabs
    /// exist. A side always has at least one.
    pub tabs: [crate::tabs::Row; 2],
    /// The folders kept to hand (SPEC 02 §11), as the state file holds them.
    pub bookmarks: Vec<fcmd_config::Bookmark>,
    /// Which colour group the open dialog is editing, or `None` for a new one
    /// (SPEC 10 §3.11).
    pub highlight_at: Option<usize>,
    /// The files that have been read or edited, oldest last — FAR's `Alt+F11`.
    ///
    /// Newest first in the list because that is the order somebody reaching back wants, and
    /// kept that way in the field so that pushing is a `insert(0, …)` nobody has to think
    /// about. It survives the session through the state file.
    pub file_history: Vec<String>,
    pub active: usize,
    pub mode: Mode,
    pub status: String,
    pub status_error: bool,
    /// The modifiers held down right now, for the key bar. Only terminals that speak the
    /// keyboard protocol ever report this, so on the others it stays empty and the bar does
    /// not change — which is what it did before.
    pub held: Mods,
    /// What the attributes dialog is about. The dialog is open while the selection could
    /// change under it, so what it will touch is settled when it opens.
    attr_targets: Vec<VPath>,
    /// The same, for the copy dialog.
    copy_sources: Vec<VPath>,
    /// What the copy dialog was last answered with, so the next one opens on it. FAR does
    /// the same: the rules somebody set once are usually the rules they want again.
    last_copy: CopyChoices,
    /// Jobs waiting for the one on screen to finish, for a copy to several destinations.
    /// One at a time and in order, because two jobs writing at once would fight over the
    /// progress box and over the disk.
    queued_jobs: std::collections::VecDeque<JobSpec>,
    /// What to delete once the archive is written, when the dialog asked for it — or when
    /// `F6` put the files into the archive, which is a move and not a copy. It is held here
    /// rather than in the job because the delete only happens if the archive was written.
    pack_delete: Option<Vec<VPath>>,
    pub quit: bool,
    pub requests: Vec<Request>,
    /// Panel geometry from the last frame, so key handling knows how far a page is.
    pub rows: usize,
    pub columns: usize,
    /// Where everything clickable was drawn on the last frame (`crate::hit`). Written by the
    /// renderer, read by the mouse.
    pub layout: crate::hit::Layout,
    /// The last press, for telling a double click from two single ones.
    last_click: Option<(u16, u16, std::time::Instant)>,
    /// Size of the viewer's text area, from the last frame.
    pub viewer_size: (u16, u16),
    /// Size of a terminal's grid, from the last frame — the room a screen has once the key
    /// bar and the status line have taken theirs.
    pub term_area: (u16, u16),
    /// What to call when something that is not a key needs the interface to draw again: a
    /// terminal's program writing output nobody asked for. `None` in a test, which draws
    /// when it chooses to.
    pub wake: Option<std::sync::Arc<dyn Fn() + Send + Sync>>,
    /// Remembered across files, like every commander does.
    pub search_mode: SearchMode,
    /// Only whole words count as a hit. FAR asks it in the dialog; so do we.
    pub search_whole_words: bool,
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
    /// What `Alt+F6` is making a link to, while its dialog is open.
    pending_link: Option<VPath>,
    /// The fetch we are waiting for, if any.
    fetch_id: Option<u64>,
    /// The comparison by content that is running, and the token that stops it. `Esc` on a
    /// panel with an empty command line stops it, which is the only way out of a comparison
    /// of ten thousand files that was started by mistake.
    /// The tunnels that are open, as the worker last described them. The tunnels themselves
    /// live on the runtime, where their listeners and their copying do; this is what the list
    /// shows and what a status line counts.
    pub tunnels: Vec<TunnelInfo>,
    compare: Option<(u64, CancelToken)>,
    /// The checksum run that is going, and the token that stops it. `Esc` on a panel with an
    /// empty command line stops it, the way it stops a comparison.
    sum: Option<(u64, CancelToken)>,
    /// What the checksum dialog was last answered with, so the next one opens on it.
    last_sum: fcmd_jobs::Algorithm,
    /// Which kind of link `Alt+F6` offers first: the last one that was made.
    last_link: fcmd_jobs::LinkKind,
    /// The same for the synchronisation dialog: the rules somebody set once are the rules
    /// they want again.
    last_sync: fcmd_jobs::sync::Options,
    /// Files with no counterpart on the other side, kept while the reading runs: they are
    /// differences too, and they are selected with the rest when it finishes.
    compare_alone: Vec<String>,
    /// How many pairs the running comparison was given, for the sentence at the end.
    compare_pairs: usize,
    /// The command line under the panels (SPEC 06 §7).
    pub cmd: CmdLine,
    /// What the commands that have run printed, for `Ctrl+O`. Capped.
    pub user_screen: Vec<String>,
    /// Which panel is hidden, if either: `Ctrl+F1` and `Ctrl+F2` give the screen to one.
    pub hidden: Option<usize>,
    /// `Ctrl+O`: both panels off, the screen given to what the commands printed. Everything
    /// else keeps working — the command line, the menus, the keys — which is the whole point
    /// of hiding the panels rather than opening a screen of their own.
    pub panels_off: bool,
    /// How many lines back from the end of the user screen it is scrolled, for the pages
    /// `PgUp`/`PgDn` turn while the panels are off.
    pub user_scroll: usize,
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
    /// Recorded macros (SPEC 08 §5), as `fcmd-macros.toml` holds them.
    pub macros: fcmd_cmd::Macros,
    /// The steps gathered so far, while `Ctrl+.` is recording, and where the recording
    /// started: a run of editor keys would mean something else entirely in a panel, so the
    /// scope is part of what is recorded rather than assumed afterwards.
    pub recording: Option<Vec<fcmd_cmd::Step>>,
    recording_scope: fcmd_cmd::Scope,
    /// A macro that has its key and its description and is waiting to be told how to play.
    macro_pending: Option<Box<fcmd_cmd::Macro>>,
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
    pub last_pack: fcmd_arc::Format,
    /// How hard the compressor works when an archive is rebuilt. The format's own level is
    /// not readable from an archive, so the one the last dialog chose is the best guess.
    pub last_pack_level: fcmd_arc::Level,
    /// The same for the replace dialog.
    pub last_replace: fcmd_jobs::ReplaceOptions,
    /// A replacement runs twice: once to count, and then, if the user says so, for real. This
    /// is the real one, waiting for the count to come back.
    pending_replace: Option<JobSpec>,
    /// Which group gives a file its colour. Compiled once: it parses mask lists.
    pub highlight: fcmd_ui::Highlight,
    /// The `[[sort_group]]` bands, as every panel and every new tab gets them.
    pub sort_groups: std::sync::Arc<fcmd_core::sort::SortGroups>,
    /// The user menu as it is on screen: one level per list, the top one first and the submenu
    /// one is inside last, each with the title it is shown under. Kept as a stack so `Esc` in
    /// a submenu goes back to the menu it was opened from rather than out of the menu.
    menu_levels: Vec<(String, Vec<fcmd_config::usermenu::Entry>)>,
    /// The lines of the chosen menu entry still to run, **unexpanded**: each one gets the
    /// substitutions and the questions a hand-typed line gets, at the moment it runs.
    menu_queue: Vec<String>,
    /// What `Ctrl+G` has left to run, one command per file. They go one at a time: a hundred
    /// copies of a converter started at once is not what anybody asked for.
    apply_queue: Vec<String>,
    /// How many commands that run started with, for counting through it.
    apply_total: usize,
    /// The shell command we are waiting for, if any, and the line that started it.
    shell: Option<(u64, String)>,
    /// Whether the screen showing what the command prints was opened by the command itself,
    /// and so should be closed again when it ends. `Esc` clears it: a user who has gone back
    /// to the panels has said what they want, and the output keeps arriving behind them.
    shell_screen: bool,
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
            highlight,
            sort_groups,
        } = start;
        // The criteria chain, and what could not be read: said out loud below rather than
        // silently sorted by name, which is what a mistyped key would otherwise look like.
        let mut problems: Vec<String> = Vec::new();
        let chain = sort_chain(&cfg.panel.sort_chain, &mut problems);
        let sort = Sort {
            chain,
            key: parse_sort(&cfg.panel.sort),
            reverse: cfg.panel.reverse,
            dirs_first: cfg.panel.dirs_first,
            selected_first: cfg.panel.selected_first,
            natural: cfg.panel.natural_sort,
            // Writing the blocks is what turns them on: a list of bands nobody asked to see
            // would be a listing that reordered itself for no reason anybody could point at.
            // `Shift+F11` is how they go off again.
            groups: !sort_groups.is_empty(),
            ..Sort::default()
        };
        let view = ViewMode::parse(&cfg.panel.view).unwrap_or(ViewMode::Full);
        let hidden = cfg.panel.show_hidden;
        // The macros, read before the struct is built because `paths` moves into it. A file
        // that does not parse is said out loud rather than silently ignored: a macro half
        // read is a key that does something unexpected.
        let mut macro_error: Option<String> = None;
        let macros = match std::fs::read_to_string(&paths.macros_file) {
            Ok(text) => match fcmd_cmd::Macros::parse(&text) {
                Ok(macros) => macros,
                Err(e) => {
                    macro_error = Some(e.to_string());
                    fcmd_cmd::Macros::default()
                }
            },
            Err(_) => fcmd_cmd::Macros::default(),
        };
        let mut app = App {
            cfg,
            paths,
            i18n,
            keymap,
            themes,
            theme,
            panels: [
                Panel::new(left, sort, view, hidden).with_groups(&sort_groups),
                Panel::new(right, sort, view, hidden).with_groups(&sort_groups),
            ],
            tabs: [crate::tabs::Row::one(), crate::tabs::Row::one()],
            bookmarks: Vec::new(),
            file_history: Vec::new(),
            highlight_at: None,
            active: 0,
            mode: Mode::Normal,
            status: String::new(),
            status_error: false,
            held: Mods::NONE,
            attr_targets: Vec::new(),
            copy_sources: Vec::new(),
            last_copy: CopyChoices::default(),
            queued_jobs: std::collections::VecDeque::new(),
            pack_delete: None,
            quit: false,
            requests: Vec::new(),
            rows: 20,
            columns: 1,
            layout: crate::hit::Layout::default(),
            last_click: None,
            viewer_size: (80, 20),
            term_area: (80, 20),
            wake: None,
            search_mode: SearchMode::Plain,
            search_whole_words: false,
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
            pending_link: None,
            fetch_id: None,
            tunnels: Vec::new(),
            compare: None,
            sum: None,
            last_sum: fcmd_jobs::Algorithm::default(),
            last_link: fcmd_jobs::LinkKind::SymlinkRelative,
            last_sync: fcmd_jobs::sync::Options {
                // The two a sync is almost always asked for, and neither can lose anything:
                // walk the trees, and leave alone what the other side does not have.
                recursive: true,
                ..fcmd_jobs::sync::Options::default()
            },
            compare_alone: Vec::new(),
            compare_pairs: 0,
            cmd: CmdLine::new(),
            user_screen: Vec::new(),
            shell_screen: false,
            hidden: None,
            panels_off: false,
            user_scroll: 0,
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
            recording_scope: fcmd_cmd::Scope::Panel,
            macro_pending: None,
            record_key: None,
            playing: 0,
            background: Vec::new(),
            last_asm: None,
            last_pack: fcmd_arc::Format::Zip,
            last_pack_level: fcmd_arc::Level::Normal,
            last_replace: fcmd_jobs::ReplaceOptions::default(),
            pending_replace: None,
            highlight,
            sort_groups,
            menu_levels: Vec::new(),
            menu_queue: Vec::new(),
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
        if let Some(problem) = problems.first() {
            app.set_error(problem.clone());
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
            // Nothing to do: the loop draws after every event, which is the whole request.
            Event::Redraw => {}
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
            Event::FreeSpace { panel, disk } => {
                if let Some(p) = self.panels.get_mut(panel) {
                    p.disk = disk;
                }
            }
            Event::Connected {
                panel,
                site,
                outcome,
            } => self.connection_result(panel, site, outcome),
            Event::Disconnected { key, connected } => {
                self.connected = connected;
                let host = match key.split_once("://") {
                    Some((_, authority)) => authority.to_string(),
                    None => key.clone(),
                };
                let msg = self
                    .i18n
                    .t_args("net-disconnected", &[("host", host.into())]);
                self.set_status(msg);
                // A panel left on a closed connection goes home instead of showing a corpse.
                for panel in 0..2 {
                    if fcmd_net::Site::key_of(&self.panels[panel].cwd).as_deref() == Some(&key) {
                        // A tab locked to a directory on that connection cannot keep its
                        // promise: the lock comes off rather than the panel staying on a
                        // directory that is no longer reachable.
                        self.panels[panel].locked = false;
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
                            button: 0,
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
                    Ok(entries) => {
                        self.panels[panel].set_listing(entries, generation);
                        // The names are new, so what was known about the old ones is not
                        // about these. Asked for again while the owners view is the one on
                        // screen, and otherwise left empty, which costs nothing.
                        self.panels[panel].owners.clear();
                        self.ask_for_owners(panel);
                        self.panels[panel].descriptions = Default::default();
                        self.ask_for_descriptions(panel);
                    }
                    Err(e) => {
                        self.panels[panel].loading = false;
                        self.panels[panel].error = Some(e.clone());
                        self.set_error(e);
                    }
                }
            }
            Event::Descriptions {
                panel,
                generation,
                descriptions,
            } => {
                if panel >= self.panels.len() || self.panels[panel].generation != generation {
                    return;
                }
                self.panels[panel].descriptions = descriptions;
                // A `Ctrl+Z` dialog that opened before the file had been read: what it was
                // asking about is known now, and an empty box is the one thing that may be
                // filled in without taking away something the user has typed.
                if let Mode::Prompt(prompt) = &self.mode {
                    if let PromptKind::Describe { name } = &prompt.kind {
                        if prompt.text.is_empty() && panel == self.active {
                            if let Some(text) = self.panels[panel].descriptions.get(name) {
                                let text = fcmd_core::descriptions::one_line(text);
                                if let Mode::Prompt(prompt) = &mut self.mode {
                                    prompt.text = text;
                                }
                            }
                        }
                    }
                }
            }
            Event::Owners {
                panel,
                generation,
                owners,
            } => {
                // A directory we have already left, or a listing that has been read again:
                // the names these belong to are not the names on the screen.
                if panel >= self.panels.len() || self.panels[panel].generation != generation {
                    return;
                }
                self.panels[panel].owners = owners;
            }
            Event::DirChanged { panel } => {
                if panel >= self.panels.len() || !self.cfg.panel.auto_refresh {
                    return;
                }
                let p = &self.panels[panel];
                // Only a local directory is watched, so only a local directory can be told
                // about; a panel that is reading already has the change coming. A directory
                // that has just been removed would re-read straight into an error dialog,
                // which is not what noticing a change is for: leave it on screen as it was.
                if p.temp.is_some()
                    || !p.cwd.is_local()
                    || p.cwd.in_container()
                    || p.loading
                    || !p.cwd.os().is_dir()
                {
                    return;
                }
                // `set_listing` keeps the cursor on the name it was on and the selection on
                // the names that are still there, so this moves nothing under the user.
                self.read_panel(panel);
            }
            Event::Tunnels { items } => {
                let opened = items.len() > self.tunnels.len();
                self.tunnels = items;
                if opened {
                    if let Some(last) = self.tunnels.last() {
                        let msg = self
                            .i18n
                            .t_args("tunnel-open", &[("what", last.description.clone().into())]);
                        self.set_status(msg);
                    }
                }
                // A list that is on screen is refreshed where it stands.
                if matches!(&self.mode, Mode::Pick(p) if p.kind == PickKind::Tunnels) {
                    self.open_tunnels();
                }
            }
            Event::TunnelFailed { spec, error } => {
                let msg = self.i18n.t_args(
                    "tunnel-failed",
                    &[("what", spec.into()), ("why", error.into())],
                );
                self.set_error(msg);
            }
            Event::SyncProgress { id, at, pairs } => {
                let Mode::Sync(view) = &mut self.mode else {
                    return;
                };
                if view.id != id {
                    return;
                }
                view.scanning = Some(at.clone());
                let msg = self.i18n.t_args(
                    "sync-walking",
                    &[("at", at.clone().into()), ("pairs", (pairs as i64).into())],
                );
                self.set_status(msg);
            }
            Event::SyncPlanned {
                id,
                pairs,
                cancelled,
                problem,
            } => {
                let Mode::Sync(view) = &mut self.mode else {
                    return;
                };
                if view.id != id {
                    return;
                }
                if let Some(problem) = problem {
                    self.mode = Mode::Normal;
                    self.set_error(problem.clone());
                    return;
                }
                view.pairs = pairs.clone();
                view.done = true;
                view.stopped = cancelled;
                view.scanning = None;
                view.cursor = 0;
                view.top = 0;
                let (copies, deletes, bytes) = view.totals();
                let msg = self.i18n.t_args(
                    "sync-planned",
                    &[
                        ("copies", (copies as i64).into()),
                        ("deletes", (deletes as i64).into()),
                        ("bytes", fcmd_core::format_size(bytes).into()),
                    ],
                );
                self.set_status(msg);
            }
            Event::SumProgress {
                id,
                name,
                done,
                total,
            } => {
                if self.sum.as_ref().map(|(running, _)| *running) != Some(id) {
                    return;
                }
                let msg = self.i18n.t_args(
                    "sum-working",
                    &[
                        ("name", name.clone().into()),
                        ("done", (done as i64).into()),
                        ("total", (total as i64).into()),
                    ],
                );
                self.set_status(msg);
            }
            Event::SumsDone {
                id,
                written,
                errors,
                cancelled,
            } => {
                if self.sum.as_ref().map(|(running, _)| *running) != Some(id) {
                    return;
                }
                self.sum = None;
                self.finish_sums(&written, &errors, cancelled);
            }
            Event::VerifyDone {
                id,
                verdicts,
                cancelled,
                problem,
            } => {
                if self.sum.as_ref().map(|(running, _)| *running) != Some(id) {
                    return;
                }
                self.sum = None;
                self.finish_verify(&verdicts, cancelled, problem.clone());
            }
            Event::CompareProgress { id, done, total } => {
                if self.compare.as_ref().map(|(running, _)| *running) != Some(id) {
                    return;
                }
                let msg = self.i18n.t_args(
                    "compare-contents-working",
                    &[
                        ("done", (done as i64).into()),
                        ("total", (total as i64).into()),
                    ],
                );
                self.set_status(msg);
            }
            Event::CompareDone {
                id,
                differ,
                line_endings,
                unreadable,
                stopped,
            } => {
                if self.compare.as_ref().map(|(running, _)| *running) != Some(id) {
                    return;
                }
                self.compare = None;
                self.apply_compare(&differ, &line_endings, unreadable, stopped);
            }
            Event::ListingCut {
                panel,
                generation,
                note,
            } => {
                if self.panels[panel].generation != generation {
                    return;
                }
                // The listing carries the fact, so the footer can keep saying `(partial)`
                // after the message has gone; the message says why, once.
                self.panels[panel].listing.truncated = true;
                self.panels[panel].message = Some(note.clone());
                self.set_status(note);
            }
            Event::ShellOutput { id, text } => {
                if self.shell.as_ref().map(|(running, _)| *running) != Some(id) {
                    return; // a command we already stopped waiting for
                }
                // The window counts back from the end, so with the panels off and nothing
                // scrolled the new lines are simply there: that is the streaming.
                self.append_user_screen(&text);
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
                // The output itself has already arrived, line by line; what comes with the
                // end is only what the run has to add — a spawn failure, where the output
                // was written, that there was more of it than we would keep.
                self.append_user_screen(&output);
                if self.shell_screen && self.apply_queue.is_empty() && self.menu_queue.is_empty() {
                    // The panels come back when the command ends, as they do in FAR, and
                    // `Ctrl+O` is how what it printed is read again.
                    self.shell_screen = false;
                    self.panels_off = false;
                }
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
                // A menu entry with several lines is the same promise: the next one runs when
                // this one is done, and a failure stops the list rather than carrying on
                // through it. The message above has already said which command failed.
                if !self.menu_queue.is_empty() {
                    if matches!(status, Some(code) if code != 0) {
                        self.menu_queue.clear();
                    } else {
                        self.run_menu_next();
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
                        ("bytes", fcmd_core::format_size(bytes).into()),
                        ("files", (files as i64).into()),
                    ],
                );
                self.panels[panel].message = Some(msg.clone());
                self.set_status(msg);
            }
        }
    }

    /// Text pasted into the terminal, which arrives whole rather than as keystrokes because
    /// the terminal is asked for bracketed paste (SPEC 00 §6).
    ///
    /// A paste is deliberately *not* treated as typing. A command copied from a page comes
    /// with its newline still on the end, and letting that newline be `Enter` would run it
    /// before it could be read; a path copied from a browser's address bar arrives the same
    /// way. So everything that is one line — the command line, a dialog row, a prompt — takes
    /// the first line and says how many more there were, and only the places that are really
    /// many lines, the editor and a real terminal, take the whole of it.
    fn paste(&mut self, text: &str) {
        // Windows clipboards carry CRLF and some terminals pass the CR through; a stray
        // carriage return in a file or on the command line is never what was meant.
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let dropped = match &mut self.mode {
            Mode::Prompt(p) => {
                let (first, dropped) = first_line(&text);
                p.text.push_str(first);
                let whole = p.text.clone();
                self.prompt_changed(&whole);
                dropped
            }
            Mode::Form(form) => form.insert_str(&text),
            Mode::Editor(editor) => {
                let outcome = editor.insert_text(&text);
                if let Err(e) = outcome {
                    self.set_error(e.to_string());
                }
                0
            }
            // A terminal is a terminal: the shell in it does with a paste whatever it does
            // with one, newlines and all, and `Enter` there is a carriage return.
            Mode::Term(term) => {
                let bytes = text.replace('\n', "\r").into_bytes();
                if let Err(e) = term.send(&bytes) {
                    self.set_error(e.to_string());
                }
                0
            }
            // On the panels the paste belongs to the command line, which is where text goes
            // when nothing else is open.
            Mode::Normal if self.cfg.ui.command_line => {
                let (first, dropped) = first_line(&text);
                self.cmd.insert_str(first);
                dropped
            }
            _ => 0,
        };
        if dropped > 0 {
            let msg = self
                .i18n
                .t_args("paste-one-line", &[("count", (dropped as i64).into())]);
            self.set_status(msg);
        }
    }

    pub fn handle_input(&mut self, input: Input) {
        match input {
            Input::Key(key) => self.handle_key(key),
            Input::Paste(text) => self.paste(&text),
            Input::Mouse(mouse) => self.handle_mouse(mouse),
            Input::Mods(mods) => self.held = mods,
            // A terminal's program has to be told, or a full-screen one stays the shape of
            // the window it started in. Everything else works out its own size while drawing.
            Input::Resize(w, h) => {
                let chrome = u16::from(self.cfg.ui.key_bar)
                    + u16::from(self.cfg.ui.status_bar)
                    + u16::from(self.cfg.ui.command_line);
                self.term_resized(w, h.saturating_sub(chrome.max(1)));
            }
            Input::Eof => self.quit = true,
        }
    }

    /// What the mouse did, answered against the layout the last frame recorded.
    ///
    /// The rule throughout is that the mouse does what a key already does — it moves the
    /// cursor, it presses a function key, it opens a menu — rather than having powers of its
    /// own. That keeps one path through the application: a click ends up in `run`, where the
    /// macro recorder, the confirmations and the error reporting already live.
    fn handle_mouse(&mut self, mouse: fcmd_tui::Mouse) {
        use fcmd_tui::{Button, MouseKind};
        let (col, row) = (mouse.col, mouse.row);
        match mouse.kind {
            MouseKind::ScrollUp => self.wheel(col, row, -1),
            MouseKind::ScrollDown => self.wheel(col, row, 1),
            MouseKind::Press(Button::Left) => {
                let double = self.is_double_click(col, row);
                self.click(col, row, double);
            }
            // The right button selects, which is what both FAR and Midnight Commander do with
            // it. Nothing here opens a context menu, so there is nothing to take it away from.
            MouseKind::Press(Button::Right) => self.right_click(col, row),
            // A drag over the panels extends nothing yet, and a release ends whatever the
            // press did. Said here rather than left to a catch-all so that the day a drag
            // means something there is one place to put it.
            MouseKind::Press(Button::Middle) | MouseKind::Release(_) | MouseKind::Drag(_) => {}
        }
    }

    /// Two presses in the same cell inside a short while: a double click, which the terminal
    /// does not report and the application has to notice for itself.
    fn is_double_click(&mut self, col: u16, row: u16) -> bool {
        const WINDOW: std::time::Duration = std::time::Duration::from_millis(400);
        let now = std::time::Instant::now();
        let double = matches!(
            self.last_click,
            Some((c, r, when)) if c == col && r == row && now.duration_since(when) < WINDOW
        );
        // A double click does not start a third: the next press is a fresh single one, or a
        // slow triple click would open the same file twice.
        self.last_click = if double { None } else { Some((col, row, now)) };
        double
    }

    fn click(&mut self, col: u16, row: u16, double: bool) {
        let Some(hit) = self.layout.hit(col, row) else {
            return;
        };
        match hit {
            crate::hit::Hit::Row { side, index } => {
                self.focus_side(side);
                self.panel_mut().cursor_to(index);
                self.scroll();
                if double {
                    // The same as pressing Enter on the row, which is what it looks like it
                    // should do: into a directory, into an archive, or open the file.
                    self.run(Command::NavEnter);
                }
            }
            crate::hit::Hit::Panel { side } => self.focus_side(side),
            crate::hit::Hit::Key(n) => {
                // The modifiers that are held decide which key bar is on the screen, so a
                // click on the bar presses the key that is written on it.
                self.handle_key(Key {
                    code: KeyCode::F(n),
                    mods: self.held,
                });
            }
            crate::hit::Hit::Menu(which) => {
                let item = self.first_item(which);
                self.mode = Mode::Menu { menu: which, item };
            }
            crate::hit::Hit::CommandLine => {}
            crate::hit::Hit::ListRow(index) => match &mut self.mode {
                Mode::Pick(pick) => {
                    pick.cursor = index.min(pick.rows.len().saturating_sub(1));
                    if double {
                        self.apply_pick();
                    }
                }
                Mode::Menu { menu, .. } => {
                    let menu = *menu;
                    // One click is enough in a menu: the row under the pointer is the one
                    // being asked for, and a dropdown that needed two would feel broken.
                    self.mode = Mode::Menu { menu, item: index };
                    self.run_menu_item(menu, index);
                }
                _ => {}
            },
            crate::hit::Hit::Text { col, row } => self.click_text(col, row),
        }
    }

    /// The right button toggles the selection of the row under it, and makes that panel the
    /// active one first — selecting in the panel one is not looking at is a selection one
    /// then loses track of.
    fn right_click(&mut self, col: u16, row: u16) {
        let Some(crate::hit::Hit::Row { side, index }) = self.layout.hit(col, row) else {
            return;
        };
        self.focus_side(side);
        self.panel_mut().cursor_to(index);
        self.scroll();
        if let Some(entry) = self.panel().current() {
            if entry.is_selectable() {
                let id = entry.id;
                self.panel_mut().selection.toggle(id);
                self.panel_mut().rebuild();
            }
        }
    }

    /// The wheel scrolls whatever is under the pointer, without moving the focus: looking at
    /// the other panel is not working in it.
    fn wheel(&mut self, col: u16, row: u16, by: isize) {
        // In the editor, the viewer and a list the wheel is the arrow keys: three rows at a
        // time, which is what every other program does and what a terminal's own scrollback
        // does when the mouse is not captured.
        const STEP: isize = 3;
        match self.layout.hit(col, row) {
            Some(crate::hit::Hit::Row { side, .. }) | Some(crate::hit::Hit::Panel { side }) => {
                let rows = self.rows.max(1);
                let len = self.panels[side].listing.len();
                let top = self.panels[side].top;
                let last = len.saturating_sub(rows);
                let top = (top as isize + by * STEP).clamp(0, last as isize) as usize;
                self.panels[side].top = top;
                // The cursor follows the window rather than the window following the cursor:
                // scrolling past the cursor and then typing a key would otherwise jump the
                // view back to where it was.
                let cursor = self.panels[side].cursor.clamp(top, top + rows - 1);
                self.panels[side].cursor_to(cursor);
            }
            Some(crate::hit::Hit::ListRow(_)) => {
                for _ in 0..STEP {
                    match &mut self.mode {
                        Mode::Pick(pick) => pick.move_cursor(by),
                        Mode::Menu { menu, item } => {
                            let (menu, item) = (*menu, *item);
                            let item = self.menu_step(menu, item, by > 0);
                            self.mode = Mode::Menu { menu, item };
                        }
                        _ => break,
                    }
                }
            }
            Some(crate::hit::Hit::Text { .. }) => {
                let code = if by < 0 { KeyCode::Up } else { KeyCode::Down };
                for _ in 0..STEP {
                    self.handle_key(Key {
                        code,
                        mods: Mods::NONE,
                    });
                }
            }
            _ => {}
        }
    }

    /// A click inside the editor's text puts the caret there. The viewer and the hex editor
    /// have no caret to put, so a click in them does nothing at all.
    fn click_text(&mut self, col: u16, row: u16) {
        let Some(editor) = self.mode.editor_mut() else {
            return;
        };
        let line = editor.top + row as usize;
        let line = line.min(editor.buffer.len().saturating_sub(1));
        let col = editor.left + col as usize;
        let col = col.min(editor.buffer.line_chars(line));
        editor.move_to(
            fcmd_edit::buffer::Position::new(line, col),
            false,
            fcmd_edit::BlockKind::Stream,
        );
    }

    /// Make `side` the active panel, if it is not already and if it is on the screen.
    fn focus_side(&mut self, side: usize) {
        if self.active != side && self.hidden != Some(side) {
            self.active = side;
        }
    }

    fn handle_key(&mut self, key: Key) {
        // The bar showed what this modifier does; the key has now been pressed, so it goes
        // back to the plain one until the terminal says the modifier is down again.
        self.held = Mods::NONE;
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
            steps.push(fcmd_cmd::Step::Press(key));
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
            Mode::Sync(view) => {
                self.mode = Mode::Sync(view);
                self.sync_key(key);
            }
            Mode::Palette(view) => {
                self.mode = Mode::Palette(view);
                self.palette_key(key);
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
            Mode::Term(term) => {
                self.mode = Mode::Term(term);
                self.term_key(key);
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
                    let key = site.key();
                    if self.connected.contains(&key) {
                        self.mode = Mode::Normal;
                        self.requests.push(Request::Disconnect { key });
                    } else {
                        let msg = self
                            .i18n
                            .t_args("net-not-connected", &[("host", site.authority().into())]);
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
        for key in &self.connected {
            if rows.iter().any(|s| &s.key() == key) {
                continue;
            }
            if let Ok(site) = Site::parse_url(&format!("{key}/")) {
                rows.push(site);
            }
        }
        rows
    }

    fn confirm_key(&mut self, key: Key, confirm: Confirm) {
        // Two buttons, as every other dialog has: what this does, and the way out. The
        // arrows and `Tab` move between them, `Enter` takes the one that is lit.
        match key.code {
            KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => {
                if let Mode::Confirm(open) = &mut self.mode {
                    open.button = 1 - open.button.min(1);
                }
                return;
            }
            KeyCode::Enter if confirm.button != 0 => {
                self.mode = Mode::Normal;
                self.restore_editor();
                return;
            }
            _ => {}
        }
        match key.code {
            KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.mode = Mode::Normal;
                match confirm.action {
                    ConfirmAction::Job(spec) => self.start_job(*spec),
                    ConfirmAction::ArcRebuild { archive, changes } => {
                        self.start_rebuild(archive, changes)
                    }
                    ConfirmAction::DiscardEdits => {
                        self.editor_parked = None;
                        self.remote_edit = None;
                    }
                    ConfirmAction::QuitWithScreens => self.quit = true,
                    ConfirmAction::PlayMacro(which) => self.run_macro(&which),
                    ConfirmAction::RunPlugin(which) => self.start_plugin_command(which),
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
            lines: vec![body],
            action: ConfirmAction::DiscardHex,
            button: 0,
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
                        fcmd_edit::Pane::Hex => fcmd_edit::Pane::Text,
                        fcmd_edit::Pane::Text => fcmd_edit::Pane::Hex,
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
                        fcmd_edit::Pane::Hex => editor.type_hex(c),
                        fcmd_edit::Pane::Text => editor.type_char(c),
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
        // The hex editor's own scope first, then the text editor's: what the two share is
        // written once, and where they differ the hex one wins.
        if let Some(cmd) = self
            .keymap
            .resolve(&[Scope::Hex, Scope::Editor, Scope::Global], key)
        {
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

        // `Ctrl+Q` armed the literal insert: this key goes in as itself, whatever it means,
        // and is not looked up in the keymap. It is the only way a control character — or a
        // tab, where tabs are expanded — gets into a file.
        if self.mode.editor().is_some_and(|e| e.literal_next) {
            let literal = literal_of(key);
            let Some(editor) = self.mode.editor_mut() else {
                return;
            };
            editor.literal_next = false;
            match literal {
                Some(c) => {
                    let outcome = editor.insert_literal(c);
                    if let Err(e) = outcome {
                        self.set_error(e.to_string());
                    }
                }
                None => {
                    // A key with no character of its own — an arrow, a function key — is not
                    // something to insert, and saying so is better than inserting nothing.
                    let m = self.i18n.t("editor-not-a-character");
                    self.set_error(m);
                }
            }
            return;
        }

        let handled = {
            let Some(editor) = self.mode.editor_mut() else {
                return;
            };
            let result: Result<bool, fcmd_edit::editor::EditorError> = (|| {
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
                        if viewer.mode == fcmd_view::Mode::Text {
                            viewer.set_mode(fcmd_view::Mode::Hex);
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
            KeyCode::Esc | KeyCode::F(10) | KeyCode::Char('q') if index => {
                self.mode = Mode::Normal;
                // Whatever the help was opened over is waiting behind it.
                self.restore_front_screen();
            }
            // Out of a chapter is back to the index; out of the index is out of the help.
            KeyCode::Esc | KeyCode::Left | KeyCode::Backspace | KeyCode::Char('q') => {
                go(self, Chapter::Index, 0, at)
            }
            KeyCode::F(1) | KeyCode::F(10) => {
                self.mode = Mode::Normal;
                self.restore_front_screen();
            }
            // From the page about where you are, on to the chapter that explains it.
            KeyCode::Enter | KeyCode::Right => {
                if let Chapter::Here(place) = chapter {
                    if let Some(i) = place.prose() {
                        go(self, Chapter::Prose(i), 0, at);
                    }
                    return;
                }
                if index {
                    let chapters = crate::help::chapters();
                    if let Some((chapter, _)) = chapters.get(at) {
                        go(self, *chapter, 0, at);
                    }
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
            // An arrow ends a quick search and leaves the cursor where the search put it.
            // The search has done its work by then — the row is found and under the cursor —
            // and what the arrow means next is what it means everywhere else, so the first
            // press closes the search and the one after it moves. Nothing is restored: the
            // row the search found is where the user wanted to be, which is the difference
            // between this and `Esc`.
            KeyCode::Up
            | KeyCode::Down
            | KeyCode::Left
            | KeyCode::Right
            | KeyCode::PageUp
            | KeyCode::PageDown
            | KeyCode::Home
            | KeyCode::End
                if matches!(prompt.kind, PromptKind::QuickSearch { .. }) =>
            {
                self.mode = Mode::Normal;
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
                if let Ok(masks) = fcmd_core::filter::MaskSet::parse(&text) {
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
            // `Ctrl+Z`: what the directory says about this file. An empty answer takes the
            // line away, which is how a description is removed — there is no other key for
            // it and `Ctrl+Z` on a described file showing the description is the dialog.
            PromptKind::Describe { ref name } => {
                let name = name.clone();
                let text = prompt.text.trim().to_string();
                let panel = self.active;
                let dir = self.panels[panel].cwd.clone();
                if !dir.is_local() || dir.in_container() {
                    let m = self.i18n.t("descr-not-here");
                    self.set_error(m);
                    return;
                }
                let mut descriptions = self.panels[panel].descriptions.clone();
                descriptions.set(&name, &text);
                // On screen at once, written behind: the panel shows what was typed without
                // waiting for a disk, and the file follows.
                self.panels[panel].descriptions = descriptions.clone();
                self.requests
                    .push(Request::WriteDescriptions { dir, descriptions });
                let msg = if text.is_empty() {
                    self.i18n
                        .t_args("descr-cleared", &[("name", name.clone().into())])
                } else {
                    self.i18n
                        .t_args("descr-set", &[("name", name.clone().into())])
                };
                self.set_status(msg);
            }
            PromptKind::Tunnel => {
                let text = prompt.text.trim().to_string();
                if text.is_empty() {
                    return;
                }
                // Parsed here rather than in the worker: a shape that is not a tunnel is a
                // typing mistake, and it should be said while the dialog is still on screen
                // and not after a round trip through a thread.
                if let Err(e) = fcmd_net::TunnelSpec::parse(&text) {
                    self.set_error(e.to_string());
                    return;
                }
                let Some(authority) = self.panel().cwd.authority().map(str::to_string) else {
                    let m = self.i18n.t("tunnel-needs-connection");
                    self.set_error(m);
                    return;
                };
                let id = self.next_job_id;
                self.next_job_id += 1;
                self.requests.push(Request::TunnelOpen {
                    id,
                    authority,
                    spec: text,
                });
            }
            PromptKind::Color => {
                let text = prompt.text.trim().to_string();
                // Back to the list either way: the editor is where the answer is judged, and
                // a refusal that threw the list away would be a refusal one could not fix.
                let Mode::Palette(view) = std::mem::replace(&mut self.mode, Mode::Normal) else {
                    return;
                };
                let mut view = view;
                match fcmd_ui::Color::parse(&text) {
                    Some(color) => {
                        if let Some(key) = view.key().map(str::to_string) {
                            self.theme.set_color(&key, color);
                            view.dirty = true;
                        }
                    }
                    None => {
                        let msg = self
                            .i18n
                            .t_args("palette-bad-color", &[("value", text.into())]);
                        self.set_error(msg);
                    }
                }
                self.mode = Mode::Palette(view);
            }
            PromptKind::Calculator => {
                let line = prompt.text.trim().to_string();
                match fcmd_core::calc::evaluate(&line) {
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
                self.ask_macro_play(fcmd_cmd::Macro {
                    key,
                    scope,
                    steps,
                    play: fcmd_cmd::Play::Once,
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
            PromptKind::TabName => {
                let raw = prompt.text.trim().to_string();
                self.panel_mut().name = (!raw.is_empty()).then_some(raw);
            }
            PromptKind::BookmarkName { row } => {
                // The row is the one the list was showing, and the list is built from the
                // folders in their order, so the row names the folder.
                let name = prompt.text.trim().to_string();
                let side = self.active;
                let rows = self.bookmark_rows(side);
                if let Some(path) = rows.get(row).map(|r| r.value.clone()) {
                    if let Some(b) = self.bookmarks.iter_mut().find(|b| b.path == path) {
                        b.name = name;
                    }
                }
                self.reopen_bookmarks(side, row);
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
                    Some((name, Ok(fcmd_edit::SaveOutcome::Saved { bytes, .. }))) => {
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
            // `Enter` on a quick search is FAR's: it closes the search *and* opens what was
            // found. Every keystroke has already moved the cursor, so a first `Enter` that
            // only confirmed what the screen had been showing all along would do nothing one
            // could see, and the second would be the one that worked. One key, one result.
            PromptKind::QuickSearch { .. } => {
                self.scroll();
                self.run(Command::NavEnter);
            }
            // The filter bar is the other way round: it is applied as it is typed, and
            // `Enter` means "keep it and give me the panel back".
            PromptKind::Filter => {}
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
        // And so is `archive!/inside` (SPEC 04 §4): the path of a container typed out, which
        // is what the other panel's directory reads as when that panel is inside one.
        if raw.contains("!/") || raw.contains("!\\") {
            let parsed = VPath::parse(raw);
            if parsed.in_container() {
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

    /// The names, or the whole paths, of what is selected onto the system clipboard — one per
    /// line, which is the shape every other program expects a list of files in.
    fn clip_sources(&mut self, whole_path: bool) {
        let sources = self.job_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let text: String = sources
            .iter()
            .map(|p| {
                if whole_path {
                    p.to_string()
                } else {
                    p.display_name()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        if !self.copy_to_clipboard(&text) {
            return;
        }
        let id = if whole_path {
            "clip-paths"
        } else {
            "clip-names"
        };
        let msg = self
            .i18n
            .t_args(id, &[("count", (sources.len() as i64).into())]);
        self.set_status(msg);
    }

    /// A message, with a menu label's `&` marker taken off.
    ///
    /// Several titles are the same words as a menu row — *Change drive*, *Sort modes*, the
    /// histories — and a menu row carries the letter that is its key. A dialog's frame is not
    /// a menu, so it shows the words and not the marker.
    fn plain(&self, id: &str) -> String {
        fcmd_ui::mnemonic(&self.i18n.t(id)).0
    }

    /// Ask for the owners of a panel's rows, if that is the view it is in.
    ///
    /// Nothing happens for any other view, for a panel that is a list rather than a
    /// directory, or for one on anything but a local filesystem: a remote provider has no
    /// owner to give, and a file inside an archive belongs to whoever wrote the archive.
    /// `Ctrl+Z`: the line the directory keeps about the file under the cursor, to be edited.
    ///
    /// The dialog opens on what is there, so changing a description is one key and a few
    /// characters rather than retyping it, and an empty answer takes the line away.
    fn open_describe(&mut self) {
        let panel = self.active;
        let p = &self.panels[panel];
        if !p.cwd.is_local() || p.cwd.in_container() || p.temp.is_some() {
            let m = self.i18n.t("descr-not-here");
            self.set_error(m);
            return;
        }
        let Some(entry) = p
            .current()
            .filter(|e| e.kind != fcmd_core::EntryKind::Parent)
        else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let name = entry.name_str().into_owned();
        // The file may not have been read yet: this view does not show descriptions, or the
        // worker has not answered. Read it here, where there is one directory to read and a
        // user waiting for a dialog rather than a panel waiting to be drawn.
        let text = self.panels[panel]
            .descriptions
            .get(&name)
            .map(fcmd_core::descriptions::one_line)
            .unwrap_or_default();
        // The file may not have been read: this view shows no descriptions, so nothing has
        // asked for it. Ask now — the dialog opens on what we have and fills in when the
        // answer arrives, which for a local file is before a hand reaches the keyboard.
        if text.is_empty() && self.cfg.panel.descriptions {
            let dir = self.panels[panel].cwd.clone();
            let generation = self.panels[panel].generation;
            self.requests.push(Request::Descriptions {
                panel,
                generation,
                dir,
            });
        }
        let title = self
            .i18n
            .t_args("descr-title", &[("name", name.clone().into())]);
        self.mode = Mode::Prompt(Prompt::new(PromptKind::Describe { name }, title, text));
    }

    /// Ask for a directory's `descript.ion`, when something on screen shows one.
    ///
    /// Like the owners, and for the same reason: a panel should not open a side file for a
    /// column nobody is looking at — on a network share that is a round trip per directory
    /// one walks through. What shows one is either of the two description views, or an
    /// information panel beside this one, which spells out the row under the cursor. `Ctrl+Z`
    /// asks for itself (see [`Self::open_describe`]).
    ///
    /// Local directories only for now: a description inside an archive or on a server is a
    /// read through a provider, and a backlog line.
    fn ask_for_descriptions(&mut self, panel: usize) {
        if panel >= self.panels.len() || !self.cfg.panel.descriptions {
            return;
        }
        let shown = self.panels[panel].view.shows_description()
            || self.panels[1 - panel].content == crate::panel::Content::Info;
        let p = &self.panels[panel];
        if !shown || p.temp.is_some() || !p.cwd.is_local() || p.cwd.in_container() {
            return;
        }
        self.requests.push(Request::Descriptions {
            panel,
            generation: p.generation,
            dir: p.cwd.clone(),
        });
    }

    fn ask_for_owners(&mut self, panel: usize) {
        if panel >= self.panels.len() {
            return;
        }
        let p = &self.panels[panel];
        if p.view != ViewMode::Owners
            || p.temp.is_some()
            || !p.cwd.is_local()
            || p.cwd.in_container()
            || p.listing.is_empty()
        {
            return;
        }
        let names: Vec<String> = (0..p.listing.len())
            .filter_map(|row| p.listing.row(row))
            .filter(|e| e.kind != fcmd_core::EntryKind::Parent)
            .map(|e| e.name_str().into_owned())
            .collect();
        if names.is_empty() {
            return;
        }
        self.requests.push(Request::Owners {
            panel,
            generation: p.generation,
            dir: p.cwd.clone(),
            names,
        });
    }

    /// Put text on the system clipboard, through the terminal. Returns whether the request
    /// went out, so that the caller only says what it copied when something was copied.
    fn copy_to_clipboard(&mut self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }
        if !self.cfg.ui.clipboard {
            let m = self.i18n.t("clip-off");
            self.set_error(m);
            return false;
        }
        self.requests.push(Request::Clipboard {
            text: text.to_string(),
        });
        true
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
            JobKind::Trash => "job-trash",
            JobKind::Replace => "job-replace",
            JobKind::Pack => "job-pack",
            JobKind::Test => "job-test",
            JobKind::Rebuild => "job-rebuild",
            JobKind::Attrs => "job-attrs",
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
                            ("bytes", fcmd_core::format_size(*dst_size).into()),
                        ],
                    ),
                    self.i18n.t_args(
                        "conflict-src",
                        &[
                            ("path", src.to_string().into()),
                            ("bytes", fcmd_core::format_size(*src_size).into()),
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
            Question::Error { path, error } => {
                let mut lines = vec![path.to_string(), String::new(), error.clone()];
                // A wastebasket that will not take a file is the one error here with an
                // obvious next move, so the dialog names it: skip, and use the key that
                // deletes outright.
                if matches!(&self.mode, Mode::Job(job) if job.kind == JobKind::Trash) {
                    lines.push(String::new());
                    lines.push(
                        self.i18n
                            .t_args("msg-trash-refused", &[("path", path.display_name().into())]),
                    );
                }
                (self.i18n.t("dialog-error"), lines)
            }
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
    pub fn start_job(&mut self, mut spec: JobSpec) {
        // `general.confirm_overwrite = false` means "do not ask me about a file that is
        // already there, write over it". It was a toggle in the settings dialog that changed
        // nothing: the engine's policy stayed `Ask` whatever the file said.
        //
        // Only for the policy the engine would have asked with. A dialog that chose `skip` or
        // `rename` on purpose is not overridden by a setting about asking.
        if !self.cfg.general.confirm_overwrite
            && spec.options.on_conflict == fcmd_jobs::ConflictPolicy::Ask
        {
            spec.options.on_conflict = fcmd_jobs::ConflictPolicy::Overwrite;
        }
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        let kind = spec.kind;
        self.mode = Mode::Job(Box::new(JobView {
            id,
            kind,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        }));
        self.requests.push(Request::Job { id, spec, cancel });
    }

    // ── what to run for a kind of file (SPEC 06 §5) ─────────────────────────────────────

    /// `Alt+F3` and `Alt+F4`: read or write this file with something that is not us.
    ///
    /// What an `[[assoc]]` block names comes first; with nothing to go on, the environment
    /// does — `$PAGER` for reading, `$VISUAL` then `$EDITOR` for writing, which is the
    /// convention every Unix program follows and the one a user has already set.
    fn open_external(&mut self, which: Which) {
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let name = entry.name_str().into_owned();
        let from_block = self
            .assoc_for(&name)
            .map(|a| match which {
                Which::View => a.view.clone(),
                Which::Edit => a.edit.clone(),
            })
            .filter(|cmd| !cmd.trim().is_empty());
        let from_env = || {
            let vars: &[&str] = match which {
                Which::View => &["PAGER"],
                Which::Edit => &["VISUAL", "EDITOR"],
            };
            vars.iter()
                .filter_map(|name| std::env::var(name).ok())
                .map(|v| v.trim().to_string())
                .find(|v| !v.is_empty())
                .map(|program| format!("{program} {{file}}"))
        };
        match from_block.or_else(from_env) {
            Some(cmd) => self.run_line(&cmd),
            None => {
                let msg = self.i18n.t(match which {
                    Which::View => "assoc-no-pager",
                    Which::Edit => "assoc-no-editor",
                });
                self.set_error(msg);
            }
        }
    }

    /// The command an `[[assoc]]` block gives for the file under the cursor, if one does.
    ///
    /// The first block whose mask matches wins, which is why they are a list and not a map:
    /// a user puts the narrow rule above the broad one and the order on the page is the order
    /// of the decision.
    fn assoc_for(&self, name: &str) -> Option<&fcmd_config::AssocCfg> {
        self.cfg.assocs.iter().find(|a| a.matches(name))
    }

    /// What `Enter` does to a file that is neither a directory nor an archive.
    ///
    /// Three answers in order, and the order is the whole of the behaviour: what the user
    /// said in `[[assoc]]`, then the program the file *is*, then whatever the desktop has
    /// registered for it. A key that did nothing and said nothing would be the worst of the
    /// four possible answers, so the fourth is to say there is nothing to open it with.
    fn open_file(&mut self) {
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let name = entry.name_str().into_owned();
        if let Some(cmd) = self.assoc_for(&name).map(|a| a.open.clone()) {
            if !cmd.trim().is_empty() {
                self.run_line(&cmd);
                return;
            }
        }
        // A program is run as itself. `{file}` is the path in quotes, so a name with a
        // space in it is one argument rather than two.
        let is_program = entry.flags.contains(fcmd_core::EntryFlags::EXECUTABLE)
            || cfg!(windows)
                && matches!(
                    name.rsplit_once('.').map(|(_, ext)| ext.to_ascii_lowercase()),
                    Some(ext) if matches!(ext.as_str(), "exe" | "bat" | "cmd" | "com")
                );
        let local = self.panel().current_path().is_some_and(|p| p.is_local());
        if is_program && local {
            self.run_line("{file}");
            return;
        }
        if self.cfg.general.desktop_open && local {
            self.run_line(fcmd_config::desktop_open_command());
            return;
        }
        let msg = self.i18n.t_args("assoc-none", &[("name", name.into())]);
        self.set_error(msg);
    }

    /// Everything that could open the file under the cursor, as a list to choose from.
    ///
    /// FAR's own dialog for this edits the list; ours shows what *applies here*, which is the
    /// half one reaches for with a file in front of them. Editing the list is editing
    /// `[[assoc]]` blocks in `fcmd.toml`, the way every other block in this program is edited.
    fn open_with_list(&mut self) {
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        let name = entry.name_str().into_owned();
        let mut rows: Vec<PickRow> = Vec::new();
        for assoc in self.cfg.assocs.iter().filter(|a| a.matches(&name)) {
            for cmd in [&assoc.open, &assoc.view, &assoc.edit] {
                if !cmd.trim().is_empty() && !rows.iter().any(|r| r.value == *cmd) {
                    rows.push(PickRow::plain(
                        format!("{}  —  {cmd}", assoc.label()),
                        cmd.clone(),
                        false,
                    ));
                }
            }
        }
        rows.push(PickRow::plain(
            self.i18n.t("assoc-desktop"),
            fcmd_config::desktop_open_command().to_string(),
            false,
        ));
        let title = self
            .i18n
            .t_args("assoc-open-with", &[("name", name.into())]);
        self.mode = Mode::Pick(Box::new(Pick::new(
            PickKind::OpenWith,
            title,
            rows,
            self.active,
        )));
    }

    // ── synchronising two directories (SPEC 03 §10) ─────────────────────────────────────

    /// The dialog: which way, how deep, what counts as the same, and what to do with extras.
    fn open_sync_form(&mut self) {
        let (left, right) = (self.panels[0].cwd.clone(), self.panels[1].cwd.clone());
        // Both sides are walked and one is written: a provider that cannot be read a chunk at
        // a time cannot be compared by content, so the honest limit today is this machine,
        // and it is said before the dialog opens rather than halfway through a run.
        if [&left, &right]
            .iter()
            .any(|p| !p.is_local() || p.in_container())
        {
            let m = self.i18n.t("sync-local-only");
            self.set_error(m);
            return;
        }
        if left == right {
            let m = self.i18n.t("sync-same-directory");
            self.set_error(m);
            return;
        }
        self.mode = Mode::Form(Box::new(Form::sync(
            left.to_string(),
            right.to_string(),
            &self.last_sync,
            &self.i18n,
        )));
    }

    /// What the dialog asked for: walk both trees and decide.
    /// OK in the link dialog: the name, and which of the three kinds of link to make.
    fn apply_link_form(&mut self, form: &Form) {
        let Some(target) = self.pending_link.take() else {
            return;
        };
        let raw = form.text_value("link.name");
        let raw = raw.trim();
        if raw.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let kind = match form.text_value("link.kind").as_str() {
            "hard" => fcmd_jobs::LinkKind::Hard,
            "symlink-absolute" => fcmd_jobs::LinkKind::SymlinkAbsolute,
            _ => fcmd_jobs::LinkKind::SymlinkRelative,
        };
        self.last_link = kind;
        let link = self.resolve_path(raw);
        self.requests.push(Request::Link { target, link, kind });
    }

    fn apply_sync_form(&mut self, form: &Form) {
        let filter = match form.text_value("sync.filter") {
            empty if empty.trim().is_empty() => None,
            mask => match fcmd_core::MaskSet::parse(&mask) {
                Ok(set) => Some(set),
                Err(e) => {
                    let msg = self
                        .i18n
                        .t_args("msg-mask-invalid", &[("error", e.to_string().into())]);
                    self.set_error(msg);
                    return;
                }
            },
        };
        let options = fcmd_jobs::sync::Options {
            recursive: form.text_value("sync.recursive") == "true",
            both_ways: form.text_value("sync.both_ways") == "true",
            delete_extra: form.text_value("sync.delete_extra") == "true",
            by_content: form.text_value("sync.by_content") == "true",
            filter,
        };
        self.last_sync = options.clone();
        let (left, right) = (self.panels[0].cwd.clone(), self.panels[1].cwd.clone());
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Sync(Box::new(SyncView {
            id,
            left: left.clone(),
            right: right.clone(),
            pairs: Vec::new(),
            cursor: 0,
            top: 0,
            scanning: None,
            done: false,
            stopped: false,
            cancel: cancel.clone(),
        }));
        self.requests.push(Request::SyncPlan {
            id,
            left,
            right,
            options: Box::new(options),
            cancel,
        });
    }

    /// `F2` in the table: turn every row that says something into a job.
    ///
    /// Up to four, in an order that is not an accident: the copies go first, so a row saying
    /// "take this away" cannot run before the row that was going to put something in its
    /// place. They are queued rather than started together, because two jobs writing at once
    /// would fight over the progress box and over the disk.
    fn run_sync(&mut self) {
        use fcmd_jobs::sync::{at, Action};
        let Mode::Sync(view) = &self.mode else {
            return;
        };
        let gather = |want: Action, from: &VPath| -> Vec<VPath> {
            view.pairs
                .iter()
                .filter(|p| p.action == want)
                .map(|p| at(from, &p.rel))
                .collect()
        };
        let to_right = gather(Action::Right, &view.left);
        let to_left = gather(Action::Left, &view.right);
        let cut_right = gather(Action::DeleteRight, &view.right);
        let cut_left = gather(Action::DeleteLeft, &view.left);
        let (left, right) = (view.left.clone(), view.right.clone());
        let to_wastebasket = self.cfg.general.use_trash;

        let copy = |sources: Vec<VPath>, from: &VPath, to: &VPath| -> Option<JobSpec> {
            (!sources.is_empty()).then(|| {
                JobSpec::new(JobKind::Copy, sources, Some(to.clone())).with_options(
                    fcmd_jobs::JobOptions {
                        // Overwrite without asking: the table *is* the question, and it was
                        // answered row by row. Asking again about every file would undo the
                        // whole point of having reviewed it.
                        on_conflict: fcmd_jobs::ConflictPolicy::Overwrite,
                        relative_to: Some(from.clone()),
                        ..fcmd_jobs::JobOptions::default()
                    },
                )
            })
        };
        let mut jobs: Vec<JobSpec> = Vec::new();
        if let Some(job) = copy(to_right, &left, &right) {
            jobs.push(job);
        }
        if let Some(job) = copy(to_left, &right, &left) {
            jobs.push(job);
        }
        for cut in [cut_right, cut_left] {
            if !cut.is_empty() {
                // The same wastebasket rule `F8` follows, for the same reason: a sync that
                // shredded what `F8` would have kept would be a nasty surprise.
                let kind = if to_wastebasket {
                    JobKind::Trash
                } else {
                    JobKind::Delete
                };
                jobs.push(JobSpec::new(kind, cut, None));
            }
        }
        if jobs.is_empty() {
            let m = self.i18n.t("sync-nothing-to-do");
            self.set_error(m);
            return;
        }
        self.mode = Mode::Normal;
        let first = jobs.remove(0);
        self.queued_jobs.extend(jobs);
        self.start_job(first);
    }

    /// The keys of the review table.
    /// The palette editor's keys. Every change is applied to the live theme at once: a colour
    /// one cannot see is a colour one cannot choose.
    fn palette_key(&mut self, key: Key) {
        let rows = self.rows.max(1);
        let Mode::Palette(view) = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                let dirty = view.dirty;
                self.mode = Mode::Normal;
                if dirty {
                    // Said plainly, because the colours on the screen are already the new
                    // ones and nothing else would tell the user the file is not.
                    let m = self.i18n.t("palette-unsaved");
                    self.set_status(m);
                }
                return;
            }
            KeyCode::Up => view.move_cursor(-1),
            KeyCode::Down => view.move_cursor(1),
            KeyCode::PageUp => view.move_cursor(-(rows as isize)),
            KeyCode::PageDown => view.move_cursor(rows as isize),
            KeyCode::Home => view.cursor = 0,
            KeyCode::End => view.cursor = view.keys.len().saturating_sub(1),
            // Enter types a colour: a name (`red`, `brightwhite`), a number for one of the
            // 256, or `#rrggbb`. The field opens on what the row is set to, so a shade is
            // nudged rather than retyped.
            KeyCode::Enter => {
                let Some(key) = view.key().map(str::to_string) else {
                    return;
                };
                let current = self.theme.color(&key).to_string();
                let title = self
                    .i18n
                    .t_args("palette-ask", &[("key", key.clone().into())]);
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::Color,
                    title,
                    text: current,
                    secret: false,
                    sources: Vec::new(),
                });
                return;
            }
            KeyCode::F(2) => {
                self.save_palette();
                return;
            }
            _ => {}
        }
        if let Mode::Palette(view) = &mut self.mode {
            view.scroll_into_view(rows);
        }
    }

    /// `F2` in the palette editor: write the live theme into the configuration file.
    ///
    /// It goes in under the theme's own id, which the theme loader already treats as an
    /// override of the built-in of that name — so the next run starts with these colours and
    /// `Shift+F12` still cycles the themes as it did.
    fn save_palette(&mut self) {
        let table = self.theme.to_table();
        let id = self.theme.id.clone();
        let mine = self
            .cfg
            .themes
            .iter()
            .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(id.as_str()));
        match mine {
            Some(at) => self.cfg.themes[at] = table,
            None => self.cfg.themes.push(table),
        }
        self.save_setup();
        if let Mode::Palette(view) = &mut self.mode {
            view.dirty = false;
        }
        let msg = self.i18n.t_args("palette-saved", &[("theme", id.into())]);
        self.set_status(msg);
    }

    fn sync_key(&mut self, key: Key) {
        let rows = self.rows.max(1);
        let Mode::Sync(view) = &mut self.mode else {
            return;
        };
        match key.code {
            KeyCode::Esc | KeyCode::F(10) => {
                view.cancel.cancel();
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Up => view.move_cursor(-1),
            KeyCode::Down => view.move_cursor(1),
            KeyCode::PageUp => view.move_cursor(-(rows as isize)),
            KeyCode::PageDown => view.move_cursor(rows as isize),
            KeyCode::Home => view.cursor = 0,
            KeyCode::End => view.cursor = view.pairs.len().saturating_sub(1),
            // Space walks the answers for this row, offering only what the pair allows, and
            // steps down — so a run of rows is answered without reaching for an arrow.
            KeyCode::Char(' ') => {
                if let Some(pair) = view.pairs.get_mut(view.cursor) {
                    pair.action = pair.action.next(pair.left.is_some(), pair.right.is_some());
                }
                view.move_cursor(1);
            }
            // The arrows say which way, which is what one reaches for on a row that should go
            // the other way.
            KeyCode::Right => {
                if let Some(pair) = view.pairs.get_mut(view.cursor) {
                    if pair.left.is_some() {
                        pair.action = fcmd_jobs::sync::Action::Right;
                    }
                }
            }
            KeyCode::Left => {
                if let Some(pair) = view.pairs.get_mut(view.cursor) {
                    if pair.right.is_some() {
                        pair.action = fcmd_jobs::sync::Action::Left;
                    }
                }
            }
            KeyCode::Char('=') => {
                if let Some(pair) = view.pairs.get_mut(view.cursor) {
                    pair.action = fcmd_jobs::sync::Action::Skip;
                }
            }
            KeyCode::F(2) if view.done => {
                self.run_sync();
                return;
            }
            _ => return,
        }
        if let Mode::Sync(view) = &mut self.mode {
            view.scroll_into_view(rows);
        }
    }

    // ── checksums (SPEC 03 §9) ──────────────────────────────────────────────────────────

    /// The files a checksum is of: what is selected, less the directories.
    ///
    /// A directory has no bytes of its own, and walking into it is a feature of its own — the
    /// other commanders hash a tree, and that wants its own counting and its own names in the
    /// list. Until it is here, a folder among the selection is left out rather than reported
    /// as unreadable once per folder.
    fn sum_sources(&self) -> Vec<VPath> {
        let panel = self.panel();
        self.job_sources()
            .into_iter()
            .filter(|path| {
                let name = path.display_name();
                match panel
                    .listing
                    .row_of_name(&name)
                    .and_then(|r| panel.listing.row(r))
                {
                    Some(entry) => !entry.kind.is_dirlike(),
                    // Not a row of this panel: the caller knows what it asked for.
                    None => true,
                }
            })
            .collect()
    }

    /// The dialog: which hash, and where the list goes.
    fn open_checksum_form(&mut self) {
        let sources = self.sum_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        // Reading the bytes is the whole of the work, so a path this filesystem cannot read a
        // chunk at a time is refused before the dialog rather than inside the run.
        if sources.iter().any(|p| !p.is_local() || p.in_container()) {
            let m = self.i18n.t("sum-local-only");
            self.set_error(m);
            return;
        }
        let what = self.sources_label(&sources);
        let name = format!(
            "{}.{}",
            self.panel().cwd.display_name(),
            self.last_sum.extension()
        );
        self.mode = Mode::Form(Box::new(Form::checksum(
            what,
            self.last_sum,
            name,
            &self.i18n,
        )));
    }

    /// What the dialog asked for: hash these, with that, into there.
    fn apply_checksum_form(&mut self, form: &Form) {
        let algorithm =
            fcmd_jobs::Algorithm::parse(&form.text_value("sum.algorithm")).unwrap_or_default();
        self.last_sum = algorithm;
        let one_file = form.text_value("sum.one_file") == "true";
        let name = form.text_value("sum.name");
        let paths = self.sum_sources();
        if paths.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let target = if one_file {
            let name = name.trim();
            if name.is_empty() {
                let m = self.i18n.t("sum-no-name");
                self.set_error(m);
                return;
            }
            Some(self.panel().cwd.join(std::ffi::OsStr::new(name)))
        } else {
            None
        };
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.sum = Some((id, cancel.clone()));
        let msg = self.i18n.t_args(
            "sum-working",
            &[
                ("name", paths[0].display_name().into()),
                ("done", 0.into()),
                ("total", (paths.len() as i64).into()),
            ],
        );
        self.set_status(msg);
        self.requests.push(Request::Checksums {
            id,
            paths,
            algorithm,
            target,
            cancel,
        });
    }

    /// Check the files beside the list under the cursor against it.
    fn start_verify(&mut self) {
        let Some(list) = self.panel().current_path() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        if !list.is_local() || list.in_container() {
            let m = self.i18n.t("sum-local-only");
            self.set_error(m);
            return;
        }
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.sum = Some((id, cancel.clone()));
        let msg = self
            .i18n
            .t_args("sum-verifying", &[("name", list.display_name().into())]);
        self.set_status(msg);
        self.requests.push(Request::Verify { id, list, cancel });
    }

    /// What a finished run of digests says, and what it leaves on the disk.
    fn finish_sums(&mut self, written: &[VPath], errors: &[(VPath, String)], cancelled: bool) {
        // The lists are new files in the directory, so the panel has to be read again or they
        // are not there until something else happens to ask.
        let side = self.active;
        self.read_panel(side);
        if cancelled {
            let m = self.i18n.t("sum-stopped");
            self.set_status(m);
            return;
        }
        if !errors.is_empty() {
            let title = self.i18n.t("sum-title");
            let lines = errors
                .iter()
                .map(|(path, why)| format!("{}: {why}", path.display_name()))
                .collect();
            self.mode = Mode::Message { title, lines };
            return;
        }
        let msg = self.i18n.t_args(
            "sum-written",
            &[(
                "name",
                written
                    .iter()
                    .map(|p| p.display_name())
                    .collect::<Vec<_>>()
                    .join(", ")
                    .into(),
            )],
        );
        self.set_status(msg);
    }

    /// What a finished check says, and what it leaves selected.
    ///
    /// The files that do not match are selected, the way a comparison by content selects what
    /// differs: the next thing one does with a bad download is copy it again or delete it, and
    /// both of those work on the selection.
    fn finish_verify(
        &mut self,
        verdicts: &[fcmd_jobs::sum::Verdict],
        cancelled: bool,
        problem: Option<String>,
    ) {
        use fcmd_jobs::sum::State;
        if let Some(problem) = problem {
            self.set_error(problem);
            return;
        }
        if cancelled {
            let m = self.i18n.t("sum-stopped");
            self.set_status(m);
            return;
        }
        let bad: Vec<&fcmd_jobs::sum::Verdict> = verdicts
            .iter()
            .filter(|v| v.state != State::Matches)
            .collect();
        let matched = verdicts.len() - bad.len();
        let side = self.active;
        let listing = self.panels[side].listing.clone();
        self.panels[side].selection.clear();
        for verdict in &bad {
            if let Some(entry) = listing
                .row_of_name(&verdict.name)
                .and_then(|r| listing.row(r))
            {
                self.panels[side].selection.set(entry.id, true);
            }
        }
        self.panels[side].rebuild();
        if bad.is_empty() {
            let msg = self
                .i18n
                .t_args("sum-all-match", &[("count", (matched as i64).into())]);
            self.set_status(msg);
            return;
        }
        let title = self.i18n.t("sum-verify-title");
        let mut lines = vec![
            self.i18n.t_args(
                "sum-verify-summary",
                &[
                    ("ok", (matched as i64).into()),
                    ("bad", (bad.len() as i64).into()),
                ],
            ),
            String::new(),
        ];
        for verdict in bad.iter().take(20) {
            let what = self.i18n.t(match verdict.state {
                State::Differs => "sum-differs",
                State::Missing => "sum-missing",
                State::Unreadable => "sum-unreadable",
                State::Matches => unreachable!("only the bad ones are listed"),
            });
            lines.push(format!("  {}  {what}", verdict.name));
        }
        if bad.len() > 20 {
            lines.push(format!("  … {}", bad.len() - 20));
        }
        self.mode = Mode::Message { title, lines };
    }

    /// Ask before deleting, unless the setting says not to, and then do it.
    ///
    /// `force` is `Shift+Del`: delete outright, whatever the wastebasket setting says.
    fn ask_delete(&mut self, sources: Vec<VPath>, force: bool) {
        let bytes = self.selected_bytes(&sources);
        // Only this machine has a wastebasket. A server, an archive and a bucket do not, and
        // one made up here would be a directory the desktop knows nothing about — so those
        // go the way they always did, and the dialog says which of the two is happening.
        let to_trash = !force && self.cfg.general.use_trash && sources.iter().all(|p| p.is_local());
        let kind = if to_trash {
            JobKind::Trash
        } else {
            JobKind::Delete
        };
        // Inside an archive there is nothing to unlink: the container is written again
        // without those entries (SPEC 04 §4). The question is the same question, so only
        // what `Enter` does changes.
        let action = match Self::changes_removing(&sources) {
            Some((archive, changes)) => ConfirmAction::ArcRebuild { archive, changes },
            None => ConfirmAction::Job(Box::new(JobSpec::new(kind, sources.clone(), None))),
        };
        if !self.cfg.general.confirm_delete {
            match action {
                ConfirmAction::ArcRebuild { archive, changes } => {
                    self.start_rebuild(archive, changes)
                }
                ConfirmAction::Job(spec) => self.start_job(*spec),
                _ => unreachable!("ask_delete builds one of those two"),
            }
            return;
        }
        let title = self.i18n.t(if to_trash {
            "confirm-trash-title"
        } else {
            "confirm-delete-title"
        });
        let body = self.i18n.t_args(
            if to_trash {
                "confirm-trash-body"
            } else {
                "confirm-delete-body"
            },
            &[
                ("count", (sources.len() as i64).into()),
                ("bytes", fcmd_core::format_size(bytes).into()),
                // One file is a name, not a count: "Delete 1 items" is the sentence nobody
                // means to write, and the name is the thing the reader is checking anyway.
                ("name", sources[0].display_name().into()),
            ],
        );
        let mut lines = vec![body, String::new()];
        if sources.len() > 1 {
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
        }
        // Which of the two this is, in a sentence: the one word that separates a safety net
        // from a shredder is worth a line of the dialog.
        lines.push(self.i18n.t(if to_trash {
            "confirm-trash-where"
        } else {
            "confirm-delete-permanent"
        }));
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action,
            button: 0,
        });
    }

    /// The removals that take these paths out of the archive they are all in, or `None` if
    /// they are not all inside one and the same container.
    fn changes_removing(sources: &[VPath]) -> Option<(VPath, Vec<fcmd_arc::Change>)> {
        let mut archive: Option<VPath> = None;
        let mut changes = Vec::with_capacity(sources.len());
        for path in sources {
            let (host, inner) = Self::inside(path)?;
            match &archive {
                Some(seen) if seen != &host => return None,
                _ => archive = Some(host),
            }
            changes.push(fcmd_arc::Change::Remove { inner });
        }
        archive.map(|a| (a, changes))
    }

    /// The renames that move these paths about inside the archive they are in, or `None` if
    /// the target is somewhere else — out of the archive, or in a different one.
    ///
    /// With one source and a name, the name is the new name. With several, or a target that
    /// ends in a separator, the target is a directory inside the archive and each keeps the
    /// name it has.
    fn changes_renaming(
        sources: &[VPath],
        target: &VPath,
        dir: bool,
    ) -> Option<(VPath, Vec<fcmd_arc::Change>)> {
        let (host, to) = Self::inside(target)?;
        let mut archive = None;
        let mut changes = Vec::with_capacity(sources.len());
        for path in sources {
            let (from_host, from) = Self::inside(path)?;
            if from_host != host {
                return None;
            }
            archive = Some(from_host);
            let to = if dir {
                format!("{}/{}", to.trim_end_matches('/'), path.display_name())
            } else {
                to.clone()
            };
            if to == from {
                continue;
            }
            changes.push(fcmd_arc::Change::Rename { from, to });
        }
        archive.map(|a| (a, changes))
    }

    /// `F5` with the other panel inside an archive: the files go in, which means the archive
    /// is written again with them (SPEC 04 §4).
    fn add_into_archive(&mut self, sources: Vec<VPath>, target: &VPath, delete_after: bool) {
        let Some((archive, dir)) = Self::inside(target) else {
            return;
        };
        let mut changes = Vec::with_capacity(sources.len());
        for path in &sources {
            // A tree would have to be walked, and walking it here would stop the interface
            // while it happened. `Shift+F1` packs a whole directory and does it in a job.
            if !path.is_local() || path.in_container() || path.as_path().is_dir() {
                let m = self.i18n.t("arc-add-files-only");
                self.set_error(m);
                return;
            }
            changes.push(fcmd_arc::Change::add(path.as_path(), &dir));
        }
        self.pack_delete = delete_after.then(|| sources.clone());
        self.start_rebuild(archive, changes);
    }

    /// Everything in `archive`, straight into `to`: the extraction that asks nothing, for the
    /// archive menu's *extract here*.
    fn unpack_into(&mut self, archive: VPath, to: VPath) {
        let options = fcmd_jobs::JobOptions {
            contents: true,
            ..Default::default()
        };
        let spec =
            JobSpec::new(JobKind::Copy, vec![archive.enter("/")], Some(to)).with_options(options);
        self.start_job(spec);
    }

    /// The same progress box again, with the archive reader behind it.
    fn start_test(&mut self, archive: VPath) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Job(Box::new(JobView {
            id,
            kind: JobKind::Test,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        }));
        self.requests.push(Request::ArcTest {
            id,
            archive,
            cancel,
        });
    }

    /// Change an archive: one job, one progress box, and the old archive untouched until
    /// the new one is written and checked.
    fn start_rebuild(&mut self, archive: VPath, changes: Vec<fcmd_arc::Change>) {
        if changes.is_empty() {
            // Nothing to write means nothing was written, and nothing written means the
            // originals a move was going to delete stay where they are.
            self.pack_delete = None;
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Job(Box::new(JobView {
            id,
            kind: JobKind::Rebuild,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        }));
        self.requests.push(Request::ArcRebuild {
            id,
            archive,
            changes,
            level: self.last_pack_level,
            cancel,
        });
    }

    /// The container a path inside an archive belongs to, and the path inside it.
    fn inside(path: &VPath) -> Option<(VPath, String)> {
        if !path.in_container() {
            return None;
        }
        let inner = path.inner_path().to_string();
        let mut host = path.clone();
        while let Some(outer) = host.leave() {
            host = outer;
        }
        Some((host, inner))
    }

    /// The archive the archive commands are about: the file under the cursor when it is one,
    /// or the container the panel is inside when it is in one.
    fn archive_at_hand(&self) -> Option<VPath> {
        let panel = self.panel();
        if panel.cwd.in_container() {
            // Out of every level: the commands are about the file on disk, not about a
            // container nested inside another one.
            let mut host = panel.cwd.clone();
            while let Some(outer) = host.leave() {
                host = outer;
            }
            return Some(host);
        }
        let entry = panel.current()?;
        let path = panel.cwd.join(entry.name_str().as_ref());
        fcmd_arc::Format::of_name(&path.display_name()).map(|_| path)
    }

    /// `Alt+F1` and `Alt+F2`: everything a panel can be sent to, as a table.
    ///
    /// FAR's change-drive dialog has columns rather than a list of paths — what the thing is,
    /// what it is called, what it is formatted as, how big it is and how much is left — and
    /// then the plugin panels under them, numbered, because a number is how anyone who uses
    /// it twice a day picks one. Ours are the open connections and the object stores that are
    /// configured.
    fn drive_list(&self, side: usize) -> Pick {
        /// `C:\` and `C:` are the letter `c`; a Unix mount point has no letter and gets none,
        /// because inventing one would make two rows answer to the same key.
        fn drive_letter(path: &str) -> Option<char> {
            let mut chars = path.chars();
            let letter = chars.next()?;
            (letter.is_ascii_alphabetic() && chars.next() == Some(':')).then_some(letter)
        }

        let mounted = crate::drives::list();
        let cells: Vec<[String; 5]> = mounted
            .iter()
            .map(|drive| {
                [
                    self.i18n.t(drive.kind.label_key()),
                    drive.label.clone().unwrap_or_default(),
                    drive.fs.clone().unwrap_or_default(),
                    drive.total.map(fcmd_core::format_size).unwrap_or_default(),
                    drive.free.map(fcmd_core::format_size).unwrap_or_default(),
                ]
            })
            .collect();
        // Each column as wide as its widest value, so the table lines up without a layout
        // engine and a column nothing filled takes no room at all.
        let mut widths = [0usize; 5];
        for row in &cells {
            for (i, text) in row.iter().enumerate() {
                widths[i] = widths[i].max(unicode_width::UnicodeWidthStr::width(text.as_str()));
            }
        }
        let path_w = mounted
            .iter()
            .map(|d| unicode_width::UnicodeWidthStr::width(d.path.as_str()))
            .max()
            .unwrap_or(0)
            .min(40);

        let here = self.panels[side].cwd.to_string();
        let mut rows: Vec<PickRow> = Vec::new();
        for (drive, cell) in mounted.iter().zip(cells.iter()) {
            let mut label = format!("{:<path_w$}", fcmd_ui::truncate_cols(&drive.path, path_w));
            for (i, text) in cell.iter().enumerate() {
                if widths[i] == 0 {
                    continue;
                }
                label.push_str("  ");
                // The two sizes read right-aligned; the words read left.
                if i >= 3 {
                    label.push_str(&format!("{text:>w$}", w = widths[i]));
                } else {
                    label.push_str(&format!("{text:<w$}", w = widths[i]));
                }
            }
            let row = PickRow::plain(
                label.trim_end(),
                drive.path.clone(),
                // The one the panel is already on. `/` matches everything, so it only counts
                // when nothing longer does.
                drive.path != "/" && here.starts_with(&drive.path),
            );
            // On Windows the drive's own letter takes it, which is what thirty years of
            // `Alt+F1` then `C` have taught everybody's fingers: the letter is the key, not a
            // way to move the cursor to a row one then has to press `Enter` on.
            rows.push(match drive_letter(&drive.path) {
                Some(letter) => row.with_key(letter),
                None => row,
            });
        }
        if !rows.iter().any(|r| r.current) {
            if let Some(root) = rows.iter_mut().find(|r| r.value == "/") {
                root.current = true;
            }
        }

        // The panels that are not filesystems, numbered the way FAR numbers them.
        let mut number = 0u32;
        let mut keyed = |label: String, value: String, current: bool| {
            let row = PickRow::plain(label, value, current);
            let row = match char::from_digit(number, 10) {
                Some(key) => row.with_key(key),
                None => row,
            };
            number += 1;
            row
        };
        for key in &self.connected {
            let kind = self.i18n.t(crate::drives::Kind::Connection.label_key());
            rows.push(keyed(
                format!("{key}  {kind}"),
                format!("{key}/"),
                fcmd_net::Site::key_of(&self.panels[side].cwd).as_deref() == Some(key.as_str()),
            ));
        }
        for service in &self.cfg.s3 {
            let kind = self.i18n.t("drive-objects");
            rows.push(keyed(
                format!("s3://{}@  {kind}", service.name),
                format!("s3://{}@/", service.name),
                false,
            ));
        }
        Pick::new(
            PickKind::Drives,
            self.plain("menu-change-drive"),
            rows,
            side,
        )
    }

    /// `Shift+F3`: what can be done with the archive at hand, as a menu. MultiArc calls it
    /// `ArcCmd`, and the list is short because everything on it is something we really do.
    fn open_arc_commands(&mut self) {
        let archive = self.archive_at_hand();
        let mut rows = Vec::new();
        if let Some(path) = &archive {
            let name = path.display_name();
            for (id, value) in [
                ("arc-op-test", "test"),
                ("arc-op-extract-here", "here"),
                ("arc-op-extract-to", "to"),
                ("arc-op-delete", "delete"),
            ] {
                rows.push(PickRow {
                    label: self.i18n.t_args(id, &[("name", name.clone().into())]),
                    value: value.to_string(),
                    current: false,
                    hotkey: None,
                });
            }
        }
        rows.push(PickRow {
            label: self.i18n.t("arc-op-add"),
            value: "add".to_string(),
            current: false,
            hotkey: None,
        });
        let title = self.i18n.t("arc-commands-title");
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::ArcCommands, title, rows, 0)));
    }

    /// The same progress box as a job, with the archive writer behind it.
    fn start_pack(
        &mut self,
        archive: VPath,
        format: fcmd_arc::Format,
        level: fcmd_arc::Level,
        sources: Vec<VPath>,
    ) {
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Job(Box::new(JobView {
            id,
            kind: JobKind::Pack,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        }));
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
                    ("bytes", fcmd_core::format_size(outcome.bytes).into()),
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

        // The dialog asked for the originals to go once the archive was written. Written
        // is the condition: nothing is deleted after a pack that failed or was stopped.
        if let Some(sources) = self.pack_delete.take() {
            if matches!(kind, JobKind::Pack | JobKind::Rebuild)
                && outcome.errors.is_empty()
                && !outcome.cancelled
            {
                self.start_job(JobSpec::new(JobKind::Delete, sources, None));
                return;
            }
        }

        // The next destination of a copy to several places. A cancelled or failed job stops
        // the rest: somebody who pressed `Esc` meant the whole thing, and a disk that filled
        // up for the first destination will fill up for the second.
        if outcome.cancelled || !outcome.errors.is_empty() {
            self.queued_jobs.clear();
        } else if let Some(next) = self.queued_jobs.pop_front() {
            self.start_job(next);
            return;
        }

        // Both panels may be looking at what just changed.
        self.panels[self.active].selection.clear();
        self.read_panel(0);
        self.read_panel(1);
    }

    fn normal_key(&mut self, key: Key) {
        // With the panels off the screen is what the commands printed, so the keys that would
        // page through a listing page through that instead. Before the command line, which
        // does not use these two, and before the keymap, whose panel commands have nothing to
        // move while there is no panel on screen.
        if self.panels_off && key.mods == Mods::NONE {
            let step = self.rows.max(1);
            match key.code {
                KeyCode::PageUp => {
                    let last = self.user_screen.len().saturating_sub(1);
                    self.user_scroll = (self.user_scroll + step).min(last);
                    return;
                }
                KeyCode::PageDown => {
                    self.user_scroll = self.user_scroll.saturating_sub(step);
                    return;
                }
                _ => {}
            }
        }
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
                    steps.push(fcmd_cmd::Step::Run(cmd));
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
                let place = self.help_place();
                self.park_front_screen();
                self.mode = Mode::Help {
                    chapter: crate::help::Chapter::Here(place),
                    scroll: 0,
                    at: 0,
                }
            }
            Command::AppFont(by) => {
                self.requests.push(Request::Font { by });
                let msg = self.i18n.t(match by {
                    0 => "msg-font-reset",
                    by if by < 0 => "msg-font-smaller",
                    _ => "msg-font-bigger",
                });
                self.set_status(msg);
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
            Command::NavBookmarkSet(digit) => self.bookmark_set(digit),
            Command::NavBookmarkGo(digit) => self.bookmark_go(digit),
            Command::NavBookmarks => self.open_pick(PickKind::Bookmarks, self.active),
            Command::PanelTabNew => self.tab_new(),
            Command::PanelTabClose => self.tab_close(),
            Command::PanelTabNext => {
                let to = self.tabs[self.active].next();
                self.tab_switch(to);
            }
            Command::PanelTabPrev => {
                let to = self.tabs[self.active].prev();
                self.tab_switch(to);
            }
            Command::PanelTabMove => self.tab_move(),
            Command::PanelTabLock => {
                let locked = !self.panel().locked;
                self.panel_mut().locked = locked;
                let msg = self.i18n.t(if locked {
                    "msg-tab-locked-on"
                } else {
                    "msg-tab-locked-off"
                });
                self.set_status(msg);
            }
            Command::PanelTabName => {
                let title = self.i18n.t("prompt-tab-name");
                let text = self.panel().name.clone().unwrap_or_default();
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::TabName,
                    title,
                    text,
                    secret: false,
                    sources: Vec::new(),
                });
            }
            Command::PanelViewSet(view) => {
                self.panel_mut().view = view;
                self.panel_mut().top = 0;
                self.scroll();
                // The owners view is the only one that needs something the listing does not
                // carry, and it is asked for when the view is chosen rather than when the
                // directory is read: nobody should pay a lookup per file for a column they
                // are not looking at.
                let active = self.active;
                self.ask_for_owners(active);
                self.ask_for_descriptions(active);
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
            // FAR's `Group`: the bands `[[sort_group]]` cuts the listing into, on or off.
            // Saying how many there are is the whole answer to "why did nothing move?",
            // which is what a toggle over an empty list looks like.
            Command::PanelSortGroups => {
                let panel = self.panel_mut();
                panel.sort.groups = !panel.sort.groups;
                let (on, count) = (panel.sort.groups, panel.groups.len());
                panel.rebuild();
                let msg = if count == 0 {
                    self.i18n.t("msg-sort-groups-none")
                } else {
                    let id = if on {
                        "msg-sort-groups-on"
                    } else {
                        "msg-sort-groups-off"
                    };
                    self.i18n.t_args(id, &[("count", (count as i64).into())])
                };
                self.set_status(msg);
            }
            // The shape of the two panels. Both are settings, so `Shift+F9` keeps them.
            Command::PanelSplit(by) => {
                let was = self.cfg.panel.split;
                self.cfg.panel.split = (was as i32 + by as i32).clamp(10, 90) as u16;
            }
            Command::PanelHeight(by) => {
                let was = self.cfg.panel.height_cut as i32;
                // The step is the panels' height and the setting is what they give up, so the
                // two have opposite signs: `Ctrl+Up` is a row shorter, which is a row more for
                // the output underneath. Never more than what leaves a panel worth looking at;
                // the renderer has the final say, but the setting should not run away either.
                self.cfg.panel.height_cut = (was - by as i32).clamp(0, 40) as u16;
            }
            // FAR's `Group` and `SelUp`: the two parts of the order that are not the key
            // itself. Both are panel-wide settings, so both panels follow — as the view and
            // the sort key already do.
            Command::PanelDirsFirst => {
                self.cfg.panel.dirs_first = !self.cfg.panel.dirs_first;
                let on = self.cfg.panel.dirs_first;
                for panel in &mut self.panels {
                    panel.sort.dirs_first = on;
                    panel.rebuild();
                }
                let msg = self.i18n.t(if on {
                    "msg-dirs-first-on"
                } else {
                    "msg-dirs-first-off"
                });
                self.set_status(msg);
            }
            Command::PanelSelectedFirst => {
                self.cfg.panel.selected_first = !self.cfg.panel.selected_first;
                let on = self.cfg.panel.selected_first;
                for panel in &mut self.panels {
                    panel.sort.selected_first = on;
                    panel.rebuild();
                }
                let msg = self.i18n.t(if on {
                    "msg-selected-first-on"
                } else {
                    "msg-selected-first-off"
                });
                self.set_status(msg);
            }
            Command::ArcCommands => self.open_arc_commands(),
            Command::ArcTest => match self.archive_at_hand() {
                Some(path) => self.start_test(path),
                None => {
                    let m = self.i18n.t("unpack-not-an-archive");
                    self.set_error(m);
                }
            },
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
                if self.panel_go(other, path, true) {
                    self.read_panel(other);
                }
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
                // Not a directory: it may still be an archive, which `Enter` goes into, and
                // otherwise it is a file to open (SPEC 06 §5).
                None => match self.archive_target() {
                    Some(path) => self.goto(path),
                    None => self.open_file(),
                },
            },
            Command::NavParent => {
                if let Some(parent) = self.panel().cwd.parent() {
                    // The cursor goes back on the directory we came out of; `goto` does that
                    // for every way of going up, which this used to try to do by itself and
                    // got wrong — it put the name in the status message instead.
                    self.goto(parent.normalised());
                }
            }
            Command::NavRoot => {
                let root = self.panel().cwd.root();
                self.goto(root);
            }
            Command::NavBack => match self.panel_mut().history_back() {
                Some(path) => {
                    let a = self.active;
                    if self.panel_go(a, path, false) {
                        self.read_panel(a);
                    }
                }
                None => {
                    let m = self.i18n.t("msg-no-history");
                    self.set_status(m)
                }
            },
            Command::NavForward => match self.panel_mut().history_forward() {
                Some(path) => {
                    let a = self.active;
                    if self.panel_go(a, path, false) {
                        self.read_panel(a);
                    }
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
                        ("bytes", fcmd_core::format_size(bytes).into()),
                        ("name", sources[0].display_name().into()),
                    ],
                );
                let mut lines = vec![body, String::new()];
                if sources.len() > 1 {
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
                }
                // Overwriting is not a promise on modern hardware, and the dialog says so
                // rather than letting a user believe something the filesystem cannot deliver.
                lines.push(self.i18n.t("confirm-wipe-honest"));
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines,
                    action: ConfirmAction::Job(Box::new(spec)),
                    button: 0,
                });
            }
            Command::FileAttributes => self.open_attributes_form(),
            Command::FileLink => {
                let Some(entry) = self.panel().current().cloned() else {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_error(m);
                    return;
                };
                if entry.kind == fcmd_core::EntryKind::Parent {
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
                let what = self
                    .i18n
                    .t_args("prompt-link", &[("name", target.display_name().into())]);
                self.pending_link = Some(target);
                self.mode =
                    Mode::Form(Box::new(Form::link(what, text, self.last_link, &self.i18n)));
            }
            Command::PanelQuickView(side) => {
                self.set_content(side, crate::panel::Content::QuickView)
            }
            Command::PanelInfo(side) => self.set_content(side, crate::panel::Content::Info),
            Command::PanelTree(side) => self.set_content(side, crate::panel::Content::Tree),
            Command::FilePack => self.open_pack_form(),
            Command::FileUnpack => self.open_unpack_form(),
            Command::NetTunnel => self.ask_tunnel(),
            Command::NetTunnels => {
                self.requests.push(Request::TunnelList);
                self.open_tunnels();
            }
            Command::ToolCompareDirs => self.compare_dirs(),
            Command::ToolCompareContents => self.compare_contents(),
            Command::ToolCalculator => {
                // It opens on the last sum: a calculator is used in runs.
                let title = self.i18n.t("calc-title");
                let last = self.last_calc.clone();
                self.mode = Mode::Prompt(Prompt::new(PromptKind::Calculator, title, last));
            }
            Command::ToolTerminal => self.toggle_terminal(),
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
            // FAR's `Ctrl+P`: the panel you are not in goes away, and comes back on the same
            // key. The focus never has to move, which is the difference from `Ctrl+F1`/`F2`.
            Command::PanelToggleOther => {
                let other = 1 - self.active;
                self.hidden = if self.hidden == Some(other) {
                    None
                } else {
                    Some(other)
                };
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
            // `Alt+F3` and `Alt+F4` are FAR's keys for the *other* viewer and editor: what an
            // `[[assoc]]` block names for this kind of file, and otherwise what the
            // environment says one reads and writes with. `F3` and `F4` stay ours, always,
            // which is the half of this that matters — a key that sometimes opened something
            // else would be a key nobody trusts.
            Command::FileViewAlt => self.open_external(Which::View),
            Command::FileEditAlt => self.open_external(Which::Edit),
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
            Command::CmdFileHistory => self.open_pick(PickKind::FileHistory, self.active),
            Command::FileClipNames => self.clip_sources(false),
            Command::FileClipPaths => self.clip_sources(true),
            Command::CmdClip => {
                if self.cmd.is_empty() {
                    let m = self.i18n.t("msg-nothing-to-do");
                    self.set_status(m);
                    return;
                }
                let line = self.cmd.text.clone();
                if self.copy_to_clipboard(&line) {
                    let m = self.i18n.t("clip-line");
                    self.set_status(m);
                }
            }
            Command::CmdInsertName => self.insert_into_line(false, false),
            Command::CmdInsertNameOther => self.insert_into_line(false, true),
            Command::CmdInsertPath => self.insert_into_line(true, false),
            Command::CmdInsertPathOther => self.insert_into_line(true, true),
            // FAR's `Ctrl+O`: the panels go away and the screen underneath comes back, with
            // the command line, the menus and the keys all still live. It is a toggle, and
            // the same key brings the panels back.
            Command::ShellOutputShow => {
                self.panels_off = !self.panels_off;
                self.user_scroll = 0;
                // Once the key has been pressed the screen is the user's: a command that is
                // still running does not take it away again when it ends.
                self.shell_screen = false;
            }
            Command::AppThemes => self.open_pick(PickKind::Themes, self.active),
            Command::AppLanguages => self.open_pick(PickKind::Languages, self.active),
            Command::AppSetupSave => self.save_setup(),
            Command::OptSystem => self.open_form(FormKind::System),
            Command::OptPanel => self.open_form(FormKind::Panel),
            Command::OptInterface => self.open_form(FormKind::Interface),
            Command::OptHighlight => self.open_pick(PickKind::Highlight, self.active),
            Command::OptConfirmations => self.open_form(FormKind::Confirmations),
            Command::OptEditor => self.open_form(FormKind::EditorSettings),
            Command::OptViewer => self.open_form(FormKind::ViewerSettings),
            Command::OptColors => {
                self.mode = Mode::Palette(Box::default());
            }
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
                    Some(Ok(fcmd_view::Started::Derived(producer))) => {
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
                // Over a disassembly the query is an instruction pattern rather than text,
                // hex or a regular expression, so the choice would be a lie: that one keeps
                // the one-line prompt, which says what it is searching.
                if self.mode.viewer().map(|v| v.mode) == Some(ViewerMode::Disasm) {
                    let title = self
                        .i18n
                        .t_args("viewer-prompt-search", &[("mode", "instructions".into())]);
                    self.mode_prompt_over_viewer(
                        PromptKind::ViewerSearch,
                        title,
                        self.last_search.clone().unwrap_or_default(),
                    );
                    return;
                }
                let form = Form::search(
                    &self.last_search.clone().unwrap_or_default(),
                    self.search_mode.as_str(),
                    !self.search_case_insensitive,
                    self.search_whole_words,
                    &self.i18n,
                );
                self.form_over_viewer(form);
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
                // Moving something that is inside an archive is a rename inside the archive,
                // so there the offer is the name it has: a target in the other panel would
                // mean taking it out, which is what `F5` is for.
                let inside = !copy && sources.iter().all(|p| p.in_container());
                let text = if inside && sources.len() == 1 {
                    sources[0].display_name()
                } else if inside {
                    String::new()
                } else {
                    let mut text = self.other().cwd.to_string();
                    if !text.ends_with(std::path::MAIN_SEPARATOR) {
                        text.push(std::path::MAIN_SEPARATOR);
                    }
                    text
                };
                let _ = title;
                // The list is settled now rather than when the dialog is answered: the
                // selection could change under an open dialog, and what the user was looking
                // at when they pressed the key is what they meant.
                self.copy_sources = sources;
                self.mode = Mode::Form(Box::new(Form::copy(
                    text,
                    !copy,
                    self.sources_label(&self.copy_sources),
                    self.last_copy.conflict,
                    self.last_copy.only_newer,
                    self.last_copy.follow_links,
                    &self.last_copy.filter,
                    &self.i18n,
                )));
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
            Command::FileAssoc => self.open_with_list(),
            Command::ToolUserMenu => self.open_user_menu(),
            Command::ToolSyncDirs => self.open_sync_form(),
            Command::FileDescribe => self.open_describe(),
            Command::FileChecksum => self.open_checksum_form(),
            Command::FileChecksumVerify => self.start_verify(),
            Command::FileDelete | Command::FileDeleteCurrent | Command::FileDeleteForce => {
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
                self.ask_delete(sources, cmd == Command::FileDeleteForce);
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
        match Editor::open(&path.os()) {
            Ok(mut editor) => {
                self.tune_editor(&mut editor);
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

    /// Read a file by name — the history list's `Enter`.
    ///
    /// A path out of the history may be on a server, or may no longer be there at all: both
    /// are said rather than ignored, because a list that does nothing when a row is chosen
    /// looks broken.
    fn view_path(&mut self, path: &str) {
        let target = VPath::parse(path);
        if target.is_remote() || target.in_container() {
            // Through the same fetch every other remote read goes through, which needs the
            // size: asking for it is the connection's business, so zero means "whatever it
            // turns out to be".
            self.fetch_remote(target, 0, Fetched::View);
            return;
        }
        let local = target.os().to_path_buf();
        if !local.is_file() {
            let msg = self
                .i18n
                .t_args("history-gone", &[("name", path.to_string().into())]);
            self.set_error(msg);
            return;
        }
        self.open_viewer_on(&local, target);
    }

    /// Edit a file by name — the history list's `F4`.
    fn edit_path(&mut self, path: &str) {
        let target = VPath::parse(path);
        if target.is_remote() || target.in_container() {
            self.fetch_remote(target, 0, Fetched::Edit);
            return;
        }
        if !target.os().is_file() {
            let msg = self
                .i18n
                .t_args("history-gone", &[("name", path.to_string().into())]);
            self.set_error(msg);
            return;
        }
        match Editor::open(&target.os()) {
            Ok(mut editor) => {
                self.tune_editor(&mut editor);
                self.mode = Mode::Editor(Box::new(editor));
            }
            Err(e) => self.set_error(format!("{target}: {e}")),
        }
    }

    /// Open the viewer on a local file, showing `shown` as its name — which is the remote
    /// path when the bytes came down from a server.
    fn open_viewer_on(&mut self, local: &std::path::Path, shown: VPath) {
        match FileSource::open(local) {
            Ok(source) => {
                let remote = shown.is_remote();
                let mut viewer = Viewer::new(Box::new(source), shown.clone());
                self.tune_viewer(&mut viewer);
                // A dBase table keeps its memo fields in a second file beside it. Opening
                // that file is the caller's job, because what a path means — on disk, inside
                // an archive, on a server — is not the viewer's to decide, and here it is a
                // path on disk: the bytes of a remote table have already been fetched to
                // one, and its memo file has not, so there is nothing to open then.
                if let Some(memo) = memo_file(local) {
                    if let Ok(memo) = FileSource::open(&memo) {
                        viewer.attach_memo(Box::new(memo));
                    }
                }
                if viewer.is_empty() {
                    let m = self.i18n.t("viewer-empty");
                    self.set_status(m);
                } else if remote {
                    let msg = self
                        .i18n
                        .t_args("net-viewing-copy", &[("name", shown.display_name().into())]);
                    self.set_status(msg);
                }
                // A README opens as a README. `Shift+F2` is the way to its source, and the
                // setting is for a reader who would rather start there.
                if self.cfg.ui.markdown {
                    viewer.as_markdown();
                }
                // And a `.csv` opens as a table, for the same reason and with `Shift+F3`
                // as the way back to the commas. Only a name that says it is tabular opens
                // that way: the key itself is less fussy (a `.txt` of tab-separated columns
                // is a table), but what a file opens as has to be predictable.
                if self.cfg.ui.table && viewer.reads_table() {
                    viewer.as_table();
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
        self.connected.contains(&site.key()).then(|| site.start())
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
                if self.panel_go(panel, start, true) {
                    self.read_panel(panel);
                }
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
                    button: 0,
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
        match fcmd_net::Site::key_of(&self.panel().cwd) {
            Some(key) => {
                self.requests.push(Request::Disconnect { key });
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
            // The line owns `Ctrl+Ins` while it has something on it — the panel's copy of the
            // names is what the key means on an empty line — because the text in front of
            // the cursor is what somebody looking at that line wants to keep.
            KeyCode::Insert if !empty && ctrl && !key.mods.contains(Mods::SHIFT) => {
                self.run(Command::CmdClip);
                true
            }
            // And the arrows walk the history while there is something on the line, which is
            // what every shell does and what anybody typing there reaches for. FAR's own
            // `Ctrl+E` and `Ctrl+X` still work, and on an empty line the arrows belong to the
            // panel, as every shared key does.
            KeyCode::Up if !empty && key.mods == Mods::NONE => {
                self.run(Command::CmdHistoryPrev);
                true
            }
            KeyCode::Down if !empty && key.mods == Mods::NONE => {
                self.run(Command::CmdHistoryNext);
                true
            }
            KeyCode::Enter if !empty && !ctrl => {
                self.run_command_line();
                true
            }
            KeyCode::Esc if !empty => {
                self.cmd.clear();
                true
            }
            // With nothing on the line, `Esc` is the way out of a comparison by content that
            // is reading more files than it was meant to.
            KeyCode::Esc if self.compare.is_some() => {
                if let Some((_, cancel)) = self.compare.take() {
                    cancel.cancel();
                }
                let m = self.i18n.t("compare-contents-stopped");
                self.set_status(m);
                true
            }
            // And out of a checksum run, which reads just as many bytes.
            KeyCode::Esc if self.sum.is_some() => {
                if let Some((_, cancel)) = self.sum.take() {
                    cancel.cancel();
                }
                let m = self.i18n.t("sum-stopped");
                self.set_status(m);
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
            // `Ctrl+W`, the one every readline has. `Ctrl+Backspace` above is FAR's own, and
            // a terminal that cannot tell `Ctrl+Backspace` from `Backspace` leaves this one.
            KeyCode::Char('w') if !empty && ctrl => {
                self.cmd.delete_word_left();
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
                .filter(|e| e.kind != fcmd_core::EntryKind::Parent)
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
                .filter(|e| e.kind != fcmd_core::EntryKind::Parent)
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
                    if e.kind != fcmd_core::EntryKind::Parent && active.selection.contains(e.id) {
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

    // ── the user menu (`F2`, SPEC 06 §4) ────────────────────────────────────────────────

    /// `F2`: the menu the user wrote. The global one, with the directory's own menu above it
    /// when the panel is a directory on this machine — which is where a `.fcmd-menu.toml`
    /// travelling with a project can be read from at all.
    fn open_user_menu(&mut self) {
        use fcmd_config::usermenu::{UserMenu, LOCAL_MENU_FILE_NAME};

        let global = match UserMenu::load(&self.paths.menu_file) {
            Ok(menu) => menu,
            Err(e) => {
                self.set_error(e);
                return;
            }
        };
        let cwd = self.panel().cwd.clone();
        let local = if cwd.is_local() && !cwd.in_container() {
            match UserMenu::load(&cwd.as_path().join(LOCAL_MENU_FILE_NAME)) {
                Ok(menu) => menu,
                Err(e) => {
                    self.set_error(e);
                    return;
                }
            }
        } else {
            UserMenu::default()
        };
        let menu = UserMenu::merged(&local, &global);
        if menu.entries.is_empty() {
            // The feature is invisible until somebody writes the file, so the one thing `F2`
            // can usefully do with no file is say where the file goes.
            let msg = self.i18n.t_args(
                "usermenu-empty",
                &[("path", self.paths.menu_file.display().to_string().into())],
            );
            self.set_status(msg);
            return;
        }

        let (rows, problems) = self.user_menu_rows(&menu.entries);
        if let Some(first) = problems.first() {
            // The row is still shown — an entry that vanished over a typo in its own `when`
            // would be impossible to work out — and the reason is on the status line.
            let msg = self.i18n.t_args(
                "usermenu-bad-condition",
                &[("problem", first.clone().into())],
            );
            self.set_error(msg);
        }
        let title = self.plain("menu-user-menu");
        self.menu_levels = vec![(title, rows)];
        self.show_user_menu_level();
    }

    /// The rows of one level, with the conditions applied. What a `when` may ask about is
    /// gathered here: the names in the panel, the one under the cursor, whether anything is
    /// selected, and whether this is a directory a shell could be pointed at.
    fn user_menu_rows(
        &self,
        entries: &[fcmd_config::usermenu::Entry],
    ) -> (Vec<fcmd_config::usermenu::Entry>, Vec<String>) {
        let panel = self.panel();
        let mut names = Vec::with_capacity(panel.listing.len());
        for row in 0..panel.listing.len() {
            if let Some(entry) = panel.listing.row(row) {
                if entry.kind != fcmd_core::EntryKind::Parent {
                    names.push(entry.name.to_string_lossy().into_owned());
                }
            }
        }
        let current = panel
            .current()
            .filter(|e| e.kind != fcmd_core::EntryKind::Parent)
            .map(|e| e.name.to_string_lossy().into_owned());
        let facts = fcmd_config::usermenu::Facts {
            names: &names,
            current: current.as_deref(),
            selected: !panel.selection.is_empty(),
            local: panel.cwd.is_local() && !panel.cwd.in_container(),
        };
        fcmd_config::usermenu::UserMenu::visible(entries, &facts)
    }

    /// The level on top of the stack, as a list. A submenu row says it is one, because a row
    /// that opened a list where every other row ran something would be a surprise.
    fn show_user_menu_level(&mut self) {
        let Some((title, level)) = self.menu_levels.last().cloned() else {
            self.mode = Mode::Normal;
            return;
        };
        let rows: Vec<PickRow> = level
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let label = if entry.is_submenu() {
                    format!("{}  \u{25b8}", entry.label)
                } else {
                    entry.label.clone()
                };
                let row = PickRow::plain(label, i.to_string(), false);
                match entry.key {
                    Some(key) => row.with_key(key),
                    None => row,
                }
            })
            .collect();
        self.mode = Mode::Pick(Box::new(Pick::new(
            PickKind::UserMenu,
            title,
            rows,
            self.active,
        )));
    }

    /// A row of the user menu was taken: open the submenu, or run what it holds.
    fn take_user_menu_row(&mut self, which: usize) {
        let Some(entry) = self
            .menu_levels
            .last()
            .and_then(|(_, level)| level.get(which))
            .cloned()
        else {
            return;
        };
        if entry.is_submenu() {
            let (rows, problems) = self.user_menu_rows(&entry.submenu);
            if let Some(first) = problems.first() {
                let msg = self.i18n.t_args(
                    "usermenu-bad-condition",
                    &[("problem", first.clone().into())],
                );
                self.set_error(msg);
            }
            self.menu_levels.push((entry.label.clone(), rows));
            self.show_user_menu_level();
            return;
        }
        self.menu_levels.clear();
        self.menu_queue = entry.run.clone();
        self.run_menu_next();
    }

    /// Run what is left of a menu entry, one line at a time and down the same road a typed
    /// line takes: the substitutions, `{ask:…}`, the internal commands and the prefixes.
    ///
    /// Who owns the next step is the whole of the logic here. A line that started a shell
    /// owns it — the queue carries on when that shell reports back, and stops if it failed. A
    /// line that asked a question owns it too, because the answer is what starts the shell. A
    /// line that was internal (`cd ..`) is finished the moment it returns, so the next one
    /// goes now. And a line that put something else on the screen — the viewer, the editor —
    /// ends the list: it is in front of the user, and commands running behind it is the
    /// opposite of what the screen says.
    fn run_menu_next(&mut self) {
        while !self.menu_queue.is_empty() {
            let line = self.menu_queue.remove(0);
            self.status_error = false;
            self.run_line(&line);
            if self.shell.is_some() || matches!(self.mode, Mode::Prompt(_)) {
                return;
            }
            if self.status_error || !matches!(self.mode, Mode::Normal) {
                self.menu_queue.clear();
                return;
            }
        }
    }

    /// The selection as a file, one name per line, for `{list}`.
    fn write_list_file(&self, names: &[String]) -> Result<std::path::PathBuf, String> {
        if names.is_empty() {
            return Err(self.i18n.t("cmd-no-selection"));
        }
        let dir = std::env::temp_dir();
        let path = dir.join(format!("fcmd-list-{}.txt", std::process::id()));
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
                Some(path) => match Editor::open(&path.os()) {
                    Ok(mut editor) => {
                        self.tune_editor(&mut editor);
                        self.mode = Mode::Editor(Box::new(editor));
                    }
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
        // The heading goes up now, not when the command ends: what it prints arrives line by
        // line and has to have somewhere to land.
        self.start_user_screen(&line);
        self.requests.push(Request::Shell {
            id,
            line: line.clone(),
            cwd: cwd.as_path().to_path_buf(),
            into,
        });
        // And the panels go off while it runs, the way the console comes forward in FAR: a
        // command whose output only turns up at the end, under a key, is a command one cannot
        // watch. Everything else stays live, `Ctrl+O` brings the panels back early, and the
        // panels come back by themselves when the command ends.
        if self.cfg.ui.command_screen && !self.panels_off && self.mode.is_normal() {
            self.shell_screen = true;
            self.panels_off = true;
            self.user_scroll = 0;
        }
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

    /// `Find next` or `Find previous` in the search dialog, over whichever of the two is
    /// open. Everything the dialog asked for is remembered, so `Shift+F7` repeats it.
    fn apply_search_form(&mut self, form: &Form, button: usize) {
        let query = form.text_value("search.what");
        let mode = match form.choice_value("search.mode").as_str() {
            "hex" => SearchMode::Hex,
            "regex" => SearchMode::Regex,
            _ => SearchMode::Plain,
        };
        let case_sensitive = form.toggle_value("search.case");
        let whole_words = form.toggle_value("search.words");
        let backwards = button == 1;

        self.restore_viewer();
        self.restore_editor();
        if query.is_empty() {
            return;
        }

        if self.mode.viewer().is_some() {
            self.search_mode = mode;
            self.search_case_insensitive = !case_sensitive;
            self.search_whole_words = whole_words;
            self.last_search = Some(query.clone());
            let found = self.mode.viewer_mut().map(|v| {
                v.search_with(
                    &query,
                    mode,
                    !case_sensitive,
                    whole_words,
                    backwards,
                    &CancelToken::new(),
                )
            });
            match found {
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
            return;
        }

        if self.mode.editor().is_some() {
            if let Some(editor) = self.mode.editor_mut() {
                editor.search_mode = mode;
                editor.search_case_insensitive = !case_sensitive;
                editor.search_whole_words = whole_words;
            }
            let found = self
                .mode
                .editor_mut()
                .map(|e| e.find(&query, backwards))
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
    }

    /// `Ctrl+A`: what the selected files *are*. The boxes start from the item under the
    /// cursor, because with several selected there is no one answer to start from and the
    /// cursor is the one the user is looking at.
    fn open_attributes_form(&mut self) {
        let sources = self.job_sources();
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let Some(entry) = self.panel().current().cloned() else {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        };
        // A container is the one place with nothing to set: an entry's permissions are the
        // archive's, and writing them would mean writing the archive again for nothing.
        if sources.iter().any(|p| p.in_container()) {
            let m = self.i18n.t("attr-in-archive");
            self.set_error(m);
            return;
        }
        let directories = sources.iter().any(|p| {
            self.panel()
                .listing
                .row_of_name(&p.display_name())
                .and_then(|row| self.panel().listing.row(row))
                .is_some_and(|e| e.kind.is_dirlike())
        });
        let what = sources[0].display_name();
        // The panel's rows do not carry a mode — no provider is asked for one, because no
        // view shows it as a number — so the one file the boxes start from is asked about
        // here. One `stat` of a file that was just listed costs nothing and happens on the
        // keystroke that opens the dialog; a path that is not local has no mode to read, and
        // the dialog then starts from the flags alone.
        let under_cursor = self.panel().path_of(&entry);
        let mode = mode_of(&under_cursor);
        self.mode = Mode::Form(Box::new(Form::attributes(
            &what,
            sources.len(),
            mode,
            entry.flags,
            entry.mtime,
            directories,
            &self.i18n,
        )));
        self.attr_targets = sources;
    }

    /// OK in the copy dialog: one job per destination, with the rules the rows asked for.
    fn apply_copy_form(&mut self, form: &Form) {
        let sources = std::mem::take(&mut self.copy_sources);
        if sources.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_error(m);
            return;
        }
        let raw = form.text_value("copy.target");
        if raw.is_empty() {
            return;
        }
        let moving = form.title == self.i18n.t("copy-title-move");

        // The filter is parsed before anything starts: a mask with a syntax error should be
        // said so about while the dialog is still on screen, not halfway down a tree.
        let typed = form.text_value("copy.filter");
        let filter = if typed.is_empty() {
            None
        } else {
            match fcmd_core::MaskSet::parse(&typed) {
                Ok(masks) => Some(masks),
                Err(e) => {
                    let msg = self
                        .i18n
                        .t_args("msg-mask-invalid", &[("error", e.to_string().into())]);
                    self.set_error(msg);
                    self.copy_sources = sources;
                    self.mode = Mode::Form(Box::new(form.clone()));
                    return;
                }
            }
        };

        let conflict = form.text_value("copy.conflict");
        let policy = match conflict.as_str() {
            "overwrite" => fcmd_jobs::ConflictPolicy::Overwrite,
            "newer" => fcmd_jobs::ConflictPolicy::OverwriteIfNewer,
            "skip" => fcmd_jobs::ConflictPolicy::Skip,
            "rename" => fcmd_jobs::ConflictPolicy::Rename,
            "append" => fcmd_jobs::ConflictPolicy::Append,
            _ => fcmd_jobs::ConflictPolicy::Ask,
        };
        let options = fcmd_jobs::JobOptions {
            on_conflict: policy,
            preserve: fcmd_jobs::Preserve {
                mtime: form.toggle_value("copy.times"),
                mode: form.toggle_value("copy.mode"),
            },
            follow_symlinks: form.toggle_value("copy.follow_links"),
            only_newer: form.toggle_value("copy.only_newer"),
            filter,
            ..fcmd_jobs::JobOptions::default()
        };
        self.last_copy = CopyChoices {
            conflict: match conflict.as_str() {
                "overwrite" => "overwrite",
                "newer" => "newer",
                "skip" => "skip",
                "rename" => "rename",
                "append" => "append",
                _ => "ask",
            },
            only_newer: options.only_newer,
            follow_links: options.follow_symlinks,
            filter: typed,
        };

        let kind = if moving { JobKind::Move } else { JobKind::Copy };
        let targets = split_targets(&raw);
        if targets.is_empty() {
            return;
        }

        // Three of the four cases are the archive's (SPEC 04 §4), and they are one
        // destination each: there is no sense in renaming one entry into two places. The
        // filesystem cannot rename inside a container, so where it is a rebuild it has to be
        // a rebuild and not a job that would fail halfway.
        if targets.len() == 1 {
            let target = self.resolve_path(&targets[0]);
            let from_archive = sources.iter().all(|p| p.in_container());
            if from_archive && kind == JobKind::Move {
                let dir =
                    targets[0].ends_with('/') || targets[0].ends_with('\\') || sources.len() > 1;
                match Self::changes_renaming(&sources, &target, dir) {
                    Some((archive, changes)) => self.start_rebuild(archive, changes),
                    None => {
                        let m = self.i18n.t("arc-move-out");
                        self.set_error(m);
                    }
                }
                return;
            }
            if target.in_container() {
                // Into an archive, `F6` is a move: the originals go once it is written.
                self.add_into_archive(sources, &target, kind == JobKind::Move);
                return;
            }
        }
        // Several destinations are several jobs, in the order they were typed. A move to
        // more than one place would mean deleting the source after the first, so the ones
        // after it are copies — which is the only reading that does not lose data.
        for (n, target) in targets.iter().enumerate() {
            let target = self.resolve_path(target);
            let kind = if n == 0 { kind } else { JobKind::Copy };
            let spec =
                JobSpec::new(kind, sources.clone(), Some(target)).with_options(options.clone());
            if n == 0 {
                self.start_job(spec);
            } else {
                self.queued_jobs.push_back(spec);
            }
        }
        if targets.len() > 1 {
            let msg = self
                .i18n
                .t_args("copy-several", &[("count", (targets.len() as i64).into())]);
            self.set_status(msg);
        }
    }

    /// OK in the attributes dialog: what the boxes say, applied to what was selected.
    fn apply_attributes_form(&mut self, form: &Form) {
        let sources = std::mem::take(&mut self.attr_targets);
        if sources.is_empty() {
            return;
        }
        let bits = fcmd_jobs::Mode {
            owner_read: form.toggle_value("attr.ur"),
            owner_write: form.toggle_value("attr.uw"),
            owner_exec: form.toggle_value("attr.ux"),
            group_read: form.toggle_value("attr.gr"),
            group_write: form.toggle_value("attr.gw"),
            group_exec: form.toggle_value("attr.gx"),
            other_read: form.toggle_value("attr.or"),
            other_write: form.toggle_value("attr.ow"),
            other_exec: form.toggle_value("attr.ox"),
            setuid: form.toggle_value("attr.setuid"),
            setgid: form.toggle_value("attr.setgid"),
            sticky: form.toggle_value("attr.sticky"),
        };
        let change = fcmd_jobs::Change {
            // The mode is only sent where the dialog offered it: on Windows the twelve boxes
            // are not there, and twelve absent toggles read as `000`, which would be a very
            // surprising thing to apply.
            mode: cfg!(unix).then(|| bits.bits()),
            readonly: Some(form.toggle_value("attr.readonly")),
            hidden: cfg!(windows).then(|| form.toggle_value("attr.hidden")),
            system: cfg!(windows).then(|| form.toggle_value("attr.system")),
            archive: cfg!(windows).then(|| form.toggle_value("attr.archive")),
            mtime: fcmd_ui::parse_stamp(&form.text_value("attr.mtime")),
            atime: fcmd_ui::parse_stamp(&form.text_value("attr.atime")),
        };
        // A time that was typed and makes no sense is worth saying so about: leaving it alone
        // silently would look like the dialog had done what it was asked.
        for (key, parsed) in [("attr.mtime", change.mtime), ("attr.atime", change.atime)] {
            let typed = form.text_value(key);
            if !typed.trim().is_empty() && parsed.is_none() {
                let msg = self.i18n.t_args("attr-bad-time", &[("text", typed.into())]);
                self.set_error(msg);
                return;
            }
        }
        if change.is_empty() {
            let m = self.i18n.t("msg-nothing-to-do");
            self.set_status(m);
            return;
        }
        let recursive = form.toggle_value("attr.recursive");
        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.mode = Mode::Job(Box::new(JobView {
            id,
            kind: JobKind::Attrs,
            progress: Progress::default(),
            question: None,
            cancel: cancel.clone(),
            choice: 0,
        }));
        self.requests.push(Request::SetAttrs {
            id,
            paths: sources,
            change: Box::new(change),
            recursive,
            cancel,
        });
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
        let chosen = fcmd_arc::Format::ALL
            .iter()
            .copied()
            .find(|f| f.extension() == form.text_value("pack.format"))
            .unwrap_or(fcmd_arc::Format::Zip);
        let format = fcmd_arc::Format::of_name(&archive.display_name()).unwrap_or(chosen);
        let level = match form.text_value("pack.level").as_str() {
            "fast" => fcmd_arc::Level::Fast,
            "best" => fcmd_arc::Level::Best,
            _ => fcmd_arc::Level::Normal,
        };
        self.last_pack = format;
        self.last_pack_level = level;
        self.pack_delete = form.toggle_value("pack.delete").then(|| sources.clone());
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
        if fcmd_arc::Format::of_name(&name).is_none() {
            let m = self.i18n.t("unpack-not-an-archive");
            self.set_error(m);
            return;
        }
        // A directory of its own, named after the archive without its extension: unpacking a
        // hundred files straight into a panel is nobody's intention.
        let stem = name
            .strip_suffix(&format!(
                ".{}",
                fcmd_arc::Format::of_name(&name)
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
        let options = fcmd_jobs::JobOptions {
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
        let options = fcmd_jobs::ReplaceOptions {
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
        let mut real_options = fcmd_jobs::JobOptions::default();
        real_options.preserve.mtime = false;
        let real = JobSpec::new(JobKind::Replace, sources.clone(), None)
            .with_replace(options.clone())
            .with_options(real_options.clone());
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
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action: ConfirmAction::Job(Box::new(real)),
            button: 0,
        });
    }

    /// `Ctrl+Alt+E`: a terminal, or back from one.
    ///
    /// Pressing it in a terminal parks that terminal and goes back to the panels, leaving the
    /// program running — which is what every terminal pane does, and the reason this is one
    /// key rather than one to open and another to close. Pressing it in the panels brings a
    /// parked terminal forward if there is one, and starts a shell if there is not: a second
    /// shell is this key again after parking the first, which is deliberate — a key that
    /// quietly started a process every time it was pressed would leave a dozen behind in a day.
    fn toggle_terminal(&mut self) {
        if matches!(self.mode, Mode::Term(_)) {
            self.show_screen(None);
            return;
        }
        if let Some(at) = self
            .background
            .iter()
            .position(|screen| matches!(screen, Mode::Term(_)))
        {
            self.show_screen(Some(at));
            return;
        }
        self.start_terminal();
    }

    /// Start a shell where the active panel is, which is what somebody looking at a directory
    /// and reaching for a shell means.
    fn start_terminal(&mut self) {
        let shell = if self.cfg.term.shell.trim().is_empty() {
            fcmd_term::default_shell()
        } else {
            self.cfg.term.shell.clone()
        };
        // A shell has to start somewhere the operating system knows. A panel inside an archive
        // or on a server has no such directory, so the shell starts where the program was
        // started rather than failing to start at all.
        let here = self.panel().cwd.clone();
        let cwd = (here.is_local() && !here.in_container())
            .then(|| here.as_path().to_path_buf())
            .filter(|p| p.is_dir());
        let (cols, rows) = self.term_area;
        let wake = self.wake.clone();
        match fcmd_term::Terminal::start(&shell, cwd.as_deref(), cols, rows, move || {
            if let Some(wake) = &wake {
                wake();
            }
        }) {
            Ok(term) => {
                // The panels go to the background, as they do for a viewer.
                self.restore_front_screen();
                let front = std::mem::replace(&mut self.mode, Mode::Term(Box::new(term)));
                if front.screen_title().is_some() {
                    self.background.push(front);
                }
                let msg = self.i18n.t_args("term-started", &[("shell", shell.into())]);
                self.set_status(msg);
            }
            Err(e) => self.set_error(format!("{shell}: {e}")),
        }
    }

    /// A key in a terminal. Almost all of them belong to the program.
    ///
    /// Exactly one is ours — whatever is bound to `tool.terminal` — because a terminal that
    /// kept a key map would be stealing keys from the shell: `F3`, `Ctrl+O` and `Tab` all mean
    /// something in there, and none of them mean what they mean in the panels.
    fn term_key(&mut self, key: Key) {
        let ours = self
            .keymap
            .resolve(&[fcmd_cmd::Scope::Panel, fcmd_cmd::Scope::Global], key)
            == Some(Command::ToolTerminal);
        if ours {
            self.show_screen(None);
            return;
        }
        let Mode::Term(term) = &mut self.mode else {
            return;
        };
        // A program that has ended leaves its last screen up to be read; the next key takes
        // it away, which is what a shell's `exit` should feel like.
        if term.finished() {
            self.close_terminal();
            return;
        }
        if let Some(bytes) = crate::termkeys::bytes_for(key) {
            if let Err(e) = term.send(&bytes) {
                self.set_error(e.to_string());
            }
        }
    }

    /// Take the terminal away and go back to the panels, stopping the program if it is still
    /// running: a shell nobody can see and nobody can type into is a shell to end.
    fn close_terminal(&mut self) {
        if let Mode::Term(term) = &mut self.mode {
            term.kill();
        }
        self.mode = Mode::Normal;
        self.read_panel(0);
        self.read_panel(1);
    }

    /// The window changed size, so the terminal in front does too — and the program is told,
    /// which is what makes a full-screen one redraw itself at the new shape.
    pub fn term_resized(&mut self, cols: u16, rows: u16) {
        self.term_area = (cols.max(20), rows.max(4));
        let (cols, rows) = self.term_area;
        if let Mode::Term(term) = &mut self.mode {
            let _ = term.resize(cols, rows);
        }
    }

    /// `Alt+F7`: the dialog, filled in with the last search.
    fn open_find_form(&mut self) {
        self.mode = Mode::Form(Box::new(Form::find(&self.last_find, &self.i18n)));
    }

    /// OK in the find dialog: work out where to search, and start.
    /// Take a colour group from its dialog (SPEC 10 §3.11).
    ///
    /// The block is compiled before it is kept, by the same function the startup uses, so
    /// what the dialog accepts is exactly what the file accepts: a condition that does not
    /// parse comes back on the dialog with the fields still filled in, and a group with no
    /// condition at all is refused rather than painting every row.
    fn apply_highlight_form(&mut self, form: &Form) {
        let group = fcmd_config::HighlightCfg {
            name: form.text_value("hl.name"),
            mask: form.text_value("hl.mask"),
            kind: form.choice_value("hl.kind"),
            size: form.text_value("hl.size"),
            age: form.text_value("hl.age"),
            date_field: form.choice_value("hl.date_field"),
            attrs: form.text_value("hl.attrs"),
            fg: form.text_value("hl.fg"),
            bg: form.text_value("hl.bg"),
        };
        let at = self.highlight_at;
        let name = highlight_name(&group, at.unwrap_or(self.cfg.highlight.len()));
        let groups = fcmd_core::filter::default_groups();
        if let Err(problem) = crate::highlight::rule(&group, &groups, &name) {
            let msg = self.i18n.t_args(
                "hl-bad",
                &[("name", name.into()), ("problem", problem.into())],
            );
            self.set_error(msg);
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        let side = self.active;
        let cursor = match at {
            Some(i) if i < self.cfg.highlight.len() => {
                self.cfg.highlight[i] = group;
                i
            }
            // A new group goes in beside the row the list was on rather than at the end: the
            // order decides which group wins, so where a new one lands matters.
            _ => {
                let at = self.highlight_at.unwrap_or(self.cfg.highlight.len());
                let at = at.min(self.cfg.highlight.len());
                self.cfg.highlight.insert(at, group);
                at
            }
        };
        self.highlight_at = None;
        self.apply_highlight();
        let msg = self.i18n.t("hl-saved");
        self.reopen_highlight(side, cursor);
        self.set_status(msg);
    }

    fn apply_find_form(&mut self, form: &Form) {
        let mask = form.text_value("find.mask");
        if mask.is_empty() {
            let m = self.i18n.t("find-needs-mask");
            self.set_error(m);
            self.mode = Mode::Form(Box::new(form.clone()));
            return;
        }
        // Checked here rather than in the worker: a mistyped size should come back on the
        // dialog that typed it, with the field still filled in, not as a failed search.
        let size = form.text_value("find.size");
        if !size.trim().is_empty() {
            if let Err(e) = fcmd_core::filter::SizeRange::parse(&size) {
                let msg = self.i18n.t_args(
                    "find-bad-size",
                    &[
                        ("text", size.clone().into()),
                        ("error", e.to_string().into()),
                    ],
                );
                self.set_error(msg);
                self.mode = Mode::Form(Box::new(form.clone()));
                return;
            }
        }
        let scope = FindScope::parse(&form.choice_value("find.scope"));
        let mut spec = FindSpec {
            mask,
            text: form.text_value("find.text"),
            case_sensitive: form.toggle_value("find.case"),
            whole_words: form.toggle_value("find.words"),
            hex: form.toggle_value("find.hex"),
            not_containing: form.toggle_value("find.not"),
            size,
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
                        && e.kind != fcmd_core::EntryKind::Parent
                        && panel.selection.contains(e.id)
                    {
                        out.push(panel.path_of(e));
                    }
                }
                if out.is_empty() {
                    if let Some(e) = panel.current() {
                        if e.kind.is_dirlike() && e.kind != fcmd_core::EntryKind::Parent {
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
        if !spec.size.trim().is_empty() {
            label.push_str(" · ");
            label.push_str(spec.size.trim());
        }
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
        if self.panel_go(index, dir, true) {
            self.read_panel(index);
        }
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
        self.start_user_screen(line);
        self.append_user_screen(output);
    }

    /// The heading a command's output goes under, written when it starts rather than when it
    /// ends: the output arrives line by line and has to have somewhere to land.
    fn start_user_screen(&mut self, line: &str) {
        if !self.user_screen.is_empty() {
            self.user_screen.push(String::new());
        }
        self.user_screen.push(format!("$ {line}"));
    }

    /// More output under the heading that is already there. A `\r` with no newline after it
    /// is what a progress bar writes to stay on one line; the screen keeps lines, so the
    /// carriage return is what it means — what follows replaces what came before it.
    fn append_user_screen(&mut self, output: &str) {
        const MAX_LINES: usize = 10_000;
        for row in output.lines() {
            let row = match row.rsplit('\r').next() {
                Some(last) => last,
                None => row,
            };
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
            // A letter takes a row: the one whose label marks that letter with `&`, and
            // failing that the first row the letter starts — which is what FAR does, and
            // what a menu with no markers in its catalogue still needs.
            KeyCode::Char(c) if c.is_alphanumeric() => {
                let wanted = c.to_lowercase().next().unwrap_or(c);
                let same = |a: char| a.to_lowercase().next().unwrap_or(a) == wanted;
                let rows: Vec<Option<String>> = menus
                    .get(menu)
                    .map(|m| {
                        m.items
                            .iter()
                            .map(|e| match e {
                                menu::Entry::Item(i) => Some(self.i18n.t(i.label)),
                                menu::Entry::Separator => None,
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let marked = rows.iter().position(|label| {
                    label
                        .as_deref()
                        .and_then(|l| fcmd_ui::mnemonic(l).1)
                        .is_some_and(same)
                });
                let starts = rows.iter().position(|label| {
                    label
                        .as_deref()
                        .and_then(|l| fcmd_ui::mnemonic(l).0.chars().next())
                        .is_some_and(same)
                });
                if let Some(index) = marked.or(starts) {
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
            Mark::TabLocked => panel.locked,
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
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                // A dialog opened over a viewer or an editor goes back to it, rather than
                // dropping the reader onto the panels.
                self.restore_viewer();
                self.restore_editor();
            }
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
                let last = form.buttons.len().saturating_sub(1);
                form.button = Some(match form.button {
                    None => 0,
                    Some(at) if at < last => at + 1,
                    Some(_) => 0,
                })
            }
            KeyCode::Enter => {
                // Enter answers the dialog wherever the cursor is, which is what a dialog
                // does everywhere else and what FAR does here: `Space` is what turns a box
                // on and off. On the last button it answers "nothing, thank you".
                let cancel = form.buttons.len().saturating_sub(1);
                match form.button {
                    Some(at) if at == cancel => {
                        self.mode = Mode::Normal;
                        self.restore_viewer();
                        self.restore_editor();
                    }
                    _ => self.apply_form(),
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
        // Which button answered: most dialogs have one that does anything, the search has
        // two — the same question in both directions.
        let button = form.button.unwrap_or(0);
        if form.kind == FormKind::Site {
            self.apply_site_form(&form);
            return;
        }
        if form.kind == FormKind::Highlight {
            self.apply_highlight_form(&form);
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
        if form.kind == FormKind::Search {
            self.apply_search_form(&form, button);
            return;
        }
        if form.kind == FormKind::Attributes {
            self.apply_attributes_form(&form);
            return;
        }
        if form.kind == FormKind::Copy {
            self.apply_copy_form(&form);
            return;
        }
        if form.kind == FormKind::Checksum {
            self.apply_checksum_form(&form);
            return;
        }
        if form.kind == FormKind::Sync {
            self.apply_sync_form(&form);
            return;
        }
        if form.kind == FormKind::Link {
            self.apply_link_form(&form);
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
        // The mouse is the terminal's business rather than a panel's, so it travels as a
        // request to the loop that owns it — and it is sent whenever the dialog touched the
        // setting, in either direction.
        if keys.contains(&"ui.mouse") {
            self.requests.push(Request::Mouse {
                on: self.cfg.ui.mouse,
            });
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
            if theme.source == fcmd_ui::ThemeSource::Config {
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
        let scheme = match form.text_value("site.scheme") {
            empty if empty.is_empty() => "sftp".to_string(),
            given => given,
        };
        let cfg = fcmd_config::SiteCfg {
            id,
            scheme,
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
                hotkey: None,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-insert", &[("at", format!("{cursor:08X}").into())]),
                value: "insert".to_string(),
                current: false,
                hotkey: None,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-copy", &[("at", format!("{cursor:08X}").into())]),
                value: "copy".to_string(),
                current: false,
                hotkey: None,
            },
            PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-move", &[("at", format!("{cursor:08X}").into())]),
                value: "move".to_string(),
                current: false,
                hotkey: None,
            },
        ];
        if span > 0 {
            // The operations that need a block, with the size they would act on.
            let size = fcmd_core::format_size(span);
            rows.insert(
                0,
                PickRow {
                    label: self
                        .i18n
                        .t_args("hex-op-write", &[("size", size.clone().into())]),
                    value: "write".to_string(),
                    current: true,
                    hotkey: None,
                },
            );
            rows.push(PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-fill", &[("size", size.clone().into())]),
                value: "fill".to_string(),
                current: false,
                hotkey: None,
            });
            rows.push(PickRow {
                label: self
                    .i18n
                    .t_args("hex-op-xor", &[("size", size.clone().into())]),
                value: "xor".to_string(),
                current: false,
                hotkey: None,
            });
            rows.push(PickRow {
                label: self.i18n.t_args(
                    "hex-op-delete",
                    &[
                        ("size", size.into()),
                        ("was", fcmd_core::format_size(len).into()),
                        ("becomes", fcmd_core::format_size(len - span).into()),
                    ],
                ),
                value: "delete".to_string(),
                current: false,
                hotkey: None,
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
            // `general.confirm_exit` is FAR's own question, off by default. It was a toggle in
            // the settings dialog that changed nothing.
            if self.cfg.general.confirm_exit {
                let title = self.i18n.t("quit-title");
                let lines = vec![self.i18n.t("quit-body"), String::new()];
                self.mode = Mode::Confirm(Confirm {
                    title,
                    lines,
                    action: ConfirmAction::QuitWithScreens,
                    button: 0,
                });
                return;
            }
            self.quit = true;
            return;
        }
        let title = self.i18n.t("screens-quit-title");
        let mut lines = vec![self.i18n.t("screens-quit-body"), String::new()];
        lines.extend(dirty);
        lines.push(String::new());
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action: ConfirmAction::QuitWithScreens,
            button: 0,
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
            .filter(|(_, c)| c.scope == scope || c.scope == fcmd_cmd::Scope::Global)
            .map(|(i, c)| PickRow {
                label: format!("{:<40} {}", c.label, c.plugin),
                value: i.to_string(),
                current: false,
                hotkey: None,
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

    /// Ask where the tunnel goes.
    fn ask_tunnel(&mut self) {
        if self.panel().cwd.authority().is_none() {
            let m = self.i18n.t("tunnel-needs-connection");
            self.set_error(m);
            return;
        }
        let title = self.i18n.t("tunnel-title");
        self.mode = Mode::Prompt(Prompt::new(PromptKind::Tunnel, title, String::new()));
    }

    /// The tunnels that are open. `Enter` on one closes it, which is the only thing there is
    /// to do to a tunnel once it is up.
    fn open_tunnels(&mut self) {
        if self.tunnels.is_empty() {
            let m = self.i18n.t("tunnel-none");
            self.set_status(m);
            self.mode = Mode::Normal;
            return;
        }
        let at = match &self.mode {
            Mode::Pick(pick) if pick.kind == PickKind::Tunnels => pick.cursor,
            _ => 0,
        };
        let rows: Vec<PickRow> = self
            .tunnels
            .iter()
            .map(|t| PickRow {
                label: format!(
                    "{:<46} {} / {} / {}",
                    t.description, t.served, t.live, t.failed
                ),
                value: t.id.to_string(),
                current: false,
                hotkey: None,
            })
            .collect();
        let title = self.i18n.t("tunnel-list-title");
        let at = at.min(rows.len().saturating_sub(1));
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Tunnels, title, rows, at)));
    }

    /// Run a plugin's command, asking first when the plugin says to.
    ///
    /// The host cannot know what somebody else's program will do, so the plugin declares the
    /// question — and a plugin that renames or writes over files is expected to.
    fn run_plugin_command(&mut self, which: usize) {
        let Some(cmd) = self.plugin_commands.get(which).cloned() else {
            return;
        };
        let Some(question) = cmd.confirm.clone() else {
            self.start_plugin_command(which);
            return;
        };
        let selected = self.panel().selection.len();
        let title = cmd.label.clone();
        let mut lines = vec![question, String::new()];
        // What it will act on, counted by us rather than promised by the plugin.
        if cmd.wants("selection") {
            let what = if selected == 0 {
                self.panel()
                    .current()
                    .map(|e| e.name_str().into_owned())
                    .unwrap_or_default()
            } else {
                self.i18n
                    .t_args("plugin-on-selected", &[("count", (selected as i64).into())])
            };
            lines.push(what);
            lines.push(String::new());
        }
        self.mode = Mode::Confirm(Confirm {
            title,
            lines,
            action: ConfirmAction::RunPlugin(which),
            button: 0,
        });
    }

    /// The part that actually asks for the program to be run.
    fn start_plugin_command(&mut self, which: usize) {
        let Some(cmd) = self.plugin_commands.get(which).cloned() else {
            return;
        };
        let mut ctx = fcmd_plug::host::Context::new(&cmd.id);
        let view = |path: &VPath| fcmd_plug::host::PanelView {
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
    fn apply_plugin_reply(&mut self, label: &str, reply: fcmd_plug::host::Reply) {
        use fcmd_plug::host::Action;
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
                    if self.panel_go(side, target, true) {
                        self.read_panel(side);
                    }
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
    pub fn scope(&self) -> fcmd_cmd::Scope {
        self.scopes()[0]
    }

    /// The scopes a key is looked up in, most specific first.
    ///
    /// One entry for everything but the hex editor, which is its own scope and falls through
    /// to the text editor's: the two share a screen and a few keys, and differ on the ones
    /// that matter — `F12` marks a block of bytes in one and lists the open screens in the
    /// other.
    pub fn scopes(&self) -> Vec<fcmd_cmd::Scope> {
        use fcmd_cmd::Scope;
        match &self.mode {
            Mode::Normal => vec![Scope::Panel],
            Mode::Viewer(_) => vec![Scope::Viewer],
            Mode::Hex(_) => vec![Scope::Hex, Scope::Editor],
            Mode::Editor(_) => vec![Scope::Editor],
            // Everything else is a dialog of some kind: a question, a list, a form.
            _ => vec![Scope::Dialog],
        }
    }

    /// `Ctrl+.`: start recording, or stop and ask which key the macro goes on.
    fn toggle_recording(&mut self) {
        match self.recording.take() {
            None => {
                self.recording = Some(Vec::new());
                self.recording_scope = self.scope();
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
            } // (the empty case is above: nothing recorded is nothing to assign)
        }
    }

    /// The key a just-recorded macro will answer to. `Esc` throws the recording away.
    fn macro_key(&mut self, key: Key, steps: Vec<fcmd_cmd::Step>) {
        if key.code == KeyCode::Esc {
            self.mode = Mode::Normal;
            let m = self.i18n.t("macro-dropped");
            self.set_status(m);
            return;
        }
        // The scope recording started in, not the one this key was pressed in: stopping a
        // recording is itself a keystroke, and by now one may be looking at something else.
        let scope = self.recording_scope;
        let title = self
            .i18n
            .t_args("macro-describe", &[("key", key.to_string().into())]);
        self.mode = Mode::Prompt(Prompt::new(
            PromptKind::MacroDescription { key, scope, steps },
            title,
            String::new(),
        ));
    }

    /// The last question: once, or once for every selected file.
    ///
    /// Asked rather than assumed, because `each-selected` is the mode that makes a macro worth
    /// recording and a mode the interface cannot set is a mode nobody finds. `Enter` on the
    /// first row is the ordinary answer, so it costs one keystroke.
    fn ask_macro_play(&mut self, which: fcmd_cmd::Macro) {
        let rows = vec![
            PickRow {
                label: self.i18n.t("macro-play-once"),
                value: fcmd_cmd::Play::Once.as_str().to_string(),
                current: true,
                hotkey: None,
            },
            PickRow {
                label: self.i18n.t("macro-play-each"),
                value: fcmd_cmd::Play::EachSelected.as_str().to_string(),
                current: false,
                hotkey: None,
            },
        ];
        let title = self.i18n.t("macro-play-title");
        self.macro_pending = Some(Box::new(which));
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::MacroPlay, title, rows, 0)));
    }

    /// Store a macro and write the file. A recording that cannot be saved is still in memory
    /// for this session, and says so rather than pretending it was written.
    fn store_macro(&mut self, new: fcmd_cmd::Macro) {
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
                hotkey: None,
            })
            .collect();
        let title = self.i18n.t("macro-list-title");
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Macros, title, rows, 0)));
    }

    /// Play one, asking first when it does something that cannot be undone.
    fn play_macro(&mut self, which: &fcmd_cmd::Macro) {
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
                ],
                action: ConfirmAction::PlayMacro(Box::new(which.clone())),
                button: 0,
            });
            return;
        }
        self.run_macro(which);
    }

    /// Hand the steps back to the application, in order.
    ///
    /// A macro is a list of commands, so playing one is running them — there is no second
    /// interpreter and no way for a macro to do something a key could not.
    fn run_macro(&mut self, which: &fcmd_cmd::Macro) {
        // Thirty-two deep is far more than anything sane and still finite, which is what
        // matters: a macro that plays itself must stop rather than fill the stack.
        if self.playing > 32 {
            let m = self.i18n.t("macro-too-deep");
            self.set_error(m);
            return;
        }
        self.playing += 1;
        match which.play {
            fcmd_cmd::Play::Once => self.run_steps(&which.steps),
            fcmd_cmd::Play::EachSelected => {
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

    fn run_steps(&mut self, steps: &[fcmd_cmd::Step]) {
        for step in steps {
            match step {
                fcmd_cmd::Step::Run(cmd) => self.run(*cmd),
                fcmd_cmd::Step::Press(key) => self.dispatch_key(*key),
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
        // A dialog over a viewer belongs to that viewer: put it back on its screen first, or
        // the list would show the panels in front and the viewer nowhere at all.
        self.park_screen_dialogs();
        let mut rows = vec![PickRow {
            label: self.i18n.t("screens-panels"),
            value: "panels".to_string(),
            current: self.mode.screen_title().is_none(),
            hotkey: None,
        }];
        // The one in front first, then the ones behind it, newest first: the order one
        // stepped away from them, which is the order one comes back through.
        if let Some(title) = self.mode.screen_title() {
            rows.push(PickRow {
                label: self.screen_label(&title, self.mode.screen_modified()),
                value: "front".to_string(),
                current: true,
                hotkey: None,
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
                hotkey: None,
            });
        }
        let at = rows.iter().position(|r| r.current).unwrap_or(0);
        let title = self.i18n.t("screens-title");
        // The list is drawn over the screen that is in front, which therefore has to be put
        // somewhere: `Esc` brings it back, and choosing another row sends it to the
        // background. Dropping it here would lose a file somebody is editing.
        self.park_front_screen();
        self.mode = Mode::Pick(Box::new(Pick::new(PickKind::Screens, title, rows, at)));
    }

    /// Put whatever screen is in front into its waiting place, whichever kind it is.
    fn park_front_screen(&mut self) {
        match &self.mode {
            Mode::Viewer(_) => {
                if let Mode::Viewer(viewer) = std::mem::replace(&mut self.mode, Mode::Normal) {
                    self.viewer_under_prompt = Some(viewer);
                }
            }
            Mode::Editor(_) => self.park_editor(),
            Mode::Hex(_) => self.park_hex(),
            _ => {}
        }
    }

    /// And take it out again. The three are exclusive — one screen is in front — so asking
    /// each in turn is the whole of it.
    fn restore_front_screen(&mut self) {
        self.restore_viewer();
        self.restore_editor();
        self.restore_hex();
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
        // The front screen may be waiting behind a list or a prompt rather than in `mode`.
        self.restore_front_screen();
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
    fn hex_arch(&mut self) -> fcmd_view::Arch {
        let head = self
            .mode
            .hex_mut()
            .map(|e| e.read(0, 4096))
            .unwrap_or_default();
        // The header alone, not the sections: this is one field, read from the front.
        if let Ok(binary) = fcmd_view::analyze(&head, false) {
            if binary.arch.disassemblable() {
                return binary.arch;
            }
        }
        let host = fcmd_view::disasm::host_arch();
        if host.disassemblable() {
            host
        } else {
            fcmd_view::Arch::X86_64
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
        match fcmd_view::assemble(line, arch, address) {
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
                    &[("size", fcmd_core::format_size(b - a).into())],
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
                    &[("size", fcmd_core::format_size(b - a).into())],
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
                                ("size", fcmd_core::format_size(bytes.len() as u64).into()),
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
                match std::fs::read(path.os()) {
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
                                ("size", fcmd_core::format_size(bytes.len() as u64).into()),
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
                    &[("size", fcmd_core::format_size(count).into())],
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
                        &[("size", fcmd_core::format_size(len).into())],
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
                        ("size", fcmd_core::format_size(span).into()),
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
            // And `ask_macro_play`, which carries the macro it is about.
            PickKind::MacroPlay => return,
            // And `open_tunnels`, which knows what the worker last reported.
            PickKind::Tunnels => return,
            // And `open_arc_commands`, which knows which archive it is about.
            PickKind::ArcCommands => return,
            // And `open_with_list`, which knows what the cursor is on.
            PickKind::OpenWith => return,
            // And `open_user_menu`, which reads the file and keeps the level stack.
            PickKind::UserMenu => return,
            PickKind::Languages => Pick::new(
                kind,
                self.plain("menu-languages"),
                fcmd_i18n::LANGUAGES
                    .iter()
                    .map(|(tag, name)| PickRow {
                        label: format!("{name}  ({tag})"),
                        value: (*tag).to_string(),
                        current: *tag == self.i18n.tag(),
                        hotkey: None,
                    })
                    .collect(),
                side,
            ),
            PickKind::Themes => Pick::new(
                kind,
                self.plain("menu-themes"),
                self.themes
                    .iter()
                    .map(|theme| PickRow {
                        label: theme.name.clone(),
                        value: theme.id.clone(),
                        current: theme.id == self.theme.id,
                        hotkey: None,
                    })
                    .collect(),
                side,
            ),
            PickKind::Drives => self.drive_list(side),
            PickKind::Bookmarks => Pick::new(
                kind,
                self.plain("menu-bookmarks"),
                self.bookmark_rows(side),
                side,
            ),
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
                        hotkey: None,
                    })
                    .collect();
                Pick::new(kind, self.plain("menu-folders-history"), rows, side)
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
                        hotkey: None,
                    })
                    .collect();
                Pick::new(kind, self.plain("menu-history"), rows, side)
            }
            PickKind::FileHistory => {
                // Already newest first, which is how it is kept.
                let rows: Vec<PickRow> = self
                    .file_history
                    .iter()
                    .map(|path| PickRow {
                        label: path.clone(),
                        value: path.clone(),
                        current: false,
                        hotkey: None,
                    })
                    .collect();
                Pick::new(kind, self.plain("menu-view-history"), rows, side)
            }
            PickKind::Highlight => {
                let rows: Vec<PickRow> = self
                    .cfg
                    .highlight
                    .iter()
                    .enumerate()
                    .map(|(i, h)| PickRow {
                        label: highlight_row(h, i),
                        value: i.to_string(),
                        current: false,
                        hotkey: None,
                    })
                    .collect();
                Pick::new(kind, self.plain("menu-highlighting"), rows, side)
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
                    self.plain("menu-sort-modes"),
                    keys.iter()
                        .map(|(key, id)| PickRow {
                            label: self.i18n.t(id),
                            value: format!("{key:?}").to_lowercase(),
                            current: panel.sort.key == *key,
                            hotkey: None,
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
        let kind = pick.kind;
        match key.code {
            // Inside a submenu, going back means the menu it was opened from — not out of the
            // menu altogether, which is what one press of `Esc` would otherwise do.
            KeyCode::Esc | KeyCode::Left
                if kind == PickKind::UserMenu && self.menu_levels.len() > 1 =>
            {
                self.menu_levels.pop();
                self.show_user_menu_level();
            }
            KeyCode::Esc | KeyCode::F(10) if kind == PickKind::UserMenu => {
                self.menu_levels.clear();
                self.mode = Mode::Normal;
            }
            KeyCode::Esc | KeyCode::F(10) if self.macro_pending.is_some() => {
                // Dismissing the last question is not throwing the recording away: it is
                // saying "the ordinary way", which is what the first row says too.
                self.mode = Mode::Normal;
                if let Some(pending) = self.macro_pending.take() {
                    self.store_macro(*pending);
                }
            }
            KeyCode::Esc | KeyCode::F(10) => {
                self.mode = Mode::Normal;
                // A list opened over a screen — the block menu over the hex editor, the
                // screens list over whatever was in front — goes back to it when it closes.
                self.restore_front_screen();
            }
            // The order is what the program reads: the first group that matches a row
            // decides it, so moving a row up is the one edit that is not inside a group.
            KeyCode::Up | KeyCode::Down
                if kind == PickKind::Highlight && key.mods.contains(Mods::CTRL) =>
            {
                let (side, at) = (pick.side, pick.cursor);
                let up = key.code == KeyCode::Up;
                let other = if up {
                    at.checked_sub(1)
                } else {
                    (at + 1 < self.cfg.highlight.len()).then_some(at + 1)
                };
                if let Some(other) = other {
                    self.cfg.highlight.swap(at, other);
                    self.apply_highlight();
                    self.reopen_highlight(side, other);
                }
            }
            KeyCode::Up => pick.move_cursor(-1),
            KeyCode::Down => pick.move_cursor(1),
            KeyCode::PageUp => pick.move_cursor(-10),
            KeyCode::PageDown => pick.move_cursor(10),
            KeyCode::Home => pick.cursor = 0,
            KeyCode::End => pick.cursor = pick.rows.len().saturating_sub(1),
            KeyCode::Enter => self.apply_pick(),
            // The hotlist is the one list one edits from inside it: a folder is kept, given
            // up or named where it is read, rather than through a dialog of its own.
            KeyCode::Insert if pick.kind == PickKind::Bookmarks => {
                let (side, cursor) = (pick.side, pick.cursor);
                // The cursor goes to what was just kept, which is the row one came for.
                let kept = self.bookmark_add();
                let cursor = kept
                    .and_then(|path| {
                        self.bookmark_rows(side)
                            .iter()
                            .position(|r| r.value == path)
                    })
                    .unwrap_or(cursor);
                self.reopen_bookmarks(side, cursor);
            }
            // The groups are edited where they are read, like the hotlist: a list of
            // colours one cannot change from inside it would send the reader to the file.
            KeyCode::F(4) if kind == PickKind::Highlight => {
                let at = pick.cursor;
                self.open_highlight_form(Some(at));
            }
            KeyCode::Insert if kind == PickKind::Highlight => {
                let at = pick.cursor;
                self.open_highlight_form(None);
                self.highlight_at = Some(at);
            }
            KeyCode::Delete if kind == PickKind::Highlight => {
                let (side, at) = (pick.side, pick.cursor);
                if at < self.cfg.highlight.len() {
                    let gone = highlight_name(&self.cfg.highlight[at], at);
                    self.cfg.highlight.remove(at);
                    self.apply_highlight();
                    let msg = self.i18n.t_args("hl-removed", &[("name", gone.into())]);
                    self.reopen_highlight(side, at.min(self.cfg.highlight.len()));
                    self.set_status(msg);
                }
            }
            // The one list where `F4` means something: a file in the history is more often
            // one to go back and change than one to read again.
            KeyCode::F(4) if kind == PickKind::FileHistory => {
                let chosen = pick.chosen().map(|r| r.value.clone());
                if let Some(path) = chosen {
                    self.mode = Mode::Normal;
                    self.edit_path(&path);
                }
            }
            KeyCode::Delete if pick.kind == PickKind::Bookmarks => {
                let (side, cursor) = (pick.side, pick.cursor);
                let gone = pick.chosen().map(|r| r.value.clone());
                if let Some(path) = gone {
                    self.bookmarks.retain(|b| b.path != path);
                    let msg = self
                        .i18n
                        .t_args("msg-bookmark-removed", &[("path", path.into())]);
                    self.set_status(msg);
                }
                self.reopen_bookmarks(side, cursor);
            }
            KeyCode::F(4) if pick.kind == PickKind::Bookmarks => {
                let row = pick.cursor;
                let path = pick.chosen().map(|r| r.value.clone());
                let Some(path) = path else { return };
                let text = self
                    .bookmarks
                    .iter()
                    .find(|b| b.path == path)
                    .map(|b| b.name.clone())
                    .unwrap_or_default();
                let title = self.i18n.t("prompt-bookmark-name");
                self.mode = Mode::Prompt(Prompt {
                    kind: PromptKind::BookmarkName { row },
                    title,
                    text,
                    secret: false,
                    sources: Vec::new(),
                });
            }
            // A row with a key of its own: the key takes it, without the arrows.
            KeyCode::Char(c) => {
                if let Some(row) = pick.by_hotkey(c) {
                    pick.cursor = row;
                    self.apply_pick();
                }
            }
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
            PickKind::OpenWith => {
                // The command goes the same road a typed line does: substitutions,
                // `{ask:…}`, conditions and all.
                self.run_line(&row.value);
            }
            PickKind::UserMenu => {
                if let Ok(which) = row.value.parse::<usize>() {
                    self.take_user_menu_row(which);
                }
            }
            PickKind::Plugins => {
                if let Ok(i) = row.value.parse::<usize>() {
                    self.run_plugin_command(i);
                }
            }
            PickKind::Tunnels => {
                if let Ok(id) = row.value.parse::<u64>() {
                    self.requests.push(Request::TunnelClose { id });
                    self.tunnels.retain(|t| t.id != id);
                    let m = self.i18n.t("tunnel-closed");
                    self.set_status(m);
                }
            }
            PickKind::MacroPlay => {
                if let Some(mut pending) = self.macro_pending.take() {
                    pending.play =
                        fcmd_cmd::Play::parse(&row.value).unwrap_or(fcmd_cmd::Play::Once);
                    self.store_macro(*pending);
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
                // Already in front: it is only waiting behind this list.
                "front" => self.restore_front_screen(),
                other => {
                    if let Ok(i) = other.parse::<usize>() {
                        self.show_screen(Some(i));
                    }
                }
            },
            PickKind::ArcCommands => {
                let archive = self.archive_at_hand();
                match (row.value.as_str(), archive) {
                    ("test", Some(path)) => self.start_test(path),
                    ("here", Some(path)) => {
                        let to = self.panel().cwd.clone();
                        self.unpack_into(path, to);
                    }
                    ("to", _) => self.open_unpack_form(),
                    ("delete", Some(path)) => self.ask_delete(vec![path], false),
                    ("add", _) => self.open_pack_form(),
                    _ => {}
                }
            }
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
                                &[("size", fcmd_core::format_size(bytes).into())],
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
            PickKind::Drives | PickKind::FoldersHistory | PickKind::Bookmarks => {
                self.active = pick.side;
                self.go_to_bookmark(&row.value);
            }
            PickKind::CommandHistory => {
                // Chosen from the list, the line goes back on the command line rather than
                // running at once: a command from history is usually a command to edit.
                self.cmd.set(row.value.clone());
            }
            PickKind::FileHistory => {
                // `Enter` reads it, which is what the list is mostly for; `F4` in the list
                // edits it instead, and that is handled where the keys are.
                self.view_path(&row.value);
            }
            PickKind::Highlight => {
                let at = row.value.parse::<usize>().unwrap_or(0);
                self.open_highlight_form(Some(at));
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
    /// Apply `[editor]` to an editor that has just been opened.
    ///
    /// It is done here rather than inside `fcmd-edit` because the editor crate knows nothing
    /// about the configuration file — and because the `all` tab policy *changes the file*,
    /// which is a decision the application makes and says out loud.
    /// The `[viewer]` section, applied to a viewer that is about to open. The chrome flags
    /// are what a file opens with; the keys change them for the file in front of the reader
    /// and write nothing back, because a row taken away to read one long line is not a
    /// preference.
    fn tune_viewer(&mut self, viewer: &mut fcmd_view::Viewer) {
        // Every viewer this program opens comes through here, which is why the history is
        // written here: the path is the one the viewer shows, so a file read off a server is
        // remembered as the remote path and not as the copy that was fetched.
        let shown = viewer.path.clone();
        self.remember_file(&shown);
        let cfg = &self.cfg.viewer;
        viewer.show_key_bar = cfg.key_bar;
        viewer.show_status_line = cfg.status_line;
        viewer.show_scrollbar = cfg.scrollbar;
        viewer.tab_width = cfg.tab_width.clamp(1, 16);
        viewer.wrap = match cfg.wrap_kind() {
            "char" => fcmd_view::Wrap::Char,
            "word" => fcmd_view::Wrap::Word,
            _ => fcmd_view::Wrap::Off,
        };
    }

    fn tune_editor(&mut self, editor: &mut Editor) {
        // As with the viewer: one place every editor passes through. A new file that has no
        // name yet is not a file anybody can come back to, so it is not remembered.
        if let Some(path) = editor.path.clone() {
            self.remember_file(&path);
        }
        let cfg = &self.cfg.editor;
        editor.tab_width = cfg.tab_width.clamp(1, 16);
        editor.expand_tabs = cfg.expand_typed();
        editor.auto_indent = cfg.auto_indent;
        editor.show_whitespace = cfg.show_whitespace;
        editor.cursor_after_match = cfg.cursor_after_match;
        if cfg.expand_on_open() {
            match editor.expand_all_tabs() {
                Ok(0) => {}
                Ok(lines) => {
                    // Said, because the file on the screen is now different from the file on
                    // disk and the next save will write that difference.
                    let msg = self.i18n.t_args(
                        "editor-tabs-expanded",
                        &[("lines", lines.to_string().into())],
                    );
                    self.set_status(msg);
                }
                Err(e) => self.set_error(e.to_string()),
            }
        }
    }

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
        if path.in_container() || path.is_remote() {
            // Fetch, edit the copy, and write it back on save (SPEC 04 §5). Out of a
            // container or off a server, the shape is the same; what differs is what the
            // write-back does, and that is `write_back`'s business.
            self.fetch_remote(path, entry.size, Fetched::Edit);
            return;
        }
        // A raw device is bytes by definition and never lines: it opens in hex, read-only,
        // without the text editor being given a chance to try (SPEC 05 §1.10).
        if std::fs::metadata(path.os())
            .map(|m| fcmd_core::is_device(&m))
            .unwrap_or(false)
        {
            self.open_hex(&path.os());
            return;
        }
        // A file a text view would mangle opens in hex instead (SPEC 05 §2.5). The reader's
        // own test decides, so `F3` and `F4` agree about what a binary is.
        if looks_binary_file(&path.os()) {
            self.open_hex(&path.os());
            return;
        }
        match Editor::open(&path.os()) {
            Ok(mut editor) => {
                self.tune_editor(&mut editor);
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
                            ("size", fcmd_core::format_size(editor.len()).into()),
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
            match Editor::open(&path.os()) {
                Ok(mut editor) => {
                    self.tune_editor(&mut editor);
                    self.mode = Mode::Editor(Box::new(editor));
                }
                Err(e) => self.set_error(format!("{path}: {e}")),
            }
            return;
        }
        self.open_hex(&path.os());
    }

    /// `Shift+F4`: a new file in the active panel's directory.
    pub(crate) fn open_new_editor(&mut self) {
        let mut editor = Editor::empty(None);
        self.tune_editor(&mut editor);
        self.mode = Mode::Editor(Box::new(editor));
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
            Ok(mut editor) => {
                self.tune_editor(&mut editor);
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
        let dir = std::env::temp_dir().join(format!("fcmd-{}", std::process::id()));
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
        // A file that came out of an archive goes back into it, which means writing the
        // archive again (SPEC 04 §4). There is no upload and no stamp to check: the container
        // is a local file, and the job engine is what notices if somebody else moved it.
        if let Some((archive, inner)) = Self::inside(&edit.remote) {
            let msg = self.i18n.t_args(
                "arc-writing-back",
                &[("name", edit.remote.to_string().into())],
            );
            self.set_status(msg);
            self.remote_edit = None;
            self.start_rebuild(
                archive,
                vec![fcmd_arc::Change::Put {
                    inner,
                    source: edit.local,
                }],
            );
            return;
        }
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
            Ok(fcmd_edit::SaveOutcome::Saved { bytes, .. }) => {
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
            Ok(fcmd_edit::SaveOutcome::NothingToDo) => {
                let msg = self
                    .i18n
                    .t_args("editor-nothing-to-save", &[("name", name.into())]);
                self.set_status(msg);
            }
            Err(fcmd_edit::editor::EditorError::NoPath) => self.editor_prompt(
                PromptKind::EditorSaveAs,
                "editor-prompt-save-as",
                String::new(),
            ),
            Err(fcmd_edit::editor::EditorError::ChangedOnDisk(_)) => {
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
                    button: 0,
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
            button: 0,
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
            Command::EditorSaveQuit => {
                self.save_editor(false);
                // Only if the save really happened. A file with no name opens the Save As
                // prompt and a file that changed under us opens a question: closing the
                // editor under either would throw the work away to answer it.
                let saved = self
                    .mode
                    .editor()
                    .map(|e| !e.buffer.modified())
                    .unwrap_or(false);
                if saved {
                    self.close_editor();
                }
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
                let editor = self.mode.editor();
                let text = editor
                    .and_then(|e| e.last_search.clone())
                    .unwrap_or_default();
                let (mode, case, words) = editor
                    .map(|e| {
                        (
                            e.search_mode.as_str(),
                            !e.search_case_insensitive,
                            e.search_whole_words,
                        )
                    })
                    .unwrap_or(("text", false, false));
                let form = Form::search(&text, mode, case, words, &self.i18n);
                self.park_editor();
                self.mode = Mode::Form(Box::new(form));
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

        let rows = self.editor_size.1.max(1) as usize;
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
                Command::EditorShiftRight => editor.shift_right().map_err(|e| e.to_string())?,
                Command::EditorAppend => {
                    if !editor.append_block() {
                        return Err("editor-no-block".into());
                    }
                }
                // Armed, not done: the next key goes in as itself, and `editor_key` is where
                // that happens because that is where a key arrives.
                Command::EditorLiteral => editor.literal_next = true,
                // The screen rather than the file: the cursor goes to the top or the bottom
                // of what is on it, and the view does not move.
                Command::EditorScreenTop => {
                    let line = editor.top.min(editor.buffer.len().saturating_sub(1));
                    editor.move_to(
                        fcmd_edit::buffer::Position::new(line, editor.cursor.col),
                        false,
                        fcmd_edit::BlockKind::Stream,
                    );
                }
                Command::EditorScreenBottom => {
                    let last = editor.buffer.len().saturating_sub(1);
                    let line = (editor.top + rows.saturating_sub(1)).min(last);
                    editor.move_to(
                        fcmd_edit::buffer::Position::new(line, editor.cursor.col),
                        false,
                        fcmd_edit::BlockKind::Stream,
                    );
                }
                Command::EditorOutdent => editor.outdent().map_err(|e| e.to_string())?,
                Command::EditorDeleteToEnd => {
                    editor.delete_to_line_end().map_err(|e| e.to_string())?
                }
                Command::EditorDeleteWordLeft => {
                    editor.delete_word_left().map_err(|e| e.to_string())?
                }
                Command::EditorDeleteWordRight => {
                    editor.delete_word_right().map_err(|e| e.to_string())?
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
        // A block copied or cut in the editor goes on the system clipboard as well as on the
        // editor's own, which is what makes `Ctrl+Ins` there mean the same thing it means in
        // every other program. The editor's clipboard stays the one `Ctrl+V` pastes from:
        // what the terminal holds cannot be read back (SPEC 06 §11).
        if matches!(
            cmd,
            Command::EditorCopy | Command::EditorCut | Command::EditorAppend
        ) && self.cfg.ui.clipboard
        {
            let block = self
                .mode
                .editor()
                .and_then(|e| e.clipboard.clone())
                .map(|(text, _)| text);
            if let Some(text) = block {
                self.copy_to_clipboard(&text);
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

        if format == fcmd_view::Format::Wasm {
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

    /// The same, for a dialog with rows rather than one line.
    fn form_over_viewer(&mut self, form: Form) {
        if let Mode::Viewer(viewer) = std::mem::replace(&mut self.mode, Mode::Normal) {
            self.viewer_under_prompt = Some(viewer);
        }
        self.mode = Mode::Form(Box::new(form));
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
                // In text and dump mode the arrows scroll sideways, because there is a line
                // wider than the window to scroll. A hex dump has no such line: its width is
                // the row width, so the arrows set *that*, one drawable width at a time.
                // FAR puts this on `Alt+arrow`; here it is the plain arrow, because in hex
                // mode it is the only thing sideways can mean.
                Command::ViewerRight if viewer.mode == fcmd_view::Mode::Hex => {
                    viewer.set_bytes_per_row(1, width)
                }
                Command::ViewerLeft if viewer.mode == fcmd_view::Mode::Hex => {
                    viewer.set_bytes_per_row(-1, width)
                }
                Command::ViewerLeft => viewer.scroll_h(-1),
                Command::ViewerRight => viewer.scroll_h(1),
                // And `Ctrl+arrow` moves the whole dump by a single byte, so a structure that
                // does not start on a multiple of the row width can be read in line rather
                // than diagonally. The same keys FAR uses for it.
                // In a table `Ctrl` and an arrow jumps a whole column, which is the unit a
                // table has and a wall of text has not.
                Command::ViewerShiftBack if viewer.mode == fcmd_view::Mode::Table => {
                    viewer.scroll_h_column(false)
                }
                Command::ViewerShiftForward if viewer.mode == fcmd_view::Mode::Table => {
                    viewer.scroll_h_column(true)
                }
                Command::ViewerShiftBack => viewer.shift(-1),
                Command::ViewerShiftForward => viewer.shift(1),
                Command::ViewerWrap => viewer.toggle_wrap(),
                Command::ViewerHex => viewer.toggle_hex(),
                Command::ViewerModeNext => {
                    // text → dump → hex → text, with the rendered document in the ring where
                    // there is one. Disassembly is F5, not part of the cycle, because it only
                    // applies to some files.
                    let renders = viewer.renders_markdown();
                    let tabular = viewer.reads_table();
                    let next = match viewer.mode {
                        fcmd_view::Mode::Markdown | fcmd_view::Mode::Table => fcmd_view::Mode::Text,
                        fcmd_view::Mode::Text => fcmd_view::Mode::Dump,
                        fcmd_view::Mode::Dump => fcmd_view::Mode::Hex,
                        fcmd_view::Mode::Hex | fcmd_view::Mode::Disasm if renders => {
                            fcmd_view::Mode::Markdown
                        }
                        fcmd_view::Mode::Hex | fcmd_view::Mode::Disasm if tabular => {
                            fcmd_view::Mode::Table
                        }
                        fcmd_view::Mode::Hex | fcmd_view::Mode::Disasm => fcmd_view::Mode::Text,
                    };
                    viewer.set_mode(next);
                }
                Command::ViewerMarkdown => {
                    if viewer.mode == fcmd_view::Mode::Markdown {
                        viewer.set_mode(fcmd_view::Mode::Text);
                    } else if !viewer.as_markdown() {
                        error = Some("viewer-not-markdown".into());
                    }
                }
                Command::ViewerTable => {
                    if viewer.mode == fcmd_view::Mode::Table {
                        viewer.set_mode(fcmd_view::Mode::Text);
                    } else if !viewer.as_table() {
                        error = Some("viewer-not-table".into());
                    }
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
                // The chrome, one row at a time. Nothing is remembered in the configuration
                // by pressing these: the setting is what a file *opens* with, and this is
                // what this reader wants of this file now.
                Command::ViewerChromeKeyBar => viewer.show_key_bar = !viewer.show_key_bar,
                Command::ViewerChromeStatus => viewer.show_status_line = !viewer.show_status_line,
                Command::ViewerChromeScrollbar => viewer.show_scrollbar = !viewer.show_scrollbar,
                Command::ViewerPaneNext => viewer.toggle_struct_focus(),
                Command::ViewerCodepageNext => viewer.next_codepage(),
                Command::ViewerFollow => viewer.toggle_follow(),
                Command::ViewerBytesLess => viewer.set_bytes_per_row(-1, width),
                Command::ViewerBytesMore => viewer.set_bytes_per_row(1, width),
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
        if entry.kind != fcmd_core::EntryKind::File {
            return None;
        }
        let name = entry.name_str();
        if !fcmd_vfs::ArcVfs::looks_like_archive(std::path::Path::new(&*name)) {
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
            let mut wanted: Vec<fcmd_core::EntryId> = Vec::new();
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

    /// Compare the two panels by what is *in* the files.
    ///
    /// The other comparison asks whether two files have the same name, size and time, which is
    /// what every file manager means by comparing folders and what is wanted nine times out of
    /// ten. This is the tenth: two trees that were built from the same source, or copied by
    /// something that did not keep the timestamps, where the only question that means anything
    /// is whether the bytes are the same.
    ///
    /// The pairs are matched by name, as the other one matches them. A pair whose sizes differ
    /// is settled from the listing without opening anything; the rest are read. A file with no
    /// counterpart on the other side is selected too, because it is a difference, and it is
    /// counted separately so the message does not read as if its bytes had been compared with
    /// something.
    fn compare_contents(&mut self) {
        // Reading is through the local filesystem — the same limit the search has. A remote
        // panel would mean a fetch per file, and a fetch per file is a different feature with
        // a different progress bar.
        if self
            .panels
            .iter()
            .any(|p| !p.cwd.is_local() || p.cwd.in_container())
        {
            let m = self.i18n.t("compare-contents-local");
            self.set_error(m);
            return;
        }
        if let Some((_, cancel)) = self.compare.take() {
            cancel.cancel();
        }

        let key = |entry: &Entry| entry.name_str().to_lowercase();
        let mut right: std::collections::HashMap<String, (VPath, u64)> =
            std::collections::HashMap::new();
        let listing = &self.panels[1].listing;
        for row in 0..listing.len() {
            let entry = listing.row(row).expect("row in range");
            if entry.kind.is_dirlike() {
                continue;
            }
            right.insert(key(entry), (self.panels[1].path_of(entry), entry.size));
        }

        let mut pairs = Vec::new();
        let mut alone: Vec<String> = Vec::new();
        let listing = self.panels[0].listing.clone();
        for row in 0..listing.len() {
            let entry = listing.row(row).expect("row in range");
            if entry.kind.is_dirlike() {
                continue;
            }
            match right.get(&key(entry)) {
                Some((path, size)) => pairs.push(ComparePair {
                    name: entry.name_str().into_owned(),
                    left: self.panels[0].path_of(entry),
                    right: path.clone(),
                    left_size: entry.size,
                    right_size: *size,
                }),
                None => alone.push(entry.name_str().into_owned()),
            }
        }
        // And the other way round: a file only on the right is also a difference.
        let matched: std::collections::HashSet<String> =
            pairs.iter().map(|p| p.name.to_lowercase()).collect();
        let listing = self.panels[1].listing.clone();
        for row in 0..listing.len() {
            let entry = listing.row(row).expect("row in range");
            if entry.kind.is_dirlike() || matched.contains(&key(entry)) {
                continue;
            }
            alone.push(entry.name_str().into_owned());
        }

        self.compare_alone = alone;
        if pairs.is_empty() {
            let unmatched = std::mem::take(&mut self.compare_alone);
            self.select_in_both(&unmatched);
            let m = self.i18n.t("compare-contents-none");
            self.set_status(m);
            return;
        }

        let id = self.next_job_id;
        self.next_job_id += 1;
        let cancel = CancelToken::new();
        self.compare = Some((id, cancel.clone()));
        let total = pairs.len() as i64;
        self.compare_pairs = pairs.len();
        let msg = self.i18n.t_args(
            "compare-contents-working",
            &[("done", 0.into()), ("total", total.into())],
        );
        self.set_status(msg);
        self.requests
            .push(Request::CompareContents { id, pairs, cancel });
    }

    /// What a finished comparison does to the panels: select what differs on both sides, and
    /// say what was found in one line.
    fn apply_compare(
        &mut self,
        differ: &[String],
        line_endings: &[String],
        unreadable: u64,
        stopped: bool,
    ) {
        let alone = std::mem::take(&mut self.compare_alone);
        let mut names: Vec<String> = differ.to_vec();
        names.extend(line_endings.iter().cloned());
        names.extend(alone.iter().cloned());
        let pairs = self.compare_pairs;
        self.select_in_both(&names);

        if stopped {
            let m = self.i18n.t("compare-contents-stopped");
            self.set_status(m);
            return;
        }
        let mut line = if differ.is_empty() && alone.is_empty() && line_endings.is_empty() {
            self.i18n
                .t_args("compare-contents-same", &[("pairs", (pairs as i64).into())])
        } else {
            self.i18n.t_args(
                "compare-contents-done",
                &[
                    ("differ", (differ.len() as i64).into()),
                    ("pairs", (pairs as i64).into()),
                ],
            )
        };
        if !line_endings.is_empty() {
            let more = self.i18n.t_args(
                "compare-contents-eol",
                &[("count", (line_endings.len() as i64).into())],
            );
            line.push_str(", ");
            line.push_str(&more);
        }
        if !alone.is_empty() {
            let more = self.i18n.t_args(
                "compare-contents-only",
                &[("count", (alone.len() as i64).into())],
            );
            line.push_str(", ");
            line.push_str(&more);
        }
        if unreadable > 0 {
            let more = self.i18n.t_args(
                "compare-contents-unread",
                &[("count", (unreadable as i64).into())],
            );
            line.push_str(", ");
            line.push_str(&more);
        }
        self.set_status(line);
    }

    /// Mark these names in whichever panel has them, and nothing else.
    fn select_in_both(&mut self, names: &[String]) {
        let wanted: std::collections::HashSet<String> =
            names.iter().map(|n| n.to_lowercase()).collect();
        for side in 0..2 {
            let listing = self.panels[side].listing.clone();
            self.panels[side].selection.clear();
            for row in 0..listing.len() {
                let entry = listing.row(row).expect("row in range");
                if entry.kind.is_dirlike() {
                    continue;
                }
                if wanted.contains(&entry.name_str().to_lowercase()) {
                    self.panels[side].selection.set(entry.id, true);
                }
            }
            self.panels[side].rebuild();
        }
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
            let here = fcmd_vfs::local::real_path(&self.panels[1 - index].cwd);
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
        if next == Content::Info {
            // The information panel spells out the row under the cursor, description and
            // all, so the file that holds them is worth reading now (SPEC 02 §9).
            let described = 1 - index;
            self.ask_for_descriptions(described);
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
                if self.panels[other].cwd != path && self.panel_go(other, path, true) {
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
        match fcmd_view::FileSource::open(&path.os()) {
            Ok(source) => {
                let mut viewer = Viewer::new(Box::new(source), path.clone());
                self.tune_viewer(&mut viewer);
                // A quick view is a panel's width of rows: it has no chrome of its own to
                // take away, and the scrollbar would sit on the panel's frame.
                viewer.show_key_bar = false;
                viewer.show_status_line = false;
                viewer.show_scrollbar = false;
                self.quick = Some(Box::new(viewer));
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
        // Going *up* puts the cursor back on what was left, which is what FAR does and what
        // makes stepping in and out of a tree bearable: one `Enter` and one `..` leave the
        // cursor where it was rather than at the top of a directory with four hundred rows
        // in it. It is set here rather than in the three callers — `Enter` on `..`, the
        // `..` key, `cd ..` — because all three are this one call, and because the fourth
        // case is the one that is easiest to miss: coming out of an archive, where the name
        // to go back to is the archive's own and not the directory's inside it.
        let leaving = self.panels[index].cwd.clone();
        let came_from = leaving
            .parent()
            .map(|parent| parent.normalised())
            .filter(|parent| *parent == path.normalised())
            .map(|_| leaving.display_name());
        if !self.panel_go(index, path, true) {
            return;
        }
        if let Some(name) = came_from {
            self.panels[index].want_cursor = Some(name);
        }
        self.read_panel(index);
    }

    // ── the folders kept to hand (SPEC 02 §11) ──────────────────────────────────────────

    /// `Ctrl+Shift+0..9`: this panel's directory answers to that digit from now on.
    ///
    /// A digit holds one folder, so setting it over a folder that had it replaces that
    /// folder — and the name with it, because the name belonged to the old one. A folder
    /// already in the list without a digit is given the digit rather than added twice.
    fn bookmark_set(&mut self, digit: u8) {
        let path = self.panel().cwd.to_string();
        self.bookmarks
            .retain(|b| b.digit != Some(digit) || b.path == path);
        match self.bookmarks.iter_mut().find(|b| b.path == path) {
            Some(known) => {
                if known.digit != Some(digit) {
                    known.digit = Some(digit);
                }
            }
            None => self.bookmarks.push(fcmd_config::Bookmark {
                path: path.clone(),
                name: String::new(),
                digit: Some(digit),
            }),
        }
        let msg = self.i18n.t_args(
            "msg-bookmark-set",
            &[("digit", digit.to_string().into()), ("path", path.into())],
        );
        self.set_status(msg);
    }

    /// `Alt+0..9`: go to the folder that answers to that digit.
    fn bookmark_go(&mut self, digit: u8) {
        let Some(path) = self
            .bookmarks
            .iter()
            .find(|b| b.digit == Some(digit))
            .map(|b| b.path.clone())
        else {
            let msg = self
                .i18n
                .t_args("msg-bookmark-unset", &[("digit", digit.to_string().into())]);
            self.set_error(msg);
            return;
        };
        self.go_to_bookmark(&path);
    }

    /// Where a folder in the list takes you. A bookmark may be on a server, in which case
    /// going there is connecting first — the same route the drive list takes.
    fn go_to_bookmark(&mut self, path: &str) {
        let target = VPath::parse(path);
        if target.is_remote() {
            match fcmd_net::Site::parse_url(path) {
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

    /// Where the copies fetched off a server and out of archives are put. The same
    /// expression `fetch_entry` builds, in one place so that the two cannot disagree.
    pub fn fetch_dir() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("fcmd-{}", std::process::id()))
    }

    /// Put a file at the top of the view-and-edit history (SPEC 06 §6).
    ///
    /// A file that is already in it moves rather than appearing twice: the list is "where I
    /// have been", and a file read five times is one place. Anything inside the temporary
    /// directory is left out — the copies fetched off a server and out of archives live
    /// there, and a history of paths that will not exist tomorrow is worse than no history.
    fn remember_file(&mut self, path: &VPath) {
        let text = path.to_string();
        if text.is_empty() {
            return;
        }
        // The copies fetched off a server and out of archives live in a directory of our own
        // under the temporary one, and they will not exist tomorrow: a remote file is
        // remembered by its remote path (which is what the viewer is given) and its copy is
        // left out. Everything else under the temporary directory is a real file somebody
        // chose to open.
        if path.is_local() && path.os().starts_with(Self::fetch_dir()) {
            return;
        }
        self.file_history.retain(|p| *p != text);
        self.file_history.insert(0, text);
        let keep = self.cfg.ui.file_history.max(1);
        self.file_history.truncate(keep);
    }

    /// The rows of the hotlist: the ones with a digit first and in order, then the rest as
    /// they were added. The digit is the row's own key, so one press takes it.
    fn bookmark_rows(&self, side: usize) -> Vec<PickRow> {
        let current = self.panels[side].cwd.to_string();
        let mut order: Vec<usize> = (0..self.bookmarks.len()).collect();
        order.sort_by_key(|&i| (self.bookmarks[i].digit.unwrap_or(u8::MAX), i));
        order
            .into_iter()
            .map(|i| {
                let b = &self.bookmarks[i];
                // The digit is the row's key, and the list draws a column of those itself:
                // putting it in the label too would print it twice.
                let label = if b.name.trim().is_empty() {
                    b.path.clone()
                } else {
                    format!("{}  —  {}", b.name, b.path)
                };
                let row = PickRow::plain(label, b.path.clone(), b.path == current);
                match b.digit {
                    Some(d) => row.with_key((b'0' + d) as char),
                    None => row,
                }
            })
            .collect()
    }

    /// `Ins` in the hotlist: keep this panel's directory, with no digit of its own. The
    /// path is handed back so the list can reopen with the cursor on what was just kept.
    fn bookmark_add(&mut self) -> Option<String> {
        let path = self.panels[self.active].cwd.to_string();
        if self.bookmarks.iter().any(|b| b.path == path) {
            let msg = self.i18n.t("msg-bookmark-known");
            self.set_error(msg);
            return None;
        }
        self.bookmarks.push(fcmd_config::Bookmark {
            path: path.clone(),
            name: String::new(),
            digit: None,
        });
        let msg = self
            .i18n
            .t_args("msg-bookmark-added", &[("path", path.clone().into())]);
        self.set_status(msg);
        Some(path)
    }

    /// Open the hotlist again after a key that changed it, with the cursor where it was.
    /// The colours group `index` paints with, for the row that stands for it in the list.
    ///
    /// `None` when the group names neither ink — there is nothing to show, and the row keeps
    /// the dialog's own colours rather than being drawn in a guess.
    pub fn highlight_ink(&self, index: usize) -> Option<fcmd_ui::Style> {
        let h = self.cfg.highlight.get(index)?;
        if h.fg.is_empty() && h.bg.is_empty() {
            return None;
        }
        let ink = |text: &str, fallback: &str| {
            if text.is_empty() {
                self.theme.color(fallback)
            } else {
                fcmd_ui::highlight::Ink::parse(text).color(&self.theme)
            }
        };
        Some(fcmd_ui::Style::new(
            ink(&h.fg, "dialog.fg"),
            ink(&h.bg, "dialog.bg"),
        ))
    }

    /// Open the list of colour groups again, with the cursor where the edit left it.
    fn reopen_highlight(&mut self, side: usize, cursor: usize) {
        let rows: Vec<PickRow> = self
            .cfg
            .highlight
            .iter()
            .enumerate()
            .map(|(i, h)| PickRow {
                label: highlight_row(h, i),
                value: i.to_string(),
                current: false,
                hotkey: None,
            })
            .collect();
        let title = self.plain("menu-highlighting");
        let mut pick = Pick::new(PickKind::Highlight, title, rows, side);
        pick.cursor = cursor.min(pick.rows.len().saturating_sub(1));
        self.mode = Mode::Pick(Box::new(pick));
    }

    /// The dialog for one colour group, or for a new one (`index` is `None`).
    pub fn open_highlight_form(&mut self, index: Option<usize>) {
        let group = index.and_then(|i| self.cfg.highlight.get(i)).cloned();
        self.highlight_at = index;
        self.mode = Mode::Form(Box::new(Form::highlight(group.as_ref(), &self.i18n)));
    }

    /// Compile the groups again and put them on the panels. Called after every change, so
    /// the screen behind the dialog shows what was just asked for.
    ///
    /// A group that does not parse cannot get this far — the dialog refuses it — but the
    /// file can still hold one, and the warning it produced at startup has already been
    /// shown: here it is simply left out, as it was then.
    fn apply_highlight(&mut self) {
        let mut problems = Vec::new();
        self.highlight = crate::highlight::groups(&self.cfg, &mut problems);
    }

    fn reopen_bookmarks(&mut self, side: usize, cursor: usize) {
        let rows = self.bookmark_rows(side);
        let title = self.plain("menu-bookmarks");
        let mut pick = Pick::new(PickKind::Bookmarks, title, rows, side);
        pick.cursor = cursor.min(pick.rows.len().saturating_sub(1));
        self.mode = Mode::Pick(Box::new(pick));
    }

    /// Put back the tabs a previous session had on this side (SPEC 02 §1.6).
    ///
    /// Two rules, both of them about what a user asked for most recently. The directory this
    /// session was started in wins for the tab that is on screen, because that is what
    /// running a program in a directory means — unless that tab is locked, since staying put
    /// is the whole of what a lock promises. And a tab that was on a server or in a bucket is
    /// not restored at all: the connection it needs is not open, and a row of tabs that each
    /// say "not connected" is worse than a row that is not there.
    pub fn restore_tabs(&mut self, side: usize, saved: &[fcmd_config::TabState], current: usize) {
        let keep: Vec<&fcmd_config::TabState> = saved
            .iter()
            .filter(|t| VPath::parse(&t.path).is_local())
            .collect();
        if keep.len() < 2 {
            return;
        }
        // Which of the ones that survived was on screen: the saved index counts the dropped
        // ones too, so it is the nearest survivor rather than the number itself.
        let current = saved
            .get(current)
            .and_then(|want| keep.iter().position(|t| std::ptr::eq(*t, want)))
            .unwrap_or(0);
        let here = self.panels[side].cwd.clone();
        let panel = &self.panels[side];
        let (sort, view, hidden) = (panel.sort, panel.view, panel.show_hidden);
        let mut row = crate::tabs::Row::one();
        for (i, saved) in keep.iter().enumerate() {
            let at = if i == current && !saved.locked {
                here.clone()
            } else {
                VPath::parse(&saved.path)
            };
            let mut panel = Panel::new(at, sort, view, hidden).with_groups(&self.sort_groups);
            panel.name = (!saved.name.trim().is_empty()).then(|| saved.name.clone());
            panel.locked = saved.locked;
            if i == 0 {
                self.panels[side] = panel;
            } else {
                row.append(panel);
            }
        }
        self.tabs[side] = row;
        self.tabs[side].switch(current, &mut self.panels[side], &mut self.trees[side]);
        self.read_panel(side);
    }

    /// Another tab on this side: the directory under the cursor if the cursor is on one, and
    /// otherwise the one this tab is already showing — a duplicate, which is what one wants
    /// before going two different ways from the same place.
    fn tab_new(&mut self) {
        let side = self.active;
        let panel = &self.panels[side];
        let under = panel
            .current()
            .filter(|e| e.kind != fcmd_core::EntryKind::Parent && e.kind.is_dirlike());
        let where_to = match under {
            Some(entry) => panel.path_of(entry),
            None => panel.cwd.clone(),
        };
        let fresh = Panel::new(where_to, panel.sort, panel.view, panel.show_hidden)
            .with_groups(&self.sort_groups);
        self.tabs[side].open(fresh, &mut self.panels[side], &mut self.trees[side]);
        self.read_panel(side);
    }

    /// Close this tab and show its neighbour.
    fn tab_close(&mut self) {
        let side = self.active;
        if self.tabs[side].close(&mut self.panels[side], &mut self.trees[side]) {
            self.read_panel(side);
        } else {
            let msg = self.i18n.t("msg-tab-last");
            self.set_error(msg);
        }
    }

    /// Show tab `to` of the active side. The directory is read again: a tab left open for an
    /// hour is showing what was there an hour ago.
    fn tab_switch(&mut self, to: usize) {
        let side = self.active;
        if self.tabs[side].switch(to, &mut self.panels[side], &mut self.trees[side]) {
            self.read_panel(side);
        }
    }

    /// Hand this tab to the other side. The focus stays where it is, on the tab that took
    /// its place: moving a tab across is about where the directory should live, not about
    /// which panel one is working in.
    fn tab_move(&mut self) {
        let side = self.active;
        let other = 1 - side;
        let taken = self.tabs[side].take(&mut self.panels[side], &mut self.trees[side]);
        let Some(tab) = taken else {
            let msg = self.i18n.t("msg-tab-last");
            self.set_error(msg);
            return;
        };
        self.tabs[other].give(tab, &mut self.panels[other], &mut self.trees[other]);
        self.read_panel(side);
        self.read_panel(other);
        let msg = self.i18n.t("msg-tab-moved");
        self.set_status(msg);
    }

    /// Change a panel's directory, unless its tab is locked.
    ///
    /// The refusal lives in `Panel::go`, which has nothing but the panel to say it with; this
    /// is the half with a status line in it. A lock that silently did nothing would read as a
    /// key that had stopped working.
    fn panel_go(&mut self, index: usize, path: VPath, push_history: bool) -> bool {
        if self.panels[index].go(path, push_history) {
            return true;
        }
        let msg = self.i18n.t("msg-tab-locked");
        self.set_error(msg);
        false
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
            // Three keys the editor has always answered and the bar never said: mark a
            // block, hand the file to the viewer, assemble what is in it. A key that works
            // and is not written down is a key nobody presses.
            (3, self.i18n.t("key-mark")),
            (4, self.i18n.t("key-quit")),
            (6, self.i18n.t("key-view")),
            (7, self.i18n.t("key-search")),
            (8, self.i18n.t("key-replace")),
            (9, self.i18n.t("key-asm")),
            (10, self.i18n.t("key-quit")),
            (12, self.i18n.t("key-screens")),
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
            // F4 is a toggle, so it is named after where it goes rather than after where it
            // is: in hex it offers the way back, to text or to the dump, whichever the file
            // was being read as.
            (4, self.i18n.t(self.viewer_toggle_key())),
            (5, self.i18n.t("key-disasm")),
            (6, self.i18n.t("key-edit")),
            (7, self.i18n.t("key-search")),
            (8, self.i18n.t("key-code")),
            (10, self.i18n.t("key-quit")),
            (11, self.i18n.t("key-struct")),
            (12, self.i18n.t("key-screens")),
        ]
    }

    /// What F4 switches the viewer to from where it is now.
    fn viewer_toggle_key(&self) -> &'static str {
        match self.mode.viewer() {
            Some(v) if v.mode == fcmd_view::Mode::Hex => match v.base_mode {
                fcmd_view::Mode::Dump => "key-dump",
                fcmd_view::Mode::Markdown => "key-doc",
                _ => "key-text",
            },
            _ => "key-hex",
        }
    }

    /// The viewer's status line.
    pub fn viewer_status(&self, status: &fcmd_view::Status) -> String {
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
        if status.wrap != fcmd_view::Wrap::Off {
            text.push_str(&format!("  {}", status.wrap.as_str()));
        }
        if status.follow {
            text.push_str(&format!("  {}", self.i18n.t("key-follow")));
        }
        if let Some(query) = &status.search {
            text.push_str(&format!("  /{query}"));
        }
        // A table says what it was read as, because the delimiter was a guess and the shape
        // is what tells the reader the guess was right.
        if let Some(note) = &status.table {
            text.push_str(&format!("  {note}"));
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
            // `F11` has opened the plugin commands since the plugins did, and the bar did
            // not say so: FAR writes `Plugin` there too.
            (11, "key-plugins"),
            (12, "key-screens"),
        ]
        .iter()
        .map(|(n, id)| (*n, self.i18n.t(id)))
        .collect()
    }

    /// The key bar while a modifier is held down.
    ///
    /// FAR swaps the whole bar as you press Shift, Ctrl or Alt, which is how anyone ever
    /// learns what those keys do. The bar is built from the keymap rather than written out,
    /// so a key nobody has bound shows nothing instead of promising something the program
    /// will not do — and a keymap of the reader's own is described correctly for free.
    pub fn keybar_for(&self, mods: Mods) -> Vec<(u8, String)> {
        if mods == Mods::NONE {
            return self.keybar();
        }
        (1..=12u8)
            .filter_map(|n| {
                let key = Key {
                    code: KeyCode::F(n),
                    mods,
                };
                let cmd = self.keymap.resolve(&[Scope::Panel], key)?;
                Some((n, self.i18n.t(keybar_label(cmd)?)))
            })
            .collect()
    }

    /// Where `F1` was pressed, which is what it should be about.
    fn help_place(&self) -> crate::help::Where {
        use crate::help::Where;
        match &self.mode {
            Mode::Viewer(_) => Where::Viewer,
            Mode::Editor(_) => Where::Editor,
            Mode::Hex(_) => Where::Hex,
            Mode::Form(_) | Mode::Pick(_) | Mode::Prompt(_) | Mode::Confirm(_) => Where::Dialog,
            _ => Where::Panels,
        }
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
            Chapter::Here(place) => self.help_here(place, width),
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

    /// The keys of one screen, each with what it does — the page `F1` opens.
    ///
    /// Generated from the keymap as loaded and from each command's own description, so it
    /// describes the program that is running rather than the one that was written about: a
    /// binding of the reader's own appears here with the right words next to it.
    fn help_here(&self, place: crate::help::Where, width: usize) -> Vec<String> {
        let mut lines = vec![format!("# {}", place.title()), String::new()];

        // One row per command rather than per binding: a command reached by three keys is
        // one thing you can do, and listing it three times is how a key list becomes
        // unreadable.
        let mut rows: Vec<(Command, Vec<String>)> = Vec::new();
        for scope in place.scopes() {
            for (bound, key, cmd) in self.keymap.bindings() {
                if bound != *scope {
                    continue;
                }
                match rows.iter_mut().find(|(c, _)| *c == cmd) {
                    Some((_, keys)) => keys.push(key.to_string()),
                    None => rows.push((cmd, vec![key.to_string()])),
                }
            }
        }

        // By subject rather than by the order the keymap file happens to be written in: a
        // list of keys one reads top to bottom is a list grouped the way one thinks.
        rows.sort_by_key(|(cmd, _)| (help_group(cmd.id().as_str()).0, cmd.id()));

        let width_of_keys = rows
            .iter()
            .map(|(_, keys)| keys.join(", ").chars().count())
            .max()
            .unwrap_or(0)
            .min(24);
        let mut group = usize::MAX;
        for (cmd, keys) in &rows {
            let (order, title) = help_group(cmd.id().as_str());
            if order != group {
                group = order;
                if lines.len() > 2 {
                    lines.push(String::new());
                }
                lines.push(format!("## {title}"));
            }
            let keys = keys.join(", ");
            let text = format!("  {keys:<width_of_keys$}  {}", cmd.describe());
            // The renderer wraps prose; this is a table, so it is cut rather than folded.
            lines.push(fcmd_ui::truncate_cols(&text, width.max(20)));
        }

        lines.push(String::new());
        match place.prose() {
            Some(_) => lines.push(
                "Esc for the rest of the manual, where this screen has a chapter of its own."
                    .to_string(),
            ),
            None => lines.push("Esc for the rest of the manual.".to_string()),
        }
        lines
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
            "`fcmd --dump-config` prints the same thing to a terminal.".into(),
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
        Ok(read) => fcmd_view::looks_binary(&head[..read]),
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
            | Command::ViewerMarkdown
            | Command::ViewerFollowBranch
            | Command::ViewerBack
            | Command::ViewerStructure
            | Command::ViewerChromeKeyBar
            | Command::ViewerChromeStatus
            | Command::ViewerChromeScrollbar
            | Command::ViewerPaneNext
            | Command::ViewerModeNext
            | Command::ViewerCodepageNext
            | Command::ViewerFollow
            | Command::ViewerLeft
            | Command::ViewerRight
            | Command::ViewerUndoPosition
            | Command::ViewerBytesLess
            | Command::ViewerBytesMore
            | Command::ViewerShiftBack
            | Command::ViewerShiftForward
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
    sort_key(s).unwrap_or(SortKey::Name)
}

/// A sort key by name, or `None` for a word that is not one. The chain needs to tell the two
/// apart — a tie-breaker nobody can name is a line worth a message, not a quiet sort by name.
fn sort_key(s: &str) -> Option<SortKey> {
    Some(match s.trim().to_ascii_lowercase().as_str() {
        "name" => SortKey::Name,
        "ext" | "extension" => SortKey::Ext,
        "size" => SortKey::Size,
        "mtime" | "time" | "date" => SortKey::MTime,
        "btime" | "created" => SortKey::BTime,
        "atime" | "accessed" => SortKey::ATime,
        "none" | "unsorted" => SortKey::Unsorted,
        _ => return None,
    })
}

/// The criteria chain of the configuration, with what could not be read named back.
///
/// Three slots: a fourth tie-breaker after three that all came out equal has nothing left to
/// decide, and the name has the last word in any case.
fn sort_chain(words: &[String], problems: &mut Vec<String>) -> [Option<SortKey>; 3] {
    let mut chain = [None; 3];
    let mut at = 0;
    for word in words {
        match sort_key(word) {
            Some(key) if at < chain.len() => {
                chain[at] = Some(key);
                at += 1;
            }
            Some(_) => problems.push(format!(
                "sort chain: `{word}` is past the third and was dropped"
            )),
            None => problems.push(format!("sort chain: `{word}` is not a sort key")),
        }
    }
    chain
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

/// The six-column word for a command on the key bar, in FAR's own wording. `None` means the
/// command has no place on the bar: the slot stays empty rather than being filled with a
/// label too long to read.
fn keybar_label(cmd: Command) -> Option<&'static str> {
    use fcmd_cmd::Side;
    use fcmd_core::SortKey;
    Some(match cmd {
        // Shift
        Command::FilePack => "key-add",
        Command::FileUnpack => "key-extract",
        Command::ArcCommands => "key-arccmd",
        Command::FileEditNew => "key-edit-new",
        Command::FileCopyCurrent => "key-copy",
        Command::FileMoveCurrent => "key-rename",
        Command::FileDeleteCurrent => "key-delete",
        Command::AppSetupSave => "key-save",
        Command::PanelSortGroups => "key-group",
        Command::PanelSelectedFirst => "key-selup",
        // Ctrl
        Command::PanelToggle(Side::Left) | Command::PanelDrive(Side::Left) => "key-left",
        Command::PanelToggle(Side::Right) | Command::PanelDrive(Side::Right) => "key-right",
        Command::PanelSortBy(SortKey::Name) => "key-sort-name",
        Command::PanelSortBy(SortKey::Ext) => "key-sort-ext",
        Command::PanelSortBy(SortKey::MTime) => "key-sort-write",
        Command::PanelSortBy(SortKey::Size) => "key-sort-size",
        Command::PanelSortBy(SortKey::Unsorted) => "key-sort-unsort",
        Command::PanelSortBy(SortKey::BTime) => "key-sort-create",
        Command::PanelSortBy(SortKey::ATime) => "key-sort-access",
        Command::PanelSortMenu => "key-sort",
        // Alt
        Command::FileLink => "key-mklink",
        Command::ToolFind => "key-find",
        Command::CmdHistory => "key-history",
        Command::PanelHistory => "key-folders",
        Command::PluginCommands => "key-plugins",
        _ => return None,
    })
}

/// Which part of the help a command belongs under, and in what order the parts come. The
/// identifier's own prefix says it, which is the point of the identifiers being shaped that
/// way: `file.copy` is a file operation whether or not anyone remembered to say so.
/// The memo file beside a dBase table, if the table has one and it is there.
///
/// dBase writes `.dbt`, FoxPro writes `.fpt`, and either may be spelled in capitals beside a
/// table spelled in capitals — a DOS habit that outlived DOS. Nothing else is guessed at: a
/// file that is not a `.dbf` has no memo file, and a name that is there is used as it is.
fn memo_file(table: &std::path::Path) -> Option<std::path::PathBuf> {
    let name = table.file_name()?.to_string_lossy().into_owned();
    let (stem, ext) = name.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case("dbf") {
        return None;
    }
    let upper = ext.chars().all(|c| c.is_ascii_uppercase());
    for memo in ["dbt", "fpt"] {
        let memo = if upper {
            memo.to_uppercase()
        } else {
            memo.into()
        };
        let beside = table.with_file_name(format!("{stem}.{memo}"));
        if beside.is_file() {
            return Some(beside);
        }
        // And the other spelling, for a table whose two files disagree about it.
        let beside = table.with_file_name(format!("{stem}.{}", flip_case(&memo)));
        if beside.is_file() {
            return Some(beside);
        }
    }
    None
}

/// `dbt` ↔ `DBT`: the other spelling of an extension, for the case above.
fn flip_case(ext: &str) -> String {
    if ext.chars().all(|c| c.is_ascii_uppercase()) {
        ext.to_lowercase()
    } else {
        ext.to_uppercase()
    }
}

fn help_group(id: &str) -> (usize, &'static str) {
    let prefix = id.split('.').next().unwrap_or("");
    match prefix {
        "nav" => (0, "Moving about"),
        "sel" => (1, "Choosing files"),
        "file" => (2, "Files"),
        "arc" => (3, "Archives"),
        "panel" => (4, "The panels"),
        "viewer" => (5, "Reading"),
        "editor" => (6, "Editing"),
        "cmd" | "shell" => (7, "The command line"),
        "tool" | "net" | "plugin" | "macro" => (8, "Tools"),
        _ => (9, "The program"),
    }
}

/// What a colour group is called in a message and in the list: its own name, or its number.
fn highlight_name(h: &fcmd_config::HighlightCfg, index: usize) -> String {
    if h.name.is_empty() {
        format!("#{}", index + 1)
    } else {
        h.name.clone()
    }
}

/// One row of the colour-group list: what it is called, then what it matches, in the words
/// the configuration file uses. The row is drawn in the group's own colours, which is the
/// other half of what the list is for.
fn highlight_row(h: &fcmd_config::HighlightCfg, index: usize) -> String {
    let mut parts: Vec<String> = Vec::new();
    if !h.mask.is_empty() {
        parts.push(h.mask.clone());
    }
    if !h.kind.is_empty() {
        parts.push(h.kind.clone());
    }
    if !h.size.is_empty() {
        parts.push(h.size.clone());
    }
    if !h.age.is_empty() {
        let field = if h.date_field.is_empty() {
            "write".to_string()
        } else {
            h.date_field.clone()
        };
        parts.push(format!("{field} {}d", h.age));
    }
    if !h.attrs.is_empty() {
        parts.push(h.attrs.clone());
    }
    let name = highlight_name(h, index);
    if parts.is_empty() {
        name
    } else {
        format!("{name}  ·  {}", parts.join(" "))
    }
}

/// The first line of a paste, and how many lines of substance came after it, for the places
/// that are one line long. Blank trailing lines are not counted: a copy that took the newline
/// at the end of the line has not lost anything worth mentioning.
fn first_line(text: &str) -> (&str, usize) {
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default();
    (first, lines.filter(|l| !l.trim().is_empty()).count())
}

/// The character a key means when `Ctrl+Q` has armed the literal insert.
///
/// `Ctrl+A` is byte 1 and so on up to `Ctrl+Z`; the five that follow in the ASCII table are
/// the ones a terminal can send (`Ctrl+[` is escape, and `Ctrl+Space` is a NUL). A plain key
/// is itself, `Tab` is a real tab even where tabs are expanded, and anything with no
/// character of its own — an arrow, a function key — is nothing to insert.
fn literal_of(key: Key) -> Option<char> {
    match key.code {
        KeyCode::Char(c) if key.mods.contains(Mods::CTRL) => {
            let lower = c.to_ascii_lowercase();
            match lower {
                'a'..='z' => char::from_u32(lower as u32 - 'a' as u32 + 1),
                '[' => Some('\u{1b}'),
                '\\' => Some('\u{1c}'),
                ']' => Some('\u{1d}'),
                '^' => Some('\u{1e}'),
                '_' => Some('\u{1f}'),
                ' ' => Some('\0'),
                _ => Some(c),
            }
        }
        KeyCode::Char(c) => Some(c),
        KeyCode::Tab => Some('\t'),
        KeyCode::Esc => Some('\u{1b}'),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::{EntryFlags, EntryId, EntryKind};
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
            paths: Paths::discover(Some(std::path::Path::new("/tmp/fcmd-test/fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local("/home/mb"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
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

    /// Fill a text row of the open dialog, the way typing into it would, so a test can say
    /// what it wants in the field without walking the caret there first.
    fn set_text(app: &mut App, want: &str, text: &str) {
        let Mode::Form(form) = &mut app.mode else {
            panic!("no dialog")
        };
        for field in &mut form.fields {
            if let crate::form::Field::Text {
                key, value, caret, ..
            } = field
            {
                if *key == want {
                    *value = text.to_string();
                    *caret = value.chars().count();
                    return;
                }
            }
        }
        panic!("no field {want}");
    }

    // ── the filesystem watch (SPEC 02 §1.25) ───────────────────────────────────────────────

    #[test]
    fn a_directory_that_changed_is_re_read_and_the_setting_takes_it_away() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut app = app();
        app.panels[0].cwd = VPath::local(dir.path());
        app.requests.clear();
        app.handle_event(Event::DirChanged { panel: 0 });
        assert!(
            matches!(app.requests.first(), Some(Request::Read { panel: 0, .. })),
            "a change in the directory is a re-read of that panel: {:?}",
            app.requests
        );
        app.requests.clear();
        app.cfg.panel.auto_refresh = false;
        app.handle_event(Event::DirChanged { panel: 0 });
        assert!(app.requests.is_empty(), "the setting is the whole switch");
    }

    #[test]
    fn a_remote_panel_and_a_directory_that_went_away_are_left_as_they_are() {
        let mut app = app();
        app.panels[0].cwd = VPath::parse("sftp://mb@host:22/srv");
        app.requests.clear();
        app.handle_event(Event::DirChanged { panel: 0 });
        assert!(app.requests.is_empty(), "a server sends no local events");
        app.panels[0].cwd = VPath::local("/no/such/directory/at/all");
        app.handle_event(Event::DirChanged { panel: 0 });
        assert!(
            app.requests.is_empty(),
            "re-reading a directory that is gone would only raise an error"
        );
        // A panel index from a watch that outlived the panel it was armed for.
        app.handle_event(Event::DirChanged { panel: 9 });
        assert!(app.requests.is_empty());
    }

    // ── tabs (SPEC 02 §1.6) ─────────────────────────────────────────────────────────────────

    #[test]
    fn a_new_tab_opens_on_the_directory_under_the_cursor_and_the_other_one_waits() {
        let mut app = app();
        app.panel_mut().cursor_to(1); // docs
        press(&mut app, "Ctrl+Alt+T");
        assert_eq!(app.tabs[0].len(), 2);
        assert_eq!(app.tabs[0].current(), 1, "the new one is the one on screen");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb/docs"));
        // The one it came from is where it was, cursor and all.
        press(&mut app, "Ctrl+Alt+P");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        assert_eq!(app.panels[0].cursor, 1);
        // And the right panel knows nothing about any of it.
        assert_eq!(app.tabs[1].len(), 1);
    }

    #[test]
    fn a_new_tab_on_a_file_is_a_duplicate_of_the_one_it_came_from() {
        let mut app = app();
        app.panel_mut().cursor_to(2); // a.txt
        press(&mut app, "Ctrl+Alt+T");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        assert_eq!(app.tabs[0].len(), 2);
    }

    #[test]
    fn the_ring_of_tabs_wraps_and_every_tab_keeps_its_own_place() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].go(VPath::local("/srv"), true);
        press(&mut app, "Ctrl+Alt+N");
        assert_eq!(app.tabs[0].current(), 0, "past the end is the first one");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        press(&mut app, "Ctrl+Alt+P");
        assert_eq!(app.panels[0].cwd, VPath::local("/srv"));
    }

    #[test]
    fn the_last_tab_of_a_panel_is_neither_closed_nor_moved() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+W");
        assert_eq!(app.tabs[0].len(), 1);
        assert!(app.status_error, "it says why rather than doing nothing");
        app.status_error = false;
        press(&mut app, "Ctrl+Alt+M");
        assert_eq!(app.tabs[0].len(), 1);
        assert_eq!(app.tabs[1].len(), 1);
        assert!(app.status_error);
    }

    #[test]
    fn closing_a_tab_shows_its_neighbour() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].go(VPath::local("/srv"), true);
        press(&mut app, "Ctrl+Alt+W");
        assert_eq!(app.tabs[0].len(), 1);
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
    }

    #[test]
    fn a_tab_is_handed_to_the_other_panel_and_the_focus_stays_put() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].go(VPath::local("/srv"), true);
        press(&mut app, "Ctrl+Alt+M");
        assert_eq!(app.tabs[0].len(), 1);
        assert_eq!(app.tabs[1].len(), 2);
        assert_eq!(app.panels[1].cwd, VPath::local("/srv"));
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        assert_eq!(app.active, 0, "one is still working in the left panel");
    }

    #[test]
    fn a_locked_tab_stays_on_its_directory_and_says_so() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+K");
        assert!(app.panels[0].locked);
        app.panel_mut().cursor_to(1); // docs
        press(&mut app, "Enter");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        assert!(
            app.status_error,
            "a lock that said nothing would read as a bug"
        );
        // Off again, and the same key goes in.
        press(&mut app, "Ctrl+Alt+K");
        press(&mut app, "Enter");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb/docs"));
    }

    #[test]
    fn a_locked_panel_cannot_be_driven_from_the_other_side_either() {
        let mut app = app();
        press(&mut app, "Ctrl+Alt+K");
        press(&mut app, "Tab"); // the right panel has the focus
        press(&mut app, "Ctrl+="); // the other panel follows this one
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        assert!(app.status_error);
    }

    #[test]
    fn a_tab_can_be_given_a_name_and_an_empty_one_takes_it_off() {
        let mut app = app();
        open(&mut app, 0, "menu-tab-name");
        type_text(&mut app, "work");
        press(&mut app, "Enter");
        assert_eq!(app.panels[0].name.as_deref(), Some("work"));
        assert_eq!(app.tabs[0].labels(&app.panels[0]), ["work"]);
        open(&mut app, 0, "menu-tab-name");
        for _ in 0..4 {
            press(&mut app, "Backspace");
        }
        press(&mut app, "Enter");
        assert_eq!(app.panels[0].name, None);
        assert_eq!(app.tabs[0].labels(&app.panels[0]), ["mb"]);
    }

    #[test]
    fn the_tabs_of_the_last_session_come_back_and_the_one_on_screen_is_where_the_shell_was() {
        let mut app = app();
        let saved = vec![
            fcmd_config::TabState {
                path: "/etc".into(),
                name: "config".into(),
                locked: false,
            },
            fcmd_config::TabState {
                path: "/var".into(),
                name: String::new(),
                locked: false,
            },
            // A tab on a server: there is no connection to it, so it is not restored.
            fcmd_config::TabState {
                path: "sftp://mb@host/srv".into(),
                name: String::new(),
                locked: false,
            },
        ];
        app.restore_tabs(0, &saved, 1);
        assert_eq!(app.tabs[0].len(), 2);
        assert_eq!(app.tabs[0].current(), 1);
        assert_eq!(
            app.panels[0].cwd,
            VPath::local("/home/mb"),
            "the directory this session started in wins for the tab on screen"
        );
        assert_eq!(app.tabs[0].labels(&app.panels[0]), ["config", "mb"]);
    }

    #[test]
    fn a_locked_tab_keeps_its_own_directory_when_the_session_comes_back() {
        let mut app = app();
        let saved = vec![
            fcmd_config::TabState {
                path: "/etc".into(),
                name: String::new(),
                locked: false,
            },
            fcmd_config::TabState {
                path: "/var/log".into(),
                name: "logs".into(),
                locked: true,
            },
        ];
        app.restore_tabs(0, &saved, 1);
        assert_eq!(app.panels[0].cwd, VPath::local("/var/log"));
        assert!(app.panels[0].locked);
    }

    #[test]
    fn one_saved_tab_leaves_the_panel_as_it_is() {
        let mut app = app();
        let saved = vec![fcmd_config::TabState {
            path: "/etc".into(),
            name: String::new(),
            locked: false,
        }];
        app.restore_tabs(0, &saved, 0);
        assert_eq!(app.tabs[0].len(), 1);
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
    }

    // ── the panels' height, and the font ────────────────────────────────────────────────────

    /// `Ctrl+Up` brings the panels' bottom edge *up*, which is less panel and more of the
    /// output underneath. It read the other way round until somebody pressed it.
    #[test]
    fn ctrl_up_makes_the_panels_shorter_and_ctrl_down_gives_the_rows_back() {
        let mut app = app();
        assert_eq!(app.cfg.panel.height_cut, 0);
        press(&mut app, "Ctrl+Up");
        press(&mut app, "Ctrl+Up");
        assert_eq!(app.cfg.panel.height_cut, 2, "two rows off the panels");
        press(&mut app, "Ctrl+Down");
        assert_eq!(app.cfg.panel.height_cut, 1);
        press(&mut app, "Ctrl+Down");
        press(&mut app, "Ctrl+Down");
        assert_eq!(app.cfg.panel.height_cut, 0, "and no further");
    }

    #[test]
    fn the_font_keys_ask_the_terminal_and_say_that_is_what_they_did() {
        let mut app = app();
        for (key, want) in [("Ctrl++", 1i8), ("Ctrl+-", -1), ("Ctrl+0", 0)] {
            press(&mut app, key);
            let by = app
                .take_requests()
                .into_iter()
                .find_map(|r| match r {
                    Request::Font { by } => Some(by),
                    _ => None,
                })
                .unwrap_or_else(|| panic!("{key} asked for nothing"));
            assert_eq!(by, want, "{key}");
            // A program cannot know whether the terminal did it, and the line says so.
            assert!(app.status.contains("terminal"), "{}", app.status);
        }
    }

    // ── checksums (SPEC 03 §9) ──────────────────────────────────────────────────────────────

    #[test]
    fn the_checksum_dialog_opens_on_the_hash_and_the_name_a_commander_would_use() {
        let mut app = app();
        app.panel_mut().cursor_to(2);
        app.run(Command::FileChecksum);
        let Mode::Form(form) = &app.mode else {
            panic!("the dialog opens")
        };
        assert_eq!(form.kind, FormKind::Checksum);
        assert_eq!(form.text_value("sum.algorithm"), "sha256");
        assert_eq!(form.text_value("sum.name"), "mb.sha256");
        assert_eq!(form.text_value("sum.one_file"), "true");
    }

    #[test]
    fn a_checksum_is_refused_where_the_bytes_are_not_on_this_machine() {
        let mut app = app();
        app.panels[0].cwd = VPath::remote(fcmd_core::Scheme::Sftp, "mb@host", "/srv");
        app.panel_mut().cursor_to(2);
        app.run(Command::FileChecksum);
        assert!(
            matches!(&app.mode, Mode::Normal),
            "no dialog for something it cannot read"
        );
        assert!(app.status_error);
    }

    #[test]
    fn the_checksum_dialog_asks_the_engine_for_what_it_was_answered_with() {
        let mut app = app();
        app.panel_mut().cursor_to(2);
        app.run(Command::FileChecksum);
        // MD5 instead, and one list beside each file: the rows are answered the way the keys
        // answer them, found by key rather than by where they happen to be.
        let on_row = |app: &mut App, key: &str| {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(|f| match f {
                    crate::form::Field::Choice { key: k, .. } => *k == key,
                    crate::form::Field::Toggle { key: k, .. } => *k == key,
                    _ => false,
                })
                .unwrap_or_else(|| panic!("no row `{key}`"));
        };
        on_row(&mut app, "sum.algorithm");
        press(&mut app, "Right"); // sha256 → sha1
        press(&mut app, "Right"); // sha1 → md5
        on_row(&mut app, "sum.one_file");
        press(&mut app, "Space");
        press(&mut app, "F10");
        let request = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::Checksums {
                    paths,
                    algorithm,
                    target,
                    ..
                } => Some((paths, algorithm, target)),
                _ => None,
            })
            .expect("a checksum request");
        assert_eq!(request.0, vec![VPath::local("/home/mb/a.txt")]);
        assert_eq!(request.1, fcmd_jobs::Algorithm::Md5);
        assert_eq!(request.2, None, "one list beside each file");
        // And the next dialog opens on the hash that was chosen.
        app.run(Command::FileChecksum);
        let Mode::Form(form) = &app.mode else {
            panic!("the dialog opens")
        };
        assert_eq!(form.text_value("sum.algorithm"), "md5");
        assert_eq!(form.text_value("sum.name"), "mb.md5");
    }

    #[test]
    fn a_list_that_is_not_a_list_of_digests_says_so() {
        let mut app = app();
        app.panel_mut().cursor_to(2); // a.txt, which is not a list
        app.run(Command::FileChecksumVerify);
        let id = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::Verify { id, .. } => Some(id),
                _ => None,
            })
            .expect("a verify request");
        app.handle_event(Event::VerifyDone {
            id,
            verdicts: Vec::new(),
            cancelled: false,
            problem: Some("a.txt: not a list of digests this can check".into()),
        });
        assert!(app.status_error);
        assert!(app.status.contains("a.txt"), "{}", app.status);
    }

    #[test]
    fn a_check_that_all_matches_says_so_and_selects_nothing() {
        use fcmd_jobs::sum::{State, Verdict};
        let mut app = app();
        app.panel_mut().cursor_to(2);
        app.run(Command::FileChecksumVerify);
        let id = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::Verify { id, .. } => Some(id),
                _ => None,
            })
            .expect("a verify request");
        app.handle_event(Event::VerifyDone {
            id,
            verdicts: vec![
                Verdict {
                    name: "a.txt".into(),
                    state: State::Matches,
                },
                Verdict {
                    name: "b.rs".into(),
                    state: State::Matches,
                },
            ],
            cancelled: false,
            problem: None,
        });
        assert!(matches!(&app.mode, Mode::Normal), "no report to read");
        assert!(app.panel().selection.is_empty());
        assert!(app.status.contains('2'), "{}", app.status);
    }

    // ── the wastebasket (SPEC 03 §4) ────────────────────────────────────────────────────────

    /// What `F8` asked the engine for: the kind of the job that was queued.
    fn queued_kind(app: &mut App) -> Option<JobKind> {
        app.take_requests().into_iter().find_map(|r| match r {
            Request::Job { spec, .. } => Some(spec.kind),
            _ => None,
        })
    }

    #[test]
    fn f8_goes_to_the_wastebasket_and_says_so() {
        let mut app = app();
        app.panel_mut().cursor_to(2); // a.txt
        press(&mut app, "F8");
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("the question is asked first")
        };
        assert!(
            confirm.title.to_lowercase().contains("wastebasket"),
            "the title says which of the two this is: {}",
            confirm.title
        );
        press(&mut app, "Enter");
        assert_eq!(queued_kind(&mut app), Some(JobKind::Trash));
    }

    #[test]
    fn shift_del_never_uses_the_wastebasket() {
        let mut app = app();
        app.panel_mut().cursor_to(2);
        press(&mut app, "Shift+Del");
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("the question is asked first")
        };
        assert!(
            !confirm.title.to_lowercase().contains("wastebasket"),
            "{}",
            confirm.title
        );
        assert!(
            confirm
                .lines
                .iter()
                .any(|l| l.to_lowercase().contains("permanent")),
            "and it says there is nothing to catch them: {:?}",
            confirm.lines
        );
        press(&mut app, "Enter");
        assert_eq!(queued_kind(&mut app), Some(JobKind::Delete));
    }

    #[test]
    fn with_the_setting_off_f8_deletes_outright() {
        let mut app = app();
        app.cfg.general.use_trash = false;
        app.panel_mut().cursor_to(2);
        press(&mut app, "F8");
        press(&mut app, "Enter");
        assert_eq!(queued_kind(&mut app), Some(JobKind::Delete));
    }

    #[test]
    fn there_is_no_wastebasket_on_a_server_so_f8_deletes_there() {
        let mut app = app();
        // The panel is on a server; the wastebasket is this machine's and nothing else's.
        app.panels[0].cwd = VPath::remote(fcmd_core::Scheme::Sftp, "mb@host", "/srv");
        app.panel_mut().cursor_to(2);
        press(&mut app, "F8");
        press(&mut app, "Enter");
        assert_eq!(queued_kind(&mut app), Some(JobKind::Delete));
    }

    // ── the folders kept to hand (SPEC 02 §11) ──────────────────────────────────────────────

    #[test]
    fn a_digit_holds_a_folder_and_takes_you_back_to_it() {
        let mut app = app();
        press(&mut app, "Ctrl+Shift+3");
        assert_eq!(app.bookmarks.len(), 1);
        assert_eq!(app.bookmarks[0].digit, Some(3));
        assert_eq!(app.bookmarks[0].path, "/home/mb");
        app.panel_mut().cursor_to(1); // docs
        press(&mut app, "Enter");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb/docs"));
        press(&mut app, "Alt+3");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
    }

    #[test]
    fn a_digit_that_holds_nothing_says_so() {
        let mut app = app();
        press(&mut app, "Alt+7");
        assert!(app.status_error);
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
    }

    #[test]
    fn setting_a_digit_over_a_folder_replaces_that_folder_and_its_name() {
        let mut app = app();
        press(&mut app, "Ctrl+Shift+1");
        app.bookmarks[0].name = "home".into();
        app.panels[0].go(VPath::local("/srv"), true);
        press(&mut app, "Ctrl+Shift+1");
        assert_eq!(app.bookmarks.len(), 1, "a digit holds one folder");
        assert_eq!(app.bookmarks[0].path, "/srv");
        assert_eq!(
            app.bookmarks[0].name, "",
            "the name belonged to the old one"
        );
    }

    #[test]
    fn a_folder_already_kept_is_given_the_digit_rather_than_kept_twice() {
        let mut app = app();
        press(&mut app, "Ctrl+D");
        press(&mut app, "Ins"); // keep /home/mb, with no digit
        press(&mut app, "Esc");
        press(&mut app, "Ctrl+Shift+5");
        assert_eq!(app.bookmarks.len(), 1);
        assert_eq!(app.bookmarks[0].digit, Some(5));
    }

    #[test]
    fn the_hotlist_keeps_names_gives_folders_up_and_goes_where_a_row_says() {
        let mut app = app();
        press(&mut app, "Ctrl+Shift+2"); // /home/mb on 2
        app.panels[0].go(VPath::local("/srv"), true);
        press(&mut app, "Ctrl+D");
        press(&mut app, "Ins"); // and /srv in the list with no digit
        assert_eq!(app.bookmarks.len(), 2);
        // Named from inside the list, which is where one reads it.
        press(&mut app, "F4");
        type_text(&mut app, "server");
        press(&mut app, "Enter");
        assert_eq!(
            app.bookmarks
                .iter()
                .find(|b| b.path == "/srv")
                .unwrap()
                .name,
            "server"
        );
        // The digit comes first in the row, and its own key takes it.
        let Mode::Pick(pick) = &app.mode else {
            panic!("the list is open again after naming a row")
        };
        assert_eq!(pick.rows.len(), 2);
        assert!(pick.rows[0].label.contains("/home/mb"), "{:?}", pick.rows);
        assert!(pick.rows[1].label.contains("server"), "{:?}", pick.rows);
        assert_eq!(pick.rows[0].hotkey, Some('2'));
        assert_eq!(pick.rows[1].hotkey, None);
        assert_eq!(
            pick.cursor, 1,
            "the cursor stayed on the row that was named"
        );
        // The digit is the row's own key: one press takes it, without the arrows.
        press(&mut app, "2");
        assert_eq!(app.panels[0].cwd, VPath::local("/home/mb"));
        // And given up from the same list.
        press(&mut app, "Ctrl+D");
        press(&mut app, "Del");
        assert_eq!(app.bookmarks.len(), 1);
        assert_eq!(app.bookmarks[0].path, "/srv");
    }

    #[test]
    fn keeping_a_folder_that_is_already_kept_says_so_rather_than_keeping_it_twice() {
        let mut app = app();
        press(&mut app, "Ctrl+D");
        press(&mut app, "Ins");
        press(&mut app, "Ins");
        assert_eq!(app.bookmarks.len(), 1);
        assert!(app.status_error);
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
        press(&mut app, "Ctrl+Shift+N");
        press(&mut app, "Ins");
        let Mode::Form(form) = &app.mode else {
            panic!("the session editor did not open")
        };
        assert_eq!(form.kind, FormKind::Site);
        assert_eq!(form.site, None, "Ins makes a new one, it does not edit");

        type_text(&mut app, "jump");
        press(&mut app, "Down"); // the protocol, which stays as it is
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
        press(&mut app, "Down"); // the protocol
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
        app.cfg.sites.push(fcmd_config::SiteCfg {
            id: "box".into(),
            host: "old.example.net".into(),
            ..Default::default()
        });
        let (sites, problems) = Site::list_from_config(&app.cfg);
        assert!(problems.is_empty(), "{problems:?}");
        app.sites = sites;

        press(&mut app, "Ctrl+Shift+N");
        press(&mut app, "F4");
        let Mode::Form(form) = &app.mode else {
            panic!("the session editor did not open")
        };
        assert_eq!(form.site, Some(0));
        assert_eq!(form.text_value("site.host"), "old.example.net");

        // Clear the host and type another one: Backspace works where the caret is.
        press(&mut app, "Down"); // the protocol
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
        press(&mut app, "Down"); // the protocol
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
                assert_eq!(spec.kind, fcmd_jobs::JobKind::Wipe);
                assert_eq!(spec.sources.len(), 1);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn alt_f6_offers_the_three_kinds_of_link_and_remembers_the_last_one() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        app.run(Command::FileLink);
        let Mode::Form(form) = &app.mode else {
            panic!("the link dialog did not open: {:?}", app.mode)
        };
        assert_eq!(form.kind, FormKind::Link);
        // Pre-filled with the passive panel's directory, as a copy would be — joined with
        // whatever separator this platform uses.
        let expected = std::path::PathBuf::from("/tmp").join("a.txt");
        assert_eq!(form.text_value("link.name"), expected.to_string_lossy());
        // A relative symbolic link is the default: it is the one that survives the whole tree
        // being moved somewhere else.
        assert_eq!(form.text_value("link.kind"), "symlink-relative");

        press(&mut app, "F10");
        match app.take_requests().into_iter().next() {
            Some(Request::Link { target, link, kind }) => {
                assert!(target.to_string().ends_with("a.txt"), "{target}");
                assert_eq!(link.as_path(), expected);
                assert_eq!(kind, fcmd_jobs::LinkKind::SymlinkRelative);
            }
            other => panic!("{other:?}"),
        }

        // Choose a hard link, and the next dialog opens on it: the choice is made once per
        // run of them, which is how the pack and checksum dialogs behave too.
        app.run(Command::FileLink);
        press(&mut app, "Down"); // off the name, onto the kind
        press(&mut app, "Right"); // relative -> absolute
        press(&mut app, "Right"); // absolute -> hard
        press(&mut app, "F10");
        match app.take_requests().into_iter().next() {
            Some(Request::Link { kind, .. }) => {
                assert_eq!(kind, fcmd_jobs::LinkKind::Hard)
            }
            other => panic!("{other:?}"),
        }
        app.run(Command::FileLink);
        let Mode::Form(form) = &app.mode else {
            panic!("the link dialog did not open again")
        };
        assert_eq!(form.text_value("link.kind"), "hard");
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
            .select_all(&listing, fcmd_core::MaskScope::FilesOnly);
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
        assert_eq!(spec.kind, fcmd_jobs::JobKind::Replace);
        assert!(spec.options.dry_run, "the first run must not write");
        assert_eq!(spec.sources.len(), 2);

        // What the count found comes back, and the program asks before writing.
        let counted = fcmd_jobs::Outcome {
            files: 1,
            replacements: vec![(fcmd_core::VPath::local(dir.path().join("one.txt")), 2)],
            ..Default::default()
        };
        app.handle_event(Event::JobDone {
            id,
            kind: fcmd_jobs::JobKind::Replace,
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
            kind: fcmd_jobs::JobKind::Replace,
            outcome: fcmd_jobs::Outcome::default(),
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
    fn a_size_condition_reaches_the_search_and_a_bad_one_keeps_the_dialog_open() {
        let mut app = app();
        app.run(Command::ToolFind);
        set_text(&mut app, "find.size", "100M");
        press(&mut app, "F10");

        // The frame says what was asked for, so the results are read in context.
        match &app.mode {
            Mode::Find(view) => assert!(view.label.contains("100M"), "{}", view.label),
            other => panic!("{other:?}"),
        }
        match app.take_requests().into_iter().next() {
            Some(Request::Find { spec, .. }) => assert_eq!(spec.size, "100M"),
            other => panic!("{other:?}"),
        }

        // And it comes back next time, like the rest of the dialog.
        app.mode = Mode::Normal;
        app.run(Command::ToolFind);
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.text_value("find.size"), "100M");

        // A size that is not a size is refused on the spot, with the dialog still open and
        // still holding what was typed.
        set_text(&mut app, "find.size", "lots");
        press(&mut app, "F10");
        assert!(app.status_error, "{}", app.status);
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.text_value("find.size"), "lots");
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
                entry: Box::new(entry(name, fcmd_core::EntryKind::File, 10)),
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
            entry: Box::new(entry("one.rs", fcmd_core::EntryKind::File, 10)),
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
            entry: Box::new(entry("one.rs", fcmd_core::EntryKind::File, 10)),
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
                entry("two.txt", fcmd_core::EntryKind::File, 2),
            ),
            (
                VPath::local(&one),
                entry("one.txt", fcmd_core::EntryKind::File, 1),
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
        app.run(Command::PanelSortBy(fcmd_core::SortKey::Size));
        app.handle_input(Input::Key("Home".parse().unwrap()));
        assert_eq!(app.panel().current().unwrap().name_str(), "one.txt");
        assert_eq!(app.panel().current_path().unwrap().as_path(), one);
        app.handle_input(Input::Key("Down".parse().unwrap()));
        assert_eq!(app.panel().current_path().unwrap().as_path(), two);

        // An operation works on the real file, not on the panel's own directory joined to a name.
        app.run(Command::FileCopyCurrent);
        assert!(matches!(&app.mode, Mode::Form(f) if f.kind == FormKind::Copy));
        assert_eq!(app.copy_sources, vec![VPath::local(&two)]);
        app.mode = Mode::Normal;

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
                    entry("gone.txt", fcmd_core::EntryKind::File, 1),
                ),
                (
                    VPath::local(&stays),
                    entry("stays.txt", fcmd_core::EntryKind::File, 1),
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
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.kind, FormKind::Copy);
        assert_eq!(app.copy_sources.len(), 1);
        // Selecting everything while the dialog is open must not widen the copy.
        let listing = app.panel().listing.clone();
        app.panel_mut()
            .selection
            .select_all(&listing, fcmd_core::MaskScope::FilesAndDirs);
        press(&mut app, "Enter");
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
        typed(&mut app, "cargo test -p fcmd");
        press(&mut app, "Enter");
        app.requests.clear();
        press(&mut app, "Alt+F8");
        let Mode::Pick(pick) = &app.mode else {
            panic!("no history list")
        };
        assert_eq!(pick.rows[0].value, "cargo test -p fcmd");
        press(&mut app, "Enter");
        assert_eq!(app.cmd.text, "cargo test -p fcmd");
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
        let dir = std::env::temp_dir().join(format!("fcmd-cmd-{}", std::process::id()));
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
        // `Ctrl+O` takes the panels off the screen rather than opening a screen of its own:
        // the mode is still the normal one, so the command line and the keys keep working.
        press(&mut app, "Ctrl+O");
        assert!(app.panels_off);
        assert!(app.mode.is_normal());
        assert!(app.user_screen.iter().any(|l| l == "$ false"));
        assert!(app.user_screen.iter().any(|l| l == "nothing to say"));
        // A command typed with the panels off runs exactly as it does with them on.
        typed(&mut app, "echo again");
        press(&mut app, "Enter");
        assert!(shell_requests(&app).iter().any(|c| c == "echo again"));
        // And the same key brings them back.
        press(&mut app, "Ctrl+O");
        assert!(!app.panels_off);
        assert!(app.mode.is_normal());
    }

    /// The id of the shell request the last `Enter` queued.
    fn shell_id(app: &App) -> u64 {
        match app.requests.iter().find_map(|r| match r {
            Request::Shell { id, .. } => Some(*id),
            _ => None,
        }) {
            Some(id) => id,
            None => panic!("no shell request"),
        }
    }

    #[test]
    fn what_a_command_prints_arrives_while_it_runs_and_the_panels_come_back_after_it() {
        let mut app = app();
        typed(&mut app, "make");
        press(&mut app, "Enter");
        let id = shell_id(&app);
        // The heading is up before a single line has been printed: the output has to have
        // somewhere to land, and the panels are off so it can be watched arriving.
        assert_eq!(app.user_screen.last().map(String::as_str), Some("$ make"));
        assert!(app.panels_off, "the screen shows the command while it runs");
        assert!(app.mode.is_normal(), "everything else stays live");

        app.handle_event(Event::ShellOutput {
            id,
            text: "cc -c main.c\ncc -c util.c\n".into(),
        });
        assert_eq!(
            app.user_screen.last().map(String::as_str),
            Some("cc -c util.c")
        );
        app.handle_event(Event::ShellOutput {
            id,
            text: "cc -o prog\n".into(),
        });
        assert_eq!(
            app.user_screen.last().map(String::as_str),
            Some("cc -o prog")
        );
        // The window counts back from the end, so nothing had to be scrolled to see them.
        assert_eq!(app.user_scroll, 0);

        app.handle_event(Event::ShellDone {
            id,
            line: "make".into(),
            output: String::new(),
            status: Some(0),
        });
        // The panels come back by themselves, and what was printed is still there under
        // `Ctrl+O` — and only once, which is the point of the output not being sent twice.
        assert!(!app.panels_off);
        assert_eq!(
            app.user_screen
                .iter()
                .filter(|l| *l == "cc -o prog")
                .count(),
            1
        );
    }

    #[test]
    fn the_screen_the_command_took_is_given_back_the_moment_the_key_asks_for_it() {
        let mut app = app();
        typed(&mut app, "make");
        press(&mut app, "Enter");
        let id = shell_id(&app);
        assert!(app.panels_off);
        // `Ctrl+O` while it runs: the panels come back and the command carries on, its
        // output still landing on the screen behind them.
        press(&mut app, "Ctrl+O");
        assert!(!app.panels_off);
        app.handle_event(Event::ShellOutput {
            id,
            text: "still going\n".into(),
        });
        assert!(app.user_screen.iter().any(|l| l == "still going"));
        // And the end of the command does not take the screen away a second time.
        app.handle_event(Event::ShellDone {
            id,
            line: "make".into(),
            output: String::new(),
            status: Some(0),
        });
        assert!(!app.panels_off);
    }

    #[test]
    fn the_panels_stay_when_the_setting_says_so_and_the_output_still_arrives_as_it_is_printed() {
        let mut app = app();
        app.cfg.ui.command_screen = false;
        typed(&mut app, "make");
        press(&mut app, "Enter");
        let id = shell_id(&app);
        assert!(!app.panels_off, "the panels are left alone");
        app.handle_event(Event::ShellOutput {
            id,
            text: "cc -c main.c\n".into(),
        });
        // The line is on the screen `Ctrl+O` opens as soon as it is printed, which is the
        // half of this that is not about which screen is in front.
        assert!(app.user_screen.iter().any(|l| l == "cc -c main.c"));
    }

    #[test]
    fn a_line_rewritten_in_place_is_kept_as_what_it_ended_up_saying() {
        let mut app = app();
        typed(&mut app, "copy");
        press(&mut app, "Enter");
        let id = shell_id(&app);
        // A progress bar writes its line over and over with a carriage return between the
        // tries. The screen keeps lines, so what is kept is the last of them.
        app.handle_event(Event::ShellOutput {
            id,
            text: "10%\r50%\r100%\n".into(),
        });
        assert_eq!(app.user_screen.last().map(String::as_str), Some("100%"));
    }

    #[test]
    fn with_the_panels_off_the_pages_turn_through_what_was_printed() {
        let mut app = app();
        app.user_screen = (0..200).map(|n| format!("line {n}")).collect();
        app.rows = 20;
        press(&mut app, "Ctrl+O");
        assert_eq!(
            app.user_scroll, 0,
            "it opens at the end, where the last command is"
        );
        press(&mut app, "PgUp");
        assert_eq!(app.user_scroll, 20);
        press(&mut app, "PgUp");
        assert_eq!(app.user_scroll, 40);
        press(&mut app, "PgDn");
        assert_eq!(app.user_scroll, 20);
        // Never past the first line, and never past the last.
        for _ in 0..40 {
            press(&mut app, "PgUp");
        }
        assert_eq!(app.user_scroll, 199);
        for _ in 0..40 {
            press(&mut app, "PgDn");
        }
        assert_eq!(app.user_scroll, 0);
        // With the panels on, the same keys page through the listing again.
        press(&mut app, "Ctrl+O");
        press(&mut app, "PgUp");
        assert_eq!(app.user_scroll, 0);
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
        // The code pages list is still to come — the viewer cycles them with `F8`, but the
        // list-and-pick version is not built. Options is the fourth menu.
        open(&mut app, 3, "menu-codepages");
        assert!(
            app.status.contains("settings.codepages"),
            "a missing feature must say so instead of doing nothing quietly: `{}`",
            app.status
        );
        assert!(app.mode.is_normal());
    }

    /// `Ctrl+A`: what a file is, and what the dialog then does about it.
    #[test]
    fn the_attributes_dialog_shows_what_a_file_is_and_sets_what_it_is_told() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("one.txt");
        std::fs::write(&file, b"x").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        }

        let mut app = app();
        app.panels[app.active].cwd = VPath::local(dir.path());
        let entries =
            fcmd_vfs::local::list_blocking(&app.panels[app.active].cwd, &CancelToken::new())
                .unwrap();
        let (panel, generation) = (app.active, app.panels[app.active].generation);
        app.handle_event(Event::Listed {
            panel,
            generation,
            result: Ok(entries),
        });
        app.requests.clear();
        let row = app.panels[app.active]
            .listing
            .row_of_name("one.txt")
            .expect("the file is listed");
        app.panels[app.active].cursor_to(row);

        press(&mut app, "Ctrl+A");
        let Mode::Form(form) = &app.mode else {
            panic!("Ctrl+A should open the attributes dialog: {:?}", app.mode)
        };
        assert_eq!(form.kind, FormKind::Attributes);
        // The name is in it, and on Unix so is what the file is now.
        let text: String = form
            .fields
            .iter()
            .map(|f| format!("{f:?}"))
            .collect::<Vec<_>>()
            .join(" ");
        assert!(text.contains("one.txt"), "{text}");
        #[cfg(unix)]
        assert!(text.contains("rw-r--r--"), "{text}");

        // Unix: the twelve boxes are there and start from the mode the file has.
        #[cfg(unix)]
        {
            assert!(form.toggle_value("attr.ur"), "the owner may read");
            assert!(form.toggle_value("attr.uw"), "and write");
            assert!(!form.toggle_value("attr.ux"), "but not run it");
            assert!(!form.toggle_value("attr.ow"), "and the world may not write");
        }

        // Take the group's read bit off and answer the dialog: the file changes, and only in
        // the way the box said.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let Mode::Form(form) = &mut app.mode else {
                unreachable!("the dialog is open")
            };
            let at = form
                .fields
                .iter()
                .position(
                    |f| matches!(f, crate::form::Field::Toggle { key, .. } if *key == "attr.gr"),
                )
                .expect("the row is there");
            form.cursor = at;
            press(&mut app, "Space");
            press(&mut app, "Enter");
            let (paths, change, recursive) = match app.take_requests().into_iter().next() {
                Some(Request::SetAttrs {
                    paths,
                    change,
                    recursive,
                    ..
                }) => (paths, change, recursive),
                other => panic!("no attribute change was started: {other:?}"),
            };
            assert_eq!(change.mode, Some(0o604), "only that bit moved");
            assert_eq!(paths, vec![VPath::local(&file)]);
            assert!(!recursive, "nothing here is a directory");

            // And the worker's half of it, which is the engine doing what the box said.
            let outcome = fcmd_jobs::set_attributes(
                &fcmd_jobs::LocalFs,
                &paths,
                &change,
                recursive,
                &CancelToken::new(),
                &mut |_, _| {},
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            assert_eq!(
                std::fs::metadata(&file).unwrap().permissions().mode() & 0o777,
                0o604
            );
        }
    }

    /// The key bar follows the modifier, and what it shows is what the keymap really does.
    #[test]
    fn the_key_bar_changes_with_the_modifier_and_never_lies() {
        let mut app = app();
        let plain = app.keybar_for(Mods::NONE);
        assert_eq!(plain, app.keybar());

        let shift = app.keybar_for(Mods::SHIFT);
        let labels: Vec<&str> = shift.iter().map(|(_, l)| l.as_str()).collect();
        assert!(labels.contains(&"Add"), "{labels:?}");
        assert!(labels.contains(&"Extrct"), "{labels:?}");
        assert!(labels.contains(&"ArcCmd"), "{labels:?}");
        assert!(labels.contains(&"SelUp"), "{labels:?}");

        let ctrl = app.keybar_for(Mods::CTRL);
        assert_eq!(
            ctrl.iter().find(|(n, _)| *n == 1).map(|(_, l)| l.as_str()),
            Some("Left")
        );
        let alt = app.keybar_for(Mods::ALT);
        assert_eq!(
            alt.iter().find(|(n, _)| *n == 7).map(|(_, l)| l.as_str()),
            Some("Find")
        );

        // Every label on a modifier bar belongs to a key that really runs something: the bar
        // is built from the keymap, so it cannot promise what the program will not do.
        for mods in [Mods::SHIFT, Mods::CTRL, Mods::ALT] {
            for (n, label) in app.keybar_for(mods) {
                let key = Key {
                    code: KeyCode::F(n),
                    mods,
                };
                assert!(
                    app.keymap.resolve(&[Scope::Panel], key).is_some(),
                    "F{n} is on the bar as `{label}` and bound to nothing"
                );
            }
        }

        // And the bar goes back to plain as soon as the key is pressed.
        app.handle_input(Input::Mods(Mods::SHIFT));
        assert_eq!(app.held, Mods::SHIFT);
        press(&mut app, "Shift+F11");
        assert_eq!(app.held, Mods::NONE);
        // `Shift+F11` is FAR's `Group`: the sort groups, not the folders-first flag, which
        // is a setting in the panel dialog here as it is there.
        assert!(
            app.panel().sort.groups,
            "Shift+F11 did not turn the sort groups on"
        );
    }

    /// The archive dialog can be told to clear up after itself — but only if the archive was
    /// really written. A pack that failed must leave the originals where they are.
    #[test]
    fn the_originals_go_only_after_an_archive_that_was_written() {
        let run = |clean: bool| {
            let mut app = app();
            app.panels[0].cursor = 2; // a.txt
            app.run(Command::FilePack);
            let Mode::Form(form) = &app.mode else {
                panic!("no dialog: {:?}", app.mode)
            };
            // Down to the `delete after archiving` row and tick it, as a user would.
            let row = form
                .fields
                .iter()
                .position(|f| matches!(f, crate::form::Field::Toggle { key, .. } if *key == "pack.delete"))
                .expect("the dialog has the row");
            for _ in 0..row {
                press(&mut app, "Down");
            }
            press(&mut app, "Space");
            assert!(
                matches!(&app.mode, Mode::Form(f) if f.toggle_value("pack.delete")),
                "the row did not tick"
            );
            app.requests.clear();
            press(&mut app, "F10"); // the shortcut for OK
            let id = match app.requests.iter().find_map(|r| match r {
                Request::Pack { id, .. } => Some(*id),
                _ => None,
            }) {
                Some(id) => id,
                None => panic!("no pack was started: {:?}", app.requests),
            };
            app.requests.clear();
            let outcome = Outcome {
                files: 1,
                bytes: 100,
                errors: if clean {
                    Vec::new()
                } else {
                    vec![(VPath::local("/home/mb/a.txt"), "no room".into())]
                },
                ..Outcome::default()
            };
            app.handle_event(Event::JobDone {
                id,
                kind: JobKind::Pack,
                outcome,
            });
            app.requests
                .iter()
                .filter_map(|r| match r {
                    Request::Job { spec, .. } if spec.kind == JobKind::Delete => {
                        Some(spec.sources.clone())
                    }
                    _ => None,
                })
                .next()
        };

        let deleted = run(true).expect("the originals go after a clean pack");
        assert_eq!(deleted.len(), 1);
        assert!(deleted[0].to_string().ends_with("a.txt"), "{deleted:?}");
        assert!(
            run(false).is_none(),
            "a pack that failed must not delete the files it was given"
        );
    }

    /// `Alt+F1` is a table, not a list of paths: every row says what the thing is, and the
    /// rows that are not filesystems carry a number to reach them by.
    #[test]
    fn the_drive_dialog_is_a_table_and_its_panels_are_numbered() {
        let mut app = app();
        app.connected.push("sftp://mb@host:22".into());
        app.run(Command::PanelDrive(fcmd_cmd::Side::Left));
        let Mode::Pick(pick) = &app.mode else {
            panic!("no dialog: {:?}", app.mode)
        };
        assert_eq!(pick.kind, PickKind::Drives);

        // The first row is a place on this machine — `/` on Unix, a drive letter on Windows
        // — and it says what it is rather than only where it is.
        let first = pick.rows.first().expect("at least one drive");
        assert!(
            first.label.split_whitespace().count() > 1,
            "the row has no columns: {}",
            first.label
        );
        assert!(
            pick.rows.iter().any(|r| r.value.ends_with(['/', '\\'])),
            "no filesystem among the drives: {:?}",
            pick.rows
        );

        // The connection is at the bottom, numbered, and the number reaches it.
        let last = pick.rows.last().expect("a row");
        assert_eq!(last.value, "sftp://mb@host:22/");
        assert_eq!(last.hotkey, Some('0'));
        assert_eq!(pick.by_hotkey('0'), Some(pick.rows.len() - 1));
        // And the number column is drawn for everyone once anybody has one.
        let lines = pick.lines();
        assert!(lines.last().unwrap().contains('0'), "{:?}", lines.last());

        press(&mut app, "0");
        assert!(app.mode.is_normal(), "the number did not take the row");
        assert!(
            app.panels[0]
                .cwd
                .to_string()
                .starts_with("sftp://mb@host:22"),
            "{}",
            app.panels[0].cwd
        );
    }

    /// `Shift+F3` on an archive: the menu names it, and what is on the menu is what we do.
    #[test]
    fn the_archive_menu_knows_which_archive_it_is_about() {
        let mut app = app();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].listing.generation,
            result: Ok(vec![
                entry("tree.zip", EntryKind::File, 100),
                entry("notes.txt", EntryKind::File, 10),
            ]),
        });
        app.panels[0].cursor = app.panels[0]
            .listing
            .order
            .iter()
            .position(|i| app.panels[0].listing.entries[*i as usize].name_str() == "tree.zip")
            .expect("the archive is in the listing");

        app.run(Command::ArcCommands);
        let Mode::Pick(pick) = &app.mode else {
            panic!("no menu: {:?}", app.mode)
        };
        assert_eq!(pick.kind, PickKind::ArcCommands);
        let labels: Vec<String> = pick.rows.iter().map(|r| r.label.clone()).collect();
        assert!(
            labels.iter().any(|l| l.contains("tree.zip")),
            "the menu does not say which archive: {labels:?}"
        );
        let values: Vec<&str> = pick.rows.iter().map(|r| r.value.as_str()).collect();
        assert_eq!(values, ["test", "here", "to", "delete", "add"]);

        // On a file that is not an archive, only the one entry that does not need one.
        app.mode = Mode::Normal;
        app.panels[0].cursor = app.panels[0]
            .listing
            .order
            .iter()
            .position(|i| app.panels[0].listing.entries[*i as usize].name_str() == "notes.txt")
            .expect("the file is in the listing");
        app.run(Command::ArcCommands);
        let Mode::Pick(pick) = &app.mode else {
            panic!("no menu: {:?}", app.mode)
        };
        let values: Vec<&str> = pick.rows.iter().map(|r| r.value.as_str()).collect();
        assert_eq!(values, ["add"]);
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
        // Hidden files are shown by default, so the box starts ticked and Space clears it.
        assert!(app.panels[0].show_hidden);
        open(&mut app, 3, "menu-panel-settings");
        press(&mut app, "Space"); // show hidden files
        press(&mut app, "F10"); // the shortcut for OK
        assert!(!app.cfg.panel.show_hidden);
        assert!(
            !app.panels[0].show_hidden,
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
        let dir = std::env::temp_dir().join(format!("fcmd-setup-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("fcmd.toml");
        let mut app = app();
        app.paths.config_file = file.clone();
        app.cfg.panel.show_hidden = true;
        press(&mut app, "Shift+F9");
        assert!(!app.status_error, "{}", app.status);
        let text = std::fs::read_to_string(&file).unwrap();
        let (back, problems) =
            Config::parse("fcmd.toml", &text).expect("what we write, we can read");
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

    // ── what to run for a kind of file (SPEC 06 §5) ─────────────────────────────────────────

    /// The command line a key asked to run, if it asked for one.
    fn shell_line(app: &mut App) -> Option<String> {
        app.take_requests().into_iter().find_map(|r| match r {
            Request::Shell { line, .. } => Some(line),
            _ => None,
        })
    }

    // ── the user menu (`F2`, SPEC 06 §4) ────────────────────────────────────────────────

    /// An app whose `F2` reads the menu written here, in a directory of this test's own. The
    /// counter is what keeps two of these apart: the tests run in parallel, and a name made
    /// from the text itself would collide the moment two menus said the same thing.
    fn app_with_menu(text: &str) -> (App, std::path::PathBuf) {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("fcmd-menu-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(fcmd_config::usermenu::MENU_FILE_NAME);
        std::fs::write(&file, text).unwrap();
        let mut app = app();
        app.paths.menu_file = file;
        (app, dir)
    }

    fn pick_labels(app: &App) -> Vec<String> {
        let Mode::Pick(pick) = &app.mode else {
            panic!("no list is open; status: {}", app.status)
        };
        assert_eq!(pick.kind, PickKind::UserMenu);
        pick.rows.iter().map(|r| r.label.clone()).collect()
    }

    #[test]
    fn with_no_menu_file_f2_says_where_one_goes() {
        let mut app = app();
        app.paths.menu_file = std::path::PathBuf::from("/nowhere/fcmd-menu.toml");
        press(&mut app, "F2");
        assert!(matches!(app.mode, Mode::Normal), "a list opened anyway");
        assert!(
            app.status.contains("/nowhere/fcmd-menu.toml"),
            "{}",
            app.status
        );
    }

    #[test]
    fn the_menu_shows_what_the_file_holds_and_leaves_out_what_a_condition_rules_out() {
        let (mut app, dir) = app_with_menu(
            r#"
[[menu]]
key = "w"
label = "Count the lines"
run = ["wc -l {file}"]

[[menu]]
key = "m"
label = "Make"
when = "panel.has('Makefile')"
run = ["make"]

[[menu]]
key = "t"
label = "Touch the text"
when = "panel.has('*.txt')"
run = ["touch {file}"]
"#,
        );
        press(&mut app, "F2");
        // No Makefile in this panel, and an `a.txt` there is.
        assert_eq!(pick_labels(&app), ["Count the lines", "Touch the text"]);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_row_runs_its_command_with_the_substitutions_already_in_it() {
        let (mut app, dir) =
            app_with_menu("[[menu]]\nkey = \"w\"\nlabel = \"Count\"\nrun = [\"wc -l {file}\"]\n");
        app.panel_mut().cursor_to(2); // a.txt
        press(&mut app, "F2");
        press(&mut app, "Enter");
        let line = shell_line(&mut app).expect("the command ran");
        assert!(line.starts_with("wc -l "), "{line}");
        assert!(line.contains("a.txt"), "{line}");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_key_a_row_carries_takes_it_without_the_arrows() {
        let (mut app, dir) = app_with_menu(
            r#"
[[menu]]
key = "a"
label = "First"
run = ["true"]

[[menu]]
key = "z"
label = "Second"
run = ["echo second"]
"#,
        );
        press(&mut app, "F2");
        press(&mut app, "z");
        let line = shell_line(&mut app).expect("the second ran");
        assert_eq!(line, "echo second");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_submenu_opens_and_esc_goes_back_to_the_menu_it_came_from() {
        let (mut app, dir) = app_with_menu(
            r#"
[[menu]]
key = "g"
label = "Git"
submenu = [
  { key = "s", label = "status", run = ["git status -sb"] },
  { key = "l", label = "log", run = ["git log --oneline"] },
]

[[menu]]
key = "o"
label = "Other"
run = ["true"]
"#,
        );
        press(&mut app, "F2");
        let labels = pick_labels(&app);
        assert!(labels[0].starts_with("Git"), "{labels:?}");
        assert!(
            labels[0].contains('\u{25b8}'),
            "a submenu does not say so: {labels:?}"
        );

        press(&mut app, "Enter");
        assert_eq!(pick_labels(&app), ["status", "log"]);

        // One `Esc` is the way back up, not the way out.
        press(&mut app, "Esc");
        assert_eq!(pick_labels(&app).len(), 2);
        assert!(pick_labels(&app)[1] == "Other");

        // And the second closes the menu.
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Normal));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_lines_of_one_row_run_in_order_and_a_failure_stops_the_rest() {
        let (mut app, dir) = app_with_menu(
            r#"
[[menu]]
label = "Three"
run = ["echo one", "echo two", "echo three"]
"#,
        );
        press(&mut app, "F2");
        press(&mut app, "Enter");
        let first = shell_requests(&app);
        assert_eq!(first, ["echo one"], "{first:?}");
        assert_eq!(app.menu_queue.len(), 2, "the rest is waiting");

        let (id, line) = app.shell.clone().expect("one is running");
        app.handle_event(Event::ShellDone {
            id,
            line,
            output: String::new(),
            status: Some(0),
        });
        assert_eq!(shell_requests(&app), ["echo one", "echo two"]);

        // The second fails, and the third never runs.
        let (id, line) = app.shell.clone().expect("two is running");
        app.handle_event(Event::ShellDone {
            id,
            line,
            output: String::new(),
            status: Some(2),
        });
        assert!(app.menu_queue.is_empty(), "it carried on after a failure");
        assert_eq!(shell_requests(&app).len(), 2, "the third ran anyway");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_internal_command_in_a_row_does_not_wait_for_a_shell_that_never_started() {
        // A directory that exists on every platform this runs on, in TOML's literal string
        // form so a Windows path's backslashes arrive as they were written.
        let there = std::env::temp_dir();
        let (mut app, dir) = app_with_menu(&format!(
            "[[menu]]\nlabel = \"There then run\"\nrun = ['cd {}', 'echo here']\n",
            there.display()
        ));
        press(&mut app, "F2");
        press(&mut app, "Enter");
        // `cd` is ours and never reaches a shell, so the next line went at once.
        assert_eq!(
            shell_requests(&app),
            ["echo here"],
            "status: {}",
            app.status
        );
        assert!(app.menu_queue.is_empty());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_menu_file_that_cannot_mean_anything_says_so_instead_of_opening_half_of_it() {
        let (mut app, dir) = app_with_menu("[[menu]]\nlabel = \"Nothing\"\n");
        press(&mut app, "F2");
        assert!(matches!(app.mode, Mode::Normal), "a list opened anyway");
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("Nothing"), "{}", app.status);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn enter_runs_what_an_association_says_for_that_kind_of_file() {
        let mut app = app();
        app.cfg.assocs.push(fcmd_config::AssocCfg {
            mask: "*.txt".into(),
            open: "less {file}".into(),
            ..fcmd_config::AssocCfg::default()
        });
        app.panel_mut().cursor_to(2); // a.txt
        press(&mut app, "Enter");
        let line = shell_line(&mut app).expect("a command was asked for");
        assert!(line.starts_with("less "), "{line}");
        assert!(line.contains("a.txt"), "with the file in it: {line}");
    }

    #[test]
    fn the_alternative_keys_use_what_an_association_names_and_f3_stays_ours() {
        let block = || fcmd_config::AssocCfg {
            mask: "*.txt".into(),
            view: "bat {file}".into(),
            edit: "nano {file}".into(),
            ..fcmd_config::AssocCfg::default()
        };

        // The file under the cursor is a.txt, which the block above matches.
        let on_a_txt = || {
            let mut app = app();
            app.cfg.assocs.push(block());
            app.panel_mut().cursor_to(2);
            app
        };

        let mut alt_f3 = on_a_txt();
        press(&mut alt_f3, "Alt+F3");
        let line = shell_line(&mut alt_f3).expect("the viewer command ran");
        assert!(line.starts_with("bat "), "{line}");
        assert!(
            alt_f3.mode.viewer().is_none(),
            "and ours did not open underneath it"
        );

        let mut alt_f4 = on_a_txt();
        press(&mut alt_f4, "Alt+F4");
        let line = shell_line(&mut alt_f4).expect("the editor command ran");
        assert!(line.starts_with("nano "), "{line}");

        // `F3` is ours and asks the association nothing, which is the point of having two keys.
        let mut f3 = on_a_txt();
        press(&mut f3, "F3");
        assert!(
            shell_line(&mut f3).is_none(),
            "nothing else was asked to read it"
        );
    }

    #[test]
    fn a_file_no_block_matches_is_not_opened_with_somebody_elses_command() {
        let mut app = app();
        app.cfg.assocs.push(fcmd_config::AssocCfg {
            mask: "*.txt".into(),
            open: "less {file}".into(),
            ..fcmd_config::AssocCfg::default()
        });
        app.panel_mut().cursor_to(3); // b.rs, which no block matches
        press(&mut app, "Enter");
        let line = shell_line(&mut app).expect("a command was asked for");
        assert!(!line.starts_with("less "), "{line}");
        assert!(line.contains("b.rs"), "{line}");
    }

    #[test]
    fn with_nothing_to_say_otherwise_enter_hands_the_file_to_the_desktop() {
        let mut app = app();
        app.panel_mut().cursor_to(2);
        press(&mut app, "Enter");
        let line = shell_line(&mut app).expect("a command was asked for");
        assert_eq!(
            line.split_whitespace().next(),
            fcmd_config::desktop_open_command()
                .split_whitespace()
                .next(),
            "the platform's own opener: {line}"
        );
        assert!(line.contains("a.txt"), "{line}");
    }

    #[test]
    fn with_the_desktop_turned_off_enter_says_there_is_nothing_to_open_it_with() {
        let mut app = app();
        app.cfg.general.desktop_open = false;
        app.panel_mut().cursor_to(2);
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
        assert!(app.status.contains("a.txt"), "{}", app.status);
        assert!(
            shell_line(&mut app).is_none(),
            "and nothing was run behind the message"
        );
    }

    #[test]
    fn open_with_lists_what_applies_here_and_runs_the_one_chosen() {
        let mut app = app();
        app.cfg.assocs.push(fcmd_config::AssocCfg {
            mask: "*.txt".into(),
            name: "Pager".into(),
            open: "less {file}".into(),
            edit: "nano {file}".into(),
            ..fcmd_config::AssocCfg::default()
        });
        app.cfg.assocs.push(fcmd_config::AssocCfg {
            mask: "*.rs".into(),
            open: "rustfmt {file}".into(),
            ..fcmd_config::AssocCfg::default()
        });
        app.panel_mut().cursor_to(2); // a.txt
        app.run(Command::FileAssoc);
        let Mode::Pick(pick) = &app.mode else {
            panic!("the list opens")
        };
        let values: Vec<&str> = pick.rows.iter().map(|r| r.value.as_str()).collect();
        assert!(values.contains(&"less {file}"), "{values:?}");
        assert!(values.contains(&"nano {file}"), "both commands: {values:?}");
        assert!(
            !values.iter().any(|v| v.starts_with("rustfmt")),
            "and nothing from a block that does not match: {values:?}"
        );
        assert_eq!(
            values.last().copied(),
            Some(fcmd_config::desktop_open_command()),
            "with the desktop's own opener at the end: {values:?}"
        );
        assert!(pick.rows[0].label.contains("Pager"), "{:?}", pick.rows[0]);
        press(&mut app, "Enter");
        let line = shell_line(&mut app).expect("the chosen command ran");
        assert!(line.starts_with("less "), "{line}");
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
    /// `fcmd-macros.toml` and reads it back — which is the half of the feature that a unit
    /// test of the type cannot reach.
    fn app_with_macros(dir: &std::path::Path) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local("/home/mb"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
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
        use fcmd_cmd::{Play, Step};
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
        // And the last question: how it plays. The first row is the ordinary answer.
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.rows.len(), 2);
        assert!(pick.rows[0].current);
        press(&mut app, "Enter");
        assert!(app.recording.is_none());
        assert_eq!(app.macros.all.len(), 1);
        let saved = &app.macros.all[0];
        assert_eq!(saved.key.to_string(), "Alt+9");
        assert_eq!(saved.description, "hide-and-step");
        assert_eq!(saved.play, Play::Once);
        assert!(!saved.trusted, "recording never trusts a macro");

        // It is on disk, and in the shape the file documents.
        let written = std::fs::read_to_string(dir.path().join("fcmd-macros.toml")).unwrap();
        assert!(written.contains("key = \"Alt+9\""), "{written}");
        assert!(written.contains("panel.hidden.toggle"), "{written}");

        // The scope is the one recording started in, which here is the panel.
        assert_eq!(saved.scope, fcmd_cmd::Scope::Panel);

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
    fn a_macro_recorded_in_the_hex_editor_belongs_to_the_hex_editor() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("notes.txt");
        std::fs::write(&text, "one\ntwo\n").unwrap();
        let mut app = app_with_macros(dir.path());

        // Record inside the editor, and assign the macro while looking at something else:
        // the scope is where the recording started, not where it was stopped.
        app.open_hex(&text);
        press(&mut app, "Ctrl+.");
        press(&mut app, "Down");
        press(&mut app, "Ctrl+.");
        let Mode::MacroKey { .. } = &app.mode else {
            panic!("{:?}", app.mode)
        };
        press(&mut app, "Alt+7");
        press(&mut app, "Enter");
        // Esc out of the play question is "the ordinary way", not "throw it away".
        press(&mut app, "Esc");
        assert_eq!(app.macros.all.len(), 1, "{:?}", app.status);
        assert_eq!(app.macros.all[0].play, fcmd_cmd::Play::Once);
        // The hex editor's own scope, not the text editor's: a macro of block operations and
        // byte edits has no meaning in a buffer of text, and the two are told apart now.
        assert_eq!(app.macros.all[0].scope, fcmd_cmd::Scope::Hex);

        // So the key does nothing in the panels, and the macro is still there.
        app.mode = Mode::Normal;
        let before = app.panels[app.active].cursor;
        press(&mut app, "Alt+7");
        assert_eq!(app.panels[app.active].cursor, before);
    }

    #[test]
    fn a_macro_can_be_told_to_run_once_for_every_selected_file() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with_macros(dir.path());

        // Record one step, assign it, and choose the second play mode.
        press(&mut app, "Ctrl+.");
        press(&mut app, "Ctrl+H");
        press(&mut app, "Ctrl+.");
        press(&mut app, "Alt+6");
        press(&mut app, "Enter"); // no description
        let Mode::Pick(pick) = &mut app.mode else {
            panic!("{:?}", app.mode)
        };
        pick.cursor = 1;
        press(&mut app, "Enter");
        assert_eq!(app.macros.all.len(), 1);
        assert_eq!(app.macros.all[0].play, fcmd_cmd::Play::EachSelected);

        // It survives the file: the mode is written and read back.
        let written = std::fs::read_to_string(dir.path().join("fcmd-macros.toml")).unwrap();
        assert!(written.contains("each-selected"), "{written}");
        let again = app_with_macros(dir.path());
        assert_eq!(again.macros.all[0].play, fcmd_cmd::Play::EachSelected);

        // And it runs once per selected file. The recorded step re-reads the panel, so the
        // listing is fed again first: in a test nothing serves that read.
        let entries = vec![
            Entry::parent_entry(),
            entry("a.txt", EntryKind::File, 100),
            entry("b.rs", EntryKind::File, 200),
            entry("c.rs", EntryKind::File, 300),
        ];
        app.handle_event(Event::Listed {
            panel: app.active,
            generation: app.panels[app.active].generation,
            result: Ok(entries),
        });
        app.panel_mut().cursor_to(1);
        for _ in 0..3 {
            press(&mut app, "Insert");
        }
        assert_eq!(app.panels[app.active].selection.len(), 3);
        // Three selected files mean three runs, and three toggles of the hidden flag leave
        // it flipped — an odd number is the cheapest way to count them.
        let before = app.panels[app.active].show_hidden;
        press(&mut app, "Alt+6");
        assert_ne!(
            app.panels[app.active].show_hidden, before,
            "three files means three runs"
        );
    }

    #[test]
    fn a_macro_that_deletes_asks_before_it_plays_unless_the_file_trusts_it() {
        use fcmd_cmd::{Macro, Play, Scope, Step};
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
    fn a_plugin_command_that_says_to_ask_is_asked_about_first() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_with_macros(dir.path());
        let program = dir.path().join("rename.sh");
        std::fs::write(&program, "#!/bin/sh\n").unwrap();

        // One that asks and one that does not: the plugin declares it, because the host
        // cannot know what somebody else's program will do.
        app.plugin_commands = vec![
            PluginCommand {
                plugin: "noisy".into(),
                id: "user.noisy.rename".into(),
                label: "Rename everything".into(),
                scope: fcmd_cmd::Scope::Panel,
                program: program.clone(),
                needs: vec!["panel".into(), "selection".into()],
                timeout: None,
                confirm: Some("This renames files.".into()),
            },
            PluginCommand {
                plugin: "quiet".into(),
                id: "user.quiet.look".into(),
                label: "Just look".into(),
                scope: fcmd_cmd::Scope::Panel,
                program,
                needs: vec!["panel".into()],
                timeout: None,
                confirm: None,
            },
        ];

        // Two files selected, so the question can count them.
        app.panel_mut().cursor_to(1);
        press(&mut app, "Insert");
        press(&mut app, "Insert");
        assert_eq!(app.panels[app.active].selection.len(), 2);

        app.run(Command::PluginCommands);
        let Mode::Pick(pick) = &mut app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.rows.len(), 2);
        pick.cursor = 0;
        press(&mut app, "Enter");

        // The question, with the plugin's own sentence and our count of what it will act on.
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("a plugin that renames must ask: {:?}", app.mode)
        };
        assert_eq!(confirm.action, ConfirmAction::RunPlugin(0));
        assert_eq!(confirm.title, "Rename everything");
        assert!(
            confirm
                .lines
                .iter()
                .any(|l| l.contains("This renames files.")),
            "{:?}",
            confirm.lines
        );
        assert!(
            confirm.lines.iter().any(|l| l.contains('2')),
            "the question says how many files: {:?}",
            confirm.lines
        );
        // Esc runs nothing.
        press(&mut app, "Esc");
        assert!(app.take_requests().is_empty(), "Esc must not run it");

        // The one that does not ask goes straight to the worker.
        app.run(Command::PluginCommands);
        let Mode::Pick(pick) = &mut app.mode else {
            unreachable!()
        };
        pick.cursor = 1;
        press(&mut app, "Enter");
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        let asked: Vec<_> = app
            .take_requests()
            .into_iter()
            .filter(|r| matches!(r, Request::Plugin { .. }))
            .collect();
        assert_eq!(asked.len(), 1, "it should have been asked to run");
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
    fn the_screens_list_does_not_lose_the_screen_it_is_drawn_over() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("notes.txt");
        std::fs::write(&text, "one\n").unwrap();
        let mut app = app();

        // F12 with a viewer in front: the viewer is the current row, and Esc gives it back.
        app.open_viewer_on(&text, VPath::local(&text));
        app.run(Command::AppScreens);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.rows.len(), 2, "the panels and the viewer");
        assert_eq!(pick.rows[1].value, "front");
        assert!(pick.rows[1].current, "the one in front is the current row");
        press(&mut app, "Esc");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);

        // And choosing the panels sends it to the background rather than closing it.
        app.run(Command::AppScreens);
        let Mode::Pick(pick) = &mut app.mode else {
            unreachable!()
        };
        pick.cursor = 0;
        press(&mut app, "Enter");
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        assert_eq!(app.background.len(), 1);

        // A question asked over a viewer belongs to that viewer: F12 from there lists the
        // viewer, not the panels, and nothing is left behind the list.
        app.run(Command::AppScreenPrev);
        assert!(app.mode.viewer().is_some());
        app.run(Command::ViewerSearch);
        assert!(matches!(app.mode, Mode::Form(_)), "{:?}", app.mode);
        app.run(Command::AppScreens);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(
            pick.rows.iter().any(|r| r.value == "front"),
            "the viewer under the prompt is the screen in front: {:?}",
            pick.rows
        );
        press(&mut app, "Esc");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
    }

    /// The two settings in the dialog that did nothing. Both were toggles one could turn and
    /// watch change nothing at all, which is worse than not offering them.
    #[test]
    fn the_confirmation_settings_are_the_settings_they_claim_to_be() {
        // Three applications rather than one reused: each half of this is about a setting's
        // default, and a setting changed in one must not be read in the next.
        let (mut quiet, mut asking, mut jobs) = (app(), app(), app());

        // `confirm_exit` off (the default) leaves on the first F10.
        assert!(!quiet.cfg.general.confirm_exit);
        quiet.run(Command::AppQuit);
        assert!(quiet.quit);

        // On, it asks — and Esc stays.
        asking.cfg.general.confirm_exit = true;
        asking.run(Command::AppQuit);
        assert!(!asking.quit, "it must ask first");
        assert!(matches!(asking.mode, Mode::Confirm(_)), "{:?}", asking.mode);
        press(&mut asking, "Esc");
        assert!(!asking.quit);
        asking.run(Command::AppQuit);
        press(&mut asking, "Enter");
        assert!(asking.quit);

        // `confirm_overwrite` on (the default) leaves the engine asking about a file that is
        // already there; off, the job is started with the policy that writes over it.
        fn conflict_of(app: &mut App) -> fcmd_jobs::ConflictPolicy {
            app.requests.clear();
            app.start_job(JobSpec::new(
                JobKind::Copy,
                vec![VPath::local("/a")],
                Some(VPath::local("/b")),
            ));
            app.take_requests()
                .into_iter()
                .find_map(|r| match r {
                    Request::Job { spec, .. } => Some(spec.options.on_conflict),
                    _ => None,
                })
                .expect("a job was asked for")
        }
        assert!(jobs.cfg.general.confirm_overwrite);
        assert_eq!(conflict_of(&mut jobs), fcmd_jobs::ConflictPolicy::Ask);
        jobs.cfg.general.confirm_overwrite = false;
        assert_eq!(conflict_of(&mut jobs), fcmd_jobs::ConflictPolicy::Overwrite);

        // And a dialog that chose `skip` on purpose is not overridden by a setting about
        // asking: the two mean different things.
        jobs.requests.clear();
        let mut spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local("/a")],
            Some(VPath::local("/b")),
        );
        spec.options.on_conflict = fcmd_jobs::ConflictPolicy::Skip;
        jobs.start_job(spec);
        let chosen = jobs
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::Job { spec, .. } => Some(spec.options.on_conflict),
                _ => None,
            })
            .unwrap();
        assert_eq!(chosen, fcmd_jobs::ConflictPolicy::Skip);
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

    /// `F12` has to work from inside a viewer and an editor, not only from the panels: that
    /// is the whole point of having screens, and the key bar has to say so or nobody finds it.
    #[test]
    fn f12_switches_screens_from_a_viewer_and_an_editor() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"first file\n").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"second file\n").unwrap();
        let mut app = app();
        app.panels[app.active].cwd = VPath::local(dir.path());
        let entries =
            fcmd_vfs::local::list_blocking(&VPath::local(dir.path()), &CancelToken::new()).unwrap();
        let (panel, generation) = (app.active, app.panels[app.active].generation);
        app.handle_event(Event::Listed {
            panel,
            generation,
            result: Ok(entries),
        });

        // A viewer on one file and an editor on the other.
        let row = app.panels[app.active]
            .listing
            .row_of_name("one.txt")
            .unwrap();
        app.panels[app.active].cursor_to(row);
        press(&mut app, "F3");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
        // The key bar says F12 is there, which is how anybody learns it.
        assert!(
            app.keybar_viewer().iter().any(|(n, _)| *n == 12),
            "{:?}",
            app.keybar_viewer()
        );
        assert!(app.keybar_editor().iter().any(|(n, _)| *n == 12));

        // F12 from the viewer opens the list, and the viewer is in it.
        press(&mut app, "F12");
        let Mode::Pick(pick) = &app.mode else {
            panic!("F12 did not open the screens: {:?}", app.mode)
        };
        assert_eq!(pick.kind, PickKind::Screens);
        let labels: Vec<String> = pick.rows.iter().map(|r| r.label.clone()).collect();
        assert!(
            labels
                .iter()
                .any(|l| l.contains("View") && l.contains("one.txt")),
            "{labels:?}"
        );
        // And the panels are the first row, as they are the first screen in FAR.
        assert!(labels[0].to_lowercase().contains("panel"), "{labels:?}");
        // The path, not just the name: two files of one name are two screens.
        assert!(
            labels
                .iter()
                .any(|l| l.contains(&dir.path().display().to_string())),
            "{labels:?}"
        );
        press(&mut app, "Esc");
        assert!(app.mode.viewer().is_some(), "Esc goes back to the viewer");
    }

    /// Coming back out of a directory — or out of an archive — puts the cursor back on what
    /// was left, rather than at the top of a listing with four hundred rows in it.
    #[test]
    fn going_up_lands_on_what_was_left() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir(root.join("alpha")).unwrap();
        std::fs::create_dir(root.join("beta")).unwrap();
        std::fs::create_dir(root.join("gamma")).unwrap();
        // An archive, so the way out of one is tested too: the name to go back to is the
        // archive's own, not the directory's inside it.
        let zip = root.join("tree.zip");
        let file = std::fs::File::create(&zip).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        writer
            .start_file("inside.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        std::io::Write::write_all(&mut writer, b"x").unwrap();
        writer.finish().unwrap();

        let mut app = app();
        let read = |app: &mut App| {
            let path = app.panels[app.active].cwd.clone();
            let entries = if path.in_container() {
                let vfs = fcmd_vfs::ArcVfs::new(fcmd_arc::Limits::default());
                let rt = tokio::runtime::Builder::new_current_thread()
                    .build()
                    .unwrap();
                rt.block_on(fcmd_vfs::Vfs::list(&vfs, &path, CancelToken::new()))
                    .unwrap()
            } else {
                fcmd_vfs::local::list_blocking(&path, &CancelToken::new()).unwrap()
            };
            let (panel, generation) = (app.active, app.panels[app.active].generation);
            app.handle_event(Event::Listed {
                panel,
                generation,
                result: Ok(entries),
            });
        };

        app.panels[app.active].cwd = VPath::local(root);
        read(&mut app);
        // Into `beta`, and back out: the cursor is on `beta` and not on `alpha`.
        let beta = app.panels[app.active]
            .listing
            .row_of_name("beta")
            .expect("beta is there");
        app.panels[app.active].cursor_to(beta);
        app.run(Command::NavEnter);
        read(&mut app);
        assert_eq!(app.panels[app.active].cwd, VPath::local(root.join("beta")));
        app.run(Command::NavParent);
        read(&mut app);
        assert_eq!(
            app.panels[app.active]
                .current()
                .map(|e| e.name_str().into_owned()),
            Some("beta".to_string())
        );

        // Into the archive and out again by `Enter` on `..`, which is the other way up.
        let row = app.panels[app.active]
            .listing
            .row_of_name("tree.zip")
            .expect("the archive is there");
        app.panels[app.active].cursor_to(row);
        app.run(Command::NavEnter);
        read(&mut app);
        assert!(app.panels[app.active].cwd.in_container(), "inside the zip");
        app.panels[app.active].cursor_to(0); // the `..` row
        app.run(Command::NavEnter);
        read(&mut app);
        assert_eq!(app.panels[app.active].cwd, VPath::local(root));
        assert_eq!(
            app.panels[app.active]
                .current()
                .map(|e| e.name_str().into_owned()),
            Some("tree.zip".to_string()),
            "coming out of an archive lands on the archive"
        );
    }

    /// A tunnel goes over the connection the panel is on, so there has to be one, and what
    /// is typed has to be a tunnel. Both are said while the dialog is still on screen.
    #[test]
    fn a_tunnel_needs_a_connection_and_a_shape_that_parses() {
        let mut app = app();
        app.run(Command::NetTunnel);
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert!(app.status_error, "{}", app.status);

        app.panels[app.active].cwd = VPath::remote(fcmd_core::Scheme::Sftp, "mb@jump:22", "/srv");
        app.run(Command::NetTunnel);
        assert!(matches!(app.mode, Mode::Prompt(_)), "{:?}", app.mode);

        // Not a tunnel: the prompt says so and asks nothing of the worker.
        type_text(&mut app, "eight:db:5432");
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
        assert!(
            !app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::TunnelOpen { .. })),
            "nothing should have been asked for"
        );

        // A tunnel: the request carries the connection the panel is on.
        app.run(Command::NetTunnel);
        type_text(&mut app, "8080:db.internal:5432");
        press(&mut app, "Enter");
        let opened = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::TunnelOpen {
                    authority, spec, ..
                } => Some((authority, spec)),
                _ => None,
            })
            .expect("the tunnel was asked for");
        assert_eq!(
            opened,
            (
                "mb@jump:22".to_string(),
                "8080:db.internal:5432".to_string()
            )
        );
    }

    /// The list shows what is open with its counters, and choosing one closes it.
    #[test]
    fn the_tunnel_list_closes_what_is_chosen() {
        let mut app = app();
        app.handle_event(Event::Tunnels {
            items: vec![
                TunnelInfo {
                    id: 1,
                    description: "127.0.0.1:8080 → db.internal:5432 on mb@jump:22".into(),
                    served: 3,
                    live: 1,
                    failed: 0,
                },
                TunnelInfo {
                    id: 2,
                    description: "127.0.0.1:9000 → minio:9000 on mb@jump:22".into(),
                    served: 0,
                    live: 0,
                    failed: 2,
                },
            ],
        });
        // Opening one says so in the status line without the list being on screen.
        assert!(
            app.status.contains("8080") || app.status.contains("9000"),
            "{}",
            app.status
        );

        app.run(Command::NetTunnels);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.kind, PickKind::Tunnels);
        assert_eq!(pick.rows.len(), 2);
        assert!(
            pick.rows[0].label.contains("db.internal"),
            "{:?}",
            pick.rows[0].label
        );
        // The counters are in the row: carried, open, refused.
        assert!(
            pick.rows[1].label.ends_with("0 / 0 / 2"),
            "{:?}",
            pick.rows[1].label
        );

        press(&mut app, "Down");
        press(&mut app, "Enter");
        let closed = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::TunnelClose { id } => Some(id),
                _ => None,
            })
            .expect("it was closed");
        assert_eq!(closed, 2);
        assert_eq!(app.tunnels.len(), 1, "the row goes with it");
    }

    /// With nothing open, the list is a sentence rather than an empty box.
    #[test]
    fn no_tunnels_is_said_rather_than_drawn() {
        let mut app = app();
        app.run(Command::NetTunnels);
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert!(!app.status.is_empty());
    }

    /// Comparing by content: the pairs that matched by name are read, and what differs is
    /// selected on both sides along with whatever has no counterpart at all.
    #[test]
    fn comparing_by_content_reads_the_pairs_and_marks_what_differs() {
        let left = tempfile::tempdir().unwrap();
        let right = tempfile::tempdir().unwrap();
        // Same name, same size, same bytes: identical.
        std::fs::write(left.path().join("same.txt"), b"hello world").unwrap();
        std::fs::write(right.path().join("same.txt"), b"hello world").unwrap();
        // Same name, same size, different bytes — the case only a read can tell apart, and
        // the whole reason this command exists.
        std::fs::write(left.path().join("sneaky.bin"), b"AAAABBBB").unwrap();
        std::fs::write(right.path().join("sneaky.bin"), b"AAAACCCC").unwrap();
        // Same name, different size: settled without opening anything.
        std::fs::write(left.path().join("grown.log"), b"one line\n").unwrap();
        std::fs::write(right.path().join("grown.log"), b"one line\ntwo lines\n").unwrap();
        // The same text, checked out on two systems: every line differs and none of it means
        // anything. FAR has a checkbox for this; here it is a third answer.
        std::fs::write(left.path().join("windows.txt"), b"one\r\ntwo\r\n").unwrap();
        std::fs::write(right.path().join("windows.txt"), b"one\ntwo\n").unwrap();
        // Only on one side.
        std::fs::write(left.path().join("only-left.txt"), b"x").unwrap();
        std::fs::write(right.path().join("only-right.txt"), b"y").unwrap();

        let mut app = app();
        app.panels[0].cwd = VPath::local(left.path());
        app.panels[1].cwd = VPath::local(right.path());
        for (side, dir) in [(0usize, left.path()), (1, right.path())] {
            let entries =
                fcmd_vfs::local::list_blocking(&VPath::local(dir), &CancelToken::new()).unwrap();
            app.handle_event(Event::Listed {
                panel: side,
                generation: app.panels[side].generation,
                result: Ok(entries),
            });
        }

        app.run(Command::ToolCompareContents);
        let request = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                Request::CompareContents { id, pairs, cancel } => Some((id, pairs, cancel)),
                _ => None,
            })
            .expect("a comparison was asked for");
        let (id, pairs, cancel) = request;
        // Three names are on both sides; the two that are not are not pairs.
        let mut names: Vec<&str> = pairs.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            vec!["grown.log", "same.txt", "sneaky.bin", "windows.txt"]
        );

        // The real worker, on this thread: the reading, the progress and the verdict are the
        // ones the binary runs, not a second implementation written for the test.
        let (tx, rx) = std::sync::mpsc::channel();
        crate::run_compare(id, pairs, cancel, tx);
        let mut done = None;
        let mut progress = 0;
        while let Ok(event) = rx.try_recv() {
            match &event {
                Event::CompareProgress { .. } => progress += 1,
                Event::CompareDone {
                    differ,
                    line_endings,
                    ..
                } => done = Some((differ.clone(), line_endings.clone())),
                _ => {}
            }
            app.handle_event(event);
        }
        let (mut differ, eol) = done.expect("the comparison finished");
        differ.sort();
        assert_eq!(differ, vec!["grown.log", "sneaky.bin"]);
        // The pair that differs only in its line endings is named apart from the rest.
        assert_eq!(eol, vec!["windows.txt"]);
        assert!(progress >= 1, "it should have reported progress");

        let marked = |app: &App, side: usize| -> Vec<String> {
            let panel = &app.panels[side];
            let mut out: Vec<String> = (0..panel.listing.len())
                .filter_map(|row| panel.listing.row(row))
                .filter(|e| panel.selection.contains(e.id))
                .map(|e| e.name_str().into_owned())
                .collect();
            out.sort();
            out
        };
        // The identical pair is not marked anywhere; everything else is, on the side that
        // has it.
        assert_eq!(
            marked(&app, 0),
            vec!["grown.log", "only-left.txt", "sneaky.bin", "windows.txt"]
        );
        assert_eq!(
            marked(&app, 1),
            vec!["grown.log", "only-right.txt", "sneaky.bin", "windows.txt"]
        );
        // And the sentence says which kind of difference each number is.
        assert!(app.status.contains("line endings"), "{}", app.status);
        assert!(app.status.contains('2'), "{}", app.status);
    }

    /// A comparison of ten thousand files started by mistake has to be stoppable, and the
    /// key for that is the one that means "no" everywhere else.
    #[test]
    fn esc_stops_a_comparison_that_is_running() {
        let mut app = app();
        let cancel = CancelToken::new();
        app.compare = Some((7, cancel.clone()));
        press(&mut app, "Esc");
        assert!(
            cancel.is_cancelled(),
            "the token should have been cancelled"
        );
        assert!(app.compare.is_none());

        // What arrives after that is a result nobody is waiting for, and it changes nothing.
        app.handle_event(Event::CompareDone {
            id: 7,
            differ: vec!["x".into()],
            line_endings: Vec::new(),
            unreadable: 0,
            stopped: true,
        });
        assert!(app.panels[0].selection.is_empty());
    }

    /// A remote panel has no local bytes to read, and the refusal says which panel and why
    /// rather than failing somewhere in a worker.
    #[test]
    fn comparing_by_content_needs_both_panels_on_disk() {
        let mut app = app();
        app.panels[1].cwd = VPath::remote(fcmd_core::Scheme::Sftp, "mb@host:22", "/srv");
        app.run(Command::ToolCompareContents);
        assert!(
            app.take_requests()
                .iter()
                .all(|r| !matches!(r, Request::CompareContents { .. })),
            "it should not have started"
        );
        assert!(app.status_error, "it should have said why: {}", app.status);
    }

    /// A listing the provider stopped short of the end must not pass for a whole one. The
    /// panel keeps the fact with the listing — a message lasts until the next keystroke, and
    /// this has to outlive that — and says why, once.
    #[test]
    fn a_listing_cut_short_is_marked_and_explained() {
        let mut app = app();
        let generation = app.panels[0].generation;
        app.handle_event(Event::Listed {
            panel: 0,
            generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("000001.json", EntryKind::File, 10),
                entry("000002.json", EntryKind::File, 10),
            ]),
        });
        assert!(!app.panels[0].listing.truncated);

        app.handle_event(Event::ListingCut {
            panel: 0,
            generation,
            note: "the first 2 of this prefix, in 1 requests; there are more".into(),
        });
        assert!(app.panels[0].listing.truncated);
        let message = app.panels[0].message.clone().expect("a note");
        assert!(message.contains("there are more"), "{message}");

        // A keystroke clears the note, and the listing still knows.
        app.run(Command::NavDown);
        assert_eq!(app.panels[0].message, None);
        assert!(app.panels[0].listing.truncated);

        // Reading the directory again starts from a clean slate: a listing that came back
        // whole must not inherit the last one's mark.
        let next = app.panels[0].generation + 1;
        app.panels[0].generation = next;
        app.handle_event(Event::Listed {
            panel: 0,
            generation: next,
            result: Ok(vec![Entry::parent_entry()]),
        });
        assert!(!app.panels[0].listing.truncated);

        // And a note for a listing we have already left is dropped, like the listing itself.
        app.handle_event(Event::ListingCut {
            panel: 0,
            generation: next - 1,
            note: "stale".into(),
        });
        assert!(!app.panels[0].listing.truncated);
        assert_eq!(app.panels[0].message, None);
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
    fn an_arrow_ends_a_quick_search_and_leaves_the_cursor_where_it_found() {
        let mut app = app();
        app.cfg.panel.quick_search = QuickSearch::Plain;
        press(&mut app, "d"); // docs
        let found = app.panel().cursor;
        assert_eq!(found, 1);
        // `Down` is the user leaving the search, not searching further: the row the search
        // found is where they stay, and the arrow does not move them off it.
        press(&mut app, "Down");
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert_eq!(app.panel().cursor, found);
        // And the next one moves, because the search is over.
        press(&mut app, "Down");
        assert_eq!(app.panel().cursor, found + 1);
    }

    #[test]
    fn the_memo_file_beside_a_table_is_found_by_either_spelling() {
        let dir = tempfile::tempdir().unwrap();
        let table = dir.path().join("people.dbf");
        std::fs::write(&table, b"not really a table").unwrap();
        // Nothing beside it yet.
        assert_eq!(memo_file(&table), None);
        // dBase writes `.dbt`.
        let dbt = dir.path().join("people.dbt");
        std::fs::write(&dbt, b"memo").unwrap();
        assert_eq!(memo_file(&table), Some(dbt.clone()));
        std::fs::remove_file(&dbt).unwrap();
        // FoxPro writes `.fpt`.
        let fpt = dir.path().join("people.fpt");
        std::fs::write(&fpt, b"memo").unwrap();
        assert_eq!(memo_file(&table), Some(fpt));
        // A table shouting in capitals, with its memo file shouting back — a DOS habit that
        // outlived DOS, and the one reason the spellings are tried both ways.
        let loud = dir.path().join("PEOPLE.DBF");
        std::fs::write(&loud, b"not really a table").unwrap();
        let loud_memo = dir.path().join("PEOPLE.DBT");
        std::fs::write(&loud_memo, b"memo").unwrap();
        assert_eq!(memo_file(&loud), Some(loud_memo));
        // And a file that is not a table has no memo file to look for.
        assert_eq!(memo_file(&dir.path().join("notes.txt")), None);
    }

    #[test]
    fn hidden_toggle_applies_to_both_panels() {
        let mut app = app();
        press(&mut app, "Ctrl+H");
        assert!(!app.panels[0].show_hidden && !app.panels[1].show_hidden);
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
        press(&mut app, "Ctrl+Shift+I");
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
        press(&mut app, "Ctrl+Shift+T");
        assert_eq!(app.theme.id, "far");
        assert!(app.status.contains("FAR"), "{}", app.status);
    }

    /// `F1` is about where you are: the viewer's keys from the viewer, the editor's from
    /// the editor — and the screen it was opened over is still there when it closes.
    #[test]
    fn the_help_is_about_where_you_are() {
        use crate::help::{Chapter, Where};
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), b"one\ntwo\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");

        press(&mut app, "F1");
        let Mode::Help { chapter, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Here(Where::Viewer));
        let page = app.help_lines(chapter, 76);
        let text = page.join("\n");
        assert!(text.contains("The viewer"), "{text}");
        assert!(
            text.contains("wrap the long lines"),
            "the viewer's own keys are not on its page: {text}"
        );
        assert!(
            !text.contains("make a directory"),
            "the panels' keys are on the viewer's page: {text}"
        );

        // Right goes on to the chapter that explains the viewer at length.
        press(&mut app, "Right");
        let Mode::Help { chapter, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert!(matches!(chapter, Chapter::Prose(_)));

        // And out of the help is back into the viewer, not onto the panels.
        press(&mut app, "F1");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);

        // From the editor it is the editor's page.
        press(&mut app, "F6"); // viewer to editor
        press(&mut app, "F1");
        let Mode::Help { chapter, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Here(Where::Editor));
        press(&mut app, "F1");
        assert!(app.mode.editor().is_some(), "{:?}", app.mode);
    }

    #[test]
    fn the_help_is_a_book_whose_reference_chapters_are_generated() {
        use crate::help::Chapter;
        let mut app = app();

        // F1 opens the page about where you are, with the keys of this screen on it.
        press(&mut app, "F1");
        let Mode::Help { chapter, at, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Here(crate::help::Where::Panels));
        assert_eq!(at, 0);
        // Esc from there is the table of contents, which is the book behind the page.
        press(&mut app, "Esc");
        let Mode::Help { chapter, .. } = app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(chapter, Chapter::Index);
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
    fn f5_opens_the_copy_dialog_with_the_passive_panel_prefilled() {
        let mut app = app();
        press(&mut app, "Down"); // docs/
        press(&mut app, "F5");
        match &app.mode {
            Mode::Form(form) => {
                assert_eq!(form.kind, FormKind::Copy);
                let target = form.text_value("copy.target");
                assert!(target.starts_with("/tmp"), "{target}");
                // The prefill ends with the platform's separator, so typing a name appends.
                assert!(
                    form.fields.iter().any(|f| matches!(
                        f,
                        crate::form::Field::Text { key, value, .. }
                            if *key == "copy.target" && value.ends_with(std::path::MAIN_SEPARATOR)
                    )),
                    "{target}"
                );
                // What is being copied is said in the dialog, which is where the prompt used
                // to say it in its title.
                let rows: String = form.rows().iter().map(|r| r.text.clone()).collect();
                assert!(rows.contains("docs"), "{rows}");
                // The cursor starts on the destination, not on the first row, which is text.
                assert_eq!(form.cursor, 1);
            }
            other => panic!("expected the copy dialog, got {other:?}"),
        }
    }

    /// Every row of the copy dialog reaches the job: the engine could already do all of this
    /// and the one-line prompt had no way to ask for any of it.
    #[test]
    fn the_copy_dialog_carries_every_option_into_the_job() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down"); // a.txt
        press(&mut app, "F5");

        // Walk the rows and set each one, by key rather than by position.
        let set_toggle = |app: &mut App, key: &str| {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(|f| matches!(f, crate::form::Field::Toggle { key: k, .. } if *k == key))
                .unwrap_or_else(|| panic!("no row `{key}`"));
            press(app, "Space");
        };
        let type_into = |app: &mut App, key: &str, text: &str| {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(|f| matches!(f, crate::form::Field::Text { key: k, .. } if *k == key))
                .unwrap_or_else(|| panic!("no row `{key}`"));
            clear_prompt(app);
            type_text(app, text);
        };

        // The conflict policy is a choice row: Right steps it along.
        {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(
                    |f| matches!(f, crate::form::Field::Choice { key, .. } if *key == "copy.conflict"),
                )
                .expect("the policy row");
        }
        for _ in 0..5 {
            press(&mut app, "Right");
        }
        set_toggle(&mut app, "copy.only_newer");
        set_toggle(&mut app, "copy.follow_links");
        set_toggle(&mut app, "copy.times");
        type_into(&mut app, "copy.filter", "*.txt");
        let out = absolute("fcmd-copy-out");
        {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(
                    |f| matches!(f, crate::form::Field::Text { key, .. } if *key == "copy.target"),
                )
                .expect("the target row");
        }
        clear_prompt(&mut app);
        type_path(&mut app, &out.to_string_lossy());
        press(&mut app, "Enter");

        let (_, spec) = job_request(&mut app);
        assert_eq!(spec.kind, JobKind::Copy);
        assert_eq!(spec.target, Some(VPath::local(&out)));
        assert_eq!(spec.options.on_conflict, fcmd_jobs::ConflictPolicy::Append);
        assert!(spec.options.only_newer);
        assert!(spec.options.follow_symlinks);
        assert!(!spec.options.preserve.mtime, "the box was turned off");
        assert!(spec.options.preserve.mode, "and this one was left on");
        let masks = spec.options.filter.as_ref().expect("a filter was typed");
        assert!(masks.matches_name("notes.txt"));
        assert!(!masks.matches_name("notes.rs"));

        // And the next dialog opens on the answers the last one was given. (The job's
        // progress box is on screen now, so the test steps past it as finishing would.)
        app.mode = Mode::Normal;
        press(&mut app, "F5");
        let Mode::Form(form) = &app.mode else {
            panic!("the dialog reopens")
        };
        assert_eq!(form.text_value("copy.conflict"), "append");
        assert!(form.toggle_value("copy.only_newer"));
        assert_eq!(form.text_value("copy.filter"), "*.txt");
    }

    /// A mask that does not parse is said so about while the dialog is still on screen, not
    /// halfway down a tree.
    #[test]
    fn a_filter_that_is_not_a_mask_keeps_the_dialog_open() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "F5");
        {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(
                    |f| matches!(f, crate::form::Field::Text { key, .. } if *key == "copy.filter"),
                )
                .expect("the filter row");
        }
        type_text(&mut app, "/[unclosed/");
        press(&mut app, "Enter");
        assert!(
            matches!(&app.mode, Mode::Form(f) if f.kind == FormKind::Copy),
            "the dialog stays: {:?}",
            app.mode
        );
        assert!(app.take_requests().is_empty(), "and nothing starts");
        assert!(!app.status.is_empty(), "and it says why");
    }

    /// Several destinations, `;`-separated: one job each, in order, and the second is a copy
    /// even when the first was a move — deleting the source after the first would leave the
    /// second nothing to carry.
    #[test]
    fn a_copy_to_several_destinations_runs_them_one_after_the_other() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "F6");
        {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = form
                .fields
                .iter()
                .position(
                    |f| matches!(f, crate::form::Field::Text { key, .. } if *key == "copy.target"),
                )
                .expect("the target row");
        }
        clear_prompt(&mut app);
        let (one, two) = (absolute("fcmd-one"), absolute("fcmd-two"));
        type_path(
            &mut app,
            &format!("{};{}", one.to_string_lossy(), two.to_string_lossy()),
        );
        press(&mut app, "Enter");

        // The first is on its way, and the second is waiting rather than running too.
        let (id, first) = job_request(&mut app);
        assert_eq!(first.kind, JobKind::Move);
        assert_eq!(first.target, Some(VPath::local(&one)));
        assert_eq!(app.queued_jobs.len(), 1);
        assert!(app.status.contains('2'), "{}", app.status);

        // When it finishes cleanly, the next one starts — as a copy.
        app.handle_event(Event::JobDone {
            id,
            kind: JobKind::Move,
            outcome: Outcome::default(),
        });
        let (_, second) = job_request(&mut app);
        assert_eq!(second.kind, JobKind::Copy, "the source is already gone");
        assert_eq!(second.target, Some(VPath::local(&two)));
        assert!(app.queued_jobs.is_empty());
    }

    /// A destination with a `;` in its name is quoted, and a line with no separator is one
    /// destination whatever is in it.
    #[test]
    fn destinations_are_split_on_semicolons_unless_they_are_quoted() {
        assert_eq!(split_targets("/tmp/a"), vec!["/tmp/a"]);
        assert_eq!(split_targets("/tmp/a;/tmp/b"), vec!["/tmp/a", "/tmp/b"]);
        assert_eq!(split_targets(" /tmp/a ; /tmp/b "), vec!["/tmp/a", "/tmp/b"]);
        assert_eq!(split_targets("\"/tmp/a;b\""), vec!["/tmp/a;b"]);
        assert_eq!(
            split_targets("\"/tmp/a;b\";/tmp/c"),
            vec!["/tmp/a;b", "/tmp/c"]
        );
        // Empty pieces are not destinations.
        assert_eq!(split_targets(";;/tmp/a;;"), vec!["/tmp/a"]);
        assert!(split_targets("   ").is_empty());
    }

    /// Cancelling one destination cancels the rest: `Esc` meant the whole thing.
    #[test]
    fn stopping_one_destination_stops_the_queue() {
        let mut app = app();
        press(&mut app, "Down");
        press(&mut app, "Down");
        press(&mut app, "F5");
        {
            let Mode::Form(form) = &mut app.mode else {
                panic!("the dialog is open")
            };
            form.cursor = 1;
        }
        clear_prompt(&mut app);
        type_path(&mut app, "a;b;c");
        press(&mut app, "Enter");
        let (id, _) = job_request(&mut app);
        assert_eq!(app.queued_jobs.len(), 2);

        app.handle_event(Event::JobDone {
            id,
            kind: JobKind::Copy,
            outcome: Outcome {
                cancelled: true,
                ..Outcome::default()
            },
        });
        assert!(app.queued_jobs.is_empty(), "the rest is dropped");
        assert!(
            !app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Job { .. })),
            "and nothing else starts"
        );
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
                    // The wastebasket is on by default, so this is what `F8` means now.
                    ConfirmAction::Job(spec) => assert_eq!(spec.kind, JobKind::Trash),
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
        // With no wastebasket to catch them, the dialog has to say so.
        app.cfg.general.use_trash = false;
        press(&mut app, "Down");
        press(&mut app, "F8");
        match &app.mode {
            Mode::Confirm(c) => assert!(
                c.lines
                    .iter()
                    .any(|l| l.to_lowercase().contains("permanent")),
                "the user must be told it cannot be undone: {:?}",
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
        assert_eq!(spec.kind, JobKind::Trash);
    }

    /// A panel over a real directory, so `F3` can open a real file.
    fn app_on_disk(dir: &std::path::Path) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir),
            right: VPath::local(dir),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
        });
        app.requests.clear();
        let entries = fcmd_vfs::local::list_blocking(&VPath::local(dir), &CancelToken::new())
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
        assert_eq!(viewer.mode, fcmd_view::Mode::Text);

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
    fn shift_f10_saves_the_file_and_leaves_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("note.txt");
        std::fs::write(&file, b"one\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        assert!(app.mode.editor().is_some(), "the editor is open");
        type_text(&mut app, "two");
        press(&mut app, "Shift+F10");
        assert!(app.mode.is_normal(), "it saved and left");
        let on_disk = std::fs::read_to_string(&file).unwrap();
        assert!(on_disk.starts_with("two"), "{on_disk:?}");
    }

    #[test]
    fn ctrl_p_hides_the_panel_that_does_not_have_the_focus() {
        let mut app = app();
        assert_eq!(app.active, 0);
        press(&mut app, "Ctrl+P");
        assert_eq!(app.hidden, Some(1), "the other one goes");
        assert_eq!(app.active, 0, "and the focus stays where it was");
        press(&mut app, "Ctrl+P");
        assert_eq!(app.hidden, None);
        // From the right panel it is the left one that goes.
        press(&mut app, "Tab");
        press(&mut app, "Ctrl+P");
        assert_eq!(app.hidden, Some(0));
        assert_eq!(app.active, 1);
    }

    #[test]
    fn ctrl_w_takes_the_word_before_the_caret_off_the_command_line() {
        let mut app = app();
        typed(&mut app, "cp /usr/local/bin");
        press(&mut app, "Ctrl+W");
        assert_eq!(app.cmd.text, "cp /usr/local/");
        press(&mut app, "Ctrl+W");
        assert_eq!(app.cmd.text, "cp /usr/");
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
        assert_eq!(app.mode.viewer().unwrap().mode, fcmd_view::Mode::Hex);
        press(&mut app, "F4");
        assert_eq!(app.mode.viewer().unwrap().mode, fcmd_view::Mode::Text);

        press(&mut app, "F2");
        assert_eq!(app.mode.viewer().unwrap().wrap, fcmd_view::Wrap::Char);
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
        // FAR asks the whole question in one dialog: what, as what, and which way.
        assert!(matches!(app.mode, Mode::Form(_)), "{:?}", app.mode);
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

    /// The dialog asks the whole question: what, as what, whole words or not, and which
    /// way. All four have to reach the search, and be there again next time.
    #[test]
    fn the_search_dialog_asks_the_whole_question() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("log.txt"),
            b"offset one
set two
offset three
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "F7");

        for c in ["s", "e", "t"] {
            press(&mut app, c);
        }
        // Down to `Whole words only` and tick it.
        let row = match &app.mode {
            Mode::Form(form) => form
                .fields
                .iter()
                .position(|f| matches!(f, crate::form::Field::Toggle { key, .. } if *key == "search.words"))
                .expect("the dialog has the row"),
            other => panic!("{other:?}"),
        };
        for _ in 0..row {
            press(&mut app, "Down");
        }
        press(&mut app, "Space");
        press(&mut app, "Enter");

        let viewer = app.mode.viewer().expect("the viewer is back");
        assert_eq!(viewer.query(), Some("set"));
        let at = viewer.last_match.expect("a hit");
        // Not the `set` inside `offset` on the first line: the second line's own word.
        assert_eq!(at.offset, 11, "whole words was not applied");
        assert!(app.search_whole_words, "the dialog's answer was not kept");

        // And the second button searches the other way, with everything still filled in.
        press(&mut app, "F7");
        match &app.mode {
            Mode::Form(form) => {
                assert_eq!(form.text_value("search.what"), "set");
                assert!(
                    form.toggle_value("search.words"),
                    "the box forgot it was ticked"
                );
                assert_eq!(form.buttons.len(), 3, "{:?}", form.buttons);
            }
            other => panic!("{other:?}"),
        }
        press(&mut app, "Tab"); // Find next
        press(&mut app, "Tab"); // Find previous
        press(&mut app, "Enter");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
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
        assert_eq!(app.mode.viewer().unwrap().mode, fcmd_view::Mode::Dump);

        press(&mut app, "F5");
        match app.mode.viewer().unwrap().mode {
            fcmd_view::Mode::Disasm => {
                let viewer = app.mode.viewer_mut().unwrap();
                let rows = viewer.rows(100, 6);
                assert!(
                    rows.iter().any(|r| r.kind == fcmd_view::RowKind::Code),
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
        assert_eq!(app.mode.viewer().unwrap().mode, fcmd_view::Mode::Text);
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

        if !fcmd_config::command_exists("cat") {
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
        if !fcmd_config::command_exists("cat") {
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

    // ── the colour groups dialog (SPEC 10 §3.11) ───────────────────────────────────────────

    #[test]
    fn the_colour_groups_are_listed_edited_reordered_and_removed_from_the_list() {
        let mut app = app();
        app.cfg.highlight = vec![
            fcmd_config::HighlightCfg {
                name: "logs".into(),
                mask: "*.log".into(),
                fg: "#d7875f".into(),
                ..Default::default()
            },
            fcmd_config::HighlightCfg {
                name: "big".into(),
                size: "1G-".into(),
                fg: "red".into(),
                ..Default::default()
            },
        ];
        app.run(Command::OptHighlight);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.kind, crate::menu::PickKind::Highlight);
        assert_eq!(pick.rows.len(), 2);
        // The row says what the group is called and what it matches, in the file's own words.
        assert!(
            pick.rows[0].label.contains("logs"),
            "{:?}",
            pick.rows[0].label
        );
        assert!(
            pick.rows[0].label.contains("*.log"),
            "{:?}",
            pick.rows[0].label
        );
        // And it is drawn in the group's own colours.
        assert!(app.highlight_ink(0).is_some());
        assert!(app.highlight_ink(9).is_none(), "a row that is not there");

        // `Ctrl+Down` moves a group past the next one: the order is what decides which group
        // paints a row that both of them match.
        press(&mut app, "Ctrl+Down");
        assert_eq!(app.cfg.highlight[0].name, "big");
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.cursor, 1, "the cursor follows the row it moved");

        // `F4` opens the group under the cursor, filled in.
        press(&mut app, "F4");
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.kind, crate::menu::FormKind::Highlight);
        assert_eq!(form.text_value("hl.mask"), "*.log");

        // A condition that does not parse comes back on the dialog, with the fields kept.
        set_text(&mut app, "hl.size", "not a size");
        press(&mut app, "F10");
        assert!(app.status_error, "{}", app.status);
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(form.text_value("hl.size"), "not a size");

        // A sound change is in force at once, and the list comes back.
        set_text(&mut app, "hl.size", "1M-");
        press(&mut app, "F10");
        assert!(!app.status_error, "{}", app.status);
        assert_eq!(app.cfg.highlight[1].size, "1M-");
        assert!(matches!(app.mode, Mode::Pick(_)), "{:?}", app.mode);

        // A group with no condition at all would paint every row, so it is refused.
        press(&mut app, "Ins");
        set_text(&mut app, "hl.name", "nothing");
        press(&mut app, "F10");
        assert!(app.status_error, "{}", app.status);
        press(&mut app, "Esc");

        // `Del` takes one away.
        app.run(Command::OptHighlight);
        press(&mut app, "Del");
        assert_eq!(app.cfg.highlight.len(), 1);
        assert_eq!(app.cfg.highlight[0].name, "logs");
    }

    // ── the view-and-edit history (SPEC 06 §6) ─────────────────────────────────────────────

    #[test]
    fn reading_and_editing_a_file_puts_it_at_the_top_of_the_history() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"one\n").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"two\n").unwrap();
        let mut app = editing_app(dir.path());
        assert!(app.file_history.is_empty());

        press(&mut app, "Down"); // one.txt
        press(&mut app, "F3");
        press(&mut app, "Esc");
        press(&mut app, "Down"); // two.txt
        press(&mut app, "F4");
        press(&mut app, "Esc");
        // Newest first, which is the order somebody reaching back wants.
        let names: Vec<String> = app
            .file_history
            .iter()
            .map(|p| p.rsplit(['/', '\\']).next().unwrap_or(p).to_string())
            .collect();
        assert_eq!(names, ["two.txt", "one.txt"]);

        // Reading one of them again moves it rather than adding a second row: the list is
        // where I have been, and a file read twice is one place.
        press(&mut app, "Up");
        press(&mut app, "F3");
        press(&mut app, "Esc");
        assert_eq!(app.file_history.len(), 2);
        assert!(
            app.file_history[0].ends_with("one.txt"),
            "{:?}",
            app.file_history
        );

        // The list opens on it, `Enter` reads it and `F4` edits it.
        app.run(Command::CmdFileHistory);
        let Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.kind, crate::menu::PickKind::FileHistory);
        assert_eq!(pick.rows.len(), 2);
        press(&mut app, "Enter");
        assert!(app.mode.viewer().is_some(), "{:?}", app.mode);
        press(&mut app, "Esc");

        app.run(Command::CmdFileHistory);
        press(&mut app, "F4");
        assert!(app.mode.editor().is_some(), "{:?}", app.mode);
        press(&mut app, "Esc");

        // A row whose file has gone says so rather than opening nothing.
        std::fs::remove_file(dir.path().join("one.txt")).unwrap();
        app.run(Command::CmdFileHistory);
        press(&mut app, "Enter");
        assert!(app.status_error, "{}", app.status);
    }

    // ── the owners view (SPEC 02 §3) ───────────────────────────────────────────────────────

    #[test]
    fn the_owners_view_asks_who_the_files_belong_to_and_a_re_read_asks_again() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        app.take_requests();

        // No other view pays for the lookup.
        app.run(Command::PanelViewSet(ViewMode::Full));
        assert!(
            !app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Owners { .. })),
            "the full view should not ask"
        );

        app.run(Command::PanelViewSet(ViewMode::Owners));
        let (panel, generation, names) = match app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, Request::Owners { .. }))
        {
            Some(Request::Owners {
                panel,
                generation,
                names,
                ..
            }) => (panel, generation, names),
            other => panic!("{other:?}"),
        };
        assert_eq!(panel, 0);
        assert!(names.contains(&"notes.txt".to_string()), "{names:?}");
        // `..` is not a file anybody owns for this purpose: it is the directory above.
        assert!(!names.contains(&"..".to_string()), "{names:?}");

        let mut owners = std::collections::HashMap::new();
        owners.insert(
            "notes.txt".to_string(),
            ("mb".to_string(), "staff".to_string()),
        );
        app.handle_event(Event::Owners {
            panel,
            generation,
            owners: owners.clone(),
        });
        assert_eq!(
            app.panels[0].owners.get("notes.txt"),
            Some(&("mb".to_string(), "staff".to_string()))
        );

        // An answer for a listing we have already replaced is not about these names.
        app.handle_event(Event::Owners {
            panel,
            generation: generation + 7,
            owners: std::collections::HashMap::new(),
        });
        assert!(!app.panels[0].owners.is_empty(), "a stale answer was taken");

        // A fresh listing has fresh names: what was known is dropped and asked for again.
        let entries =
            fcmd_vfs::local::list_blocking(&VPath::local(dir.path()), &CancelToken::new())
                .expect("the test directory");
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(entries),
        });
        assert!(app.panels[0].owners.is_empty(), "the old names were kept");
        assert!(
            app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Owners { .. })),
            "the new listing should ask again"
        );
    }

    // ── pasting, and the clipboard (SPEC 06 §11) ────────────────────────────────────────────

    #[test]
    fn a_paste_on_the_panels_goes_to_the_command_line_one_line_at_most() {
        let mut app = app();
        app.handle_input(Input::Paste("ls -la\n".to_string()));
        assert_eq!(app.cmd.text, "ls -la");
        // One line and a newline is a whole paste: nothing was left out, so nothing is said.
        assert!(app.status.is_empty(), "{}", app.status);

        // A second line would be a second command, and running it unread is exactly what
        // must not happen: the first line goes in and the rest is reported, not run.
        app.cmd.clear();
        app.handle_input(Input::Paste("make\nmake install\n".to_string()));
        assert_eq!(app.cmd.text, "make");
        assert!(!app.status.is_empty(), "nothing said about the second line");

        // A carriage return from a Windows clipboard is not a character to insert.
        app.cmd.clear();
        app.handle_input(Input::Paste("echo hi\r\n".to_string()));
        assert_eq!(app.cmd.text, "echo hi");
    }

    #[test]
    fn a_paste_in_the_editor_keeps_every_line_of_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), b"alpha\n").unwrap();
        let mut app = editing_app(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "End");
        app.handle_input(Input::Paste("\nbeta\ngamma".to_string()));
        assert_eq!(
            app.mode.editor().unwrap().buffer.lines(),
            ["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn a_paste_fills_the_dialog_row_under_the_cursor() {
        let mut app = app();
        app.run(Command::ToolFind);
        // The first row is the mask, and the dialog opens on it.
        app.handle_input(Input::Paste("*.rs\n*.toml\n".to_string()));
        let Mode::Form(form) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        // The row holds one line; the mask it opened on is still in front of it.
        assert_eq!(form.text_value("find.mask"), "**.rs");
        assert!(!app.status.is_empty(), "nothing said about the second line");
    }

    #[test]
    fn the_names_or_the_paths_of_what_is_selected_go_to_the_clipboard() {
        let mut app = app();
        press(&mut app, "Down"); // docs
        press(&mut app, "Ins"); // selected
        press(&mut app, "Ins"); // and the next row selected too

        app.run(Command::FileClipNames);
        let names = match app.take_requests().into_iter().next() {
            Some(Request::Clipboard { text }) => text,
            other => panic!("{other:?}"),
        };
        assert_eq!(names, "docs\na.txt");
        assert!(!app.status.is_empty());

        app.run(Command::FileClipPaths);
        let paths = match app.take_requests().into_iter().next() {
            Some(Request::Clipboard { text }) => text,
            other => panic!("{other:?}"),
        };
        // Built from the panel's own path rather than written out: the separator is the
        // platform's, and Windows runs these tests too.
        let cwd = app.panel().cwd.clone();
        let want = format!("{}\n{}", cwd.join("docs"), cwd.join("a.txt"));
        assert_eq!(paths, want);

        // With the setting off nothing goes out, and that is said rather than silent.
        app.cfg.ui.clipboard = false;
        app.run(Command::FileClipNames);
        assert!(app.take_requests().is_empty());
        assert!(app.status_error, "{}", app.status);
    }

    #[test]
    fn ctrl_ins_copies_the_command_line_while_there_is_something_on_it() {
        let mut app = app();
        // Empty line: the key belongs to the panel, as every shared key does.
        press(&mut app, "Down"); // off `..`, onto a row with a name
        press(&mut app, "Ctrl+Ins");
        match app.take_requests().into_iter().next() {
            Some(Request::Clipboard { text }) => assert!(!text.contains('/'), "{text}"),
            other => panic!("{other:?}"),
        }

        app.cmd.insert_str("grep -r needle .");
        press(&mut app, "Ctrl+Ins");
        match app.take_requests().into_iter().next() {
            Some(Request::Clipboard { text }) => assert_eq!(text, "grep -r needle ."),
            other => panic!("{other:?}"),
        }
        // And the line is untouched: copying is not typing.
        assert_eq!(app.cmd.text, "grep -r needle .");
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
        assert!(!app.quit, "F10 in the editor closes the editor, not fcmd");
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
            format!("hi{}", fcmd_edit::Eol::detect("").0.bytes())
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
                connected: vec![site().key()],
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
        press(&mut app, "Ctrl+Shift+N");
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
                connected: vec![site.key()],
            },
        });
        assert_eq!(app.panels[0].cwd, site.start());
        assert_eq!(
            app.panels[0].cwd.to_string(),
            "sftp://mb@host.example:22/srv"
        );
        assert!(app.connected.contains(&site.key()));
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
            outcome: ConnectOutcome::NeedsSecret(fcmd_net::SecretKind::Password),
        });
        match &app.mode {
            Mode::Prompt(p) => {
                assert_eq!(
                    p.kind,
                    PromptKind::NetSecret(fcmd_net::SecretKind::Password)
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
            Some(Request::Disconnect { key }) => {
                assert_eq!(key, "sftp://mb@host.example:22")
            }
            other => panic!("expected a disconnect, got {other:?}"),
        }
        app.handle_event(Event::Disconnected {
            key: "sftp://mb@host.example:22".into(),
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
        app.connected = vec!["sftp://root@other:2222".into()];
        let rows = app.site_rows();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].authority(), "mb@host.example:22");
        assert_eq!(rows[1].authority(), "root@other:2222");

        // `d` on a site that is not connected does not pretend to disconnect it.
        press(&mut app, "Ctrl+Shift+N");
        press(&mut app, "d");
        assert!(app.take_requests().is_empty());
        assert!(matches!(app.mode, Mode::Sites { .. }));
        // On the connected one it does.
        press(&mut app, "Down");
        press(&mut app, "d");
        match app.take_requests().into_iter().next() {
            Some(Request::Disconnect { key }) => assert_eq!(key, "sftp://root@other:2222"),
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
            VPath::remote(fcmd_core::Scheme::Sftp, "mb@host:22", "/srv"),
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

    // ── writing into an archive (SPEC 04 §4) ────────────────────────────────────────────────

    /// A zip with three entries, and a panel standing inside it at its root.
    fn app_inside_a_zip(root: &std::path::Path) -> (App, std::path::PathBuf) {
        let zip = root.join("box.zip");
        let file = std::fs::File::create(&zip).unwrap();
        let mut writer = zip::ZipWriter::new(file);
        for (name, body) in [
            ("one.txt", &b"first"[..]),
            ("two.txt", &b"second"[..]),
            ("deep/three.txt", &b"third"[..]),
        ] {
            writer
                .start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            std::io::Write::write_all(&mut writer, body).unwrap();
        }
        writer.finish().unwrap();

        let mut app = app();
        app.panels[app.active].cwd = VPath::local(&zip).enter("/");
        let path = app.panels[app.active].cwd.clone();
        let vfs = fcmd_vfs::ArcVfs::new(fcmd_arc::Limits::default());
        let rt = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let entries = rt
            .block_on(fcmd_vfs::Vfs::list(&vfs, &path, CancelToken::new()))
            .unwrap();
        let (panel, generation) = (app.active, app.panels[app.active].generation);
        app.handle_event(Event::Listed {
            panel,
            generation,
            result: Ok(entries),
        });
        app.requests.clear();
        (app, zip)
    }

    /// Empty whatever text row is open — a prompt's line, or the row the cursor is on in a
    /// dialog — so what a test types is all that is in it.
    fn clear_prompt(app: &mut App) {
        for _ in 0..256 {
            let empty = match &app.mode {
                Mode::Prompt(p) => p.text.is_empty(),
                Mode::Form(form) => match form.fields.get(form.cursor) {
                    Some(crate::form::Field::Text { value, .. }) => value.is_empty(),
                    _ => true,
                },
                _ => true,
            };
            if empty {
                return;
            }
            press(app, "Backspace");
        }
        panic!("the row would not empty");
    }

    /// A path that is absolute on every platform. `/tmp/x` is relative on Windows, where a
    /// path with no drive letter is, and would be joined to wherever the panel happens to be.
    fn absolute(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(name)
    }

    /// Type a path, upper case included: `type_text` presses the character as a key name, and
    /// a key name of one letter carries no shift.
    fn type_path(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_input(Input::Key(Key::new(KeyCode::Char(c), Mods::NONE)));
        }
    }

    /// Put the cursor on a row by name, failing loudly if the listing has not got it.
    fn cursor_on(app: &mut App, name: &str) {
        let row = app.panels[app.active]
            .listing
            .row_of_name(name)
            .unwrap_or_else(|| panic!("no `{name}` in the listing"));
        app.panels[app.active].cursor_to(row);
    }

    /// The changes a rebuild was asked for, and the archive they are for.
    fn rebuild_asked(app: &mut App) -> (std::path::PathBuf, Vec<fcmd_arc::Change>) {
        match app.take_requests().into_iter().next() {
            Some(Request::ArcRebuild {
                archive, changes, ..
            }) => (archive.as_path().to_path_buf(), changes),
            other => panic!("expected a rebuild, got {other:?}"),
        }
    }

    /// Run the changes for real, which is what the worker thread does with the request.
    fn apply(archive: &std::path::Path, changes: &[fcmd_arc::Change]) -> Vec<String> {
        fcmd_arc::rebuild(
            archive,
            changes,
            fcmd_arc::Level::Normal,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .expect("the rebuild should succeed");
        let file = std::fs::File::open(archive).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().unwrap().into_owned())
            .collect()
    }

    /// `F8` inside an archive cannot unlink anything: the archive is written again without
    /// what was selected, and the question asked first is the same question.
    #[test]
    fn f8_inside_an_archive_takes_the_entry_out_of_it() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, zip) = app_inside_a_zip(dir.path());
        cursor_on(&mut app, "two.txt");
        press(&mut app, "F8");
        let Mode::Confirm(confirm) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            panic!("F8 should ask first: {:?}", app.mode)
        };
        assert!(
            confirm.lines.iter().any(|l| l.contains("two.txt")),
            "{:?}",
            confirm.lines
        );
        app.mode = Mode::Confirm(confirm);
        press(&mut app, "Enter");
        let (archive, changes) = rebuild_asked(&mut app);
        assert_eq!(archive, zip);
        assert_eq!(
            changes,
            vec![fcmd_arc::Change::Remove {
                inner: "/two.txt".to_string()
            }]
        );
        let names = apply(&zip, &changes);
        assert_eq!(names, vec!["one.txt", "deep/three.txt"]);
    }

    /// `Shift+F6` inside an archive is a rename inside it, so the dialog offers the name the
    /// entry has rather than the other panel's directory.
    #[test]
    fn shift_f6_inside_an_archive_renames_the_entry() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, zip) = app_inside_a_zip(dir.path());
        cursor_on(&mut app, "one.txt");
        press(&mut app, "Shift+F6");
        let Mode::Form(form) = &app.mode else {
            panic!("no dialog: {:?}", app.mode)
        };
        assert_eq!(
            form.text_value("copy.target"),
            "one.txt",
            "the offer is the name, not a path"
        );
        app.mode = Mode::Normal;
        app.run(Command::FileMoveCurrent);
        clear_prompt(&mut app);
        type_text(&mut app, "first.txt");
        press(&mut app, "Enter");
        let (archive, changes) = rebuild_asked(&mut app);
        assert_eq!(archive, zip);
        assert_eq!(
            changes,
            vec![fcmd_arc::Change::Rename {
                from: "/one.txt".to_string(),
                to: "/first.txt".to_string()
            }]
        );
        let names = apply(&zip, &changes);
        assert!(names.contains(&"first.txt".to_string()), "{names:?}");
        assert!(!names.contains(&"one.txt".to_string()), "{names:?}");
    }

    /// A target that is not inside the same archive is not a rename, and the message says
    /// which key does take something out.
    #[test]
    fn moving_out_of_an_archive_says_which_key_does_that() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, _zip) = app_inside_a_zip(dir.path());
        cursor_on(&mut app, "one.txt");
        app.run(Command::FileMoveCurrent);
        clear_prompt(&mut app);
        // An absolute path on every platform: `/tmp/x` is relative on Windows, where a path
        // with no drive letter is, and would be joined to the archive we are standing in —
        // which is the opposite of what this test is about.
        let elsewhere = std::env::temp_dir().join("elsewhere.txt");
        type_path(&mut app, &elsewhere.to_string_lossy());
        press(&mut app, "Enter");
        assert!(app.take_requests().is_empty(), "nothing should be started");
        assert!(app.status.contains("F5"), "{}", app.status);
    }

    /// `F4` on an entry fetches it, and saving sends it back into the container it came from
    /// rather than to a server.
    #[test]
    fn saving_an_edit_puts_the_file_back_into_the_archive() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, zip) = app_inside_a_zip(dir.path());
        cursor_on(&mut app, "one.txt");
        press(&mut app, "F4");
        let (id, remote, purpose) = match app.take_requests().into_iter().next() {
            Some(Request::Fetch {
                id, path, purpose, ..
            }) => (id, path, purpose),
            other => panic!("expected a fetch, got {other:?}"),
        };
        assert_eq!(purpose, Fetched::Edit);
        assert_eq!(remote.inner_path(), "/one.txt");

        // The worker brings the bytes out to a copy, which is what the editor opens.
        let local = dir.path().join("one.txt");
        std::fs::write(&local, b"first").unwrap();
        app.handle_event(Event::Fetched {
            id,
            remote,
            purpose: Fetched::Edit,
            result: Ok((
                local.clone(),
                RemoteStamp {
                    size: 5,
                    mtime: None,
                },
            )),
        });
        assert!(app.mode.editor().is_some(), "the editor should be open");
        press(&mut app, "End");
        press(&mut app, "!");
        press(&mut app, "F2");
        let (archive, changes) = rebuild_asked(&mut app);
        assert_eq!(archive, zip);
        assert_eq!(
            changes,
            vec![fcmd_arc::Change::Put {
                inner: "/one.txt".to_string(),
                source: local
            }]
        );
        apply(&zip, &changes);
        let file = std::fs::File::open(&zip).unwrap();
        let mut read = zip::ZipArchive::new(file).unwrap();
        let mut body = String::new();
        std::io::Read::read_to_string(&mut read.by_name("one.txt").unwrap(), &mut body).unwrap();
        assert_eq!(body, "first!");
    }

    /// `F5` with the target inside an archive puts the file in. A directory would have to be
    /// walked, and the message points at the key that packs a whole tree.
    #[test]
    fn f5_into_an_archive_adds_the_file() {
        let dir = tempfile::tempdir().unwrap();
        let (mut app, zip) = app_inside_a_zip(dir.path());
        let source = dir.path().join("new.txt");
        std::fs::write(&source, b"fresh").unwrap();

        app.panels[app.active].cwd = VPath::local(dir.path());
        let entries =
            fcmd_vfs::local::list_blocking(&app.panels[app.active].cwd, &CancelToken::new())
                .unwrap();
        let (panel, generation) = (app.active, app.panels[app.active].generation);
        app.handle_event(Event::Listed {
            panel,
            generation,
            result: Ok(entries),
        });
        app.requests.clear();

        cursor_on(&mut app, "new.txt");
        app.run(Command::FileCopyCurrent);
        clear_prompt(&mut app);
        type_path(&mut app, &VPath::local(&zip).enter("/deep").to_string());
        press(&mut app, "Enter");
        let (archive, changes) = rebuild_asked(&mut app);
        assert_eq!(archive, zip);
        assert_eq!(
            changes,
            vec![fcmd_arc::Change::Put {
                inner: "/deep/new.txt".to_string(),
                source: source.clone()
            }]
        );
        let names = apply(&zip, &changes);
        assert!(names.contains(&"deep/new.txt".to_string()), "{names:?}");

        // A directory one at a time is refused, with the key that does it in the message.
        std::fs::create_dir(dir.path().join("tree")).unwrap();
        app.run(Command::FileCopyCurrent);
        app.mode = Mode::Normal;
        app.add_into_archive(
            vec![VPath::local(dir.path().join("tree"))],
            &VPath::local(&zip).enter("/"),
            false,
        );
        assert!(app.take_requests().is_empty(), "no rebuild for a tree");
        assert!(app.status.contains("Shift+F1"), "{}", app.status);
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
    // ── the mouse ──────────────────────────────────────────────────────────────────────────

    /// A frame has to have been drawn for a click to mean anything, since the layout is what
    /// the renderer recorded. This draws one at a known size.
    fn drawn(app: &mut App, w: u16, h: u16) {
        let mut buf = fcmd_ui::Buffer::new(w, h);
        crate::render::draw(app, &mut buf);
    }

    fn click(app: &mut App, col: u16, row: u16) {
        app.handle_input(Input::Mouse(fcmd_tui::Mouse {
            kind: fcmd_tui::MouseKind::Press(fcmd_tui::Button::Left),
            col,
            row,
            mods: Mods::NONE,
        }));
    }

    fn wheel(app: &mut App, col: u16, row: u16, down: bool) {
        app.handle_input(Input::Mouse(fcmd_tui::Mouse {
            kind: if down {
                fcmd_tui::MouseKind::ScrollDown
            } else {
                fcmd_tui::MouseKind::ScrollUp
            },
            col,
            row,
            mods: Mods::NONE,
        }));
    }

    #[test]
    fn a_click_moves_the_cursor_and_a_second_one_opens_the_row() {
        let mut app = app();
        drawn(&mut app, 80, 25);
        let row = app.layout.panels[0].expect("the left panel was drawn").area;

        // The third row of the left panel is `a.txt`: `..`, `docs`, `a.txt`.
        click(&mut app, row.x + 1, row.y + 2);
        assert_eq!(app.active, 0);
        assert_eq!(app.panel().cursor, 2);
        assert_eq!(app.panel().current().unwrap().name_str(), "a.txt");
        assert!(app.take_requests().is_empty(), "one click opens nothing");

        // Two clicks in the same place in quick succession are a double click, which is
        // Enter: on a file that is the viewer or whatever opens it, and either way it is not
        // nothing.
        click(&mut app, row.x + 1, row.y + 1); // `docs`, a directory
        click(&mut app, row.x + 1, row.y + 1);
        assert!(
            app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Read { .. })),
            "a double click on a directory did not go into it"
        );
    }

    #[test]
    fn a_click_in_the_other_panel_makes_it_the_active_one() {
        let mut app = app();
        drawn(&mut app, 80, 25);
        assert_eq!(app.active, 0);
        let right = app.layout.panels[1].expect("the right panel").area;
        click(&mut app, right.x + 1, right.y + 1);
        assert_eq!(app.active, 1, "the click did not change panels");
    }

    #[test]
    fn a_click_on_the_key_bar_presses_that_key() {
        let mut app = app();
        drawn(&mut app, 80, 25);
        let (bar, keys) = app.layout.key_bar.clone().expect("the key bar was drawn");
        let slot = 80 / keys.len() as u16;
        // Whichever cell `F5` is in — the bar holds only the keys that are bound, so counting
        // from one would be wrong — clicking it opens the copy dialog on the file under the
        // cursor, which is what pressing `F5` does.
        let fifth = keys.iter().position(|n| *n == 5).expect("F5 is on the bar");
        app.panel_mut().cursor_to(2);
        click(&mut app, slot * fifth as u16, bar);
        assert!(
            matches!(&app.mode, Mode::Form(f) if f.kind == FormKind::Copy),
            "F5 did not open the copy dialog: {:?}",
            app.mode
        );

        // And a cell is the key written in it: `F1` is help, wherever it sits.
        app.mode = Mode::Normal;
        let first = keys.iter().position(|n| *n == 1).expect("F1 is on the bar");
        click(&mut app, slot * first as u16, bar);
        assert!(
            matches!(&app.mode, Mode::Help { .. }),
            "F1 did not open the help: {:?}",
            app.mode
        );
    }

    #[test]
    fn a_click_on_the_menu_bar_opens_that_menu_and_a_click_on_a_row_runs_it() {
        let mut app = app();
        drawn(&mut app, 80, 25);
        // The menu bar is only on the screen while a menu is open, so F9 comes first.
        press(&mut app, "F9");
        drawn(&mut app, 80, 25);
        let (bar, starts) = app.layout.menu_bar.clone().expect("the menu bar");
        assert!(starts.len() > 2, "{starts:?}");
        click(&mut app, starts[2], bar);
        assert!(
            matches!(app.mode, Mode::Menu { menu: 2, .. }),
            "{:?}",
            app.mode
        );
    }

    #[test]
    fn the_wheel_scrolls_the_panel_under_it_without_taking_the_focus() {
        let mut app = app();
        // A listing longer than the panel, so there is something to scroll.
        let mut entries = vec![Entry::parent_entry()];
        for i in 0..100 {
            entries.push(entry(&format!("file{i:03}.txt"), EntryKind::File, 10));
        }
        app.handle_event(Event::Listed {
            panel: 1,
            generation: app.panels[1].generation,
            result: Ok(entries),
        });
        drawn(&mut app, 80, 25);
        let right = app.layout.panels[1].expect("the right panel").area;
        assert_eq!(app.panels[1].top, 0);

        wheel(&mut app, right.x + 1, right.y + 1, true);
        assert_eq!(app.panels[1].top, 3, "three rows a notch");
        assert_eq!(app.active, 0, "scrolling is not working in it");
        // The cursor is dragged along rather than left behind the window.
        assert!(app.panels[1].cursor >= 3);

        wheel(&mut app, right.x + 1, right.y + 1, false);
        assert_eq!(app.panels[1].top, 0);
        // And it stops at the top rather than going negative.
        wheel(&mut app, right.x + 1, right.y + 1, false);
        assert_eq!(app.panels[1].top, 0);
    }

    #[test]
    fn the_right_button_selects_the_row_it_is_on() {
        let mut app = app();
        drawn(&mut app, 80, 25);
        let area = app.layout.panels[0].expect("the left panel").area;
        app.handle_input(Input::Mouse(fcmd_tui::Mouse {
            kind: fcmd_tui::MouseKind::Press(fcmd_tui::Button::Right),
            col: area.x + 1,
            row: area.y + 2,
            mods: Mods::NONE,
        }));
        assert_eq!(app.panel().cursor, 2, "the cursor went to the row");
        assert_eq!(
            app.panel().listing.totals.selected_files,
            1,
            "the row was not selected"
        );
        // And again takes it back off: it is a toggle, not a select.
        app.handle_input(Input::Mouse(fcmd_tui::Mouse {
            kind: fcmd_tui::MouseKind::Press(fcmd_tui::Button::Right),
            col: area.x + 1,
            row: area.y + 2,
            mods: Mods::NONE,
        }));
        assert_eq!(app.panel().listing.totals.selected_files, 0);
    }

    #[test]
    fn a_click_outside_an_open_dialog_does_not_reach_the_panel_behind_it() {
        let mut app = app();
        app.run(Command::AppLanguages);
        drawn(&mut app, 80, 25);
        let before = app.panel().cursor;
        // The top left corner: the dialog is centred, so this is outside it.
        click(&mut app, 0, 1);
        assert!(matches!(app.mode, Mode::Pick(_)), "the dialog closed");
        assert_eq!(app.panel().cursor, before, "the cursor moved underneath it");

        // On a row of the list it moves the list's cursor instead.
        let list = app.layout.list.expect("the list was drawn");
        click(&mut app, list.area.x + 1, list.area.y + 1);
        let Mode::Pick(pick) = &app.mode else {
            panic!("the dialog closed")
        };
        assert_eq!(pick.cursor, 1);
    }

    #[test]
    fn the_mouse_is_only_asked_for_when_the_setting_says_so() {
        let mut app = app();
        assert!(
            !app.cfg.ui.mouse,
            "off by default: the terminal keeps its own"
        );
        app.cfg.ui.mouse = true;
        app.apply_config(&["ui.mouse"]);
        assert!(
            app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Mouse { on: true })),
            "turning it on did not reach the terminal"
        );
        app.cfg.ui.mouse = false;
        app.apply_config(&["ui.mouse"]);
        assert!(
            app.take_requests()
                .iter()
                .any(|r| matches!(r, Request::Mouse { on: false })),
            "turning it off did not reach the terminal either"
        );
    }
    // ── the palette editor ─────────────────────────────────────────────────────────────────

    #[test]
    fn the_palette_editor_changes_a_colour_on_the_screen_and_then_in_the_file() {
        let mut app = app();
        app.run(Command::OptColors);
        let Mode::Palette(view) = &app.mode else {
            panic!("the palette editor did not open: {:?}", app.mode)
        };
        assert!(view.keys.len() > 20, "every colour the interface asks for");
        assert_eq!(view.keys[0], "panel.bg");
        assert!(!view.dirty);

        // Down to a key worth looking at, then Enter to type a colour. The field opens on
        // what the row is set to, so a shade is nudged rather than retyped.
        press(&mut app, "Down");
        let Mode::Palette(view) = &app.mode else {
            panic!("it closed")
        };
        let key = view.key().expect("a row").to_string();
        assert_eq!(key, "panel.fg");
        press(&mut app, "Enter");
        let Mode::Prompt(prompt) = &app.mode else {
            panic!("no colour was asked for: {:?}", app.mode)
        };
        assert_eq!(prompt.kind, PromptKind::Color);
        assert_eq!(prompt.text, app.theme.color("panel.fg").to_string());

        let Mode::Prompt(prompt) = std::mem::replace(&mut app.mode, Mode::Normal) else {
            unreachable!()
        };
        let mut typed = prompt.clone();
        typed.text = "#ff8000".into();
        app.mode = Mode::Palette(Box::new(PaletteView {
            cursor: 1,
            ..PaletteView::new()
        }));
        app.prompt_accept(typed);
        // On the screen at once: a colour one cannot see is a colour one cannot choose.
        assert_eq!(
            app.theme.color("panel.fg"),
            fcmd_ui::Color::Rgb(255, 128, 0)
        );
        let Mode::Palette(view) = &app.mode else {
            panic!("it did not go back to the list: {:?}", app.mode)
        };
        assert!(view.dirty, "and it knows the file has not been written");

        // Rubbish is refused by name and changes nothing.
        let before = app.theme.color("panel.bg");
        let mut bad = Prompt::new(PromptKind::Color, "x".into(), "octarine".into());
        bad.kind = PromptKind::Color;
        app.mode = Mode::Palette(Box::default());
        app.prompt_accept(bad);
        assert_eq!(app.theme.color("panel.bg"), before);
        assert!(app.status_error, "{}", app.status);

        // F2 writes the live theme into the configuration as a `[[theme]]` block under its
        // own id, which is the path the loader already has for retuning a built-in.
        app.mode = Mode::Palette(Box::default());
        press(&mut app, "F2");
        assert_eq!(app.cfg.themes.len(), 1);
        let block = &app.cfg.themes[0];
        assert_eq!(block.get("id").and_then(|v| v.as_str()), Some("commander"));
        let colors = block
            .get("colors")
            .and_then(|v| v.as_table())
            .expect("the colours are in the block");
        assert_eq!(
            colors.get("panel.fg").and_then(|v| v.as_str()),
            Some("#ff8000"),
            "the edited colour was not written"
        );
        let Mode::Palette(view) = &app.mode else {
            panic!("saving closed the editor")
        };
        assert!(!view.dirty, "saved, so there is nothing unwritten left");

        // Saving again replaces the block rather than adding a second one with the same id.
        press(&mut app, "F2");
        assert_eq!(app.cfg.themes.len(), 1);
    }

    #[test]
    fn leaving_the_palette_editor_with_unsaved_colours_says_so() {
        let mut app = app();
        app.mode = Mode::Palette(Box::new(PaletteView {
            dirty: true,
            ..PaletteView::new()
        }));
        press(&mut app, "Esc");
        assert!(matches!(app.mode, Mode::Normal));
        assert!(!app.status.is_empty(), "nothing was said about the change");

        // With nothing changed it closes quietly.
        app.set_status(String::new());
        app.mode = Mode::Palette(Box::default());
        press(&mut app, "Esc");
        assert!(app.status.is_empty(), "{}", app.status);
    }
    // ── the editor's settings and its smaller keys ─────────────────────────────────────────

    /// Open the editor on a file, the way `F4` does: the panel is read from the file's own
    /// directory and the cursor put on it, so the path goes through the same code a key does.
    fn editing(dir: &std::path::Path, name: &str) -> App {
        let mut app = app_on_disk(dir);
        let listing = &app.panel().listing;
        let at = (0..listing.len())
            .find(|i| listing.row(*i).map(|e| e.name_str()).as_deref() == Some(name))
            .unwrap_or_else(|| panic!("{name} is not in the panel"));
        app.panel_mut().cursor_to(at);
        app
    }

    #[test]
    fn the_editor_opens_with_what_the_configuration_says() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("mixed.txt"), b"\tone\n  two\n").unwrap();

        let mut app = editing(dir.path(), "mixed.txt");
        app.cfg.editor.tab_width = 8;
        app.cfg.editor.tabs = "keep".into();
        app.cfg.editor.auto_indent = false;
        app.cfg.editor.show_whitespace = true;
        app.cfg.editor.cursor_after_match = true;
        press(&mut app, "F4");
        let editor = app.mode.editor().expect("the editor opened");
        assert_eq!(editor.tab_width, 8);
        assert!(!editor.expand_tabs, "`keep` leaves a typed tab alone");
        assert!(!editor.auto_indent);
        assert!(editor.show_whitespace);
        assert!(editor.cursor_after_match);
        assert_eq!(
            editor.buffer.line(0),
            "\tone",
            "`keep` leaves the file's own tabs where they are"
        );

        // `all` expands what is in the file when it is opened, and says so — the next save
        // will write that difference.
        let mut app = editing(dir.path(), "mixed.txt");
        app.cfg.editor.tabs = "all".into();
        app.cfg.editor.tab_width = 4;
        press(&mut app, "F4");
        let editor = app.mode.editor().expect("the editor opened");
        assert_eq!(editor.buffer.line(0), "    one");
        assert!(editor.expand_tabs, "`all` expands what is typed too");
        assert!(!app.status.is_empty(), "nothing was said about the change");
    }

    #[test]
    fn ctrl_q_puts_the_next_key_in_as_itself() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.txt"), b"ab\n").unwrap();
        let mut app = editing(dir.path(), "x.txt");
        app.cfg.editor.tabs = "typed".into();
        press(&mut app, "F4");

        press(&mut app, "Ctrl+Q");
        assert!(
            app.mode.editor().expect("the editor").literal_next,
            "Ctrl+Q did not arm the literal insert"
        );
        press(&mut app, "Tab");
        let editor = app.mode.editor().expect("the editor");
        assert!(!editor.literal_next, "it is one key, not a mode");
        assert_eq!(
            editor.buffer.line(0),
            "\tab",
            "a literal tab where a typed one would have been spaces"
        );

        // A key with no character of its own says so rather than inserting nothing.
        press(&mut app, "Ctrl+Q");
        press(&mut app, "F5");
        assert!(app.status_error, "{}", app.status);
        assert_eq!(
            app.mode.editor().expect("the editor").buffer.line(0),
            "\tab"
        );
    }

    #[test]
    fn the_screen_keys_move_the_cursor_without_moving_the_screen() {
        let dir = tempfile::tempdir().unwrap();
        let text: String = (0..200).map(|i| format!("line {i}\n")).collect();
        std::fs::write(dir.path().join("long.txt"), text).unwrap();
        let mut app = editing(dir.path(), "long.txt");
        app.editor_size = (80, 20);
        press(&mut app, "F4");
        // Somewhere in the middle, so there is a screen above and below.
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.top = 100;
            editor.cursor = fcmd_edit::buffer::Position::new(110, 0);
        }
        press(&mut app, "Ctrl+N");
        let editor = app.mode.editor().expect("the editor");
        assert_eq!(editor.cursor.line, 100, "the first line on the screen");
        assert_eq!(editor.top, 100, "and the screen did not move");

        press(&mut app, "Ctrl+E");
        let editor = app.mode.editor().expect("the editor");
        assert_eq!(editor.cursor.line, 119, "the last line on the screen");
    }

    #[test]
    fn one_enter_ends_the_quick_search_and_opens_what_it_found() {
        let mut app = app();
        app.cfg.panel.quick_search = QuickSearch::Plain;
        let was = app.panel().cwd.clone();
        // `d` lands on `docs`, the only directory in the fixture.
        press(&mut app, "d");
        assert!(
            matches!(&app.mode, Mode::Prompt(p) if matches!(p.kind, PromptKind::QuickSearch { .. })),
            "the quick search did not open"
        );

        press(&mut app, "Enter");
        assert!(matches!(app.mode, Mode::Normal), "the search is still open");
        // One key, one result: the panel went into the directory, rather than waiting for a
        // second `Enter` to do what the first looked like it had done.
        assert_eq!(
            app.panel().cwd.display_name(),
            "docs",
            "the first Enter did not open it (was {was})"
        );
    }

    #[test]
    fn the_attributes_dialog_opens_on_what_the_file_carries() {
        let mut app = app();
        app.panel_mut().cursor_to(2); // a.txt
                                      // A read-only file: the box has to open ticked, because `OK` writes every box and an
                                      // empty one would take the bit away from a file nobody meant to change.
        let mut entries = app.panels[0].listing.entries.clone();
        entries[2].flags.insert(fcmd_core::EntryFlags::READONLY);
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(entries),
        });
        app.panel_mut().cursor_to(2);
        app.run(Command::FileAttributes);
        let Mode::Form(form) = &app.mode else {
            panic!("the attributes dialog did not open: {:?}", app.mode)
        };
        assert_eq!(form.text_value("attr.readonly"), "true");
    }

    #[test]
    fn a_marked_letter_takes_its_menu_row_and_the_marker_is_never_drawn() {
        let mut app = app();
        // The Left menu's `&Medium` row: `m` takes it, and the view changes.
        open(&mut app, 0, "menu-medium");
        assert_eq!(app.panels[0].view, ViewMode::Medium);

        let mut by_key = super::tests::app();
        by_key.run(Command::AppMenu);
        press(&mut by_key, "m");
        assert_eq!(by_key.panels[0].view, ViewMode::Medium);
        assert!(by_key.mode.is_normal(), "the menu stayed open");
    }

    #[test]
    fn deleting_one_file_asks_about_that_file_and_not_about_one_items() {
        let mut app = app();
        app.panel_mut().cursor_to(2); // a.txt
        app.run(Command::FileDelete);
        let Mode::Confirm(confirm) = &app.mode else {
            panic!("the delete confirmation did not open: {:?}", app.mode)
        };
        let first = &confirm.lines[0];
        assert!(first.contains("a.txt"), "{first}");
        assert!(
            !first.contains("1 item"),
            "one file is a name, not a count: {first}"
        );
        // And the name is not then repeated underneath, which it was when the question
        // counted instead of naming.
        assert_eq!(
            confirm.lines.iter().filter(|l| l.contains("a.txt")).count(),
            1,
            "{:?}",
            confirm.lines
        );

        // Several are a count, with the names under it.
        let mut many = super::tests::app();
        many.panel_mut().cursor_to(2);
        many.run(Command::SelToggle);
        many.run(Command::SelToggle);
        many.run(Command::FileDelete);
        let Mode::Confirm(confirm) = &many.mode else {
            panic!("no confirmation for the selection")
        };
        assert!(confirm.lines[0].contains('2'), "{:?}", confirm.lines);
    }

    #[test]
    fn the_viewer_settings_dialog_writes_what_it_was_told() {
        let mut app = app();
        app.run(Command::OptViewer);
        let Mode::Form(form) = &app.mode else {
            panic!("the viewer settings did not open: {:?}", app.mode)
        };
        assert_eq!(form.kind, FormKind::ViewerSettings);
        assert_eq!(form.text_value("viewer.status_line"), "true");
        assert_eq!(form.text_value("viewer.scrollbar"), "false");
        assert_eq!(form.text_value("viewer.wrap"), "off");

        // The first row is the top line; `Space` turns it off and `F10` writes it.
        press(&mut app, "Space");
        press(&mut app, "F10");
        assert!(!app.cfg.viewer.status_line);
        assert!(app.cfg.viewer.key_bar, "the other rows were left alone");
    }

    #[test]
    fn a_viewer_opens_the_way_the_configuration_says_and_the_keys_change_that_file_only() {
        let mut app = app();
        app.cfg.viewer.status_line = false;
        app.cfg.viewer.scrollbar = true;
        app.cfg.viewer.wrap = "word".into();
        app.cfg.viewer.tab_width = 4;

        let dir = std::env::temp_dir().join(format!("fcmd-viewcfg-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("readme.txt");
        std::fs::write(&file, b"a\tb\n").unwrap();
        app.open_viewer_on(&file, VPath::local(&file));

        let viewer = app.mode.viewer().expect("the viewer opened");
        assert!(!viewer.show_status_line);
        assert!(viewer.show_scrollbar);
        assert!(viewer.show_key_bar);
        assert_eq!(viewer.tab_width, 4);
        assert_eq!(viewer.wrap, fcmd_view::Wrap::Word);

        // The keys are for the file in front of the reader: the configuration is untouched.
        press(&mut app, "Ctrl+S");
        press(&mut app, "Ctrl+B");
        let viewer = app.mode.viewer().expect("the viewer");
        assert!(!viewer.show_scrollbar);
        assert!(!viewer.show_key_bar);
        assert!(app.cfg.viewer.scrollbar, "a key wrote to the configuration");
        assert!(app.cfg.viewer.key_bar, "a key wrote to the configuration");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_editor_settings_dialog_writes_what_it_was_told() {
        let mut app = app();
        app.run(Command::OptEditor);
        let Mode::Form(form) = &app.mode else {
            panic!("the editor settings did not open: {:?}", app.mode)
        };
        assert_eq!(form.kind, FormKind::EditorSettings);
        assert_eq!(form.text_value("editor.tabs"), "typed");
        assert_eq!(form.text_value("editor.show_whitespace"), "false");

        // The tab policy is three values, not a box: one step along is `all`.
        press(&mut app, "Right");
        press(&mut app, "F10");
        assert_eq!(app.cfg.editor.tabs, "all");
        assert!(app.cfg.editor.expand_on_open());
    }
}
