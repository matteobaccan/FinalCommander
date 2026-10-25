use std::time::SystemTime;

use chrono::{DateTime, Local};
use fcmd_core::{
    format_count, format_size, Entry, EntryFlags, EntryKind, Listing, Selection, ViewMode,
};
use unicode_width::UnicodeWidthStr;

use crate::buffer::{elide_middle, truncate_cols, Border, Buffer, Rect, Style};
use crate::theme::Theme;

/// The heading of each column of a panel, already translated.
#[derive(Debug, Clone, Copy)]
pub struct ColumnTitles<'a> {
    pub name: &'a str,
    pub size: &'a str,
    pub allocated: &'a str,
    /// What the full view calls its one date column: a date and a time in one heading.
    pub date_time: &'a str,
    /// What the detailed view calls the same column when it has three of them.
    pub modified: &'a str,
    pub created: &'a str,
    pub accessed: &'a str,
    pub attr: &'a str,
}

/// Everything the panel widget needs to know. The panel owns no state of its own.
pub struct PanelRender<'a> {
    pub area: Rect,
    pub path: &'a str,
    pub listing: &'a Listing,
    pub selection: &'a Selection,
    pub cursor: usize,
    pub top: usize,
    pub focused: bool,
    pub view: ViewMode,
    pub sort_label: &'a str,
    /// Ascending or descending, for the letter in the corner.
    pub sort_reverse: bool,
    /// Draw the column titles row (FAR's `Show column titles`, on by default).
    pub column_titles: bool,
    pub filter: Option<&'a str>,
    pub message: Option<&'a str>,
    /// The item under the cursor, spelled out in the panel's own bottom row. `None` keeps
    /// the row out of the layout entirely (FAR's `Show status line`).
    pub status: Option<&'a str>,
    /// Free bytes on the device the panel is on, for the bottom frame.
    pub free: Option<u64>,
    /// Draw the bar in the right frame when the listing is longer than the panel.
    pub scrollbar: bool,
    /// Columns the name column is scrolled by, for names wider than it.
    pub name_scroll: usize,
    /// Which group decides a row's colour (FAR's default highlighting).
    pub highlight: &'a crate::Highlight,
    /// What the size column says for `..` and for a directory, localised by the caller
    /// (FAR's `MListUp` and `MListFolder`).
    /// The allocation unit of the device the panel is on, in bytes, or zero where nothing
    /// said: it is what the detailed view's allocated column is computed from.
    pub cluster: u64,
    /// What the columns are called, in the language the program is running in. Passed in
    /// like the two words below rather than looked up here: this crate draws and does not
    /// know about catalogues.
    pub titles: ColumnTitles<'a>,
    pub up_label: &'a str,
    pub folder_label: &'a str,
}

/// Rows that fit in a panel of this height. The titles row and the panel's own status line
/// each cost one row, and the status line costs a second for the rule above it.
pub fn panel_rows(area: Rect, column_titles: bool, status_line: bool) -> usize {
    let chrome = 2 + usize::from(column_titles) + if status_line { 2 } else { 0 };
    (area.h as usize).saturating_sub(chrome)
}

/// Columns of rows in a multi-column view: the mode's own number, less when the panel is too
/// narrow to give each column something readable.
pub fn brief_columns(area: Rect, view: ViewMode) -> usize {
    let inner = area.inner().w as usize;
    let least = if view.shows_size() { 24 } else { 14 };
    (inner / least).clamp(1, view.columns())
}

pub fn draw_panel(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>) {
    let area = p.area;
    if area.w < 8 || area.h < 3 {
        return;
    }
    let base = theme.panel();
    draw_pane(buf, theme, area, p.path, p.focused);

    let mut inner = area.inner();
    let rows = panel_rows(area, p.column_titles, p.status.is_some());
    if p.column_titles {
        draw_column_titles(buf, theme, p, inner);
        inner = Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1));
    } else {
        // Without a titles row the sort letter sits on the frame itself, as FAR does.
        draw_sort_mark(buf, theme, p, area.y);
    }

    if p.listing.is_empty() {
        buf.put_str(inner.x + 1, inner.y, "(empty)", base, inner.w);
    } else if p.view == ViewMode::Brief || p.view == ViewMode::Wide {
        draw_brief(buf, theme, p, inner, rows);
    } else {
        draw_full(buf, theme, p, inner, rows);
    }

    draw_row_rules(buf, theme, p, inner, rows);

    if let Some(status) = p.status {
        draw_panel_status(buf, theme, p, status);
    }
    if p.scrollbar {
        draw_scrollbar(buf, theme, p, inner, rows);
    }
    draw_footer(buf, theme, p);
    // Last, because it is drawn over whatever row it lands on: the rule when the panel has a
    // status line, the bottom frame when it has not. That is where FAR puts it too.
    let bar = if p.status.is_some() {
        p.area.bottom().saturating_sub(3)
    } else {
        p.area.bottom().saturating_sub(1)
    };
    draw_selected_bar(buf, theme, p, bar);
}

/// Where the vertical rules between the columns go. FAR runs them the whole height of the
/// list rather than only under the titles, which is what makes the columns read as columns.
fn column_rules(p: &PanelRender<'_>, inner: Rect) -> Vec<u16> {
    let mut xs = match p.view {
        ViewMode::Brief | ViewMode::Wide => {
            let cols = brief_columns(p.area, p.view);
            let col_w = (inner.w as usize / cols).max(4);
            (1..cols)
                .map(|col| inner.x + (col * col_w) as u16 - 1)
                .collect()
        }
        _ => {
            let c = columns_of(inner.w, p.view, p.cluster);
            // One rule at the end of every column that has something after it.
            let mut xs = Vec::new();
            let mut x = inner.x + c.name;
            for w in [c.size, c.allocated, c.write, c.created, c.accessed, c.attr] {
                xs.push(x);
                if w == 0 {
                    break;
                }
                x += w + 1;
            }
            xs.pop();
            xs
        }
    };
    xs.retain(|x| *x > inner.x && *x < inner.right());
    xs
}

/// The rules themselves, down the rows of the list.
///
/// Each one keeps the background of the row it crosses, so the cursor bar and a selected row
/// run through it unbroken — FAR's `HighlightBorder` does the same thing for the same reason.
fn draw_row_rules(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let fg = theme.color("panel.frame");
    let bg = theme.color("panel.bg");
    for x in column_rules(p, inner) {
        for row in 0..rows.min(inner.h as usize) {
            let y = inner.y + row as u16;
            let behind = buf.get(x, y).map_or(bg, |c| c.style.bg);
            buf.set(x, y, '│', Style::new(fg, behind));
        }
    }
}

/// `Bytes: 20 879, files: 2, folders: 0`, centred and on its own background.
///
/// FAR gives the selected totals a line of their own rather than a corner of the footer
/// (`ShowSelectedSize` in `filelist.cpp`), which is what makes a selection impossible to miss
/// from across the room.
fn draw_selected_bar(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, y: u16) {
    let t = &p.listing.totals;
    if t.selected_files == 0 {
        return;
    }
    let text = format!(
        " Bytes: {}, files: {}, folders: {} ",
        format_count(t.selected_bytes),
        format_count(t.selected_files - t.selected_dirs),
        format_count(t.selected_dirs)
    );
    let room = p.area.w.saturating_sub(4) as usize;
    let text = truncate_cols(&text, room);
    let tw = text.width() as u16;
    let style = Style::new(
        theme.color("panel.info.selected.fg"),
        theme.color("panel.info.selected.bg"),
    );
    let x = p.area.x + 2 + (room as u16).saturating_sub(tw) / 2;
    buf.put_str(x, y, &text, style, tw);
}

/// A panel-shaped frame with its title and nothing in it: the frame every panel has, and the
/// one the special panel kinds (quick view, info) draw their own content inside. Returns the
/// area left over for that content.
pub fn draw_pane(buf: &mut Buffer, theme: &Theme, area: Rect, title: &str, focused: bool) -> Rect {
    let base = theme.panel();
    buf.fill(area, ' ', base);
    let frame = Style::new(theme.color("panel.frame"), theme.color("panel.bg"));
    // FAR draws the panels themselves with a double line, and everything laid over them
    // with a single one.
    buf.draw_bordered(area, frame, None, Border::Double);

    // The title is elided in the middle, so both ends of a long path stay readable.
    let style = if focused {
        Style::new(
            theme.color("panel.title.active.fg"),
            theme.color("panel.title.active.bg"),
        )
    } else {
        Style::new(
            theme.color("panel.header.inactive.fg"),
            theme.color("panel.header.bg"),
        )
    };
    let room = area.w.saturating_sub(4) as usize;
    let title = elide_middle(title, room);
    let tw = title.width() as u16;
    let tx = area.x + 1 + (area.w.saturating_sub(2).saturating_sub(tw + 2)) / 2;
    buf.put_str(tx, area.y, &format!(" {title} "), style, tw + 2);
    area.inner()
}

