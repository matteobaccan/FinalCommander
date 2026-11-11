//! What a plugin command is given, and what it may ask for back (SPEC 08 §1).
//!
//! A plugin command is a **program**, not code in our address space. That is the whole of the
//! design and the reason it exists at this tier: one artifact works on every platform, there
//! is no sandbox to get wrong, nothing has to match our compiler version, and a plugin that
//! crashes crashes alone. The cost is a process per invocation, which for "check out a branch"
//! or "run the tests here" is nothing at all.
//!
//! The conversation is one JSON object in and a line of JSON per action out:
//!
//! ```text
//! host → plugin   {"api":1,"command":"user.git.branch","panel":{"cwd":"/src"}, ...}  on stdin
//! plugin → host   {"status":"on main"}                                        one per line
//!                 {"run":"panel.reread"}
//!                 {"chdir":"/src/docs"}
//! ```
//!
//! An action that names a command names one of *ours* — the vocabulary every key and macro
//! uses — so SPEC 08's first sentence holds literally: everything a plugin can do, a key can
//! do too. There is no second vocabulary to keep in step, and no way for a plugin to reach
//! past what a keystroke could have done.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The protocol version, in every context object. A plugin that does not recognise it should
/// say so and exit rather than guess.
pub const API_VERSION: u32 = 1;

/// What a panel looks like from outside.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelView {
    /// Where the panel is, as the program spells it — which for a file inside an archive is a
    /// container path (`/tmp/x.zip!/inner`), not something to hand to `open`.
    pub cwd: String,
    /// The real directory on disk, when there is one. A plugin that runs a program wants this
    /// and should say so when it is missing rather than inventing a path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}

/// Everything a command asked for, and nothing it did not.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    pub api: u32,
    /// Which command of this plugin is being run: one program may serve several.
    pub command: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub panel: Option<PanelView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other: Option<PanelView>,
    /// The name under the cursor, when `cursor` was asked for.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
    /// The selected names, when `selection` was asked for. Names rather than paths, because
    /// the plugin runs in the panel's directory.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub selection: Vec<String>,
}

impl Context {
    pub fn new(command: &str) -> Context {
        Context {
            api: API_VERSION,
            command: command.to_string(),
            ..Context::default()
        }
    }
}

/// One thing a plugin asks the application to do.
///
/// Deliberately small. Each variant is something a person could have done with the keyboard,
/// and nothing here reaches past that: there is no "write this file" action, because writing a
/// file is what the plugin's own process is for.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    /// A line for the status bar.
    Status(String),
    /// A line for the status bar, drawn as an error.
    Error(String),
    /// Run a command of the vocabulary, by id — `panel.reread`, `file.view`, anything
    /// `fcmd --list-commands` prints.
    Run(String),
    /// Take a panel to a directory. `panel` is `active`, `other`, `left` or `right`.
    Chdir {
        path: String,
        #[serde(default)]
        panel: Option<String>,
    },
    /// Show a message, which is how a plugin reports something worth reading.
    Message { title: String, lines: Vec<String> },
    /// Put the cursor on a name in the active panel.
    Cursor(String),
    /// Select these names in the active panel, replacing what was selected.
    Select(Vec<String>),
}

/// What the host made of a plugin's output.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
    pub actions: Vec<Action>,
    /// Lines of output that were not actions. A plugin writing to standard output for a human
    /// is not an error — it is how a half-written plugin behaves — so they are kept and shown
    /// rather than dropped.
    pub text: Vec<String>,
    /// Lines the plugin wrote to standard error, which is where it is supposed to complain.
    pub complaints: Vec<String>,
}

/// Read a plugin's standard output: one action per line, anything else kept as text.
pub fn parse_output(stdout: &str, stderr: &str) -> Reply {
    let mut reply = Reply::default();
    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match serde_json::from_str::<Action>(line) {
            Ok(action) => reply.actions.push(action),
            // A line that is JSON but not an action is a mistake worth seeing, and a line that
            // is not JSON at all is prose. Both end up in front of the user either way.
            Err(_) => reply.text.push(line.to_string()),
        }
    }
    reply.complaints = stderr
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    reply
}

