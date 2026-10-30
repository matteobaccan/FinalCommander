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
    /// Which column the cursor is in: the side a merge writes into, and the side the key bar
    /// names.
    pub right_focused: bool,
    /// How each file ends its lines, for a merge: a line taken from the other side is re-ended
    /// to match the file it is going into, so merging a Unix file into a DOS one does not
    /// leave one bare newline in the middle of it.
    ///
    /// The lines themselves keep whatever they came with — the split is on `\n` alone, so a
    /// `\r` stays at the end of the line it belongs to. That is what lets a file be written
    /// back exactly as it was read, and what [`Ignore::line_endings`] is there to see past.
    left_crlf: bool,
    right_crlf: bool,
    /// Did each file end with a newline? A file that did not is not given one back.
    left_final_eol: bool,
    right_final_eol: bool,
    /// Has either side been merged into since it was read? What is on screen is then not what
    /// is on the disk, and `F2` is the only thing that puts it there.
    pub left_edited: bool,
    pub right_edited: bool,
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
        // Which ending the file uses, by counting rather than by the first one found: a file
        // that is mostly DOS with one stray Unix line is a DOS file, and a merge into it
        // should write DOS lines.
        let crlf = |bytes: &[u8]| -> bool {
            let lf = bytes.iter().filter(|b| **b == b'\n').count();
            let crlf = bytes.windows(2).filter(|w| w == b"\r\n").count();
            lf > 0 && crlf * 2 > lf
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
            left_crlf: crlf(left_bytes),
            right_crlf: crlf(right_bytes),
            left_final_eol: left_bytes.last() == Some(&b'\n'),
            right_final_eol: right_bytes.last() == Some(&b'\n'),
            left_edited: false,
            right_edited: false,
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

    /// The run of rows the hunk at the top of the screen covers, or `None` when the top row
    /// is a line both files agree about.
    ///
    /// The top row is the position in this view — there is no cursor of its own — so the
    /// hunk one acts on is the one at the top, which is exactly where `Alt+↓` parks one.
    fn hunk_rows(&self) -> Option<std::ops::Range<usize>> {
        if self.rows.get(self.top)?.kind == Side::Same {
            return None;
        }
        let start = self.rows[..self.top]
            .iter()
            .rposition(|r| r.kind == Side::Same)
            .map_or(0, |at| at + 1);
        let end = self.rows[self.top..]
            .iter()
            .position(|r| r.kind == Side::Same)
            .map_or(self.rows.len(), |at| self.top + at);
        Some(start..end)
    }

    /// Take the other side's version of the hunk at the top: Midnight Commander's `F5`.
    ///
    /// The focused column is the one written into, so `Tab` chooses the direction and the
    /// key bar says which way it will go. What moves is whole lines, because a hunk is whole
    /// lines: a row only the other side has is inserted, a row only this side has goes, and a
    /// row both have with different text is replaced.
    ///
    /// Nothing is written to any disk. The side is marked as edited and `F2` is the only
    /// thing that puts it there, which is what lets one merge four hunks and look at the
    /// result before committing to it.
    ///
    /// `false` when the top row is not in a hunk, which is the caller's cue to say so.
    pub fn merge_hunk(&mut self) -> bool {
        let Some(rows) = self.hunk_rows() else {
            return false;
        };
        // Where the hunk starts in each file, counted rather than read off the line numbers:
        // a hunk one side has no lines in has no line number to read, and the place it goes
        // is still exactly "after everything before it".
        let before = &self.rows[..rows.start];
        let (left_from, right_from) = (
            before.iter().filter(|r| r.left_no.is_some()).count(),
            before.iter().filter(|r| r.right_no.is_some()).count(),
        );
        let inside = &self.rows[rows.clone()];
        let (left_len, right_len) = (
            inside.iter().filter(|r| r.left_no.is_some()).count(),
            inside.iter().filter(|r| r.right_no.is_some()).count(),
        );
        // Re-ended to match the file it is going into: a Unix line dropped into a DOS file
        // would otherwise be the one line in it that ends differently.
        let reend = |lines: &[String], crlf: bool| -> Vec<String> {
            lines
                .iter()
                .map(|line| {
                    let bare = line.strip_suffix('\r').unwrap_or(line);
                    if crlf {
                        format!("{bare}\r")
                    } else {
                        bare.to_string()
                    }
                })
                .collect()
        };
        if self.right_focused {
            let taken = reend(&self.left[left_from..left_from + left_len], self.right_crlf);
            self.right.splice(right_from..right_from + right_len, taken);
            self.right_edited = true;
        } else {
            let taken = reend(
                &self.right[right_from..right_from + right_len],
                self.left_crlf,
            );
            self.left.splice(left_from..left_from + left_len, taken);
            self.left_edited = true;
        }
        // Lined up again, and the eye kept where it was: the hunk that was at the top has
        // gone, so the top row now holds whatever took its place.
        self.align();
        self.top = rows.start.min(self.rows.len().saturating_sub(1));
        true
    }

    /// Has either side been merged into since it was read?
    pub fn edited(&self) -> bool {
        self.left_edited || self.right_edited
    }

    /// One side as it now stands, ready to be written: its own line endings, because the `\r`
    /// never left the lines, and its own answer about a final newline.
    pub fn text(&self, right: bool) -> String {
        let (lines, final_eol) = if right {
            (&self.right, self.right_final_eol)
        } else {
            (&self.left, self.left_final_eol)
        };
        let mut out = lines.join("\n");
        if final_eol && !lines.is_empty() {
            out.push('\n');
        }
        out
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

    /// `F5`: the focused side takes the other one's version of the hunk at the top. Three
    /// shapes, because a hunk is one of three things — a line both sides have and differ
    /// about, a line only one side has, and a line only the other side has.
    #[test]
    fn a_hunk_is_taken_from_the_other_side_into_the_focused_one() {
        // A changed line, merged leftwards: the left file ends up as the right one.
        let mut d = diff("one\ntwo\nthree\n", "one\nTWO\nthree\n");
        assert!(!d.identical());
        assert!(d.next_hunk(), "there is a hunk to stand on");
        assert!(d.merge_hunk(), "and it is taken");
        assert!(d.left_edited && !d.right_edited, "the focused side changed");
        assert_eq!(d.text(false), "one\nTWO\nthree\n");
        assert!(d.identical(), "and the two files now agree: {:?}", d.rows());

        // A line only the right side has: merged leftwards it is inserted, in its place.
        let mut d = diff("one\nthree\n", "one\ntwo\nthree\n");
        assert!(d.next_hunk());
        assert!(d.merge_hunk());
        assert_eq!(d.text(false), "one\ntwo\nthree\n");
        assert!(d.identical());

        // A line only the left side has: merged *rightwards* it is added there, and merged
        // leftwards it would go. `Tab` is what chooses, so this is the same key both ways.
        let mut d = diff("one\ntwo\nthree\n", "one\nthree\n");
        d.right_focused = true;
        assert!(d.next_hunk());
        assert!(d.merge_hunk());
        assert!(d.right_edited && !d.left_edited);
        assert_eq!(d.text(true), "one\ntwo\nthree\n");
        assert!(d.identical());
    }

    /// The top row is the position, so a merge asked for on a line both files agree about is
    /// not a merge of the next hunk by accident: it says there is nothing here.
    #[test]
    fn nothing_is_merged_from_a_line_the_two_files_agree_about() {
        let mut d = diff("one\ntwo\n", "one\nTWO\n");
        assert_eq!(d.top, 0, "which is a line they agree about");
        assert!(!d.merge_hunk());
        assert!(!d.edited());
        assert_eq!(d.text(false), "one\ntwo\n");
    }

    /// A file read and written back without being touched is the same file, down to the line
    /// endings and the last byte: the `\r` of a DOS file stays on its line, and a file that
    /// ended without a newline does not get one.
    #[test]
    fn a_side_written_back_untouched_is_the_file_that_was_read() {
        for text in ["a\nb\n", "a\r\nb\r\n", "a\nb", "", "\n"] {
            let d = diff(text, "zzz\n");
            assert_eq!(d.text(false), text, "{text:?}");
        }
    }

    /// A line taken out of a Unix file into a DOS one is written as a DOS line: one bare
    /// newline in the middle of a file is the kind of thing that shows up in a later diff
    /// and not in this one.
    ///
    /// With the line endings ignored, which is the only way these two files have one hunk
    /// between them rather than three — and the way anybody actually compares a file off a
    /// Windows machine with the copy here.
    #[test]
    fn a_line_merged_into_a_dos_file_is_given_dos_line_endings() {
        let mut d = Diff::new(
            "left".into(),
            b"one\r\ntwo\r\nthree\r\n",
            "right".into(),
            b"one\nTWO\nthree\n",
            Ignore {
                line_endings: true,
                ..Ignore::default()
            },
        );
        assert!(d.next_hunk(), "the one line they differ about");
        assert!(d.merge_hunk(), "leftwards, into the DOS file");
        assert_eq!(d.text(false), "one\r\nTWO\r\nthree\r\n");
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
