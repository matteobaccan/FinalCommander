//! What a shell printed, as lines rather than as a grid (SPEC 06 §7).
//!
//! The terminal pane keeps a [`crate::Screen`]: a grid of cells, which is what `vim` and `top`
//! need. A shell sitting behind the command line wants the other thing — what it printed, line
//! by line, to go under the panels where `Ctrl+O` finds it and stays as scrollback. So this is
//! the same parser driving a different sink: printable text into a line, the control codes that
//! move within a line honoured, everything that paints a screen thrown away.
//!
//! It also reads the three things a shell can be asked to say about itself, which is how the
//! panel and the shell stay in the same directory:
//!
//!   * `OSC 7` — *I am in this directory now*. The sequence terminals already use for it, so a
//!     shell whose own configuration emits it (most distributions ship one that does) is
//!     understood without being asked twice.
//!   * `OSC 133;C` and `OSC 133;D;<status>` — *a command starts here* and *it finished, like
//!     this*. The semantic-prompt marks. Between them is the command's own output; outside
//!     them is the prompt and the echo of what was typed, which the user screen already shows
//!     in its heading and must not show twice.
//!   * `OSC 0` / `OSC 2` — the title, which is what a shell calls itself.
//!
//! Nothing here assumes a shell that cooperates: without the marks every line is kept, which
//! is the old behaviour of capturing a command's output and no worse.

use std::path::PathBuf;

/// How many whole lines may wait to be taken. A command that prints a million lines is a
/// command whose first lines are long gone from the user screen anyway, and the memory is
/// better spent on the ones that will be read.
const MAX_WAITING: usize = 20_000;

/// Where the parser is, between the marks a shell puts round a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Where {
    /// Before anything: the shell's own banner, and the line that installed the hooks.
    Starting,
    /// At a prompt. What arrives is the prompt itself and the echo of what is typed.
    Prompt,
    /// Inside a command. What arrives is its output.
    Output,
}

/// A shell's output, read into lines.
#[derive(Debug)]
pub struct Transcript {
    /// The line being printed, as characters, because `\r` and `\b` move within it.
    line: Vec<char>,
    /// Where the next character goes, which is not always the end.
    at: usize,
    /// Whole lines nobody has taken yet.
    done: Vec<String>,
    /// Lines dropped because [`MAX_WAITING`] was reached, so the caller can say so.
    pub lost: u64,
    place: Where,
    /// The command just sent, until the terminal echoes it back.
    echo: Option<String>,
    /// The directory the shell last said it was in.
    pub cwd: Option<PathBuf>,
    /// How many commands have been seen to finish since one was sent. Zero at the moment a
    /// command goes in, so a prompt seen before it cannot be taken for its end.
    pub finished: u64,
    /// How the last one ended.
    pub status: Option<i32>,
    /// What the shell calls itself.
    pub title: Option<String>,
}

impl Default for Transcript {
    fn default() -> Self {
        Transcript {
            line: Vec::new(),
            at: 0,
            done: Vec::new(),
            lost: 0,
            place: Where::Starting,
            echo: None,
            cwd: None,
            finished: 0,
            status: None,
            title: None,
        }
    }
}

impl Transcript {
    /// The whole lines printed since the last time they were taken, oldest first.
    ///
    /// The line being printed is not among them: a progress bar rewriting itself with `\r` is
    /// one line and should not become a thousand. It is [`Transcript::pending`] until the
    /// newline comes.
    pub fn take_lines(&mut self) -> Vec<String> {
        std::mem::take(&mut self.done)
    }

    /// The line that has been printed but not ended.
    pub fn pending(&self) -> String {
        self.line.iter().collect()
    }

    /// Is a command running — has one started and not yet been seen to finish?
    pub fn running(&self) -> bool {
        self.place == Where::Output
    }

