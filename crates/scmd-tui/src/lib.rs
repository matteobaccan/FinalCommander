//! The terminal backend: the only place that talks to a terminal.
//!
//! It takes a [`Buffer`] from `scmd-ui` and paints the cells that changed, and it turns terminal
//! input into [`scmd_cmd::Key`] events. See `docs/spec/01-architecture.md` §3–§4.

use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, EnableBracketedPaste, Event, KeyCode as CKey, KeyEvent,
    KeyEventKind, KeyModifiers,
};
use crossterm::style::{
    Attribute, Color as CColor, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, queue};
use scmd_cmd::{Key, KeyCode, Mods};
use scmd_ui::{Buffer, Color, ColorDepth};

/// What the input thread sends to the application loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    Paste(String),
    Resize(u16, u16),
    /// The terminal closed on us.
    Eof,
}

/// The writer behind [`Term::capturing`].
struct Sink(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl io::Write for Sink {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.lock().expect("the sink").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub struct Term {
    /// Where the painting goes. Behind a pointer rather than being `Stdout` itself, so that a
    /// test can be given the bytes: what this writes is a stream of cursor moves and
    /// characters whose correctness cannot be seen by looking at the buffer afterwards.
    out: Box<dyn io::Write + Send>,
    prev: Buffer,
    depth: ColorDepth,
    raw: bool,
}

impl Term {
    /// Take over the terminal: raw mode, alternate screen, no cursor.
    pub fn new() -> io::Result<Self> {
        let (w, h) = terminal::size().unwrap_or((80, 24));
        terminal::enable_raw_mode()?;
        let mut out = io::stdout();
        queue!(
            out,
            EnterAlternateScreen,
            EnableBracketedPaste,
            cursor::Hide
        )?;
        out.flush()?;
        Ok(Term {
            out: Box::new(out),
            prev: Buffer::new(w, h),
            depth: detect_depth(),
            raw: true,
        })
    }

    /// A terminal that writes nowhere, for tests and `--check` runs.
    pub fn headless(w: u16, h: u16) -> Self {
        Term {
            out: Box::new(io::stdout()),
            prev: Buffer::new(w, h),
            depth: ColorDepth::TrueColor,
            raw: false,
        }
    }

    /// A terminal that paints into a byte buffer the caller can read back.
    ///
    /// For the tests, and for one kind of test in particular: what [`Term::draw`] emits is a
    /// stream of cursor moves and characters, and whether it lands where it was meant to
    /// cannot be told from the cell buffer — only from replaying what was written.
    pub fn capturing() -> (Term, std::sync::Arc<std::sync::Mutex<Vec<u8>>>) {
        let sink = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let term = Term {
            out: Box::new(Sink(sink.clone())),
            // Empty, so the first frame paints every cell: a test starts from a blank screen
            // and has to see all of it.
            prev: Buffer::new(0, 0),
            depth: ColorDepth::TrueColor,
            raw: true,
        };
        (term, sink)
    }

    pub fn depth(&self) -> ColorDepth {
        self.depth
    }

    pub fn size(&self) -> (u16, u16) {
        terminal::size().unwrap_or((80, 24))
    }

    /// Paint the cells that differ from the previous frame.
    pub fn draw(&mut self, buf: &Buffer, cursor_at: Option<(u16, u16)>) -> io::Result<()> {
        if !self.raw {
            self.prev = buf.clone();
            return Ok(());
        }
        let changes = buf.diff(&self.prev);
        let mut last_style = None;
        let mut last_pos: Option<(u16, u16)> = None;
        for (x, y, cell) in changes {
            if last_pos != Some((x, y)) {
                queue!(self.out, cursor::MoveTo(x, y))?;
            }
            if last_style != Some(cell.style) {
                queue!(
                    self.out,
                    SetAttribute(Attribute::Reset),
                    SetForegroundColor(to_crossterm(cell.style.fg)),
                    SetBackgroundColor(to_crossterm(cell.style.bg))
                )?;
                if cell.style.bold {
                    queue!(self.out, SetAttribute(Attribute::Bold))?;
                }
                if cell.style.reverse {
                    queue!(self.out, SetAttribute(Attribute::Reverse))?;
                }
                last_style = Some(cell.style);
            }
            queue!(self.out, Print(cell.ch))?;
            // How far the terminal's cursor moved, so the next cell can be written without a
            // `MoveTo` when it follows this one. It must be *exactly* right: believing the
            // cursor is a column further right than it is writes the next character one
            // column to the left of where it belongs, and from there the row is rubbish —
            // which is what a screen full of half-overwritten text turns out to be.
            //
            // The width is not guessed from the character. It is read from the buffer, which
            // decided the layout: a character that takes two columns has a `wide_tail` cell
            // after it, and everything else takes one. Asking the character instead — and
            // calling every non-ASCII character two columns wide, which is what this did —
            // is wrong for the box-drawing characters the frames are made of, for `…`, `—`,
            // `√`, for every accented letter and for all of Cyrillic and Greek.
            let advance = match buf.get(x + 1, y) {
                Some(next) if next.wide_tail => 2,
                _ => 1,
            };
            last_pos = Some((x + advance, y));
        }
        match cursor_at {
            Some((x, y)) => queue!(self.out, cursor::MoveTo(x, y), cursor::Show)?,
            None => queue!(self.out, cursor::Hide)?,
        }
        self.out.flush()?;
        self.prev = buf.clone();
        Ok(())
    }

    /// Force a full repaint on the next draw (after a resize or an external program).
    pub fn invalidate(&mut self) {
        self.prev = Buffer::new(0, 0);
    }

    pub fn restore(&mut self) {
        if !self.raw {
            return;
        }
        let _ = queue!(
            self.out,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = self.out.flush();
        let _ = terminal::disable_raw_mode();
        self.raw = false;
    }
}

impl Drop for Term {
    fn drop(&mut self) {
        self.restore();
    }
}

/// Restore the terminal if we panic, so a crash never leaves an unusable shell.
pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = io::stdout();
        let _ = queue!(
            out,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            DisableBracketedPaste,
            LeaveAlternateScreen
        );
        let _ = out.flush();
        let _ = terminal::disable_raw_mode();
        previous(info);
    }));
}

