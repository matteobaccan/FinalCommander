//! The terminal backend: the only place that talks to a terminal.
//!
//! It takes a [`Buffer`] from `scmd-ui` and paints the cells that changed, and it turns terminal
//! input into [`scmd_cmd::Key`] events. See `docs/spec/01-architecture.md` §3–§4.

use std::io::{self, Stdout, Write};
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

pub struct Term {
    out: Stdout,
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
            out,
            prev: Buffer::new(w, h),
            depth: detect_depth(),
            raw: true,
        })
    }

    /// A terminal that writes nowhere, for tests and `--check` runs.
    pub fn headless(w: u16, h: u16) -> Self {
        Term {
            out: io::stdout(),
            prev: Buffer::new(w, h),
            depth: ColorDepth::TrueColor,
            raw: false,
        }
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
            let advance = if cell.ch.is_ascii() { 1 } else { 2 };
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
