//! Configuration: discovery, TOML parsing, defaults, and a dump of what is in effect.
//!
//! Unknown keys are warnings, never errors: a config written for a later version still loads.
//! See `docs/spec/07-config-theming.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub mod usermenu;

/// The single configuration file. There is no configuration *directory*: FinalCommander
/// ships as one executable and the only directory it ever looks for is the plugins folder
/// (see `docs/spec/07-config-theming.md` §1).
pub const CONFIG_FILE_NAME: &str = "fcmd.toml";
/// Histories, panel paths and other remembered state: one file, next to the config.
pub const STATE_FILE_NAME: &str = "fcmd-state.toml";
/// Recorded keyboard macros (SPEC 08 §5), beside the configuration.
pub const MACROS_FILE_NAME: &str = "fcmd-macros.toml";
/// The one directory: optional, and the only thing that may live outside the binary.
pub const PLUGINS_DIR_NAME: &str = "plugins";

/// The names these files had while the program was called SupremeCommander, and the new name
/// each one answers to now. A configuration somebody already has is not something to lose
/// over a rename: every file is looked for under its own name first and the old one after,
/// `--paths` shows which one was found, and the startup screen says so once.
pub const LEGACY_NAMES: [(&str, &str); 4] = [
    (CONFIG_FILE_NAME, "scmd.toml"),
    (STATE_FILE_NAME, "scmd-state.toml"),
    (MACROS_FILE_NAME, "scmd-macros.toml"),
    (usermenu::MENU_FILE_NAME, "scmd-menu.toml"),
];

/// `dir/name` unless it is not there and `dir/legacy` is, which is the file from before the
/// rename and the one to go on using.
fn kept(dir: &Path, name: &str, legacy: &str) -> PathBuf {
    let fresh = dir.join(name);
    if fresh.exists() {
        return fresh;
    }
    let old = dir.join(legacy);
    if old.exists() {
        old
    } else {
        fresh
    }
}

/// The old name for a file we read, if that is the one on disk.
fn legacy_of(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    LEGACY_NAMES
        .iter()
        .find(|(_, legacy)| *legacy == name)
        .map(|(fresh, _)| *fresh)
}

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

/// Where this run reads and writes. Three paths, no trees.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Paths {
    /// The directory the executable sits in — where portable mode and plugins are looked for.
    pub exe_dir: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    /// Recorded macros. Beside the configuration, because they are configuration one made by
    /// pressing keys rather than by typing TOML.
    pub macros_file: PathBuf,
    /// The user menu `F2` opens (SPEC 06 §4). A file of its own, because a directory may
    /// carry one too and a block inside `fcmd.toml` could not travel with a project.
    pub menu_file: PathBuf,
    pub plugins_dir: PathBuf,
    /// True when the configuration sits next to the executable.
    pub portable: bool,
}

