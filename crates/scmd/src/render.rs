use scmd_ui::{widgets, Buffer, Rect, Style};

use crate::app::{App, Mode};
use crate::panel as scmd_panel;

/// Draw a whole frame. Returns where the text cursor should sit, if anywhere.
pub fn draw(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    buf.clear(app.theme.panel());
    if w < 20 || h < 5 {
        buf.put_str(0, 0, "terminal too small", app.theme.panel(), w);
        return None;
    }

    // The hex editor takes the whole screen too, and has no prompts of its own yet.
    if app.mode.hex().is_some() {
        if let Mode::Hex(mut editor) = std::mem::replace(&mut app.mode, Mode::Normal) {
            let cursor = draw_hex(app, &mut editor, buf);
            app.mode = Mode::Hex(editor);
            return cursor;
        }
    }

    // The editor takes the whole screen; its prompts and confirmations draw on top of it.
    if app.mode.editor().is_some() || app.editor_parked_for_render() {
        if let Some((mut editor, parked)) = app.editor_for_render() {
            let cursor = draw_editor(app, &mut editor, buf);
            app.return_editor(editor, parked);
            if !parked {
                return cursor;
            }
            // A prompt or confirmation is open over the editor.
            let screen = Rect::new(0, 0, w, h.saturating_sub(1));
            match &app.mode {
                Mode::Prompt(prompt) => {
                    let (_, cx, cy) = widgets::draw_prompt(
                        buf,
                        &app.theme,
                        screen,
                        &prompt.title,
                        &prompt.text,
                        prompt.text.chars().count(),
                    );
                    return Some((cx, cy));
                }
                Mode::Confirm(confirm) => {
                    widgets::draw_choice(
                        buf,
                        &app.theme,
                        screen,
                        &confirm.title,
                        &confirm.lines,
                        &[],
                        0,
                    );
                    return None;
                }
                _ => return None,
            }
        }
    }

    // `Ctrl+O`: what the commands printed, full screen.
    if let Mode::UserScreen { scroll } = app.mode {
        draw_user_screen(app, buf, scroll);
        return None;
    }

    // The viewer takes the whole screen; its prompts draw on top of it.
    if let Some((mut viewer, parked)) = app.viewer_for_render() {
        draw_viewer(app, &mut viewer, buf);
        app.return_viewer(viewer, parked);
        if parked {
            if let Mode::Prompt(prompt) = &app.mode {
                let (_, cx, cy) = widgets::draw_prompt(
                    buf,
                    &app.theme,
                    Rect::new(0, 0, w, h.saturating_sub(1)),
                    &prompt.title,
                    &prompt.text,
                    prompt.text.chars().count(),
                );
                return Some((cx, cy));
            }
        }
        return None;
    }

    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let status_h = u16::from(app.cfg.ui.status_bar);
    let cmd_h = u16::from(app.cfg.ui.command_line);
    let panels_h = h.saturating_sub(keybar_h + status_h + cmd_h).max(3);

    // `Ctrl+F1`/`Ctrl+F2` hide one panel and give the screen to the other: a zero-width area
    // draws nothing, which is all "panel off" has to mean.
    let areas = match app.hidden {
        Some(0) => [Rect::new(0, 0, 0, panels_h), Rect::new(0, 0, w, panels_h)],
        Some(_) => [Rect::new(0, 0, w, panels_h), Rect::new(w, 0, 0, panels_h)],
        None => {
            let left_w = w / 2;
            [
                Rect::new(0, 0, left_w, panels_h),
                Rect::new(left_w, 0, w - left_w, panels_h),
            ]
        }
    };

    // Remember the geometry so key handling knows how far a page is. It is the active
    // panel's, because that is the one the keys move through.
    app.rows = widgets::panel_rows(
        areas[app.active],
        app.cfg.panel.column_titles,
        app.cfg.panel.status_line,
    );
    app.columns = widgets::brief_columns(areas[app.active], app.panels[app.active].view);
    for i in 0..2 {
        let (rows, cols) = (app.rows, app.columns);
        app.panels[i].scroll_into_view(rows, cols);
    }

    let up_label = app.i18n.t("list-up");
    let folder_label = app.i18n.t("list-folder");
    for (i, area) in areas.iter().enumerate() {
        // The special kinds have no rows, so they draw themselves inside the same frame and
        // the row machinery below never sees them (SPEC 02 §7).
        match app.panels[i].content {
            scmd_panel::Content::QuickView => {
                draw_quick_view(app, i, *area, buf);
                continue;
            }
            scmd_panel::Content::Info => {
                draw_info(app, i, *area, buf);
                continue;
            }
            scmd_panel::Content::Tree => {
                draw_tree(app, i, *area, buf);
                continue;
            }
            scmd_panel::Content::Files => {}
        }
        let panel = &app.panels[i];
        // A list panel says what it is a list of; a directory panel says where it is.
        let path = match &panel.temp {
            Some(list) => list.title.clone(),
            None => panel.cwd.to_string(),
        };
        let status = app.cfg.panel.status_line.then(|| cursor_line(app, panel));
        let message = panel
            .error
            .as_deref()
            .or(panel.message.as_deref())
            .map(|s| s.to_string());
        let render = widgets::PanelRender {
            area: *area,
            path: &path,
            listing: &panel.listing,
            selection: &panel.selection,
            cursor: panel.cursor,
            top: panel.top,
            focused: i == app.active,
            view: panel.view,
            sort_label: panel.sort.key.as_str(),
            sort_reverse: panel.sort.reverse,
            column_titles: app.cfg.panel.column_titles,
            filter: panel.filter.as_deref(),
            message: message.as_deref(),
            status: status.as_deref(),
            free: if app.cfg.panel.free_space {
                panel.free
            } else {
                None
            },
            scrollbar: app.cfg.panel.scrollbar,
            name_scroll: panel.name_scroll,
            highlight: &app.highlight,
            up_label: &up_label,
            folder_label: &folder_label,
        };
        widgets::draw_panel(buf, &app.theme, &render);
    }

    // The command line: the panel's path as the prompt, then what is being typed.
    let mut cursor = None;
    if cmd_h == 1 {
        let y = panels_h;
        let style = Style::new(app.theme.color("cmdline.fg"), app.theme.color("cmdline.bg"));
        buf.fill(Rect::new(0, y, w, 1), ' ', style);
        let prompt = format!("{}> ", app.panels[app.active].cwd);
        let prompt = scmd_ui::elide_middle(&prompt, (w / 2) as usize);
        let prompt_w = unicode_width::UnicodeWidthStr::width(prompt.as_str()) as u16;
        buf.put_str(0, y, &prompt, style, prompt_w);
        let room = w.saturating_sub(prompt_w);
        let text = scmd_ui::truncate_cols(&app.cmd.text, room as usize);
        buf.put_str(prompt_w, y, &text, style, room);
        let caret = app
            .cmd
            .text
            .chars()
            .take(app.cmd.caret)
            .filter_map(unicode_width::UnicodeWidthChar::width)
            .sum::<usize>() as u16;
        cursor = Some((prompt_w + caret.min(room.saturating_sub(1)), y));
    }

    if status_h == 1 {
        let text = if app.status.is_empty() {
            status_hint(app)
        } else {
            app.status.clone()
        };
        let row = Rect::new(0, panels_h + cmd_h, w, 1);
        widgets::draw_status(buf, &app.theme, row, &text, app.status_error);
        // The background-screen counter, in the corner FAR keeps it in. It is drawn only when
        // there is something behind the panels, because `[0]` is noise.
        if !app.background.is_empty() {
            let mark = format!("[{}]", app.background.len());
            let width = unicode_width::UnicodeWidthStr::width(mark.as_str()) as u16;
            if width < w {
                let style = Style::new(app.theme.color("status.fg"), app.theme.color("status.bg"));
                buf.put_str(w - width, row.y, &mark, style, width);
            }
        }
    }

    if keybar_h == 1 {
        let labels = app.keybar();
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
    }

    let screen = Rect::new(0, 0, w, panels_h);
    if app.mode.is_normal() {
        return cursor;
    }
    if let Some(view) = app.mode.job() {
        draw_job(app, view, buf, screen);
        return None;
    }
    match &app.mode {
        Mode::Normal => None,
        Mode::Find(view) => {
            draw_find(app, view, buf, screen);
            None
        }
        Mode::Prompt(prompt) => {
            // A password is never put on the screen, not even for a frame.
            let shown = if prompt.secret {
                "*".repeat(prompt.text.chars().count())
            } else {
                prompt.text.clone()
            };
            let (_, cx, cy) = widgets::draw_prompt(
                buf,
                &app.theme,
                screen,
                &prompt.title,
                &shown,
                shown.chars().count(),
            );
            Some((cx, cy))
        }
        Mode::Confirm(confirm) => {
            widgets::draw_choice(
                buf,
                &app.theme,
                screen,
                &confirm.title,
                &confirm.lines,
                &[],
                0,
            );
            None
        }
        Mode::Job(_) => None,    // handled above, through `Mode::job()`
        Mode::Viewer(_) => None, // handled above, before the panels are drawn
        Mode::Editor(_) | Mode::Hex(_) => None, // handled above, before the panels are drawn
        Mode::Message { title, lines } => {
            widgets::draw_dialog(buf, &app.theme, screen, title, lines, None);
            None
        }
        Mode::Sites { cursor } => {
            draw_sites(app, buf, screen, *cursor);
            None
        }
        Mode::Menu { menu, item } => {
            draw_menus(app, buf, Rect::new(0, 0, w, panels_h), *menu, *item);
            None
        }
        Mode::Form(form) => {
            let rows = form.rows();
            let buttons = vec![app.i18n.t("btn-ok"), app.i18n.t("btn-cancel")];
            let area = widgets::draw_form(
                buf,
                &app.theme,
                screen,
                &widgets::FormView {
                    title: &form.title,
                    rows: &rows,
                    selected: form.cursor,
                    buttons: &buttons,
                    button: form.button,
                },
            );
            // A text row shows a real caret; everything else leaves the cursor hidden.
            form.caret()
                .map(|(row, col)| (area.x + 2 + col as u16, area.y + 1 + row as u16))
        }
        Mode::Pick(pick) => {
            draw_pick(app, buf, screen, pick);
            None
        }
        Mode::UserScreen { .. } => None, // drawn full screen, before the panels
        Mode::Help {
            chapter,
            scroll,
            at,
        } => {
            draw_help(app, buf, *chapter, *scroll, *at, screen);
            None
        }
    }
}

/// `Ctrl+O`: the output of the commands that have run. Plain text, scrollable, and it keeps
/// the key bar so the way out is on the screen.
fn draw_user_screen(app: &App, buf: &mut Buffer, scroll: usize) {
    let (w, h) = (buf.width(), buf.height());
    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let base = app.theme.panel();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    let header = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("status.bg"),
    );
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    let title = app.i18n.t_args(
        "user-screen-title",
        &[
            ("lines", (app.user_screen.len() as i64).into()),
            ("at", (scroll as i64 + 1).into()),
        ],
    );
    buf.put_str(1, 0, &title, header, w.saturating_sub(2));

    let body = Rect::new(0, 1, w, h.saturating_sub(1 + keybar_h));
    if app.user_screen.is_empty() {
        buf.put_str(1, body.y, &app.i18n.t("user-screen-empty"), base, w - 1);
    }
    let heading = Style::new(
        app.theme.color("panel.column.fg"),
        app.theme.color("panel.bg"),
    );
    for (i, line) in app
        .user_screen
        .iter()
        .skip(scroll)
        .take(body.h as usize)
        .enumerate()
    {
        let style = if line.starts_with("$ ") {
            heading
        } else {
            base
        };
        let text = scmd_ui::truncate_cols(line, w as usize);
        buf.put_str(0, body.y + i as u16, &text, style, w);
    }

    if keybar_h == 1 {
        let labels = [(1u8, app.i18n.t("key-help")), (10, app.i18n.t("key-quit"))];
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
    }
}

