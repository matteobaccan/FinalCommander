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
}
