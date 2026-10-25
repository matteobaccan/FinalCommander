//! What a terminal's output means, as a grid of cells.
//!
//! A program writing to a terminal does not write a screen; it writes a stream of characters
//! with escape sequences in it that *move* around a screen. This is the thing that keeps that
//! screen: the parser from Alacritty ([`vte`]) recognises the sequences, and the rest of this
//! file is what each one does to the grid.
//!
//! The grid is a [`fcmd_ui::Buffer`], the same type the panels and the viewer draw into, so a
//! terminal's screen is copied onto ours rather than translated: a cell is a character, a
//! foreground, a background, bold and reverse, and that is as true of `htop` as it is of a
//! panel. Drawing it is therefore a copy, and the colours a program asks for are the colours
//! the theme engine already knows how to send.
//!
//! **What is implemented, and why that list and not another.** Everything a shell, a pager, an
//! editor and a full-screen monitor need, taken from what they actually emit: cursor movement
//! absolute and relative, erase in display and in line, insert and delete characters and
//! lines, scroll up and down with a scrolling region, the SGR attributes including 256-colour
//! and 24-bit colour, save and restore the cursor, the alternate screen, autowrap, and the
//! title. What is not here is refused by being ignored rather than by printing its bytes:
//! a sequence nobody understands must never end up on the screen as text, because that is how
//! a terminal emulator turns one unsupported feature into an unreadable screen.

use std::collections::VecDeque;

use fcmd_ui::{Buffer, Cell, Color, Rect, Style};

/// How many lines of what has scrolled off the top are kept.
pub const SCROLLBACK: usize = 5_000;

/// The screen a program on the other end of a pseudo-terminal is writing.
pub struct Screen {
    /// The grid as it is now. On the alternate screen this is the alternate one.
    grid: Buffer,
    /// The ordinary screen, kept while the alternate one is in use.
    saved_grid: Option<Buffer>,
    /// Lines that have scrolled off the top, newest last.
    history: VecDeque<Vec<Cell>>,
    /// Where the cursor is, in cells.
    cursor: (u16, u16),
    /// `DECSC`'s copy of it, with the pen.
    saved_cursor: Option<((u16, u16), Style)>,
    /// What is written with now.
    pen: Style,
    /// The rows a scroll moves, as `(top, bottom)` inclusive. `DECSTBM`.
    region: (u16, u16),
    /// Does a character at the right edge wrap to the next line? `DECAWM`, on by default.
    autowrap: bool,
    /// Set once the last column has been written and the next character wraps. A terminal
    /// does not wrap when the last column is *reached*, only when something else arrives —
    /// which is why a line exactly as wide as the screen does not leave a blank line after it.
    pending_wrap: bool,
    /// Is the cursor to be drawn? `DECTCEM`.
    pub cursor_visible: bool,
    /// What the program called itself, from `OSC 0` or `OSC 2`.
    pub title: Option<String>,
    /// Set when the program rang the bell, for a caller that wants to say so.
    pub bell: bool,
    /// Has anything changed since the caller last looked?
    pub dirty: bool,
}

impl std::fmt::Debug for Screen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Screen")
            .field("size", &(self.grid.width(), self.grid.height()))
            .field("cursor", &self.cursor)
            .field("alternate", &self.saved_grid.is_some())
            .field("history", &self.history.len())
            .finish_non_exhaustive()
    }
}

impl Screen {
    pub fn new(cols: u16, rows: u16) -> Screen {
        let (cols, rows) = (cols.max(1), rows.max(1));
        Screen {
            grid: blank(cols, rows),
            saved_grid: None,
            history: VecDeque::new(),
            cursor: (0, 0),
            saved_cursor: None,
            pen: Style::default(),
            region: (0, rows - 1),
            autowrap: true,
            pending_wrap: false,
            cursor_visible: true,
            title: None,
            bell: false,
            dirty: true,
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.grid.width(), self.grid.height())
    }

    /// The grid as it is now, for drawing.
    pub fn grid(&self) -> &Buffer {
        &self.grid
    }

    /// Where the cursor is, if it is to be drawn.
    pub fn cursor(&self) -> Option<(u16, u16)> {
        self.cursor_visible.then_some(self.cursor)
    }

    /// How many lines have scrolled off the top.
    pub fn history_len(&self) -> usize {
        self.history.len()
    }

    /// One line of history, oldest first, as text.
    pub fn history_line(&self, at: usize) -> Option<String> {
        self.history.get(at).map(|row| row_text(row))
    }

