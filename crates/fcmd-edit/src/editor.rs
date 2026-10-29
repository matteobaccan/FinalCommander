use std::path::{Path, PathBuf};

use fcmd_core::VPath;
use fcmd_view::SearchMode;
use regex::Regex;

use crate::buffer::{Buffer, BufferError, Position, Range};

/// Stream selection takes whole lines between its ends; column selection takes the same column
/// range out of every line it spans — FAR's vertical block, which is the reason people keep
/// using these editors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
    Stream,
    Column,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Block {
    pub anchor: Position,
    pub head: Position,
    pub kind: BlockKind,
}

impl Block {
    pub fn range(&self) -> Range {
        Range::new(self.anchor, self.head)
    }

    /// `(first line, last line, first column, last column)` for a column block.
    pub fn columns(&self) -> (usize, usize, usize, usize) {
        let (top, bottom) = if self.anchor.line <= self.head.line {
            (self.anchor.line, self.head.line)
        } else {
            (self.head.line, self.anchor.line)
        };
        let (left, right) = if self.anchor.col <= self.head.col {
            (self.anchor.col, self.head.col)
        } else {
            (self.head.col, self.anchor.col)
        };
        (top, bottom, left, right)
    }
}

/// What a save did, so the UI can say it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved { path: VPath, bytes: usize },
    NothingToDo,
}

#[derive(Debug, thiserror::Error)]
pub enum EditorError {
    #[error(transparent)]
    Buffer(#[from] BufferError),
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no file name: use save as")]
    NoPath,
    #[error("{0} changed on disk since it was opened")]
    ChangedOnDisk(String),
    #[error("invalid search: {0}")]
    Search(String),
}

/// The editor: a buffer, a cursor, a block, and the state the UI needs to draw and drive it.
pub struct Editor {
    pub buffer: Buffer,
    pub path: Option<VPath>,
    pub cursor: Position,
    pub block: Option<Block>,
    /// First visible line and first visible column.
    pub top: usize,
    pub left: usize,
    pub tab_width: usize,
    pub expand_tabs: bool,
    pub auto_indent: bool,
    /// Draw a space as `·` and a tab as `→`. The editor does not draw itself, so this is only
    /// kept here: it belongs to the file being edited, not to the window it is in.
    pub show_whitespace: bool,
    /// Where the cursor lands on a match: at its start, or just past its end.
    pub cursor_after_match: bool,
    /// May the cursor stand past the end of a line? Off, it is clamped to the last character,
    /// which is what a text editor does; on, the columns past the end are somewhere to be,
    /// and the line is padded with spaces only when something is actually typed there.
    pub beyond_eol: bool,
    /// The next key typed goes in as itself, whatever it means — `Ctrl+Q` armed it. It is how
    /// a control character, or a tab where tabs are expanded, gets into a file at all.
    pub literal_next: bool,
    pub bookmarks: [Option<Position>; 10],
    /// The editor's own clipboard: the OS one arrives with `fcmd-platform`.
    pub clipboard: Option<(String, BlockKind)>,
    /// Modification time when the file was read, to notice an external change.
    mtime: Option<std::time::SystemTime>,
    pub last_search: Option<String>,
    pub last_replace: Option<String>,
    pub search_mode: SearchMode,
    pub search_case_insensitive: bool,
    /// Only whole words count as a hit (FAR asks it in the dialog).
    pub search_whole_words: bool,
    /// What the file is being read as, and how much of it has been scanned so far.
    pub highlight: crate::Highlight,
}

impl std::fmt::Debug for Editor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Editor")
            .field("path", &self.path)
            .field("cursor", &self.cursor)
            .field("lines", &self.buffer.len())
            .field("modified", &self.buffer.modified())
            .finish_non_exhaustive()
    }
}

impl Editor {
    pub fn new(buffer: Buffer, path: Option<VPath>) -> Editor {
        let mtime = path
            .as_ref()
            .and_then(|p| std::fs::metadata(p.as_path()).ok())
            .and_then(|m| m.modified().ok());
        // The name is what says which language this is: a file opened from an archive or
        // fetched from a server is read the same way as one on disk.
        let highlight = match &path {
            Some(path) => crate::Highlight::for_name(&path.display_name()),
            None => crate::Highlight::default(),
        };
        Editor {
            buffer,
            path,
            cursor: Position::default(),
            block: None,
            top: 0,
            left: 0,
            tab_width: 4,
            expand_tabs: true,
            auto_indent: true,
            show_whitespace: false,
            cursor_after_match: false,
            beyond_eol: false,
            literal_next: false,
            bookmarks: [None; 10],
            clipboard: None,
            mtime,
            last_search: None,
            last_replace: None,
            search_mode: SearchMode::Plain,
            search_case_insensitive: true,
            search_whole_words: false,
            highlight,
        }
    }

    /// The coloured runs of one line, with the scan kept in step with the edits.
    ///
    /// It is a method of the editor rather than of the buffer because the two have to be
    /// borrowed together: the cache is extended while the buffer is read.
    pub fn spans(&mut self, line: usize) -> Vec<fcmd_syntax::Span> {
        let Editor {
            buffer, highlight, ..
        } = self;
        if let Some(from) = buffer.take_touched() {
            highlight.forget_from(from);
        }
        highlight.spans(buffer, line)
    }

    pub fn open(path: &Path) -> Result<Editor, EditorError> {
        let buffer = Buffer::open(path)?;
        Ok(Editor::new(buffer, Some(VPath::local(path))))
    }