/// FAR's own status line: a rule across the panel and, under it, the item the cursor is on.
/// It is the panel's row rather than the screen's, which is why it is drawn inside the frame.
fn draw_panel_status(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, status: &str) {
    let area = p.area;
    if area.h < 6 {
        return;
    }
    let frame = Style::new(theme.color("panel.frame"), theme.color("panel.bg"));
    let rule = area.bottom() - 3;
    buf.set(area.x, rule, '├', frame);
    for x in area.x + 1..area.right() - 1 {
        buf.set(x, rule, '─', frame);
    }
    buf.set(area.right() - 1, rule, '┤', frame);
    for x in column_rules(p, area.inner()) {
        buf.set(x, rule, '┴', frame);
    }
    let style = theme.panel();
    let w = area.w.saturating_sub(2);
    buf.fill(Rect::new(area.x + 1, rule + 1, w, 1), ' ', style);
    buf.put_str(
        area.x + 1,
        rule + 1,
        &truncate_cols(status, w as usize),
        style,
        w,
    );
}

/// The bar in the right frame: a thumb as tall as the share of the listing on screen, in the
/// place FAR draws it, and nothing at all when everything fits.
fn draw_scrollbar(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let total = p.listing.len();
    let per_screen = rows.max(1)
        * if p.view == ViewMode::Brief || p.view == ViewMode::Wide {
            brief_columns(p.area, p.view)
        } else {
            1
        };
    if total <= per_screen || inner.h == 0 {
        return;
    }
    let style = Style::new(theme.color("panel.scrollbar.fg"), theme.color("panel.bg"));
    let x = p.area.right() - 1;
    let track = inner.h as usize;
    let thumb = ((per_screen * track) / total).clamp(1, track);
    // Where the first visible row sits in what is left to scroll through. `span` cannot be
    // zero here — everything fitting on screen returned above — but say so with the maximum
    // rather than with a branch.
    let span = total.saturating_sub(per_screen).max(1);
    let at = (p.top.min(span) * (track - thumb) + span / 2) / span;
    for i in 0..track {
        let ch = if i >= at && i < at + thumb {
            '█'
        } else {
            '░'
        };
        buf.set(x, inner.y + i as u16, ch, style);
    }
}

/// The shadow a floating window drops: two columns to the right and one row below, which is
/// the geometry FAR uses (`DropShadow` in `interf.cpp`).
pub fn drop_shadow(buf: &mut Buffer, area: Rect) {
    buf.shadow(Rect::new(area.x + 2, area.bottom(), area.w, 1));
    buf.shadow(Rect::new(area.right(), area.y + 1, 2, area.h));
}

/// Centre a column title in its column, the way FAR does.
fn centre(buf: &mut Buffer, x: u16, y: u16, w: u16, text: &str, style: Style) {
    let tw = text.width() as u16;
    if tw >= w {
        buf.put_str(x, y, &truncate_cols(text, w as usize), style, w);
        return;
    }
    buf.put_str(x + (w - tw) / 2, y, text, style, tw);
}

/// The sort-mode letter, and a `*` when a filter is on: lower case for ascending order and
/// upper case for descending, in the column-title colour, exactly where FAR puts it.
fn draw_sort_mark(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, y: u16) {
    let style = Style::new(theme.color("panel.column.fg"), theme.color("panel.bg"));
    let letter = p.sort_label.chars().next().unwrap_or('n');
    let letter = if p.sort_reverse {
        letter.to_ascii_uppercase()
    } else {
        letter.to_ascii_lowercase()
    };
    buf.set(p.area.x + 1, y, letter, style);
    if p.filter.is_some() {
        buf.set(p.area.x + 2, y, '*', style);
    }
}

/// `Name | Size | Date | Time` under the top frame, with the sort letter at its left.
fn draw_column_titles(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect) {
    let style = Style::new(theme.color("panel.column.fg"), theme.color("panel.bg"));
    let y = inner.y;
    buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);
    draw_sort_mark(buf, theme, p, y);
    let frame = Style::new(theme.color("panel.frame"), theme.color("panel.bg"));
    for x in column_rules(p, inner) {
        buf.set(x, y, '│', frame);
    }
    match p.view {
        ViewMode::Brief | ViewMode::Wide => {
            let cols = brief_columns(p.area, p.view);
            let col_w = (inner.w as usize / cols).max(4);
            for col in 0..cols {
                let x = inner.x + (col * col_w) as u16;
                centre(buf, x, y, col_w as u16 - 1, p.titles.name, style);
            }
        }
        _ => {
            let c = columns_of(inner.w, p.view, p.cluster);
            // With one date column it is the panel's date, as the full view has always
            // titled it; with three they have to say which is which.
            let write = if c.created > 0 {
                p.titles.modified
            } else {
                p.titles.date_time
            };
            let mut x = inner.x;
            centre(buf, x, y, c.name, p.titles.name, style);
            x += c.name + 1;
            for (w, title) in [
                (c.size, p.titles.size),
                (c.allocated, p.titles.allocated),
                (c.write, write),
                (c.created, p.titles.created),
                (c.accessed, p.titles.accessed),
                (c.attr, p.titles.attr),
            ] {
                if w == 0 {
                    continue;
                }
                centre(buf, x, y, w, title, style);
                x += w + 1;
            }
        }
    }
}

/// The colour of one row: the highlighting group decides, then the selection, then the
/// cursor — the order FAR applies them in.
///
/// Nothing here is bold. FAR draws with console attributes, which have no bold, so a bold
/// face would be a difference anyone comparing the two would see; the colour carries the
/// meaning on its own.
fn row_style(
    theme: &Theme,
    highlight: &crate::Highlight,
    e: &Entry,
    selected: bool,
    cursor: bool,
    focused: bool,
) -> Style {
    let fg = if selected {
        theme.color("selected.fg")
    } else {
        theme.color(highlight.key(e))
    };
    if cursor {
        let (cfg, cbg) = if focused {
            ("cursor.fg", "cursor.bg")
        } else {
            ("cursor.inactive.fg", "cursor.inactive.bg")
        };
        let mut s = Style::new(theme.color(cfg), theme.color(cbg));
        if selected {
            // FAR's Panel.Cursor.Selected: yellow on the cursor bar.
            s.fg = theme.color("selected.fg");
        } else if !focused {
            // The passive panel's cursor is not a bar, so the file keeps its own colour.
            s.fg = theme.color(highlight.key(e));
        }
        s
    } else {
        Style::new(fg, theme.color("panel.bg"))
    }
}

fn draw_brief(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let cols = brief_columns(p.area, p.view);
    let col_w = (inner.w as usize / cols).max(4);
    // FAR's wide view puts the size at the right-hand end of each column.
    let size_w: u16 = if p.view.shows_size() { 9 } else { 0 };
    for col in 0..cols {
        for row in 0..rows {
            let idx = p.top + col * rows + row;
            let Some(e) = p.listing.row(idx) else {
                continue;
            };
            let selected = p.selection.contains(e.id);
            let style = row_style(theme, p.highlight, e, selected, idx == p.cursor, p.focused);
            let x = inner.x + (col * col_w) as u16;
            let w = col_w as u16 - 1;
            buf.fill(Rect::new(x, inner.y + row as u16, w, 1), ' ', style);
            let name_w = w.saturating_sub(size_w);
            let name = shift_name(&decorate(e), p.name_scroll);
            buf.put_str(
                x,
                inner.y + row as u16,
                &truncate_cols(&name, name_w as usize),
                style,
                name_w,
            );
            if size_w > 0 {
                // A directory says so in words, as it does in the full view, rather than
                // showing a size that means nothing.
                let text = match e.kind {
                    EntryKind::Parent => p.up_label.to_string(),
                    EntryKind::Dir => p.folder_label.to_string(),
                    _ => format_size(e.size),
                };
                put_right(buf, x + name_w, inner.y + row as u16, size_w, &text, style);
            }
        }
    }
}

/// A name shifted sideways by `by` columns, for a column narrower than the names. Scrolling
/// past the end of a short name would blank its row, so a name that would disappear entirely
/// keeps its last column on screen.
fn shift_name(name: &str, by: usize) -> String {
    if by == 0 {
        return name.to_string();
    }
    let total = name.chars().count();
    let by = by.min(total.saturating_sub(1));
    name.chars().skip(by).collect()
}

/// What the extension column takes out of the name field in the full view: four columns for
/// the extension itself and one that keeps it off the end of the name. Four because `toml`,
/// `json`, `html` and `yaml` are the longest extensions common enough to size a column for;
/// anything longer stays in the name, where it is read as part of it anyway.
const EXT_W: u16 = 5;

/// A name split the way the full view draws it: what goes in the name field, and the
/// extension that goes in the column of its own, right-aligned against the size.
///
/// `None` for everything that has no extension to speak of — a directory (a folder named
/// `docs.old` is not a `.old` file), a dotfile, whose leading dot starts a name rather than
/// an extension, a trailing dot, and an extension too long for the column. In each of those
/// the name is drawn whole, across both fields.
fn split_ext<'a>(name: &'a str, e: &Entry) -> Option<(&'a str, &'a str)> {
    if e.opens_as_dir() || matches!(e.kind, EntryKind::Dir | EntryKind::Parent) {
        return None;
    }
    let cut = name.rfind('.')?;
    let (stem, dotted) = name.split_at(cut);
    let ext = &dotted[1..];
    if stem.is_empty() || ext.is_empty() || ext.width() > (EXT_W - 1) as usize {
        return None;
    }
    Some((stem, ext))
}

