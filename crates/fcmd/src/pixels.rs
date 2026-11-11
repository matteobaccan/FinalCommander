//! Pictures in real pixels, between the viewer and the terminal (SPEC 05 §1.14a).
//!
//! Three parties have a hand in a picture drawn as pixels. The viewer has the picture; a
//! worker scales and encodes it, because that is tens of milliseconds for a photograph and
//! the drawing thread has none to spare; and the terminal backend sends it, because only it
//! knows what is on the screen. This module is the table they meet at: what has been asked
//! for, what has come back, and what this frame wants where.
//!
//! The half blocks never go away. A picture is drawn in them until its pixels are ready, and
//! again whenever anything is drawn across it — a dialog, a menu — because an image lies on
//! top of the cells and would cover whatever was put there.

use std::sync::Arc;

use fcmd_tui::{Pixels, Placement};
use fcmd_ui::{Buffer, Rect, Style};
use fcmd_view::picture::Picture;
use fcmd_view::Viewer;

/// The size of a cell when nothing says otherwise: the eight by sixteen of a VGA text mode,
/// which is still the shape most terminal fonts come close to.
pub const DEFAULT_CELL: (u16, u16) = (8, 16);

/// Windows Terminal draws sixel as a VT340 would have: a cell is ten pixels by twenty
/// whatever the font is, and it scales the image to the cells it really has.
pub const WINDOWS_TERMINAL_CELL: (u16, u16) = (10, 20);

/// The cell a picture's pixels are written into while a frame is being drawn.
///
/// A character from the private use area that nothing else puts on the screen. Whatever is
/// drawn over the picture afterwards replaces it, which is how the end of the frame knows —
/// and it is turned into a space before the frame leaves, so the terminal never sees one.
const MARK: char = '\u{e0ff}';

/// Which of the two windows that draw pictures a picture is in. Each keeps one encoded
/// picture, so the quick view walking a folder does not throw away what `F3` is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Slot {
    Viewer,
    Quick,
}

/// Everything an encoded picture depends on. Two asks with the same key would produce the
/// same bytes, so the second is never made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key {
    /// The picture's own number (`Viewer::picture_shared`).
    pub serial: u64,
    /// The window it is fitted into, in cells.
    pub cols: u16,
    pub rows: u16,
    pub cell: (u16, u16),
    pub protocol: Pixels,
    /// What shows through where the picture is transparent — for sixel, which is given the
    /// colour because it cannot be given the transparency.
    pub under: [u8; 3],
}

/// A picture to encode, for a worker.
#[derive(Debug, Clone)]
pub struct Ask {
    pub key: Key,
    pub picture: Arc<Picture>,
}

/// Two asks for the same key are the same ask: the key is everything the answer depends on.
impl PartialEq for Ask {
    fn eq(&self, other: &Ask) -> bool {
        self.key == other.key
    }
}

/// What came back: the cells the picture takes and the bytes to send for them.
#[derive(Debug, Clone)]
pub struct Drawn {
    pub cols: u16,
    pub rows: u16,
    pub data: Arc<Vec<u8>>,
}

/// Scale and encode a picture for the terminal. Runs on a worker; `None` when there is no
/// room to draw in or the encoder refused.
pub fn encode(ask: &Ask) -> Option<Drawn> {
    let Key {
        cols, rows, cell, ..
    } = ask.key;
    let picture = &ask.picture;
    let fit = fcmd_view::pixels::fit(picture.width, picture.height, cols, rows, cell)?;
    let data = match ask.key.protocol {
        Pixels::Kitty | Pixels::Iterm => fcmd_view::pixels::png(picture, fit, cell).ok()?,
        Pixels::Sixel => fcmd_view::pixels::sixel(picture, fit, cell, ask.key.under),
    };
    (!data.is_empty()).then(|| Drawn {
        cols: fit.cols,
        rows: fit.rows,
        data: Arc::new(data),
    })
}

struct Ready {
    key: Key,
    slot: Slot,
    drawn: Drawn,
}

/// The state of pictures-as-pixels for the whole program.
#[derive(Default)]
pub struct Screen {
    /// The protocol in use; `None` is half blocks, which is also what every test and every
    /// screenshot gets.
    protocol: Option<Pixels>,
    cell: (u16, u16),
    ready: Vec<Ready>,
    /// What a worker is encoding now, and for whom.
    pending: Vec<(Key, Slot)>,
    /// What this frame wants on the screen, for the backend.
    placements: Vec<Placement>,
    /// The style each placement's cells were marked in, to tell a cell that was drawn over
    /// from one that was not.
    marks: Vec<Style>,
    /// Set for the second drawing of a frame whose pictures something was drawn across.
    covered: bool,
    asks: Vec<Ask>,
}