    /// An empty buffer for `Shift+F4`. A file we create ends with a newline, as text files do.
    pub fn empty(path: Option<VPath>) -> Editor {
        let mut buffer = Buffer::from_text("");
        buffer.final_newline = true;
        Editor::new(buffer, path)
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.display_name())
            .unwrap_or_else(|| "(new file)".to_string())
    }

    // ── movement ───────────────────────────────────────────────────────────────────────────

    /// Move the cursor; `extend` keeps (or starts) the block.
    pub fn move_to(&mut self, position: Position, extend: bool, kind: BlockKind) {
        let position = self.clamp(position);
        if extend {
            match &mut self.block {
                Some(block) if block.kind == kind => block.head = position,
                _ => {
                    self.block = Some(Block {
                        anchor: self.cursor,
                        head: position,
                        kind,
                    })
                }
            }
        } else {
            self.block = None;
        }
        self.cursor = position;
        self.buffer.end_group(self.cursor);
    }

    pub fn move_by(&mut self, lines: isize, cols: isize, extend: bool, kind: BlockKind) {
        let mut line = (self.cursor.line as isize + lines).max(0) as usize;
        line = line.min(self.buffer.len() - 1);
        let mut col = self.cursor.col as isize + cols;

        // Moving left at the start of a line goes to the end of the previous one — unless the
        // cursor is allowed past the end of a line, where the columns to the right of the text
        // are somewhere to be and `Right` goes into them rather than to the next line. That is
        // the whole of what the setting changes about the arrows.
        if cols != 0 && !self.beyond_eol {
            while col < 0 && line > 0 {
                line -= 1;
                col += self.buffer.line_chars(line) as isize + 1;
            }
            let len = self.buffer.line_chars(line) as isize;
            while col > len && line + 1 < self.buffer.len() {
                col -= len + 1;
                line += 1;
            }
        }
        self.move_to(Position::new(line, col.max(0) as usize), extend, kind);
    }

    /// Where a position actually is, given whether the cursor may stand past the text.
    ///
    /// The line is always clamped — there is no line beyond the last one — and the column is
    /// clamped only when the cursor belongs inside the text. Everything below the editor
    /// clamps as well, so a column past the end of a line is safe to carry around: it reads
    /// as the end of the line wherever a character is actually wanted.
    fn clamp(&self, position: Position) -> Position {
        if self.beyond_eol {
            let line = position.line.min(self.buffer.len() - 1);
            Position::new(line, position.col)
        } else {
            self.buffer.clamp(position)
        }
    }

    /// How far past the end of its line the cursor is, in characters. Zero whenever it is on
    /// the text, which is always unless the setting is on.
    fn virtual_cols(&self) -> usize {
        self.cursor
            .col
            .saturating_sub(self.buffer.line_chars(self.cursor.line))
    }

    /// Put real spaces where the cursor is standing on nothing, so that what is typed there
    /// lands in the column it was typed in.
    ///
    /// This is the one moment the setting changes the *file*, and it is the moment the user
    /// asked for it: moving about past the end of a line writes nothing, typing there does.
    fn fill_to_cursor(&mut self) -> Result<(), EditorError> {
        let pad = self.virtual_cols();
        if pad == 0 {
            return Ok(());
        }
        let end = Position::new(self.cursor.line, self.buffer.line_chars(self.cursor.line));
        self.buffer.insert(end, &" ".repeat(pad))?;
        self.cursor = Position::new(self.cursor.line, end.col + pad);
        Ok(())
    }

    pub fn move_line_start(&mut self, extend: bool) {
        // Smart home: to the first non-blank, then to column zero.
        let line = self.buffer.line(self.cursor.line);
        let indent = line.chars().take_while(|c| c.is_whitespace()).count();
        let col = if self.cursor.col == indent { 0 } else { indent };
        self.move_to(
            Position::new(self.cursor.line, col),
            extend,
            BlockKind::Stream,
        );
    }

    pub fn move_line_end(&mut self, extend: bool) {
        let col = self.buffer.line_chars(self.cursor.line);
        self.move_to(
            Position::new(self.cursor.line, col),
            extend,
            BlockKind::Stream,
        );
    }

    pub fn move_file_start(&mut self, extend: bool) {
        self.move_to(Position::default(), extend, BlockKind::Stream);
    }

    pub fn move_file_end(&mut self, extend: bool) {
        self.move_to(self.buffer.end(), extend, BlockKind::Stream);
    }

    pub fn page(&mut self, down: bool, height: usize, extend: bool) {
        let delta = height.max(1) as isize;
        self.move_by(
            if down { delta } else { -delta },
            0,
            extend,
            BlockKind::Stream,
        );
    }

    /// Keep the cursor on screen. Returns the visible line range.
    pub fn scroll_into_view(&mut self, height: usize, width: usize) -> std::ops::Range<usize> {
        let height = height.max(1);
        if self.cursor.line < self.top {
            self.top = self.cursor.line;
        } else if self.cursor.line >= self.top + height {
            self.top = self.cursor.line + 1 - height;
        }
        let display_col = self.display_col(self.cursor);
        if display_col < self.left {
            self.left = display_col;
        } else if width > 0 && display_col >= self.left + width {
            self.left = display_col + 1 - width;
        }
        self.top..(self.top + height).min(self.buffer.len())
    }

    /// Character column → display column, expanding tabs.
    pub fn display_col(&self, position: Position) -> usize {
        let line = self.buffer.line(position.line);
        let mut column = 0usize;
        for (i, ch) in line.chars().enumerate() {
            if i >= position.col {
                break;
            }
            if ch == '\t' {
                column += self.tab_width - (column % self.tab_width);
            } else {
                column += unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
            }
        }
        // A cursor past the end of the line stands in columns that hold nothing, and nothing
        // is one column wide: there is no character there to be two.
        column + position.col.saturating_sub(line.chars().count())
    }

    // ── editing ────────────────────────────────────────────────────────────────────────────

    pub fn insert_char(&mut self, ch: char) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        self.buffer.begin_group(self.cursor);
        self.fill_to_cursor()?;
        if ch == '\t' && self.expand_tabs {
            let pad = self.tab_width - (self.display_col(self.cursor) % self.tab_width);
            self.cursor = self.buffer.insert(self.cursor, &" ".repeat(pad))?;
        } else {
            self.cursor = self.buffer.insert(self.cursor, &ch.to_string())?;
        }
        Ok(())
    }

    /// Insert a character exactly as it is, with none of the rules `insert_char` applies: a
    /// tab stays a tab where tabs are expanded, and a control character goes in as itself.
    /// This is what `Ctrl+Q` is for.
    pub fn insert_literal(&mut self, ch: char) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        self.buffer.begin_group(self.cursor);
        self.fill_to_cursor()?;
        self.cursor = self.buffer.insert(self.cursor, &ch.to_string())?;
        Ok(())
    }

    pub fn insert_text(&mut self, text: &str) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        self.buffer.begin_group(self.cursor);
        self.fill_to_cursor()?;
        self.cursor = self.buffer.insert(self.cursor, text)?;
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    /// Enter: split the line, carrying the indentation when auto-indent is on. Past the end
    /// of a line there is nothing to split, so nothing is padded: it is a new empty line.
    pub fn insert_newline(&mut self) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        let indent: String = if self.auto_indent {
            self.buffer
                .line(self.cursor.line)
                .chars()
                .take_while(|c| *c == ' ' || *c == '\t')
                .collect()
        } else {
            String::new()
        };
        self.buffer.begin_group(self.cursor);
        self.cursor = self.buffer.insert(self.cursor, &format!("\n{indent}"))?;
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    pub fn backspace(&mut self) -> Result<(), EditorError> {
        if self.delete_block_if_any()? {
            return Ok(());
        }
        if self.cursor == Position::default() {
            return Ok(());
        }
        // Past the end of a line there is nothing to take out: the cursor moves back and the
        // file is left alone, which is the only reading of `Backspace` over nothing.
        if self.virtual_cols() > 0 {
            self.cursor = Position::new(self.cursor.line, self.cursor.col - 1);
            return Ok(());
        }
        let end = self.cursor;
        let start = if self.cursor.col > 0 {
            Position::new(self.cursor.line, self.cursor.col - 1)
        } else {
            let previous = self.cursor.line - 1;
            Position::new(previous, self.buffer.line_chars(previous))
        };
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(start, end))?;
        self.cursor = start;
        Ok(())
    }

    pub fn delete_forward(&mut self) -> Result<(), EditorError> {
        if self.delete_block_if_any()? {
            return Ok(());
        }
        // The same the other way: `Del` in the empty columns past a line deletes nothing. It
        // does *not* pull the next line up — the cursor is not at the end of this one, and a
        // key that joined two lines from three columns away would be a surprise.
        if self.virtual_cols() > 0 {
            return Ok(());
        }
        let start = self.cursor;
        let end = if self.cursor.col < self.buffer.line_chars(self.cursor.line) {
            Position::new(self.cursor.line, self.cursor.col + 1)
        } else if self.cursor.line + 1 < self.buffer.len() {
            Position::new(self.cursor.line + 1, 0)
        } else {
            return Ok(());
        };
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(start, end))?;
        Ok(())
    }

    pub fn delete_to_line_end(&mut self) -> Result<(), EditorError> {
        let end = Position::new(self.cursor.line, self.buffer.line_chars(self.cursor.line));
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(self.cursor, end))?;
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    pub fn delete_word_left(&mut self) -> Result<(), EditorError> {
        let line = self.buffer.line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;
        while col > 0 && chars[col - 1].is_whitespace() {
            col -= 1;
        }
        while col > 0 && !chars[col - 1].is_whitespace() {
            col -= 1;
        }
        let start = Position::new(self.cursor.line, col);
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(start, self.cursor))?;
        self.cursor = start;
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    /// The other half of the pair, and FAR's `Ctrl+T`: the word after the cursor goes, with
    /// the run of spaces that follows it, so holding the key walks a line apart word by word.
    pub fn delete_word_right(&mut self) -> Result<(), EditorError> {
        let line = self.buffer.line(self.cursor.line);
        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.col;
        while col < chars.len() && !chars[col].is_whitespace() {
            col += 1;
        }
        while col < chars.len() && chars[col].is_whitespace() {
            col += 1;
        }
        let end = Position::new(self.cursor.line, col);
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(self.cursor, end))?;
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    pub fn delete_line(&mut self) -> Result<(), EditorError> {
        self.buffer.delete_line(self.cursor.line)?;
        self.buffer.end_group(self.cursor);
        self.cursor = self.clamp(self.cursor);
        self.block = None;
        Ok(())
    }

    pub fn duplicate_line(&mut self) -> Result<(), EditorError> {
        self.buffer.duplicate_line(self.cursor.line)?;
        self.buffer.end_group(self.cursor);
        self.cursor = Position::new(self.cursor.line + 1, self.cursor.col);
        Ok(())
    }

    pub fn move_line(&mut self, down: bool) -> Result<(), EditorError> {
        let line = self.buffer.move_line(self.cursor.line, down)?;
        self.cursor = Position::new(line, self.cursor.col);
        self.cursor = self.clamp(self.cursor);
        Ok(())
    }

    /// Tab on a block indents it; otherwise it inserts.
    pub fn indent(&mut self) -> Result<(), EditorError> {
        match self.block {
            Some(block) => {
                let (top, bottom, _, _) = block.columns();
                self.buffer.indent(top..bottom + 1, self.tab_width)?;
                Ok(())
            }
            None => self.insert_char('\t'),
        }
    }

    /// `Alt+I`: move the block, or the line the cursor is on, one indent to the right.
    ///
    /// Not the same as `Tab`, which types a tab where there is no block — that is the key one
    /// presses while writing, and this is the one for moving what is already written.
    pub fn shift_right(&mut self) -> Result<(), EditorError> {
        let (top, bottom) = match self.block {
            Some(block) => {
                let (top, bottom, _, _) = block.columns();
                (top, bottom)
            }
            None => (self.cursor.line, self.cursor.line),
        };
        self.buffer.indent(top..bottom + 1, self.tab_width)?;
        self.cursor = self.clamp(self.cursor);
        Ok(())
    }

    pub fn outdent(&mut self) -> Result<(), EditorError> {
        let (top, bottom) = match self.block {
            Some(block) => {
                let (top, bottom, _, _) = block.columns();
                (top, bottom)
            }
            None => (self.cursor.line, self.cursor.line),
        };
        self.buffer.outdent(top..bottom + 1, self.tab_width)?;
        self.cursor = self.clamp(self.cursor);
        Ok(())
    }

    pub fn undo(&mut self) -> bool {
        match self.buffer.undo() {
            Some(cursor) => {
                self.cursor = self.buffer.clamp(cursor);
                self.block = None;
                true
            }
            None => false,
        }
    }

    pub fn redo(&mut self) -> bool {
        match self.buffer.redo() {
            Some(cursor) => {
                self.cursor = self.buffer.clamp(cursor);
                self.block = None;
                true
            }
            None => false,
        }
    }

    // ── blocks ─────────────────────────────────────────────────────────────────────────────

    pub fn select_all(&mut self) {
        self.block = Some(Block {
            anchor: Position::default(),
            head: self.buffer.end(),
            kind: BlockKind::Stream,
        });
        self.cursor = self.buffer.end();
    }

    pub fn clear_block(&mut self) {
        self.block = None;
    }

    /// The selected text. A column block joins its slices with newlines, so pasting it puts
    /// the column back.
    pub fn block_text(&self) -> Option<String> {
        let block = self.block?;
        match block.kind {
            BlockKind::Stream => Some(self.buffer.text_in(block.range())),
            BlockKind::Column => {
                let (top, bottom, left, right) = block.columns();
                let mut out = Vec::new();
                for line in top..=bottom.min(self.buffer.len() - 1) {
                    let text: String = self
                        .buffer
                        .line(line)
                        .chars()
                        .skip(left)
                        .take(right.saturating_sub(left))
                        .collect();
                    out.push(text);
                }
                Some(out.join("\n"))
            }
        }
    }

    pub fn copy_block(&mut self) -> bool {
        match (self.block_text(), self.block) {
            (Some(text), Some(block)) => {
                self.clipboard = Some((text, block.kind));
                true
            }
            _ => false,
        }
    }

    /// Add the block to what is already on the clipboard rather than replacing it, which is
    /// how a few lines are gathered from here and there before being pasted in one go.
    ///
    /// The kind of the clipboard is kept: appending a column block to a stream one would make
    /// a paste that is neither, so the text joins as lines either way and the first block's
    /// kind decides how it goes back in.
    pub fn append_block(&mut self) -> bool {
        let Some(text) = self.block_text() else {
            return false;
        };
        match &mut self.clipboard {
            Some((held, _)) => {
                if !held.ends_with('\n') {
                    held.push('\n');
                }
                held.push_str(&text);
            }
            None => {
                let kind = self.block.map(|b| b.kind).unwrap_or(BlockKind::Stream);
                self.clipboard = Some((text, kind));
            }
        }
        true
    }

    /// Turn every tab already in the file into spaces, as the `all` tab policy asks.
    ///
    /// It is done on open and it changes the file on the first save, which is why that policy
    /// is not the default: expanding what somebody else wrote is a diff nobody asked for.
    pub fn expand_all_tabs(&mut self) -> Result<usize, EditorError> {
        let width = self.tab_width.max(1);
        let mut touched = 0;
        for line in 0..self.buffer.len() {
            let text = self.buffer.line(line).to_string();
            if !text.contains('\t') {
                continue;
            }
            let mut out = String::with_capacity(text.len() + width);
            for c in text.chars() {
                if c == '\t' {
                    let to = width - (out.chars().count() % width);
                    out.push_str(&" ".repeat(to));
                } else {
                    out.push(c);
                }
            }
            self.buffer.set_line(line, &out)?;
            touched += 1;
        }
        if touched > 0 {
            self.cursor = self.buffer.clamp(self.cursor);
        }
        Ok(touched)
    }

    pub fn cut_block(&mut self) -> Result<bool, EditorError> {
        if !self.copy_block() {
            return Ok(false);
        }
        self.delete_block_if_any()?;
        Ok(true)
    }

    /// Paste: a column block is pushed into the lines below the cursor, column-wise.
    pub fn paste(&mut self) -> Result<bool, EditorError> {
        let Some((text, kind)) = self.clipboard.clone() else {
            return Ok(false);
        };
        self.delete_block_if_any()?;
        match kind {
            BlockKind::Stream => {
                self.insert_text(&text)?;
            }
            BlockKind::Column => {
                let col = self.cursor.col;
                self.buffer.begin_group(self.cursor);
                for (i, piece) in text.split('\n').enumerate() {
                    let line = self.cursor.line + i;
                    while line >= self.buffer.len() {
                        let end = self.buffer.end();
                        self.buffer.insert(end, "\n")?;
                    }
                    let pad = col.saturating_sub(self.buffer.line_chars(line));
                    let at = Position::new(line, self.buffer.line_chars(line).min(col));
                    let text = format!("{}{piece}", " ".repeat(pad));
                    self.buffer.insert(at, &text)?;
                }
                self.buffer.end_group(self.cursor);
            }
        }
        Ok(true)
    }

    /// Delete the block if there is one. Returns whether anything was deleted.
    fn delete_block_if_any(&mut self) -> Result<bool, EditorError> {
        let Some(block) = self.block.take() else {
            return Ok(false);
        };
        match block.kind {
            BlockKind::Stream => {
                let range = block.range();
                self.buffer.begin_group(self.cursor);
                self.buffer.delete(range)?;
                self.cursor = range.start;
            }
            BlockKind::Column => {
                let (top, bottom, left, right) = block.columns();
                self.buffer.begin_group(self.cursor);
                for line in top..=bottom.min(self.buffer.len() - 1) {
                    let len = self.buffer.line_chars(line);
                    if left >= len {
                        continue;
                    }
                    self.buffer.delete(Range::new(
                        Position::new(line, left),
                        Position::new(line, right.min(len)),
                    ))?;
                }
                self.cursor = Position::new(top, left);
            }
        }
        self.buffer.end_group(self.cursor);
        Ok(true)
    }

    // ── search ─────────────────────────────────────────────────────────────────────────────

    fn compile(&self, query: &str) -> Result<Regex, EditorError> {
        let pattern = match self.search_mode {
            SearchMode::Regex => query.to_string(),
            _ => regex::escape(query),
        };
        // Whole words wraps whatever it is in boundaries, which is what makes looking for
        // `set` in a file full of `offset` bearable.
        let pattern = if self.search_whole_words {
            format!(r"\b(?:{pattern})\b")
        } else {
            pattern
        };
        regex::RegexBuilder::new(&pattern)
            .case_insensitive(self.search_case_insensitive)
            .build()
            .map_err(|e| EditorError::Search(e.to_string()))
    }

    /// Find `query` from just after the cursor (or before it, going backwards).
    pub fn find(&mut self, query: &str, backwards: bool) -> Result<bool, EditorError> {
        let re = self.compile(query)?;
        self.last_search = Some(query.to_string());
        let lines = self.buffer.len();

        // A match *at* the cursor counts — unless the block already covers it, which means a
        // previous find put us here and the user asked for the next one.
        let on_a_hit = self
            .block
            .is_some_and(|b| b.kind == BlockKind::Stream && b.anchor == self.cursor);

        for step in 0..=lines {
            let line = if backwards {
                (self.cursor.line + lines - step) % lines
            } else {
                (self.cursor.line + step) % lines
            };
            let text = self.buffer.line(line);
            let mut hits: Vec<(usize, usize)> = re
                .find_iter(text)
                .map(|m| {
                    (
                        text[..m.start()].chars().count(),
                        text[..m.end()].chars().count(),
                    )
                })
                .collect();
            if backwards {
                hits.reverse();
            }
            for (start, end) in hits {
                let is_current = line == self.cursor.line && step == 0;
                let ok = if backwards {
                    !is_current || end <= self.cursor.col
                } else if !is_current {
                    true
                } else if on_a_hit {
                    start > self.cursor.col
                } else {
                    start >= self.cursor.col
                };
                if ok {
                    self.cursor = if self.cursor_after_match {
                        Position::new(line, end)
                    } else {
                        Position::new(line, start)
                    };
                    self.block = Some(Block {
                        anchor: Position::new(line, start),
                        head: Position::new(line, end),
                        kind: BlockKind::Stream,
                    });
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Replace the current match (if the block holds one) and find the next.
    pub fn replace_once(&mut self, query: &str, with: &str) -> Result<bool, EditorError> {
        self.last_replace = Some(with.to_string());
        let found = match self.block {
            Some(block) if block.kind == BlockKind::Stream && !block.range().is_empty() => true,
            _ => self.find(query, false)?,
        };
        if !found {
            return Ok(false);
        }
        self.delete_block_if_any()?;
        self.insert_text(with)?;
        Ok(true)
    }

    /// Replace every match. Returns how many.
    pub fn replace_all(&mut self, query: &str, with: &str) -> Result<usize, EditorError> {
        let re = self.compile(query)?;
        self.last_search = Some(query.to_string());
        self.last_replace = Some(with.to_string());
        let mut count = 0usize;
        self.buffer.begin_group(self.cursor);
        for line in 0..self.buffer.len() {
            let text = self.buffer.line(line).to_string();
            let hits = re.find_iter(&text).count();
            if hits == 0 {
                continue;
            }
            let replaced = re.replace_all(&text, with).into_owned();
            self.buffer.set_line(line, &replaced)?;
            count += hits;
        }
        self.buffer.end_group(self.cursor);
        self.cursor = self.buffer.clamp(self.cursor);
        self.block = None;
        Ok(count)
    }

    // ── bookmarks, goto ────────────────────────────────────────────────────────────────────

    pub fn set_bookmark(&mut self, slot: usize) -> bool {
        match self.bookmarks.get_mut(slot) {
            Some(place) => {
                *place = Some(self.cursor);
                true
            }
            None => false,
        }
    }

    pub fn goto_bookmark(&mut self, slot: usize) -> bool {
        match self.bookmarks.get(slot).copied().flatten() {
            Some(position) => {
                self.move_to(position, false, BlockKind::Stream);
                true
            }
            None => false,
        }
    }

    /// `12`, `12:5` — one-based, as every editor and compiler prints them.
    pub fn goto(&mut self, text: &str) -> bool {
        let mut parts = text.trim().split(':');
        let Some(Ok(line)) = parts.next().map(str::parse::<usize>) else {
            return false;
        };
        let col = parts
            .next()
            .and_then(|c| c.parse::<usize>().ok())
            .unwrap_or(1);
        self.move_to(
            Position::new(line.saturating_sub(1), col.saturating_sub(1)),
            false,
            BlockKind::Stream,
        );
        true
    }

    // ── saving ─────────────────────────────────────────────────────────────────────────────

    /// Did the file change under us since it was opened?
    pub fn changed_on_disk(&self) -> bool {
        let Some(path) = &self.path else {
            return false;
        };
        let Ok(meta) = std::fs::metadata(path.as_path()) else {
            return false;
        };
        match (meta.modified().ok(), self.mtime) {
            (Some(now), Some(then)) => now != then,
            _ => false,
        }
    }

    /// Write the buffer. `force` saves over an external change.
    ///
    /// The file is written to `name.fcmd-tmp` and renamed, so an interrupted save cannot leave
    /// a half-written file where the original was — the same rule as the copy engine.
    pub fn save(&mut self, force: bool) -> Result<SaveOutcome, EditorError> {
        let Some(path) = self.path.clone() else {
            return Err(EditorError::NoPath);
        };
        if !self.buffer.modified() {
            return Ok(SaveOutcome::NothingToDo);
        }
        if !force && self.changed_on_disk() {
            return Err(EditorError::ChangedOnDisk(path.to_string()));
        }
        self.write_to(&path)?;
        self.buffer.mark_saved();
        self.mtime = std::fs::metadata(path.as_path())
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(SaveOutcome::Saved {
            bytes: self.buffer.to_bytes().len(),
            path,
        })
    }

    pub fn save_as(&mut self, path: &Path) -> Result<SaveOutcome, EditorError> {
        let target = VPath::local(path);
        self.write_to(&target)?;
        self.path = Some(target.clone());
        self.buffer.mark_saved();
        self.mtime = std::fs::metadata(path).ok().and_then(|m| m.modified().ok());
        Ok(SaveOutcome::Saved {
            bytes: self.buffer.to_bytes().len(),
            path: target,
        })
    }

    fn write_to(&self, path: &VPath) -> Result<(), EditorError> {
        let bytes = self.buffer.to_bytes();
        let temp = temp_path(path.as_path());
        std::fs::write(&temp, &bytes).map_err(|source| EditorError::Io {
            path: temp.display().to_string(),
            source,
        })?;
        // Keep the original permissions: saving a file should not change its mode.
        if let Ok(meta) = std::fs::metadata(path.as_path()) {
            let _ = std::fs::set_permissions(&temp, meta.permissions());
        }
        std::fs::rename(&temp, path.as_path()).map_err(|source| {
            let _ = std::fs::remove_file(&temp);
            EditorError::Io {
                path: path.to_string(),
                source,
            }
        })?;
        Ok(())
    }

    /// Re-read the file, throwing away unsaved changes.
    pub fn reload(&mut self) -> Result<(), EditorError> {
        let Some(path) = self.path.clone() else {
            return Err(EditorError::NoPath);
        };
        let buffer = Buffer::open(path.as_path())?;
        let cursor = self.cursor;
        self.buffer = buffer;
        self.cursor = self.buffer.clamp(cursor);
        self.block = None;
        self.mtime = std::fs::metadata(path.as_path())
            .ok()
            .and_then(|m| m.modified().ok());
        Ok(())
    }

    /// The status line: name, position, flags.
    pub fn status(&self) -> String {
        let mut text = format!(
            "{}:{}  {} lines  {}  {}",
            self.cursor.line + 1,
            self.cursor.col + 1,
            self.buffer.len(),
            self.buffer.eol.as_str(),
            self.buffer.codepage.name()
        );
        if self.buffer.mixed_eol {
            text.push_str("  mixed EOL");
        }
        if self.buffer.read_only {
            text.push_str("  read-only");
        }
        if self.buffer.modified() {
            text.push_str("  *");
        }
        if let Some(block) = &self.block {
            if block.kind == BlockKind::Column {
                text.push_str("  column");
            }
        }
        text
    }
}

fn temp_path(path: &Path) -> PathBuf {
    let mut name = path
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".fcmd-tmp");
    match path.parent() {
        Some(parent) => parent.join(name),
        None => PathBuf::from(name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Eol;

    fn editor(text: &str) -> Editor {
        Editor::new(Buffer::from_text(text), None)
    }

    #[test]
    fn typing_inserts_and_the_cursor_follows() {
        let mut e = editor("");
        for ch in "hello".chars() {
            e.insert_char(ch).unwrap();
        }
        assert_eq!(e.buffer.line(0), "hello");
        assert_eq!(e.cursor, Position::new(0, 5));
        e.insert_newline().unwrap();
        e.insert_char('x').unwrap();
        // The buffer started empty, so its line ending is this platform's.
        assert_eq!(
            e.buffer.to_text(),
            format!("hello{}x", e.buffer.eol.bytes())
        );
        assert_eq!(e.cursor, Position::new(1, 1));
    }

    #[test]
    fn a_tab_expands_to_the_next_tab_stop() {
        let mut e = editor("ab");
        e.move_to(Position::new(0, 2), false, BlockKind::Stream);
        e.insert_char('\t').unwrap();
        assert_eq!(e.buffer.line(0), "ab  ", "two spaces reach column 4");
        e.expand_tabs = false;
        e.insert_char('\t').unwrap();
        assert!(e.buffer.line(0).contains('\t'));
    }

    #[test]
    fn auto_indent_carries_the_indentation() {
        let mut e = editor("    if x:");
        e.move_line_end(false);
        e.insert_newline().unwrap();
        assert_eq!(e.buffer.line(1), "    ");
        assert_eq!(e.cursor, Position::new(1, 4));
        e.auto_indent = false;
        e.insert_newline().unwrap();
        assert_eq!(e.buffer.line(2), "");
    }

    #[test]
    fn backspace_joins_lines_and_delete_eats_forward() {
        let mut e = editor("ab\ncd\n");
        e.move_to(Position::new(1, 0), false, BlockKind::Stream);
        e.backspace().unwrap();
        assert_eq!(e.buffer.to_text(), "abcd\n");
        assert_eq!(e.cursor, Position::new(0, 2));
        e.delete_forward().unwrap();
        assert_eq!(e.buffer.line(0), "abd");
        // Backspace at the very start does nothing rather than erroring.
        e.move_file_start(false);
        e.backspace().unwrap();
        assert_eq!(e.buffer.line(0), "abd");
    }

    #[test]
    fn the_word_after_the_cursor_goes_with_the_spaces_behind_it() {
        let mut e = editor("one two three\n");
        e.move_file_start(false);
        e.delete_word_right().unwrap();
        assert_eq!(e.buffer.line(0), "two three");
        e.delete_word_right().unwrap();
        assert_eq!(e.buffer.line(0), "three");
        // At the end of the line there is nothing after the cursor to take.
        e.move_line_end(false);
        e.delete_word_right().unwrap();
        assert_eq!(e.buffer.line(0), "three");
    }

    #[test]
    fn word_and_line_deletions() {
        let mut e = editor("one two three\n");
        e.move_line_end(false);
        e.delete_word_left().unwrap();
        assert_eq!(e.buffer.line(0), "one two ");
        e.delete_to_line_end().unwrap();
        assert_eq!(e.buffer.line(0), "one two ");
        e.move_to(Position::new(0, 4), false, BlockKind::Stream);
        e.delete_to_line_end().unwrap();
        assert_eq!(e.buffer.line(0), "one ");
        e.delete_line().unwrap();
        assert!(e.buffer.is_empty());
    }

    #[test]
    fn smart_home_goes_to_the_text_then_to_the_margin() {
        let mut e = editor("    indented\n");
        e.move_line_end(false);
        e.move_line_start(false);
        assert_eq!(e.cursor.col, 4, "first to the text");
        e.move_line_start(false);
        assert_eq!(e.cursor.col, 0, "then to the margin");
    }

    #[test]
    fn moving_past_a_line_end_wraps_to_the_next_line() {
        let mut e = editor("ab\ncd\n");
        e.move_to(Position::new(0, 2), false, BlockKind::Stream);
        e.move_by(0, 1, false, BlockKind::Stream);
        assert_eq!(e.cursor, Position::new(1, 0));
        e.move_by(0, -1, false, BlockKind::Stream);
        assert_eq!(e.cursor, Position::new(0, 2));
    }

    #[test]
    fn display_columns_account_for_tabs_and_wide_characters() {
        let mut e = editor("a\tb\n日本\n");
        e.tab_width = 4;
        assert_eq!(e.display_col(Position::new(0, 1)), 1);
        assert_eq!(e.display_col(Position::new(0, 2)), 4, "the tab stop");
        assert_eq!(
            e.display_col(Position::new(1, 1)),
            2,
            "a wide glyph is two columns"
        );
    }

    #[test]
    fn a_stream_block_is_copied_cut_and_pasted() {
        let mut e = editor("one\ntwo\nthree\n");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(1, 3), true, BlockKind::Stream);
        assert_eq!(e.block_text().as_deref(), Some("one\ntwo"));
        assert!(e.cut_block().unwrap());
        assert_eq!(e.buffer.to_text(), "\nthree\n");
        e.move_file_end(false);
        assert!(e.paste().unwrap());
        // The buffer kept its final newline, and the paste landed at the end of `three`.
        assert_eq!(e.buffer.to_text(), "\nthreeone\ntwo\n");
    }

    #[test]
    fn a_column_block_takes_the_same_columns_from_every_line() {
        let mut e = editor("abcdef\nABCDEF\n123456\n");
        e.move_to(Position::new(0, 2), false, BlockKind::Column);
        e.move_to(Position::new(2, 4), true, BlockKind::Column);
        assert_eq!(e.block_text().as_deref(), Some("cd\nCD\n34"));

        assert!(e.cut_block().unwrap());
        assert_eq!(e.buffer.to_text(), "abef\nABEF\n1256\n");
        assert_eq!(e.cursor, Position::new(0, 2));

        // Pasting a column block puts the column back, line by line.
        e.move_to(Position::new(0, 4), false, BlockKind::Stream);
        assert!(e.paste().unwrap());
        assert_eq!(e.buffer.to_text(), "abefcd\nABEFCD\n125634\n");
    }

    #[test]
    fn pasting_a_column_block_pads_short_lines() {
        let mut e = editor("long line here\nx\n");
        e.clipboard = Some(("AA\nBB".to_string(), BlockKind::Column));
        e.move_to(Position::new(0, 5), false, BlockKind::Stream);
        assert!(e.paste().unwrap());
        assert_eq!(e.buffer.line(0), "long AAline here");
        assert_eq!(e.buffer.line(1), "x    BB", "the short line is padded");
    }

    #[test]
    fn typing_over_a_block_replaces_it() {
        let mut e = editor("hello world\n");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(0, 5), true, BlockKind::Stream);
        e.insert_char('X').unwrap();
        assert_eq!(e.buffer.line(0), "X world");
        assert!(e.block.is_none());
    }

    #[test]
    fn tab_indents_a_block_and_shift_tab_outdents_it() {
        let mut e = editor("a\nb\nc\n");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(1, 1), true, BlockKind::Stream);
        e.indent().unwrap();
        assert_eq!(e.buffer.to_text(), "    a\n    b\nc\n");
        e.outdent().unwrap();
        assert_eq!(e.buffer.to_text(), "a\nb\nc\n");
    }

    #[test]
    fn undo_restores_the_cursor_as_well_as_the_text() {
        let mut e = editor("hello\n");
        e.move_line_end(false);
        e.insert_text(" world").unwrap();
        assert_eq!(e.buffer.line(0), "hello world");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "hello");
        assert_eq!(e.cursor, Position::new(0, 5));
        assert!(e.redo());
        assert_eq!(e.buffer.line(0), "hello world");
        assert!(!e.undo() || true);
    }

    #[test]
    fn line_operations_move_the_cursor_with_the_line() {
        let mut e = editor("one\ntwo\nthree\n");
        e.move_to(Position::new(0, 1), false, BlockKind::Stream);
        e.duplicate_line().unwrap();
        assert_eq!(e.buffer.to_text(), "one\none\ntwo\nthree\n");
        assert_eq!(e.cursor.line, 1);
        e.move_line(true).unwrap();
        assert_eq!(e.buffer.to_text(), "one\ntwo\none\nthree\n");
        assert_eq!(e.cursor.line, 2);
    }

    #[test]
    fn search_finds_forwards_backwards_and_wraps() {
        let mut e = editor("alpha\nbeta\ngamma beta\n");
        assert!(e.find("beta", false).unwrap());
        assert_eq!(e.cursor, Position::new(1, 0));
        assert!(e.find("beta", false).unwrap());
        assert_eq!(e.cursor, Position::new(2, 6));
        // Wrapping round the end comes back to the first hit.
        assert!(e.find("beta", false).unwrap());
        assert_eq!(e.cursor, Position::new(1, 0));
        assert!(!e.find("zeta", false).unwrap());
    }

    #[test]
    fn search_is_case_insensitive_by_default_and_can_be_a_regex() {
        let mut e = editor("Hello World\n");
        assert!(e.find("hello", false).unwrap());
        e.search_case_insensitive = false;
        e.move_file_start(false);
        assert!(!e.find("hello", false).unwrap());
        e.search_mode = SearchMode::Regex;
        e.move_file_start(false);
        assert!(e.find(r"W\w+d", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 6));
        assert!(e.find("(unclosed", false).is_err());
    }

    #[test]
    fn replace_one_and_replace_all() {
        let mut e = editor("a a a\nb a\n");
        assert!(e.replace_once("a", "X").unwrap());
        assert_eq!(e.buffer.line(0), "X a a");
        let count = e.replace_all("a", "Y").unwrap();
        assert_eq!(count, 3);
        assert_eq!(e.buffer.to_text(), "X Y Y\nb Y\n");
        assert_eq!(e.replace_all("zzz", "!").unwrap(), 0);
    }

    #[test]
    fn goto_takes_one_based_line_and_column() {
        let mut e = editor("one\ntwo\nthree\n");
        assert!(e.goto("2"));
        assert_eq!(e.cursor, Position::new(1, 0));
        assert!(e.goto("3:4"));
        assert_eq!(e.cursor, Position::new(2, 3));
        assert!(!e.goto("not a line"));
        assert!(e.goto("999"), "past the end is clamped, not refused");
        assert_eq!(e.cursor.line, 2);
    }

    #[test]
    fn bookmarks_remember_positions() {
        let mut e = editor("1\n2\n3\n4\n");
        e.move_to(Position::new(2, 0), false, BlockKind::Stream);
        assert!(e.set_bookmark(5));
        e.move_file_start(false);
        assert!(e.goto_bookmark(5));
        assert_eq!(e.cursor, Position::new(2, 0));
        assert!(!e.goto_bookmark(9));
        assert!(!e.set_bookmark(99));
    }

    #[test]
    fn scrolling_keeps_the_cursor_on_screen() {
        let mut e = editor("1\n2\n3\n4\n5\n6\n7\n8\n");
        let visible = e.scroll_into_view(3, 20);
        assert_eq!(visible, 0..3);
        e.move_to(Position::new(7, 0), false, BlockKind::Stream);
        let visible = e.scroll_into_view(3, 20);
        assert_eq!(visible.end, 8);
        assert_eq!(e.top, 5);
    }

    #[test]
    fn saving_writes_atomically_and_keeps_the_file_shape() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"first\r\nsecond\r\n").unwrap();

        let mut e = Editor::open(&path).unwrap();
        assert_eq!(e.buffer.eol, Eol::Crlf);
        assert_eq!(e.buffer.len(), 2);
        e.move_file_end(false);
        e.insert_text("third").unwrap();
        assert!(e.buffer.modified());

        let outcome = e.save(false).unwrap();
        assert!(matches!(outcome, SaveOutcome::Saved { .. }));
        assert!(!e.buffer.modified());
        let written = std::fs::read_to_string(&path).unwrap();
        assert_eq!(
            written, "first\r\nsecondthird\r\n",
            "the CRLF endings must survive an edit"
        );
        // No temporary file left behind.
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains("fcmd-tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");

        assert_eq!(e.save(false).unwrap(), SaveOutcome::NothingToDo);
    }

    #[test]
    fn a_file_changed_on_disk_is_not_overwritten_silently() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("shared.txt");
        std::fs::write(&path, b"mine\n").unwrap();
        let mut e = Editor::open(&path).unwrap();
        e.insert_text("edited ").unwrap();

        // Someone else writes the file.
        std::thread::sleep(std::time::Duration::from_millis(10));
        std::fs::write(&path, b"theirs\n").unwrap();
        assert!(e.changed_on_disk());
        let err = e.save(false).unwrap_err();
        assert!(matches!(err, EditorError::ChangedOnDisk(_)), "{err}");
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "theirs\n");

        // Forcing writes anyway; reloading throws our changes away.
        assert!(e.save(true).is_ok());
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .starts_with("edited"));
        std::fs::write(&path, b"outside\n").unwrap();
        e.reload().unwrap();
        assert_eq!(e.buffer.line(0), "outside");
        assert!(!e.buffer.modified());
    }

    #[test]
    fn save_as_writes_a_new_file_and_adopts_it() {
        let dir = tempfile::tempdir().unwrap();
        let mut e = editor("content\n");
        assert!(matches!(e.save(false), Err(EditorError::NoPath)));
        let target = dir.path().join("new.txt");
        e.insert_text("more ").unwrap();
        e.save_as(&target).unwrap();
        assert_eq!(std::fs::read_to_string(&target).unwrap(), "more content\n");
        assert_eq!(e.path.as_ref().unwrap().display_name(), "new.txt");
        assert!(!e.buffer.modified());
    }

    #[test]
    fn a_utf16_file_is_read_and_written_back_as_utf16() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("wide.txt");
        let mut bytes = vec![0xFF, 0xFE];
        for unit in "ciao\r\n".encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }
        std::fs::write(&path, &bytes).unwrap();

        let mut e = Editor::open(&path).unwrap();
        assert_eq!(e.buffer.codepage.name(), "UTF-16LE");
        assert!(e.buffer.bom);
        assert_eq!(e.buffer.line(0), "ciao");
        e.move_line_end(false);
        e.insert_text("!").unwrap();
        e.save(false).unwrap();

        let written = std::fs::read(&path).unwrap();
        assert_eq!(&written[..2], &[0xFF, 0xFE], "the BOM must survive");
        let text = fcmd_view::Codepage::Utf16Le.decode(&written[2..]);
        assert_eq!(text, "ciao!\r\n");
    }

    #[test]
    fn the_status_line_reports_what_matters() {
        let mut e = editor("a\nb\n");
        let status = e.status();
        assert!(status.starts_with("1:1  2 lines  LF  UTF-8"), "{status}");
        e.insert_char('x').unwrap();
        assert!(e.status().ends_with('*'), "{}", e.status());
        e.buffer.read_only = true;
        assert!(e.status().contains("read-only"));
    }

    #[test]
    fn a_read_only_buffer_reports_instead_of_editing() {
        let mut e = editor("locked\n");
        e.move_line_end(false);
        e.buffer.read_only = true;
        assert!(e.insert_char('x').is_err());
        assert!(e.backspace().is_err());
        assert!(e.delete_line().is_err());
        assert_eq!(e.buffer.line(0), "locked");
    }
    #[test]
    fn a_block_is_added_to_the_clipboard_rather_than_replacing_it() {
        let mut e = editor("one\ntwo\nthree\n");
        // The first line, copied.
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(0, 3), true, BlockKind::Stream);
        assert!(e.copy_block());
        assert_eq!(e.clipboard.as_ref().unwrap().0, "one");

        // The third, appended: the two are now one paste, in the order they were taken.
        e.move_to(Position::new(2, 0), false, BlockKind::Stream);
        e.move_to(Position::new(2, 5), true, BlockKind::Stream);
        assert!(e.append_block());
        assert_eq!(e.clipboard.as_ref().unwrap().0, "one\nthree");

        // With nothing on the clipboard, appending is just a copy.
        let mut e = editor("x\n");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(0, 1), true, BlockKind::Stream);
        assert!(e.append_block());
        assert_eq!(e.clipboard.as_ref().unwrap().0, "x");
        // And with no block there is nothing to add.
        e.clear_block();
        assert!(!e.append_block());
    }

    #[test]
    fn a_literal_insert_puts_in_what_the_rules_would_have_changed() {
        let mut e = editor("ab\n");
        e.expand_tabs = true;
        e.tab_width = 4;
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);

        // An ordinary tab becomes spaces where tabs are expanded…
        e.insert_char('\t').unwrap();
        assert_eq!(e.buffer.line(0), "    ab");

        // …and a literal one stays a tab, which is the whole point of the key.
        e.insert_literal('\t').unwrap();
        assert!(
            e.buffer.line(0).starts_with("    \tab"),
            "{}",
            e.buffer.line(0)
        );

        // A control character goes in as itself.
        e.insert_literal('\u{1b}').unwrap();
        assert!(e.buffer.line(0).contains('\u{1b}'));
    }

    #[test]
    fn shifting_right_moves_the_line_the_cursor_is_on_where_tab_would_type_one() {
        let mut e = editor("one\ntwo\n");
        e.tab_width = 2;
        e.expand_tabs = true;
        e.move_to(Position::new(0, 1), false, BlockKind::Stream);

        e.shift_right().unwrap();
        assert_eq!(e.buffer.line(0), "  one", "the line moved, not the caret");
        e.outdent().unwrap();
        assert_eq!(e.buffer.line(0), "one");

        // With a block it moves every line of it, which is what it always did.
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(1, 1), true, BlockKind::Stream);
        e.shift_right().unwrap();
        assert_eq!(e.buffer.line(0), "  one");
        assert_eq!(e.buffer.line(1), "  two");
    }

    #[test]
    fn the_all_tabs_policy_expands_what_is_already_in_the_file() {
        let mut e = editor("\tone\ntwo\n\t\tthree\n");
        e.tab_width = 4;
        assert_eq!(
            e.expand_all_tabs().unwrap(),
            2,
            "two lines had tabs in them"
        );
        assert_eq!(e.buffer.line(0), "    one");
        assert_eq!(e.buffer.line(1), "two");
        assert_eq!(e.buffer.line(2), "        three");
        // Again changes nothing: there are no tabs left.
        assert_eq!(e.expand_all_tabs().unwrap(), 0);
    }

    /// Off — the default, and what a text editor does — the cursor cannot leave the text.
    #[test]
    fn by_default_the_cursor_stops_at_the_end_of_the_line() {
        let mut e = editor("ab\ncd\n");
        e.move_by(0, 5, false, BlockKind::Stream);
        // Past the end it went to the next line, which is where `Right` goes.
        assert_eq!(e.cursor, Position::new(1, 2));
        e.move_to(Position::new(0, 9), false, BlockKind::Stream);
        assert_eq!(e.cursor, Position::new(0, 2), "clamped onto the text");
    }

    /// On, the columns past the text are somewhere to be — and nothing is written there
    /// until something is typed, which is the whole bargain of the setting.
    #[test]
    fn the_cursor_may_stand_past_the_end_of_a_line_and_typing_there_pads_it() {
        let mut e = editor("ab\ncd\n");
        e.beyond_eol = true;
        // `Right` four times goes into the empty columns rather than to the next line.
        e.move_by(0, 4, false, BlockKind::Stream);
        assert_eq!(e.cursor, Position::new(0, 4));
        assert_eq!(e.buffer.line(0), "ab", "moving about writes nothing");
        // The display column counts the empty ones as one each: there is nothing there to
        // be two columns wide.
        assert_eq!(e.display_col(e.cursor), 4);

        // Typing there is the moment the spaces appear, and the character lands in the
        // column it was typed in.
        e.insert_char('x').unwrap();
        assert_eq!(e.buffer.line(0), "ab  x");
        assert_eq!(e.cursor, Position::new(0, 5));

        // And one undo takes the padding and the character together: they are one edit,
        // because they were one keystroke.
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "ab");
    }

    #[test]
    fn past_the_end_of_a_line_backspace_and_del_change_nothing() {
        let mut e = editor("ab\ncd\n");
        e.beyond_eol = true;
        e.move_to(Position::new(0, 5), false, BlockKind::Stream);
        e.backspace().unwrap();
        assert_eq!(e.cursor, Position::new(0, 4), "the cursor moves back");
        assert_eq!(e.buffer.line(0), "ab", "and the file is left alone");
        e.delete_forward().unwrap();
        assert_eq!(e.buffer.line(0), "ab");
        assert_eq!(e.buffer.len(), 2, "and the next line is not pulled up");
        // Back on the text the two keys mean what they always did.
        e.move_to(Position::new(0, 2), false, BlockKind::Stream);
        e.backspace().unwrap();
        assert_eq!(e.buffer.line(0), "a");
    }

    /// The reason anybody turns this on: the same columns down a file, whatever the lines
    /// are. A rectangular block marked through the short lines still takes what is there.
    #[test]
    fn a_rectangle_may_be_marked_through_lines_that_are_too_short() {
        let mut e = editor("aaaaaa\nbb\ncccccc\n");
        e.beyond_eol = true;
        e.move_to(Position::new(0, 2), false, BlockKind::Column);
        e.move_to(Position::new(2, 5), true, BlockKind::Column);
        assert_eq!(e.block_text().as_deref(), Some("aaa\n\nccc"));
        // Enter past the end of a line splits nothing and pads nothing: a new empty line.
        e.clear_block();
        e.move_to(Position::new(1, 6), false, BlockKind::Stream);
        e.insert_newline().unwrap();
        assert_eq!(e.buffer.line(1), "bb");
        assert_eq!(e.buffer.line(2), "");
    }

    #[test]
    fn a_search_can_leave_the_cursor_after_what_it_found() {
        let mut e = editor("the needle is here\n");
        assert!(e.find("needle", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 4), "at the start of the match");
        let block = e.block.expect("the match is selected");
        assert_eq!(block.anchor, Position::new(0, 4));
        assert_eq!(block.head, Position::new(0, 10));

        let mut e = editor("the needle is here\n");
        e.cursor_after_match = true;
        assert!(e.find("needle", false).unwrap());
        assert_eq!(
            e.cursor,
            Position::new(0, 10),
            "just past it, for typing over what was found"
        );
        // Either way the match itself is still what is selected.
        let block = e.block.expect("the match is selected");
        assert_eq!(block.anchor, Position::new(0, 4));
        assert_eq!(block.head, Position::new(0, 10));
    }
}
