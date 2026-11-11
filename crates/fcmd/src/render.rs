use fcmd_ui::{widgets, Buffer, Rect, Style};

use crate::app::{App, Mode, PromptKind};
use crate::hit::{Layout, ListArea, PanelArea};
use crate::panel as fcmd_panel;

/// Draw a whole frame. Returns where the text cursor should sit, if anywhere.
pub fn draw(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    // A picture drawn as real pixels lies on top of the cells, so it can only be placed
    // where nothing else was drawn. Whether anything was is known when the frame is whole:
    // if a dialog or a menu crossed a picture, the frame is drawn once more with every
    // picture in half blocks, which sit under whatever is put over them (SPEC 05 §1.14a).
    app.pixels.begin(false);
    let mut cursor = draw_frame(app, buf);
    if !app.pixels.settle(buf) {
        app.pixels.begin(true);
        cursor = draw_frame(app, buf);
    }
    for ask in app.pixels.take_asks() {
        app.requests.push(crate::app::Request::Pixels(ask));
    }
    cursor
}

/// One whole frame into the buffer.
fn draw_frame(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
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
    // And its questions are drawn over it: a replacement asked about one match at a time
    // is a question about the bytes behind the dialog.
    if let Some(mut editor) = app.hex_under_dialog() {
        draw_hex(app, &mut editor, buf);
        app.return_hex(editor);
        return draw_dialog_over_screen(app, buf);
    }

    // The editor takes the whole screen; its prompts and confirmations draw on top of it.
    // Not when a whole screen has been opened over a parked editor — the comparison with the
    // disk, a proposed rewrite of its block, the preview of what a question sends: those
    // are drawn below as what they are, and the editor comes back when they close.
    let screen_over_editor = app.mode.diff().is_some() || app.mode.viewer().is_some();
    if (app.mode.editor().is_some() || app.editor_parked_for_render()) && !screen_over_editor {
        if let Some((mut editor, parked)) = app.editor_for_render() {
            let cursor = draw_editor(app, &mut editor, buf);
            app.return_editor(editor, parked);
            if !parked {
                return cursor;
            }
            // A prompt or confirmation is open over the editor.
            return draw_dialog_over_screen(app, buf);
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
    // A prompt with `$_` in it is more than one line (FAR's #970): the lines before the last
    // go on the rows above the command line, taken from the panels, at most three of them.
    let prompt_full = if cmd_h == 1 {
        app.prompt()
    } else {
        String::new()
    };
    let prompt_lines: Vec<&str> = prompt_full.split('\n').collect();
    let prompt_extra = (prompt_lines.len().saturating_sub(1)).min(3) as u16;
    let chrome = keybar_h + status_h + cmd_h + menu_h + prompt_extra;
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

    // A side with more than one tab gives its top row to the tab bar (SPEC 02 §8). A bar
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
        "col-git",
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
        git: &titles[13],
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
        // The AI conversation is read, not walked: the panel is the conversation itself, as
        // it goes, the way the quick view shows a file.
        if app.panels[i].temp_origin() == Some(":aichat") && app.ai_session.is_some() {
            draw_ai_chat(app, i, *area, buf);
            continue;
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
            git: &panel.git_status,
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
        // The line an output search found is drawn the way the cursor row is, so the eye
        // goes to it on a screen of a build's worth of text.
        let hit = app.output_hit.as_ref().map(|(at, _)| *at);
        let marked = app.theme.fg_on("cursor.fg", "cursor.bg");
        for (i, line) in app.user_screen[start..end].iter().enumerate() {
            let text = fcmd_ui::truncate_cols(line, w as usize);
            let y = area.y + i as u16;
            if hit == Some(start + i) {
                buf.fill(Rect::new(0, y, w, 1), ' ', marked);
                buf.put_str(0, y, &text, marked, w);
            } else {
                buf.put_str(0, y, &text, style, w);
            }
        }
    }

    // The command line: the panel's path as the prompt, then what is being typed.
    let mut cursor = None;
    if cmd_h == 1 {
        let y = menu_h + full_h + prompt_extra;
        let style = app.theme.fg_on("cmdline.fg", "cmdline.bg");
        let skip = prompt_lines.len() - 1 - prompt_extra as usize;
        for (i, line) in prompt_lines[skip..prompt_lines.len() - 1]
            .iter()
            .enumerate()
        {
            let row = menu_h + full_h + i as u16;
            buf.fill(Rect::new(0, row, w, 1), ' ', style);
            buf.put_str(0, row, line, style, w);
        }
        buf.fill(Rect::new(0, y, w, 1), ' ', style);
        let prompt = prompt_lines.last().copied().unwrap_or("").to_string();
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
            let ink = app.theme.fg_on("cmdline.bg", "cmdline.fg");
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
            let dim = app.theme.fg_on("cmdline.hint.fg", "cmdline.bg");
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
                let style = app.theme.fg_on("dialog.fg", "dialog.bg");
                buf.fill(area, ' ', style);
                // The window onto the list, so the cursor is always on a row that is drawn.
                let first = c.cursor.saturating_sub(rows.saturating_sub(1));
                for (i, row) in c.rows.iter().skip(first).take(rows).enumerate() {
                    let here = first + i == c.cursor;
                    let style = if here {
                        app.theme
                            .fg_on("dialog.button.selected.fg", "dialog.button.selected.bg")
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
        let row = Rect::new(0, menu_h + full_h + prompt_extra + cmd_h, w, 1);
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
        // A question out to an AI: which service, and the seconds it has been thinking, with
        // a wheel that turns, until the answer or the error comes back.
        if let Some((name, since)) = &app.ai_busy {
            let secs = since.elapsed().as_secs();
            let wheel = ['◐', '◓', '◑', '◒'][(secs % 4) as usize];
            let text = app.i18n.t_args(
                "ai-busy",
                &[
                    ("name", name.clone().into()),
                    ("secs", (secs as i64).into()),
                ],
            );
            mark = format!("{wheel} {text} {mark}");
        }
        {
            let mark = mark.trim_end().to_string();
            let width = unicode_width::UnicodeWidthStr::width(mark.as_str()) as u16;
            if width > 0 && width < w {
                // While recording, the corner is drawn as an error is: it has to be noticed.
                let mut style = app.theme.fg_on("status.fg", "status.bg");
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
    let corner = if app.cfg.ui.menu_bar {
        app.theme.fg_on("hmenu.fg", "hmenu.bg")
    } else {
        app.theme.fg_on("panel.frame", "panel.bg")
    };
    let mut right = w;
    if app.cfg.ui.clock && w > 10 {
        let now = format!(" {} ", fcmd_ui::clock_now());
        let width = unicode_width::UnicodeWidthStr::width(now.as_str()) as u16;
        buf.put_str(w - width, 0, &now, corner, width);
        right = w - width;
    }
    // The jobs nobody is watching, next to the clock: how many run or wait, with a mark when
    // one of them is asking something. `Alt+Shift+F5` lists them (SPEC 03 §5).
    let unseen: Vec<&crate::queue::Job> = app
        .queue
        .jobs
        .iter()
        .filter(|j| j.state != crate::queue::JobState::Done)
        .filter(|j| !matches!(&app.mode, Mode::Job(v) if v.id == j.id))
        .collect();
    if !unseen.is_empty() && right > 20 {
        let asking = unseen.iter().any(|j| j.question.is_some());
        let mark = format!(" ⧗{}{} ", unseen.len(), if asking { "?" } else { "" });
        let width = unicode_width::UnicodeWidthStr::width(mark.as_str()) as u16;
        buf.put_str(right - width, 0, &mark, corner, width);
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
        Mode::LearnKeys(learn) => {
            draw_learn_keys(app, learn, buf, screen);
            None
        }
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

    let style = app.theme.fg_on("panel.header.fg", "status.bg");
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

    let header = app.theme.fg_on("panel.header.fg", "status.bg");
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
    let heading = app.theme.fg_on("panel.column.fg", "panel.bg");
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
    let frame = app.theme.fg_on("panel.frame", "panel.bg");
    let group = app.theme.fg_on("panel.column.fg", "panel.bg");
    let dim = app.theme.fg_on("panel.footer.fg", "panel.bg");
    let picked = app.theme.fg_on("cursor.fg", "cursor.bg");
    let unfocused = app.theme.fg_on("cursor.inactive.fg", "cursor.inactive.bg");

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
    let (area, first) = widgets::draw_menu(buf, &app.theme, below, at, &rows, item);
    let shown = area.h.saturating_sub(2);
    let list = (shown > 0).then(|| ListArea {
        area: Rect::new(area.x + 1, area.y + 1, area.w.saturating_sub(2), shown),
        // A dropdown is as tall as it needs to be when the screen has the room, and scrolls
        // with the cursor when it does not.
        top: first,
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
        crate::menu::PickKind::Frecent => "frecent-hint",
        crate::menu::PickKind::Highlight => "hl-hint",
        crate::menu::PickKind::Filters => "filter-hint",
        crate::menu::PickKind::Git => "git-hint",
        crate::menu::PickKind::CharMap => "charmap-hint",
        crate::menu::PickKind::AiServices => "ai-services-hint",
        crate::menu::PickKind::AiContext => "ai-context-hint",
        crate::menu::PickKind::AiThreads => "ai-threads-hint",
        crate::menu::PickKind::StripMeta => "strip-hint",
        crate::menu::PickKind::PluginsConfig => "plugins-config-hint",
        crate::menu::PickKind::ReplaceTemplates => "replace-tpl-hint",
        crate::menu::PickKind::EditorMatches => "editor-matches-hint",
        crate::menu::PickKind::EditorConvert => "editor-convert-hint",
        crate::menu::PickKind::Jobs => "jobs-hint",
        crate::menu::PickKind::LeftJobs => "journal-hint",
        crate::menu::PickKind::SavedSyncs => "sync-saved-hint",
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

    // The plugin commands that cannot run from here: still listed, so the reader knows they
    // exist, but in the menu's dim ink — the colour FAR gives a menu item that is not
    // available. The row under the cursor keeps the cursor's colours, as above.
    if pick.kind == crate::menu::PickKind::Plugins {
        let width = area.w.saturating_sub(2) as usize;
        let dim = app.theme.fg_on("menu.dim.fg", "menu.bg");
        for (row, line) in shown.iter().enumerate() {
            let y = area.y + 1 + row as u16;
            if y >= area.y + area.h.saturating_sub(1) || row == pick.cursor.saturating_sub(start) {
                continue;
            }
            let value = pick.rows.get(start + row).map(|r| r.value.as_str());
            if !value.is_some_and(|v| app.plugin_dim.iter().any(|d| d == v)) {
                continue;
            }
            let text = format!(" {:<w$}", line, w = width.saturating_sub(1));
            buf.put_str(area.x + 1, y, &text, dim, width as u16);
        }
    }

    // The rules between groups of rows, drawn across the frame and joined to it the way the
    // panel's own separators are: `lines()` leaves them blank because only this knows where
    // the frame is.
    let frame = app.theme.fg_on("menu.frame", "menu.bg");
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
        buf.set(area.x, y, '╟', frame);
        buf.set(area.right() - 1, y, '╢', frame);
        // A rule with a name is the heading of the group under it, written into the line the
        // way FAR writes a titled separator: centred, with a space either side.
        if let Some(name) = pick
            .rows
            .get(start + i)
            .map(|r| r.label.trim())
            .filter(|l| !l.is_empty())
        {
            let room = (area.w as usize).saturating_sub(6);
            let mut shown = String::new();
            for c in name.chars() {
                if unicode_width::UnicodeWidthStr::width(format!("{shown}{c}").as_str()) > room {
                    break;
                }
                shown.push(c);
            }
            let text = format!(" {shown} ");
            let width = unicode_width::UnicodeWidthStr::width(text.as_str()) as u16;
            let x = area.x + (area.w.saturating_sub(width)) / 2;
            buf.put_str(x, y, &text, frame, width);
        }
    }

    // The key each row answers to, in the colour a menu gives a hotkey: in the drive dialog
    // that is the drive's own letter and the number of a panel that is not a drive, and it
    // is the half of the row anybody who uses it twice a day actually reads.
    let keyed = pick.rows.iter().any(|r| r.hotkey.is_some());
    if keyed {
        let ink = app.theme.fg_on("menu.hotkey.fg", "menu.bg");
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

    let style = app.theme.fg_on("menu.selected.fg", "menu.selected.bg");
    let row = pick.cursor.saturating_sub(start);
    let y = area.y + 1 + row as u16;
    if row < shown.len() && y < area.y + area.h.saturating_sub(1) {
        let width = area.w.saturating_sub(2) as usize;
        let text = format!(" {:<w$}", shown[row], w = width.saturating_sub(1));
        buf.put_str(area.x + 1, y, &text, style, width as u16);
        // The key stays yellow on the row one is standing on, as it does in FAR's menus.
        if let Some(key) = pick.rows.get(pick.cursor).and_then(|r| r.hotkey) {
            let ink = app.theme.fg_on("menu.hotkey.fg", "menu.selected.bg");
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
            // A job of the queue can also go to the background, and the box says so.
            hint: &app.i18n.t(if app.queue.get(view.id).is_some() {
                "job-hint-queue"
            } else {
                "job-hint"
            }),
        },
    );
}

/// The colour runs of a hex or a disassembly row, as `(start, end, theme key)` in bytes of
/// `text`. Read off the text the viewer laid out, whose columns are fixed: an offset, then the
/// bytes, then the characters between bars — or an address, the bytes, and the instruction.
fn binary_runs(kind: fcmd_view::RowKind, text: &str) -> Vec<(usize, usize, &'static str)> {
    use fcmd_view::RowKind;
    let mut out = Vec::new();
    match kind {
        RowKind::Hex => {
            let Some(first_space) = text.find(' ') else {
                return out;
            };
            out.push((0, first_space, "syntax.type"));
            let bar = text.find('|').unwrap_or(text.len());
            // The bytes: each pair coloured by what kind of byte it is.
            let mut i = first_space;
            let bytes = text.as_bytes();
            while i + 1 < bar {
                if bytes[i].is_ascii_hexdigit() && bytes[i + 1].is_ascii_hexdigit() {
                    let value = u8::from_str_radix(&text[i..i + 2], 16).unwrap_or(0);
                    out.push((i, i + 2, byte_class(value)));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            // The characters: what reads as text in the string colour, the dots dim.
            if let Some(last) = text.rfind('|').filter(|&l| l > bar) {
                out.push((bar, bar + 1, "syntax.comment"));
                for (offset, ch) in text[bar + 1..last].char_indices() {
                    let at = bar + 1 + offset;
                    let key = if ch == '.' {
                        "syntax.comment"
                    } else {
                        "syntax.string"
                    };
                    out.push((at, at + ch.len_utf8(), key));
                }
                out.push((last, last + 1, "syntax.comment"));
            }
        }
        RowKind::Label => {
            if let Some(space) = text.find(' ') {
                out.push((0, space, "syntax.comment"));
                out.push((space, text.len(), "syntax.function"));
            }
        }
        RowKind::Code => {
            // An address, a gap, the bytes, a wider gap, the instruction.
            let Some(addr_end) = text.find(' ') else {
                return out;
            };
            out.push((0, addr_end, "syntax.type"));
            let rest = &text[addr_end..];
            let lead = rest.len() - rest.trim_start().len();
            let bytes_start = addr_end + lead;
            let Some(gap) = text[bytes_start..].find("  ") else {
                return out;
            };
            let bytes_end = bytes_start + gap;
            out.push((bytes_start, bytes_end, "syntax.comment"));
            let after = &text[bytes_end..];
            let insn_start = bytes_end + (after.len() - after.trim_start().len());
            let insn = &text[insn_start..];
            let mnemonic_len = insn.find(' ').unwrap_or(insn.len());
            let mnemonic = &insn[..mnemonic_len];
            out.push((
                insn_start,
                insn_start + mnemonic_len,
                mnemonic_class(mnemonic),
            ));
            operand_runs(text, insn_start + mnemonic_len, &mut out);
        }
        _ => {}
    }
    out
}

/// Zero, the padding most of a binary is, dim; the bytes that are letters and digits plain;
/// control bytes and the high half in colours of their own; `FF` loud, because erased flash
/// and sentinel values are what one goes looking for.
fn byte_class(value: u8) -> &'static str {
    match value {
        0x00 => "syntax.comment",
        0xFF => "syntax.keyword",
        0x20..=0x7E => "syntax.text",
        0x80..=0xFE => "syntax.preproc",
        _ => "syntax.number",
    }
}

/// What an instruction does to the flow, which is what the eye follows down a listing.
fn mnemonic_class(mnemonic: &str) -> &'static str {
    let m = mnemonic.to_ascii_lowercase();
    let flow = m.starts_with('j')
        || m.starts_with("call")
        || m.starts_with("ret")
        || m.starts_with("loop")
        || matches!(
            m.as_str(),
            "b" | "bl"
                | "blr"
                | "br"
                | "bx"
                | "cbz"
                | "cbnz"
                | "tbz"
                | "tbnz"
                | "syscall"
                | "sysenter"
                | "int"
                | "iret"
                | "iretq"
                | "leave"
                | "hlt"
                | "ud2"
                | "svc"
        )
        || (m.len() <= 4 && m.starts_with("b."));
    if flow {
        "syntax.preproc"
    } else if matches!(m.as_str(), "nop" | "int3" | "endbr64" | "endbr32") {
        "syntax.comment"
    } else if m.starts_with("push") || m.starts_with("pop") || m == "stp" || m == "ldp" {
        "syntax.function"
    } else {
        "syntax.keyword"
    }
}

/// The operands after the mnemonic: a `<symbol>` in the function colour, numbers in the
/// number colour, the size words (`qword ptr`, `short`) dim, and registers in the string
/// colour.
fn operand_runs(text: &str, from: usize, out: &mut Vec<(usize, usize, &'static str)>) {
    let bytes = text.as_bytes();
    let mut i = from;
    while i < text.len() {
        let c = bytes[i];
        if c == b'<' {
            let end = text[i..].find('>').map(|e| i + e + 1).unwrap_or(text.len());
            out.push((i, end, "syntax.function"));
            i = end;
        } else if c.is_ascii_alphanumeric() || c == b'_' || c == b'#' {
            let start = i;
            while i < text.len()
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'#')
            {
                i += 1;
            }
            let word = &text[start..i];
            let number = word.starts_with('#')
                || word.starts_with("0x")
                || word.bytes().all(|b| b.is_ascii_digit())
                || (word.ends_with('h')
                    && word[..word.len() - 1]
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit()));
            let size = matches!(
                word,
                "ptr"
                    | "byte"
                    | "word"
                    | "dword"
                    | "qword"
                    | "xmmword"
                    | "ymmword"
                    | "short"
                    | "near"
                    | "far"
                    | "tbyte"
            );
            // Registers in the string colour: the type colour is the viewer's own cyan in
            // FAR's palette, and a register that looks like the text around it is lost.
            let key = if number {
                "syntax.number"
            } else if size {
                "syntax.comment"
            } else {
                "syntax.string"
            };
            out.push((start, i, key));
        } else {
            i += 1;
        }
    }
}

/// A prompt, a dialog or a confirmation over an editor that has already been drawn, and
/// where the caret goes in it. The text editor and the hex editor share it, so a question
/// looks the same over lines and over bytes.
fn draw_dialog_over_screen(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
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
            Some((cx, cy))
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
            form.caret().map(|(row, col)| {
                (
                    area.x + 2 + col as u16,
                    // A dialog with pages has its row of pages under the title.
                    area.y + 1 + u16::from(!form.pages.is_empty()) + row as u16,
                )
            })
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
        // A list opened from the editor — the matches of a search, the characters, the
        // replacements kept — is drawn over it like the dialogs are. Without this arm the
        // list was there and answered keys, and nobody could see it.
        Mode::Pick(pick) => {
            let list = draw_pick(app, buf, screen, pick);
            app.layout.list = list;
            None
        }
        _ => None,
    }
}

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

    let changed = app.theme.fg_on("selected.fg", "panel.bg");
    let marked = app
        .theme
        .fg_on("panel.info.selected.fg", "panel.info.selected.bg");
    // `Editor.Text.Selected`: a marked block, which in FAR's palette is the status
    // line's colour rather than the panel cursor's.
    let on = app.theme.fg_on("editor.selected.fg", "editor.selected.bg");
    let offset_style = app.theme.fg_on("panel.column.fg", "panel.bg");
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
    let changes = editor.first_changes(4);
    if !changes.is_empty() && body.h > 6 {
        let rows = (changes.len() as u16).min(4) + 1;
        let top = body.y + body.h - rows;
        buf.fill(Rect::new(0, top, w, rows), ' ', base);
        let rule = app.theme.fg_on("panel.frame", "panel.bg");
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
    let selected = app.theme.fg_on("editor.selected.fg", "editor.selected.bg");

    // Where the text ended up, for the mouse: a click puts the caret there and the wheel
    // scrolls. The editor draws its lines from column zero, so this is the body as it is.
    app.layout.text = Some(body);

    let colouring = app.cfg.ui.syntax;
    // The lines down the blocks of a Pascal unit or a form, where the viewer draws them and
    // under the same switch as the colours they are worked out with. Owned, because the
    // editor is borrowed again for every line below.
    let guides: Option<Vec<fcmd_view::guides::Guides>> = colouring
        .then(|| editor.block_guides(visible.clone()).map(<[_]>::to_vec))
        .flatten();
    let guide = app.theme.fg_on("syntax.guide", "editor.bg");
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
                let style = app.theme.styled(span.class.theme_key(), base);
                let room = body.w - from as u16;
                buf.put_str(from as u16, y, &slice, style, room);
            }
        }

        // The guides, after the colours and before the block: a line runs only through
        // blank cells — never over text — and a marked block is drawn over it as over
        // anything else. Blank is asked of the line itself and not of what is on screen,
        // where a space may be showing as `·`.
        if let Some(g) = guides.as_ref().and_then(|g| g.get(row)) {
            let line = expand_line(editor, line_index);
            for (cols, glyph) in [(&g.through, "│"), (&g.ends, "└")] {
                for &c in cols {
                    let blank = line.chars().nth(c).is_none_or(char::is_whitespace);
                    if blank && c >= editor.left && c - editor.left < body.w as usize {
                        buf.put_str((c - editor.left) as u16, y, glyph, guide, 1);
                    }
                }
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
/// A picture, painted into `area`: in real pixels where the terminal draws them and they are
/// ready (SPEC 05 §1.14a), in half blocks otherwise (§1.14). Does nothing unless the viewer
/// is in that mode, so the callers can ask without checking first.
///
/// Both of them paint it the same way — the viewer across `F3`'s whole window, the quick view
/// inside the passive panel's frame — and they differ only in the colour a transparent pixel
/// is mixed with, which is whatever that window is drawn in and so follows from which of the
/// two it is.
fn paint_picture(
    pixels: &mut crate::pixels::Screen,
    slot: crate::pixels::Slot,
    viewer: &mut fcmd_view::Viewer,
    theme: &fcmd_ui::Theme,
    buf: &mut Buffer,
    area: Rect,
    base: Style,
) -> bool {
    if viewer.mode != fcmd_view::Mode::Image {
        return false;
    }
    let bg_key = match slot {
        crate::pixels::Slot::Viewer => "viewer.bg",
        crate::pixels::Slot::Quick => "panel.bg",
    };
    let under = match theme.color(bg_key) {
        fcmd_ui::Color::Rgb(r, g, b) => [r, g, b],
        // A theme written in palette colours has no bytes to mix a transparent pixel with,
        // so the picture is composited onto black.
        _ => [0, 0, 0],
    };
    // Real pixels where the terminal draws them and they are ready; the cells are then
    // the picture's, and the terminal backend is handed the image (SPEC 05 §1.14a).
    if pixels.place(slot, viewer, buf, area, under, base) {
        return true;
    }
    // Half blocks otherwise — and in a terminal that draws pixels, in the cells the pixels
    // take, so the picture does not change size when they arrive or a dialog covers them.
    let area = pixels.window(viewer, buf, area);
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
    // A film's frame is asked for once the frame it goes in has a size (SPEC 05 §1.15).
    if let Some(mut quick) = app.quick.take() {
        app.first_frame(&mut quick, app.quick_size);
        app.quick = Some(quick);
    }
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
    let drew = paint_picture(
        &mut app.pixels,
        crate::pixels::Slot::Quick,
        viewer,
        &app.theme,
        buf,
        body,
        base,
    );

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
    let foot = app.theme.fg_on("panel.frame", "panel.bg");
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

/// The AI conversation in its own panel: the session's title on the frame, the
/// conversation below it as the quick view would show it, the newest at the foot.
fn draw_ai_chat(app: &mut App, index: usize, area: Rect, buf: &mut Buffer) {
    if area.w < 8 || area.h < 3 {
        return;
    }
    let title = app.panels[index]
        .temp
        .as_ref()
        .map(|t| t.title.clone())
        .unwrap_or_default();
    let inner = widgets::draw_pane(buf, &app.theme, area, &title, index == app.active);
    let base = app.theme.panel();
    app.quick_size = (inner.w.saturating_sub(1), inner.h);
    app.ai_live_view(index);
    let Some(viewer) = app.quick.as_mut() else {
        return;
    };
    let width = inner.w.saturating_sub(1);
    let rows = viewer.rows(width, inner.h);
    for (n, row) in rows.iter().enumerate() {
        let y = inner.y + n as u16;
        // A question is a heading in the conversation file, and it goes to the right, as in
        // every chat; the answers stay on the left, where text is read from.
        let question = row
            .spans
            .first()
            .is_some_and(|s| s.class == fcmd_view::markdown::Class::Heading);
        let text_w = unicode_width::UnicodeWidthStr::width(row.text.trim_end()) as u16;
        let shift = if question {
            width.saturating_sub(text_w)
        } else {
            0
        };
        let x0 = inner.x + 1 + shift;
        let width = width - shift;
        buf.put_str(x0, y, &row.text, base, width);
        // The Markdown view's own runs: headings, bullets, code and quotes in their colours.
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
            if x >= width {
                continue;
            }
            let mut style = app.theme.styled(span.class.theme_key(), base);
            style.bold |= span.bold;
            buf.put_str(x0 + x, y, text, style, width - x);
        }
    }
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
    let on = app.theme.fg_on("panel.cursor.fg", "panel.cursor.bg");
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
    let label = app.theme.fg_on("panel.header.fg", "panel.bg");
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
        let mut status_text = app.viewer_status(&status);
        // A film says where its frame is, how long it runs and what it is encoded as, ahead
        // of the rest: on a frame that is the position that means something.
        if viewer.mode == fcmd_view::Mode::Image {
            if let Some(note) = viewer.film_note() {
                status_text = format!("{note}  {status_text}");
            }
        }
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

    // A film's first frame is asked for here, now that the window it will be drawn in has
    // a size (SPEC 05 §1.15).
    app.first_frame(viewer, (body.w, body.h));

    // Where the content ended up, for the mouse: the wheel over a viewer scrolls it, and
    // the sideways wheel moves a long line. Recorded after the scrollbar and the structure
    // pane have taken their columns, so a notch over the pane is not a notch over the text.
    app.layout.text = Some(body);

    // Content. `Viewer.Text.Selected` marks a search hit — its own role in FAR's palette,
    // which the panel's background had no business being half of.
    let hit = app
        .theme
        .fg_on("viewer.selected.fg", "viewer.selected.bg")
        .bold();
    let rows = viewer.rows(body.w, body.h);
    // The syntax, where the file is one we recognise and the mode is the one that shows it
    // as text. What the first row starts inside is read from the lines above it, a bounded
    // stretch of them, so a block comment opened above the window — a form's binary data —
    // keeps its colour when its first line scrolls off.
    let syntax = (app.cfg.ui.syntax && viewer.mode == fcmd_view::Mode::Text)
        .then(|| fcmd_syntax::of_name(&viewer.path.display_name()))
        .flatten();
    let mut state = match syntax {
        Some(syntax) => viewer.syntax_state_at_top(syntax),
        None => fcmd_syntax::State::default(),
    };
    // The lines down the blocks of a Pascal unit or a form, as the Delphi IDE draws them.
    let guides = syntax.and_then(|syntax| viewer.block_guides(syntax, body.h));
    let guide = app.theme.fg_on("syntax.guide", "viewer.bg");
    // In a table the row under the cursor is drawn as a panel draws its cursor, and a marked
    // row in the colour a panel marks a file with: the same two things, said the same way.
    let table = viewer.mode == fcmd_view::Mode::Table;
    let cursor_bg = app.theme.color("panel.cursor.bg");
    let cursor_fg = app.theme.color("panel.cursor.fg");
    let marked_fg = app.theme.color("selected.fg");
    for (i, row) in rows.iter().enumerate() {
        let y = body.y + i as u16;
        let (on_cursor, marked) = if table {
            viewer.table_row_state(i)
        } else {
            (false, false)
        };
        let mut style = base;
        if on_cursor {
            style = Style::new(cursor_fg, cursor_bg);
        }
        if marked {
            style = style.with_fg(marked_fg);
        }
        if on_cursor || marked {
            buf.put_str(0, y, &" ".repeat(body.w as usize), style, body.w);
        }
        buf.put_str(0, y, &row.text, style, body.w);
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
            let mut style = app.theme.styled(span.class.theme_key(), base);
            style.bold |= span.bold;
            buf.put_str(x, y, text, style, body.w - x);
        }
        // The bytes and the instructions get colours of their own, from the same theme keys
        // the source colouring uses, so every theme already has them: a page of hex in one
        // colour is a page nobody can find a zero, a string or a jump in.
        if app.cfg.ui.syntax && row.spans.is_empty() {
            for (start, end, key) in binary_runs(row.kind, &row.text) {
                let text = &row.text[start..end];
                let x = row.text[..start]
                    .chars()
                    .filter_map(unicode_width::UnicodeWidthChar::width)
                    .sum::<usize>() as u16;
                if x < body.w && !text.is_empty() {
                    let mut style = base.with_fg(app.theme.color(key));
                    style.bold = key == "syntax.function" && row.kind == fcmd_view::RowKind::Label;
                    buf.put_str(x, y, text, style, body.w - x);
                }
            }
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
                    let style = app.theme.styled(span.class.theme_key(), base);
                    buf.put_str(x, y, text, style, body.w - x);
                }
            }
        }
        if let Some(g) = guides.as_ref().and_then(|g| g.get(i)) {
            for (cols, glyph) in [(&g.through, "│"), (&g.ends, "└")] {
                for &c in cols {
                    let blank = row.text.chars().nth(c).is_none_or(char::is_whitespace);
                    if blank && (c as u16) < body.w {
                        buf.put_str(c as u16, y, glyph, guide, 1);
                    }
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
    paint_picture(
        &mut app.pixels,
        crate::pixels::Slot::Viewer,
        viewer,
        &app.theme,
        buf,
        body,
        base,
    );

    if let Some(at) = pane {
        draw_structure(app, viewer, buf, at);
    }

    // Where in the file we are, as a bar. It says *where* and not *how much*: how many bytes
    // a screenful holds is not a fixed fraction of a file — a wrapped line, a hex dump and a
    // rendered Markdown page each answer differently — so the thumb is one row and the
    // position is the percentage the status line already shows.
    if let Some(x) = bar {
        let style = app.theme.fg_on("panel.scrollbar.fg", "viewer.bg");
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
            .unwrap_or_else(|| app.keybar_viewer_for(app.held));
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
            // The exact size, in bytes, as FAR's status line has it: the column above already
            // says it short, and this line is where the whole number is read.
            let size = if e.kind.is_dirlike() {
                String::new()
            } else {
                format!("  {}", e.size)
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
    let frame = app.theme.fg_on("panel.frame", "panel.bg");
    let dim = app.theme.fg_on("panel.footer.fg", "panel.bg");
    let picked = app.theme.fg_on("cursor.fg", "cursor.bg");

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
    buf.clear(app.theme.fg_on("panel.fg", "panel.bg"));
    for y in 0..h.min(grab.height()) {
        for x in 0..w.min(grab.width()) {
            if let Some(cell) = grab.screen.get(x, y) {
                buf.set(x, y, cell.ch, cell.style);
            }
        }
    }
    // What is marked, in the colours a selection has everywhere else.
    let area = grab.area();
    let marked = app.theme.fg_on("selected.fg", "selected.bg");
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
        let style = app.theme.fg_on("status.fg", "status.bg");
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
    let frame = app.theme.fg_on("panel.frame", "panel.bg");
    let dim = app.theme.fg_on("panel.footer.fg", "panel.bg");
    // A diff's own two colours, the ones a `.diff` has in the viewer: what the left side has
    // and the right does not is red, what the right side brings is green. A changed line is
    // both — red on the left, green on the right — with the part that differs marked.
    // The whole line is painted, not only its sign, so a change is seen from across the
    // room; the sign stays for a screen without colour and for a copy of the text.
    // A theme that paints no backgrounds at all — the black one — leaves the two lines the
    // panel's own ground, and then the text itself carries the red and the green.
    let painted = |bg: &str| app.theme.color(bg) != app.theme.color("panel.bg");
    let removed = if painted("diff.removed.bg") {
        app.theme.fg_on("diff.line.fg", "diff.removed.bg")
    } else {
        app.theme.fg_on("syntax.removed", "panel.bg")
    };
    let added = if painted("diff.added.bg") {
        app.theme.fg_on("diff.line.fg", "diff.added.bg")
    } else {
        app.theme.fg_on("syntax.added", "panel.bg")
    };
    let inner_mark = app.theme.fg_on("selected.fg", "selected.bg");

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
    // The number, then `-`, `+` or a blank, then a space: the sign is what a unified diff
    // would have put in front of the line.
    let number_w = 7u16;
    let text_w = side_w.saturating_sub(number_w + 1).max(4) as usize;

    // A side that has been merged into carries a `*`, the way every editor has said "this is
    // not what is on the disk" since there were editors.
    let name = |text: &str, edited: bool| {
        let room = (side_w as usize).saturating_sub(if edited { 4 } else { 2 });
        let shown = fcmd_ui::elide_middle(text, room);
        if edited {
            format!("*{shown}")
        } else {
            shown
        }
    };
    let names = format!(
        "- {}  ↔  + {}",
        name(&view.left_name, view.left_edited),
        name(&view.right_name, view.right_edited)
    );
    buf.put_str(inner.x, inner.y, &names, dim, inner.w);

    let top = inner.y + 1;
    for (i, row) in view.window(rows_h as u16).iter().enumerate() {
        let y = top + i as u16;
        // What each side of this row is: the left side of a row that is not the same on both
        // is what went away, the right side what came.
        let (left_style, right_style, left_sign, right_sign) = match row.kind {
            Side::Same => (base, base, ' ', ' '),
            Side::Removed => (removed, base, '-', ' '),
            Side::Added => (base, added, ' ', '+'),
            Side::Changed => (removed, added, '-', '+'),
        };
        // The rule down the middle, drawn whatever is on either side of it.
        buf.put_str(inner.x + side_w, y, "│", frame, 1);
        let mut side = |x: u16,
                        number: Option<usize>,
                        text: &str,
                        mark: &std::ops::Range<usize>,
                        style: Style,
                        sign: char| {
            let label = match number {
                Some(n) => format!("{n:>5}"),
                None => " ".repeat(5),
            };
            buf.put_str(x, y, &label, dim, 5);
            let sign_style = if sign == ' ' { dim } else { style.bold() };
            buf.put_str(x + 5, y, &format!("{sign} "), sign_style, 2);
            let from = view.col.min(text.len());
            let shown: String = text[from..].chars().take(text_w).collect();
            if sign != ' ' {
                buf.fill(Rect::new(x + number_w, y, text_w as u16, 1), ' ', style);
            }
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
        side(
            inner.x,
            row.left_no,
            &row.left,
            &row.left_inner,
            left_style,
            left_sign,
        );
        side(
            inner.x + side_w + 1,
            row.right_no,
            &row.right,
            &row.right_inner,
            right_style,
            right_sign,
        );
    }

    let foot = if view.identical() {
        app.i18n.t("diff-same")
    } else {
        // How much went and how much came, as `git diff --stat` counts it: a changed line is
        // one of each.
        let (gone, came) = view
            .rows()
            .iter()
            .fold((0i64, 0i64), |(g, c), r| match r.kind {
                Side::Same => (g, c),
                Side::Removed => (g + 1, c),
                Side::Added => (g, c + 1),
                Side::Changed => (g + 1, c + 1),
            });
        app.i18n.t_args(
            "diff-where",
            &[
                ("at", (view.hunk_at() as i64).into()),
                ("count", (view.hunks() as i64).into()),
                ("removed", gone.into()),
                ("added", came.into()),
            ],
        )
    };
    // git's own diff says the key that stages or unstages a hunk.
    let keys = app.i18n.t(match app.diff_git {
        Some((_, false)) => "diff-hint-git",
        Some((_, true)) => "diff-hint-git-staged",
        // A rewrite proposed for the editor's block is answered, not merged.
        None if app.ai_rewrite_shown() => "diff-hint-rewrite",
        None => "diff-hint",
    });
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
    let frame = app.theme.fg_on("panel.frame", "panel.bg");
    let dim = app.theme.fg_on("panel.footer.fg", "panel.bg");
    let picked = app.theme.fg_on("cursor.fg", "cursor.bg");
    let gone = app.theme.fg_on("file.exec", "panel.bg");
    let clash = app.theme.fg_on("selected.fg", "panel.bg");

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
    let frame = app.theme.fg_on("panel.frame", "panel.bg");
    let dim = app.theme.fg_on("panel.footer.fg", "panel.bg");
    let picked = app.theme.fg_on("cursor.fg", "cursor.bg");

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
        app.theme.fg_on("panel.column.fg", "panel.bg"),
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
    let selected = app.theme.fg_on("cursor.fg", "cursor.bg");
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

/// Which keys this terminal delivers (SPEC 06 §7).
///
/// A row per family, a cell per key, and a cell is lit once that key has arrived. What one is
/// looking for is the cell that never lights up: that key is not reaching the program, and no
/// binding will make it.
fn draw_learn_keys(app: &App, learn: &crate::app::LearnKeys, buf: &mut Buffer, screen: Rect) {
    let rows = crate::app::learn_rows();
    // The widest label, so the cells of every row start in the same column.
    let label_w = rows
        .iter()
        .map(|(label, _)| label.chars().count())
        .max()
        .unwrap_or(8);
    let cells = rows.iter().map(|(_, keys)| keys.len()).max().unwrap_or(12);
    let cell_w = rows
        .iter()
        .flat_map(|(_, keys)| keys.iter().map(|(cell, _)| cell.chars().count()))
        .max()
        .unwrap_or(3)
        + 2;
    let box_w = ((label_w + 2 + cells * cell_w) as u16 + 4)
        .min(screen.w.saturating_sub(2))
        .max(24);
    let box_h = (rows.len() as u16 + 7)
        .min(screen.h.saturating_sub(1))
        .max(8);
    let area = Rect::new(
        screen.x + (screen.w.saturating_sub(box_w)) / 2,
        screen.y + (screen.h.saturating_sub(box_h)) / 2,
        box_w,
        box_h,
    );
    let style = app.theme.dialog();
    buf.fill(area, ' ', style);
    let title = app.i18n.t("learn-title");
    buf.draw_bordered(
        area,
        app.theme.fg_on("dialog.frame", "dialog.bg"),
        Some(&title),
        fcmd_ui::Border::Double,
    );
    let inner = area.inner();
    // A dialog has no colour of its own for "not yet": the lit cell is drawn bold and
    // carries a dot, which reads the same on a terminal with eight colours and on paper.
    let dim = style;
    let mut seen_style = style;
    seen_style.bold = true;
    let mut y = inner.y;
    buf.put_str(inner.x, y, &app.i18n.t("learn-hint"), dim, inner.w);
    y += 2;
    for (label, keys) in &rows {
        if y >= inner.bottom() {
            break;
        }
        buf.put_str(inner.x, y, label, style, inner.w);
        let mut x = inner.x + label_w as u16 + 2;
        for (cell, name) in keys {
            if x + cell_w as u16 > inner.right() {
                break;
            }
            let seen = learn.seen.iter().any(|key| key == name);
            let text = if seen {
                format!("{cell}·")
            } else {
                cell.clone()
            };
            buf.put_str(
                x,
                y,
                &text,
                if seen { seen_style } else { dim },
                cell_w as u16,
            );
            x += cell_w as u16;
        }
        y += 1;
    }
    // What arrived last, and what it does: the other half of the question.
    if let Some((name, bound)) = &learn.last {
        let what = match bound {
            Some(cmd) => cmd.id().to_string(),
            None => app.i18n.t("learn-unbound"),
        };
        let line = app.i18n.t_args(
            "learn-last",
            &[("key", name.clone().into()), ("command", what.into())],
        );
        if y < inner.bottom() {
            buf.put_str(inner.x, y + 1, &line, style, inner.w);
        }
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
    buf.draw_bordered(
        area,
        app.theme.fg_on("dialog.frame", "dialog.bg"),
        Some(&title),
        fcmd_ui::Border::Double,
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
            let style = app.theme.fg_on("menu.selected.fg", "menu.selected.bg");
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
        app.theme.fg_on("dialog.title", "dialog.bg"),
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
            line.contains("╢╟") || line.contains("╝╚"),
            "the box ran into the other panel: {line}"
        );
    }

    /// The frequent folders: the most used at the top, what was typed on the frame, and a
    /// footer that names the list's own keys.
    #[test]
    fn the_frequent_folders_are_listed_best_first_with_their_own_footer() {
        let mut app = app("far", "en");
        let here = app.panels[0].cwd.clone();
        let now = crate::history::now();
        for (name, times) in [("rarely", 1), ("often", 5)] {
            for _ in 0..times {
                app.visits.visit(&here.join(name).to_string(), now);
            }
        }
        app.run(fcmd_cmd::Command::NavGotoFuzzy);
        let text = frame(&mut app, 100, 30);
        assert!(text.contains("Frequent folders"), "{text}");
        assert!(text.contains("Del forget"), "{text}");
        let line_of = |name: &str| {
            text.lines()
                .position(|line| line.contains(name))
                .unwrap_or_else(|| panic!("no {name} in\n{text}"))
        };
        assert!(line_of("often") < line_of("rarely"), "{text}");

        // Typed letters are on the frame, and what does not hold them is gone.
        press(&mut app, "f");
        press(&mut app, "t");
        press(&mut app, "n");
        let text = frame(&mut app, 100, 30);
        assert!(text.contains("Frequent folders: ftn"), "{text}");
        assert!(text.contains("often") && !text.contains("rarely"), "{text}");
    }

    /// The find dialog says which keys keep a search and bring one back, because nothing
    /// else on it would; and the list of kept searches says what each one looks for.
    #[test]
    fn the_find_dialog_names_its_keys_and_the_kept_searches_say_what_they_look_for() {
        let mut app = app("far", "en");
        app.run(fcmd_cmd::Command::ToolFind);
        let text = frame(&mut app, 100, 30);
        assert!(text.contains("F2 keeps this search under a name"), "{text}");
        // The whole dialog still fits the smallest screen anybody runs: its first row, the
        // rows this dialog grew by, and its buttons.
        let small = frame(&mut app, 80, 24);
        for part in [
            "File mask",
            "Approximate name",
            "Metadata",
            "Inside archives",
            "Ask the system's index",
            "Cancel",
        ] {
            assert!(small.contains(part), "no {part:?} in\n{small}");
        }

        app.searches.push(fcmd_config::SavedSearch {
            name: "big logs".into(),
            mask: "*.log".into(),
            size: "100M".into(),
            roots: vec![app.panels[0].cwd.to_string()],
            ..Default::default()
        });
        press(&mut app, "F9");
        let text = frame(&mut app, 100, 30);
        assert!(text.contains("Saved searches"), "{text}");
        let row = text
            .lines()
            .find(|line| line.contains("big logs"))
            .unwrap_or_else(|| panic!("no row in\n{text}"));
        assert!(row.contains("*.log") && row.contains("100M"), "{row}");
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

        // ---- Alt+F1, with drives that are the same on every machine ------------------------
        use crate::drives::{Drive, Kind};
        let drive =
            |path: &str, kind, label: &str, fs: &str, dev: &str, total: u64, free: u64| Drive {
                path: path.into(),
                kind,
                label: (!label.is_empty()).then(|| label.into()),
                fs: (!fs.is_empty()).then(|| fs.into()),
                device: (!dev.is_empty()).then(|| dev.into()),
                total: (total > 0).then_some(total),
                free: (total > 0).then_some(free),
            };
        const G: u64 = 1 << 30;
        let found = vec![
            drive(
                "/",
                Kind::Fixed,
                "system",
                "ext4",
                "/dev/nvme0n1p2",
                476 * G,
                211 * G,
            ),
            drive("/home/mb", Kind::Home, "", "", "", 0, 0),
            drive(
                "/boot/efi",
                Kind::Fixed,
                "EFI",
                "vfat",
                "/dev/nvme0n1p1",
                G / 2,
                G / 3,
            ),
            drive(
                "/media/mb/USB",
                Kind::Removable,
                "USB",
                "exfat",
                "/dev/sda1",
                58 * G,
                41 * G,
            ),
            drive(
                "/mnt/nas",
                Kind::Network,
                "",
                "nfs4",
                "nas:/volume1",
                3725 * G,
                1220 * G,
            ),
            drive("/tmp", Kind::Memory, "", "tmpfs", "tmpfs", 16 * G, 15 * G),
        ];
        app.mode = Mode::Pick(Box::new(app.drive_pick(0, found)));
        write("drives", &shot(&mut app, 100, 30));
        app.mode = Mode::Normal;

        // ---- an object storage panel, and a listing that says it is not all of it ----------
        let mut objects = app_with_objects(&when(0));
        write("s3", &shot(&mut objects, 100, 30));

        // ---- the viewer, the hex view, the editor -----------------------------------------
        let work = std::env::temp_dir().join("fcmd-shots/project");
        let _ = std::fs::remove_dir_all(work.parent().expect("a parent"));
        std::fs::create_dir_all(&work).expect("the working directory");
        std::fs::write(work.join("sort.rs"), SAMPLE_SOURCE).expect("the sample");
        // A real program, small enough to read whole, and a picture: both drawn here.
        std::fs::write(work.join("square"), sample_elf()).expect("the sample program");
        std::fs::write(work.join("sunset.bmp"), sample_bmp(96, 52)).expect("the sample image");
        // The same date on every run, so a picture with the panels behind it does not change
        // with the clock.
        for name in ["sort.rs", "square", "sunset.bmp"] {
            std::fs::File::options()
                .write(true)
                .open(work.join(name))
                .and_then(|f| f.set_modified(when(2).expect("a time")))
                .expect("the date is set");
        }
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

        // ---- a program: the bytes, the disassembly, and the bytes again to change ----------
        app.panels[0].cursor = app.panels[0]
            .listing
            .row_of_name("square")
            .expect("the program is in the listing");
        press(&mut app, "F3");
        while app
            .mode
            .viewer()
            .is_some_and(|v| v.mode != fcmd_view::Mode::Hex)
        {
            press(&mut app, "F4");
        }
        // Down to the code, where the bytes are worth reading.
        for _ in 0..30 {
            press(&mut app, "Down");
        }
        write("hex", &shot(&mut app, 100, 30));
        press(&mut app, "F5"); // the disassembler, from the entry point
        write("disasm", &shot(&mut app, 100, 30));
        press(&mut app, "Esc");
        app.open_hex(&work.join("square"));
        if let Some(editor) = app.mode.hex_mut() {
            // `helper` returns 42 instead of 7: one byte changed and not saved yet, which is
            // what the picture is for.
            editor.top = 0x1C0;
            editor.cursor = 0x221;
            editor.set_byte(0x221, 0x2A);
        }
        write("hexedit", &shot(&mut app, 100, 30));
        app.mode = Mode::Normal;

        // ---- a picture, drawn in half blocks in any terminal -------------------------------
        app.panels[0].cursor = app.panels[0]
            .listing
            .row_of_name("sunset.bmp")
            .expect("the picture is in the listing");
        press(&mut app, "F3");
        write("picture", &shot(&mut app, 100, 30));
        press(&mut app, "Esc");

        // ---- the branch view, and the split dialog over it ---------------------------------
        let tree = work.parent().expect("a parent").join("tree");
        for (path, len) in [
            ("Cargo.toml", 1_431usize),
            ("README.md", 8_212),
            ("src/main.rs", 3_904),
            ("src/app/panels.rs", 21_560),
            ("src/app/queue.rs", 9_877),
            ("docs/help/files.md", 26_331),
            ("docs/shots/branch.svg", 61_440),
            ("assets/logo.svg", 4_096),
            ("release/fcmd-1.0.tar.gz", 7_506_272),
        ] {
            let at = tree.join(path);
            std::fs::create_dir_all(at.parent().expect("a folder")).expect("the folder");
            std::fs::write(&at, vec![b'x'; len]).expect("the file");
            std::fs::File::options()
                .write(true)
                .open(&at)
                .and_then(|f| f.set_modified(when((len % 9) as u64).expect("a time")))
                .expect("the date is set");
        }
        // The folders' own dates too, which the right panel shows.
        for dir in [
            "src/app",
            "src",
            "docs/help",
            "docs/shots",
            "docs",
            "assets",
            "release",
            "",
        ] {
            if let Ok(f) = std::fs::File::open(tree.join(dir)) {
                let _ = f.set_modified(when(3).expect("a time"));
            }
        }
        let mut tree_app = app_on_disk(&tree);
        tree_app.theme = Theme::builtin("far").expect("the far theme");
        tree_app.cfg.panel.status_line = true;
        tree_app.requests.clear();
        press(&mut tree_app, "Ctrl+Alt+B");
        if let Some(crate::app::Request::Branch {
            panel,
            root,
            cancel,
        }) = tree_app
            .take_requests()
            .into_iter()
            .find(|r| matches!(r, crate::app::Request::Branch { .. }))
        {
            let branch = fcmd_jobs::branch::walk(&fcmd_jobs::LocalFs, &root, 100, &cancel);
            tree_app.handle_event(Event::Branched {
                panel,
                root,
                branch,
            });
        }
        // The panel title and the status line name the folder, which is a temporary
        // directory that differs between runs: it is put back to a fixed one for the picture.
        if let Some(list) = &mut tree_app.panels[0].temp {
            list.title = "Branch: /home/mb/fcmd".to_string();
        }
        tree_app.panels[0].cursor = tree_app.panels[0]
            .listing
            .row_of_name("release/fcmd-1.0.tar.gz")
            .unwrap_or(0);
        // The other panel and the prompt show the same folder under its real name.
        let shown = VPath::local("/home/mb/fcmd");
        tree_app.panels[0].cwd = shown.clone();
        tree_app.panels[1].cwd = shown;
        write("branch", &shot(&mut tree_app, 100, 30));
        tree_app.run(fcmd_cmd::Command::FileSplit);
        if let Mode::Form(form) = &mut tree_app.mode {
            for field in &mut form.fields {
                if let crate::form::Field::Text { key, value, .. } = field {
                    if *key == "split.target" {
                        *value = "/media/mb/USB".to_string();
                    }
                    if *key == "split.size" {
                        *value = "4G".to_string();
                    }
                }
            }
        }
        write("split", &shot(&mut tree_app, 100, 30));
        tree_app.mode = Mode::Normal;

        // ---- git in a panel: what changed, in words, staged apart from not staged ---------
        let mut git_app = self::app("far", "en");
        git_app.cfg.panel.status_line = true;
        // The files are there, with sizes and dates, as in a repository on disk.
        let changed: [(&str, &str, usize); 7] = [
            ("M ", "src/app.rs", 41_208),
            ("A ", "src/live.rs", 3_912),
            ("MM", "src/ai.rs", 18_770),
            (" M", "docs/config.md", 9_431),
            (" M", "CHANGELOG.md", 28_604),
            ("??", "shots/git.svg", 61_002),
            ("??", "shots/ai.svg", 58_310),
        ];
        for (_, path, len) in changed {
            let at = work.join(path);
            std::fs::create_dir_all(at.parent().expect("a folder")).expect("the folder");
            std::fs::write(&at, vec![b'x'; len]).expect("the file");
            std::fs::File::options()
                .write(true)
                .open(&at)
                .and_then(|f| f.set_modified(when((len % 5) as u64).expect("a time")))
                .expect("the date is set");
        }
        let mut files: Vec<(String, String)> = changed
            .iter()
            .map(|(code, path, _)| (code.to_string(), path.to_string()))
            .collect();
        files.insert(5, (" D".into(), "docs/old-notes.md".into()));
        git_app.handle_event(Event::GitDone {
            panel: 0,
            result: Ok(crate::app::GitOut::Status {
                root: work.clone(),
                files,
            }),
        });
        git_app.active = 0;
        write("git", &shot(&mut git_app, 140, 30));

        // ---- the side-by-side diff, the lines that differ painted whole --------------------
        let before = b"fn ask(service: &Service) -> Result<String, String> {\n    let client = client()?;\n    let reply = client.post(&service.url).send()?;\n    let text = reply.text()?;\n    answer(service.api, &text)\n}\n";
        let after = b"fn ask(service: &Service) -> Result<String, String> {\n    let client = client()?;\n    let url = endpoint(service.api, &service.url);\n    let reply = client.post(&url).send()?;\n    let text = reply.text()?;\n    // Any shape: a server picked as Ollama may answer the OpenAI way.\n    answer(service.api, &text)\n}\n";
        git_app.mode = Mode::Diff(Box::new(fcmd_view::Diff::new(
            "ai.rs (last commit)".into(),
            before,
            "ai.rs (on disk)".into(),
            after,
            fcmd_core::diff::Ignore::default(),
        )));
        write("diff", &shot(&mut git_app, 100, 30));
        git_app.mode = Mode::Normal;

        // ---- the AI panel: the conversation itself, as it goes ---------------------------
        let thread = work.join("ai-why-a-panel.md");
        std::fs::write(
            &thread,
            "# Why is the conversation a panel?\n\n*local · qwen3:32b · 412 + 236 tokens*\n\n\
             Because a file manager already has the two things a conversation needs:\n\n\
             - **a place to read** — the panel, with the viewer's Markdown;\n\
             - **a place to pick from** — the other panel, where `F5` puts files into the context.\n\n\
             ---\n\n## And the files I add?\n\n*local · qwen3:32b · lib.rs · 812 + 406 tokens*\n\n\
             They go with every question until `F8` takes them out. `lib.rs` declares the\n\
             modules of the crate; `sort.rs` is where the panel decides its order.\n",
        )
        .expect("the conversation");
        let mut ai_app = self::app("far", "en");
        ai_app.cfg.ais = vec![fcmd_config::AiCfg {
            name: "local".into(),
            url: "http://localhost:11434/api/chat".into(),
            model: "qwen3:32b".into(),
            api: "ollama".into(),
            ..Default::default()
        }];
        ai_app.open_ai_panel(1);
        ai_app.panels[1].cursor = 0;
        ai_app.ai_logged_in("local\nhttp://localhost:11434/api/tags\n200\n18\n3\ntrue\n32768");
        ai_app.handle_event(Event::AiDone {
            result: Ok(crate::app::AiAnswer {
                file: thread,
                text: String::new(),
                purpose: crate::app::AiUse::Answer,
                tokens: (Some(812), Some(406)),
                model: "qwen3:32b".into(),
                redacted: 0,
            }),
        });
        ai_app.mode = Mode::Normal;
        write("ai", &shot(&mut ai_app, 100, 30));

        // ---- the help, which is half prose and half the live keymap -----------------------
        press(&mut app, "F1");
        write("help", &shot(&mut app, 100, 30));

        // ---- every theme on the same scene: the panels under a pulled-down menu, and a
        //      Pascal unit in the viewer with the lines down its blocks ----------------------
        let delphi = work.parent().expect("a parent").join("delphi");
        std::fs::create_dir_all(delphi.join("__history")).expect("the project");
        for (name, text) in [
            ("Unit1.pas", SAMPLE_PASCAL),
            (
                "Unit1.dfm",
                "object Form1: TForm1\n  Caption = 'Orders'\nend\n",
            ),
            ("Project1.dpr", "program Project1;\nbegin\nend.\n"),
            ("Project1.dproj", "<Project/>\n"),
            ("Orders.inc", "{$DEFINE TRACE}\n"),
            ("Project1.res", "res"),
            ("Unit1.dcu", "dcu"),
        ] {
            let at = delphi.join(name);
            std::fs::write(&at, text).expect("a project file");
            std::fs::File::options()
                .write(true)
                .open(&at)
                .and_then(|f| f.set_modified(when((name.len() % 5) as u64).expect("a time")))
                .expect("the date is set");
        }
        for dir in ["__history", ""] {
            if let Ok(f) = std::fs::File::open(delphi.join(dir)) {
                let _ = f.set_modified(when(4).expect("a time"));
            }
        }
        for (id, _) in fcmd_ui::BUILTIN_THEMES {
            let mut scene = app_on_disk(&delphi);
            scene.theme = Theme::builtin(id).expect("a built-in theme");
            scene.cfg.panel.status_line = true;
            scene.cfg.ui.menu_bar = true;
            for name in ["Unit1.dfm", "Orders.inc"] {
                let row = scene.panels[0].listing.row_of_name(name).expect("the file");
                let id = scene.panels[0].listing.row(row).expect("a row").id;
                scene.panels[0].selection.set(id, true);
            }
            scene.panels[0].cursor = scene.panels[0]
                .listing
                .row_of_name("Unit1.pas")
                .expect("the unit");
            // The last menu, the right panel's, so the left one shows its marked files.
            press(&mut scene, "F9");
            press(&mut scene, "Left");
            press(&mut scene, "Down");
            let top = shot(&mut scene, 100, 22);
            press(&mut scene, "Esc");
            press(&mut scene, "Esc");
            press(&mut scene, "F3");
            // Down to the nested blocks, where the lines are worth seeing.
            for _ in 0..30 {
                press(&mut scene, "Down");
            }
            let bottom = shot(&mut scene, 100, 24);
            press(&mut scene, "Esc");
            // One picture of the two, the one above the other.
            let mut both = Buffer::new(100, 46);
            for (from, y0) in [(&top, 0u16), (&bottom, 22)] {
                for y in 0..from.height() {
                    for x in 0..from.width() {
                        if let (Some(c), Some(to)) = (from.get(x, y), both.cell_mut(x, y + y0)) {
                            *to = *c;
                        }
                    }
                }
            }
            write(&format!("theme-{id}"), &both);
        }

        let _ = std::fs::remove_dir_all(work.parent().expect("a parent"));
    }

    /// A Delphi unit for the theme pictures: a class, a nested `try`, a `case`, a `repeat`,
    /// strings, numbers, a directive and comments of both kinds.
    const SAMPLE_PASCAL: &str = r#"unit Unit1;

{$MODE DELPHI}

interface

uses
  System.SysUtils, System.Classes;

type
  TOrder = class(TObject)
  private
    FTotal: Currency;
    FLines: TStringList;
  public
    constructor Create;
    destructor Destroy; override;
    function Total: Currency;
  end;

implementation

{ TOrder }

constructor TOrder.Create;
begin
  inherited Create;
  FLines := TStringList.Create;
  FTotal := 0;
end;

function TOrder.Total: Currency;
var
  I: Integer;
begin
  Result := 0;
  try
    for I := 0 to FLines.Count - 1 do
    begin
      case FLines[I][1] of
        '+': Result := Result + StrToCurr(Copy(FLines[I], 2, MaxInt));
        '-': Result := Result - StrToCurr(Copy(FLines[I], 2, MaxInt));
      end;
    end;
  except
    // A line that is not a number is left out.
    on E: EConvertError do
      Result := FTotal;
  end;
  repeat
    Dec(I);
  until I < $10;
end;

destructor TOrder.Destroy;
begin
  FLines.Free;
  inherited;
end;

end.
"#;

    /// A small x86-64 ELF with two functions and their symbols, for the hex view and the
    /// disassembler: `square` squares its argument and adds what `helper` returns.
    fn sample_elf() -> Vec<u8> {
        let mut f = vec![0u8; 0x440];
        f[..4].copy_from_slice(b"\x7fELF");
        f[4] = 2; // ELF64
        f[5] = 1; // little endian
        f[6] = 1;
        f[16..18].copy_from_slice(&2u16.to_le_bytes()); // EXEC
        f[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // x86-64
        f[20..24].copy_from_slice(&1u32.to_le_bytes());
        f[24..32].copy_from_slice(&0x401000u64.to_le_bytes()); // e_entry
        f[32..40].copy_from_slice(&64u64.to_le_bytes()); // e_phoff
        f[40..48].copy_from_slice(&0x300u64.to_le_bytes()); // e_shoff
        f[52..54].copy_from_slice(&64u16.to_le_bytes()); // e_ehsize
        f[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        f[56..58].copy_from_slice(&1u16.to_le_bytes()); // e_phnum
        f[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        f[60..62].copy_from_slice(&5u16.to_le_bytes()); // e_shnum
        f[62..64].copy_from_slice(&1u16.to_le_bytes()); // e_shstrndx
                                                        // One PT_LOAD, read and execute, mapping the code at 0x401000.
        let ph = 64;
        f[ph..ph + 4].copy_from_slice(&1u32.to_le_bytes());
        f[ph + 4..ph + 8].copy_from_slice(&5u32.to_le_bytes());
        f[ph + 8..ph + 16].copy_from_slice(&0x200u64.to_le_bytes());
        f[ph + 16..ph + 24].copy_from_slice(&0x401000u64.to_le_bytes());
        f[ph + 24..ph + 32].copy_from_slice(&0x401000u64.to_le_bytes());
        f[ph + 32..ph + 40].copy_from_slice(&0x40u64.to_le_bytes());
        f[ph + 40..ph + 48].copy_from_slice(&0x40u64.to_le_bytes());
        f[ph + 48..ph + 56].copy_from_slice(&0x1000u64.to_le_bytes());
        // .text at 0x200: square(edi) = edi * edi + helper(), then helper() = 7.
        let code: &[u8] = &[
            0x55, // push rbp
            0x48, 0x89, 0xE5, // mov rbp, rsp
            0x89, 0x7D, 0xFC, // mov [rbp-4], edi
            0x8B, 0x45, 0xFC, // mov eax, [rbp-4]
            0x0F, 0xAF, 0xC0, // imul eax, eax
            0x89, 0xC2, // mov edx, eax
            0xE8, 0x0C, 0x00, 0x00, 0x00, // call helper
            0x01, 0xD0, // add eax, edx
            0x5D, // pop rbp
            0xC3, // ret
            0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, // padding to 0x20
            0xB8, 0x07, 0x00, 0x00, 0x00, // helper: mov eax, 7
            0x85, 0xFF, // test edi, edi
            0x74, 0x02, // je +2
            0xF7, 0xD8, // neg eax
            0xC3, // ret
        ];
        f[0x200..0x200 + code.len()].copy_from_slice(code);
        // The names: section names, then symbol names.
        let shstr = b"\0.shstrtab\0.text\0.symtab\0.strtab\0";
        f[0x260..0x260 + shstr.len()].copy_from_slice(shstr);
        let strtab = b"\0square\0helper\0";
        f[0x290..0x290 + strtab.len()].copy_from_slice(strtab);
        // .symtab at 0x2a0: the null symbol, then the two functions.
        let sym = |f: &mut Vec<u8>, at: usize, name: u32, value: u64, size: u64| {
            f[at..at + 4].copy_from_slice(&name.to_le_bytes());
            f[at + 4] = 0x12; // global function
            f[at + 6..at + 8].copy_from_slice(&2u16.to_le_bytes()); // in .text
            f[at + 8..at + 16].copy_from_slice(&value.to_le_bytes());
            f[at + 16..at + 24].copy_from_slice(&size.to_le_bytes());
        };
        sym(&mut f, 0x2a0 + 24, 1, 0x401000, 0x18);
        sym(&mut f, 0x2a0 + 48, 8, 0x401020, 0x0c);
        let sh = |f: &mut Vec<u8>, i: usize, v: [u64; 8]| {
            let at = 0x300 + i * 64;
            f[at..at + 4].copy_from_slice(&(v[0] as u32).to_le_bytes());
            f[at + 4..at + 8].copy_from_slice(&(v[1] as u32).to_le_bytes());
            f[at + 8..at + 16].copy_from_slice(&v[2].to_le_bytes());
            f[at + 16..at + 24].copy_from_slice(&v[3].to_le_bytes());
            f[at + 24..at + 32].copy_from_slice(&v[4].to_le_bytes());
            f[at + 32..at + 40].copy_from_slice(&v[5].to_le_bytes());
            f[at + 40..at + 44].copy_from_slice(&(v[6] as u32).to_le_bytes());
            f[at + 56..at + 64].copy_from_slice(&v[7].to_le_bytes());
        };
        // name, type, flags, addr, offset, size, link, entsize
        sh(&mut f, 1, [1, 3, 0, 0, 0x260, shstr.len() as u64, 0, 0]);
        sh(&mut f, 2, [11, 1, 6, 0x401000, 0x200, 0x40, 0, 0]);
        sh(&mut f, 3, [17, 2, 0, 0, 0x2a0, 72, 4, 24]);
        sh(&mut f, 4, [25, 3, 0, 0, 0x290, strtab.len() as u64, 0, 0]);
        // sh_info of .symtab: the first global symbol.
        f[0x300 + 3 * 64 + 44..0x300 + 3 * 64 + 48].copy_from_slice(&1u32.to_le_bytes());
        f
    }

    /// A 24-bit BMP drawn here, so the picture needs no file and no encoder: a sky that goes
    /// from deep blue to orange, a sun, and the sea under it.
    fn sample_bmp(w: u32, h: u32) -> Vec<u8> {
        let row = (w * 3).div_ceil(4) * 4;
        let size = 54 + row * h;
        let mut b = Vec::with_capacity(size as usize);
        b.extend_from_slice(b"BM");
        b.extend_from_slice(&size.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&54u32.to_le_bytes());
        b.extend_from_slice(&40u32.to_le_bytes());
        b.extend_from_slice(&(w as i32).to_le_bytes());
        b.extend_from_slice(&(h as i32).to_le_bytes()); // bottom-up
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&24u16.to_le_bytes());
        b.extend_from_slice(&[0u8; 24]);
        let horizon = h as f32 * 0.42;
        for y in 0..h {
            let fy = y as f32; // 0 is the bottom row
            for x in 0..w {
                let fx = x as f32;
                let (r, g, bl) = if fy < horizon {
                    // The sea: dark, with the sun's light down the middle.
                    let glow = (1.0 - ((fx - w as f32 / 2.0).abs() / 14.0)).max(0.0)
                        * (!(fy as u32 + x / 3).is_multiple_of(3) as u8 as f32);
                    (
                        (20.0 + 200.0 * glow) as u8,
                        (40.0 + 120.0 * glow) as u8,
                        (90.0 + 40.0 * glow) as u8,
                    )
                } else {
                    let t = (fy - horizon) / (h as f32 - horizon); // 0 at the horizon
                    let sky = (
                        (250.0 - 220.0 * t) as u8,
                        (120.0 - 90.0 * t) as u8,
                        (40.0 + 110.0 * t) as u8,
                    );
                    let (dx, dy) = (fx - w as f32 / 2.0, (fy - horizon - 6.0) * 1.0);
                    if dx * dx + dy * dy < 110.0 {
                        (255, 220, 80)
                    } else {
                        sky
                    }
                };
                b.extend_from_slice(&[bl, g, r]);
            }
            b.extend(std::iter::repeat_n(0u8, (row - w * 3) as usize));
        }
        b
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
        assert!(
            out.contains("Cargo.toml  412  "),
            "the size in bytes: {out}"
        );
    }

    /// A plugin command that cannot run here is drawn in the menu's dim ink, so the list says
    /// at a glance which rows will do something.
    #[test]
    fn a_plugin_command_that_cannot_run_here_is_greyed_out() {
        let mut app = app("far", "en");
        let cmd = |id: &str, when: fcmd_plug::When| crate::app::PluginCommand {
            plugin: "demo".into(),
            id: format!("user.demo.{id}"),
            label: format!("Command {id}"),
            scope: fcmd_cmd::Scope::Panel,
            program: std::path::PathBuf::from("/bin/true"),
            needs: Vec::new(),
            timeout: None,
            confirm: None,
            when,
        };
        app.plugin_commands = vec![
            cmd("ready", Default::default()),
            cmd(
                "blocked",
                fcmd_plug::When {
                    here: vec!["nothing-here-by-this-name".into()],
                    ..Default::default()
                },
            ),
        ];
        app.handle_input(Input::Key("F11".parse().unwrap()));
        let mut buf = Buffer::new(100, 30);
        draw(&mut app, &mut buf);
        let text = buf.to_text();
        let row_of = |what: &str| text.lines().position(|l| l.contains(what)).unwrap() as u16;
        let col_of = |what: &str| {
            let line = text.lines().find(|l| l.contains(what)).unwrap();
            line[..line.find(what).unwrap()].chars().count() as u16
        };
        let blocked = buf
            .get(col_of("Command blocked"), row_of("Command blocked"))
            .unwrap()
            .style;
        assert_eq!(blocked.fg, app.theme.color("menu.dim.fg"));
        // The one that can run is not, whether or not the cursor is on it.
        app.handle_input(Input::Key("Down".parse().unwrap()));
        app.handle_input(Input::Key("Down".parse().unwrap()));
        draw(&mut app, &mut buf);
        let ready = buf
            .get(col_of("Command ready"), row_of("Command ready"))
            .unwrap()
            .style;
        assert_ne!(ready.fg, app.theme.color("menu.dim.fg"));
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
                when: Default::default(),
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
                when: Default::default(),
            })
            .collect();
        app.handle_input(Input::Key("F11".parse().unwrap()));
        for _ in 0..12 {
            app.handle_input(Input::Key("Down".parse().unwrap()));
        }
        println!("{}", frame(&mut app, 80, 20));
    }

    /// Midnight Commander's learn keys: the cell of a key that has arrived is marked, and
    /// the one that never arrives is the answer to "why does that key do nothing".
    #[test]
    fn the_key_test_marks_what_the_terminal_sent_and_says_what_it_does() {
        let mut app = app("far", "en");
        app.run(fcmd_cmd::Command::AppLearnKeys);
        assert!(matches!(app.mode, crate::app::Mode::LearnKeys(_)));

        // Nothing has arrived yet, so no cell is marked.
        let before = frame(&mut app, 100, 30);
        assert!(before.contains("F1-F12"), "{before}");
        assert!(!before.contains("1·"), "{before}");

        press(&mut app, "F5");
        press(&mut app, "Shift+F3");
        let after = frame(&mut app, 100, 30);
        assert!(after.contains("5·"), "F5 arrived: {after}");
        assert!(after.contains("3·"), "and Shift+F3: {after}");
        // And the last one says what it does here, by the name of the command.
        assert!(
            after.contains("file.copy") || after.contains("Shift+F3"),
            "{after}"
        );

        // One Esc is a key like any other; two leave.
        press(&mut app, "Esc");
        assert!(matches!(app.mode, crate::app::Mode::LearnKeys(_)));
        press(&mut app, "Esc");
        assert!(app.mode.is_normal());
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

    /// `$_` in the prompt puts what comes before it on the row above the command line, taken
    /// from the panels (FAR's #970).
    #[test]
    fn a_prompt_with_a_new_line_takes_a_row_above_the_command_line() {
        let mut app = app("commander", "en");
        app.cfg.cmdline.prompt = "FIRSTLINE$_SECOND$g".into();
        let out = frame(&mut app, 80, 24);
        let rows: Vec<&str> = out.lines().collect();
        let first = rows.iter().position(|r| r.starts_with("FIRSTLINE"));
        let second = rows.iter().position(|r| r.starts_with("SECOND>"));
        assert!(
            matches!((first, second), (Some(a), Some(b)) if b == a + 1),
            "{out}"
        );
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
        for id in [
            "commander",
            "black",
            "white",
            "matrix",
            "tron",
            "blade",
            "github-dark",
            "delphi-dark-2027",
            "delphi-inspiration",
        ] {
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

    /// A job running in the background is counted in the top row, next to where the clock
    /// goes, so nobody forgets one is there (SPEC 03 §5).
    #[test]
    fn a_job_in_the_background_is_counted_in_the_corner() {
        let mut app = app("commander", "en");
        let top = |app: &mut App| frame(app, 80, 24).lines().next().unwrap().to_string();
        assert!(!top(&mut app).contains('⧗'));
        app.start_job(fcmd_jobs::JobSpec::new(
            fcmd_jobs::JobKind::Copy,
            vec![fcmd_core::VPath::local("/home/mb/a.txt")],
            Some(fcmd_core::VPath::local("/home/mb/one")),
        ));
        // On screen it is the box, not a count.
        assert!(!top(&mut app).contains('⧗'));
        app.mode = Mode::Normal;
        assert!(top(&mut app).contains("⧗1"), "{}", top(&mut app));
    }

    /// The colours of a hex row and of an instruction come from what each piece is.
    #[test]
    fn hex_and_disassembly_rows_are_coloured_by_what_their_pieces_are() {
        use fcmd_view::RowKind;
        let key_of = |runs: &[(usize, usize, &'static str)], text: &str, piece: &str| {
            let at = text.find(piece).unwrap();
            runs.iter()
                .find(|(s, e, _)| *s <= at && at < *e)
                .map(|(_, _, k)| *k)
        };
        let hex = "00000200  55 00 FF 0A 7D FC 8B 45  |U....}.E|";
        let runs = binary_runs(RowKind::Hex, hex);
        assert_eq!(key_of(&runs, hex, "00000200"), Some("syntax.type"));
        assert_eq!(
            key_of(&runs, hex, "00 FF"),
            Some("syntax.comment"),
            "zero is dim"
        );
        assert_eq!(key_of(&runs, hex, "FF"), Some("syntax.keyword"));
        assert_eq!(
            key_of(&runs, hex, "0A"),
            Some("syntax.number"),
            "a control byte"
        );
        assert_eq!(
            key_of(&runs, hex, "FC"),
            Some("syntax.preproc"),
            "the high half"
        );
        assert_eq!(key_of(&runs, hex, "U."), Some("syntax.string"));

        let code = "0000000000401010  E8 0C 00 00 00            call    0000000000401020h <helper>";
        let runs = binary_runs(RowKind::Code, code);
        assert_eq!(key_of(&runs, code, "0000000000401010"), Some("syntax.type"));
        assert_eq!(key_of(&runs, code, "E8 0C"), Some("syntax.comment"));
        assert_eq!(
            key_of(&runs, code, "call"),
            Some("syntax.preproc"),
            "flow stands out"
        );
        assert_eq!(
            key_of(&runs, code, "0000000000401020h"),
            Some("syntax.number")
        );
        assert_eq!(key_of(&runs, code, "<helper>"), Some("syntax.function"));
        let code = "0000000000401004  89 7D FC                  mov     dword ptr [rbp-4],edi";
        let runs = binary_runs(RowKind::Code, code);
        assert_eq!(key_of(&runs, code, "mov"), Some("syntax.keyword"));
        assert_eq!(key_of(&runs, code, "dword"), Some("syntax.comment"));
        assert_eq!(key_of(&runs, code, "rbp"), Some("syntax.string"));
        assert_eq!(key_of(&runs, code, "edi"), Some("syntax.string"));
        let label = "0000000000401000 <square>:";
        let runs = binary_runs(RowKind::Label, label);
        assert_eq!(key_of(&runs, label, "<square>"), Some("syntax.function"));
        // A text row is the source colouring's business, not this.
        assert!(binary_runs(RowKind::Text, "anything").is_empty());
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

    /// The dropdown hangs right under the bar, and a menu taller than the window scrolls
    /// with the cursor: its last item can be reached and seen.
    #[test]
    fn a_menu_taller_than_the_window_scrolls_and_sits_under_the_bar() {
        let mut app = app("far", "en");
        let items = crate::menu::menus()[2].items.len();
        app.mode = crate::app::Mode::Menu {
            menu: 2,
            item: 0,
            open: true,
        };
        let mut buf = Buffer::new(80, 12);
        draw(&mut app, &mut buf);
        let line = |buf: &Buffer, y: u16| -> String {
            (0..80)
                .filter_map(|x| buf.get(x, y))
                .map(|c| c.ch)
                .collect()
        };
        assert!(line(&buf, 1).contains('╔'), "no gap: {}", line(&buf, 1));
        let last = (0..items)
            .rev()
            .find(|i| {
                matches!(
                    crate::menu::menus()[2].items[*i],
                    crate::menu::Entry::Item(_)
                )
            })
            .unwrap();
        let crate::menu::Entry::Item(item) = &crate::menu::menus()[2].items[last] else {
            unreachable!()
        };
        let label = fcmd_ui::mnemonic(&app.i18n.t(item.label)).0;
        let screen = |buf: &Buffer| (0..12).map(|y| line(buf, y)).collect::<Vec<_>>().join("\n");
        assert!(!screen(&buf).contains(&label), "{}", screen(&buf));
        assert!(screen(&buf).contains('▼'), "{}", screen(&buf));
        app.mode = crate::app::Mode::Menu {
            menu: 2,
            item: last,
            open: true,
        };
        let mut buf = Buffer::new(80, 12);
        draw(&mut app, &mut buf);
        assert!(screen(&buf).contains(&label), "{}", screen(&buf));
        assert!(screen(&buf).contains('▲'), "{}", screen(&buf));
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

    /// A line that differs is painted whole in the side-by-side diff: red under what went,
    /// green under what came, as well as its `-` or `+`.
    #[test]
    fn the_diff_paints_the_lines_that_differ() {
        let mut app = app("far", "en");
        let view = fcmd_view::Diff::new(
            "a.txt".into(),
            b"same\ngone\n",
            "b.txt".into(),
            b"same\ncame\nnew\n",
            fcmd_core::diff::Ignore::default(),
        );
        app.mode = crate::app::Mode::Diff(Box::new(view));
        let buf = frame_buf(&mut app, 80, 20);
        let removed = app.theme.color("diff.removed.bg");
        let added = app.theme.color("diff.added.bg");
        let find = |word: &str| {
            (0..20u16)
                .find_map(|y| {
                    let line: String = (0..80)
                        .filter_map(|x| buf.get(x, y))
                        .map(|c| c.ch)
                        .collect();
                    line.find(word)
                        .map(|x| (line[..x].chars().count() as u16, y))
                })
                .unwrap_or_else(|| panic!("{word} is not on screen"))
        };
        let (x, y) = find("new");
        assert_eq!(
            buf.get(x, y).unwrap().style.bg,
            added,
            "an added line is green"
        );
        assert_eq!(
            buf.get(x + 10, y).unwrap().style.bg,
            added,
            "the whole line, not only the text"
        );
        let (x, y) = find("same");
        assert_ne!(
            buf.get(x, y).unwrap().style.bg,
            removed,
            "a line in both is plain"
        );
        assert_ne!(buf.get(x, y).unwrap().style.bg, added);
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
            name: String::new(),
            report: None,
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

    /// The list of what a dead session left: each job with what it was doing and how far it
    /// had got, an unreadable journal as a row of its own, and a footer naming the two keys.
    #[test]
    fn the_unfinished_jobs_are_listed_with_how_far_each_had_got() {
        let mut app = app("far", "en");
        let spec = fcmd_jobs::JobSpec::new(
            fcmd_jobs::JobKind::Copy,
            vec![
                fcmd_core::VPath::local("/home/mb/project/film.mkv"),
                fcmd_core::VPath::local("/home/mb/project/notes.txt"),
            ],
            Some(fcmd_core::VPath::local("/media/stick")),
        );
        let mut journal = fcmd_jobs::Journal::of(&spec, 0).expect("a copy keeps a journal");
        journal
            .done
            .insert(fcmd_core::VPath::local("/media/stick").join("notes.txt"));
        app.offer_left_jobs(fcmd_jobs::journal::Left {
            journals: vec![(std::path::PathBuf::from("7-1.journal"), journal)],
            problems: vec![(
                std::path::PathBuf::from("7-2.journal"),
                fcmd_jobs::JournalError::Foreign,
            )],
        });
        app.run(fcmd_cmd::Command::JobJournal);
        let out = frame(&mut app, 110, 20);
        assert!(out.contains("Unfinished jobs"), "{out}");
        let row = out
            .lines()
            .find(|l| l.contains("film.mkv"))
            .expect("the job's row");
        assert!(row.contains("(+1)"), "the second source is counted: {row}");
        assert!(row.contains("files done: 1"), "{row}");
        assert!(
            out.contains("cannot be read: 7-2.journal"),
            "the unreadable one is listed too: {out}"
        );
        assert!(
            out.contains("Del discard"),
            "the footer names its keys: {out}"
        );
    }

    /// The dialog's two new rows and its middle button, and the list the button opens.
    #[test]
    fn the_sync_dialog_offers_a_name_a_report_and_what_was_saved() {
        let mut app = app("far", "en");
        app.run(fcmd_cmd::Command::ToolSyncDirs);
        let out = frame(&mut app, 100, 30);
        assert!(out.contains("Write a report of the run to"), "{out}");
        assert!(out.contains("Save this pair as"), "{out}");
        let buttons = out
            .lines()
            .find(|l| l.contains("Saved…"))
            .expect("the row of buttons");
        assert!(
            buttons.contains("OK") && buttons.contains("Cancel"),
            "{out}"
        );
        press(&mut app, "Esc");

        app.cfg.syncs = vec![
            fcmd_config::SyncCfg {
                name: "Evening backup".to_string(),
                left: "work".to_string(),
                right: "stick".to_string(),
                ..fcmd_config::SyncCfg::default()
            },
            fcmd_config::SyncCfg {
                name: "Photos".to_string(),
                left: "photos".to_string(),
                right: "nas".to_string(),
                both_ways: true,
                ..fcmd_config::SyncCfg::default()
            },
        ];
        app.run(fcmd_cmd::Command::ToolSyncSaved);
        let out = frame(&mut app, 100, 20);
        assert!(out.contains("Saved synchronisations"), "{out}");
        assert!(
            out.contains("Evening backup: work \u{2192} stick"),
            "one way is one arrow: {out}"
        );
        assert!(
            out.contains("Photos: photos \u{2194} nas"),
            "both ways is the double one: {out}"
        );
        assert!(
            out.contains("F4 change"),
            "the footer names its keys: {out}"
        );
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

    /// The editor draws the lines down Pascal's blocks that the viewer draws, in the same
    /// colour, and keeps them in step with the text: a block closed by typing loses its line
    /// in the next frame.
    #[test]
    fn the_editor_draws_block_guides_and_follows_the_edits() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("Unit1.pas");
        std::fs::write(
            &file,
            "begin\n  try\n    x := 1;\n\n  finally\n    y;\n  end;\nend;\n",
        )
        .unwrap();
        let mut app = super::tests::app("far", "en");
        app.mode =
            crate::app::Mode::Editor(Box::new(fcmd_edit::Editor::open(&file).expect("it opens")));
        let buf = frame_buf(&mut app, 60, 14);
        let at = |buf: &Buffer, x: u16, y: u16| {
            let cell = buf.get(x, y).unwrap();
            (cell.ch, cell.style.fg)
        };
        let guide = app.theme.color("syntax.guide");
        // Row 0 is the status line, so line n of the file is row n + 1.
        assert_eq!(at(&buf, 0, 1).0, 'b', "the opening line has none");
        assert_eq!(at(&buf, 0, 2), ('│', guide), "inside begin");
        assert_eq!(at(&buf, 0, 3), ('│', guide));
        assert_eq!(at(&buf, 2, 3), ('│', guide), "inside try as well");
        assert_eq!(at(&buf, 2, 4), ('│', guide), "through an empty line too");
        assert_eq!(at(&buf, 2, 5).0, 'f', "never over text: `finally` is there");
        assert_eq!(at(&buf, 2, 6), ('│', guide));
        assert_eq!(at(&buf, 0, 7), ('│', guide), "begin goes on past try's end");
        assert_eq!(at(&buf, 2, 7).0, 'e', "the `end` itself is text");
        assert_eq!(at(&buf, 0, 8).0, 'e');

        // Take `try` out, and its line is gone from the rows below in the same frame.
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.cursor = fcmd_edit::Position::new(1, 0);
            editor.delete_line().unwrap();
        }
        let buf = frame_buf(&mut app, 60, 14);
        assert_eq!(at(&buf, 0, 2), ('│', guide), "begin's line is still there");
        assert_ne!(at(&buf, 2, 3).0, '│', "try's line outlived try");

        // Undo puts `try` back, and its line with it.
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.undo();
        }
        let buf = frame_buf(&mut app, 60, 14);
        assert_eq!(at(&buf, 2, 3), ('│', guide), "undo brought it back");

        // Scrolled one column sideways the lines move with the text: try's is in column 1,
        // and begin's, which was in the column that scrolled off, is not drawn anywhere.
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.cursor = fcmd_edit::Position::new(2, 1);
            editor.left = 1;
        }
        let buf = frame_buf(&mut app, 60, 14);
        assert_eq!(at(&buf, 1, 3), ('│', guide), "try's line moved left");
        assert_eq!(at(&buf, 0, 3).0, ' ', "begin's line came along into view");
        {
            let editor = app.mode.editor_mut().expect("the editor");
            editor.left = 0;
        }

        // And the switch that turns the colours off turns these off with them.
        app.cfg.ui.syntax = false;
        let buf = frame_buf(&mut app, 60, 14);
        assert_eq!(at(&buf, 0, 2).0, ' ');
        assert_eq!(at(&buf, 2, 3).0, ' ');
    }

    /// A file that is not Pascal has blocks of its own and no guides: the editor draws them
    /// for exactly the languages the viewer does.
    #[test]
    fn the_editor_draws_no_guides_where_the_viewer_draws_none() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("main.rs");
        std::fs::write(&file, "fn main() {\n    if x {\n        y();\n    }\n}\n").unwrap();
        let mut app = super::tests::app("far", "en");
        app.mode =
            crate::app::Mode::Editor(Box::new(fcmd_edit::Editor::open(&file).expect("it opens")));
        let screen = frame(&mut app, 60, 10);
        assert!(!screen.contains('│'), "{screen}");
    }

    /// The question about a file changed on disk has four answers, and a button that does
    /// not fit is an answer nobody can give: all four are on one row of an 80-column
    /// screen, in every language.
    #[test]
    fn the_changed_on_disk_question_shows_its_four_buttons_in_every_language() {
        for (tag, _) in fcmd_i18n::LANGUAGES {
            let mut app = super::tests::app("far", tag);
            let confirm = crate::app::Confirm {
                title: app.i18n.t("editor-changed-title"),
                lines: vec![app
                    .i18n
                    .t_args("editor-changed-body", &[("name", "notes.txt".into())])],
                action: crate::app::ConfirmAction::OverwriteChanged,
                button: 0,
            };
            let labels = confirm.buttons(&app.i18n);
            assert_eq!(labels.len(), 4, "{tag}");
            app.mode = crate::app::Mode::Confirm(confirm);
            let screen = frame(&mut app, 80, 24);
            let row = screen
                .lines()
                .find(|line| line.contains(labels[0].as_str()) && line.contains(labels[3].as_str()))
                .unwrap_or_else(|| panic!("{tag}: the buttons are not on one row\n{screen}"));
            for label in &labels {
                assert!(row.contains(label.as_str()), "{tag}: no {label} in {row:?}");
            }
        }
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
        assert!(with.contains("╟"), "no rule above the status row: {with}");
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
            // The default button in braces, as FAR marks it; the others in brackets.
            out.contains("{ Overwrite }") && out.contains("[ Cancel ]"),
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

        let mut buf = Buffer::new(80, 10);
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
        for id in [
            "commander",
            "black",
            "white",
            "matrix",
            "tron",
            "blade",
            "github-dark",
            "delphi-dark-2027",
            "delphi-inspiration",
        ] {
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

    /// A question the hex editor asks is drawn over the bytes it is about: the match a
    /// replacement stops at has to be on the screen while one decides about it.
    #[test]
    fn a_replacement_asks_about_each_match_over_the_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("program.bin");
        std::fs::write(&path, b"....mz....mz....").unwrap();
        let mut app = app("commander", "en");
        app.open_hex(&path);
        let key = |app: &mut App, k: &str| app.handle_input(Input::Key(k.parse().unwrap()));

        // The key bar says the two keys are there.
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("Search") && out.contains("Replac"), "{out}");

        key(&mut app, "F7");
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("Search for"), "{out}");
        assert!(out.contains("00000000"), "the bytes stay behind it: {out}");
        key(&mut app, "Esc");

        key(&mut app, "Ctrl+F7");
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("Replace with"), "{out}");
        assert!(out.contains("All"), "{out}");
        for c in "mz".chars() {
            key(&mut app, &c.to_string());
        }
        key(&mut app, "Down");
        for c in "pe".chars() {
            key(&mut app, &c.to_string());
        }
        key(&mut app, "Enter");
        let out = frame(&mut app, 100, 24);
        assert!(
            out.contains("Replace 6D 7A at 00000004 with 70 65?"),
            "{out}"
        );
        assert!(out.contains("Skip"), "{out}");
        assert!(out.contains("00000000"), "{out}");
        key(&mut app, "Enter");
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("at 0000000A"), "on to the second: {out}");
        key(&mut app, "Esc");
        let out = frame(&mut app, 100, 24);
        assert!(out.contains("....pe....mz...."), "{out}");
        assert!(app.status.contains('1'), "{}", app.status);
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

    // ── pictures in real pixels, and films (SPEC 05 §1.14a, §1.15) ─────────────────────────

    /// The environment of a terminal, for the tests that choose a protocol from one.
    fn terminal(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    /// Serve the pictures the app asked to have encoded, as the loop's workers do.
    fn encode_pictures(app: &mut App) -> usize {
        let mut served = 0;
        for request in app.take_requests() {
            if let crate::app::Request::Pixels(ask) = request {
                let drawn = crate::pixels::encode(&ask);
                app.handle_event(Event::Pixels {
                    key: ask.key,
                    drawn,
                });
                served += 1;
            }
        }
        served
    }

    #[test]
    fn with_pixels_off_a_picture_is_half_blocks_even_in_a_terminal_that_draws_them() {
        let (_dir, mut app) = app_with_viewer("sunset.bmp", &sample_bmp(64, 48));
        app.cfg.viewer.pixels = "off".into();
        // kitty itself, with a cell size the system reported: everything `auto` would need.
        let (protocol, cell) = crate::pixels_for(
            &app.cfg.viewer,
            Some((8, 16)),
            terminal(&[("TERM", "xterm-kitty"), ("KITTY_WINDOW_ID", "1")]),
        );
        assert_eq!(protocol, None);
        app.pixels.configure(protocol, cell);

        let out = frame(&mut app, 80, 24);
        assert!(
            out.contains('▀'),
            "the picture is drawn in half blocks:\n{out}"
        );
        assert!(app.pixels.placements().is_empty());
        assert_eq!(
            encode_pictures(&mut app),
            0,
            "and nothing is asked of a worker"
        );
    }

    #[test]
    fn the_setting_and_the_terminal_decide_the_protocol_and_the_size_of_a_cell() {
        let mut cfg = Config::default().viewer;
        let kitty = &[("TERM", "xterm-kitty")];
        let foot = &[("TERM", "foot")];
        let wt = &[("TERM", "xterm-256color"), ("WT_SESSION", "1")];
        let plain = &[("TERM", "xterm-256color")];
        use fcmd_tui::Pixels;
        // `auto`, the default: what the terminal is known to draw.
        assert_eq!(
            crate::pixels_for(&cfg, Some((9, 18)), terminal(kitty)),
            (Some(Pixels::Kitty), (9, 18))
        );
        assert_eq!(
            crate::pixels_for(&cfg, None, terminal(kitty)),
            (Some(Pixels::Kitty), (8, 16)),
            "kitty is told the cells, so a cell of unknown size is no obstacle"
        );
        assert_eq!(
            crate::pixels_for(&cfg, Some((9, 18)), terminal(foot)),
            (Some(Pixels::Sixel), (9, 18))
        );
        assert_eq!(
            crate::pixels_for(&cfg, None, terminal(foot)).0,
            None,
            "sixel is not guessed at"
        );
        assert_eq!(
            crate::pixels_for(&cfg, Some((9, 18)), terminal(plain)).0,
            None
        );
        assert_eq!(crate::pixels_for(&cfg, Some((9, 18)), terminal(wt)).0, None);
        // Asked for by name, in a terminal the environment cannot vouch for. Windows
        // Terminal draws sixel on cells of ten by twenty whatever its font is.
        cfg.pixels = "sixel".into();
        assert_eq!(
            crate::pixels_for(&cfg, Some((9, 18)), terminal(wt)),
            (Some(Pixels::Sixel), (10, 20))
        );
        assert_eq!(
            crate::pixels_for(&cfg, None, terminal(plain)),
            (Some(Pixels::Sixel), (8, 16))
        );
        // And what the configuration states about a cell beats everything.
        cfg.cell_width = 7;
        cfg.cell_height = 15;
        assert_eq!(
            crate::pixels_for(&cfg, Some((9, 18)), terminal(wt)),
            (Some(Pixels::Sixel), (7, 15))
        );
    }

    #[test]
    fn where_the_terminal_draws_pixels_the_picture_is_placed_once_it_is_encoded() {
        let (_dir, mut app) = app_with_viewer("sunset.bmp", &sample_bmp(640, 480));
        let (protocol, cell) = crate::pixels_for(
            &app.cfg.viewer,
            Some((8, 16)),
            terminal(&[("TERM", "xterm-kitty")]),
        );
        app.pixels.configure(protocol, cell);

        // The first frame cannot wait for the encoder: half blocks, and one ask.
        let out = frame(&mut app, 80, 24);
        assert!(out.contains('▀'), "{out}");
        assert!(app.pixels.placements().is_empty());
        assert_eq!(encode_pictures(&mut app), 1);

        // The next has the pixels: the cells are blank, and the backend gets the picture.
        // The body is 80×22 cells of 8×16, which is 640×352 pixels: the height is what runs
        // out, so the picture is 469 pixels across — 59 cells — and all 22 rows.
        let mut buf = Buffer::new(80, 24);
        draw(&mut app, &mut buf);
        assert!(!buf.to_text().contains('▀'), "{}", buf.to_text());
        let placed = app.pixels.placements().to_vec();
        assert_eq!(placed.len(), 1);
        let p = &placed[0];
        assert_eq!((p.cols, p.rows), (59, 22));
        assert_eq!((p.x, p.y), (10, 1));
        assert_eq!(p.protocol, fcmd_tui::Pixels::Kitty);
        assert!(p.data.starts_with(b"\x89PNG"));
        assert_eq!(
            encode_pictures(&mut app),
            0,
            "encoded once, not once a frame"
        );

        // The same picture on the next frame is the same placement, which is what lets the
        // backend leave it alone.
        draw(&mut app, &mut buf);
        assert_eq!(app.pixels.placements(), &placed[..]);

        // A dialog across it — `F7`, the search — and the picture is half blocks again
        // under the dialog, because an image would lie on top of it. They are drawn in the
        // cells the pixels had, so the picture blurs where it stands.
        press(&mut app, "F7");
        draw(&mut app, &mut buf);
        assert!(app.pixels.placements().is_empty());
        assert!(buf.to_text().contains('▀'), "{}", buf.to_text());
        // The dialog closes and the pixels are back, with nothing encoded again.
        press(&mut app, "Esc");
        draw(&mut app, &mut buf);
        assert_eq!(app.pixels.placements().len(), 1);
        assert_eq!(encode_pictures(&mut app), 0);

        // Leaving the viewer leaves nothing placed.
        press(&mut app, "Esc");
        draw(&mut app, &mut buf);
        assert!(app.pixels.placements().is_empty());
    }

    #[test]
    fn the_quick_view_draws_a_picture_in_pixels_too() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("sunset.bmp"), sample_bmp(64, 48)).unwrap();
        let mut app = app_on_disk(dir.path());
        app.pixels.configure(Some(fcmd_tui::Pixels::Sixel), (8, 16));
        press(&mut app, "Down");
        press(&mut app, "Ctrl+Q");
        let mut buf = Buffer::new(100, 30);
        draw(&mut app, &mut buf);
        assert_eq!(encode_pictures(&mut app), 1);
        draw(&mut app, &mut buf);
        let placed = app.pixels.placements();
        assert_eq!(placed.len(), 1);
        assert!(placed[0].x >= 50, "in the other panel: {:?}", placed[0]);
        assert!(placed[0].data.starts_with(b"\x1bP0;1;0q"));
    }

    /// The first bytes of an MP4: enough for the viewer to ask whether it is a film.
    fn mp4_head() -> Vec<u8> {
        let mut bytes = b"\0\0\0\x18ftypisom\0\0\0\0isommp42".to_vec();
        bytes.extend_from_slice(&[0u8; 256]);
        bytes
    }

    fn a_film() -> fcmd_view::media::Film {
        fcmd_view::media::Film {
            seconds: 120.0,
            codec: "H.264/AVC".into(),
            width: 64,
            height: 48,
        }
    }

    /// A frame made here: a picture of one colour, at a moment.
    fn a_frame(at: f64, shade: u8) -> crate::video::Frame {
        crate::video::Frame {
            film: a_film(),
            at,
            picture: fcmd_view::picture::Picture::from_rgba(
                64,
                48,
                "PNG",
                [shade, 0, 255 - shade, 255].repeat(64 * 48),
            )
            .expect("the right number of bytes"),
        }
    }

    /// The one frame the app asked for, taken off its requests.
    fn frame_asked(app: &mut App) -> (VPath, crate::video::Ask, bool) {
        let mut found = Vec::new();
        for request in app.take_requests() {
            match request {
                crate::app::Request::VideoFrame { key, ask } => found.push((key, ask, false)),
                crate::app::Request::VideoPlay { key, ask } => found.push((key, ask, true)),
                _ => {}
            }
        }
        assert_eq!(found.len(), 1, "one frame asked for: {found:?}");
        found.remove(0)
    }

    #[test]
    fn a_film_opens_on_the_frame_a_tenth_of_the_way_in_and_steps_by_tenths() {
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        let out = frame(&mut app, 80, 24);
        assert!(
            !out.contains('▀'),
            "no frame yet: the bytes, as before\n{out}"
        );
        // `F3` asked a worker for the first frame, sized to the window in pixels.
        let (key, ask, play) = frame_asked(&mut app);
        assert!(!play);
        assert_eq!(ask.at, crate::video::At::Fraction(0.1));
        assert_eq!(ask.film, None, "the worker reads the container");
        assert_eq!(ask.bounds, (80 * 8, 22 * 16));

        // The frame arrives: the viewer turns to it, and the top line says where it is, how
        // long the film runs and what it is encoded as.
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: ask.number,
            result: Ok(a_frame(12.0, 40)),
        });
        let out = frame(&mut app, 80, 24);
        assert!(out.contains('▀'), "{out}");
        let top = out.lines().next().unwrap();
        assert!(top.contains("00:12 / 02:00  H.264/AVC 64×48"), "{top}");

        // `PgDn` is a tenth of the film on; two of them before any answer are two tenths.
        press(&mut app, "PgDn");
        let (_, first, _) = frame_asked(&mut app);
        assert_eq!(first.at, crate::video::At::Seconds(24.0));
        assert_eq!(
            first.film,
            Some(a_film()),
            "the container is not read twice"
        );
        press(&mut app, "Right");
        let (_, second, _) = frame_asked(&mut app);
        assert_eq!(second.at, crate::video::At::Seconds(36.0));
        // The answer to the first is late, and dropped; the second's is shown.
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: first.number,
            result: Ok(a_frame(24.0, 80)),
        });
        assert!(frame(&mut app, 80, 24).contains("00:12 / 02:00"));
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: second.number,
            result: Ok(a_frame(36.0, 120)),
        });
        assert!(frame(&mut app, 80, 24).contains("00:36 / 02:00"));

        // Back, and the two ends.
        press(&mut app, "PgUp");
        assert_eq!(frame_asked(&mut app).1.at, crate::video::At::Seconds(24.0));
        press(&mut app, "Home");
        assert_eq!(frame_asked(&mut app).1.at, crate::video::At::Seconds(0.0));
        press(&mut app, "End");
        assert_eq!(frame_asked(&mut app).1.at, crate::video::At::Seconds(119.0));

        // `F4` still goes on to the bytes, where the same keys turn pages again.
        press(&mut app, "F4");
        press(&mut app, "PgDn");
        assert!(app.take_requests().is_empty());
    }

    #[test]
    fn a_film_left_on_its_frame_opens_as_its_bytes_until_a_frame_comes_again() {
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        let before = frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key,
            number: ask.number,
            result: Ok(a_frame(12.0, 40)),
        });
        // Left while the frame was showing, so what is remembered of the file is "a picture".
        press(&mut app, "Esc");
        press(&mut app, "F3");
        // Opened again, it is not a picture mode with no picture in it: on a machine where
        // `ffmpeg` has gone since, that is how it would stay.
        let again = frame(&mut app, 80, 24);
        assert_eq!(again, before);
        assert!(!again.contains("not a picture"), "{again}");
        // And the frame is asked for afresh.
        assert_eq!(frame_asked(&mut app).1.at, crate::video::At::Fraction(0.1));
    }

    #[test]
    fn space_plays_a_film_and_pauses_it() {
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: ask.number,
            result: Ok(a_frame(12.0, 40)),
        });

        press(&mut app, "Space");
        let (_, playing, play) = frame_asked(&mut app);
        assert!(play, "a film to play, not a frame to fetch");
        assert_eq!(playing.at, crate::video::At::Seconds(12.0));
        assert!(app.status.contains("8 frames a second"), "{}", app.status);
        // Frames arrive under the one number, each moving the clock on.
        for (at, shade) in [(12.0, 40), (12.125, 41), (13.0, 48)] {
            app.handle_event(Event::VideoFrame {
                key: key.clone(),
                number: playing.number,
                result: Ok(a_frame(at, shade)),
            });
        }
        assert!(frame(&mut app, 80, 24).contains("00:13 / 02:00"));

        // `Space` again: paused where it was, and a frame still on its way is not shown.
        press(&mut app, "Space");
        assert!(app.take_requests().is_empty(), "pausing asks for nothing");
        assert!(app.status.contains("00:13"), "{}", app.status);
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: playing.number,
            result: Ok(a_frame(14.0, 56)),
        });
        assert!(frame(&mut app, 80, 24).contains("00:13 / 02:00"));

        // Played to the end, the film says so and `Space` starts it over.
        press(&mut app, "Space");
        let (_, again, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: again.number,
            result: Ok(a_frame(119.5, 200)),
        });
        app.handle_event(Event::VideoEnded {
            key,
            number: again.number,
            result: Ok(()),
        });
        assert!(app.status.contains("ended"), "{}", app.status);
        press(&mut app, "Space");
        assert_eq!(frame_asked(&mut app).1.at, crate::video::At::Seconds(0.0));
    }

    #[test]
    fn without_ffmpeg_the_viewer_says_what_to_install_and_shows_the_file_as_before() {
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        let before = frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: ask.number,
            result: Err(crate::video::Failure::NoProgram(a_film())),
        });
        assert!(
            app.status.contains("`ffmpeg`") && app.status.contains("not installed"),
            "{}",
            app.status
        );
        assert!(app.status.contains("H.264/AVC 64×48"), "{}", app.status);
        assert!(app.status.contains("F11"), "{}", app.status);
        // Nothing else changed: no frame, no picture mode, and the keys turn pages.
        app.status.clear();
        assert_eq!(frame(&mut app, 80, 24), before);
        press(&mut app, "PgDn");
        press(&mut app, "Space");
        assert!(app.take_requests().is_empty());

        // A container of sound alone is not a film, and nothing is said about it at all.
        let (_dir, mut app) = app_with_viewer("song.m4a", &mp4_head());
        frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.status.clear();
        app.handle_event(Event::VideoFrame {
            key,
            number: ask.number,
            result: Err(crate::video::Failure::NotAFilm),
        });
        assert!(app.status.is_empty(), "{}", app.status);

        // And a frame `ffmpeg` could not give is an error with its reason in it.
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key,
            number: ask.number,
            result: Err(crate::video::Failure::Failed("moov atom not found".into())),
        });
        assert!(app.status.contains("moov atom not found"), "{}", app.status);
        assert!(app.status_error);
    }

    #[test]
    fn the_quick_view_shows_a_film_by_its_frame_and_keeps_quiet_without_ffmpeg() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("film.mp4"), mp4_head()).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "Ctrl+Q");
        let mut buf = Buffer::new(100, 30);
        draw(&mut app, &mut buf);
        let (key, ask, _) = frame_asked(&mut app);
        assert_eq!(ask.at, crate::video::At::Fraction(0.1));
        // Drawing again does not ask again: the cursor has not moved.
        draw(&mut app, &mut buf);
        assert!(app.take_requests().is_empty());

        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: ask.number,
            result: Ok(a_frame(12.0, 90)),
        });
        draw(&mut app, &mut buf);
        assert!(buf.to_text().contains('▀'), "{}", buf.to_text());

        // On a machine without the program the quick view says nothing — it would say it
        // on every film the cursor passed.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("film.mp4"), mp4_head()).unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        press(&mut app, "Ctrl+Q");
        draw(&mut app, &mut buf);
        let (key, ask, _) = frame_asked(&mut app);
        app.status.clear();
        app.handle_event(Event::VideoFrame {
            key,
            number: ask.number,
            result: Err(crate::video::Failure::NoProgram(a_film())),
        });
        assert!(app.status.is_empty(), "{}", app.status);
    }

    #[test]
    fn a_frame_for_a_viewer_that_was_closed_stops_the_film_it_came_from() {
        let (_dir, mut app) = app_with_viewer("film.mp4", &mp4_head());
        frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        app.handle_event(Event::VideoFrame {
            key: key.clone(),
            number: ask.number,
            result: Ok(a_frame(12.0, 40)),
        });
        press(&mut app, "Space");
        let (_, playing, _) = frame_asked(&mut app);
        let newest = playing.newest.clone();
        let now = |cell: &std::sync::Arc<std::sync::atomic::AtomicU64>| {
            cell.load(std::sync::atomic::Ordering::Relaxed)
        };
        assert_eq!(now(&newest), playing.number, "the worker plays on");
        // `Esc` closes the viewer; the next frame finds nobody, and that outdates the ask —
        // which is what the worker looks at before every frame.
        press(&mut app, "Esc");
        app.handle_event(Event::VideoFrame {
            key,
            number: playing.number,
            result: Ok(a_frame(12.125, 41)),
        });
        assert_ne!(now(&newest), playing.number);
    }

    #[test]
    fn ffmpeg_draws_a_real_film_in_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        let Some((_, film)) = crate::video::tests::a_film_in(dir.path()) else {
            eprintln!("skipped: ffmpeg is not installed here");
            return;
        };
        let (_keep, mut app) = app_with_viewer("film.mkv", &std::fs::read(&film).unwrap());
        frame(&mut app, 80, 24);
        let (key, ask, _) = frame_asked(&mut app);
        // What the loop's worker does with the request.
        let result = crate::video::frame(&ask);
        assert!(result.is_ok(), "{result:?}");
        app.handle_event(Event::VideoFrame {
            key,
            number: ask.number,
            result,
        });
        let out = frame(&mut app, 80, 24);
        assert!(out.contains('▀'), "{out}");
        let top = out.lines().next().unwrap();
        assert!(top.contains("00:00 / 00:02"), "{top}");
        assert!(top.contains("64×48"), "{top}");
    }

    /// A test that found damage ends in a list: every entry that did not come out, with the
    /// reason it gave, under a title that says how many of how many — and `Enter` goes to
    /// the entry in the archive. A test that found none says so on the status line, and one
    /// that could not read the archive at all is an error, not a list.
    #[test]
    fn a_test_that_found_damage_lists_every_bad_entry_and_enter_goes_to_one() {
        use fcmd_jobs::{JobKind, Outcome};
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("backup.zip"), b"PK\x05\x06").unwrap();
        let mut app = app_on_disk(dir.path());
        press(&mut app, "Down");
        let archive = app.panel().cwd.join("backup.zip");
        let test = |app: &mut App, outcome: Outcome| {
            app.mode = Mode::Normal;
            app.take_requests();
            app.run(fcmd_cmd::Command::ArcTest);
            let id = app
                .take_requests()
                .into_iter()
                .find_map(|r| match r {
                    crate::app::Request::ArcTest { id, .. } => Some(id),
                    _ => None,
                })
                .expect("a test was asked for");
            app.handle_event(Event::JobDone {
                id,
                kind: JobKind::Test,
                outcome,
            });
        };

        test(
            &mut app,
            Outcome {
                files: 12,
                bytes: 3 * 1024 * 1024,
                ..Outcome::default()
            },
        );
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert!(!app.status_error, "{}", app.status);
        assert!(app.status.contains("backup.zip"), "{}", app.status);
        assert!(app.status.contains("12"), "{}", app.status);
        assert!(app.status.contains("whole"), "{}", app.status);

        let mut errors: Vec<(VPath, String)> = (0..20)
            .map(|n| {
                (
                    archive.enter(format!("/photos/img{n:02}.jpg")),
                    "CRC mismatch".to_string(),
                )
            })
            .collect();
        errors.push((
            archive.enter("/src/main.rs"),
            "invalid checksum".to_string(),
        ));
        test(
            &mut app,
            Outcome {
                files: 40,
                errors,
                ..Outcome::default()
            },
        );
        let text = frame(&mut app, 80, 24);
        assert!(
            text.contains("backup.zip: 21 of 40 entries did not come out whole"),
            "{text}"
        );
        assert!(text.contains("photos/img00.jpg — CRC mismatch"), "{text}");
        // All of them are in the list, not the first eight: the last is a keystroke away.
        press(&mut app, "End");
        let text = frame(&mut app, 80, 24);
        assert!(text.contains("src/main.rs — invalid checksum"), "{text}");
        app.take_requests();
        press(&mut app, "Enter");
        assert!(matches!(app.mode, Mode::Normal), "{:?}", app.mode);
        assert_eq!(app.panel().cwd, archive.enter("/src"));
        assert_eq!(app.panel().want_cursor.as_deref(), Some("main.rs"));

        // The archive itself would not open: one reason, in the box every job's errors use.
        app.panels[app.active].go(VPath::local(dir.path()), true);
        let entries = fcmd_vfs::local::list_blocking(
            &VPath::local(dir.path()),
            &fcmd_core::CancelToken::new(),
        )
        .unwrap();
        app.handle_event(Event::Listed {
            panel: app.active,
            generation: app.panel().generation,
            result: Ok(entries),
        });
        press(&mut app, "Down");
        test(
            &mut app,
            Outcome {
                errors: vec![(archive.clone(), "broken archive (no index)".to_string())],
                ..Outcome::default()
            },
        );
        assert!(app.status_error, "{}", app.status);
        assert!(matches!(app.mode, Mode::Message { .. }), "{:?}", app.mode);
    }
}
