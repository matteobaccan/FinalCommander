//! Two files side by side: what lines up, what does not, and where the eye should go.
//!
//! The alignment comes from `fcmd_core::diff`; this is the screen's half of it — the rows to
//! draw, the position in them, and the hunks to jump between (SPEC 05 §3). It holds both
//! files' lines in memory, as the Markdown render does and for the same reason: a side-by-side
//! view has to know how wide every line is before it can draw the first one, and a column
//! width guessed from the part on screen would shift as one scrolled.

use fcmd_core::diff::{self, Ignore, Op};

use crate::decode::Codepage;

/// Both files read whole, up to this much each. Two files one actually compares are source,
/// configuration or output; sixteen megabytes of either is a lot of reading.
pub const MOST: u64 = 16 * 1024 * 1024;

/// What a row of the two columns is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// The same line on both sides.
    Same,
    /// A line only the left side has.
    Removed,
    /// A line only the right side has.
    Added,
    /// A line both sides have in that place, with different text: the two inner ranges say
    /// which part of each differs.
    Changed,
}

/// One row of the side-by-side view: at most one line from each file.
#[derive(Debug, Clone)]
pub struct Row {
    pub kind: Side,
    /// The line number in each file, one-based, and `None` where that side has no line.
    pub left_no: Option<usize>,
    pub right_no: Option<usize>,
    pub left: String,
    pub right: String,
    /// On a `Changed` row, the part of each line that differs, as byte ranges.
    pub left_inner: std::ops::Range<usize>,
    pub right_inner: std::ops::Range<usize>,
}

impl Row {
    fn blank() -> Row {
        Row {
            kind: Side::Same,
            left_no: None,
            right_no: None,
            left: String::new(),
            right: String::new(),
            left_inner: 0..0,
            right_inner: 0..0,
        }
    }
}

/// Two files, lined up.
#[derive(Debug)]
pub struct Diff {
    pub left_name: String,
    pub right_name: String,
    left: Vec<String>,
    right: Vec<String>,
    rows: Vec<Row>,
    /// The first row of each hunk — a run of rows that are not `Same` — for `Alt+↑↓`.
    hunks: Vec<usize>,
    pub ignore: Ignore,
    /// First visible row.
    pub top: usize,
    /// Horizontal scroll, in characters, shared by both columns so the eye can compare.
    pub col: usize,
    /// Which column the cursor is in, for the key bar and for a future copy-to-the-other-side.
    pub right_focused: bool,
}

impl Diff {
    /// Read two files and line them up.
    ///
    /// Each side is decoded with whatever it looks like, so a file written under DOS and one
    /// written today compare as text rather than as bytes.
    pub fn new(
        left_name: String,
        left_bytes: &[u8],
        right_name: String,
        right_bytes: &[u8],
        ignore: Ignore,
    ) -> Diff {
        let split = |bytes: &[u8]| -> Vec<String> {
            let text = Codepage::decode(crate::decode::detect(bytes), bytes);
            // `split` and not `lines`, so a file that ends without a newline does not look
            // like one that does — and an empty file is no lines rather than one.
            let mut out: Vec<String> = text.split('\n').map(|l| l.to_string()).collect();
            if out.last().is_some_and(String::is_empty) {
                out.pop();
            }
            out
        };
        let mut diff = Diff {
            left_name,
            right_name,
            left: split(left_bytes),
            right: split(right_bytes),
            rows: Vec::new(),
            hunks: Vec::new(),
            ignore,
            top: 0,
            col: 0,
            right_focused: false,
        };
        diff.align();
        diff
    }

    /// Line the two sides up again — after the ignore options change, say.
    pub fn align(&mut self) {
        let left: Vec<&str> = self.left.iter().map(String::as_str).collect();
        let right: Vec<&str> = self.right.iter().map(String::as_str).collect();
        let ops = diff::lines(&left, &right, &self.ignore);

        self.rows.clear();
        for op in ops {
            match op {
                Op::Same { left, right, count } => {
                    for n in 0..count {
                        self.rows.push(Row {
                            kind: Side::Same,
                            left_no: Some(left + n + 1),
                            right_no: Some(right + n + 1),
                            left: self.left[left + n].clone(),
                            right: self.right[right + n].clone(),
                            ..Row::blank()
                        });
                    }
                }
                Op::Removed { left, count } => {
                    for n in 0..count {
                        self.rows.push(Row {
                            kind: Side::Removed,
                            left_no: Some(left + n + 1),
                            left: self.left[left + n].clone(),
                            ..Row::blank()
                        });
                    }
                }
                Op::Added { right, count } => {
                    for n in 0..count {
                        self.rows.push(Row {
                            kind: Side::Added,
                            right_no: Some(right + n + 1),
                            right: self.right[right + n].clone(),
                            ..Row::blank()
                        });
                    }
                }
            }
        }
        self.pair_up();
        self.find_hunks();
        let last = self.rows.len().saturating_sub(1);
        self.top = self.top.min(last);
    }