impl Screen {
    /// Say how pictures are drawn from now on. Changing the protocol or the size of a cell
    /// makes everything encoded so far the wrong bytes, and it is dropped.
    pub fn configure(&mut self, protocol: Option<Pixels>, cell: (u16, u16)) {
        if (protocol, cell) != (self.protocol, self.cell) {
            self.protocol = protocol;
            self.cell = cell;
            self.ready.clear();
            self.pending.clear();
        }
    }

    /// The size of a cell in pixels, as far as anybody knows it.
    pub fn cell(&self) -> (u16, u16) {
        if self.cell.0 == 0 || self.cell.1 == 0 {
            DEFAULT_CELL
        } else {
            self.cell
        }
    }

    /// A new frame: nothing is placed yet.
    pub fn begin(&mut self, covered: bool) {
        self.placements.clear();
        self.marks.clear();
        self.covered = covered;
    }

    /// The rows of `area` a picture may take. Sixel and iTerm2 leave the cursor under the
    /// image they drew, and a cursor pushed off the last row scrolls the screen; kitty is
    /// told to leave it where it was.
    fn rows_for(protocol: Pixels, buf: &Buffer, area: Rect) -> u16 {
        if protocol != Pixels::Kitty && area.y.saturating_add(area.h) >= buf.height() {
            area.h.saturating_sub(1)
        } else {
            area.h
        }
    }

    /// Where in `area` the picture is drawn when it is drawn in half blocks.
    ///
    /// Without a protocol that is all of it, as it always was. With one it is the cells the
    /// pixels take: the half blocks are then a coarse copy of the same picture in the same
    /// place — while the pixels are being encoded, under a dialog — and it sharpens or
    /// blurs where it stands instead of jumping to another size.
    pub fn window(&self, viewer: &mut Viewer, buf: &Buffer, area: Rect) -> Rect {
        let Some(protocol) = self.protocol else {
            return area;
        };
        let Some((_, picture)) = viewer.picture_shared() else {
            return area;
        };
        let rows = Self::rows_for(protocol, buf, area);
        match fcmd_view::pixels::fit(picture.width, picture.height, area.w, rows, self.cell()) {
            Some(fit) => Rect::new(
                area.x + (area.w - fit.cols) / 2,
                area.y + (rows - fit.rows) / 2,
                fit.cols,
                fit.rows,
            ),
            None => area,
        }
    }

    /// Draw the viewer's picture into `area` as pixels, if that can be done now.
    ///
    /// `true` when the cells were taken: they are marked, and the backend will be handed the
    /// image. `false` is every other case — no protocol, no picture, pixels not encoded yet
    /// (they are asked for here), something drawn across the picture last time round — and
    /// the caller draws half blocks, which is never wrong.
    pub fn place(
        &mut self,
        slot: Slot,
        viewer: &mut Viewer,
        buf: &mut Buffer,
        area: Rect,
        under: [u8; 3],
        base: Style,
    ) -> bool {
        let Some(protocol) = self.protocol else {
            return false;
        };
        if self.covered {
            return false;
        }
        let Some((serial, picture)) = viewer.picture_shared() else {
            return false;
        };
        let rows = Self::rows_for(protocol, buf, area);
        if rows == 0 || area.w == 0 {
            return false;
        }
        let key = Key {
            serial,
            cols: area.w,
            rows,
            cell: self.cell(),
            protocol,
            under,
        };
        let exact = self.ready.iter().position(|r| r.key == key);
        if exact.is_none() && !self.pending.iter().any(|(k, _)| *k == key) {
            self.pending.push((key, slot));
            self.asks.push(Ask { key, picture });
        }
        let found = exact;
        let Some(found) = found else {
            return false;
        };
        let ready = &self.ready[found];
        let (cols, rows_used) = (ready.drawn.cols.min(area.w), ready.drawn.rows.min(rows));
        // Centred, as the half blocks are.
        let x = area.x + (area.w - cols) / 2;
        let y = area.y + (rows - rows_used) / 2;
        buf.fill(Rect::new(x, y, cols, rows_used), MARK, base);
        self.placements.push(Placement {
            x,
            y,
            cols,
            rows: rows_used,
            protocol,
            id: ready.key.serial,
            data: ready.drawn.data.clone(),
        });
        self.marks.push(base);
        true
    }