/// The structure pane beside the content: what the file is, and where anything in it lives
/// (SPEC 05 §1.8). A group heading is a label, a field shows its raw bytes and their meaning,
/// and a row that points somewhere is what `Enter` follows.
fn draw_structure(app: &App, viewer: &mut scmd_view::Viewer, buf: &mut Buffer, area: Rect) {
    let base = app.theme.panel();
    let frame = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let group = Style::new(
        app.theme.color("panel.column.fg"),
        app.theme.color("panel.bg"),
    );
    let dim = Style::new(
        app.theme.color("panel.footer.fg"),
        app.theme.color("panel.bg"),
    );
    let picked = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));
    let unfocused = Style::new(
        app.theme.color("cursor.inactive.fg"),
        app.theme.color("cursor.inactive.bg"),
    );

    buf.fill(area, ' ', base);
    for y in area.y..area.bottom() {
        buf.set(area.x, y, '│', frame);
    }
    let title = app.i18n.t("viewer-structure");
    buf.put_str(area.x + 2, area.y, &title, group, area.w.saturating_sub(3));

    let inner = Rect::new(
        area.x + 1,
        area.y + 1,
        area.w.saturating_sub(1),
        area.h.saturating_sub(2),
    );
    let focus = viewer.struct_focus;
    let (rows, cursor) = viewer.structure_rows(inner.h);
    for (i, row) in rows.iter().enumerate() {
        let y = inner.y + i as u16;
        let on = i == cursor;
        let style = match (on, focus, row.kind) {
            (true, true, _) => picked,
            (true, false, _) => unfocused,
            (_, _, scmd_view::structure::RowKind::Group) => group,
            (_, _, scmd_view::structure::RowKind::Field) => base,
            _ => base,
        };
        if on {
            buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);
        }
        match row.kind {
            scmd_view::structure::RowKind::Group => {
                let text = scmd_ui::truncate_cols(&row.label, inner.w.saturating_sub(2) as usize);
                buf.put_str(inner.x + 1, y, &text, style, inner.w.saturating_sub(1));
            }
            _ => {
                // label on the left, value on the right, raw bytes in between when they fit.
                let label_w = (inner.w / 2).max(8).min(inner.w.saturating_sub(4));
                let label = scmd_ui::truncate_cols(&row.label, label_w as usize);
                buf.put_str(inner.x + 1, y, &label, style, label_w);
                let right_x = inner.x + 1 + label_w;
                let right_w = inner.right().saturating_sub(right_x);
                let value = if row.raw.is_empty() {
                    row.value.clone()
                } else {
                    format!("{} {}", row.raw, row.value)
                };
                let value = scmd_ui::truncate_cols(&value, right_w as usize);
                let value_style = if on {
                    style
                } else if row.is_target() {
                    base
                } else {
                    dim
                };
                buf.put_str(right_x, y, &value, value_style, right_w);
            }
        }
    }

    let hint = app.i18n.t(if focus {
        "viewer-structure-hint"
    } else {
        "viewer-structure-hint-tab"
    });
    let hint = scmd_ui::truncate_cols(&hint, area.w.saturating_sub(3) as usize);
    buf.put_str(area.x + 2, area.bottom() - 1, &hint, dim, area.w - 3);
}

/// The menu bar across the top with one menu pulled down (SPEC 10 §2).
fn draw_menus(app: &App, buf: &mut Buffer, screen: Rect, menu: usize, item: usize) {
    let menus = crate::menu::menus();
    let titles: Vec<String> = menus.iter().map(|m| app.i18n.t(m.title)).collect();
    let starts =
        widgets::draw_menu_bar(buf, &app.theme, Rect::new(0, 0, screen.w, 1), &titles, menu);
    let Some(open) = menus.get(menu) else { return };
    let rows: Vec<widgets::MenuRow> = open
        .items
        .iter()
        .map(|entry| match entry {
            crate::menu::Entry::Separator => widgets::MenuRow::separator(),
            crate::menu::Entry::Item(i) => widgets::MenuRow {
                label: app.i18n.t(i.label),
                // The shortcut comes from the keymap in force, so a rebinding shows here too;
                // the entry's own string is only the hint for what is not wired up yet.
                keys: match &i.action {
                    crate::menu::Action::Run(cmd) => app
                        .keymap
                        .key_for(*cmd)
                        .map(|k| k.to_string())
                        .unwrap_or_default(),
                    // For what is not here yet, the entry's own string: FAR's key, kept free
                    // for the feature. A test makes sure it is still free.
                    crate::menu::Action::Todo(_) => i.keys.to_string(),
                },
                checked: app.menu_mark(&i.mark, open.side),
                separator: false,
                dim: matches!(i.action, crate::menu::Action::Todo(_)),
            },
        })
        .collect();
    let at = starts.get(menu).copied().unwrap_or(0);
    let below = Rect::new(screen.x, 1, screen.w, screen.h.saturating_sub(1));
    widgets::draw_menu(buf, &app.theme, below, at, &rows, item);
}

/// A choose-one list: languages, themes, drives, folder history, sort modes (SPEC 10 §4).
fn draw_pick(app: &App, buf: &mut Buffer, screen: Rect, pick: &crate::form::Pick) {
    let mut lines = pick.lines();
    if lines.is_empty() {
        lines.push(app.i18n.t("pick-empty"));
    }
    let hint = app.i18n.t("pick-hint");
    let area = widgets::draw_dialog(buf, &app.theme, screen, &pick.title, &lines, Some(&hint));
    if pick.rows.is_empty() {
        return;
    }
    let style = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));
    let y = area.y + 1 + pick.cursor as u16;
    if y < area.y + area.h.saturating_sub(1) {
        let width = area.w.saturating_sub(2) as usize;
        let text = format!(" {:<w$}", lines[pick.cursor], w = width.saturating_sub(1));
        buf.put_str(area.x + 1, y, &text, style, width as u16);
    }
}

/// The progress overlay, or the question the engine is blocked on.
fn draw_job(app: &App, view: &crate::app::JobView, buf: &mut Buffer, screen: Rect) {
    if let Some(pending) = &view.question {
        let (title, lines) = app.question_text(&pending.question);
        let labels: Vec<String> = pending.buttons.iter().map(|(l, _)| l.clone()).collect();
        widgets::draw_choice(
            buf,
            &app.theme,
            screen,
            &title,
            &lines,
            &labels,
            view.choice,
        );
        return;
    }

    let p = &view.progress;
    let current = p
        .current
        .as_ref()
        .map(|path| path.to_string())
        .unwrap_or_else(|| app.i18n.t("job-planning"));
    let counters = app.i18n.t_args(
        "job-counters",
        &[
            ("files", (p.files_done as i64).into()),
            ("filestotal", (p.files_total as i64).into()),
            ("bytes", scmd_core::format_size(p.bytes_done).into()),
            ("bytestotal", scmd_core::format_size(p.bytes_total).into()),
        ],
    );
    // Speed, elapsed and what is left. All three come from the average rather than the last
    // tick, and the estimate is simply absent until there is enough to compute it: a number
    // that jumps between four seconds and four hours is worse than no number.
    let rates = match (p.bytes_per_sec(), p.eta()) {
        (Some(speed), eta) => app.i18n.t_args(
            if eta.is_some() {
                "job-rates"
            } else {
                "job-rates-no-eta"
            },
            &[
                ("speed", scmd_core::format_size(speed as u64).into()),
                ("elapsed", clock(p.elapsed).into()),
                ("eta", eta.map(clock).unwrap_or_default().into()),
            ],
        ),
        _ => String::new(),
    };
    widgets::draw_progress(
        buf,
        &app.theme,
        screen,
        &widgets::ProgressView {
            title: &app.job_title(view.kind),
            current: &current,
            counters: &counters,
            rates: &rates,
            total_percent: p.percent(),
            file_percent: p.file_percent(),
            show_file_bar: p.files_total > 1,
            hint: &app.i18n.t("job-hint"),
        },
    );
}

/// The editor, full screen. Returns where the text cursor sits.
/// The hex editor (SPEC 05 §2.5): offsets, bytes, and the same bytes as characters, with the
/// ones that have changed in their own colour and the block behind them.
fn draw_hex(
    app: &mut App,
    editor: &mut scmd_edit::HexEdit,
    buf: &mut Buffer,
) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let body = Rect::new(0, 1, w, h.saturating_sub(1 + keybar_h));
    let base = app.theme.panel();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    // How many bytes fit: eight offset columns, then three per byte, then one per character,
    // in groups of eight with a space between. Powers of two only, so the offsets stay round.
    let room = w.saturating_sub(12) as usize;
    let per = [32usize, 16, 8, 4]
        .into_iter()
        .find(|n| 8 + 2 + n * 3 + n / 8 + 1 + n <= room + 12)
        .unwrap_or(4);
    editor.bytes_per_row = per;
    app.editor_size = (body.w, body.h);
    editor.scroll_into_view(body.h as usize);

    // The header: the file, and where the cursor is in it.
    let header = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("status.bg"),
    );
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    let status = format!(
        "{:08X}/{:08X}  {}  {}{}",
        editor.cursor,
        editor.len(),
        if editor.insert { "ins" } else { "ovr" },
        match editor.pane {
            scmd_edit::Pane::Hex => "hex",
            scmd_edit::Pane::Text => "txt",
        },
        if editor.modified() { "  *" } else { "" },
    );
    let status_w = (status.chars().count() as u16).min(w);
    let name_w = w.saturating_sub(status_w + 2);
    let title = editor
        .path
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_else(|| editor.name());
    buf.put_str(
        0,
        0,
        &scmd_ui::elide_middle(&title, name_w as usize),
        header,
        name_w,
    );
    buf.put_str(w.saturating_sub(status_w), 0, &status, header, status_w);

    let changed = Style::new(app.theme.color("selected.fg"), app.theme.color("panel.bg"));
    let marked = Style::new(
        app.theme.color("panel.info.selected.fg"),
        app.theme.color("panel.info.selected.bg"),
    );
    let on = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));
    let offset_style = Style::new(
        app.theme.color("panel.column.fg"),
        app.theme.color("panel.bg"),
    );
    let mut cursor_at = None;

    for row in 0..body.h {
        let at = editor.top + row as u64 * per as u64;
        if at > editor.len() {
            break;
        }
        let y = body.y + row;
        let bytes = editor.read(at, per);
        buf.put_str(0, y, &format!("{at:08X}"), offset_style, 8);

        for (i, byte) in bytes.iter().enumerate() {
            let offset = at + i as u64;
            // Three columns a byte, with a wider gap every eight: the shape every hex dump
            // has had since DOS, so the eye can count.
            let x = 10 + (i * 3 + i / 8) as u16;
            let text_x = 10 + (per * 3 + per / 8) as u16 + 1 + i as u16;
            let in_block = editor
                .block
                .map(|(a, b)| offset >= a && offset < b)
                .unwrap_or(false);
            let style = if in_block {
                marked
            } else if editor.is_changed(offset) {
                changed
            } else {
                base
            };
            let hex_style = if offset == editor.cursor && editor.pane == scmd_edit::Pane::Hex {
                on
            } else {
                style
            };
            let txt_style = if offset == editor.cursor && editor.pane == scmd_edit::Pane::Text {
                on
            } else {
                style
            };
            buf.put_str(x, y, &format!("{byte:02X}"), hex_style, 2);
            // A byte that is not a printable character shows as a dot, as every dump does.
            let shown = if (0x20..0x7f).contains(byte) {
                *byte as char
            } else {
                '.'
            };
            buf.put_str(text_x, y, &shown.to_string(), txt_style, 1);
            if offset == editor.cursor {
                cursor_at = Some(match editor.pane {
                    scmd_edit::Pane::Hex => (x, y),
                    scmd_edit::Pane::Text => (text_x, y),
                });
            }
        }
    }

    // The patch list, when there is one: what would change if this were saved now.
    let changes = editor.changes();
    if !changes.is_empty() && body.h > 6 {
        let rows = (changes.len() as u16).min(4) + 1;
        let top = body.y + body.h - rows;
        buf.fill(Rect::new(0, top, w, rows), ' ', base);
        let rule = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
        buf.put_str(0, top, &"─".repeat(w as usize), rule, w);
        for (i, change) in changes.iter().take(4).enumerate() {
            let hex = |bytes: &[u8]| {
                bytes
                    .iter()
                    .take(8)
                    .map(|b| format!("{b:02X}"))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            let line = format!(
                "{:08X}  {:<24}  →  {}",
                change.at,
                hex(&change.old),
                hex(&change.new)
            );
            buf.put_str(1, top + 1 + i as u16, &line, changed, w.saturating_sub(1));
        }
    }

    if app.cfg.ui.key_bar {
        let labels = app.hex_keybar();
        let pairs: Vec<(u8, &str)> = labels.iter().map(|(n, t)| (*n, t.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &pairs);
    }
    cursor_at
}

fn draw_editor(
    app: &mut App,
    editor: &mut scmd_edit::Editor,
    buf: &mut Buffer,
) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let body = Rect::new(0, 1, w, h.saturating_sub(1 + keybar_h));
    app.editor_size = (body.w, body.h);

    let base = app.theme.panel();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    // Header: the file on the left, the position and flags on the right.
    let header = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("status.bg"),
    );
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    let status = editor.status();
    let status_w = (status.chars().count() as u16).min(w);
    let name_w = w.saturating_sub(status_w + 2);
    // A fetched copy is shown under the name it has on the server, not its temp path.
    let title = match app.remote_edit() {
        Some(edit) => edit.remote.to_string(),
        None => editor
            .path
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_else(|| editor.name()),
    };
    buf.put_str(
        0,
        0,
        &scmd_ui::elide_middle(&title, name_w as usize),
        header,
        name_w,
    );
    buf.put_str(w - status_w, 0, &status, header, status_w);

    let visible = editor.scroll_into_view(body.h as usize, body.w as usize);
    let selected = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));

    for (row, line_index) in visible.clone().enumerate() {
        let y = body.y + row as u16;
        let text = expand_line(editor, line_index);
        let shown: String = text
            .chars()
            .skip(editor.left)
            .take(body.w as usize)
            .collect();
        buf.put_str(0, y, &shown, base, body.w);

        // The block, in display columns.
        if let Some((from, to)) = block_columns(editor, line_index) {
            let from = from.saturating_sub(editor.left);
            let to = to.saturating_sub(editor.left);
            let slice: String = shown
                .chars()
                .skip(from)
                .take(to.saturating_sub(from))
                .collect();
            if !slice.is_empty() && (from as u16) < body.w {
                buf.put_str(from as u16, y, &slice, selected, body.w - from as u16);
            }
        }
    }

    if keybar_h == 1 {
        let labels = app.keybar_editor();
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
    }

    // The cursor: display column minus the horizontal scroll.
    let cursor_col = editor.display_col(editor.cursor);
    if editor.cursor.line >= visible.start && editor.cursor.line < visible.end {
        let x = cursor_col.saturating_sub(editor.left) as u16;
        let y = body.y + (editor.cursor.line - visible.start) as u16;
        if x < body.w {
            return Some((x, y));
        }
    }
    None
}

