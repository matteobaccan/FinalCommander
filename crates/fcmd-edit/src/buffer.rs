use std::path::Path;

use fcmd_view::Codepage;

/// Which line terminator the file uses. The editor keeps what it found: changing every line
/// ending of a file because someone fixed a typo is the kind of thing that ruins a diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Eol {
    #[default]
    Lf,
    Crlf,
    Cr,
}

impl Eol {
    pub fn as_str(self) -> &'static str {
        match self {
            Eol::Lf => "LF",
            Eol::Crlf => "CRLF",
            Eol::Cr => "CR",
        }
    }

    pub fn bytes(self) -> &'static str {
        match self {
            Eol::Lf => "\n",
            Eol::Crlf => "\r\n",
            Eol::Cr => "\r",
        }
    }

    /// The dominant terminator in `text`; mixed files keep the majority and are reported.
    pub fn detect(text: &str) -> (Eol, bool) {
        let crlf = text.matches("\r\n").count();
        let cr = text.matches('\r').count() - crlf;
        let lf = text.matches('\n').count() - crlf;
        let mixed = [crlf, cr, lf].iter().filter(|n| **n > 0).count() > 1;
        if crlf == 0 && cr == 0 && lf == 0 {
            // A new or single-line file has nothing to preserve: use the platform's.
            let default = if cfg!(windows) { Eol::Crlf } else { Eol::Lf };
            return (default, false);
        }
        let eol = if crlf >= lf && crlf >= cr {
            Eol::Crlf
        } else if cr > lf {
            Eol::Cr
        } else {
            Eol::Lf
        };
        (eol, mixed)
    }
}

/// A position in the buffer: line index, and column in **characters** (not bytes, not display
/// columns — those conversions belong to the view).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

impl Position {
    pub const fn new(line: usize, col: usize) -> Self {
        Position { line, col }
    }
}

/// A half-open range of text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

impl Range {
    pub fn new(start: Position, end: Position) -> Self {
        if start <= end {
            Range { start, end }
        } else {
            Range {
                start: end,
                end: start,
            }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

/// One undoable step. Edits are stored as what to do to get *back*, which is all an undo stack
/// needs and makes redo the inverse of the inverse.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Edit {
    /// Put `text` back at `at` (undoing a delete).
    Insert { at: Position, text: String },
    /// Remove `range` (undoing an insert).
    Delete { range: Range, text: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Group {
    edits: Vec<Edit>,
    /// Where the cursor was before the change, so undo puts it back.
    cursor_before: Position,
    cursor_after: Position,
}

#[derive(Debug, thiserror::Error)]
pub enum BufferError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("{path}: {size} bytes is over the {limit} byte editor limit")]
    TooBig { path: String, size: u64, limit: u64 },
    #[error("the buffer is read-only")]
    ReadOnly,
}

/// Files above this open read-only, rather than pulling a gigabyte into memory and pretending.
pub const MAX_OPEN: u64 = 256 * 1024 * 1024;

/// A text buffer: lines, the metadata needed to write them back unchanged, and undo history.
#[derive(Debug, Clone)]
pub struct Buffer {
    lines: Vec<String>,
    pub eol: Eol,
    pub mixed_eol: bool,
    pub codepage: Codepage,
    pub bom: bool,
    /// A file with no terminator on its last line keeps it that way.
    pub final_newline: bool,
    pub read_only: bool,
    modified: bool,
    undo: Vec<Group>,
    redo: Vec<Group>,
    /// Edits are grouped until this is taken; a burst of typing is one undo step.
    open_group: Option<Group>,
    /// The lowest line changed since anybody last asked, or `usize::MAX` for none.
    ///
    /// Syntax highlighting remembers what it scanned, and a cache has to be told how far
    /// back to throw away. Every change goes through `insert_raw` or `delete_raw`, so that
    /// is where it is recorded — undo and redo included, since they go through them too.
    touched: usize,
}

impl Default for Buffer {
    fn default() -> Self {
        Buffer::from_text("")
    }
}

impl Buffer {
    pub fn from_text(text: &str) -> Buffer {
        let (eol, mixed_eol) = Eol::detect(text);
        let final_newline = text.ends_with('\n') || text.ends_with('\r');
        let lines = split_lines(text);
        Buffer {
            lines,
            eol,
            mixed_eol,
            codepage: Codepage::Utf8,
            bom: false,
            final_newline,
            read_only: false,
            modified: false,
            undo: Vec::new(),
            redo: Vec::new(),
            open_group: None,
            touched: usize::MAX,
        }
    }

    /// Read a file, keeping its encoding, its BOM and its line endings.
    pub fn open(path: &Path) -> Result<Buffer, BufferError> {
        let meta = std::fs::metadata(path).map_err(|source| BufferError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let bytes = std::fs::read(path).map_err(|source| BufferError::Io {
            path: path.display().to_string(),
            source,
        })?;
        let too_big = meta.len() > MAX_OPEN;

        let (codepage, bom_len) = match fcmd_view::sniff_bom(&bytes) {
            Some((cp, len)) => (cp, len),
            None => (fcmd_view::detect(&bytes[..bytes.len().min(4096)]), 0),
        };
        let text = codepage.decode(&bytes[bom_len..]);
        let mut buffer = Buffer::from_text(&text);
        buffer.codepage = codepage;
        buffer.bom = bom_len > 0;
        buffer.read_only = too_big || meta.permissions().readonly();
        Ok(buffer)
    }

    /// The whole buffer as text, with the original line endings and final-newline policy.
    pub fn to_text(&self) -> String {
        let mut out = String::with_capacity(self.lines.iter().map(|l| l.len() + 2).sum());
        for (i, line) in self.lines.iter().enumerate() {
            out.push_str(line);
            if i + 1 < self.lines.len() || self.final_newline {
                out.push_str(self.eol.bytes());
            }
        }
        out
    }

    /// Encode for writing: the BOM, if there was one, then the text in its codepage.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            match self.codepage {
                Codepage::Utf8 => out.extend_from_slice(&[0xEF, 0xBB, 0xBF]),
                Codepage::Utf16Le => out.extend_from_slice(&[0xFF, 0xFE]),
                Codepage::Utf16Be => out.extend_from_slice(&[0xFE, 0xFF]),
                _ => {}
            }
        }
        out.extend_from_slice(&self.codepage.encode(&self.to_text()));
        out
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.len() == 1 && self.lines[0].is_empty()
    }

