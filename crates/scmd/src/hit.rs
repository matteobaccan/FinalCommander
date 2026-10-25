//! Where things were drawn, so that a click can be answered.
//!
//! The drawing code knows the geometry and the key handling does not, which is fine until a
//! mouse arrives: a click is a column and a row, and nothing else on the screen says what is
//! under it. So [`draw`](crate::render::draw) records the handful of rectangles that can be
//! clicked into [`Layout`], and this module turns a position into the thing that was hit.
//!
//! All of it is arithmetic over rectangles, which is why it is here rather than inside the
//! event handler: every case below is a test that needs no terminal.

use scmd_ui::Rect;

/// What a click landed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hit {
    /// A row of a panel: which side, and which entry of the listing.
    Row { side: usize, index: usize },
    /// Inside a panel, but past the last row — the empty space below the listing.
    Panel { side: usize },
    /// A function key on the key bar: 1 to 12.
    Key(u8),
    /// A title on the menu bar.
    Menu(usize),
    /// The command line.
    CommandLine,
    /// A row of the list a dialog is showing: which row of the visible window.
    ListRow(usize),
    /// Inside the text of the viewer, the editor or the terminal, at this cell of its area.
    Text { col: u16, row: u16 },
}

/// Where the clickable parts of the screen were on the last frame.
///
/// Everything is `Option` because most of it is not always there: a key bar can be switched
/// off, a panel can be hidden, a dialog is open or it is not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Layout {
    /// The rows area of each panel — the inside of the frame, below the column titles — and
    /// the index of the first row shown in it, with the number of visible rows and columns.
    pub panels: [Option<PanelArea>; 2],
    /// The key bar's row, and the function key in each of its cells.
    ///
    /// The numbers rather than a count, because the bar is not always `F1` to `F12`: hold a
    /// modifier and it shows only the keys that modifier has, so the third cell may be `F5`.
    pub key_bar: Option<(u16, Vec<u8>)>,
    /// The menu bar's row, and where each title starts.
    pub menu_bar: Option<(u16, Vec<u16>)>,
    pub command_line: Option<u16>,
    /// The rows of an open list (a menu, a pick dialog), and the first row shown in it.
    pub list: Option<ListArea>,
    /// The text area of the viewer, the editor, the hex editor or the terminal.
    pub text: Option<Rect>,
}

/// One panel's list, as it was drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelArea {
    pub area: Rect,
    /// The first entry shown.
    pub top: usize,
    /// How many rows tall the list is, and how many columns wide.
    pub rows: usize,
    pub columns: usize,
    /// How many entries there are, so a click past the end is not a row.
    pub len: usize,
}

/// An open list's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListArea {
    pub area: Rect,
    pub top: usize,
    pub len: usize,
}

impl Layout {
    /// What is at this cell, if anything.
    ///
    /// The order is the order things are drawn in: a list that is open is over the panels, and
    /// the bars at the edges of the screen are not under either.
    pub fn hit(&self, col: u16, row: u16) -> Option<Hit> {
        if let Some((bar, keys)) = &self.key_bar {
            if row == *bar {
                return key_cell(col, keys, self.width_hint()).map(Hit::Key);
            }
        }
        if let Some((bar, starts)) = &self.menu_bar {
            if row == *bar {
                return menu_cell(col, starts).map(Hit::Menu);
            }
        }
        if self.command_line == Some(row) {
            return Some(Hit::CommandLine);
        }
        if let Some(list) = &self.list {
            if list.area.contains(col, row) {
                let offset = (row - list.area.y) as usize;
                let index = list.top + offset;
                return (index < list.len).then_some(Hit::ListRow(index));
            }
            // A click outside an open list is not a click on what is underneath it.
            return None;
        }
        if let Some(text) = self.text {
            if text.contains(col, row) {
                return Some(Hit::Text {
                    col: col - text.x,
                    row: row - text.y,
                });
            }
            return None;
        }
        for (side, panel) in self.panels.iter().enumerate() {
            let Some(panel) = panel else { continue };
            if !panel.area.contains(col, row) {
                continue;
            }
            return Some(match panel.index_at(col, row) {
                Some(index) => Hit::Row { side, index },
                None => Hit::Panel { side },
            });
        }
        None
    }