/// The columns of the one-column views, left to right, with the width each takes. A width of
/// zero is a column this view or this panel has no room for. The titles row, the rules
/// between the columns and the file rows all read this, so they cannot drift apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Cols {
    name: u16,
    size: u16,
    /// What the file takes on the device. Only a device that has said how big its clusters
    /// are can answer it, so a remote panel has no column here rather than a blank one.
    allocated: u16,
    /// When the file was last written: the column the full view has always had.
    write: u16,
    /// When it was made, and when it was last read. The detailed view shows them when the
    /// panel is wide enough to keep a readable name as well — FAR's own detailed row.
    created: u16,
    accessed: u16,
    attr: u16,
}

const SIZE_W: u16 = 9;
const TIME_W: u16 = 16; // `2026-10-23 17:42`
const ATTR_W: u16 = 5;
/// Under this the name is no longer worth reading, so the dates that are extras go instead.
const NAME_MIN: u16 = 16;

impl Cols {
    /// What everything to the right of the name takes, the one-column gaps included.
    fn tail(&self) -> u16 {
        let widths = [
            self.size,
            self.allocated,
            self.write,
            self.created,
            self.accessed,
            self.attr,
        ];
        widths.iter().sum::<u16>() + widths.iter().filter(|w| **w > 0).count() as u16
    }
}

fn columns_of(inner_w: u16, view: ViewMode, cluster: u64) -> Cols {
    let detail = view == ViewMode::Detail;
    let mut c = Cols {
        name: 0,
        size: SIZE_W,
        allocated: 0,
        write: TIME_W,
        created: 0,
        accessed: 0,
        attr: if detail { ATTR_W } else { 0 },
    };
    // The extras of the detailed view, in the order they are drawn and one at a time: a panel
    // that is not wide enough for the next one keeps the name readable instead. A half-width
    // panel therefore shows what the full view shows with the attributes after it, and a
    // maximised one shows FAR's whole row.
    if detail {
        #[derive(Clone, Copy)]
        enum Extra {
            Allocated,
            Created,
            Accessed,
        }
        for extra in [Extra::Allocated, Extra::Created, Extra::Accessed] {
            let mut wider = c;
            match extra {
                // Nothing to put in it: the device never said how big its clusters are.
                Extra::Allocated if cluster == 0 => continue,
                Extra::Allocated => wider.allocated = SIZE_W,
                Extra::Created => wider.created = TIME_W,
                Extra::Accessed => wider.accessed = TIME_W,
            }
            if inner_w.saturating_sub(wider.tail()) < NAME_MIN {
                break;
            }
            c = wider;
        }
    }
    c.name = inner_w.saturating_sub(c.tail()).max(4);
    c
}

fn draw_full(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let c = columns_of(inner.w, p.view, p.cluster);
    let (name_w, size_w) = (c.name, c.size);

    for row in 0..rows {
        let idx = p.top + row;
        let Some(e) = p.listing.row(idx) else {
            continue;
        };
        let y = inner.y + row as u16;
        let selected = p.selection.contains(e.id);
        let style = row_style(theme, p.highlight, e, selected, idx == p.cursor, p.focused);
        buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);

        let name = shift_name(&decorate(e), p.name_scroll);
        // The extension in a column of its own, as FAR draws the full view: the names line up
        // down the left and the extensions down the right of the same field. Not while the
        // names are being scrolled sideways — the field is then a window onto one long name
        // and there is no room to keep a column in it — and not in the detailed view, which
        // has columns of its own after the time.
        let split = if p.view == ViewMode::Full && p.name_scroll == 0 && name_w > EXT_W + 3 {
            split_ext(&name, e)
        } else {
            None
        };
        match split {
            Some((stem, ext)) => {
                let stem_w = name_w - EXT_W;
                buf.put_str(
                    inner.x,
                    y,
                    &truncate_cols(stem, stem_w as usize),
                    style,
                    stem_w,
                );
                put_right(buf, inner.x + stem_w, y, EXT_W, ext, style);
            }
            None => {
                buf.put_str(
                    inner.x,
                    y,
                    &truncate_cols(&name, name_w as usize),
                    style,
                    name_w,
                );
            }
        }

        let sx = inner.x + name_w + 1;
        // A link to a directory says `Folder` too: its own size is the size of the link, a
        // number that means nothing to anybody, and what the row opens as is a directory.
        // The `@` on the name is what says it is a link.
        match if e.opens_as_dir() && e.kind != EntryKind::Parent {
            EntryKind::Dir
        } else {
            e.kind
        } {
            // A word, not a number: FAR centres it in a field as wide as the longer of the
            // two words, so `Up` and `Folder` line up with each other down the column
            // whatever the language calls them.
            EntryKind::Parent | EntryKind::Dir => {
                let label = if e.kind == EntryKind::Parent {
                    p.up_label
                } else {
                    p.folder_label
                };
                let field = p
                    .up_label
                    .width()
                    .max(p.folder_label.width())
                    .min(size_w as usize);
                let text = truncate_cols(label, field);
                let pad = (field - text.width()) / 2;
                let x = sx + size_w.saturating_sub(field as u16) + pad as u16;
                buf.put_str(x, y, &text, style, size_w);
            }
            _ => put_right(buf, sx, y, size_w, &format_size(e.size), style),
        }

        let mut x = sx + size_w + 1;
        if c.allocated > 0 {
            // A directory has no size of its own to round up, so it has nothing here either.
            if !e.opens_as_dir() && e.kind != EntryKind::Parent {
                let taken = fcmd_core::allocated(e.size, p.cluster);
                put_right(buf, x, y, c.allocated, &format_size(taken), style);
            }
            x += c.allocated + 1;
        }

        // The dates, in the order FAR puts them in, and only the ones this panel has room
        // for. A filesystem that does not keep one of them leaves the column blank rather
        // than inventing an epoch.
        for (w, when) in [
            (c.write, e.mtime),
            (c.created, e.btime),
            (c.accessed, e.atime),
        ] {
            if w == 0 {
                continue;
            }
            let text = when.map(format_mtime).unwrap_or_default();
            buf.put_str(x, y, &truncate_cols(&text, w as usize), style, w);
            x += w + 1;
        }

        if c.attr > 0 {
            buf.put_str(x, y, &attrs(e), style, c.attr);
        }
    }
}

fn put_right(buf: &mut Buffer, x: u16, y: u16, w: u16, text: &str, style: Style) {
    let text = truncate_cols(text, w as usize);
    let pad = w.saturating_sub(text.width() as u16);
    buf.put_str(x + pad, y, &text, style, w);
}

fn decorate(e: &Entry) -> String {
    let name = e.name_str().into_owned();
    match e.kind {
        // No trailing slash: the size column says `Folder`, and FAR leaves the name alone.
        EntryKind::Dir => name,
        EntryKind::Symlink => format!("{name}@"),
        _ if e.flags.contains(EntryFlags::EXECUTABLE) => format!("{name}*"),
        _ => name,
    }
}

fn attrs(e: &Entry) -> String {
    let mut s = String::new();
    s.push(match e.kind {
        EntryKind::Dir | EntryKind::Parent => 'd',
        EntryKind::Symlink => 'l',
        _ => '-',
    });
    s.push(if e.flags.contains(EntryFlags::READONLY) {
        'r'
    } else {
        '-'
    });
    s.push(if e.flags.contains(EntryFlags::EXECUTABLE) {
        'x'
    } else {
        '-'
    });
    s.push(if e.flags.contains(EntryFlags::HIDDEN) {
        'h'
    } else {
        '-'
    });
    s.push(if e.flags.contains(EntryFlags::SYSTEM) {
        's'
    } else {
        '-'
    });
    s
}

/// `2026-10-23 17:42`, in local time.
pub fn format_mtime(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.format("%Y-%m-%d %H:%M").to_string()
}