    /// A row of the screen as text, for a test or a search.
    pub fn line(&self, row: u16) -> String {
        let mut out = String::new();
        for x in 0..self.grid.width() {
            out.push(self.grid.get(x, row).map(|c| c.ch).unwrap_or(' '));
        }
        out.trim_end().to_string()
    }

    /// Every row of the screen as text.
    pub fn text(&self) -> String {
        (0..self.grid.height())
            .map(|y| self.line(y))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// A new size.
    ///
    /// The grid keeps what fits at the top left, which is what every terminal does and the
    /// only thing that can be done without re-running the program: the bytes that drew the
    /// old screen are gone, and only the program knows how to draw the new one. A program
    /// that asked to be told (`SIGWINCH`) is told by the pseudo-terminal, not by this.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let (cols, rows) = (cols.max(1), rows.max(1));
        if (cols, rows) == self.size() {
            return;
        }
        self.grid = reshape(&self.grid, cols, rows);
        if let Some(saved) = self.saved_grid.take() {
            self.saved_grid = Some(reshape(&saved, cols, rows));
        }
        self.region = (0, rows - 1);
        self.cursor = (self.cursor.0.min(cols - 1), self.cursor.1.min(rows - 1));
        self.pending_wrap = false;
        self.dirty = true;
    }

    /// Clear the screen, the history and the modes: what a caller does before reusing one.
    pub fn reset(&mut self) {
        let (cols, rows) = self.size();
        *self = Screen::new(cols, rows);
    }

    // ── the grid ───────────────────────────────────────────────────────────────────────────

    fn blank_cell(&self) -> Cell {
        // Erased cells take the *background* of the pen and nothing else: a program that set
        // a colour and cleared the screen means that colour, and one that set bold does not
        // mean bold spaces.
        Cell {
            ch: ' ',
            style: Style::new(Color::Default, self.pen.bg),
            wide_tail: false,
        }
    }

    fn put(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.grid.width() && y < self.grid.height() {
            self.grid.set(x, y, cell.ch, cell.style);
        }
    }

    fn erase(&mut self, rect: Rect) {
        let blank = self.blank_cell();
        self.grid.fill(rect, blank.ch, blank.style);
    }

    /// Move everything in the scrolling region up by `n`, keeping what leaves the top.
    fn scroll_up(&mut self, n: u16) {
        let (top, bottom) = self.region;
        let n = n.min(bottom - top + 1);
        let whole_screen = top == 0 && bottom == self.grid.height() - 1;
        for _ in 0..n {
            // A line that leaves the top of a full-height screen is history; one that leaves
            // a scrolling region is not — the region is a window a program is drawing in, and
            // keeping its lines would fill the history with a progress bar's old frames.
            if whole_screen && self.saved_grid.is_none() {
                let row: Vec<Cell> = (0..self.grid.width())
                    .map(|x| self.grid.get(x, top).cloned().unwrap_or_default())
                    .collect();
                self.history.push_back(row);
                while self.history.len() > SCROLLBACK {
                    self.history.pop_front();
                }
            }
            for y in top..bottom {
                for x in 0..self.grid.width() {
                    let cell = self.grid.get(x, y + 1).cloned().unwrap_or_default();
                    self.grid.set(x, y, cell.ch, cell.style);
                }
            }
            self.erase(Rect::new(0, bottom, self.grid.width(), 1));
        }
    }

    /// The other way: everything down by `n`, the bottom lines falling off.
    fn scroll_down(&mut self, n: u16) {
        let (top, bottom) = self.region;
        let n = n.min(bottom - top + 1);
        for _ in 0..n {
            let mut y = bottom;
            while y > top {
                for x in 0..self.grid.width() {
                    let cell = self.grid.get(x, y - 1).cloned().unwrap_or_default();
                    self.grid.set(x, y, cell.ch, cell.style);
                }
                y -= 1;
            }
            self.erase(Rect::new(0, top, self.grid.width(), 1));
        }
    }

    fn newline(&mut self) {
        let (_, bottom) = self.region;
        if self.cursor.1 == bottom {
            self.scroll_up(1);
        } else if self.cursor.1 + 1 < self.grid.height() {
            self.cursor.1 += 1;
        }
        self.pending_wrap = false;
    }

    fn goto(&mut self, x: u16, y: u16) {
        let (cols, rows) = self.size();
        self.cursor = (x.min(cols - 1), y.min(rows - 1));
        self.pending_wrap = false;
    }