    pub fn line(&self, index: usize) -> &str {
        self.lines.get(index).map(String::as_str).unwrap_or("")
    }

    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    pub fn line_chars(&self, index: usize) -> usize {
        self.line(index).chars().count()
    }

    /// Say that `line` and everything after it may have changed.
    pub fn touch(&mut self, line: usize) {
        self.touched = self.touched.min(line);
    }

    /// The lowest line changed since this was last called, and forget it.
    pub fn take_touched(&mut self) -> Option<usize> {
        let touched = std::mem::replace(&mut self.touched, usize::MAX);
        (touched != usize::MAX).then_some(touched)
    }

    pub fn modified(&self) -> bool {
        self.modified
    }

    pub fn mark_saved(&mut self) {
        self.modified = false;
    }

    /// The last position in the buffer.
    pub fn end(&self) -> Position {
        let line = self.lines.len() - 1;
        Position::new(line, self.line_chars(line))
    }

    /// Clamp a position onto real text.
    pub fn clamp(&self, position: Position) -> Position {
        let line = position.line.min(self.lines.len() - 1);
        Position::new(line, position.col.min(self.line_chars(line)))
    }

    pub fn text_in(&self, range: Range) -> String {
        let range = Range::new(self.clamp(range.start), self.clamp(range.end));
        if range.is_empty() {
            return String::new();
        }
        if range.start.line == range.end.line {
            return self
                .line(range.start.line)
                .chars()
                .skip(range.start.col)
                .take(range.end.col - range.start.col)
                .collect();
        }
        let mut out: String = self
            .line(range.start.line)
            .chars()
            .skip(range.start.col)
            .collect();
        for line in range.start.line + 1..range.end.line {
            out.push('\n');
            out.push_str(self.line(line));
        }
        out.push('\n');
        out.extend(self.line(range.end.line).chars().take(range.end.col));
        out
    }

    // ── editing ────────────────────────────────────────────────────────────────────────────

