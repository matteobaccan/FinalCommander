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

/// One line of the unified layout: a row of the two columns, or one side of it.
///
/// The unified layout is a second way of drawing the same rows, not a second alignment: a
/// line both files have is one line, and a run of rows that differ is every line the left
/// file loses followed by every line the right one brings, as `diff -u` writes a hunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line {
    /// `' '`, `'-'` or `'+'`: what a unified diff puts in front of the line.
    pub sign: char,
    /// The row of the two columns this line is, or is one side of.
    pub row: usize,
}

/// How many lines of context a saved patch carries around each change, which is what `diff
/// -u` and git write and what `patch` expects to find its place by.
pub const CONTEXT: usize = 3;

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
    /// The same rows as one column, for the unified layout, and where in it each row starts.
    lines: Vec<Line>,
    line_of_row: Vec<usize>,
    /// One column rather than two (`U`). The position, the hunks and a merge are the rows'
    /// either way, so the two layouts are two views of one place.
    pub unified: bool,
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
            lines: Vec::new(),
            line_of_row: Vec::new(),
            unified: false,
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
        self.lay_unified();
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

    /// The rows again as one column. A run of rows that differ becomes the left file's lines
    /// of it and then the right file's, so a run of `n` rows is between `n` and `2n` lines;
    /// each of its rows is given a place spread evenly through those lines, which is what
    /// lets the position stay a row while the screen scrolls through every line.
    fn lay_unified(&mut self) {
        self.lines.clear();
        self.line_of_row.clear();
        self.line_of_row.resize(self.rows.len(), 0);
        let mut at = 0;
        while at < self.rows.len() {
            if self.rows[at].kind == Side::Same {
                self.line_of_row[at] = self.lines.len();
                self.lines.push(Line { sign: ' ', row: at });
                at += 1;
                continue;
            }
            let end = self.rows[at..]
                .iter()
                .position(|r| r.kind == Side::Same)
                .map_or(self.rows.len(), |n| at + n);
            let first = self.lines.len();
            for row in at..end {
                if self.rows[row].left_no.is_some() {
                    self.lines.push(Line { sign: '-', row });
                }
            }
            for row in at..end {
                if self.rows[row].right_no.is_some() {
                    self.lines.push(Line { sign: '+', row });
                }
            }
            let (rows, lines) = (end - at, self.lines.len() - first);
            for n in 0..rows {
                self.line_of_row[at + n] = first + n * lines / rows;
            }
            at = end;
        }
    }

    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// The lines of the unified layout on screen, given the height: from the line the top
    /// row is at.
    pub fn unified_window(&self, height: u16) -> &[Line] {
        let from = self
            .line_of_row
            .get(self.top)
            .copied()
            .unwrap_or(self.lines.len());
        let end = (from + height as usize).min(self.lines.len());
        &self.lines[from.min(end)..end]
    }

    /// What a line of the unified layout shows: its number in each file, its text, and on a
    /// changed line the part that differs.
    pub fn line(
        &self,
        line: &Line,
    ) -> (Option<usize>, Option<usize>, &str, std::ops::Range<usize>) {
        let row = &self.rows[line.row];
        let changed = row.kind == Side::Changed;
        match line.sign {
            '-' => (
                row.left_no,
                None,
                &row.left,
                if changed {
                    row.left_inner.clone()
                } else {
                    0..0
                },
            ),
            '+' => (
                None,
                row.right_no,
                &row.right,
                if changed {
                    row.right_inner.clone()
                } else {
                    0..0
                },
            ),
            _ => (row.left_no, row.right_no, &row.left, 0..0),
        }
    }

    /// The whole comparison as a unified diff, the kind `patch` and `git apply` take: the
    /// two names, then each hunk with [`CONTEXT`] lines around it, the lines exactly as the
    /// files have them. `None` when the two files do not differ, which is a patch of nothing.
    ///
    /// It is written from the alignment on screen, so with blanks or case ignored a line the
    /// screen calls the same is context here in the left file's spelling: a patch to read,
    /// as `diff -uw` makes one, rather than one that turns the left file into the right.
    pub fn patch(&self) -> Option<String> {
        // Each line as it will be written, with whether it is the last of a file that does
        // not end in a newline — which a patch has to say, or applying it would add one.
        struct Out<'a> {
            sign: char,
            text: &'a str,
            bare: bool,
        }
        let mut out: Vec<Out> = Vec::with_capacity(self.lines.len() + 1);
        for line in &self.lines {
            let (left_no, right_no, text, _) = self.line(line);
            let left_bare = !self.left_final_eol && left_no == Some(self.left.len());
            let right_bare = !self.right_final_eol && right_no == Some(self.right.len());
            match line.sign {
                // The last line of both, where only one file ends it with a newline: the
                // text is the same and the files are not, so it is a change of its own.
                ' ' if left_bare != right_bare => {
                    out.push(Out {
                        sign: '-',
                        text,
                        bare: left_bare,
                    });
                    out.push(Out {
                        sign: '+',
                        text: &self.rows[line.row].right,
                        bare: right_bare,
                    });
                }
                sign => out.push(Out {
                    sign,
                    text,
                    bare: if sign == '+' { right_bare } else { left_bare },
                }),
            }
        }
        let changed: Vec<usize> = (0..out.len()).filter(|i| out[*i].sign != ' ').collect();
        if changed.is_empty() {
            return None;
        }
        let mut text = format!("--- {}\n+++ {}\n", self.left_name, self.right_name);
        let mut at = 0;
        while at < changed.len() {
            // A hunk runs on for as long as the next change is close enough for the two
            // contexts to touch.
            let mut last = at;
            while last + 1 < changed.len() && changed[last + 1] - changed[last] <= 2 * CONTEXT {
                last += 1;
            }
            let from = changed[at].saturating_sub(CONTEXT);
            let to = (changed[last] + CONTEXT + 1).min(out.len());
            let count = |lines: &[Out], skip: char| lines.iter().filter(|l| l.sign != skip).count();
            let (left_len, right_len) = (count(&out[from..to], '+'), count(&out[from..to], '-'));
            // A side with no lines in the hunk is named by the line before it, as diff does.
            let start = |before: usize, len: usize| if len == 0 { before } else { before + 1 };
            text.push_str(&format!(
                "@@ -{},{} +{},{} @@\n",
                start(count(&out[..from], '+'), left_len),
                left_len,
                start(count(&out[..from], '-'), right_len),
                right_len
            ));
            for line in &out[from..to] {
                text.push(line.sign);
                text.push_str(line.text);
                text.push('\n');
                if line.bare {
                    text.push_str("\\ No newline at end of file\n");
                }
            }
            at = last + 1;
        }
        Some(text)
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

    /// The hunk at the top as the body of a unified patch from the left side to the right,
    /// with no lines of context: `@@ -a,n +b,m @@`, the left's lines with `-`, the right's
    /// with `+`, each line exactly as the file has it. What `git apply --unidiff-zero` takes
    /// to stage one hunk and leave the rest. `None` when the top row is not in a hunk.
    pub fn hunk_patch(&self) -> Option<String> {
        let rows = self.hunk_rows()?;
        let before = &self.rows[..rows.start];
        let (left_from, right_from) = (
            before.iter().filter(|r| r.left_no.is_some()).count(),
            before.iter().filter(|r| r.right_no.is_some()).count(),
        );
        let inside = &self.rows[rows];
        let (left_len, right_len) = (
            inside.iter().filter(|r| r.left_no.is_some()).count(),
            inside.iter().filter(|r| r.right_no.is_some()).count(),
        );
        // An empty range is named by the line before it, as diff writes one.
        let start = |from: usize, len: usize| if len == 0 { from } else { from + 1 };
        let mut out = format!(
            "@@ -{},{} +{},{} @@\n",
            start(left_from, left_len),
            left_len,
            start(right_from, right_len),
            right_len
        );
        let mut lines = |sign: char, all: &[String], from: usize, len: usize, eol: bool| {
            for line in &all[from..from + len] {
                out.push(sign);
                out.push_str(line);
                out.push('\n');
            }
            if len > 0 && from + len == all.len() && !eol {
                out.push_str("\\ No newline at end of file\n");
            }
        };
        lines('-', &self.left, left_from, left_len, self.left_final_eol);
        lines(
            '+',
            &self.right,
            right_from,
            right_len,
            self.right_final_eol,
        );
        Some(out)
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

    /// The hunk at the top as a patch with no context: a changed line, a line added, a
    /// line gone, and the last line of a file with no newline after it.
    #[test]
    fn a_hunk_becomes_a_patch_of_its_own() {
        let mut d = diff("a\nb\nc\n", "a\nB\nc\nd\n");
        assert!(d.next_hunk());
        assert_eq!(d.hunk_patch().unwrap(), "@@ -2,1 +2,1 @@\n-b\n+B\n");
        assert!(d.next_hunk());
        assert_eq!(d.hunk_patch().unwrap(), "@@ -3,0 +4,1 @@\n+d\n");
        let mut d = diff("a\nb\nc", "a\nc");
        assert!(d.next_hunk());
        assert_eq!(d.hunk_patch().unwrap(), "@@ -2,1 +1,0 @@\n-b\n");
        let mut d = diff("a\nb", "a\nc");
        assert!(d.next_hunk());
        assert_eq!(
            d.hunk_patch().unwrap(),
            "@@ -2,1 +2,1 @@\n-b\n\\ No newline at end of file\n+c\n\\ No newline at end of file\n"
        );
        d.to_start();
        assert!(d.hunk_patch().is_none(), "a line both have is no hunk");
    }

    /// The unified layout is the same rows as one column: a line both have once, and a run
    /// of rows that differ as what went and then what came.
    #[test]
    fn the_unified_layout_is_what_went_and_then_what_came() {
        let mut d = diff("a\nb\nc\nd\n", "a\nB\nC\nd\ne\n");
        let shown = |d: &Diff| -> Vec<String> {
            d.unified_window(100)
                .iter()
                .map(|l| {
                    let (left, right, text, _) = d.line(l);
                    let no = |n: Option<usize>| n.map_or("-".to_string(), |n| n.to_string());
                    format!("{} {} {}{}", no(left), no(right), l.sign, text)
                })
                .collect()
        };
        assert_eq!(
            shown(&d),
            ["1 1  a", "2 - -b", "3 - -c", "- 2 +B", "- 3 +C", "4 4  d", "- 5 +e"]
        );
        // The part of a changed line that differs is marked on each of its two lines.
        let lines = d.unified_window(100).to_vec();
        assert_eq!(d.line(&lines[1]).3, 0..1);
        assert_eq!(d.line(&lines[3]).3, 0..1);
        assert_eq!(d.line(&lines[0]).3, 0..0);
        // The position is still a row: the second hunk starts at the line that adds `e`.
        assert!(d.next_hunk());
        assert!(d.next_hunk());
        assert_eq!(shown(&d), ["- 5 +e"]);
        // And the second row of the first hunk is half way through its four lines.
        d.top = 2;
        assert_eq!(shown(&d)[0], "- 2 +B");
    }

    /// The comparison saved as a patch: the names, three lines of context, two changes close
    /// together in one hunk and one far away in its own.
    #[test]
    fn the_comparison_is_written_as_a_unified_diff() {
        let left: String = (1..=20).map(|n| format!("line {n}\n")).collect();
        let right = left
            .replace("line 2\n", "line two\n")
            .replace("line 5\n", "")
            .replace("line 18\n", "line 18\nline 18 and a half\n");
        let d = diff(&left, &right);
        assert_eq!(
            d.patch().unwrap(),
            "--- left\n+++ right\n\
             @@ -1,8 +1,7 @@\n line 1\n-line 2\n+line two\n line 3\n line 4\n-line 5\n line 6\n line 7\n line 8\n\
             @@ -16,5 +15,6 @@\n line 16\n line 17\n line 18\n+line 18 and a half\n line 19\n line 20\n"
        );
        assert!(
            diff("same\n", "same\n").patch().is_none(),
            "nothing to save"
        );
    }

    /// The edges a patch has to get right: a file that was empty, a file that becomes empty,
    /// and a last line with no newline after it — on one side, or on both.
    #[test]
    fn a_patch_says_what_an_empty_file_and_a_missing_newline_are() {
        assert_eq!(
            diff("", "a\nb\n").patch().unwrap(),
            "--- left\n+++ right\n@@ -0,0 +1,2 @@\n+a\n+b\n"
        );
        assert_eq!(
            diff("a\n", "").patch().unwrap(),
            "--- left\n+++ right\n@@ -1,1 +0,0 @@\n-a\n"
        );
        assert_eq!(
            diff("a\nb", "a\nc").patch().unwrap(),
            "--- left\n+++ right\n@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+c\n\\ No newline at end of file\n"
        );
        // The same text, and only one of the two ends it with a newline.
        assert_eq!(
            diff("a\nb", "a\nb\n").patch().unwrap(),
            "--- left\n+++ right\n@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+b\n"
        );
        // A context line that is the last of both files, neither ending it.
        assert_eq!(
            diff("x\nb", "y\nb").patch().unwrap(),
            "--- left\n+++ right\n@@ -1,2 +1,2 @@\n-x\n+y\n b\n\\ No newline at end of file\n"
        );
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