    /// Put a removed line and the added line that replaced it on one row.
    ///
    /// The alignment says "these went, those came"; a reader comparing two versions of a file
    /// wants the old line and the new one on the same row, with the words that differ marked.
    /// Only a run of removals immediately followed by a run of additions is paired, and only
    /// as far as the shorter of the two: what is left over stays a row of its own, because a
    /// line paired with nothing is a line that was added or taken away.
    fn pair_up(&mut self) {
        let mut out: Vec<Row> = Vec::with_capacity(self.rows.len());
        let mut at = 0;
        while at < self.rows.len() {
            let removed = self.rows[at..]
                .iter()
                .take_while(|r| r.kind == Side::Removed)
                .count();
            if removed == 0 {
                out.push(self.rows[at].clone());
                at += 1;
                continue;
            }
            let added = self.rows[at + removed..]
                .iter()
                .take_while(|r| r.kind == Side::Added)
                .count();
            let pairs = removed.min(added);
            for n in 0..pairs {
                let old = &self.rows[at + n];
                let new = &self.rows[at + removed + n];
                let (left_inner, right_inner) = diff::inside(&old.left, &new.right);
                out.push(Row {
                    kind: Side::Changed,
                    left_no: old.left_no,
                    right_no: new.right_no,
                    left: old.left.clone(),
                    right: new.right.clone(),
                    left_inner,
                    right_inner,
                });
            }
            for n in pairs..removed {
                out.push(self.rows[at + n].clone());
            }
            for n in pairs..added {
                out.push(self.rows[at + removed + n].clone());
            }
            at += removed + added;
        }
        self.rows = out;
    }