    /// Begin (or continue) an undo group. Typing bursts share one.
    pub fn begin_group(&mut self, cursor: Position) {
        if self.open_group.is_none() {
            self.open_group = Some(Group {
                edits: Vec::new(),
                cursor_before: cursor,
                cursor_after: cursor,
            });
        }
    }

    /// Close the current undo group: the next edit starts a new one.
    pub fn end_group(&mut self, cursor: Position) {
        if let Some(mut group) = self.open_group.take() {
            if group.edits.is_empty() {
                return;
            }
            group.cursor_after = cursor;
            self.undo.push(group);
            self.redo.clear();
        }
    }

    fn record(&mut self, edit: Edit, cursor: Position) {
        self.begin_group(cursor);
        if let Some(group) = &mut self.open_group {
            group.edits.push(edit);
        }
        self.modified = true;
    }

    /// Insert text at `at`, returning where the cursor ends up.
    pub fn insert(&mut self, at: Position, text: &str) -> Result<Position, BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        let at = self.clamp(at);
        let end = self.insert_raw(at, text);
        self.record(
            Edit::Delete {
                range: Range::new(at, end),
                text: text.to_string(),
            },
            at,
        );
        Ok(end)
    }

    fn insert_raw(&mut self, at: Position, text: &str) -> Position {
        self.touch(at.line);
        let line = self.lines[at.line].clone();
        let before: String = line.chars().take(at.col).collect();
        let after: String = line.chars().skip(at.col).collect();

        let pieces: Vec<&str> = text.split('\n').collect();
        if pieces.len() == 1 {
            self.lines[at.line] = format!("{before}{text}{after}");
            return Position::new(at.line, at.col + text.chars().count());
        }

        self.lines[at.line] = format!("{before}{}", pieces[0]);
        let mut insert_at = at.line + 1;
        for piece in &pieces[1..pieces.len() - 1] {
            self.lines.insert(insert_at, (*piece).to_string());
            insert_at += 1;
        }
        let last = pieces[pieces.len() - 1];
        self.lines.insert(insert_at, format!("{last}{after}"));
        Position::new(insert_at, last.chars().count())
    }

    /// Delete `range`, returning the text that was there.
    pub fn delete(&mut self, range: Range) -> Result<String, BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        let range = Range::new(self.clamp(range.start), self.clamp(range.end));
        if range.is_empty() {
            return Ok(String::new());
        }
        let text = self.text_in(range);
        self.delete_raw(range);
        self.record(
            Edit::Insert {
                at: range.start,
                text: text.clone(),
            },
            range.end,
        );
        Ok(text)
    }

    fn delete_raw(&mut self, range: Range) {
        self.touch(range.start.line);
        if range.start.line == range.end.line {
            let line = &self.lines[range.start.line];
            let kept: String = line
                .chars()
                .take(range.start.col)
                .chain(line.chars().skip(range.end.col))
                .collect();
            self.lines[range.start.line] = kept;
            return;
        }
        let head: String = self.lines[range.start.line]
            .chars()
            .take(range.start.col)
            .collect();
        let tail: String = self.lines[range.end.line]
            .chars()
            .skip(range.end.col)
            .collect();
        self.lines[range.start.line] = format!("{head}{tail}");
        self.lines.drain(range.start.line + 1..=range.end.line);
    }

    /// Undo one group. Returns where the cursor should go.
    pub fn undo(&mut self) -> Option<Position> {
        self.end_group(Position::default());
        let group = self.undo.pop()?;
        for edit in group.edits.iter().rev() {
            match edit {
                Edit::Insert { at, text } => {
                    self.insert_raw(*at, text);
                }
                Edit::Delete { range, .. } => self.delete_raw(*range),
            }
        }
        let cursor = group.cursor_before;
        self.redo.push(group);
        self.modified = true;
        Some(cursor)
    }

    /// Redo one group.
    pub fn redo(&mut self) -> Option<Position> {
        let group = self.redo.pop()?;
        for edit in &group.edits {
            match edit {
                // The inverse of the inverse: a recorded "insert this back" is undone by
                // deleting it again.
                Edit::Insert { at, text } => {
                    let end = end_of(*at, text);
                    self.delete_raw(Range::new(*at, end));
                }
                Edit::Delete { range, text } => {
                    self.insert_raw(range.start, text);
                }
            }
        }
        let cursor = group.cursor_after;
        self.undo.push(group);
        self.modified = true;
        Some(cursor)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
            || self
                .open_group
                .as_ref()
                .is_some_and(|g| !g.edits.is_empty())
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    // ── line operations ────────────────────────────────────────────────────────────────────

    pub fn delete_line(&mut self, index: usize) -> Result<(), BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        if index >= self.lines.len() {
            return Ok(());
        }
        let range = if index + 1 < self.lines.len() {
            Range::new(Position::new(index, 0), Position::new(index + 1, 0))
        } else {
            Range::new(
                Position::new(index, 0),
                Position::new(index, self.line_chars(index)),
            )
        };
        self.delete(range)?;
        Ok(())
    }

    pub fn duplicate_line(&mut self, index: usize) -> Result<(), BufferError> {
        let text = format!("{}\n", self.line(index));
        self.insert(Position::new(index, 0), &text)?;
        Ok(())
    }

    /// Swap a line with the one above or below. Returns the line's new index.
    pub fn move_line(&mut self, index: usize, down: bool) -> Result<usize, BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        let other = if down {
            if index + 1 >= self.lines.len() {
                return Ok(index);
            }
            index + 1
        } else {
            if index == 0 {
                return Ok(index);
            }
            index - 1
        };
        self.begin_group(Position::new(index, 0));
        let text = self.line(index).to_string();
        self.delete_line(index)?;
        let target = other.min(self.lines.len());
        self.insert(Position::new(target, 0), &format!("{text}\n"))?;
        self.end_group(Position::new(target, 0));
        Ok(target)
    }

    #[allow(clippy::needless_range_loop)]
    pub fn indent(
        &mut self,
        lines: std::ops::Range<usize>,
        width: usize,
    ) -> Result<(), BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        let pad = " ".repeat(width);
        self.begin_group(Position::new(lines.start, 0));
        for line in lines {
            if line >= self.lines.len() || self.line(line).is_empty() {
                continue;
            }
            self.insert(Position::new(line, 0), &pad)?;
        }
        self.end_group(Position::new(0, 0));
        Ok(())
    }

    pub fn outdent(
        &mut self,
        lines: std::ops::Range<usize>,
        width: usize,
    ) -> Result<(), BufferError> {
        if self.read_only {
            return Err(BufferError::ReadOnly);
        }
        self.begin_group(Position::new(lines.start, 0));
        for line in lines {
            if line >= self.lines.len() {
                continue;
            }
            let spaces = self
                .line(line)
                .chars()
                .take(width)
                .take_while(|c| *c == ' ' || *c == '\t')
                .count();
            if spaces > 0 {
                self.delete(Range::new(
                    Position::new(line, 0),
                    Position::new(line, spaces),
                ))?;
            }
        }
        self.end_group(Position::new(0, 0));
        Ok(())
    }

    /// Replace the content of a line, as a single edit.
    pub fn set_line(&mut self, index: usize, text: &str) -> Result<(), BufferError> {
        let end = Position::new(index, self.line_chars(index));
        self.begin_group(Position::new(index, 0));
        self.delete(Range::new(Position::new(index, 0), end))?;
        self.insert(Position::new(index, 0), text)?;
        self.end_group(Position::new(index, text.chars().count()));
        Ok(())
    }
}

