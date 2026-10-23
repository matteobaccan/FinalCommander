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
use scmd_cmd::{Command as ScCommand, Keymap};
use scmd_config::{Config, Paths};
use scmd_core::{CancelToken, VPath};
use scmd_i18n::I18n;
use scmd_jobs::JobSpec;
use scmd_tui::{Input, Term};
use scmd_ui::{Buffer, Theme, ThemeSet};
use scmd_vfs::{LocalVfs, Vfs};

use crate::app::{App, Event, Request, Startup};

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
    let keymap = load_keymap(&cfg, &origin)?;
    let themes = ThemeSet::builtin_only().with_tables(&origin, &cfg.themes);
    warnings.extend(themes.load_errors.clone());

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
    use scmd_core::{SortKey, ViewMode};
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
                Request::Job { id, spec, cancel } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || run_job(id, spec, cancel, tx));
                }
                Request::Decompile { id, rule, path } => {
                    let tx = tx.clone();
                    runtime.spawn_blocking(move || {
                        let tool = rule.name.clone();
                        let result = run_decompiler(&rule, &path);
                        let _ = tx.send(Event::Decompiled { id, tool, result });
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

/// Run one job to completion on a worker thread, reporting through the event channel.
///
/// `ask` blocks this thread until the UI answers, which is exactly what the engine wants: the
/// UI thread keeps drawing while the job waits.
fn run_job(id: u64, spec: JobSpec, cancel: CancelToken, tx: mpsc::Sender<Event>) {
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

    let outcome = match scmd_jobs::plan(&scmd_jobs::LocalFs, &spec, &cancel) {
        Ok(plan) => scmd_jobs::execute(&scmd_jobs::LocalFs, &spec, plan, &mut reporter, &cancel),
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
fn dir_size(path: &VPath, cancel: &CancelToken, depth: u32) -> (u64, u64) {
    if depth > 64 || cancel.is_cancelled() {
        return (0, 0);
    }
    let Ok(entries) = scmd_vfs::local::list_blocking(path, cancel) else {
        return (0, 0);
    };
    let mut bytes = 0u64;
    let mut files = 0u64;
    for e in entries {
        match e.kind {
            scmd_core::EntryKind::Parent | scmd_core::EntryKind::Symlink => {}
            scmd_core::EntryKind::Dir => {
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

        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&root.path().join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(&left),
            right: VPath::local(&right),
        });

        // Serve the initial panel reads from the real filesystem.
        let pump = |app: &mut App| {
            for request in app.take_requests() {
                match request {
                    Request::Read {
                        panel,
                        path,
                        generation,
                    } => {
                        let result = scmd_vfs::local::list_blocking(&path, &CancelToken::new())
                            .map_err(|e| e.to_string());
                        app.handle_event(Event::Listed {
                            panel,
                            generation,
                            result,
                        });
                    }
                    Request::Job { id, spec, cancel } => {
                        let (tx, rx) = mpsc::channel();
                        run_job(id, spec, cancel, tx);
                        while let Ok(event) = rx.try_recv() {
                            app.handle_event(event);
                        }
                    }
                    Request::DirSize { .. } | Request::Decompile { .. } => {}
                }
            }
        };
        pump(&mut app);

        // Cursor onto report.txt, then F5 and Enter to accept the prefilled destination.
        let key = |app: &mut App, k: &str| app.handle_input(Input::Key(k.parse().unwrap()));
        key(&mut app, "Down");
        assert_eq!(
            app.panel().current().unwrap().name_str(),
            "report.txt",
            "the cursor is not where the test thinks"
        );
        key(&mut app, "F5");
        key(&mut app, "Enter");
        pump(&mut app);
        pump(&mut app); // the refresh reads queued by the finished job

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

        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&root.path().join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(&dir),
            right: VPath::local(root.path()),
        });
        let pump = |app: &mut App| {
            for request in app.take_requests() {
                match request {
                    Request::Read {
                        panel,
                        path,
                        generation,
                    } => {
                        let result = scmd_vfs::local::list_blocking(&path, &CancelToken::new())
                            .map_err(|e| e.to_string());
                        app.handle_event(Event::Listed {
                            panel,
                            generation,
                            result,
                        });
                    }
                    Request::Job { id, spec, cancel } => {
                        let (tx, rx) = mpsc::channel();
                        run_job(id, spec, cancel, tx);
                        while let Ok(event) = rx.try_recv() {
                            app.handle_event(event);
                        }
                    }
                    Request::DirSize { .. } | Request::Decompile { .. } => {}
                }
            }
        };
        pump(&mut app);

        let key = |app: &mut App, k: &str| app.handle_input(Input::Key(k.parse().unwrap()));
        // Select the subdirectory and the file, then delete both.
        key(&mut app, "Down");
        key(&mut app, "Ins");
        key(&mut app, "Ins");
        assert_eq!(app.panel().selection.len(), 2);
        key(&mut app, "F8");
        key(&mut app, "Enter"); // confirm
        pump(&mut app);
        pump(&mut app);

        assert!(!dir.join("sub").exists(), "the directory survived");
        assert!(!dir.join("two.txt").exists(), "the file survived");
        assert_eq!(app.panel().listing.totals.files, 0);
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
        assert!(load_keymap(&cfg, "scmd.toml").unwrap().len() > 30);

        cfg.ui.keymap = "klingon".into();
        assert!(load_keymap(&cfg, "scmd.toml").is_err());

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
        let km = load_keymap(&with_block, "scmd.toml").unwrap();
        assert_eq!(km.name(), "klingon");
    }

    #[test]
    fn paths_describe_one_file_and_one_optional_directory() {
        let paths = Paths::discover(
            Some(std::path::Path::new("/tmp/sc-x/scmd.toml")),
            Some(std::path::Path::new("/tmp/sc-x/plugins")),
        );
        let text = paths.describe();
        assert!(text.contains("/tmp/sc-x/scmd.toml"), "{text}");
        assert!(text.contains("/tmp/sc-x/scmd-state.toml"), "{text}");
        assert!(text.contains("/tmp/sc-x/plugins"), "{text}");
    }
}