impl Paths {
    /// `--config <file>` wins. Otherwise, if `fcmd.toml` sits next to the executable we are
    /// portable (a USB stick, a release zip); failing that, the per-OS config location — as a
    /// single file, `<config>/fcmd.toml`, not a directory of our own.
    ///
    /// The plugins directory is `--plugins <dir>`, then `$FCMD_PLUGINS`, then `plugins/` next to
    /// the executable, then `fcmd-plugins/` beside the config file.
    pub fn discover(config: Option<&Path>, plugins: Option<&Path>) -> Paths {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));

        let portable_config = kept(&exe_dir, CONFIG_FILE_NAME, LEGACY_NAMES[0].1);
        let (config_file, portable) = match config {
            Some(explicit) => (explicit.to_path_buf(), false),
            None if portable_config.exists() => (portable_config, true),
            None => (
                kept(
                    &dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")),
                    CONFIG_FILE_NAME,
                    LEGACY_NAMES[0].1,
                ),
                false,
            ),
        };

        let config_parent = config_file
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        let state_file = kept(&config_parent, STATE_FILE_NAME, LEGACY_NAMES[1].1);
        let macros_file = kept(&config_parent, MACROS_FILE_NAME, LEGACY_NAMES[2].1);
        let menu_file = kept(&config_parent, usermenu::MENU_FILE_NAME, LEGACY_NAMES[3].1);

        let plugins_dir = plugins
            .map(Path::to_path_buf)
            .or_else(|| std::env::var_os("FCMD_PLUGINS").map(PathBuf::from))
            .unwrap_or_else(|| {
                let beside_exe = exe_dir.join(PLUGINS_DIR_NAME);
                if beside_exe.is_dir() {
                    beside_exe
                } else {
                    let fresh = config_parent.join("fcmd-plugins");
                    let old = config_parent.join("scmd-plugins");
                    if !fresh.is_dir() && old.is_dir() {
                        old
                    } else {
                        fresh
                    }
                }
            });

        Paths {
            exe_dir,
            config_file,
            state_file,
            macros_file,
            menu_file,
            plugins_dir,
            portable,
        }
    }

    /// The files being read under the name they had before the rename, each with the name it
    /// should have. Empty for anyone who started with FinalCommander.
    pub fn legacy(&self) -> Vec<(String, &'static str)> {
        [
            &self.config_file,
            &self.state_file,
            &self.macros_file,
            &self.menu_file,
        ]
        .into_iter()
        .filter(|p| p.exists())
        .filter_map(|p| legacy_of(p).map(|fresh| (p.display().to_string(), fresh)))
        .collect()
    }

    /// Human-readable summary for `fcmd --paths`: the first question anyone asks.
    pub fn describe(&self) -> String {
        format!(
            "config   {}{}\nstate    {}\nmacros   {}{}\nmenu     {}{}\nplugins  {}{}\n",
            self.config_file.display(),
            if self.config_file.exists() {
                ""
            } else {
                "   (not present; defaults are compiled in)"
            },
            self.state_file.display(),
            self.macros_file.display(),
            if self.macros_file.exists() {
                ""
            } else {
                "   (not present; nothing recorded yet)"
            },
            self.menu_file.display(),
            if self.menu_file.exists() {
                ""
            } else {
                "   (not present; F2 says so and where to put it)"
            },
            self.plugins_dir.display(),
            if self.plugins_dir.is_dir() {
                ""
            } else {
                "   (not present; no plugins loaded)"
            }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct General {
    pub confirm_exit: bool,
    pub confirm_delete: bool,
    pub confirm_overwrite: bool,
    /// With nothing in `[[assoc]]` to say otherwise, `Enter` on a file that is not a program
    /// hands it to whatever the desktop has registered for it — `xdg-open`, `open`, `start`.
    ///
    /// On, because that is what `Enter` means in every file manager, and because the honest
    /// alternative is a key that says "not implemented". Off, `Enter` says there is no
    /// association instead, which is what somebody working over a connection with no desktop
    /// at the other end will want.
    pub desktop_open: bool,
    /// `F8` hands a file to the system's own wastebasket rather than unlinking it. On by
    /// default, as it is in FAR and in Total Commander: a wastebasket is a safety net, and
    /// the one key everybody presses by accident is the one that should land in it.
    /// `Shift+Del` deletes outright whatever this says.
    pub use_trash: bool,
}
// There is no `update_check` here, and that is the point: a setting nobody reads is a setting
// that lies, and this one was a promise wearing a setting's clothes. The promise is kept in
// the code instead — the program opens no connection the user did not ask for, and the only
// outbound code in the tree is the SFTP client behind `Alt+F1`. If a release check is ever
// wanted it will be a command one runs, not a flag that phones home at startup.

impl Default for General {
    fn default() -> Self {
        General {
            confirm_exit: false,
            confirm_delete: true,
            confirm_overwrite: true,
            desktop_open: true,
            use_trash: true,
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
    /// What breaks the ties the sort key leaves, in order: `["size", "name"]` sorts by size
    /// and settles two files of the same size by name (SPEC 02 §4). At most three; the name
    /// has the last word in any case.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub sort_chain: Vec<String>,
    pub reverse: bool,
    pub dirs_first: bool,
    pub selected_first: bool,
    pub natural_sort: bool,
    /// On by default, and deliberately: a file manager that quietly leaves files out is
    /// lying about the directory. `.git` and `.env` are files like any other.
    pub show_hidden: bool,
    pub allow_reverse_sort: bool,
    /// The `Name | Size | Date | Time` row under the panel frame (FAR has it on by default).
    pub column_titles: bool,
    /// The item under the cursor, spelled out in the panel's own bottom row (FAR's
    /// `Show status line`), under a rule that also carries the selected totals.
    pub status_line: bool,
    /// A bar in the right frame showing where the view sits in a long listing.
    pub scrollbar: bool,
    /// Free space on the device, in the bottom frame.
    pub free_space: bool,
    /// Re-read a local directory when something in it changes, without `Ctrl+R`. On by
    /// default: a panel that is showing a directory as it was ten minutes ago is lying.
    pub auto_refresh: bool,
    /// How much of the width the left panel takes, as a percentage. `Ctrl+Left` and
    /// `Ctrl+Right` move it; 50 is the even split FAR starts from.
    pub split: u16,
    /// Rows taken off the bottom of both panels, which the user screen gets instead.
    /// `Ctrl+Up` and `Ctrl+Down` move it, and it is FAR's way of watching a command's output
    /// without leaving the panels.
    pub height_cut: u16,
    pub quick_search: QuickSearch,
    /// Read the directory's own `descript.ion` and show what it says (SPEC 02 §9). One small
    /// file per directory read, which is what FAR does; off for somebody who keeps no
    /// descriptions and would rather the file were never opened.
    pub descriptions: bool,
}

impl Default for PanelCfg {
    fn default() -> Self {
        PanelCfg {
            view: "full".into(),
            sort: "name".into(),
            sort_chain: Vec::new(),
            reverse: false,
            dirs_first: true,
            selected_first: false,
            natural_sort: true,
            show_hidden: true,
            allow_reverse_sort: true,
            column_titles: true,
            status_line: true,
            scrollbar: true,
            free_space: true,
            auto_refresh: true,
            split: 50,
            height_cut: 0,
            quick_search: QuickSearch::Alt,
            descriptions: true,
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
    /// Colour what the viewer and the editor recognise as a language or a markup.
    pub syntax: bool,
    /// Open a Markdown file laid out to be read rather than as its source. `Shift+F2` in the
    /// viewer switches either way whatever this says; this is only what the first screen is.
    pub markdown: bool,
    /// The command line under the panels. Without it, typing goes to quick search.
    pub command_line: bool,
    /// Show what a command prints while it runs, full screen, the way the console does in
    /// FAR — and go back to the panels when it ends. `Esc` leaves early and the command keeps
    /// running. Off means the output only goes to the screen `Ctrl+O` opens, which is still
    /// filled line by line rather than all at the end.
    pub command_screen: bool,
    /// Let the mouse move the cursor, press the key bar and open the menus.
    ///
    /// Off by default, and that is a decision rather than an oversight: while a terminal is
    /// reporting the mouse it stops handling it, so selecting text with the mouse and the
    /// middle-click paste — the user's own tools, which this program cannot replace — go
    /// away. Somebody who wants the mouse says so once; everybody else keeps their terminal.
    pub mouse: bool,
    /// How many command lines to keep between sessions.
    pub command_history: usize,
    /// How many read-or-edited files to keep between sessions (`Alt+F11`).
    pub file_history: usize,
    /// How many days a history line nobody has pinned is kept after it was last used.
    ///
    /// FAR's own lifetime, and FAR's own default of ninety days: a history is worth having
    /// because one recognises what is in it, and a command typed once a year ago is noise
    /// between the ones typed this week. `0` keeps them for ever, and a pinned line is never
    /// counted — that is what pinning it says.
    pub history_lifetime: u32,
    /// Open a tabular file (`.csv`, `.tsv`, `.jsonl`, `.dbf`) as a table rather than as its
    /// source.
    /// `Shift+F3` in the viewer switches either way whatever this says; this is only what the
    /// first screen is.
    pub table: bool,
    /// Ask the terminal to put what is copied on the system clipboard (`OSC 52`).
    ///
    /// On by default because it is the one way to reach the clipboard that also works over
    /// `ssh`, and a terminal that does not know the sequence ignores it. It is a setting
    /// because a terminal that does know it is being asked by a program running on another
    /// machine, which somebody may not want.
    pub clipboard: bool,
}

impl Default for UiCfg {
    fn default() -> Self {
        UiCfg {
            theme: "commander".into(),
            keymap: "far".into(),
            language: "auto".into(),
            key_bar: true,
            status_bar: true,
            syntax: true,
            markdown: true,
            table: true,
            command_line: true,
            command_screen: true,
            mouse: false,
            command_history: 512,
            file_history: 512,
            history_lifetime: 90,
            clipboard: true,
        }
    }
}

/// `[net]`: the netbox's own settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NetCfg {
    /// Where host keys are recorded. Empty means `~/.ssh/known_hosts`.
    pub known_hosts: String,
    /// Keepalive for a connection, in seconds. 0 turns it off.
    pub keepalive_secs: u64,
    /// The largest remote file the viewer and the editor will fetch into a temporary file.
    pub fetch_limit_mb: u64,
}

impl Default for NetCfg {
    fn default() -> Self {
        NetCfg {
            known_hosts: String::new(),
            keepalive_secs: 30,
            fetch_limit_mb: 512,
        }
    }
}

/// `[[s3]]` blocks: the object storage services a path may name.
///
/// There is deliberately no place here for an access key. A secret key is a password, and a
/// password in `fcmd.toml` is a password in a backup, in a dotfiles repository and in a screen
/// share. The two places the rest of the world already keeps these are read instead: the
/// `AWS_*` environment variables and `~/.aws/credentials`, which every AWS tool shares.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct S3Cfg {
    /// What a path calls this service: `s3://work@bucket/` names `work`. The service with an
    /// empty name is the default, reached by `s3://bucket/`.
    pub name: String,
    /// `https://minio.lan:9000`, `http://127.0.0.1:9000`, or empty for AWS itself.
    pub endpoint: String,
    /// The region the signature is scoped to. Empty means `AWS_REGION`, then `us-east-1`.
    pub region: String,
    /// Put the bucket in the path rather than in front of the host. Unset means the sensible
    /// default for the endpoint: AWS is addressed by host, everything else by path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_style: Option<bool>,
    /// `env`, `aws`, `aws:<profile>`, or `anonymous` for a bucket with a public read policy.
    pub credentials: String,
    /// How many rows a listing with no delimiter — every key under a prefix, however deep —
    /// will gather before it stops and says that it stopped. Zero means no limit, which on a
    /// bucket with millions of objects in it means a listing that takes minutes and a panel
    /// that holds all of them.
    pub listing_cap: usize,
}

impl Default for S3Cfg {
    fn default() -> S3Cfg {
        S3Cfg {
            name: String::new(),
            endpoint: String::new(),
            region: String::new(),
            path_style: None,
            credentials: "env".into(),
            listing_cap: 100_000,
        }
    }
}

/// `[compare]`: what counts as the same file when the two panels are compared (SPEC 03 §11).
///
/// FAR has this as a plugin (`AdvCmpEx`) with a dialog of its own, and the options are the
/// reason: whether the time matters, how close is close enough, whether the bytes are read at
/// all and how many of them. The dialog writes its answers here, so the next comparison is the
/// one that was set up rather than the default again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CompareCfg {
    /// Match the names with their case, rather than `Readme.md` pairing with `README.MD`.
    pub case: bool,
    /// A different size is a difference.
    pub size: bool,
    /// A different time is a difference.
    pub time: bool,
    /// How many seconds apart two times may be and still count as the same moment. Two is
    /// the useful one: a FAT filesystem keeps the time to the nearest two seconds, so a file
    /// copied onto a memory stick comes back one second different and is not changed.
    pub seconds: u64,
    /// Read the bytes, rather than trusting the size and the time.
    pub contents: bool,
    /// Read them only for a pair whose times differ, which is what makes a comparison of a
    /// large tree finish: files of the same size at the same moment are taken as the same.
    pub when_time_differs: bool,
    /// Read only the first so many kibibytes of each file. Zero means the whole of it.
    pub fragment_kb: u64,
    /// Two files that differ only in how they end their lines are the same file — the same
    /// checkout on Windows and on Linux differs in every line and in nothing that matters.
    ///
    /// Off by default, because the comparison counts such a pair separately and *says* so in
    /// its sentence: the reader finds out that the only difference is the line endings
    /// without having had to decide in advance that it would not matter.
    pub ignore_eol: bool,
    /// Compare only what is selected, rather than the whole of both listings.
    pub selected_only: bool,
    /// Stop at the first difference, for the question "are these two trees the same at all".
    pub stop_on_first: bool,
    /// Select only the copy that is newer, rather than every copy that differs: what one
    /// wants before pressing `F5`. Off selects both sides of every difference, which is what
    /// one wants when looking for *what* differs rather than copying.
    pub newer_only: bool,
    /// Compare the whole of both trees rather than the two listings. The answer cannot be
    /// shown as a selection in two single-directory panels, so it comes back as a panel of
    /// what differs (SPEC 03 §11b).
    pub subdirs: bool,
    /// How deep to go when it does: 1 is the two directories themselves, 0 is all the way
    /// down.
    pub depth: u64,
}

impl Default for CompareCfg {
    fn default() -> Self {
        CompareCfg {
            case: false,
            size: true,
            time: true,
            seconds: 0,
            contents: false,
            when_time_differs: false,
            fragment_kb: 0,
            ignore_eol: false,
            selected_only: false,
            stop_on_first: false,
            // On, which is what the comparison has always done and what one wants before
            // pressing `F5`: the copy that is newer is the one to send across.
            newer_only: true,
            subdirs: false,
            depth: 0,
        }
    }
}

/// `[cmdline]`: the command line itself (SPEC 06 §2b).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CmdLineCfg {
    /// What is written to the left of what one types.
    ///
    /// The letters are FAR's — `$p` the path, `$w` its last part, `$n` the drive or the
    /// server, `$g` `>`, `$l` `<`, `$a` `&`, `$b` `|`, `$c` `(`, `$f` `)`, `$q` `=`, `$s` a
    /// space, `$d` the date, `$t` the time, `$@` a marker for an elevated session, `$h` a
    /// backspace, `$+` one plus per directory `pushd` has put away, `$$` a `$` — with two of
    /// our own for the repository one is in: `$r` is
    /// ` (branch)` inside a repository and nothing outside one, and `%GITBRANCH%` is the
    /// bare branch name, spelled the way the plugin everybody installs spells it. Empty
    /// means the default.
    pub prompt: String,
    /// What the line completes from while one types (SPEC 06 §7.4).
    pub complete: CompleteCfg,
}

/// Autocomplete on the command line: four sources, each one on, off, or only when asked.
///
/// FAR has the same four and the same three states, and the reason each is separate is that
/// they cost different things: the panel's own names are free, the history is free, the
/// environment is free, and the `PATH` is a directory read per entry in it — which is why
/// that one waits to be asked by default.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CompleteCfg {
    /// The names in the directory the panel is showing, and under a path one has typed.
    /// `on`, `off`, or `asked` for only when `Ctrl+Space` asks.
    pub files: String,
    /// The command lines that have run: this one completes the **whole line**, as FAR's does.
    pub history: String,
    /// The executables on the `PATH`. `asked` by default: it is gathered once, and gathering
    /// it is a read of every directory on the `PATH`.
    pub path: String,
    /// The environment's variable names, for a word that starts with `$` or `%`.
    pub env: String,
    /// How many answers the list holds at once.
    pub rows: u16,
    /// How many answers are kept at all, however many there are.
    pub limit: u16,
}

impl Default for CompleteCfg {
    fn default() -> Self {
        CompleteCfg {
            files: "on".into(),
            history: "on".into(),
            path: "asked".into(),
            env: "on".into(),
            rows: 8,
            limit: 200,
        }
    }
}

impl Default for CmdLineCfg {
    fn default() -> Self {
        CmdLineCfg {
            prompt: "$p$r$g ".to_string(),
            complete: CompleteCfg::default(),
        }
    }
}

/// `[drives]`: which columns the change-drive dialog shows, and which drives (SPEC 10 §3.4).
///
/// FAR keeps the same settings in one `DriveMenuMode` word and turns them on and off with
/// `Ctrl` and a digit from inside the dialog; this is that word, written out. Every field is
/// a column or a kind of drive, in the order of the digit that toggles it, and the dialog
/// writes the change straight back here, so what one chose is still chosen tomorrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DrivesCfg {
    /// `Ctrl+1`: the kind of thing it is — disk, stick, share, memory.
    pub show_type: bool,
    /// `Ctrl+2`: what it is a mount of — the device on Unix, the share behind a mapped
    /// letter on Windows. FAR calls it the associated path and leaves it off.
    pub show_device: bool,
    /// `Ctrl+3`: the volume's own name.
    pub show_label: bool,
    /// `Ctrl+4`: the filesystem.
    pub show_fs: bool,
    /// `Ctrl+5`: the two sizes, total and free.
    pub show_size: bool,
    /// `Ctrl+6`: list the removable drives at all.
    pub show_removable: bool,
    /// `Ctrl+7`: list the panels that are not drives — the sessions, the processes, the
    /// temporary panel.
    pub show_panels: bool,
    /// `Ctrl+8`: list the optical drives at all.
    pub show_optical: bool,
    /// `Ctrl+9`: list the network shares at all.
    pub show_network: bool,
}

