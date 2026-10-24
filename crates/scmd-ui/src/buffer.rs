use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::color::Color;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub w: u16,
    pub h: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, w: u16, h: u16) -> Self {
        Rect { x, y, w, h }
    }

    pub const fn right(&self) -> u16 {
        self.x + self.w
    }

    pub const fn bottom(&self) -> u16 {
        self.y + self.h
    }

    pub const fn is_empty(&self) -> bool {
        self.w == 0 || self.h == 0
    }

    /// Shrink by one cell on every side (the inside of a frame).
    pub fn inner(&self) -> Rect {
        Rect {
            x: self.x + 1,
            y: self.y + 1,
            w: self.w.saturating_sub(2),
            h: self.h.saturating_sub(2),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub fg: Color,
    pub bg: Color,
    pub bold: bool,
    pub reverse: bool,
}

impl Style {
    pub const fn new(fg: Color, bg: Color) -> Self {
        Style {
            fg,
            bg,
            bold: false,
            reverse: false,
        }
    }

    pub const fn bold(mut self) -> Self {
        self.bold = true;
        self
    }

    pub const fn with_fg(mut self, fg: Color) -> Self {
        self.fg = fg;
        self
    }

    pub const fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub style: Style,
    /// Second half of a double-width glyph: the backend must not emit anything for it.
    pub wide_tail: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            ch: ' ',
            style: Style::default(),
            wide_tail: false,
        }
    }
}

/// The screen as a grid of cells. This is the only thing the UI code knows about: there is no
/// second rendering backend (the interface is strictly TUI), the indirection exists so that a
/// whole screen can be rendered into a string and compared in a test.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    w: u16,
    h: u16,
    cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(w: u16, h: u16) -> Self {
        Buffer {
            w,
            h,
            cells: vec![Cell::default(); w as usize * h as usize],
        }
    }

    pub fn width(&self) -> u16 {
        self.w
    }

    pub fn height(&self) -> u16 {
        self.h
    }

    pub fn area(&self) -> Rect {
        Rect::new(0, 0, self.w, self.h)
    }

    pub fn resize(&mut self, w: u16, h: u16) {
        self.w = w;
        self.h = h;
        self.cells.clear();
        self.cells.resize(w as usize * h as usize, Cell::default());
    }

    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    fn index(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.w && y < self.h {
            Some(y as usize * self.w as usize + x as usize)
        } else {
            None
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index(x, y).map(|i| &self.cells[i])
    }

    pub fn set(&mut self, x: u16, y: u16, ch: char, style: Style) {
        if let Some(i) = self.index(x, y) {
            self.cells[i] = Cell {
                ch,
                style,
                wide_tail: false,
            };
        }
    }

    /// Darken what is already in this rectangle, keeping the characters: the shadow a dialog
    /// drops. Cells outside the buffer are ignored.
    pub fn shadow(&mut self, area: Rect) {
        for y in area.y..area.bottom() {
            for x in area.x..area.right() {
                if let Some(i) = self.index(x, y) {
                    let cell = &mut self.cells[i];
                    cell.style.fg = cell.style.fg.darkened();
                    cell.style.bg = cell.style.bg.darkened();
                    cell.style.bold = false;
                }
            }
        }
    }

    pub fn clear(&mut self, style: Style) {
        self.cells.fill(Cell {
            ch: ' ',
            style,
            wide_tail: false,
        });
    }

    pub fn fill(&mut self, rect: Rect, ch: char, style: Style) {
        for y in rect.y..rect.bottom().min(self.h) {
            for x in rect.x..rect.right().min(self.w) {
                self.set(x, y, ch, style);
            }
        }
    }

    /// Write a string, clipped to `max_cols`, honouring double-width glyphs.
    ///
    /// Returns the number of columns written. A double-width glyph that would be cut in half
    /// is replaced by a space, so the grid never tears.
    pub fn put_str(&mut self, x: u16, y: u16, s: &str, style: Style, max_cols: u16) -> u16 {
        let mut col = 0u16;
        for ch in s.chars() {
            if ch == '\n' || ch == '\r' || ch == '\t' {
                continue;
            }
            let cw = ch.width().unwrap_or(0) as u16;
            if cw == 0 {
                continue; // combining marks: not supported as separate cells in M1
            }
            if col + cw > max_cols {
                if col < max_cols {
                    self.set(x + col, y, ' ', style);
                    col += 1;
                }
                break;
            }
            self.set(x + col, y, ch, style);
            if cw == 2 {
                if let Some(i) = self.index(x + col + 1, y) {
                    self.cells[i] = Cell {
                        ch: ' ',
                        style,
                        wide_tail: true,
                    };
                }
            }
            col += cw;
        }
        col
    }

    /// Single-line box with an optional centred title.
    pub fn draw_frame(&mut self, rect: Rect, style: Style, title: Option<&str>) {
        if rect.w < 2 || rect.h < 2 {
            return;
        }
        let (x0, y0, x1, y1) = (rect.x, rect.y, rect.right() - 1, rect.bottom() - 1);
        self.set(x0, y0, '┌', style);
        self.set(x1, y0, '┐', style);
        self.set(x0, y1, '└', style);
        self.set(x1, y1, '┘', style);
        for x in x0 + 1..x1 {
            self.set(x, y0, '─', style);
            self.set(x, y1, '─', style);
        }
        for y in y0 + 1..y1 {
            self.set(x0, y, '│', style);
            self.set(x1, y, '│', style);
        }
        if let Some(t) = title {
            let avail = rect.w.saturating_sub(4);
            let text = truncate_cols(t, avail as usize);
            let w = text.width() as u16;
            if w > 0 {
                let start = x0 + 1 + (rect.w.saturating_sub(2).saturating_sub(w + 2)) / 2;
                self.put_str(start, y0, &format!(" {text} "), style, w + 2);
            }
        }
    }

    /// The rendered screen as lines of text — the basis of the snapshot tests.
    pub fn to_lines(&self) -> Vec<String> {
        (0..self.h)
            .map(|y| {
                let mut line = String::with_capacity(self.w as usize);
                for x in 0..self.w {
                    let c = self.get(x, y).expect("in range");
                    if !c.wide_tail {
                        line.push(c.ch);
                    }
                }
                line.trim_end().to_string()
            })
            .collect()
    }

    pub fn to_text(&self) -> String {
        self.to_lines().join("\n")
    }

    /// Cells that differ from `prev`, in writing order. The backend emits only these.
    pub fn diff<'a>(&'a self, prev: &Buffer) -> Vec<(u16, u16, &'a Cell)> {
        let mut out = Vec::new();
        if prev.w != self.w || prev.h != self.h {
            for y in 0..self.h {
                for x in 0..self.w {
                    let c = self.get(x, y).expect("in range");
                    if !c.wide_tail {
                        out.push((x, y, c));
                    }
                }
            }
            return out;
        }
        for y in 0..self.h {
            for x in 0..self.w {
                let (a, b) = (
                    self.get(x, y).expect("in range"),
                    prev.get(x, y).expect("in range"),
                );
                if a != b && !a.wide_tail {
                    out.push((x, y, a));
                }
            }
        }
        out
    }
}