/// `2026-10-23 17:42:05`, in local time: what the attributes dialog shows and takes back.
///
/// The panel's column has no room for the seconds and does not need them; a dialog that sets a
/// time does, or saving it unchanged would quietly round it down to the minute.
pub fn format_stamp(t: SystemTime) -> String {
    let dt: DateTime<Local> = t.into();
    dt.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// A time typed into a dialog, in local time. The seconds may be left off.
///
/// `None` for anything else, including an empty string — which is how a row the user did not
/// touch says "leave this alone".
pub fn parse_stamp(text: &str) -> Option<SystemTime> {
    use chrono::TimeZone;

    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let naive = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .or_else(|_| chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M"))
        .or_else(|_| {
            chrono::NaiveDate::parse_from_str(text, "%Y-%m-%d")
                .map(|d| d.and_hms_opt(0, 0, 0).expect("midnight exists"))
        })
        .ok()?;
    // A local time in the hour a clock goes forward does not exist, and one in the hour it
    // goes back happens twice; the earlier of the two is the answer that does not surprise.
    Local
        .from_local_datetime(&naive)
        .earliest()
        .map(SystemTime::from)
}

/// The bottom frame: how much is in the panel and how much room is left on the device,
/// centred the way FAR centres it (`ShowTotalSize` in `filelist.cpp`) — `14,1 M (6/0) ═══ 2,65 G`,
/// with the rule between the two drawn in the frame's own colour so the eye separates them.
fn draw_footer(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>) {
    let area = p.area;
    let y = area.bottom() - 1;
    let room = area.w.saturating_sub(4) as usize;
    let style = Style::new(theme.color("panel.footer.fg"), theme.color("panel.bg"));
    let frame = Style::new(theme.color("panel.frame"), theme.color("panel.bg"));

    let totals = &p.listing.totals;
    // A listing the provider cut short says so for as long as it is on screen. The note
    // explaining why is a message, and a message lasts until the next keystroke; this does
    // not, because "2034 files" is otherwise a number that reads like the whole truth.
    let partial = if p.listing.truncated {
        " (partial)"
    } else {
        ""
    };
    let (left, right) = if let Some(msg) = p.message {
        (msg.to_string(), None)
    } else if let Some(f) = p.filter {
        (format!("filter: {f}{partial}"), None)
    } else {
        let sorted = if p.column_titles {
            String::new()
        } else {
            // The sort mode is in the corner when the titles row is on, so it is not repeated.
            format!(" [{}]", p.sort_label)
        };
        (
            format!(
                "{} ({}/{}){sorted}{partial}",
                format_size(totals.bytes),
                format_count(totals.files),
                format_count(totals.dirs)
            ),
            p.free.map(format_size),
        )
    };

    // The free space is dropped rather than squeezed when the panel is too narrow for both.
    let rule = "═══";
    let full = match &right {
        Some(free) if left.width() + free.width() + rule.width() + 4 <= room => {
            format!(" {left} {rule} {free} ")
        }
        _ => format!(" {left} "),
    };
    let text = truncate_cols(&full, room);
    let tw = text.width() as u16;
    let x = area.x + 2 + (room as u16).saturating_sub(tw) / 2;
    let style = if p.filter.is_some() || p.message.is_some() {
        style.with_fg(theme.color("filter.fg"))
    } else {
        style
    };
    buf.put_str(x, y, &text, style, tw);
    // The piece of double line between the totals and the free space belongs to the frame,
    // not to the text, and FAR colours it as such.
    if right.is_some() {
        if let Some(at) = text.find(rule) {
            let before = text[..at].width() as u16;
            for i in 0..rule.chars().count() as u16 {
                buf.set(x + before + i, y, '═', frame);
            }
        }
    }
}

/// The `F1..F12` bar. `labels` is `(number, label)`, already localised by the caller.
///
/// Three colours, as FAR has them: the number in `Keybar.Num`, the label in `Keybar.Text`,
/// and whatever is left of the row in `Keybar.Background` — which is *not* the label colour,
/// so a bar with room to spare ends in the background rather than in a cyan tail.
pub fn draw_keybar(buf: &mut Buffer, theme: &Theme, area: Rect, labels: &[(u8, &str)]) {
    let bar = theme.keybar();
    let num = Style::new(theme.color("keybar.num.fg"), theme.color("keybar.num.bg"));
    let background = Style::new(
        theme.color("keybar.background.fg"),
        theme.color("keybar.background.bg"),
    );
    buf.fill(area, ' ', background);
    if labels.is_empty() {
        return;
    }
    let slot = (area.w as usize / labels.len()).max(4);
    for (i, (n, label)) in labels.iter().enumerate() {
        let x = area.x + (i * slot) as u16;
        if x >= area.right() {
            break;
        }
        let n_text = n.to_string();
        let used = buf.put_str(x, area.y, &n_text, num, 2);
        let avail = (slot as u16)
            .saturating_sub(used)
            .min(area.right() - x - used);
        // The label block is padded to its slot, so the bar reads as one strip broken by the
        // number boxes rather than as words floating on the background.
        let text = format!(
            "{:<width$}",
            truncate_cols(label, avail as usize),
            width = avail as usize
        );
        buf.put_str(x + used, area.y, &text, bar, avail);
    }
}

/// One-line status/message bar.
pub fn draw_status(buf: &mut Buffer, theme: &Theme, area: Rect, text: &str, error: bool) {
    let mut style = Style::new(theme.color("status.fg"), theme.color("status.bg"));
    if error {
        style.fg = theme.color("error.fg");
        style.bold = true;
    }
    buf.fill(area, ' ', style);
    buf.put_str(
        area.x + 1,
        area.y,
        &truncate_cols(text, area.w.saturating_sub(2) as usize),
        style,
        area.w.saturating_sub(2),
    );
}

/// The command line under the panels. Returns the screen column of the text cursor.
pub fn draw_cmdline(
    buf: &mut Buffer,
    theme: &Theme,
    area: Rect,
    prompt: &str,
    text: &str,
    cursor: usize,
) -> u16 {
    let style = Style::new(theme.color("cmdline.fg"), theme.color("cmdline.bg"));
    buf.fill(area, ' ', style);
    let pw = buf.put_str(area.x, area.y, prompt, style.bold(), area.w);
    let avail = area.w.saturating_sub(pw);
    buf.put_str(area.x + pw, area.y, text, style, avail);
    area.x + pw + (cursor as u16).min(avail.saturating_sub(1))
}

/// Centred message box. Returns the box area.
pub fn draw_dialog(
    buf: &mut Buffer,
    theme: &Theme,
    screen: Rect,
    title: &str,
    lines: &[String],
    hint: Option<&str>,
) -> Rect {
    let content_w = lines
        .iter()
        .map(|l| l.width())
        .chain(std::iter::once(title.width() + 4))
        .chain(hint.map(|h| h.width()))
        .max()
        .unwrap_or(20)
        .min(screen.w.saturating_sub(4) as usize);
    let h = (lines.len() + usize::from(hint.is_some()) + 2).min(screen.h as usize) as u16;
    let w = content_w as u16 + 4;
    let x = screen.x + (screen.w.saturating_sub(w)) / 2;
    let y = screen.y + (screen.h.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);

    let style = theme.dialog();
    buf.fill(area, ' ', style);
    buf.draw_frame(
        area,
        Style::new(theme.color("dialog.frame"), theme.color("dialog.bg")),
        Some(title),
    );
    for (i, line) in lines.iter().enumerate() {
        if (i as u16) + 1 >= area.h.saturating_sub(1) {
            break;
        }
        buf.put_str(area.x + 2, area.y + 1 + i as u16, line, style, w - 4);
    }
    if let Some(h) = hint {
        buf.put_str(
            area.x + 2,
            area.bottom() - 1,
            &truncate_cols(h, (w - 4) as usize),
            Style::new(theme.color("dialog.title"), theme.color("dialog.bg")),
            w - 4,
        );
    }
    drop_shadow(buf, area);
    area
}

/// Centred single-line input box. Returns the cursor column.
pub fn draw_prompt(
    buf: &mut Buffer,
    theme: &Theme,
    screen: Rect,
    title: &str,
    text: &str,
    cursor: usize,
) -> (Rect, u16, u16) {
    let w = (screen.w * 2 / 3).clamp(20, screen.w.saturating_sub(4));
    let h = 3;
    let x = screen.x + (screen.w.saturating_sub(w)) / 2;
    let y = screen.y + (screen.h.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);
    let style = theme.dialog();
    buf.fill(area, ' ', style);
    buf.draw_frame(
        area,
        Style::new(theme.color("dialog.frame"), theme.color("dialog.bg")),
        Some(title),
    );
    let field = Rect::new(area.x + 2, area.y + 1, w - 4, 1);
    let input = Style::new(theme.color("dialog.edit.fg"), theme.color("dialog.edit.bg"));
    buf.fill(field, ' ', input);
    let shown = truncate_cols(text, field.w as usize);
    buf.put_str(field.x, field.y, &shown, input, field.w);
    drop_shadow(buf, area);
    (
        area,
        field.x + (cursor as u16).min(field.w.saturating_sub(1)),
        field.y,
    )
}

/// A horizontal bar: `███████░░░░░░░  43%`.
pub fn draw_bar(buf: &mut Buffer, area: Rect, percent: u16, filled: Style, empty: Style) {
    if area.w < 6 {
        return;
    }
    let label = format!("{:>3}%", percent.min(100));
    let bar_w = area.w.saturating_sub(label.width() as u16 + 1);
    let done = (bar_w as u32 * percent.min(100) as u32 / 100) as u16;
    for x in 0..bar_w {
        let (ch, style) = if x < done {
            ('█', filled)
        } else {
            ('░', empty)
        };
        buf.set(area.x + x, area.y, ch, style);
    }
    buf.put_str(
        area.x + bar_w + 1,
        area.y,
        &label,
        empty,
        label.width() as u16,
    );
}

/// What the progress overlay shows. Strings are already localised by the caller.
pub struct ProgressView<'a> {
    pub title: &'a str,
    /// The item being worked on right now.
    pub current: &'a str,
    pub counters: &'a str,
    /// Speed, elapsed and what is left: the three numbers people actually watch. Empty keeps
    /// the row out of the box.
    pub rates: &'a str,
    pub total_percent: u16,
    pub file_percent: u16,
    /// Shown only when there is more than one file.
    pub show_file_bar: bool,
    pub hint: &'a str,
}

/// Centred progress box: current item, one or two bars, counters, and how to cancel.
pub fn draw_progress(
    buf: &mut Buffer,
    theme: &Theme,
    screen: Rect,
    view: &ProgressView<'_>,
) -> Rect {
    let w = (screen.w * 3 / 4).clamp(30, screen.w.saturating_sub(4));
    let h = 7 + u16::from(view.show_file_bar) + u16::from(!view.rates.is_empty());
    let h = h.min(screen.h);
    let x = screen.x + (screen.w.saturating_sub(w)) / 2;
    let y = screen.y + (screen.h.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);

    let style = theme.dialog();
    let accent = Style::new(theme.color("dialog.title"), theme.color("dialog.bg"));
    buf.fill(area, ' ', style);
    buf.draw_frame(
        area,
        Style::new(theme.color("dialog.frame"), theme.color("dialog.bg")),
        Some(view.title),
    );

    let inner = area.inner();
    let mut row = inner.y;
    buf.put_str(
        inner.x,
        row,
        &elide_middle(view.current, inner.w as usize),
        style,
        inner.w,
    );
    row += 1;

    if view.show_file_bar {
        draw_bar(
            buf,
            Rect::new(inner.x, row, inner.w, 1),
            view.file_percent,
            accent,
            style,
        );
        row += 1;
    }
    draw_bar(
        buf,
        Rect::new(inner.x, row, inner.w, 1),
        view.total_percent,
        accent,
        style,
    );
    row += 2;
    buf.put_str(
        inner.x,
        row,
        &truncate_cols(view.counters, inner.w as usize),
        style,
        inner.w,
    );
    if !view.rates.is_empty() {
        row += 1;
        buf.put_str(
            inner.x,
            row,
            &truncate_cols(view.rates, inner.w as usize),
            style,
            inner.w,
        );
    }
    buf.put_str(
        inner.x,
        area.bottom() - 1,
        &truncate_cols(view.hint, inner.w as usize),
        accent,
        inner.w,
    );
    drop_shadow(buf, area);
    area
}

/// A row of buttons, the selected one highlighted. Returns the columns each button starts at,
/// so a click can be mapped back to a choice.
pub fn draw_buttons(
    buf: &mut Buffer,
    theme: &Theme,
    area: Rect,
    labels: &[String],
    selected: usize,
) -> Vec<u16> {
    let normal = Style::new(
        theme.color("dialog.button.fg"),
        theme.color("dialog.button.bg"),
    );
    let active = Style::new(
        theme.color("dialog.button.selected.fg"),
        theme.color("dialog.button.selected.bg"),
    )
    .bold();
    let total: usize = labels.iter().map(|l| l.width() + 4).sum();
    let mut x = area.x + (area.w as usize).saturating_sub(total) as u16 / 2;
    let mut starts = Vec::with_capacity(labels.len());
    for (i, label) in labels.iter().enumerate() {
        let style = if i == selected { active } else { normal };
        let text = format!("[ {label} ]");
        let width = text.width() as u16;
        if x + width > area.right() {
            break;
        }
        starts.push(x);
        buf.put_str(x, area.y, &text, style, width);
        x += width;
    }
    starts
}

/// A dialog with message lines and a row of buttons (conflicts, errors, confirmations).
pub fn draw_choice(
    buf: &mut Buffer,
    theme: &Theme,
    screen: Rect,
    title: &str,
    lines: &[String],
    buttons: &[String],
    selected: usize,
) -> Rect {
    let buttons_w: usize = buttons.iter().map(|l| l.width() + 4).sum();
    let content_w = lines
        .iter()
        .map(|l| l.width())
        .chain(std::iter::once(buttons_w))
        .chain(std::iter::once(title.width() + 4))
        .max()
        .unwrap_or(24)
        .min(screen.w.saturating_sub(4) as usize);
    let w = content_w as u16 + 4;
    let h = (lines.len() as u16 + 4).min(screen.h);
    let x = screen.x + (screen.w.saturating_sub(w)) / 2;
    let y = screen.y + (screen.h.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);

    let style = theme.dialog();
    buf.fill(area, ' ', style);
    buf.draw_frame(
        area,
        Style::new(theme.color("dialog.frame"), theme.color("dialog.bg")),
        Some(title),
    );
    for (i, line) in lines.iter().enumerate() {
        if area.y + 1 + i as u16 >= area.bottom() - 2 {
            break;
        }
        buf.put_str(
            area.x + 2,
            area.y + 1 + i as u16,
            &truncate_cols(line, (w - 4) as usize),
            style,
            w - 4,
        );
    }
    draw_buttons(
        buf,
        theme,
        Rect::new(area.x + 1, area.bottom() - 2, w - 2, 1),
        buttons,
        selected,
    );
    drop_shadow(buf, area);
    area
}

#[cfg(test)]
mod tests {
    use super::mnemonic;

    #[test]
    fn the_marker_names_a_key_and_never_reaches_the_screen() {
        assert_eq!(mnemonic("&Medium"), ("Medium".into(), Some('M')));
        assert_eq!(mnemonic("M&acros"), ("Macros".into(), Some('a')));
        assert_eq!(mnemonic("Brief"), ("Brief".into(), None));
        // The first marker wins; a second one is a typo, not a second key.
        assert_eq!(mnemonic("&Save &as"), ("Save as".into(), Some('S')));
        // `&&` is how one writes an ampersand, as in every toolkit with this convention.
        assert_eq!(mnemonic("Find && replace"), ("Find & replace".into(), None));
        // A trailing marker marks nothing and is simply gone.
        assert_eq!(mnemonic("Quit&"), ("Quit".into(), None));
    }

    use super::*;
    use fcmd_core::{EntryId, Sort, SortKey};
    use std::ffi::OsString;

    /// What the program's own catalogue says in English, so a test reads like the screen.
    fn english_titles() -> ColumnTitles<'static> {
        ColumnTitles {
            name: "Name",
            size: "Size",
            allocated: "Allocated",
            date_time: "Date       Time",
            modified: "Modified",
            created: "Created",
            accessed: "Accessed",
            attr: "Attr",
        }
    }

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000)),
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn listing() -> Listing {
        Listing::new(
            vec![
                Entry::parent_entry(),
                entry("src", EntryKind::Dir, 0),
                entry("Cargo.toml", EntryKind::File, 412),
                entry("README.md", EntryKind::File, 2048),
            ],
            1,
        )
    }

    fn render(view: ViewMode, cursor: usize, selection: &Selection) -> String {
        render_listing(&listing(), view, cursor, selection)
    }

    /// The same panel on a screen wide enough for every column the detailed view has.
    fn render_wide(listing: &Listing, view: ViewMode, width: u16) -> String {
        render_device(listing, view, width, 4096)
    }

    /// With the device's allocation unit, or zero for a panel whose device said nothing.
    fn render_device(listing: &Listing, view: ViewMode, width: u16, cluster: u64) -> String {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(width, 10);
        let selection = Selection::default();
        let p = PanelRender {
            area: Rect::new(0, 0, width, 9),
            path: "/home/mb/project",
            listing,
            selection: &selection,
            cursor: 0,
            top: 0,
            focused: true,
            view,
            sort_label: "name",
            sort_reverse: false,
            column_titles: true,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        buf.to_text()
    }

    #[test]
    fn the_detailed_view_shows_the_dates_the_panel_has_room_for() {
        let mut made = entry("Cargo.toml", EntryKind::File, 412);
        made.btime = Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000));
        made.atime = Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_750_000_000));
        let listing = Listing::new(vec![Entry::parent_entry(), made], 1);

        // Wide: every column FAR's detailed row has, in FAR's order.
        let out = render_wide(&listing, ViewMode::Detail, 100);
        let titles = out.lines().nth(1).unwrap_or_default().to_string();
        let order: Vec<&str> = [
            "Name",
            "Size",
            "Allocated",
            "Modified",
            "Created",
            "Accessed",
            "Attr",
        ]
        .into_iter()
        .collect();
        let mut at = 0;
        for title in &order {
            let Some(i) = titles[at..].find(title) else {
                panic!("`{title}` is not in the titles row: {titles:?}");
            };
            at += i + title.len();
        }
        // And the three dates are drawn, not just titled.
        assert!(out.contains("2020-09-13"), "the created date: {out}");
        assert!(out.contains("2025-06-15"), "the accessed date: {out}");
        // 412 bytes of file in a 4 K cluster takes 4 K of device.
        assert!(out.contains("4.00 K"), "the allocated size: {out}");

        // A device that said nothing about its clusters has no column to put a figure in,
        // rather than a column of blanks: that is what a remote panel looks like.
        let out = render_device(&listing, ViewMode::Detail, 100, 0);
        let titles = out.lines().nth(1).unwrap_or_default().to_string();
        assert!(!titles.contains("Allocated"), "{titles:?}");
        assert!(titles.contains("Accessed"), "the dates stay: {titles:?}");

        // Narrow: the extras go rather than the name, and the date column is the panel's
        // own again.
        let out = render_wide(&listing, ViewMode::Detail, 50);
        let titles = out.lines().nth(1).unwrap_or_default().to_string();
        assert!(titles.contains("Date"), "{titles:?}");
        assert!(!titles.contains("Accessed"), "{titles:?}");
        assert!(titles.contains("Attr"), "the attributes stay: {titles:?}");
    }

    fn render_listing(
        listing: &Listing,
        view: ViewMode,
        cursor: usize,
        selection: &Selection,
    ) -> String {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(44, 10);
        let p = PanelRender {
            area: Rect::new(0, 0, 44, 9),
            path: "/home/mb/project",
            listing,
            selection,
            cursor,
            top: 0,
            focused: true,
            view,
            sort_label: "name",
            sort_reverse: false,
            column_titles: false,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster: 4096,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        buf.to_text()
    }

    fn tab_row(width: u16, labels: &[&str], current: usize, locked: &[bool]) -> String {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(width, 1);
        let labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();
        draw_tabs(
            &mut buf,
            &theme,
            &TabRow {
                area: Rect::new(0, 0, width, 1),
                labels: &labels,
                current,
                locked,
                focused: true,
            },
        );
        buf.to_text().trim_end().to_string()
    }

    #[test]
    fn the_tab_row_is_the_names_one_after_another() {
        let out = tab_row(40, &["src", "docs", "target"], 1, &[false; 3]);
        assert_eq!(out, " src  docs  target");
    }

    #[test]
    fn a_locked_tab_is_marked() {
        let out = tab_row(40, &["src", "docs"], 0, &[false, true]);
        assert_eq!(out, " src  \u{25aa}docs");
    }

    #[test]
    fn a_name_longer_than_the_room_for_one_is_cut() {
        let out = tab_row(
            40,
            &["a-directory-with-a-very-long-name", "docs"],
            0,
            &[false; 2],
        );
        assert!(out.contains('\u{2026}'), "{out}");
        assert!(out.ends_with(" docs"), "the second tab still fits: {out}");
    }

    #[test]
    fn the_row_scrolls_to_the_tab_that_is_on_screen() {
        let labels = ["one", "two", "three", "four", "five", "six"];
        // Narrow enough that they cannot all be drawn.
        let out = tab_row(20, &labels, 5, &[false; 6]);
        assert!(
            out.contains("six"),
            "the current tab is always whole: {out}"
        );
        assert!(out.starts_with('\u{2039}'), "and the row says so: {out}");
        assert!(!out.contains("one"), "{out}");
        // On the first tab the row starts at the left and says there is more to the right.
        let out = tab_row(20, &labels, 0, &[false; 6]);
        assert!(out.starts_with(" one "), "{out}");
        assert!(out.ends_with('\u{203a}'), "{out}");
    }

    #[test]
    fn the_full_view_puts_the_extension_in_a_column_of_its_own() {
        let listing = Listing::new(
            vec![
                Entry::parent_entry(),
                entry("docs.old", EntryKind::Dir, 0),
                entry("Cargo.toml", EntryKind::File, 412),
                entry(".gitignore", EntryKind::File, 40),
                entry("notes", EntryKind::File, 10),
                entry("db.backup", EntryKind::File, 10),
            ],
            1,
        );
        let out = render_listing(&listing, ViewMode::Full, 0, &Selection::default());
        // The name on the left, the extension right-aligned against the size column.
        assert!(out.contains("Cargo      toml"), "{out}");
        // A folder with a dot in its name is not a file with an extension.
        assert!(out.contains("docs.old"), "{out}");
        // A dotfile's leading dot starts its name; there is nothing to move right.
        assert!(out.contains(".gitignore"), "{out}");
        assert!(out.contains("notes"), "{out}");
        // An extension too long for the column stays where it was written.
        assert!(out.contains("db.backup"), "{out}");
        // In the other views the name is whole, extension and all.
        let brief = render_listing(&listing, ViewMode::Brief, 0, &Selection::default());
        assert!(brief.contains("Cargo.toml"), "{brief}");
    }

    #[test]
    fn full_view_shows_names_sizes_and_times() {
        let out = render(ViewMode::Full, 0, &Selection::default());
        assert!(out.contains("/home/mb/project"), "{out}");
        // A directory is named as it is: the size column is what says it is one.
        assert!(out.contains("src ") && !out.contains("src/"), "{out}");
        assert!(out.contains("Folder"), "{out}");
        assert!(out.contains("412 B"), "{out}");
        assert!(out.contains("2.00 K"), "{out}");
        assert!(!out.contains("22…"), "the time column is too narrow: {out}");
        // The bottom frame counts them the way FAR does: `(files/folders)`.
        assert!(out.contains("(2/1)"), "{out}");
    }

    #[test]
    fn brief_view_lays_names_out_in_columns() {
        let out = render(ViewMode::Brief, 0, &Selection::default());
        let body: Vec<&str> = out.lines().skip(1).take(4).collect();
        // With 4 entries and several columns, `..` and `src` share the first row.
        assert!(body[0].contains(".."), "{out}");
        assert!(
            out.contains("Cargo.toml") && out.contains("README.md"),
            "{out}"
        );
    }

    #[test]
    fn the_selected_totals_get_a_bar_of_their_own() {
        let listing = listing();
        let mut sel = Selection::default();
        let id = listing.row(2).unwrap().id;
        sel.toggle(id);
        let mut l2 = listing.clone();
        l2.rebuild(Sort::default(), false, None, &sel);
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(44, 9);
        let p = PanelRender {
            area: Rect::new(0, 0, 44, 9),
            path: "/x",
            listing: &l2,
            selection: &sel,
            cursor: 2,
            top: 0,
            focused: true,
            view: ViewMode::Full,
            sort_label: "name",
            sort_reverse: false,
            column_titles: false,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster: 4096,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        let out = buf.to_text();
        // FAR's own wording, centred on its own background rather than tucked into a corner.
        assert!(out.contains("Bytes: 412, files: 1, folders: 0"), "{out}");
    }

    /// A listing the provider cut short must not read like a complete one.
    #[test]
    fn a_partial_listing_says_so_in_the_footer() {
        let theme = Theme::default_theme();
        let mut listing = listing();
        let mut buf = Buffer::new(44, 9);
        let render = |listing: &Listing, buf: &mut Buffer| {
            let p = PanelRender {
                area: Rect::new(0, 0, 44, 9),
                path: "s3://logs/2024/",
                listing,
                selection: &Selection::default(),
                cursor: 0,
                top: 0,
                focused: true,
                view: ViewMode::Full,
                sort_label: "name",
                sort_reverse: false,
                column_titles: false,
                filter: None,
                message: None,
                status: None,
                free: None,
                scrollbar: false,
                name_scroll: 0,
                highlight: &crate::Highlight::far_default(),
                cluster: 4096,
                titles: english_titles(),
                up_label: "Up",
                folder_label: "Folder",
            };
            draw_panel(buf, &theme, &p);
            buf.to_text()
        };

        let whole = render(&listing, &mut buf);
        assert!(!whole.contains("partial"), "{whole}");

        listing.truncated = true;
        let mut buf = Buffer::new(44, 9);
        let cut = render(&listing, &mut buf);
        assert!(cut.contains("(partial)"), "{cut}");
        // The count is still there: the marker qualifies it, it does not replace it.
        assert!(cut.contains("(2/1)"), "{cut}");
    }

    #[test]
    fn a_long_path_is_elided_in_the_middle() {
        let theme = Theme::default_theme();
        let listing = listing();
        let mut buf = Buffer::new(30, 6);
        let p = PanelRender {
            area: Rect::new(0, 0, 30, 6),
            path: "/very/long/path/that/will/not/fit/in/thirty/columns",
            listing: &listing,
            selection: &Selection::default(),
            cursor: 0,
            top: 0,
            focused: false,
            view: ViewMode::Full,
            sort_label: "name",
            sort_reverse: false,
            column_titles: false,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster: 4096,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        let first = buf.to_lines()[0].clone();
        assert!(first.contains('…'), "{first}");
        assert!(first.width() <= 30, "{first}");
    }

    #[test]
    fn keybar_shows_numbers_and_labels() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(80, 1);
        let labels = [
            (1u8, "Help"),
            (2, "Menu"),
            (3, "View"),
            (4, "Edit"),
            (5, "Copy"),
            (6, "RenMov"),
            (7, "MkDir"),
            (8, "Delete"),
            (9, "PullDn"),
            (10, "Quit"),
        ];
        draw_keybar(&mut buf, &theme, Rect::new(0, 0, 80, 1), &labels);
        let out = buf.to_text();
        assert!(out.starts_with("1Help"), "{out}");
        assert!(out.contains("5Copy") && out.contains("10Quit"), "{out}");
    }

    #[test]
    fn dialog_is_centred_and_framed() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(40, 11);
        let screen = buf.area();
        let area = draw_dialog(
            &mut buf,
            &theme,
            screen,
            "Error",
            &["cannot read directory".to_string()],
            Some("Esc closes"),
        );
        assert!(area.x > 0 && area.y > 0);
        let out = buf.to_text();
        assert!(out.contains("Error"), "{out}");
        assert!(out.contains("cannot read directory"), "{out}");
        assert!(out.contains("Esc closes"), "{out}");
    }

    #[test]
    fn cjk_names_do_not_break_the_grid() {
        let theme = Theme::default_theme();
        let l = Listing::new(
            vec![
                Entry::parent_entry(),
                entry("文档目录", EntryKind::Dir, 0),
                entry("プロジェクト.txt", EntryKind::File, 1024),
            ],
            1,
        );
        let mut buf = Buffer::new(40, 8);
        let p = PanelRender {
            area: Rect::new(0, 0, 40, 8),
            path: "/x",
            listing: &l,
            selection: &Selection::default(),
            cursor: 1,
            top: 0,
            focused: true,
            view: ViewMode::Full,
            sort_label: "name",
            sort_reverse: false,
            column_titles: false,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster: 4096,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        for line in buf.to_lines() {
            assert!(line.width() <= 40, "line wider than the panel: {line:?}");
        }
    }

    #[test]
    fn the_progress_overlay_shows_two_bars_and_the_counters() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(60, 20);
        let screen = buf.area();
        let area = draw_progress(
            &mut buf,
            &theme,
            screen,
            &ProgressView {
                title: "Copying",
                current: "/home/mb/project/target/debug/deps/libfoo.rlib",
                counters: "12 / 40 files · 4.1 M / 22.7 M",
                rates: "",
                total_percent: 43,
                file_percent: 80,
                show_file_bar: true,
                hint: "Esc cancels",
            },
        );
        let out = buf.to_text();
        assert!(area.w >= 30);
        assert!(out.contains("Copying"), "{out}");
        assert!(out.contains("43%") && out.contains("80%"), "{out}");
        assert!(out.contains("12 / 40 files"), "{out}");
        assert!(out.contains("Esc cancels"), "{out}");
        assert!(out.contains('█') && out.contains('░'), "{out}");
    }

    #[test]
    fn a_single_file_job_shows_one_bar() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(50, 16);
        let screen = buf.area();
        let two = draw_progress(
            &mut buf,
            &theme,
            screen,
            &ProgressView {
                title: "Copying",
                current: "x",
                counters: "c",
                rates: "",
                total_percent: 10,
                file_percent: 10,
                show_file_bar: true,
                hint: "h",
            },
        );
        let one = draw_progress(
            &mut buf,
            &theme,
            screen,
            &ProgressView {
                title: "Copying",
                current: "x",
                counters: "c",
                rates: "",
                total_percent: 10,
                file_percent: 10,
                show_file_bar: false,
                hint: "h",
            },
        );
        assert_eq!(one.h + 1, two.h);
    }

    #[test]
    fn the_choice_dialog_highlights_the_selected_button() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(60, 14);
        let screen = buf.area();
        let buttons = vec![
            "Overwrite".to_string(),
            "All".to_string(),
            "Skip".to_string(),
            "Cancel".to_string(),
        ];
        draw_choice(
            &mut buf,
            &theme,
            screen,
            "The destination exists",
            &["destination  /tmp/a.txt   412 B".to_string()],
            &buttons,
            2,
        );
        let out = buf.to_text();
        assert!(out.contains("The destination exists"), "{out}");
        assert!(
            out.contains("[ Skip ]") && out.contains("[ Overwrite ]"),
            "{out}"
        );

        // The selected button is drawn with the selected-button colours, and the others are
        // not: a dialog where every button looks chosen is a dialog nobody can answer.
        let mut selected = 0;
        let mut plain = 0;
        for y in 0..buf.height() {
            for x in 0..buf.width() {
                let cell = buf.get(x, y).unwrap();
                if cell.ch != 'S' && cell.ch != 'O' {
                    continue;
                }
                if cell.style.bg == theme.color("dialog.button.selected.bg") {
                    selected += 1;
                } else if cell.style.bg == theme.color("dialog.button.bg") {
                    plain += 1;
                }
            }
        }
        assert_eq!(selected, 1, "expected exactly one highlighted button");
        assert!(plain >= 1, "the other buttons lost their colours");
    }

    #[test]
    fn a_dialog_drops_a_shadow_on_what_is_behind_it() {
        let theme = Theme::builtin("far").unwrap();
        let mut buf = Buffer::new(40, 12);
        // Something bright behind the dialog, so the darkening is visible.
        let bright = Style::new(theme.color("panel.fg"), theme.color("panel.bg"));
        buf.clear(bright);
        let screen = buf.area();
        let area = draw_dialog(&mut buf, &theme, screen, "T", &["hello".into()], None);
        let under = buf
            .get(area.x + 2, area.bottom())
            .expect("a cell below the dialog");
        assert_eq!(
            under.style.fg,
            theme.color("panel.fg").darkened(),
            "the shadow did not darken the cell below the dialog"
        );
        let beside = buf
            .get(area.right(), area.y + 1)
            .expect("a cell beside the dialog");
        assert_eq!(beside.style.bg, theme.color("panel.bg").darkened());
        // The row above the dialog is untouched: a shadow falls down and to the right.
        let above = buf.get(area.x + 2, area.y.saturating_sub(1)).unwrap();
        assert_eq!(above.style.fg, theme.color("panel.fg"));
    }

    #[test]
    fn a_bar_too_narrow_to_draw_is_skipped_rather_than_panicking() {
        let theme = Theme::default_theme();
        let mut buf = Buffer::new(8, 3);
        draw_bar(
            &mut buf,
            Rect::new(0, 0, 4, 1),
            50,
            theme.dialog(),
            theme.dialog(),
        );
        assert_eq!(buf.to_lines()[0], "");
    }

    #[test]
    fn sort_label_appears_in_the_footer() {
        let theme = Theme::default_theme();
        let l = listing();
        let mut buf = Buffer::new(44, 9);
        let p = PanelRender {
            area: Rect::new(0, 0, 44, 9),
            path: "/x",
            listing: &l,
            selection: &Selection::default(),
            cursor: 0,
            top: 0,
            focused: true,
            view: ViewMode::Full,
            sort_label: SortKey::Size.as_str(),
            sort_reverse: false,
            column_titles: false,
            filter: None,
            message: None,
            status: None,
            free: None,
            scrollbar: false,
            name_scroll: 0,
            highlight: &crate::Highlight::far_default(),
            cluster: 4096,
            titles: english_titles(),
            up_label: "Up",
            folder_label: "Folder",
        };
        draw_panel(&mut buf, &theme, &p);
        assert!(buf.to_text().contains("[size]"));
    }
}