/// A line with its tabs expanded, which is what the screen shows.
fn expand_line(editor: &scmd_edit::Editor, line: usize) -> String {
    let raw = editor.buffer.line(line);
    if !raw.contains('\t') {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len() + 8);
    let mut column = 0usize;
    for ch in raw.chars() {
        if ch == '\t' {
            let pad = editor.tab_width - (column % editor.tab_width);
            out.extend(std::iter::repeat_n(' ', pad));
            column += pad;
        } else {
            out.push(ch);
            column += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    out
}

/// The highlighted display-column range on `line`, if the block covers it.
fn block_columns(editor: &scmd_edit::Editor, line: usize) -> Option<(usize, usize)> {
    let block = editor.block?;
    let width = expand_line(editor, line).chars().count();
    match block.kind {
        scmd_edit::BlockKind::Column => {
            let (top, bottom, left, right) = block.columns();
            if line < top || line > bottom {
                return None;
            }
            Some((
                editor.display_col(scmd_edit::Position::new(line, left)),
                editor.display_col(scmd_edit::Position::new(line, right)),
            ))
        }
        scmd_edit::BlockKind::Stream => {
            let range = block.range();
            if line < range.start.line || line > range.end.line {
                return None;
            }
            let from = if line == range.start.line {
                editor.display_col(range.start)
            } else {
                0
            };
            let to = if line == range.end.line {
                editor.display_col(range.end)
            } else {
                width.max(1)
            };
            Some((from, to))
        }
    }
}

/// The viewer, full screen: name and status on the top line, content, key bar at the bottom.
/// The quick view (`Ctrl+Q`): the viewer's own rows, inside a panel's frame, on whatever the
/// other panel's cursor is on.
fn draw_quick_view(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let title = app.i18n.t("panel-quick-view");
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);
    let base = app.theme.panel();

    app.quick_view_for(1 - index);
    // A page in the preview is a page of what is on screen, so the keys need the geometry.
    app.quick_size = (inner.w.saturating_sub(1), inner.h);
    let Some(viewer) = app.quick.as_mut() else {
        // Nothing under the cursor, or nothing we can read from here: the reason is a line of
        // its own rather than an empty frame that looks broken.
        let text = app
            .quick_why
            .clone()
            .unwrap_or_else(|| app.i18n.t("msg-nothing-to-do"));
        for (n, line) in wrap_lines(&text, inner.w as usize).into_iter().enumerate() {
            if n as u16 >= inner.h {
                break;
            }
            buf.put_str(inner.x + 1, inner.y + n as u16, &line, base, inner.w);
        }
        return;
    };

    // The name on the frame's foot, as a panel's totals are, so the preview itself is all
    // the room the rows get.
    let rows = viewer.rows(inner.w.saturating_sub(1), inner.h);
    for (n, row) in rows.iter().enumerate() {
        let text = scmd_ui::truncate_cols(&row.text, inner.w.saturating_sub(1) as usize);
        buf.put_str(
            inner.x + 1,
            inner.y + n as u16,
            &text,
            base,
            inner.w.saturating_sub(1),
        );
    }
    let name = viewer.path.display_name();
    let foot = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let name = scmd_ui::elide_middle(&name, area.w.saturating_sub(4) as usize);
    let nw = unicode_width::UnicodeWidthStr::width(name.as_str()) as u16;
    buf.put_str(
        area.x + 2,
        area.y + area.h.saturating_sub(1),
        &format!(" {name} "),
        foot,
        nw + 2,
    );
}

/// The tree panel (`Ctrl+T`): the directories under this panel's root, as far as they have
/// been opened, with the cursor on the one the other panel is showing.
fn draw_tree(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let root = app.trees[index]
        .as_ref()
        .map(|t| t.root.to_string())
        .unwrap_or_default();
    let title = format!("{}  {root}", app.i18n.t("panel-tree"));
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);
    let base = app.theme.panel();
    let on = Style::new(
        app.theme.color("panel.cursor.fg"),
        app.theme.color("panel.cursor.bg"),
    );
    let Some(tree) = app.trees[index].as_ref() else {
        return;
    };
    for n in 0..inner.h {
        let row = tree.top + n as usize;
        if row >= tree.rows.len() {
            break;
        }
        let style = if row == tree.cursor && index == app.active {
            on
        } else {
            base
        };
        let text = tree.line(row);
        let text = scmd_ui::truncate_cols(&text, inner.w.saturating_sub(1) as usize);
        // The cursor's stripe runs the width of the panel, as a file panel's does.
        buf.fill(Rect::new(inner.x, inner.y + n, inner.w, 1), ' ', style);
        buf.put_str(
            inner.x + 1,
            inner.y + n,
            &text,
            style,
            inner.w.saturating_sub(1),
        );
    }
}

/// The info panel (`Ctrl+L`): what the other panel is looking at, and what is under its
/// cursor. Everything here is already known — nothing is read from the disk to draw it.
fn draw_info(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let title = app.i18n.t("panel-info");
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);
    let base = app.theme.panel();
    let label = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("panel.bg"),
    );

    let other = &app.panels[1 - index];
    let mut lines: Vec<(String, String)> = Vec::new();
    let t = |id: &str| app.i18n.t(id);

    lines.push((t("info-path"), other.cwd.to_string()));
    lines.push((
        t("info-kind"),
        if other.cwd.in_container() {
            app.i18n.t_args(
                "info-kind-archive",
                &[(
                    "name",
                    other
                        .cwd
                        .leave()
                        .map(|h| h.display_name())
                        .unwrap_or_default()
                        .into(),
                )],
            )
        } else if other.cwd.is_remote() {
            app.i18n.t_args(
                "info-kind-remote",
                &[(
                    "host",
                    other.cwd.authority().unwrap_or("").to_string().into(),
                )],
            )
        } else if other.temp.is_some() {
            t("info-kind-list")
        } else {
            t("info-kind-local")
        },
    ));
    if let Some(free) = other.free {
        lines.push((t("info-free"), scmd_core::format_size(free)));
    }
    let totals = &other.listing.totals;
    lines.push((
        t("info-contents"),
        app.i18n.t_args(
            "info-contents-value",
            &[
                ("files", (totals.files as i64).into()),
                ("dirs", (totals.dirs as i64).into()),
                ("bytes", scmd_core::format_size(totals.bytes).into()),
            ],
        ),
    ));
    if !other.selection.is_empty() {
        lines.push((
            t("info-selected"),
            app.i18n.t_args(
                "info-selected-value",
                &[
                    ("count", (totals.selected_files as i64).into()),
                    (
                        "bytes",
                        scmd_core::format_size(totals.selected_bytes).into(),
                    ),
                ],
            ),
        ));
    }
    if let Some(entry) = other.current() {
        lines.push((String::new(), String::new()));
        lines.push((t("info-cursor"), entry.name_str().into_owned()));
        if !entry.kind.is_dirlike() {
            lines.push((t("info-size"), scmd_core::format_size(entry.size)));
        }
        if let Some(mtime) = entry.mtime {
            lines.push((t("info-modified"), widgets::format_mtime(mtime)));
        }
    }

    let width = lines
        .iter()
        .map(|(k, _)| unicode_width::UnicodeWidthStr::width(k.as_str()))
        .max()
        .unwrap_or(0) as u16;
    for (n, (key, value)) in lines.iter().enumerate() {
        let y = inner.y + n as u16;
        if y >= inner.y + inner.h {
            break;
        }
        buf.put_str(inner.x + 1, y, key, label, width);
        let x = inner.x + 2 + width;
        let room = (inner.x + inner.w).saturating_sub(x);
        let value = scmd_ui::elide_middle(value, room as usize);
        buf.put_str(x, y, &value, base, room);
    }
}

/// Break `text` into lines that fit, on spaces where there are any.
fn wrap_lines(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
            out.push(std::mem::take(&mut line));
        }
        if !line.is_empty() {
            line.push(' ');
        }
        line.push_str(word);
    }
    if !line.is_empty() {
        out.push(line);
    }
    out
}