    /// The key bar spans the screen, so its width is the widest thing recorded.
    fn width_hint(&self) -> u16 {
        let mut widest = 0;
        for panel in self.panels.iter().flatten() {
            widest = widest.max(panel.area.right());
        }
        if let Some(text) = self.text {
            widest = widest.max(text.right());
        }
        widest
    }
}

impl PanelArea {
    /// Which entry is at this cell, or `None` for the space past the last one.
    ///
    /// The columns run top to bottom and then across, which is how the brief and wide views
    /// are drawn: entry `top + column * rows + row`.
    pub fn index_at(&self, col: u16, row: u16) -> Option<usize> {
        if !self.area.contains(col, row) || self.rows == 0 {
            return None;
        }
        let column = if self.columns <= 1 {
            0
        } else {
            let width = (self.area.w as usize / self.columns).max(4);
            ((col - self.area.x) as usize / width).min(self.columns - 1)
        };
        let down = (row - self.area.y) as usize;
        if down >= self.rows {
            return None;
        }
        let index = self.top + column * self.rows + down;
        (index < self.len).then_some(index)
    }
}

/// Which function key is at column `col` of a key bar showing `keys` across `width` columns.
///
/// The bar is drawn in equal slots, at least four columns wide each — the same arithmetic as
/// `widgets::draw_keybar`, and the one place where the two have to agree.
pub fn key_cell(col: u16, keys: &[u8], width: u16) -> Option<u8> {
    if keys.is_empty() {
        return None;
    }
    let slot = (width as usize / keys.len()).max(4);
    keys.get(col as usize / slot).copied()
}