/// The environment a plugin is run with, beside the context on standard input.
///
/// The same facts, for a plugin that would rather read a variable than parse JSON — a shell
/// script, which is what most of these will be.
pub fn environment(ctx: &Context) -> BTreeMap<String, String> {
    let mut env = BTreeMap::new();
    env.insert("FCMD_API".to_string(), ctx.api.to_string());
    env.insert("FCMD_COMMAND".to_string(), ctx.command.clone());
    if let Some(panel) = &ctx.panel {
        env.insert("FCMD_CWD".to_string(), panel.cwd.clone());
        if let Some(dir) = &panel.dir {
            env.insert("FCMD_DIR".to_string(), dir.clone());
        }
    }
    if let Some(other) = &ctx.other {
        env.insert("FCMD_OTHER".to_string(), other.cwd.clone());
        if let Some(dir) = &other.dir {
            env.insert("FCMD_OTHER_DIR".to_string(), dir.clone());
        }
    }
    if let Some(cursor) = &ctx.cursor {
        env.insert("FCMD_CURSOR".to_string(), cursor.clone());
    }
    if !ctx.selection.is_empty() {
        // One per line rather than space-separated: a file name may contain a space, and a
        // variable a script cannot split correctly is worse than no variable.
        env.insert("FCMD_SELECTED".to_string(), ctx.selection.join("\n"));
        env.insert(
            "FCMD_SELECTED_COUNT".to_string(),
            ctx.selection.len().to_string(),
        );
    }
    env
}

/// Why a plugin command did not run, or did not finish.
#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("{program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{program} did not finish within {seconds} seconds and was stopped")]
    Timeout { program: String, seconds: u64 },
    #[error("{program} failed: exit status {status}")]
    Status { program: String, status: String },
}

/// How long a plugin command may take when it does not say.
pub const DEFAULT_TIMEOUT: u64 = 20;

/// Run a plugin command: the context on standard input, the actions on standard output.
///
/// It blocks, with a timeout. A plugin that wants to take a minute says so in its manifest; a
/// plugin that hangs is killed and the reason said out loud, because a file manager that stops
/// answering the keyboard because somebody's shell script is waiting for input is worse than
/// no plugin at all.
pub fn run_command(
    program: &std::path::Path,
    cwd: Option<&std::path::Path>,
    ctx: &Context,
    timeout: Option<u64>,
) -> Result<Reply, RunError> {
    use std::io::Write;
    use std::process::{Command as Proc, Stdio};

    let name = program.display().to_string();
    let seconds = timeout.unwrap_or(DEFAULT_TIMEOUT).clamp(1, 600);
    let mut proc = Proc::new(program);
    proc.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some(cwd) = cwd {
        proc.current_dir(cwd);
    }
    for (key, value) in environment(ctx) {
        proc.env(key, value);
    }
    let mut child = proc.spawn().map_err(|source| RunError::Spawn {
        program: name.clone(),
        source,
    })?;

    // The context goes in and the pipe is closed, so a plugin reading to end-of-file is not
    // left waiting for one.
    if let Some(mut stdin) = child.stdin.take() {
        let json = serde_json::to_string(ctx).unwrap_or_else(|_| "{}".to_string());
        let _ = stdin.write_all(json.as_bytes());
        let _ = stdin.write_all(b"\n");
    }

    // Waited for in slices rather than with a thread: the output is read after the process
    // has gone, which is enough for the small outputs this protocol is for.
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let out = child.wait_with_output().map_err(|source| RunError::Spawn {
                    program: name.clone(),
                    source,
                })?;
                let reply = parse_output(
                    &String::from_utf8_lossy(&out.stdout),
                    &String::from_utf8_lossy(&out.stderr),
                );
                if !status.success() {
                    // A failing exit status is reported, but whatever the plugin managed to
                    // say first is not thrown away with it.
                    if reply.actions.is_empty()
                        && reply.text.is_empty()
                        && reply.complaints.is_empty()
                    {
                        return Err(RunError::Status {
                            program: name,
                            status: status.to_string(),
                        });
                    }
                }
                return Ok(reply);
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(RunError::Timeout {
                        program: name,
                        seconds,
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(source) => {
                return Err(RunError::Spawn {
                    program: name,
                    source,
                })
            }
        }
    }
}

