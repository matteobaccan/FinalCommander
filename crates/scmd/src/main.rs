//! SupremeCommander — an orthodox file manager for the terminal.
//!
//! The interface is strictly TUI; see `docs/spec/00-overview.md`.

mod app;
mod cmdline;
mod drives;
mod expand;
mod form;
mod help;
mod menu;
mod panel;
mod render;
mod tabs;
mod termkeys;
mod tree;

use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::Parser;
use scmd_cmd::{Command as ScCommand, Keymap, Scope as ScmdScope};
use scmd_config::{Config, Paths};
use scmd_core::{CancelToken, VPath};
use scmd_i18n::I18n;
use scmd_jobs::{ArcFs, JobSpec};
use scmd_net::{Pool, RoutedFs, Router, Site};
use scmd_s3::jobs::ObjectFs;
use scmd_tui::{Input, Term};
use scmd_ui::{Buffer, Theme, ThemeSet};
use scmd_vfs::Vfs;

use crate::app::{
    App, ConnectOutcome, Event, PluginCommand, RemoteStamp, Request, Startup, UploadError,
};

#[derive(Debug, Parser)]
#[command(
    name = "scmd",
    version,
    about = "SupremeCommander — orthodox file manager for the terminal",
    long_about = "SupremeCommander is a two-panel file manager for the terminal that \
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

    /// Print the manual page, in roff, for a packager to install as scmd.1.
    ///
    /// It is generated from these very options, so it cannot describe a flag that does not
    /// exist or miss one that does. `docs/scmd.1` in the repository is this output, and a
    /// test fails if the two ever differ.
    #[arg(long, hide = true)]
    man: bool,
}

