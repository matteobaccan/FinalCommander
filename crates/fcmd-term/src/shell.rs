//! The shell behind the command line: one shell, for the whole session (SPEC 06 §7).
//!
//! Typing `cd ..` into a command line that runs each command in a child of its own does
//! nothing a person can see: the child changes its own directory and dies with it. The same
//! goes for `export`, for a shell function, for a `TMPDIR` set once — everything that makes a
//! shell a place rather than a calculator. Midnight Commander solved this in the eighties by
//! keeping one shell alive behind the panels and writing the typed line into it, and the panel
//! and the shell then share a directory. This is that.
//!
//! What is sent is the line, as typed, followed by a newline — there is no second shell in
//! between, so a `*`, a `$HOME` or an alias means what the user's own shell says it means.
//!
//! What comes back is read as lines by [`Transcript`], and the shell is asked, when it starts,
//! to say three things about itself: that a command has begun, that it has ended and with what
//! status, and which directory it is in. They are the sequences terminals already use for
//! this (`OSC 133` and `OSC 7`), so a shell whose own configuration emits them is understood
//! anyway.
//!
//! A shell that cannot be asked — `cmd.exe`, which has no prompt hook at all — has no entry in
//! [`hook_for`], and the caller is expected to keep running such commands one child at a time.
//! Half a subshell, where `cd` works and the status does not, would be worse than neither.

use std::io::{Read, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use crate::pty::{self, TermError};
use crate::transcript::Transcript;

/// A shell that stays, and what it has printed.
pub struct Shell {
    writer: Box<dyn Write + Send>,
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    out: Arc<Mutex<Transcript>>,
    gone: Arc<AtomicBool>,
    /// A command line written before the shell had reached its first prompt, waiting for it.
    ///
    /// A shell takes a moment to start, and the hook is installed in that moment. A line
    /// written into it before then would be answered by the *startup* prompt — the mark that
    /// prompt carries would be read as this command having ended, before it had run. So the
    /// line waits here, and goes in when the shell is ready to be typed at.
    waiting: Option<String>,
    /// The shell as it was named, for the message that says one is running.
    pub program: String,
}

impl std::fmt::Debug for Shell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Shell")
            .field("program", &self.program)
            .field("gone", &self.gone.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Shell {
    /// Start `program` in `cwd` and ask it to mark its prompts.
    ///
    /// `wake` is called after every read, and must not block: it is how an interface that
    /// waits for events hears about output nobody asked for.
    pub fn start(
        program: &str,
        cwd: Option<&Path>,
        cols: u16,
        rows: u16,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Shell, TermError> {
        let pty::Open {
            writer,
            mut reader,
            master,
            child,
            head,
        } = pty::open(program, cwd, cols, rows)?;

        let out = Arc::new(Mutex::new(Transcript::default()));
        let gone = Arc::new(AtomicBool::new(false));
        {
            let out = out.clone();
            let gone = gone.clone();
            std::thread::Builder::new()
                .name("fcmd-shell".into())
                .spawn(move || {
                    let mut parser = vte::Parser::new();
                    let mut buffer = [0u8; 8 * 1024];
                    loop {
                        match reader.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => {
                                if let Ok(mut out) = out.lock() {
                                    parser.advance(&mut *out, &buffer[..read]);
                                }
                                wake();
                            }
                        }
                    }
                    gone.store(true, Ordering::Relaxed);
                    wake();
                })
                .map_err(TermError::Io)?;
        }

        let mut shell = Shell {
            writer,
            master,
            child,
            out,
            gone,
            waiting: None,
            program: head.clone(),
        };
        if let Some(hook) = hook_for(&head) {
            shell.line(&hook)?;
        }
        Ok(shell)
    }

    /// Run a command line: what the user typed, into the shell, as if typed at its prompt.
    ///
    /// A line that arrives before the shell has reached its first prompt waits for it — see
    /// [`Shell::waiting`] — and [`Shell::nudge`] is what lets it in.
    pub fn run(&mut self, line: &str) -> Result<(), TermError> {
        if !self.with_out(|out| out.ready()) {
            self.waiting = Some(line.to_string());
            return Ok(());
        }
        self.with_out(|out| out.sent(line));
        self.line(line)
    }

    /// Let a waiting line in, if the shell has become ready for it. Called wherever the
    /// output is collected, which is after every read.
    pub fn nudge(&mut self) -> Result<(), TermError> {
        if self.waiting.is_none() || !self.with_out(|out| out.ready()) {
            return Ok(());
        }
        let Some(line) = self.waiting.take() else {
            return Ok(());
        };
        self.with_out(|out| out.sent(&line));
        self.line(&line)
    }

    /// Follow the panel: the shell changes directory too.
    ///
    /// Silent, which is the point — it is not a command the user ran, so it leaves nothing in
    /// the user screen and does not count as one. Its own `cd` and the prompt that follows are
    /// swallowed because the transcript drops what arrives at a prompt.
    pub fn cd(&mut self, dir: &Path) -> Result<(), TermError> {
        let line = format!("cd {}", quote(&dir.display().to_string()));
        self.line(&line)
    }

    /// Bytes straight to the shell, for the keys a running program is owed: `Ctrl+C`,
    /// `Ctrl+D`, and what somebody types at a program that asked a question.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), TermError> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// One line, with the newline that runs it.
    fn line(&mut self, line: &str) -> Result<(), TermError> {
        self.send(line.as_bytes())?;
        self.send(b"\n")
    }

    /// Look at what has been printed. The lock is held only while the caller looks.
    pub fn with_out<T>(&self, f: impl FnOnce(&mut Transcript) -> T) -> T {
        let mut out = self.out.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut out)
    }

    /// Has the shell exited — `exit`, or killed from outside?
    pub fn gone(&self) -> bool {
        self.gone.load(Ordering::Relaxed)
    }

    /// Is a command running in it?
    pub fn running(&self) -> bool {
        self.with_out(|out| out.running())
    }

    /// Is there a command in it at all — running, or still waiting for the first prompt?
    ///
    /// This is the one to ask before saying a command has ended: a line that has not gone in
    /// yet has certainly not finished.
    pub fn busy(&self) -> bool {
        self.waiting.is_some() || self.running()
    }

    /// Tell the shell how wide the screen is, so what it prints wraps where it is shown.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermError> {
        self.master
            .resize(portable_pty::PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TermError::Spawn(e.to_string()))
    }

    /// Stop it. A shell nobody is reading has no business still running.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Shell {
    fn drop(&mut self) {
        self.kill();
    }
}