/// How long one run of a column's program may take when the column does not say. Shorter
/// than a command's: a command is asked for once, a column for every file of a directory.
pub const COLUMN_TIMEOUT: u64 = 10;

/// The longest value a column keeps. A cell is a few characters wide; this only stops a
/// program that printed a file's whole contents from being kept in memory row by row.
const COLUMN_VALUE_MAX: usize = 200;

/// What a program printed, as a cell: one line, nothing that would move the cursor.
fn column_cell(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(COLUMN_VALUE_MAX)
        .collect()
}

/// Run a program to its end with `input` on its standard input, and hand back what it
/// printed. Both pipes are worked from threads of their own — unlike [`run_command`], whose
/// outputs are small by the protocol — because a batch for a large directory writes more
/// paths and prints more lines than a pipe holds, and a program that is waiting for its
/// output to be read while we wait for it to finish is a timeout nobody could explain.
fn run_to_end(
    mut proc: std::process::Command,
    name: String,
    input: Vec<u8>,
    seconds: u64,
) -> Result<String, RunError> {
    use std::io::{Read, Write};
    use std::process::Stdio;

    let seconds = seconds.clamp(1, 600);
    proc.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    // A script that has only just been written can be refused as "busy" for an instant:
    // Linux will not execute a file some process still holds open for writing, and a
    // process forked at the wrong moment holds everything its parent had open until it
    // becomes its own program. A column asks hundreds of times in a row, so the instant is
    // waited out here rather than reported as a column that does not work.
    let mut tries = 0;
    let mut child = loop {
        match proc.spawn() {
            Ok(child) => break child,
            Err(source)
                if source.kind() == std::io::ErrorKind::ExecutableFileBusy && tries < 50 =>
            {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            Err(source) => {
                return Err(RunError::Spawn {
                    program: name,
                    source,
                })
            }
        }
    };
    let stdin = child.stdin.take();
    let writer = std::thread::spawn(move || {
        if let Some(mut stdin) = stdin {
            // A program that never reads its input closes the pipe by ending: not an error.
            let _ = stdin.write_all(&input);
        }
    });
    let stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        if let Some(mut stdout) = stdout {
            let _ = stdout.read_to_end(&mut out);
        }
        out
    });

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(seconds);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    // The two threads end with the pipes, which the kill has just closed —
                    // unless a grandchild still holds them, which is not worth waiting for.
                    return Err(RunError::Timeout {
                        program: name,
                        seconds,
                    });
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(source) => {
                return Err(RunError::Spawn {
                    program: name,
                    source,
                })
            }
        }
    };
    let _ = writer.join();
    let out = reader.join().unwrap_or_default();
    if !status.success() {
        return Err(RunError::Status {
            program: name,
            status: status.to_string(),
        });
    }
    Ok(String::from_utf8_lossy(&out).into_owned())
}

/// The environment a column's program runs in: which column, and where.
fn column_proc(
    program: &std::path::Path,
    dir: &std::path::Path,
    column: &str,
) -> std::process::Command {
    let mut proc = std::process::Command::new(program);
    proc.current_dir(dir)
        .env("FCMD_API", API_VERSION.to_string())
        .env("FCMD_COLUMN", column)
        .env("FCMD_DIR", dir);
    proc
}

/// Ask a column's program about one file (SPEC 08 §1d).
///
/// The path is the first argument — and `FCMD_FILE`, for a script that prefers the
/// environment — the directory it is in is where the program runs, and the first line it
/// prints is the value. Printing nothing is an answer too: an empty cell.
pub fn column_value(
    program: &std::path::Path,
    dir: &std::path::Path,
    file: &std::path::Path,
    column: &str,
    timeout: Option<u64>,
) -> Result<String, RunError> {
    let mut proc = column_proc(program, dir, column);
    proc.arg(file).env("FCMD_FILE", file);
    let out = run_to_end(
        proc,
        program.display().to_string(),
        Vec::new(),
        timeout.unwrap_or(COLUMN_TIMEOUT),
    )?;
    Ok(column_cell(out.lines().next().unwrap_or("")))
}