    /// A command line has just been written to the shell.
    ///
    /// Told rather than inferred, for two reasons. The output starts arriving before any mark
    /// does, and a shell that cannot be made to mark where a command begins — only where it
    /// ends — would otherwise have its output taken for part of the prompt. And the terminal
    /// echoes what was typed, which the user screen already shows in its own heading: given
    /// the text, the echo of it can be left out without guessing.
    pub fn sent(&mut self, line: &str) {
        self.line.clear();
        self.at = 0;
        self.place = Where::Output;
        self.finished = 0;
        self.status = None;
        let line = line.trim();
        self.echo = (!line.is_empty()).then(|| line.to_string());
    }

    /// Has the shell reached a prompt at least once, so it is ready to be typed at?
    pub fn ready(&self) -> bool {
        self.place != Where::Starting
    }

    /// Give the pending line to the waiting ones. Called when a command ends, so its last
    /// line is not held back by a shell that printed no newline after it.
    fn flush(&mut self) {
        if self.line.is_empty() {
            return;
        }
        self.end_line();
    }

    fn end_line(&mut self) {
        let text: String = self.line.iter().collect();
        self.line.clear();
        self.at = 0;
        // Only what a command printed is kept. Before the first prompt there is the shell's
        // own banner and the line that installed the hooks; at a prompt there is the prompt
        // and the echo of what was typed, which the user screen already shows in its heading.
        // A shell that marks nothing never reaches either state: the command line tells it a
        // command was sent, and from then on every line is a command's.
        if self.place != Where::Output {
            return;
        }
        if let Some(sent) = self.echo.take() {
            // The echo is the command, sometimes with the prompt printed in front of it.
            if text.trim_end().ends_with(&sent) {
                return;
            }
        }
        if self.done.len() >= MAX_WAITING {
            self.done.remove(0);
            self.lost += 1;
        }
        self.done.push(text);
    }

    fn put(&mut self, c: char) {
        if self.at < self.line.len() {
            self.line[self.at] = c;
        } else {
            self.line.push(c);
        }
        self.at += 1;
    }

    /// `OSC 7`: `file://host/path`, which is how a shell names a directory to a terminal. The
    /// host is whose directory it is — ours only when it is this machine's or unsaid.
    fn osc_cwd(&mut self, text: &str) {
        let rest = match text.strip_prefix("file://") {
            Some(rest) => rest,
            // Some shells send the path alone. It is unambiguous, so it is taken.
            None if text.starts_with('/') => text,
            None => return,
        };
        let path = match rest.find('/') {
            Some(cut) => &rest[cut..],
            None if rest.starts_with('/') => rest,
            None => return,
        };
        let path = percent_decode(path);
        if !path.is_empty() {
            self.cwd = Some(PathBuf::from(path));
        }
    }

    /// `OSC 133`: where a command starts and where it ends.
    fn osc_mark(&mut self, params: &[&[u8]]) {
        let Some(kind) = params.get(1).and_then(|p| p.first()).copied() else {
            return;
        };
        match kind {
            // A command starts here: what follows is its output.
            b'C' => {
                self.line.clear();
                self.at = 0;
                self.echo = None;
                self.place = Where::Output;
            }
            // And it has finished, with a status when the shell says one.
            b'D' => {
                self.flush();
                self.place = Where::Prompt;
                self.finished = self.finished.saturating_add(1);
                self.status = params
                    .get(2)
                    .and_then(|p| std::str::from_utf8(p).ok())
                    .and_then(|s| s.trim().parse::<i32>().ok());
            }
            // The prompt begins, or its input does.
            b'A' | b'B' | b'P' => self.place = Where::Prompt,
            _ => {}
        }
    }
}