fn draw_viewer(app: &mut App, viewer: &mut scmd_view::Viewer, buf: &mut Buffer) {
    let (w, h) = (buf.width(), buf.height());
    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let body = Rect::new(0, 1, w, h.saturating_sub(1 + keybar_h));
    app.viewer_size = (body.w, body.h);

    let base = app.theme.panel();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    // Top line: the file on the left, the status on the right.
    let header = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("status.bg"),
    );
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    let status = viewer.status();
    let status_text = app.viewer_status(&status);
    let status_w = status_text.chars().count().min(w as usize) as u16;
    let name_w = w.saturating_sub(status_w + 2);
    buf.put_str(
        0,
        0,
        &scmd_ui::elide_middle(&viewer.path.to_string(), name_w as usize),
        header,
        name_w,
    );
    buf.put_str(w - status_w, 0, &status_text, header, status_w);

    // The structure pane takes what the content does not need. In hex the row width is
    // decided by the data rather than the window, so the pane gets the remainder — that way
    // the printable gutter stays whole instead of being cut off by the pane's frame.
    let mut body = body;
    let pane = if viewer.show_structure {
        let third = (body.w / 3).clamp(28, 48);
        let pane_w = match viewer.mode {
            scmd_view::Mode::Hex => body
                .w
                .saturating_sub(viewer.hex_row_cols(body.w.saturating_sub(third)))
                // The pane absorbs the slack a fixed-width dump leaves over, rather than
                // leaving a strip of empty columns between the bytes and its frame.
                .clamp(24, 64),
            _ => third,
        }
        .min(body.w.saturating_sub(24));
        if pane_w >= 24 {
            let at = Rect::new(body.right() - pane_w, body.y, pane_w, body.h);
            body = Rect::new(body.x, body.y, body.w - pane_w, body.h);
            app.viewer_size = (body.w, body.h);
            Some(at)
        } else {
            None
        }
    } else {
        None
    };

    // Content.
    let hit = Style::new(app.theme.color("search.fg"), app.theme.color("panel.bg")).bold();
    let rows = viewer.rows(body.w, body.h);
    for (i, row) in rows.iter().enumerate() {
        let y = body.y + i as u16;
        buf.put_str(0, y, &row.text, base, body.w);
        for (start, len) in &row.hits {
            let text: String = row.text.chars().skip(*start).take(*len).collect();
            if text.is_empty() {
                continue;
            }
            let x = row
                .text
                .chars()
                .take(*start)
                .filter_map(unicode_width::UnicodeWidthChar::width)
                .sum::<usize>() as u16;
            if x < body.w {
                buf.put_str(x, y, &text, hit, body.w - x);
            }
        }
    }
    if rows.is_empty() {
        buf.put_str(0, body.y, &app.i18n.t("viewer-empty"), base, body.w);
    }

    if let Some(at) = pane {
        draw_structure(app, viewer, buf, at);
    }

    if keybar_h == 1 {
        let labels = app.keybar_viewer();
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
    }
}

/// A duration as a clock: `1:05` or `2:14:03`. Seconds alone read as a number rather than as
/// a time, and nobody watching a copy wants three decimal places.
fn clock(d: std::time::Duration) -> String {
    let secs = d.as_secs();
    let (h, m, s) = (secs / 3600, (secs % 3600) / 60, secs % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

fn status_hint(app: &App) -> String {
    cursor_line(app, app.panel())
}

/// The item under the cursor, spelled out: what the screen's status bar shows for the active
/// panel, and what each panel's own status line shows for itself.
fn cursor_line(app: &App, panel: &crate::panel::Panel) -> String {
    match panel.current() {
        Some(e) => {
            let size = if e.kind.is_dirlike() {
                String::new()
            } else {
                format!("  {}", scmd_core::format_size(e.size))
            };
            let time = e
                .mtime
                .map(|t| format!("  {}", widgets::format_mtime(t)))
                .unwrap_or_default();
            format!("{}{size}{time}", e.name_str())
        }
        None if panel.loading => app.i18n.t("panel-loading"),
        None => app.i18n.t("panel-empty"),
    }
}

/// A search: its results so far, what it is scanning, and what the keys do. Full screen,
/// because a list of results is the thing you are working with, not a message about one.
fn draw_find(app: &App, view: &crate::app::FindView, buf: &mut Buffer, screen: Rect) {
    let base = app.theme.panel();
    let frame = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let dim = Style::new(
        app.theme.color("panel.footer.fg"),
        app.theme.color("panel.bg"),
    );
    let picked = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));

    buf.fill(screen, ' ', base);
    buf.draw_frame(screen, frame, Some(&view.label));
    let inner = screen.inner();
    if inner.h < 3 {
        return;
    }
    // Two rows at the foot: what is happening, and what the keys do.
    let list_h = inner.h.saturating_sub(2) as usize;
    let name_w = inner.w.saturating_sub(28) as usize;

    for i in 0..list_h {
        let Some((path, entry)) = view.rows.get(view.top + i) else {
            break;
        };
        let y = inner.y + i as u16;
        let on = view.top + i == view.cursor;
        let style = if on { picked } else { base };
        if on {
            buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);
        }
        // The whole path, elided in the middle: on a results list the directory is the half
        // of the answer, so it cannot be the half that is cut off.
        let shown = scmd_ui::elide_middle(&path.to_string(), name_w);
        buf.put_str(inner.x, y, &shown, style, name_w as u16);
        let size = if entry.kind.is_dirlike() {
            String::new()
        } else {
            scmd_core::format_size(entry.size)
        };
        let when = entry.mtime.map(widgets::format_mtime).unwrap_or_default();
        let right = format!("{size:>9}  {when:>16}");
        let x = inner.x + name_w as u16 + 1;
        if x < inner.right() {
            buf.put_str(x, y, &right, style, inner.right() - x);
        }
    }

    let counters = if view.done {
        let seconds = view.elapsed.as_secs_f64();
        app.i18n.t_args(
            if view.stopped {
                "find-stopped"
            } else {
                "find-done"
            },
            &[
                ("count", (view.rows.len() as i64).into()),
                ("files", (view.files as i64).into()),
                ("elapsed", format!("{seconds:.1} s").into()),
            ],
        )
    } else {
        app.i18n.t_args(
            "find-results",
            &[
                ("count", (view.rows.len() as i64).into()),
                ("files", (view.files as i64).into()),
                ("dirs", (view.dirs as i64).into()),
            ],
        )
    };
    let scanning = view.scanning.as_deref().unwrap_or_default();
    let status = if scanning.is_empty() || view.done {
        counters
    } else {
        format!("{counters}  ·  {}", scmd_ui::elide_middle(scanning, 40))
    };
    let y = inner.bottom() - 2;
    buf.put_str(
        inner.x,
        y,
        &scmd_ui::truncate_cols(&status, inner.w as usize),
        dim,
        inner.w,
    );
    let hint = app.i18n.t(if view.done {
        "find-hint-done"
    } else {
        "find-hint"
    });
    buf.put_str(
        inner.x,
        inner.bottom() - 1,
        &scmd_ui::truncate_cols(&hint, inner.w as usize),
        Style::new(
            app.theme.color("panel.column.fg"),
            app.theme.color("panel.bg"),
        ),
        inner.w,
    );
}

/// The connection manager: the site list, what is connected, and what the keys do.
fn draw_sites(app: &App, buf: &mut Buffer, screen: Rect, cursor: usize) {
    let rows = app.site_rows();
    let mut lines: Vec<String> = Vec::new();
    if rows.is_empty() {
        lines.push(app.i18n.t("net-no-sites"));
    }
    let mut group: Option<String> = None;
    for site in &rows {
        if site.group != group {
            group = site.group.clone();
            if let Some(name) = &group {
                lines.push(format!("[{name}]"));
            }
        }
        let mark = if app.connected.contains(&site.authority()) {
            '*'
        } else {
            ' '
        };
        lines.push(format!(
            "{mark} {}  ({})",
            site.label(),
            site.auth.describe()
        ));
    }
    let hint = app.i18n.t("net-sites-hint");
    let title = app.i18n.t("net-sites-title");
    let area = widgets::draw_dialog(buf, &app.theme, screen, &title, &lines, Some(&hint));

    // Highlight the row under the cursor, skipping the group headings.
    let selected = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));
    let mut row = 0usize;
    for (index, line) in lines.iter().enumerate() {
        if line.starts_with('[') {
            continue;
        }
        if row == cursor && !rows.is_empty() {
            let y = area.y + 1 + index as u16;
            if y < area.y + area.h {
                // The bar spans the inside of the frame, with the text where the other
                // rows have it: one column of padding.
                let width = area.w.saturating_sub(2);
                let text = format!(" {line:<w$}", w = width.saturating_sub(1) as usize);
                buf.put_str(area.x + 1, y, &text, selected, width);
            }
            break;
        }
        row += 1;
    }
}