/// Ask a column's program about a whole directory in one run (`batch = true`).
///
/// The paths go in on standard input, one per line; the answers come back as
/// `path<TAB>value`, the path spelled as it was given, and are handed back in the order of
/// `files`. A file the program says nothing about has an empty cell; a line that names no
/// file that was asked about is dropped rather than guessed at.
pub fn column_batch(
    program: &std::path::Path,
    dir: &std::path::Path,
    files: &[std::path::PathBuf],
    column: &str,
    timeout: Option<u64>,
) -> Result<Vec<String>, RunError> {
    let asked: Vec<String> = files.iter().map(|f| f.display().to_string()).collect();
    let mut input = asked.join("\n").into_bytes();
    input.push(b'\n');
    let out = run_to_end(
        column_proc(program, dir, column),
        program.display().to_string(),
        input,
        timeout.unwrap_or(COLUMN_TIMEOUT),
    )?;
    let index: std::collections::HashMap<&str, usize> = asked
        .iter()
        .enumerate()
        .map(|(i, path)| (path.as_str(), i))
        .collect();
    let mut values = vec![String::new(); asked.len()];
    let mut seen = vec![false; asked.len()];
    for line in out.lines() {
        let Some((path, value)) = line.split_once('\t') else {
            continue;
        };
        if let Some(&i) = index.get(path) {
            if !seen[i] {
                seen[i] = true;
                values[i] = column_cell(value);
            }
        }
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_context_carries_only_what_was_asked_for() {
        let bare = Context::new("user.git.branch");
        let json = serde_json::to_string(&bare).unwrap();
        assert!(json.contains("\"api\":1"), "{json}");
        assert!(json.contains("user.git.branch"), "{json}");
        // Nothing asked for is nothing sent: no empty panel, no empty selection.
        assert!(!json.contains("panel"), "{json}");
        assert!(!json.contains("selection"), "{json}");

        let full = Context {
            panel: Some(PanelView {
                cwd: "/src".into(),
                dir: Some("/src".into()),
            }),
            selection: vec!["a.rs".into(), "b rs.txt".into()],
            cursor: Some("a.rs".into()),
            ..Context::new("user.x")
        };
        let json = serde_json::to_string(&full).unwrap();
        let back: Context = serde_json::from_str(&json).unwrap();
        assert_eq!(back, full);
    }

    #[test]
    fn the_environment_says_the_same_things_as_the_json() {
        let ctx = Context {
            panel: Some(PanelView {
                cwd: "/src".into(),
                dir: Some("/src".into()),
            }),
            other: Some(PanelView {
                cwd: "/tmp/x.zip!/inner".into(),
                dir: None,
            }),
            selection: vec!["one two.txt".into(), "three.txt".into()],
            cursor: Some("one two.txt".into()),
            ..Context::new("user.x")
        };
        let env = environment(&ctx);
        assert_eq!(env["FCMD_CWD"], "/src");
        assert_eq!(env["FCMD_DIR"], "/src");
        // A path inside an archive has no directory on disk, and the variable is simply absent
        // rather than holding something that cannot be opened.
        assert_eq!(env["FCMD_OTHER"], "/tmp/x.zip!/inner");
        assert!(!env.contains_key("FCMD_OTHER_DIR"));
        // The selection is one name per line: a name with a space in it survives.
        assert_eq!(env["FCMD_SELECTED"], "one two.txt\nthree.txt");
        assert_eq!(env["FCMD_SELECTED_COUNT"], "2");
    }

    /// Run a program a test has just written, trying again while the system says the file is
    /// busy.
    ///
    /// Not a workaround for a bug of ours: between a `fork` in another thread and its `exec`,
    /// that child holds every descriptor this process has open — including the one a test has
    /// just written its script through — and Linux refuses to execute a file another process
    /// holds open for writing (`ETXTBSY`). Nothing the host can do closes that window, and the
    /// tests here write and run programs in parallel, so the answer is to try again rather
    /// than to fail a build over a race between two tests.
    fn run_written(
        path: &std::path::Path,
        cwd: Option<&std::path::Path>,
        ctx: &Context,
        seconds: Option<u64>,
    ) -> Result<Reply, RunError> {
        for _ in 0..50 {
            match run_command(path, cwd, ctx, seconds) {
                Err(RunError::Spawn { source, .. })
                    if source.kind() == std::io::ErrorKind::ExecutableFileBusy =>
                {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                other => return other,
            }
        }
        run_command(path, cwd, ctx, seconds)
    }

    /// The whole conversation, through a real process. A shell script on Unix and a batch
    /// file on Windows: the protocol has to work with whatever a user actually writes.
    #[test]
    fn a_real_program_is_run_and_its_actions_come_back() {
        let dir = tempfile::tempdir().unwrap();
        let (path, body) = if cfg!(windows) {
            (
                dir.path().join("hello.cmd"),
                "@echo off\r\n                 echo {\"status\":\"%FCMD_COMMAND%\"}\r\n                 echo {\"run\":\"panel.reread\"}\r\n"
                    .to_string(),
            )
        } else {
            (
                dir.path().join("hello.sh"),
                "#!/bin/sh\n                 printf '{\"status\":\"%s\"}\\n' \"$FCMD_COMMAND\"\n                 echo '{\"run\":\"panel.reread\"}'\n                 # the context is on standard input, as JSON\n                 cat >/dev/null\n"
                    .to_string(),
            )
        };
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let ctx = Context {
            panel: Some(PanelView {
                cwd: dir.path().display().to_string(),
                dir: Some(dir.path().display().to_string()),
            }),
            ..Context::new("user.hello")
        };
        let reply = run_written(&path, Some(dir.path()), &ctx, Some(10)).expect("it runs");
        assert_eq!(reply.actions.len(), 2, "{:?}", reply);
        assert_eq!(reply.actions[0], Action::Status("user.hello".into()));
        assert_eq!(reply.actions[1], Action::Run("panel.reread".into()));
    }

    /// A plugin that never finishes is stopped, because a file manager that stops answering
    /// the keyboard because of somebody's script is worse than no plugin at all.
    #[test]
    #[cfg(unix)]
    fn a_program_that_hangs_is_killed_and_said_so() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hang.sh");
        std::fs::write(&path, "#!/bin/sh\nsleep 60\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        let err = run_written(&path, None, &Context::new("user.hang"), Some(1)).unwrap_err();
        assert!(matches!(err, RunError::Timeout { .. }), "{err}");
    }

    #[test]
    fn a_program_that_is_not_there_says_which_one() {
        let err = run_command(
            std::path::Path::new("/nowhere/at/all/plugin-xyz"),
            None,
            &Context::new("user.nope"),
            Some(1),
        )
        .unwrap_err();
        assert!(err.to_string().contains("plugin-xyz"), "{err}");
    }

    #[test]
    fn output_is_read_as_actions_and_anything_else_is_kept() {
        let out = concat!(
            "{\"status\":\"on main\"}\n",
            "{\"run\":\"panel.reread\"}\n",
            "{\"chdir\":{\"path\":\"/src\",\"panel\":\"other\"}}\n",
            "{\"message\":{\"title\":\"Branches\",\"lines\":[\"main\",\"next\"]}}\n",
            "{\"select\":[\"a.rs\"]}\n",
            "\n",
            "just some prose\n",
            "{\"teleport\":\"mars\"}\n",
        );
        let reply = parse_output(out, "a warning\n\n");
        assert_eq!(reply.actions.len(), 5, "{:?}", reply.actions);
        assert_eq!(reply.actions[0], Action::Status("on main".into()));
        assert_eq!(reply.actions[1], Action::Run("panel.reread".into()));
        assert_eq!(
            reply.actions[2],
            Action::Chdir {
                path: "/src".into(),
                panel: Some("other".into())
            }
        );
        assert!(matches!(reply.actions[3], Action::Message { .. }));
        assert_eq!(reply.actions[4], Action::Select(vec!["a.rs".into()]));
        // Prose and an action nobody knows both end up in front of the user.
        assert_eq!(
            reply.text,
            vec!["just some prose", "{\"teleport\":\"mars\"}"]
        );
        assert_eq!(reply.complaints, vec!["a warning"]);
    }
    /// A script for a column, where there is a shell to run it: `None` on a machine without
    /// `/bin/sh`, and the tests that need one say nothing there rather than fail.
    #[cfg(unix)]
    fn script(dir: &std::path::Path, name: &str, body: &str) -> Option<std::path::PathBuf> {
        use std::os::unix::fs::PermissionsExt;
        if !std::path::Path::new("/bin/sh").exists() {
            return None;
        }
        let path = dir.join(name);
        std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        Some(path)
    }

    #[test]
    #[cfg(unix)]
    fn a_column_program_is_asked_about_one_file_and_its_first_line_is_the_cell() {
        let dir = tempfile::tempdir().unwrap();
        let Some(program) = script(
            dir.path(),
            "lines.sh",
            "n=$(wc -l < \"$1\")\necho \"  $n $FCMD_COLUMN\"\necho ignored",
        ) else {
            return;
        };
        let file = dir.path().join("three.txt");
        std::fs::write(&file, "a\nb\nc\n").unwrap();
        let value = column_value(&program, dir.path(), &file, "lines", Some(10)).expect("it runs");
        assert_eq!(value.split_whitespace().collect::<Vec<_>>(), ["3", "lines"]);

        // Nothing printed is an empty cell; a failing status is an error with the program's
        // name in it, and a program that never ends is stopped.
        let quiet = script(dir.path(), "quiet.sh", "exit 0").unwrap();
        assert_eq!(
            column_value(&quiet, dir.path(), &file, "q", Some(10)).unwrap(),
            ""
        );
        let failing = script(dir.path(), "fail.sh", "echo half; exit 3").unwrap();
        let err = column_value(&failing, dir.path(), &file, "f", Some(10)).unwrap_err();
        assert!(matches!(err, RunError::Status { .. }), "{err}");
        assert!(err.to_string().contains("fail.sh"), "{err}");
        let hanging = script(dir.path(), "hang.sh", "exec sleep 60").unwrap();
        let err = column_value(&hanging, dir.path(), &file, "h", Some(1)).unwrap_err();
        assert!(matches!(err, RunError::Timeout { .. }), "{err}");
    }

    #[test]
    #[cfg(unix)]
    fn a_batch_answers_for_a_directory_in_one_run_by_path() {
        let dir = tempfile::tempdir().unwrap();
        // More files than a pipe holds the names of, so that a program reading while it
        // writes is the case that is tested.
        let Some(program) = script(
            dir.path(),
            "sizes.sh",
            "while IFS= read -r f; do\n  case \"$f\" in *skip*) continue;; esac\n  \
             printf '%s\\t%s\\n' \"$f\" \"${#f}\"\ndone\nprintf 'not a path\\t1\\nno tab here\\n'",
        ) else {
            return;
        };
        let files: Vec<std::path::PathBuf> = (0..3000)
            .map(|i| {
                dir.path().join(if i == 7 {
                    "skip-me".to_string()
                } else {
                    format!("file-{i:05}-with-a-name-long-enough-to-fill-a-pipe.txt")
                })
            })
            .collect();
        let values =
            column_batch(&program, dir.path(), &files, "sizes", Some(60)).expect("it runs");
        assert_eq!(values.len(), files.len());
        for (i, (file, value)) in files.iter().zip(&values).enumerate() {
            if i == 7 {
                assert_eq!(value, "", "what the program skipped is an empty cell");
            } else {
                assert_eq!(value, &file.display().to_string().len().to_string());
            }
        }
    }

    #[test]
    fn a_cell_is_one_line_with_nothing_that_moves_the_cursor() {
        assert_eq!(column_cell("  12 KiB\r"), "12 KiB");
        assert_eq!(column_cell("a\x1b[31mb\x07"), "a[31mb");
        assert_eq!(column_cell(&"x".repeat(1000)).len(), COLUMN_VALUE_MAX);
    }
}
