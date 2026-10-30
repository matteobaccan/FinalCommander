//! FinalCommander — an orthodox file manager for the terminal.
//!
//! The interface is strictly TUI; see `docs/spec/00-overview.md`.

mod app;
mod cmdline;
mod colset;
mod complete;
mod drives;
mod expand;
mod form;
mod grab;
mod help;
mod highlight;
mod history;
mod hit;
mod menu;
mod panel;
mod proccard;
mod prompt;
mod render;
mod tabs;
mod termkeys;
mod tree;
mod watch;

use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::Parser;
use fcmd_cmd::{Command as FcCommand, Keymap, Scope as FcmdScope};
use fcmd_config::{Config, Paths};
use fcmd_core::{CancelToken, VPath};
use fcmd_i18n::I18n;
use fcmd_jobs::{ArcFs, JobSpec};
use fcmd_net::{Pool, RoutedFs, Router, Site};
use fcmd_s3::jobs::ObjectFs;
use fcmd_tui::{Input, Term};
use fcmd_ui::{Buffer, Theme, ThemeSet};
use fcmd_vfs::Vfs;

use crate::app::{
    App, ConnectOutcome, Event, PluginCommand, RemoteStamp, Request, Startup, UploadError,
};

#[derive(Debug, Parser)]
#[command(
    name = "fcmd",
    version,
    about = "FinalCommander — orthodox file manager for the terminal",
    long_about = "FinalCommander is a two-panel file manager for the terminal that \
works the same way on Linux, macOS and Windows: one program, one set of keys, one \
configuration file.\n\n\
Two panels side by side, the function keys along the bottom saying what they do right now, \
and a command line underneath. Copying, moving and deleting run as jobs with progress and a \
way to stop them. Archives open as directories. The viewer streams a file of any size and \
disassembles a binary; the editor edits lines or bytes. SFTP panels are panels like any \
other.\n\n\
The keys are FAR's by default, with a Midnight Commander profile; every action is a named \
command and every command can be bound to a key. Press F1 inside the program for the manual, \
whose key and command chapters are generated from the keymap in force."
)]
struct Cli {
    /// Directory for the left panel, and optionally for the right one.
    #[arg(value_name = "PATH", num_args = 0..=2)]
    paths: Vec<PathBuf>,

    /// Use this configuration file instead of the default one.
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Use this plugins directory (the only directory sc ever reads besides the config file).
    #[arg(long, value_name = "DIR")]
    plugins: Option<PathBuf>,

    /// Theme id (commander, black, white, matrix, tron, blade, or an external one).
    #[arg(long, value_name = "ID")]
    theme: Option<String>,

    /// Keymap profile: a built-in one (far, mc) or a [[keymap]] block from the config file.
    #[arg(long, value_name = "NAME")]
    keymap: Option<String>,

    /// Interface language: en, it, de, fr, es, ru, zh, or auto.
    #[arg(long, value_name = "TAG")]
    lang: Option<String>,

    /// Override a setting, e.g. --set panel.show_hidden=true
    #[arg(long = "set", value_name = "KEY=VALUE")]
    set: Vec<String>,

    /// List the themes found in this run and where each came from.
    #[arg(long)]
    list_themes: bool,

    /// List every command id (the vocabulary keys, menus and plugins bind to).
    #[arg(long)]
    list_commands: bool,

    /// List the interface languages compiled in.
    #[arg(long)]
    list_languages: bool,

    /// Validate the keymap and print the resolved bindings.
    #[arg(long)]
    check_keymap: bool,

    /// Print a theme as TOML, to start an external theme from.
    #[arg(long, value_name = "ID")]
    export_theme: Option<String>,

    /// Print the effective configuration.
    #[arg(long)]
    dump_config: bool,

    /// Print where the config file, the state file and the plugins directory are.
    #[arg(long = "paths")]
    show_paths: bool,

    /// List the plugins found in the plugins directory.
    #[arg(long)]
    list_plugins: bool,

    /// Print the manual page, in roff, for a packager to install as fcmd.1.
    ///
    /// It is generated from these very options, so it cannot describe a flag that does not
    /// exist or miss one that does. `docs/fcmd.1` in the repository is this output, and a
    /// test fails if the two ever differ.
    #[arg(long, hide = true)]
    man: bool,
}

/// A search on a worker thread. The engine streams events; each one becomes an event the app
/// already knows how to draw, so the list fills while the walk is still going.
///
/// This is the one place that turns what the dialog asked for into what the engine takes: the
/// dialog knows nothing about walks, and the engine knows nothing about panels.
fn run_find(
    id: u64,
    spec: app::FindSpec,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<fcmd_arc::Cache>,
    groups: &fcmd_core::filter::Groups,
) {
    use fcmd_core::filter::{Filter, MaskSet, SizeRange};
    use fcmd_find::{Content, Needle};

    let started = std::time::Instant::now();
    let fail = |message: String| {
        let _ = tx.send(Event::FindDone {
            id,
            files: 0,
            dirs: 0,
            elapsed: started.elapsed(),
            problems: Vec::new(),
            error: Some(message),
        });
    };

    let masks = match MaskSet::parse_with(&spec.mask, groups, spec.case_sensitive) {
        Ok(masks) => masks,
        Err(e) => return fail(e.to_string()),
    };
    let mut filter = Filter::named("find");
    filter.masks = Some(masks);
    if !spec.size.trim().is_empty() {
        match SizeRange::parse(&spec.size) {
            Ok(range) => filter.size = Some(range),
            Err(e) => return fail(e.to_string()),
        }
    }

    let content = if spec.text.is_empty() {
        None
    } else {
        let needle = if spec.hex {
            match Needle::hex(&spec.text) {
                Ok(needle) => needle,
                Err(e) => return fail(e.to_string()),
            }
        } else {
            Needle::text(spec.text.clone())
                .case_sensitive(spec.case_sensitive)
                .whole_words(spec.whole_words)
        };
        Some(if spec.not_containing {
            Content::not_containing(needle)
        } else {
            Content::containing(needle)
        })
    };

    let query = fcmd_find::Query {
        roots: spec.roots.clone(),
        filter,
        // "This folder" is the only scope that does not go down.
        subdirectories: spec.scope != app::FindScope::Panel,
        // The search dialog has no depth of its own yet; the comparison does.
        depth: None,
        follow_links: spec.follow_links,
        include_dirs: spec.directories,
        content,
        in_archives: spec.in_archives,
    };

    let mut problems: Vec<String> = Vec::new();
    let mut last_report = std::time::Instant::now();
    // The archive-aware filesystem only when the box is ticked: without it the walk is the
    // plain local one, and a `.zip` is a file like any other.
    let in_archives = fcmd_find::ArcFind::new(archives.clone(), fcmd_vfs::local::LocalVfs);
    let fs: &dyn fcmd_find::FindFs = if spec.in_archives {
        &in_archives
    } else {
        &fcmd_vfs::local::LocalVfs
    };
    let result = fcmd_find::find(fs, &query, &cancel, &mut |event| match event {
        fcmd_find::Event::Hit(hit) => {
            let _ = tx.send(Event::FindHit {
                id,
                path: hit.path,
                entry: Box::new(hit.entry),
            });
        }
        fcmd_find::Event::Scanning(dir) => {
            // Not every directory: a walk over a large tree would spend its time telling
            // the screen about itself, and ten a second is already more than a reader can
            // follow.
            if last_report.elapsed() >= std::time::Duration::from_millis(100) {
                last_report = std::time::Instant::now();
                let _ = tx.send(Event::FindProgress {
                    id,
                    scanning: dir.to_string(),
                    files: 0,
                    dirs: 0,
                });
            }
        }
        fcmd_find::Event::Problem(problem) => {
            // Capped: a search over a whole drive meets unreadable directories by the
            // dozen, and ten thousand of them help nobody.
            if problems.len() < 200 {
                problems.push(format!("{}: {}", problem.path, problem.message));
            }
        }
    });
    match result {
        Ok(stats) => {
            let _ = tx.send(Event::FindDone {
                id,
                files: stats.files_seen,
                dirs: stats.dirs_seen,
                elapsed: started.elapsed(),
                problems,
                error: None,
            });
        }
        Err(e) => fail(e.to_string()),
    }
}

/// The device a local directory is on: free space for the panel frame, and the size and the
/// allocation unit for the information panel. A remote provider has no such thing to report.
fn device(path: &fcmd_core::VPath) -> Option<fcmd_platform::Disk> {
    if path.is_remote() || path.in_container() {
        return None;
    }
    fcmd_platform::disk(&path.os())
}

/// Who the named files of one directory belong to: the user and the group, by name.
///
/// One cache for the whole directory, which is the point of doing it per directory rather
/// than per file: a listing is nearly always one or two accounts repeated, and turning an id
/// or a SID into a name can mean a directory server on the other side of a network. A file
/// that has gone away between the listing and this is left out rather than guessed at.
fn owners_of(
    dir: &fcmd_core::VPath,
    names: &[String],
) -> std::collections::HashMap<String, (String, String)> {
    let mut cache = fcmd_platform::OwnerCache::default();
    let mut out = std::collections::HashMap::with_capacity(names.len());
    let base = dir.os();
    for name in names {
        if let Some(found) = fcmd_platform::owner(&base.join(name), &mut cache) {
            out.insert(name.clone(), (found.user, found.group));
        }
    }
    out
}

/// The program's name, drawn, for the top of what it says on the command line.
///
/// Plain ASCII on purpose: this goes to whatever terminal the program was started from,
/// before anything is known about what that terminal can draw, and a name that comes out as
/// question marks is worse than no name at all.
const BANNER: &str = r"  __                    _
 / _| ___ _ __ ___   __| |
| |_ / __| '_ ` _ \ / _` |
|  _| (__| | | | | | (_| |
|_|  \___|_| |_| |_|\__,_|";

/// Write the banner, when there is somebody to read it.
///
/// Not into a pipe or a file: `--dump-config > config.toml` and `--list-commands | grep` are
/// how these outputs are used, and a banner in front of them is something to strip. A
/// terminal is the one place a name drawn in letters is worth the three lines it costs.
fn print_banner() {
    use std::io::IsTerminal;
    if !std::io::stdout().is_terminal() {
        return;
    }
    println!("{BANNER}");
    println!("FinalCommander {}", env!("CARGO_PKG_VERSION"));
    println!();
}

fn main() -> Result<()> {
    // `--help` and `--version` never reach `run`: clap answers them itself and exits while
    // parsing, so the banner above them goes here.
    let asked =
        std::env::args().any(|a| matches!(a.as_str(), "-h" | "--help" | "-V" | "--version"));
    if asked {
        print_banner();
    }
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("fcmd: {e:#}");
            std::process::exit(1);
        }
    }
}