/// Truncate to `cols` display columns, appending `…` when it does not fit.
pub fn truncate_cols(s: &str, cols: usize) -> String {
    if s.width() <= cols {
        return s.to_string();
    }
    if cols == 0 {
        return String::new();
    }
    let mut out = String::new();
    let mut used = 0usize;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > cols.saturating_sub(1) {
            break;
        }
        out.push(ch);
        used += cw;
    }
    out.push('…');
    out
}

/// Elide the middle of a path so both ends stay readable: `/home/mb/…/src/main.rs`.
pub fn elide_middle(s: &str, cols: usize) -> String {
    if s.width() <= cols || cols < 5 {
        return truncate_cols(s, cols);
    }
    let keep = cols - 1;
    let head_cols = keep / 2;
    let tail_cols = keep - head_cols;

    let mut head = String::new();
    let mut used = 0;
    for ch in s.chars() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > head_cols {
            break;
        }
        head.push(ch);
        used += cw;
    }

    let mut tail: Vec<char> = Vec::new();
    let mut used = 0;
    for ch in s.chars().rev() {
        let cw = ch.width().unwrap_or(0);
        if used + cw > tail_cols {
            break;
        }
        tail.push(ch);
        used += cw;
    }
    tail.reverse();
    format!("{head}…{}", tail.into_iter().collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> Style {
        Style::new(Color::Ansi(7), Color::Ansi(4))
    }

    #[test]
    fn writes_and_clips_text() {
        let mut b = Buffer::new(10, 2);
        let written = b.put_str(0, 0, "hello world", style(), 8);
        assert_eq!(written, 8);
        assert_eq!(b.to_lines()[0], "hello wo");
    }

    #[test]
    fn double_width_glyphs_occupy_two_cells() {
        let mut b = Buffer::new(8, 1);
        let written = b.put_str(0, 0, "日本語", style(), 8);
        assert_eq!(written, 6);
        assert_eq!(b.get(0, 0).unwrap().ch, '日');
        assert!(b.get(1, 0).unwrap().wide_tail);
        assert_eq!(b.to_lines()[0], "日本語");
    }

    #[test]
    fn a_wide_glyph_is_never_cut_in_half() {
        let mut b = Buffer::new(8, 1);
        // Only 3 columns available: 日 fits, 本 does not.
        let written = b.put_str(0, 0, "日本", style(), 3);
        assert_eq!(written, 3);
        assert_eq!(b.to_lines()[0], "日");
        assert!(!b.get(2, 0).unwrap().wide_tail);
    }

    #[test]
    fn frame_with_title() {
        let mut b = Buffer::new(12, 3);
        b.draw_frame(Rect::new(0, 0, 12, 3), style(), Some("src"));
        let lines = b.to_lines();
        assert!(lines[0].starts_with('┌') && lines[0].ends_with('┐'));
        assert!(lines[0].contains("src"));
        assert_eq!(lines[1], "│          │");
        assert!(lines[2].starts_with('└'));
    }

    #[test]
    fn diff_reports_only_changes() {
        let mut a = Buffer::new(4, 1);
        let mut b = Buffer::new(4, 1);
        a.put_str(0, 0, "ab", style(), 4);
        b.put_str(0, 0, "ab", style(), 4);
        assert!(b.diff(&a).is_empty());
        b.put_str(1, 0, "x", style(), 1);
        let d = b.diff(&a);
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].0, d[0].1, d[0].2.ch), (1, 0, 'x'));
    }

    #[test]
    fn resize_forces_a_full_repaint() {
        let a = Buffer::new(4, 1);
        let mut b = Buffer::new(6, 1);
        b.put_str(0, 0, "abcdef", style(), 6);
        assert_eq!(b.diff(&a).len(), 6);
    }

    #[test]
    fn truncation_and_elision() {
        assert_eq!(truncate_cols("abcdef", 10), "abcdef");
        assert_eq!(truncate_cols("abcdef", 4), "abc…");
        assert_eq!(
            elide_middle("/home/matteo/src/project/main.rs", 16).width(),
            16
        );
        assert!(elide_middle("/home/matteo/src/project/main.rs", 16).contains('…'));
        assert!(elide_middle("/a/b", 16).starts_with("/a/b"));
    }
}