impl Default for DrivesCfg {
    fn default() -> Self {
        DrivesCfg {
            show_type: true,
            // Off, as it is in FAR: on a laptop with one disk the device column says
            // `/dev/nvme0n1p2` and nothing else.
            show_device: false,
            show_label: true,
            show_fs: true,
            show_size: true,
            show_removable: true,
            show_panels: true,
            show_optical: true,
            show_network: true,
        }
    }
}

/// `[arc]`: the guards every archive read goes through (SPEC 04 §4).
///
/// The terminal (`Ctrl+Alt+E`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TermCfg {
    /// What to run. Empty means the shell the session already uses — `$SHELL` on Unix,
    /// `ComSpec` on Windows — because that is where somebody's aliases and prompt live.
    ///
    /// It is a command line, split the way a shell splits one, so `bash -l` works and a path
    /// with a space in it can be quoted. Nothing expands it: there is no shell in between,
    /// since the thing being started is the shell.
    pub shell: String,
    /// How many lines of what has scrolled off the top to keep.
    pub scrollback: usize,
}

impl Default for TermCfg {
    fn default() -> Self {
        TermCfg {
            shell: String::new(),
            scrollback: fcmd_term::SCROLLBACK,
        }
    }
}

/// They are configuration because a zip bomb and a legitimate archive of a disk image look
/// alike from the outside, and only the person at the keyboard knows which one this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ArcCfg {
    /// How many entries one archive may list.
    pub max_entries: usize,
    /// The total unpacked size an archive may add up to, in megabytes.
    pub max_total_mb: u64,
    /// Unpacked over packed. Text compresses 10:1 and a disk image far better, so the default
    /// is generous; a thousandfold archive is a bomb or a curiosity.
    pub max_ratio: u64,
    /// How many containers deep a path may go.
    pub max_depth: usize,
    /// How large an archive that is not a file on this disk may be, in megabytes, before it is
    /// refused rather than copied here to be read — one inside another archive, one on a
    /// server. Every decoder reads a file and seeks in it, so such a container is copied to
    /// the temporary directory first, and that copy is paid for in disk (SPEC 04 §4).
    pub max_spill_mb: u64,
}

impl Default for ArcCfg {
    fn default() -> Self {
        let limits = fcmd_arc::Limits::default();
        ArcCfg {
            max_entries: limits.entries,
            max_total_mb: limits.total_bytes >> 20,
            max_ratio: limits.ratio,
            max_depth: limits.depth,
            max_spill_mb: limits.spill_bytes >> 20,
        }
    }
}

impl ArcCfg {
    pub fn limits(&self) -> fcmd_arc::Limits {
        fcmd_arc::Limits {
            entries: self.max_entries,
            total_bytes: self.max_total_mb.saturating_mul(1 << 20),
            ratio: self.max_ratio,
            depth: self.max_depth,
            spill_bytes: self.max_spill_mb.saturating_mul(1 << 20),
        }
    }
}