// ── menu bar and drop-down menus (SPEC 10) ─────────────────────────────────────────────────

/// One row of a drop-down menu.
#[derive(Debug, Clone, Default)]
pub struct MenuRow {
    pub label: String,
    /// The keyboard shortcut, shown right-aligned.
    pub keys: String,
    /// `√` for the mode or toggle that is currently on.
    pub checked: bool,
    pub separator: bool,
    /// Drawn dim: the item exists in the menu but the feature is not here yet.
    pub dim: bool,
}

/// A label and the letter `&` marks in it: `&Medium` is `Medium` with `M` as its key.
///
/// FAR underlines one letter per row and takes that key, and the theme has had a colour for
/// it (`menu.hotkey.fg`) since the first menu — only nothing ever consumed the marker, so a
/// row written `&Medium` was drawn with the ampersand in it. `&&` is a literal ampersand,
/// which is the escape every toolkit with this convention uses.
pub fn mnemonic(label: &str) -> (String, Option<char>) {
    let mut text = String::with_capacity(label.len());
    let mut key = None;
    let mut chars = label.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '&' {
            text.push(c);
            continue;
        }
        match chars.peek() {
            Some('&') => {
                text.push('&');
                chars.next();
            }
            // The first marker names the key; a second one is a typo rather than a second
            // key, and a marker at the end of a label marks nothing.
            Some(&next) if key.is_none() => key = Some(next),
            _ => {}
        }
    }
    (text, key)
}

