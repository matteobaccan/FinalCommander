use std::time::SystemTime;

use chrono::{DateTime, Local};
use sc_core::{
    format_count, format_size, Entry, EntryFlags, EntryKind, Listing, Selection, ViewMode,
};
use unicode_width::UnicodeWidthStr;

use crate::buffer::{elide_middle, truncate_cols, Buffer, Rect, Style};
use crate::theme::Theme;

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
    pub filter: Option<&'a str>,
    pub message: Option<&'a str>,
}

/// Rows that fit in a panel of this height.
pub fn panel_rows(area: Rect) -> usize {
    area.h.saturating_sub(2) as usize
}

/// Columns of names in brief view.
pub fn brief_columns(area: Rect) -> usize {
    let inner = area.inner().w as usize;
    (inner / 20).clamp(1, 4)
}

pub fn draw_panel(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>) {
    let area = p.area;
    if area.w < 8 || area.h < 3 {
        return;
    }
    let base = theme.panel();
    buf.fill(area, ' ', base);
    let frame = Style::new(theme.color("panel.frame"), theme.color("panel.bg"));
    buf.draw_frame(area, frame, None);

    // ── header: the path, elided in the middle so both ends stay readable
    let header_style = Style::new(
        if p.focused {
            theme.color("panel.header.fg")
        } else {
            theme.color("panel.header.inactive.fg")
        },
        theme.color("panel.header.bg"),
    );
    let header_w = area.w.saturating_sub(4) as usize;
    let title = elide_middle(p.path, header_w);
    let tw = title.width() as u16;
    let tx = area.x + 1 + (area.w.saturating_sub(2).saturating_sub(tw + 2)) / 2;
    buf.put_str(tx, area.y, &format!(" {title} "), header_style, tw + 2);

    let inner = area.inner();
    let rows = panel_rows(area);

    if p.listing.is_empty() {
        buf.put_str(inner.x + 1, inner.y, "(empty)", base, inner.w);
    } else if p.view == ViewMode::Brief || p.view == ViewMode::Wide {
        draw_brief(buf, theme, p, inner, rows);
    } else {
        draw_full(buf, theme, p, inner, rows);
    }

    draw_footer(buf, theme, p);
}

fn row_style(theme: &Theme, e: &Entry, selected: bool, cursor: bool, focused: bool) -> Style {
    let fg = if selected {
        theme.color("selected.fg")
    } else if e.flags.contains(EntryFlags::BROKEN_LINK) {
        theme.color("file.broken")
    } else {
        match e.kind {
            EntryKind::Dir | EntryKind::Parent => theme.color("file.dir"),
            EntryKind::Symlink => theme.color("file.symlink"),
            _ if e.flags.contains(EntryFlags::HIDDEN) => theme.color("file.hidden"),
            _ if e.flags.contains(EntryFlags::EXECUTABLE) => theme.color("file.exec"),
            _ => theme.color("panel.fg"),
        }
    };
    if cursor {
        let (cfg, cbg) = if focused {
            ("cursor.fg", "cursor.bg")
        } else {
            ("cursor.inactive.fg", "cursor.inactive.bg")
        };
        let mut s = Style::new(theme.color(cfg), theme.color(cbg));
        if selected {
            s.fg = theme.color("selected.fg");
            s.bold = true;
        }
        s
    } else {
        let mut s = Style::new(fg, theme.color("panel.bg"));
        if matches!(e.kind, EntryKind::Dir | EntryKind::Parent) {
            s.bold = true;
        }
        s
    }
}

fn draw_brief(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let cols = brief_columns(p.area);
    let col_w = (inner.w as usize / cols).max(4);
    for col in 0..cols {
        for row in 0..rows {
            let idx = p.top + col * rows + row;
            let Some(e) = p.listing.row(idx) else {
                continue;
            };
            let selected = p.selection.contains(e.id);
            let style = row_style(theme, e, selected, idx == p.cursor, p.focused);
            let x = inner.x + (col * col_w) as u16;
            let w = col_w as u16 - 1;
            buf.fill(Rect::new(x, inner.y + row as u16, w, 1), ' ', style);
            let name = decorate(e);
            buf.put_str(
                x,
                inner.y + row as u16,
                &truncate_cols(&name, w as usize),
                style,
                w,
            );
        }
    }
}