/// `[editor]`: how the editor behaves. The viewer's own settings are still to come.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct EditCfg {
    /// How wide a tab is drawn, and how far one indent moves a line.
    pub tab_width: usize,
    /// What happens to tabs, which is three policies and not a boolean (FAR has the same
    /// three): `keep` leaves every tab alone, `typed` turns a newly typed tab into spaces and
    /// leaves the ones already in the file, and `all` also expands the ones in the file when
    /// it is opened — which changes the file on the first save, and is why it is not default.
    pub tabs: String,
    /// A new line starts with the indent of the one above it.
    pub auto_indent: bool,
    /// Draw a space as `·` and a tab as `→`, so trailing space and a mixed indent show.
    pub show_whitespace: bool,
    /// Where the cursor lands on a match: at its start, or just after it. FAR offers both,
    /// because the first is for reading and the second for typing over what was found.
    pub cursor_after_match: bool,
}

impl Default for EditCfg {
    fn default() -> Self {
        EditCfg {
            tab_width: 4,
            tabs: "typed".into(),
            auto_indent: true,
            show_whitespace: false,
            cursor_after_match: false,
        }
    }
}

/// `[viewer]`: how the viewer behaves and how much of its chrome is drawn (SPEC 05 §1).
///
/// The three chrome rows are settings *and* keys (`Ctrl+B`, `Ctrl+Shift+B`, `Ctrl+S`), which
/// is FAR's arrangement: the setting is what every file opens with, and the key takes a row
/// away for the one long line or wide dump that needs the whole window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ViewCfg {
    /// Is the key bar drawn under the content? `Ctrl+B` toggles it.
    pub key_bar: bool,
    /// Is the top line — the name and the position — drawn? `Ctrl+Shift+B` toggles it.
    pub status_line: bool,
    /// Is a scrollbar drawn down the right edge? `Ctrl+S` toggles it. Off by default, as it
    /// is in FAR, and because the status line already says the position as a percentage.
    pub scrollbar: bool,
    /// How a file opens: `off`, `char` or `word`. FAR opens without wrapping and `F2`
    /// changes it; this is what that starts from.
    pub wrap: String,
    /// What a tab expands to. Eight, which is what a terminal and `cat` do — a file that
    /// lined something up with tabs was written against that number.
    pub tab_width: usize,
}

impl Default for ViewCfg {
    fn default() -> Self {
        ViewCfg {
            key_bar: true,
            status_line: true,
            scrollbar: false,
            wrap: "off".into(),
            // The same eight `fcmd_view::TAB_WIDTH` has. This crate does not depend on the
            // viewer, so the number is written twice and a test compares the two.
            tab_width: 8,
        }
    }
}

impl ViewCfg {
    /// The wrap a file opens with. Anything the setting does not name is `off`, because a
    /// viewer that wrapped when it was told something it did not understand would be worse
    /// than one that did not wrap.
    pub fn wrap_kind(&self) -> &str {
        match self.wrap.trim() {
            "char" | "on" | "true" => "char",
            "word" => "word",
            _ => "off",
        }
    }
}

impl EditCfg {
    /// Should a tab typed now become spaces?
    pub fn expand_typed(&self) -> bool {
        matches!(self.tabs.trim(), "typed" | "all")
    }

    /// Should the tabs already in the file become spaces when it is opened?
    pub fn expand_on_open(&self) -> bool {
        self.tabs.trim() == "all"
    }
}

/// A `[[site]]` block: one entry of the netbox's site list.
///
/// Secrets are never here. A site says *how* to authenticate (`agent`, `key:<path>`, `ask`)
/// and the password or passphrase is asked for when the connection needs it
/// (`docs/spec/04-vfs.md` §6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SiteCfg {
    /// Short name, shown in the site list and usable from the command line.
    #[serde(default)]
    pub id: String,
    /// `sftp`, `ftp` or `ftps`. The port it implies — 22 or 21 — is what a site with no
    /// `port` of its own gets.
    #[serde(default = "default_scheme")]
    pub scheme: String,
    pub host: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keepalive_secs: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compress: Option<bool>,
}

/// A blank site, as the session editor starts from: the scheme is the only field with a
/// sensible value, and `serde` fills the same one in for a `[[site]]` that leaves it out.
impl Default for SiteCfg {
    fn default() -> SiteCfg {
        SiteCfg {
            id: String::new(),
            scheme: default_scheme(),
            host: String::new(),
            port: None,
            user: None,
            auth: None,
            start_dir: None,
            group: None,
            keepalive_secs: None,
            compress: None,
        }
    }
}

fn default_scheme() -> String {
    "sftp".to_string()
}

/// A `[[decompile]]` rule: an external tool that turns a compiled file into readable text.
///
/// FinalCommander disassembles x86 code and prints WebAssembly itself; real decompilation is
/// a research project, so it calls the specialists instead of reimplementing them.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DecompileRule {
    /// Shown in the viewer's status line.
    pub name: String,
    /// `*.class`, `*.pyc`, or an exact file name.
    #[serde(rename = "match")]
    pub mask: String,
    /// The command. `{file}` is replaced with the path; it is executed directly, never
    /// through a shell.
    pub cmd: String,
}

/// Does `mask` apply to `name`? `*.ext` compares the extension, anything else the whole file
/// name, both without regard to case.
///
/// One function for both the `[[decompile]]` and the `[[assoc]]` blocks, because it is one
/// rule and the two had better agree about it. A name with no dot in it has no extension, so
/// `*.md` does not match a file called `md` — which the first spelling of this got wrong, and
/// which a test now says.
pub fn mask_matches(mask: &str, name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let mask = mask.trim().to_ascii_lowercase();
    if mask.is_empty() {
        return false;
    }
    match mask.strip_prefix("*.") {
        Some(ext) => match name.rsplit_once('.') {
            Some((stem, found)) => !stem.is_empty() && found == ext,
            None => false,
        },
        None => name == mask,
    }
}

impl DecompileRule {
    /// Does this rule apply to `name`? See [`mask_matches`].
    pub fn matches(&self, name: &str) -> bool {
        mask_matches(&self.mask, name)
    }

    /// The program the rule runs, for an availability check.
    pub fn program(&self) -> &str {
        self.cmd.split_whitespace().next().unwrap_or("")
    }
}

/// `[[assoc]]` blocks: what to run for a kind of file (SPEC 06 §5).
///
/// A mask and up to three commands, each optional: what `Enter` does, what `F3` does instead
/// of the internal viewer, what `F4` does instead of the internal editor. A command goes
/// through the same substitutions the command line and the user menu use, so `{file}` is the
/// path and `{file}` is the path in quotes.
/// `[[highlight]]` blocks: which rows the panel paints differently, and how (SPEC 02 §5).
///
/// A group is a condition and a colour. The condition is written in the words a filter uses,
/// because it *is* a filter — the same masks, the same size and age spellings, the same
/// attribute letters — and what is left out is not asked about. The first group that matches
/// a row decides, so the order in the file is the order of precedence; the built-in groups
/// FAR ships with come after them all.
///
/// ```toml
/// [[highlight]]
/// name = "Rust"
/// mask = "*.rs"
/// fg = "#d7875f"
///
/// [[highlight]]
/// name = "Fresh"
/// age = "<=1"
/// fg = "#87d787"
/// ```
/// One `[[sort_group]]` block: a band of the listing, in the order the blocks are written.
///
/// ```toml
/// [[sort_group]]
/// name = "sources"
/// mask = "*.rs,*.toml"
///
/// [[sort_group]]
/// name = "build output"
/// mask = "*.o,*.rlib,*.d"
/// place = "back"
/// ```
///
/// The bands in front keep the order they are written in, and so do the ones behind; the
/// files that match nothing sit between them. `Shift+F11` turns the whole thing on and off.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SortGroupCfg {
    /// What to call the band, for the message that says which one a file is in.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A mask list, with the groups a filter knows: `*.rs`, `<arc>`, `*.log|*.tmp`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mask: String,
    /// `front` (the default) or `back`: which side of the ungrouped files this band sits on.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub place: String,
}

