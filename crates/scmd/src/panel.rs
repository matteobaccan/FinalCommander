use scmd_core::{Entry, EntryKind, Listing, Selection, Sort, VPath, ViewMode};

/// One panel's state. Pure data plus cursor arithmetic — no I/O, so it is easy to test.
#[derive(Debug, Clone)]
pub struct Panel {
    pub cwd: VPath,
    pub listing: Listing,
    pub cursor: usize,
    pub top: usize,
    pub selection: Selection,
    pub sort: Sort,
    pub view: ViewMode,
    pub show_hidden: bool,
    /// What was typed in the filter bar, for the status line.
    pub filter: Option<String>,
    /// The same thing compiled. Both are set together, or neither is.
    pub filter_set: Option<scmd_core::filter::MaskSet>,
    pub history: Vec<VPath>,
    pub history_pos: usize,
    pub generation: u64,
    pub loading: bool,
    pub error: Option<String>,
    pub message: Option<String>,
    /// A name to put the cursor on when the next listing arrives — `goto:` asks for this, and
    /// the read it waits for has not happened yet.
    pub want_cursor: Option<String>,
}

impl Panel {
    pub fn new(cwd: VPath, sort: Sort, view: ViewMode, show_hidden: bool) -> Self {
        Panel {
            history: vec![cwd.clone()],
            cwd,
            listing: Listing::default(),
            cursor: 0,
            top: 0,
            selection: Selection::default(),
            sort,
            view,
            show_hidden,
            filter: None,
            filter_set: None,
            want_cursor: None,
            history_pos: 0,
            generation: 0,
            loading: true,
            error: None,
            message: None,
        }
    }

    pub fn current(&self) -> Option<&Entry> {
        self.listing.row(self.cursor)
    }

    pub fn rebuild(&mut self) {
        self.listing.rebuild(
            self.sort,
            self.show_hidden,
            self.filter_set.as_ref(),
            &self.selection,
        );
        self.clamp();
    }

    /// Replace the listing after a read, keeping the cursor on the same name when possible.
    pub fn set_listing(&mut self, entries: Vec<Entry>, generation: u64) {
        let keep = self.current().map(|e| e.name_str().into_owned());
        self.listing = Listing::new(entries, generation);
        self.selection.retain_existing(&self.listing);
        self.rebuild();
        // A name somebody asked for wins over the one we were on, once.
        let wanted = self.want_cursor.take();
        self.cursor = wanted
            .and_then(|name| self.listing.row_of_name(&name))
            .or_else(|| keep.and_then(|name| self.listing.row_of_name(&name)))
            .unwrap_or(0);
        self.loading = false;
        self.error = None;
        self.clamp();
    }

    /// How many entries fit on screen, given the rows available and the view mode.
    pub fn capacity(&self, rows: usize, columns: usize) -> usize {
        match self.view {
            ViewMode::Brief | ViewMode::Wide => rows * columns.max(1),
            _ => rows,
        }
        .max(1)
    }

    pub fn move_cursor(&mut self, delta: isize) {
        let len = self.listing.len();
        if len == 0 {
            self.cursor = 0;
            return;
        }
        let next = self.cursor as isize + delta;
        self.cursor = next.clamp(0, len as isize - 1) as usize;
    }

    pub fn cursor_to(&mut self, row: usize) {
        let len = self.listing.len();
        self.cursor = if len == 0 { 0 } else { row.min(len - 1) };
    }

    /// Keep the cursor inside the listing and the view scrolled onto the cursor.
    pub fn clamp(&mut self) {
        let len = self.listing.len();
        if len == 0 {
            self.cursor = 0;
            self.top = 0;
            return;
        }
        self.cursor = self.cursor.min(len - 1);
    }

    pub fn scroll_into_view(&mut self, rows: usize, columns: usize) {
        let cap = self.capacity(rows, columns);
        if self.cursor < self.top {
            self.top = self.cursor;
        } else if self.cursor >= self.top + cap {
            self.top = self.cursor + 1 - cap;
        }
        let len = self.listing.len();
        if len <= cap {
            self.top = 0;
        } else if self.top + cap > len {
            self.top = len - cap;
        }
        // In brief view the top must stay on a column boundary, or rows shuffle while scrolling.
        if matches!(self.view, ViewMode::Brief | ViewMode::Wide) && rows > 0 {
            self.top -= self.top % rows;
        }
    }

    /// Change directory, recording history. `push_history` is false when walking history.
    pub fn go(&mut self, path: VPath, push_history: bool) {
        if push_history && path != self.cwd {
            self.history.truncate(self.history_pos + 1);
            self.history.push(path.clone());
            self.history_pos = self.history.len() - 1;
        }
        self.cwd = path;
        self.cursor = 0;
        self.top = 0;
        self.filter = None;
        self.filter_set = None;
        self.selection.clear();
        self.loading = true;
        self.listing = Listing::default();
    }

    pub fn history_back(&mut self) -> Option<VPath> {
        if self.history_pos == 0 {
            return None;
        }
        self.history_pos -= 1;
        Some(self.history[self.history_pos].clone())
    }

