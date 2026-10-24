use std::collections::HashMap;

use scmd_core::{Entry, EntryId, EntryKind, Listing, Selection, Sort, VPath, ViewMode};

/// A panel made of a list somebody handed us rather than of a directory: find results,
/// `panelize`, and later a plugin's own list. The rows are files wherever they live, so the
/// panel cannot work out a path by joining its own directory onto a name — it has to be told,
/// and the map is keyed by entry id because the listing re-sorts under it.
#[derive(Debug, Clone)]
pub struct TempList {
    /// What the panel says it is showing, in its frame.
    pub title: String,
    pub paths: HashMap<EntryId, VPath>,
}

impl TempList {
    /// One row per path. The id is the hash of the whole path rather than of the name, because
    /// two results of a search are very often the same name in different directories.
    pub fn new(title: String, items: Vec<(VPath, Entry)>) -> (TempList, Vec<Entry>) {
        let mut paths = HashMap::with_capacity(items.len());
        let mut entries = Vec::with_capacity(items.len());
        for (path, mut entry) in items {
            entry.id = EntryId::Name(path_hash(&path));
            paths.insert(entry.id, path);
            entries.push(entry);
        }
        (TempList { title, paths }, entries)
    }
}

fn path_hash(path: &VPath) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    path.to_string().hash(&mut hasher);
    hasher.finish()
}

/// What a panel is showing (SPEC 02 §7).
///
/// FAR's rule, and ours: all three special kinds live in the *passive* panel, the key that
/// turns one on turns it off again, and what the panel shows follows the other panel's
/// cursor. The special kinds have no rows of their own, so nothing that acts on a selection
/// has anything to act on — which is why the focus goes to the file panel when one opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Content {
    /// Rows: a directory, or a list of files from anywhere.
    #[default]
    Files,
    /// The viewer, on whatever the other panel's cursor is on (`Ctrl+Q`).
    QuickView,
    /// Facts about the device, the directory and the selection (`Ctrl+L`).
    Info,
    /// The directory tree (`Ctrl+T`). The one special kind that takes the focus, because
    /// moving its cursor is how it changes the other panel's directory.
    Tree,
}

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
    /// Free bytes on the device this directory is on, as the last read reported them. `None`
    /// where the platform or the provider cannot say.
    pub free: Option<u64>,
    /// Columns the name column is scrolled by, for names wider than it.
    pub name_scroll: usize,
    /// Set when this panel is a list rather than a directory.
    pub temp: Option<TempList>,
    /// What this panel shows: rows, or one of the special kinds.
    pub content: Content,
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
            free: None,
            name_scroll: 0,
            temp: None,
            content: Content::default(),
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
        self.temp = None;
        self.cursor = 0;
        self.top = 0;
        self.filter = None;
        self.filter_set = None;
        self.name_scroll = 0;
        self.free = None;
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

    /// Where a row's file actually is. For a directory panel that is the panel's own path with
    /// the name on the end; for a list panel it is whatever the list said, which is the whole
    /// reason this method exists rather than `cwd.join(name)` spread across the program.
    pub fn path_of(&self, e: &Entry) -> VPath {
        match &self.temp {
            Some(list) => list
                .paths
                .get(&e.id)
                .cloned()
                .unwrap_or_else(|| self.cwd.join(&e.name)),
            None => self.cwd.join(&e.name),
        }
    }

    /// The path under the cursor, if the cursor is on something addressable.
    pub fn current_path(&self) -> Option<VPath> {
        let e = self.current()?;
        (e.kind != EntryKind::Parent).then(|| self.path_of(e))
    }

    /// Become a list of files rather than a directory. The paths are kept beside the rows, so
    /// sorting, selecting, copying and deleting all work on the real files.
    pub fn set_temp(&mut self, title: String, items: Vec<(VPath, Entry)>, generation: u64) {
        let (list, entries) = TempList::new(title, items);
        self.temp = Some(list);
        self.selection.clear();
        self.filter = None;
        self.filter_set = None;
        self.name_scroll = 0;
        self.set_listing(entries, generation);
    }

    /// Every path the list holds, in listing order: what a re-read starts from.
    pub fn temp_paths(&self) -> Vec<VPath> {
        let Some(list) = &self.temp else {
            return Vec::new();
        };
        (0..self.listing.len())
            .filter_map(|row| self.listing.row(row))
            .filter_map(|e| list.paths.get(&e.id).cloned())
            .collect()
    }

    /// Target of `Enter` / `Ctrl+PgDn`: `Some(path)` for a directory, `None` for a file.
    pub fn enter_target(&self) -> Option<VPath> {
        let e = self.current()?;
        match e.kind {
            EntryKind::Parent => self.cwd.parent().map(|p| p.normalised()),
            // On a list panel, `Enter` on a file goes to the directory it lives in — the point
            // of a result is to take you there (FAR's temporary panel does the same).
            EntryKind::Dir | EntryKind::Symlink => Some(self.path_of(e)),
            _ if self.temp.is_some() => self.path_of(e).parent().map(|p| p.normalised()),
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
            btime: None,
            atime: None,
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