/// One `[[mask_group]]` block: a named mask list that every mask field can use as `<name>`.
///
/// ```toml
/// [[mask_group]]
/// name = "images"
/// mask = "*.png,*.jpg,*.jpeg,*.gif,*.webp,*.svg"
/// ```
///
/// The three FAR ships with — `<arc>`, `<temp>`, `<exec>` — are there without being written
/// down, and a block of the same name replaces one of them rather than adding to it: that is
/// how somebody who keeps `.cbz` files teaches `<arc>` about them.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MaskGroupCfg {
    /// What it is called between the angle brackets. Case does not matter.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A mask list, which may name other groups: groups nest.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mask: String,
}

/// One column of a `[[column_set]]` block.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnCfg {
    /// What the column holds: `name`, `ext`, `size`, `allocated`, `mtime`, `btime`, `atime`,
    /// `attrs`, `owner`, `group`, `links`, `descr`.
    #[serde(rename = "type", skip_serializing_if = "String::is_empty")]
    pub kind: String,
    /// `10`, `"20%"`, `"*"` to take what is left, or `0` for the column's own natural width —
    /// FAR's own spelling, which is why it is a number *or* a string.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<toml::Value>,
    /// `left` or `right`, where the column's own default is not what is wanted.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub align: String,
    /// `human` or `bytes` on a size; a `strftime` string on a date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fmt: Option<String>,
}

/// A `[[column_set]]` block: the columns one view mode draws.
///
/// `id` is the digit the view is on, so `id = 3` replaces what `Ctrl+3` draws. A block with no
/// column, or whose columns cannot be read, is reported on the startup screen and the built-in
/// mode is drawn instead — a panel that drew nothing would be a worse answer than a panel that
/// looks as it always did.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColumnSetCfg {
    pub id: u8,
    /// What to call it, for the message when something in it does not read.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub columns: Vec<ColumnCfg>,
    /// FAR's own pair of strings instead of the list above, for a mode somebody already has
    /// in their FAR configuration: `far_types = "N,SC,DM"` with `far_widths = "0,9,16"`
    /// (SPEC 02 §3b). What we have no column for is reported rather than dropped in silence.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub far_types: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub far_widths: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HighlightCfg {
    /// What to call the group — in a message when it does not parse, and in the dialog that
    /// will one day list them.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// A mask list, with the groups a filter knows: `*.rs`, `<arc>`, `*.log|*.tmp`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub mask: String,
    /// `file`, `dir`, or empty for both.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub kind: String,
    /// A size window, in the spelling the filter dialog takes: `1M-10M`, `-4K` up to four
    /// kibibytes, `1G-` a gibibyte and over. A lone `1M` is a lower bound.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub size: String,
    /// How old, in whole days and in the dialog's spelling: `<=1` for today and yesterday,
    /// `>=30` for a month old and older, `7-30` for a window between the two.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub age: String,
    /// Which timestamp `age` is about: `write` (the default), `create`, `access`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub date_field: String,
    /// The attribute letters: `h` hidden, `r` read-only, `x` executable, `s` system,
    /// `b` broken link — each one optionally after a `-` for "must not".
    #[serde(skip_serializing_if = "String::is_empty")]
    pub attrs: String,
    /// The ink: `#rrggbb`, a colour name, or the name of a theme key such as `file.archive`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub fg: String,
    /// The background, written the same way. Left out, the row keeps the panel's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bg: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssocCfg {
    /// `*.md`, or an exact file name like `Makefile`.
    #[serde(rename = "match")]
    pub mask: String,
    /// Shown in the *Open with* list; the mask is used when this is empty.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub name: String,
    /// What `Enter` runs.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub open: String,
    /// What `F3` runs instead of the internal viewer.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub view: String,
    /// What `F4` runs instead of the internal editor.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub edit: String,
}

impl AssocCfg {
    /// Does this block apply to `name`? See [`mask_matches`], which the `[[decompile]]`
    /// blocks share: it is one rule and the two had better agree about it.
    pub fn matches(&self, name: &str) -> bool {
        mask_matches(&self.mask, name)
    }

    /// What the list calls it.
    pub fn label(&self) -> &str {
        if self.name.trim().is_empty() {
            &self.mask
        } else {
            &self.name
        }
    }
}

/// The command that hands a file to whatever the desktop has registered for it.
///
/// Every platform has exactly one of these and they all take the path as their argument, so
/// there is nothing to configure and nothing to guess: `xdg-open` on Unix (the freedesktop
/// one, which is also what the BSDs ship), `open` on macOS, and `cmd /c start` on Windows,
/// where the shell builtin is the opener and `start`'s first quoted argument is a window
/// title rather than a file — hence the empty one.
pub fn desktop_open_command() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "open {file}"
    }
    #[cfg(target_os = "windows")]
    {
        "cmd /c start \"\" {file}"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "xdg-open {file}"
    }
}

/// Rules compiled in, used when the config file does not override them. Each is tried only if
/// its program is actually installed.
pub fn default_decompile_rules() -> Vec<DecompileRule> {
    let rule = |name: &str, mask: &str, cmd: &str| DecompileRule {
        name: name.to_string(),
        mask: mask.to_string(),
        cmd: cmd.to_string(),
    };
    vec![
        rule("javap", "*.class", "javap -c -p -constants {file}"),
        rule("ilspycmd", "*.dll", "ilspycmd {file}"),
        rule("python -m dis", "*.pyc", "python3 -m dis {file}"),
        rule("wasm2wat", "*.wasm", "wasm2wat {file}"),
        rule("objdump", "*.o", "objdump -d -C {file}"),
        rule("objdump", "*.so", "objdump -d -C {file}"),
        rule("objdump", "*.a", "objdump -d -C {file}"),
        rule(
            "retdec",
            "*.exe",
            "retdec-decompiler --cleanup -o /dev/stdout {file}",
        ),
    ]
}

/// Is `program` on the PATH (or an existing path of its own)?
pub fn command_exists(program: &str) -> bool {
    if program.is_empty() {
        return false;
    }
    let candidate = Path::new(program);
    if candidate.components().count() > 1 {
        return candidate.is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| {
        let direct = dir.join(program);
        if direct.is_file() {
            return true;
        }
        // Windows: the extension is implied.
        ["exe", "bat", "cmd", "com"]
            .iter()
            .any(|ext| dir.join(format!("{program}.{ext}")).is_file())
    })
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub general: General,
    pub panel: PanelCfg,
    pub ui: UiCfg,
    /// `[[theme]]` blocks: themes live in the config file, not in a themes directory.
    #[serde(rename = "theme")]
    pub themes: Vec<toml::Table>,
    /// `[[keymap]]` blocks: same reasoning.
    #[serde(rename = "keymap")]
    pub keymaps: Vec<toml::Table>,
    /// `[[decompile]]` blocks: external tools the viewer may call.
    #[serde(rename = "decompile")]
    pub decompile: Vec<DecompileRule>,
    pub net: NetCfg,
    pub cmdline: CmdLineCfg,
    pub compare: CompareCfg,
    pub drives: DrivesCfg,
    pub arc: ArcCfg,
    pub term: TermCfg,
    pub editor: EditCfg,
    pub viewer: ViewCfg,
    /// `[[site]]` blocks: the netbox's site list.
    #[serde(rename = "site")]
    pub sites: Vec<SiteCfg>,
    /// `[[s3]]` blocks: object storage services.
    #[serde(rename = "s3")]
    pub s3: Vec<S3Cfg>,
    /// `[[assoc]]` blocks: what to run for a kind of file (SPEC 06 §5).
    #[serde(rename = "assoc")]
    pub assocs: Vec<AssocCfg>,
    /// `[[highlight]]` blocks: the panel's own colours by condition (SPEC 02 §5).
    #[serde(rename = "highlight")]
    pub highlight: Vec<HighlightCfg>,
    /// `[[sort_group]]` blocks: the bands the sort groups cut a listing into (SPEC 02 §4).
    #[serde(rename = "sort_group")]
    pub sort_groups: Vec<SortGroupCfg>,
    /// `[[column_set]]` blocks: the user's own columns for a view mode (SPEC 02 §3a).
    #[serde(rename = "column_set")]
    pub column_sets: Vec<ColumnSetCfg>,
    /// `[[mask_group]]` blocks: the named mask lists every mask field can use (SPEC 02 §5b).
    #[serde(rename = "mask_group")]
    pub mask_groups: Vec<MaskGroupCfg>,
    /// Anything we did not recognise, kept so `--dump-config` can show it and warn.
    #[serde(flatten, skip_serializing)]
    pub unknown: BTreeMap<String, toml::Value>,
}