/// The manual page, in roff, from the options above plus the sections clap cannot know about.
fn man_page() -> String {
    use clap::CommandFactory;
    let command = Cli::command();
    let mut page = Vec::new();
    clap_mangen::Man::new(command)
        .title("FCMD")
        .section("1")
        .manual("FinalCommander")
        .render(&mut page)
        .expect("writing to a vector cannot fail");
    let mut text = String::from_utf8(page).expect("roff is text");
    // What a man page has and a `--help` does not: where the files are, what the keys are, and
    // where to look next. Written here rather than in the roff by hand, so the generated part
    // stays generated.
    text.push_str(
        r#".SH FILES
.TP
.I ~/.config/fcmd/config.toml
The settings. B\-\-dump\-configR prints the effective ones. On macOS this is under
.I ~/Library/Application Support/fcmd
and on Windows under
.I %APPDATA%\fcmd .
.TP
.I ~/.config/fcmd/keymap.toml
Your own key bindings, applied over the profile named by B\-\-keymapR.
.TP
.I ~/.config/fcmd/session.toml
Where the panels were when the program last exited.
.TP
.I ~/.config/fcmd/themes/
Colour schemes. A TOML file dropped in here is found at startup; B\-\-export\-themeR
prints a built-in one to start from.
.TP
.I ~/.config/fcmd/sites.toml
Stored connections: the host, the user and the path, never a password.
.SH KEYS
.B F1
help,
.B F3
view,
.B F4
edit,
.B F5
copy,
.B F6
move or rename,
.B F7
make a directory,
.B F8
delete,
.B F9
the menu bar,
.B F10
leave,
.B F12
the screens.
.B Tab
moves between the panels and
.B Insert
selects.
.PP
A panel holds more than one directory at a time:
.B Ctrl+Alt+T
opens another tab,
.B Ctrl+Alt+N
and
.B Ctrl+Alt+P
step along the row,
.B Ctrl+Alt+W
closes one,
.B Ctrl+Alt+M
hands it to the other panel and
.B Ctrl+Alt+K
locks it to its directory.
.PP
Folders are kept to hand on ten digits:
.B Ctrl+Shift+0..9
sets one,
.B Alt+0..9
goes to it, and
.B Ctrl+D
opens the whole list.
.PP
.B Ctrl+Up
and
.B Ctrl+Down
change how much of the screen the panels take, and
.B Ctrl++
and
.B Ctrl+\-
ask the terminal for a larger or smaller font \(em which xterm answers and most other
terminals handle themselves before the program ever sees the key.
.PP
Those are the defaults of the
.B far
profile; the
.B mc
profile is Midnight Commander's. Every action is a named command,
.B \-\-list\-commands
prints them all, and
.B \-\-check\-keymap
says whether a keymap is sound. The help inside the program (BF1R) generates its key and
command chapters from the keymap actually in force, so it cannot drift.
.SH EXIT STATUS
.TP
0
Normal exit, including every informational option above.
.TP
1
The configuration, the keymap or an argument could not be used; the reason is on standard
error.
.SH SEE ALSO
The documentation under
.I docs/
in the source tree: the specification in
.IR docs/spec/ ,
the feature inventory in
.IR docs/02\-features.md ,
and what is deliberately not done yet in
.IR docs/BACKLOG.md .
.SH BUGS
https://github.com/matteobaccan/FinalCommander/issues
"#,
    );
    text
}

fn run(cli: Cli) -> Result<()> {
    let paths = Paths::discover(cli.config.as_deref(), cli.plugins.as_deref());
    let (mut cfg, mut warnings) = Config::load(&paths).context("loading the configuration")?;

    for assignment in &cli.set {
        let (key, value) = assignment
            .split_once('=')
            .with_context(|| format!("--set expects KEY=VALUE, got `{assignment}`"))?;
        cfg.set(key.trim(), value.trim())
            .map_err(anyhow::Error::msg)?;
    }
    if let Some(t) = &cli.theme {
        cfg.ui.theme = t.clone();
    }
    if let Some(k) = &cli.keymap {
        cfg.ui.keymap = k.clone();
    }
    if let Some(l) = &cli.lang {
        cfg.ui.language = l.clone();
    }

    let i18n = I18n::new(&cfg.ui.language);
    let origin = paths.config_file.display().to_string();
    let themes = ThemeSet::builtin_only().with_tables(&origin, &cfg.themes);
    // Plugins are read before the theme and the keymap are chosen, so a plugin can supply
    // either one.
    let (themes, plugin_keymaps, plugin_commands) = load_plugins(&paths, themes, &mut warnings);
    warnings.extend(themes.load_errors.clone());
    let keymap = load_keymap(&cfg, &origin, &plugin_keymaps)?;

    let theme = match themes.get(&cfg.ui.theme) {
        Some(t) => t.clone(),
        None => {
            warnings.push(format!(
                "unknown theme `{}`, using `{}`",
                cfg.ui.theme,
                fcmd_ui::DEFAULT_THEME
            ));
            Theme::default_theme()
        }
    };
    warnings.extend(theme.warnings().iter().cloned());

    // ── informational modes: print and exit, no terminal takeover
    //
    // Each of these says something on the command line rather than taking the screen, so each
    // gets the name above it — except `--man`, whose bytes are a file a packager installs and
    // which a test compares against `docs/fcmd.1` byte for byte.
    if cli.show_paths
        || cli.list_plugins
        || cli.list_themes
        || cli.list_commands
        || cli.list_languages
        || cli.check_keymap
        || cli.export_theme.is_some()
        || cli.dump_config
    {
        print_banner();
    }
    if cli.show_paths {
        print!("{}", paths.describe());
        return Ok(());
    }
    if cli.list_plugins {
        list_plugins(&paths);
        return Ok(());
    }
    if cli.list_themes {
        for t in themes.iter() {
            let source = match &t.source {
                fcmd_ui::ThemeSource::BuiltIn => "built-in".to_string(),
                fcmd_ui::ThemeSource::Config => origin.clone(),
                fcmd_ui::ThemeSource::Plugin(p) => format!("plugin:{p}"),
            };
            println!("{:<12} {:<16} {source}", t.id, t.name);
        }
        return Ok(());
    }
    if cli.list_commands {
        for id in command_ids() {
            println!("{id}");
        }
        return Ok(());
    }
    if cli.list_languages {
        for (tag, name) in fcmd_i18n::LANGUAGES {
            println!("{tag:<4} {name}");
        }
        return Ok(());
    }
    if cli.check_keymap {
        print!("{keymap}");
        return Ok(());
    }
    if let Some(id) = &cli.export_theme {
        let t = themes
            .get(id)
            .cloned()
            .or_else(|| Theme::builtin(id).ok())
            .with_context(|| format!("unknown theme `{id}`"))?;
        print!("{}", t.to_toml());
        return Ok(());
    }
    if cli.man {
        print!("{}", man_page());
        return Ok(());
    }
    if cli.dump_config {
        print!("{}", cfg.dump());
        for t in themes
            .iter()
            .filter(|t| t.source == fcmd_ui::ThemeSource::Config)
        {
            print!("\n{}", t.to_toml());
        }
        for w in &warnings {
            eprintln!("warning: {w}");
        }
        return Ok(());
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let left = VPath::local(cli.paths.first().cloned().unwrap_or_else(|| cwd.clone())).normalised();
    let right = VPath::local(cli.paths.get(1).cloned().unwrap_or(cwd)).normalised();

    let (sites, site_problems) = Site::list_from_config(&cfg);
    warnings.extend(site_problems);

    // The `[[highlight]]` groups, compiled once: a condition and an ink per group, asked in
    // the order they were written (SPEC 02 §5).
    let highlight = highlight::groups(&cfg, &mut warnings);
    // And the `[[sort_group]]` bands, compiled the same way and from the same file.
    let sort_groups = std::sync::Arc::new(highlight::sort_groups(&cfg, &mut warnings));
    // And the `[[column_set]]` blocks: the user's own columns for a view mode (SPEC 02 §3a).
    let column_sets = colset::sets(&cfg, &mut warnings);

    // The histories from the last session. A broken state file is a warning, never a stop.
    let (state, state_problem) = fcmd_config::State::load(&paths);
    if let Some(problem) = state_problem {
        warnings.push(problem);
    }

    let mut app = App::new(Startup {
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
        column_sets,
    });
    app.cmd.load_history(state.command_history);
    app.file_history = state.file_history;
    // What the histories say about themselves: which lines are pinned, and when each was
    // last used. A state file written before any of this has none, and every line in it
    // simply counts as used now.
    app.marks = crate::history::Marks::from_state(&state.history_marks);
    app.seen = state.seen;
    app.field_history = state
        .field_history
        .into_iter()
        .map(|row| (row.key, row.lines))
        .collect();
    app.bookmarks = state.bookmarks;
    // The tree comes up with what the last session walked, so `Ctrl+T` and `Alt+F10` are
    // useful before anything has been read again.
    app.tree_cache = crate::tree::Cache::from_state(&state.tree);
    app.restore_tabs(0, &state.left_tabs, state.left_tab);
    app.restore_tabs(1, &state.right_tabs, state.right_tab);
    if !warnings.is_empty() {
        // Startup problems are shown once, as a dialog: they are easy to miss in a status line.
        app.show_message("Startup warnings", warnings.clone());
    }
    interactive(app)
}

/// A `[[keymap]]` block from the config file wins over a built-in profile of the same name.
fn load_keymap(
    cfg: &Config,
    origin: &str,
    from_plugins: &[(String, toml::Table)],
) -> Result<Keymap> {
    let wanted = cfg.ui.keymap.trim();
    let named = |table: &toml::Table| -> bool {
        table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            == wanted
    };
    // The config file wins over a plugin, so a plugin can never take a keymap away.
    for table in &cfg.keymaps {
        if named(table) {
            return Keymap::from_table(origin, table).map_err(anyhow::Error::new);
        }
    }
    for (plugin_origin, table) in from_plugins {
        if named(table) {
            return Keymap::from_table(plugin_origin, table).map_err(anyhow::Error::new);
        }
    }
    Keymap::builtin(wanted).map_err(anyhow::Error::new)
}

/// What is in the plugins directory, as the discovery sees it.
fn list_plugins(paths: &Paths) {
    let found = fcmd_plug::discover(&paths.plugins_dir);
    println!("{}", paths.plugins_dir.display());
    print!("{}", found.describe());

    // A `.wasm` or a `.lua` in there is somebody waiting for the plugin host. Saying nothing
    // about it would leave them guessing why their plugin does nothing — but saying it about
    // every file reports the README beside them as a plugin that will not load, so the test
    // is on the extension and not merely on being a file.
    const FOR_A_HOST: [&str; 5] = ["wasm", "lua", "so", "dll", "dylib"];
    let mut waiting: Vec<String> = Vec::new();
    if let Ok(read) = std::fs::read_dir(&paths.plugins_dir) {
        for entry in read.flatten() {
            let path = entry.path();
            let wanted = path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| FOR_A_HOST.contains(&e.to_ascii_lowercase().as_str()));
            if path.is_file() && wanted {
                waiting.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    waiting.sort();
    if !waiting.is_empty() {
        println!();
        for name in &waiting {
            println!("?  {name}");
        }
        println!(
            "({} file{} needing a plugin host; there is no plugin host in this build yet — \
             the declarative tier above is a folder with a plugin.toml, see \
             docs/spec/08-plugins.md)",
            waiting.len(),
            if waiting.len() == 1 { "" } else { "s" }
        );
    } else if found.is_empty() && found.problems.is_empty() {
        println!(
            "(nothing here: a plugin is a folder with a plugin.toml — docs/spec/08-plugins.md)"
        );
    }
}

/// Everything the plugins declare, folded into what the program already loads. A plugin that
/// does not load is a warning on the startup screen, never a reason not to start.
fn load_plugins(
    paths: &Paths,
    themes: ThemeSet,
    warnings: &mut Vec<String>,
) -> (ThemeSet, Vec<(String, toml::Table)>, Vec<PluginCommand>) {
    let found = fcmd_plug::discover(&paths.plugins_dir);
    warnings.extend(found.problems.iter().cloned());
    warnings.extend(found.warnings.iter().cloned());
    let mut themes = themes;
    let mut keymaps = Vec::new();
    let mut commands: Vec<PluginCommand> = Vec::new();
    for plugin in &found.plugins {
        if !plugin.supports(env!("CARGO_PKG_VERSION")) {
            warnings.push(format!(
                "{}: declares fcmd {} and this is {} — loaded anyway, it may not fit",
                plugin.id(),
                plugin.manifest.plugin.fcmd,
                env!("CARGO_PKG_VERSION")
            ));
        }
        let theme_tables = plugin.tables(fcmd_plug::ItemKind::Theme);
        if !theme_tables.is_empty() {
            themes = themes.with_plugin_tables(plugin.id(), &theme_tables);
        }
        for table in plugin.tables(fcmd_plug::ItemKind::Keymap) {
            keymaps.push((plugin.origin(), table));
        }
        // The commands it provides, with the program resolved. A name already taken is a
        // warning and the first one wins: two plugins answering to one id would make which of
        // them ran a matter of directory order.
        for (declared, program) in plugin.commands() {
            if let Some(first) = commands.iter().find(|c| c.id == declared.id) {
                warnings.push(format!(
                    "{}: command `{}` is already provided by {} — the second one is ignored",
                    plugin.id(),
                    declared.id,
                    first.plugin
                ));
                continue;
            }
            let scope = declared
                .scope
                .as_deref()
                .and_then(FcmdScope::parse)
                .unwrap_or(FcmdScope::Panel);
            commands.push(PluginCommand {
                plugin: plugin.id().to_string(),
                id: declared.id.clone(),
                label: declared.label.clone(),
                scope,
                program,
                needs: declared.needs.clone(),
                timeout: declared.timeout,
                confirm: declared.confirm.clone(),
            });
        }
    }
    (themes, keymaps, commands)
}

fn command_ids() -> Vec<String> {
    FcCommand::all().into_iter().map(|c| c.id()).collect()
}

fn interactive(mut app: App) -> Result<()> {
    fcmd_tui::install_panic_hook();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("starting the async runtime")?;

    let mut term = Term::new().context("taking over the terminal")?;
    // Only if the configuration asks: see `UiCfg::mouse` for why it is not the default.
    if app.cfg.ui.mouse {
        let _ = term.set_mouse(true);
    }
    let (tx, rx) = mpsc::channel::<Event>();

    // The filesystem watch. It reports on the same channel the listings arrive on, so a
    // directory that changes under us needs no timer in the loop (see `watch`).
    let watch = watch::Watch::start(tx.clone());

    // Terminal input lives on its own thread: `read` blocks.
    let input_tx = tx.clone();
    std::thread::spawn(move || {
        let (raw_tx, raw_rx) = mpsc::channel::<Input>();
        std::thread::spawn(move || fcmd_tui::read_input_loop(raw_tx));
        while let Ok(input) = raw_rx.recv() {
            if input_tx.send(Event::Input(input)).is_err() {
                return;
            }
        }
    });

    // How a terminal's program says it has written something. It is not a key and nobody
    // asked for it, so the loop — which blocks on this very channel — has to be woken.
    {
        let wake_tx = tx.clone();
        app.wake = Some(std::sync::Arc::new(move || {
            let _ = wake_tx.send(Event::Redraw);
        }));
    }

    // One pool of connections, shared by the panels, the viewer and every job.
    let pool = Pool::new();
    if !app.cfg.net.known_hosts.trim().is_empty() {
        pool.set_policy(fcmd_net::HostKeyPolicy {
            known_hosts: Some(fcmd_net::site::expand_tilde(app.cfg.net.known_hosts.trim())),
            accept_new: false,
        });
    }
    // The tunnels live here, beside the connection pool: their listeners and their copying
    // run on the runtime, and the panels only ever see what `tunnel_infos` describes.
    let tunnels: Tunnels = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let objects = std::sync::Arc::new(object_storage(&app.cfg));
    let vfs = Router::with_limits(pool.clone(), app.cfg.arc.limits()).with_objects(objects.clone());
    // One container cache for the whole program: the index a panel read to draw itself is the
    // one an unpack then reads from.
    let archives = vfs.archives().cache();
    // And the way to reach a container that is not a file on this disk: an archive on a server
    // or in a bucket is copied here, once, and read from the copy (SPEC 04 §4).
    archives.set_fetch(std::sync::Arc::new(BringHere {
        pool: pool.clone(),
        objects: objects.clone(),
        handle: runtime.handle().clone(),
    }));
    // And the way to reach a `descript.ion` wherever the panel stands: the same cache for a
    // container, and the pool and the bucket for a server (SPEC 02 §13).
    let sides = Sides {
        archives: archives.clone(),
        away: Some(Away {
            pool: pool.clone(),
            objects: objects.clone(),
            handle: runtime.handle().clone(),
        }),
    };
    let mut cancels: [Option<CancelToken>; 2] = [None, None];
    let (mut w, mut h) = term.size();
    let mut buf = Buffer::new(w, h);

    loop {
        let (nw, nh) = term.size();
        if (nw, nh) != (w, h) {
            (w, h) = (nw, nh);
            buf.resize(w, h);
            term.invalidate();
        }
        let cursor = render::draw(&mut app, &mut buf);
        term.draw(&buf, cursor)?;

        for request in app.take_requests() {
            match request {
                Request::Read {
                    panel,
                    path,
                    generation,
                    facts,
                } => {
                    // Watch what we are about to read, if it is a directory a watch can see:
                    // a remote one has no local events, and one inside an archive is a file.
                    watch.set(
                        panel,
                        (app.cfg.panel.auto_refresh && path.is_local() && !path.in_container())
                            .then(|| path.os().to_path_buf()),
                    );
                    if let Some(old) = cancels[panel].replace(CancelToken::new()) {
                        old.cancel();
                    }
                    let token = cancels[panel].clone().expect("just set");
                    let tx = tx.clone();
                    let vfs = vfs.clone();
                    runtime.spawn(async move {
                        // `list_dir` rather than `list`: a provider that stopped before the
                        // end of the directory says so here, and the panel has to be told.
                        let read = vfs.list_dir(&path, token).await.map_err(|e| e.to_string());
                        let mut cut = None;
                        let result = read.map(|dir| {
                            if !dir.complete {
                                cut = dir
                                    .note
                                    .clone()
                                    .or_else(|| Some("this listing is not all of it".to_string()));
                            }
                            dir.entries
                        });
                        let _ = tx.send(Event::Listed {
                            panel,
                            generation,
                            result,
                        });
                        if let Some(note) = cut {
                            let _ = tx.send(Event::ListingCut {
                                panel,
                                generation,
                                note,
                            });
                        }
                        let _ = tx.send(Event::FreeSpace {
                            panel,
                            disk: device(&path),
                        });
                        // Where this directory is, for the information panel: the mount is one
                        // small file, the git index is a `stat` per tracked file, and neither
                        // is read unless something on screen is showing it.
                        if facts && path.is_local() && !path.in_container() {
                            let local = path.os().to_path_buf();
                            let _ = tx.send(Event::Place {
                                panel,
                                mount: fcmd_platform::mount_of(&local),
                                git: fcmd_core::git::describe(&local).map(Box::new),
                            });
                        }
                    });
                }
                Request::Job { id, spec, cancel } => {
                    let tx = tx.clone();
                    // A job runs on a blocking thread, which is what lets the remote
                    // provider's async calls be driven with `block_on` (see `fcmd-net`).
                    let fs = ArcFs::new(
                        archives.clone(),
                        ObjectFs::new(
                            objects.clone(),
                            RoutedFs::new(pool.clone(), runtime.handle().clone()),
                            runtime.handle().clone(),
                        ),
                    );
                    runtime.spawn_blocking(move || run_job(id, spec, cancel, tx, &fs));
                }
                Request::SetAttrs {
                    id,
                    paths,
                    change,
                    recursive,
                    cancel,
                } => {
                    let tx = tx.clone();
                    // The same filesystem a copy uses, so a path on a server or in a bucket
                    // reaches the provider that owns it and is refused by the one that has
                    // nothing to set.
                    let fs = ArcFs::new(
                        archives.clone(),
                        ObjectFs::new(
                            objects.clone(),
                            RoutedFs::new(pool.clone(), runtime.handle().clone()),
                            runtime.handle().clone(),
                        ),
                    );
                    runtime.spawn_blocking(move || {
                        run_attrs(id, &paths, &change, recursive, cancel, tx, &fs)
                    });
                }
                Request::Shell {
                    id,
                    line,
                    cwd,
                    into,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        // The lines go as they are printed, not at the end: a build that
                        // takes a minute is a minute of nothing to look at otherwise.
                        let report = tx.clone();
                        let (output, status) = run_shell(&line, &cwd, into.as_deref(), &|text| {
                            let _ = report.send(Event::ShellOutput { id, text });
                        });
                        let _ = tx.send(Event::ShellDone {
                            id,
                            line,
                            output,
                            status,
                        });
                    });
                }
                Request::ServiceControl { side, id, action } => {
                    let tx = tx.clone();
                    // Off the loop: a service manager takes seconds over a restart, and the
                    // panels have to keep answering while it does.
                    runtime.spawn_blocking(move || {
                        let result = fcmd_platform::control(&id, action);
                        let _ = tx.send(Event::ServiceControlled {
                            side,
                            id,
                            action,
                            result,
                        });
                    });
                }
                Request::Decompile { id, rule, path } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        let tool = rule.name.clone();
                        let result = run_decompiler(&rule, &path);
                        let _ = tx.send(Event::Decompiled { id, tool, result });
                    });
                }
                Request::Connect {
                    panel,
                    site,
                    secret,
                    accept_host_key,
                } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    runtime.spawn(async move {
                        if accept_host_key {
                            pool.accept_new_host_key(true);
                        }
                        let outcome = match pool.connect(&site, secret.as_deref()).await {
                            Ok(remote) => {
                                // Ask the server where we are: a site may say `.` or a
                                // relative directory, and `~` is the login directory.
                                let start = remote
                                    .canonicalize(&site.start())
                                    .await
                                    .unwrap_or_else(|_| site.start());
                                ConnectOutcome::Open {
                                    start,
                                    connected: pool.connected(),
                                }
                            }
                            Err(fcmd_net::NetError::NeedsSecret { kind, .. }) => {
                                ConnectOutcome::NeedsSecret(kind)
                            }
                            Err(fcmd_net::NetError::UnknownHostKey { fingerprint, .. }) => {
                                ConnectOutcome::UnknownHostKey { fingerprint }
                            }
                            Err(fcmd_net::NetError::ChangedHostKey { path, line, .. }) => {
                                ConnectOutcome::ChangedHostKey {
                                    path: path.display().to_string(),
                                    line,
                                }
                            }
                            Err(e) => ConnectOutcome::Failed(e.to_string()),
                        };
                        let _ = tx.send(Event::Connected {
                            panel,
                            site,
                            outcome,
                        });
                    });
                }
                Request::Disconnect { key } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    runtime.spawn(async move {
                        pool.disconnect(&key).await;
                        let _ = tx.send(Event::Disconnected {
                            key,
                            connected: pool.connected(),
                        });
                    });
                }
                Request::Fetch { id, path, purpose } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    let archives = archives.clone();
                    let objects_for_fetch = objects.clone();
                    runtime.spawn(async move {
                        let result = if path.in_container() {
                            let inner = path.clone();
                            tokio::task::spawn_blocking(move || fetch_entry(&archives, &inner))
                                .await
                                .unwrap_or_else(|e| Err(format!("the unpack failed: {e}")))
                        } else if path.scheme() == fcmd_core::Scheme::S3 {
                            fetch_object(&objects_for_fetch, &path).await
                        } else {
                            fetch_remote(&pool, &path).await
                        };
                        let _ = tx.send(Event::Fetched {
                            id,
                            remote: path,
                            purpose,
                            result,
                        });
                    });
                }
                Request::Upload {
                    id,
                    local,
                    remote,
                    expect,
                    force,
                } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    let objects_for_upload = objects.clone();
                    runtime.spawn(async move {
                        let result = if remote.scheme() == fcmd_core::Scheme::S3 {
                            put_object(&objects_for_upload, &local, &remote, expect, force).await
                        } else {
                            write_back(&pool, &local, &remote, expect, force).await
                        };
                        let _ = tx.send(Event::Uploaded { id, remote, result });
                    });
                }
                Request::Find { id, spec, cancel } => {
                    let tx = tx.clone();
                    let archives = archives.clone();
                    // The named mask lists as they are now: the editor in the Options menu
                    // can have changed them since the program started.
                    let groups = app.cfg.mask_groups();
                    // A blocking thread: the walk is synchronous, and it must not sit in the
                    // way of the runtime that is also serving the panels.
                    runtime.spawn_blocking(move || {
                        run_find(id, *spec, cancel, tx, &archives, &groups)
                    });
                }
                Request::CompareContents {
                    id,
                    pairs,
                    work,
                    cancel,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || run_compare(id, pairs, work, cancel, tx));
                }
                Request::Duplicates { id, root, cancel } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || run_duplicates(id, root, cancel, tx));
                }
                Request::CompareTrees {
                    id,
                    left,
                    right,
                    work,
                    cancel,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        run_compare_trees(id, left, right, work, cancel, tx)
                    });
                }
                Request::Checksums {
                    id,
                    paths,
                    algorithm,
                    target,
                    cancel,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        run_checksums(id, &paths, algorithm, target, cancel, tx)
                    });
                }
                Request::Verify { id, list, cancel } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || run_verify(id, &list, cancel, tx));
                }
                // The only request the loop answers itself: the terminal is here, and asking
                // it for a font is one write and no waiting.
                Request::Font { by } => {
                    let _ = term.font_step(by);
                }
                Request::Clipboard { text } => {
                    // Nothing to report: the terminal does not answer, and a terminal that
                    // does not understand the request ignores it.
                    let _ = term.set_clipboard(&text);
                }
                Request::Owners {
                    panel,
                    generation,
                    dir,
                    names,
                } => {
                    // A thread of its own: a lookup per file, and on Windows one that can
                    // cross a network to a domain controller. The panel is already on screen
                    // and the two columns fill in when the answer arrives.
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        let owners = owners_of(&dir, &names);
                        let _ = tx.send(Event::Owners {
                            panel,
                            generation,
                            owners,
                        });
                    });
                }
                Request::Links {
                    panel,
                    generation,
                    dir,
                    names,
                } => {
                    // A thread of its own: a lookup per file, and on Windows one that opens
                    // each file to ask. The panel is already on screen and the column fills
                    // in when the answer arrives.
                    let tx = tx.clone();
                    std::thread::spawn(move || {
                        let links = links_of(&dir, &names);
                        let _ = tx.send(Event::Links {
                            panel,
                            generation,
                            links,
                        });
                    });
                }
                Request::Descriptions {
                    panel,
                    generation,
                    dir,
                    rules,
                } => {
                    // One small file, off the UI thread because it is still a read from a
                    // disk that may be a network share pretending to be one.
                    let tx = tx.clone();
                    let sides = sides.clone();
                    runtime.spawn_blocking(move || {
                        let descriptions = read_descriptions(&dir, &sides, &rules);
                        let _ = tx.send(Event::Descriptions {
                            panel,
                            generation,
                            descriptions,
                        });
                    });
                }
                Request::WriteDescriptions {
                    dir,
                    descriptions,
                    rules,
                } => {
                    let sides = sides.clone();
                    runtime.spawn_blocking(move || {
                        write_descriptions(&dir, &descriptions, &sides, &rules)
                    });
                }
                Request::CarryDescriptions(carry) => {
                    let sides = sides.clone();
                    runtime.spawn_blocking(move || carry_descriptions(&carry, &sides));
                }
                Request::Mouse { on } => {
                    // A terminal that was never asked to report the mouse is not asked to
                    // stop, and one that is already reporting is not asked twice: `set_mouse`
                    // answers both, and a failure is the terminal's to refuse.
                    let _ = term.set_mouse(on);
                }
                Request::SyncPlan {
                    id,
                    left,
                    right,
                    options,
                    cancel,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        run_sync_plan(id, &left, &right, &options, cancel, tx)
                    });
                }
                Request::TunnelOpen {
                    id,
                    authority,
                    spec,
                } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    let tunnels = tunnels.clone();
                    runtime.spawn(async move {
                        let event = match open_tunnel(&pool, &tunnels, id, &authority, &spec).await
                        {
                            Ok(()) => Event::Tunnels {
                                items: tunnel_infos(&tunnels),
                            },
                            Err(error) => Event::TunnelFailed { spec, error },
                        };
                        let _ = tx.send(event);
                    });
                }
                Request::TunnelClose { id } => {
                    // Dropping the tunnel closes its listener and every connection through
                    // it: there is nothing else to undo.
                    tunnels.lock().expect("the tunnels").retain(|t| t.id != id);
                    let _ = tx.send(Event::Tunnels {
                        items: tunnel_infos(&tunnels),
                    });
                }
                Request::TunnelList => {
                    let _ = tx.send(Event::Tunnels {
                        items: tunnel_infos(&tunnels),
                    });
                }
                Request::TempRefresh {
                    panel,
                    generation,
                    title,
                    paths,
                } => {
                    // A list of results is not a directory: there is nothing to watch.
                    watch.set(panel, None);
                    let tx = tx.clone();
                    let vfs = vfs.clone();
                    runtime.spawn(async move {
                        // A file that has gone since the list was built simply drops out: a
                        // results panel still showing what was deleted is a liar.
                        let mut items = Vec::with_capacity(paths.len());
                        for path in paths {
                            if let Ok(entry) = vfs.stat(&path).await {
                                items.push((path, entry));
                            }
                        }
                        let _ = tx.send(Event::TempListed {
                            panel,
                            generation,
                            title,
                            items,
                        });
                    });
                }
                Request::TreeScan {
                    root,
                    limit,
                    cancel,
                } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        let (dirs, whole) = scan_tree(&root, limit, &cancel);
                        let _ = tx.send(Event::TreeScanned { root, dirs, whole });
                    });
                }
                Request::TreeRead { panel, path } => {
                    let tx = tx.clone();
                    let vfs = vfs.clone();
                    runtime.spawn(async move {
                        // Only the subdirectories: a tree does not show files, and the panel
                        // that does is right beside it. A symbolic link that points at a
                        // directory is one — on macOS `/tmp`, `/var` and `/etc` all are, and
                        // a tree that dropped them could not show where the user is.
                        let dirs = match vfs.list(&path, CancelToken::new()).await {
                            Ok(entries) => entries
                                .into_iter()
                                .filter_map(|e| {
                                    let child = path.join(&e.name);
                                    match e.kind {
                                        fcmd_core::EntryKind::Dir => Some(child),
                                        fcmd_core::EntryKind::Symlink
                                            if child.is_local()
                                                && std::fs::metadata(child.as_path())
                                                    .map(|m| m.is_dir())
                                                    .unwrap_or(false) =>
                                        {
                                            Some(child)
                                        }
                                        _ => None,
                                    }
                                })
                                .collect(),
                            // A directory we cannot read is an empty branch rather than an
                            // error in a dialog: the tree is a way of looking around.
                            Err(_) => Vec::new(),
                        };
                        let _ = tx.send(Event::TreeListed { panel, path, dirs });
                    });
                }
                Request::Pack {
                    id,
                    archive,
                    format,
                    level,
                    solid,
                    sources,
                    cancel,
                } => {
                    let tx = tx.clone();
                    let archives = archives.clone();
                    runtime.spawn_blocking(move || {
                        run_pack(
                            id, archive, format, level, solid, sources, cancel, tx, &archives,
                        )
                    });
                }
                Request::ArcPassword { archive, password } => {
                    // Straight into the cache, and not on a thread: the read that follows is
                    // the next request in this queue and has to find the password already
                    // there.
                    archives.remember_password(&archive, &password);
                }
                // A command whose output is a list of paths rather than a screenful of
                // text: it runs on the pool like any other, and what it printed comes back
                // as lines (SPEC 02 §7b).
                Request::Panelize { id, line, cwd } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        let lines = std::sync::Mutex::new(Vec::new());
                        let (_, status) = run_shell(&line, &cwd, None, &|text| {
                            if let Ok(mut lines) = lines.lock() {
                                lines.extend(text.lines().map(str::to_string));
                            }
                        });
                        let lines = lines.into_inner().unwrap_or_default();
                        let _ = tx.send(Event::Panelized {
                            id,
                            line,
                            lines,
                            status,
                        });
                    });
                }
                Request::ArcTest {
                    id,
                    archive,
                    cancel,
                } => {
                    let tx = tx.clone();
                    let archives = archives.clone();
                    runtime.spawn_blocking(move || run_test(id, archive, cancel, tx, &archives));
                }
                Request::ArcRebuild {
                    id,
                    archive,
                    changes,
                    level,
                    cancel,
                } => {
                    let tx = tx.clone();
                    let archives = archives.clone();
                    runtime.spawn_blocking(move || {
                        run_rebuild(id, archive, changes, level, cancel, tx, &archives)
                    });
                }
                Request::Plugin {
                    id,
                    label,
                    program,
                    cwd,
                    ctx,
                    timeout,
                } => {
                    let tx = tx.clone();
                    // Somebody else's program, on the worker pool: the interface stays
                    // answerable while it thinks, and a plugin that hangs is its own problem
                    // rather than everybody's.
                    runtime.spawn_blocking(move || {
                        let result =
                            fcmd_plug::host::run_command(&program, cwd.as_deref(), &ctx, timeout)
                                .map_err(|e| e.to_string());
                        let _ = tx.send(Event::PluginDone { id, label, result });
                    });
                }
                Request::Link { target, link, kind } => {
                    let tx = tx.clone();
                    let fs = ObjectFs::new(
                        objects.clone(),
                        RoutedFs::new(pool.clone(), runtime.handle().clone()),
                        runtime.handle().clone(),
                    );
                    runtime.spawn_blocking(move || {
                        let error = fcmd_jobs::create_link(&fs, &target, &link, kind)
                            .err()
                            .map(|e| e.to_string());
                        let _ = tx.send(Event::LinkDone { link, error });
                    });
                }
                Request::DirSize { panel, path } => {
                    let tx = tx.clone();
                    // A directory inside an archive has a size too, and it is what the
                    // archive says rather than what a disk would give it.
                    let fs = ArcFs::new(
                        archives.clone(),
                        ObjectFs::new(
                            objects.clone(),
                            RoutedFs::new(pool.clone(), runtime.handle().clone()),
                            runtime.handle().clone(),
                        ),
                    );
                    runtime.spawn_blocking(move || {
                        let (bytes, files) = dir_size(&fs, &path, &CancelToken::new(), 0);
                        let _ = tx.send(Event::DirSized {
                            panel,
                            path,
                            bytes,
                            files,
                        });
                    });
                }
            }
        }

        if app.quit {
            break;
        }

        // The loop is input-driven: it blocks until something happens, which is why the
        // program costs nothing while nobody is touching it. The clock is the one thing that
        // changes on its own, so when it is on the wait has a deadline — the moment the
        // minute turns — and the frame after it draws the new time (SPEC 10 §3.3).
        let event = if app.cfg.ui.clock {
            let secs = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| 60 - d.as_secs() % 60)
                .unwrap_or(60);
            match rx.recv_timeout(std::time::Duration::from_secs(secs.max(1))) {
                Ok(event) => Some(event),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => None,
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
            }
        } else {
            match rx.recv() {
                Ok(event) => Some(event),
                Err(_) => break,
            }
        };
        if let Some(event) = event {
            app.handle_event(event);
        }
        // Coalesce whatever else is already queued, then draw once.
        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }
        // `Alt+Ins` asked for the screen grabber, and the screen it wants is the frame that
        // is on the terminal — which is this buffer, still holding what was last drawn
        // (SPEC 06 §11b). It is handed over here rather than in the key handler because the
        // key handler cannot see it, and now rather than next time round because the grabber
        // has to be on the screen before the loop waits for another key.
        if app.grab_wanted {
            app.open_grab(&buf);
        }
    }

    term.restore();

    // What survives the session: the command history, and where the panels ended up. A failure
    // here is reported on the way out rather than swallowed — losing a history silently is how
    // a program teaches people not to trust it.
    // The lines nobody has pinned and nobody has used for `ui.history_lifetime` days go
    // here, once, where all three histories are in hand (SPEC 06 §6).
    app.sweep_histories();
    // FAR's *Auto save setup*: the settings the dialogs changed, written without having been
    // asked to. Off by default — a setting changed to see what it does should not become
    // permanent by quitting — and the panels' folders go into the state file below either
    // way, which is what brings the next start back where this one ended.
    if app.cfg.ui.save_setup_on_exit {
        app.save_setup();
    }
    let state = fcmd_config::State {
        command_history: app.cmd.history().to_vec(),
        file_history: app.file_history.clone(),
        folder_history: app.panels[app.active]
            .history
            .iter()
            .map(|p| p.to_string())
            .collect(),
        left: app.panels[0].cwd.to_string(),
        right: app.panels[1].cwd.to_string(),
        left_tabs: app.tabs[0].tabs_state(&app.panels[0]),
        right_tabs: app.tabs[1].tabs_state(&app.panels[1]),
        left_tab: app.tabs[0].current(),
        right_tab: app.tabs[1].current(),
        bookmarks: app.bookmarks.clone(),
        // The tree's own cache, capped: a walk of a whole disk is worth keeping some of, and
        // a state file nobody can open is worth keeping none of.
        tree: app.tree_cache.to_state(crate::app::TREE_CACHE_KEPT),
        history_marks: app.marks.to_state(),
        // Where each file was left, so the next `F3` on it opens there (SPEC 05 §1.12). The
        // viewer still on the screen at this moment counts, which is the one the user is most
        // likely to come back to.
        seen: {
            if let Some(viewer) = app.live_viewer() {
                let row = crate::app::App::seen_now(viewer);
                app.remember_seen(row);
            }
            app.seen.clone()
        },
        field_history: app
            .field_history
            .iter()
            .filter(|(_, lines)| !lines.is_empty())
            .map(|(key, lines)| fcmd_config::FieldHistory {
                key: key.clone(),
                lines: lines.clone(),
            })
            .collect(),
    };
    if let Err(e) = state.save(&app.paths) {
        eprintln!("fcmd: cannot write {}: {e}", app.paths.state_file.display());
    }
    Ok(())
}