    /// Print one character where the cursor is, wrapping if it has to.
    fn print(&mut self, ch: char) {
        let width = unicode_width(ch);
        if width == 0 {
            // A combining mark. Joining it to the cell before would need a string per cell,
            // which the grid has not got; dropping it keeps the columns in line, and the
            // columns are what a terminal is for.
            return;
        }
        let cols = self.grid.width();
        if self.pending_wrap && self.autowrap {
            self.cursor.0 = 0;
            self.newline();
        }
        // A double-width character at the very last column does not fit: it goes to the next
        // line whole rather than being cut in half.
        if width == 2 && self.cursor.0 + 1 >= cols && self.autowrap {
            self.erase(Rect::new(self.cursor.0, self.cursor.1, 1, 1));
            self.cursor.0 = 0;
            self.newline();
        }
        let (x, y) = self.cursor;
        let pen = self.pen;
        self.put(
            x,
            y,
            Cell {
                ch,
                style: pen,
                wide_tail: false,
            },
        );
        if width == 2 && x + 1 < cols {
            // The second half carries no character of its own: the backend must not emit
            // anything for it, or every wide glyph would be drawn twice.
            self.grid.set(x + 1, y, ' ', pen);
            if let Some(cell) = self.grid.cell_mut(x + 1, y) {
                cell.wide_tail = true;
            }
        }
        let advance = width as u16;
        if x + advance < cols {
            self.cursor.0 = x + advance;
        } else {
            self.cursor.0 = cols - 1;
            self.pending_wrap = true;
        }
        self.dirty = true;
    }

    // ── the sequences ──────────────────────────────────────────────────────────────────────

    /// `SGR`: what to write with from here on.
    fn sgr(&mut self, params: &[Vec<u16>]) {
        if params.is_empty() {
            self.pen = Style::default();
            return;
        }
        let mut i = 0;
        let flat: Vec<&[u16]> = params.iter().map(|p| p.as_slice()).collect();
        while i < flat.len() {
            let group = flat[i];
            let code = group.first().copied().unwrap_or(0);
            match code {
                0 => self.pen = Style::default(),
                1 => self.pen.bold = true,
                7 => self.pen.reverse = true,
                // Faint and italic have no cell of their own here; dim is drawn by the
                // colour's own `dimmed`, and italic by nothing, so they are left alone
                // rather than approximated by bold's opposite.
                2 | 3 | 4 | 5 | 8 | 9 => {}
                21 | 22 => self.pen.bold = false,
                27 => self.pen.reverse = false,
                23 | 24 | 25 | 28 | 29 => {}
                30..=37 => self.pen.fg = Color::Ansi((code - 30) as u8),
                38 => {
                    if let Some(colour) = extended(group, &flat, &mut i) {
                        self.pen.fg = colour;
                    }
                }
                39 => self.pen.fg = Color::Default,
                40..=47 => self.pen.bg = Color::Ansi((code - 40) as u8),
                48 => {
                    if let Some(colour) = extended(group, &flat, &mut i) {
                        self.pen.bg = colour;
                    }
                }
                49 => self.pen.bg = Color::Default,
                // The bright sixteen, which `ls` and every prompt use.
                90..=97 => self.pen.fg = Color::Ansi((code - 90 + 8) as u8),
                100..=107 => self.pen.bg = Color::Ansi((code - 100 + 8) as u8),
                _ => {}
            }
            i += 1;
        }
    }

    /// `SM` / `RM`, the private ones (`?`): the modes a program turns on and off.
    fn mode(&mut self, params: &[Vec<u16>], on: bool) {
        for group in params {
            match group.first().copied().unwrap_or(0) {
                7 => self.autowrap = on,
                25 => self.cursor_visible = on,
                // The alternate screen, in its three spellings. 1049 is the one everything
                // modern sends, and it saves the cursor as well as the screen.
                47 | 1047 | 1049 => self.alternate(on, true),
                1048 => {
                    if on {
                        self.saved_cursor = Some((self.cursor, self.pen));
                    } else if let Some((at, pen)) = self.saved_cursor.take() {
                        self.cursor = at;
                        self.pen = pen;
                    }
                }
                _ => {}
            }
        }
    }

    /// Switch to the alternate screen, or back.
    ///
    /// This is what makes `vim` and `htop` work and what makes them leave the shell's output
    /// alone when they exit: the alternate screen is a second grid with no history of its
    /// own, and leaving it puts the first one back exactly as it was.
    fn alternate(&mut self, on: bool, with_cursor: bool) {
        if on == self.saved_grid.is_some() {
            return;
        }
        let (cols, rows) = self.size();
        if on {
            if with_cursor {
                self.saved_cursor = Some((self.cursor, self.pen));
            }
            let ordinary = std::mem::replace(&mut self.grid, blank(cols, rows));
            self.saved_grid = Some(ordinary);
            self.cursor = (0, 0);
            self.region = (0, rows - 1);
        } else if let Some(ordinary) = self.saved_grid.take() {
            self.grid = ordinary;
            self.region = (0, rows - 1);
            if with_cursor {
                if let Some((at, pen)) = self.saved_cursor.take() {
                    self.cursor = at;
                    self.pen = pen;
                }
            }
        }
        self.pending_wrap = false;
        self.dirty = true;
    }
}

