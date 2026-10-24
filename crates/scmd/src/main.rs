//! SupremeCommander — an orthodox file manager for the terminal.
//!
//! The interface is strictly TUI; see `docs/spec/00-overview.md`.

mod app;
mod cmdline;
mod expand;
mod form;
mod menu;
mod panel;
mod render;
mod tree;

use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::Parser;
use scmd_cmd::{Command as ScCommand, Keymap};
use scmd_config::{Config, Paths};
use scmd_core::{CancelToken, VPath};
use scmd_i18n::I18n;
use scmd_jobs::{ArcFs, JobSpec};
use scmd_net::{Pool, RoutedFs, Router, Site};
use scmd_tui::{Input, Term};
use scmd_ui::{Buffer, Theme, ThemeSet};
use scmd_vfs::Vfs;

use crate::app::{App, ConnectOutcome, Event, RemoteStamp, Request, Startup, UploadError};

#[derive(Debug, Parser)]
#[command(
    name = "scmd",
    version,
    about = "SupremeCommander — orthodox file manager for the terminal",
    long_about = None
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
    let (themes, plugin_keymaps) = load_plugins(&paths, themes, &mut warnings);
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
    });
    app.cmd.load_history(state.command_history);
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
    // about it would leave them guessing why their plugin does nothing.
    let mut waiting: Vec<String> = Vec::new();
    if let Ok(read) = std::fs::read_dir(&paths.plugins_dir) {
        for entry in read.flatten() {
            if entry.path().is_file() {
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
) -> (ThemeSet, Vec<(String, toml::Table)>) {
    let found = scmd_plug::discover(&paths.plugins_dir);
    warnings.extend(found.problems.iter().cloned());
    warnings.extend(found.warnings.iter().cloned());
    let mut themes = themes;
    let mut keymaps = Vec::new();
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
    }
    (themes, keymaps)
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

    // One pool of connections, shared by the panels, the viewer and every job.
    let pool = Pool::new();
    if !app.cfg.net.known_hosts.trim().is_empty() {
        pool.set_policy(scmd_net::HostKeyPolicy {
            known_hosts: Some(scmd_net::site::expand_tilde(app.cfg.net.known_hosts.trim())),
            accept_new: false,
        });
    }
    let vfs = Router::with_limits(pool.clone(), app.cfg.arc.limits());
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
                        let result = vfs.list(&path, token).await.map_err(|e| e.to_string());
                        let _ = tx.send(Event::Listed {
                            panel,
                            generation,
                            result,
                        });
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
                        RoutedFs::new(pool.clone(), runtime.handle().clone()),
                    );
                    runtime.spawn_blocking(move || run_job(id, spec, cancel, tx, &fs));
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
                    runtime.spawn(async move {
                        let result = if path.in_container() {
                            let inner = path.clone();
                            tokio::task::spawn_blocking(move || fetch_entry(&archives, &inner))
                                .await
                                .unwrap_or_else(|e| Err(format!("the unpack failed: {e}")))
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
                    runtime.spawn(async move {
                        let result = write_back(&pool, &local, &remote, expect, force).await;
                        let _ = tx.send(Event::Uploaded { id, remote, result });
                    });
                }
                Request::Find { id, spec, cancel } => {
                    let tx = tx.clone();
                    // A blocking thread: the walk is synchronous, and it must not sit in the
                    // way of the runtime that is also serving the panels.
                    runtime.spawn_blocking(move || run_find(id, *spec, cancel, tx));
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
                        // that does is right beside it.
                        let dirs = match vfs.list(&path, CancelToken::new()).await {
                            Ok(entries) => entries
                                .into_iter()
                                .filter(|e| e.kind == scmd_core::EntryKind::Dir)
                                .map(|e| path.join(&e.name))
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
                Request::Link { target, link, kind } => {
                    let tx = tx.clone();
                    let fs = RoutedFs::new(pool.clone(), runtime.handle().clone());
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
                        RoutedFs::new(pool.clone(), runtime.handle().clone()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

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
                Request::Link { target, link, kind } => {
                    let error = scmd_jobs::create_link(&scmd_jobs::LocalFs, &target, &link, kind)
                        .err()
                        .map(|e| e.to_string());
                    app.handle_event(Event::LinkDone { link, error });
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
                                .filter(|e| e.kind == scmd_core::EntryKind::Dir)
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
        assert_eq!(
            tree.path(),
            Some(VPath::local(&here)),
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