/// Run one job to completion on a worker thread, reporting through the event channel.
///
/// `ask` blocks this thread until the UI answers, which is exactly what the engine wants: the
/// UI thread keeps drawing while the job waits.
fn run_job(
    id: u64,
    spec: JobSpec,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    fs: &dyn fcmd_jobs::JobFs,
) {
    struct ChannelReporter {
        id: u64,
        tx: mpsc::Sender<Event>,
        last: std::time::Instant,
    }

    impl fcmd_jobs::Reporter for ChannelReporter {
        fn progress(&mut self, progress: &fcmd_jobs::Progress) {
            // 20 frames a second is plenty; a tight copy loop must not flood the channel.
            let now = std::time::Instant::now();
            let done = progress.phase == fcmd_jobs::Phase::Done;
            if !done && now.duration_since(self.last) < std::time::Duration::from_millis(50) {
                return;
            }
            self.last = now;
            let _ = self.tx.send(Event::JobProgress {
                id: self.id,
                progress: progress.clone(),
            });
        }

        fn ask(&mut self, question: fcmd_jobs::Question) -> fcmd_jobs::Decision {
            let (reply, answer) = mpsc::channel();
            if self
                .tx
                .send(Event::JobQuestion {
                    id: self.id,
                    question,
                    reply,
                })
                .is_err()
            {
                return fcmd_jobs::Decision::Cancel;
            }
            answer.recv().unwrap_or(fcmd_jobs::Decision::Cancel)
        }
    }

    let kind = spec.kind;
    let mut reporter = ChannelReporter {
        id,
        tx: tx.clone(),
        last: std::time::Instant::now() - std::time::Duration::from_secs(1),
    };

    let outcome = match fcmd_jobs::plan(fs, &spec, &cancel) {
        Ok(plan) => fcmd_jobs::execute(fs, &spec, plan, &mut reporter, &cancel),
        Err(e) => fcmd_jobs::Outcome {
            errors: vec![(
                spec.target.clone().unwrap_or_else(|| VPath::local(".")),
                e.to_string(),
            )],
            ..fcmd_jobs::Outcome::default()
        },
    };
    let _ = tx.send(Event::JobDone { id, kind, outcome });
}

/// Run an external decompiler and capture its text.
///
/// The command is executed **directly**, never through a shell, so a path with spaces or
/// shell characters cannot turn into a command. Output is capped: a decompiler pointed at the
/// wrong file can print for ever.
/// Run a command line the way a command line is meant to run: through the user's own shell, so
/// pipes, redirections and aliases work as they type them. Output is merged and capped; a file
/// given by `run:file<command` gets it instead of the user screen.
///
/// The two reader threads are not decoration: a child that fills the 64 KiB pipe buffer while
/// we wait for it to exit deadlocks, so somebody has to be draining both pipes meanwhile.
/// Run a line in the user's shell, reporting what it prints as it prints it.
///
/// `report` is handed whole lines as they arrive, which is what makes a long command watchable
/// rather than a minute of nothing followed by everything. What comes back is the *note* the
/// run has to add — a spawn failure, where the output was written, that it was truncated —
/// and not the output itself, which has already gone through `report`. The one exception is a
/// run with a file to write into: nothing of that is printed, so nothing is reported.
fn run_shell(
    line: &str,
    cwd: &std::path::Path,
    into: Option<&std::path::Path>,
    report: &dyn Fn(String),
) -> (String, Option<i32>) {
    use std::io::Read;
    use std::process::{Command, Stdio};
    const MAX_OUTPUT: usize = 4 * 1024 * 1024;

    let mut command = if cfg!(windows) {
        let mut c = Command::new(std::env::var("COMSPEC").unwrap_or_else(|_| "cmd.exe".into()));
        c.arg("/C").arg(line);
        c
    } else {
        let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".into());
        let mut c = Command::new(shell);
        c.arg("-c").arg(line);
        c
    };
    command
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(e) => return (format!("{line}: {e}\n"), Some(-1)),
    };

    // Both pipes are pumped into one channel, so the two come out interleaved the way they
    // were printed rather than all of one and then all of the other. A thread each, because
    // reading one pipe while the other fills is how a process that writes to both deadlocks.
    let mut pipes: Vec<Box<dyn Read + Send>> = Vec::new();
    if let Some(pipe) = child.stdout.take() {
        pipes.push(Box::new(pipe));
    }
    if let Some(pipe) = child.stderr.take() {
        pipes.push(Box::new(pipe));
    }
    let (chunks, arriving) = std::sync::mpsc::channel::<Vec<u8>>();
    let pumps: Vec<_> = pipes
        .into_iter()
        .map(|mut pipe| {
            let chunks = chunks.clone();
            std::thread::spawn(move || {
                let mut buf = [0u8; 8 * 1024];
                loop {
                    match pipe.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if chunks.send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                    }
                }
            })
        })
        .collect();
    drop(chunks); // the loop below ends when the last pump has dropped its own

    let mut bytes = Vec::new();
    // What has been read but is not yet a whole line. A line is reported when its newline
    // arrives, so a cell of a progress bar does not go out one character at a time.
    let mut partial: Vec<u8> = Vec::new();
    let mut truncated = false;
    let streaming = into.is_none();
    for chunk in arriving {
        if bytes.len() + chunk.len() > MAX_OUTPUT {
            truncated = true;
            if !streaming {
                // The file gets what fits and nothing after it.
                let room = MAX_OUTPUT.saturating_sub(bytes.len());
                bytes.extend_from_slice(&chunk[..room]);
            }
            continue;
        }
        if !streaming {
            bytes.extend_from_slice(&chunk);
            continue;
        }
        bytes.extend_from_slice(&chunk);
        partial.extend_from_slice(&chunk);
        if let Some(at) = partial.iter().rposition(|b| *b == b'\n') {
            let whole: Vec<u8> = partial.drain(..=at).collect();
            report(String::from_utf8_lossy(&whole).into_owned());
        }
    }
    for pump in pumps {
        let _ = pump.join();
    }
    // A last line with no newline after it is still a line.
    if streaming && !partial.is_empty() {
        report(String::from_utf8_lossy(&partial).into_owned());
    }
    let status = child.wait().ok().and_then(|s| s.code());

    if let Some(path) = into {
        let text = String::from_utf8_lossy(&bytes).into_owned();
        return match std::fs::write(path, text.as_bytes()) {
            Ok(()) => (format!("output written to {}\n", path.display()), status),
            Err(e) => (format!("{}: {e}\n", path.display()), Some(-1)),
        };
    }
    // The output has gone already, line by line; what is left to say is whether there was
    // more of it than we were willing to keep.
    let note = if truncated {
        "… output truncated\n".to_string()
    } else {
        String::new()
    };
    (note, status)
}

/// How many names each of these files has, by name (SPEC 02 §3).
///
/// A file nobody can ask about is left out rather than given a one: the column is then blank,
/// which says "not known" where a 1 would say "only this name".
fn links_of(dir: &VPath, names: &[String]) -> std::collections::HashMap<String, u64> {
    let here = dir.os();
    names
        .iter()
        .filter_map(|name| fcmd_platform::links(&here.join(name)).map(|n| (name.clone(), n)))
        .collect()
}

/// Where a `descript.ion` can be, and the one value that knows how to get at it.
///
/// Beside the files on this disk, inside the container they are in, or on the server or in
/// the bucket they came from: a panel may stand in any of the four, and the side file follows
/// its directory wherever that is.
#[derive(Clone)]
struct Sides {
    archives: std::sync::Arc<fcmd_arc::Cache>,
    /// `None` where there is no network to reach: the tests' pump, which never leaves the
    /// local disk, and a program started with neither a connection nor a bucket.
    away: Option<Away>,
}

impl Sides {
    /// The disk and the containers on it, and nothing further: what the tests' pump has.
    #[cfg(test)]
    fn here(archives: std::sync::Arc<fcmd_arc::Cache>) -> Sides {
        Sides {
            archives,
            away: None,
        }
    }
}

/// The three verbs a side file needs when its directory is not on this disk.
///
/// Small files only, and blocking: a `descript.ion` is a few kilobytes and these run on a
/// worker's thread, which is where the rest of the description reading already runs.
#[derive(Clone)]
struct Away {
    pool: std::sync::Arc<Pool>,
    objects: std::sync::Arc<fcmd_s3::vfs::S3Vfs>,
    handle: tokio::runtime::Handle,
}

impl Away {
    /// The bytes, or nothing: a file that is not there, a connection that is not open and a
    /// file too big to be a list of descriptions all mean the same thing to the caller.
    fn read(&self, path: &VPath, limit: u64) -> Option<Vec<u8>> {
        let cancel = CancelToken::new();
        if path.scheme() == fcmd_core::Scheme::S3 {
            let entry = self
                .handle
                .block_on(self.objects.stat_object(path, &cancel))
                .ok()?;
            if entry.size > limit {
                return None;
            }
            return self
                .handle
                .block_on(self.objects.read_object(path, None, &cancel))
                .ok();
        }
        let remote = self.pool.require(path).ok()?;
        if self.handle.block_on(stamp_of(&remote, path))?.size > limit {
            return None;
        }
        let wire = remote.wire(path).ok()?;
        self.handle.block_on(async move {
            use tokio::io::AsyncReadExt;
            let mut reader = remote.sftp().open_read(&wire).await.ok()?;
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.ok()?;
            Some(bytes)
        })
    }

    /// Put a side file there, whole. The bytes rather than the text, because a
    /// `descript.ion` may go back in the code page it was read in (SPEC 02 §13).
    fn write_bytes(&self, path: &VPath, bytes: Vec<u8>) -> Result<(), String> {
        if path.scheme() == fcmd_core::Scheme::S3 {
            return self
                .handle
                .block_on(self.objects.write_bytes(path, bytes, &CancelToken::new()))
                .map(|_| ())
                .map_err(|e| format!("{path}: {e}"));
        }
        let remote = self
            .pool
            .require(path)
            .map_err(|e| format!("{path}: {e}"))?;
        let wire = remote.wire(path).map_err(|e| format!("{path}: {e}"))?;
        self.handle
            .block_on(async move {
                use tokio::io::AsyncWriteExt;
                let mut writer = remote
                    .sftp()
                    .create(&wire)
                    .await
                    .map_err(|e| e.to_string())?;
                writer.write_all(&bytes).await.map_err(|e| e.to_string())?;
                writer.shutdown().await.map_err(|e| e.to_string())
            })
            .map_err(|e| format!("{path}: {e}"))
    }

    /// Take the file away. One that is not there is already gone, which is not an error.
    fn remove(&self, path: &VPath) -> Result<(), String> {
        let cancel = CancelToken::new();
        if path.scheme() == fcmd_core::Scheme::S3 {
            if self
                .handle
                .block_on(self.objects.stat_object(path, &cancel))
                .is_err()
            {
                return Ok(());
            }
            return self
                .handle
                .block_on(self.objects.delete_object(path, &cancel))
                .map_err(|e| format!("{path}: {e}"));
        }
        let remote = self
            .pool
            .require(path)
            .map_err(|e| format!("{path}: {e}"))?;
        if !self.handle.block_on(remote.exists(path)) {
            return Ok(());
        }
        self.handle
            .block_on(remote.remove_file(path))
            .map_err(|e| format!("{path}: {e}"))
    }
}

/// A directory's `descript.ion`, or whatever it is called here.
///
/// The names are tried in order and the first file that is there is the one that counts; a
/// directory with neither has no descriptions, which is not an error and not worth a message.
/// The text is decoded the way the viewer decodes a text file — a `descript.ion` written under
/// DOS is in a DOS code page, and one written today is UTF-8.
fn read_descriptions(
    dir: &VPath,
    sides: &Sides,
    rules: &fcmd_core::descriptions::Rules,
) -> fcmd_core::descriptions::Descriptions {
    use fcmd_core::descriptions::Descriptions;
    const LIMIT: u64 = 4 * 1024 * 1024;

    // Inside an archive the file is an entry like any other, and the index has already been
    // read: looking for it costs a lookup rather than a round trip, which is why a panel in
    // a container may ask for descriptions at all.
    if dir.in_container() {
        let Ok((archive, inner)) = sides.archives.open_in(dir, &CancelToken::new()) else {
            return Descriptions::default();
        };
        for name in &rules.names {
            let at = if inner == "/" {
                format!("/{name}")
            } else {
                format!("{inner}/{name}")
            };
            let Some(entry) = archive.entry(&at) else {
                continue;
            };
            if entry.kind == fcmd_core::EntryKind::Dir || entry.size > LIMIT {
                continue;
            }
            let Ok(bytes) = archive.read(&at, &CancelToken::new()) else {
                continue;
            };
            let codepage = fcmd_view::decode::detect(&bytes);
            return Descriptions::parse(&codepage.decode(&bytes));
        }
        return Descriptions::default();
    }
    // On a server and in a bucket it is one small file fetched whole, in the same code page
    // rule as everywhere else. The panel asks for it only when a column or the information
    // panel shows one, so this is a round trip somebody is waiting to see the result of.
    if !dir.is_local() {
        let Some(away) = &sides.away else {
            return Descriptions::default();
        };
        for name in &rules.names {
            let Some(bytes) = away.read(&dir.join(name), LIMIT) else {
                continue;
            };
            let codepage = fcmd_view::decode::detect(&bytes);
            return Descriptions::parse(&codepage.decode(&bytes));
        }
        return Descriptions::default();
    }
    let here = dir.os();
    for name in &rules.names {
        // The name is matched as the filesystem matches it: `DESCRIPT.ION` is the same file
        // on Windows and a different one on Linux, which is the filesystem's business.
        let path = here.join(name);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        if !meta.is_file() || meta.len() > LIMIT {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let codepage = fcmd_view::decode::detect(&bytes);
        return Descriptions::parse(&codepage.decode(&bytes));
    }
    Descriptions::default()
}

/// Write one back, in UTF-8, or take the file away when there is nothing left to say.
///
/// The file is written beside the directory it describes, which is the whole point of the
/// format; a directory nobody may write to keeps the descriptions it had, and the message
/// the caller sees is the one the filesystem — or the server — gave. Inside a container it
/// is a rebuild, which `Ctrl+Z` refuses before it gets this far.
fn write_descriptions(
    dir: &VPath,
    descriptions: &fcmd_core::descriptions::Descriptions,
    sides: &Sides,
    rules: &fcmd_core::descriptions::Rules,
) -> Result<(), String> {
    if !dir.is_local() || dir.in_container() {
        let Some(away) = sides.away.as_ref().filter(|_| !dir.in_container()) else {
            return Err(format!("{dir}: descriptions cannot be written here"));
        };
        let path = dir.join(rules.write_name());
        return if descriptions.is_empty() {
            away.remove(&path)
        } else {
            let old = away.read(&path, 4 * 1024 * 1024);
            let bytes = description_bytes(old.as_deref(), &descriptions.to_text(), rules);
            away.write_bytes(&path, bytes)
        };
    }
    let path = dir.os().join(rules.write_name());
    if descriptions.is_empty() {
        // An empty file is a file that says nothing and still has to be carried around.
        return match std::fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        };
    }
    let old = std::fs::read(&path).ok();
    let bytes = description_bytes(old.as_deref(), &descriptions.to_text(), rules);
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))
}

/// The bytes a `descript.ion` is written as (SPEC 02 §9).
///
/// UTF-8 unless the rules say otherwise, and then the code page the file was already in —
/// there is nothing else to go on, and guessing the machine's own page would turn a Russian
/// `descript.ion` into a Latin one on a Latin machine. A file that is not there yet, and one
/// that was already UTF-8 or UTF-16, is written in UTF-8 either way.
///
/// A page that cannot hold what somebody typed would write a `?` in its place, which is a
/// description quietly ruined, so the file goes out in UTF-8 instead: one a DOS tool cannot
/// read is better than a line that lost its words.
fn description_bytes(
    old: Option<&[u8]>,
    text: &str,
    rules: &fcmd_core::descriptions::Rules,
) -> Vec<u8> {
    use fcmd_view::decode::Codepage;
    if rules.utf8 {
        return text.as_bytes().to_vec();
    }
    let Some(old) = old.filter(|bytes| !bytes.is_empty()) else {
        return text.as_bytes().to_vec();
    };
    let page = fcmd_view::decode::detect(old);
    if matches!(page, Codepage::Utf8 | Codepage::Utf16Le | Codepage::Utf16Be) {
        return text.as_bytes().to_vec();
    }
    match page.can_encode(text) {
        true => page.encode(text),
        false => text.as_bytes().to_vec(),
    }
}

/// Carry descriptions after a job: the lines follow what they describe, or go when it goes.
///
/// Read, change, write — both ends, and only what changed. A directory whose descriptions did
/// not change is not written at all, so a copy into a folder nobody has described leaves no
/// file behind.
///
/// The sources are grouped by the directory they were in, because a list gathered from a
/// search holds files from several at once and each of those has a side file of its own.
fn carry_descriptions(carry: &app::Carry, sides: &Sides) {
    let mut moved: Vec<(String, String)> = Vec::new();
    for group in &carry.from {
        let mut source = read_descriptions(&group.dir, sides, &carry.rules);
        let mut changed = false;
        for (there, here) in &group.names {
            if let Some(text) = source.get(there) {
                moved.push((here.clone(), text.to_string()));
            }
            if !carry.keep {
                changed |= source.remove(there).is_some();
            }
        }
        if changed {
            let _ = write_descriptions(&group.dir, &source, sides, &carry.rules);
        }
    }
    if moved.is_empty() {
        return; // nothing described, nothing to carry
    }
    if let Some(to) = &carry.to {
        let mut target = read_descriptions(to, sides, &carry.rules);
        for (name, text) in &moved {
            target.set(name, text);
        }
        let _ = write_descriptions(to, &target, sides, &carry.rules);
    }
}

fn run_decompiler(rule: &fcmd_config::DecompileRule, path: &VPath) -> Result<String, String> {
    const MAX_OUTPUT: usize = 32 * 1024 * 1024;

    let mut parts = rule.cmd.split_whitespace();
    let program = parts.next().ok_or_else(|| "empty command".to_string())?;
    let args: Vec<String> = parts
        .map(|arg| {
            if arg.contains("{file}") {
                arg.replace("{file}", &path.to_string())
            } else {
                arg.to_string()
            }
        })
        .collect();
    // A rule that forgot `{file}` still gets the path, at the end.
    let args = if rule.cmd.contains("{file}") {
        args
    } else {
        let mut args = args;
        args.push(path.to_string());
        args
    };

    let output = std::process::Command::new(program)
        .args(&args)
        .output()
        .map_err(|e| format!("{program}: {e}"))?;

    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    if !output.status.success() && text.trim().is_empty() {
        return Err(format!("exited with {}", output.status));
    }
    if text.len() > MAX_OUTPUT {
        text.truncate(MAX_OUTPUT);
        text.push_str("\n… output truncated\n");
    }
    Ok(text)
}

/// Recursive size of a directory. Depth-limited, cancellable, and it never follows symlinks.
fn dir_size(
    fs: &dyn fcmd_jobs::JobFs,
    path: &VPath,
    cancel: &CancelToken,
    depth: u32,
) -> (u64, u64) {
    if depth > 64 || cancel.is_cancelled() {
        return (0, 0);
    }
    let Ok(entries) = fs.read_dir(path) else {
        return (0, 0);
    };
    let mut bytes = 0u64;
    let mut files = 0u64;
    for (name, facts) in entries {
        match facts.kind {
            fcmd_core::EntryKind::Parent | fcmd_core::EntryKind::Symlink => {}
            fcmd_core::EntryKind::Dir => {
                let (b, f) = dir_size(fs, &path.join(&name), cancel, depth + 1);
                bytes += b;
                files += f;
            }
            _ => {
                bytes += facts.size;
                files += 1;
            }
        }
    }
    (bytes, files)
}

/// Bring a remote file down into a private temporary directory, for the viewer.
///
/// The directory is created under the system temp directory with the session's process id, so
/// two running copies never collide, and it is removed when the process exits.
async fn fetch_remote(pool: &Pool, path: &VPath) -> Result<(PathBuf, RemoteStamp), String> {
    let remote = pool.require(path).map_err(|e| e.to_string())?;
    let dir = app::App::fetch_dir();
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    let local = remote
        .fetch_to(path, &dir, &CancelToken::new())
        .await
        .map_err(|e| e.to_string())?;
    let stamp = stamp_of(&remote, path).await.unwrap_or(RemoteStamp {
        size: 0,
        mtime: None,
    });
    Ok((local, stamp))
}

/// The tunnels that are open, held where the runtime can reach them.
type Tunnels = std::sync::Arc<std::sync::Mutex<Vec<fcmd_net::Tunnel>>>;

