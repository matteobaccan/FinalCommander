//! End-to-end checks of the promise in SPEC 00 §2 G6: one executable, nothing required on disk,
//! and at most a plugins folder.

use std::fs;
use std::process::Command;

fn fcmd() -> Command {
    Command::new(env!("CARGO_BIN_EXE_fcmd"))
}

fn run(args: &[&str]) -> (String, String, bool) {
    let out = fcmd().args(args).output().expect("fcmd runs");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
        out.status.success(),
    )
}

#[test]
fn runs_with_no_configuration_at_all() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("fcmd.toml");
    let (stdout, stderr, ok) = run(&["--config", missing.to_str().unwrap(), "--list-themes"]);
    assert!(ok, "{stderr}");
    for id in ["commander", "black", "white", "matrix", "tron", "blade"] {
        assert!(stdout.contains(id), "{id} missing from:\n{stdout}");
    }
    assert!(stdout.contains("built-in"), "{stdout}");
}

/// The name is drawn above what the program says on the command line — but into a terminal,
/// never into a pipe, which is what these tests read through. An output with a banner in it
/// is one somebody has to strip before using it.
#[test]
fn nothing_the_program_prints_into_a_pipe_has_a_banner_in_front_of_it() {
    for args in [
        vec!["--list-languages"],
        vec!["--list-commands"],
        vec!["--version"],
        vec!["--man"],
    ] {
        let (stdout, stderr, ok) = run(&args);
        assert!(ok, "{args:?}: {stderr}");
        assert!(
            !stdout.contains("|_|  \\___|"),
            "{args:?} printed the banner into a pipe:\n{stdout}"
        );
    }
}

#[test]
fn paths_reports_every_file_it_reads_and_the_one_directory() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("fcmd.toml");
    let (stdout, stderr, ok) = run(&["--config", cfg.to_str().unwrap(), "--paths"]);
    assert!(ok, "{stderr}");
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 5, "{stdout}");
    assert!(lines[0].contains("fcmd.toml"), "{stdout}");
    assert!(lines[1].contains("fcmd-state.toml"), "{stdout}");
    assert!(lines[2].contains("fcmd-macros.toml"), "{stdout}");
    assert!(lines[3].contains("fcmd-menu.toml"), "{stdout}");
    assert!(lines[4].starts_with("plugins"), "{stdout}");
    // Each line says whether what it names is there: the point of the option is to answer
    // "where does it look", including for the files nobody has made yet.
    assert!(lines[2].contains("not present"), "{stdout}");
    assert!(lines[3].contains("not present"), "{stdout}");
}

#[test]
fn a_theme_declared_in_the_config_file_overrides_a_builtin() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("fcmd.toml");
    fs::write(
        &cfg,
        r##"
[ui]
theme = "commander"

[[theme]]
id = "commander"
name = "Mine"
inherit = "commander"
[theme.colors]
"selected.fg" = "#ff00ff"
"##,
    )
    .unwrap();
    let (stdout, stderr, ok) = run(&["--config", cfg.to_str().unwrap(), "--list-themes"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("Mine"), "{stdout}");
    assert!(stdout.contains("fcmd.toml"), "{stdout}");
    // Still seven: the block replaced the built-in rather than adding an eighth.
    assert_eq!(stdout.lines().count(), 7, "{stdout}");
}

#[test]
fn a_keymap_declared_in_the_config_file_is_used() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("fcmd.toml");
    fs::write(
        &cfg,
        r#"
[ui]
keymap = "mine"

[[keymap]]
name = "mine"
inherit = "far"
[[keymap.bind]]
keys = ["F4"]
cmd = "file.view"
"#,
    )
    .unwrap();
    let (stdout, stderr, ok) = run(&["--config", cfg.to_str().unwrap(), "--check-keymap"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("keymap `mine`"), "{stdout}");
    assert!(
        stdout
            .lines()
            .any(|l| l.contains("F4") && l.contains("file.view")),
        "{stdout}"
    );
}

#[test]
fn a_broken_config_file_fails_loudly_and_names_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let cfg = dir.path().join("fcmd.toml");
    fs::write(&cfg, "[ui\ntheme =").unwrap();
    let (_, stderr, ok) = run(&["--config", cfg.to_str().unwrap(), "--list-themes"]);
    assert!(!ok, "a broken config must not be ignored");
    assert!(stderr.contains("fcmd.toml"), "{stderr}");
}

#[test]
fn an_unknown_setting_is_rejected_with_a_usable_message() {
    let (_, stderr, ok) = run(&["--set", "ui.colour_of_magic=octarine", "--paths"]);
    assert!(!ok);
    assert!(stderr.contains("unknown setting"), "{stderr}");
}

#[test]
fn the_plugins_directory_is_the_only_directory_read() {
    let dir = tempfile::tempdir().unwrap();
    let plugins = dir.path().join("plugins");
    fs::create_dir(&plugins).unwrap();
    fs::write(plugins.join("netbox.wasm"), b"\0asm").unwrap();
    fs::write(plugins.join("hello.lua"), b"-- fcmd.plugin").unwrap();
    let (stdout, stderr, ok) = run(&["--plugins", plugins.to_str().unwrap(), "--list-plugins"]);
    assert!(ok, "{stderr}");
    assert!(
        stdout.contains("netbox.wasm") && stdout.contains("hello.lua"),
        "{stdout}"
    );
    assert!(stdout.contains("no plugin host"), "{stdout}");

    // A declarative plugin — a folder with a manifest — does load, and a broken one is
    // reported rather than skipped in silence.
    let good = plugins.join("sunset");
    fs::create_dir(&good).unwrap();
    fs::write(
        good.join("plugin.toml"),
        br#"
[plugin]
id = "sunset"
name = "Sunset"
version = "1.0.0"
[[theme]]
id = "sunset"
name = "Sunset"
inherit = "far"
"#,
    )
    .unwrap();
    let bad = plugins.join("wrongname");
    fs::create_dir(&bad).unwrap();
    fs::write(bad.join("plugin.toml"), b"[plugin]\nid = \"other\"\n").unwrap();

    let (stdout, stderr, ok) = run(&["--plugins", plugins.to_str().unwrap(), "--list-plugins"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("sunset"), "{stdout}");
    assert!(stdout.contains("wrongname"), "{stdout}");

    // And the theme it declares is in the theme list, saying where it came from.
    let (stdout, stderr, ok) = run(&["--plugins", plugins.to_str().unwrap(), "--list-themes"]);
    assert!(ok, "{stderr}");
    assert!(stdout.contains("plugin:sunset"), "{stdout}");
}

#[test]
fn every_informational_mode_works_without_a_terminal() {
    for args in [
        vec!["--list-languages"],
        vec!["--list-commands"],
        vec!["--dump-config"],
        vec!["--check-keymap"],
        vec!["--keymap", "mc", "--check-keymap"],
        vec!["--export-theme", "blade"],
        vec!["--version"],
        vec!["--help"],
    ] {
        let (stdout, stderr, ok) = run(&args);
        assert!(ok, "{args:?} failed: {stderr}");
        assert!(!stdout.is_empty(), "{args:?} printed nothing");
    }
}
