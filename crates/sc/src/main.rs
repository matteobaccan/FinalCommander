//! SupremeCommander — an orthodox file manager for the terminal.
//!
//! The interface is strictly TUI; see `docs/spec/00-overview.md`.

mod app;
mod panel;
mod render;

use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::{Context, Result};
use clap::Parser;
use sc_cmd::{Command as ScCommand, Keymap};
use sc_config::{Config, Paths};
use sc_core::VPath;
use sc_i18n::I18n;
use sc_tui::{Input, Term};
use sc_ui::{Buffer, Theme, ThemeSet};
use sc_vfs::{CancelToken, LocalVfs, Vfs};

use crate::app::{App, Event, Request, Startup};

#[derive(Debug, Parser)]
#[command(
    name = "sc",
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

fn main() -> Result<()> {
    let cli = Cli::parse();
    match run(cli) {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("sc: {e:#}");
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
    let keymap = load_keymap(&cfg, &origin)?;
    let themes = ThemeSet::builtin_only().with_tables(&origin, &cfg.themes);
    warnings.extend(themes.load_errors.clone());

    let theme = match themes.get(&cfg.ui.theme) {
        Some(t) => t.clone(),
        None => {
            warnings.push(format!(
                "unknown theme `{}`, using `{}`",
                cfg.ui.theme,
                sc_ui::DEFAULT_THEME
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
                sc_ui::ThemeSource::BuiltIn => "built-in".to_string(),
                sc_ui::ThemeSource::Config => origin.clone(),
                sc_ui::ThemeSource::Plugin(p) => format!("plugin:{p}"),
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
        for (tag, name) in sc_i18n::LANGUAGES {
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
            .filter(|t| t.source == sc_ui::ThemeSource::Config)
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

    let mut app = App::new(Startup {
        cfg,
        paths,
        i18n,
        keymap,
        themes,
        theme,
        left,
        right,
    });
    if !warnings.is_empty() {
        // Startup problems are shown once, as a dialog: they are easy to miss in a status line.
        app.show_message("Startup warnings", warnings.clone());
    }
    interactive(app)
}

/// A `[[keymap]]` block from the config file wins over a built-in profile of the same name.
fn load_keymap(cfg: &Config, origin: &str) -> Result<Keymap> {
    let wanted = cfg.ui.keymap.trim();
    for table in &cfg.keymaps {
        let name = table
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        if name == wanted {
            return Keymap::from_table(origin, table).map_err(anyhow::Error::new);
        }
    }
    Keymap::builtin(wanted).map_err(anyhow::Error::new)
}

/// Everything in the plugins directory, with a note that the host is not here yet.
fn list_plugins(paths: &Paths) {
    if !paths.plugins_dir.is_dir() {
        println!("no plugins directory at {}", paths.plugins_dir.display());
        return;
    }
    let mut found = Vec::new();
    if let Ok(read) = std::fs::read_dir(&paths.plugins_dir) {
        for entry in read.flatten() {
            found.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    found.sort();
    if found.is_empty() {
        println!("{}: empty", paths.plugins_dir.display());
        return;
    }
    for name in found {
        println!("{name}");
    }
    println!("(no plugin host in this build yet: viewer, editor and netbox come first — see docs/spec/08-plugins.md)");
}

fn command_ids() -> Vec<String> {
    use sc_core::{SortKey, ViewMode};
    let mut ids: Vec<String> = [
        ScCommand::AppQuit,
        ScCommand::AppHelp,
        ScCommand::AppMenu,
        ScCommand::AppPalette,
        ScCommand::AppThemeNext,
        ScCommand::PanelFocusToggle,
        ScCommand::PanelSwap,
        ScCommand::PanelReread,
        ScCommand::PanelHiddenToggle,
        ScCommand::PanelSortReverse,
        ScCommand::PanelFilterQuick,
        ScCommand::PanelOtherSync,
        ScCommand::NavUp,
        ScCommand::NavDown,
        ScCommand::NavPageUp,
        ScCommand::NavPageDown,
        ScCommand::NavTop,
        ScCommand::NavBottom,
        ScCommand::NavEnter,
        ScCommand::NavParent,
        ScCommand::NavRoot,
        ScCommand::NavBack,
        ScCommand::NavForward,
        ScCommand::NavGotoPath,
        ScCommand::SelToggle,
        ScCommand::SelAll,
        ScCommand::SelNone,
        ScCommand::SelInvert,
        ScCommand::SelMaskAdd,
        ScCommand::SelMaskRemove,
        ScCommand::SelRestore,
        ScCommand::FileView,
        ScCommand::FileEdit,
        ScCommand::FileCopy,
        ScCommand::FileMove,
        ScCommand::FileMkdir,
        ScCommand::FileDelete,
        ScCommand::FileDirSize,
        ScCommand::ToolUserMenu,
        ScCommand::ToolFind,
        ScCommand::ShellOutputShow,
    ]
    .iter()
    .map(|c| c.id())
    .collect();
    for v in [
        ViewMode::Brief,
        ViewMode::Full,
        ViewMode::Detail,
        ViewMode::Wide,
    ] {
        ids.push(ScCommand::PanelViewSet(v).id());
    }
    for k in [
        SortKey::Name,
        SortKey::Ext,
        SortKey::Size,
        SortKey::MTime,
        SortKey::Unsorted,
    ] {
        ids.push(ScCommand::PanelSortBy(k).id());
    }
    ids.sort();
    ids
}

fn interactive(mut app: App) -> Result<()> {
    sc_tui::install_panic_hook();
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
        std::thread::spawn(move || sc_tui::read_input_loop(raw_tx));
        while let Ok(input) = raw_rx.recv() {
            if input_tx.send(Event::Input(input)).is_err() {
                return;
            }
        }
    });

    let vfs = LocalVfs;
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
                    runtime.spawn(async move {
                        let result = vfs.list(&path, token).await.map_err(|e| e.to_string());
                        let _ = tx.send(Event::Listed {
                            panel,
                            generation,
                            result,
                        });
                    });
                }
                Request::DirSize { panel, path } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        let (bytes, files) = dir_size(&path, &CancelToken::new(), 0);
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
    Ok(())
}

/// Recursive size of a directory. Depth-limited, cancellable, and it never follows symlinks.
fn dir_size(path: &VPath, cancel: &CancelToken, depth: u32) -> (u64, u64) {
    if depth > 64 || cancel.is_cancelled() {
        return (0, 0);
    }
    let Ok(entries) = sc_vfs::local::list_blocking(path, cancel) else {
        return (0, 0);
    };
    let mut bytes = 0u64;
    let mut files = 0u64;
    for e in entries {
        match e.kind {
            sc_core::EntryKind::Parent | sc_core::EntryKind::Symlink => {}
            sc_core::EntryKind::Dir => {
                let (b, f) = dir_size(&path.join(&e.name), cancel, depth + 1);
                bytes += b;
                files += f;
            }
            _ => {
                bytes += e.size;
                files += 1;
            }
        }
    }
    (bytes, files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn cli_definition_is_valid() {
        Cli::command().debug_assert();
    }

    #[test]
    fn cli_parses_paths_and_overrides() {
        let cli = Cli::parse_from([
            "sc",
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
    fn dir_size_walks_a_real_tree() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), vec![0u8; 10]).unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("sub/b"), vec![0u8; 32]).unwrap();
        let (bytes, files) = dir_size(&VPath::local(dir.path()), &CancelToken::new(), 0);
        assert_eq!((bytes, files), (42, 2));
    }

    #[test]
    fn dir_size_stops_when_cancelled() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), vec![0u8; 10]).unwrap();
        let token = CancelToken::new();
        token.cancel();
        assert_eq!(dir_size(&VPath::local(dir.path()), &token, 0), (0, 0));
    }

    #[test]
    fn keymap_comes_from_the_config_block_or_falls_back_to_a_builtin() {
        let mut cfg = Config::default();
        assert!(load_keymap(&cfg, "sc.toml").unwrap().len() > 30);

        cfg.ui.keymap = "klingon".into();
        assert!(load_keymap(&cfg, "sc.toml").is_err());

        let (with_block, _) = Config::parse(
            "sc.toml",
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
        let km = load_keymap(&with_block, "sc.toml").unwrap();
        assert_eq!(km.name(), "klingon");
    }

    #[test]
    fn paths_describe_one_file_and_one_optional_directory() {
        let paths = Paths::discover(
            Some(std::path::Path::new("/tmp/sc-x/sc.toml")),
            Some(std::path::Path::new("/tmp/sc-x/plugins")),
        );
        let text = paths.describe();
        assert!(text.contains("/tmp/sc-x/sc.toml"), "{text}");
        assert!(text.contains("/tmp/sc-x/sc-state.toml"), "{text}");
        assert!(text.contains("/tmp/sc-x/plugins"), "{text}");
    }
}