/// Open one over the connection `authority` names, and keep it.
async fn open_tunnel(
    pool: &std::sync::Arc<Pool>,
    tunnels: &Tunnels,
    id: u64,
    authority: &str,
    spec: &str,
) -> Result<(), String> {
    let spec = fcmd_net::TunnelSpec::parse(spec).map_err(|e| e.to_string())?;
    let path = VPath::remote(fcmd_core::Scheme::Sftp, authority, "/");
    let remote = pool.require(&path).map_err(|e| e.to_string())?;
    let tunnel = fcmd_net::tunnel::open(remote, id, spec)
        .await
        .map_err(|e| e.to_string())?;
    tunnels.lock().expect("the tunnels").push(tunnel);
    Ok(())
}

/// What the panels are told about them.
fn tunnel_infos(tunnels: &Tunnels) -> Vec<app::TunnelInfo> {
    tunnels
        .lock()
        .expect("the tunnels")
        .iter()
        .map(|t| {
            let (served, live, failed) = t.counts();
            app::TunnelInfo {
                id: t.id,
                description: t.describe(),
                served,
                live,
                failed,
            }
        })
        .collect()
}

/// Read both sides of every pair and report which differ.
///
/// On a blocking thread, because the reading is blocking and a comparison of a thousand files
/// must not sit in front of the panels. Progress goes out every few pairs rather than every
/// pair: a thousand messages a second is a slower program than no messages at all.
/// Walk both trees and decide about every pair (SPEC 03 §10).
///
/// Nothing is written here. What comes back is a table, and the jobs that follow are started
/// by the user pressing `F2` over it.
fn run_sync_plan(
    id: u64,
    left: &VPath,
    right: &VPath,
    options: &fcmd_jobs::sync::Options,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
) {
    let fs = fcmd_jobs::LocalFs;
    let mut last = std::time::Instant::now();
    let result = fcmd_jobs::sync::plan(&fs, left, right, options, &cancel, &mut |at, pairs| {
        if last.elapsed() < std::time::Duration::from_millis(100) {
            return;
        }
        last = std::time::Instant::now();
        let _ = tx.send(Event::SyncProgress {
            id,
            at: at.to_string(),
            pairs,
        });
    });
    let event = match result {
        Ok(pairs) => Event::SyncPlanned {
            id,
            pairs,
            cancelled: cancel.is_cancelled(),
            problem: None,
        },
        Err(e) => Event::SyncPlanned {
            id,
            pairs: Vec::new(),
            cancelled: false,
            problem: Some(e.to_string()),
        },
    };
    let _ = tx.send(event);
}

/// Hash what was chosen and write the list (SPEC 03 §9).
///
/// `target` is the one file they all go into; `None` writes a list beside each file, named
/// after it. Writing is `std::fs` rather than the job filesystem because a checksum is a
/// local operation today: the reading half (`JobFs::read_chunks`) is implemented by the local
/// filesystem alone, and the app refuses anything else before the dialog opens.
fn run_checksums(
    id: u64,
    paths: &[VPath],
    algorithm: fcmd_jobs::Algorithm,
    target: Option<VPath>,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
) {
    let fs = fcmd_jobs::LocalFs;
    let total = paths.len() as u64;
    let mut last = std::time::Instant::now();
    let (lines, outcome) = fcmd_jobs::sum::create(
        &fs,
        paths,
        algorithm,
        256 * 1024,
        &cancel,
        &mut |path, done| {
            if last.elapsed() < std::time::Duration::from_millis(100) {
                return;
            }
            last = std::time::Instant::now();
            let _ = tx.send(Event::SumProgress {
                id,
                name: path.display_name(),
                done,
                total,
            });
        },
    );

    let mut written = Vec::new();
    let mut errors = outcome.errors.clone();
    if !outcome.cancelled {
        match &target {
            Some(file) => {
                let body = fcmd_jobs::sum::write_list(&lines, algorithm);
                match std::fs::write(file.as_path(), body) {
                    Ok(()) => written.push(file.clone()),
                    Err(e) => errors.push((file.clone(), e.to_string())),
                }
            }
            None => {
                // One list beside each file, named `file.sha256` — which is what a release
                // page offers next to each download.
                for (line, path) in lines.iter().zip(paths.iter()) {
                    let beside = VPath::local(format!(
                        "{}.{}",
                        path.as_path().display(),
                        algorithm.extension()
                    ));
                    let body = fcmd_jobs::sum::write_list(std::slice::from_ref(line), algorithm);
                    match std::fs::write(beside.as_path(), body) {
                        Ok(()) => written.push(beside),
                        Err(e) => errors.push((beside, e.to_string())),
                    }
                }
            }
        }
    }
    let _ = tx.send(Event::SumsDone {
        id,
        written,
        errors,
        cancelled: outcome.cancelled,
    });
}

/// Check the files beside a list of digests against it.
fn run_verify(id: u64, list: &VPath, cancel: CancelToken, tx: mpsc::Sender<Event>) {
    let fs = fcmd_jobs::LocalFs;
    let text = match std::fs::read_to_string(list.os()) {
        Ok(text) => text,
        Err(e) => {
            let _ = tx.send(Event::VerifyDone {
                id,
                verdicts: Vec::new(),
                cancelled: false,
                problem: Some(format!("{}: {e}", list.display_name())),
            });
            return;
        }
    };
    let (lines, algorithm) = fcmd_jobs::sum::parse_list(&text);
    let Some(algorithm) = algorithm.filter(|_| !lines.is_empty()) else {
        let _ = tx.send(Event::VerifyDone {
            id,
            verdicts: Vec::new(),
            cancelled: false,
            // Said as the file's own problem rather than as a failure of the check: a file
            // that is not a list of digests was never going to be checked.
            problem: Some(format!(
                "{}: not a list of digests this can check",
                list.display_name()
            )),
        });
        return;
    };
    let dir = list
        .parent()
        .map(|p| p.normalised())
        .unwrap_or_else(|| list.clone());
    let total = lines.len() as u64;
    let mut last = std::time::Instant::now();
    let (verdicts, outcome) = fcmd_jobs::sum::verify(
        &fs,
        &dir,
        &lines,
        algorithm,
        256 * 1024,
        &cancel,
        &mut |name, done| {
            if last.elapsed() < std::time::Duration::from_millis(100) {
                return;
            }
            last = std::time::Instant::now();
            let _ = tx.send(Event::SumProgress {
                id,
                name: name.to_string(),
                done,
                total,
            });
        },
    );
    let _ = tx.send(Event::VerifyDone {
        id,
        verdicts,
        cancelled: outcome.cancelled,
        problem: None,
    });
}

fn run_compare(
    id: u64,
    pairs: Vec<app::ComparePair>,
    work: app::CompareWork,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
) {
    use fcmd_find::FindFs;
    use std::io::Read;
    let fs = fcmd_vfs::LocalVfs;
    let total = pairs.len() as u64;
    let mut differ = Vec::new();
    let mut line_endings = Vec::new();
    let mut unreadable = 0u64;
    let mut stopped = false;

    for (index, pair) in pairs.iter().enumerate() {
        if cancel.is_cancelled() {
            stopped = true;
            break;
        }
        // Two different sizes are two different files, and the listing already said so —
        // unless only a fragment is being read, where the question is whether the two
        // *start* the same way and the rest of the file is beside the point.
        let same = if pair.left_size != pair.right_size && work.fragment == 0 {
            Some(false)
        } else {
            match (fs.open(&pair.left), fs.open(&pair.right)) {
                (Ok(left), Ok(right)) => {
                    // A fragment is a `take` on each side, so the comparison itself does not
                    // have to know there is a limit.
                    let (mut left, mut right): (Box<dyn Read>, Box<dyn Read>) = if work.fragment > 0
                    {
                        (
                            Box::new(left.take(work.fragment)),
                            Box::new(right.take(work.fragment)),
                        )
                    } else {
                        (Box::new(left), Box::new(right))
                    };
                    match fcmd_core::compare::same_contents(
                        &mut left,
                        &mut right,
                        &cancel,
                        &mut |_| true,
                    ) {
                        Ok(same) => Some(same),
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                            stopped = true;
                            None
                        }
                        // A file that cannot be read is not a file that is the same: it is
                        // counted and named as unread, which is a different thing to act on.
                        Err(_) => {
                            unreadable += 1;
                            None
                        }
                    }
                }
                _ => {
                    unreadable += 1;
                    None
                }
            }
        };
        if stopped {
            break;
        }
        if same == Some(false) {
            // Different bytes. Before calling it a difference, ask the other question: two
            // checkouts of one tree on two systems differ in every line and in nothing that
            // means anything, and FAR's compare plugin has a checkbox for that. Asking it
            // here, after the fact, means nobody has to have ticked it in advance.
            let eol = pair.left_size > 0
                && pair.right_size > 0
                && match (fs.open(&pair.left), fs.open(&pair.right)) {
                    (Ok(mut left), Ok(mut right)) => fcmd_core::compare::same_but_for_line_endings(
                        &mut left, &mut right, &cancel,
                    )
                    .unwrap_or(false),
                    _ => false,
                };
            if eol {
                line_endings.push(pair.name.clone());
            } else {
                differ.push(pair.name.clone());
            }
        }
        let done = index as u64 + 1;
        if done.is_multiple_of(8) || done == total {
            let _ = tx.send(Event::CompareProgress { id, done, total });
        }
        // "Stop on difference": the answer to "are these two the same at all" is already in,
        // and the other nine thousand files need not be read for it.
        if work.stop_on_first && (!differ.is_empty() || !line_endings.is_empty()) {
            break;
        }
    }

    let _ = tx.send(Event::CompareDone {
        id,
        differ,
        line_endings,
        unreadable,
        stopped,
    });
}

/// Compare the whole of two trees (SPEC 03 §11b).
///
/// Both sides are walked first — the search engine's own walker, with the depth the dialog
/// asked for — into a map from the path a file has *under* its root to what the listing says
/// about it. Then the two maps are read against each other: a path only one side has is a
/// difference by itself, and a path both have is settled by the size, the time, or the bytes,
/// exactly as the two-directory comparison settles it.
///
/// Reading the bytes is the expensive half, so it is asked for last and only where the
/// cheaper answers did not already settle it.
#[allow(clippy::too_many_arguments)]
fn run_compare_trees(
    id: u64,
    left_root: VPath,
    right_root: VPath,
    work: app::TreeWork,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
) {
    use fcmd_find::FindFs;
    use std::collections::HashMap;
    use std::io::Read;

    let fs = fcmd_vfs::LocalVfs;
    /// One side's files: the path under the root → where it really is, how big, when.
    type Side = HashMap<String, (VPath, u64, Option<std::time::SystemTime>)>;

    let mut seen = 0u64;
    let mut unreadable = 0u64;
    let mut cancelled = false;
    let mut last = std::time::Instant::now();
    let mut walk = |root: &VPath| -> Side {
        let mut out: Side = HashMap::new();
        let root_text = root.to_string();
        let query = fcmd_find::Query {
            roots: vec![root.clone()],
            subdirectories: true,
            depth: work.depth,
            ..fcmd_find::Query::default()
        };
        let stats = fcmd_find::find(&fs, &query, &cancel, &mut |event| match event {
            fcmd_find::Event::Hit(hit) => {
                if hit.entry.kind.is_dirlike() {
                    return;
                }
                let full = hit.path.to_string();
                let under = full
                    .strip_prefix(&root_text)
                    .map(|rest| rest.trim_start_matches(['/', '\\']).to_string())
                    .unwrap_or_else(|| full.clone());
                let key = if work.case {
                    under.clone()
                } else {
                    under.to_lowercase()
                };
                seen += 1;
                out.insert(key, (hit.path, hit.entry.size, hit.entry.mtime));
                if last.elapsed() >= std::time::Duration::from_millis(120) {
                    last = std::time::Instant::now();
                    let _ = tx.send(Event::CompareTreesProgress { id, seen, read: 0 });
                }
            }
            fcmd_find::Event::Problem(_) => unreadable += 1,
            fcmd_find::Event::Scanning(_) => {}
        });
        if stats.map(|s| s.cancelled).unwrap_or(false) {
            cancelled = true;
        }
        out
    };
    let left = walk(&left_root);
    let right = walk(&right_root);

    let tolerance = std::time::Duration::from_secs(work.seconds);
    let apart = |a: std::time::SystemTime, b: std::time::SystemTime| -> std::time::Duration {
        a.duration_since(b).unwrap_or_else(|e| e.duration())
    };
    let mut rows: Vec<(String, VPath, u64, app::TreeDiff)> = Vec::new();
    let mut pairs = 0u64;
    let mut same = 0u64;
    let mut read = 0u64;

    for (key, (path, size, when)) in &left {
        if cancel.is_cancelled() {
            cancelled = true;
            break;
        }
        let shown = path
            .to_string()
            .strip_prefix(&left_root.to_string())
            .map(|rest| rest.trim_start_matches(['/', '\\']).to_string())
            .unwrap_or_else(|| path.to_string());
        let Some((other, other_size, other_when)) = right.get(key) else {
            rows.push((shown, path.clone(), *size, app::TreeDiff::OnlyLeft));
            continue;
        };
        pairs += 1;
        let size_differs = work.size && size != other_size;
        let time_differs = work.time
            && match (*when, *other_when) {
                (Some(a), Some(b)) => apart(a, b) > tolerance,
                (None, None) => false,
                _ => true,
            };
        // The bytes, where they were asked for and where the cheap answers leave the
        // question open.
        let mut verdict = None;
        if work.contents && !(work.when_time_differs && !time_differs && !size_differs) {
            let settled = *size != *other_size && work.fragment == 0;
            if settled {
                verdict = Some(app::TreeDiff::Differs);
            } else {
                match (fs.open(path), fs.open(other)) {
                    (Ok(a), Ok(b)) => {
                        let (mut a, mut b): (Box<dyn Read>, Box<dyn Read>) = if work.fragment > 0 {
                            (
                                Box::new(a.take(work.fragment)),
                                Box::new(b.take(work.fragment)),
                            )
                        } else {
                            (Box::new(a), Box::new(b))
                        };
                        match fcmd_core::compare::same_contents(&mut a, &mut b, &cancel, &mut |n| {
                            read += n;
                            true
                        }) {
                            Ok(true) => {}
                            Ok(false) => {
                                // The same question the two-directory comparison asks after
                                // the fact: is the only difference the line endings?
                                let eol = match (fs.open(path), fs.open(other)) {
                                    (Ok(mut a), Ok(mut b)) => {
                                        fcmd_core::compare::same_but_for_line_endings(
                                            &mut a, &mut b, &cancel,
                                        )
                                        .unwrap_or(false)
                                    }
                                    _ => false,
                                };
                                verdict = Some(if eol {
                                    app::TreeDiff::LineEndings
                                } else {
                                    app::TreeDiff::Differs
                                });
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {
                                cancelled = true;
                            }
                            Err(_) => unreadable += 1,
                        }
                    }
                    _ => unreadable += 1,
                }
            }
        } else if size_differs || time_differs {
            verdict = Some(app::TreeDiff::Differs);
        }
        match verdict {
            Some(what) => rows.push((shown, path.clone(), *size, what)),
            None => same += 1,
        }
        if last.elapsed() >= std::time::Duration::from_millis(120) {
            last = std::time::Instant::now();
            let _ = tx.send(Event::CompareTreesProgress { id, seen, read });
        }
        if work.stop_on_first && !rows.is_empty() {
            break;
        }
    }
    // And what only the other side has.
    if !(work.stop_on_first && !rows.is_empty()) {
        for (key, (path, size, _)) in &right {
            if left.contains_key(key) {
                continue;
            }
            let shown = path
                .to_string()
                .strip_prefix(&right_root.to_string())
                .map(|rest| rest.trim_start_matches(['/', '\\']).to_string())
                .unwrap_or_else(|| path.to_string());
            rows.push((shown, path.clone(), *size, app::TreeDiff::OnlyRight));
        }
    }
    // In a settled order: the mark groups them, the path sorts inside a group.
    rows.sort_by(|a, b| (a.3.mark(), &a.0).cmp(&(b.3.mark(), &b.0)));

    let _ = tx.send(Event::CompareTreesDone {
        id,
        rows,
        pairs,
        same,
        unreadable,
        cancelled,
    });
}

/// Find the files under a directory that are copies of each other (SPEC 03 §12).
///
/// Three passes, each cheaper than the one that would do the job on its own:
///
/// 1. **walk**, collecting every file's path and size — the search engine's own walker, so a
///    link that points back up the tree ends its branch here as it does there;
/// 2. **group by size**, and throw away every group of one: two files of different lengths
///    cannot be copies, and this settles almost everything without opening a file;
/// 3. **hash** what is left, group by digest, and keep the groups of two or more.
///
/// A hash and not a side-by-side read, because this is the one case where hashing wins: each
/// file is compared against every other file of its size, and reading each one once beats
/// reading it once per pair. The digest is SHA-256, which is not a hash anybody has shown two
/// files for.
///
/// Empty files are left out. They are all copies of each other, there are hundreds of them on
/// any machine, and a list that begins with four hundred empty files is a list nobody reads.
fn run_duplicates(id: u64, root: VPath, cancel: CancelToken, tx: mpsc::Sender<Event>) {
    use std::collections::HashMap;

    let find_fs = fcmd_vfs::LocalVfs;
    let job_fs = fcmd_jobs::LocalFs;
    let query = fcmd_find::Query::under(root);
    let mut by_size: HashMap<u64, Vec<VPath>> = HashMap::new();
    let mut seen = 0u64;
    let mut unreadable = 0u64;
    let mut last = std::time::Instant::now();

    // The walk. Every file, by size; the engine reports what it could not read itself.
    let stats = fcmd_find::find(&find_fs, &query, &cancel, &mut |event| match event {
        fcmd_find::Event::Hit(hit) => {
            if hit.entry.kind.is_dirlike() || hit.entry.size == 0 {
                return;
            }
            seen += 1;
            by_size.entry(hit.entry.size).or_default().push(hit.path);
            if last.elapsed() >= std::time::Duration::from_millis(120) {
                last = std::time::Instant::now();
                let _ = tx.send(Event::DuplicatesProgress {
                    id,
                    seen,
                    hashed: 0,
                });
            }
        }
        fcmd_find::Event::Problem(_) => unreadable += 1,
        fcmd_find::Event::Scanning(_) => {}
    });
    let cancelled = match stats {
        Ok(stats) => stats.cancelled,
        // The only error is a content condition that will not compile, and there is none.
        Err(_) => false,
    };

    // The hashing, for the sizes that more than one file has.
    let mut by_digest: HashMap<(u64, String), Vec<VPath>> = HashMap::new();
    let mut hashed = 0u64;
    let mut stopped = cancelled;
    'sizes: for (size, paths) in by_size {
        if paths.len() < 2 {
            continue;
        }
        for path in paths {
            if cancel.is_cancelled() {
                stopped = true;
                break 'sizes;
            }
            let mut tick = |n: u64| {
                hashed += n;
                !cancel.is_cancelled()
            };
            match fcmd_jobs::sum::digest(
                &job_fs,
                &path,
                fcmd_jobs::Algorithm::Sha256,
                256 * 1024,
                &mut tick,
            ) {
                Ok(digest) => by_digest.entry((size, digest)).or_default().push(path),
                Err(_) => unreadable += 1,
            }
            if last.elapsed() >= std::time::Duration::from_millis(120) {
                last = std::time::Instant::now();
                let _ = tx.send(Event::DuplicatesProgress { id, seen, hashed });
            }
        }
    }

    // The groups, largest waste first: the whole point of the list is what to delete.
    let mut groups: Vec<Vec<(VPath, u64)>> = by_digest
        .into_iter()
        .filter(|(_, paths)| paths.len() > 1)
        .map(|((size, _), mut paths)| {
            paths.sort_by_key(|p| p.to_string());
            paths.into_iter().map(|p| (p, size)).collect()
        })
        .collect();
    groups.sort_by_key(|group| {
        let size = group.first().map(|(_, size)| *size).unwrap_or(0);
        std::cmp::Reverse(size * (group.len() as u64 - 1))
    });

    let _ = tx.send(Event::DuplicatesDone {
        id,
        groups,
        seen,
        unreadable,
        cancelled: stopped,
    });
}

/// Bring an object down to a temporary file, which is what `F3` and `F11` then read.
///
/// The same shape as [`fetch_remote`], and for the same reason: the viewer seeks, the
/// disassembler reads an executable's headers from four places at once, and neither is worth
/// doing with a range request each over a network that charges by the request. The copy is
/// written under the process's own directory in the temporary folder, as a remote file's is,
/// and the panel says that a copy is what is being looked at.
async fn fetch_object(
    objects: &fcmd_s3::vfs::S3Vfs,
    path: &VPath,
) -> Result<(PathBuf, RemoteStamp), String> {
    let cancel = CancelToken::new();
    let entry = Vfs::stat(objects, path).await.map_err(|e| e.to_string())?;
    let bytes = objects
        .read_object(path, None, &cancel)
        .await
        .map_err(|e| e.to_string())?;
    let dir = std::env::temp_dir().join(format!("fcmd-{}", std::process::id()));
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("{}: {e}", dir.display()))?;
    // The key's last segment, and nothing of the rest of it: a key holds slashes, and a file
    // name must not. An empty name cannot happen — a path with no key is a prefix, which has
    // no bytes to fetch — but a name is given anyway rather than assumed.
    let name = path.display_name();
    let name = if name.is_empty() { "object" } else { &name };
    let local = dir.join(name);
    tokio::fs::write(&local, &bytes)
        .await
        .map_err(|e| format!("{}: {e}", local.display()))?;
    Ok((
        local,
        RemoteStamp {
            size: entry.size,
            // The stamp is in whole seconds since the epoch, which is what the wire carries
            // for an SFTP file and what the one comparison it is for needs.
            mtime: entry
                .mtime
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| u32::try_from(d.as_secs()).ok()),
        },
    ))
}

/// Build an archive, reporting through the job channel so that the progress box, the counters
/// and the status line are the ones every other job uses.
///
/// Not through the job engine: a zip's central directory and a compressed tar's single stream
/// are not a sequence of independent file copies, so there is no plan to make. What the user
/// sees is the same; what runs underneath is one pass through `fcmd-arc`.
#[allow(clippy::too_many_arguments)]
fn run_pack(
    id: u64,
    archive: VPath,
    format: fcmd_arc::Format,
    level: fcmd_arc::Level,
    solid: bool,
    sources: Vec<VPath>,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<fcmd_arc::Cache>,
) {
    use fcmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    // A local pack only: writing an archive straight onto a server means uploading it as it
    // grows, and SPEC 04 §4's rebuild is where that belongs.
    let not_local = std::iter::once(&archive)
        .chain(sources.iter())
        .find(|p| !p.is_local() || p.in_container());
    if let Some(path) = not_local {
        let _ = tx.send(Event::JobDone {
            id,
            kind: fcmd_jobs::JobKind::Pack,
            outcome: Outcome {
                errors: vec![(
                    path.clone(),
                    "an archive can only be built from local files, onto a local disk".to_string(),
                )],
                ..Outcome::default()
            },
        });
        return;
    }

    let paths: Vec<std::path::PathBuf> =
        sources.iter().map(|p| p.as_path().to_path_buf()).collect();
    let mut last = std::time::Instant::now();
    let mut progress = |current: &std::path::Path, bytes: u64| {
        // Every file would be a redraw per file: a tree of ten thousand small files would
        // spend its time painting. A tenth of a second is still a live bar.
        if last.elapsed() < std::time::Duration::from_millis(100) {
            return;
        }
        last = std::time::Instant::now();
        let _ = tx.send(Event::JobProgress {
            id,
            progress: Progress {
                phase: Phase::Running,
                bytes_done: bytes,
                current: Some(VPath::local(current)),
                elapsed: started.elapsed(),
                ..Progress::default()
            },
        });
    };
    let result = fcmd_arc::create(
        archive.as_path(),
        format,
        &paths,
        level,
        solid,
        &cancel,
        &mut progress,
    );
    // The archive on disk is new, so any index of a file by that name is not it any more.
    archives.forget(&archive.os());

    let outcome = match result {
        Ok(written) => Outcome {
            files: written.files,
            dirs: written.dirs,
            bytes: written.bytes,
            ..Outcome::default()
        },
        Err(fcmd_arc::ArcError::Cancelled) => Outcome {
            cancelled: true,
            ..Outcome::default()
        },
        Err(e) => Outcome {
            errors: vec![(archive.clone(), e.to_string())],
            ..Outcome::default()
        },
    };
    let _ = tx.send(Event::JobDone {
        id,
        kind: fcmd_jobs::JobKind::Pack,
        outcome,
    });
}

