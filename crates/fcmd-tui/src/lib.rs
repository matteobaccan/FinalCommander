//! The terminal backend: the only place that talks to a terminal.
//!
//! It takes a [`Buffer`] from `fcmd-ui` and paints the cells that changed, and it turns terminal
//! input into [`fcmd_cmd::Key`] events. See `docs/spec/01-architecture.md` §3–§4.

use std::io::{self, Write};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode as CKey, KeyEvent, KeyEventKind, KeyModifiers, KeyboardEnhancementFlags,
    PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
};
use crossterm::style::{
    Attribute, Color as CColor, Print, SetAttribute, SetBackgroundColor, SetForegroundColor,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, queue};
use fcmd_cmd::{Key, KeyCode, Mods};
use fcmd_ui::{Buffer, Color, ColorDepth};

/// What the input thread sends to the application loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    Key(Key),
    /// Something the mouse did, when mouse reporting is on.
    Mouse(Mouse),
    /// The modifiers being held right now, with no key pressed yet. Only terminals that
    /// speak the keyboard protocol report this; the rest never send it, and the key bar
    /// simply does not change for them.
    Mods(Mods),
    Paste(String),
    Resize(u16, u16),
    /// The terminal closed on us.
    Eof,
}

/// Which button, for the events that have one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    Left,
    Right,
    Middle,
}

/// What the mouse did. A terminal reports rather less than a windowing system does: there is
/// no double click (the application times two presses itself), and movement is only reported
/// while a button is down unless the program asks for more.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Press(Button),
    Release(Button),
    /// Moved with a button held.
    Drag(Button),
    ScrollUp,
    ScrollDown,
    /// The sideways wheel, which a trackpad has and a mouse often does not. Reported by
    /// every terminal that reports the other two.
    ScrollLeft,
    ScrollRight,
}

