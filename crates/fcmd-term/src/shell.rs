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
    pty: fcmd_platform::pty::Pty,
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
    /// When it was started, so a shell that never reaches a marked prompt can be given up on.
    pub started: std::time::Instant,
    /// What ends a line written into it: `\r` for PowerShell, which reads its console the way
    /// a person's `Enter` arrives, and `\n` for the rest, as it always was.
    newline: &'static [u8],
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
            pty,
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
            pty,
            out,
            gone,
            waiting: None,
            program: head.clone(),
            started: std::time::Instant::now(),
            newline: if is_powershell(&head) { b"\r" } else { b"\n" },
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
        let newline = self.newline;
        self.send(newline)
    }

    /// Has it reached a marked prompt at least once? A shell that never does is one whose
    /// marks are not getting through — a console host that drops the sequences it does not
    /// know — and the caller gives it up after a while rather than wait for ever.
    pub fn ready(&self) -> bool {
        self.with_out(|out| out.ready())
    }

    /// The line that was waiting for the first prompt, taken back: the caller runs it some
    /// other way when this shell is given up on.
    pub fn take_waiting(&mut self) -> Option<String> {
        self.waiting.take()
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
        Ok(self.pty.resize(cols, rows)?)
    }

    /// Stop it. A shell nobody is reading has no business still running.
    pub fn kill(&mut self) {
        self.pty.kill();
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
/// None of them puts the directory *in* the prompt. A prompt belongs to the shell's line
/// editor, which counts its width to know where the cursor is -- and an escape sequence it was
/// not told to ignore is counted as letters on the screen. A directory long enough to carry
/// that count past the edge of the terminal is then redrawn, and a redrawn prompt is not the
/// prompt: `readline` repeats the character that falls in the last column, which in a path is
/// a letter that was never there. So the marks are printed *to the terminal*, from a hook that
/// runs before the prompt is drawn, and what the editor is handed is empty.
///
/// `None` for a shell with no hook to set. The caller runs commands in a child of their own
/// for those, which is what everything did before there was a shell to keep.
///
/// What this replaces is this shell's `PROMPT_COMMAND` (or `precmd`), not the user's: the
/// prompt of a shell nobody looks at does no other work, and their own configuration has
/// already run by the time this line is sent.
pub fn hook_for(program: &str) -> Option<String> {
    const DONE: &str = r#"printf "\033]133;D;%s\007\033]7;file://%s%s\007""#;
    let name = shell_name(program);
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
        // no hook before a command and no `PROMPT_COMMAND`. What it does have is a prompt it
        // expands afresh every time it draws one, which is the same moment: so the prompt is
        // one command substitution, it prints the marks straight to the terminal, and it
        // expands to nothing. Nothing is what the line editor is then given to measure — see
        // above for what it does to a prompt it measures wrongly.
        "sh" | "dash" | "ash" | "busybox" | "ksh" | "mksh" => {
            Some(format!("PS1='$({DONE} \"$?\" \"\" \"$PWD\" >/dev/tty)'"))
        }
        // PowerShell has the hook too: `prompt` is a function it calls before every prompt,
        // and what it writes to the console goes to the terminal before the prompt does. The
        // status is `$?` read first, before anything in the function can change it, with the
        // exit code of a program that failed in place of a bare 1. The directory is the
        // provider's path with forward slashes, which is how a file URL spells `C:\Users`.
        // PSReadLine is unloaded in this one shell nobody types at: it redraws the line it
        // is given in colour, and the plain echo is what the transcript knows to leave out.
        "pwsh" | "powershell" => Some(
            "Remove-Module PSReadLine -ErrorAction SilentlyContinue; \
             function global:prompt { \
             $ok = $?; \
             $s = if ($ok) { 0 } elseif ($global:LASTEXITCODE) { $global:LASTEXITCODE } else { 1 }; \
             $p = $PWD.ProviderPath -replace '\\\\', '/'; \
             if (-not $p.StartsWith('/')) { $p = '/' + $p }; \
             $e = [char]27; $b = [char]7; \
             [Console]::Write(\"$e]133;D;$s$b$e]7;file://$env:COMPUTERNAME$p$b\"); \
             ' ' }"
                .to_string(),
        ),
        _ => None,
    }
}