impl Config {
    /// The named mask lists in force: the three FAR ships with, and the `[[mask_group]]`
    /// blocks over them.
    ///
    /// A block with the name of a built-in one replaces it, because that is what somebody
    /// writing `<arc>` again means; a block with no name or no masks is nothing and is
    /// passed over rather than refused — a mask field that found an empty group would say
    /// something unhelpful about a line nobody is looking at.
    pub fn mask_groups(&self) -> fcmd_core::filter::Groups {
        let mut groups = fcmd_core::filter::default_groups();
        for block in &self.mask_groups {
            let (name, mask) = (block.name.trim(), block.mask.trim());
            if name.is_empty() || mask.is_empty() {
                continue;
            }
            groups.insert(name, mask);
        }
        groups
    }

    /// Load the config file if it exists. A missing file is not an error; a broken one is.
    pub fn load(paths: &Paths) -> Result<(Config, Vec<String>), ConfigError> {
        // Said before anything is parsed, so that it is said even when the file is empty.
        let mut warnings: Vec<String> = paths
            .legacy()
            .into_iter()
            .map(|(path, fresh)| {
                format!("{path}: the name from before the rename — still read, and still written; rename it to `{fresh}` when convenient")
            })
            .collect();
        let file = paths.config_file.clone();
        if !file.exists() {
            return Ok((Config::default(), warnings));
        }
        let src = std::fs::read_to_string(&file).map_err(|source| ConfigError::Io {
            path: file.display().to_string(),
            source,
        })?;
        let (cfg, more) = Self::parse(&file.display().to_string(), &src)?;
        warnings.extend(more);
        Ok((cfg, warnings))
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

    /// The first rule that applies to `name` and whose program is installed: the config
    /// file's rules first, then the built-in ones.
    /// A site by its id, case-insensitively.
    pub fn site(&self, id: &str) -> Option<&SiteCfg> {
        self.sites
            .iter()
            .find(|s| s.id.eq_ignore_ascii_case(id.trim()))
    }

    pub fn decompiler_for(&self, name: &str) -> Option<DecompileRule> {
        self.decompile
            .iter()
            .cloned()
            .chain(default_decompile_rules())
            .find(|rule| rule.matches(name) && command_exists(rule.program()))
    }

    /// The effective configuration as TOML (`fcmd --dump-config`).
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
            "ui.command_line" => self.ui.command_line = parse_bool(value)?,
            "ui.command_history" => {
                self.ui.command_history = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number"))?
            }
            "ui.file_history" => {
                self.ui.file_history = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number"))?
            }
            "ui.history_lifetime" => {
                self.ui.history_lifetime = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number"))?
            }
            "ui.status_bar" => self.ui.status_bar = parse_bool(value)?,
            "ui.syntax" => self.ui.syntax = parse_bool(value)?,
            "ui.markdown" => self.ui.markdown = parse_bool(value)?,
            "ui.table" => self.ui.table = parse_bool(value)?,
            "ui.mouse" => self.ui.mouse = parse_bool(value)?,
            "ui.clipboard" => self.ui.clipboard = parse_bool(value)?,
            "ui.command_screen" => self.ui.command_screen = parse_bool(value)?,
            "editor.tab_width" => {
                self.editor.tab_width = parse_usize(value)?.clamp(1, 16);
            }
            "editor.tabs" => {
                let value = value.trim();
                if !matches!(value, "keep" | "typed" | "all") {
                    return Err(format!("`{value}` is not keep, typed or all"));
                }
                self.editor.tabs = value.to_string();
            }
            "editor.auto_indent" => self.editor.auto_indent = parse_bool(value)?,
            "editor.show_whitespace" => self.editor.show_whitespace = parse_bool(value)?,
            "editor.cursor_after_match" => self.editor.cursor_after_match = parse_bool(value)?,
            "viewer.key_bar" => self.viewer.key_bar = parse_bool(value)?,
            "viewer.status_line" => self.viewer.status_line = parse_bool(value)?,
            "viewer.scrollbar" => self.viewer.scrollbar = parse_bool(value)?,
            "viewer.wrap" => {
                let value = value.trim();
                if !matches!(value, "off" | "char" | "word") {
                    return Err(format!("`{value}` is not off, char or word"));
                }
                self.viewer.wrap = value.to_string();
            }
            "viewer.tab_width" => {
                self.viewer.tab_width = parse_usize(value)?.clamp(1, 16);
            }
            "term.shell" => self.term.shell = value.to_string(),
            "term.scrollback" => self.term.scrollback = parse_usize(value)?,
            "panel.view" => self.panel.view = value.to_string(),
            "panel.sort" => self.panel.sort = value.to_string(),
            "panel.sort_chain" => {
                self.panel.sort_chain = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            }
            "panel.reverse" => self.panel.reverse = parse_bool(value)?,
            "panel.show_hidden" => self.panel.show_hidden = parse_bool(value)?,
            "panel.dirs_first" => self.panel.dirs_first = parse_bool(value)?,
            "panel.selected_first" => self.panel.selected_first = parse_bool(value)?,
            "panel.natural_sort" => self.panel.natural_sort = parse_bool(value)?,
            "panel.split" => {
                self.panel.split = value
                    .trim()
                    .parse::<u16>()
                    .map_err(|_| format!("`{value}` is not a number"))?
                    .clamp(10, 90)
            }
            "panel.height_cut" => {
                self.panel.height_cut = value
                    .trim()
                    .parse()
                    .map_err(|_| format!("`{value}` is not a number"))?
            }
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
            "net.known_hosts" => self.net.known_hosts = value.to_string(),
            "net.keepalive_secs" => {
                self.net.keepalive_secs = value
                    .parse()
                    .map_err(|_| format!("net.keepalive_secs: `{value}` is not a number"))?
            }
            "net.fetch_limit_mb" => {
                self.net.fetch_limit_mb = value
                    .parse()
                    .map_err(|_| format!("net.fetch_limit_mb: `{value}` is not a number"))?
            }
            "panel.allow_reverse_sort" => self.panel.allow_reverse_sort = parse_bool(value)?,
            "panel.descriptions" => self.panel.descriptions = parse_bool(value)?,
            "panel.column_titles" => self.panel.column_titles = parse_bool(value)?,
            "panel.status_line" => self.panel.status_line = parse_bool(value)?,
            "panel.scrollbar" => self.panel.scrollbar = parse_bool(value)?,
            "panel.free_space" => self.panel.free_space = parse_bool(value)?,
            "panel.auto_refresh" => self.panel.auto_refresh = parse_bool(value)?,
            "general.confirm_exit" => self.general.confirm_exit = parse_bool(value)?,
            "general.confirm_delete" => self.general.confirm_delete = parse_bool(value)?,
            "general.use_trash" => self.general.use_trash = parse_bool(value)?,
            "general.desktop_open" => self.general.desktop_open = parse_bool(value)?,
            "general.confirm_overwrite" => self.general.confirm_overwrite = parse_bool(value)?,
            other => return Err(format!("unknown setting `{other}`")),
        }
        Ok(())
    }
}

