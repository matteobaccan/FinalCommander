//! The screen grabber (`Alt+Ins`): copy a piece of the screen as it stands (SPEC 06 §11b).
//!
//! FAR has this, and the reason it earns its key is not the panels — it is everything else.
//! A process card, a help page, a hex dump, the foot of a diff: text the program drew and has
//! nowhere to copy from, because none of it is a file. The terminal's own selection takes
//! frames, column rules and padding with it, and over `ssh` it takes whatever the far end
//! happens to have scrolled. This takes the rectangle one asks for and nothing else.
//!
//! What it holds is the screen as it was when the key was pressed: a snapshot rather than the
//! live program, so moving the cursor about cannot change what is being copied, and the
//! program underneath is not running while one reads it.

use fcmd_ui::{Buffer, Rect};

/// The grabber: a frame, a cursor over it, and where the mark was dropped.
#[derive(Debug, Clone)]
pub struct Grab {
    /// The screen the key was pressed on, cells and colours and all.
    pub screen: Buffer,
    /// Where the cursor is, in screen columns and rows.
    pub x: u16,
    pub y: u16,
    /// Where `Shift+arrow` first dropped the mark. Without one there is no rectangle, and
    /// the key that copies takes the row the cursor is on.
    pub anchor: Option<(u16, u16)>,
}

impl Grab {
    /// Open the grabber on a frame, with the cursor in the top left corner.
    pub fn new(screen: Buffer) -> Grab {
        Grab {
            screen,
            x: 0,
            y: 0,
            anchor: None,
        }
    }

    pub fn width(&self) -> u16 {
        self.screen.width()
    }

    pub fn height(&self) -> u16 {
        self.screen.height()
    }

    /// Move the cursor. `mark` is whether the key was pressed with `Shift`: the first such
    /// key drops the mark where the cursor was, so the rectangle grows from where one started
    /// rather than from where one noticed.
    pub fn move_by(&mut self, dx: i32, dy: i32, mark: bool) {
        if mark {
            self.anchor.get_or_insert((self.x, self.y));
        } else {
            self.anchor = None;
        }
        let last_x = self.width().saturating_sub(1);
        let last_y = self.height().saturating_sub(1);
        self.x = (self.x as i32 + dx).clamp(0, last_x as i32) as u16;
        self.y = (self.y as i32 + dy).clamp(0, last_y as i32) as u16;
    }

    /// `Home`, `End`, `Ctrl+Home`, `Ctrl+End`: the ends of the row and of the screen.
    pub fn go(&mut self, x: Option<u16>, y: Option<u16>, mark: bool) {
        if mark {
            self.anchor.get_or_insert((self.x, self.y));
        } else {
            self.anchor = None;
        }
        if let Some(x) = x {
            self.x = x.min(self.width().saturating_sub(1));
        }
        if let Some(y) = y {
            self.y = y.min(self.height().saturating_sub(1));
        }
    }

    /// Mark the whole screen, for the one case where the rectangle is all of it.
    pub fn all(&mut self) {
        self.anchor = Some((0, 0));
        self.x = self.width().saturating_sub(1);
        self.y = self.height().saturating_sub(1);
    }

    /// The rectangle that is marked: the cursor and the mark are two corners of it, so it
    /// does not matter which way round they were made. With no mark it is the row the cursor
    /// is on — "copy this line", which is what one wants from a grabber more often than a
    /// rectangle.
    pub fn area(&self) -> Rect {
        match self.anchor {
            Some((ax, ay)) => {
                let (x0, x1) = (self.x.min(ax), self.x.max(ax));
                let (y0, y1) = (self.y.min(ay), self.y.max(ay));
                Rect::new(x0, y0, x1 - x0 + 1, y1 - y0 + 1)
            }
            None => Rect::new(0, self.y, self.width(), 1),
        }
    }

    /// What is inside that rectangle.
    pub fn text(&self) -> String {
        self.screen.region_text(self.area())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_ui::{Color, Style};

    fn screen() -> Buffer {
        let mut buf = Buffer::new(20, 4);
        let style = Style::new(Color::Ansi(7), Color::Ansi(0));
        buf.put_str(0, 0, "one two three", style, 20);
        buf.put_str(0, 1, "four five", style, 20);
        buf.put_str(0, 2, "six", style, 20);
        buf
    }

    #[test]
    fn with_no_mark_the_row_under_the_cursor_is_what_is_copied() {
        let mut grab = Grab::new(screen());
        grab.move_by(0, 1, false);
        assert_eq!(grab.area(), Rect::new(0, 1, 20, 1));
        assert_eq!(grab.text(), "four five");
    }

    #[test]
    fn shift_drops_the_mark_where_the_cursor_was_and_the_rectangle_grows_from_it() {
        let mut grab = Grab::new(screen());
        grab.go(Some(4), Some(0), false);
        assert!(grab.anchor.is_none());
        // Four columns right and one row down, with the mark dropped at the first key.
        for _ in 0..3 {
            grab.move_by(1, 0, true);
        }
        grab.move_by(0, 1, true);
        assert_eq!(grab.anchor, Some((4, 0)));
        assert_eq!(grab.area(), Rect::new(4, 0, 4, 2));
        // Four columns of each row: `two ` with its trailing blank gone, and the four
        // columns of `four five` that stand under it.
        assert_eq!(grab.text(), "two\n fiv");
    }

    #[test]
    fn the_mark_may_be_below_and_right_of_the_cursor() {
        let mut grab = Grab::new(screen());
        grab.go(Some(8), Some(1), false);
        grab.move_by(-4, -1, true);
        assert_eq!(grab.area(), Rect::new(4, 0, 5, 2));
        assert_eq!(grab.text(), "two t\n five");
    }

    #[test]
    fn a_plain_arrow_drops_the_mark() {
        let mut grab = Grab::new(screen());
        grab.move_by(1, 0, true);
        assert!(grab.anchor.is_some());
        grab.move_by(1, 0, false);
        assert!(grab.anchor.is_none(), "an arrow on its own is not a mark");
    }

    #[test]
    fn the_cursor_stays_on_the_screen() {
        let mut grab = Grab::new(screen());
        grab.move_by(-5, -5, false);
        assert_eq!((grab.x, grab.y), (0, 0));
        grab.move_by(100, 100, false);
        assert_eq!((grab.x, grab.y), (19, 3));
        grab.go(Some(999), Some(999), false);
        assert_eq!((grab.x, grab.y), (19, 3));
    }

    #[test]
    fn the_whole_screen_can_be_marked_at_once() {
        let mut grab = Grab::new(screen());
        grab.all();
        assert_eq!(grab.area(), Rect::new(0, 0, 20, 4));
        assert_eq!(grab.text(), "one two three\nfour five\nsix\n");
    }
}
