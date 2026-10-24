//! The tree panel's own state (SPEC 02 §7).
//!
//! A tree is a list of visible rows, not a recursive structure: the panel draws a window of
//! rows and the keys move a cursor through them, exactly as a file panel does. Expanding a
//! directory splices its children in; collapsing one cuts them out again.
//!
//! What a directory holds is remembered in a [`Cache`] that outlives the panel, so collapsing
//! and expanding the same directory twice costs one read, and so does leaving the tree panel
//! and coming back.

use std::collections::HashMap;

use scmd_core::VPath;

/// One visible row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub path: VPath,
    /// How far in: the root is 0.
    pub depth: usize,
    pub expanded: bool,
    /// What we know about its children: `None` until the directory has been read.
    pub children: Option<usize>,
    /// The last row at this depth under its parent, for the `└─` rather than `├─`.
    pub last: bool,
}

impl Row {
    pub fn name(&self) -> String {
        self.path.display_name()
    }
}

/// The subdirectories of each directory we have read, by path.
#[derive(Debug, Default)]
pub struct Cache {
    dirs: HashMap<VPath, Vec<VPath>>,
}

impl Cache {
    pub fn get(&self, path: &VPath) -> Option<&[VPath]> {
        self.dirs.get(path).map(|v| &v[..])
    }

    pub fn insert(&mut self, path: VPath, children: Vec<VPath>) {
        self.dirs.insert(path, children);
    }

    /// Forget one directory, for a re-read (`Ctrl+R`).
    pub fn forget(&mut self, path: &VPath) {
        self.dirs.remove(path);
    }
}

/// How many rows one `*` may add, so expanding everything under `/` cannot hang the panel.
pub const EXPAND_LIMIT: usize = 2_000;

#[derive(Debug, Clone)]
pub struct Tree {
    pub root: VPath,
    pub rows: Vec<Row>,
    pub cursor: usize,
    pub top: usize,
    /// A path we are opening our way towards, a level per read: the directory the other panel
    /// is in, so the tree comes up with the cursor where the user already is.
    pub want: Option<VPath>,
}

impl Tree {
    /// A tree showing `root` and nothing else yet.
    pub fn new(root: VPath) -> Tree {
        Tree {
            rows: vec![Row {
                path: root.clone(),
                depth: 0,
                expanded: false,
                children: None,
                last: true,
            }],
            root,
            cursor: 0,
            top: 0,
            want: None,
        }
    }

    pub fn current(&self) -> Option<&Row> {
        self.rows.get(self.cursor)
    }

    pub fn path(&self) -> Option<VPath> {
        self.current().map(|r| r.path.clone())
    }

    pub fn move_cursor(&mut self, delta: isize, page: usize) {
        if self.rows.is_empty() {
            self.cursor = 0;
            return;
        }
        let last = self.rows.len() - 1;
        self.cursor = (self.cursor as isize + delta).clamp(0, last as isize) as usize;
        if self.cursor < self.top {
            self.top = self.cursor;
        }
        let page = page.max(1);
        if self.cursor >= self.top + page {
            self.top = self.cursor + 1 - page;
        }
    }

    /// Put the cursor on `path` if it is visible, and say whether it was.
    pub fn select(&mut self, path: &VPath, page: usize) -> bool {
        match self.rows.iter().position(|r| &r.path == path) {
            Some(i) => {
                let delta = i as isize - self.cursor as isize;
                self.move_cursor(delta, page);
                true
            }
            None => false,
        }
    }

    /// Is the row under the cursor expanded?
    pub fn is_expanded(&self) -> bool {
        self.current().map(|r| r.expanded).unwrap_or(false)
    }

    /// Splice what the cache knows about the row under the cursor into the rows below it.
    ///
    /// Returns false when the directory has not been read yet, which is the caller's cue to
    /// ask for it: the tree never reads anything itself.
    pub fn expand(&mut self, cache: &Cache) -> bool {
        let Some(row) = self.rows.get(self.cursor) else {
            return true;
        };
        if row.expanded {
            return true;
        }
        let Some(children) = cache.get(&row.path) else {
            return false;
        };
        let depth = row.depth + 1;
        let at = self.cursor + 1;
        let new: Vec<Row> = children
            .iter()
            .enumerate()
            .map(|(i, path)| Row {
                path: path.clone(),
                depth,
                expanded: false,
                children: None,
                last: i + 1 == children.len(),
            })
            .collect();
        self.rows[self.cursor].expanded = true;
        self.rows[self.cursor].children = Some(new.len());
        self.rows.splice(at..at, new);
        true
    }

    /// Cut out everything under the row the cursor is on.
    pub fn collapse(&mut self) {
        let Some(row) = self.rows.get(self.cursor) else {
            return;
        };
        if !row.expanded {
            return;
        }
        let depth = row.depth;
        let from = self.cursor + 1;
        let to = self.rows[from..]
            .iter()
            .position(|r| r.depth <= depth)
            .map(|n| from + n)
            .unwrap_or(self.rows.len());
        self.rows.drain(from..to);
        self.rows[self.cursor].expanded = false;
    }

    /// `*`: expand everything under the cursor that the cache already knows, up to
    /// [`EXPAND_LIMIT`] rows. Returns the paths it still needs read, in the order it met them.
    pub fn expand_all(&mut self, cache: &Cache) -> Vec<VPath> {
        let mut wanted = Vec::new();
        let start = self.cursor;
        let depth = self.rows.get(start).map(|r| r.depth).unwrap_or(0);
        let mut at = start;
        let mut added = 0;
        while at < self.rows.len() {
            // Past the subtree we were asked about.
            if at > start && self.rows[at].depth <= depth {
                break;
            }
            let path = self.rows[at].path.clone();
            if !self.rows[at].expanded {
                let before = self.rows.len();
                let saved = self.cursor;
                self.cursor = at;
                let known = self.expand(cache);
                self.cursor = saved;
                if !known {
                    wanted.push(path);
                } else {
                    added += self.rows.len() - before;
                    if added >= EXPAND_LIMIT {
                        break;
                    }
                }
            }
            at += 1;
        }
        wanted
    }