    fn find_hunks(&mut self) {
        self.hunks.clear();
        let mut inside = false;
        for (at, row) in self.rows.iter().enumerate() {
            match (row.kind == Side::Same, inside) {
                (false, false) => {
                    self.hunks.push(at);
                    inside = true;
                }
                (true, true) => inside = false,
                _ => {}
            }
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// How many rows the view has, which is not either file's line count: a line only one
    /// side has takes a row of its own.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// How many hunks there are, for the status line.
    pub fn hunks(&self) -> usize {
        self.hunks.len()
    }

    /// Are the two files the same, under the options in force?
    pub fn identical(&self) -> bool {
        self.hunks.is_empty()
    }

    /// The rows on screen, given the height.
    pub fn window(&self, height: u16) -> &[Row] {
        let height = height as usize;
        let end = (self.top + height).min(self.rows.len());
        &self.rows[self.top.min(end)..end]
    }

    pub fn scroll(&mut self, delta: i64, height: u16) {
        let last = self.rows.len().saturating_sub(1);
        let top = self.top as i64 + delta;
        self.top = top.clamp(0, last as i64) as usize;
        let _ = height;
    }

    pub fn to_start(&mut self) {
        self.top = 0;
    }

    pub fn to_end(&mut self, height: u16) {
        let height = (height as usize).max(1);
        self.top = self.rows.len().saturating_sub(height);
    }

    pub fn scroll_sideways(&mut self, delta: i64) {
        let col = self.col as i64 + delta;
        self.col = col.max(0) as usize;
    }

    /// The next hunk after the top row, or the first one when there is none after it.
    ///
    /// `Alt+↓` in a diff is the key one presses forty times, so it never does nothing when
    /// there is somewhere to go: at the last hunk it says so by returning `false`.
    pub fn next_hunk(&mut self) -> bool {
        match self.hunks.iter().find(|at| **at > self.top) {
            Some(at) => {
                self.top = *at;
                true
            }
            None => false,
        }
    }

    pub fn previous_hunk(&mut self) -> bool {
        match self.hunks.iter().rev().find(|at| **at < self.top) {
            Some(at) => {
                self.top = *at;
                true
            }
            None => false,
        }
    }

    /// Which hunk the top row is in or after, one-based, for the status line.
    pub fn hunk_at(&self) -> usize {
        self.hunks.iter().filter(|at| **at <= self.top).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diff(left: &str, right: &str) -> Diff {
        Diff::new(
            "left".into(),
            left.as_bytes(),
            "right".into(),
            right.as_bytes(),
            Ignore::default(),
        )
    }

    #[test]
    fn two_files_that_are_the_same_have_no_hunk() {
        let d = diff("one\ntwo\n", "one\ntwo\n");
        assert_eq!(d.len(), 2);
        assert!(d.identical());
        assert!(d.rows().iter().all(|r| r.kind == Side::Same));
    }

    #[test]
    fn a_changed_line_is_one_row_with_the_words_marked() {
        let d = diff("the quick brown fox\nsame\n", "the quick red fox\nsame\n");
        assert_eq!(d.len(), 2, "{:?}", d.rows());
        let row = &d.rows()[0];
        assert_eq!(row.kind, Side::Changed);
        assert_eq!(row.left_no, Some(1));
        assert_eq!(row.right_no, Some(1));
        assert_eq!(&row.left[row.left_inner.clone()], "brown");
        assert_eq!(&row.right[row.right_inner.clone()], "red");
        assert_eq!(d.hunks(), 1);
    }

    #[test]
    fn an_added_line_takes_a_row_with_nothing_on_the_left() {
        let d = diff("one\ntwo\n", "one\nand a half\ntwo\n");
        assert_eq!(d.len(), 3, "{:?}", d.rows());
        let row = &d.rows()[1];
        assert_eq!(row.kind, Side::Added);
        assert_eq!(row.left_no, None);
        assert_eq!(row.right_no, Some(2));
        assert_eq!(row.right, "and a half");
    }

    /// Two removals and one addition: the first pairs, the second stays its own row. A line
    /// paired with nothing is a line that was taken away.
    #[test]
    fn a_run_pairs_as_far_as_the_shorter_side() {
        let d = diff("a\nb\nkeep\n", "A\nkeep\n");
        assert_eq!(d.len(), 3, "{:?}", d.rows());
        assert_eq!(d.rows()[0].kind, Side::Changed);
        assert_eq!(d.rows()[1].kind, Side::Removed);
        assert_eq!(d.rows()[1].left, "b");
        assert_eq!(d.rows()[2].kind, Side::Same);
    }

    #[test]
    fn the_hunks_are_what_alt_arrows_walk() {
        let d = &mut diff(
            "same\nsame\nA\nsame\nsame\nsame\nB\nsame\n",
            "same\nsame\nx\nsame\nsame\nsame\ny\nsame\n",
        );
        assert_eq!(d.hunks(), 2);
        assert_eq!(d.top, 0);
        assert!(d.next_hunk());
        assert_eq!(d.top, 2);
        assert_eq!(d.hunk_at(), 1);
        assert!(d.next_hunk());
        assert_eq!(d.top, 6);
        assert_eq!(d.hunk_at(), 2);
        assert!(!d.next_hunk(), "there is no third hunk");
        assert!(d.previous_hunk());
        assert_eq!(d.top, 2);
        assert!(!d.previous_hunk());
    }

    #[test]
    fn what_is_ignored_can_make_two_files_the_same() {
        let mut d = diff("One  Two\n", "one two\n");
        assert!(!d.identical());
        d.ignore = Ignore {
            whitespace: true,
            case: true,
            line_endings: true,
        };
        d.align();
        assert!(d.identical(), "{:?}", d.rows());
    }

    #[test]
    fn an_empty_file_against_a_full_one_is_all_additions() {
        let d = diff("", "one\ntwo\n");
        assert_eq!(d.len(), 2);
        assert!(d.rows().iter().all(|r| r.kind == Side::Added));
        let d = diff("", "");
        assert!(d.is_empty() && d.identical());
    }

    #[test]
    fn the_window_is_what_the_screen_can_hold() {
        let body: String = (0..100).map(|n| format!("line {n}\n")).collect();
        let mut d = diff(&body, &body);
        assert_eq!(d.window(10).len(), 10);
        d.scroll(95, 10);
        assert_eq!(d.top, 95);
        assert_eq!(d.window(10).len(), 5, "the end is the end");
        d.to_end(10);
        assert_eq!(d.top, 90);
        d.to_start();
        assert_eq!(d.top, 0);
        d.scroll(-5, 10);
        assert_eq!(d.top, 0, "and it does not go negative");
    }
}