/// A search on a worker thread. The engine streams events; each one becomes an event the app
/// already knows how to draw, so the list fills while the walk is still going.
///
/// This is the one place that turns what the dialog asked for into what the engine takes: the
/// dialog knows nothing about walks, and the engine knows nothing about panels.
fn run_find(id: u64, spec: app::FindSpec, cancel: CancelToken, tx: mpsc::Sender<Event>) {
    use scmd_core::filter::{Filter, MaskSet};
    use scmd_find::{Content, Needle};

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

    let groups = scmd_core::filter::default_groups();
    let masks = match MaskSet::parse_with(&spec.mask, &groups, spec.case_sensitive) {
        Ok(masks) => masks,
        Err(e) => return fail(e.to_string()),
    };
    let mut filter = Filter::named("find");
    filter.masks = Some(masks);

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

    let query = scmd_find::Query {
        roots: spec.roots.clone(),
        filter,
        // "This folder" is the only scope that does not go down.
        subdirectories: spec.scope != app::FindScope::Panel,
        follow_links: spec.follow_links,
        include_dirs: spec.directories,
        content,
    };

    let mut problems: Vec<String> = Vec::new();
    let mut last_report = std::time::Instant::now();
    let result =
        scmd_find::find(
            &scmd_vfs::local::LocalVfs,
            &query,
            &cancel,
            &mut |event| match event {
                scmd_find::Event::Hit(hit) => {
                    let _ = tx.send(Event::FindHit {
                        id,
                        path: hit.path,
                        entry: Box::new(hit.entry),
                    });
                }
                scmd_find::Event::Scanning(dir) => {
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
                scmd_find::Event::Problem(problem) => {
                    // Capped: a search over a whole drive meets unreadable directories by the
                    // dozen, and ten thousand of them help nobody.
                    if problems.len() < 200 {
                        problems.push(format!("{}: {}", problem.path, problem.message));
                    }
                }
            },
        );
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

/// Free bytes on the device a local directory is on, for the panel frame. `std` does not
/// expose `statvfs`, so this is the one call that needs a platform crate; a remote provider
/// does not report free space at all, and Windows wants `GetDiskFreeSpaceEx`, so both answer
/// `None` rather than a figure nobody checked.
#[cfg(unix)]
fn free_space(path: &scmd_core::VPath) -> Option<u64> {
    if path.is_remote() {
        return None;
    }
    let stat = nix::sys::statvfs::statvfs(path.as_path()).ok()?;
    // `blocks_available` is what an unprivileged process may actually use, which is the
    // number a user is asking about; the free count includes the reserve only root can touch.
    Some(stat.blocks_available() as u64 * stat.fragment_size())
}

#[cfg(not(unix))]
fn free_space(_path: &scmd_core::VPath) -> Option<u64> {
    None
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("scmd: {e:#}");
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
        .title("SCMD")
        .section("1")
        .manual("SupremeCommander")
        .render(&mut page)
        .expect("writing to a vector cannot fail");
    let mut text = String::from_utf8(page).expect("roff is text");
    // What a man page has and a `--help` does not: where the files are, what the keys are, and
    // where to look next. Written here rather than in the roff by hand, so the generated part
    // stays generated.
    text.push_str(
        r#".SH FILES
.TP
.I ~/.config/scmd/config.toml
The settings. B\-\-dump\-configR prints the effective ones. On macOS this is under
.I ~/Library/Application Support/scmd
and on Windows under
.I %APPDATA%\scmd .
.TP
.I ~/.config/scmd/keymap.toml
Your own key bindings, applied over the profile named by B\-\-keymapR.
.TP
.I ~/.config/scmd/session.toml
Where the panels were when the program last exited.
.TP
.I ~/.config/scmd/themes/
Colour schemes. A TOML file dropped in here is found at startup; B\-\-export\-themeR
prints a built-in one to start from.
.TP
.I ~/.config/scmd/sites.toml
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
https://github.com/matteobaccan/SupremeCommander/issues
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
                scmd_ui::DEFAULT_THEME
            ));
            Theme::default_theme()
        }
    };
    warnings.extend(theme.warnings().iter().cloned());

    // ── informational modes: print and exit, no terminal takeover
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
                scmd_ui::ThemeSource::BuiltIn => "built-in".to_string(),
                scmd_ui::ThemeSource::Config => origin.clone(),
                scmd_ui::ThemeSource::Plugin(p) => format!("plugin:{p}"),
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
        for (tag, name) in scmd_i18n::LANGUAGES {
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
            .filter(|t| t.source == scmd_ui::ThemeSource::Config)
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

    // The histories from the last session. A broken state file is a warning, never a stop.
    let (state, state_problem) = scmd_config::State::load(&paths);
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
    });
    app.cmd.load_history(state.command_history);
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
    let found = scmd_plug::discover(&paths.plugins_dir);
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
    let found = scmd_plug::discover(&paths.plugins_dir);
    warnings.extend(found.problems.iter().cloned());
    warnings.extend(found.warnings.iter().cloned());
    let mut themes = themes;
    let mut keymaps = Vec::new();
    let mut commands: Vec<PluginCommand> = Vec::new();
    for plugin in &found.plugins {
        if !plugin.supports(env!("CARGO_PKG_VERSION")) {
            warnings.push(format!(
                "{}: declares scmd {} and this is {} — loaded anyway, it may not fit",
                plugin.id(),
                plugin.manifest.plugin.scmd,
                env!("CARGO_PKG_VERSION")
            ));
        }
        let theme_tables = plugin.tables(scmd_plug::ItemKind::Theme);
        if !theme_tables.is_empty() {
            themes = themes.with_plugin_tables(plugin.id(), &theme_tables);
        }
        for table in plugin.tables(scmd_plug::ItemKind::Keymap) {
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
                .and_then(ScmdScope::parse)
                .unwrap_or(ScmdScope::Panel);
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
    ScCommand::all().into_iter().map(|c| c.id()).collect()
}

fn interactive(mut app: App) -> Result<()> {
    scmd_tui::install_panic_hook();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .context("starting the async runtime")?;

    let mut term = Term::new().context("taking over the terminal")?;
    let (tx, rx) = mpsc::channel::<Event>();

    // Terminal input lives on its own thread: `read` blocks.
    let input_tx = tx.clone();
    std::thread::spawn(move || {
        let (raw_tx, raw_rx) = mpsc::channel::<Input>();
        std::thread::spawn(move || scmd_tui::read_input_loop(raw_tx));
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
        pool.set_policy(scmd_net::HostKeyPolicy {
            known_hosts: Some(scmd_net::site::expand_tilde(app.cfg.net.known_hosts.trim())),
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
                } => {
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
                            bytes: free_space(&path),
                        });
                    });
                }
                Request::Job { id, spec, cancel } => {
                    let tx = tx.clone();
                    // A job runs on a blocking thread, which is what lets the remote
                    // provider's async calls be driven with `block_on` (see `scmd-net`).
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
                        let (output, status) = run_shell(&line, &cwd, into.as_deref());
                        let _ = tx.send(Event::ShellDone {
                            id,
                            line,
                            output,
                            status,
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
                            Err(scmd_net::NetError::NeedsSecret { kind, .. }) => {
                                ConnectOutcome::NeedsSecret(kind)
                            }
                            Err(scmd_net::NetError::UnknownHostKey { fingerprint, .. }) => {
                                ConnectOutcome::UnknownHostKey { fingerprint }
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
                Request::Disconnect { authority } => {
                    let tx = tx.clone();
                    let pool = pool.clone();
                    runtime.spawn(async move {
                        pool.disconnect(&authority).await;
                        let _ = tx.send(Event::Disconnected {
                            authority,
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
                        } else if path.scheme() == scmd_core::Scheme::S3 {
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
                        let result = if remote.scheme() == scmd_core::Scheme::S3 {
                            put_object(&objects_for_upload, &local, &remote, expect, force).await
                        } else {
                            write_back(&pool, &local, &remote, expect, force).await
                        };
                        let _ = tx.send(Event::Uploaded { id, remote, result });
                    });
                }
                Request::Find { id, spec, cancel } => {
                    let tx = tx.clone();
                    // A blocking thread: the walk is synchronous, and it must not sit in the
                    // way of the runtime that is also serving the panels.
                    runtime.spawn_blocking(move || run_find(id, *spec, cancel, tx));
                }
                Request::CompareContents { id, pairs, cancel } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || run_compare(id, pairs, cancel, tx));
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
                                        scmd_core::EntryKind::Dir => Some(child),
                                        scmd_core::EntryKind::Symlink
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
                    sources,
                    cancel,
                } => {
                    let tx = tx.clone();
                    let archives = archives.clone();
                    runtime.spawn_blocking(move || {
                        run_pack(id, archive, format, level, sources, cancel, tx, &archives)
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
                            scmd_plug::host::run_command(&program, cwd.as_deref(), &ctx, timeout)
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
                        let error = scmd_jobs::create_link(&fs, &target, &link, kind)
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

        match rx.recv() {
            Ok(event) => app.handle_event(event),
            Err(_) => break,
        }
        // Coalesce whatever else is already queued, then draw once.
        while let Ok(event) = rx.try_recv() {
            app.handle_event(event);
        }
    }

    term.restore();

    // What survives the session: the command history, and where the panels ended up. A failure
    // here is reported on the way out rather than swallowed — losing a history silently is how
    // a program teaches people not to trust it.
    let state = scmd_config::State {
        command_history: app.cmd.history().to_vec(),
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
    };
    if let Err(e) = state.save(&app.paths) {
        eprintln!("scmd: cannot write {}: {e}", app.paths.state_file.display());
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
    fs: &dyn scmd_jobs::JobFs,
) {
    struct ChannelReporter {
        id: u64,
        tx: mpsc::Sender<Event>,
        last: std::time::Instant,
    }

    impl scmd_jobs::Reporter for ChannelReporter {
        fn progress(&mut self, progress: &scmd_jobs::Progress) {
            // 20 frames a second is plenty; a tight copy loop must not flood the channel.
            let now = std::time::Instant::now();
            let done = progress.phase == scmd_jobs::Phase::Done;
            if !done && now.duration_since(self.last) < std::time::Duration::from_millis(50) {
                return;
            }
            self.last = now;
            let _ = self.tx.send(Event::JobProgress {
                id: self.id,
                progress: progress.clone(),
            });
        }

        fn ask(&mut self, question: scmd_jobs::Question) -> scmd_jobs::Decision {
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
                return scmd_jobs::Decision::Cancel;
            }
            answer.recv().unwrap_or(scmd_jobs::Decision::Cancel)
        }
    }

    let kind = spec.kind;
    let mut reporter = ChannelReporter {
        id,
        tx: tx.clone(),
        last: std::time::Instant::now() - std::time::Duration::from_secs(1),
    };

    let outcome = match scmd_jobs::plan(fs, &spec, &cancel) {
        Ok(plan) => scmd_jobs::execute(fs, &spec, plan, &mut reporter, &cancel),
        Err(e) => scmd_jobs::Outcome {
            errors: vec![(
                spec.target.clone().unwrap_or_else(|| VPath::local(".")),
                e.to_string(),
            )],
            ..scmd_jobs::Outcome::default()
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
fn run_shell(
    line: &str,
    cwd: &std::path::Path,
    into: Option<&std::path::Path>,
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

    let mut out = child.stdout.take();
    let mut err = child.stderr.take();
    let reader = |pipe: Option<std::process::ChildStdout>| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            if let Some(mut pipe) = pipe {
                let _ = pipe.read_to_end(&mut buf);
            }
            buf
        })
    };
    let out_thread = reader(out.take());
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = err.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });

    let status = child.wait().ok().and_then(|s| s.code());
    let mut bytes = out_thread.join().unwrap_or_default();
    bytes.extend(err_thread.join().unwrap_or_default());

    let mut text = String::from_utf8_lossy(&bytes).into_owned();
    if text.len() > MAX_OUTPUT {
        text.truncate(MAX_OUTPUT);
        text.push_str("\n… output truncated\n");
    }
    if let Some(path) = into {
        return match std::fs::write(path, text.as_bytes()) {
            Ok(()) => (format!("output written to {}\n", path.display()), status),
            Err(e) => (format!("{}: {e}\n", path.display()), Some(-1)),
        };
    }
    (text, status)
}

fn run_decompiler(rule: &scmd_config::DecompileRule, path: &VPath) -> Result<String, String> {
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
    fs: &dyn scmd_jobs::JobFs,
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
            scmd_core::EntryKind::Parent | scmd_core::EntryKind::Symlink => {}
            scmd_core::EntryKind::Dir => {
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
    let dir = std::env::temp_dir().join(format!("scmd-{}", std::process::id()));
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
type Tunnels = std::sync::Arc<std::sync::Mutex<Vec<scmd_net::Tunnel>>>;

/// Open one over the connection `authority` names, and keep it.
async fn open_tunnel(
    pool: &std::sync::Arc<Pool>,
    tunnels: &Tunnels,
    id: u64,
    authority: &str,
    spec: &str,
) -> Result<(), String> {
    let spec = scmd_net::TunnelSpec::parse(spec).map_err(|e| e.to_string())?;
    let path = VPath::remote(scmd_core::Scheme::Sftp, authority, "/");
    let remote = pool.require(&path).map_err(|e| e.to_string())?;
    let tunnel = scmd_net::tunnel::open(remote, id, spec)
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
fn run_compare(
    id: u64,
    pairs: Vec<app::ComparePair>,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
) {
    use scmd_find::FindFs;
    let fs = scmd_vfs::LocalVfs;
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
        // Two different sizes are two different files, and the listing already said so.
        let same = if pair.left_size != pair.right_size {
            Some(false)
        } else {
            match (fs.open(&pair.left), fs.open(&pair.right)) {
                (Ok(mut left), Ok(mut right)) => {
                    match scmd_core::compare::same_contents(
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
                    (Ok(mut left), Ok(mut right)) => scmd_core::compare::same_but_for_line_endings(
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
    }

    let _ = tx.send(Event::CompareDone {
        id,
        differ,
        line_endings,
        unreadable,
        stopped,
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
    objects: &scmd_s3::vfs::S3Vfs,
    path: &VPath,
) -> Result<(PathBuf, RemoteStamp), String> {
    let cancel = CancelToken::new();
    let entry = Vfs::stat(objects, path).await.map_err(|e| e.to_string())?;
    let bytes = objects
        .read_object(path, None, &cancel)
        .await
        .map_err(|e| e.to_string())?;
    let dir = std::env::temp_dir().join(format!("scmd-{}", std::process::id()));
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
/// sees is the same; what runs underneath is one pass through `scmd-arc`.
#[allow(clippy::too_many_arguments)]
fn run_pack(
    id: u64,
    archive: VPath,
    format: scmd_arc::Format,
    level: scmd_arc::Level,
    sources: Vec<VPath>,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<scmd_arc::Cache>,
) {
    use scmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    // A local pack only: writing an archive straight onto a server means uploading it as it
    // grows, and SPEC 04 §4's rebuild is where that belongs.
    let not_local = std::iter::once(&archive)
        .chain(sources.iter())
        .find(|p| !p.is_local() || p.in_container());
    if let Some(path) = not_local {
        let _ = tx.send(Event::JobDone {
            id,
            kind: scmd_jobs::JobKind::Pack,
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
    let result = scmd_arc::create(
        archive.as_path(),
        format,
        &paths,
        level,
        &cancel,
        &mut progress,
    );
    // The archive on disk is new, so any index of a file by that name is not it any more.
    archives.forget(archive.as_path());

    let outcome = match result {
        Ok(written) => Outcome {
            files: written.files,
            dirs: written.dirs,
            bytes: written.bytes,
            ..Outcome::default()
        },
        Err(scmd_arc::ArcError::Cancelled) => Outcome {
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
        kind: scmd_jobs::JobKind::Pack,
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
    archives: &std::sync::Arc<scmd_arc::Cache>,
) {
    use scmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    let outcome = match archives.open(archive.as_path(), &cancel) {
        Ok(handle) => {
            let total = handle
                .entries()
                .iter()
                .filter(|e| e.kind != scmd_core::EntryKind::Dir)
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
                Err(scmd_arc::ArcError::Cancelled) => Outcome {
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
        kind: scmd_jobs::JobKind::Test,
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
    changes: Vec<scmd_arc::Change>,
    level: scmd_arc::Level,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    archives: &std::sync::Arc<scmd_arc::Cache>,
) {
    use scmd_jobs::{Outcome, Phase, Progress};

    let started = std::time::Instant::now();
    archives.forget(archive.as_path());
    let total = changes.len() as u64;
    let mut done = 0u64;
    let mut last = std::time::Instant::now();
    let result = scmd_arc::rebuild(
        archive.as_path(),
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
    archives.forget(archive.as_path());
    let outcome = match result {
        Ok(written) => Outcome {
            files: written.files,
            dirs: written.dirs,
            bytes: written.bytes,
            ..Outcome::default()
        },
        Err(scmd_arc::ArcError::Cancelled) => Outcome {
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
        kind: scmd_jobs::JobKind::Rebuild,
        outcome,
    });
}

/// Copy one entry out of an archive into the temporary directory, for the viewer and the
/// editor — which both read a real file (SPEC 04 §5, the same shape as a remote fetch).
fn fetch_entry(
    archives: &std::sync::Arc<scmd_arc::Cache>,
    path: &VPath,
) -> Result<(PathBuf, RemoteStamp), String> {
    let host = path
        .leave()
        .ok_or_else(|| format!("{path}: not a path inside an archive"))?;
    let archive = archives
        .open(host.as_path(), &CancelToken::new())
        .map_err(|e| e.to_string())?;
    let inner = path.inner_path();
    let entry = archive
        .entry(inner)
        .ok_or_else(|| format!("{path}: no such entry"))?;
    let dir = std::env::temp_dir().join(format!("scmd-{}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let local = dir.join(path.display_name());
    archive
        .extract(inner, &local, &CancelToken::new(), &mut |_| {})
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

/// What the file looks like on the server right now.
async fn stamp_of(remote: &scmd_net::Remote, path: &VPath) -> Option<RemoteStamp> {
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
    change: &scmd_jobs::Change,
    recursive: bool,
    cancel: CancelToken,
    tx: mpsc::Sender<Event>,
    fs: &dyn scmd_jobs::JobFs,
) {
    use scmd_jobs::{Phase, Progress};

    let started = std::time::Instant::now();
    let total = paths.len() as u64;
    let mut last = std::time::Instant::now();
    let outcome =
        scmd_jobs::set_attributes(fs, paths, change, recursive, &cancel, &mut |path, done| {
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
        kind: scmd_jobs::JobKind::Attrs,
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
    objects: &scmd_s3::vfs::S3Vfs,
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
fn stamp_of_entry(entry: &scmd_core::Entry) -> RemoteStamp {
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
/// each block says *where* its keys come from, and `scmd-s3` reads them from the environment
/// or from `~/.aws/credentials` like every other tool on the machine.
fn object_storage(cfg: &scmd_config::Config) -> scmd_s3::vfs::S3Vfs {
    use scmd_s3::creds;
    use scmd_s3::endpoint::Endpoint;
    use scmd_s3::vfs::Service;

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
    scmd_s3::vfs::S3Vfs::new(services)
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
scmd = ">={}"

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
            "#!/bin/sh\n             cat >/dev/null\n             printf '{\"status\":\"here: %s\"}\\n' \"$SCMD_CWD\"\n             echo '{\"select\":[\"one.txt\",\"two.txt\"]}'\n             echo '{\"cursor\":\"two.txt\"}'\n",
        )
        .unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();

        // Loaded the way the binary loads it.
        let paths = Paths::discover(Some(&left.join("scmd.toml")), Some(&plugins));
        let mut warnings = Vec::new();
        let (_, _, commands) = load_plugins(&paths, ThemeSet::builtin_only(), &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(commands.len(), 1);
        assert_eq!(commands[0].id, "user.demo.look");
        assert_eq!(commands[0].scope, ScmdScope::Panel);
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
                    "[plugin]\nid = \"bad\"\nname = \"Bad\"\nscmd = \">={}\"\n{body}",
                    env!("CARGO_PKG_VERSION")
                ),
            )
            .unwrap();
            let paths = Paths::discover(Some(&root.path().join("scmd.toml")), Some(&plugins));
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
        let paths = Paths::discover(Some(&root.join("scmd.toml")), Some(&root));
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
        use scmd_s3::fake::FakeS3;
        use scmd_s3::vfs::{S3Vfs, Service};
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
            use scmd_jobs::{JobFs, JobKind, JobSpec};

            struct Quiet;
            impl scmd_jobs::Reporter for Quiet {
                fn progress(&mut self, _p: &scmd_jobs::Progress) {}
                fn ask(&mut self, _q: scmd_jobs::Question) -> scmd_jobs::Decision {
                    scmd_jobs::Decision::Overwrite
                }
            }

            /// Plan it and run it, as the worker thread does.
            fn plan_and_run(
                fs: &dyn scmd_jobs::JobFs,
                spec: &JobSpec,
                reporter: &mut dyn scmd_jobs::Reporter,
                cancel: &CancelToken,
            ) -> scmd_jobs::Outcome {
                let plan = scmd_jobs::plan(fs, spec, cancel).expect("the job can be planned");
                scmd_jobs::execute(fs, spec, plan, reporter, cancel)
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
        use scmd_s3::fake::FakeS3;
        use scmd_s3::vfs::{S3Vfs, Service};
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
        use scmd_s3::fake::FakeS3;
        use scmd_s3::vfs::{S3Vfs, Service};
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
        let plugin = scmd_plug::Plugin::load(&root).expect("it loads");
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
        App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&left.join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(left),
            right: VPath::local(right),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
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
                } => {
                    let cancel = CancelToken::new();
                    let result = if path.in_container() {
                        scmd_vfs::ArcVfs::new(scmd_arc::Limits::default())
                            .list_blocking(&path, &cancel)
                    } else {
                        scmd_vfs::local::list_blocking(&path, &cancel)
                    };
                    app.handle_event(Event::Listed {
                        panel,
                        generation,
                        result: result.map_err(|e| e.to_string()),
                    });
                }
                Request::Job { id, spec, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    // The archive layer, as the program has it: a job whose source is inside
                    // a zip has to read it out rather than copy the zip.
                    let fs = ArcFs::new(
                        std::sync::Arc::new(scmd_arc::Cache::new(Default::default())),
                        scmd_jobs::LocalFs,
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
                        scmd_plug::host::run_command(&program, cwd.as_deref(), &ctx, timeout)
                            .map_err(|e| e.to_string());
                    app.handle_event(Event::PluginDone { id, label, result });
                }
                Request::Link { target, link, kind } => {
                    let error = scmd_jobs::create_link(&scmd_jobs::LocalFs, &target, &link, kind)
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
                        &scmd_jobs::LocalFs,
                    );
                    for event in rx {
                        app.handle_event(event);
                    }
                }
                Request::TunnelOpen { .. } | Request::TunnelClose { .. } | Request::TunnelList => {}
                Request::CompareContents { id, pairs, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    run_compare(id, pairs, cancel, tx);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::Find { id, spec, cancel } => {
                    let (tx, rx) = mpsc::channel();
                    run_find(id, *spec, cancel, tx);
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
                            scmd_vfs::local::stat_blocking(&path)
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
                    let archives = std::sync::Arc::new(scmd_arc::Cache::new(Default::default()));
                    let result = fetch_entry(&archives, &path);
                    app.handle_event(Event::Fetched {
                        id,
                        remote: path,
                        purpose,
                        result,
                    });
                }
                Request::TreeRead { panel, path } => {
                    let dirs = scmd_vfs::local::list_blocking(&path, &CancelToken::new())
                        .map(|entries| {
                            entries
                                .into_iter()
                                .filter(|e| {
                                    e.kind == scmd_core::EntryKind::Dir
                                        || (e.kind == scmd_core::EntryKind::Symlink
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
                    sources,
                    cancel,
                } => {
                    let (tx, rx) = mpsc::channel();
                    let archives = std::sync::Arc::new(scmd_arc::Cache::new(Default::default()));
                    run_pack(id, archive, format, level, sources, cancel, tx, &archives);
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
                    let archives = std::sync::Arc::new(scmd_arc::Cache::new(Default::default()));
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
                    let archives = std::sync::Arc::new(scmd_arc::Cache::new(Default::default()));
                    run_rebuild(id, archive, changes, level, cancel, tx, &archives);
                    while let Ok(event) = rx.try_recv() {
                        app.handle_event(event);
                    }
                }
                Request::DirSize { .. }
                | Request::Shell { .. }
                | Request::Decompile { .. }
                | Request::Connect { .. }
                | Request::Disconnect { .. }
                | Request::Upload { .. } => {}
            }
        }
    }

    /// A key, by its name in the keymap.
    fn key(app: &mut App, k: &str) {
        app.handle_input(Input::Key(k.parse().unwrap()));
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
        let committed = include_str!("../../../docs/scmd.1");
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
            "docs/scmd.1 is out of date — regenerate it with:\n\
             \n    cargo run -p scmd -- --man > docs/scmd.1\n"
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
            "scmd",
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
                ScCommand::parse(&id).is_some(),
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
                scmd_jobs::JobKind::Copy,
                vec![VPath::local(dir.path().join("src"))],
                Some(VPath::local(dir.path().join("dst"))),
            ),
            CancelToken::new(),
            tx,
            &scmd_jobs::LocalFs,
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
                    assert_eq!((id, kind), (7, scmd_jobs::JobKind::Copy));
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
        app.run(ScCommand::ToolFind);
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
            Some(scmd_vfs::local::real_path(&VPath::local(&here)))
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
            scmd_arc::Archive::open(&archive, &scmd_arc::Limits::default(), &CancelToken::new())
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
        let real = scmd_vfs::local::real_path(&VPath::local(&here));
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
        app.run(ScCommand::ToolFind);
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
                scmd_jobs::JobKind::Delete,
                vec![VPath::local(dir.path().join("ghost"))],
                None,
            ),
            CancelToken::new(),
            tx,
            &scmd_jobs::LocalFs,
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
            &scmd_jobs::LocalFs,
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
            dir_size(&scmd_jobs::LocalFs, &VPath::local(dir.path()), &token, 0),
            (0, 0)
        );
    }

    #[test]
    fn keymap_comes_from_the_config_block_or_falls_back_to_a_builtin() {
        let mut cfg = Config::default();
        assert!(load_keymap(&cfg, "scmd.toml", &[]).unwrap().len() > 30);

        cfg.ui.keymap = "klingon".into();
        assert!(load_keymap(&cfg, "scmd.toml", &[]).is_err());

        let (with_block, _) = Config::parse(
            "scmd.toml",
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
        let km = load_keymap(&with_block, "scmd.toml", &[]).unwrap();
        assert_eq!(km.name(), "klingon");
    }

    #[test]
    fn paths_describe_one_file_and_one_optional_directory() {
        // A native directory, so every separator in the output is this platform's.
        let base = std::env::temp_dir().join("sc-x");
        let paths = Paths::discover(Some(&base.join("scmd.toml")), Some(&base.join("plugins")));
        let text = paths.describe();
        for expected in ["scmd.toml", "scmd-state.toml", "plugins"] {
            let path = base.join(expected);
            assert!(text.contains(&path.display().to_string()), "{text}");
        }
    }
}