/// The shell's name as the tables here know it: the file name without its directory, written
/// either way, and without `.exe`, in lower case. `C:\Program Files\PowerShell\7\pwsh.exe`
/// is `pwsh` on every system, which matters because the configuration is read on all three.
pub fn shell_name(program: &str) -> String {
    let base = program.rsplit(['/', '\\']).next().unwrap_or(program);
    Path::new(base)
        .file_stem()
        .map(|s| s.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default()
}

/// Is this one of the PowerShells? They take a line ended the way a console's `Enter` ends
/// it, and say their first prompt later than the others, so the caller waits longer and has a
/// way out if the marks never come.
pub fn is_powershell(program: &str) -> bool {
    matches!(shell_name(program).as_str(), "pwsh" | "powershell")
}

/// The program and the arguments that run one command line in `shell` — the shell the
/// configuration names (`term.shell`, a command line of its own, so `pwsh -NoProfile` works),
/// or, empty, the session's own.
///
/// Each shell is asked the way it is meant to be asked: `cmd.exe` with `/C`, PowerShell
/// (`pwsh`, `powershell`) with `-Command` and no banner or prompt to wait at, Nushell and
/// every POSIX shell, `fish` and the rest with `-c`. The line itself is passed whole, as one
/// argument: it is the shell's to parse, with its own pipes and quotes.
pub fn run_line_with(shell: &str, line: &str) -> (String, Vec<String>) {
    let mut words = crate::pty::shell_split(shell);
    if words.is_empty() {
        words = crate::pty::shell_split(&crate::pty::default_shell());
    }
    let program = words.remove(0);
    // The name whichever way the path is written: a Windows path read on any system.
    let name = shell_name(&program);
    let mut args = words;
    match name.as_str() {
        "cmd" => {
            args.push("/C".into());
            args.push(line.into());
        }
        "pwsh" | "powershell" => {
            for flag in ["-NoLogo", "-NonInteractive"] {
                if !args.iter().any(|a| a.eq_ignore_ascii_case(flag)) {
                    args.push(flag.into());
                }
            }
            args.push("-Command".into());
            args.push(line.into());
        }
        _ => {
            args.push("-c".into());
            args.push(line.into());
        }
    }
    (program, args)
}

/// One argument, safe to hand a shell: wrapped in single quotes, with any of its own closed,
/// escaped and reopened. The form every POSIX shell and `fish` read the same way.
pub fn quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each shell is handed a line the way it reads one.
    #[test]
    fn each_shell_is_asked_its_own_way() {
        let run = |shell: &str| run_line_with(shell, "ls | wc -l");
        assert_eq!(
            run("C:\\Windows\\System32\\cmd.exe"),
            (
                "C:\\Windows\\System32\\cmd.exe".into(),
                vec!["/C".into(), "ls | wc -l".into()]
            )
        );
        assert_eq!(
            run("pwsh"),
            (
                "pwsh".into(),
                vec![
                    "-NoLogo".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    "ls | wc -l".into()
                ]
            )
        );
        // The configuration's own flags stay, and are not given twice.
        let (program, args) = run("powershell.exe -NoProfile -nologo");
        assert_eq!(program, "powershell.exe");
        assert_eq!(
            args,
            [
                "-NoProfile",
                "-nologo",
                "-NonInteractive",
                "-Command",
                "ls | wc -l"
            ]
        );
        assert_eq!(
            run("/usr/bin/fish"),
            (
                "/usr/bin/fish".into(),
                vec!["-c".into(), "ls | wc -l".into()]
            )
        );
        assert_eq!(
            run("bash -l").1,
            vec!["-l".to_string(), "-c".into(), "ls | wc -l".into()]
        );
        // Nothing configured: the session's own shell.
        assert!(!run("").0.is_empty());
    }

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
        assert!(hook_for("C:\\Windows\\System32\\cmd.exe").is_none());
        assert!(hook_for("/bin/bash").is_some());
        assert!(hook_for("/usr/bin/zsh").is_some());
        assert!(hook_for("/bin/sh").is_some());
    }

    /// PowerShell is kept like the POSIX shells (issue 43): a `prompt` function that marks the
    /// end of each command with its status and says where it is, under every spelling of its
    /// name, and with `Enter` as a console sends it.
    #[test]
    fn powershell_has_a_prompt_hook_under_every_spelling_of_its_name() {
        for name in [
            "pwsh",
            "powershell",
            "PowerShell.exe",
            "C:\\Program Files\\PowerShell\\7\\pwsh.exe",
            "/usr/local/bin/pwsh",
        ] {
            let hook = hook_for(name).unwrap_or_else(|| panic!("{name} has no hook"));
            assert!(hook.contains("function global:prompt"), "{hook}");
            assert!(hook.contains("133;D;$s"), "the status is marked: {hook}");
            assert!(hook.contains("]7;file://"), "the directory is said: {hook}");
            assert!(is_powershell(name), "{name}");
        }
        assert!(!is_powershell("/bin/bash"));
        assert!(!is_powershell("cmd.exe"));
        // The status is read before anything else in the function can change it.
        let hook = hook_for("pwsh").unwrap();
        let body = &hook[hook.find("function global:prompt").unwrap()..];
        assert!(body.find("$ok = $?") < body.find("$s ="), "{body}");
    }

    /// The directory does not travel in the prompt, and this is the line that says so.
    ///
    /// A line editor measures the prompt to know where the cursor is, counting an escape
    /// sequence it was not told to ignore as letters on the screen. A prompt it measures
    /// wrongly it redraws wrongly — `readline` repeats the character that falls in the last
    /// column — and in a path that is a letter that was never there, so the panel follows a
    /// directory nobody has. The prompt is therefore one substitution that prints to the
    /// terminal and expands to nothing.
    #[test]
    fn the_posix_hook_keeps_the_directory_out_of_the_prompt() {
        let hook = hook_for("/bin/sh").expect("a POSIX shell has a hook to set");
        let (_, prompt) = hook.split_once("PS1=").expect("it is the prompt it sets");
        assert!(
            prompt.starts_with("'$(") && prompt.ends_with(")'"),
            "the whole of it is one substitution: {prompt}"
        );
        assert!(
            prompt.contains(">/dev/tty"),
            "which prints to the terminal rather than into the prompt: {prompt}"
        );
    }

    /// And the same thing proved against a real shell that has a line editor: `bash` under
    /// the name `sh` reads the POSIX hook and brings `readline` with it. The directory is
    /// named longer than the terminal is wide, which is the case that used to come back with
    /// a letter doubled in it.
    #[cfg(unix)]
    #[test]
    fn a_shell_with_a_line_editor_names_a_long_directory_letter_for_letter() {
        let Some(bash) = ["/bin/bash", "/usr/bin/bash"]
            .into_iter()
            .map(Path::new)
            .find(|at| at.exists())
        else {
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        // Under the name `sh`: that is what puts `bash` in POSIX mode, and what picks the
        // POSIX hook for it.
        let sh = dir.path().join("sh");
        std::os::unix::fs::symlink(bash, &sh).unwrap();
        let deep = dir.path().join("a".repeat(120));
        std::fs::create_dir(&deep).unwrap();
        let want = std::fs::canonicalize(&deep).unwrap();

        let mut shell = Shell::start(&sh.display().to_string(), Some(dir.path()), 40, 24, || {})
            .expect("a shell starts");
        assert!(wait_until(&shell, |out| out.ready()), "it reaches a prompt");
        shell.cd(&deep).unwrap();
        // Canonicalised on both sides, because a shell answers with the path the system gives
        // it and the test holds the one it asked for — on macOS those are two names for one
        // place. A mangled path is no name for anywhere and does not canonicalise at all.
        assert!(
            wait_until(&shell, |out| out
                .cwd
                .as_deref()
                .and_then(|at| std::fs::canonicalize(at).ok())
                .is_some_and(|at| at == want)),
            "it named the directory as it is: {:?}",
            shell.with_out(|out| out.cwd.clone())
        );
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