    /// The frame is drawn: is every picture still whole?
    ///
    /// `true` turns the marks into the blank cells the terminal is sent. `false` means
    /// something was drawn over a picture — one cell is enough — and the frame has to be
    /// drawn again without them ([`Screen::begin`] with `covered`).
    pub fn settle(&mut self, buf: &mut Buffer) -> bool {
        let whole = self.placements.iter().zip(&self.marks).all(|(p, style)| {
            (p.y..p.y + p.rows).all(|y| {
                (p.x..p.x + p.cols).all(|x| {
                    buf.get(x, y)
                        .is_some_and(|cell| cell.ch == MARK && cell.style == *style)
                })
            })
        });
        if !whole {
            self.placements.clear();
            self.marks.clear();
            return false;
        }
        for (p, style) in self.placements.iter().zip(&self.marks) {
            buf.fill(Rect::new(p.x, p.y, p.cols, p.rows), ' ', *style);
        }
        true
    }

    /// The pictures this frame puts on the screen.
    pub fn placements(&self) -> &[Placement] {
        &self.placements
    }

    /// The pictures to encode, handed over once.
    pub fn take_asks(&mut self) -> Vec<Ask> {
        std::mem::take(&mut self.asks)
    }

    /// A worker's answer. An answer nobody is waiting for — the protocol changed meanwhile —
    /// is dropped; one that failed is forgotten, and the picture stays in half blocks.
    pub fn arrived(&mut self, key: Key, drawn: Option<Drawn>) {
        let Some(at) = self.pending.iter().position(|(k, _)| *k == key) else {
            return;
        };
        let (_, slot) = self.pending.remove(at);
        let Some(drawn) = drawn else {
            return;
        };
        // One picture a window: the one before it is megabytes nobody will draw again.
        self.ready.retain(|r| r.slot != slot);
        self.ready.push(Ready { key, slot, drawn });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::VPath;
    use fcmd_view::BytesSource;

    /// A viewer on a PNG of `w × h` red pixels, written here.
    fn viewer_on(w: u32, h: u32) -> Viewer {
        let picture = Picture::from_rgba(w, h, "PNG", [255, 0, 0, 255].repeat((w * h) as usize))
            .expect("the right number of bytes");
        let fit = fcmd_view::pixels::fit(w, h, w as u16, h as u16, (1, 1)).unwrap();
        let png = fcmd_view::pixels::png(&picture, fit, (1, 1)).unwrap();
        Viewer::new(
            Box::new(BytesSource::new(png)),
            VPath::local("red.png".to_string()),
        )
    }

    fn screen(protocol: Pixels) -> Screen {
        let mut s = Screen::default();
        s.configure(Some(protocol), (8, 16));
        s
    }

    #[test]
    fn without_a_protocol_nothing_is_placed_and_nothing_is_asked() {
        let mut s = Screen::default();
        let mut v = viewer_on(64, 64);
        let mut buf = Buffer::new(40, 12);
        s.begin(false);
        let area = Rect::new(0, 0, 40, 10);
        assert!(!s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default()
        ));
        assert!(s.take_asks().is_empty());
        assert!(s.placements().is_empty());
    }