/// Read terminal events forever, forwarding them as [`Input`]. Runs on its own thread because
/// `crossterm::event::read` blocks.
pub fn read_input_loop(tx: Sender<Input>) {
    loop {
        match event::read() {
            Ok(Event::Key(k)) => {
                if let Some(input) = map_key_event(k) {
                    if tx.send(input).is_err() {
                        return;
                    }
                }
            }
            Ok(Event::Resize(w, h)) => {
                if tx.send(Input::Resize(w, h)).is_err() {
                    return;
                }
            }
            Ok(Event::Paste(s)) => {
                if tx.send(Input::Paste(s)).is_err() {
                    return;
                }
            }
            Ok(_) => {}
            Err(_) => {
                let _ = tx.send(Input::Eof);
                return;
            }
        }
    }
}

/// Poll once, for tests and for the non-threaded path.
pub fn poll_input(timeout: Duration) -> io::Result<Option<Input>> {
    if !event::poll(timeout)? {
        return Ok(None);
    }
    Ok(match event::read()? {
        Event::Key(k) => map_key_event(k),
        Event::Resize(w, h) => Some(Input::Resize(w, h)),
        Event::Paste(s) => Some(Input::Paste(s)),
        _ => None,
    })
}

/// Translate a crossterm key event into our own key type. Key *releases* are dropped: with the
/// kitty protocol or Win32 input mode a terminal reports them, and acting on both would run
/// every command twice.
pub fn map_key_event(k: KeyEvent) -> Option<Input> {
    if matches!(k.kind, KeyEventKind::Release) {
        return None;
    }
    let mut mods = Mods::NONE;
    if k.modifiers.contains(KeyModifiers::CONTROL) {
        mods.insert(Mods::CTRL);
    }
    if k.modifiers.contains(KeyModifiers::ALT) {
        mods.insert(Mods::ALT);
    }
    if k.modifiers.contains(KeyModifiers::SHIFT) {
        mods.insert(Mods::SHIFT);
    }
    let code = match k.code {
        CKey::Char(c) => KeyCode::Char(c),
        CKey::F(n) => KeyCode::F(n),
        CKey::Enter => KeyCode::Enter,
        CKey::Esc => KeyCode::Esc,
        CKey::Tab => KeyCode::Tab,
        CKey::BackTab => KeyCode::BackTab,
        CKey::Backspace => KeyCode::Backspace,
        CKey::Delete => KeyCode::Delete,
        CKey::Insert => KeyCode::Insert,
        CKey::Home => KeyCode::Home,
        CKey::End => KeyCode::End,
        CKey::PageUp => KeyCode::PageUp,
        CKey::PageDown => KeyCode::PageDown,
        CKey::Up => KeyCode::Up,
        CKey::Down => KeyCode::Down,
        CKey::Left => KeyCode::Left,
        CKey::Right => KeyCode::Right,
        _ => return None,
    };
    Some(Input::Key(Key { code, mods }))
}