impl vte::Perform for Transcript {
    fn print(&mut self, c: char) {
        self.put(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' => self.end_line(),
            // Back to the start of the line, which a progress bar does to rewrite it.
            b'\r' => self.at = 0,
            b'\t' => {
                let stop = (self.at / 8 + 1) * 8;
                while self.at < stop {
                    self.put(' ');
                }
            }
            0x08 => self.at = self.at.saturating_sub(1),
            _ => {}
        }
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(first) = params.first() else {
            return;
        };
        match *first {
            b"7" => {
                if let Some(text) = params.get(1).and_then(|p| std::str::from_utf8(p).ok()) {
                    self.osc_cwd(text);
                }
            }
            b"133" => self.osc_mark(params),
            b"0" | b"2" => {
                if let Some(text) = params.get(1).and_then(|p| std::str::from_utf8(p).ok()) {
                    self.title = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &vte::Params, _: &[u8], _: bool, action: char) {
        // Only the two that mean something to a line. Everything else paints a screen, and a
        // transcript has none to paint.
        match action {
            // Erase in line: a shell redrawing its prompt, and `clear` at the end of its work.
            'K' => {
                let how = params.iter().next().and_then(|p| p.first()).copied();
                match how {
                    Some(1) => {
                        for i in 0..self.at.min(self.line.len()) {
                            self.line[i] = ' ';
                        }
                    }
                    Some(2) => {
                        self.line.clear();
                        self.at = 0;
                    }
                    // To the end of the line, which is the default.
                    _ => self.line.truncate(self.at),
                }
            }
            // Cursor to a column, which a prompt uses to line itself up.
            'G' => {
                let col = params
                    .iter()
                    .next()
                    .and_then(|p| p.first())
                    .copied()
                    .unwrap_or(1)
                    .max(1) as usize;
                self.at = col - 1;
                while self.line.len() < self.at {
                    self.line.push(' ');
                }
            }
            _ => {}
        }
    }
}

/// `%20` back into a space, for a path a shell wrapped in a URL.
fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok();
            if let Some(byte) = hex.and_then(|h| u8::from_str_radix(h, 16).ok()) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feed bytes through the parser, as the reader thread does.
    fn feed(out: &mut Transcript, bytes: &[u8]) {
        let mut parser = vte::Parser::new();
        parser.advance(out, bytes);
    }

    /// A transcript with a command running in it, which is the state the lines arrive in: a
    /// fresh one is still reading the shell's own banner and keeps none of it.
    fn running(command: &str) -> Transcript {
        let mut out = Transcript::default();
        out.sent(command);
        out
    }

    #[test]
    fn whole_lines_are_taken_and_the_one_being_printed_waits() {
        let mut out = running("a command");
        feed(&mut out, b"one\ntwo\nthree");
        assert_eq!(out.take_lines(), ["one", "two"]);
        assert_eq!(out.pending(), "three");
        // Taken once, not twice.
        assert!(out.take_lines().is_empty());
        feed(&mut out, b"-and-a-bit\n");
        assert_eq!(out.take_lines(), ["three-and-a-bit"]);
    }

    /// A progress bar is one line rewriting itself, not a thousand lines.
    #[test]
    fn a_carriage_return_rewrites_the_line_it_is_in() {
        let mut out = running("a command");
        feed(&mut out, b"  0%\r 50%\r100%\n");
        assert_eq!(out.take_lines(), ["100%"]);
    }

    /// What the shell says about itself: where it is, and how the command went.
    #[test]
    fn the_shell_says_which_directory_it_is_in_and_how_the_command_ended() {
        let mut out = Transcript::default();
        feed(&mut out, b"\x1b]7;file://home/tmp/a%20place\x07");
        assert_eq!(out.cwd, Some(PathBuf::from("/tmp/a place")));
        assert_eq!(out.finished, 0);
        feed(&mut out, b"\x1b]133;C\x07made it\n\x1b]133;D;3\x07");
        assert_eq!(out.take_lines(), ["made it"]);
        assert_eq!(out.finished, 1);
        assert_eq!(out.status, Some(3));
        assert!(!out.running(), "the command has ended");
    }

    /// The prompt and the echo of the typed command are the shell talking to itself: the user
    /// screen writes its own heading and a second copy would be noise.
    #[test]
    fn the_prompt_and_the_echo_of_the_command_are_left_out() {
        let mut out = Transcript::default();
        // A prompt, the command echoed after it, then the command's own output.
        feed(&mut out, b"\x1b]133;D;0\x07user@host:~$ ls -l\r\n");
        feed(
            &mut out,
            b"\x1b]133;C\x07total 0\nfile.txt\n\x1b]133;D;0\x07",
        );
        assert_eq!(out.take_lines(), ["total 0", "file.txt"]);
    }

    /// A shell that can be made to say where a command *ends* but not where it begins — a
    /// plain POSIX shell, whose only hook is its prompt — still has its output kept, because
    /// the command line says when it sent one. The echo of the command goes.
    #[test]
    fn a_shell_that_marks_only_its_prompt_keeps_the_output_and_drops_the_echo() {
        let mut out = Transcript::default();
        feed(&mut out, b"\x1b]133;D;0\x07");
        out.sent("ls -l");
        assert!(out.running(), "the command line said it sent one");
        feed(&mut out, b"$ ls -l\r\ntotal 0\nfile.txt\n");
        assert_eq!(out.take_lines(), ["total 0", "file.txt"]);
        feed(&mut out, b"\x1b]133;D;0\x07");
        assert!(!out.running());
        assert_eq!(out.status, Some(0));
    }

    /// What a shell prints before its first prompt is its own business: the banner, the
    /// message of the day, and the line that installed the hooks. None of it was asked for.
    #[test]
    fn nothing_printed_before_the_first_prompt_is_kept() {
        let mut out = Transcript::default();
        feed(&mut out, b"Welcome to a shell\n__fcmd_e=$(printf ...)\n");
        assert!(out.take_lines().is_empty());
        assert!(!out.ready());
        // From the first prompt it is ready, and from the command it keeps what it prints.
        feed(&mut out, b"\x1b]133;D;0\x07");
        assert!(out.ready());
        out.sent("echo hello");
        feed(&mut out, b"hello\n");
        assert_eq!(out.take_lines(), ["hello"]);
    }

    /// A shell that marks nothing is not punished for it: every line a command printed is
    /// kept, which is what capturing the output of a child did before any of this. What such
    /// a shell cannot say is when the command *ended*, which is the reason only a shell with
    /// a hook to set is kept alive behind the command line at all.
    #[test]
    fn a_shell_that_marks_nothing_has_all_of_its_lines_kept() {
        let mut out = running("a command");
        feed(&mut out, b"a plain line\nand another\n");
        assert_eq!(out.take_lines(), ["a plain line", "and another"]);
        assert_eq!(out.finished, 0, "and it never says the command ended");
    }

    /// The line a shell erases while redrawing its prompt does not come out doubled.
    #[test]
    fn erasing_to_the_end_of_the_line_shortens_it() {
        let mut out = running("a command");
        feed(&mut out, b"a long line\r\x1b[Kshort\n");
        assert_eq!(out.take_lines(), ["short"]);
    }

    #[test]
    fn a_tab_goes_to_the_next_stop_and_a_backspace_takes_one_back() {
        let mut out = running("a command");
        feed(&mut out, b"ab\tc\n");
        assert_eq!(out.take_lines(), ["ab      c"]);
        feed(&mut out, b"xyz\x08\x08!\n");
        assert_eq!(out.take_lines(), ["x!z"]);
    }

    #[test]
    fn the_waiting_lines_do_not_grow_without_end() {
        let mut out = running("a command");
        for i in 0..MAX_WAITING + 10 {
            feed(&mut out, format!("line {i}\n").as_bytes());
        }
        let lines = out.take_lines();
        assert_eq!(lines.len(), MAX_WAITING);
        assert_eq!(out.lost, 10);
        // The ones that were lost are the oldest: the newest are what somebody is reading.
        assert_eq!(lines.last().unwrap(), &format!("line {}", MAX_WAITING + 9));
    }
}