/// The line that asks a shell to mark its prompts, chosen by which shell it is.
///
/// Each one sets two hooks: one before a command runs, which says a command starts here, and
/// one before each prompt, which says the last one ended with this status and names the
/// directory. They are written with `printf`, whose `\033` the shell's own builtin expands, so
/// the line itself is plain text.
///
/// `None` for a shell with no hook to set. The caller runs commands in a child of their own
/// for those, which is what everything did before there was a shell to keep.
///
/// What this replaces is this shell's `PROMPT_COMMAND` (or `precmd`), not the user's: the
/// prompt of a shell nobody looks at does no other work, and their own configuration has
/// already run by the time this line is sent.
pub fn hook_for(program: &str) -> Option<String> {
    const DONE: &str = r#"printf "\033]133;D;%s\007\033]7;file://%s%s\007""#;
    let name = Path::new(program)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| program.to_string())
        .to_ascii_lowercase();
    match name.as_str() {
        "bash" => Some(format!(
            "PS0=$'\\033]133;C\\007'; PROMPT_COMMAND='{DONE} \"$?\" \"$HOSTNAME\" \"$PWD\"'"
        )),
        "zsh" => Some(format!(
            "precmd() {{ {DONE} \"$?\" \"$HOST\" \"$PWD\" }}; \
             preexec() {{ printf \"\\033]133;C\\007\" }}"
        )),
        "fish" => Some(
            "function __fcmd_pre --on-event fish_preexec; printf \"\\033]133;C\\007\"; end; \
             function __fcmd_post --on-event fish_postexec; \
             printf \"\\033]133;D;%s\\007\\033]7;file://%s%s\\007\" $status $hostname $PWD; end"
                .to_string(),
        ),
        // A plain POSIX shell — `dash`, `ash`, busybox, and whatever `/bin/sh` is here — has
        // no hook before a command and no `PROMPT_COMMAND`. It does expand `$?` and `$PWD` in
        // its prompt, every time it draws one, which is the same thing arriving at the same
        // moment. The escapes have to be real bytes, because a POSIX prompt is not unescaped.
        "sh" | "dash" | "ash" | "busybox" | "ksh" | "mksh" => Some(
            "__fcmd_e=$(printf '\\033'); __fcmd_a=$(printf '\\007'); \
             PS1=\"${__fcmd_e}]133;D;\\$?${__fcmd_a}${__fcmd_e}]7;file://\\$PWD${__fcmd_a}\""
                .to_string(),
        ),
        _ => None,
    }
}