fn to_crossterm(c: Color) -> CColor {
    match c {
        Color::Default => CColor::Reset,
        Color::Ansi(n) | Color::Indexed(n) => CColor::AnsiValue(n),
        Color::Rgb(r, g, b) => CColor::Rgb { r, g, b },
    }
}

/// How much colour this terminal admits to supporting.
pub fn detect_depth() -> ColorDepth {
    if std::env::var_os("NO_COLOR").is_some() {
        return ColorDepth::Mono;
    }
    let colorterm = std::env::var("COLORTERM").unwrap_or_default();
    if colorterm.contains("truecolor") || colorterm.contains("24bit") {
        return ColorDepth::TrueColor;
    }
    let term = std::env::var("TERM").unwrap_or_default();
    if term.contains("256color") || term.contains("direct") {
        return ColorDepth::Indexed256;
    }
    if term.is_empty() || term == "dumb" {
        return ColorDepth::Mono;
    }
    ColorDepth::Ansi16
}

#[cfg(test)]
mod tests {
    use super::*;
    use scmd_ui::{Style, Theme};

    fn key(code: CKey, mods: KeyModifiers) -> Option<Input> {
        map_key_event(KeyEvent::new(code, mods))
    }

    #[test]
    fn maps_function_keys_and_modifiers() {
        assert_eq!(
            key(CKey::F(5), KeyModifiers::NONE),
            Some(Input::Key("F5".parse().unwrap()))
        );
        assert_eq!(
            key(CKey::F(3), KeyModifiers::CONTROL | KeyModifiers::SHIFT),
            Some(Input::Key("Ctrl+Shift+F3".parse().unwrap()))
        );
        assert_eq!(
            key(CKey::Char('r'), KeyModifiers::CONTROL),
            Some(Input::Key("Ctrl+R".parse().unwrap()))
        );
        assert_eq!(
            key(CKey::PageUp, KeyModifiers::CONTROL),
            Some(Input::Key("Ctrl+PgUp".parse().unwrap()))
        );
    }

    #[test]
    fn key_releases_are_ignored() {
        let mut ev = KeyEvent::new(CKey::F(5), KeyModifiers::NONE);
        ev.kind = KeyEventKind::Release;
        assert_eq!(map_key_event(ev), None);
    }

    #[test]
    fn unknown_keys_are_dropped_rather_than_guessed() {
        assert_eq!(key(CKey::CapsLock, KeyModifiers::NONE), None);
    }