impl MenuRow {
    pub fn separator() -> MenuRow {
        MenuRow {
            separator: true,
            ..MenuRow::default()
        }
    }
}

/// The menu bar across the top. Returns the column each title starts at, so the drop-down can
/// be drawn under its own title.
pub fn draw_menu_bar(
    buf: &mut Buffer,
    theme: &Theme,
    area: Rect,
    titles: &[String],
    active: usize,
) -> Vec<u16> {
    let bar = Style::new(theme.color("hmenu.fg"), theme.color("hmenu.bg"));
    let picked = Style::new(
        theme.color("hmenu.selected.fg"),
        theme.color("hmenu.selected.bg"),
    );
    buf.fill(area, ' ', bar);
    let mut x = area.x + 2;
    let mut starts = Vec::with_capacity(titles.len());
    for (i, title) in titles.iter().enumerate() {
        starts.push(x);
        let text = format!(" {title} ");
        let width = text.width() as u16;
        if x + width > area.right() {
            break;
        }
        buf.put_str(
            x,
            area.y,
            &text,
            if i == active { picked } else { bar },
            width,
        );
        x += width + 1;
    }
    starts
}

/// What a side's tab row is drawn from.
pub struct TabRow<'a> {
    pub area: Rect,
    /// What each tab is called, left to right.
    pub labels: &'a [String],
    /// Which of them is on screen.
    pub current: usize,
    /// Which of them refuse to leave their directory.
    pub locked: &'a [bool],
    pub focused: bool,
}