/// `38;5;n` and `38;2;r;g;b`, in both the colon and the semicolon spelling.
///
/// A terminal receives these two ways round: `38:5:n` arrives as one parameter group with
/// sub-parameters, and `38;5;n` as three groups. Both mean the same colour, and a parser that
/// handled only one of them would lose the colour from half the programs in use.
fn extended(group: &[u16], flat: &[&[u16]], i: &mut usize) -> Option<Color> {
    if group.len() > 1 {
        return match group[1] {
            5 => group.get(2).map(|n| Color::Indexed(*n as u8)),
            2 => match (group.get(2), group.get(3), group.get(4)) {
                (Some(r), Some(g), Some(b)) => Some(Color::Rgb(*r as u8, *g as u8, *b as u8)),
                _ => None,
            },
            _ => None,
        };
    }
    let kind = flat.get(*i + 1)?.first().copied()?;
    match kind {
        5 => {
            let n = flat.get(*i + 2)?.first().copied()?;
            *i += 2;
            Some(Color::Indexed(n as u8))
        }
        2 => {
            let r = flat.get(*i + 2)?.first().copied()?;
            let g = flat.get(*i + 3)?.first().copied()?;
            let b = flat.get(*i + 4)?.first().copied()?;
            *i += 4;
            Some(Color::Rgb(r as u8, g as u8, b as u8))
        }
        _ => None,
    }
}

/// The same grid at a new size, keeping what fits at the top left.
///
/// `Buffer::resize` starts again, which is right for a screen that is about to be drawn from
/// scratch and wrong here: the bytes that drew this screen are gone, and only the program on
/// the other end knows how to draw it again. What fits is therefore kept, which is what every
/// terminal does, and the program is told the new size and redraws if it cares.
fn reshape(old: &Buffer, cols: u16, rows: u16) -> Buffer {
    let mut out = blank(cols, rows);
    for y in 0..rows.min(old.height()) {
        for x in 0..cols.min(old.width()) {
            if let Some(cell) = old.get(x, y) {
                out.set(x, y, cell.ch, cell.style);
                if cell.wide_tail {
                    if let Some(target) = out.cell_mut(x, y) {
                        target.wide_tail = true;
                    }
                }
            }
        }
    }
    out
}

fn blank(cols: u16, rows: u16) -> Buffer {
    let mut buffer = Buffer::new(cols, rows);
    buffer.fill(Rect::new(0, 0, cols, rows), ' ', Style::default());
    buffer
}

fn row_text(row: &[Cell]) -> String {
    row.iter()
        .map(|c| c.ch)
        .collect::<String>()
        .trim_end()
        .to_string()
}

/// The same table the panels measure names with, so a double-width name takes two columns in
/// a listing and two in a terminal.
fn unicode_width(ch: char) -> usize {
    fcmd_ui::char_cols(ch)
}

