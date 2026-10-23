use sc_ui::{widgets, Buffer, Rect, Style};

use crate::app::{App, Mode};

/// Draw a whole frame. Returns where the text cursor should sit, if anywhere.
pub fn draw(app: &mut App, buf: &mut Buffer) -> Option<(u16, u16)> {
    let (w, h) = (buf.width(), buf.height());
    buf.clear(app.theme.panel());
    if w < 20 || h < 5 {
        buf.put_str(0, 0, "terminal too small", app.theme.panel(), w);
        return None;
    }

    let keybar_h = u16::from(app.cfg.ui.key_bar);
    let status_h = u16::from(app.cfg.ui.status_bar);
    let panels_h = h.saturating_sub(keybar_h + status_h).max(3);

    let left_w = w / 2;
    let areas = [
        Rect::new(0, 0, left_w, panels_h),
        Rect::new(left_w, 0, w - left_w, panels_h),
    ];

    // Remember the geometry so key handling knows how far a page is.
    app.rows = widgets::panel_rows(areas[0]);
    app.columns = widgets::brief_columns(areas[0]);
    for i in 0..2 {
        let (rows, cols) = (app.rows, app.columns);
        app.panels[i].scroll_into_view(rows, cols);
    }

    for (i, area) in areas.iter().enumerate() {
        let panel = &app.panels[i];
        let path = panel.cwd.to_string();
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
            filter: panel.filter.as_deref(),
            message: message.as_deref(),
        };
        widgets::draw_panel(buf, &app.theme, &render);
    }

    if status_h == 1 {
        let text = if app.status.is_empty() {
            status_hint(app)
        } else {
            app.status.clone()
        };
        widgets::draw_status(
            buf,
            &app.theme,
            Rect::new(0, panels_h, w, 1),
            &text,
            app.status_error,
        );
    }

    if keybar_h == 1 {
        let labels = app.keybar();
        let refs: Vec<(u8, &str)> = labels.iter().map(|(n, s)| (*n, s.as_str())).collect();
        widgets::draw_keybar(buf, &app.theme, Rect::new(0, h - 1, w, 1), &refs);
    }

    match app.mode.clone() {
        Mode::Normal => None,
        Mode::Prompt(prompt) => {
            let screen = Rect::new(0, 0, w, panels_h);
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
        Mode::Message { title, lines } => {
            let screen = Rect::new(0, 0, w, panels_h);
            widgets::draw_dialog(buf, &app.theme, screen, &title, &lines, None);
            None
        }
        Mode::Help { scroll } => {
            draw_help(app, buf, scroll, Rect::new(0, 0, w, panels_h));
            None
        }
    }
}

fn status_hint(app: &App) -> String {
    let panel = app.panel();
    match panel.current() {
        Some(e) => {
            let size = if e.kind.is_dirlike() {
                String::new()
            } else {
                format!("  {}", sc_core::format_size(e.size))
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

fn draw_help(app: &App, buf: &mut Buffer, scroll: usize, screen: Rect) {
    let lines = app.help_lines();
    let box_w = screen.w.saturating_sub(6).clamp(24, 60);
    let box_h = screen.h.saturating_sub(2).max(6);
    let x = screen.x + (screen.w - box_w) / 2;
    let y = screen.y + (screen.h - box_h) / 2;
    let area = Rect::new(x, y, box_w, box_h);

    let style = app.theme.dialog();
    buf.fill(area, ' ', style);
    let title = app.i18n.t_args(
        "dialog-help-title",
        &[("profile", app.keymap.name().into())],
    );
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
    if lines.len().saturating_sub(scroll) < visible {
        let row = lines.len() - scroll;
        buf.put_str(
            inner.x,
            inner.y + row as u16 + 1,
            &format!("config: {}", app.config_dir().display()),
            style,
            inner.w,
        );
    }
    let hint = app.i18n.t("dialog-help-hint");
    buf.put_str(
        inner.x,
        area.bottom() - 1,
        &sc_ui::truncate_cols(&hint, inner.w as usize),
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
    use sc_cmd::Keymap;
    use sc_config::{Config, Paths};
    use sc_core::{Entry, EntryFlags, EntryId, EntryKind, VPath};
    use sc_i18n::I18n;
    use sc_tui::Input;
    use sc_ui::{Theme, ThemeSet};
    use std::ffi::OsString;
    use std::path::Path;

    use crate::app::{Event, Startup};

    fn entry(name: &str, kind: EntryKind, size: u64) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size,
            mtime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn app(theme: &str, lang: &str) -> App {
        let mut app = App::new(Startup {
            cfg: Config::default(),
            paths: Paths::discover(Some(Path::new("/tmp/sc-test"))),
            i18n: I18n::new(lang),
            keymap: Keymap::builtin("far").unwrap(),
            themes: ThemeSet::builtin_only(),
            theme: Theme::builtin(theme).unwrap(),
            left: VPath::local("/home/mb/project"),
            right: VPath::local("/tmp"),
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
    fn the_help_overlay_lists_bindings() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("F1".parse().unwrap()));
        let out = frame(&mut app, 80, 24);
        assert!(out.contains("Keyboard reference"), "{out}");
        assert!(
            out.contains("[global]") && out.contains("app.quit"),
            "{out}"
        );
        assert!(out.contains("Esc or F1 closes"), "{out}");
        // Bindings further down are reachable by scrolling.
        app.handle_input(Input::Key("PgDn".parse().unwrap()));
        let scrolled = frame(&mut app, 80, 24);
        assert_ne!(scrolled, out, "PgDn did not scroll the help page");
    }

    #[test]
    fn a_prompt_is_drawn_over_the_panels_with_a_cursor() {
        let mut app = app("commander", "en");
        app.handle_input(Input::Key("Ctrl+L".parse().unwrap()));
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
    fn every_language_renders_the_key_bar() {
        for (tag, _) in sc_i18n::LANGUAGES {
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
    fn brief_view_geometry_is_fed_back_to_the_app() {
        let mut app = app("commander", "en");
        app.run(sc_cmd::Command::PanelViewSet(sc_core::ViewMode::Brief));
        let _ = frame(&mut app, 100, 30);
        assert!(
            app.columns >= 2,
            "brief view should use columns: {}",
            app.columns
        );
        assert!(app.rows > 20, "rows = {}", app.rows);
    }
}