/// Read every entry of an archive and report what does not come out whole.
///
/// It reports as a job: the same progress box, the same counters, the same Esc to stop. What
/// it finds goes into the outcome's errors, one line per entry, which is where the job's own
/// summary already knows how to show them.
fn run_test(
    id: u64,
    archive: VPath,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<fcmd_arc::Cache>,
) {
    use fcmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    // Through `open_host`, so testing an archive that is itself inside an archive tests that
    // one rather than the container it came out of.
    let outcome = match archives.open_host(&archive, &cancel) {
        Ok(handle) => {
            let total = handle
                .entries()
                .iter()
                .filter(|e| e.kind != fcmd_core::EntryKind::Dir)
                .count() as u64;
            let mut done = 0u64;
            let mut last = std::time::Instant::now();
            let report = handle.verify(&cancel, &mut |name, bytes| {
                done += 1;
                if last.elapsed() < std::time::Duration::from_millis(100) {
                    return;
                }
                last = std::time::Instant::now();
                let _ = tx.send(Event::JobProgress {
                    id,
                    progress: Progress {
                        phase: Phase::Running,
                        files_done: done,
                        files_total: total,
                        bytes_done: bytes,
                        current: Some(archive.enter(name)),
                        elapsed: started.elapsed(),
                        ..Progress::default()
                    },
                });
            });
            match report {
                Ok(report) => Outcome {
                    files: report.members,
                    bytes: report.bytes,
                    errors: report
                        .bad
                        .into_iter()
                        .map(|(name, why)| (archive.enter(name), why))
                        .collect(),
                    ..Outcome::default()
                },
                Err(fcmd_arc::ArcError::Cancelled) => Outcome {
                    cancelled: true,
                    ..Outcome::default()
                },
                Err(e) => Outcome {
                    errors: vec![(archive.clone(), e.to_string())],
                    ..Outcome::default()
                },
            }
        }
        Err(e) => Outcome {
            errors: vec![(archive.clone(), e.to_string())],
            ..Outcome::default()
        },
    };
    let _ = tx.send(Event::JobDone {
        id,
        kind: fcmd_jobs::JobKind::Test,
        outcome,
    });
}

/// Write a new archive out of an old one with the changes applied (SPEC 04 §4).
///
/// The cache is told to forget the archive first: its index describes the file as it was, and
/// after this the file is a different one. Forgetting it before rather than after means a
/// rebuild that fails half way cannot leave a stale index behind either.
fn run_rebuild(
    id: u64,
    archive: VPath,
    changes: Vec<fcmd_arc::Change>,
    level: fcmd_arc::Level,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<fcmd_arc::Cache>,
) {
    use fcmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    let total = changes.len() as u64;
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    // A container inside another container is rebuilt too, by rebuilding the copy it is read
    // from and putting that copy back into the archive it came out of, as far out as the file
    // on this disk (SPEC 04 §4). One on a server is still read-only, and says so.
    let result = fcmd_arc::rebuild_in(
        archives,
        &archive,
        &changes,
        level,
        &cancel,
        &mut |name, bytes| {
            done += 1;
            if last.elapsed() < std::time::Duration::from_millis(100) {
                return;
            }
            last = std::time::Instant::now();
            let _ = tx.send(Event::JobProgress {
                id,
                progress: Progress {
                    phase: Phase::Running,
                    files_done: done,
                    files_total: total.max(done),
                    bytes_done: bytes,
                    current: Some(archive.enter(name)),
                    elapsed: started.elapsed(),
                    ..Progress::default()
                },
            });
        },
    );
    let outcome = match result {
        Ok(written) => Outcome {
            files: written.files,
            dirs: written.dirs,
            bytes: written.bytes,
            ..Outcome::default()
        },
        Err(fcmd_arc::ArcError::Cancelled) => Outcome {
            cancelled: true,
            ..Outcome::default()
        },
        Err(e) => Outcome {
            errors: vec![(archive.clone(), e.to_string())],
            ..Outcome::default()
        },
    };
    let _ = tx.send(Event::JobDone {
        id,
        kind: fcmd_jobs::JobKind::Rebuild,
        outcome,
    });
}

/// Copy one entry out of an archive into the temporary directory, for the viewer and the
/// editor — which both read a real file (SPEC 04 §5, the same shape as a remote fetch).
fn fetch_entry(
    archives: &std::sync::Arc<fcmd_arc::Cache>,
    path: &VPath,
) -> Result<(PathBuf, RemoteStamp), String> {
    if path.leave().is_none() {
        return Err(format!("{path}: not a path inside an archive"));
    }
    let (archive, inner) = archives
        .open_in(path, &CancelToken::new())
        .map_err(|e| e.to_string())?;
    let entry = archive
        .entry(&inner)
        .ok_or_else(|| format!("{path}: no such entry"))?;
    let dir = std::env::temp_dir().join(format!("fcmd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let local = dir.join(path.display_name());
    archive
        .extract(&inner, &local, &CancelToken::new(), &mut |_| {})
        .map_err(|e| e.to_string())?;
    Ok((
        local,
        RemoteStamp {
            size: entry.size,
            // The stamp exists to tell whether a file changed under the editor; an archive
            // entry's own time is in seconds since the epoch, trimmed like the wire's.
            mtime: entry
                .mtime
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as u32),
        },
    ))
}

/// How a container that is not on this disk gets here (SPEC 04 §4).
///
/// The archive crate reads files and seeks in them, so an archive on a server or in a bucket
/// has to be a file here before its index can be read. This is the one thing that knows how to
/// make it one: it is handed to the container cache at startup, and from there an archive on a
/// server opens with `Enter` like any other.
struct BringHere {
    pool: std::sync::Arc<Pool>,
    objects: std::sync::Arc<fcmd_s3::vfs::S3Vfs>,
    handle: tokio::runtime::Handle,
}

impl BringHere {
    fn refused(path: &VPath, reason: String) -> fcmd_arc::ArcError {
        fcmd_arc::ArcError::Refused {
            path: path.to_string(),
            reason,
        }
    }
}

impl fcmd_arc::Fetch for BringHere {
    fn fetch(
        &self,
        path: &VPath,
        to: &std::path::Path,
        cancel: &CancelToken,
    ) -> fcmd_arc::Result<()> {
        if path.scheme() == fcmd_core::Scheme::S3 {
            let bytes = self
                .handle
                .block_on(self.objects.read_object(path, None, cancel))
                .map_err(|e| BringHere::refused(path, e.to_string()))?;
            return std::fs::write(to, &bytes).map_err(|e| fcmd_arc::ArcError::Io {
                path: to.display().to_string(),
                source: e,
            });
        }
        let remote = self
            .pool
            .require(path)
            .map_err(|e| BringHere::refused(path, e.to_string()))?;
        let mut on_bytes = |_: u64| !cancel.is_cancelled();
        self.handle
            .block_on(remote.download(path, to, &mut on_bytes))
            .map(|_| ())
            .map_err(|e| BringHere::refused(path, e.to_string()))
    }

    fn size(&self, path: &VPath) -> Option<u64> {
        if path.scheme() == fcmd_core::Scheme::S3 {
            return self
                .handle
                .block_on(Vfs::stat(&*self.objects, path))
                .ok()
                .map(|entry| entry.size);
        }
        let remote = self.pool.require(path).ok()?;
        self.handle
            .block_on(stamp_of(&remote, path))
            .map(|s| s.size)
    }

    fn stamp(&self, path: &VPath) -> Option<String> {
        let entry = if path.scheme() == fcmd_core::Scheme::S3 {
            self.handle.block_on(Vfs::stat(&*self.objects, path)).ok()?
        } else {
            let remote = self.pool.require(path).ok()?;
            let stamp = self.handle.block_on(stamp_of(&remote, path))?;
            return Some(format!("{}:{:?}", stamp.size, stamp.mtime));
        };
        Some(format!("{}:{:?}", entry.size, entry.mtime))
    }
}

/// Walk the directories under `root`, breadth first, up to `limit` of them (SPEC 02 §7).
///
/// Breadth first on purpose: a walk that is stopped by its limit should have read the
/// directories near the top, which are the ones a person is looking for, rather than one deep
/// branch of a build tree. Files are never looked at — a tree holds directories — and a
/// directory that cannot be read is skipped rather than reported: this fills a cache.
///
/// The second value says whether the whole tree was read: a list that is part of one says so
/// rather than letting the reader think a folder is not there.
fn scan_tree(root: &VPath, limit: usize, cancel: &CancelToken) -> (Vec<(VPath, Vec<VPath>)>, bool) {
    let mut out: Vec<(VPath, Vec<VPath>)> = Vec::new();
    let mut queue: std::collections::VecDeque<VPath> = std::collections::VecDeque::new();
    queue.push_back(root.clone());
    let mut seen = 0usize;
    while let Some(dir) = queue.pop_front() {
        if cancel.is_cancelled() {
            return (out, false);
        }
        if seen >= limit {
            return (out, false);
        }
        seen += 1;
        let Ok(entries) = fcmd_vfs::local::list_blocking(&dir, cancel) else {
            continue;
        };
        let mut kids: Vec<VPath> = Vec::new();
        for entry in entries {
            if entry.kind == fcmd_core::EntryKind::Parent {
                continue;
            }
            let child = dir.join(&entry.name);
            // A link that points at a directory is one, as it is in the tree panel; the
            // walk remembers where it has been by path, so a loop ends a branch.
            let is_dir = entry.kind == fcmd_core::EntryKind::Dir
                || (entry.kind == fcmd_core::EntryKind::Symlink
                    && std::fs::metadata(child.as_path()).is_ok_and(|m| m.is_dir()));
            if is_dir {
                kids.push(child);
            }
        }
        for kid in &kids {
            queue.push_back(kid.clone());
        }
        out.push((dir, kids));
    }
    (out, true)
}

/// What the file looks like on the server right now.
async fn stamp_of(remote: &fcmd_net::Remote, path: &VPath) -> Option<RemoteStamp> {
    let wire = remote.wire(path).ok()?;
    let entry = remote.sftp().stat(&wire, true).await.ok()?;
    Some(RemoteStamp {
        size: entry.size,
        mtime: entry.mtime,
    })
}

/// Write the editor's local copy back to the server (SPEC 04 §5).
///
/// Unless `force` is set, the file is compared with what it looked like when it was fetched:
/// someone else's work is never overwritten without the user saying so.
async fn write_back(
    pool: &Pool,
    local: &std::path::Path,
    remote_path: &VPath,
    expect: Option<RemoteStamp>,
    force: bool,
) -> Result<u64, UploadError> {
    let remote = pool
        .require(remote_path)
        .map_err(|e| UploadError::Failed(e.to_string()))?;
    if !force {
        if let (Some(expected), Some(now)) = (expect, stamp_of(&remote, remote_path).await) {
            // A file that is not there any more is not someone else's work: write it.
            let changed = now.size != expected.size
                || (now.mtime.is_some() && expected.mtime.is_some() && now.mtime != expected.mtime);
            if changed {
                return Err(UploadError::Changed);
            }
        }
    }
    let mut ignore = |_: u64| true;
    remote
        .upload_in_place(local, remote_path, &mut ignore)
        .await
        .map_err(|e| UploadError::Failed(e.to_string()))
}

/// Change what the selected files are, reporting through the same box as a copy.
fn run_attrs(
    id: u64,
    paths: &[VPath],
    change: &fcmd_jobs::Change,
    recursive: bool,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    fs: &dyn fcmd_jobs::JobFs,
) {
    use fcmd_jobs::{Phase, Progress};

    let started = std::time::Instant::now();
    let total = paths.len() as u64;
    let mut last = std::time::Instant::now();
    let outcome =
        fcmd_jobs::set_attributes(fs, paths, change, recursive, &cancel, &mut |path, done| {
            if last.elapsed() < std::time::Duration::from_millis(100) {
                return;
            }
            last = std::time::Instant::now();
            let _ = tx.send(Event::JobProgress {
                id,
                progress: Progress {
                    phase: Phase::Running,
                    files_done: done,
                    files_total: total.max(done),
                    current: Some(path.clone()),
                    elapsed: started.elapsed(),
                    ..Progress::default()
                },
            });
        });
    let _ = tx.send(Event::JobDone {
        id,
        kind: fcmd_jobs::JobKind::Attrs,
        outcome,
    });
}

/// Send the editor's copy back to the object it came from.
///
/// The shape is [`write_back`]'s, with one `PUT` in place of the upload and rename: there is
/// nothing to rename, because an object either exists with these bytes or does not exist at
/// all, and no temporary name is needed to get that.
///
/// The change check is the same question — has somebody else written this since it was
/// fetched? — asked of the object's size and modification time, which is all object storage
/// offers. An `ETag` would be stronger, and is what a later version should compare instead.
async fn put_object(
    objects: &fcmd_s3::vfs::S3Vfs,
    local: &std::path::Path,
    remote_path: &VPath,
    expect: Option<RemoteStamp>,
    force: bool,
) -> Result<u64, UploadError> {
    let cancel = CancelToken::new();
    if !force {
        if let Some(expected) = expect {
            // Not there any more is not someone else's work: write it.
            if let Ok(entry) = objects.stat_object(remote_path, &cancel).await {
                let now = stamp_of_entry(&entry);
                let changed = now.size != expected.size
                    || (now.mtime.is_some()
                        && expected.mtime.is_some()
                        && now.mtime != expected.mtime);
                if changed {
                    return Err(UploadError::Changed);
                }
            }
        }
    }
    objects
        .write_object(remote_path, local, &cancel)
        .await
        .map_err(|e| UploadError::Failed(e.to_string()))
}

/// An entry's size and time, as the stamp the change check compares.
fn stamp_of_entry(entry: &fcmd_core::Entry) -> RemoteStamp {
    RemoteStamp {
        size: entry.size,
        mtime: entry.mtime.and_then(|t| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs() as u32)
        }),
    }
}

/// The object storage provider, built from the `[[s3]]` blocks of the configuration.
///
/// A configuration with none of them gets an empty provider, and an `s3://` path then meets
/// the sentence that says to add one. Credentials are never read from the configuration file:
/// each block says *where* its keys come from, and `fcmd-s3` reads them from the environment
/// or from `~/.aws/credentials` like every other tool on the machine.
fn object_storage(cfg: &fcmd_config::Config) -> fcmd_s3::vfs::S3Vfs {
    use fcmd_s3::creds;
    use fcmd_s3::endpoint::Endpoint;
    use fcmd_s3::vfs::Service;

    let home = dirs_home();
    let services = cfg
        .s3
        .iter()
        .map(|block| {
            let region = if block.region.trim().is_empty() {
                creds::region_from_env().unwrap_or_else(|| "us-east-1".to_string())
            } else {
                block.region.trim().to_string()
            };
            let endpoint = if block.endpoint.trim().is_empty() {
                Endpoint::aws(region)
            } else {
                Endpoint::parse(block.endpoint.trim(), region)
            };
            let endpoint = match block.path_style {
                Some(path_style) => endpoint.with_path_style(path_style),
                None => endpoint,
            };
            let source = creds::Source::parse(&block.credentials);
            Service::new(block.name.trim(), endpoint)
                .with_credentials(creds::resolve(&source, home.as_deref()))
                .with_listing_cap(block.listing_cap)
        })
        .collect();
    fcmd_s3::vfs::S3Vfs::new(services)
}