impl vte::Perform for Screen {
    fn print(&mut self, ch: char) {
        Screen::print(self, ch);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            0x07 => self.bell = true,
            // Backspace moves; it does not erase. What erases is whatever the program writes
            // next, which for a shell deleting a character is a space.
            0x08 => {
                if self.cursor.0 > 0 {
                    self.cursor.0 -= 1;
                }
                self.pending_wrap = false;
            }
            0x09 => {
                let cols = self.grid.width();
                let next = (self.cursor.0 / 8 + 1) * 8;
                self.cursor.0 = next.min(cols - 1);
                self.pending_wrap = false;
            }
            // Line feed, vertical tab and form feed all move down a line here, which is
            // what every terminal does with the last two.
            0x0a..=0x0c => self.newline(),
            0x0d => {
                self.cursor.0 = 0;
                self.pending_wrap = false;
            }
            _ => {}
        }
        self.dirty = true;
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        // `OSC 0` and `OSC 2` are the title. Everything else — the clipboard, a colour query,
        // a hyperlink — is left alone rather than answered, because answering means writing
        // back to the program and this type only reads.
        let Some(kind) = params.first() else {
            return;
        };
        if matches!(*kind, b"0" | b"2") {
            if let Some(text) = params.get(1) {
                self.title = Some(String::from_utf8_lossy(text).into_owned());
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &vte::Params,
        intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        let groups: Vec<Vec<u16>> = params.iter().map(|p| p.to_vec()).collect();
        let first = |n: usize, default: u16| -> u16 {
            groups
                .get(n)
                .and_then(|g| g.first().copied())
                .filter(|v| *v != 0)
                .unwrap_or(default)
        };
        let private = intermediates.first() == Some(&b'?');
        let (cols, rows) = self.size();

        match action {
            'A' => {
                let by = first(0, 1);
                self.cursor.1 = self.cursor.1.saturating_sub(by);
                self.pending_wrap = false;
            }
            'B' | 'e' => {
                let by = first(0, 1);
                self.cursor.1 = (self.cursor.1 + by).min(rows - 1);
                self.pending_wrap = false;
            }
            'C' | 'a' => {
                let by = first(0, 1);
                self.cursor.0 = (self.cursor.0 + by).min(cols - 1);
                self.pending_wrap = false;
            }
            'D' => {
                let by = first(0, 1);
                self.cursor.0 = self.cursor.0.saturating_sub(by);
                self.pending_wrap = false;
            }
            // Next and previous line: down or up, and to the first column.
            'E' => {
                let by = first(0, 1);
                self.cursor = (0, (self.cursor.1 + by).min(rows - 1));
                self.pending_wrap = false;
            }
            'F' => {
                let by = first(0, 1);
                self.cursor = (0, self.cursor.1.saturating_sub(by));
                self.pending_wrap = false;
            }
            'G' | '`' => {
                let col = first(0, 1);
                self.cursor.0 = (col - 1).min(cols - 1);
                self.pending_wrap = false;
            }
            'd' => {
                let row = first(0, 1);
                self.cursor.1 = (row - 1).min(rows - 1);
                self.pending_wrap = false;
            }
            'H' | 'f' => {
                let (row, col) = (first(0, 1), first(1, 1));
                self.goto(col - 1, row - 1);
            }
            'J' => match groups.first().and_then(|g| g.first().copied()).unwrap_or(0) {
                1 => {
                    // To the start of the screen, the cursor's cell included.
                    let (x, y) = self.cursor;
                    if y > 0 {
                        self.erase(Rect::new(0, 0, cols, y));
                    }
                    self.erase(Rect::new(0, y, x + 1, 1));
                }
                2 | 3 => {
                    self.erase(Rect::new(0, 0, cols, rows));
                    // `3` also clears the history, which is what `clear` sends.
                    if groups.first().and_then(|g| g.first().copied()) == Some(3) {
                        self.history.clear();
                    }
                }
                _ => {
                    let (x, y) = self.cursor;
                    self.erase(Rect::new(x, y, cols - x, 1));
                    if y + 1 < rows {
                        self.erase(Rect::new(0, y + 1, cols, rows - y - 1));
                    }
                }
            },
            'K' => {
                let (x, y) = self.cursor;
                match groups.first().and_then(|g| g.first().copied()).unwrap_or(0) {
                    1 => self.erase(Rect::new(0, y, x + 1, 1)),
                    2 => self.erase(Rect::new(0, y, cols, 1)),
                    _ => self.erase(Rect::new(x, y, cols - x, 1)),
                }
            }
            'L' => {
                // Insert lines at the cursor: the rest of the region moves down.
                let n = first(0, 1);
                let (top, bottom) = self.region;
                if self.cursor.1 >= top && self.cursor.1 <= bottom {
                    let saved = self.region;
                    self.region = (self.cursor.1, bottom);
                    self.scroll_down(n);
                    self.region = saved;
                }
            }
            'M' => {
                let n = first(0, 1);
                let (top, bottom) = self.region;
                if self.cursor.1 >= top && self.cursor.1 <= bottom {
                    let saved = self.region;
                    self.region = (self.cursor.1, bottom);
                    self.scroll_up(n);
                    self.region = saved;
                }
            }
            '@' => {
                // Insert blanks at the cursor: the rest of the line moves right.
                let n = first(0, 1).min(cols - self.cursor.0);
                let (x, y) = self.cursor;
                for i in (0..(cols - x - n)).rev() {
                    let cell = self.grid.get(x + i, y).cloned().unwrap_or_default();
                    self.grid.set(x + i + n, y, cell.ch, cell.style);
                }
                self.erase(Rect::new(x, y, n, 1));
            }
            'P' => {
                // Delete at the cursor: the rest of the line moves left.
                let n = first(0, 1).min(cols - self.cursor.0);
                let (x, y) = self.cursor;
                for i in 0..(cols - x - n) {
                    let cell = self.grid.get(x + i + n, y).cloned().unwrap_or_default();
                    self.grid.set(x + i, y, cell.ch, cell.style);
                }
                self.erase(Rect::new(cols - n, y, n, 1));
            }
            'X' => {
                // Erase in place, without moving anything.
                let n = first(0, 1).min(cols - self.cursor.0);
                let (x, y) = self.cursor;
                self.erase(Rect::new(x, y, n, 1));
            }
            'S' => {
                let n = first(0, 1);
                self.scroll_up(n);
            }
            'T' => {
                let n = first(0, 1);
                self.scroll_down(n);
            }
            'r' => {
                let top = first(0, 1).saturating_sub(1).min(rows - 1);
                let bottom = groups
                    .get(1)
                    .and_then(|g| g.first().copied())
                    .filter(|v| *v != 0)
                    .unwrap_or(rows)
                    .saturating_sub(1)
                    .min(rows - 1);
                self.region = if top < bottom {
                    (top, bottom)
                } else {
                    (0, rows - 1)
                };
                self.goto(0, self.region.0);
            }
            'm' => self.sgr(&groups),
            'h' if private => self.mode(&groups, true),
            'l' if private => self.mode(&groups, false),
            's' => self.saved_cursor = Some((self.cursor, self.pen)),
            'u' => {
                if let Some((at, pen)) = self.saved_cursor.take() {
                    self.cursor = at;
                    self.pen = pen;
                }
            }
            _ => {}
        }
        self.dirty = true;
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        match (intermediates.first(), byte) {
            // Reverse index: up one line, scrolling the region down at the top.
            (None, b'M') => {
                if self.cursor.1 == self.region.0 {
                    self.scroll_down(1);
                } else if self.cursor.1 > 0 {
                    self.cursor.1 -= 1;
                }
            }
            (None, b'D') => self.newline(),
            (None, b'E') => {
                self.cursor.0 = 0;
                self.newline();
            }
            (None, b'7') => self.saved_cursor = Some((self.cursor, self.pen)),
            (None, b'8') => {
                if let Some((at, pen)) = self.saved_cursor.take() {
                    self.cursor = at;
                    self.pen = pen;
                }
            }
            (None, b'c') => self.reset(),
            _ => {}
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vte::Parser;

    /// A screen with these bytes fed through the parser.
    fn shown(cols: u16, rows: u16, bytes: &str) -> Screen {
        let mut screen = Screen::new(cols, rows);
        let mut parser = Parser::new();
        parser.advance(&mut screen, bytes.as_bytes());
        screen
    }

    fn feed(screen: &mut Screen, bytes: &str) {
        let mut parser = Parser::new();
        parser.advance(screen, bytes.as_bytes());
    }

    #[test]
    fn plain_text_lands_where_it_is_written() {
        let screen = shown(20, 3, "hello\r\nworld");
        assert_eq!(screen.line(0), "hello");
        assert_eq!(screen.line(1), "world");
        assert_eq!(screen.cursor(), Some((5, 1)));
    }

    #[test]
    fn a_line_wraps_at_the_edge_and_not_before_it() {
        // Exactly as wide as the screen: the cursor sits on the last column and the next
        // character wraps. A terminal that wrapped on *reaching* the edge would leave a blank
        // line after every full-width line, which is the classic off-by-one here.
        let mut screen = shown(5, 3, "abcde");
        assert_eq!(screen.line(0), "abcde");
        assert_eq!(screen.line(1), "");
        feed(&mut screen, "f");
        assert_eq!(screen.line(1), "f");

        // With wrapping off the last column is overwritten instead.
        let screen = shown(5, 2, "\x1b[?7labcdefgh");
        assert_eq!(screen.line(0), "abcdh");
        assert_eq!(screen.line(1), "");
    }

    #[test]
    fn the_cursor_goes_where_it_is_told() {
        let screen = shown(10, 4, "\x1b[3;5Hx\x1b[Hy\x1b[2;2Hz\x1b[Cw\x1b[Av\x1b[2Dq");
        assert_eq!(screen.line(2).trim_end(), "    x");
        assert_eq!(screen.line(0).chars().next(), Some('y'));
        assert_eq!(screen.line(1).chars().nth(1), Some('z'));
        // Right one, up one, left two: `w` then `v` then `q` on the row above.
        assert!(screen.line(1).contains('w'), "{}", screen.line(1));
        assert!(screen.line(0).contains('v'), "{}", screen.line(0));
        assert!(screen.line(0).contains('q'), "{}", screen.line(0));
    }

    #[test]
    fn erasing_clears_what_it_says_and_no_more() {
        // To the end of the line.
        let screen = shown(10, 2, "abcdefghij\x1b[1;4H\x1b[K");
        assert_eq!(screen.line(0), "abc");

        // To the start of the line, the cursor's cell included.
        let screen = shown(10, 2, "abcdefghij\x1b[1;4H\x1b[1K");
        assert_eq!(screen.line(0), "    efghij");

        // The whole line, and then the whole screen.
        let screen = shown(10, 2, "abcdefghij\x1b[2K");
        assert_eq!(screen.line(0), "");
        let screen = shown(10, 3, "one\r\ntwo\r\nthree\x1b[2J");
        assert_eq!(screen.text().trim(), "");

        // To the end of the screen from the middle of it.
        let screen = shown(10, 3, "one\r\ntwo\r\nthree\x1b[2;2H\x1b[J");
        assert_eq!(screen.line(0), "one");
        assert_eq!(screen.line(1), "t");
        assert_eq!(screen.line(2), "");
    }

    #[test]
    fn scrolling_off_the_top_keeps_the_line_in_the_history() {
        let mut screen = Screen::new(10, 2);
        feed(&mut screen, "one\r\ntwo\r\nthree\r\nfour");
        assert_eq!(screen.line(0), "three");
        assert_eq!(screen.line(1), "four");
        assert_eq!(screen.history_len(), 2);
        assert_eq!(screen.history_line(0).as_deref(), Some("one"));
        assert_eq!(screen.history_line(1).as_deref(), Some("two"));
    }

    #[test]
    fn a_scrolling_region_scrolls_on_its_own_and_keeps_no_history() {
        // Four rows, with the middle two as the region: the heading and the footer stay.
        let mut screen = Screen::new(10, 4);
        feed(&mut screen, "head\r\na\r\nb\r\nfoot");
        feed(&mut screen, "\x1b[2;3r\x1b[3;1Hc\n");
        assert_eq!(screen.line(0), "head", "the heading stays");
        assert_eq!(screen.line(3), "foot", "and so does the footer");
        // A progress bar redrawing in a region must not fill the history with its frames.
        assert_eq!(screen.history_len(), 0);
    }

    #[test]
    fn inserting_and_deleting_move_the_rest_along() {
        let screen = shown(10, 2, "abcdef\x1b[1;3H\x1b[2@");
        assert_eq!(screen.line(0), "ab  cdef");
        let screen = shown(10, 2, "abcdef\x1b[1;3H\x1b[2P");
        assert_eq!(screen.line(0), "abef");
        // Erase in place leaves what is after it where it was.
        let screen = shown(10, 2, "abcdef\x1b[1;3H\x1b[2X");
        assert_eq!(screen.line(0), "ab  ef");

        // And the same for lines.
        let screen = shown(10, 4, "one\r\ntwo\r\nthree\x1b[2;1H\x1b[L");
        assert_eq!(screen.line(0), "one");
        assert_eq!(screen.line(1), "");
        assert_eq!(screen.line(2), "two");
        let screen = shown(10, 4, "one\r\ntwo\r\nthree\x1b[2;1H\x1b[M");
        assert_eq!(screen.line(1), "three");
    }

    #[test]
    fn the_attributes_a_program_asks_for_are_the_cell_it_gets() {
        let screen = shown(
            20,
            2,
            "\x1b[1;31mred\x1b[0m \x1b[7mrev\x1b[0m \x1b[38;5;208mo\x1b[0m\x1b[38;2;10;20;30mt\x1b[0m",
        );
        let at = |x: u16| screen.grid().get(x, 0).unwrap().style;
        assert_eq!(at(0).fg, Color::Ansi(1));
        assert!(at(0).bold);
        assert_eq!(at(4).fg, Color::Default, "reset put it back");
        assert!(at(4).reverse);
        assert_eq!(at(8).fg, Color::Indexed(208));
        assert_eq!(at(9).fg, Color::Rgb(10, 20, 30));
        // The bright sixteen, which every prompt uses.
        let screen = shown(10, 1, "\x1b[91mx");
        assert_eq!(screen.grid().get(0, 0).unwrap().style.fg, Color::Ansi(9));
        // And the colon spelling of an extended colour, which is what some programs send.
        let screen = shown(10, 1, "\x1b[38:5:33mx");
        assert_eq!(
            screen.grid().get(0, 0).unwrap().style.fg,
            Color::Indexed(33)
        );
    }

    #[test]
    fn a_cleared_screen_keeps_the_colour_it_was_cleared_with() {
        // What a program with a coloured background does, and the reason erased cells take
        // the pen's background and nothing else.
        let screen = shown(5, 1, "\x1b[44m\x1b[2J");
        let cell = screen.grid().get(2, 0).unwrap();
        assert_eq!(cell.style.bg, Color::Ansi(4));
        assert_eq!(cell.style.fg, Color::Default, "and not its foreground");
        assert!(!cell.style.bold);
    }

    #[test]
    fn the_alternate_screen_leaves_the_first_one_alone() {
        let mut screen = Screen::new(10, 3);
        feed(&mut screen, "shell output\r\n$ vim");
        let before = screen.text();

        // In: a fresh grid, and the history is not added to while it is in use.
        feed(&mut screen, "\x1b[?1049h");
        assert_eq!(screen.text().trim(), "");
        feed(&mut screen, "editing\r\nmore\r\nand more\r\nscrolled");
        assert_eq!(
            screen.history_len(),
            0,
            "the editor's screen is not history"
        );

        // Out: exactly what was there.
        feed(&mut screen, "\x1b[?1049l");
        assert_eq!(screen.text(), before);
    }

    #[test]
    fn a_title_is_read_and_nothing_else_is_answered() {
        let screen = shown(10, 1, "\x1b]0;a shell\x07x");
        assert_eq!(screen.title.as_deref(), Some("a shell"));
        assert_eq!(screen.line(0), "x", "the title is not printed");
        // A sequence we do not implement leaves no text behind either, which is the whole
        // point of a parser: an unknown escape must not become characters on the screen.
        let screen = shown(20, 1, "\x1b[?2004h\x1b]777;notify;x\x07\x1b[>4;2mok");
        assert_eq!(screen.line(0), "ok");
    }

    #[test]
    fn a_tab_goes_to_the_next_stop_and_backspace_only_moves() {
        let screen = shown(20, 1, "a\tb");
        assert_eq!(screen.line(0), "a       b");
        let screen = shown(10, 1, "abc\x08\x08x");
        assert_eq!(screen.line(0), "axc", "backspace moves, it does not erase");
    }

    #[test]
    fn a_double_width_character_takes_two_columns() {
        let screen = shown(10, 1, "a漢b");
        assert_eq!(screen.grid().get(1, 0).unwrap().ch, '漢');
        assert!(
            screen.grid().get(2, 0).unwrap().wide_tail,
            "the second half carries nothing"
        );
        assert_eq!(screen.grid().get(3, 0).unwrap().ch, 'b');

        // One that does not fit at the edge goes to the next line whole.
        let screen = shown(4, 2, "abc漢");
        assert_eq!(screen.line(1), "漢");
    }

    #[test]
    fn the_cursor_can_be_saved_and_put_back() {
        let screen = shown(10, 3, "\x1b[2;3H\x1b7\x1b[1;1Hx\x1b8y");
        assert_eq!(screen.line(0).chars().next(), Some('x'));
        assert_eq!(screen.line(1).chars().nth(2), Some('y'));
    }

    #[test]
    fn resizing_keeps_what_fits_and_the_cursor_inside() {
        let mut screen = Screen::new(10, 3);
        feed(&mut screen, "abcdefghij\r\nsecond");
        screen.resize(5, 2);
        assert_eq!(screen.size(), (5, 2));
        assert_eq!(screen.line(0), "abcde");
        let (x, y) = screen.cursor().expect("the cursor is visible");
        assert!(x < 5 && y < 2, "the cursor stays on the screen: {x},{y}");
    }

    /// The real thing: what a prompt and a `ls` with colour actually send.
    #[test]
    fn a_shell_prompt_with_colour_reads_as_what_it_looks_like() {
        let screen = shown(
            40,
            4,
            "\x1b[32muser@host\x1b[0m:\x1b[34m~/work\x1b[0m$ ls\r\n\
             \x1b[0m\x1b[01;34msrc\x1b[0m  \x1b[01;32mrun.sh\x1b[0m  notes.txt\r\n\
             \x1b[32muser@host\x1b[0m:\x1b[34m~/work\x1b[0m$ ",
        );
        assert_eq!(screen.line(0), "user@host:~/work$ ls");
        assert_eq!(screen.line(1), "src  run.sh  notes.txt");
        let dir = screen.grid().get(0, 1).unwrap().style;
        assert_eq!(dir.fg, Color::Ansi(4));
        assert!(dir.bold, "a directory is bold blue");
        assert_eq!(screen.cursor(), Some((18, 2)));
    }
}