fn draw_help(
    app: &App,
    buf: &mut Buffer,
    chapter: crate::help::Chapter,
    scroll: usize,
    at: usize,
    screen: Rect,
) {
    let index = chapter == crate::help::Chapter::Index;
    // The written chapters are prose and want room to be read; the index is a short list and
    // looks silly stretched across a wide terminal. Prose is also easier to read short of the
    // full width, so a wide terminal gets a column rather than a sentence from edge to edge.
    let box_w = if index {
        screen.w.saturating_sub(6).clamp(24, 60)
    } else {
        screen.w.saturating_sub(4).clamp(24, 86)
    };
    // The text is laid out for the box it will be drawn in, not for the panel's width.
    let lines = app.help_lines(chapter, box_w.saturating_sub(4) as usize);
    let box_h = screen.h.saturating_sub(2).max(6);
    let x = screen.x + (screen.w - box_w) / 2;
    let y = screen.y + (screen.h - box_h) / 2;
    let area = Rect::new(x, y, box_w, box_h);

    let style = app.theme.dialog();
    buf.fill(area, ' ', style);
    // The frame names the chapter one is in, and the index names the program.
    let title = match crate::help::chapters()
        .into_iter()
        .find(|(c, _)| *c == chapter)
    {
        Some((_, name)) => name,
        // The index's frame names the program and the version: the first question a tester
        // asks, and the one place it is always to hand.
        None => app.i18n.t_args(
            "dialog-help-index",
            &[("version", env!("CARGO_PKG_VERSION").into())],
        ),
    };
    buf.draw_frame(
        area,
        Style::new(
            app.theme.color("dialog.frame"),
            app.theme.color("dialog.bg"),
        ),
        Some(&title),
    );

    let inner = area.inner();
    let visible = inner.h.saturating_sub(1) as usize;
    let max_scroll = lines.len().saturating_sub(visible);
    let scroll = scroll.min(max_scroll);
    for (row, line) in lines.iter().skip(scroll).take(visible).enumerate() {
        // In the index the row under the cursor is drawn as chosen, because the index is a
        // list one picks from rather than a page one reads.
        if index && scroll + row == at {
            let style = Style::new(
                app.theme.color("menu.selected.fg"),
                app.theme.color("menu.selected.bg"),
            );
            let text = format!(" {line:<w$}", w = inner.w.saturating_sub(1) as usize);
            buf.put_str(inner.x, inner.y + row as u16, &text, style, inner.w);
            continue;
        }
        let line_style = if line.starts_with('[') {
            Style::new(
                app.theme.color("dialog.title"),
                app.theme.color("dialog.bg"),
            )
            .bold()
        } else {
            style
        };
        buf.put_str(inner.x, inner.y + row as u16, line, line_style, inner.w);
    }
    // The last body line names the configuration directory: the first thing anyone asks.
    if index && lines.len().saturating_sub(scroll) < visible {
        let row = lines.len() - scroll;
        buf.put_str(
            inner.x,
            inner.y + row as u16 + 1,
            &format!("config: {}", app.config_file().display()),
            style,
            inner.w,
        );
    }
    let hint = app.i18n.t(if index {
        "dialog-help-hint-index"
    } else {
        "dialog-help-hint"
    });
    buf.put_str(
        inner.x,
        area.bottom() - 1,
        &scmd_ui::truncate_cols(&hint, inner.w as usize),
        Style::new(
            app.theme.color("dialog.title"),
            app.theme.color("dialog.bg"),
        ),
        inner.w,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use scmd_cmd::Keymap;
    use scmd_config::{Config, Paths};
    use scmd_core::{Entry, EntryFlags, EntryId, EntryKind, VPath};
    use scmd_i18n::I18n;
    use scmd_tui::Input;
    use scmd_ui::{Theme, ThemeSet};
    use std::ffi::OsString;
    use std::path::Path;

    use crate::app::{Event, Startup};

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            btime: None,
            atime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn app(theme: &str, lang: &str) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(Path::new("/tmp/scmd-test/scmd.toml")), None),
            i18n: I18n::new(lang),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::builtin(theme).unwrap(),
            left: VPath::local("/home/mb/project"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
        });
        app.requests.clear();
        for panel in 0..2 {
            app.handle_event(Event::Listed {
                panel,
                generation: 1,
                result: Ok(vec![
                    Entry::parent_entry(),
                    entry("src", EntryKind::Dir, 0),
                    entry("Cargo.toml", EntryKind::File, 412),
                ]),
            });
        }
        app
    }

    fn frame(app: &mut App, w: u16, h: u16) -> String {
        let mut buf = Buffer::new(w, h);
        draw(app, &mut buf);
        buf.to_text()
    }

    #[test]
    fn a_full_frame_has_two_panels_a_status_line_and_a_key_bar() {
        let mut app = app("commander", "en");
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 24);
        assert!(lines[0].contains("/home/mb/project"), "{out}");
        assert!(lines[0].contains("/tmp"), "{out}");
        assert!(
            lines[23].contains("Help") && lines[23].contains("Quit"),
            "{out}"
        );
        assert!(out.contains("Cargo.toml"), "{out}");
    }

    #[test]
    fn the_status_line_describes_the_entry_under_the_cursor() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("Down".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Cargo.toml  412 B"), "{out}");
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_help() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("F1".parse().unwrap()));
        println!("{}", frame(&mut app, 100, 30));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        println!("{}", frame(&mut app, 100, 30));
    }

    #[test]
    fn the_help_opens_on_its_contents_and_draws_a_chapter() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("F1".parse().unwrap()));
        let index = frame(&mut app, 80, 24);
        // The frame names the program and the version, and the body is the chapter list.
        assert!(index.contains("SupremeCommander"), "{index}");
        assert!(index.contains("What this is"), "{index}");
        assert!(index.contains("Every command"), "{index}");
        assert!(index.contains("Enter opens a chapter"), "{index}");

        // Into the keys chapter: its own title in the frame, and the bindings in the body.
        let keys_at = crate::help::chapters()
            .iter()
            .position(|(c, _)| *c == crate::help::Chapter::Keys)
            .unwrap();
        for _ in 0..keys_at {
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let keys = frame(&mut app, 80, 24);
        assert!(keys.contains("Keys, as they are bound now"), "{keys}");
        assert!(
            keys.contains("[global]") && keys.contains("app.quit"),
            "{keys}"
        );
        // Bindings further down are reachable by scrolling.
        app.handle_input(Input::Key("PgDn".parse().unwrap()));
        let scrolled = frame(&mut app, 80, 24);
        assert_ne!(scrolled, keys, "PgDn did not scroll the chapter");

        // And a written chapter is prose, drawn wide.
        app.handle_input(Input::Key("Esc".parse().unwrap()));
        app.handle_input(Input::Key("Home".parse().unwrap()));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let prose = frame(&mut app, 80, 24);
        assert!(prose.contains("two-panel file manager"), "{prose}");
    }

    #[test]
    fn a_prompt_is_drawn_over_the_panels_with_a_cursor() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Ctrl+Shift+L".parse().unwrap()));
        let mut buf = Buffer::new(80, 24);
        let cursor = draw(&mut app, &mut buf);
        assert!(cursor.is_some());
        assert!(
            buf.to_text().contains("Change directory"),
            "{}",
            buf.to_text()
        );
    }

    #[test]
    fn every_theme_renders_a_frame_without_panicking() {
        for id in ["commander", "black", "white", "matrix", "tron", "blade"] {
            let mut app = app(id, "en");
            let out = frame(&mut app, 60, 15);
            assert!(out.contains("src/"), "theme {id} rendered nothing: {out}");
        }
    }

    #[test]
    fn the_command_line_takes_a_row_and_shows_where_it_will_run() {
        let mut app = app("far", "en");
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        // One row above the status line, with the panel's path as the prompt.
        let cmd_row = lines[lines.len() - 3];
        assert!(cmd_row.contains("/home/mb/project>"), "{cmd_row}");

        app.cmd.set("make -j8");
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[lines.len() - 3].contains("make -j8"), "{out}");

        // Switched off, the panels take the row back.
        app.cfg.ui.command_line = false;
        let taller = frame(&mut app, 80, 24);
        assert!(!taller.contains("make -j8"), "{taller}");
    }

    #[test]
    fn the_user_screen_shows_what_ran_and_how_it_went() {
        let mut app = app("far", "en");
        app.user_screen = vec!["$ make".into(), "cc -c main.c".into(), "error: no".into()];
        app.mode = crate::app::Mode::UserScreen { scroll: 0 };
        let out = frame(&mut app, 80, 12);
        assert!(out.contains("$ make"), "{out}");
        assert!(out.contains("cc -c main.c"), "{out}");
        // The panels are gone: the output owns the screen.
        assert!(!out.contains("Cargo.toml"), "{out}");
    }

    #[test]
    fn the_panel_wears_fars_chrome() {
        let mut app = app("far", "en");
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        // Row 1 is the column titles, with the sort letter in the corner: lower case for
        // ascending order, upper case for descending.
        assert!(
            lines[1].contains("Name") && lines[1].contains("Size"),
            "{out}"
        );
        assert!(
            lines[1].starts_with("│n"),
            "no sort letter in the corner: {}",
            lines[1]
        );
        // The footer no longer repeats the sort mode.
        assert!(!out.contains("[name]"), "{out}");

        app.panels[0].sort.reverse = true;
        app.panels[0].rebuild();
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].starts_with("│N"), "{}", lines[1]);

        // A filter puts a mark next to the letter, as FAR does.
        app.panels[0].filter = Some("rs".into());
        app.panels[0].rebuild();
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].starts_with("│N*"), "{}", lines[1]);
    }

    #[test]
    fn the_titles_row_costs_one_file_row_and_the_panel_knows_it() {
        let mut with = app("far", "en");
        let rows_with = {
            let mut buf = Buffer::new(80, 24);
            draw(&mut with, &mut buf);
            with.rows
        };
        let mut without = app("far", "en");
        without.cfg.panel.column_titles = false;
        let rows_without = {
            let mut buf = Buffer::new(80, 24);
            draw(&mut without, &mut buf);
            without.rows
        };
        assert_eq!(rows_without, rows_with + 1);
    }

    #[test]
    fn the_menu_bar_shows_five_menus_and_pulls_one_down() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("F9".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        let bar = out.lines().next().unwrap();
        for title in ["Left", "Files", "Commands", "Options", "Right"] {
            assert!(bar.contains(title), "`{title}` is not on the bar: {bar}");
        }
        // The open menu shows its entries, the key that does the same thing, and a mark on
        // the view the panel is in.
        assert!(out.contains("Brief"), "{out}");
        assert!(out.contains("Ctrl+1"), "{out}");
        assert!(
            out.contains('√'),
            "nothing is marked as being in force: {out}"
        );
    }

    #[test]
    fn the_menu_never_advertises_a_key_that_does_something_else() {
        let app = app("commander", "en");
        for m in crate::menu::menus() {
            for entry in &m.items {
                let crate::menu::Entry::Item(item) = entry else {
                    continue;
                };
                let crate::menu::Action::Todo(_) = item.action else {
                    continue;
                };
                let Ok(key) = item.keys.parse::<scmd_cmd::Key>() else {
                    continue;
                };
                if let Some(cmd) = app.keymap.resolve(&[scmd_cmd::Scope::Panel], key) {
                    panic!(
                        "{} shows {} as its key, but that key runs {cmd}",
                        item.label, item.keys
                    );
                }
            }
        }
    }

    #[test]
    fn a_settings_dialog_is_drawn_with_its_checkboxes_and_buttons() {
        let mut app = app("commander", "en");
        app.run(scmd_cmd::Command::OptConfirmations);
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Confirmations"), "{out}");
        assert!(out.contains("[x] Delete"), "{out}");
        assert!(out.contains("OK") && out.contains("Cancel"), "{out}");
    }

    #[test]
    fn the_language_list_is_drawn_with_the_current_language_marked() {
        let mut app = app("commander", "en");
        app.run(scmd_cmd::Command::AppLanguages);
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Italiano"), "{out}");
        assert!(out.contains("√ English"), "{out}");
    }

    #[test]
    fn every_menu_and_dialog_fits_a_small_terminal_in_every_language() {
        for (tag, _) in scmd_i18n::LANGUAGES {
            let mut app = app("commander", tag);
            app.handle_input(Input::Key("F9".parse().unwrap()));
            for menu in 0..5 {
                for size in [(80u16, 24u16), (60, 15), (40, 10)] {
                    app.mode = crate::app::Mode::Menu { menu, item: 0 };
                    let out = frame(&mut app, size.0, size.1);
                    assert_eq!(out.lines().count(), size.1 as usize, "{tag} menu {menu}");
                }
            }
            for cmd in [
                scmd_cmd::Command::OptSystem,
                scmd_cmd::Command::OptPanel,
                scmd_cmd::Command::OptInterface,
                scmd_cmd::Command::OptConfirmations,
                scmd_cmd::Command::AppLanguages,
                scmd_cmd::Command::AppThemes,
                scmd_cmd::Command::PanelSortMenu,
                scmd_cmd::Command::PanelHistory,
                scmd_cmd::Command::PanelDrive(scmd_cmd::Side::Left),
            ] {
                app.run(cmd);
                for size in [(80u16, 24u16), (60, 15), (40, 10)] {
                    let out = frame(&mut app, size.0, size.1);
                    assert_eq!(out.lines().count(), size.1 as usize, "{tag} {cmd}");
                }
            }
        }
    }

    /// A small but well-formed ELF64: enough header for the pane to decode, one section table
    /// to jump into.
    fn elf_bytes() -> Vec<u8> {
        let mut f = vec![0u8; 0x200];
        f[..4].copy_from_slice(b"\x7fELF");
        f[4] = 2; // ELF64
        f[5] = 1; // little endian
        f[6] = 1;
        f[16..18].copy_from_slice(&2u16.to_le_bytes()); // EXEC
        f[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // x86-64
        f[24..32].copy_from_slice(&0x1000u64.to_le_bytes()); // entry
        f[40..48].copy_from_slice(&0x80u64.to_le_bytes()); // e_shoff
        f[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        f[60..62].copy_from_slice(&1u16.to_le_bytes()); // e_shnum
        f
    }

    fn app_with_viewer(name: &str, bytes: &[u8]) -> (tempfile::TempDir, App) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(name), bytes).unwrap();
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.path().join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::builtin("far").unwrap(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
        });
        app.requests.clear();
        let entries = scmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &scmd_core::CancelToken::new(),
        )
        .unwrap();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(entries),
        });
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F3".parse().unwrap()));
        (dir, app)
    }

    #[test]
    fn f11_decodes_the_file_beside_the_bytes() {
        let (_dir, mut app) = app_with_viewer("prog.bin", &elf_bytes());
        app.handle_input(Input::Key("F4".parse().unwrap())); // hex
        let out = frame(&mut app, 100, 20);
        assert!(!out.contains("Structure"), "the pane starts closed");

        app.handle_input(Input::Key("F11".parse().unwrap()));
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("Structure"), "{out}");
        assert!(out.contains("e_machine"), "{out}");
        assert!(out.contains("x86-64"), "{out}");
        assert!(out.contains("EXEC"), "{out}");
        // The content keeps the rest of the width, and its rows are still hex.
        assert!(out.contains("7F 45 4C 46"), "{out}");
        assert!(app.viewer_size.0 < 100, "the content did not make room");

        app.handle_input(Input::Key("F11".parse().unwrap()));
        let out = frame(&mut app, 100, 20);
        assert!(!out.contains("Structure"), "F11 closes it again");
    }

    #[test]
    fn enter_in_the_pane_takes_the_bytes_there() {
        let (_dir, mut app) = app_with_viewer("prog.bin", &elf_bytes());
        app.handle_input(Input::Key("F11".parse().unwrap()));
        let _ = frame(&mut app, 100, 20);
        // Walk down to the row for the section table, which points at 0x80.
        let mut found = false;
        for _ in 0..24 {
            let viewer = app.mode.viewer_mut().unwrap();
            if viewer.structure_target() == Some(0x80) {
                found = true;
                break;
            }
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        assert!(found, "no row pointed at the section table");
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let viewer = app.mode.viewer_mut().unwrap();
        assert_eq!(viewer.top, 0x80, "Enter did not move the bytes");
        // An offset into a binary is bytes: a file that opened in dump mode already shows
        // them, and one that opened as text switches to hex rather than guessing a line.
        assert!(
            matches!(viewer.mode, scmd_view::Mode::Hex | scmd_view::Mode::Dump),
            "{:?}",
            viewer.mode
        );
        assert!(app.status.contains("0x80"), "{}", app.status);
    }

    #[test]
    fn the_arrows_go_back_to_the_bytes_when_the_pane_lets_go() {
        let (_dir, mut app) = app_with_viewer("prog.bin", &elf_bytes());
        app.handle_input(Input::Key("F4".parse().unwrap()));
        app.handle_input(Input::Key("F11".parse().unwrap()));
        let _ = frame(&mut app, 100, 20);
        let before = app.mode.viewer_mut().unwrap().top;
        app.handle_input(Input::Key("Down".parse().unwrap()));
        assert_eq!(
            app.mode.viewer_mut().unwrap().top,
            before,
            "with the pane focused the arrows belong to it"
        );
        app.handle_input(Input::Key("Tab".parse().unwrap()));
        app.handle_input(Input::Key("Down".parse().unwrap()));
        assert_ne!(
            app.mode.viewer_mut().unwrap().top,
            before,
            "after Tab the arrows scroll the file again"
        );
    }

    #[test]
    fn f11_on_a_text_file_says_there_is_nothing_to_decode() {
        let (_dir, mut app) = app_with_viewer("notes.txt", b"nothing binary here\n");
        app.handle_input(Input::Key("F11".parse().unwrap()));
        assert!(app.status_error, "{}", app.status);
        assert!(!app.mode.viewer().unwrap().show_structure);
        let out = frame(&mut app, 100, 20);
        assert!(!out.contains("Structure"), "{out}");
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_structure() {
        // Our own binary is to hand and is a real ELF.
        let path = std::env::current_exe().unwrap();
        let mut app = app("far", "en");
        let source = scmd_view::source::FileSource::open(&path).unwrap();
        let mut viewer = scmd_view::Viewer::new(Box::new(source), VPath::local(&path));
        viewer.set_mode(scmd_view::Mode::Hex);
        assert!(viewer.toggle_structure(), "our own binary must decode");
        app.mode = crate::app::Mode::Viewer(Box::new(viewer));
        println!("=== structure pane ===\n{}", frame(&mut app, 100, 26));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball() {
        let mut editor = app("commander", "en");
        editor.open_site_form(None);
        println!("=== session editor ===\n{}", frame(&mut editor, 80, 24));
        for cmd in [
            scmd_cmd::Command::AppMenu,
            scmd_cmd::Command::OptSystem,
            scmd_cmd::Command::OptPanel,
            scmd_cmd::Command::OptInterface,
            scmd_cmd::Command::OptConfirmations,
            scmd_cmd::Command::AppLanguages,
            scmd_cmd::Command::PanelSortMenu,
            scmd_cmd::Command::PanelDrive(scmd_cmd::Side::Left),
        ] {
            let mut app = app("commander", "en");
            app.run(cmd);
            if let crate::app::Mode::Menu { .. } = app.mode {
                app.mode = crate::app::Mode::Menu { menu: 1, item: 0 };
            }
            println!("=== {cmd} ===\n{}", frame(&mut app, 80, 24));
        }
    }

    /// A project directory as it really looks, so a screenshot shows columns doing work
    /// rather than three rows of placeholder.
    fn project(app: &mut App) {
        let at = |days: u64| {
            Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(days * 86_400))
        };
        let mut rows = vec![Entry::parent_entry()];
        let dirs = [
            ("crates", 20_700),
            ("docs", 20_720),
            ("target", 20_740),
            (".git", 20_690),
        ];
        for (name, day) in dirs {
            let mut e = entry(name, EntryKind::Dir, 0);
            e.mtime = at(day);
            if name.starts_with('.') {
                e.flags.insert(EntryFlags::HIDDEN);
            }
            rows.push(e);
        }
        let files = [
            ("Cargo.toml", 1_284u64, 20_741),
            ("Cargo.lock", 48_912, 20_741),
            ("CHANGELOG.md", 7_430, 20_740),
            ("LICENSE", 1_513, 20_612),
            ("README.md", 12_088, 20_739),
            ("rust-toolchain.toml", 94, 20_612),
            ("scmd.toml", 2_240, 20_738),
        ];
        for (name, size, day) in files {
            let mut e = entry(name, EntryKind::File, size);
            e.mtime = at(day);
            rows.push(e);
        }
        for panel in 0..2 {
            app.handle_event(Event::Listed {
                panel,
                generation: 1,
                result: Ok(rows.clone()),
            });
        }
    }

    fn type_line(app: &mut App, text: &str) {
        for c in text.chars() {
            app.handle_input(Input::Key(scmd_cmd::Key::new(
                scmd_cmd::KeyCode::Char(c),
                scmd_cmd::Mods::NONE,
            )));
        }
    }

    fn frame_buf(app: &mut App, w: u16, h: u16) -> Buffer {
        let mut buf = Buffer::new(w, h);
        draw(app, &mut buf);
        buf
    }

    /// Every screen worth a look, each as the buffer a terminal would actually receive.
    fn scenes() -> Vec<(String, Buffer)> {
        const W: u16 = 100;
        const H: u16 = 28;
        let mut out: Vec<(String, Buffer)> = Vec::new();

        let mut a = app("far", "en");
        project(&mut a);
        out.push(("panel".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        for _ in 0..4 {
            a.handle_input(Input::Key("Down".parse().unwrap()));
        }
        a.handle_input(Input::Key("Insert".parse().unwrap()));
        a.handle_input(Input::Key("Insert".parse().unwrap()));
        type_line(&mut a, "cargo test -p scmd-view structure");
        out.push((
            "a command typed, two files marked".into(),
            frame_buf(&mut a, W, H),
        ));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(scmd_cmd::Command::PanelFilterQuick);
        type_line(&mut a, "*.toml,*.md|Cargo*");
        out.push(("quick filter".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.user_screen = vec![
            "scmd $ cargo build --release".into(),
            "   Compiling scmd-core v0.0.3".into(),
            "   Compiling scmd-view v0.0.3".into(),
            "    Finished `release` profile [optimized] in 41.72s".into(),
            String::new(),
            "scmd $ git log --oneline -3".into(),
            "0d7fbfe feat(core): one filter object".into(),
            "3b19a04 feat(view): a structure pane for binaries".into(),
            "a71c2e8 feat(cmd): the command line".into(),
        ];
        a.run(scmd_cmd::Command::ShellOutputShow);
        out.push(("user screen".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(scmd_cmd::Command::PanelViewSet(scmd_core::ViewMode::Brief));
        out.push(("brief view".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(scmd_cmd::Command::AppMenu);
        out.push(("menu bar".into(), frame_buf(&mut a, W, H)));

        for (title, cmd) in [
            ("system settings", scmd_cmd::Command::OptSystem),
            ("panel settings", scmd_cmd::Command::OptPanel),
            ("interface settings", scmd_cmd::Command::OptInterface),
            ("confirmations", scmd_cmd::Command::OptConfirmations),
            ("sort modes", scmd_cmd::Command::PanelSortMenu),
            (
                "drives",
                scmd_cmd::Command::PanelDrive(scmd_cmd::Side::Left),
            ),
        ] {
            let mut a = app("far", "en");
            project(&mut a);
            a.run(cmd);
            out.push((title.into(), frame_buf(&mut a, W, H)));
        }

        let mut a = app("far", "en");
        project(&mut a);
        a.open_site_form(None);
        out.push(("session editor".into(), frame_buf(&mut a, W, H)));

        // Our own binary is to hand and is a real ELF.
        let path = std::env::current_exe().unwrap();
        let mut a = app("far", "en");
        if let Ok(source) = scmd_view::source::FileSource::open(&path) {
            let mut viewer = scmd_view::Viewer::new(Box::new(source), VPath::local(&path));
            viewer.set_mode(scmd_view::Mode::Hex);
            if viewer.toggle_structure() {
                a.mode = crate::app::Mode::Viewer(Box::new(viewer));
                out.push(("structure pane".into(), frame_buf(&mut a, W, H)));
            }
        }

        out
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_panels() {
        for (title, buf) in scenes() {
            println!("=== {title} ===\n{}", buf.to_text());
        }
    }

    /// The same screens as HTML, one `<pre>` each, with the cell colours the theme really
    /// produced: a screenshot that can go in a release note without a terminal recording.
    /// Writes to `$SCMD_SHOTS` and does nothing when that is unset.
    #[test]
    #[ignore = "writes a file, on request"]
    fn shots_html() {
        let Ok(dest) = std::env::var("SCMD_SHOTS") else {
            return;
        };
        let mut page = String::new();
        for (title, buf) in scenes() {
            page.push_str(&format!(
                "<section data-title=\"{title}\"><h3>{title}</h3>\n{}</section>\n",
                as_html(&buf)
            ));
        }
        std::fs::write(&dest, page).unwrap();
    }

    /// One `<pre>` of `<span>` runs, a run per stretch of cells sharing a style.
    fn as_html(buf: &Buffer) -> String {
        let mut out = String::from("<pre class=\"shot\">");
        for y in 0..buf.height() {
            let mut run = String::new();
            let mut style: Option<Style> = None;
            let flush = |out: &mut String, run: &mut String, style: &Option<Style>| {
                if run.is_empty() {
                    return;
                }
                let s = style.unwrap_or_default();
                out.push_str(&format!(
                    "<span style=\"color:{};background:{}{}\">{}</span>",
                    css(s.fg, "#c0c0c0"),
                    css(s.bg, "#000000"),
                    if s.bold { ";font-weight:700" } else { "" },
                    escape(run)
                ));
                run.clear();
            };
            for x in 0..buf.width() {
                let cell = buf.get(x, y).expect("in range");
                if cell.wide_tail {
                    continue;
                }
                if style != Some(cell.style) {
                    flush(&mut out, &mut run, &style);
                    style = Some(cell.style);
                }
                run.push(cell.ch);
            }
            flush(&mut out, &mut run, &style);
            out.push('\n');
        }
        out.push_str("</pre>");
        out
    }

    fn css(color: scmd_ui::Color, fallback: &str) -> String {
        // The sixteen console colours, so a theme that names them lands on the same values a
        // terminal would show.
        const ANSI: [&str; 16] = [
            "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
            "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
        ];
        match color {
            scmd_ui::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            scmd_ui::Color::Ansi(n) => ANSI[(n as usize).min(15)].to_string(),
            scmd_ui::Color::Indexed(n) if (n as usize) < 16 => ANSI[n as usize].to_string(),
            scmd_ui::Color::Indexed(_) | scmd_ui::Color::Default => fallback.to_string(),
        }
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    #[test]
    fn one_panel_can_take_the_whole_screen() {
        let mut app = app("far", "en");
        project(&mut app);
        app.run(scmd_cmd::Command::PanelToggle(scmd_cmd::Side::Left));
        assert_eq!(app.hidden, Some(0));
        assert_eq!(app.active, 1, "the keys followed the screen");
        let out = frame(&mut app, 100, 28);
        let top = out.lines().next().unwrap();
        assert_eq!(
            top.matches('┌').count(),
            1,
            "two panels are still drawn: {top}"
        );
        assert!(top.contains("/tmp"), "{top}");
        // The same key brings it back.
        app.run(scmd_cmd::Command::PanelToggle(scmd_cmd::Side::Left));
        assert_eq!(app.hidden, None);
        let out = frame(&mut app, 100, 28);
        assert_eq!(out.lines().next().unwrap().matches('┌').count(), 2);
    }

    #[test]
    fn the_panel_status_line_shows_the_item_under_the_cursor() {
        let mut app = app("far", "en");
        project(&mut app);
        let without = frame(&mut app, 100, 28);
        app.cfg.panel.status_line = true;
        app.handle_input(Input::Key("Down".parse().unwrap()));
        let with = frame(&mut app, 100, 28);
        assert!(with.contains("├"), "no rule above the status row: {with}");
        // The name is on its own row inside the panel, not only in the screen's status bar.
        let rows: Vec<&str> = with
            .lines()
            .filter(|l| l.contains("crates") && !l.contains("FOLDER"))
            .collect();
        assert!(!rows.is_empty(), "the status row is missing: {with}");
        assert!(
            app.rows < without.lines().count(),
            "the status row has to cost a file row"
        );
    }

    #[test]
    fn the_scrollbar_appears_only_when_there_is_more_than_fits() {
        let mut app = app("far", "en");
        project(&mut app);
        let short = frame(&mut app, 100, 28);
        assert!(!short.contains('█'), "a short listing drew a bar: {short}");
        let many: Vec<Entry> = (0..200)
            .map(|i| entry(&format!("file{i:03}.rs"), EntryKind::File, 10))
            .collect();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(many),
        });
        let long = frame(&mut app, 100, 28);
        assert!(long.contains('█'), "no thumb: {long}");
        assert!(long.contains('░'), "no track: {long}");
    }

    #[test]
    fn free_space_sits_in_the_frame_and_the_setting_takes_it_away() {
        let mut app = app("far", "en");
        project(&mut app);
        app.handle_event(Event::FreeSpace {
            panel: 0,
            bytes: Some(12 * 1024 * 1024 * 1024),
        });
        let out = frame(&mut app, 100, 28);
        assert!(out.contains("12.0 G free"), "{out}");
        app.cfg.panel.free_space = false;
        let out = frame(&mut app, 100, 28);
        assert!(!out.contains("free"), "{out}");
    }

    #[test]
    fn a_long_name_scrolls_sideways() {
        let mut app = app("far", "en");
        let long = "a-name-longer-than-the-column-will-ever-be.tar.gz";
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(vec![Entry::parent_entry(), entry(long, EntryKind::File, 1)]),
        });
        let before = frame(&mut app, 100, 12);
        assert!(before.contains("a-name-longer"), "{before}");
        app.run(scmd_cmd::Command::PanelScrollNames(1));
        app.run(scmd_cmd::Command::PanelScrollNames(1));
        let after = frame(&mut app, 100, 12);
        assert!(after.contains("name-longer"), "{after}");
        assert!(!after.contains("a-name-longer"), "it did not move: {after}");
        // Going back past the start stops at the start rather than wrapping.
        for _ in 0..5 {
            app.run(scmd_cmd::Command::PanelScrollNames(-1));
        }
        assert_eq!(app.panels[0].name_scroll, 0);
    }

    #[test]
    fn every_language_renders_the_key_bar() {
        for (tag, _) in scmd_i18n::LANGUAGES {
            let mut app = app("commander", tag);
            let out = frame(&mut app, 80, 24);
            let keybar = out.lines().last().unwrap().to_string();
            assert!(
                keybar.contains('1') && keybar.len() > 20,
                "language {tag} produced `{keybar}`"
            );
        }
    }

    #[test]
    fn tiny_terminals_say_so_instead_of_rendering_garbage() {
        let mut app = app("commander", "en");
        let out = frame(&mut app, 18, 4);
        assert!(out.contains("too small"), "{out}");
    }

    #[test]
    fn the_delete_confirmation_lists_what_will_go() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F8".parse().unwrap()));
        let out = frame(&mut app, 70, 20);
        assert!(out.contains("Delete"), "{out}");
        assert!(out.contains("src"), "{out}");
        assert!(out.contains("Enter = yes"), "{out}");
    }

    #[test]
    fn the_progress_overlay_renders_while_a_job_runs() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F5".parse().unwrap()));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let id = match app.take_requests().into_iter().next() {
            Some(crate::app::Request::Job { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        app.handle_event(Event::JobProgress {
            id,
            progress: scmd_jobs::Progress {
                files_done: 3,
                files_total: 10,
                bytes_done: 1024,
                bytes_total: 4096,
                file_bytes_done: 512,
                file_bytes_total: 1024,
                current: Some(VPath::local("/home/mb/project/src/main.rs")),
                ..scmd_jobs::Progress::default()
            },
        });
        let out = frame(&mut app, 70, 20);
        assert!(out.contains("Copying"), "{out}");
        assert!(out.contains("main.rs"), "{out}");
        assert!(out.contains("3 / 10 files"), "{out}");
        assert!(out.contains(" 25%"), "{out}");
        assert!(out.contains(" 50%"), "the per-file bar is missing: {out}");
        assert!(out.contains("Esc cancels"), "{out}");
    }

    #[test]
    fn a_conflict_dialog_shows_both_sides_and_the_buttons() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F5".parse().unwrap()));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let id = match app.take_requests().into_iter().next() {
            Some(crate::app::Request::Job { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        let (reply, _answers) = std::sync::mpsc::channel();
        app.handle_event(Event::JobQuestion {
            id,
            question: scmd_jobs::Question::Conflict {
                src: VPath::local("/home/mb/project/src"),
                dst: VPath::local("/tmp/src"),
                src_size: 412,
                dst_size: 1024,
                src_newer: true,
            },
            reply,
        });
        let out = frame(&mut app, 76, 20);
        assert!(out.contains("The destination exists"), "{out}");
        assert!(out.contains("/tmp/src") && out.contains("1.00 K"), "{out}");
        assert!(
            out.contains("[ Overwrite ]") && out.contains("[ Cancel ]"),
            "{out}"
        );
        assert!(out.contains("the source is newer"), "{out}");
    }

    #[test]
    fn job_overlays_render_in_every_language() {
        for (tag, _) in scmd_i18n::LANGUAGES {
            let mut app = app("commander", tag);
            app.handle_input(Input::Key("Down".parse().unwrap()));
            app.handle_input(Input::Key("F8".parse().unwrap()));
            let out = frame(&mut app, 76, 20);
            for line in out.lines() {
                assert!(
                    unicode_width::UnicodeWidthStr::width(line) <= 76,
                    "language {tag} overflowed the screen: {line:?}"
                );
            }
            assert!(out.contains("src"), "language {tag}: {out}");
        }
    }

    #[test]
    fn the_viewer_takes_the_whole_screen_with_a_status_and_its_own_key_bar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("readme.txt"),
            b"first line
second line
third line
",
        )
        .unwrap();
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.path().join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
        });
        app.requests.clear();
        let entries = scmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &scmd_core::CancelToken::new(),
        )
        .unwrap();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(entries.clone()),
        });
        app.handle_event(Event::Listed {
            panel: 1,
            generation: 1,
            result: Ok(entries),
        });

        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F3".parse().unwrap()));
        let out = frame(&mut app, 100, 12);
        let lines: Vec<&str> = out.lines().collect();

        assert!(lines[0].contains("readme.txt"), "{out}");
        assert!(
            lines[0].contains("UTF-8") && lines[0].contains('%'),
            "{out}"
        );
        assert!(lines[1].contains("first line"), "{out}");
        assert!(lines[3].contains("third line"), "{out}");
        assert!(
            lines[11].contains("Wrap") && lines[11].contains("Hex"),
            "{out}"
        );
        assert!(lines[11].contains("Search"), "{out}");
        // The panels are gone: the viewer owns the screen.
        assert!(!out.contains('┌'), "a panel frame is still visible: {out}");

        // A search hit is highlighted with the theme's search colour.
        app.handle_input(Input::Key("F7".parse().unwrap()));
        for c in ["s", "e", "c"] {
            app.handle_input(Input::Key(c.parse().unwrap()));
        }
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let mut buf = Buffer::new(100, 12);
        draw(&mut app, &mut buf);
        let highlighted = (0..buf.height())
            .flat_map(|y| (0..buf.width()).map(move |x| (x, y)))
            .filter(|(x, y)| buf.get(*x, *y).unwrap().style.fg == app.theme.color("search.fg"))
            .count();
        assert!(highlighted >= 3, "the search hit is not highlighted");
    }

    #[test]
    fn the_viewer_renders_a_binary_as_hex_without_overflowing() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = vec![0x7f, b'E', b'L', b'F'];
        bytes.extend((0..200u16).map(|i| (i % 251) as u8));
        std::fs::write(dir.path().join("prog.bin"), &bytes).unwrap();
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.path().join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
        });
        app.requests.clear();
        let entries = scmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &scmd_core::CancelToken::new(),
        )
        .unwrap();
        app.handle_event(Event::Listed {
            panel: 0,
            generation: 1,
            result: Ok(entries),
        });
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("F3".parse().unwrap()));
        // A binary opens in dump mode; F4 switches to hex.
        assert_eq!(app.mode.viewer().unwrap().mode, scmd_view::Mode::Dump);
        app.handle_input(Input::Key("F4".parse().unwrap()));
        let out = frame(&mut app, 80, 10);
        assert!(out.contains("00000000  7F 45 4C 46"), "{out}");
        for line in out.lines() {
            assert!(
                unicode_width::UnicodeWidthStr::width(line) <= 80,
                "overflow: {line:?}"
            );
        }
    }

    #[test]
    fn brief_view_geometry_is_fed_back_to_the_app() {
        let mut app = app("commander", "en");
        app.run(scmd_cmd::Command::PanelViewSet(scmd_core::ViewMode::Brief));
        let _ = frame(&mut app, 100, 30);
        assert!(
            app.columns >= 2,
            "brief view should use columns: {}",
            app.columns
        );
        assert!(app.rows > 20, "rows = {}", app.rows);
    }

    /// A panel over a real directory, so `F4` can open a real file.
    fn app_on_disk(dir: &std::path::Path) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.join("scmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir),
            right: VPath::local(dir),
            sites: Vec::new(),
        });
        app.requests.clear();
        let entries =
            scmd_vfs::local::list_blocking(&VPath::local(dir), &scmd_core::CancelToken::new())
                .unwrap();
        for panel in 0..2 {
            app.handle_event(Event::Listed {
                panel,
                generation: 1,
                result: Ok(entries.clone()),
            });
        }
        app
    }

    fn press(app: &mut App, key: &str) {
        app.handle_input(Input::Key(key.parse().unwrap()));
    }

    #[test]
    fn the_editor_takes_the_whole_screen_with_a_status_and_its_own_key_bar() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("notes.txt"),
            b"first line\nsecond line\nthird line\n",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");

        let mut buf = Buffer::new(100, 12);
        let cursor = draw(&mut app, &mut buf);
        let out = buf.to_text();
        let lines: Vec<&str> = out.lines().collect();

        assert!(lines[0].contains("notes.txt"), "{out}");
        assert!(lines[0].contains("1:1"), "the position is missing: {out}");
        assert!(lines[0].contains("UTF-8"), "{out}");
        assert!(lines[1].contains("first line"), "{out}");
        assert!(lines[3].contains("third line"), "{out}");
        assert!(
            lines[11].contains("Save") && lines[11].contains("Quit"),
            "{out}"
        );
        // The panels are gone: the editor owns the screen.
        assert!(!out.contains('┌'), "a panel frame is still visible: {out}");
        assert_eq!(cursor, Some((0, 1)), "the cursor is not on the first char");

        // The cursor follows the text, tabs and all.
        press(&mut app, "Down");
        press(&mut app, "End");
        let mut buf = Buffer::new(100, 12);
        let cursor = draw(&mut app, &mut buf);
        assert_eq!(cursor, Some((11, 2)));
        assert!(
            buf.to_text().lines().next().unwrap().contains("2:12"),
            "{}",
            buf.to_text()
        );
        // The editor is still open under its own frame.
        assert!(app.mode.editor().is_some());
    }

    #[test]
    fn a_block_is_highlighted_and_tabs_are_expanded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"\tindented\nplain text\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");

        let mut buf = Buffer::new(40, 8);
        draw(&mut app, &mut buf);
        let first = buf.to_text().lines().nth(1).unwrap().to_string();
        assert!(
            first.starts_with("    indented"),
            "the tab was not expanded: {first:?}"
        );

        // Shift+Down marks a stream block over the first line.
        press(&mut app, "Shift+Down");
        let mut buf = Buffer::new(40, 8);
        draw(&mut app, &mut buf);
        let highlighted = (0..buf.height())
            .flat_map(|y| (0..buf.width()).map(move |x| (x, y)))
            .filter(|(x, y)| buf.get(*x, *y).unwrap().style.bg == app.theme.color("cursor.bg"))
            .count();
        assert!(
            highlighted >= 12,
            "the block is not highlighted ({highlighted} cells)"
        );
    }

    #[test]
    fn a_prompt_over_the_editor_keeps_the_text_behind_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"needle in here\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "F7"); // search

        let mut buf = Buffer::new(60, 10);
        let cursor = draw(&mut app, &mut buf);
        let out = buf.to_text();
        assert!(cursor.is_some(), "the prompt has no cursor");
        assert!(out.contains("Search"), "{out}");
        assert!(out.contains("needle in here"), "the editor is gone: {out}");
    }

    #[test]
    fn long_lines_scroll_sideways_instead_of_overflowing() {
        let dir = tempfile::tempdir().unwrap();
        let long = "abcdefghij".repeat(12); // 120 columns
        std::fs::write(dir.path().join("wide.txt"), format!("{long}\n")).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "End");

        let mut buf = Buffer::new(40, 8);
        let cursor = draw(&mut app, &mut buf);
        let out = buf.to_text();
        for line in out.lines() {
            assert!(
                unicode_width::UnicodeWidthStr::width(line) <= 40,
                "overflow: {line:?}"
            );
        }
        let (x, _) = cursor.expect("the cursor should be on screen");
        assert!(x < 40, "the cursor left the screen at {x}");
        assert!(out.contains("1:121"), "{out}");
    }

    #[test]
    fn every_theme_and_language_renders_the_editor() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"hello\n").unwrap();
        for id in ["commander", "black", "white", "matrix", "tron", "blade"] {
            for (tag, _) in scmd_i18n::LANGUAGES {
                let mut app = app_on_disk(dir.path());
                app.theme = Theme::builtin(id).unwrap();
                app.i18n = I18n::new(tag);
                press(&mut app, "Down");
                press(&mut app, "F4");
                let mut buf = Buffer::new(60, 10);
                draw(&mut app, &mut buf);
                let out = buf.to_text();
                assert!(out.contains("hello"), "theme {id} / {tag}: {out}");
                for line in out.lines() {
                    assert!(
                        unicode_width::UnicodeWidthStr::width(line) <= 60,
                        "theme {id} / {tag} overflowed: {line:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn the_site_list_shows_the_sites_and_what_is_connected() {
        let mut with_sites = app("commander", "en");
        let mut site = scmd_net::Site::parse_url("sftp://mb@jump.example:2222/srv").unwrap();
        site.id = "jump".into();
        site.group = Some("work".into());
        with_sites.sites = vec![site.clone()];
        with_sites.connected = vec![site.authority()];
        with_sites.handle_input(Input::Key("Alt+N".parse().unwrap()));

        let out = frame(&mut with_sites, 80, 24);
        assert!(out.contains("Connections"), "{out}");
        assert!(
            out.contains("[work]"),
            "the group heading is missing: {out}"
        );
        assert!(out.contains("jump.example:2222"), "{out}");
        assert!(out.contains("agent"), "the auth method is missing: {out}");
        assert!(out.contains('*'), "the connected marker is missing: {out}");
        assert!(out.contains("Esc"), "the hint is missing: {out}");

        // Nothing configured: the dialog says what to do instead of showing an empty box.
        let mut empty = app("commander", "en");
        empty.handle_input(Input::Key("Alt+N".parse().unwrap()));
        let out = frame(&mut empty, 80, 24);
        assert!(out.contains("No sites"), "{out}");
    }

    #[test]
    fn a_password_prompt_shows_stars_and_not_the_password() {
        let mut app = app("commander", "en");
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(scmd_net::Site::parse_url("sftp://mb@host:22/").unwrap()),
            outcome: crate::app::ConnectOutcome::NeedsSecret(scmd_net::SecretKind::Password),
        });
        for c in ["s", "e", "c", "r", "e", "t"] {
            app.handle_input(Input::Key(c.parse().unwrap()));
        }
        let mut buf = Buffer::new(70, 20);
        let cursor = draw(&mut app, &mut buf);
        let out = buf.to_text();
        assert!(cursor.is_some());
        assert!(out.contains("******"), "the stars are missing: {out}");
        assert!(
            !out.contains("secret"),
            "the password is on the screen: {out}"
        );
    }

    #[test]
    fn f4_on_a_binary_opens_the_hex_editor_and_draws_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("program.bin");
        // Something a text view would mangle: NULs and control bytes.
        let bytes: Vec<u8> = (0u8..64).collect();
        std::fs::write(&path, &bytes).unwrap();

        let mut app = app("commander", "en");
        app.panels[0].cwd = VPath::local(dir.path());
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("program.bin", EntryKind::File, 64),
            ]),
        });
        app.panels[0].cursor_to(1);
        app.handle_input(Input::Key("F4".parse().unwrap()));

        assert!(app.mode.hex().is_some(), "a binary should open as bytes");
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("program.bin"), "{out}");
        // The offsets, the bytes and the characters, as every dump has them.
        assert!(out.contains("00000000"), "{out}");
        assert!(out.contains("00 01 02 03"), "{out}");
        assert!(out.contains("00000010"), "{out}");
        assert!(out.contains("ovr"), "overwrite is the default: {out}");

        // Typing two hex digits changes a byte and the patch list says so.
        app.handle_input(Input::Key("4".parse().unwrap()));
        app.handle_input(Input::Key("1".parse().unwrap()));
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("41"), "{out}");
        assert!(out.contains("→"), "the patch list should be shown: {out}");

        // `Esc` on a modified buffer asks rather than throwing the edits away.
        app.handle_input(Input::Key("Esc".parse().unwrap()));
        assert!(
            matches!(app.mode, crate::app::Mode::Confirm(_)),
            "{:?}",
            app.mode
        );
    }

    /// The block keys, end to end: mark some bytes, fill them, exclusive-or them, write them
    /// out, and take them away — each through the menu a user would walk.
    #[test]
    fn a_block_of_bytes_is_marked_and_operated_on() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("program.bin");
        std::fs::write(&path, (0u8..32).collect::<Vec<u8>>()).unwrap();

        let mut app = app("commander", "en");
        // A name typed into a dialog is relative to the panel, so the panel is where the
        // file is.
        app.panels[0].cwd = VPath::local(dir.path());
        app.open_hex(&path);
        let key = |app: &mut App, k: &str| app.handle_input(Input::Key(k.parse().unwrap()));

        // Mark four bytes from the start.
        key(&mut app, "F3");
        for _ in 0..4 {
            key(&mut app, "Right");
        }
        key(&mut app, "F3");
        assert_eq!(app.mode.hex().unwrap().block, Some((0, 4)));

        // `F12` offers what can be done with them, sized.
        key(&mut app, "F12");
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("Block"), "{out}");
        assert!(out.contains("Fill"), "{out}");
        assert!(out.contains("Delete"), "{out}");

        // Fill them with one byte, which repeats over the block.
        while !matches!(&app.mode, crate::app::Mode::Pick(p)
            if p.rows[p.cursor].value == "fill")
        {
            key(&mut app, "Down");
        }
        key(&mut app, "Enter");
        for c in "ff".chars() {
            key(&mut app, &c.to_string());
        }
        key(&mut app, "Enter");
        let editor = app.mode.hex_mut().expect("still editing");
        assert_eq!(editor.read(0, 6), vec![0xff, 0xff, 0xff, 0xff, 0x04, 0x05]);
        assert_eq!(editor.len(), 32, "a fill never changes the length");

        // Exclusive-or the same block back to what it was.
        key(&mut app, "F12");
        while !matches!(&app.mode, crate::app::Mode::Pick(p)
            if p.rows[p.cursor].value == "xor")
        {
            key(&mut app, "Down");
        }
        key(&mut app, "Enter");
        for c in "ff".chars() {
            key(&mut app, &c.to_string());
        }
        key(&mut app, "Enter");
        assert_eq!(app.mode.hex_mut().unwrap().read(0, 4), vec![0, 0, 0, 0]);
        assert!(app.status.contains("not encryption"), "{}", app.status);

        // Write the block out, and then delete it: the length changes and says so.
        key(&mut app, "F12");
        while !matches!(&app.mode, crate::app::Mode::Pick(p)
            if p.rows[p.cursor].value == "write")
        {
            key(&mut app, "Down");
        }
        key(&mut app, "Enter");
        for c in "block.bin".chars() {
            key(&mut app, &c.to_string());
        }
        key(&mut app, "Enter");
        let written = dir.path().join("block.bin");
        assert_eq!(std::fs::read(&written).unwrap(), vec![0, 0, 0, 0]);

        key(&mut app, "F12");
        while !matches!(&app.mode, crate::app::Mode::Pick(p)
            if p.rows[p.cursor].value == "delete")
        {
            key(&mut app, "Down");
        }
        key(&mut app, "Enter");
        assert_eq!(app.mode.hex().unwrap().len(), 28);
        assert!(app.mode.hex().unwrap().block.is_none());

        // And the file on disk is untouched until it is saved.
        assert_eq!(std::fs::read(&path).unwrap().len(), 32);
    }

    #[test]
    fn ctrl_q_previews_the_file_under_the_cursor_in_the_other_panel() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("notes.txt"),
            b"the first line\nthe second line\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("other.txt"), b"something else entirely\n").unwrap();
        let mut app = app("commander", "en");
        app.panels[0].cwd = VPath::local(dir.path());
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("notes.txt", EntryKind::File, 30),
                entry("other.txt", EntryKind::File, 24),
            ]),
        });
        app.panels[0].cursor_to(1);
        app.active = 0;

        app.handle_input(Input::Key("Ctrl+Q".parse().unwrap()));
        // The preview goes in the passive panel and the keys stay with the files.
        assert_eq!(app.panels[1].content, crate::panel::Content::QuickView);
        assert_eq!(app.active, 0);

        let out = frame(&mut app, 100, 14);
        assert!(out.contains("Quick view"), "{out}");
        assert!(out.contains("the first line"), "{out}");
        assert!(out.contains("notes.txt"), "{out}");

        // The cursor moves, the preview follows.
        app.handle_input(Input::Key("Down".parse().unwrap()));
        let out = frame(&mut app, 100, 14);
        assert!(out.contains("something else entirely"), "{out}");
        assert!(!out.contains("the first line"), "{out}");

        // A directory says what it is rather than showing an empty frame.
        app.handle_input(Input::Key("Home".parse().unwrap()));
        let out = frame(&mut app, 100, 14);
        assert!(out.contains("is a directory"), "{out}");

        // And the key puts the files back.
        app.handle_input(Input::Key("Ctrl+Q".parse().unwrap()));
        assert_eq!(app.panels[1].content, crate::panel::Content::Files);
    }

    #[test]
    fn ctrl_l_tells_the_other_panel_what_it_is_looking_at() {
        let mut app = app("commander", "en");
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir, 0),
                entry("a.txt", EntryKind::File, 1024),
            ]),
        });
        app.handle_event(Event::FreeSpace {
            panel: 0,
            bytes: Some(42 * 1024 * 1024),
        });
        app.panels[0].cursor_to(2);
        app.active = 0;

        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
        assert_eq!(app.panels[1].content, crate::panel::Content::Info);
        let out = frame(&mut app, 100, 16);
        assert!(out.contains("Info"), "{out}");
        assert!(out.contains("/home/mb/project"), "{out}");
        assert!(out.contains("a directory on this machine"), "{out}");
        // The device, the directory's own totals, and the row under the cursor.
        assert!(out.contains("42"), "{out}");
        assert!(out.contains("1 files, 1 directories"), "{out}");
        assert!(out.contains("a.txt"), "{out}");

        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
        assert_eq!(app.panels[1].content, crate::panel::Content::Files);
    }

    #[test]
    fn a_remote_panel_shows_the_url_in_its_header() {
        let mut app = app("commander", "en");
        let site = scmd_net::Site::parse_url("sftp://mb@host.example:22/srv/www").unwrap();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site.clone()),
            outcome: crate::app::ConnectOutcome::Open {
                start: site.start(),
                connected: vec![site.authority()],
            },
        });
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                entry("index.html", EntryKind::File, 42),
            ]),
        });
        let out = frame(&mut app, 100, 12);
        assert!(out.contains("sftp://mb@host.example:22/srv/www"), "{out}");
        assert!(out.contains("index.html"), "{out}");
    }
}