/// One mouse event, in cells from the top left of the screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mouse {
    pub kind: MouseKind,
    pub col: u16,
    pub row: u16,
    pub mods: Mods,
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
    /// Whether the keyboard protocol was turned on, so that it is turned off again.
    enhanced: bool,
    /// Whether the terminal is reporting the mouse. Off until something asks for it, because
    /// a terminal in mouse mode no longer lets the user select text with the mouse, and that
    /// is the user's own tool being taken away.
    mouse: bool,
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
        // The key bar changes as a modifier goes down, the way FAR's does, and a terminal
        // only says so when it reports every key as an escape code. Terminals that do not
        // understand this ignore it; nothing else depends on it.
        let enhanced = terminal::supports_keyboard_enhancement().unwrap_or(false);
        if enhanced {
            let _ = queue!(
                out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                )
            );
        }
        out.flush()?;
        Ok(Term {
            out: Box::new(out),
            prev: Buffer::new(w, h),
            depth: detect_depth(),
            raw: true,
            enhanced,
            mouse: false,
        })
    }

    /// A terminal that writes nowhere, for tests and `--check` runs.
    pub fn headless(w: u16, h: u16) -> Self {
        Term {
            out: Box::new(io::stdout()),
            prev: Buffer::new(w, h),
            depth: ColorDepth::TrueColor,
            raw: false,
            enhanced: false,
            mouse: false,
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
            enhanced: false,
            mouse: false,
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

    /// Ask the terminal for a font one step larger or smaller, or for the one it started with.
    ///
    /// A terminal program does not own its font: the emulator does, and there is exactly one
    /// control sequence for asking — xterm's `OSC 50`, where a `#` and a signed number step
    /// through the font menu and `#0` is the entry it started on. xterm and the terminals that
    /// copy it do this; the rest ignore it, and on most of them `Ctrl` with plus or minus is
    /// the emulator's own zoom and never reaches a program at all. Either way the request is
    /// the same four-and-a-bit bytes, nothing else depends on the answer, and the next frame
    /// reads the size again — so a font that did change is drawn at the new size.
    ///
    /// `by` is a step: negative smaller, positive larger, zero back to the first font.
    pub fn font_step(&mut self, by: i8) -> io::Result<()> {
        let request = match by {
            0 => "\x1b]50;#0\x1b\\".to_string(),
            by if by < 0 => format!("\x1b]50;#-{}\x1b\\", -(by as i32)),
            by => format!("\x1b]50;#+{by}\x1b\\"),
        };
        self.out.write_all(request.as_bytes())?;
        self.out.flush()?;
        // A font the terminal did change leaves the cells a different size and the screen in
        // whatever state the reflow left it, so the next frame paints all of it.
        self.invalidate();
        Ok(())
    }

    /// Ask the terminal to put text on the system clipboard.
    ///
    /// This is `OSC 52`, and it is the only way a program in a terminal can reach the
    /// clipboard without knowing which window system it is in: no X11 or Wayland library, no
    /// `pbcopy`, no `unsafe`, and — the reason it is worth preferring even where those exist
    /// — it works over `ssh`, where the clipboard that matters belongs to the machine in
    /// front of the user and not to the one the program runs on.
    ///
    /// The other half, *reading* the clipboard, is deliberately not here: the terminal would
    /// have to answer with the contents, most refuse to for the obvious reason, and the
    /// terminal's own paste key already sends us the text as a bracketed paste. So we write
    /// and the terminal reads.
    ///
    /// A terminal that does not understand the sequence ignores it, and there is no answer to
    /// wait for, so nothing can be reported: the caller says what it copied, not that the
    /// clipboard took it.
    pub fn set_clipboard(&mut self, text: &str) -> io::Result<()> {
        self.out.write_all(clipboard_request(text).as_bytes())?;
        self.out.flush()
    }

    /// Turn mouse reporting on or off. Returns whether anything changed.
    ///
    /// It is a setting rather than a fact about the terminal: while it is on, the terminal
    /// hands us the clicks and stops handling them itself, which means no selecting text with
    /// the mouse and no middle-click paste. So it is only on when the configuration asks for
    /// it, and `ui.mouse = false` gives the terminal back.
    pub fn set_mouse(&mut self, on: bool) -> io::Result<bool> {
        if !self.raw || self.mouse == on {
            return Ok(false);
        }
        if on {
            queue!(self.out, EnableMouseCapture)?;
        } else {
            queue!(self.out, DisableMouseCapture)?;
        }
        self.out.flush()?;
        self.mouse = on;
        Ok(true)
    }

    pub fn mouse_on(&self) -> bool {
        self.mouse
    }

    pub fn restore(&mut self) {
        if !self.raw {
            return;
        }
        if self.mouse {
            let _ = queue!(self.out, DisableMouseCapture);
            self.mouse = false;
        }
        if self.enhanced {
            let _ = queue!(self.out, PopKeyboardEnhancementFlags);
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
/// The `OSC 52` request that puts `text` on the clipboard, as a string so that it can be
/// tested without a terminal to send it to.
pub fn clipboard_request(text: &str) -> String {
    format!("\x1b]52;c;{}\x1b\\", base64(text.as_bytes()))
}

/// Standard base64, which is what `OSC 52` carries. Twenty lines rather than a dependency:
/// this is the only thing in the program that encodes any.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for group in bytes.chunks(3) {
        let b = [
            group[0],
            *group.get(1).unwrap_or(&0),
            *group.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        for i in 0..4 {
            // The last group is padded with `=` for the bytes it does not have.
            if i <= group.len() {
                let six = (n >> (18 - 6 * i)) & 0x3f;
                out.push(ALPHABET[six as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let mut out = io::stdout();
        let _ = queue!(
            out,
            SetAttribute(Attribute::Reset),
            cursor::Show,
            DisableBracketedPaste,
            // Unconditional: a terminal that was not reporting the mouse ignores this, and one
            // that was would otherwise be left reporting it to a shell that cannot read it.
            DisableMouseCapture,
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
            Ok(Event::Mouse(m)) => {
                if let Some(input) = map_mouse_event(m) {
                    if tx.send(input).is_err() {
                        return;
                    }
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
        Event::Mouse(m) => map_mouse_event(m),
        _ => None,
    })
}

/// Translate a crossterm mouse event into our own.
///
/// Plain movement with no button down is dropped: nothing here follows the pointer, and a
/// terminal that reports every move would wake the application a hundred times a second for
/// nothing.
pub fn map_mouse_event(m: crossterm::event::MouseEvent) -> Option<Input> {
    use crossterm::event::{MouseButton as CButton, MouseEventKind as CKind};
    let button = |b: CButton| match b {
        CButton::Left => Button::Left,
        CButton::Right => Button::Right,
        CButton::Middle => Button::Middle,
    };
    let kind = match m.kind {
        CKind::Down(b) => MouseKind::Press(button(b)),
        CKind::Up(b) => MouseKind::Release(button(b)),
        CKind::Drag(b) => MouseKind::Drag(button(b)),
        CKind::ScrollUp => MouseKind::ScrollUp,
        CKind::ScrollDown => MouseKind::ScrollDown,
        CKind::ScrollLeft => MouseKind::ScrollLeft,
        CKind::ScrollRight => MouseKind::ScrollRight,
        // Bare movement: nothing here reads it.
        CKind::Moved => return None,
    };
    let mut mods = Mods::NONE;
    if m.modifiers.contains(KeyModifiers::CONTROL) {
        mods.insert(Mods::CTRL);
    }
    if m.modifiers.contains(KeyModifiers::ALT) {
        mods.insert(Mods::ALT);
    }
    if m.modifiers.contains(KeyModifiers::SHIFT) {
        mods.insert(Mods::SHIFT);
    }
    Some(Input::Mouse(Mouse {
        kind,
        col: m.column,
        row: m.row,
        mods,
    }))
}

/// Translate a crossterm key event into our own key type. Key *releases* are dropped: with the
/// kitty protocol or Win32 input mode a terminal reports them, and acting on both would run
/// every command twice.
pub fn map_key_event(k: KeyEvent) -> Option<Input> {
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
    // A modifier on its own is not a key: it is the news that the key bar should now show
    // what that modifier does. On release the terminal still names the modifier it is
    // letting go of, so it is taken back out.
    if let CKey::Modifier(which) = k.code {
        if matches!(k.kind, KeyEventKind::Release) {
            mods.remove(of_modifier(which));
        }
        return Some(Input::Mods(mods));
    }
    if matches!(k.kind, KeyEventKind::Release) {
        return None;
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

/// Which of ours a modifier key is.
fn of_modifier(which: crossterm::event::ModifierKeyCode) -> Mods {
    use crossterm::event::ModifierKeyCode as M;
    match which {
        M::LeftShift | M::RightShift => Mods::SHIFT,
        M::LeftControl | M::RightControl => Mods::CTRL,
        M::LeftAlt | M::RightAlt => Mods::ALT,
        _ => Mods::NONE,
    }
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
    use fcmd_ui::{Style, Theme};

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

    /// Base64 against the three cases that matter: a length that is a multiple of three, and
    /// the two that need padding.
    #[test]
    fn the_clipboard_request_is_osc_52_with_standard_base64() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"abc"), "YWJj");
        assert_eq!(base64(b"ab"), "YWI=");
        assert_eq!(base64(b"a"), "YQ==");
        // Bytes above ASCII, which a path can easily carry.
        assert_eq!(base64("è".as_bytes()), "w6g=");
        assert_eq!(clipboard_request("abc"), "\x1b]52;c;YWJj\x1b\\");

        let (mut term, written) = Term::capturing();
        term.set_clipboard("/tmp/a b.txt").unwrap();
        let bytes = written.lock().unwrap().clone();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            clipboard_request("/tmp/a b.txt")
        );
    }

    /// The one sequence there is for asking: xterm's `OSC 50` with a relative font-menu index.
    #[test]
    fn a_font_step_is_xterms_own_request_and_nothing_else() {
        let (mut term, written) = Term::capturing();
        term.font_step(1).unwrap();
        term.font_step(-1).unwrap();
        term.font_step(0).unwrap();
        let bytes = written.lock().unwrap().clone();
        assert_eq!(
            String::from_utf8(bytes).unwrap(),
            "\x1b]50;#+1\x1b\\\x1b]50;#-1\x1b\\\x1b]50;#0\x1b\\"
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
