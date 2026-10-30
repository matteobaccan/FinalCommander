//! The pseudo-terminal itself: a shell on the other end of a pair of file descriptors.
//!
//! A program decides whether it is talking to a terminal by asking the operating system about
//! the thing it is writing to, which is why a command run through a pipe gives no colour, no
//! prompt and no full-screen interface. Running it through a *pseudo*-terminal is what makes
//! `vim`, `top` and `ssh` work: the kernel (or, on Windows, ConPTY) presents one end as a real
//! terminal to the program and hands the other end to us.
//!
//! The reading happens on a thread of its own, because a read from a terminal blocks until
//! something is written to it and the interface cannot wait. What it reads goes through the
//! parser into a [`Screen`] behind a lock, so the drawing side only ever looks at a screen
//! that is whole.

use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use portable_pty::{CommandBuilder, PtySize};

use crate::screen::Screen;

/// What went wrong starting or talking to a terminal.
#[derive(Debug, thiserror::Error)]
pub enum TermError {
    #[error("{0}")]
    Spawn(String),
    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// A running terminal: the program, its screen, and the two ends of the pair.
pub struct Terminal {
    /// What the program has drawn, as a grid. Shared with the reader thread.
    screen: Arc<Mutex<Screen>>,
    /// The writing end, for the keys.
    writer: Box<dyn Write + Send>,
    /// The handle that resizes the pair, which tells the program (`SIGWINCH`).
    master: Box<dyn portable_pty::MasterPty + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    /// Set by the reader thread when the other end closes: the program has gone.
    finished: Arc<AtomicBool>,
    /// What this terminal is called in a list of screens: the shell, or the title it set.
    pub name: String,
}

impl std::fmt::Debug for Terminal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Terminal")
            .field("name", &self.name)
            .field("finished", &self.finished.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Terminal {
    /// Start `program` with a terminal of this size, in `cwd`.
    ///
    /// `program` is a command line, split the way a shell splits one, so a caller can start
    /// `bash -l` as easily as `bash`. Nothing is interpreted beyond that splitting: there is
    /// no shell in between to expand a `*` or a `$HOME`, because the thing being started *is*
    /// the shell.
    ///
    /// `wake` is called after every read, and is how an interface that waits for events hears
    /// about output nobody asked for. It must not block: it exists to put one message on a
    /// queue. A caller with nothing to wake passes `|| {}`.
    pub fn start(
        program: &str,
        cwd: Option<&std::path::Path>,
        cols: u16,
        rows: u16,
        wake: impl Fn() + Send + 'static,
    ) -> Result<Terminal, TermError> {
        let Open {
            writer,
            mut reader,
            master,
            child,
            head,
        } = open(program, cwd, cols, rows)?;

        let screen = Arc::new(Mutex::new(Screen::new(cols, rows)));
        let finished = Arc::new(AtomicBool::new(false));
        {
            let screen = screen.clone();
            let finished = finished.clone();
            std::thread::Builder::new()
                .name("fcmd-pty".into())
                .spawn(move || {
                    let mut parser = vte::Parser::new();
                    let mut buffer = [0u8; 8 * 1024];
                    loop {
                        match reader.read(&mut buffer) {
                            Ok(0) | Err(_) => break,
                            Ok(read) => {
                                // One lock per read rather than one per byte: a program that
                                // writes a screenful at a time should cost one lock, and the
                                // drawing side should never see half of it.
                                if let Ok(mut screen) = screen.lock() {
                                    parser.advance(&mut *screen, &buffer[..read]);
                                }
                                // Drawn when the interface next looks, which is now.
                                wake();
                            }
                        }
                    }
                    finished.store(true, Ordering::Relaxed);
                    // The last wake: a shell that exited leaves a screen to redraw and a row
                    // in the screen list to take away.
                    wake();
                })
                .map_err(TermError::Io)?;
        }

        Ok(Terminal {
            screen,
            writer,
            master,
            child,
            finished,
            name: head,
        })
    }

    /// The screen, for drawing. The lock is held only while the caller looks.
    pub fn with_screen<T>(&self, f: impl FnOnce(&mut Screen) -> T) -> T {
        let mut screen = self.screen.lock().unwrap_or_else(|e| e.into_inner());
        f(&mut screen)
    }

    /// Has the program finished?
    pub fn finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    /// The exit status, once there is one.
    pub fn status(&mut self) -> Option<u32> {
        match self.child.try_wait() {
            Ok(Some(status)) => Some(status.exit_code()),
            _ => None,
        }
    }

    /// Send bytes to the program, as if they had been typed.
    pub fn send(&mut self, bytes: &[u8]) -> Result<(), TermError> {
        self.writer.write_all(bytes)?;
        self.writer.flush()?;
        Ok(())
    }

    /// A new size, for the screen and for the program — which is told, so a full-screen
    /// program redraws itself rather than staying the shape of the old window.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<(), TermError> {
        self.master
            .resize(PtySize {
                rows: rows.max(1),
                cols: cols.max(1),
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TermError::Spawn(e.to_string()))?;
        self.with_screen(|screen| screen.resize(cols, rows));
        Ok(())
    }

    /// Stop the program. A terminal whose screen is closed must not leave a shell running
    /// with nothing reading it.
    pub fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        self.kill();
    }
}

/// A pair that has just been opened, with a program already running on the far end of it.
pub(crate) struct Open {
    pub writer: Box<dyn Write + Send>,
    pub reader: Box<dyn Read + Send>,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
    /// The program, as it was named: what to call this in a list.
    pub head: String,
}

/// Open a pseudo-terminal and start `program` on the far end of it.
///
/// Shared by the terminal pane and the shell behind the command line, because the two differ
/// in what they do with the output and in nothing else.
pub(crate) fn open(
    program: &str,
    cwd: Option<&std::path::Path>,
    cols: u16,
    rows: u16,
) -> Result<Open, TermError> {
    let parts = shell_split(program);
    let Some((head, tail)) = parts.split_first() else {
        return Err(TermError::Spawn("no program to run".into()));
    };
    let pair = portable_pty::native_pty_system()
        .openpty(PtySize {
            rows: rows.max(1),
            cols: cols.max(1),
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| TermError::Spawn(e.to_string()))?;

    let mut command = CommandBuilder::new(head);
    command.args(tail);
    if let Some(cwd) = cwd {
        command.cwd(cwd);
    }
    // What the program is told it is talking to. `xterm-256color` is what every terminal in
    // use claims and what every program has a table for; claiming something rarer would mean
    // programs falling back to no colour at all.
    command.env("TERM", "xterm-256color");
    command.env("COLORTERM", "truecolor");

    let child = pair
        .slave
        .spawn_command(command)
        .map_err(|e| TermError::Spawn(e.to_string()))?;
    // The slave end is held by the child now. Dropping ours is what lets a read of the master
    // end report end-of-file when the child exits — without it the read would block for ever
    // on a terminal nobody is writing to.
    drop(pair.slave);

    let writer = pair
        .master
        .take_writer()
        .map_err(|e| TermError::Spawn(e.to_string()))?;
    let reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| TermError::Spawn(e.to_string()))?;
    Ok(Open {
        writer,
        reader,
        master: pair.master,
        child,
        head: head.clone(),
    })
}

/// Split a command line the way a shell splits one, keeping quoted words whole.
fn shell_split(line: &str) -> Vec<String> {
    shell_words::split(line).unwrap_or_else(|_| {
        // An unbalanced quote is not a reason to run nothing: the words as they stand are a
        // better guess than a refusal, and the program that is not found says so itself.
        line.split_whitespace().map(|s| s.to_string()).collect()
    })
}

/// The shell to start when nothing says otherwise.
///
/// What the user's own session uses, which is what they expect their aliases and their prompt
/// to come from: `$SHELL` on Unix, `ComSpec` on Windows, and a last resort that exists on
/// every installation of each.
pub fn default_shell() -> String {
    if cfg!(windows) {
        std::env::var("ComSpec").unwrap_or_else(|_| "cmd.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Wait for the screen to say something, so a test does not race the reader thread.
    fn wait_for(term: &Terminal, want: &str) -> String {
        for _ in 0..200 {
            let text = term.with_screen(|s| s.text());
            if text.contains(want) {
                return text;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        term.with_screen(|s| s.text())
    }

    #[cfg(unix)]
    #[test]
    fn a_program_runs_and_what_it_writes_arrives() {
        let mut term = Terminal::start(
            "/bin/sh -c 'printf \"hello from a pty\\n\"'",
            None,
            40,
            6,
            || {},
        )
        .expect("a shell starts");
        let text = wait_for(&term, "hello");
        assert!(text.contains("hello from a pty"), "{text}");
        for _ in 0..200 {
            if term.finished() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(term.finished(), "and the program ends");
        assert_eq!(term.status(), Some(0));
    }

    /// The whole point of a pseudo-terminal: the program believes it has a terminal, so it
    /// behaves as it does for a person. Through a pipe `test -t 1` is false.
    #[cfg(unix)]
    #[test]
    fn the_program_believes_it_has_a_terminal() {
        let term = Terminal::start(
            "/bin/sh -c 'if [ -t 1 ]; then echo yes-a-tty; else echo no-tty; fi'",
            None,
            40,
            4,
            || {},
        )
        .expect("a shell starts");
        let text = wait_for(&term, "tty");
        assert!(text.contains("yes-a-tty"), "{text}");
    }

    /// And it is told how big it is, which is what a full-screen program lays itself out from.
    #[cfg(unix)]
    #[test]
    fn the_program_is_told_the_size_and_told_again_when_it_changes() {
        let mut term = Terminal::start(
            "/bin/sh -c 'stty size; trap \"stty size\" WINCH; sleep 5'",
            None,
            72,
            19,
            || {},
        )
        .expect("a shell starts");
        let text = wait_for(&term, "19 72");
        assert!(text.contains("19 72"), "the size it started with: {text}");

        term.resize(50, 11).expect("it resizes");
        let text = wait_for(&term, "11 50");
        assert!(
            text.contains("11 50"),
            "and the size it was changed to: {text}"
        );
        term.kill();
    }

    /// Keys typed reach the program, and what it echoes comes back.
    #[cfg(unix)]
    #[test]
    fn what_is_typed_reaches_the_program() {
        let mut term = Terminal::start(
            "/bin/sh -c 'read line; echo \"got: $line\"'",
            None,
            40,
            4,
            || {},
        )
        .expect("a shell starts");
        term.send(b"a word\n").expect("the keys go through");
        let text = wait_for(&term, "got:");
        assert!(text.contains("got: a word"), "{text}");
    }

    /// A program that draws a full screen leaves the shell's output alone, because it uses
    /// the alternate screen — and that is what makes `vim` usable inside a file manager.
    #[cfg(unix)]
    #[test]
    fn a_full_screen_program_uses_the_other_screen_and_gives_it_back() {
        let mut term = Terminal::start(
            "/bin/sh -c 'echo before; printf \"\\033[?1049h\"; echo inside; sleep 5'",
            None,
            30,
            5,
            || {},
        )
        .expect("a shell starts");
        let text = wait_for(&term, "inside");
        assert!(text.contains("inside"), "{text}");
        assert!(
            !text.contains("before"),
            "the shell's output is behind it: {text}"
        );
        term.kill();
    }

    #[test]
    fn a_command_line_is_split_as_a_shell_splits_one() {
        assert_eq!(shell_split("bash -l"), vec!["bash", "-l"]);
        assert_eq!(
            shell_split("/bin/sh -c 'echo a b'"),
            vec!["/bin/sh", "-c", "echo a b"]
        );
        // An unbalanced quote still runs something rather than nothing.
        assert_eq!(shell_split("sh -c 'oops"), vec!["sh", "-c", "'oops"]);
    }

    #[test]
    fn a_program_that_is_not_there_says_so_rather_than_hanging() {
        let error = Terminal::start("/nonexistent/fcmd-not-a-shell", None, 20, 4, || {})
            .expect_err("it cannot start");
        assert!(matches!(error, TermError::Spawn(_)), "{error:?}");
    }

    #[test]
    fn there_is_always_a_shell_to_fall_back_to() {
        let shell = default_shell();
        assert!(!shell.is_empty());
    }
}