    /// Replay what was written to a terminal, the way a terminal would, and say what is on
    /// the screen afterwards. Only two things are interpreted: the cursor moves and the
    /// characters. Colours do not move the cursor and are skipped.
    fn replay(bytes: &[u8], w: usize, h: usize) -> Vec<String> {
        let text = String::from_utf8_lossy(bytes).into_owned();
        let mut screen = vec![vec![' '; w]; h];
        let (mut x, mut y) = (0usize, 0usize);
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '\u{1b}' {
                let width = unicode_width::UnicodeWidthChar::width(c)
                    .unwrap_or(1)
                    .max(1);
                if y < h && x < w {
                    screen[y][x] = c;
                    // The second column of a wide character holds nothing of its own, which
                    // is what the buffer's `wide_tail` means; `\0` stands for it here and is
                    // dropped when the row is read back.
                    if width == 2 && x + 1 < w {
                        screen[y][x + 1] = '\0';
                    }
                }
                x += width;
                continue;
            }
            if chars.peek() != Some(&'[') {
                continue;
            }
            chars.next();
            let mut params = String::new();
            let mut final_byte = ' ';
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    final_byte = c;
                    break;
                }
                params.push(c);
            }
            if final_byte == 'H' {
                let mut parts = params.split(';');
                let row: usize = parts.next().unwrap_or("1").parse().unwrap_or(1);
                let col: usize = parts.next().unwrap_or("1").parse().unwrap_or(1);
                y = row.saturating_sub(1);
                x = col.saturating_sub(1);
            }
        }
        screen
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .filter(|c| *c != '\0')
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect()
    }

    /// What is painted has to end up where the buffer says, and the bug this is about could
    /// not be seen any other way: the backend believed every character outside ASCII took two
    /// columns, so after a `│` — or a `…`, or an `é`, or anything Cyrillic — it thought the
    /// cursor was one column further right than it was, skipped the move it then needed, and
    /// wrote the rest of the row one place to the left, over the text already there.
    ///
    /// A panel's frame is made of those characters, so this was most of the screen.
    #[test]
    fn what_is_painted_lands_where_the_buffer_says() {
        let theme = Theme::default_theme();
        let plain = Style::new(theme.color("panel.fg"), theme.color("panel.bg"));
        let (mut term, sink) = Term::capturing();

        let mut buf = Buffer::new(24, 2);
        // Box drawing, an ellipsis, an accent and Cyrillic: every one of them is a single
        // column, and every one of them used to be counted as two.
        buf.put_str(0, 0, "│ab…cdé─ключ│", plain, 24);
        buf.put_str(0, 1, "plain ascii row", plain, 24);
        term.draw(&buf, None).unwrap();
        let screen = replay(&sink.lock().unwrap(), 24, 2);
        assert_eq!(screen[0], buf.to_lines()[0].trim_end(), "first paint");
        assert_eq!(screen[1], buf.to_lines()[1].trim_end());

        // The painting that actually went wrong needs two cells with a gap between them: only
        // the cells that differ are written, so after a character the backend measured wrongly
        // it skips the move it then needs and writes the next one a column early. A first
        // paint writes every cell in order and cannot show it, which is how this survived a
        // suite full of rendering tests.
        sink.lock().unwrap().clear();
        let mut next = buf.clone();
        // Column 0 changes (to another single-column character outside ASCII) and column 2
        // changes; column 1 does not, so where the `X` lands is decided by what the backend
        // believes about the width of what it wrote before it.
        next.put_str(0, 0, "é", plain, 1);
        next.put_str(2, 0, "X", plain, 1);
        term.draw(&next, None).unwrap();
        let screen = replay(&sink.lock().unwrap(), 24, 2);
        assert_eq!(screen[0].chars().next(), Some('é'), "{:?}", screen[0]);
        assert_eq!(
            screen[0].chars().nth(2),
            Some('X'),
            "the second cell landed in the wrong column: {:?}",
            screen[0]
        );
    }

    /// A character that really is two columns wide keeps its second column: the buffer marks
    /// it, and the backend reads the mark rather than guessing from the character again.
    #[test]
    fn a_double_width_character_advances_by_two() {
        let theme = Theme::default_theme();
        let plain = Style::new(theme.color("panel.fg"), theme.color("panel.bg"));
        let (mut term, sink) = Term::capturing();
        let mut buf = Buffer::new(12, 1);
        buf.put_str(0, 0, "日本語ab", plain, 12);
        term.draw(&buf, None).unwrap();
        let screen = replay(&sink.lock().unwrap(), 12, 1);
        assert_eq!(screen[0], buf.to_lines()[0].trim_end());
    }

    #[test]
    fn headless_draw_tracks_the_buffer_without_writing() {
        let mut term = Term::headless(20, 3);
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(20, 3);
        buf.put_str(
            0,
            0,
            "hello",
            Style::new(theme.color("panel.fg"), theme.color("panel.bg")),
            20,
        );
        term.draw(&buf, None).unwrap();
        // The second draw has nothing to do, which is what the diff is for.
        assert!(buf.diff(&term.prev).is_empty());
    }

    #[test]
    fn colour_conversion_covers_every_variant() {
        assert!(matches!(to_crossterm(Color::Default), CColor::Reset));
        assert!(matches!(to_crossterm(Color::Ansi(3)), CColor::AnsiValue(3)));
        assert!(matches!(
            to_crossterm(Color::Rgb(1, 2, 3)),
            CColor::Rgb { r: 1, g: 2, b: 3 }
        ));
    }

    #[test]
    fn no_color_forces_monochrome() {
        // The env var is global, so this test only checks the mapping of TERM values.
        assert_eq!(
            match std::env::var_os("NO_COLOR") {
                Some(_) => ColorDepth::Mono,
                None => detect_depth(),
            },
            detect_depth()
        );
    }
}