fn parse_usize(v: &str) -> Result<usize, String> {
    v.trim()
        .parse()
        .map_err(|_| format!("expected a whole number, got `{}`", v.trim()))
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
    fn a_configuration_from_before_the_rename_is_still_read_and_says_so() {
        let dir = std::env::temp_dir().join(format!(
            "fcmd-legacy-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        std::fs::create_dir_all(&dir).expect("the fixture directory");
        let old = dir.join("scmd.toml");
        std::fs::write(&old, "[ui]\ntheme = \"midnight\"\n").expect("the old file");
        std::fs::write(dir.join("scmd-state.toml"), "").expect("the old state");

        let paths = Paths::discover(None, None);
        assert!(
            paths.legacy().is_empty() || !paths.config_file.exists(),
            "nothing of this machine's own is claimed to be legacy"
        );

        // `--config` names a file outright, so the fallback is exercised through `kept`.
        assert_eq!(kept(&dir, CONFIG_FILE_NAME, "scmd.toml"), old);
        std::fs::write(dir.join(CONFIG_FILE_NAME), "").expect("the new file");
        assert_eq!(
            kept(&dir, CONFIG_FILE_NAME, "scmd.toml"),
            dir.join(CONFIG_FILE_NAME),
            "the new name wins when both are there"
        );
        std::fs::remove_file(dir.join(CONFIG_FILE_NAME)).expect("undo");

        let paths = Paths::discover(Some(&old), None);
        assert_eq!(paths.config_file, old);
        assert_eq!(paths.state_file, dir.join("scmd-state.toml"));
        let (cfg, warnings) = Config::load(&paths).expect("it loads");
        assert_eq!(cfg.ui.theme, "midnight");
        assert_eq!(
            warnings.len(),
            2,
            "one line per file, and no more: {warnings:?}"
        );
        assert!(
            warnings.iter().all(|w| w.contains("before the rename")),
            "{warnings:?}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn defaults_are_the_documented_ones() {
        let c = Config::default();
        assert_eq!(c.ui.theme, "commander");
        assert_eq!(c.ui.keymap, "far");
        assert_eq!(c.panel.view, "full");
        assert!(c.panel.dirs_first);
        // Nothing to assert about an update check: there is no such setting, on purpose.
        // What there is to assert is that asking about an overwrite is on by default, which
        // is the one of these three that costs data when it is wrong.
        assert!(c.general.confirm_overwrite);
        assert_eq!(c.panel.quick_search, QuickSearch::Alt);
        // The viewer opens with the chrome it has always had and no scrollbar, as FAR does.
        assert!(c.viewer.status_line && c.viewer.key_bar);
        assert!(!c.viewer.scrollbar);
        assert_eq!(c.viewer.wrap_kind(), "off");
    }

    #[test]
    fn the_viewers_tab_width_default_is_the_one_the_viewer_itself_uses() {
        // The number is written in both crates — this one does not depend on the viewer — so
        // the two are compared here rather than left to drift.
        assert_eq!(ViewCfg::default().tab_width, 8);
    }

    #[test]
    fn a_wrap_nobody_recognises_means_no_wrap_rather_than_a_guess() {
        let mut cfg = ViewCfg {
            wrap: "sideways".into(),
            ..ViewCfg::default()
        };
        assert_eq!(cfg.wrap_kind(), "off");
        cfg.wrap = "word".into();
        assert_eq!(cfg.wrap_kind(), "word");
        // `on` and `true` are what somebody types for a box that is now three choices.
        cfg.wrap = "on".into();
        assert_eq!(cfg.wrap_kind(), "char");

        // And the setter refuses what the dialog would never send.
        let mut whole = Config::default();
        assert!(whole.set("viewer.wrap", "sideways").is_err());
        assert!(whole.set("viewer.wrap", "word").is_ok());
        assert_eq!(whole.viewer.wrap, "word");
        assert!(whole.set("viewer.tab_width", "4").is_ok());
        assert_eq!(whole.viewer.tab_width, 4);
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
    fn the_state_file_survives_a_round_trip_and_a_broken_file_is_not_fatal() {
        let dir = std::env::temp_dir().join(format!("fcmd-state-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let paths = Paths::discover(Some(&dir.join("fcmd.toml")), None);
        let (state, problem) = State::load(&paths);
        assert!(problem.is_none(), "a missing state file is the first run");
        assert!(state.command_history.is_empty());

        let state = State {
            command_history: vec!["make".into(), "cargo test".into()],
            folder_history: vec!["/tmp".into()],
            file_history: vec!["/home/mb/notes.txt".into()],
            left: "/home".into(),
            right: "/tmp".into(),
            left_tabs: vec![
                TabState {
                    path: "/home".into(),
                    name: String::new(),
                    locked: false,
                },
                TabState {
                    path: "/etc".into(),
                    name: "config".into(),
                    locked: true,
                },
            ],
            right_tabs: Vec::new(),
            left_tab: 0,
            right_tab: 0,
            bookmarks: vec![Bookmark {
                path: "/usr/share".into(),
                name: "shared".into(),
                digit: Some(3),
            }],
            // A dialog field's own history, and what the histories say about themselves,
            // travel with their lines.
            field_history: vec![FieldHistory {
                key: "find.mask".into(),
                lines: vec!["*.rs".into()],
            }],
            history_marks: vec![HistoryMark {
                kind: "commands".into(),
                text: "make".into(),
                when: 1_700_000_000,
                locked: true,
            }],
            // The tree cache travels with the rest: a tree that opens filled is the whole
            // point of writing it down.
            tree: vec![TreeDir {
                path: "/home".into(),
                dirs: vec!["/home/mb".into()],
            }],
        };
        state.save(&paths).unwrap();
        let (back, problem) = State::load(&paths);
        assert!(problem.is_none());
        assert_eq!(back, state);

        // A file from a version that had no tabs: the row is simply empty.
        std::fs::write(&paths.state_file, "left = \"/srv\"\nright = \"/tmp\"\n").unwrap();
        let (back, problem) = State::load(&paths);
        assert!(problem.is_none(), "{problem:?}");
        assert_eq!(back.left, "/srv");
        assert!(back.left_tabs.is_empty());

        std::fs::write(&paths.state_file, "this is not toml {{{").unwrap();
        let (back, problem) = State::load(&paths);
        assert_eq!(
            back,
            State::default(),
            "a broken file falls back to nothing"
        );
        assert!(problem.unwrap().contains("fcmd-state.toml"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_association_matches_an_extension_or_a_whole_name() {
        let md = AssocCfg {
            mask: "*.MD".into(),
            open: "glow {file}".into(),
            ..AssocCfg::default()
        };
        assert!(
            md.matches("README.md"),
            "the case of the mask does not matter"
        );
        assert!(md.matches("NOTES.MD"));
        assert!(!md.matches("md"), "an extension is not a whole name");
        assert!(!md.matches("readme.markdown"));

        let make = AssocCfg {
            mask: "Makefile".into(),
            ..AssocCfg::default()
        };
        assert!(make.matches("makefile"));
        assert!(!make.matches("Makefile.in"));

        // An empty mask matches nothing rather than everything: a block somebody half wrote
        // should do nothing at all.
        assert!(!AssocCfg::default().matches("anything.txt"));

        // A dotfile is a name, not an extension: `.md` is called `.md`, and `*.md` is about
        // files whose *names* end in `.md`.
        assert!(!md.matches(".md"));
        let dotfile = AssocCfg {
            mask: ".gitignore".into(),
            ..AssocCfg::default()
        };
        assert!(dotfile.matches(".gitignore"));
    }

    #[test]
    fn an_association_goes_by_its_name_in_the_list_and_by_its_mask_without_one() {
        let mut a = AssocCfg {
            mask: "*.md".into(),
            ..AssocCfg::default()
        };
        assert_eq!(a.label(), "*.md");
        a.name = "Markdown, laid out".into();
        assert_eq!(a.label(), "Markdown, laid out");
    }

    #[test]
    fn association_blocks_survive_a_round_trip_through_the_config_file() {
        let mut c = Config::default();
        c.assocs.push(AssocCfg {
            mask: "*.md".into(),
            name: "Markdown".into(),
            open: "glow {file}".into(),
            ..AssocCfg::default()
        });
        let back: Config = toml::from_str(&c.dump()).expect("what we wrote, we can read");
        assert_eq!(back.assocs, c.assocs);
        assert!(back.general.desktop_open, "and the default came back too");
    }

    #[test]
    fn dump_round_trips() {
        let mut c = Config::default();
        c.ui.theme = "blade".into();
        c.panel.show_hidden = true;
        // What the session editor stores has to survive `Save setup` as well.
        c.sites.push(SiteCfg {
            id: "jump".into(),
            host: "jump.example.net".into(),
            port: Some(2222),
            auth: Some("key:~/.ssh/id_ed25519".into()),
            ..Default::default()
        });
        let (again, warnings) = Config::parse("dump.toml", &c.dump()).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(again.ui.theme, "blade");
        assert!(again.panel.show_hidden);
        assert_eq!(again.sites.len(), 1);
        assert_eq!(again.sites[0].host, "jump.example.net");
        assert_eq!(again.sites[0].port, Some(2222));
        assert_eq!(again.sites[0].scheme, "sftp");
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
    fn an_explicit_config_file_wins_and_state_sits_beside_it() {
        let p = Paths::discover(Some(Path::new("/tmp/scconf/fcmd.toml")), None);
        assert_eq!(p.config_file, PathBuf::from("/tmp/scconf/fcmd.toml"));
        assert_eq!(p.state_file, PathBuf::from("/tmp/scconf/fcmd-state.toml"));
        assert!(!p.portable);
    }

    #[test]
    fn an_explicit_plugins_dir_wins() {
        let p = Paths::discover(None, Some(Path::new("/opt/sc/plugins")));
        assert_eq!(p.plugins_dir, PathBuf::from("/opt/sc/plugins"));
    }

    #[test]
    fn discovery_produces_one_file_and_no_directory_of_our_own() {
        let p = Paths::discover(None, None);
        assert_eq!(
            p.config_file.file_name().unwrap(),
            std::ffi::OsStr::new("fcmd.toml")
        );
        // `~/.config/fcmd.toml`, not `~/.config/sc/config.toml`: nothing to create, nothing to
        // leave behind but one file.
        let parent = p.config_file.parent().unwrap();
        assert_ne!(parent.file_name(), Some(std::ffi::OsStr::new("fcmd")));
        assert!(p.describe().contains("plugins"));
    }

    #[test]
    fn decompile_masks_match_extensions_and_names() {
        let rule = DecompileRule {
            name: "javap".into(),
            mask: "*.CLASS".into(),
            cmd: "javap -c {file}".into(),
        };
        assert!(rule.matches("Main.class"));
        assert!(rule.matches("main.CLASS"));
        assert!(!rule.matches("main.classx"));
        assert!(!rule.matches("classfile"));
        assert_eq!(rule.program(), "javap");

        let exact = DecompileRule {
            name: "x".into(),
            mask: "a.out".into(),
            cmd: "objdump -d {file}".into(),
        };
        assert!(exact.matches("a.out"));
        assert!(!exact.matches("b.out"));
    }

    #[test]
    fn a_configured_rule_wins_over_the_builtin_one() {
        let (cfg, warnings) = Config::parse(
            "fcmd.toml",
            r#"
            [[decompile]]
            name = "my tool"
            match = "*.class"
            cmd = "sh -c true {file}"
            "#,
        )
        .unwrap();
        assert!(warnings.is_empty());
        assert_eq!(cfg.decompile.len(), 1);
        // `sh` exists on any unix runner; on Windows this falls through to the built-ins.
        if command_exists("sh") {
            assert_eq!(cfg.decompiler_for("A.class").unwrap().name, "my tool");
        }
        assert!(cfg.decompiler_for("whatever.unknown").is_none());
    }

    #[test]
    fn command_existence_is_checked_before_a_tool_is_offered() {
        assert!(command_exists("cargo") || command_exists("rustc"));
        assert!(!command_exists("definitely-not-a-real-program-42"));
        assert!(!command_exists(""));
    }

    #[test]
    fn themes_and_keymaps_come_from_the_config_file() {
        let (cfg, warnings) = Config::parse(
            "fcmd.toml",
            r##"
            [ui]
            theme = "mine"

            [[theme]]
            id = "mine"
            inherit = "matrix"
            [theme.colors]
            "selected.fg" = "#ff00ff"

            [[keymap]]
            name = "mine"
            inherit = "far"
            [[keymap.bind]]
            keys = ["F4"]
            cmd = "file.view"
            "##,
        )
        .unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(cfg.themes.len(), 1);
        assert_eq!(cfg.keymaps.len(), 1);
        assert_eq!(cfg.themes[0]["id"].as_str(), Some("mine"));
        assert_eq!(cfg.keymaps[0]["name"].as_str(), Some("mine"));
    }
}

/// What survives between sessions and is not configuration: the histories, and where the
/// panels were. A missing file is the normal first run; a broken one is reported and ignored,
/// because losing a history is never a reason to refuse to start.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    /// Oldest first, as the command line walks it.
    #[serde(default)]
    pub command_history: Vec<String>,
    /// Where each panel had been, oldest first.
    #[serde(default)]
    pub folder_history: Vec<String>,
    /// The files that were read or edited, newest first (`Alt+F11`).
    #[serde(default)]
    pub file_history: Vec<String>,
    #[serde(default)]
    pub left: String,
    #[serde(default)]
    pub right: String,
    /// The tabs each side had, in the order they were drawn, and which of them was on
    /// screen. Empty is a session that ran before tabs existed, or one that never opened a
    /// second tab: one tab is then built from `left` / `right` above, which is why those two
    /// are still written.
    #[serde(default)]
    pub left_tabs: Vec<TabState>,
    #[serde(default)]
    pub right_tabs: Vec<TabState>,
    #[serde(default)]
    pub left_tab: usize,
    #[serde(default)]
    pub right_tab: usize,
    /// The folders kept to hand (SPEC 02 §11). They live here rather than in `fcmd.toml`
    /// because a key sets them: a shortcut that waited for *Save setup* and vanished without
    /// it would be a bug, not a setting.
    #[serde(default)]
    pub bookmarks: Vec<Bookmark>,
    /// What has been typed into the dialogs' own fields, newest first, one row per field.
    #[serde(default)]
    pub field_history: Vec<FieldHistory>,
    /// What each history line has to say for itself: when it was last used, and whether it
    /// is pinned. The lines themselves are in the three lists above; this is the table
    /// beside them, and a line with no row here is simply one nobody has pinned and nobody
    /// has used since this was written.
    #[serde(default)]
    pub history_marks: Vec<HistoryMark>,
    /// What the tree has read: one row per directory, with the subdirectories it held
    /// (SPEC 02 §7). It is a cache and nothing else — a row that is out of date costs a
    /// re-read — so it is written last and read first, and a missing one is simply a tree
    /// that opens empty and fills as it is walked.
    #[serde(default)]
    pub tree: Vec<TreeDir>,
}

/// One dialog field's history, as the state file holds it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldHistory {
    /// The field's own key: `find.mask`, `copy.target`.
    pub key: String,
    /// Newest first, which is the order the list shows.
    #[serde(default)]
    pub lines: Vec<String>,
}

/// One history line's mark, as the state file holds it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryMark {
    /// `commands`, `folders` or `files`.
    pub kind: String,
    pub text: String,
    /// Seconds since the epoch, or `0` for a line that has never been stamped.
    #[serde(default)]
    pub when: i64,
    #[serde(default)]
    pub locked: bool,
}

/// One directory of the tree cache, as the state file holds it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeDir {
    pub path: String,
    /// Its subdirectories, in the order the listing gave them.
    #[serde(default)]
    pub dirs: Vec<String>,
}

/// One folder kept to hand: where it is, what it is called, and the digit it answers to.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bookmark {
    pub path: String,
    /// What the list calls it, or empty for one that goes by its path.
    #[serde(default)]
    pub name: String,
    /// `0` to `9` for a folder shortcut; `None` for one that is only in the list.
    #[serde(default)]
    pub digit: Option<u8>,
}

/// One tab of one side, as the state file holds it.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TabState {
    pub path: String,
    /// What it was called, or empty for a tab that goes by its directory.
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub locked: bool,
}

impl State {
    /// Read the state file. The second value is a problem to report, not an error to die on.
    pub fn load(paths: &Paths) -> (State, Option<String>) {
        let file = &paths.state_file;
        if !file.exists() {
            return (State::default(), None);
        }
        match std::fs::read_to_string(file) {
            Ok(src) => match toml::from_str::<State>(&src) {
                Ok(state) => (state, None),
                Err(e) => (State::default(), Some(format!("{}: {e}", file.display()))),
            },
            Err(e) => (State::default(), Some(format!("{}: {e}", file.display()))),
        }
    }

    pub fn save(&self, paths: &Paths) -> std::io::Result<()> {
        if let Some(parent) = paths.state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)
            .unwrap_or_else(|e| format!("# cannot serialise the state: {e}\n"));
        std::fs::write(&paths.state_file, body)
    }
}