    /// The deepest visible row `path` is inside, which is the next one to expand on the way
    /// to it.
    pub fn towards(&self, path: &VPath) -> Option<usize> {
        self.rows.iter().rposition(|r| path.starts_with(&r.path))
    }

    /// The guides and the name for one row: `│  ├─ name`.
    pub fn line(&self, index: usize) -> String {
        let Some(row) = self.rows.get(index) else {
            return String::new();
        };
        if row.depth == 0 {
            return row.path.to_string();
        }
        let mut out = String::new();
        // A vertical guide for every ancestor that has rows after it.
        for level in 1..row.depth {
            out.push_str(if self.has_more_below(index, level) {
                "│  "
            } else {
                "   "
            });
        }
        out.push_str(if row.last { "└─ " } else { "├─ " });
        out.push_str(&row.name());
        if row.expanded {
            out.push('/');
        } else if row.children.is_none() {
            // Not read yet: there may or may not be something in it, and saying `+` would be
            // a claim we cannot back.
            out.push_str(" …");
        }
        out
    }

    /// Is there another row at `level` after `index`? That is what decides whether the guide
    /// at that depth continues past this row.
    fn has_more_below(&self, index: usize, level: usize) -> bool {
        for row in &self.rows[index + 1..] {
            if row.depth < level {
                return false;
            }
            if row.depth == level {
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cache() -> Cache {
        let mut cache = Cache::default();
        cache.insert(
            VPath::local("/a"),
            vec![VPath::local("/a/one"), VPath::local("/a/two")],
        );
        cache.insert(VPath::local("/a/one"), vec![VPath::local("/a/one/deep")]);
        cache.insert(VPath::local("/a/two"), vec![]);
        cache.insert(VPath::local("/a/one/deep"), vec![]);
        cache
    }

    #[test]
    fn expanding_splices_children_in_and_collapsing_cuts_them_out() {
        let cache = cache();
        let mut tree = Tree::new(VPath::local("/a"));
        assert_eq!(tree.rows.len(), 1);

        assert!(tree.expand(&cache));
        assert_eq!(tree.rows.len(), 3);
        assert_eq!(tree.rows[1].path, VPath::local("/a/one"));
        assert_eq!(tree.rows[1].depth, 1);
        assert!(!tree.rows[1].last && tree.rows[2].last);

        // Expanding a child puts its own children under it, not at the end.
        tree.move_cursor(1, 10);
        assert!(tree.expand(&cache));
        assert_eq!(
            tree.rows
                .iter()
                .map(|r| r.path.to_string())
                .collect::<Vec<_>>(),
            ["/a", "/a/one", "/a/one/deep", "/a/two"]
        );

        // Collapsing the root takes the whole subtree with it.
        tree.cursor = 0;
        tree.collapse();
        assert_eq!(tree.rows.len(), 1);
        assert!(!tree.rows[0].expanded);
    }

    #[test]
    fn a_directory_we_have_not_read_says_so_rather_than_expanding_to_nothing() {
        let cache = Cache::default();
        let mut tree = Tree::new(VPath::local("/a"));
        assert!(!tree.expand(&cache), "it cannot know what is in there yet");
        assert_eq!(tree.rows.len(), 1);
        assert!(tree.line(0).contains("/a"));
    }

    #[test]
    fn star_expands_what_is_known_and_asks_for_the_rest() {
        let mut cache = Cache::default();
        cache.insert(
            VPath::local("/a"),
            vec![VPath::local("/a/one"), VPath::local("/a/two")],
        );
        let mut tree = Tree::new(VPath::local("/a"));
        let wanted = tree.expand_all(&cache);
        // The root expanded from the cache; its two children are not read yet.
        assert_eq!(tree.rows.len(), 3);
        assert_eq!(wanted, vec![VPath::local("/a/one"), VPath::local("/a/two")]);
    }

    #[test]
    fn the_guides_say_where_a_branch_continues() {
        let cache = cache();
        let mut tree = Tree::new(VPath::local("/a"));
        tree.expand(&cache);
        tree.cursor = 1;
        tree.expand(&cache);
        // /a, ├─ one/, │  └─ deep, └─ two
        let lines: Vec<String> = (0..tree.rows.len()).map(|i| tree.line(i)).collect();
        assert_eq!(lines[1], "├─ one/");
        assert_eq!(lines[2], "│  └─ deep …");
        assert_eq!(lines[3], "└─ two …");
    }

    #[test]
    fn the_cursor_stays_on_a_row_and_the_window_follows_it() {
        let mut cache = Cache::default();
        let kids: Vec<VPath> = (0..20).map(|i| VPath::local(format!("/a/{i}"))).collect();
        cache.insert(VPath::local("/a"), kids);
        let mut tree = Tree::new(VPath::local("/a"));
        tree.expand(&cache);

        tree.move_cursor(100, 5);
        assert_eq!(tree.cursor, 20, "the cursor stops at the last row");
        assert_eq!(tree.top, 16, "the window shows the end");
        tree.move_cursor(-100, 5);
        assert_eq!((tree.cursor, tree.top), (0, 0));
        assert!(tree.select(&VPath::local("/a/7"), 5));
        assert_eq!(tree.path(), Some(VPath::local("/a/7")));
        assert!(!tree.select(&VPath::local("/nowhere"), 5));
    }
}
