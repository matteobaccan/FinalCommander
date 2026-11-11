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
    /// The pictures on the screen after the last frame, each with the number kitty knows it
    /// by (zero for the protocols that have no such thing).
    shown: Vec<(Placement, u32)>,
    /// The next number to give kitty for an image.
    next_image: u32,
    /// Set by [`Term::invalidate`]: the screen may no longer hold what `shown` says, so the
    /// next frame takes every picture off and puts it on again.
    forget_pictures: bool,
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
            shown: Vec::new(),
            next_image: 1,
            forget_pictures: false,
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
            shown: Vec::new(),
            next_image: 1,
            forget_pictures: false,
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
            shown: Vec::new(),
            next_image: 1,
            forget_pictures: false,
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
        self.draw_with(buf, cursor_at, &[])
    }

    /// Paint the cells that differ from the previous frame, and the pictures that lie over
    /// them (SPEC 05 §1.14a).
    ///
    /// A picture is not a cell and the diff knows nothing of it, so the two are kept apart
    /// and made to agree by three rules:
    ///
    /// - **The cells under a picture are the caller's, and blank.** They are painted like any
    ///   others, the picture is written after them, and from then on they do not change —
    ///   so the diff never writes over the picture, which is what keeps it on the screen
    ///   without sending it again every frame.
    /// - **A picture that is gone is taken off.** kitty keeps an image until it is told to
    ///   delete it, so it is told; sixel and iTerm2 keep theirs until text is written over
    ///   it, so the cells it covered are forgotten from the previous frame and the diff
    ///   writes every one of them again.
    /// - **A picture whose cells were written is sent again**, where writing a cell erases
    ///   what was drawn in it (sixel, iTerm2): after a resize, after another program had the
    ///   screen, after anything was drawn across it.
    ///
    /// Nothing is ever asked of the terminal and nothing is expected back: kitty is told to
    /// stay quiet (`q=2`), and the other two protocols have no answers to give.
    pub fn draw_with(
        &mut self,
        buf: &Buffer,
        cursor_at: Option<(u16, u16)>,
        pictures: &[Placement],
    ) -> io::Result<()> {
        if !self.raw {
            self.prev = buf.clone();
            return Ok(());
        }
        if self.forget_pictures {
            self.forget_pictures = false;
            if self.shown.iter().any(|(p, _)| p.protocol == Pixels::Kitty) {
                self.out.write_all(KITTY_DELETE_ALL.as_bytes())?;
            }
            self.shown.clear();
        }
        // What was on the screen and is not wanted any more, in this place, as it was.
        let mut kept: Vec<(Placement, u32)> = Vec::new();
        for (old, image) in std::mem::take(&mut self.shown) {
            if pictures.contains(&old) {
                kept.push((old, image));
                continue;
            }
            if old.protocol == Pixels::Kitty {
                self.out.write_all(kitty_delete(image).as_bytes())?;
            }
            // Another picture is about to be drawn over exactly these cells — the next frame
            // of a film — and covers every pixel of this one: writing the cells blank in
            // between would only make it blink.
            if pictures.iter().any(|p| {
                p.protocol == old.protocol
                    && (p.x, p.y, p.cols, p.rows) == (old.x, old.y, old.cols, old.rows)
            }) {
                continue;
            }
            for y in old.y..old.y.saturating_add(old.rows) {
                for x in old.x..old.x.saturating_add(old.cols) {
                    if let Some(cell) = self.prev.cell_mut(x, y) {
                        // A character no frame ever draws, so the diff finds the cell changed.
                        cell.ch = '\0';
                    }
                }
            }
        }
        let mut written = vec![false; pictures.len()];
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
                if cell.style.italic {
                    queue!(self.out, SetAttribute(Attribute::Italic))?;
                }
                if cell.style.underline {
                    queue!(self.out, SetAttribute(Attribute::Underlined))?;
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
            for (i, p) in pictures.iter().enumerate() {
                written[i] |= p.covers(x, y);
            }
        }
        for (i, picture) in pictures.iter().enumerate() {
            let at = kept.iter().position(|(old, _)| old == picture);
            // kitty draws its images on a layer of their own: writing a cell does not touch
            // one, so one already there stays, whatever happened underneath it.
            let stays = match picture.protocol {
                Pixels::Kitty => at.is_some(),
                Pixels::Iterm | Pixels::Sixel => at.is_some() && !written[i],
            };
            if stays {
                self.shown
                    .push(kept[at.expect("it stays, so it was kept")].clone());
                continue;
            }
            if let (Some(at), Pixels::Kitty) = (at, picture.protocol) {
                self.out.write_all(kitty_delete(kept[at].1).as_bytes())?;
            }
            queue!(
                self.out,
                SetAttribute(Attribute::Reset),
                cursor::MoveTo(picture.x, picture.y)
            )?;
            let image = match picture.protocol {
                Pixels::Kitty => {
                    let image = self.next_image;
                    // Numbers are reused after sixteen million images; by then the first is
                    // long deleted.
                    self.next_image = self.next_image % 0x00ff_ffff + 1;
                    self.out.write_all(
                        kitty_image(&picture.data, image, picture.cols, picture.rows).as_bytes(),
                    )?;
                    image
                }
                Pixels::Iterm => {
                    self.out.write_all(
                        iterm_image(&picture.data, picture.cols, picture.rows).as_bytes(),
                    )?;
                    0
                }
                Pixels::Sixel => {
                    self.out.write_all(&picture.data)?;
                    0
                }
            };
            self.shown.push((picture.clone(), image));
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
        self.forget_pictures = true;
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

    /// Give the terminal back to the shell for a program that takes all of it (an editor, a
    /// pager, another commander): the screen as the shell left it, a cursor, no raw mode.
    /// [`Term::take_back`] undoes it.
    pub fn hand_over(&mut self) -> bool {
        let mouse = self.mouse;
        self.restore();
        mouse
    }

    /// Take the terminal again after [`Term::hand_over`], and draw everything afresh: what is
    /// on the screen now is whatever the program left.
    pub fn take_back(&mut self, mouse: bool) -> io::Result<()> {
        terminal::enable_raw_mode()?;
        queue!(
            self.out,
            EnterAlternateScreen,
            EnableBracketedPaste,
            cursor::Hide
        )?;
        if self.enhanced {
            let _ = queue!(
                self.out,
                PushKeyboardEnhancementFlags(
                    KeyboardEnhancementFlags::REPORT_EVENT_TYPES
                        | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES
                )
            );
        }
        self.out.flush()?;
        self.raw = true;
        if mouse {
            let _ = self.set_mouse(true);
        }
        self.invalidate();
        Ok(())
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
        // An image kitty was given is kitty's until it is deleted, and the screen it was on
        // is about to be handed back.
        if self.shown.iter().any(|(p, _)| p.protocol == Pixels::Kitty) {
            let _ = self.out.write_all(KITTY_DELETE_ALL.as_bytes());
        }
        self.shown.clear();
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

/// Standard base64, which is what `OSC 52` carries, and what kitty and iTerm2 take a picture
/// in. Twenty lines rather than a dependency.
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

// ── pictures in real pixels (SPEC 05 §1.14a) ───────────────────────────────────────────────

/// The three ways a terminal takes a picture as pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Pixels {
    /// kitty's graphics protocol: kitty, Ghostty, Konsole. A PNG in an `APC`, kept by the
    /// terminal on a layer of its own until it is deleted.
    Kitty,
    /// iTerm2's inline images: iTerm2, WezTerm, VS Code's terminal. A PNG in `OSC 1337`.
    Iterm,
    /// Sixel: foot, mlterm, contour, `xterm -ti 340`, Windows Terminal from 1.22.
    Sixel,
}

/// One picture on the screen: the cells it covers and what to send for them.
///
/// `data` is the image and nothing else — a PNG for kitty and iTerm2, which is wrapped in
/// their escape sequences here, or a whole sixel string, which is sent as it is. Shared,
/// because it is megabytes and the same bytes are handed over every frame the picture stays.
#[derive(Debug, Clone)]
pub struct Placement {
    pub x: u16,
    pub y: u16,
    pub cols: u16,
    pub rows: u16,
    pub protocol: Pixels,
    /// Which picture this is. Two placements with the same number in the same cells are the
    /// same picture and are not sent twice; the bytes themselves are never compared.
    pub id: u64,
    pub data: std::sync::Arc<Vec<u8>>,
}

/// The same picture in the same place. The number stands for the bytes, which are megabytes
/// and are asked about on every frame.
impl PartialEq for Placement {
    fn eq(&self, other: &Placement) -> bool {
        (self.x, self.y, self.cols, self.rows, self.protocol, self.id)
            == (
                other.x,
                other.y,
                other.cols,
                other.rows,
                other.protocol,
                other.id,
            )
    }
}

impl Placement {
    fn covers(&self, x: u16, y: u16) -> bool {
        x >= self.x
            && x < self.x.saturating_add(self.cols)
            && y >= self.y
            && y < self.y.saturating_add(self.rows)
    }
}

/// Take every image kitty holds off the screen and free it. `q=2`: say nothing back.
const KITTY_DELETE_ALL: &str = "\x1b_Ga=d,d=A,q=2\x1b\\";

/// Take one image off the screen and free it.
pub fn kitty_delete(image: u32) -> String {
    format!("\x1b_Ga=d,d=I,i={image},q=2\x1b\\")
}

/// A PNG as kitty's graphics protocol wants it: transmitted and shown in one go (`a=T`,
/// `f=100`), stretched over `cols × rows` cells from where the cursor is, in pieces of at
/// most 4096 characters of base64 as the protocol requires.
///
/// Two keys are there for our sake and not the picture's. `q=2` tells the terminal to send
/// no answer, neither `OK` nor an error — an answer would arrive as input and be read as
/// keys. `C=1` leaves the cursor where it was, so drawing a picture on the last row cannot
/// scroll the screen.
pub fn kitty_image(png: &[u8], image: u32, cols: u16, rows: u16) -> String {
    let payload = base64(png);
    let mut out = String::with_capacity(payload.len() + payload.len() / 4096 * 16 + 64);
    let mut rest = payload.as_str();
    let mut first = true;
    loop {
        // Base64 is ASCII, so any byte offset is a character boundary.
        let (piece, tail) = rest.split_at(rest.len().min(4096));
        let more = u8::from(!tail.is_empty());
        if first {
            out.push_str(&format!(
                "\x1b_Ga=T,f=100,i={image},c={cols},r={rows},C=1,q=2,m={more};"
            ));
            first = false;
        } else {
            out.push_str(&format!("\x1b_Gm={more};"));
        }
        out.push_str(piece);
        out.push_str("\x1b\\");
        if tail.is_empty() {
            break;
        }
        rest = tail;
    }
    out
}

/// A PNG as iTerm2's inline image: `OSC 1337 ; File = … : base64 BEL`, stretched over
/// `cols × rows` cells from where the cursor is. The shape is not kept by the terminal
/// because it was already settled here: the image is made to be exactly those cells.
pub fn iterm_image(png: &[u8], cols: u16, rows: u16) -> String {
    format!(
        "\x1b]1337;File=inline=1;size={};width={cols};height={rows};preserveAspectRatio=0:{}\x07",
        png.len(),
        base64(png)
    )
}

/// Which protocol the terminal we are in is known to draw, from the environment alone.
///
/// The terminal is never asked. Asking means writing a query and reading the answer off the
/// keyboard's own channel: a terminal that does not answer leaves the program waiting, and
/// one that answers late types its answer into whatever has the focus by then. So this
/// reads what the terminal said about itself before the program started — the variables it
/// exports — and says yes only to the ones known to draw what they are sent. Everything
/// else gets `None`, which is half blocks, which work everywhere; `viewer.pixels` is there
/// for the reader who knows better than the environment does.
///
/// `cell_known` is whether the size of a cell in pixels was learnt from the system. Sixel
/// is drawn pixel for pixel, so without it the picture's size on screen would be a guess,
/// and a wrong guess draws over the rows beside it: sixel is then not chosen.
pub fn pixels_from_env(var: impl Fn(&str) -> Option<String>, cell_known: bool) -> Option<Pixels> {
    let var = |name: &str| var(name).filter(|v| !v.is_empty());
    let term = var("TERM").unwrap_or_default();
    let program = var("TERM_PROGRAM").unwrap_or_default();
    // A multiplexer draws the screen itself and passes on what it understands, which is
    // not this.
    if var("TMUX").is_some()
        || var("STY").is_some()
        || var("ZELLIJ").is_some()
        || term.starts_with("screen")
        || term.starts_with("tmux")
    {
        return None;
    }
    if var("KITTY_WINDOW_ID").is_some() || term == "xterm-kitty" {
        return Some(Pixels::Kitty);
    }
    if term == "xterm-ghostty" || program.eq_ignore_ascii_case("ghostty") {
        return Some(Pixels::Kitty);
    }
    // WezTerm draws all three; iTerm2's is the one it has on without being configured.
    if program == "iTerm.app" || program == "WezTerm" {
        return Some(Pixels::Iterm);
    }
    let sixel = term.starts_with("foot")
        || term == "mlterm"
        || term.starts_with("contour")
        || program.eq_ignore_ascii_case("contour");
    if sixel && cell_known {
        return Some(Pixels::Sixel);
    }
    // Not decided here, on purpose: Konsole draws kitty's images, but not all of the
    // protocol in every version, and which parts cannot be told from a version number; VS
    // Code's terminal draws iTerm2's images only with a setting of its own turned on;
    // Windows Terminal draws sixel from 1.22 and does not say which version it is; xterm
    // draws sixel only when started as a VT340. Each is a value of `viewer.pixels` away.
    None
}

/// What `viewer.pixels` asks for, given what the environment suggested for `auto`.
pub fn pixels_setting(setting: &str, auto: Option<Pixels>) -> Option<Pixels> {
    match setting.trim() {
        "kitty" => Some(Pixels::Kitty),
        "iterm" => Some(Pixels::Iterm),
        "sixel" => Some(Pixels::Sixel),
        "auto" => auto,
        _ => None,
    }
}

/// The size of one cell in pixels, if the system knows it.
///
/// Read from the terminal's window size — an `ioctl` on the terminal's own device, answered
/// by the kernel and never by the terminal — so it cannot hang and nothing comes back on
/// the input. Many terminals fill the pixel fields in and some leave them zero; Windows has
/// no such call. `None` is all of those.
pub fn cell_pixels() -> Option<(u16, u16)> {
    let size = terminal::window_size().ok()?;
    cell_of(size.width, size.height, size.columns, size.rows)
}

/// A window of `width × height` pixels holding `cols × rows` cells: how large one cell is.
fn cell_of(width: u16, height: u16, cols: u16, rows: u16) -> Option<(u16, u16)> {
    if cols == 0 || rows == 0 {
        return None;
    }
    let cell = (width / cols, height / rows);
    // A cell under four pixels is a device that reported something other than pixels.
    (cell.0 >= 4 && cell.1 >= 4).then_some(cell)
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
    read_input_loop_with(tx, &Handover::default())
}

/// Whether the keyboard is ours. While a program has been given the terminal, the reader
/// must not take its keys: it stops reading, says it has stopped, and waits to be given the
/// keyboard back.
#[derive(Clone, Default)]
pub struct Handover {
    away: std::sync::Arc<std::sync::atomic::AtomicBool>,
    stopped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl Handover {
    /// Stop reading the keyboard, and return once the reader has (or after half a second,
    /// when it has not said so: a reader stuck in a read is one that will not take a key the
    /// program gets first anyway).
    pub fn give_away(&self) {
        use std::sync::atomic::Ordering;
        self.away.store(true, Ordering::SeqCst);
        let deadline = std::time::Instant::now() + Duration::from_millis(500);
        while !self.stopped.load(Ordering::SeqCst) && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    /// Read the keyboard again.
    pub fn take_back(&self) {
        self.away.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}

/// [`read_input_loop`], with a way to stop it reading while the terminal is someone else's.
pub fn read_input_loop_with(tx: Sender<Input>, handover: &Handover) {
    use std::sync::atomic::Ordering;
    loop {
        if handover.away.load(Ordering::SeqCst) {
            handover.stopped.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(10));
            continue;
        }
        handover.stopped.store(false, Ordering::SeqCst);
        // Asked with a timeout rather than read outright, so a hand-over is noticed within a
        // moment instead of after the next key — which would be the program's key.
        match event::poll(Duration::from_millis(50)) {
            Ok(false) => continue,
            Ok(true) => {}
            Err(_) => {
                let _ = tx.send(Input::Eof);
                return;
            }
        }
        let event = match event::read() {
            Ok(event) => event,
            Err(_) => {
                let _ = tx.send(Input::Eof);
                return;
            }
        };
        // A key that types something, with more already waiting behind it, may be the first
        // of a paste the terminal delivered as keys (the Windows console does, whatever the
        // terminal on top of it supports). The burst is read whole and, if it is one, sent as
        // the paste it was — so the editor does not indent every pasted line again.
        let event = match event {
            Event::Key(k) if typed(&k).is_some() && pending() => {
                let mut burst = vec![k];
                let mut after = None;
                while pending() {
                    match event::read() {
                        Ok(Event::Key(k)) if matches!(k.kind, KeyEventKind::Release) => {}
                        Ok(Event::Key(k)) if typed(&k).is_some() => burst.push(k),
                        Ok(other) => {
                            after = Some(other);
                            break;
                        }
                        Err(_) => break,
                    }
                }
                match pasted(&burst) {
                    Some(text) => {
                        if tx.send(Input::Paste(text)).is_err() {
                            return;
                        }
                    }
                    None => {
                        for k in burst {
                            if !forward(&tx, Event::Key(k)) {
                                return;
                            }
                        }
                    }
                }
                match after {
                    Some(event) => event,
                    None => continue,
                }
            }
            other => other,
        };
        if !forward(&tx, event) {
            return;
        }
    }
}

/// Is another event already waiting? Asked with no timeout: a person typing never has one.
fn pending() -> bool {
    event::poll(Duration::ZERO).unwrap_or(false)
}

/// Send one event on as input; false once nobody is listening.
fn forward(tx: &Sender<Input>, event: Event) -> bool {
    let input = match event {
        Event::Key(k) => map_key_event(k),
        Event::Resize(w, h) => Some(Input::Resize(w, h)),
        Event::Paste(s) => Some(Input::Paste(s)),
        Event::Mouse(m) => map_mouse_event(m),
        _ => None,
    };
    match input {
        Some(input) => tx.send(input).is_ok(),
        None => true,
    }
}

/// The character a key press types into text, if it types one: a character with no Ctrl or
/// Alt, Enter as a line break, Tab as a tab.
fn typed(k: &KeyEvent) -> Option<char> {
    if !matches!(k.kind, KeyEventKind::Press | KeyEventKind::Repeat)
        || k.modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
    {
        return None;
    }
    match k.code {
        CKey::Char(c) => Some(c),
        CKey::Enter => Some('\n'),
        CKey::Tab => Some('\t'),
        _ => None,
    }
}

/// The text of a burst of typed keys, when the burst is a paste: more than one line, with
/// something on them. A key held down repeats one character, and Enter held down repeats
/// line breaks alone; neither is a paste, and both stay keys.
fn pasted(burst: &[KeyEvent]) -> Option<String> {
    let text: String = burst.iter().filter_map(typed).collect();
    let lines = text.contains('\n');
    let words = text.chars().any(|c| !c.is_whitespace());
    (lines && words && burst.len() > 2).then_some(text)
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

    /// A paste the console delivered as keys is known by its shape: several lines with text
    /// on them. What a hand does — a key held down, Enter held down, a shortcut — is not.
    #[test]
    fn a_burst_of_typed_lines_is_a_paste_and_a_held_key_is_not() {
        let keys = |text: &str| -> Vec<KeyEvent> {
            text.chars()
                .map(|c| match c {
                    '\n' => KeyEvent::new(CKey::Enter, KeyModifiers::NONE),
                    '\t' => KeyEvent::new(CKey::Tab, KeyModifiers::NONE),
                    c => KeyEvent::new(CKey::Char(c), KeyModifiers::NONE),
                })
                .collect()
        };
        assert_eq!(
            pasted(&keys("- one\n  two\n")).as_deref(),
            Some("- one\n  two\n"),
            "the lines come as they are, indentation and all"
        );
        assert_eq!(pasted(&keys("\tx\ny")).as_deref(), Some("\tx\ny"));
        assert_eq!(pasted(&keys("aaaaaaa")), None, "a letter held down");
        assert_eq!(pasted(&keys("\n\n\n\n")), None, "Enter held down");
        assert_eq!(
            pasted(&keys("a\n")),
            None,
            "too short to be anything but typing"
        );
        // A shortcut is not text, and a released key types nothing.
        let ctrl = KeyEvent::new(CKey::Char('v'), KeyModifiers::CONTROL);
        assert_eq!(typed(&ctrl), None);
        let mut up = KeyEvent::new(CKey::Char('a'), KeyModifiers::NONE);
        up.kind = KeyEventKind::Release;
        assert_eq!(typed(&up), None);
        let shifted = KeyEvent::new(CKey::Char('A'), KeyModifiers::SHIFT);
        assert_eq!(typed(&shifted), Some('A'));
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

    // ── pictures in real pixels ────────────────────────────────────────────────────────────

    fn env(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let pairs: Vec<(String, String)> = pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
        }
    }

    #[test]
    fn the_terminal_is_recognised_by_what_it_exports_and_never_asked() {
        type Case<'a> = (&'a [(&'a str, &'a str)], Option<Pixels>);
        let table: &[Case] = &[
            (&[("TERM", "xterm-kitty")], Some(Pixels::Kitty)),
            (
                &[("TERM", "xterm-256color"), ("KITTY_WINDOW_ID", "3")],
                Some(Pixels::Kitty),
            ),
            (&[("TERM", "xterm-ghostty")], Some(Pixels::Kitty)),
            (
                &[("TERM", "xterm-256color"), ("TERM_PROGRAM", "ghostty")],
                Some(Pixels::Kitty),
            ),
            (
                &[("TERM", "xterm-256color"), ("TERM_PROGRAM", "iTerm.app")],
                Some(Pixels::Iterm),
            ),
            (
                &[("TERM", "xterm-256color"), ("TERM_PROGRAM", "WezTerm")],
                Some(Pixels::Iterm),
            ),
            (&[("TERM", "foot")], Some(Pixels::Sixel)),
            (&[("TERM", "foot-extra")], Some(Pixels::Sixel)),
            (&[("TERM", "mlterm")], Some(Pixels::Sixel)),
            (&[("TERM", "contour")], Some(Pixels::Sixel)),
            // The ones that cannot be told from here stay on half blocks: Konsole (how much
            // of kitty's protocol it draws depends on more than its version), VS Code (its
            // images are a setting of its own), Windows Terminal (no version), a plain
            // xterm, the Linux console, and nothing at all.
            (
                &[("TERM", "xterm-256color"), ("KONSOLE_VERSION", "230804")],
                None,
            ),
            (
                &[("TERM", "xterm-256color"), ("TERM_PROGRAM", "vscode")],
                None,
            ),
            (&[("TERM", "xterm-256color"), ("WT_SESSION", "abc")], None),
            (&[("TERM", "xterm-256color")], None),
            (&[("TERM", "linux")], None),
            (&[], None),
            // An empty variable is an unset one.
            (&[("TERM", "xterm-256color"), ("KITTY_WINDOW_ID", "")], None),
        ];
        for (vars, want) in table {
            assert_eq!(pixels_from_env(env(vars), true), *want, "{vars:?}");
        }
    }

    #[test]
    fn a_multiplexer_in_between_means_half_blocks() {
        for vars in [
            &[("TERM", "xterm-kitty"), ("TMUX", "/tmp/tmux-0/default,1,0")][..],
            &[("TERM", "screen-256color"), ("KITTY_WINDOW_ID", "1")],
            &[("TERM", "tmux-256color"), ("TERM_PROGRAM", "iTerm.app")],
            &[("TERM", "foot"), ("STY", "1234.pts-0.host")],
            &[("TERM", "xterm-kitty"), ("ZELLIJ", "0")],
        ] {
            assert_eq!(pixels_from_env(env(vars), true), None, "{vars:?}");
        }
    }

    #[test]
    fn sixel_is_not_chosen_without_knowing_how_large_a_cell_is() {
        assert_eq!(pixels_from_env(env(&[("TERM", "foot")]), false), None);
        // The two that are told how many cells to cover do not need it.
        assert_eq!(
            pixels_from_env(env(&[("TERM", "xterm-kitty")]), false),
            Some(Pixels::Kitty)
        );
        assert_eq!(cell_of(1280, 800, 160, 50), Some((8, 16)));
        assert_eq!(
            cell_of(0, 0, 80, 24),
            None,
            "a terminal that left them zero"
        );
        assert_eq!(cell_of(1280, 800, 0, 0), None);
    }

    #[test]
    fn the_setting_overrides_what_the_environment_suggests() {
        assert_eq!(
            pixels_setting("auto", Some(Pixels::Kitty)),
            Some(Pixels::Kitty)
        );
        assert_eq!(pixels_setting("auto", None), None);
        assert_eq!(pixels_setting("off", Some(Pixels::Kitty)), None);
        assert_eq!(pixels_setting("sixel", None), Some(Pixels::Sixel));
        assert_eq!(
            pixels_setting("iterm", Some(Pixels::Kitty)),
            Some(Pixels::Iterm)
        );
        assert_eq!(pixels_setting(" kitty ", None), Some(Pixels::Kitty));
        // A word it does not know is half blocks, which cannot go wrong.
        assert_eq!(pixels_setting("pixels please", Some(Pixels::Kitty)), None);
    }

    #[test]
    fn a_small_picture_is_one_kitty_command_that_asks_for_no_answer() {
        // Three bytes are four characters of base64 and no padding.
        assert_eq!(
            kitty_image(b"abc", 7, 10, 4),
            "\x1b_Ga=T,f=100,i=7,c=10,r=4,C=1,q=2,m=0;YWJj\x1b\\"
        );
        assert_eq!(kitty_delete(7), "\x1b_Ga=d,d=I,i=7,q=2\x1b\\");
    }

    #[test]
    fn a_large_picture_goes_to_kitty_in_pieces_of_4096() {
        // 6 000 bytes are 8 000 characters: one whole piece and the rest.
        let sent = kitty_image(&[0u8; 6000], 1, 80, 24);
        let pieces: Vec<&str> = sent.split_terminator("\x1b\\").collect();
        assert_eq!(pieces.len(), 2);
        let (head, first) = pieces[0].split_once(';').unwrap();
        assert_eq!(head, "\x1b_Ga=T,f=100,i=1,c=80,r=24,C=1,q=2,m=1");
        assert_eq!(first.len(), 4096);
        let (head, last) = pieces[1].split_once(';').unwrap();
        assert_eq!(
            head, "\x1b_Gm=0",
            "only the first piece says what the image is"
        );
        assert_eq!(last.len(), 8000 - 4096);
        assert!(sent.ends_with("\x1b\\"));
    }

    #[test]
    fn a_picture_for_iterm_says_its_size_in_bytes_and_in_cells() {
        assert_eq!(
            iterm_image(b"abcd", 12, 5),
            "\x1b]1337;File=inline=1;size=4;width=12;height=5;preserveAspectRatio=0:YWJjZA==\x07"
        );
    }

    fn placement(protocol: Pixels, id: u64, data: &[u8]) -> Placement {
        Placement {
            x: 2,
            y: 1,
            cols: 4,
            rows: 2,
            protocol,
            id,
            data: std::sync::Arc::new(data.to_vec()),
        }
    }

    fn blank(w: u16, h: u16) -> Buffer {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(w, h);
        buf.clear(Style::new(theme.color("panel.fg"), theme.color("panel.bg")));
        buf
    }

    fn text_of(sink: &std::sync::Arc<std::sync::Mutex<Vec<u8>>>) -> String {
        let text = String::from_utf8_lossy(&sink.lock().unwrap()).into_owned();
        sink.lock().unwrap().clear();
        text
    }

    #[test]
    fn a_picture_is_written_after_the_cells_and_not_again_while_nothing_changes() {
        let (mut term, sink) = Term::capturing();
        let buf = blank(10, 4);
        let sixel = placement(Pixels::Sixel, 1, b"\x1bPq#0~~\x1b\\");
        term.draw_with(&buf, None, std::slice::from_ref(&sixel))
            .unwrap();
        let first = text_of(&sink);
        // The cursor is put on the picture's corner (rows and columns count from one on the
        // wire), then the picture, and it comes after the last cell.
        let at = first
            .find("\x1b[2;3H\x1bPq#0~~\x1b\\")
            .expect("the picture");
        assert!(at > first.rfind(' ').expect("the cells"), "{first:?}");

        // The same frame again: no cell changed, so neither is the picture sent.
        term.draw_with(&buf, None, std::slice::from_ref(&sixel))
            .unwrap();
        assert!(!text_of(&sink).contains("\x1bPq"));

        // A cell beside it changes: still not sent. A cell under it: sent again, because
        // writing a cell erased that part of it.
        let mut beside = buf.clone();
        beside.set(8, 3, 'x', Style::default());
        term.draw_with(&beside, None, std::slice::from_ref(&sixel))
            .unwrap();
        assert!(!text_of(&sink).contains("\x1bPq"));
        let mut under = beside.clone();
        under.set(3, 2, 'y', Style::default());
        term.draw_with(&under, None, &[sixel]).unwrap();
        assert!(text_of(&sink).contains("\x1bPq"));
    }

    #[test]
    fn a_picture_that_is_gone_has_its_cells_written_again() {
        let (mut term, sink) = Term::capturing();
        let buf = blank(10, 4);
        term.draw_with(&buf, None, &[placement(Pixels::Iterm, 1, b"png")])
            .unwrap();
        assert!(text_of(&sink).contains("\x1b]1337;File=inline=1;size=3;width=4;height=2;"));
        // The same cells, no picture: nothing differs in the buffer, and yet the eight cells
        // it covered are written, because that is the only thing that takes it off.
        term.draw_with(&buf, None, &[]).unwrap();
        let after = text_of(&sink);
        assert_eq!(after.matches(' ').count(), 8, "{after:?}");
        assert!(after.contains("\x1b[2;3H") && after.contains("\x1b[3;3H"));
        // And once is enough.
        term.draw_with(&buf, None, &[]).unwrap();
        assert_eq!(text_of(&sink).matches(' ').count(), 0);
    }

    #[test]
    fn kitty_is_told_to_delete_an_image_and_keeps_one_through_a_cell_being_written() {
        let (mut term, sink) = Term::capturing();
        let buf = blank(10, 4);
        let one = placement(Pixels::Kitty, 1, b"abc");
        term.draw_with(&buf, None, std::slice::from_ref(&one))
            .unwrap();
        assert!(text_of(&sink).contains("\x1b_Ga=T,f=100,i=1,c=4,r=2,C=1,q=2,m=0;YWJj"));

        // kitty's image is on a layer of its own: a cell written under it changes nothing.
        let mut under = buf.clone();
        under.set(3, 2, 'y', Style::default());
        term.draw_with(&under, None, &[one]).unwrap();
        assert!(!text_of(&sink).contains("\x1b_G"));

        // Another picture in the same cells — the next frame of a film: the old one is
        // deleted by its number, the new one gets the next, and the cells in between are
        // not written blank.
        let two = placement(Pixels::Kitty, 2, b"abd");
        term.draw_with(&under, None, &[two]).unwrap();
        let swap = text_of(&sink);
        assert!(swap.contains("\x1b_Ga=d,d=I,i=1,q=2\x1b\\"), "{swap:?}");
        assert!(swap.contains("\x1b_Ga=T,f=100,i=2,"), "{swap:?}");
        assert_eq!(swap.matches(' ').count(), 0, "{swap:?}");

        // No picture at all: deleted, and the cells written.
        term.draw_with(&under, None, &[]).unwrap();
        let gone = text_of(&sink);
        assert!(gone.contains("\x1b_Ga=d,d=I,i=2,q=2\x1b\\"));
        assert_eq!(
            gone.matches(' ').count(),
            7,
            "all but the one with a letter in it"
        );
    }

    #[test]
    fn after_the_screen_was_someone_elses_every_picture_is_sent_again() {
        let (mut term, sink) = Term::capturing();
        let buf = blank(10, 4);
        let one = placement(Pixels::Kitty, 1, b"abc");
        term.draw_with(&buf, None, std::slice::from_ref(&one))
            .unwrap();
        text_of(&sink);
        // A resize, or a program that ran on this screen: what is on it is not known.
        term.invalidate();
        term.draw_with(&buf, None, &[one]).unwrap();
        let again = text_of(&sink);
        assert!(again.starts_with("\x1b_Ga=d,d=A,q=2\x1b\\"), "{again:?}");
        assert!(again.contains("\x1b_Ga=T,f=100,i=2,"));
        // Leaving the terminal takes the images with it.
        term.restore();
        assert!(text_of(&sink).contains("\x1b_Ga=d,d=A,q=2\x1b\\"));
    }

    #[test]
    fn a_frame_without_pictures_writes_no_graphics_at_all() {
        let (mut term, sink) = Term::capturing();
        term.draw(&blank(10, 4), None).unwrap();
        term.invalidate();
        term.draw(&blank(10, 4), None).unwrap();
        let text = text_of(&sink);
        assert!(!text.contains("\x1b_G") && !text.contains("\x1bP") && !text.contains("1337"));
    }
}