/// The row of tabs above a panel.
///
/// It is drawn only by a side that has more than one tab, so the width is never generous:
/// the current tab is kept whole and the row is scrolled to it, with the guillemets saying
/// that there are more tabs off either end. A name longer than `NAME_COLS` is cut, because
/// one tab called after a deeply nested directory must not take the row from the other six.
pub fn draw_tabs(buf: &mut Buffer, theme: &Theme, row: &TabRow) {
    const NAME_COLS: usize = 18;
    let area = row.area;
    if area.w == 0 || area.h == 0 || row.labels.is_empty() {
        return;
    }
    let bar = Style::new(theme.color("panel.tab.fg"), theme.color("panel.tab.bg"));
    let picked = if row.focused {
        Style::new(
            theme.color("panel.tab.active.fg"),
            theme.color("panel.tab.active.bg"),
        )
    } else {
        Style::new(
            theme.color("panel.header.inactive.fg"),
            theme.color("panel.tab.bg"),
        )
    };
    buf.fill(area, ' ', bar);

    let cells: Vec<String> = row
        .labels
        .iter()
        .enumerate()
        .map(|(i, label)| {
            let lock = if row.locked.get(i).copied().unwrap_or(false) {
                "\u{25aa}"
            } else {
                ""
            };
            format!(" {lock}{} ", truncate_cols(label, NAME_COLS))
        })
        .collect();
    let widths: Vec<usize> = cells.iter().map(|c| c.width()).collect();
    let room = area.w as usize;
    let current = row.current.min(cells.len() - 1);

    // Where the row starts. Everything fits until it does not, and then the current tab is
    // what has to be whole: walk left from it while there is still room, keeping a column
    // for the mark that says the row goes on.
    let mut first = 0;
    if widths.iter().sum::<usize>() > room {
        let mut used = widths[current] + 1;
        first = current;
        while first > 0 && used + widths[first - 1] <= room {
            first -= 1;
            used += widths[first];
        }
    }

    let mut x = area.x + u16::from(first > 0);
    let mut last = first;
    for (i, cell) in cells.iter().enumerate().skip(first) {
        let w = widths[i] as u16;
        // The last column is left for the closing mark unless this is the last tab there is.
        let edge = area.right() - u16::from(i + 1 < cells.len());
        if x + w > edge {
            break;
        }
        buf.put_str(x, area.y, cell, if i == current { picked } else { bar }, w);
        x += w;
        last = i;
    }
    if first > 0 {
        buf.put_str(area.x, area.y, "\u{2039}", bar, 1);
    }
    if last + 1 < cells.len() {
        buf.put_str(area.right() - 1, area.y, "\u{203a}", bar, 1);
    }
}

