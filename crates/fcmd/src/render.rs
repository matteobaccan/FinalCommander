use fcmd_ui::{widgets, Buffer, Rect, Style};

use crate::app::{App, Mode, PromptKind};
use crate::hit::{Layout, ListArea, PanelArea};
use crate::panel as fcmd_panel;

/// Draw a whole frame. Returns where the text cursor should sit, if anywhere.
pub fn draw(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    // Where things end up is recorded as they are drawn, for the mouse to read afterwards.
    // Cleared first, so that what is not on the screen this frame cannot be clicked.
    app.layout = Layout::default();
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
                Mode::Form(form) => {
                    let rows = form.rows();
                    let tabs = form.tabs();
                    let area = widgets::draw_form(
                        buf,
                        &app.theme,
                        screen,
                        &widgets::FormView {
                            title: &form.title,
                            rows: &rows,
                            selected: form.cursor_row(),
                            buttons: &form.buttons,
                            button: form.button,
                            tabs: &tabs,
                            tab: form.page,
                            over: None,
                        },
                    );
                    return form.caret().map(|(row, col)| {
                        (
                            area.x + 2 + col as u16,
                            // A dialog with pages has its row of pages under the title.
                            area.y + 1 + u16::from(!form.pages.is_empty()) + row as u16,
                        )
                    });
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
                Mode::Form(form) => {
                    let rows = form.rows();
                    let tabs = form.tabs();
                    let area = widgets::draw_form(
                        buf,
                        &app.theme,
                        screen,
                        &widgets::FormView {
                            title: &form.title,
                            rows: &rows,
                            selected: form.cursor_row(),
                            buttons: &form.buttons,
                            button: form.button,
                            tabs: &tabs,
                            tab: form.page,
                            over: None,
                        },
                    );
                    return form.caret().map(|(row, col)| {
                        (
                            area.x + 2 + col as u16,
                            // A dialog with pages has its row of pages under the title.
                            area.y + 1 + u16::from(!form.pages.is_empty()) + row as u16,
                        )
                    });
                }
                _ => {}
            }
        }
        return None;
    }

    if matches!(app.mode, Mode::Term(_)) {
        return draw_terminal(app, buf);
    }

    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let status_h = u16::from(app.cfg.ui.status_bar);
    let cmd_h = u16::from(app.cfg.ui.command_line);
    // FAR's *Always show menu bar*: the top row belongs to the menu titles rather than to
    // the panels' own frame, which costs a row of files and saves pressing `F9` to see what
    // the menus are called. Off by default, as it is in FAR.
    let menu_h = u16::from(app.cfg.ui.menu_bar);
    let chrome = keybar_h + status_h + cmd_h + menu_h;
    let full_h = h.saturating_sub(chrome).max(3);
    // `Ctrl+Up`/`Ctrl+Down` take rows off the panels and give them to the user screen, which
    // is how FAR lets you watch what a command printed without leaving the panels. Six rows
    // is the least a panel is worth drawing at: a frame, a titles row, the foot and a file.
    let panels_h = if app.panels_off {
        // `Ctrl+O`: no panels at all, and the whole screen above the command line is what the
        // commands printed. The rest of the program is exactly where it was.
        0
    } else {
        full_h
            .saturating_sub(app.cfg.panel.height_cut)
            .max(6)
            .min(full_h)
    };

    // `Ctrl+F1`/`Ctrl+F2` hide one panel and give the screen to the other: a zero-width area
    // draws nothing, which is all "panel off" has to mean.
    let areas = match app.hidden {
        Some(0) => [
            Rect::new(0, menu_h, 0, panels_h),
            Rect::new(0, menu_h, w, panels_h),
        ],
        Some(_) => [
            Rect::new(0, menu_h, w, panels_h),
            Rect::new(w, menu_h, 0, panels_h),
        ],
        None => {
            // The split is a percentage, so it survives a resize of the terminal; a panel
            // narrower than ten columns is not a panel, so neither side can be squeezed out.
            let left_w = ((w as u32 * app.cfg.panel.split as u32) / 100) as u16;
            let left_w = left_w.clamp(10.min(w), w.saturating_sub(10));
            [
                Rect::new(0, menu_h, left_w, panels_h),
                Rect::new(left_w, menu_h, w - left_w, panels_h),
            ]
        }
    };

    // A side with more than one tab gives its top row to the tab bar (SPEC 02 §1.6). A bar
    // that was always there would cost every panel a row of files to say that it has one
    // tab, so one tab draws nothing and the screen is what it always was.
    let mut areas = areas;
    let mut bars = [None, None];
    for i in 0..2 {
        if app.tabs[i].many() && areas[i].w > 0 && areas[i].h > 6 {
            bars[i] = Some(Rect::new(areas[i].x, areas[i].y, areas[i].w, 1));
            areas[i] = Rect::new(areas[i].x, areas[i].y + 1, areas[i].w, areas[i].h - 1);
        }
    }
    for (i, bar) in bars.iter().enumerate() {
        let Some(bar) = bar else { continue };
        let labels = app.tabs[i].labels(&app.panels[i]);
        let locked: Vec<bool> = (0..app.tabs[i].len())
            .filter_map(|n| app.tabs[i].panel(n, &app.panels[i]))
            .map(|p| p.locked)
            .collect();
        widgets::draw_tabs(
            buf,
            &app.theme,
            &widgets::TabRow {
                area: *bar,
                labels: &labels,
                current: app.tabs[i].current(),
                locked: &locked,
                focused: i == app.active,
            },
        );
    }

    // Remember the geometry so key handling knows how far a page is. It is the active
    // panel's, because that is the one the keys move through.
    app.rows = if app.panels_off {
        // There is no panel to page through; a page is the screen the output is on.
        full_h as usize
    } else {
        widgets::panel_rows(
            areas[app.active],
            app.cfg.panel.column_titles,
            app.cfg.panel.status_line,
        )
    };
    app.columns = widgets::brief_columns(
        areas[app.active],
        app.panels[app.active].view,
        app.panels[app.active].listing.widest,
    );
    for i in 0..2 {
        let (rows, cols) = (app.rows, app.columns);
        app.panels[i].scroll_into_view(rows, cols);
    }

    let up_label = app.i18n.t("list-up");
    let folder_label = app.i18n.t("list-folder");
    // The column headings, in the language the program is running in. Gathered once per
    // frame rather than per panel: both sides use the same words.
    let titles = [
        "col-name",
        "col-ext",
        "col-size",
        "col-allocated",
        "col-date-time",
        "col-modified",
        "col-created",
        "col-accessed",
        "col-attr",
        "col-owner",
        "col-group",
        "col-descr",
        "col-links",
    ]
    .map(|id| app.i18n.t(id));
    let titles = widgets::ColumnTitles {
        name: &titles[0],
        ext: &titles[1],
        size: &titles[2],
        allocated: &titles[3],
        date_time: &titles[4],
        modified: &titles[5],
        created: &titles[6],
        accessed: &titles[7],
        attr: &titles[8],
        owner: &titles[9],
        group: &titles[10],
        descr: &titles[11],
        links: &titles[12],
    };
    for (i, area) in areas.iter().enumerate() {
        // The special kinds have no rows, so they draw themselves inside the same frame and
        // the row machinery below never sees them (SPEC 02 §7).
        match app.panels[i].content {
            fcmd_panel::Content::QuickView => {
                draw_quick_view(app, i, *area, buf);
                continue;
            }
            fcmd_panel::Content::Info => {
                draw_info(app, i, *area, buf);
                continue;
            }
            fcmd_panel::Content::Tree => {
                draw_tree(app, i, *area, buf);
                continue;
            }
            fcmd_panel::Content::Files => {}
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
            cluster: panel.disk.map(|d| d.cluster).unwrap_or(0),
            titles,
            free: if app.cfg.panel.free_space {
                panel.disk.map(|d| d.free)
            } else {
                None
            },
            scrollbar: app.cfg.panel.scrollbar,
            name_scroll: panel.name_scroll,
            highlight: &app.highlight,
            owners: &panel.owners,
            descriptions: &panel.descriptions,
            links: &panel.links,
            // The user's own columns for this view, if they wrote a set on its digit
            // (SPEC 02 §3a); without one the built-in mode draws, as it always has.
            columns: crate::colset::for_slot(&app.column_sets, panel.view.slot()),
            up_label: &up_label,
            folder_label: &folder_label,
        };
        widgets::draw_panel(buf, &app.theme, &render);

        // Where its rows landed, so a click can be turned back into an entry. The same
        // arithmetic as `draw_panel`'s own, which is why both read it from `panel_rows`.
        let rows = widgets::panel_rows(*area, app.cfg.panel.column_titles, status.is_some());
        let mut inner = area.inner();
        if app.cfg.panel.column_titles {
            inner = Rect::new(inner.x, inner.y + 1, inner.w, inner.h.saturating_sub(1));
        }
        if rows > 0 && !inner.is_empty() {
            app.layout.panels[i] = Some(PanelArea {
                area: Rect::new(inner.x, inner.y, inner.w, rows as u16),
                top: panel.top,
                rows,
                columns: widgets::brief_columns(*area, panel.view, panel.listing.widest),
                len: panel.listing.len(),
            });
        }
    }

    // What the panels gave up: the last lines of the user screen, which is the point of
    // giving them up at all.
    if panels_h < full_h {
        let area = Rect::new(0, menu_h + panels_h, w, full_h - panels_h);
        // The terminal's own colours, not the theme's: what is shown here came out of the
        // shell, and it looked like that when it was printed.
        let style = Style::new(fcmd_ui::Color::Default, fcmd_ui::Color::Default);
        buf.fill(area, ' ', style);
        let rows = area.h as usize;
        // Scrolled back by whatever `PgUp` has asked for, and never past the first line.
        let end = app
            .user_screen
            .len()
            .saturating_sub(app.user_scroll)
            .max(rows.min(app.user_screen.len()));
        let start = end.saturating_sub(rows);
        for (i, line) in app.user_screen[start..end].iter().enumerate() {
            let text = fcmd_ui::truncate_cols(line, w as usize);
            buf.put_str(0, area.y + i as u16, &text, style, w);
        }
    }

    // The command line: the panel's path as the prompt, then what is being typed.
    let mut cursor = None;
    if cmd_h == 1 {
        let y = menu_h + full_h;
        let style = Style::new(app.theme.color("cmdline.fg"), app.theme.color("cmdline.bg"));
        buf.fill(Rect::new(0, y, w, 1), ' ', style);
        let prompt = app.prompt();
        let prompt = fcmd_ui::elide_middle(&prompt, (w / 2) as usize);
        let prompt_w = unicode_width::UnicodeWidthStr::width(prompt.as_str()) as u16;
        buf.put_str(0, y, &prompt, style, prompt_w);
        let room = w.saturating_sub(prompt_w);
        let text = fcmd_ui::truncate_cols(&app.cmd.text, room as usize);
        buf.put_str(prompt_w, y, &text, style, room);
        // What is marked out, drawn the way a selected row is: the colours swapped. Marking
        // text and not seeing it marked is the one thing that would make the keys useless.
        if let Some((from, to)) = app.cmd.selection() {
            let width = |upto: usize| {
                app.cmd
                    .text
                    .chars()
                    .take(upto)
                    .filter_map(unicode_width::UnicodeWidthChar::width)
                    .sum::<usize>() as u16
            };
            let (x0, x1) = (width(from), width(to));
            let marked: String = app.cmd.text.chars().take(to).skip(from).collect();
            let ink = Style::new(app.theme.color("cmdline.bg"), app.theme.color("cmdline.fg"));
            if x0 < room {
                buf.put_str(
                    prompt_w + x0,
                    y,
                    &fcmd_ui::truncate_cols(&marked, (room - x0) as usize),
                    ink,
                    (x1 - x0).min(room - x0),
                );
            }
        }
        let caret = app
            .cmd
            .text
            .chars()
            .take(app.cmd.caret)
            .filter_map(unicode_width::UnicodeWidthChar::width)
            .sum::<usize>() as u16;
        // What the best answer would add, after the caret and dimmed: it is not in the line,
        // so `Enter` runs what was typed and `→` is what takes it (SPEC 06 §7.4).
        if let Some(hint) = app.completion_hint() {
            let dim = Style::new(
                app.theme.color("cmdline.hint.fg"),
                app.theme.color("cmdline.bg"),
            );
            let left = room.saturating_sub(caret);
            if left > 0 {
                buf.put_str(
                    prompt_w + caret,
                    y,
                    &fcmd_ui::truncate_cols(hint, left as usize),
                    dim,
                    left,
                );
            }
        }
        cursor = Some((prompt_w + caret.min(room.saturating_sub(1)), y));
        app.layout.command_line = Some(y);

        // The completion list, above the line rather than below it: below is the key bar and
        // the status line, and a list that covered those would hide the one row that says
        // what the keys do (SPEC 06 §7.4). It is drawn over the bottom of the panels, which
        // is where a list belongs while one is typing rather than looking at them.
        if let Some(c) = &app.complete {
            let rows = (app.cfg.cmdline.complete.rows.max(1) as usize).min(c.rows.len());
            let rows = rows.min(y.saturating_sub(1) as usize);
            if rows > 0 {
                // The widest row, so the box is as wide as it has to be and no wider; it
                // starts under the caret's own word, as a completion list does everywhere.
                let widest = c
                    .rows
                    .iter()
                    .map(|r| unicode_width::UnicodeWidthStr::width(r.text.as_str()))
                    .max()
                    .unwrap_or(0)
                    .clamp(8, (w.saturating_sub(4)) as usize) as u16;
                let box_w = widest + 2;
                let x = prompt_w.min(w.saturating_sub(box_w));
                let top = y - rows as u16;
                let area = Rect::new(x, top, box_w, rows as u16);
                let style = Style::new(app.theme.color("dialog.fg"), app.theme.color("dialog.bg"));
                buf.fill(area, ' ', style);
                // The window onto the list, so the cursor is always on a row that is drawn.
                let first = c.cursor.saturating_sub(rows.saturating_sub(1));
                for (i, row) in c.rows.iter().skip(first).take(rows).enumerate() {
                    let here = first + i == c.cursor;
                    let style = if here {
                        Style::new(
                            app.theme.color("dialog.button.selected.fg"),
                            app.theme.color("dialog.button.selected.bg"),
                        )
                    } else {
                        style
                    };
                    let text = fcmd_ui::truncate_cols(&row.text, (box_w - 2) as usize);
                    buf.fill(Rect::new(area.x, area.y + i as u16, area.w, 1), ' ', style);
                    buf.put_str(area.x + 1, area.y + i as u16, &text, style, box_w - 2);
                }
            }
        }
    }

    if status_h == 1 {
        let text = if app.status.is_empty() {
            status_hint(app)
        } else {
            app.status.clone()
        };
        let row = Rect::new(0, menu_h + full_h + cmd_h, w, 1);
        widgets::draw_status(buf, &app.theme, row, &text, app.status_error);
        // The corner: whether a macro is recording, and how many screens are behind the
        // panels. The recording light comes first because it is the one that matters while it
        // is on — every key is going into a file.
        let mut mark = String::new();
        if let Some(steps) = &app.recording {
            mark.push_str(&format!("REC {} ", steps.len()));
        }
        if !app.background.is_empty() {
            mark.push_str(&format!("[{}]", app.background.len()));
        }
        {
            let mark = mark.trim_end().to_string();
            let width = unicode_width::UnicodeWidthStr::width(mark.as_str()) as u16;
            if width > 0 && width < w {
                // While recording, the corner is drawn as an error is: it has to be noticed.
                let mut style =
                    Style::new(app.theme.color("status.fg"), app.theme.color("status.bg"));
                if app.recording.is_some() {
                    style.fg = app.theme.color("error.fg");
                    style.bold = true;
                }
                buf.put_str(w - width, row.y, &mark, style, width);
            }
        }
    }

    if keybar_h == 1 {
        // A dialog's own row, where one is open: its keys are not the panels' keys, and FAR
        // changes the bar for the same reason.
        let labels = app
            .keybar_dialog(app.held)
            .unwrap_or_else(|| app.keybar_for(app.held));
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
        app.layout.key_bar = Some((h - 1, labels.iter().map(|(n, _)| *n).collect()));
    }

    // The clock, in the top right corner where FAR puts it: over the panel frame, or at the
    // end of the menu bar when that is always on. Four characters and a space either side,
    // which is the whole of what a clock in a file manager has to be.
    if app.cfg.ui.clock && w > 10 {
        let now = format!(" {} ", fcmd_ui::clock_now());
        let width = unicode_width::UnicodeWidthStr::width(now.as_str()) as u16;
        let style = if app.cfg.ui.menu_bar {
            Style::new(app.theme.color("hmenu.fg"), app.theme.color("hmenu.bg"))
        } else {
            Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"))
        };
        buf.put_str(w - width, 0, &now, style, width);
    }

    // The bar itself, when it is always on and no menu is open: the open one draws it with
    // its own title lit, below.
    if menu_h == 1 && !matches!(app.mode, Mode::Menu { .. }) {
        let titles: Vec<String> = crate::menu::menus()
            .iter()
            .map(|m| app.i18n.t(m.title))
            .collect();
        let starts = widgets::draw_menu_bar(
            buf,
            &app.theme,
            Rect::new(0, 0, w, 1),
            &titles,
            // No title is lit: the bar is showing what there is, not where one is.
            usize::MAX,
        );
        app.layout.menu_bar = Some((0, starts));
    }

    let screen = Rect::new(0, menu_h, w, panels_h);
    if app.mode.is_normal() {
        return cursor;
    }
    if let Some(view) = app.mode.job() {
        draw_job(app, view, buf, screen);
        return None;
    }
    if let Some(view) = app.mode.diff() {
        draw_diff(app, view, buf, screen);
        return None;
    }
    // The grabber draws the screen it was opened on, not the program: the whole point is
    // that what gets copied is what was there (SPEC 06 §11b).
    if let Mode::Grab(grab) = &app.mode {
        return draw_grab(app, grab, buf);
    }
    match &app.mode {
        Mode::Normal => None,
        Mode::Find(view) => {
            draw_find(app, view, buf, screen);
            None
        }
        Mode::Sync(view) => {
            draw_sync(app, view, buf, screen);
            None
        }
        Mode::Palette(view) => {
            draw_palette(app, view, buf, screen);
            None
        }
        Mode::Prompt(prompt) => {
            // A password is never put on the screen, not even for a frame.
            let shown = if prompt.secret {
                "*".repeat(prompt.text.chars().count())
            } else {
                prompt.text.clone()
            };
            // The quick search belongs to a panel, not to the screen, and FAR draws it in
            // that panel's own bottom frame: a box in the middle of the screen covers the
            // rows one is reading, and the whole point of the search is to read them
            // (SPEC 02 §6). Everything else is a question about the program and is asked
            // where a dialog is asked.
            let at = matches!(prompt.kind, PromptKind::QuickSearch { .. })
                .then(|| quick_search_box(areas[app.active]))
                .flatten();
            let (_, cx, cy) = match at {
                Some(area) => widgets::draw_prompt_at(
                    buf,
                    &app.theme,
                    area,
                    &prompt.title,
                    &shown,
                    shown.chars().count(),
                ),
                None => widgets::draw_prompt(
                    buf,
                    &app.theme,
                    screen,
                    &prompt.title,
                    &shown,
                    shown.chars().count(),
                ),
            };
            Some((cx, cy))
        }
        Mode::Confirm(confirm) => {
            widgets::draw_choice(
                buf,
                &app.theme,
                screen,
                &confirm.title,
                &confirm.lines,
                &confirm.buttons(&app.i18n),
                confirm.button,
            );
            None
        }
        Mode::Job(_) => None,    // handled above, through `Mode::job()`
        Mode::Diff(_) => None,   // handled above, through `Mode::diff()`
        Mode::Grab(_) => None,   // handled above, on the frame it was opened with
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
        Mode::Menu { menu, item, open } => {
            let (bar, list) = draw_menus(
                app,
                buf,
                Rect::new(0, 0, w, menu_h + panels_h),
                *menu,
                *item,
                *open,
            );
            app.layout.menu_bar = Some((0, bar));
            app.layout.list = list;
            None
        }
        Mode::Form(form) => {
            let rows = form.rows();
            let tabs = form.tabs();
            let buttons = form.buttons.clone();
            let moved = (app.form_offset != (0, 0)).then(|| {
                let (dx, dy) = app.form_offset;
                Rect::new(
                    screen.x.saturating_add_signed(dx),
                    screen.y.saturating_add_signed(dy),
                    screen.w,
                    screen.h,
                )
            });
            let area = widgets::draw_form(
                buf,
                &app.theme,
                screen,
                &widgets::FormView {
                    title: &form.title,
                    rows: &rows,
                    selected: form.cursor_row(),
                    buttons: &buttons,
                    button: form.button,
                    tabs: &tabs,
                    tab: form.page,
                    // Where `Ctrl+F5` has pushed it. The dialog is centred on what it is
                    // *about*, so moving it is moving that rectangle; `draw_form` clamps the
                    // result to the screen, which is why the offset needs no limit of its own.
                    over: moved,
                },
            );
            // A text row shows a real caret; everything else leaves the cursor hidden.
            form.caret().map(|(row, col)| {
                (
                    area.x + 2 + col as u16,
                    // A dialog with pages has its row of pages under the title.
                    area.y + 1 + u16::from(!form.pages.is_empty()) + row as u16,
                )
            })
        }
        Mode::Pick(pick) => {
            let list = draw_pick(app, buf, screen, pick);
            app.layout.list = list;
            None
        }
        // Waiting for the key a just-recorded macro goes on: a message, with the step count.
        Mode::MacroKey { steps } => {
            let title = app.i18n.t("macro-key-title");
            let lines = vec![
                app.i18n
                    .t_args("macro-key-body", &[("steps", (steps.len() as i64).into())]),
                String::new(),
                steps
                    .iter()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>()
                    .join("  "),
            ];
            widgets::draw_dialog(buf, &app.theme, screen, &title, &lines, None);
            None
        }
        Mode::UserScreen { .. } => None, // drawn full screen, before the panels
        // Drawn full screen too, and before this: `draw` returns early for it.
        Mode::Term(_) => None,
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

/// `Ctrl+Alt+E`: a real terminal, with a program drawing in it.
///
/// Drawing it is a copy. The program's output went through the VT parser into a grid of the
/// same type the panels draw into — a character, a foreground, a background, bold and reverse
/// — so there is nothing to translate here: the cells are put on the screen as they are, and
/// the colours a program asked for are the ones the backend already knows how to send.
///
/// The chrome is one row at the top, which says what is running and how to get out. The key
/// bar is deliberately *not* drawn: every function key belongs to the program in here, and a
/// bar that offered ours would be telling the reader something untrue.
/// Where the quick search's box goes: standing on the panel's own bottom frame, two columns
/// in from its left edge (SPEC 02 §6).
///
/// It covers the last rows of the list rather than the middle of it, which is where FAR puts
/// it and why: the rows one has already walked past are the ones one can afford to lose
/// sight of. `None` when the panel is too small to hold it — a panel ten columns wide is not
/// somewhere to draw a box — and the box is then asked for in the middle of the screen,
/// which is at least somewhere it fits.
fn quick_search_box(panel: Rect) -> Option<Rect> {
    const MIN_W: u16 = 24;
    if panel.w < MIN_W + 4 || panel.h < 6 {
        return None;
    }
    let w = (panel.w / 2).max(MIN_W).min(panel.w.saturating_sub(4));
    Some(Rect::new(panel.x + 2, panel.y + panel.h - 3, w, 3))
}

fn draw_terminal(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    let header = Rect::new(0, 0, w, 1);
    let body = Rect::new(0, 1, w, h.saturating_sub(1));
    app.term_area = (body.w, body.h);

    let Mode::Term(term) = &mut app.mode else {
        return None;
    };
    // The grid may be a size behind the window: it is resized when the program is told, and
    // the program is told from the event loop rather than from here.
    let (title, finished, cursor) = term.with_screen(|screen| {
        let title = screen.title.clone();
        screen.dirty = false;
        (title, false, screen.cursor())
    });
    let finished = finished || term.finished();
    let name = term.name.clone();

    // The cells, as they are.
    term.with_screen(|screen| {
        let grid = screen.grid();
        for y in 0..body.h.min(grid.height()) {
            for x in 0..body.w.min(grid.width()) {
                if let Some(cell) = grid.get(x, y) {
                    if cell.wide_tail {
                        continue;
                    }
                    buf.set(body.x + x, body.y + y, cell.ch, cell.style);
                }
            }
        }
    });

    let style = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("status.bg"),
    );
    buf.fill(header, ' ', style);
    let what = title.unwrap_or(name);
    let hint = app
        .i18n
        .t(if finished { "term-ended" } else { "term-hint" });
    let hint_w = hint.chars().count().min(w as usize) as u16;
    buf.put_str(0, 0, &what, style, w.saturating_sub(hint_w + 2));
    buf.put_str(w - hint_w, 0, &hint, style, hint_w);

    // The program's own cursor, which is the one a reader is typing at.
    cursor
        .filter(|_| !finished)
        .map(|(x, y)| (body.x + x.min(body.w - 1), body.y + y.min(body.h - 1)))
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
        let text = fcmd_ui::truncate_cols(line, w as usize);
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
fn draw_structure(app: &App, viewer: &mut fcmd_view::Viewer, buf: &mut Buffer, area: Rect) {
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
            (_, _, fcmd_view::structure::RowKind::Group) => group,
            (_, _, fcmd_view::structure::RowKind::Field) => base,
            _ => base,
        };
        if on {
            buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);
        }
        match row.kind {
            fcmd_view::structure::RowKind::Group => {
                let text = fcmd_ui::truncate_cols(&row.label, inner.w.saturating_sub(2) as usize);
                buf.put_str(inner.x + 1, y, &text, style, inner.w.saturating_sub(1));
            }
            _ => {
                // label on the left, value on the right, raw bytes in between when they fit.
                let label_w = (inner.w / 2).max(8).min(inner.w.saturating_sub(4));
                let label = fcmd_ui::truncate_cols(&row.label, label_w as usize);
                buf.put_str(inner.x + 1, y, &label, style, label_w);
                let right_x = inner.x + 1 + label_w;
                let right_w = inner.right().saturating_sub(right_x);
                let value = if row.raw.is_empty() {
                    row.value.clone()
                } else {
                    format!("{} {}", row.raw, row.value)
                };
                let value = fcmd_ui::truncate_cols(&value, right_w as usize);
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
    let hint = fcmd_ui::truncate_cols(&hint, area.w.saturating_sub(3) as usize);
    buf.put_str(area.x + 2, area.bottom() - 1, &hint, dim, area.w - 3);
}

/// The menu bar across the top, with the menu under the picked title pulled down once it has
/// been opened (SPEC 10 §2). Returns where each title starts on the bar, and where the open
/// menu's rows landed.
fn draw_menus(
    app: &App,
    buf: &mut Buffer,
    screen: Rect,
    menu: usize,
    item: usize,
    dropped: bool,
) -> (Vec<u16>, Option<ListArea>) {
    let menus = crate::menu::menus();
    let titles: Vec<String> = menus.iter().map(|m| app.i18n.t(m.title)).collect();
    let starts =
        widgets::draw_menu_bar(buf, &app.theme, Rect::new(0, 0, screen.w, 1), &titles, menu);
    // The bar on its own is a state of its own: `F9` lights it and nothing covers the panels
    // until a menu is actually opened.
    if !dropped {
        return (starts, None);
    }
    let Some(open) = menus.get(menu) else {
        return (starts, None);
    };
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
    let area = widgets::draw_menu(buf, &app.theme, below, at, &rows, item);
    let shown = area.h.saturating_sub(2);
    let list = (shown > 0).then(|| ListArea {
        area: Rect::new(area.x + 1, area.y + 1, area.w.saturating_sub(2), shown),
        // A dropdown is not windowed: it is as tall as it needs to be, so the first row
        // shown is the first item.
        top: 0,
        len: rows.len(),
    });
    (starts, list)
}

/// A choose-one list: languages, themes, drives, folder history, sort modes (SPEC 10 §4).
/// Returns where the rows landed, so that a click can pick one.
fn draw_pick(
    app: &App,
    buf: &mut Buffer,
    screen: Rect,
    pick: &crate::form::Pick,
) -> Option<ListArea> {
    let mut lines = pick.lines();
    // Scrolled sideways (`Alt+←/→`), for a list of paths wider than the box: the rows slide
    // and the window labels below do not, because they are the renderer's own words rather
    // than part of the list.
    if pick.left > 0 {
        for line in &mut lines {
            *line = line.chars().skip(pick.left).collect();
        }
    }
    if lines.is_empty() {
        lines.push(app.i18n.t("pick-empty"));
    }
    // The drive dialog has keys of its own, so it says so rather than repeating the general
    // line — which is FAR's habit: every dialog's footer is about that dialog.
    let hint = app.i18n.t(match pick.kind {
        crate::menu::PickKind::Drives => "drives-hint",
        crate::menu::PickKind::Bookmarks => "bookmarks-hint",
        crate::menu::PickKind::Highlight => "hl-hint",
        // A history has three keys of its own, and a footer that does not name them is a
        // footer that keeps them secret.
        crate::menu::PickKind::CommandHistory
        | crate::menu::PickKind::FileHistory
        | crate::menu::PickKind::FoldersHistory => "history-hint",
        // The code pages have keys of their own, and a footer that names them.
        crate::menu::PickKind::Codepages => "cp-hint",
        // So does the settings browser, where three of the four keys are its own.
        crate::menu::PickKind::Settings => "opt-settings-hint",
        _ => "pick-hint",
    });

    // A list longer than the screen is windowed, not clipped. The folder and command
    // histories are the long ones in practice — and the plugin commands now — and before
    // this the rows past the bottom were drawn nowhere and the cursor went with them.
    //
    // The window is a third of a screen of room on each side of the cursor where there is
    // list to show, the same rule the panel scrolls by.
    let room = screen.h.saturating_sub(3).max(1) as usize; // frame, hint, and the row itself
    let (start, more_above, more_below) = if lines.len() <= room {
        (0, false, false)
    } else {
        // At least one row of margin, so the cursor is never the row a label will take.
        let margin = (room / 3).max(1);
        let start = pick
            .cursor
            .saturating_sub(margin)
            .min(lines.len().saturating_sub(room));
        (start, start > 0, start + room < lines.len())
    };
    let mut shown: Vec<String> = lines[start..(start + room).min(lines.len())].to_vec();
    // Say that there is more, in the row that would otherwise be the first or last one: a
    // list that silently has more in it is a list one does not scroll. The label takes a row
    // and so hides the entry that was in it, which the count has to include — `start` rows
    // are above the window and the first one of it is now the label, so `start + 1` are out
    // of sight.
    if more_above {
        if let Some(first) = shown.first_mut() {
            *first = format!("↑ {} more above", start + 1);
        }
    }
    if more_below {
        if let Some(last) = shown.last_mut() {
            *last = format!("↓ {} more below", lines.len() - (start + room - 1));
        }
    }

    // The drive dialog belongs to its panel and opens over it, five columns in from its
    // frame, which is where FAR puts it: a list that will send the *left* panel somewhere
    // and opens in the middle of the screen is a list one has to read the title of. The
    // other lists are about the program and stay centred.
    let over = (pick.kind == crate::menu::PickKind::Drives)
        .then(|| app.layout.panels[pick.side].map(|p| p.area))
        .flatten()
        .map(|area| Rect::new(area.x.saturating_sub(1), area.y, area.w, area.h));
    // A list one picks from is a menu, and FAR draws it in the menu's colours — cyan, with
    // the key in yellow and the row under the cursor on black. That is why its drive dialog
    // does not look like its settings dialogs: one is a list to choose from, the other a
    // form to fill in.
    let area = widgets::draw_menu_over(
        buf,
        &app.theme,
        screen,
        over,
        // A list one types into says what has been typed: three rows out of four thousand
        // need a reason on the frame.
        &pick.heading(),
        &shown,
        Some(&hint),
    );
    if pick.rows.is_empty() {
        return None;
    }
    let list = ListArea {
        area: Rect::new(
            area.x + 1,
            area.y + 1,
            area.w.saturating_sub(2),
            shown.len() as u16,
        ),
        top: start,
        len: lines.len(),
    };
    // A list of colours one cannot see would send the reader to the file to find out what a
    // group does. Each row is drawn in the colours its group paints with, over what the
    // dialog has already drawn; the row under the cursor keeps the cursor's colours, because
    // knowing where one is matters more.
    if pick.kind == crate::menu::PickKind::Highlight {
        let width = area.w.saturating_sub(2) as usize;
        for (row, line) in shown.iter().enumerate() {
            let y = area.y + 1 + row as u16;
            if y >= area.y + area.h.saturating_sub(1) || row == pick.cursor.saturating_sub(start) {
                continue;
            }
            let Some(ink) = app.highlight_ink(start + row) else {
                continue;
            };
            let text = format!(" {:<w$}", line, w = width.saturating_sub(1));
            buf.put_str(area.x + 1, y, &text, ink, width as u16);
        }
    }

    // The rules between groups of rows, drawn across the frame and joined to it the way the
    // panel's own separators are: `lines()` leaves them blank because only this knows where
    // the frame is.
    let frame = Style::new(app.theme.color("menu.frame"), app.theme.color("menu.bg"));
    for i in 0..shown.len() {
        if !pick.rows.get(start + i).is_some_and(|r| r.separator) {
            continue;
        }
        let y = area.y + 1 + i as u16;
        if y >= area.bottom() - 1 {
            break;
        }
        for x in area.x + 1..area.right() - 1 {
            buf.set(x, y, '─', frame);
        }
        buf.set(area.x, y, '├', frame);
        buf.set(area.right() - 1, y, '┤', frame);
    }

    // The key each row answers to, in the colour a menu gives a hotkey: in the drive dialog
    // that is the drive's own letter and the number of a panel that is not a drive, and it
    // is the half of the row anybody who uses it twice a day actually reads.
    let keyed = pick.rows.iter().any(|r| r.hotkey.is_some());
    if keyed {
        let ink = Style::new(
            app.theme.color("menu.hotkey.fg"),
            app.theme.color("menu.bg"),
        );
        for (i, row) in pick.rows.iter().skip(start).take(shown.len()).enumerate() {
            let Some(key) = row.hotkey else { continue };
            if start + i == pick.cursor {
                continue; // the cursor row is painted whole, below
            }
            let y = area.y + 1 + i as u16;
            if y >= area.y + area.h.saturating_sub(1) {
                break;
            }
            // `lines()` writes the mark, a space, then the key: two columns in from the text.
            buf.put_str(area.x + 4, y, &key.to_string(), ink, 1);
        }
    }

    let style = Style::new(
        app.theme.color("menu.selected.fg"),
        app.theme.color("menu.selected.bg"),
    );
    let row = pick.cursor.saturating_sub(start);
    let y = area.y + 1 + row as u16;
    if row < shown.len() && y < area.y + area.h.saturating_sub(1) {
        let width = area.w.saturating_sub(2) as usize;
        let text = format!(" {:<w$}", shown[row], w = width.saturating_sub(1));
        buf.put_str(area.x + 1, y, &text, style, width as u16);
        // The key stays yellow on the row one is standing on, as it does in FAR's menus.
        if let Some(key) = pick.rows.get(pick.cursor).and_then(|r| r.hotkey) {
            let ink = Style::new(
                app.theme.color("menu.hotkey.fg"),
                app.theme.color("menu.selected.bg"),
            );
            buf.put_str(area.x + 4, y, &key.to_string(), ink, 1);
        }
    }
    Some(list)
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
            ("bytes", fcmd_core::format_size(p.bytes_done).into()),
            ("bytestotal", fcmd_core::format_size(p.bytes_total).into()),
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
                ("speed", fcmd_core::format_size(speed as u64).into()),
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
    editor: &mut fcmd_edit::HexEdit,
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
    // The hex pane's rows, for the wheel. Sideways means nothing over a dump — its width is
    // the row width, which `Alt+arrow` sets — so only the vertical notch does anything here.
    app.layout.text = Some(body);
    editor.scroll_into_view(body.h as usize);

    // The header: the file, and where the cursor is in it. The hex pane is the editor in
    // another mode, so it wears `Editor.Status` like the editor does.
    let header = app.theme.editor_status();
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    let status = format!(
        "{:08X}/{:08X}  {}  {}{}",
        editor.cursor,
        editor.len(),
        if editor.insert { "ins" } else { "ovr" },
        match editor.pane {
            fcmd_edit::Pane::Hex => "hex",
            fcmd_edit::Pane::Text => "txt",
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
        &fcmd_ui::elide_middle(&title, name_w as usize),
        header,
        name_w,
    );
    buf.put_str(w.saturating_sub(status_w), 0, &status, header, status_w);

    let changed = Style::new(app.theme.color("selected.fg"), app.theme.color("panel.bg"));
    let marked = Style::new(
        app.theme.color("panel.info.selected.fg"),
        app.theme.color("panel.info.selected.bg"),
    );
    // `Editor.Text.Selected`: a marked block, which in FAR's palette is the status
    // line's colour rather than the panel cursor's.
    let on = Style::new(
        app.theme.color("editor.selected.fg"),
        app.theme.color("editor.selected.bg"),
    );
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
            let hex_style = if offset == editor.cursor && editor.pane == fcmd_edit::Pane::Hex {
                on
            } else {
                style
            };
            let txt_style = if offset == editor.cursor && editor.pane == fcmd_edit::Pane::Text {
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
                    fcmd_edit::Pane::Hex => (x, y),
                    fcmd_edit::Pane::Text => (text_x, y),
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
    editor: &mut fcmd_edit::Editor,
    buf: &mut Buffer,
) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let body = Rect::new(0, 1, w, h.saturating_sub(1 + keybar_h));
    app.editor_size = (body.w, body.h);

    let base = app.theme.editor();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    // Header: the file on the left, the position and flags on the right. `Editor.Status`.
    let header = app.theme.editor_status();
    buf.fill(Rect::new(0, 0, w, 1), ' ', header);
    // The language first, where FAR's Colorer puts it: it is the quickest way to see that
    // a file is being read as something other than what you meant.
    let status = match editor.highlight.language() {
        Some(language) if app.cfg.ui.syntax => format!("{language}  {}", editor.status()),
        _ => editor.status(),
    };
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
        &fcmd_ui::elide_middle(&title, name_w as usize),
        header,
        name_w,
    );
    buf.put_str(w - status_w, 0, &status, header, status_w);

    let visible = editor.scroll_into_view(body.h as usize, body.w as usize);
    // `Editor.Text.Selected`: a marked block, which in FAR's palette is the status
    // line's colour rather than the panel cursor's.
    let selected = Style::new(
        app.theme.color("editor.selected.fg"),
        app.theme.color("editor.selected.bg"),
    );

    // Where the text ended up, for the mouse: a click puts the caret there and the wheel
    // scrolls. The editor draws its lines from column zero, so this is the body as it is.
    app.layout.text = Some(body);

    let colouring = app.cfg.ui.syntax;
    for (row, line_index) in visible.clone().enumerate() {
        let y = body.y + row as u16;
        let (text, columns) = expand_with_map(editor, line_index);
        let shown: String = text
            .chars()
            .skip(editor.left)
            .take(body.w as usize)
            .collect();
        buf.put_str(0, y, &shown, base, body.w);

        // The syntax, painted over the line in the foreground colour its class asks for.
        // The background stays the editor's, so a coloured run is a colour and not a block.
        if colouring {
            for span in editor.spans(line_index) {
                let (from, to) = (
                    columns.get(span.start).copied().unwrap_or(0),
                    columns.get(span.end).copied().unwrap_or(0),
                );
                let from = from.saturating_sub(editor.left);
                let to = to.saturating_sub(editor.left);
                if to <= from || from >= body.w as usize {
                    continue;
                }
                let slice: String = shown.chars().skip(from).take(to - from).collect();
                if slice.is_empty() {
                    continue;
                }
                let style = base.with_fg(app.theme.color(span.class.theme_key()));
                let room = body.w - from as u16;
                buf.put_str(from as u16, y, &slice, style, room);
            }
        }

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
        // The same swap as under the panels: a dialog over the editor is what the bar is
        // about while it is there.
        let labels = app
            .keybar_dialog(app.held)
            .unwrap_or_else(|| app.keybar_editor());
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

/// The expanded line, and where each byte of the raw one lands in it.
///
/// The syntax scan works in bytes of the line as it is stored; the screen works in the
/// columns of the line as it is drawn, which differ wherever there is a tab. `columns[b]` is
/// the column the byte at `b` starts in, with one extra entry for the end of the line so
/// that a span reaching the end has somewhere to point.
fn expand_with_map(editor: &fcmd_edit::Editor, line: usize) -> (String, Vec<usize>) {
    let raw = editor.buffer.line(line);
    let mut text = String::with_capacity(raw.len() + 8);
    let mut columns = Vec::with_capacity(raw.len() + 1);
    let mut column = 0usize;
    for (at, ch) in raw.char_indices() {
        while columns.len() <= at {
            columns.push(column);
        }
        if ch == '\t' {
            let pad = editor.tab_width - (column % editor.tab_width);
            // With whitespace shown, a tab is an arrow and then its padding, so a mixed
            // indent — four spaces here, a tab there — can be seen rather than guessed at.
            if editor.show_whitespace {
                text.push('→');
                text.extend(std::iter::repeat_n(' ', pad.saturating_sub(1)));
            } else {
                text.extend(std::iter::repeat_n(' ', pad));
            }
            column += pad;
        } else if ch == ' ' && editor.show_whitespace {
            text.push('·');
            column += 1;
        } else {
            text.push(ch);
            // Columns, not characters: the editor scrolls by what is on screen.
            column += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        }
    }
    while columns.len() <= raw.len() {
        columns.push(column);
    }
    (text, columns)
}

/// A line with its tabs expanded, which is what the screen shows.
fn expand_line(editor: &fcmd_edit::Editor, line: usize) -> String {
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
fn block_columns(editor: &fcmd_edit::Editor, line: usize) -> Option<(usize, usize)> {
    let block = editor.block?;
    let width = expand_line(editor, line).chars().count();
    match block.kind {
        fcmd_edit::BlockKind::Column => {
            let (top, bottom, left, right) = block.columns();
            if line < top || line > bottom {
                return None;
            }
            Some((
                editor.display_col(fcmd_edit::Position::new(line, left)),
                editor.display_col(fcmd_edit::Position::new(line, right)),
            ))
        }
        fcmd_edit::BlockKind::Stream => {
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
/// A picture, painted into `area` in half blocks (SPEC 05 §1.14). Does nothing unless the
/// viewer is in that mode, so the callers can ask without checking first.
///
/// Both of them paint it the same way — the viewer across `F3`'s whole window, the quick view
/// inside the passive panel's frame — and they differ only in the colour a transparent pixel
/// is mixed with, which is whatever that window is drawn in.
fn paint_picture(
    viewer: &mut fcmd_view::Viewer,
    theme: &fcmd_ui::Theme,
    buf: &mut Buffer,
    area: Rect,
    base: Style,
    bg_key: &str,
) -> bool {
    if viewer.mode != fcmd_view::Mode::Image {
        return false;
    }
    let under = match theme.color(bg_key) {
        fcmd_ui::Color::Rgb(r, g, b) => [r, g, b],
        // A theme written in palette colours has no bytes to mix a transparent pixel with,
        // so the picture is composited onto black.
        _ => [0, 0, 0],
    };
    let cells = viewer.picture_cells(area.w, area.h, under);
    // Centred, as every picture viewer draws one: the window is almost never the shape of
    // the picture, and a picture in the corner looks like a mistake.
    let shown_h = cells.len() as u16;
    let shown_w = cells.first().map(|row| row.len() as u16).unwrap_or(0);
    let x0 = area.x + area.w.saturating_sub(shown_w) / 2;
    let y0 = area.y + area.h.saturating_sub(shown_h) / 2;
    for (row, line) in cells.iter().enumerate() {
        for (col, (top, bottom)) in line.iter().enumerate() {
            let style = Style::new(
                fcmd_ui::Color::Rgb(top[0], top[1], top[2]),
                fcmd_ui::Color::Rgb(bottom[0], bottom[1], bottom[2]),
            );
            buf.set(x0 + col as u16, y0 + row as u16, '▀', style);
        }
    }
    // Nothing could be decoded: the window says why rather than staying blank.
    if cells.is_empty() {
        if let Err(why) = viewer.picture_note() {
            let text = fcmd_ui::truncate_cols(&why, area.w as usize);
            buf.put_str(area.x, area.y, &text, base, area.w);
        }
    }
    true
}

fn draw_quick_view(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let title = app.i18n.t("panel-quick-view");
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);
    let base = app.theme.panel();

    // A folder has no bytes to preview, and FAR fills the frame with what the device under
    // it has to say instead — which is the same block the information panel draws.
    let other = &app.panels[1 - index];
    if let Some(entry) = other.current().filter(|e| e.opens_as_dir()) {
        let mut lines: Vec<(String, String)> =
            vec![(app.i18n.t("info-folder"), entry.name_str().into_owned())];
        if let Some(mtime) = entry.mtime {
            lines.push((app.i18n.t("info-modified"), widgets::format_mtime(mtime)));
        }
        lines.push((String::new(), String::new()));
        lines.extend(device_lines(app, &app.panels[1 - index]));
        draw_pairs(app, buf, inner, &lines);
        return;
    }

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

    // A picture is painted rather than written, here as in the viewer: without this the
    // quick view of a `.png` is an empty frame, because a picture has no rows of text.
    let body = Rect::new(inner.x + 1, inner.y, inner.w.saturating_sub(1), inner.h);
    let drew = paint_picture(viewer, &app.theme, buf, body, base, "panel.bg");

    // The name on the frame's foot, as a panel's totals are, so the preview itself is all
    // the room the rows get.
    let rows = if drew {
        Vec::new()
    } else {
        viewer.rows(inner.w.saturating_sub(1), inner.h)
    };
    for (n, row) in rows.iter().enumerate() {
        let text = fcmd_ui::truncate_cols(&row.text, inner.w.saturating_sub(1) as usize);
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
    let name = fcmd_ui::elide_middle(&name, area.w.saturating_sub(4) as usize);
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
        let text = fcmd_ui::truncate_cols(&text, inner.w.saturating_sub(1) as usize);
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
/// What the device under a directory has to say: what it holds, what is left on it, how big
/// its allocation unit is, how much memory the machine has, and what the clusters cost the
/// listing on screen. The information panel and a quick view of a folder both show it, which
/// is what FAR does with the same numbers.
fn device_lines(app: &App, panel: &crate::panel::Panel) -> Vec<(String, String)> {
    let mut lines: Vec<(String, String)> = Vec::new();
    let t = |id: &str| app.i18n.t(id);
    // The device, as FAR's information panel has it: what it holds, what is left, and how
    // big its allocation unit is. A server and the inside of an archive have none of this.
    if let Some(d) = panel.disk {
        lines.push((
            t("info-device"),
            app.i18n.t_args(
                "info-device-value",
                &[
                    ("total", fcmd_core::format_size(d.total).into()),
                    ("free", fcmd_core::format_size(d.free).into()),
                    ("cluster", fcmd_core::format_size(d.cluster).into()),
                ],
            ),
        ));
    }
    if let Some(m) = fcmd_platform::memory() {
        lines.push((
            t("info-memory"),
            match m.available {
                Some(free) => app.i18n.t_args(
                    "info-memory-value",
                    &[
                        ("free", fcmd_core::format_size(free).into()),
                        ("total", fcmd_core::format_size(m.total).into()),
                    ],
                ),
                // macOS will say how much there is and not how much is free without a great
                // deal more machinery; half an answer beats a wrong one.
                None => app.i18n.t_args(
                    "info-memory-total",
                    &[("total", fcmd_core::format_size(m.total).into())],
                ),
            },
        ));
    }
    // What the directory is sitting on, which is the other half of "where am I": the
    // filesystem's name, and how it was mounted where the platform keeps that. `ro` and
    // `noexec` are the two words that explain most of the refusals a panel can produce.
    if let Some(m) = &panel.mount {
        lines.push((
            t("info-filesystem"),
            if m.options.is_empty() {
                m.fs.clone()
            } else {
                format!("{} ({})", m.fs, m.options_line())
            },
        ));
        lines.push((
            t("info-mount"),
            format!("{} on {}", m.source, m.target.display()),
        ));
    }
    // The repository, as its own files say it. The branch and the commit are two small reads;
    // the count is a `stat` per tracked file, and says nothing rather than guess when the
    // index is one this reader does not know or is too big to glance at.
    if let Some(repo) = &panel.git {
        let mut value = repo.name();
        if let (Some(_), Some(head)) = (&repo.branch, &repo.head) {
            value = format!("{value} · {head}");
        }
        if let Some(state) = repo.state {
            // Spelled out rather than built from the state's name: a message id that is not
            // written anywhere is a message id nobody can find, and the catalogue's own test
            // says so.
            value = format!(
                "{value} · {}",
                t(match state {
                    fcmd_core::git::State::Merge => "info-git-merge",
                    fcmd_core::git::State::Rebase => "info-git-rebase",
                    fcmd_core::git::State::CherryPick => "info-git-cherry-pick",
                    fcmd_core::git::State::Revert => "info-git-revert",
                    fcmd_core::git::State::Bisect => "info-git-bisect",
                })
            );
        }
        match repo.index {
            Some(index) if index.differing == 0 => {
                value = format!("{value} · {}", t("info-git-clean"));
            }
            Some(index) => {
                value = format!(
                    "{value} · {}",
                    app.i18n.t_args(
                        "info-git-differing",
                        &[
                            ("count", (index.differing as i64).into()),
                            ("tracked", (index.tracked as i64).into()),
                        ],
                    )
                );
            }
            None => value = format!("{value} · {}", t("info-git-uncounted")),
        }
        lines.push((t("info-git"), value));
    }
    let totals = &panel.listing.totals;
    lines.push((
        t("info-contents"),
        app.i18n.t_args(
            "info-contents-value",
            &[
                ("files", (totals.files as i64).into()),
                ("dirs", (totals.dirs as i64).into()),
                ("bytes", fcmd_core::format_size(totals.bytes).into()),
            ],
        ),
    ));
    // What the cluster size costs: the tail of the last cluster of every file, which is the
    // figure FAR calls slack and the reason a directory of small files takes more room than
    // it holds. Only with a cluster size to count it with, and counted over the listing that
    // is on screen rather than down the tree.
    if let Some(cluster) = panel.disk.map(|d| d.cluster).filter(|c| *c > 0) {
        let slack: u64 = (0..panel.listing.len())
            .filter_map(|row| panel.listing.row(row))
            .filter(|e| !e.kind.is_dirlike())
            .map(|e| fcmd_core::allocated(e.size, cluster) - e.size)
            .sum();
        lines.push((t("info-slack"), fcmd_core::format_size(slack)));
    }
    lines
}

fn draw_info(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let title = app.i18n.t("panel-info");
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);

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
    lines.extend(device_lines(app, other));
    let totals = &other.listing.totals;
    if !other.selection.is_empty() {
        lines.push((
            t("info-selected"),
            app.i18n.t_args(
                "info-selected-value",
                &[
                    ("count", (totals.selected_files as i64).into()),
                    (
                        "bytes",
                        fcmd_core::format_size(totals.selected_bytes).into(),
                    ),
                ],
            ),
        ));
    }
    if let Some(entry) = other.current() {
        lines.push((String::new(), String::new()));
        lines.push((t("info-cursor"), entry.name_str().into_owned()));
        if !entry.kind.is_dirlike() {
            lines.push((t("info-size"), fcmd_core::format_size(entry.size)));
        }
        if let Some(mtime) = entry.mtime {
            lines.push((t("info-modified"), widgets::format_mtime(mtime)));
        }
        // What the directory says about it, if it says anything (SPEC 02 §9). The
        // information panel is where FAR shows a description in full, and it is the one
        // place with room for one.
        if let Some(descr) = other.descriptions.get(entry.name_str().as_ref()) {
            lines.push((
                t("info-description"),
                fcmd_core::descriptions::one_line(descr),
            ));
        }
    }

    draw_pairs(app, buf, inner, &lines);
}

/// A column of labels and a column of values, as the information panel and the quick view of
/// a folder both want them: the labels in the header colour, aligned on the widest of them.
fn draw_pairs(app: &App, buf: &mut Buffer, inner: Rect, lines: &[(String, String)]) {
    let base = app.theme.panel();
    let label = Style::new(
        app.theme.color("panel.header.fg"),
        app.theme.color("panel.bg"),
    );
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
        let value = fcmd_ui::elide_middle(value, room as usize);
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

fn draw_viewer(app: &mut App, viewer: &mut fcmd_view::Viewer, buf: &mut Buffer) {
    let (w, h) = (buf.width(), buf.height());
    // The chrome the reader is allowed to take away, a row at a time (`Ctrl+B`,
    // `Ctrl+Shift+B`, `Ctrl+S`): what it leaves goes to the content, which is the point of
    // taking it away. The key bar also answers to the interface setting, because a user who
    // turned it off everywhere did not mean "except in the viewer".
    let keybar_h = u16::from(app.cfg.ui.key_bar && viewer.show_key_bar);
    let header_h = u16::from(viewer.show_status_line);
    let body = Rect::new(0, header_h, w, h.saturating_sub(header_h + keybar_h));
    app.viewer_size = (body.w, body.h);

    let base = app.theme.viewer();
    buf.fill(Rect::new(0, 0, w, h.saturating_sub(keybar_h)), ' ', base);

    // Top line: the file on the left, the status on the right. `Viewer.Status` in FAR's own
    // palette, which is a colour of its own and not the panel header's over the status bar's.
    let status = viewer.status();
    if header_h == 1 {
        let header = app.theme.viewer_status();
        buf.fill(Rect::new(0, 0, w, 1), ' ', header);
        let status_text = app.viewer_status(&status);
        let status_w = status_text.chars().count().min(w as usize) as u16;
        let name_w = w.saturating_sub(status_w + 2);
        buf.put_str(
            0,
            0,
            &fcmd_ui::elide_middle(&viewer.path.to_string(), name_w as usize),
            header,
            name_w,
        );
        buf.put_str(w - status_w, 0, &status_text, header, status_w);
    }

    // The scrollbar takes a column off the content before anything else is laid out: a bar
    // drawn over the last column of a line would hide what it is there to help find.
    let mut body = body;
    let bar = (viewer.show_scrollbar && body.w > 2 && body.h > 0).then(|| {
        let x = body.right() - 1;
        body = Rect::new(body.x, body.y, body.w - 1, body.h);
        app.viewer_size = (body.w, body.h);
        x
    });

    // The structure pane takes what the content does not need. In hex the row width is
    // decided by the data rather than the window, so the pane gets the remainder — that way
    // the printable gutter stays whole instead of being cut off by the pane's frame.
    let pane = if viewer.show_structure {
        let third = (body.w / 3).clamp(28, 48);
        let pane_w = match viewer.mode {
            fcmd_view::Mode::Hex => body
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

    // Where the content ended up, for the mouse: the wheel over a viewer scrolls it, and
    // the sideways wheel moves a long line. Recorded after the scrollbar and the structure
    // pane have taken their columns, so a notch over the pane is not a notch over the text.
    app.layout.text = Some(body);

    // Content. `Viewer.Text.Selected` marks a search hit — its own role in FAR's palette,
    // which the panel's background had no business being half of.
    let hit = Style::new(
        app.theme.color("viewer.selected.fg"),
        app.theme.color("viewer.selected.bg"),
    )
    .bold();
    let rows = viewer.rows(body.w, body.h);
    // The syntax, where the file is one we recognise and the mode is the one that shows it
    // as text. The scan starts at the first row on screen rather than at the start of the
    // file: the viewer never reads more than it shows, so a block comment opened above the
    // window is not carried — which is the price of opening a two-gigabyte log instantly.
    let syntax = (app.cfg.ui.syntax && viewer.mode == fcmd_view::Mode::Text)
        .then(|| fcmd_syntax::of_name(&viewer.path.display_name()))
        .flatten();
    let mut state = fcmd_syntax::State::default();
    for (i, row) in rows.iter().enumerate() {
        let y = body.y + i as u16;
        buf.put_str(0, y, &row.text, base, body.w);
        // A rendered row carries its own runs: the Markdown view decided what a heading, a
        // bullet and a table's frame are when it laid the line out, and re-scanning the text
        // it produced would only guess at it again.
        for span in &row.spans {
            let Some(text) = row.text.get(span.start..span.end) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            let x = row.text[..span.start]
                .chars()
                .filter_map(unicode_width::UnicodeWidthChar::width)
                .sum::<usize>() as u16;
            if x >= body.w {
                continue;
            }
            let mut style = base.with_fg(app.theme.color(span.class.theme_key()));
            style.bold = span.bold;
            buf.put_str(x, y, text, style, body.w - x);
        }
        if let Some(syntax) = syntax {
            let (spans, next) = syntax.line(&row.text, state);
            state = next;
            for span in spans {
                let text = &row.text[span.start..span.end];
                let x = row.text[..span.start]
                    .chars()
                    .filter_map(unicode_width::UnicodeWidthChar::width)
                    .sum::<usize>() as u16;
                if x < body.w && !text.is_empty() {
                    let style = base.with_fg(app.theme.color(span.class.theme_key()));
                    buf.put_str(x, y, text, style, body.w - x);
                }
            }
        }
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
    if rows.is_empty() && viewer.mode != fcmd_view::Mode::Image {
        buf.put_str(0, body.y, &app.i18n.t("viewer-empty"), base, body.w);
    }
    paint_picture(viewer, &app.theme, buf, body, base, "viewer.bg");

    if let Some(at) = pane {
        draw_structure(app, viewer, buf, at);
    }

    // Where in the file we are, as a bar. It says *where* and not *how much*: how many bytes
    // a screenful holds is not a fixed fraction of a file — a wrapped line, a hex dump and a
    // rendered Markdown page each answer differently — so the thumb is one row and the
    // position is the percentage the status line already shows.
    if let Some(x) = bar {
        let style = Style::new(
            app.theme.color("panel.scrollbar.fg"),
            app.theme.color("viewer.bg"),
        );
        let track = body.h as usize;
        let at = (usize::from(status.percent.min(100)) * track.saturating_sub(1) + 50) / 100;
        for i in 0..track {
            let ch = if i == at { '█' } else { '░' };
            buf.set(x, body.y + i as u16, ch, style);
        }
    }

    if keybar_h == 1 {
        let labels = app
            .keybar_dialog(app.held)
            .unwrap_or_else(|| app.keybar_viewer());
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
                format!("  {}", fcmd_core::format_size(e.size))
            };
            let time = e
                .mtime
                .map(|t| format!("  {}", widgets::format_mtime(t)))
                .unwrap_or_default();
            // And what the directory says about it, when the directory says anything: the
            // status line is where a description is read while the view is not one that
            // shows it (SPEC 02 §9).
            let name = e.name_str();
            let descr = panel
                .descriptions
                .get(name.as_ref())
                .map(|d| format!("  {}", fcmd_core::descriptions::one_line(d)))
                .unwrap_or_default();
            format!("{name}{size}{time}{descr}")
        }
        None if panel.loading => app.i18n.t("panel-loading"),
        None => app.i18n.t("panel-empty"),
    }
}

/// A search: its results so far, what it is scanning, and what the keys do. Full screen,
/// because a list of results is the thing you are working with, not a message about one.
/// The synchronisation table (SPEC 03 §10): one row per pair, with what will happen to it.
///
/// The shape is the point. The arrow is in the middle, between the two sides' sizes and
/// times, so which way a file is about to go is read in the same place on every row — and the
/// rows that will do nothing are dim, so what the eye lands on is what is about to change.
/// The palette editor: one row per colour, with the colour itself shown twice — as ink on the
/// panel background and as a background behind the panel's ink — because a colour is only ever
/// used as one or the other, and a swatch that showed one of them would be half an answer.
fn draw_palette(app: &App, view: &crate::app::PaletteView, buf: &mut Buffer, screen: Rect) {
    let base = app.theme.panel();
    let frame = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let dim = Style::new(
        app.theme.color("panel.footer.fg"),
        app.theme.color("panel.bg"),
    );
    let picked = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));

    buf.fill(screen, ' ', base);
    let title = app
        .i18n
        .t_args("palette-title", &[("theme", app.theme.id.clone().into())]);
    buf.draw_frame(screen, frame, Some(&title));
    let inner = screen.inner();
    if inner.h < 3 {
        return;
    }
    // One row at the foot for what the keys do.
    let list_h = inner.h.saturating_sub(1) as usize;
    // `#rrggbb` is the longest a colour is written as, and the two swatches are four cells
    // each; the name takes what is left.
    let value_w = 10u16;
    let swatch_w = 5u16;
    // The longest key is about thirty columns, so the name column stops there rather than
    // stretching to the frame and leaving the colour it names a screen away from it.
    let name_w = inner
        .w
        .saturating_sub(value_w + swatch_w * 2 + 2)
        .clamp(8, 32);

    for i in 0..list_h {
        let Some(key) = view.keys.get(view.top + i) else {
            break;
        };
        let y = inner.y + i as u16;
        let on = view.top + i == view.cursor;
        let row = if on { picked } else { base };
        buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', row);

        let mut x = inner.x;
        buf.put_str(
            x,
            y,
            &fcmd_ui::truncate_cols(key, name_w as usize),
            row,
            name_w,
        );
        x += name_w + 1;

        let color = app.theme.color(key);
        let text = color.to_string();
        buf.put_str(
            x,
            y,
            &fcmd_ui::truncate_cols(&text, value_w as usize),
            row,
            value_w,
        );
        x += value_w + 1;

        // As ink, then as background. `Default` means "whatever the terminal does", which is
        // exactly what the two swatches then show.
        let as_ink = Style::new(color, app.theme.color("panel.bg"));
        buf.put_str(x, y, " Abc ", as_ink, swatch_w);
        x += swatch_w;
        let as_paper = Style::new(app.theme.color("panel.fg"), color);
        buf.put_str(x, y, " Abc ", as_paper, swatch_w);
    }

    let hint = app.i18n.t("palette-hint");
    let y = inner.y + inner.h.saturating_sub(1);
    buf.put_str(
        inner.x,
        y,
        &fcmd_ui::truncate_cols(&hint, inner.w as usize),
        dim,
        inner.w,
    );
}

/// Two files side by side, with what differs marked (SPEC 05 §3).
///
/// The two columns are the same width, because the eye compares by position: a left column
/// sized to its longest line and a right one sized to its own would put the same line in two
/// places. Both scroll sideways together for the same reason.
/// The screen grabber: the frame it was opened on, the marked rectangle inverted, and a line
/// of keys where the status line is (SPEC 06 §11b).
///
/// The cursor is left where the terminal's own goes, so the one the reader is moving is the
/// terminal's: a block over a letter is what every editor shows, and drawing a second one
/// would be a second thing to follow.
fn draw_grab(app: &App, grab: &crate::grab::Grab, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    // The frame as it was. A terminal resized since the key was pressed leaves a margin,
    // which is filled rather than left as whatever was under it.
    buf.clear(Style::new(
        app.theme.color("panel.fg"),
        app.theme.color("panel.bg"),
    ));
    for y in 0..h.min(grab.height()) {
        for x in 0..w.min(grab.width()) {
            if let Some(cell) = grab.screen.get(x, y) {
                buf.set(x, y, cell.ch, cell.style);
            }
        }
    }
    // What is marked, in the colours a selection has everywhere else.
    let area = grab.area();
    let marked = Style::new(
        app.theme.color("selected.fg"),
        app.theme.color("selected.bg"),
    );
    for y in area.y..area.bottom().min(h) {
        for x in area.x..area.right().min(w) {
            if let Some(cell) = buf.cell_mut(x, y) {
                cell.style = marked;
            }
        }
    }
    // The keys, on the last row: the grabber has keys of its own, and a reader who cannot see
    // them has to guess.
    if h > 0 {
        let hint = app.i18n.t("grab-hint");
        let hint = fcmd_ui::truncate_cols(&hint, w as usize);
        let style = Style::new(app.theme.color("status.fg"), app.theme.color("status.bg"));
        buf.fill(Rect::new(0, h - 1, w, 1), ' ', style);
        buf.put_str(0, h - 1, &hint, style, w);
    }
    Some((
        grab.x.min(w.saturating_sub(1)),
        grab.y.min(h.saturating_sub(1)),
    ))
}

fn draw_diff(app: &App, view: &fcmd_view::Diff, buf: &mut Buffer, screen: Rect) {
    use fcmd_view::Side;

    let base = app.theme.panel();
    let frame = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let dim = Style::new(
        app.theme.color("panel.footer.fg"),
        app.theme.color("panel.bg"),
    );
    // One ink per kind, taken from what the panel already uses for the same idea: a line that
    // is only on one side is the colour of a file that is not there any more, and the part of
    // a line that changed is the selection's.
    let removed = Style::new(app.theme.color("file.exec"), app.theme.color("panel.bg"));
    let added = Style::new(app.theme.color("file.archive"), app.theme.color("panel.bg"));
    let inner_mark = Style::new(
        app.theme.color("selected.fg"),
        app.theme.color("selected.bg"),
    );

    buf.fill(screen, ' ', base);
    let title = app.i18n.t("diff-title");
    buf.draw_frame(screen, frame, Some(&title));
    let inner = screen.inner();
    if inner.h < 3 || inner.w < 20 {
        return;
    }
    // A row at the top for the two names, and one at the foot for the keys and the count.
    let rows_h = inner.h.saturating_sub(2) as usize;
    // Each side: a line number, a space, and the text. The middle column is one character.
    let side_w = (inner.w.saturating_sub(1) / 2).max(8);
    let number_w = 6u16;
    let text_w = side_w.saturating_sub(number_w + 1).max(4) as usize;

    let names = format!(
        "{}  ↔  {}",
        fcmd_ui::elide_middle(&view.left_name, (side_w as usize).saturating_sub(2)),
        fcmd_ui::elide_middle(&view.right_name, (side_w as usize).saturating_sub(2))
    );
    buf.put_str(inner.x, inner.y, &names, dim, inner.w);

    let top = inner.y + 1;
    for (i, row) in view.window(rows_h as u16).iter().enumerate() {
        let y = top + i as u16;
        let style = match row.kind {
            Side::Same => base,
            Side::Removed => removed,
            Side::Added => added,
            Side::Changed => base,
        };
        // The rule down the middle, drawn whatever is on either side of it.
        buf.put_str(inner.x + side_w, y, "│", frame, 1);
        let mut side =
            |x: u16, number: Option<usize>, text: &str, mark: &std::ops::Range<usize>| {
                let label = match number {
                    Some(n) => format!("{n:>5} "),
                    None => " ".repeat(number_w as usize),
                };
                buf.put_str(x, y, &label, dim, number_w);
                let from = view.col.min(text.len());
                let shown: String = text[from..].chars().take(text_w).collect();
                buf.put_str(x + number_w, y, &shown, style, text_w as u16);
                // The part that differs, over the top of what was just drawn: the eye goes
                // there, which is the whole point of a word diff.
                if row.kind == Side::Changed && !mark.is_empty() {
                    let start = mark.start.max(from);
                    let end = mark.end.max(start);
                    if end > from {
                        let before = text[from..start].chars().count();
                        let part: String = text[start..end].chars().take(text_w).collect();
                        if before < text_w {
                            buf.put_str(
                                x + number_w + before as u16,
                                y,
                                &part,
                                inner_mark,
                                (text_w - before) as u16,
                            );
                        }
                    }
                }
            };
        side(inner.x, row.left_no, &row.left, &row.left_inner);
        side(
            inner.x + side_w + 1,
            row.right_no,
            &row.right,
            &row.right_inner,
        );
    }

    let foot = if view.identical() {
        app.i18n.t("diff-same")
    } else {
        app.i18n.t_args(
            "diff-where",
            &[
                ("at", (view.hunk_at() as i64).into()),
                ("count", (view.hunks() as i64).into()),
            ],
        )
    };
    let keys = app.i18n.t("diff-hint");
    let y = inner.y + inner.h - 1;
    buf.put_str(inner.x, y, &foot, dim, inner.w);
    // The keys go at the right end, where there is room for them.
    let room = inner.w as usize;
    let used = foot.chars().count() + 2;
    let want = keys.chars().count();
    if room > used + want {
        buf.put_str(inner.x + (room - want) as u16, y, &keys, dim, want as u16);
    }
}

fn draw_sync(app: &App, view: &crate::app::SyncView, buf: &mut Buffer, screen: Rect) {
    use fcmd_jobs::sync::{Action, Verdict};

    let base = app.theme.panel();
    let frame = Style::new(app.theme.color("panel.frame"), app.theme.color("panel.bg"));
    let dim = Style::new(
        app.theme.color("panel.footer.fg"),
        app.theme.color("panel.bg"),
    );
    let picked = Style::new(app.theme.color("cursor.fg"), app.theme.color("cursor.bg"));
    let gone = Style::new(app.theme.color("file.exec"), app.theme.color("panel.bg"));
    let clash = Style::new(app.theme.color("selected.fg"), app.theme.color("panel.bg"));

    buf.fill(screen, ' ', base);
    let title = app.i18n.t("sync-title");
    buf.draw_frame(screen, frame, Some(&title));
    let inner = screen.inner();
    if inner.h < 4 {
        return;
    }
    // Three rows at the foot: the two roots, what the totals are, and what the keys do.
    let list_h = inner.h.saturating_sub(3) as usize;
    // ` size  time ` on each side of a three-column arrow, wide enough for the whole of a
    // time: a date cut off halfway is worse than no date.
    let side_w = 28u16;
    let arrow_w = 4u16;
    let name_w = inner.w.saturating_sub(side_w * 2 + arrow_w + 2).max(8);

    for i in 0..list_h {
        let Some(pair) = view.pairs.get(view.top + i) else {
            break;
        };
        let y = inner.y + i as u16;
        let on = view.top + i == view.cursor;
        let style = if on {
            picked
        } else {
            match (pair.verdict, pair.action) {
                (_, Action::DeleteLeft | Action::DeleteRight) => gone,
                (Verdict::Conflict, _) => clash,
                (Verdict::Same, _) => dim,
                _ => base,
            }
        };
        if on {
            buf.fill(Rect::new(inner.x, y, inner.w, 1), ' ', style);
        }
        let name = if pair.is_dir() {
            format!("{}/", pair.rel)
        } else {
            pair.rel.clone()
        };
        buf.put_str(
            inner.x,
            y,
            &fcmd_ui::elide_middle(&name, name_w as usize),
            style,
            name_w,
        );
        let facts = |side: &Option<fcmd_jobs::Facts>| -> String {
            match side {
                Some(f) if f.is_dir() => format!("{:>9}  {:<16}", "", ""),
                Some(f) => format!(
                    "{:>9}  {:<16}",
                    fcmd_core::format_size(f.size),
                    f.mtime.map(widgets::format_mtime).unwrap_or_default()
                ),
                None => String::new(),
            }
        };
        let mut x = inner.x + name_w + 1;
        buf.put_str(x, y, &facts(&pair.left), style, side_w);
        x += side_w;
        buf.put_str(x, y, &format!(" {} ", pair.action.mark()), style, arrow_w);
        x += arrow_w;
        if x < inner.right() {
            buf.put_str(x, y, &facts(&pair.right), style, inner.right() - x);
        }
    }

    // The two roots, so there is never a doubt which side is which.
    let roots = Rect::new(inner.x, inner.bottom() - 3, inner.w, 1);
    let between = app.i18n.t_args(
        "sync-between",
        &[
            ("left", view.left.to_string().into()),
            ("right", view.right.to_string().into()),
        ],
    );
    buf.put_str(
        roots.x,
        roots.y,
        &fcmd_ui::elide_middle(&between, roots.w as usize),
        dim,
        roots.w,
    );

    let (copies, deletes, bytes) = view.totals();
    let foot = if view.done {
        app.i18n.t_args(
            "sync-planned",
            &[
                ("copies", (copies as i64).into()),
                ("deletes", (deletes as i64).into()),
                ("bytes", fcmd_core::format_size(bytes).into()),
            ],
        )
    } else {
        app.i18n.t_args(
            "sync-walking",
            &[
                ("at", view.scanning.clone().unwrap_or_default().into()),
                ("pairs", (view.pairs.len() as i64).into()),
            ],
        )
    };
    let y = inner.bottom() - 2;
    buf.put_str(
        inner.x,
        y,
        &fcmd_ui::truncate_cols(&foot, inner.w as usize),
        base,
        inner.w,
    );
    let hint = app.i18n.t(if view.done {
        "sync-hint"
    } else {
        "sync-hint-walking"
    });
    buf.put_str(
        inner.x,
        inner.bottom() - 1,
        &fcmd_ui::truncate_cols(&hint, inner.w as usize),
        dim,
        inner.w,
    );
}

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
        let shown = fcmd_ui::elide_middle(&path.to_string(), name_w);
        buf.put_str(inner.x, y, &shown, style, name_w as u16);
        let size = if entry.kind.is_dirlike() {
            String::new()
        } else {
            fcmd_core::format_size(entry.size)
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
        format!("{counters}  ·  {}", fcmd_ui::elide_middle(scanning, 40))
    };
    let y = inner.bottom() - 2;
    buf.put_str(
        inner.x,
        y,
        &fcmd_ui::truncate_cols(&status, inner.w as usize),
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
        &fcmd_ui::truncate_cols(&hint, inner.w as usize),
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
        let mark = if app.connected.contains(&site.key()) {
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

/// The theme key a Markdown run is drawn with inside the help dialog.
///
/// The viewer paints these runs with the syntax theme, which is written for a panel's dark
/// background; the help is a dialog, and on a dialog those colours are the one thing nobody
/// can read. So the dialog has inks of its own, and a fenced block in a language we highlight
/// is drawn as plain code here rather than in three colours that were not chosen for grey.
fn help_ink(class: fcmd_view::markdown::Class) -> &'static str {
    use fcmd_view::markdown::Class;
    match class {
        Class::Heading | Class::Emphasis => "help.heading.fg",
        Class::Strong | Class::Text => "dialog.fg",
        Class::Code | Class::Syntax(_) => "help.code.fg",
        Class::Link => "help.link.fg",
        Class::Marker => "help.marker.fg",
        Class::Strike | Class::Url | Class::Frame => "help.frame.fg",
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
    let lines = app.help_rows(chapter, box_w.saturating_sub(4) as usize);
    let box_h = screen.h.saturating_sub(2).max(6);
    let x = screen.x + (screen.w - box_w) / 2;
    let y = screen.y + (screen.h - box_h) / 2;
    let area = Rect::new(x, y, box_w, box_h);

    let style = app.theme.dialog();
    buf.fill(area, ' ', style);
    // The frame names the chapter one is in, and the index names the program.
    let title = match crate::help::chapters(&app.i18n)
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
        let y = inner.y + row as u16;
        // In the index the row under the cursor is drawn as chosen, because the index is a
        // list one picks from rather than a page one reads.
        if index && scroll + row == at {
            let style = Style::new(
                app.theme.color("menu.selected.fg"),
                app.theme.color("menu.selected.bg"),
            );
            let text = format!(
                " {text:<w$}",
                text = line.text,
                w = inner.w.saturating_sub(1) as usize
            );
            buf.put_str(inner.x, y, &text, style, inner.w);
            continue;
        }
        buf.put_str(inner.x, y, &line.text, style, inner.w);
        // The row says what its own runs are — a heading, a bullet, a table's frame, a code
        // span — the way the viewer's Markdown view does, and the dialog paints them in its
        // own inks rather than the viewer's: a help page is read on a dialog background.
        for span in &line.spans {
            let Some(text) = line.text.get(span.start..span.end) else {
                continue;
            };
            if text.is_empty() {
                continue;
            }
            let x = inner.x
                + line.text[..span.start]
                    .chars()
                    .filter_map(unicode_width::UnicodeWidthChar::width)
                    .sum::<usize>() as u16;
            if x >= inner.right() {
                continue;
            }
            let mut painted = style.with_fg(app.theme.color(help_ink(span.class)));
            painted.bold = span.bold;
            buf.put_str(x, y, text, painted, inner.right() - x);
        }
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
        &fcmd_ui::truncate_cols(&hint, inner.w as usize),
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
    use fcmd_cmd::Keymap;
    use fcmd_config::{Config, Paths};
    use fcmd_core::{Entry, EntryFlags, EntryId, EntryKind, VPath};
    use fcmd_i18n::I18n;
    use fcmd_tui::Input;
    use fcmd_ui::{Theme, ThemeSet};
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
            paths: Paths::discover(Some(Path::new("/tmp/fcmd-test/fcmd.toml")), None),
            i18n: I18n::new(lang),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::builtin(theme).unwrap(),
            left: VPath::local("/home/mb/project"),
            right: VPath::local("/tmp"),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
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

    /// The completion list is drawn above the line and over the panels, because below it are
    /// the status line and the key bar — the one row that says what the keys do.
    #[test]
    fn the_completion_list_is_drawn_above_the_command_line() {
        let mut app = app("far", "en");
        for key in ["v", "i", "m", "Space", "C", "a", "r"] {
            press(&mut app, key);
        }
        assert!(app.complete.is_some(), "nothing for {:?}", app.cmd.text);
        let screen = frame(&mut app, 100, 24);
        // `Cargo.toml` is in the panel's listing, so the list offers it — and it is offered
        // for `car`, because a completion that minded the case would be one nobody uses.
        assert!(screen.contains("Cargo.toml"), "{screen}");
        let lines: Vec<&str> = screen.lines().collect();
        let line_row = lines
            .iter()
            .position(|l| l.contains("vim car"))
            .unwrap_or_else(|| panic!("{screen}"));
        let list_row = lines
            .iter()
            .enumerate()
            .filter(|(i, l)| *i != line_row && l.contains("Cargo.toml"))
            .map(|(i, _)| i)
            .next_back()
            .unwrap_or_else(|| panic!("no list in\n{screen}"));
        assert!(list_row < line_row, "the list is above the line: {screen}");
        // The key bar is still there, which is the reason it goes above.
        assert!(screen.contains("Quit"), "{screen}");
    }

    /// The best answer, drawn after the caret and *not* in the line: `Enter` still runs what
    /// was typed, and `→` is what takes the offer (SPEC 06 §7.4).
    #[test]
    fn the_best_answer_is_offered_after_the_caret_and_the_arrow_takes_it() {
        let mut app = app("far", "en");
        for key in ["v", "i", "m", "Space", "s"] {
            press(&mut app, key);
        }
        // `src` is a directory in the panel's listing, so the answer is it with a separator.
        assert_eq!(app.completion_hint(), Some("rc/"));
        assert_eq!(app.cmd.text, "vim s", "the line is what was typed");
        assert_eq!(app.cmd.selection(), None, "and nothing of it is marked");
        let screen = frame(&mut app, 100, 24);
        assert!(screen.contains("vim src/"), "{screen}");

        // `→` at the end of the line takes it.
        press(&mut app, "Right");
        assert_eq!(app.cmd.text, "vim src/");
        assert_eq!(app.cmd.caret, "vim src/".chars().count());

        // Walking into the list puts the cursor's row in charge, so the hint goes: two
        // answers on the screen at once would be one too many.
        app.cmd.clear();
        for key in ["v", "i", "m", "Space", "s"] {
            press(&mut app, key);
        }
        press(&mut app, "Down");
        assert_eq!(app.completion_hint(), None);
    }

    /// An answer that would change what is already typed is not appended: `Cargo.toml` is
    /// offered for `car` — a completion that minded the case would be one nobody uses — but
    /// drawing `go.toml` after `car` would show a line that is not the line one would get.
    /// `Tab` takes that one.
    #[test]
    fn an_answer_that_rewrites_what_was_typed_is_not_offered_inline() {
        let mut app = app("far", "en");
        for key in ["v", "i", "m", "Space", "c", "a", "r"] {
            press(&mut app, key);
        }
        assert!(app.complete.is_some(), "it is in the list");
        assert_eq!(app.completion_hint(), None, "but not after the caret");
        press(&mut app, "Tab");
        assert_eq!(app.cmd.text, "vim Cargo.toml");
    }

    /// FAR's own appending, for somebody who wants FAR: the rest goes into the line, marked,
    /// so the next character replaces it.
    #[test]
    fn fars_appending_writes_into_the_line_and_a_keystroke_undoes_it() {
        let mut app = app("far", "en");
        app.cfg.cmdline.complete.append = "line".into();
        for key in ["v", "i", "m", "Space", "s"] {
            press(&mut app, key);
        }
        assert_eq!(app.cmd.text, "vim src/");
        // What was added is marked out, which is what makes it safe: every key that types or
        // deletes takes the selection first.
        assert_eq!(app.cmd.selected_text().as_deref(), Some("rc/"));
        // And no hint is drawn on top of it.
        assert_eq!(app.completion_hint(), None);

        // Typing on replaces the offer rather than adding to it.
        press(&mut app, "y");
        assert!(
            app.cmd.text.starts_with("vim sy"),
            "the typed line comes back: {:?}",
            app.cmd.text
        );
    }

    /// FAR's list owns `Enter` the moment it appears. Ours waits to be walked into, unless
    /// the configuration asks for FAR's manners.
    #[test]
    fn a_modal_list_takes_enter_at_once() {
        let mut app = app("far", "en");
        app.cfg.cmdline.complete.append = "off".into();
        for key in ["v", "i", "m", "Space", "C", "a", "r"] {
            press(&mut app, key);
        }
        // Ours: `Enter` runs the line, which clears it and does not complete it.
        assert!(app.complete.is_some());
        press(&mut app, "Enter");
        assert_ne!(app.cmd.text, "vim Cargo.toml");

        app.cfg.cmdline.complete.modal = true;
        for key in ["v", "i", "m", "Space", "C", "a", "r"] {
            press(&mut app, key);
        }
        press(&mut app, "Enter");
        assert_eq!(
            app.cmd.text, "vim Cargo.toml",
            "FAR's list takes it at once"
        );
    }

    /// A `[[column_set]]` on a view's own digit is what that view draws (SPEC 02 §3a): FAR's
    /// ten modes are the editable ones, and so are ours.
    #[test]
    fn the_users_own_columns_are_what_their_view_draws() {
        let mut app = app("far", "en");
        // What the full view draws without a set of one's own: a date column.
        let before = frame(&mut app, 100, 24);
        assert!(before.contains("Date"), "{before}");

        // Name, attributes and size — on `Ctrl+3`, which is the full view's digit.
        let (cfg, problems) = Config::parse(
            "test.toml",
            r#"
            [[column_set]]
            id = 3
            columns = [
              { type = "name", width = "*" },
              { type = "attrs", width = 5 },
              { type = "size", width = 12, fmt = "bytes" },
            ]
            "#,
        )
        .expect("it parses");
        assert!(problems.is_empty(), "{problems:?}");
        let mut warnings = Vec::new();
        app.column_sets = crate::colset::sets(&cfg, &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");

        let after = frame(&mut app, 100, 24);
        // The date is gone, the attributes are there, and the size is written out in full
        // rather than abbreviated — all three of them because the set said so.
        assert!(!after.contains("Date"), "{after}");
        assert!(after.contains("Attr"), "{after}");
        assert!(after.contains("412"), "{after}");
        // The other panel is in the same view, so it changed too; and the rules between the
        // columns moved with them, which is the thing that would break if the four readers of
        // the layout had drifted apart.
        assert_eq!(
            after.matches("Attr").count(),
            2,
            "both panels draw the set: {after}"
        );
    }

    /// The switches beside the columns in FAR's own dialog, which are per mode (SPEC 02 §3b).
    #[test]
    fn a_view_mode_can_ask_for_capitals_and_for_the_whole_screen() {
        let set = |src: &str| {
            let (cfg, problems) = Config::parse("test.toml", src).expect("it parses");
            assert!(problems.is_empty(), "{problems:?}");
            let mut warnings = Vec::new();
            let sets = crate::colset::sets(&cfg, &mut warnings);
            assert!(warnings.is_empty(), "{warnings:?}");
            sets
        };

        let mut app = app("far", "en");
        let plain = frame(&mut app, 100, 24);
        assert!(plain.contains("src"), "the fixture's directory: {plain}");
        assert!(plain.contains("Cargo"), "and its file: {plain}");

        // Directories in capitals, files in small letters, on the full view's own digit.
        app.column_sets = set(r#"
            [[column_set]]
            id = 3
            columns = [{ type = "name", width = "*" }, { type = "size", width = 10 }]
            folders_upper = true
            files_lower = true
            "#);
        let shouted = frame(&mut app, 100, 24);
        assert!(
            shouted.contains("SRC"),
            "a directory in capitals: {shouted}"
        );
        assert!(
            shouted.contains("cargo"),
            "a file in small letters: {shouted}"
        );
        assert!(!shouted.contains("Cargo"), "{shouted}");

        // And the whole screen, which is `Ctrl+F1` done by the mode rather than by a key.
        assert_eq!(app.hidden, None);
        app.column_sets = set(r#"
            [[column_set]]
            id = 3
            columns = [{ type = "name", width = "*" }]
            fullscreen = true
            "#);
        app.run(fcmd_cmd::Command::PanelViewSet(fcmd_core::ViewMode::Full));
        assert_eq!(app.hidden, Some(1), "the other panel gave up the screen");
        // Leaving the mode gives it back, because the switch belongs to the mode.
        app.run(fcmd_cmd::Command::PanelViewSet(fcmd_core::ViewMode::Brief));
        assert_eq!(app.hidden, None);
    }

    /// The quick search is drawn in the panel it is searching, standing on that panel's own
    /// bottom frame — not in the middle of the screen, over the rows one is reading.
    #[test]
    fn the_quick_search_box_sits_at_the_foot_of_its_own_panel() {
        let mut app = app("far", "en");
        app.cfg.panel.quick_search = fcmd_config::QuickSearch::Plain;
        press(&mut app, "s"); // the fixture's `src`
        assert!(matches!(app.mode, Mode::Prompt(_)), "{:?}", app.mode);

        let text = frame(&mut app, 100, 24);
        let rows: Vec<&str> = text.lines().collect();
        let row = rows
            .iter()
            .position(|line| line.contains("Quick search"))
            .unwrap_or_else(|| panic!("no search box in\n{text}"));
        // The panels end three rows above the key bar (command line, status, keys), so the
        // box standing on the bottom frame is near the foot of the screen rather than at
        // its middle.
        assert!(
            row > rows.len() / 2,
            "the box is at row {row} of {}",
            rows.len()
        );

        // And in the *left* panel, which is the one being searched: its frame is at column 0,
        // so the box starts a couple of columns in from there and never crosses the middle.
        let line = rows[row];
        let at = line.chars().position(|c| c == '┌' || c == '╔').unwrap();
        assert_eq!(at, 2, "two columns in from the panel's own frame: {line}");

        // The right panel is still whole: the two frames meet, so the box ended before it.
        assert!(
            line.contains("┤├") || line.contains("╝╚"),
            "the box ran into the other panel: {line}"
        );
    }

    /// The drive dialog belongs to its panel. FAR opens it five columns in from that panel's
    /// own frame, and so does this: a list that will send the *right* panel somewhere and
    /// opens in the middle of the screen is a list one has to read the title of.
    #[test]
    fn the_drive_dialog_opens_over_the_panel_it_is_about() {
        let edge = |side: fcmd_cmd::Side| -> usize {
            let mut app = app("far", "en");
            app.run(fcmd_cmd::Command::PanelDrive(side));
            let text = frame(&mut app, 100, 24);
            text.lines()
                .find(|line| line.contains("Change drive"))
                // By character, not by byte: the panel's own frame is three bytes a column.
                .and_then(|line| line.chars().position(|c| c == '┌' || c == '╔'))
                .unwrap_or_else(|| panic!("no dialog in\n{text}"))
        };
        let left = edge(fcmd_cmd::Side::Left);
        let right = edge(fcmd_cmd::Side::Right);
        // Five columns in from the left panel's frame, which is FAR's own arithmetic.
        assert_eq!(left, 5, "the left panel's dialog opened at {left}");
        // And further right for the right panel — how much further depends on how wide the
        // table came out, because a dialog is never drawn off the edge of the screen.
        assert!(
            right > left,
            "both dialogs opened at {left}: the right one is not over its own panel"
        );
    }

    /// The screenshots in `docs/shots`, drawn by the renderer itself from the same buffer the
    /// terminal would be given.
    ///
    /// Not run by `cargo test`: `cargo test -p fcmd --bin fcmd write_the_screenshots --
    /// --ignored` writes them. They are in the repository because a picture of a terminal
    /// program is the first thing anybody wants to see and the last thing anybody remembers to
    /// update; made this way, regenerating them is one command and they cannot show a version
    /// of the panel that no longer exists.
    ///
    /// Everything in them is fixed on purpose — the paths, the sizes, the dates — so that
    /// running this twice produces the same bytes and a commit shows a change in the drawing
    /// rather than a change in the clock.
    #[test]
    #[ignore]
    fn write_the_screenshots() {
        use std::time::{Duration, SystemTime};

        let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(|p| p.parent())
            .expect("the workspace is two directories up")
            .join("docs/shots");
        std::fs::create_dir_all(&out).expect("the shots directory");
        let write = |name: &str, buf: &Buffer| {
            let path = out.join(format!("{name}.svg"));
            std::fs::write(&path, fcmd_ui::to_svg(buf)).expect("the picture is written");
            eprintln!("{}", path.display());
        };
        let shot = |app: &mut App, w: u16, h: u16| -> Buffer {
            let mut buf = Buffer::new(w, h);
            draw(app, &mut buf);
            buf
        };
        // 1 August 2024, 09:41:00 UTC, so a date column says the same thing every time.
        let when = |days: u64| -> Option<SystemTime> {
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1_722_505_260 - days * 86_400))
        };
        let file = |name: &str, size: u64, days: u64| -> Entry {
            let mut e = entry(name, EntryKind::File, size);
            e.mtime = when(days);
            e
        };
        let dir = |name: &str, days: u64| -> Entry {
            let mut e = entry(name, EntryKind::Dir, 0);
            e.mtime = when(days);
            e
        };

        // ---- the panels, which is what the program is ------------------------------------
        let mut app = app("far", "en");
        app.cfg.panel.status_line = true;
        app.panels[0].cwd = VPath::local("/home/mb/fcmd/crates/fcmd-core/src");
        app.panels[1].cwd = VPath::local("/srv/www/releases");
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                dir("tests", 3),
                file("entry.rs", 4_812, 1),
                file("listing.rs", 11_904, 1),
                file("lib.rs", 2_240, 6),
                file("selection.rs", 3_671, 12),
                file("sort.rs", 9_458, 0),
                file("vpath.rs", 16_380, 2),
                file("cancel.rs", 980, 40),
                file("device.rs", 3_112, 5),
                file("filter.rs", 7_705, 9),
            ]),
        });
        app.handle_event(Event::Listed {
            panel: 1,
            generation: app.panels[1].generation,
            result: Ok(vec![
                Entry::parent_entry(),
                file("fcmd-linux-x86_64-0.2.0.tar.gz", 3_470_221, 0),
                file("fcmd-macos-aarch64-0.2.0.tar.gz", 3_118_904, 0),
                file("fcmd-windows-x86_64-0.2.0.zip", 3_902_118, 0),
                file("SHA256SUMS", 312, 0),
                file("CHANGELOG.md", 28_440, 0),
            ]),
        });
        // Two files marked, because the selection's colours are half of what a panel looks
        // like and an empty panel never shows them.
        for name in ["listing.rs", "sort.rs"] {
            let id = (0..app.panels[0].listing.len())
                .filter_map(|row| app.panels[0].listing.row(row))
                .find(|e| e.name_str() == name)
                .map(|e| e.id)
                .expect("the row is there");
            app.panels[0].selection.set(id, true);
        }
        app.panels[0].rebuild();
        app.panels[0].cursor = 6;
        write("panels", &shot(&mut app, 100, 30));

        // ---- the menu bar, pulled down -----------------------------------------------------
        press(&mut app, "F9");
        press(&mut app, "Right");
        press(&mut app, "Down");
        write("menu", &shot(&mut app, 100, 30));
        press(&mut app, "Esc");

        // ---- an object storage panel, and a listing that says it is not all of it ----------
        let mut objects = app_with_objects(&when(0));
        write("s3", &shot(&mut objects, 100, 30));

        // ---- the viewer, the hex view, the editor -----------------------------------------
        let work = std::env::temp_dir().join("fcmd-shots/project");
        let _ = std::fs::remove_dir_all(work.parent().expect("a parent"));
        std::fs::create_dir_all(&work).expect("the working directory");
        std::fs::write(work.join("sort.rs"), SAMPLE_SOURCE).expect("the sample");
        // A small ELF-looking file, for the hex view: the bytes are the point, not the format.
        let mut bytes: Vec<u8> = b"\x7fELF\x02\x01\x01\x00".to_vec();
        bytes.extend((0u8..=255).cycle().take(1016));
        std::fs::write(work.join("binary"), &bytes).expect("the sample");
        let mut app = app_on_disk(&work);
        app.theme = Theme::builtin("far").expect("the far theme");
        app.cfg.panel.status_line = true;

        app.panels[0].cursor = app.panels[0]
            .listing
            .row_of_name("sort.rs")
            .expect("the file is in the listing");
        press(&mut app, "F3");
        for _ in 0..3 {
            press(&mut app, "Down");
        }
        write("viewer", &shot(&mut app, 100, 30));
        press(&mut app, "F4"); // the viewer's hex mode
        write("hex", &shot(&mut app, 100, 30));
        press(&mut app, "Esc");

        press(&mut app, "F4"); // the editor
        for _ in 0..7 {
            press(&mut app, "Down");
        }
        for _ in 0..3 {
            press(&mut app, "Shift+Down");
        }
        write("editor", &shot(&mut app, 100, 30));
        press(&mut app, "Esc");
        press(&mut app, "Esc");

        // ---- the help, which is half prose and half the live keymap -----------------------
        press(&mut app, "F1");
        write("help", &shot(&mut app, 100, 30));

        let _ = std::fs::remove_dir_all(work.parent().expect("a parent"));
    }

    /// A panel on a bucket, with a listing the provider cut short — the one picture that shows
    /// what the object storage support actually looks like.
    fn app_with_objects(when: &Option<std::time::SystemTime>) -> App {
        let mut app = app("far", "en");
        app.cfg.panel.status_line = true;
        app.panels[0].cwd = VPath::parse("s3://logs/2024/08/");
        app.panels[1].cwd = VPath::local("/home/mb/incoming");
        let object = |name: &str, size: u64| -> Entry {
            let mut e = entry(name, EntryKind::File, size);
            e.mtime = *when;
            e.flags.insert(EntryFlags::READONLY);
            e
        };
        let prefix = |name: &str| entry(name, EntryKind::Dir, 0);
        let generation = app.panels[0].generation;
        app.handle_event(Event::Listed {
            panel: 0,
            generation,
            result: Ok(vec![
                Entry::parent_entry(),
                prefix("13"),
                prefix("14"),
                prefix("15"),
                object("access-2024-08-12.log.gz", 41_993_016),
                object("access-2024-08-13.log.gz", 38_117_402),
                object("errors-2024-08-13.jsonl", 2_884_120),
                object("manifest.json", 1_204),
            ]),
        });
        app.handle_event(Event::ListingCut {
            panel: 0,
            generation,
            note: "the first 100000 of this prefix, in 100 requests; there are more \
                   (s3.listing_cap)"
                .into(),
        });
        app.handle_event(Event::Listed {
            panel: 1,
            generation: app.panels[1].generation,
            result: Ok(vec![Entry::parent_entry()]),
        });
        app
    }

    /// What the viewer and the editor are shown reading. Its own source would do, but a file
    /// that is a picture of itself is a riddle rather than an illustration.
    const SAMPLE_SOURCE: &str = r#"/// Compare treating digit runs as numbers: `img2` before `img10`.
pub fn natural_cmp(a: &str, b: &str, case_sensitive: bool) -> Ordering {
    natural_cmp_bytes(a.as_bytes(), b.as_bytes(), case_sensitive)
}

/// The same comparison on the bytes of a name, which is how a listing holds it.
pub fn natural_cmp_bytes(a: &[u8], b: &[u8], case_sensitive: bool) -> Ordering {
    let common = a.len().min(b.len());
    let mut i = 0;
    while i < common && a[i] == b[i] {
        i += 1;
    }
    if i == common {
        // One name is the other with more after it, or they are the same name.
        return a.len().cmp(&b.len());
    }
    while i > 0 && a[i - 1].is_ascii_digit() {
        i -= 1;
    }
    // A continuation byte cannot begin a character, and both names are inside the same
    // character here, since every byte before this one was equal.
    while i > 0 && a[i] & 0xC0 == 0x80 {
        i -= 1;
    }
    natural_cmp_from(&a[i..], &b[i..], case_sensitive)
}
"#;

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
        // The full view keeps the extension in a column of its own, so the name is drawn in
        // two pieces: `Cargo` down the left of the field and `toml` against the size column.
        assert!(out.contains("Cargo") && out.contains("toml"), "{out}");
        assert!(!out.contains("Cargo.toml"), "{out}");
    }

    #[test]
    fn the_column_titles_are_in_the_language_the_program_is_running_in() {
        let mut app = app("commander", "it");
        app.cfg.panel.column_titles = true;
        project(&mut app);
        let out = frame(&mut app, 100, 28);
        assert!(out.contains("Nome"), "{out}");
        assert!(out.contains("Dimens."), "{out}");
        assert!(!out.contains("Size"), "{out}");
        // The detailed view's own headings come from the same catalogue.
        app.panels[0].view = fcmd_core::ViewMode::Detail;
        app.hidden = Some(1);
        let out = frame(&mut app, 120, 28);
        assert!(
            out.contains("Creazione") && out.contains("Accesso"),
            "{out}"
        );
    }

    #[test]
    fn with_the_panels_off_the_screen_is_the_output_and_the_rest_still_works() {
        let mut app = app("commander", "en");
        app.user_screen = vec!["$ cargo build".into(), "Finished in 3.70s".into()];
        app.panels_off = true;
        let out = frame(&mut app, 80, 24);
        assert!(!out.contains("Cargo.toml"), "the panels are gone: {out}");
        assert!(!out.contains("Cargo"), "including their names: {out}");
        assert!(out.contains("$ cargo build"), "{out}");
        assert!(out.contains("Finished in 3.70s"), "{out}");
        // The command line is where it was, with the path it had.
        assert!(out.contains("/home/mb/project>"), "{out}");
        // And so is the key bar: the keys all still work, which is the point.
        assert!(out.contains("Help") && out.contains("Quit"), "{out}");
    }

    #[test]
    fn the_status_line_describes_the_entry_under_the_cursor() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("Down".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Cargo.toml  412 B"), "{out}");
    }

    /// The plugin list with every example plugin loaded: twenty-four commands in a list that
    /// used to hold seven, on a terminal with room for twenty rows.
    #[test]
    fn a_long_pick_list_scrolls_instead_of_overflowing() {
        let mut app = app("commander", "en");
        app.plugin_commands = (0..24)
            .map(|i| crate::app::PluginCommand {
                plugin: format!("plugin-{i}"),
                id: format!("user.x.command-{i}"),
                label: format!("Command number {i} with a label of its own"),
                scope: fcmd_cmd::Scope::Panel,
                program: std::path::PathBuf::from("/bin/true"),
                needs: Vec::new(),
                timeout: None,
                confirm: None,
            })
            .collect();
        app.handle_input(Input::Key("F11".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Command number 0"), "{out}");
        for line in out.lines() {
            assert!(
                unicode_width::UnicodeWidthStr::width(line) <= 80,
                "a row ran off the screen: {line:?}"
            );
        }
        // The last rows are reached by moving: the cursor pulls the window down.
        for _ in 0..23 {
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        let bottom = frame(&mut app, 80, 24);
        assert!(bottom.contains("Command number 23"), "{bottom}");
        assert!(
            !bottom.contains("Command number 0 "),
            "the window did not move: {bottom}"
        );
        // And it says what is out of sight, exactly: the rows above the window plus the one
        // the label itself took.
        let above = bottom
            .lines()
            .find(|l| l.contains("more above"))
            .expect("a windowed list says there is more above")
            .to_string();
        let count: usize = above
            .split_whitespace()
            .find_map(|w| w.parse().ok())
            .expect("the label carries a number");
        let visible = (0..24)
            .filter(|i| bottom.contains(&format!("Command number {i} ")))
            .count();
        assert_eq!(
            count + visible,
            24,
            "{count} said hidden and {visible} on screen is not 24:\n{bottom}"
        );
    }

    /// The title bar of the two full-screen views is their own colour, not a mixture of the
    /// panel header's and the status bar's. The bug this guards is subtle on screen and plain
    /// here: black on cyan, which is what `Viewer.Status` is in FAR's own palette.
    #[test]
    fn the_viewer_and_the_editor_have_their_own_title_colour() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, "one\ntwo\nthree\n").unwrap();
        for theme in ["far", "commander", "tron"] {
            let mut app = app(theme, "en");
            let source = fcmd_view::source::FileSource::open(&file).unwrap();
            app.mode = crate::app::Mode::Viewer(Box::new(fcmd_view::Viewer::new(
                Box::new(source),
                VPath::local(&file),
            )));
            let mut buf = Buffer::new(80, 24);
            draw(&mut app, &mut buf);
            let top = buf.get(1, 0).unwrap().style;
            assert_eq!(
                (top.fg, top.bg),
                (
                    app.theme.color("viewer.status.fg"),
                    app.theme.color("viewer.status.bg")
                ),
                "{theme}: the viewer's title bar is not Viewer.Status"
            );
            // And not the pair it used to borrow, which in the FAR palette is light cyan on
            // whatever the status bar happens to be.
            assert_ne!(
                (top.fg, top.bg),
                (
                    app.theme.color("panel.header.fg"),
                    app.theme.color("status.bg")
                ),
                "{theme}: still the old mixture"
            );

            // The body is the view's own, which for these themes is the panel's own value —
            // the point is that it is asked for by the right name.
            let body = buf.get(1, 2).unwrap().style;
            assert_eq!(body.bg, app.theme.color("viewer.bg"), "{theme}: body");

            app.mode = crate::app::Mode::Editor(Box::new(
                fcmd_edit::Editor::open(&file).expect("it opens"),
            ));
            let mut buf = Buffer::new(80, 24);
            draw(&mut app, &mut buf);
            let top = buf.get(1, 0).unwrap().style;
            assert_eq!(
                (top.fg, top.bg),
                (
                    app.theme.color("editor.status.fg"),
                    app.theme.color("editor.status.bg")
                ),
                "{theme}: the editor's title bar is not Editor.Status"
            );
        }
    }

    /// The two full-screen views, with their colours named rather than described.
    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_viewer_colours() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("notes.txt");
        std::fs::write(&file, "one\ntwo\nthree\nfour\n").unwrap();
        for theme in ["far", "white"] {
            let mut app = app(theme, "en");
            let source = fcmd_view::source::FileSource::open(&file).unwrap();
            app.mode = crate::app::Mode::Viewer(Box::new(fcmd_view::Viewer::new(
                Box::new(source),
                VPath::local(&file),
            )));
            let mut buf = Buffer::new(72, 8);
            draw(&mut app, &mut buf);
            println!("=== {theme}: viewer");
            for y in 0..3 {
                let cell = buf.get(1, y).unwrap();
                println!("  row {y}: fg {:?} bg {:?}", cell.style.fg, cell.style.bg);
            }
            println!(
                "  Viewer.Status should be {:?} on {:?}",
                app.theme.color("viewer.status.fg"),
                app.theme.color("viewer.status.bg")
            );
            println!("{}", buf.to_text());
        }
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_long_pick() {
        let mut app = app("commander", "en");
        app.plugin_commands = (0..24)
            .map(|i| crate::app::PluginCommand {
                plugin: format!("plugin-{i}"),
                id: format!("user.x.command-{i}"),
                label: format!("Command number {i}"),
                scope: fcmd_cmd::Scope::Panel,
                program: std::path::PathBuf::from("/bin/true"),
                needs: Vec::new(),
                timeout: None,
                confirm: None,
            })
            .collect();
        app.handle_input(Input::Key("F11".parse().unwrap()));
        for _ in 0..12 {
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        println!("{}", frame(&mut app, 80, 20));
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
        // F1 is the page about where you are; the contents are one Esc behind it.
        let here = frame(&mut app, 80, 24);
        assert!(here.contains("The panels"), "{here}");
        assert!(here.contains("Moving about"), "{here}");
        // What is below the fold is in the page, which is what scrolling is for.
        let page = app.help_lines(crate::help::Chapter::Here(crate::help::Where::Panels), 76);
        assert!(
            page.iter()
                .any(|l| l.contains("F5") && l.contains("copy what is selected")),
            "{page:?}"
        );
        app.handle_input(Input::Key("Esc".parse().unwrap()));
        let index = frame(&mut app, 80, 24);
        // The frame names the program and the version, and the body is the chapter list.
        assert!(index.contains("FinalCommander"), "{index}");
        assert!(index.contains("What this is"), "{index}");
        assert!(index.contains("Every command"), "{index}");
        assert!(index.contains("Enter opens a chapter"), "{index}");

        // Into the keys chapter: its own title in the frame, and the bindings in the body.
        let keys_at = crate::help::chapters(&app.i18n)
            .iter()
            .position(|(c, _)| *c == crate::help::Chapter::Keys)
            .unwrap();
        for _ in 0..keys_at {
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let keys = frame(&mut app, 80, 24);
        assert!(keys.contains("Keys, as they are bound now"), "{keys}");
        // The scope is a heading with a rule under it, and the bindings are a block.
        assert!(keys.contains("global") && keys.contains("──────"), "{keys}");
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
            assert!(out.contains("src"), "theme {id} rendered nothing: {out}");
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
            lines[1].starts_with("║n"),
            "no sort letter in the corner: {}",
            lines[1]
        );
        // The footer no longer repeats the sort mode.
        assert!(!out.contains("[name]"), "{out}");

        app.panels[0].sort.reverse = true;
        app.panels[0].rebuild();
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].starts_with("║N"), "{}", lines[1]);

        // A filter puts a mark next to the letter, as FAR does.
        app.panels[0].filter = Some("rs".into());
        app.panels[0].rebuild();
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[1].starts_with("║N*"), "{}", lines[1]);
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

    /// The clock in the corner, on FAR's own spot: four characters over the frame, and the
    /// panels not a row shorter for it.
    #[test]
    fn the_clock_sits_in_the_corner_without_costing_a_row() {
        let mut app = app("commander", "en");
        let rows_without = {
            let mut buf = Buffer::new(80, 24);
            draw(&mut app, &mut buf);
            app.rows
        };
        assert!(
            !frame(&mut app, 80, 24)
                .lines()
                .next()
                .unwrap()
                .contains(':'),
            "there is no clock until it is asked for"
        );

        app.cfg.ui.clock = true;
        let out = frame(&mut app, 80, 24);
        let top = out.lines().next().unwrap().to_string();
        let clock = fcmd_ui::clock_now();
        assert!(top.contains(&clock), "`{clock}` is not in `{top}`");
        {
            let mut buf = Buffer::new(80, 24);
            draw(&mut app, &mut buf);
            assert_eq!(app.rows, rows_without, "it costs the panels nothing");
        }
    }

    /// FAR's *Always show menu bar*: the titles on the top row with no menu open, and the
    /// panels a row further down — which costs them a file, as the titles row does.
    #[test]
    fn the_menu_bar_can_be_kept_on_the_top_row() {
        let mut app = app("commander", "en");
        let rows_without = {
            let mut buf = Buffer::new(80, 24);
            draw(&mut app, &mut buf);
            app.rows
        };
        let out = frame(&mut app, 80, 24);
        assert!(
            !out.lines().next().unwrap().contains("Commands"),
            "the bar is not there until it is asked for: {}",
            out.lines().next().unwrap()
        );

        app.cfg.ui.menu_bar = true;
        let out = frame(&mut app, 80, 24);
        let lines: Vec<&str> = out.lines().collect();
        for title in ["Left", "Files", "Commands", "Options", "Right"] {
            assert!(
                lines[0].contains(title),
                "`{title}` is not on the bar: {}",
                lines[0]
            );
        }
        // The panel frame starts under it rather than being drawn over.
        assert!(lines[1].starts_with('╔'), "{}", lines[1]);
        assert_eq!(app.rows + 1, rows_without, "the row comes off the files");

        // `F9` from there opens a menu on the same row, and the bar is drawn once.
        app.handle_input(Input::Key("F9".parse().unwrap()));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        assert!(out.lines().next().unwrap().contains("Left"), "{out}");
    }

    #[test]
    fn the_menu_bar_shows_five_menus_and_pulls_one_down() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("F9".parse().unwrap()));
        // The bar alone first: nothing covers the panels until a menu is opened.
        assert!(!frame(&mut app, 80, 24).contains("Ctrl+1"));
        app.handle_input(Input::Key("Enter".parse().unwrap()));
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

    /// Every title on the bar wears the letter it answers to, in the bar's own hotkey ink —
    /// which is how one can see, without opening anything, that `F` is the files menu.
    #[test]
    fn the_bar_wears_the_letter_each_title_answers_to() {
        let mut app = app("far", "en");
        app.handle_input(Input::Key("F9".parse().unwrap()));
        let mut buf = Buffer::new(80, 24);
        draw(&mut app, &mut buf);
        let ink = app.theme.color("hmenu.hotkey.fg");
        let bar: String = (0..80)
            .filter_map(|x| buf.get(x, 0))
            .map(|cell| cell.ch)
            .collect();
        for letter in ['L', 'F', 'C', 'O', 'R'] {
            let marked = (0..80)
                .filter_map(|x| buf.get(x, 0))
                .any(|cell| cell.ch == letter && cell.style.fg == ink);
            assert!(marked, "`{letter}` is not marked on the bar: {bar}");
        }
        // And the letter is a key: `C` on the bar pulls the commands menu down.
        press(&mut app, "C");
        assert!(
            matches!(
                app.mode,
                crate::app::Mode::Menu {
                    menu: 2,
                    open: true,
                    ..
                }
            ),
            "{:?}",
            app.mode
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
                let Ok(key) = item.keys.parse::<fcmd_cmd::Key>() else {
                    continue;
                };
                if let Some(cmd) = app.keymap.resolve(&[fcmd_cmd::Scope::Panel], key) {
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
        app.run(fcmd_cmd::Command::OptConfirmations);
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Confirmations"), "{out}");
        assert!(out.contains("[x] Delete"), "{out}");
        assert!(out.contains("OK") && out.contains("Cancel"), "{out}");
    }

    #[test]
    fn the_language_list_is_drawn_with_the_current_language_marked() {
        let mut app = app("commander", "en");
        app.run(fcmd_cmd::Command::AppLanguages);
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Italiano"), "{out}");
        assert!(out.contains("√ English"), "{out}");
    }

    #[test]
    fn every_menu_and_dialog_fits_a_small_terminal_in_every_language() {
        for (tag, _) in fcmd_i18n::LANGUAGES {
            let mut app = app("commander", tag);
            app.handle_input(Input::Key("F9".parse().unwrap()));
            for menu in 0..5 {
                for size in [(80u16, 24u16), (60, 15), (40, 10)] {
                    app.mode = crate::app::Mode::Menu {
                        menu,
                        item: 0,
                        open: true,
                    };
                    let out = frame(&mut app, size.0, size.1);
                    assert_eq!(out.lines().count(), size.1 as usize, "{tag} menu {menu}");
                }
            }
            for cmd in [
                fcmd_cmd::Command::OptSystem,
                fcmd_cmd::Command::OptPanel,
                fcmd_cmd::Command::OptInterface,
                fcmd_cmd::Command::OptConfirmations,
                fcmd_cmd::Command::AppLanguages,
                fcmd_cmd::Command::AppThemes,
                fcmd_cmd::Command::PanelSortMenu,
                fcmd_cmd::Command::PanelHistory,
                fcmd_cmd::Command::PanelDrive(fcmd_cmd::Side::Left),
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
            paths: Paths::discover(Some(&dir.path().join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::builtin("far").unwrap(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        app.requests.clear();
        let entries = fcmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &fcmd_core::CancelToken::new(),
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

    /// Which of insert and overwrite is on is a thing one has to be able to *see*: in a
    /// binary the difference is whether the file keeps its length.
    #[test]
    fn the_hex_header_says_whether_typing_inserts_or_overwrites() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bytes.bin");
        std::fs::write(&path, b"ABCDEFGH").unwrap();
        let mut app = app("far", "en");
        app.open_hex(&path);
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("ovr"), "{out}");
        assert!(!out.contains("ins"), "{out}");

        app.handle_input(Input::Key("Ins".parse().unwrap()));
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("ins"), "{out}");
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
            matches!(viewer.mode, fcmd_view::Mode::Hex | fcmd_view::Mode::Dump),
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
        let source = fcmd_view::source::FileSource::open(&path).unwrap();
        let mut viewer = fcmd_view::Viewer::new(Box::new(source), VPath::local(&path));
        viewer.set_mode(fcmd_view::Mode::Hex);
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
            fcmd_cmd::Command::AppMenu,
            fcmd_cmd::Command::OptSystem,
            fcmd_cmd::Command::OptPanel,
            fcmd_cmd::Command::OptInterface,
            fcmd_cmd::Command::OptConfirmations,
            fcmd_cmd::Command::AppLanguages,
            fcmd_cmd::Command::PanelSortMenu,
            fcmd_cmd::Command::PanelDrive(fcmd_cmd::Side::Left),
        ] {
            let mut app = app("commander", "en");
            app.run(cmd);
            if let crate::app::Mode::Menu { .. } = app.mode {
                app.mode = crate::app::Mode::Menu {
                    menu: 1,
                    item: 0,
                    open: true,
                };
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
            ("fcmd.toml", 2_240, 20_738),
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
            app.handle_input(Input::Key(fcmd_cmd::Key::new(
                fcmd_cmd::KeyCode::Char(c),
                fcmd_cmd::Mods::NONE,
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
        type_line(&mut a, "cargo test -p fcmd-view structure");
        out.push((
            "a command typed, two files marked".into(),
            frame_buf(&mut a, W, H),
        ));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(fcmd_cmd::Command::PanelFilterQuick);
        type_line(&mut a, "*.toml,*.md|Cargo*");
        out.push(("quick filter".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.user_screen = vec![
            "fcmd $ cargo build --release".into(),
            "   Compiling fcmd-core v0.0.3".into(),
            "   Compiling fcmd-view v0.0.3".into(),
            "    Finished `release` profile [optimized] in 41.72s".into(),
            String::new(),
            "fcmd $ git log --oneline -3".into(),
            "0d7fbfe feat(core): one filter object".into(),
            "3b19a04 feat(view): a structure pane for binaries".into(),
            "a71c2e8 feat(cmd): the command line".into(),
        ];
        a.run(fcmd_cmd::Command::ShellOutputShow);
        out.push(("user screen".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(fcmd_cmd::Command::PanelViewSet(fcmd_core::ViewMode::Brief));
        out.push(("brief view".into(), frame_buf(&mut a, W, H)));

        let mut a = app("far", "en");
        project(&mut a);
        a.run(fcmd_cmd::Command::AppMenu);
        out.push(("menu bar".into(), frame_buf(&mut a, W, H)));

        for (title, cmd) in [
            ("system settings", fcmd_cmd::Command::OptSystem),
            ("panel settings", fcmd_cmd::Command::OptPanel),
            ("interface settings", fcmd_cmd::Command::OptInterface),
            ("confirmations", fcmd_cmd::Command::OptConfirmations),
            ("sort modes", fcmd_cmd::Command::PanelSortMenu),
            (
                "drives",
                fcmd_cmd::Command::PanelDrive(fcmd_cmd::Side::Left),
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
        if let Ok(source) = fcmd_view::source::FileSource::open(&path) {
            let mut viewer = fcmd_view::Viewer::new(Box::new(source), VPath::local(&path));
            viewer.set_mode(fcmd_view::Mode::Hex);
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
    /// Writes to `$FCMD_SHOTS` and does nothing when that is unset.
    #[test]
    #[ignore = "writes a file, on request"]
    fn shots_html() {
        let Ok(dest) = std::env::var("FCMD_SHOTS") else {
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

    fn css(color: fcmd_ui::Color, fallback: &str) -> String {
        // The sixteen console colours, so a theme that names them lands on the same values a
        // terminal would show.
        const ANSI: [&str; 16] = [
            "#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0",
            "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff",
        ];
        match color {
            fcmd_ui::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
            fcmd_ui::Color::Ansi(n) => ANSI[(n as usize).min(15)].to_string(),
            fcmd_ui::Color::Indexed(n) if (n as usize) < 16 => ANSI[n as usize].to_string(),
            fcmd_ui::Color::Indexed(_) | fcmd_ui::Color::Default => fallback.to_string(),
        }
    }

    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }

    /// `Ctrl+Left`/`Ctrl+Right` move the line between the panels; `Ctrl+Up`/`Ctrl+Down` take
    /// rows off them and give them to the user screen. Both are settings, so both are kept.
    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_search() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"needle in here\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F4");
        press(&mut app, "F7");
        println!("{}", frame(&mut app, 80, 16));
    }

    /// A README opens laid out: the marks are gone, the heading is ruled, the table is drawn,
    /// and every part of it is coloured as what it is rather than as the characters it is
    /// made of.
    #[test]
    fn a_markdown_file_opens_laid_out() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("read.md"),
            "\
# Title

Some **bold** text and a [link](https://example.com).

| name | size |
|------|-----:|
| a    |    1 |

- one
- two
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        let buf = frame_buf(&mut app, 60, 20);
        let text = buf.to_text();

        // The marks are gone and the layout is there.
        assert!(text.contains("Title"), "{text}");
        assert!(!text.contains("# Title"), "{text}");
        assert!(text.contains("═════"), "a heading is ruled: {text}");
        assert!(text.contains("┌──────┬──────┐"), "a table is drawn: {text}");
        assert!(text.contains("• one"), "a list is bulleted: {text}");
        assert!(!text.contains("**bold**"), "{text}");

        // And the status line says which way the file is being read.
        assert!(
            text.lines().next().is_some_and(|l| l.contains('m')),
            "the mode letter is on the status line: {text}"
        );

        // The heading is drawn in the heading colour, which is the whole point of rendering
        // it rather than printing it.
        let row = text
            .lines()
            .position(|l| l.trim_start().starts_with("Title"))
            .expect("the heading is on screen") as u16;
        let x = text.lines().nth(row as usize).unwrap().find('T').unwrap() as u16;
        assert_eq!(
            buf.get(x, row).unwrap().style.fg,
            app.theme.color("syntax.preproc"),
            "the heading takes the heading colour"
        );
        assert!(buf.get(x, row).unwrap().style.bold, "and it is bold");

        // `Shift+F2` shows the source, and again shows the layout.
        press(&mut app, "Shift+F2");
        let text = frame(&mut app, 60, 20);
        assert!(text.contains("# Title"), "the source has its marks: {text}");
        press(&mut app, "Shift+F2");
        let text = frame(&mut app, 60, 20);
        assert!(!text.contains("# Title"), "{text}");
    }

    /// Something that is not Markdown says so rather than leaving the key looking broken.
    #[test]
    fn asking_for_the_layout_of_something_else_says_no() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("t.txt"), b"# not markdown, just text\n").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        press(&mut app, "Shift+F2");
        assert!(
            app.status.to_lowercase().contains("markdown"),
            "{}",
            app.status
        );
    }

    /// A dialog with more rows than the screen has lines scrolls to the cursor. Without it
    /// the last rows of the attributes dialog — the times, and the box that takes the change
    /// into the folders — would be drawn below the bottom of the screen on a short terminal,
    /// where no key could reach them.
    #[test]
    fn a_dialog_taller_than_the_screen_scrolls_to_the_cursor() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sources")).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "Ctrl+A");

        // The rows the dialog asks something with, rather than the ones that say something:
        // the name at the top of this dialog is also a row of the panel behind it, and a test
        // that looked for it would find the panel's.
        let asked: Vec<String> = {
            let Mode::Form(form) = &app.mode else {
                panic!("the dialog is open: {:?}", app.mode)
            };
            form.rows()
                .iter()
                .filter(|r| !r.separator && !r.dim && !r.text.trim().is_empty())
                .map(|r| r.text.trim().to_string())
                .collect()
        };
        assert!(asked.len() > 4, "this dialog has rows: {asked:?}");
        let marked = |text: &str| text.contains('▲') || text.contains('▼');

        // How tall a screen this dialog needs depends on the platform — Windows has no mode
        // to offer and twelve rows fewer — and on how much of the screen the panels leave, so
        // the test finds the height that fits rather than assuming one.
        let tall = (8..60u16)
            .find(|h| !marked(&frame(&mut app, 76, *h)))
            .expect("some height fits the dialog");
        let text = frame(&mut app, 76, tall);
        for row in &asked {
            assert!(text.contains(row), "`{row}` should be on screen:\n{text}");
        }

        // Half that is not enough for it, and then the one thing that must hold is that the
        // row the cursor is on is on the screen — otherwise it could not be reached at all.
        let short = tall / 2;
        for (n, row) in asked.iter().enumerate() {
            {
                let Mode::Form(form) = &mut app.mode else {
                    unreachable!("the dialog is open")
                };
                form.cursor = form
                    .rows()
                    .iter()
                    .position(|r| r.text.trim() == row.as_str())
                    .expect("the row is there");
            }
            let text = frame(&mut app, 76, short);
            assert!(
                text.contains(row),
                "row {n} (`{row}`) is out of reach on a short screen:\n{text}"
            );
            assert!(marked(&text), "and the frame says there is more:\n{text}");
        }
    }

    /// The whole of `Ctrl+Alt+E`, against a real shell: it opens, what the program writes is
    /// on the screen with its colours, keys reach it, and the key takes you back leaving the
    /// program running.
    #[cfg(unix)]
    #[test]
    fn ctrl_alt_e_opens_a_real_terminal_and_brings_you_back() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_on_disk(dir.path());
        // The shell to run, rather than the user's: a test must not depend on anybody's
        // prompt. It echoes in colour, then waits, so the screen has something on it.
        app.cfg.term.shell =
            "/bin/sh -c 'printf \"\\033[32mgreen words\\033[0m\\n\"; read x; printf \"got %s\\n\" \"$x\"; sleep 5'"
                .to_string();

        press(&mut app, "Ctrl+Alt+E");
        assert!(
            matches!(app.mode, crate::app::Mode::Term(_)),
            "the terminal opens: {:?}",
            app.mode
        );

        // Wait for the shell to write, then draw: what it wrote is on the screen, in colour.
        let buf = wait_for_terminal(&mut app, "green words");
        let text = buf.to_text();
        assert!(text.contains("green words"), "{text}");
        let row = text
            .lines()
            .position(|l| l.contains("green words"))
            .expect("the row is there") as u16;
        let col = text.lines().nth(row as usize).unwrap().find('g').unwrap() as u16;
        assert_eq!(
            buf.get(col, row).unwrap().style.fg,
            fcmd_ui::Color::Ansi(2),
            "the colour the program asked for"
        );
        // The chrome says how to get out, and does not pretend the function keys are ours.
        assert!(
            text.lines()
                .next()
                .is_some_and(|l| l.contains("Ctrl+Alt+E")),
            "{text}"
        );
        assert!(!text.contains("1Help"), "no key bar in here: {text}");

        // Keys reach the program.
        for c in "hello".chars() {
            app.handle_input(Input::Key(fcmd_cmd::Key::char(c)));
        }
        press(&mut app, "Enter");
        let text = wait_for_terminal(&mut app, "got hello").to_text();
        assert!(text.contains("got hello"), "{text}");

        // And the key takes you back, leaving it running in the screens list.
        press(&mut app, "Ctrl+Alt+E");
        assert!(app.mode.is_normal(), "{:?}", app.mode);
        assert_eq!(app.background.len(), 1, "it is parked, not closed");
        press(&mut app, "F12");
        let text = frame(&mut app, 70, 20);
        assert!(text.contains("Term"), "the screens list has it: {text}");
        press(&mut app, "Esc");

        // Pressing it again brings that one back rather than starting a second shell.
        press(&mut app, "Ctrl+Alt+E");
        let text = frame(&mut app, 70, 20);
        assert!(text.contains("got hello"), "the same terminal: {text}");
        assert!(app.background.iter().all(|m| m.screen_title().is_none()));
    }

    /// Draw until the terminal's screen says `want`, so a test does not race the shell.
    #[cfg(unix)]
    fn wait_for_terminal(app: &mut App, want: &str) -> Buffer {
        for _ in 0..200 {
            let buf = frame_buf(app, 70, 20);
            if buf.to_text().contains(want) {
                return buf;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        frame_buf(app, 70, 20)
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_on_disk(dir.path());
        app.cfg.term.shell = "/bin/sh -c 'printf \"\\033[1;32muser@host\\033[0m:\\033[34m~\\033[0m$ ls\\n\"; printf \"\\033[1;34msrc\\033[0m  README.md\\n\"; printf \"\\033[1;32muser@host\\033[0m:\\033[34m~\\033[0m$ \"; sleep 5'".to_string();
        press(&mut app, "Ctrl+Alt+E");
        for _ in 0..40 {
            let text = frame(&mut app, 76, 16);
            if text.contains("README") {
                println!("{text}");
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        println!("{}", frame(&mut app, 76, 16));
    }

    #[test]
    fn the_tab_bar_appears_only_when_a_side_has_more_than_one_tab() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_on_disk(dir.path());
        app.panels[0].name = Some("first".into());
        let one = frame(&mut app, 76, 20);
        assert!(
            !one.contains("first"),
            "a side with one tab draws what it always drew: {one}"
        );
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].name = Some("second".into());
        app.panels[0].locked = true;
        let two = frame(&mut app, 76, 20);
        let top = two.lines().next().expect("a first row");
        assert!(top.contains("first"), "{two}");
        assert!(top.contains("\u{25aa}second"), "the lock is marked: {two}");
        // It is the panel's own row, not the screen's: the screen is as tall as it was, the
        // right panel's frame still starts at the top, and the left one's starts one lower.
        assert_eq!(one.lines().count(), two.lines().count());
        assert!(top.contains('\u{2554}'), "the right panel's corner: {top}");
        let under = two.lines().nth(1).expect("a second row");
        assert!(
            under.contains('\u{2554}'),
            "the left panel's corner: {under}"
        );
    }

    /// The table, built by hand rather than by a walk: what it draws is the point, and a real
    /// walk would put temporary directory names in a picture meant to be read.
    fn sync_view() -> crate::app::SyncView {
        use fcmd_jobs::sync::{Action, Pair, Verdict};
        let when = |secs: u64| {
            Some(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        };
        let facts = |size: u64, secs: u64| {
            Some(fcmd_jobs::Facts {
                kind: fcmd_core::EntryKind::File,
                size,
                mtime: when(secs),
                mode: None,
                readonly: false,
            })
        };
        let pair = |rel: &str, left, right, verdict, action| Pair {
            rel: rel.to_string(),
            left,
            right,
            verdict,
            action,
        };
        crate::app::SyncView {
            id: 1,
            left: fcmd_core::VPath::local("/home/mb/src/final"),
            right: fcmd_core::VPath::local("/media/stick/final"),
            pairs: vec![
                pair(
                    "README.md",
                    facts(4_210, 1_700_000_000),
                    facts(3_980, 1_699_000_000),
                    Verdict::Differ,
                    Action::Right,
                ),
                pair(
                    "Cargo.toml",
                    facts(812, 1_700_000_000),
                    facts(812, 1_700_000_000),
                    Verdict::Same,
                    Action::Skip,
                ),
                pair(
                    "notes.txt",
                    None,
                    facts(1_024, 1_700_500_000),
                    Verdict::OnlyRight,
                    Action::DeleteRight,
                ),
                pair(
                    "crates/fcmd-view/src/markdown.rs",
                    facts(31_002, 1_700_400_000),
                    None,
                    Verdict::OnlyLeft,
                    Action::Right,
                ),
                pair(
                    "docs/BACKLOG.md",
                    facts(22_140, 1_700_000_000),
                    facts(22_900, 1_700_000_000),
                    Verdict::Conflict,
                    Action::Skip,
                ),
            ],
            cursor: 0,
            top: 0,
            scanning: None,
            done: true,
            stopped: false,
            cancel: fcmd_core::CancelToken::new(),
        }
    }

    #[test]
    fn the_sync_table_shows_both_sides_and_which_way_each_row_goes() {
        let mut app = app("commander", "en");
        app.mode = crate::app::Mode::Sync(Box::new(sync_view()));
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("Synchronise"), "{out}");
        // Both roots, so there is never a doubt which side is which.
        assert!(out.contains("/home/mb/src/final"), "{out}");
        assert!(out.contains("/media/stick/final"), "{out}");
        // The arrow is what says which way, and the row that does nothing says so with `=`.
        let readme = out
            .lines()
            .find(|l| l.contains("README.md"))
            .expect("the README row");
        assert!(readme.contains('\u{2192}'), "{readme}");
        let cargo = out
            .lines()
            .find(|l| l.contains("Cargo.toml"))
            .expect("the Cargo row");
        assert!(cargo.contains('='), "{cargo}");
        // A row with only one side has nothing in the other side's columns.
        let only = out
            .lines()
            .find(|l| l.contains("notes.txt"))
            .expect("the notes row");
        assert!(only.contains('\u{2612}'), "taking it out is marked: {only}");
        // The foot counts what will happen and what it will move.
        assert!(out.contains("to copy"), "{out}");
        assert!(out.contains("F2"), "and says which key does it: {out}");
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_sync() {
        let mut app = app("commander", "en");
        app.mode = crate::app::Mode::Sync(Box::new(sync_view()));
        println!("{}", frame(&mut app, 100, 18));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_palette() {
        let mut app = app("commander", "en");
        app.mode = crate::app::Mode::Palette(Box::default());
        println!("{}", frame(&mut app, 76, 24));
    }

    #[test]
    fn the_palette_editor_shows_every_colour_with_its_value_and_two_swatches() {
        let mut app = app("commander", "en");
        app.mode = crate::app::Mode::Palette(Box::default());
        let out = frame(&mut app, 76, 24);
        assert!(out.contains("panel.bg"), "{out}");
        assert!(out.contains("panel.fg"), "{out}");
        // The colour as it is written in a theme file, and the two samples.
        let value = app.theme.color("panel.fg").to_string();
        assert!(
            out.contains(&value),
            "the value {value} is not shown: {out}"
        );
        assert!(out.contains("Abc"), "no swatch: {out}");
        assert!(
            out.contains("F2"),
            "the foot does not say how to save: {out}"
        );
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_checksum() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("release.tar.gz"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "Ins");
        app.run(fcmd_cmd::Command::FileChecksum);
        println!("{}", frame(&mut app, 76, 20));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_bookmarks() {
        let dir = tempfile::tempdir().unwrap();
        let mut app = app_on_disk(dir.path());
        app.bookmarks = vec![
            fcmd_config::Bookmark {
                path: "/home/mb/src/final".into(),
                name: "work".into(),
                digit: Some(1),
            },
            fcmd_config::Bookmark {
                path: "/var/log".into(),
                name: "logs".into(),
                digit: Some(2),
            },
            fcmd_config::Bookmark {
                path: "sftp://mb@build.example.net/srv".into(),
                name: String::new(),
                digit: None,
            },
        ];
        press(&mut app, "Ctrl+D");
        println!("{}", frame(&mut app, 76, 20));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_tabs() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("report.txt"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].name = Some("src".into());
        press(&mut app, "Ctrl+Alt+T");
        app.panels[0].name = Some("logs".into());
        app.panels[0].locked = true;
        println!("{}", frame(&mut app, 76, 20));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_copy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("report.txt"), b"x").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F5");
        println!("{}", frame(&mut app, 76, 26));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_attributes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("report.txt"), b"x").unwrap();
        std::fs::create_dir(dir.path().join("sources")).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "Ctrl+A");
        println!("{}", frame(&mut app, 76, 30));
        println!("--- and on a short terminal ---");
        println!("{}", frame(&mut app, 76, 20));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_markdown() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("read.md"),
            "\
# FinalCommander

One file. Every system. **Everything** in it.

## What it replaces

| what | instead of |
|------|-----------:|
| panels | two of them |
| viewer | `less` |

> It is the same program on all three systems.

- a bulleted list
  - nested, with its own bullet
- [ ] and a box to tick

```rust
fn main() { println!(\"hello\"); }
```

See the [docs](https://example.com/docs) for more.
",
        )
        .unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "F3");
        println!("{}", frame(&mut app, 76, 30));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_sockets() {
        let mut app = super::tests::app("far", "en");
        project(&mut app);
        app.run(fcmd_cmd::Command::PanelDrive(fcmd_cmd::Side::Left));
        app.handle_input(Input::Key("2".parse().unwrap()));
        println!("{}", frame(&mut app, 110, 24));
        app.panels[0].cursor = 0;
        app.handle_input(Input::Key("F3".parse().unwrap()));
        println!("{}", frame(&mut app, 110, 24));
    }

    #[test]
    #[ignore = "a look at the screen, not an assertion"]
    fn eyeball_drives() {
        let mut app = super::tests::app("far", "en");
        project(&mut app);
        app.connected.push("sftp://mb@host:22".into());
        app.run(fcmd_cmd::Command::PanelDrive(fcmd_cmd::Side::Left));
        println!("{}", frame(&mut app, 100, 28));
    }

    /// The editor colours what it recognises, and a buffer that is being edited keeps up:
    /// close a comment and the lines below it stop being one, in the same frame.
    #[test]
    fn the_editor_colours_the_syntax_and_follows_the_edits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("page.html");
        std::fs::write(
            &file,
            "<p class=\"x\">hello</p>\n<!-- a note\nstill a note\n",
        )
        .unwrap();
        let mut app = super::tests::app("far", "en");
        app.mode =
            crate::app::Mode::Editor(Box::new(fcmd_edit::Editor::open(&file).expect("it opens")));
        let buf = frame_buf(&mut app, 60, 12);
        let colour_at = |buf: &Buffer, x: u16, y: u16| buf.get(x, y).unwrap().style.fg;

        // `<p class="x">` — the tag's name, the attribute and the value are each their own.
        assert_eq!(
            colour_at(&buf, 1, 1),
            app.theme.color("syntax.type"),
            "the tag"
        );
        assert_eq!(
            colour_at(&buf, 3, 1),
            app.theme.color("syntax.attr"),
            "the attribute"
        );
        assert_eq!(
            colour_at(&buf, 9, 1),
            app.theme.color("syntax.string"),
            "the value"
        );
        // The text between the tags is the editor's own colour, not a syntax one.
        assert_eq!(
            colour_at(&buf, 13, 1),
            app.theme.color("editor.fg"),
            "the text"
        );
        // The comment opened on the second line is still one on the third.
        let comment = app.theme.color("syntax.comment");
        assert_eq!(colour_at(&buf, 0, 2), comment);
        assert_eq!(colour_at(&buf, 0, 3), comment, "the comment did not carry");

        // Close the comment on line two, and line three stops being a comment at once.
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.cursor = fcmd_edit::Position::new(1, editor.buffer.line_chars(1));
            editor.buffer.insert(editor.cursor, " -->").unwrap();
        }
        let buf = frame_buf(&mut app, 60, 12);
        assert_eq!(
            colour_at(&buf, 0, 2),
            comment,
            "line two is still a comment"
        );
        assert_ne!(
            colour_at(&buf, 0, 3),
            comment,
            "line three is still coloured as a comment that has been closed"
        );

        // And the setting turns the whole thing off.
        app.cfg.ui.syntax = false;
        let buf = frame_buf(&mut app, 60, 12);
        assert_eq!(colour_at(&buf, 1, 1), app.theme.color("editor.fg"));
    }

    #[test]
    fn the_panels_can_be_resized_and_what_they_give_up_shows_the_user_screen() {
        let width_of_left = |app: &mut App| {
            let out = frame(app, 100, 28);
            out.lines()
                .nth(1)
                .unwrap()
                .char_indices()
                .filter(|(_, c)| *c == '║')
                .map(|(i, _)| i)
                .nth(1)
                .expect("two frames on the row")
        };

        let mut app = app("far", "en");
        project(&mut app);
        let even = width_of_left(&mut app);
        for _ in 0..10 {
            app.handle_input(Input::Key("Ctrl+Right".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.split, 60);
        let wider = width_of_left(&mut app);
        assert!(
            wider > even,
            "the left panel did not grow: {even} -> {wider}"
        );

        for _ in 0..30 {
            app.handle_input(Input::Key("Ctrl+Left".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.split, 30);
        let narrower = width_of_left(&mut app);
        assert!(narrower < even, "the left panel did not shrink");

        // Neither side can be squeezed out of existence.
        for _ in 0..100 {
            app.handle_input(Input::Key("Ctrl+Left".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.split, 10);
        assert!(width_of_left(&mut app) >= 10);

        // And the height. `Ctrl+Up` brings the panels' bottom edge *up*: they get shorter and
        // the output underneath them gets the rows. Four presses, four rows of user screen.
        let mut app = super::tests::app("far", "en");
        project(&mut app);
        app.user_screen = vec!["$ cargo test".into(), "   Compiling fcmd".into()];
        let before = frame(&mut app, 100, 28);
        assert!(!before.contains("Compiling fcmd"), "{before}");
        for _ in 0..4 {
            app.handle_input(Input::Key("Ctrl+Up".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.height_cut, 4);
        let after = frame(&mut app, 100, 28);
        assert!(
            after.contains("Compiling fcmd"),
            "the rows the panels gave up show nothing: {after}"
        );
        // The command line and the status bar are still where they were.
        assert_eq!(
            before.lines().count(),
            after.lines().count(),
            "the screen changed height"
        );
        assert_eq!(
            before.lines().next_back(),
            after.lines().next_back(),
            "the key bar moved"
        );

        for _ in 0..4 {
            app.handle_input(Input::Key("Ctrl+Down".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.height_cut, 0);
        assert_eq!(frame(&mut app, 100, 28), before);
        // And it does not run past the bottom of the screen: the panels cannot be given rows
        // they never gave up.
        for _ in 0..4 {
            app.handle_input(Input::Key("Ctrl+Down".parse().unwrap()));
        }
        assert_eq!(app.cfg.panel.height_cut, 0);
    }

    #[test]
    fn one_panel_can_take_the_whole_screen() {
        let mut app = app("far", "en");
        project(&mut app);
        app.run(fcmd_cmd::Command::PanelToggle(fcmd_cmd::Side::Left));
        assert_eq!(app.hidden, Some(0));
        assert_eq!(app.active, 1, "the keys followed the screen");
        let out = frame(&mut app, 100, 28);
        let top = out.lines().next().unwrap();
        assert_eq!(
            top.matches('╔').count(),
            1,
            "two panels are still drawn: {top}"
        );
        assert!(top.contains("/tmp"), "{top}");
        // The same key brings it back.
        app.run(fcmd_cmd::Command::PanelToggle(fcmd_cmd::Side::Left));
        assert_eq!(app.hidden, None);
        let out = frame(&mut app, 100, 28);
        assert_eq!(out.lines().next().unwrap().matches('╔').count(), 2);
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
            disk: Some(fcmd_platform::Disk {
                total: 64 * 1024 * 1024 * 1024,
                free: 12 * 1024 * 1024 * 1024,
                cluster: 4096,
            }),
        });
        let out = frame(&mut app, 100, 28);
        // FAR's own layout: what is in the panel, then a piece of double rule, then what
        // is left on the device.
        assert!(out.contains("═══ 12.0 G"), "{out}");
        app.cfg.panel.free_space = false;
        let out = frame(&mut app, 100, 28);
        assert!(!out.contains("12.0 G"), "{out}");
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
        app.run(fcmd_cmd::Command::PanelScrollNames(1));
        app.run(fcmd_cmd::Command::PanelScrollNames(1));
        let after = frame(&mut app, 100, 12);
        assert!(after.contains("name-longer"), "{after}");
        assert!(!after.contains("a-name-longer"), "it did not move: {after}");
        // Going back past the start stops at the start rather than wrapping.
        for _ in 0..5 {
            app.run(fcmd_cmd::Command::PanelScrollNames(-1));
        }
        assert_eq!(app.panels[0].name_scroll, 0);
    }

    #[test]
    fn every_language_renders_the_key_bar() {
        for (tag, _) in fcmd_i18n::LANGUAGES {
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
        // The wastebasket is on by default, so that is what the dialog is about.
        assert!(out.contains("wastebasket"), "{out}");
        assert!(out.contains("src"), "{out}");
        // The buttons name the verb, as FAR's do — `{ Move }`, not `{ Yes }` — so a reader
        // who glances at the button knows what is about to happen.
        assert!(out.contains("Move"), "{out}");
        assert!(out.contains("Cancel"), "{out}");
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
            progress: fcmd_jobs::Progress {
                files_done: 3,
                files_total: 10,
                bytes_done: 1024,
                bytes_total: 4096,
                file_bytes_done: 512,
                file_bytes_total: 1024,
                current: Some(VPath::local("/home/mb/project/src/main.rs")),
                ..fcmd_jobs::Progress::default()
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
            question: fcmd_jobs::Question::Conflict {
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
        for (tag, _) in fcmd_i18n::LANGUAGES {
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
            paths: Paths::discover(Some(&dir.path().join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        app.requests.clear();
        let entries = fcmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &fcmd_core::CancelToken::new(),
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
            lines[11].contains("Wrap") && lines[11].contains("Dump"),
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
            .filter(|(x, y)| {
                let cell = buf.get(*x, *y).unwrap();
                cell.style.fg == app.theme.color("viewer.selected.fg")
                    && cell.style.bg == app.theme.color("viewer.selected.bg")
            })
            .count();
        assert!(highlighted >= 3, "the search hit is not highlighted");
    }

    #[test]
    fn the_viewer_gives_a_row_back_to_the_content_for_every_piece_of_chrome_taken_away() {
        let dir = tempfile::tempdir().unwrap();
        // Twenty numbered lines, so which line is on which row says what moved.
        let body: String = (1..=20).map(|i| format!("line {i}\n")).collect();
        std::fs::write(dir.path().join("readme.txt"), body.as_bytes()).unwrap();
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.path().join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        app.requests.clear();
        let entries = fcmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &fcmd_core::CancelToken::new(),
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

        // As it opens: the name on top, the key bar at the foot, no bar down the side.
        let out = frame(&mut app, 60, 12);
        let lines: Vec<&str> = out.lines().collect();
        assert!(lines[0].contains("readme.txt"), "{out}");
        assert!(lines[1].contains("line 1"), "{out}");
        // `F4` is a wheel now, and the bar says where the next turn lands: a text file's
        // next way of being read is the dump.
        assert!(lines[11].contains("Dump"), "{out}");
        assert!(
            !lines[5].contains('░'),
            "a scrollbar nobody asked for: {out}"
        );

        // `Ctrl+Shift+B` takes the top line away, and the first line of the file moves up
        // into the row it was using.
        app.handle_input(Input::Key("Ctrl+Shift+B".parse().unwrap()));
        let out = frame(&mut app, 60, 12);
        let lines: Vec<&str> = out.lines().collect();
        assert!(!lines[0].contains("readme.txt"), "{out}");
        assert!(lines[0].contains("line 1"), "{out}");

        // `Ctrl+B` takes the key bar away, so the foot of the screen is the file too.
        app.handle_input(Input::Key("Ctrl+B".parse().unwrap()));
        let out = frame(&mut app, 60, 12);
        let lines: Vec<&str> = out.lines().collect();
        assert!(!lines[11].contains("Hex"), "{out}");
        assert!(lines[11].contains("line 12"), "{out}");

        // `Ctrl+S` puts a bar down the right edge: a track, and one block saying where in the
        // file we are. At the top of a file, that block is the first row of the track.
        app.handle_input(Input::Key("Ctrl+S".parse().unwrap()));
        let out = frame(&mut app, 60, 12);
        let lines: Vec<&str> = out.lines().collect();
        let column: String = lines.iter().filter_map(|l| l.chars().nth(59)).collect();
        assert!(column.starts_with('█'), "{out}");
        assert!(column.contains('░'), "{out}");
        assert!(
            lines[0].contains("line 1"),
            "and the text is still there: {out}"
        );
    }

    #[test]
    fn the_viewer_renders_a_binary_as_hex_without_overflowing() {
        let dir = tempfile::tempdir().unwrap();
        let mut bytes = vec![0x7f, b'E', b'L', b'F'];
        bytes.extend((0..200u16).map(|i| (i % 251) as u8));
        std::fs::write(dir.path().join("prog.bin"), &bytes).unwrap();
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(&dir.path().join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir.path()),
            right: VPath::local(dir.path()),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        app.requests.clear();
        let entries = fcmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &fcmd_core::CancelToken::new(),
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
        assert_eq!(app.mode.viewer().unwrap().mode, fcmd_view::Mode::Dump);
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
        app.run(fcmd_cmd::Command::PanelViewSet(fcmd_core::ViewMode::Brief));
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
            paths: Paths::discover(Some(&dir.join("fcmd.toml")), None),
            i18n: I18n::new("en"),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::default_theme(),
            left: VPath::local(dir),
            right: VPath::local(dir),
            sites: Vec::new(),
            plugin_commands: Vec::new(),
            highlight: fcmd_ui::Highlight::far_default(),
            sort_groups: Default::default(),
            column_sets: Vec::new(),
        });
        app.requests.clear();
        let entries =
            fcmd_vfs::local::list_blocking(&VPath::local(dir), &fcmd_core::CancelToken::new())
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
            .filter(|(x, y)| {
                buf.get(*x, *y).unwrap().style.bg == app.theme.color("editor.selected.bg")
            })
            .count();
        assert!(
            highlighted >= 12,
            "the block is not highlighted ({highlighted} cells)"
        );
        // Not the panel cursor's colour, which is what it used to borrow: in FAR's palette a
        // marked block is `Editor.Text.Selected` and the two are different entries.
        assert_ne!(
            app.theme.color("editor.selected.bg"),
            app.theme.color("cursor.bg"),
            "this theme cannot tell the two apart, so the test proves nothing"
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
        assert!(cursor.is_some(), "the dialog has no cursor");
        assert!(out.contains("Search"), "{out}");
        assert!(
            out.contains("Whole words"),
            "the dialog is not FAR's: {out}"
        );
        // The editor is still behind it: the dialog covers the middle of the screen, so what
        // is left of the line is what shows at its left.
        assert!(out.contains("needle"), "the editor is gone: {out}");
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
            for (tag, _) in fcmd_i18n::LANGUAGES {
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
        let mut site = fcmd_net::Site::parse_url("sftp://mb@jump.example:2222/srv").unwrap();
        site.id = "jump".into();
        site.group = Some("work".into());
        with_sites.sites = vec![site.clone()];
        with_sites.connected = vec![site.key()];
        with_sites.handle_input(Input::Key("Ctrl+Shift+N".parse().unwrap()));

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
        empty.handle_input(Input::Key("Ctrl+Shift+N".parse().unwrap()));
        let out = frame(&mut empty, 80, 24);
        assert!(out.contains("No sites"), "{out}");
    }

    #[test]
    fn a_password_prompt_shows_stars_and_not_the_password() {
        let mut app = app("commander", "en");
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(fcmd_net::Site::parse_url("sftp://mb@host:22/").unwrap()),
            outcome: crate::app::ConnectOutcome::NeedsSecret(fcmd_net::SecretKind::Password),
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

        // A folder has no bytes to preview, so the frame fills with what the device under it
        // has to say — the same block the information panel draws, as FAR does.
        app.handle_input(Input::Key("Home".parse().unwrap()));
        let out = frame(&mut app, 100, 14);
        assert!(out.contains("Folder"), "{out}");
        assert!(out.contains("Device") || out.contains("Memory"), "{out}");
        assert!(out.contains("Holds"), "the directory's own totals: {out}");

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
            disk: Some(fcmd_platform::Disk {
                total: 256 * 1024 * 1024,
                free: 42 * 1024 * 1024,
                cluster: 4096,
            }),
        });
        app.panels[0].cursor_to(2);
        app.active = 0;

        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
        assert_eq!(app.panels[1].content, crate::panel::Content::Info);
        let out = frame(&mut app, 100, 16);
        assert!(out.contains("Info"), "{out}");
        assert!(out.contains("/home/mb/project"), "{out}");
        assert!(out.contains("a directory on this machine"), "{out}");
        // The device as FAR's own panel has it — what it holds, what is left, how big its
        // allocation unit is — then the directory's totals, what the cluster size costs on
        // them, and the row under the cursor.
        assert!(out.contains("256 M, 42.0 M free, 4.00 K clusters"), "{out}");
        assert!(out.contains("1 files, 1 directories"), "{out}");
        assert!(out.contains("Slack"), "{out}");
        assert!(
            out.contains("3.00 K"),
            "a 1 K file in a 4 K cluster wastes 3 K: {out}"
        );
        assert!(out.contains("a.txt"), "{out}");

        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
        assert_eq!(app.panels[1].content, crate::panel::Content::Files);
    }

    /// The other half of "where am I": what the directory is sitting on, and what the
    /// repository it is in is doing. Both are facts the panel is handed, so this drives the
    /// event the worker sends rather than the machine this runs on.
    #[test]
    fn the_information_panel_says_the_filesystem_and_the_repository() {
        let mut app = app("commander", "en");
        app.handle_event(Event::Listed {
            panel: 0,
            generation: app.panels[0].generation,
            result: Ok(vec![Entry::parent_entry()]),
        });
        app.handle_event(Event::Place {
            panel: 0,
            mount: Some(fcmd_platform::Mount {
                source: "/dev/vda1".into(),
                target: "/".into(),
                fs: "ext4".into(),
                options: vec!["rw".into(), "noatime".into()],
            }),
            git: Some(Box::new(fcmd_core::git::Repo {
                root: "/home/mb/project".into(),
                git_dir: "/home/mb/project/.git".into(),
                branch: Some("main".into()),
                head: Some("3a7f2e1".into()),
                state: None,
                index: Some(fcmd_core::git::Index {
                    tracked: 42,
                    differing: 3,
                }),
            })),
        });
        app.active = 0;
        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));

        let out = frame(&mut app, 150, 18);
        assert!(out.contains("ext4 (rw,noatime)"), "{out}");
        assert!(out.contains("/dev/vda1 on /"), "{out}");
        assert!(out.contains("main · 3a7f2e1"), "{out}");
        assert!(out.contains("3 of 42 differ from the index"), "{out}");

        // A repository halfway through something says which something, and a working tree
        // that matches its index says so instead of a number.
        app.handle_event(Event::Place {
            panel: 0,
            mount: None,
            git: Some(Box::new(fcmd_core::git::Repo {
                root: "/home/mb/project".into(),
                git_dir: "/home/mb/project/.git".into(),
                branch: Some("main".into()),
                head: Some("3a7f2e1".into()),
                state: Some(fcmd_core::git::State::Rebase),
                index: Some(fcmd_core::git::Index {
                    tracked: 42,
                    differing: 0,
                }),
            })),
        });
        let out = frame(&mut app, 150, 18);
        assert!(out.contains("rebasing"), "{out}");
        assert!(out.contains("as the index has it"), "{out}");
        // And with no mount to report there is no line claiming one.
        assert!(!out.contains("ext4"), "{out}");
    }

    /// Nobody pays for a git index to draw a list of files: the facts are only asked for
    /// while a panel is showing them.
    #[test]
    fn where_a_directory_is_is_only_read_while_something_shows_it() {
        let mut app = app("commander", "en");
        app.active = 0;
        app.take_requests();
        // `Ctrl+R` is a read of the panel, which is the request this is about.
        app.handle_input(Input::Key("Ctrl+R".parse().unwrap()));
        let asked = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                crate::app::Request::Read { facts, .. } => Some(facts),
                _ => None,
            })
            .expect("a refresh reads the panel");
        assert!(!asked, "a file list asks for no facts");

        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
        app.take_requests();
        app.handle_input(Input::Key("Ctrl+R".parse().unwrap()));
        let asked = app
            .take_requests()
            .into_iter()
            .find_map(|r| match r {
                crate::app::Request::Read { facts, .. } => Some(facts),
                _ => None,
            })
            .expect("a refresh reads the panel");
        assert!(asked, "the information panel asks");
    }

    #[test]
    fn a_remote_panel_shows_the_url_in_its_header() {
        let mut app = app("commander", "en");
        let site = fcmd_net::Site::parse_url("sftp://mb@host.example:22/srv/www").unwrap();
        app.handle_event(Event::Connected {
            panel: 0,
            site: Box::new(site.clone()),
            outcome: crate::app::ConnectOutcome::Open {
                start: site.start(),
                connected: vec![site.key()],
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
        // Split into the name field and the extension column, as the full view draws it.
        assert!(out.contains("index") && out.contains("html"), "{out}");
    }
}