/// One argument, safe to hand a shell: wrapped in single quotes, with any of its own closed,
/// escaped and reopened. The form every POSIX shell and `fish` read the same way.
pub fn quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wait for the transcript to hold a line, so a test does not race the reader thread.
    fn wait_for(shell: &Shell, want: &str) -> Vec<String> {
        let mut seen: Vec<String> = Vec::new();
        for _ in 0..400 {
            seen.extend(shell.with_out(|out| out.take_lines()));
            if seen.iter().any(|line| line.contains(want)) {
                return seen;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        seen
    }

    fn wait_until(shell: &Shell, done: impl Fn(&mut Transcript) -> bool) -> bool {
        for _ in 0..400 {
            if shell.with_out(&done) {
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        false
    }

    #[test]
    fn a_shell_with_no_hook_to_set_says_so_rather_than_half_setting_one() {
        assert!(hook_for("cmd.exe").is_none());
        assert!(hook_for("powershell").is_none());
        assert!(hook_for("/bin/bash").is_some());
        assert!(hook_for("/usr/bin/zsh").is_some());
        assert!(hook_for("/bin/sh").is_some());
    }

    #[test]
    fn an_argument_with_a_quote_in_it_survives_being_handed_to_a_shell() {
        assert_eq!(quote("/tmp/plain"), "'/tmp/plain'");
        assert_eq!(quote("it's here"), r"'it'\''s here'");
    }

    /// The whole point: what one command does, the next one sees.
    #[cfg(unix)]
    #[test]
    fn the_shell_stays_so_a_directory_and_a_variable_outlive_the_command() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        let mut shell =
            Shell::start("/bin/sh", Some(dir.path()), 80, 24, || {}).expect("a shell starts");
        assert!(wait_until(&shell, |out| out.ready()), "it reaches a prompt");

        shell.run("cd inner; FCMD_KEPT=yes").unwrap();
        assert!(
            wait_until(&shell, |out| !out.running()),
            "the first command ended before the second was sent"
        );
        shell
            .run("printf '%s %s\\n' \"$(basename $PWD)\" \"$FCMD_KEPT\"")
            .unwrap();
        let lines = wait_for(&shell, "inner yes");
        assert!(
            lines.iter().any(|line| line.contains("inner yes")),
            "the directory and the variable are still there: {lines:?}"
        );
    }

    /// The race this answers, which only showed itself on a slower machine: a command typed
    /// before the shell had reached its first prompt was answered by the *startup* prompt,
    /// whose mark was read as the command having ended before it had run.
    #[cfg(unix)]
    #[test]
    fn a_command_written_before_the_shell_is_ready_waits_for_it() {
        let mut shell = Shell::start("/bin/sh", None, 80, 24, || {}).expect("a shell starts");
        // Immediately, with no wait: the hook has not even been sent yet.
        shell.run("printf 'ran %s\\n' late").unwrap();
        assert!(shell.busy(), "the command is in hand, waiting for a prompt");

        let mut lines: Vec<String> = Vec::new();
        for _ in 0..400 {
            let _ = shell.nudge();
            lines.extend(shell.with_out(|out| out.take_lines()));
            if lines.iter().any(|line| line.contains("ran late")) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            lines.iter().any(|line| line.contains("ran late")),
            "the waiting command ran once the shell was ready: {lines:?}"
        );
        assert!(
            wait_until(&shell, |out| !out.running()),
            "and then it ended"
        );
        assert!(!shell.busy());
    }

    /// And it says where it is, which is what the panel follows.
    #[cfg(unix)]
    #[test]
    fn the_shell_says_which_directory_it_is_in() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner");
        std::fs::create_dir(&inner).unwrap();
        let mut shell =
            Shell::start("/bin/sh", Some(dir.path()), 80, 24, || {}).expect("a shell starts");
        assert!(
            wait_until(&shell, |out| out.cwd.is_some()),
            "it says where it starts"
        );

        shell.run("cd inner").unwrap();
        let said = wait_until(&shell, |out| {
            out.cwd.as_deref().is_some_and(|p| p.ends_with("inner"))
        });
        assert!(
            said,
            "it said it had moved: {:?}",
            shell.with_out(|o| o.cwd.clone())
        );
    }

    /// How a command ended, which is what the status line shows.
    #[cfg(unix)]
    #[test]
    fn the_status_of_a_command_comes_back() {
        let mut shell = Shell::start("/bin/sh", None, 80, 24, || {}).expect("a shell starts");
        assert!(wait_until(&shell, |out| out.ready()));
        shell.run("(exit 7)").unwrap();
        assert!(wait_until(&shell, |out| out.status == Some(7)));
    }

    /// The panel moves, and the shell follows without saying anything about it.
    #[cfg(unix)]
    #[test]
    fn following_the_panel_leaves_nothing_in_the_output() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("a dir with spaces");
        std::fs::create_dir(&inner).unwrap();
        let mut shell =
            Shell::start("/bin/sh", Some(dir.path()), 80, 24, || {}).expect("a shell starts");
        assert!(wait_until(&shell, |out| out.ready()));
        let _ = shell.with_out(|out| out.take_lines());

        shell.cd(&inner).unwrap();
        let said = wait_until(&shell, |out| {
            out.cwd
                .as_deref()
                .is_some_and(|p| p.ends_with("a dir with spaces"))
        });
        assert!(said, "the shell followed into a name with spaces in it");
        assert!(
            shell.with_out(|out| out.take_lines()).is_empty(),
            "and nothing about it reached the user screen"
        );
    }
}