/// A drop-down menu under `at`, growing down. Returns the area it covers.
pub fn draw_menu(
    buf: &mut Buffer,
    theme: &Theme,
    screen: Rect,
    at: u16,
    rows: &[MenuRow],
    selected: usize,
) -> Rect {
    let label_w = rows
        .iter()
        .map(|r| mnemonic(&r.label).0.width())
        .max()
        .unwrap_or(8);
    let keys_w = rows.iter().map(|r| r.keys.width()).max().unwrap_or(0);
    // mark + label + gap + keys, inside a frame with one column of padding.
    let inner = 2 + label_w + if keys_w > 0 { keys_w + 2 } else { 0 };
    let w = (inner as u16 + 4).min(screen.w);
    let h = (rows.len() as u16 + 2).min(screen.h.saturating_sub(1));
    let x = at.min(screen.right().saturating_sub(w));
    let y = screen.y + 1;
    let area = Rect::new(x, y, w, h);

    // A menu has its own colours, not the dialog ones: in FAR the dropdowns are cyan with
    // yellow shortcuts while dialogs are grey.
    let style = Style::new(theme.color("menu.fg"), theme.color("menu.bg"));
    let frame = Style::new(theme.color("menu.frame"), theme.color("menu.bg"));
    let picked = Style::new(
        theme.color("menu.selected.fg"),
        theme.color("menu.selected.bg"),
    );
    let keys_style = Style::new(theme.color("menu.hotkey.fg"), theme.color("menu.bg"));
    buf.fill(area, ' ', style);
    buf.draw_frame(area, frame, None);

    for (i, row) in rows.iter().enumerate() {
        let y = area.y + 1 + i as u16;
        if y >= area.bottom() - 1 {
            break;
        }
        if row.separator {
            for x in area.x + 1..area.right() - 1 {
                buf.set(x, y, '─', frame);
            }
            continue;
        }
        let on = i == selected;
        let line = Style::new(
            if on {
                theme.color("menu.selected.fg")
            } else if row.dim {
                theme.color("menu.dim.fg")
            } else {
                theme.color("menu.fg")
            },
            if on {
                theme.color("menu.selected.bg")
            } else {
                theme.color("menu.bg")
            },
        );
        let width = area.w - 2;
        buf.fill(
            Rect::new(area.x + 1, y, width, 1),
            ' ',
            if on { picked } else { style },
        );
        let mark = if row.checked { '√' } else { ' ' };
        buf.set(area.x + 2, y, mark, line);
        let (label, hotkey) = mnemonic(&row.label);
        buf.put_str(area.x + 4, y, &label, line, width.saturating_sub(3));
        // The letter the row answers to, in the colour the theme has always had for it. On
        // the selected row it stays the selection's own ink: a second colour there would
        // fight with the bar rather than help anybody find the key.
        if let Some(hotkey) = hotkey {
            if !on && !row.dim {
                if let Some(at) = label.chars().position(|c| c == hotkey) {
                    let before: String = label.chars().take(at).collect();
                    let x = area.x + 4 + before.width() as u16;
                    if x < area.right() - 1 {
                        buf.set(x, y, hotkey, line.with_fg(theme.color("menu.hotkey.fg")));
                    }
                }
            }
        }
        if !row.keys.is_empty() {
            put_right(
                buf,
                area.x + 1,
                y,
                width.saturating_sub(1),
                &row.keys,
                if on { picked } else { keys_style },
            );
        }
    }
    drop_shadow(buf, area);
    area
}

// ── forms: the settings dialogs (SPEC 10) ──────────────────────────────────────────────────

/// One row of a settings dialog: a checkbox, a field, a heading or a separator.
#[derive(Debug, Clone, Default)]
pub struct FormRow {
    pub text: String,
    pub separator: bool,
    /// Drawn dim: shown for completeness, not settable yet.
    pub dim: bool,
}

impl FormRow {
    pub fn new(text: impl Into<String>) -> FormRow {
        FormRow {
            text: text.into(),
            ..FormRow::default()
        }
    }

    pub fn separator() -> FormRow {
        FormRow {
            separator: true,
            ..FormRow::default()
        }
    }
}

/// What a settings dialog has to draw.
pub struct FormView<'a> {
    pub title: &'a str,
    pub rows: &'a [FormRow],
    /// The row the cursor is on.
    pub selected: usize,
    pub buttons: &'a [String],
    /// Which button has the focus, if the cursor has left the rows.
    pub button: Option<usize>,
}

/// A settings dialog: rows, then a row of buttons. Returns the area it covers.
pub fn draw_form(buf: &mut Buffer, theme: &Theme, screen: Rect, view: &FormView) -> Rect {
    let FormView {
        title,
        rows,
        selected,
        buttons,
        button,
    } = *view;
    let buttons_w: usize = buttons.iter().map(|b| b.width() + 4).sum();
    let content_w = rows
        .iter()
        .map(|r| r.text.width())
        .chain(std::iter::once(title.width() + 4))
        .chain(std::iter::once(buttons_w))
        .max()
        .unwrap_or(20)
        .min(screen.w.saturating_sub(4) as usize);
    let w = content_w as u16 + 4;
    let h = (rows.len() as u16 + 4).min(screen.h);
    // `first`, below, needs the row count that fits; the height is capped by the screen.
    let x = screen.x + (screen.w.saturating_sub(w)) / 2;
    let y = screen.y + (screen.h.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);

    let style = theme.dialog();
    let frame = Style::new(theme.color("dialog.frame"), theme.color("dialog.bg"));
    let picked = Style::new(theme.color("cursor.fg"), theme.color("cursor.bg"));
    buf.fill(area, ' ', style);
    buf.draw_frame(area, frame, Some(title));

    // A dialog taller than the screen scrolls to the row the cursor is on. The attributes
    // dialog has twenty rows on Unix, and a terminal of twenty-four lines would otherwise put
    // the last of them — the times, and the box that says to go into the folders — below the
    // bottom of the screen, where no key could reach them.
    let room = area.h.saturating_sub(4) as usize;
    let first = if rows.len() <= room {
        0
    } else {
        // Keep the cursor in view with a row of context either side where there is one, and
        // never scroll past the end: a dialog whose last rows are blank looks broken.
        let want = selected.saturating_sub(1);
        want.min(rows.len() - room)
    };
    if first > 0 {
        // The frame says there is more above, in the place a scrollbar would be.
        buf.set(area.right() - 1, area.y + 1, '▲', frame);
    }
    if first + room < rows.len() {
        buf.set(area.right() - 1, area.bottom() - 3, '▼', frame);
    }
    for (i, row) in rows.iter().enumerate().skip(first).take(room) {
        let ry = area.y + 1 + (i - first) as u16;
        if ry >= area.bottom() - 2 {
            break;
        }
        if row.separator {
            for x in area.x + 1..area.right() - 1 {
                buf.set(x, ry, '─', frame);
            }
            continue;
        }
        let on = i == selected && button.is_none();
        let line = Style::new(
            if on {
                theme.color("cursor.fg")
            } else if row.dim {
                theme.color("panel.footer.fg")
            } else {
                theme.color("dialog.fg")
            },
            if on {
                theme.color("cursor.bg")
            } else {
                theme.color("dialog.bg")
            },
        );
        buf.fill(
            Rect::new(area.x + 1, ry, area.w - 2, 1),
            ' ',
            if on { picked } else { style },
        );
        buf.put_str(area.x + 2, ry, &row.text, line, area.w - 4);
    }

    if !buttons.is_empty() {
        draw_buttons(
            buf,
            theme,
            Rect::new(area.x + 1, area.bottom() - 2, area.w - 2, 1),
            buttons,
            button.unwrap_or(usize::MAX),
        );
    }
    drop_shadow(buf, area);
    area
}