    pub fn history_forward(&mut self) -> Option<VPath> {
        if self.history_pos + 1 >= self.history.len() {
            return None;
        }
        self.history_pos += 1;
        Some(self.history[self.history_pos].clone())
    }

    /// Target of `Enter` / `Ctrl+PgDn`: `Some(path)` for a directory, `None` for a file.
    pub fn enter_target(&self) -> Option<VPath> {
        let e = self.current()?;
        match e.kind {
            EntryKind::Parent => self.cwd.parent().map(|p| p.normalised()),
            EntryKind::Dir => Some(self.cwd.join(&e.name)),
            EntryKind::Symlink => Some(self.cwd.join(&e.name)),
            _ => None,
        }
    }

    /// First row whose name starts with (then contains) `needle`, from the cursor onwards.
    pub fn find_row(&self, needle: &str, from: usize) -> Option<usize> {
        if needle.is_empty() {
            return None;
        }
        let needle = needle.to_lowercase();
        let len = self.listing.len();
        for pass in 0..2 {
            for i in 0..len {
                let row = (from + i) % len;
                let name = self.listing.row(row)?.name_str().to_lowercase();
                let hit = if pass == 0 {
                    name.starts_with(&needle)
                } else {
                    name.contains(&needle)
                };
                if hit {
                    return Some(row);
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use scmd_core::{EntryFlags, EntryId};
    use std::ffi::OsString;

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry {
            name: OsString::from(name),
            kind,
            size: 1,
            mtime: None,
            flags: EntryFlags::empty(),
            id: EntryId::Name(name.bytes().map(u64::from).sum()),
            link_target: None,
        }
    }

    fn panel() -> Panel {
        let mut p = Panel::new(
            VPath::local("/home/mb"),
            Sort::default(),
            ViewMode::Full,
            false,
        );
        p.set_listing(
            vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir),
                entry("alpha.txt", EntryKind::File),
                entry("beta.txt", EntryKind::File),
                entry("gamma.txt", EntryKind::File),
            ],
            1,
        );
        p
    }

    #[test]
    fn cursor_stays_inside_the_listing() {
        let mut p = panel();
        p.move_cursor(-5);
        assert_eq!(p.cursor, 0);
        p.move_cursor(100);
        assert_eq!(p.cursor, 4);
    }

    #[test]
    fn scrolling_follows_the_cursor() {
        let mut p = panel();
        p.cursor_to(4);
        p.scroll_into_view(2, 1);
        assert_eq!(p.top, 3);
        p.cursor_to(0);
        p.scroll_into_view(2, 1);
        assert_eq!(p.top, 0);
    }

    #[test]
    fn brief_view_scrolls_by_whole_columns() {
        let mut p = panel();
        p.view = ViewMode::Brief;
        p.cursor_to(4);
        p.scroll_into_view(2, 2); // 2 rows, 2 columns = 4 visible
        assert_eq!(p.top % 2, 0, "top must stay on a column boundary");
    }

    #[test]
    fn enter_target_handles_parent_dirs_and_files() {
        let mut p = panel();
        p.cursor_to(0);
        assert_eq!(p.enter_target(), Some(VPath::local("/home")));
        p.cursor_to(1);
        assert_eq!(p.enter_target(), Some(VPath::local("/home/mb/docs")));
        p.cursor_to(2);
        assert_eq!(p.enter_target(), None);
    }

    #[test]
    fn history_walks_back_and_forward() {
        let mut p = panel();
        p.go(VPath::local("/home/mb/docs"), true);
        p.go(VPath::local("/home/mb/docs/sub"), true);
        assert_eq!(p.history_back(), Some(VPath::local("/home/mb/docs")));
        assert_eq!(p.history_back(), Some(VPath::local("/home/mb")));
        assert_eq!(p.history_back(), None);
        assert_eq!(p.history_forward(), Some(VPath::local("/home/mb/docs")));
    }

    #[test]
    fn quick_search_prefers_prefixes_then_substrings() {
        let p = panel();
        assert_eq!(p.find_row("be", 0), p.listing.row_of_name("beta.txt"));
        assert_eq!(p.find_row("amma", 0), p.listing.row_of_name("gamma.txt"));
        assert_eq!(p.find_row("zzz", 0), None);
    }

    #[test]
    fn a_reread_keeps_the_cursor_on_the_same_name() {
        let mut p = panel();
        p.cursor_to(3); // beta.txt
        let before = p.current().unwrap().name_str().into_owned();
        p.set_listing(
            vec![
                Entry::parent_entry(),
                entry("docs", EntryKind::Dir),
                entry("alpha.txt", EntryKind::File),
                entry("beta.txt", EntryKind::File),
                entry("new.txt", EntryKind::File),
            ],
            2,
        );
        assert_eq!(p.current().unwrap().name_str(), before);
    }

    #[test]
    fn changing_directory_clears_selection_and_filter() {
        let mut p = panel();
        p.selection
            .select_all(&p.listing.clone(), scmd_core::MaskScope::FilesAndDirs);
        p.filter = Some("txt".into());
        p.go(VPath::local("/tmp"), true);
        assert!(p.selection.is_empty());
        assert!(p.filter.is_none());
        assert!(p.loading);
    }
}
