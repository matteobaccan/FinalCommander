//! A row of tabs on one side (SPEC 02 §1.6).
//!
//! A tab *is* a panel. There is no tab object holding a panel beside a name and a lock,
//! because everything a tab remembers — where it is, how it is sorted, what is selected,
//! where it has been, what it is called, whether it is locked — is already what a panel
//! remembers. So a row is a list of panels with one of them on screen.
//!
//! The one on screen is not in the list. `App::panels[side]` is that tab, and its slot here
//! is empty: a parked copy of it would be a second answer to where the panel is, and the two
//! would drift apart within a keystroke. Switching tabs is therefore a move out of the slot
//! and the live one's move into it — no clone, whatever the directory holds.

use crate::panel::Panel;
use crate::tree::Tree;

/// A tab that is not on screen: the panel, and the tree it had open if it had one.
#[derive(Debug)]
pub struct Parked {
    pub panel: Panel,
    pub tree: Option<Box<Tree>>,
}

/// One side's tabs, in the order they are drawn.
#[derive(Debug)]
pub struct Row {
    /// One slot per tab. `slots[current]` is `None` — that tab is the live panel.
    slots: Vec<Option<Box<Parked>>>,
    current: usize,
}

impl Default for Row {
    fn default() -> Self {
        Row::one()
    }
}