    #[test]
    fn a_picture_is_half_blocks_until_its_pixels_come_back_and_then_it_is_placed() {
        let mut s = screen(Pixels::Kitty);
        let mut v = viewer_on(64, 64);
        let mut buf = Buffer::new(40, 12);
        let area = Rect::new(0, 1, 40, 10);
        let base = Style::default();

        // The first frame: nothing encoded yet, so the caller draws half blocks, and one
        // ask goes out.
        s.begin(false);
        assert!(!s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        let asks = s.take_asks();
        assert_eq!(asks.len(), 1);
        assert_eq!((asks[0].key.cols, asks[0].key.rows), (40, 10));
        // The second frame, still waiting: the same picture is not asked for twice.
        s.begin(false);
        assert!(!s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        assert!(s.take_asks().is_empty());

        // The worker answers. 64 pixels in cells of 8×16 are eight cells by four, centred.
        let drawn = encode(&asks[0]).expect("a PNG");
        assert_eq!((drawn.cols, drawn.rows), (8, 4));
        assert!(drawn.data.starts_with(b"\x89PNG"));
        s.arrived(asks[0].key, Some(drawn));
        s.begin(false);
        assert!(s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        assert!(s.settle(&mut buf));
        let p = &s.placements()[0];
        assert_eq!((p.x, p.y, p.cols, p.rows), (16, 4, 8, 4));
        assert_eq!(p.protocol, Pixels::Kitty);
        // What the terminal is sent under the picture is blank cells, not the mark.
        assert!(buf.cells().iter().all(|c| c.ch != MARK));
        assert_eq!(buf.get(16, 4).map(|c| c.ch), Some(' '));
    }

    #[test]
    fn half_blocks_are_drawn_in_the_cells_the_pixels_will_take() {
        let mut v = viewer_on(64, 64);
        let buf = Buffer::new(40, 12);
        let area = Rect::new(0, 1, 40, 10);
        // No protocol: the whole window, as it always was.
        assert_eq!(Screen::default().window(&mut v, &buf, area), area);
        // With one: eight cells by four in the middle, where the pixels go when they come.
        let s = screen(Pixels::Kitty);
        assert_eq!(s.window(&mut v, &buf, area), Rect::new(16, 4, 8, 4));
    }

    #[test]
    fn anything_drawn_across_a_picture_sends_the_frame_back_to_half_blocks() {
        let mut s = screen(Pixels::Kitty);
        let mut v = viewer_on(64, 64);
        let mut buf = Buffer::new(40, 12);
        let area = Rect::new(0, 1, 40, 10);
        let base = Style::default();
        s.begin(false);
        s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base);
        let ask = s.take_asks().remove(0);
        s.arrived(ask.key, encode(&ask));

        // A dialog's corner lands on one cell of the picture.
        s.begin(false);
        assert!(s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        buf.set(17, 5, '║', base);
        assert!(!s.settle(&mut buf), "one cell is enough");
        assert!(s.placements().is_empty());
        // The frame is drawn again, and this time the picture declines its place.
        s.begin(true);
        assert!(!s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        assert!(s.settle(&mut buf));

        // A shadow changes no character, only the colours: that is drawn over too.
        s.begin(false);
        assert!(s.place(Slot::Viewer, &mut v, &mut buf, area, [0; 3], base));
        buf.shadow(Rect::new(18, 6, 2, 1));
        assert!(!s.settle(&mut buf));
    }

    #[test]
    fn sixel_keeps_off_the_last_row_of_the_screen_and_kitty_need_not() {
        // The window reaches the bottom of a 12-row screen.
        let area = Rect::new(0, 2, 40, 10);
        let mut buf = Buffer::new(40, 12);
        let mut v = viewer_on(400, 400);
        let mut s = screen(Pixels::Sixel);
        s.begin(false);
        s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default(),
        );
        assert_eq!(
            s.take_asks()[0].key.rows,
            9,
            "a row short, so nothing scrolls"
        );
        let mut s = screen(Pixels::Kitty);
        s.begin(false);
        s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default(),
        );
        assert_eq!(s.take_asks()[0].key.rows, 10);
    }

    #[test]
    fn sixel_is_encoded_over_the_colour_of_the_window() {
        let mut s = screen(Pixels::Sixel);
        let mut v = viewer_on(16, 16);
        let mut buf = Buffer::new(40, 12);
        s.begin(false);
        s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            Rect::new(0, 0, 40, 10),
            [0, 0, 128],
            Style::default(),
        );
        let ask = s.take_asks().remove(0);
        let drawn = encode(&ask).expect("sixel");
        assert_eq!((drawn.cols, drawn.rows), (2, 1));
        assert!(drawn.data.starts_with(b"\x1bP0;1;0q\"1;1;16;16"));
        assert!(drawn.data.ends_with(b"\x1b\\"));
    }

    #[test]
    fn a_new_setting_forgets_what_was_encoded_for_the_old_one() {
        let mut s = screen(Pixels::Kitty);
        let mut v = viewer_on(64, 64);
        let mut buf = Buffer::new(40, 12);
        let area = Rect::new(0, 1, 40, 10);
        s.begin(false);
        s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default(),
        );
        let ask = s.take_asks().remove(0);
        // The reader turns pixels off while the worker is still at it: the answer is for
        // nobody, and the next frame is half blocks with nothing asked.
        s.configure(None, (8, 16));
        s.arrived(ask.key, encode(&ask));
        s.begin(false);
        assert!(!s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default()
        ));
        assert!(s.take_asks().is_empty());
        // And an encoder that had nothing to give leaves the picture in half blocks.
        s.configure(Some(Pixels::Iterm), (8, 16));
        s.begin(false);
        s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default(),
        );
        let ask = s.take_asks().remove(0);
        s.arrived(ask.key, None);
        s.begin(false);
        assert!(!s.place(
            Slot::Viewer,
            &mut v,
            &mut buf,
            area,
            [0; 3],
            Style::default()
        ));
    }
}