/// Split text into lines, treating `\r\n` as **one** terminator — splitting on either
/// character separately turns every CRLF file into twice as many lines, which is exactly the
/// bug the round-trip test caught.
fn split_lines(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut current));
            }
            '\n' => lines.push(std::mem::take(&mut current)),
            other => current.push(other),
        }
    }
    if !current.is_empty() || lines.is_empty() {
        lines.push(current);
    }
    lines
}

/// Where inserting `text` at `at` ends.
fn end_of(at: Position, text: &str) -> Position {
    let pieces: Vec<&str> = text.split('\n').collect();
    if pieces.len() == 1 {
        Position::new(at.line, at.col + text.chars().count())
    } else {
        Position::new(
            at.line + pieces.len() - 1,
            pieces[pieces.len() - 1].chars().count(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_round_trips_with_its_line_endings() {
        for (text, eol) in [
            ("a\nb\n", Eol::Lf),
            ("a\r\nb\r\n", Eol::Crlf),
            ("a\rb\r", Eol::Cr),
        ] {
            let buffer = Buffer::from_text(text);
            assert_eq!(buffer.eol, eol, "{text:?}");
            assert_eq!(buffer.len(), 2, "{text:?}");
            assert_eq!(buffer.to_text(), text, "{text:?}");
        }
    }

    #[test]
    fn a_file_without_a_final_newline_keeps_it_that_way() {
        let buffer = Buffer::from_text("one\ntwo");
        assert!(!buffer.final_newline);
        assert_eq!(buffer.to_text(), "one\ntwo");
        let buffer = Buffer::from_text("one\ntwo\n");
        assert!(buffer.final_newline);
        assert_eq!(buffer.to_text(), "one\ntwo\n");
    }

    #[test]
    fn a_file_with_no_terminator_uses_the_platform_default() {
        let expected = if cfg!(windows) { Eol::Crlf } else { Eol::Lf };
        assert_eq!(Buffer::from_text("").eol, expected);
        assert_eq!(Buffer::from_text("one line").eol, expected);
    }

    #[test]
    fn mixed_line_endings_are_noticed_not_silently_unified() {
        let buffer = Buffer::from_text("a\r\nb\nc\n");
        assert!(buffer.mixed_eol);
        assert_eq!(buffer.eol, Eol::Lf, "the majority wins");
    }

    #[test]
    fn inserting_and_deleting_inside_a_line() {
        let mut buffer = Buffer::from_text("hello world\n");
        let end = buffer.insert(Position::new(0, 5), ",").unwrap();
        assert_eq!(end, Position::new(0, 6));
        assert_eq!(buffer.line(0), "hello, world");
        assert!(buffer.modified());

        let removed = buffer
            .delete(Range::new(Position::new(0, 5), Position::new(0, 7)))
            .unwrap();
        assert_eq!(removed, ", ");
        assert_eq!(buffer.line(0), "helloworld");
    }

    #[test]
    fn inserting_a_newline_splits_the_line() {
        let mut buffer = Buffer::from_text("hello world\n");
        let end = buffer.insert(Position::new(0, 5), "\n").unwrap();
        assert_eq!(end, Position::new(1, 0));
        assert_eq!(buffer.lines(), &["hello".to_string(), " world".to_string()]);
        assert_eq!(buffer.to_text(), "hello\n world\n");
    }

    #[test]
    fn inserting_several_lines_at_once() {
        let mut buffer = Buffer::from_text("ad\n");
        let end = buffer.insert(Position::new(0, 1), "b\nc").unwrap();
        assert_eq!(end, Position::new(1, 1));
        assert_eq!(buffer.to_text(), "ab\ncd\n");
    }

    #[test]
    fn deleting_across_lines_joins_them() {
        let mut buffer = Buffer::from_text("one\ntwo\nthree\n");
        let removed = buffer
            .delete(Range::new(Position::new(0, 1), Position::new(2, 2)))
            .unwrap();
        assert_eq!(removed, "ne\ntwo\nth");
        assert_eq!(buffer.to_text(), "oree\n");
    }

    #[test]
    fn text_in_a_range_reads_across_lines() {
        let buffer = Buffer::from_text("one\ntwo\nthree\n");
        assert_eq!(
            buffer.text_in(Range::new(Position::new(0, 1), Position::new(1, 2))),
            "ne\ntw"
        );
        assert_eq!(
            buffer.text_in(Range::new(Position::new(1, 0), Position::new(1, 3))),
            "two"
        );
        // A reversed range is the same range.
        assert_eq!(
            buffer.text_in(Range::new(Position::new(1, 3), Position::new(1, 0))),
            "two"
        );
    }

    #[test]
    fn undo_and_redo_walk_the_history_and_the_cursor() {
        let mut buffer = Buffer::from_text("hello\n");
        buffer.insert(Position::new(0, 5), " world").unwrap();
        buffer.end_group(Position::new(0, 11));
        assert_eq!(buffer.line(0), "hello world");

        let cursor = buffer.undo().expect("something to undo");
        assert_eq!(buffer.line(0), "hello");
        assert_eq!(cursor, Position::new(0, 5));

        let cursor = buffer.redo().expect("something to redo");
        assert_eq!(buffer.line(0), "hello world");
        assert_eq!(cursor, Position::new(0, 11));
        assert!(buffer.undo().is_some());
        assert!(buffer.undo().is_none(), "the history is exhausted");
    }

    #[test]
    fn a_typing_burst_is_one_undo_step() {
        let mut buffer = Buffer::from_text("");
        buffer.begin_group(Position::new(0, 0));
        for (i, c) in "hello".chars().enumerate() {
            buffer.insert(Position::new(0, i), &c.to_string()).unwrap();
        }
        buffer.end_group(Position::new(0, 5));
        assert_eq!(buffer.line(0), "hello");
        buffer.undo();
        assert_eq!(buffer.line(0), "", "the whole burst should go at once");
    }

    #[test]
    fn undo_restores_a_multi_line_delete() {
        let mut buffer = Buffer::from_text("one\ntwo\nthree\n");
        buffer
            .delete(Range::new(Position::new(0, 0), Position::new(2, 0)))
            .unwrap();
        buffer.end_group(Position::new(0, 0));
        assert_eq!(buffer.to_text(), "three\n");
        buffer.undo();
        assert_eq!(buffer.to_text(), "one\ntwo\nthree\n");
    }

    #[test]
    fn line_operations() {
        let mut buffer = Buffer::from_text("one\ntwo\nthree\n");
        buffer.duplicate_line(1).unwrap();
        assert_eq!(buffer.to_text(), "one\ntwo\ntwo\nthree\n");
        buffer.delete_line(1).unwrap();
        assert_eq!(buffer.to_text(), "one\ntwo\nthree\n");
        let moved = buffer.move_line(0, true).unwrap();
        assert_eq!(moved, 1);
        assert_eq!(buffer.to_text(), "two\none\nthree\n");
        assert_eq!(buffer.move_line(0, false).unwrap(), 0, "the top line stays");
    }

    #[test]
    fn deleting_the_last_line_leaves_a_buffer_not_a_hole() {
        let mut buffer = Buffer::from_text("only\n");
        buffer.delete_line(0).unwrap();
        assert_eq!(buffer.len(), 1);
        assert!(buffer.is_empty());
    }

    #[test]
    fn indent_and_outdent_skip_empty_lines() {
        let mut buffer = Buffer::from_text("a\n\nb\n");
        buffer.indent(0..3, 2).unwrap();
        assert_eq!(buffer.to_text(), "  a\n\n  b\n");
        buffer.outdent(0..3, 2).unwrap();
        assert_eq!(buffer.to_text(), "a\n\nb\n");
        // Outdenting unindented text changes nothing.
        buffer.outdent(0..3, 2).unwrap();
        assert_eq!(buffer.to_text(), "a\n\nb\n");
    }

    #[test]
    fn a_read_only_buffer_refuses_every_edit() {
        let mut buffer = Buffer::from_text("locked\n");
        buffer.read_only = true;
        assert!(matches!(
            buffer.insert(Position::new(0, 0), "x"),
            Err(BufferError::ReadOnly)
        ));
        assert!(matches!(
            buffer.delete(Range::new(Position::new(0, 0), Position::new(0, 1))),
            Err(BufferError::ReadOnly)
        ));
        assert!(matches!(buffer.delete_line(0), Err(BufferError::ReadOnly)));
        assert_eq!(buffer.to_text(), "locked\n");
        assert!(!buffer.modified());
    }

    #[test]
    fn positions_are_clamped_onto_real_text() {
        let buffer = Buffer::from_text("ab\ncd\n");
        assert_eq!(buffer.clamp(Position::new(9, 9)), Position::new(1, 2));
        assert_eq!(buffer.clamp(Position::new(0, 5)), Position::new(0, 2));
        assert_eq!(buffer.end(), Position::new(1, 2));
    }

    #[test]
    fn unicode_columns_count_characters_not_bytes() {
        let mut buffer = Buffer::from_text("héllo wörld\n");
        assert_eq!(buffer.line_chars(0), 11);
        buffer.insert(Position::new(0, 1), "ü").unwrap();
        assert_eq!(buffer.line(0), "hüéllo wörld");
        let removed = buffer
            .delete(Range::new(Position::new(0, 1), Position::new(0, 3)))
            .unwrap();
        assert_eq!(removed, "üé");
    }
}