/// Where `~` is. The same answer the configuration paths use, asked again here because the
/// credentials file is not ours to find any other way.
fn dirs_home() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// A real command, run the way the application runs one: what it prints has to arrive
    /// while it runs, and what comes back at the end has to be the note and not the output
    /// a second time.
    #[test]
    #[cfg(unix)]
    fn a_command_reports_its_lines_as_it_prints_them() {
        use std::sync::Mutex;

        let seen: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let cwd = std::env::current_dir().unwrap();
        let (note, status) =
            run_shell("echo one; echo two 1>&2; echo three", &cwd, None, &|text| {
                seen.lock().unwrap().push(text)
            });
        assert_eq!(status, Some(0));
        // Nothing is left to add: the lines have gone already.
        assert_eq!(note, "");
        let printed: String = seen.lock().unwrap().concat();
        // Both pipes, and every line of both.
        for line in ["one", "two", "three"] {
            assert!(printed.contains(line), "{printed:?}");
        }
        // Whole lines, never half of one.
        assert!(printed.ends_with('\n'), "{printed:?}");
    }

    /// FAR's two settings for the side file: which names it may be under, and whether a
    /// write goes out as UTF-8 (SPEC 02 §9).
    #[test]
    fn the_descriptions_file_is_looked_for_under_the_names_asked_for() {
        use fcmd_core::descriptions::{Descriptions, Rules};

        let dir = tempfile::tempdir().unwrap();
        let here = VPath::local(dir.path());
        let sides = Sides::here(std::sync::Arc::new(
            fcmd_arc::Cache::new(Default::default()),
        ));
        std::fs::write(
            dir.path().join("notes.ion"),
            "a.txt the one in a name of its own
",
        )
        .unwrap();

        // The defaults do not know that name, so there is nothing to find.
        assert!(read_descriptions(&here, &sides, &Rules::default()).is_empty());

        let mine = Rules::new(&["notes.ion".to_string()], true);
        assert_eq!(
            read_descriptions(&here, &sides, &mine).get("a.txt"),
            Some("the one in a name of its own")
        );
        // And a write goes back under the first name, not under `descript.ion`.
        let mut descriptions = Descriptions::default();
        descriptions.set("a.txt", "written back");
        write_descriptions(&here, &descriptions, &sides, &mine).expect("written");
        assert!(!dir.path().join("descript.ion").exists());
        assert_eq!(
            read_descriptions(&here, &sides, &mine).get("a.txt"),
            Some("written back")
        );
    }

    /// With UTF-8 off the file goes back in the page it was read in — and in UTF-8 anyway
    /// when that page cannot hold what was typed, because a description that lost its words
    /// is worse than one a DOS tool cannot read.
    #[test]
    fn a_descriptions_file_keeps_its_code_page_when_asked_to() {
        use fcmd_core::descriptions::Rules;

        let keep = Rules::new(&[], false);
        // A file written in a single-byte page, as a `descript.ion` of that age is. Which
        // page exactly is the detector's business — what matters is that it is not UTF-8 and
        // that the file goes back in the one it came in.
        let old = fcmd_view::decode::Codepage::Cp1252.encode("fiché.txt nota");
        let page = fcmd_view::decode::detect(&old);
        assert_ne!(page, fcmd_view::decode::Codepage::Utf8, "the fixture");

        let bytes = description_bytes(Some(&old), "fiché.txt altra nota", &keep);
        assert_eq!(page.decode(&bytes), "fiché.txt altra nota");
        assert!(
            std::str::from_utf8(&bytes).is_err(),
            "it is not UTF-8 any more"
        );

        // A character that page has no room for keeps the whole file in UTF-8, rather than
        // writing the `?` the encoder would put in its place.
        let bytes = description_bytes(Some(&old), "fiché.txt 日本語", &keep);
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "fiché.txt 日本語");

        // No file yet, and a file that was already UTF-8: UTF-8 either way.
        assert_eq!(description_bytes(None, "a b\n", &keep), b"a b\n".to_vec());
        assert_eq!(
            description_bytes(Some("é x\n".as_bytes()), "é y\n", &keep),
            "é y\n".as_bytes().to_vec()
        );
        // And with the setting on, nothing is converted at all.
        assert_eq!(
            description_bytes(Some(&old), "fiché.txt x", &Rules::default()),
            "fiché.txt x".as_bytes().to_vec()
        );
    }

    /// A `descript.ion` inside an archive is an entry like any other, and the panel shows
    /// what it says.
    #[test]
    fn descriptions_are_read_inside_an_archive_too() {
        use std::io::Write;

        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        let zip = left.join("tree.zip");
        {
            let file = std::fs::File::create(&zip).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            for (name, body) in [
                ("readme.md", &b"# readme\n"[..]),
                (
                    "descript.ion",
                    &b"readme.md what to read first inside the archive\n"[..],
                ),
            ] {
                writer
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                writer.write_all(body).unwrap();
            }
            writer.finish().unwrap();
        }

        let mut app = app_on(&left, &right);
        serve(&mut app);
        // `Enter` on the archive is how a user gets in there, and it is what asks for the
        // listing the descriptions then follow.
        let row = (0..app.panels[0].listing.len())
            .find(|i| {
                app.panels[0]
                    .listing
                    .row(*i)
                    .map(|e| e.name_str() == "tree.zip")
                    .unwrap_or(false)
            })
            .expect("the archive is listed");
        app.panels[0].cursor = row;
        key(&mut app, "Enter");
        serve(&mut app);
        assert!(app.panels[0].cwd.in_container(), "{}", app.panels[0].cwd);
        key(&mut app, "Ctrl+6");
        serve(&mut app);
        assert_eq!(
            app.panels[0].descriptions.get("readme.md"),
            Some("what to read first inside the archive")
        );
    }

    /// A directory's descriptions, end to end: the file on disk is read when a view that
    /// shows it is chosen, `Ctrl+Z` edits one, and the file comes back with it.
    #[test]
    fn descriptions_are_read_from_the_directory_shown_and_written_back_by_ctrl_z() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("readme.md"), b"# readme\n").unwrap();
        std::fs::write(left.join("build.sh"), b"make\n").unwrap();
        std::fs::write(
            left.join("descript.ion"),
            "readme.md what to read first\nbuild.sh runs the build\n",
        )
        .unwrap();

        let mut app = app_on(&left, &right);
        serve(&mut app);
        // Nothing has asked yet: the full view shows no description, and a side file nobody
        // is looking at is a file nobody should open.
        assert!(app.panels[0].descriptions.is_empty());

        // `Ctrl+6` is the view that shows them, and choosing it is what asks.
        key(&mut app, "Ctrl+6");
        serve(&mut app);
        assert_eq!(app.panels[0].view, fcmd_core::ViewMode::Descriptions);
        assert_eq!(
            app.panels[0].descriptions.get("readme.md"),
            Some("what to read first")
        );

        // `Ctrl+Z` opens on what the file says about the row under the cursor.
        let row = (0..app.panels[0].listing.len())
            .find(|i| {
                app.panels[0]
                    .listing
                    .row(*i)
                    .map(|e| e.name_str() == "build.sh")
                    .unwrap_or(false)
            })
            .expect("build.sh is listed");
        app.panels[0].cursor = row;
        key(&mut app, "Ctrl+Z");
        match &app.mode {
            crate::app::Mode::Prompt(p) => assert_eq!(p.text, "runs the build"),
            other => panic!("{other:?}"),
        }
        // Typed over and accepted: the panel says so at once and the file follows.
        for c in ["Space", "t", "w", "i", "c", "e"] {
            key(&mut app, c);
        }
        key(&mut app, "Enter");
        assert_eq!(
            app.panels[0].descriptions.get("build.sh"),
            Some("runs the build twice")
        );
        serve(&mut app);
        let written = std::fs::read_to_string(left.join("descript.ion")).unwrap();
        assert_eq!(
            written,
            "readme.md what to read first\nbuild.sh runs the build twice\n"
        );

        // And an empty answer takes the line away, which is how a description is removed.
        key(&mut app, "Ctrl+Z");
        for _ in 0.."runs the build twice".len() {
            key(&mut app, "Backspace");
        }
        key(&mut app, "Enter");
        serve(&mut app);
        let written = std::fs::read_to_string(left.join("descript.ion")).unwrap();
        assert_eq!(written, "readme.md what to read first\n");
    }

    /// A description follows what it describes: a copy leaves one behind and puts one at the
    /// other end, a move takes it with it, a delete takes it away.
    #[test]
    fn a_description_follows_a_copy_a_move_and_a_delete() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("notes.txt"), b"hello\n").unwrap();
        std::fs::write(left.join("descript.ion"), "notes.txt what it is\n").unwrap();

        let described = |dir: &std::path::Path| -> String {
            std::fs::read_to_string(dir.join("descript.ion")).unwrap_or_default()
        };

        let mut app = app_on(&left, &right);
        serve(&mut app);
        let row = (0..app.panels[0].listing.len())
            .find(|i| {
                app.panels[0]
                    .listing
                    .row(*i)
                    .map(|e| e.name_str() == "notes.txt")
                    .unwrap_or(false)
            })
            .expect("notes.txt is listed");
        app.panels[0].cursor = row;

        // `F5`, and the description is at both ends: the file is in two places now. Twice,
        // because the job's own end is what asks for the descriptions to be carried, and
        // `serve` drains the queue once.
        key(&mut app, "F5");
        key(&mut app, "Enter");
        serve(&mut app);
        serve(&mut app);
        assert!(right.join("notes.txt").is_file(), "the copy was made");
        assert_eq!(described(&right), "notes.txt what it is\n");
        assert_eq!(described(&left), "notes.txt what it is\n");

        // `F6` out of the way and back: the line goes with the file rather than staying.
        std::fs::remove_file(right.join("notes.txt")).unwrap();
        std::fs::remove_file(right.join("descript.ion")).unwrap();
        serve(&mut app);
        key(&mut app, "Ctrl+R");
        serve(&mut app);
        let row = (0..app.panels[0].listing.len())
            .find(|i| {
                app.panels[0]
                    .listing
                    .row(*i)
                    .map(|e| e.name_str() == "notes.txt")
                    .unwrap_or(false)
            })
            .expect("notes.txt is still listed");
        app.panels[0].cursor = row;
        key(&mut app, "F6");
        key(&mut app, "Enter");
        serve(&mut app);
        serve(&mut app);
        assert!(right.join("notes.txt").is_file(), "the move was made");
        assert_eq!(described(&right), "notes.txt what it is\n");
        assert_eq!(
            described(&left),
            "",
            "a moved file leaves no description behind"
        );

        // And a delete takes the line with the file: a description that outlived its file
        // would land on the next file to be given that name.
        app.active = 1;
        key(&mut app, "Ctrl+R");
        serve(&mut app);
        let row = (0..app.panels[1].listing.len())
            .find(|i| {
                app.panels[1]
                    .listing
                    .row(*i)
                    .map(|e| e.name_str() == "notes.txt")
                    .unwrap_or(false)
            })
            .expect("notes.txt arrived");
        app.panels[1].cursor = row;
        key(&mut app, "F8");
        key(&mut app, "Enter");
        serve(&mut app);
        serve(&mut app);
        assert!(!right.join("notes.txt").exists(), "the delete happened");
        assert_eq!(described(&right), "");
    }

    #[test]
    fn a_search_result_carries_a_description_out_of_each_directory() {
        use fcmd_core::{Entry, EntryId, EntryKind};

        let root = tempfile::tempdir().unwrap();
        let one = root.path().join("one");
        let two = root.path().join("two");
        let right = root.path().join("right");
        for dir in [&one, &two, &right] {
            std::fs::create_dir_all(dir).unwrap();
        }
        std::fs::write(one.join("a.txt"), b"a\n").unwrap();
        std::fs::write(one.join("descript.ion"), "a.txt the first one\n").unwrap();
        std::fs::write(two.join("b.txt"), b"b\n").unwrap();
        std::fs::write(two.join("descript.ion"), "b.txt the second one\n").unwrap();

        let mut app = app_on(root.path(), &right);
        serve(&mut app);

        // A list as a search leaves one: two files that were never in the same directory.
        let items: Vec<(VPath, Entry)> = [one.join("a.txt"), two.join("b.txt")]
            .iter()
            .enumerate()
            .map(|(at, path)| {
                let entry = Entry {
                    name: path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .into_owned()
                        .into(),
                    kind: EntryKind::File,
                    size: 2,
                    mtime: None,
                    btime: None,
                    atime: None,
                    flags: fcmd_core::EntryFlags::empty(),
                    id: EntryId::Name(at as u64),
                    link_target: None,
                };
                (VPath::local(path), entry)
            })
            .collect();
        let generation = app.panels[0].generation + 1;
        app.panels[0].generation = generation;
        app.panels[0].set_temp("Find".to_string(), items, generation);
        app.panels[0].cursor = 0;

        // Both rows chosen, and `F5` onto the other panel.
        key(&mut app, "Insert");
        key(&mut app, "Insert");
        key(&mut app, "F5");
        key(&mut app, "Enter");
        serve(&mut app);
        serve(&mut app);

        assert!(right.join("a.txt").is_file() && right.join("b.txt").is_file());
        let landed = std::fs::read_to_string(right.join("descript.ion")).unwrap_or_default();
        assert!(
            landed.contains("a.txt the first one"),
            "the first directory's line came along: {landed}"
        );
        assert!(
            landed.contains("b.txt the second one"),
            "and the second directory's too: {landed}"
        );
        // A copy leaves both where they were.
        assert_eq!(
            std::fs::read_to_string(one.join("descript.ion")).unwrap(),
            "a.txt the first one\n"
        );
    }

    #[test]
    fn descriptions_are_read_and_written_on_a_server() {
        use fcmd_core::descriptions::{Descriptions, Rules};

        let runtime = tokio::runtime::Runtime::new().unwrap();
        let pool = Pool::new();
        let site = fcmd_net::Site::parse_url("sftp://mb@host:22/srv").unwrap();
        let remote = fcmd_net::Remote::new(site, fcmd_net::MemoryFs::new().with_dir("/srv"));
        pool.insert(remote.clone());
        let sides = Sides {
            archives: std::sync::Arc::new(fcmd_arc::Cache::new(Default::default())),
            away: Some(Away {
                pool: pool.clone(),
                objects: std::sync::Arc::new(fcmd_s3::vfs::S3Vfs::empty()),
                handle: runtime.handle().clone(),
            }),
        };
        let dir = remote.site().start();

        // A directory with no side file describes nothing, which is not an error.
        assert!(read_descriptions(&dir, &sides, &Rules::default()).is_empty());

        let mut descriptions = Descriptions::default();
        descriptions.set("notes.txt", "what it is");
        write_descriptions(&dir, &descriptions, &sides, &Rules::default())
            .expect("written to the server");
        assert_eq!(
            read_descriptions(&dir, &sides, &Rules::default()).get("notes.txt"),
            Some("what it is"),
            "and read back off it"
        );

        // Nothing left to say takes the file away rather than leaving an empty one.
        write_descriptions(&dir, &Descriptions::default(), &sides, &Rules::default())
            .expect("removed");
        assert!(read_descriptions(&dir, &sides, &Rules::default()).is_empty());
        assert!(
            !runtime.block_on(remote.exists(&dir.join("descript.ion"))),
            "the file itself is gone"
        );
    }

    /// A plugin on disk, a command it provides, and the whole conversation: the manifest is
    /// loaded the way the binary loads one, the program is a real script, and what it asks for
    /// is carried out. This is the half of the plugin host API that no unit test reaches.
    #[test]
    #[cfg(unix)]
    fn a_plugin_provides_a_command_and_the_application_does_what_it_asks() {
        use std::os::unix::fs::PermissionsExt;
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(left.join("sub")).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("one.txt"), "1").unwrap();
        std::fs::write(left.join("two.txt"), "2").unwrap();

        // The plugin: a manifest and a shell script that answers in the protocol.
        let plugins = root.path().join("plugins");
        let dir = plugins.join("demo");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("plugin.toml"),
            format!(
                r#"
[plugin]
id = "demo"
name = "Demo"
fcmd = ">={}"

[[command]]
id = "user.demo.look"
label = "Demo: say where we are and select everything"
run = "look.sh"
needs = ["panel", "cursor", "selection"]
timeout = 10
"#,
                env!("CARGO_PKG_VERSION")
            ),
        )
        .unwrap();
        let script = dir.join("look.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\n             cat >/dev/null\n             printf '{\"status\":\"here: %s\"}\\n' \"$FCMD_CWD\"\n             echo '{\"select\":[\"one.txt\",\"two.txt\"]}'\n             echo '{\"cursor\":\"two.txt\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Loaded the way the binary loads it.
        let paths = Paths::discover(Some(&left.join("fcmd.toml")), Some(&plugins));
        let mut warnings = Vec::new();
        let (_, _, commands) = load_plugins(&paths, ThemeSet::builtin_only(), &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, "user.demo.look");
        assert_eq!(commands[0].scope, FcmdScope::Panel);
        assert_eq!(commands[0].program, script);

        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths,
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(&left),
            right: VPath::local(&right),
            sites: Vec::new(),
            plugin_commands: commands,
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        serve(&mut app);

        // F11 lists it, with the plugin it came from beside it.
        app.handle_input(Input::Key("F11".parse().unwrap()));
        let crate::app::Mode::Pick(pick) = &app.mode else {
            panic!("{:?}", app.mode)
        };
        assert_eq!(pick.rows.len(), 1);
        assert!(
            pick.rows[0].label.contains("Demo: say where"),
            "{:?}",
            pick.rows[0]
        );
        assert!(pick.rows[0].label.contains("demo"), "{:?}", pick.rows[0]);

        // Enter runs it, through the same request channel everything else uses.
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        serve(&mut app);

        // What it asked for happened: a status line, a selection, a cursor.
        assert!(app.status.contains("here:"), "{}", app.status);
        assert!(
            app.status.contains(&left.display().to_string()),
            "{}",
            app.status
        );
        assert_eq!(app.panels[app.active].selection.len(), 2);
        let under = app.panels[app.active]
            .current()
            .map(|e| e.name_str().into_owned());
        assert_eq!(under.as_deref(), Some("two.txt"));
    }

    /// A manifest the host refuses, and the reasons it gives. A plugin that would do something
    /// surprising must not load at all, rather than surprise somebody at the first press.
    #[test]
    fn a_command_a_plugin_may_not_declare_stops_it_loading() {
        let root = tempfile::tempdir().unwrap();
        let plugins = root.path().join("plugins");
        let write = |body: &str| {
            let dir = plugins.join("bad");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("run.sh"), "#!/bin/sh\n").unwrap();
            std::fs::write(
                dir.join("plugin.toml"),
                format!(
                    "[plugin]\nid = \"bad\"\nname = \"Bad\"\nfcmd = \">={}\"\n{body}",
                    env!("CARGO_PKG_VERSION")
                ),
            )
            .unwrap();
            let paths = Paths::discover(Some(&root.path().join("fcmd.toml")), Some(&plugins));
            let mut warnings = Vec::new();
            let (_, _, commands) = load_plugins(&paths, ThemeSet::builtin_only(), &mut warnings);
            (commands.len(), warnings)
        };

        // A name in the built-in vocabulary is not a plugin's to define.
        let (count, problems) =
            write("[[command]]\nid = \"file.delete\"\nlabel = \"Nope\"\nrun = \"run.sh\"\n");
        assert_eq!(count, 0);
        assert!(
            problems.iter().any(|p| p.contains("file.delete")),
            "{problems:?}"
        );

        // Nor is a program outside the plugin's own directory.
        let (count, problems) =
            write("[[command]]\nid = \"user.x\"\nlabel = \"X\"\nrun = \"../../../bin/sh\"\n");
        assert_eq!(count, 0);
        assert!(!problems.is_empty(), "an escaping path must be refused");

        // Nor a context nobody knows how to give.
        let (count, problems) = write(
            "[[command]]\nid = \"user.x\"\nlabel = \"X\"\nrun = \"run.sh\"\nneeds = [\"ssh-keys\"]\n",
        );
        assert_eq!(count, 0);
        assert!(
            problems.iter().any(|p| p.contains("ssh-keys")),
            "{problems:?}"
        );

        // And one that is sound loads.
        let (count, problems) = write(
            "[[command]]\nid = \"user.x\"\nlabel = \"X\"\nrun = \"run.sh\"\nneeds = [\"panel\"]\n",
        );
        assert_eq!(count, 1, "{problems:?}");
    }

    /// Every example plugin in the repository loads, and each one's program is there and
    /// runnable. An example that does not work is worse than no example — and these are what
    /// somebody copies to start from.
    #[test]
    fn the_example_plugins_load() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("the workspace is two directories up")
            .join("examples/plugins");
        assert!(root.is_dir(), "examples/plugins is missing");
        let paths = Paths::discover(Some(&root.join("fcmd.toml")), Some(&root));
        let mut warnings = Vec::new();
        let (_, _, commands) = load_plugins(&paths, ThemeSet::builtin_only(), &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");

        // Every folder under examples/plugins is a plugin, and every one of them has to have
        // produced at least one command: a folder nobody loads is an example nobody can use.
        // Several provide more than one — three ways to change the case of a name is one
        // program answering to three ids — so this is per folder, not a total.
        let folders: Vec<String> = std::fs::read_dir(&root)
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            folders.len() >= 10,
            "only {} example plugins?",
            folders.len()
        );
        for folder in &folders {
            assert!(
                commands.iter().any(|c| &c.plugin == folder),
                "{folder} loaded no command: {:?}",
                commands.iter().map(|c| &c.id).collect::<Vec<_>>()
            );
        }
        // And no two of them answer to the same id, which the loader warns about rather than
        // refusing — a warning this test has already insisted is absent.
        let mut ids: Vec<&String> = commands.iter().map(|c| &c.id).collect();
        ids.sort();
        let before = ids.len();
        ids.dedup();
        assert_eq!(before, ids.len(), "two commands share an id");

        for command in &commands {
            assert!(
                command.id.starts_with("user."),
                "{} is not in the plugin name space",
                command.id
            );
            assert!(!command.label.is_empty(), "{} has no label", command.id);
            assert!(
                command.program.is_file(),
                "{}: {} is not there",
                command.id,
                command.program.display()
            );
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mode = std::fs::metadata(&command.program)
                    .unwrap()
                    .permissions()
                    .mode();
                assert!(
                    mode & 0o111 != 0,
                    "{}: the host runs it, so it has to be executable",
                    command.id
                );
            }
        }
    }

    /// Every shell script under `examples/plugins` parses. `sh -n` is cheap and catches the
    /// mistake that matters: a plugin whose script does not even start, which in the program
    /// looks like a command that quietly does nothing.
    #[test]
    #[cfg(unix)]
    fn every_example_script_parses() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap()
            .join("examples/plugins");
        let mut checked = 0;
        for folder in std::fs::read_dir(&root).unwrap().flatten() {
            if !folder.path().is_dir() {
                continue;
            }
            for file in std::fs::read_dir(folder.path()).unwrap().flatten() {
                let path = file.path();
                if path.extension().is_some_and(|e| e == "sh") {
                    let out = std::process::Command::new("sh")
                        .arg("-n")
                        .arg(&path)
                        .output()
                        .expect("sh is here");
                    assert!(
                        out.status.success(),
                        "{} does not parse:\n{}",
                        path.display(),
                        String::from_utf8_lossy(&out.stderr)
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked >= 10, "only {checked} scripts checked?");
    }

    /// The whole of `F5`, `F7` and `F8` on a bucket, through the real job engine against the
    /// in-process service: the engine does not know object storage exists, and that is the
    /// point — the copy dialog, the progress and the overwrite question are the ones that
    /// were already there.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_job_engine_copies_to_a_bucket_and_back() {
        use fcmd_s3::fake::FakeS3;
        use fcmd_s3::vfs::{S3Vfs, Service};
        use std::collections::BTreeMap;
        use std::sync::Arc;

        let mut buckets = BTreeMap::new();
        buckets.insert("logs".to_string(), BTreeMap::new());
        let fake = FakeS3::start(buckets).await;
        let objects = Arc::new(S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]));

        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("one.txt"), b"first file").unwrap();
        std::fs::write(dir.path().join("two.txt"), b"second file").unwrap();

        let fs = Arc::new(ObjectFs::new(
            objects.clone(),
            RoutedFs::new(Pool::new(), tokio::runtime::Handle::current()),
            tokio::runtime::Handle::current(),
        ));
        let here = dir.path().to_path_buf();
        let worker = fs.clone();
        tokio::task::spawn_blocking(move || {
            use fcmd_jobs::{JobFs, JobKind, JobSpec};

            struct Quiet;
            impl fcmd_jobs::Reporter for Quiet {
                fn progress(&mut self, _p: &fcmd_jobs::Progress) {}
                fn ask(&mut self, _q: fcmd_jobs::Question) -> fcmd_jobs::Decision {
                    fcmd_jobs::Decision::Overwrite
                }
            }

            /// Plan it and run it, as the worker thread does.
            fn plan_and_run(
                fs: &dyn fcmd_jobs::JobFs,
                spec: &JobSpec,
                reporter: &mut dyn fcmd_jobs::Reporter,
                cancel: &CancelToken,
            ) -> fcmd_jobs::Outcome {
                let plan = fcmd_jobs::plan(fs, spec, cancel).expect("the job can be planned");
                fcmd_jobs::execute(fs, spec, plan, reporter, cancel)
            }

            let bucket = VPath::parse("s3://logs/incoming/");
            let cancel = CancelToken::new();

            // `F7` on the bucket: a prefix that holds nothing yet.
            let outcome = plan_and_run(
                &*worker,
                &JobSpec::new(JobKind::Mkdir, vec![], Some(bucket.clone())),
                &mut Quiet,
                &cancel,
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);

            // `F5`: two local files into the bucket.
            let sources = vec![
                VPath::local(here.join("one.txt")),
                VPath::local(here.join("two.txt")),
            ];
            let outcome = plan_and_run(
                &*worker,
                &JobSpec::new(JobKind::Copy, sources, Some(bucket.clone())),
                &mut Quiet,
                &cancel,
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            assert_eq!(outcome.files, 2);
            let one = VPath::parse("s3://logs/incoming/one.txt");
            assert_eq!(worker.facts(&one).expect("it is up there").size, 10);

            // `F5` the other way: the object comes back down as a file.
            let back = here.join("back");
            std::fs::create_dir(&back).unwrap();
            let outcome = plan_and_run(
                &*worker,
                &JobSpec::new(JobKind::Copy, vec![one.clone()], Some(VPath::local(&back))),
                &mut Quiet,
                &cancel,
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            assert_eq!(std::fs::read(back.join("one.txt")).unwrap(), b"first file");

            // `F6`: a rename inside the bucket is a copy and a delete, and the engine asks
            // for a rename first — which this provider can do.
            let renamed = VPath::parse("s3://logs/incoming/first.txt");
            let outcome = plan_and_run(
                &*worker,
                &JobSpec::new(JobKind::Move, vec![one.clone()], Some(renamed.clone())),
                &mut Quiet,
                &cancel,
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            assert!(worker.exists(&renamed));
            assert!(!worker.exists(&one));

            // `F8`: the objects go.
            let outcome = plan_and_run(
                &*worker,
                &JobSpec::new(JobKind::Delete, vec![renamed.clone()], None),
                &mut Quiet,
                &cancel,
            );
            assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
            assert!(!worker.exists(&renamed));
        })
        .await
        .unwrap();
    }

    /// The editor's save, for an object: one `PUT`, and a file somebody else has written
    /// since it was fetched brings up the question rather than being overwritten.
    #[tokio::test]
    async fn saving_the_editors_copy_writes_the_object() {
        use fcmd_s3::fake::FakeS3;
        use fcmd_s3::vfs::{S3Vfs, Service};
        use std::collections::BTreeMap;

        let mut bucket = BTreeMap::new();
        bucket.insert("notes/readme.txt".to_string(), b"as it was".to_vec());
        let mut buckets = BTreeMap::new();
        buckets.insert("logs".to_string(), bucket);
        let fake = FakeS3::start(buckets).await;
        let objects = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);

        let path = VPath::parse("s3://logs/notes/readme.txt");
        let (local, stamp) = fetch_object(&objects, &path).await.expect("it comes down");
        std::fs::write(&local, b"edited here").unwrap();

        // The stamp matches what was fetched, so the write goes through.
        let written = put_object(&objects, &local, &path, Some(stamp), false)
            .await
            .expect("the save works");
        assert_eq!(written, 11);
        assert_eq!(
            objects
                .read_object(&path, None, &CancelToken::new())
                .await
                .expect("it reads back"),
            b"edited here"
        );

        // Now the object is eleven bytes and the stale stamp says nine: the save asks first.
        let stale = RemoteStamp {
            size: 9,
            mtime: stamp.mtime,
        };
        assert!(matches!(
            put_object(&objects, &local, &path, Some(stale), false).await,
            Err(UploadError::Changed)
        ));
        // And `force` is the answer to that question.
        assert!(put_object(&objects, &local, &path, Some(stale), true)
            .await
            .is_ok());
        let _ = std::fs::remove_file(&local);
    }

    /// `F3` on an object: the bytes come down to a temporary file and the viewer reads that,
    /// because the viewer seeks and a range request per movement over a network that charges
    /// by the request is not how to read a file.
    ///
    /// Against the in-process S3 service, so the whole path is exercised — the signature, the
    /// listing, the `HEAD` for the size and the `GET` for the bytes — without an account
    /// anywhere.
    #[tokio::test]
    async fn an_object_is_fetched_for_the_viewer() {
        use fcmd_s3::fake::FakeS3;
        use fcmd_s3::vfs::{S3Vfs, Service};
        use std::collections::BTreeMap;

        let mut bucket = BTreeMap::new();
        bucket.insert(
            "notes/readme.txt".to_string(),
            b"hello from a bucket".to_vec(),
        );
        let mut buckets = BTreeMap::new();
        buckets.insert("logs".to_string(), bucket);
        let fake = FakeS3::start(buckets).await;
        let objects = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);

        let path = VPath::parse("s3://logs/notes/readme.txt");
        let (local, stamp) = fetch_object(&objects, &path).await.expect("it comes down");
        assert_eq!(
            std::fs::read(&local).unwrap(),
            b"hello from a bucket".to_vec()
        );
        // The copy is named after the key's last segment, not after the whole key.
        assert_eq!(local.file_name().unwrap(), "readme.txt");
        assert_eq!(stamp.size, 19);
        assert!(stamp.mtime.is_some(), "the object has a date");
        let _ = std::fs::remove_file(&local);

        // And a key that is not there is a sentence, not a panic.
        let missing = fetch_object(&objects, &VPath::parse("s3://logs/nope.txt"))
            .await
            .unwrap_err();
        assert!(missing.contains("no such object"), "{missing}");
    }

    /// `sh -n` on this machine is not `sh -n` on every machine, and one difference has bitten
    /// once already: in bash 3.2 — which is the bash macOS ships, and has shipped since 2007
    /// because of a licence change — a `case` written inside a `$( … )` does not parse. The
    /// `)` that closes the first pattern closes the substitution instead, and what follows is
    /// a syntax error at the `;;`, reported against a line that looks perfectly fine.
    ///
    /// A newer shell parses it, so the test above cannot see it and did not: it passed here
    /// and failed on macOS. This one is a reading of the text rather than a parse, and it
    /// holds on every machine. The fix in a script is to move the `case` into a function
    /// defined outside the substitution, or to write the condition as a test.
    #[test]
    fn no_example_script_hides_a_case_inside_a_command_substitution() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap()
            .join("examples/plugins");
        let mut checked = 0;
        for folder in std::fs::read_dir(&root).unwrap().flatten() {
            if !folder.path().is_dir() {
                continue;
            }
            for file in std::fs::read_dir(folder.path()).unwrap().flatten() {
                let path = file.path();
                if !path.extension().is_some_and(|e| e == "sh") {
                    continue;
                }
                let text = std::fs::read_to_string(&path).unwrap();
                for span in substitutions(&text) {
                    assert!(
                        !has_word(&without_comments(span), "case"),
                        "{}: a `case` inside a `$( … )` does not parse in bash 3.2:\n{span}",
                        path.display()
                    );
                }
                checked += 1;
            }
        }
        assert!(checked >= 10, "only {checked} scripts checked?");
    }

    /// Every `$( … )` of a script, with its parentheses balanced so a nested one does not end
    /// the outer span early.
    fn substitutions(text: &str) -> Vec<&str> {
        let bytes = text.as_bytes();
        let mut spans = Vec::new();
        let mut i = 0;
        while i + 1 < bytes.len() {
            if bytes[i] != b'$' || bytes[i + 1] != b'(' {
                i += 1;
                continue;
            }
            let start = i;
            let mut depth = 1;
            let mut j = i + 2;
            while j < bytes.len() && depth > 0 {
                match bytes[j] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                j += 1;
            }
            spans.push(&text[start..j.min(text.len())]);
            i = j;
        }
        spans
    }

    /// The same text without its comments, so that a comment *about* the rule — of which
    /// there are three, in the scripts the rule was written for — does not break it.
    fn without_comments(text: &str) -> String {
        text.lines()
            .map(|line| match line.find('#') {
                // Only a `#` that begins a word is a comment. One inside a word is a shell
                // variable's length or part of a name, and neither starts a comment.
                Some(i) if i == 0 || line[..i].ends_with(char::is_whitespace) => &line[..i],
                _ => line,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Is `word` in `text` as a word, rather than inside a longer one? `lowercase` holds the
    /// word `case` and is not a `case` statement.
    fn has_word(text: &str, word: &str) -> bool {
        text.split(|c: char| !c.is_alphanumeric() && c != '_')
            .any(|piece| piece == word)
    }

    /// The one example that ships a program for each system: on Unix the host picks the shell
    /// script, on Windows the `.cmd`, and both files are in the folder either way.
    #[test]
    fn an_example_with_two_programs_picks_the_one_for_this_system() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .unwrap()
            .join("examples/plugins/git-here");
        let plugin = fcmd_plug::Plugin::load(&root).expect("it loads");
        let (declared, program) = plugin.commands().into_iter().next().expect("one command");
        assert_eq!(declared.run, "status.sh");
        assert_eq!(declared.run_windows.as_deref(), Some("status.cmd"));
        let name = program.file_name().unwrap().to_string_lossy().into_owned();
        assert_eq!(
            name,
            if cfg!(windows) {
                "status.cmd"
            } else {
                "status.sh"
            }
        );
        // The other system's program is in the folder too, so the example is complete on a
        // machine that cannot run it.
        assert!(root.join("status.sh").is_file());
        assert!(root.join("status.cmd").is_file());
    }

    /// An app looking at two real directories.
    fn app_on(left: &std::path::Path, right: &std::path::Path) -> App {
        // These tests run real jobs against real files, and the wastebasket they would use is
        // the desktop's own — not something a build machine has, and not something a test
        // should be putting things into. So `F8` here deletes, and what the setting does to
        // the job is tested where no job runs (`app::tests`).
        let mut cfg = Config::default();
        cfg.general.use_trash = false;
        App::new(Startup {
            cfg,
            paths: Paths::discover(Some(&left.join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(left),
            right: VPath::local(right),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        })
    }

    /// Serve every request the app has queued, from the real filesystem and through the real
    /// workers, so a test drives the same code the binary does.
    fn serve(app: &mut App) {
        for request in app.take_requests() {
            match request {
                Request::Read {
                    panel,
                    path,
                    generation,
                    facts,
                } => {
                    let cancel = CancelToken::new();
                    let result = if path.in_container() {
                        fcmd_vfs::ArcVfs::new(fcmd_arc::Limits::default())
                            .list_blocking(&path, &cancel)
                    } else {
                        fcmd_vfs::local::list_blocking(&path, &cancel)
                    };
                    app.handle_event(Event::Listed {
                        panel,
                        generation,
                        result: result.map_err(|e| e.to_string()),
                    });
                    if facts && path.is_local() && !path.in_container() {
                        let local = path.os().to_path_buf();
                        app.handle_event(Event::Place {
                            panel,
                            mount: fcmd_platform::mount_of(&local),
                            git: fcmd_core::git::describe(&local).map(Box::new),
                        });
                    }
                }
                // The side file, read and written for real, so a test drives the same two
                // functions the binary does.
                Request::Links {
                    panel,
                    generation,
                    dir,
                    names,
                } => {
                    let links = links_of(&dir, &names);
                    app.handle_event(Event::Links {
                        panel,
                        generation,
                        links,
                    });
                }
                Request::Descriptions {
                    panel,
                    generation,
                    dir,
                    rules,
                } => {
                    let sides = Sides::here(std::sync::Arc::new(fcmd_arc::Cache::new(
                        Default::default(),
                    )));
                    let descriptions = read_descriptions(&dir, &sides, &rules);
                    app.handle_event(Event::Descriptions {
                        panel,
                        generation,
                        descriptions,
                    });
                }
                Request::WriteDescriptions {
                    dir,
                    descriptions,
                    rules,
                } => {
                    let sides = Sides::here(std::sync::Arc::new(fcmd_arc::Cache::new(
                        Default::default(),
                    )));
                    let _ = write_descriptions(&dir, &descriptions, &sides, &rules);
                }
                Request::CarryDescriptions(carry) => carry_descriptions(
                    &carry,
                    &Sides::here(std::sync::Arc::new(
                        fcmd_arc::Cache::new(Default::default()),
                    )),
                ),
                Request::Job { id, spec, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    // The archive layer, as the program has it: a job whose source is inside
                    // a zip has to read it out rather than copy the zip.
                    let fs = ArcFs::new(
                        std::sync::Arc::new(fcmd_arc::Cache::new(Default::default())),
                        fcmd_jobs::LocalFs,
                    );
                    run_job(id, spec, cancel, tx, &fs);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::Plugin {
                    id,
                    label,
                    program,
                    cwd,
                    ctx,
                    timeout,
                } => {
                    // In the pump it runs where it stands: a test wants the answer, not a
                    // thread.
                    let result =
                        fcmd_plug::host::run_command(&program, cwd.as_deref(), &ctx, timeout)
                            .map_err(|e| e.to_string());
                    app.handle_event(Event::PluginDone { id, label, result });
                }
                Request::Link { target, link, kind } => {
                    let error = fcmd_jobs::create_link(&fcmd_jobs::LocalFs, &target, &link, kind)
                        .err()
                        .map(|e| e.to_string());
                    app.handle_event(Event::LinkDone { link, error });
                }
                // A tunnel needs a runtime and a connection, and the test server has
                // neither: the request is seen and nothing is opened.
                Request::SetAttrs {
                    id,
                    paths,
                    change,
                    recursive,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    run_attrs(
                        id,
                        &paths,
                        &change,
                        recursive,
                        cancel,
                        tx,
                        &fcmd_jobs::LocalFs,
                    );
                    for event in rx {
                        app.handle_event(event);
                    }
                }
                Request::TunnelOpen { .. } | Request::TunnelClose { .. } | Request::TunnelList => {}
                Request::Duplicates { id, root, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    run_duplicates(id, root, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::CompareTrees {
                    id,
                    left,
                    right,
                    work,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    run_compare_trees(id, left, right, work, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::CompareContents {
                    id,
                    pairs,
                    work,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    run_compare(id, pairs, work, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::Checksums {
                    id,
                    paths,
                    algorithm,
                    target,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    run_checksums(id, &paths, algorithm, target, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::Verify { id, list, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    run_verify(id, &list, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                // No terminal in a test: what the request carries is checked where it is made.
                Request::Font { .. }
                | Request::Mouse { .. }
                | Request::Clipboard { .. }
                | Request::Owners { .. } => {}
                Request::SyncPlan {
                    id,
                    left,
                    right,
                    options,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    run_sync_plan(id, &left, &right, &options, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::Find { id, spec, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    let archives = std::sync::Arc::new(fcmd_arc::Cache::new(Default::default()));
                    let groups = app.cfg.mask_groups();
                    run_find(id, *spec, cancel, tx, &archives, &groups);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::TempRefresh {
                    panel,
                    generation,
                    title,
                    paths,
                } => {
                    let items = paths
                        .into_iter()
                        .filter_map(|path| {
                            fcmd_vfs::local::stat_blocking(&path)
                                .ok()
                                .map(|entry| (path, entry))
                        })
                        .collect();
                    app.handle_event(Event::TempListed {
                        panel,
                        generation,
                        title,
                        items,
                    });
                }
                Request::Fetch { id, path, purpose } => {
                    let archives = std::sync::Arc::new(fcmd_arc::Cache::new(Default::default()));
                    let result = fetch_entry(&archives, &path);
                    app.handle_event(Event::Fetched {
                        id,
                        remote: path,
                        purpose,
                        result,
                    });
                }
                Request::TreeScan {
                    root,
                    limit,
                    cancel,
                } => {
                    // In the pump it runs where it stands: a test wants the answer, not a
                    // thread.
                    let (dirs, whole) = scan_tree(&root, limit, &cancel);
                    app.handle_event(Event::TreeScanned { root, dirs, whole });
                }
                Request::TreeRead { panel, path } => {
                    let dirs = fcmd_vfs::local::list_blocking(&path, &CancelToken::new())
                        .map(|entries| {
                            entries
                                .into_iter()
                                .filter(|e| {
                                    e.kind == fcmd_core::EntryKind::Dir
                                        || (e.kind == fcmd_core::EntryKind::Symlink
                                            && std::fs::metadata(path.join(&e.name).as_path())
                                                .map(|m| m.is_dir())
                                                .unwrap_or(false))
                                })
                                .map(|e| path.join(&e.name))
                                .collect()
                        })
                        .unwrap_or_default();
                    app.handle_event(Event::TreeListed { panel, path, dirs });
                }
                Request::Pack {
                    id,
                    archive,
                    format,
                    level,
                    solid,
                    sources,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    let archives = std::sync::Arc::new(fcmd_arc::Cache::new(Default::default()));
                    run_pack(
                        id, archive, format, level, solid, sources, cancel, tx, &archives,
                    );
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::ArcTest {
                    id,
                    archive,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    let archives = std::sync::Arc::new(fcmd_arc::Cache::new(Default::default()));
                    run_test(id, archive, cancel, tx, &archives);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::ArcRebuild {
                    id,
                    archive,
                    changes,
                    level,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    let archives = std::sync::Arc::new(fcmd_arc::Cache::new(Default::default()));
                    run_rebuild(id, archive, changes, level, cancel, tx, &archives);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                // The one request a test serves for real: it runs no program unless there
                // is a service manager, and on a machine without one it answers at once.
                Request::ServiceControl { side, id, action } => {
                    let result = fcmd_platform::control(&id, action);
                    app.handle_event(Event::ServiceControlled {
                        side,
                        id,
                        action,
                        result,
                    });
                }
                // Panelize runs its command in the tests too: what it prints is what the
                // panel then holds, and that is the whole of the feature.
                Request::Panelize { id, line, cwd } => {
                    let lines = std::sync::Mutex::new(Vec::new());
                    let (_, status) = run_shell(&line, &cwd, None, &|text| {
                        if let Ok(mut lines) = lines.lock() {
                            lines.extend(text.lines().map(str::to_string));
                        }
                    });
                    let lines = lines.into_inner().unwrap_or_default();
                    app.handle_event(Event::Panelized {
                        id,
                        line,
                        lines,
                        status,
                    });
                }
                Request::DirSize { .. }
                | Request::Shell { .. }
                | Request::Decompile { .. }
                | Request::Connect { .. }
                | Request::Disconnect { .. }
                | Request::ArcPassword { .. }
                | Request::Upload { .. } => {}
            }
        }
    }

    /// A key, by its name in the keymap.
    fn key(app: &mut App, k: &str) {
        app.handle_input(Input::Key(k.parse().unwrap()));
    }

    #[test]
    fn the_banner_is_the_name_in_plain_letters() {
        // Five lines, and nothing in them a terminal might not have: this is written before
        // anything is known about what the terminal can draw.
        assert_eq!(BANNER.lines().count(), 5);
        assert!(BANNER.is_ascii(), "{BANNER}");
        assert!(!BANNER.contains('\t'));
        // The widest line still fits the narrowest terminal anybody uses.
        assert!(BANNER.lines().all(|l| l.len() <= 40), "{BANNER}");
    }

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    /// The manual page in the repository is `--man`'s own output. A packager installs that
    /// file, so it has to be the page this binary describes — including its version, which
    /// is why a release that forgets to regenerate it fails here rather than shipping a page
    /// that names the version before.
    #[test]
    fn the_manual_page_in_the_tree_is_the_one_this_binary_generates() {
        let generated = man_page();
        let committed = include_str!("../../../docs/fcmd.1");
        // Compared line by line with the line endings taken off: on Windows the checkout
        // converts them, so a byte-for-byte comparison would fail for a file nobody touched.
        let lines = |text: &str| -> Vec<String> {
            text.lines()
                .map(|l| l.trim_end_matches('\r').to_string())
                .collect()
        };
        assert_eq!(
            lines(&generated),
            lines(committed),
            "docs/fcmd.1 is out of date — regenerate it with:\n\
             \n    cargo run -p fcmd -- --man > docs/fcmd.1\n"
        );
        // And it is a manual page rather than a dump of `--help`: the sections a reader
        // expects are there, and the ones clap cannot know about are the point of writing it.
        for section in [
            ".SH NAME",
            ".SH SYNOPSIS",
            ".SH DESCRIPTION",
            ".SH OPTIONS",
            ".SH FILES",
            ".SH KEYS",
            ".SH EXIT STATUS",
            ".SH SEE ALSO",
            ".SH BUGS",
        ] {
            assert!(
                generated.contains(section),
                "no {section} in the manual page"
            );
        }
        // Every option of the command line is described. A flag added without a line in the
        // page is impossible by construction, and this says so out loud.
        for flag in [
            "\\-\\-config",
            "\\-\\-keymap",
            "\\-\\-theme",
            "\\-\\-lang",
            "\\-\\-dump\\-config",
            "\\-\\-check\\-keymap",
            "\\-\\-list\\-commands",
        ] {
            assert!(generated.contains(flag), "{flag} is not in the manual page");
        }
    }

    #[test]
    fn cli_parses_paths_and_overrides() {
        let cli = Cli::parse_from([
            "fcmd",
            "/tmp",
            "/home",
            "--theme",
            "matrix",
            "--set",
            "panel.show_hidden=true",
        ]);
        assert_eq!(cli.paths.len(), 2);
        assert_eq!(cli.theme.as_deref(), Some("matrix"));
        assert_eq!(cli.set, vec!["panel.show_hidden=true"]);
    }

    #[test]
    fn command_ids_are_unique_and_parseable() {
        let ids = command_ids();
        let mut sorted = ids.clone();
        sorted.dedup();
        assert_eq!(ids.len(), sorted.len(), "duplicate command id");
        for id in ids {
            assert!(
                FcCommand::parse(&id).is_some(),
                "`{id}` does not parse back"
            );
        }
    }

    #[test]
    fn run_job_copies_for_real_and_reports_through_the_channel() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.txt"), b"hello").unwrap();
        std::fs::create_dir(dir.path().join("dst")).unwrap();

        let (tx, rx) = mpsc::channel();
        run_job(
            7,
            JobSpec::new(
                fcmd_jobs::JobKind::Copy,
                vec![VPath::local(dir.path().join("src"))],
                Some(VPath::local(dir.path().join("dst"))),
            ),
            CancelToken::new(),
            tx,
            &fcmd_jobs::LocalFs,
        );

        let mut done = None;
        let mut progress_frames = 0;
        while let Ok(event) = rx.try_recv() {
            match event {
                Event::JobProgress { id, .. } => {
                    assert_eq!(id, 7);
                    progress_frames += 1;
                }
                Event::JobDone { id, kind, outcome } => {
                    assert_eq!((id, kind), (7, fcmd_jobs::JobKind::Copy));
                    done = Some(outcome);
                }
                other => panic!("unexpected event {other:?}"),
            }
        }
        let outcome = done.expect("the job reported no result");
        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!((outcome.files, outcome.bytes), (1, 5));
        assert!(progress_frames > 0, "no progress was reported");
        assert_eq!(
            std::fs::read(dir.path().join("dst/src/a.txt")).unwrap(),
            b"hello"
        );
    }

    /// The whole loop, without a terminal: keys in, job out, filesystem changed, panels
    /// refreshed. This is the test that would have caught every wiring mistake.
    #[test]
    fn f5_copies_a_real_file_end_to_end() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(&left).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("report.txt"), b"the numbers").unwrap();

        let mut app = app_on(&left, &right);

        // Serve the initial panel reads from the real filesystem.
        serve(&mut app);

        // Cursor onto report.txt, then F5 and Enter to accept the prefilled destination.
        key(&mut app, "Down");
        assert_eq!(
            app.panel().current().unwrap().name_str(),
            "report.txt",
            "the cursor is not where the test thinks"
        );
        key(&mut app, "F5");
        key(&mut app, "Enter");
        serve(&mut app);
        serve(&mut app); // the refresh reads queued by the finished job

        assert_eq!(
            std::fs::read(right.join("report.txt")).unwrap(),
            b"the numbers",
            "the copy did not happen"
        );
        assert!(
            app.panels[1].listing.row_of_name("report.txt").is_some(),
            "the passive panel was not refreshed"
        );
        assert!(app.status.contains("11 B"), "{}", app.status);
    }

    /// The whole chain: two trees on disk, the table, a row answered by hand, and then the
    /// files where the table said they would be.
    #[test]
    fn a_synchronisation_does_what_its_table_said_and_nothing_else() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("source");
        let right = root.path().join("backup");
        std::fs::create_dir_all(left.join("shared")).unwrap();
        std::fs::create_dir_all(left.join("fresh/deeper")).unwrap();
        std::fs::create_dir_all(right.join("shared")).unwrap();
        std::fs::write(left.join("new.txt"), b"brand new").unwrap();
        // A file in a directory both sides have: it has to land at its own relative path.
        std::fs::write(left.join("shared/under.txt"), b"in a subdirectory").unwrap();
        // A whole directory only the left side has: one row, and the copy takes its tree.
        std::fs::write(left.join("fresh/deeper/one.txt"), b"deep inside").unwrap();
        // Different lengths on purpose: written in the same instant, two files of the same
        // size would look identical to a comparison by size and time, which is what the
        // *Compare the bytes* option exists for and not what this test is about.
        std::fs::write(left.join("changed.txt"), b"the new text, which is longer").unwrap();
        std::fs::write(right.join("changed.txt"), b"the old text").unwrap();
        std::fs::write(right.join("extra.txt"), b"only on the right").unwrap();

        let mut app = app_on(&left, &right);
        serve(&mut app);
        app.run(fcmd_cmd::Command::ToolSyncDirs);
        assert!(
            matches!(&app.mode, app::Mode::Form(f) if f.kind == menu::FormKind::Sync),
            "the options come first"
        );
        key(&mut app, "F10"); // the defaults: one way, into the subdirectories
        serve(&mut app);

        // The table, before anything has happened.
        let app::Mode::Sync(view) = &app.mode else {
            panic!("the table is on screen, not {:?}", app.mode)
        };
        assert!(view.done);
        let action = |rel: &str| {
            view.pairs
                .iter()
                .find(|p| p.rel == rel)
                .unwrap_or_else(|| panic!("no row for {rel}"))
                .action
        };
        use fcmd_jobs::sync::Action;
        assert_eq!(action("new.txt"), Action::Right);
        assert_eq!(action("changed.txt"), Action::Right);
        assert_eq!(action("shared/under.txt"), Action::Right);
        assert_eq!(
            action("fresh"),
            Action::Right,
            "a directory only one side has is one row, not one per file inside it"
        );
        assert_eq!(
            action("extra.txt"),
            Action::Skip,
            "one way leaves an extra file alone unless asked"
        );
        // Nothing has been written yet: the table is a question.
        assert!(!right.join("new.txt").exists());

        // Answer one row by hand — the extra file is to go — and run it.
        let row = view
            .pairs
            .iter()
            .position(|p| p.rel == "extra.txt")
            .expect("the extra row");
        let app::Mode::Sync(view) = &mut app.mode else {
            unreachable!("just matched")
        };
        view.cursor = row;
        // `Space` answers the row *and steps down*, so answering one row twice means putting
        // the cursor back on it first — otherwise the second press answers its neighbour.
        let answer_again = |app: &mut App, row: usize| {
            let app::Mode::Sync(view) = &mut app.mode else {
                panic!("still the table")
            };
            view.cursor = row;
            key(app, "Space");
            let app::Mode::Sync(view) = &app.mode else {
                panic!("still the table")
            };
            view.pairs[row].action
        };
        // The ring offers only what this pair allows. A row only the right side has cannot be
        // copied rightwards and cannot be deleted on the left, so it is: leave it, bring it
        // over, take it out.
        assert_eq!(answer_again(&mut app, row), Action::Left, "bring it over");
        assert_eq!(
            answer_again(&mut app, row),
            Action::DeleteRight,
            "then take it out"
        );
        // And nothing else was answered on the way: every other row is as the walk left it.
        let app::Mode::Sync(view) = &app.mode else {
            panic!("still the table")
        };
        assert_eq!(
            view.pairs
                .iter()
                .filter(|p| p.action == Action::Right)
                .count(),
            4,
            "the four the walk decided are still the four"
        );
        // The wastebasket is off in these tests, so this is a plain delete.
        key(&mut app, "F2");
        for _ in 0..6 {
            serve(&mut app);
        }

        assert_eq!(
            std::fs::read(right.join("new.txt")).unwrap(),
            b"brand new",
            "the new file came over"
        );
        assert_eq!(
            std::fs::read(right.join("changed.txt")).unwrap(),
            b"the new text, which is longer",
            "the changed one was overwritten without asking: the table was the question"
        );
        assert_eq!(
            std::fs::read(right.join("shared/under.txt")).unwrap(),
            b"in a subdirectory",
            "and the one in a subdirectory kept its path"
        );
        assert_eq!(
            std::fs::read(right.join("fresh/deeper/one.txt")).unwrap(),
            b"deep inside",
            "the new directory brought its whole tree"
        );
        assert!(
            !right.join("extra.txt").exists(),
            "the row that was answered by hand took it out"
        );
    }

    #[test]
    fn a_checksum_list_is_written_and_then_checks_out() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("rel");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("one.bin"), b"abc").unwrap();
        std::fs::write(dir.join("two.bin"), b"def").unwrap();

        let mut app = app_on(&dir, root.path());
        serve(&mut app);

        // Select both files and open the dialog from the Files menu.
        key(&mut app, "Down");
        key(&mut app, "Ins");
        key(&mut app, "Ins");
        assert_eq!(app.panel().selection.len(), 2);
        app.run(fcmd_cmd::Command::FileChecksum);
        assert!(
            matches!(&app.mode, app::Mode::Form(f) if f.kind == menu::FormKind::Checksum),
            "the dialog opens first"
        );
        // Accept the defaults: SHA-256, one list, named after the directory.
        key(&mut app, "F10");
        serve(&mut app);
        serve(&mut app); // the panel read the list queued

        let list = dir.join("rel.sha256");
        let body = std::fs::read_to_string(&list).expect("the list was written");
        assert!(body.contains("  one.bin\n"), "{body}");
        assert!(
            body.starts_with("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"),
            "the digest of `abc`: {body}"
        );

        // Now change one of the files and check the list: the bad one is named and selected.
        std::fs::write(dir.join("two.bin"), b"something else").unwrap();
        serve(&mut app);
        let row = app
            .panel()
            .listing
            .row_of_name("rel.sha256")
            .expect("the list is in the panel");
        app.panel_mut().cursor_to(row);
        app.run(fcmd_cmd::Command::FileChecksumVerify);
        serve(&mut app);
        match &app.mode {
            app::Mode::Message { lines, .. } => {
                assert!(
                    lines.iter().any(|l| l.contains("two.bin")),
                    "the one that changed is named: {lines:?}"
                );
                assert!(
                    !lines.iter().any(|l| l.contains("one.bin")),
                    "and the one that did not is not: {lines:?}"
                );
            }
            other => panic!("expected the report, got {other:?}"),
        }
        assert_eq!(
            app.panel().selection.len(),
            1,
            "what does not match is left selected"
        );
    }

    #[test]
    fn f8_deletes_a_real_file_end_to_end() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("work");
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sub/one.txt"), b"x").unwrap();
        std::fs::write(dir.join("two.txt"), b"yy").unwrap();

        let mut app = app_on(&dir, root.path());
        serve(&mut app);

        // Select the subdirectory and the file, then delete both.
        key(&mut app, "Down");
        key(&mut app, "Ins");
        key(&mut app, "Ins");
        assert_eq!(app.panel().selection.len(), 2);
        key(&mut app, "F8");
        key(&mut app, "Enter"); // confirm
        serve(&mut app);
        serve(&mut app);

        assert!(!dir.join("sub").exists(), "the directory survived");
        assert!(!dir.join("two.txt").exists(), "the file survived");
        assert_eq!(app.panel().listing.totals.files, 0);
    }

    /// The whole chain through the real engine: a tree on disk, a search by mask and by text,
    /// the results in the list, and then the results in the panel.
    #[test]
    fn a_search_finds_what_is_there_and_the_panel_can_hold_it() {
        let root = tempfile::tempdir().unwrap();
        let src = root.path().join("src");
        std::fs::create_dir_all(src.join("deep")).unwrap();
        std::fs::write(src.join("one.rs"), b"fn needle() {}\n").unwrap();
        std::fs::write(src.join("deep/two.rs"), b"// needle here\n").unwrap();
        std::fs::write(src.join("three.rs"), b"nothing of the sort\n").unwrap();
        std::fs::write(src.join("notes.txt"), b"needle in text\n").unwrap();

        let mut app = app_on(&src, root.path());
        serve(&mut app);

        app.last_find = app::FindSpec {
            mask: "*.rs".into(),
            text: "needle".into(),
            ..app::FindSpec::default()
        };
        app.run(FcCommand::ToolFind);
        key(&mut app, "F10");
        serve(&mut app);

        let rows = match &app.mode {
            app::Mode::Find(view) => {
                assert!(view.done, "the search did not finish");
                assert!(view.files >= 4, "only {} files looked at", view.files);
                view.rows.clone()
            }
            // A search that cannot start closes the list and says why in the status line.
            _ => panic!("{}", app.status),
        };
        // The two `.rs` files that contain it — not the one that does not, and not the `.txt`
        // that does.
        let names: Vec<String> = rows.iter().map(|(p, _)| p.display_name()).collect();
        assert_eq!(rows.len(), 2, "{names:?}");
        assert!(names.contains(&"one.rs".to_string()), "{names:?}");
        assert!(names.contains(&"two.rs".to_string()), "{names:?}");

        // F5 puts them in the panel, where the rest of the keys work on them. The two files
        // live in different directories, so only a temporary panel can hold both.
        key(&mut app, "F5");
        assert_eq!(app.panel().listing.len(), 2);
        let under_cursor = app.panel().current_path().expect("a path for the row");
        assert!(
            under_cursor.to_string().ends_with(".rs"),
            "{under_cursor:?}"
        );
    }

    /// `Enter` on an archive, the panel inside it, `F3` on an entry, and a copy out of it —
    /// the whole of what 0.0.5 promises, through the real engine and the real provider.
    #[test]
    fn enter_goes_into_an_archive_and_everything_else_works_in_there() {
        let root = tempfile::tempdir().unwrap();
        let out = root.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let zip = root.path().join("tree.zip");
        {
            let file = std::fs::File::create(&zip).unwrap();
            let mut writer = zip::ZipWriter::new(file);
            for (name, body) in [
                ("readme.md", &b"# the readme"[..]),
                ("src/a.txt", &b"aaaa"[..]),
            ] {
                writer
                    .start_file(name, zip::write::SimpleFileOptions::default())
                    .unwrap();
                std::io::Write::write_all(&mut writer, body).unwrap();
            }
            writer.finish().unwrap();
        }

        let mut app = app_on(root.path(), &out);
        serve(&mut app);

        // The cursor onto the zip — `..`, then the `out` directory, then the file — and in.
        key(&mut app, "Down");
        key(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "tree.zip");
        key(&mut app, "Enter");
        serve(&mut app);
        assert!(app.panel().cwd.in_container(), "{}", app.panel().cwd);
        let names: Vec<String> = (0..app.panel().listing.len())
            .map(|i| app.panel().listing.row(i).unwrap().name_str().into_owned())
            .collect();
        // Directories first, like any other panel: the sort is the panel's, not the archive's.
        assert_eq!(names, ["..", "src", "readme.md"], "{}", app.status);
        // The frame says the archive's name, not the whole path.
        assert_eq!(app.panel().cwd.display_name(), "tree.zip");

        // `F3` brings the bytes out and reads them.
        key(&mut app, "Down");
        key(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "readme.md");
        key(&mut app, "F3");
        serve(&mut app);
        let shown = {
            let viewer = app.mode.viewer_mut().expect("the viewer");
            (viewer.path.display_name(), viewer.len())
        };
        assert_eq!(shown, ("readme.md".to_string(), 12));
        key(&mut app, "Esc");

        // `F5` copies out of the archive through the ordinary job engine.
        key(&mut app, "F5");
        key(&mut app, "Enter"); // accept the prefilled destination
        serve(&mut app);
        serve(&mut app);
        assert_eq!(
            std::fs::read(out.join("readme.md")).unwrap(),
            b"# the readme"
        );

        // And `..` at the root of the archive leaves it, with the cursor back on the file.
        key(&mut app, "Home");
        key(&mut app, "Enter");
        serve(&mut app);
        assert!(!app.panel().cwd.in_container());
        assert_eq!(app.panel().cwd, VPath::local(root.path()));
    }

    /// The tree on a path that goes through a symbolic link — `/tmp` and `/var` on macOS are
    /// links, and a tree that dropped them could not show where the user is, which is what
    /// made `Ctrl+T` spin there.
    #[cfg(unix)]
    #[test]
    fn the_tree_follows_a_link_to_a_directory() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("real/inside")).unwrap();
        std::os::unix::fs::symlink(root.path().join("real"), root.path().join("link")).unwrap();

        // The other panel sits inside the link, so the tree has to open its way through it.
        let here = root.path().join("link/inside");
        let mut app = app_on(&here, root.path());
        serve(&mut app);

        key(&mut app, "Ctrl+T");
        for _ in 0..40 {
            if app.requests.is_empty() {
                break;
            }
            serve(&mut app);
        }
        let tree = app.trees[1].as_ref().expect("a tree");
        assert!(
            tree.want.is_none(),
            "it is still looking for {:?}",
            tree.want
        );
        // Through the link, the real directory is the one the tree lands on.
        assert_eq!(
            tree.path(),
            Some(fcmd_vfs::local::real_path(&VPath::local(&here)))
        );
        assert!(
            tree.rows.iter().any(|r| r.name() == "real"),
            "the link is not in the tree: {:?}",
            tree.rows.iter().map(|r| r.name()).collect::<Vec<_>>()
        );
    }

    /// `Shift+F1` builds an archive out of the selection, and `Shift+F2` takes one apart —
    /// through the real writer, the real reader and the real job engine.
    #[test]
    fn an_archive_is_built_from_the_selection_and_taken_apart_again() {
        let root = tempfile::tempdir().unwrap();
        let left = root.path().join("left");
        let right = root.path().join("right");
        std::fs::create_dir_all(left.join("src")).unwrap();
        std::fs::create_dir_all(&right).unwrap();
        std::fs::write(left.join("src/main.rs"), b"fn main() {}\n").unwrap();
        std::fs::write(left.join("readme.md"), b"# readme\n").unwrap();

        let mut app = app_on(&left, &right);
        serve(&mut app);

        // Select both rows and pack them: the dialog offers the archive in the other panel,
        // named after this directory.
        key(&mut app, "Down");
        key(&mut app, "Ins");
        key(&mut app, "Ins");
        assert_eq!(app.panel().selection.len(), 2);
        key(&mut app, "Shift+F1");
        let name = match &app.mode {
            crate::app::Mode::Form(form) => form.text_value("pack.name"),
            other => panic!("{other:?}"),
        };
        assert_eq!(name, right.join("left.zip").to_string_lossy());
        key(&mut app, "F10");
        serve(&mut app);

        let archive = right.join("left.zip");
        assert!(archive.exists(), "{}", app.status);
        let read =
            fcmd_arc::Archive::open(&archive, &fcmd_arc::Limits::default(), &CancelToken::new())
                .expect("the archive we just wrote");
        let names: Vec<String> = read
            .list("/")
            .iter()
            .map(|e| e.name_str().into_owned())
            .collect();
        assert_eq!(names, ["readme.md", "src"]);
        assert_eq!(
            read.read("/src/main.rs", &CancelToken::new()).unwrap(),
            b"fn main() {}\n"
        );

        // Now the other way: the cursor onto the archive, `Shift+F2`, and its contents land
        // in a directory of its own — not in one called `left.zip`.
        key(&mut app, "Tab");
        serve(&mut app);
        while app.panel().current().map(|e| e.name_str().into_owned()) != Some("left.zip".into()) {
            key(&mut app, "Down");
        }
        key(&mut app, "Shift+F2");
        let to = match &app.mode {
            crate::app::Mode::Form(form) => form.text_value("unpack.to"),
            other => panic!("{other:?}"),
        };
        assert_eq!(to, left.join("left").to_string_lossy());
        key(&mut app, "F10");
        serve(&mut app);
        serve(&mut app);

        assert_eq!(
            std::fs::read(left.join("left/src/main.rs")).unwrap(),
            b"fn main() {}\n"
        );
        assert!(
            !left.join("left/left.zip").exists(),
            "the contents were put inside a directory named after the archive"
        );
    }

    /// A file that is not an archive, and a pack with nothing selected: both say so.
    #[test]
    fn the_two_ways_to_press_the_archive_keys_wrongly_both_say_why() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("notes.txt"), b"text").unwrap();
        let mut app = app_on(root.path(), root.path());
        serve(&mut app);

        key(&mut app, "Down");
        assert_eq!(app.panel().current().unwrap().name_str(), "notes.txt");
        key(&mut app, "Shift+F2");
        assert!(app.mode.is_normal(), "a dialog opened on a text file");
        assert!(app.status_error, "{}", app.status);

        // `..` is not something to pack.
        key(&mut app, "Home");
        key(&mut app, "Shift+F1");
        assert!(app.mode.is_normal());
        assert!(app.status_error, "{}", app.status);
    }

    /// `Ctrl+T`: the tree panel opens its way down to where the other panel is, and moving
    /// its cursor takes that panel with it.
    #[test]
    fn the_tree_panel_opens_on_the_other_panels_directory_and_drives_it() {
        let root = tempfile::tempdir().unwrap();
        for dir in ["one", "one/deep", "two"] {
            std::fs::create_dir_all(root.path().join(dir)).unwrap();
        }
        std::fs::write(root.path().join("one/a.txt"), b"a").unwrap();

        let here = root.path().join("one");
        let mut app = app_on(&here, root.path());
        serve(&mut app);

        key(&mut app, "Ctrl+T");
        // Rooted at the filesystem root and opening its way down takes several reads.
        for _ in 0..40 {
            if app.requests.is_empty() {
                break;
            }
            serve(&mut app);
        }
        assert_eq!(app.panels[1].content, crate::panel::Content::Tree);
        assert_eq!(app.active, 1, "the tree takes the focus");
        let tree = app.trees[1].as_ref().expect("a tree");
        // The filesystem's own spelling of the directory: on macOS a temporary path goes
        // through `/private`, and on Windows it can arrive in its short `RUNNER~1` form.
        let real = fcmd_vfs::local::real_path(&VPath::local(&here));
        assert_eq!(
            tree.path(),
            Some(real),
            "the cursor should be on the other panel's own directory"
        );
        assert!(tree.want.is_none(), "it stopped looking");

        // `→` opens the row under the cursor — `one` holds `deep` — and `←` closes it again.
        let before = app.trees[1].as_ref().unwrap().rows.len();
        key(&mut app, "Right");
        serve(&mut app);
        let opened = app.trees[1].as_ref().unwrap();
        assert!(opened.rows.len() > before, "nothing opened");
        assert!(
            opened.rows.iter().any(|r| r.name() == "deep"),
            "{:?}",
            opened.rows.iter().map(|r| r.name()).collect::<Vec<_>>()
        );
        key(&mut app, "Left");
        assert_eq!(app.trees[1].as_ref().unwrap().rows.len(), before);

        // Up one row is the parent; the file panel follows the cursor.
        key(&mut app, "Up");
        serve(&mut app);
        let on = app.trees[1].as_ref().unwrap().path().unwrap();
        assert_eq!(app.panels[0].cwd, on, "the other panel follows the cursor");

        // And the key puts the files back, with the focus where the keys belong.
        key(&mut app, "Ctrl+T");
        assert_eq!(app.panels[1].content, crate::panel::Content::Files);
        assert!(app.trees[1].is_none());
    }

    /// A search that finds nothing has to say so, rather than leaving an empty list that
    /// looks like a search still running.
    #[test]
    fn a_search_for_what_is_not_there_says_so_rather_than_nothing() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("one.rs"), b"fn main() {}\n").unwrap();

        let mut app = app_on(root.path(), root.path());
        serve(&mut app);

        app.last_find = app::FindSpec {
            mask: "*".into(),
            text: "absolutely-not-in-there".into(),
            ..app::FindSpec::default()
        };
        app.run(FcCommand::ToolFind);
        key(&mut app, "F10");
        serve(&mut app);

        match &app.mode {
            app::Mode::Find(view) => {
                assert!(view.done);
                assert!(view.rows.is_empty());
                assert!(view.files >= 1, "it did look at the file");
            }
            _ => panic!("{}", app.status),
        }
    }

    #[test]
    fn run_job_reports_a_plan_failure_instead_of_panicking() {
        let dir = tempfile::tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        run_job(
            1,
            JobSpec::new(
                fcmd_jobs::JobKind::Delete,
                vec![VPath::local(dir.path().join("ghost"))],
                None,
            ),
            CancelToken::new(),
            tx,
            &fcmd_jobs::LocalFs,
        );
        let event = rx.recv().expect("an event");
        match event {
            Event::JobDone { outcome, .. } => {
                assert!(!outcome.is_clean());
                assert_eq!(outcome.errors.len(), 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn dir_size_walks_a_real_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), vec![0u8; 10]).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b"), vec![0u8; 32]).unwrap();
        let (bytes, files) = dir_size(
            &fcmd_jobs::LocalFs,
            &VPath::local(dir.path()),
            &CancelToken::new(),
            0,
        );
        assert_eq!((bytes, files), (42, 2));
    }

    #[test]
    fn dir_size_stops_when_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), vec![0u8; 10]).unwrap();
        let token = CancelToken::new();
        token.cancel();
        assert_eq!(
            dir_size(&fcmd_jobs::LocalFs, &VPath::local(dir.path()), &token, 0),
            (0, 0)
        );
    }

    #[test]
    fn keymap_comes_from_the_config_block_or_falls_back_to_a_builtin() {
        let mut cfg = Config::default();
        assert!(load_keymap(&cfg, "fcmd.toml", &[]).unwrap().len() > 30);

        cfg.ui.keymap = "klingon".into();
        assert!(load_keymap(&cfg, "fcmd.toml", &[]).is_err());

        let (with_block, _) = Config::parse(
            "fcmd.toml",
            r#"
            [ui]
            keymap = "klingon"
            [[keymap]]
            name = "klingon"
            inherit = "far"
            [[keymap.bind]]
            keys = ["F4"]
            cmd = "file.view"
            "#,
        )
        .unwrap();
        let km = load_keymap(&with_block, "fcmd.toml", &[]).unwrap();
        assert_eq!(km.name(), "klingon");
    }

    #[test]
    fn the_highlight_groups_are_compiled_and_a_bad_one_is_a_warning() {
        let text = r##"
[[highlight]]
name = "Rust"
mask = "*.rs"
fg = "#d7875f"

[[highlight]]
name = "Fresh"
age = "<=1"
date_field = "create"
fg = "file.archive"
bg = "idx:235"

[[highlight]]
name = "Big and hidden"
size = "1G-"
attrs = "h-s"
fg = "red"

[[highlight]]
name = "Nonsense"
size = "a handful"
fg = "red"

[[highlight]]
name = "Empty"
fg = "red"
"##;
        let (cfg, _) = Config::parse("test", text).expect("the file parses");
        assert_eq!(cfg.highlight.len(), 5);
        let mut warnings = Vec::new();
        let highlight = highlight::groups(&cfg, &mut warnings);

        // Three groups are good; the two that are not are named in the warnings and left out.
        assert_eq!(highlight.rules().len(), 3, "{warnings:?}");
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(
            warnings[0].contains("Nonsense") && warnings[0].contains("size"),
            "{warnings:?}"
        );
        assert!(
            warnings[1].contains("Empty") && warnings[1].contains("every row"),
            "{warnings:?}"
        );

        // And the conditions are the ones the words said.
        let theme = Theme::default_theme();
        let now = std::time::SystemTime::now();
        let rs = fcmd_core::Entry {
            name: "main.rs".into(),
            kind: fcmd_core::EntryKind::File,
            size: 10,
            mtime: None,
            btime: None,
            atime: None,
            flags: fcmd_core::EntryFlags::empty(),
            id: fcmd_core::EntryId::Name(1),
            link_target: None,
        };
        assert_eq!(
            highlight.paint_at(&rs, now, &theme).fg,
            Some(fcmd_ui::Color::Rgb(0xd7, 0x87, 0x5f))
        );
    }

    #[test]
    fn the_sort_group_blocks_become_bands_in_the_order_they_are_written() {
        let text = r##"
[[sort_group]]
name = "sources"
mask = "*.rs,*.toml"

[[sort_group]]
name = "output"
mask = "*.o"
place = "back"

[[sort_group]]
name = "Broken"
mask = "*.rs|*.o|*.h"

[[sort_group]]
name = "Empty"
mask = ""

[[sort_group]]
name = "Sideways"
mask = "*.md"
place = "diagonally"
"##;
        let (cfg, _) = Config::parse("test", text).expect("the file parses");
        assert_eq!(cfg.sort_groups.len(), 5);
        let mut warnings = Vec::new();
        let bands = highlight::sort_groups(&cfg, &mut warnings);

        // The two bad blocks are named and left out; the one with a bad `place` is kept, in
        // front, because where it sits is a smaller mistake than the band not existing.
        assert_eq!(bands.len(), 3, "{warnings:?}");
        assert_eq!(warnings.len(), 3, "{warnings:?}");
        assert!(warnings[0].contains("Broken"), "{warnings:?}");
        assert!(warnings[1].contains("Empty"), "{warnings:?}");
        assert!(
            warnings[2].contains("Sideways") && warnings[2].contains("diagonally"),
            "{warnings:?}"
        );

        let file = |name: &str| fcmd_core::Entry {
            name: name.into(),
            kind: fcmd_core::EntryKind::File,
            size: 0,
            mtime: None,
            btime: None,
            atime: None,
            flags: fcmd_core::EntryFlags::empty(),
            id: fcmd_core::EntryId::Name(1),
            link_target: None,
        };
        // In front, among, behind.
        assert!(bands.rank(&file("main.rs")) < 0);
        assert_eq!(bands.rank(&file("notes.txt")), 0);
        assert!(bands.rank(&file("main.o")) > 0);
        assert_eq!(
            bands.group_of(&file("main.rs")).map(|g| g.name.as_str()),
            Some("sources")
        );
    }

    #[test]
    fn paths_describe_one_file_and_one_optional_directory() {
        // A native directory, so every separator in the output is this platform's.
        let base = std::env::temp_dir().join("sc-x");
        let paths = Paths::discover(Some(&base.join("fcmd.toml")), Some(&base.join("plugins")));
        let text = paths.describe();
        for expected in ["fcmd.toml", "fcmd-state.toml", "plugins"] {
            let path = base.join(expected);
            assert!(text.contains(&path.display().to_string()), "{text}");
        }
    }
}