/// Which menu title is at column `col`, given where each one starts.
pub fn menu_cell(col: u16, starts: &[u16]) -> Option<usize> {
    if starts.is_empty() || col < starts[0] {
        return None;
    }
    // The last title that starts at or before this column.
    starts
        .iter()
        .rposition(|start| *start <= col)
        .filter(|i| *i < starts.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel(x: u16, top: usize, columns: usize, len: usize) -> PanelArea {
        PanelArea {
            area: Rect::new(x, 2, 40, 10),
            top,
            rows: 10,
            columns,
            len,
        }
    }

    #[test]
    fn a_click_on_a_row_is_that_row_whichever_panel_it_is_in() {
        let layout = Layout {
            panels: [Some(panel(0, 0, 1, 40)), Some(panel(40, 5, 1, 40))],
            ..Layout::default()
        };
        assert_eq!(layout.hit(3, 2), Some(Hit::Row { side: 0, index: 0 }));
        assert_eq!(layout.hit(3, 6), Some(Hit::Row { side: 0, index: 4 }));
        // The right panel is scrolled, so the same row is a different entry.
        assert_eq!(layout.hit(45, 2), Some(Hit::Row { side: 1, index: 5 }));
        // Above the list and below it is nothing at all.
        assert_eq!(layout.hit(3, 1), None);
        assert_eq!(layout.hit(3, 12), None);
    }

    #[test]
    fn a_click_past_the_last_entry_is_the_panel_and_not_a_row() {
        let layout = Layout {
            panels: [Some(panel(0, 0, 1, 3)), None],
            ..Layout::default()
        };
        assert_eq!(layout.hit(3, 4), Some(Hit::Row { side: 0, index: 2 }));
        // The fourth row of a three-entry listing is the panel's own empty space: clicking it
        // focuses the panel and leaves the cursor where it was.
        assert_eq!(layout.hit(3, 5), Some(Hit::Panel { side: 0 }));
    }

    #[test]
    fn the_columns_of_a_brief_view_run_down_and_then_across() {
        // Three columns of ten rows: the first column is entries 0..9, the second 10..19.
        let layout = Layout {
            panels: [Some(panel(0, 0, 3, 30)), None],
            ..Layout::default()
        };
        assert_eq!(layout.hit(1, 2), Some(Hit::Row { side: 0, index: 0 }));
        assert_eq!(layout.hit(14, 2), Some(Hit::Row { side: 0, index: 10 }));
        assert_eq!(layout.hit(27, 11), Some(Hit::Row { side: 0, index: 29 }));
    }

    #[test]
    fn the_key_bar_divides_into_equal_slots() {
        // Twelve keys across eighty columns: six and a bit each, so the slot is six.
        let all: Vec<u8> = (1..=12).collect();
        assert_eq!(key_cell(0, &all, 80), Some(1));
        assert_eq!(key_cell(5, &all, 80), Some(1));
        assert_eq!(key_cell(6, &all, 80), Some(2));
        assert_eq!(key_cell(66, &all, 80), Some(12));
        // Past the last slot, and a bar with nothing on it.
        assert_eq!(key_cell(79, &all, 80), None);
        assert_eq!(key_cell(0, &[], 80), None);
        // A narrow terminal: the slot never goes below four columns, as the drawing does not.
        assert_eq!(key_cell(4, &all, 20), Some(2));

        // Holding a modifier shows only the keys that modifier has, so the cell says which
        // key is written in it rather than counting from one.
        // Four keys across eighty columns is twenty each.
        let shifted = [2u8, 5, 6, 8];
        assert_eq!(key_cell(0, &shifted, 80), Some(2));
        assert_eq!(key_cell(19, &shifted, 80), Some(2));
        assert_eq!(key_cell(20, &shifted, 80), Some(5));
        assert_eq!(key_cell(79, &shifted, 80), Some(8));
        assert_eq!(key_cell(80, &shifted, 80), None, "past the end of the bar");
    }

    #[test]
    fn a_menu_title_is_the_last_one_that_starts_before_the_click() {
        let starts = [2u16, 10, 18, 26];
        assert_eq!(menu_cell(2, &starts), Some(0));
        assert_eq!(menu_cell(9, &starts), Some(0));
        assert_eq!(menu_cell(10, &starts), Some(1));
        assert_eq!(menu_cell(30, &starts), Some(3));
        // To the left of the first title there is no title.
        assert_eq!(menu_cell(0, &starts), None);
        assert_eq!(menu_cell(5, &[]), None);
    }

    #[test]
    fn an_open_list_takes_the_clicks_that_land_on_it_and_swallows_the_rest() {
        let layout = Layout {
            panels: [Some(panel(0, 0, 1, 40)), None],
            list: Some(ListArea {
                area: Rect::new(10, 4, 20, 5),
                top: 3,
                len: 9,
            }),
            ..Layout::default()
        };
        assert_eq!(layout.hit(12, 4), Some(Hit::ListRow(3)));
        assert_eq!(layout.hit(12, 8), Some(Hit::ListRow(7)));
        // Past the end of the list's own rows, and outside the list: nothing, and in
        // particular not the panel underneath — a click outside an open dialog must not move
        // a cursor the user cannot see.
        assert_eq!(layout.hit(12, 9), None);
        assert_eq!(layout.hit(2, 5), None);
    }

    #[test]
    fn the_viewer_and_the_editor_report_the_cell_that_was_clicked() {
        let layout = Layout {
            text: Some(Rect::new(0, 1, 80, 20)),
            ..Layout::default()
        };
        assert_eq!(layout.hit(0, 1), Some(Hit::Text { col: 0, row: 0 }));
        assert_eq!(layout.hit(10, 5), Some(Hit::Text { col: 10, row: 4 }));
        assert_eq!(layout.hit(10, 0), None);
    }

    #[test]
    fn the_bars_are_above_everything_else() {
        let layout = Layout {
            panels: [Some(panel(0, 0, 1, 40)), None],
            key_bar: Some((23, (1..=12).collect())),
            menu_bar: Some((0, vec![2, 10, 18])),
            command_line: Some(22),
            ..Layout::default()
        };
        assert_eq!(layout.hit(0, 23), Some(Hit::Key(1)));
        assert_eq!(layout.hit(11, 0), Some(Hit::Menu(1)));
        assert_eq!(layout.hit(4, 22), Some(Hit::CommandLine));
        assert_eq!(layout.hit(4, 3), Some(Hit::Row { side: 0, index: 1 }));
    }
}