fn draw_full(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>, inner: Rect, rows: usize) {
    let size_w: u16 = 9;
    let time_w: u16 = 16; // `2026-10-23 17:42`
    let detail = p.view == ViewMode::Detail;
    let attr_w: u16 = if detail { 5 } else { 0 };
    let fixed = size_w + time_w + attr_w + 2 + u16::from(detail);
    let name_w = inner.w.saturating_sub(fixed).max(4);

    for row in 0..rows {
        let idx = p.top + row;
        let Some(e) = p.listing.row(idx) else {
            continue;
        };
        let y = inner.y + row as u16;
        let selected = p.selection.contains(e.id);
        let style = row_style(theme, e, selected, idx == p.cursor, p.focused);
        buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);

        let name = decorate(e);
        buf.put_str(
            inner.x,
            y,
            &truncate_cols(&name, name_w as usize),
            style,
            name_w,
        );

        let size_text = match e.kind {
            EntryKind::Parent => "UP--".to_string(),
            EntryKind::Dir => "FOLDER".to_string(),
            _ => format_size(e.size),
        };
        let sx = inner.x + name_w + 1;
        put_right(buf, sx, y, size_w, &size_text, style);

        let tx = sx + size_w + 1;
        let time_text = e.mtime.map(format_mtime).unwrap_or_default();
        buf.put_str(
            tx,
            y,
            &truncate_cols(&time_text, time_w as usize),
            style,
            time_w,
        );

        if detail {
            let ax = tx + time_w + 1;
            buf.put_str(ax, y, &attrs(e), style, attr_w);
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
        EntryKind::Dir => format!("{name}/"),
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

fn draw_footer(buf: &mut Buffer, theme: &Theme, p: &PanelRender<'_>) {
    let area = p.area;
    let y = area.bottom() - 1;
    let w = area.w.saturating_sub(4);
    let style = Style::new(theme.color("panel.footer.fg"), theme.color("panel.bg"));

    let totals = &p.listing.totals;
    let text = if let Some(msg) = p.message {
        msg.to_string()
    } else if let Some(f) = p.filter {
        format!("filter: {f}")
    } else if totals.selected_files > 0 {
        format!(
            "{} of {} selected, {}",
            format_count(totals.selected_files),
            format_count(totals.files),
            format_size(totals.selected_bytes)
        )
    } else {
        format!(
            "{} files, {}  [{}]",
            format_count(totals.files),
            format_size(totals.bytes),
            p.sort_label
        )
    };
    let text = truncate_cols(&text, w as usize);
    let tw = text.width() as u16;
    let style = if p.filter.is_some() || p.message.is_some() {
        style.with_fg(theme.color("filter.fg"))
    } else {
        style
    };
    buf.put_str(area.x + 2, y, &text, style, tw);
}

/// The `F1..F12` bar. `labels` is `(number, label)`, already localised by the caller.
pub fn draw_keybar(buf: &mut Buffer, theme: &Theme, area: Rect, labels: &[(u8, &str)]) {
    let bar = theme.keybar();
    let num = Style::new(theme.color("keybar.num.fg"), theme.color("keybar.num.bg"));
    buf.fill(area, ' ', bar);
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
        let avail = slot as u16 - used - 1;
        buf.put_str(
            x + used,
            area.y,
            &truncate_cols(label, avail as usize),
            bar,
            avail,
        );
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
    let input = Style::new(theme.color("dialog.fg"), theme.color("dialog.bg")).bold();
    buf.fill(field, ' ', input);
    let shown = truncate_cols(text, field.w as usize);
    buf.put_str(field.x, field.y, &shown, input, field.w);
    (
        area,
        field.x + (cursor as u16).min(field.w.saturating_sub(1)),
        field.y,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sc_core::{EntryId, Sort, SortKey};
    use std::ffi::OsString;

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: Some(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000)),
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
        let theme = Theme::default_theme();
        let listing = listing();
        let mut buf = Buffer::new(44, 10);
        let p = PanelRender {
            area: Rect::new(0, 0, 44, 9),
            path: "/home/mb/project",
            listing: &listing,
            selection,
            cursor,
            top: 0,
            focused: true,
            view,
            sort_label: "name",
            filter: None,
            message: None,
        };
        draw_panel(&mut buf, &theme, &p);
        buf.to_text()
    }

    #[test]
    fn full_view_shows_names_sizes_and_times() {
        let out = render(ViewMode::Full, 0, &Selection::default());
        assert!(out.contains("/home/mb/project"), "{out}");
        assert!(out.contains("src/"), "{out}");
        assert!(out.contains("FOLDER"), "{out}");
        assert!(out.contains("412 B"), "{out}");
        assert!(out.contains("2.00 K"), "{out}");
        assert!(!out.contains("22…"), "the time column is too narrow: {out}");
        assert!(out.contains("2 files"), "{out}"); // src/ counts as a directory
    }

    #[test]
    fn brief_view_lays_names_out_in_columns() {
        let out = render(ViewMode::Brief, 0, &Selection::default());
        let body: Vec<&str> = out.lines().skip(1).take(4).collect();
        // With 4 entries and several columns, `..` and `src/` share the first row.
        assert!(body[0].contains(".."), "{out}");
        assert!(
            out.contains("Cargo.toml") && out.contains("README.md"),
            "{out}"
        );
    }

    #[test]
    fn selection_and_cursor_are_visible_in_the_footer() {
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
            filter: None,
            message: None,
        };
        draw_panel(&mut buf, &theme, &p);
        let out = buf.to_text();
        assert!(out.contains("1 of 2 selected"), "{out}");
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
            filter: None,
            message: None,
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
            filter: None,
            message: None,
        };
        draw_panel(&mut buf, &theme, &p);
        for line in buf.to_lines() {
            assert!(line.width() <= 40, "line wider than the panel: {line:?}");
        }
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
            filter: None,
            message: None,
        };
        draw_panel(&mut buf, &theme, &p);
        assert!(buf.to_text().contains("[size]"));
    }
}