impl Row {
    /// A side with one tab, which is whatever panel that side already has.
    pub fn one() -> Row {
        Row {
            slots: vec![None],
            current: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.slots.len()
    }

    pub fn current(&self) -> usize {
        self.current
    }

    /// Is there more than one tab? The tab bar is drawn only then, because a bar that is
    /// always there costs a row of files to say that there is one tab.
    pub fn many(&self) -> bool {
        self.slots.len() > 1
    }

    /// The panel of tab `i`, which for the one on screen is the live one.
    pub fn panel<'a>(&'a self, i: usize, live: &'a Panel) -> Option<&'a Panel> {
        if i == self.current {
            return (i < self.slots.len()).then_some(live);
        }
        self.slots.get(i)?.as_ref().map(|p| &p.panel)
    }

    /// What each tab is called, in order: its name if it was given one, else where it is.
    pub fn labels(&self, live: &Panel) -> Vec<String> {
        (0..self.len())
            .filter_map(|i| self.panel(i, live).map(label_of))
            .collect()
    }

    /// Where each tab is, in order — what gets written to the state file.
    pub fn tabs_state(&self, live: &Panel) -> Vec<fcmd_config::TabState> {
        (0..self.len())
            .filter_map(|i| self.panel(i, live))
            .map(|p| fcmd_config::TabState {
                path: p.cwd.to_string(),
                name: p.name.clone().unwrap_or_default(),
                locked: p.locked,
            })
            .collect()
    }

    /// Put the live panel in `to`'s place and take `to`'s out. `false` if there is nowhere
    /// to go, which is every case where nothing should happen.
    pub fn switch(&mut self, to: usize, live: &mut Panel, tree: &mut Option<Box<Tree>>) -> bool {
        if to == self.current || to >= self.slots.len() {
            return false;
        }
        let Some(incoming) = self.slots[to].take() else {
            // Only the current tab's slot is empty, and `to` is not it. Reaching here means
            // the invariant is broken; refusing is better than unparking nothing.
            debug_assert!(false, "tab {to} is neither on screen nor parked");
            return false;
        };
        let was = std::mem::replace(live, incoming.panel);
        let was_tree = std::mem::replace(tree, incoming.tree);
        self.slots[self.current] = Some(Box::new(Parked {
            panel: was,
            tree: was_tree,
        }));
        self.current = to;
        true
    }

    /// A new tab after the current one, which becomes the one on screen.
    pub fn open(&mut self, fresh: Panel, live: &mut Panel, tree: &mut Option<Box<Tree>>) {
        let at = self.current + 1;
        self.slots.insert(at, None);
        // The insert pushed the current tab's empty slot along if it was at or after `at`;
        // it was before it, so `current` still names it and it is the one to park into.
        let was = std::mem::replace(live, fresh);
        let was_tree = tree.take();
        self.slots[self.current] = Some(Box::new(Parked {
            panel: was,
            tree: was_tree,
        }));
        self.current = at;
    }

    /// Close the current tab and show its neighbour. `false` for the last tab of a side: a
    /// panel always has a tab, and a row with nothing in it has no panel to draw.
    pub fn close(&mut self, live: &mut Panel, tree: &mut Option<Box<Tree>>) -> bool {
        if self.slots.len() < 2 {
            return false;
        }
        self.slots.remove(self.current);
        // The one to the right takes its place; at the end of the row, the one to the left.
        let next = self.current.min(self.slots.len() - 1);
        let incoming = self.slots[next]
            .take()
            .expect("a tab that is not on screen is parked");
        *live = incoming.panel;
        *tree = incoming.tree;
        self.current = next;
        true
    }

    /// Take the current tab off this side, to be given to the other one. `None` for the last
    /// tab of a side, for the reason `close` refuses it.
    pub fn take(&mut self, live: &mut Panel, tree: &mut Option<Box<Tree>>) -> Option<Parked> {
        if self.slots.len() < 2 {
            return None;
        }
        self.slots.remove(self.current);
        let next = self.current.min(self.slots.len() - 1);
        let incoming = self.slots[next]
            .take()
            .expect("a tab that is not on screen is parked");
        let gone = std::mem::replace(live, incoming.panel);
        let gone_tree = std::mem::replace(tree, incoming.tree);
        self.current = next;
        Some(Parked {
            panel: gone,
            tree: gone_tree,
        })
    }

    /// Receive a tab from the other side, after the current one, and show it.
    pub fn give(&mut self, tab: Parked, live: &mut Panel, tree: &mut Option<Box<Tree>>) {
        self.open(tab.panel, live, tree);
        *tree = tab.tree;
    }

    /// The tab after this one, wrapping. The row is a ring because the alternative is a key
    /// that stops working at the end of it.
    pub fn next(&self) -> usize {
        (self.current + 1) % self.slots.len()
    }

    pub fn prev(&self) -> usize {
        (self.current + self.slots.len() - 1) % self.slots.len()
    }

    /// Append a tab at the end without showing it: what restoring a saved row does.
    pub fn append(&mut self, panel: Panel) {
        self.slots
            .push(Some(Box::new(Parked { panel, tree: None })));
    }
}

/// What a tab is called: the name it was given, or the last part of where it is. The root of
/// a filesystem has no last part, so it keeps its own spelling.
pub fn label_of(panel: &Panel) -> String {
    if let Some(name) = &panel.name {
        if !name.trim().is_empty() {
            return name.clone();
        }
    }
    if let Some(list) = &panel.temp {
        return list.title.clone();
    }
    let name = panel.cwd.display_name();
    if name.is_empty() {
        panel.cwd.to_string()
    } else {
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::{Sort, VPath, ViewMode};

    fn panel(at: &str) -> Panel {
        Panel::new(VPath::local(at), Sort::default(), ViewMode::Full, false)
    }

    /// The row and the live panel, as `App` holds them.
    struct Side {
        row: Row,
        live: Panel,
        tree: Option<Box<Tree>>,
    }

    impl Side {
        fn new(at: &str) -> Side {
            Side {
                row: Row::one(),
                live: panel(at),
                tree: None,
            }
        }
        fn open(&mut self, at: &str) {
            let fresh = panel(at);
            self.row.open(fresh, &mut self.live, &mut self.tree);
        }
        fn where_am_i(&self) -> String {
            self.live.cwd.to_string()
        }
        fn row_of_names(&self) -> Vec<String> {
            self.row.labels(&self.live)
        }
    }

    #[test]
    fn a_side_starts_with_one_tab_and_it_is_the_panel_itself() {
        let side = Side::new("/home/mb");
        assert_eq!(side.row.len(), 1);
        assert!(!side.row.many(), "one tab draws no bar");
        assert_eq!(side.row.current(), 0);
        assert_eq!(side.row_of_names(), vec!["mb".to_string()]);
    }

    #[test]
    fn a_new_tab_lands_after_the_current_one_and_takes_the_screen() {
        let mut side = Side::new("/home/mb");
        side.open("/tmp");
        side.open("/usr");
        assert_eq!(side.row.current(), 2);
        assert_eq!(side.where_am_i(), "/usr");
        assert_eq!(side.row_of_names(), ["mb", "tmp", "usr"]);
        // Back to the first, then a new one goes between it and the second.
        side.row.switch(0, &mut side.live, &mut side.tree);
        side.open("/etc");
        assert_eq!(side.row_of_names(), ["mb", "etc", "tmp", "usr"]);
        assert_eq!(side.where_am_i(), "/etc");
    }

    #[test]
    fn switching_keeps_every_tab_where_it_was() {
        let mut side = Side::new("/home/mb");
        side.live.cursor = 7;
        side.open("/tmp");
        side.live.cursor = 2;
        assert!(side.row.switch(0, &mut side.live, &mut side.tree));
        assert_eq!(side.where_am_i(), "/home/mb");
        assert_eq!(side.live.cursor, 7, "the first tab kept its cursor");
        assert!(side.row.switch(1, &mut side.live, &mut side.tree));
        assert_eq!(side.live.cursor, 2, "and so did the second");
    }

    #[test]
    fn switching_to_where_we_already_are_does_nothing() {
        let mut side = Side::new("/home/mb");
        side.open("/tmp");
        assert!(!side.row.switch(1, &mut side.live, &mut side.tree));
        assert!(!side.row.switch(9, &mut side.live, &mut side.tree));
        assert_eq!(side.where_am_i(), "/tmp");
    }

    #[test]
    fn the_ring_wraps_both_ways() {
        let mut side = Side::new("/a");
        side.open("/b");
        side.open("/c");
        assert_eq!(side.row.next(), 0, "the last one's next is the first");
        side.row.switch(0, &mut side.live, &mut side.tree);
        assert_eq!(
            side.row.prev(),
            2,
            "and the first one's previous is the last"
        );
    }

    #[test]
    fn closing_shows_the_one_to_the_right_then_the_one_to_the_left() {
        let mut side = Side::new("/a");
        side.open("/b");
        side.open("/c");
        side.row.switch(1, &mut side.live, &mut side.tree);
        assert!(side.row.close(&mut side.live, &mut side.tree));
        assert_eq!(
            side.where_am_i(),
            "/c",
            "the one to the right took its place"
        );
        assert_eq!(side.row_of_names(), ["a", "c"]);
        assert!(side.row.close(&mut side.live, &mut side.tree));
        assert_eq!(
            side.where_am_i(),
            "/a",
            "at the end of the row, the left one"
        );
    }

    #[test]
    fn the_last_tab_of_a_side_cannot_be_closed_or_moved() {
        let mut side = Side::new("/a");
        assert!(!side.row.close(&mut side.live, &mut side.tree));
        assert!(side.row.take(&mut side.live, &mut side.tree).is_none());
        assert_eq!(side.row.len(), 1);
        assert_eq!(side.where_am_i(), "/a");
    }

    #[test]
    fn a_tab_moves_to_the_other_side_and_leaves_this_one() {
        let mut left = Side::new("/a");
        left.open("/b");
        let mut right = Side::new("/x");
        let moved = left
            .row
            .take(&mut left.live, &mut left.tree)
            .expect("two tabs, so one can go");
        right.row.give(moved, &mut right.live, &mut right.tree);
        assert_eq!(left.row_of_names(), ["a"]);
        assert_eq!(left.where_am_i(), "/a");
        assert_eq!(right.row_of_names(), ["x", "b"]);
        assert_eq!(right.where_am_i(), "/b");
    }

    #[test]
    fn a_name_is_what_the_tab_is_called_and_otherwise_the_directory_is() {
        let mut side = Side::new("/home/mb/src/final");
        assert_eq!(side.row_of_names(), ["final"]);
        side.live.name = Some("work".into());
        assert_eq!(side.row_of_names(), ["work"]);
        side.live.name = Some("   ".into());
        assert_eq!(side.row_of_names(), ["final"], "a blank name is no name");
    }

    #[test]
    fn a_restored_row_comes_back_in_order() {
        let mut side = Side::new("/a");
        side.row.append(panel("/b"));
        side.row.append(panel("/c"));
        assert_eq!(side.row_of_names(), ["a", "b", "c"]);
        assert_eq!(side.row.current(), 0);
        assert!(side.row.switch(2, &mut side.live, &mut side.tree));
        assert_eq!(side.where_am_i(), "/c");
    }

    #[test]
    fn the_state_of_a_row_is_every_tab_with_its_name_and_its_lock() {
        let mut side = Side::new("/a");
        side.live.locked = true;
        side.open("/b");
        side.live.name = Some("scratch".into());
        let state = side.row.tabs_state(&side.live);
        assert_eq!(state.len(), 2);
        assert_eq!(state[0].path, "/a");
        assert!(state[0].locked);
        assert_eq!(state[1].name, "scratch");
        assert!(!state[1].locked);
    }
}
