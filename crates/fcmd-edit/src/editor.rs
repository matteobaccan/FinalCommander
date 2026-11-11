use std::path::{Path, PathBuf};

use fcmd_core::VPath;
use fcmd_view::guides::{self, Guides};
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

/// The four things `Alt+C` does to the letters of a block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    Upper,
    Lower,
    /// A capital at the start of every word and small letters after it.
    Title,
    /// Every capital made small and every small letter a capital.
    Toggle,
}

impl Case {
    pub const ALL: [Case; 4] = [Case::Upper, Case::Lower, Case::Title, Case::Toggle];

    pub fn as_str(self) -> &'static str {
        match self {
            Case::Upper => "upper",
            Case::Lower => "lower",
            Case::Title => "title",
            Case::Toggle => "toggle",
        }
    }

    pub fn parse(text: &str) -> Option<Case> {
        Case::ALL.into_iter().find(|c| c.as_str() == text)
    }

    /// `text` in this case. Unicode's own mappings, so `straße` becomes `STRASSE` and the
    /// text may come out longer than it went in; what has no case comes out as it was.
    pub fn apply(self, text: &str) -> String {
        match self {
            Case::Upper => text.to_uppercase(),
            Case::Lower => text.to_lowercase(),
            Case::Toggle => text
                .chars()
                .flat_map(|c| -> Vec<char> {
                    if c.is_uppercase() {
                        c.to_lowercase().collect()
                    } else if c.is_lowercase() {
                        c.to_uppercase().collect()
                    } else {
                        vec![c]
                    }
                })
                .collect(),
            Case::Title => {
                let mut out = String::with_capacity(text.len());
                // An apostrophe inside a word does not start another: `don't`, `l'arte`
                // as one word each is the lesser evil, and `Don'T` would be the greater.
                let mut inside = false;
                for c in text.chars() {
                    if c.is_alphanumeric() {
                        if inside {
                            out.extend(c.to_lowercase());
                        } else {
                            out.extend(c.to_uppercase());
                        }
                        inside = true;
                    } else {
                        inside = inside && matches!(c, '\'' | '\u{2019}');
                        out.push(c);
                    }
                }
                out
            }
        }
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

/// One hit of a search: the line it is on and the columns, in characters, it runs between.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hit {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

/// The kinds of key that are undone together when they come one after the other: typing a
/// word and then rubbing half of it out are two things done, and so two steps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Burst {
    Typing,
    Backspace,
    Delete,
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
    /// Number the lines in a gutter left of the text (mcedit's `Alt+N`). Kept here for the
    /// same reason: the editor does not draw itself, but the gutter takes columns the text
    /// would have had, and the scrolling has to know how many.
    pub line_numbers: bool,
    /// Where the cursor lands on a match: at its start, or just past its end.
    pub cursor_after_match: bool,
    /// May the cursor stand past the end of a line? Off, it is clamped to the last character,
    /// which is what a text editor does; on, the columns past the end are somewhere to be,
    /// and the line is padded with spaces only when something is actually typed there.
    pub beyond_eol: bool,
    /// What `.editorconfig` said about trailing blanks for this file, which wins over the
    /// setting when it says anything (SPEC 05 §2.9).
    pub trim_on_save: Option<bool>,
    /// The word completion in progress, so a second `Alt+/` offers the next word instead of
    /// completing the completion.
    completion: Option<Completion>,
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
    /// FAR's *preserve style*: a plain replacement takes the capitals of what it replaces,
    /// so `foo` → `bar` also turns `Foo` into `Bar` and `FOO` into `BAR`.
    pub replace_preserve_style: bool,
    /// The last key of a burst and where it left the cursor. The next key continues the
    /// burst only if it is of the same kind and the cursor has not been anywhere since.
    burst: Option<(Burst, Position)>,
    /// What the file is being read as, and how much of it has been scanned so far.
    pub highlight: crate::Highlight,
    /// The block guides of the rows last asked for, kept until an edit reaches them.
    guides: Option<GuideMemo>,
}

/// The guides of one window of the buffer, and everything they were worked out from that
/// can change without the text changing.
struct GuideMemo {
    rows: std::ops::Range<usize>,
    tab_width: usize,
    syntax: &'static str,
    guides: Vec<Guides>,
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
            line_numbers: false,
            cursor_after_match: false,
            beyond_eol: false,
            trim_on_save: None,
            completion: None,
            literal_next: false,
            bookmarks: [None; 10],
            clipboard: None,
            mtime,
            last_search: None,
            last_replace: None,
            search_mode: SearchMode::Plain,
            search_case_insensitive: true,
            search_whole_words: false,
            replace_preserve_style: false,
            burst: None,
            highlight,
            guides: None,
        }
    }

    /// The coloured runs of one line, with the scan kept in step with the edits.
    ///
    /// It is a method of the editor rather than of the buffer because the two have to be
    /// borrowed together: the cache is extended while the buffer is read.
    pub fn spans(&mut self, line: usize) -> Vec<fcmd_syntax::Span> {
        self.forget_touched();
        let Editor {
            buffer, highlight, ..
        } = self;
        highlight.spans(buffer, line)
    }

    /// Throw away what was worked out from lines that have changed since: the scan from
    /// the first of them down, and the guides if the change is in or above their window.
    ///
    /// The buffer says once how far up an edit reached, so both caches have to hear it
    /// here, in one place — asked separately, whoever came second would be told nothing.
    fn forget_touched(&mut self) {
        if let Some(from) = self.buffer.take_touched() {
            self.highlight.forget_from(from);
            if self
                .guides
                .as_ref()
                .is_some_and(|memo| from < memo.rows.end)
            {
                self.guides = None;
            }
        }
    }

    /// The lines the Delphi IDE draws down the blocks of a Pascal unit or a form, for the
    /// buffer lines `rows`: one entry per line, columns counted as the screen shows them
    /// with tabs expanded and before any horizontal scroll. `None` for a syntax that has no
    /// blocks to draw — the same answer [`guides::applies`] gives the viewer.
    ///
    /// Only the window is worked out, with the lines above it read back as far as the
    /// viewer reads ([`guides::LOOKBACK`]) to find the blocks already open, and the answer
    /// is kept: moving the cursor, or typing below the window, costs nothing, and an edit
    /// in the window or above it costs that one scan again.
    pub fn block_guides(&mut self, rows: std::ops::Range<usize>) -> Option<&[Guides]> {
        let syntax = self.highlight.syntax().filter(|s| guides::applies(s))?;
        self.forget_touched();
        let end = rows.end.min(self.buffer.len());
        let rows = rows.start.min(end)..end;
        let kept = self.guides.as_ref().is_some_and(|memo| {
            memo.rows == rows && memo.tab_width == self.tab_width && memo.syntax == syntax.name
        });
        if !kept {
            // Back from the window until the look-back is spent, a whole line at a time.
            let mut start = rows.start;
            let mut budget = guides::LOOKBACK as usize;
            while start > 0 {
                let cost = self.buffer.line(start - 1).len() + 1;
                if cost > budget {
                    break;
                }
                budget -= cost;
                start -= 1;
            }
            // The scanner begins where the colouring says that line begins: inside a
            // comment opened above the look-back, its words still open nothing.
            let state = self.highlight.state_at(&self.buffer, start);
            let lines: Vec<String> = (start..rows.end)
                .map(|i| fcmd_view::view::expand_tabs(self.buffer.line(i), self.tab_width))
                .collect();
            self.guides = Some(GuideMemo {
                guides: guides::guides_from(syntax, &lines, rows.start - start, state),
                rows,
                tab_width: self.tab_width,
                syntax: syntax.name,
            });
        }
        self.guides.as_ref().map(|memo| memo.guides.as_slice())
    }

    /// How many columns the line numbers take, the blank that parts them from the text
    /// included; nothing when they are off. As wide as the last line's number and never
    /// narrower than three digits, so the text does not step sideways when a short file
    /// grows past its ninth or ninety-ninth line.
    pub fn gutter_width(&self) -> usize {
        if !self.line_numbers {
            return 0;
        }
        self.buffer.len().max(1).to_string().len().max(3) + 1
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

    /// Run one key of a burst. Characters typed one after the other on a line are one undo
    /// step, and so is a run of `Backspace` or of `Del`, as in FAR: nobody wants to press
    /// `Ctrl+Z` eleven times for a word. A key of another kind, a command, or the cursor
    /// having been anywhere else in between starts a new step. There is no clock in this —
    /// the event loop keeps no timer, and what was done is a better boundary than when.
    fn in_burst(
        &mut self,
        kind: Burst,
        key: impl FnOnce(&mut Editor) -> Result<(), EditorError>,
    ) -> Result<(), EditorError> {
        let continues = self.burst == Some((kind, self.cursor));
        self.buffer.begin_burst(self.cursor, continues);
        let result = key(self);
        // Also when the key failed: a burst left armed would swallow the next command.
        self.buffer.end_burst(self.cursor);
        self.burst = result.is_ok().then_some((kind, self.cursor));
        result
    }

    pub fn insert_char(&mut self, ch: char) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        self.in_burst(Burst::Typing, |e| {
            e.fill_to_cursor()?;
            if ch == '\t' && e.expand_tabs {
                let pad = e.tab_width - (e.display_col(e.cursor) % e.tab_width);
                e.cursor = e.buffer.insert(e.cursor, &" ".repeat(pad))?;
            } else {
                e.cursor = e.buffer.insert(e.cursor, &ch.to_string())?;
            }
            Ok(())
        })
    }

    /// Insert a character exactly as it is, with none of the rules `insert_char` applies: a
    /// tab stays a tab where tabs are expanded, and a control character goes in as itself.
    /// This is what `Ctrl+Q` is for.
    pub fn insert_literal(&mut self, ch: char) -> Result<(), EditorError> {
        self.delete_block_if_any()?;
        self.in_burst(Burst::Typing, |e| {
            e.fill_to_cursor()?;
            e.cursor = e.buffer.insert(e.cursor, &ch.to_string())?;
            Ok(())
        })
    }

    /// `Alt+/`: finish the word before the cursor with another word of the buffer that
    /// starts the same way — the nearest one first, above or below — and, pressed again
    /// straight away, the next one instead, then back to what was typed. Emacs' dabbrev and
    /// FAR's EditorCompletion, without a dictionary: the words are the file's own.
    pub fn complete_word(&mut self) -> Result<bool, EditorError> {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let line_text: Vec<char> = self.buffer.line(self.cursor.line).chars().collect();
        // Still the same completion? Then the cursor sits right after what was put in.
        let again = self.completion.as_ref().is_some_and(|c| {
            c.line == self.cursor.line && c.start + c.shown.chars().count() == self.cursor.col
        });
        if !again {
            let col = self.cursor.col.min(line_text.len());
            let mut start = col;
            while start > 0 && is_word(line_text[start - 1]) {
                start -= 1;
            }
            if start == col {
                return Ok(false);
            }
            let prefix: String = line_text[start..col].iter().collect();
            let mut seen = std::collections::HashSet::new();
            let mut candidates: Vec<String> = Vec::new();
            let here = self.cursor.line;
            let count = self.buffer.len();
            // The nearest lines first: this one, then one above, one below, and so on out.
            let mut order = vec![here];
            for d in 1..count.max(1) {
                if here >= d {
                    order.push(here - d);
                }
                if here + d < count {
                    order.push(here + d);
                }
            }
            for l in order {
                let text = self.buffer.line(l);
                for word in text.split(|c: char| !is_word(c)) {
                    if word.len() > prefix.len()
                        && word.starts_with(prefix.as_str())
                        && seen.insert(word.to_string())
                    {
                        candidates.push(word.to_string());
                    }
                }
                if candidates.len() >= 200 {
                    break;
                }
            }
            if candidates.is_empty() {
                return Ok(false);
            }
            self.completion = Some(Completion {
                line: here,
                start,
                shown: prefix.clone(),
                prefix,
                candidates,
                next: 0,
            });
        }
        let c = self.completion.as_mut().expect("just set");
        // The candidates, then the prefix as it was typed, then round again.
        let pick = if c.next < c.candidates.len() {
            c.candidates[c.next].clone()
        } else {
            c.prefix.clone()
        };
        c.next = (c.next + 1) % (c.candidates.len() + 1);
        let range = Range {
            start: Position::new(c.line, c.start),
            end: Position::new(c.line, c.start + c.shown.chars().count()),
        };
        c.shown = pick.clone();
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(range)?;
        self.cursor = self.buffer.insert(range.start, &pick)?;
        self.buffer.end_group(self.cursor);
        Ok(true)
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
        self.in_burst(Burst::Backspace, |e| {
            e.buffer.delete(Range::new(start, end))?;
            e.cursor = start;
            Ok(())
        })
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
        self.in_burst(Burst::Delete, |e| {
            e.buffer.delete(Range::new(start, end))?;
            Ok(())
        })
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

    /// `Alt+B`: to the bracket that matches the one under the cursor, or the one just before
    /// it — mcedit's key. `()`, `[]`, `{}` and `<>`, counted across lines. Brackets inside a
    /// string or a comment count like any other, which is what a plain text editor can know;
    /// it is the editor of a file manager, not of one language. `false` when there is no
    /// bracket there or nothing matches it.
    pub fn match_bracket(&mut self) -> bool {
        const PAIRS: [(char, char); 4] = [('(', ')'), ('[', ']'), ('{', '}'), ('<', '>')];
        let line: Vec<char> = self.buffer.line(self.cursor.line).chars().collect();
        let at = self.cursor.col;
        let (col, ch) = match line.get(at) {
            Some(c) if PAIRS.iter().any(|(o, cl)| o == c || cl == c) => (at, *c),
            _ => match at.checked_sub(1).and_then(|b| line.get(b).map(|c| (b, *c))) {
                Some((b, c)) if PAIRS.iter().any(|(o, cl)| *o == c || *cl == c) => (b, c),
                _ => return false,
            },
        };
        let Some(&(open, close)) = PAIRS.iter().find(|(o, c)| *o == ch || *c == ch) else {
            return false;
        };
        let forward = ch == open;
        let mut depth = 0usize;
        let mut pos = Position::new(self.cursor.line, col);
        loop {
            let text: Vec<char> = self.buffer.line(pos.line).chars().collect();
            if let Some(&c) = text.get(pos.col) {
                if c == ch {
                    depth += 1;
                } else if c == if forward { close } else { open } {
                    depth -= 1;
                    if depth == 0 {
                        self.cursor = pos;
                        return true;
                    }
                }
            }
            // One step along, over line ends.
            if forward {
                if pos.col + 1 < text.len() {
                    pos.col += 1;
                } else if pos.line + 1 < self.buffer.len() {
                    pos = Position::new(pos.line + 1, 0);
                } else {
                    return false;
                }
            } else if pos.col > 0 {
                pos.col -= 1;
            } else if pos.line > 0 {
                let above = self.buffer.line(pos.line - 1).chars().count();
                pos = Position::new(pos.line - 1, above.saturating_sub(1));
            } else {
                return false;
            }
        }
    }

    /// `Alt+J`: the next line joins this one, its indent replaced by one space — none when
    /// either side already ends or starts with nothing to separate.
    pub fn join_line(&mut self) -> Result<bool, EditorError> {
        let line = self.cursor.line;
        if line + 1 >= self.buffer.len() {
            return Ok(false);
        }
        let here = self.buffer.line(line).trim_end().to_string();
        let next = self.buffer.line(line + 1);
        let skipped = next.chars().take_while(|c| c.is_whitespace()).count();
        let glue = if here.is_empty() || next.trim().is_empty() {
            ""
        } else {
            " "
        };
        self.buffer.begin_group(self.cursor);
        self.buffer.delete(Range::new(
            Position::new(line, here.chars().count()),
            Position::new(line + 1, skipped),
        ))?;
        let at = Position::new(line, here.chars().count());
        self.buffer.insert(at, glue)?;
        self.cursor = Position::new(line, here.chars().count() + glue.chars().count());
        self.buffer.end_group(self.cursor);
        Ok(true)
    }

    /// The lines a block covers, first and last. A stream block that ends at column 0 of a
    /// line has not taken anything of that line, so it stops at the one before.
    fn block_lines(&self) -> Option<(usize, usize)> {
        let block = self.block?;
        let (top, mut bottom, _, _) = block.columns();
        if block.kind == BlockKind::Stream && bottom > top {
            let end = if block.anchor.line > block.head.line {
                block.anchor
            } else {
                block.head
            };
            if end.col == 0 {
                bottom -= 1;
            }
        }
        Some((top, bottom))
    }

    /// `Alt+Shift+S`: the lines of the block in order. Byte order of their text, which is
    /// the one that sorts the same everywhere; the block's lines only, so nothing outside the
    /// selection moves. One undo step.
    pub fn sort_lines(&mut self) -> Result<bool, EditorError> {
        self.reshape_lines(|lines| lines.sort())
    }

    /// `Alt+Shift+R`: the lines of the block in the opposite order.
    pub fn reverse_lines(&mut self) -> Result<bool, EditorError> {
        self.reshape_lines(|lines| lines.reverse())
    }

    /// `Alt+Shift+U`: every line of the block that is the same as one above it, gone — the
    /// first of each stays where it was, which is `awk '!seen[$0]++'` rather than `uniq`, so a
    /// block need not be sorted first. Returns how many lines went, or `None` with no block.
    pub fn unique_lines(&mut self) -> Result<Option<usize>, EditorError> {
        let Some((top, bottom)) = self.block_lines() else {
            return Ok(None);
        };
        let mut seen = std::collections::HashSet::new();
        let lines: Vec<String> = (top..=bottom)
            .map(|i| self.buffer.line(i).to_string())
            .collect();
        let kept: Vec<String> = lines
            .iter()
            .filter(|l| seen.insert(l.as_str()))
            .cloned()
            .collect();
        let gone = lines.len() - kept.len();
        if gone > 0 {
            self.replace_lines(top, bottom, &kept)?;
            // The block shrinks with what it covers, so a second press finds nothing to do.
            self.block = Some(Block {
                kind: BlockKind::Stream,
                anchor: Position::new(top, 0),
                head: Position::new(
                    top + kept.len() - 1,
                    self.buffer.line(top + kept.len() - 1).chars().count(),
                ),
            });
        }
        Ok(Some(gone))
    }

    /// `Alt+P`: the paragraph the cursor is in — or every paragraph of the marked block —
    /// laid out again so that no line is wider than `width` columns, which is mcedit's
    /// *format paragraph* and what FAR leaves to its `align` plugin.
    ///
    /// A paragraph is a run of lines with something on them; an empty line parts two and is
    /// kept. Its words are put back with one space between them, as many to a line as fit,
    /// and every line gets the indent the paragraph's first line had — so a paragraph of a
    /// list or of an indented comment stays where it was. A word longer than the width has
    /// a line to itself rather than being cut. One undo step; `false` when the cursor is on
    /// an empty line and nothing is marked.
    pub fn reflow(&mut self, width: usize) -> Result<bool, EditorError> {
        let blank = |text: &str| text.trim().is_empty();
        let (top, bottom) = match self.block_lines() {
            Some(lines) => lines,
            None => {
                let here = self.cursor.line;
                if blank(self.buffer.line(here)) {
                    return Ok(false);
                }
                let mut top = here;
                while top > 0 && !blank(self.buffer.line(top - 1)) {
                    top -= 1;
                }
                let mut bottom = here;
                while bottom + 1 < self.buffer.len() && !blank(self.buffer.line(bottom + 1)) {
                    bottom += 1;
                }
                (top, bottom)
            }
        };
        let before: Vec<String> = (top..=bottom)
            .map(|i| self.buffer.line(i).to_string())
            .collect();
        let mut after: Vec<String> = Vec::with_capacity(before.len());
        let mut at = 0;
        while at < before.len() {
            if blank(&before[at]) {
                after.push(String::new());
                at += 1;
                continue;
            }
            let end = (at..before.len())
                .find(|&i| blank(&before[i]))
                .unwrap_or(before.len());
            let first = &before[at];
            let indent = &first[..first.len() - first.trim_start().len()];
            let indent_cols = fcmd_view::view::expand_tabs(indent, self.tab_width)
                .chars()
                .count();
            let mut line = String::new();
            let mut cols = 0;
            for word in before[at..end].iter().flat_map(|l| l.split_whitespace()) {
                let word_cols = unicode_width::UnicodeWidthStr::width(word);
                if !line.is_empty() && cols + 1 + word_cols > width {
                    after.push(std::mem::take(&mut line));
                }
                if line.is_empty() {
                    line.push_str(indent);
                    cols = indent_cols;
                } else {
                    line.push(' ');
                    cols += 1;
                }
                line.push_str(word);
                cols += word_cols;
            }
            after.push(line);
            at = end;
        }
        if after != before {
            self.replace_lines(top, bottom, &after)?;
        }
        // The block's ends are positions in the text as it was.
        self.block = None;
        self.cursor = self.clamp(Position::new(top, 0));
        Ok(true)
    }

    /// Take out every line that matches `pattern`, or with `keep` every line that does not —
    /// vim's `:g/re/d` and `:v/re/d` (FAR's issue #10). Over the block when there is one, the
    /// whole file otherwise; one undo step. A file left with nothing is one empty line.
    pub fn delete_matching(&mut self, pattern: &Regex, keep: bool) -> Result<usize, EditorError> {
        let (top, bottom) = self
            .block_lines()
            .unwrap_or((0, self.buffer.len().saturating_sub(1)));
        let lines: Vec<String> = (top..=bottom)
            .map(|i| self.buffer.line(i).to_string())
            .collect();
        let mut kept: Vec<String> = lines
            .iter()
            .filter(|l| pattern.is_match(l) == keep)
            .cloned()
            .collect();
        let gone = lines.len() - kept.len();
        if gone == 0 {
            return Ok(0);
        }
        if kept.is_empty() {
            kept.push(String::new());
        }
        self.replace_lines(top, bottom, &kept)?;
        self.block = None;
        let last = self.buffer.len().saturating_sub(1);
        self.cursor = Position::new(top.min(last), 0);
        Ok(gone)
    }

    /// [`Editor::delete_matching`] from the text of a pattern, which is what a prompt has.
    pub fn delete_matching_text(&mut self, pattern: &str, keep: bool) -> Result<usize, String> {
        let re = Regex::new(pattern).map_err(|e| e.to_string())?;
        self.delete_matching(&re, keep).map_err(|e| e.to_string())
    }

    /// `Alt+Shift+N`: a number in front of every line of the block, from 1, right-aligned to
    /// the widest so the text after them still lines up.
    pub fn number_lines(&mut self) -> Result<bool, EditorError> {
        self.reshape_lines(|lines| {
            let width = lines.len().to_string().len();
            for (i, line) in lines.iter_mut().enumerate() {
                *line = format!("{:>width$} {line}", i + 1);
            }
        })
    }

    /// The block's lines, rewritten by `f` and put back as one undo step; lines that come out
    /// the same are not touched. `false` when there is no block.
    fn reshape_lines(&mut self, f: impl FnOnce(&mut Vec<String>)) -> Result<bool, EditorError> {
        let Some((top, bottom)) = self.block_lines() else {
            return Ok(false);
        };
        let before: Vec<String> = (top..=bottom)
            .map(|i| self.buffer.line(i).to_string())
            .collect();
        let mut lines = before.clone();
        f(&mut lines);
        if lines == before {
            return Ok(true);
        }
        self.buffer.begin_group(self.cursor);
        for (offset, text) in lines.iter().enumerate() {
            if *text != before[offset] {
                self.rewrite_line(top + offset, text)?;
            }
        }
        self.buffer.end_group(self.cursor);
        self.cursor = self.clamp(self.cursor);
        Ok(true)
    }

    /// Lines `top..=bottom` replaced by `lines`, which may be fewer, as one undo step.
    fn replace_lines(
        &mut self,
        top: usize,
        bottom: usize,
        lines: &[String],
    ) -> Result<(), EditorError> {
        self.buffer.begin_group(self.cursor);
        let end = Position::new(bottom, self.buffer.line(bottom).chars().count());
        self.buffer.delete(Range::new(Position::new(top, 0), end))?;
        self.buffer
            .insert(Position::new(top, 0), &lines.join("\n"))?;
        self.cursor = self.clamp(Position::new(top, 0));
        self.buffer.end_group(self.cursor);
        Ok(())
    }

    /// The spaces and tabs at the end of every line, gone. How many lines changed: zero is a
    /// file that was clean, and leaves the undo history alone.
    pub fn trim_trailing(&mut self) -> Result<usize, EditorError> {
        let dirty: Vec<usize> = (0..self.buffer.len())
            .filter(|&i| {
                let l = self.buffer.line(i);
                l.len() != l.trim_end_matches([' ', '\t']).len()
            })
            .collect();
        if dirty.is_empty() {
            return Ok(0);
        }
        self.buffer.begin_group(self.cursor);
        for &i in &dirty {
            let trimmed = self
                .buffer
                .line(i)
                .trim_end_matches([' ', '\t'])
                .to_string();
            self.buffer.set_line(i, &trimmed)?;
        }
        self.buffer.end_group(self.cursor);
        self.cursor = self.clamp(self.cursor);
        Ok(dirty.len())
    }

    /// A line's text replaced inside whatever undo group is open — unlike `Buffer::set_line`,
    /// which is a group of its own, so a change over many lines would be many undo steps.
    fn rewrite_line(&mut self, index: usize, text: &str) -> Result<(), EditorError> {
        let end = Position::new(index, self.buffer.line(index).chars().count());
        self.buffer
            .delete(Range::new(Position::new(index, 0), end))?;
        self.buffer.insert(Position::new(index, 0), text)?;
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
        self.burst = None;
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
        self.burst = None;
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

    /// Put `text` where the marked block is, as **one** undo step, and leave what was put
    /// there marked — so the next thing done to "the block" is done to the new text, and a
    /// single undo brings the old one back. Only a stream block: a column of a rewritten
    /// text has no shape to go back into. False when there is no such block.
    pub fn replace_block(&mut self, text: &str) -> Result<bool, EditorError> {
        let Some(block) = self.block else {
            return Ok(false);
        };
        if block.kind != BlockKind::Stream {
            return Ok(false);
        }
        let range = block.range();
        self.buffer.begin_group(self.cursor);
        let done = self
            .buffer
            .delete(range)
            .and_then(|_| self.buffer.insert(range.start, text));
        // The group is closed whichever way it went: a refusal must not leave a step open
        // for the next key to fall into.
        match done {
            Ok(end) => {
                self.cursor = end;
                self.buffer.end_group(self.cursor);
                self.block = Some(Block {
                    anchor: range.start,
                    head: end,
                    kind: BlockKind::Stream,
                });
                Ok(true)
            }
            Err(e) => {
                self.buffer.end_group(self.cursor);
                Err(e.into())
            }
        }
    }

    /// The word the cursor is on, or stands right after, as the columns it runs between on
    /// the cursor's line. Letters, digits and the underscore make a word, as they do for
    /// *whole words* in the search.
    pub fn word_at_cursor(&self) -> Option<(usize, usize)> {
        let is_word = |c: char| c.is_alphanumeric() || c == '_';
        let chars: Vec<char> = self.buffer.line(self.cursor.line).chars().collect();
        let col = self.cursor.col.min(chars.len());
        let on = chars.get(col).copied().is_some_and(is_word);
        if !on && !(col > 0 && is_word(chars[col - 1])) {
            return None;
        }
        let mut start = col;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        let mut end = col;
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        Some((start, end))
    }

    /// Change the case of the marked block — every line's piece of a column block — or,
    /// with nothing marked, of the word the cursor is on (FAR's `editcase`). One undo step;
    /// the block stays marked, so a second choice is made on the same text. `false` when
    /// there is neither a block nor a word to act on.
    pub fn change_case(&mut self, case: Case) -> Result<bool, EditorError> {
        let was = self.cursor;
        match self.block {
            Some(block) if block.kind == BlockKind::Stream && !block.range().is_empty() => {
                let before = self.buffer.text_in(block.range());
                let after = case.apply(&before);
                if after != before {
                    self.replace_block(&after)?;
                    // Where it was, when the text is still the same length: changing the
                    // case of a selection is not a reason for the cursor to move.
                    if after.chars().count() == before.chars().count() {
                        self.cursor = was;
                    }
                }
                Ok(true)
            }
            Some(block) if block.kind == BlockKind::Column => {
                let (top, bottom, left, right) = block.columns();
                self.buffer.begin_group(was);
                let mut done = Ok(());
                for line in top..=bottom.min(self.buffer.len() - 1) {
                    let len = self.buffer.line_chars(line);
                    if left >= len || right <= left {
                        continue;
                    }
                    let range = Range::new(
                        Position::new(line, left),
                        Position::new(line, right.min(len)),
                    );
                    let before = self.buffer.text_in(range);
                    let after = case.apply(&before);
                    if after != before {
                        done = self
                            .buffer
                            .delete(range)
                            .and_then(|_| self.buffer.insert(range.start, &after))
                            .map(|_| ());
                        if done.is_err() {
                            break;
                        }
                    }
                }
                self.buffer.end_group(was);
                done.map(|()| true).map_err(Into::into)
            }
            _ => {
                let Some((start, end)) = self.word_at_cursor() else {
                    return Ok(false);
                };
                let range =
                    Range::new(Position::new(was.line, start), Position::new(was.line, end));
                let before = self.buffer.text_in(range);
                let after = case.apply(&before);
                if after != before {
                    self.buffer.begin_group(was);
                    let done = self
                        .buffer
                        .delete(range)
                        .and_then(|_| self.buffer.insert(range.start, &after));
                    self.buffer.end_group(was);
                    done?;
                    self.cursor = self.clamp(was);
                }
                Ok(true)
            }
        }
    }

    /// Read the file as another language, or as none: the choice made by hand in
    /// `Alt+Shift+Y`, for the file whose name says nothing or says the wrong thing. The
    /// colours and the block guides are both worked out again from the first line.
    pub fn set_syntax(&mut self, syntax: Option<&'static fcmd_syntax::Syntax>) {
        self.highlight.set_syntax(syntax);
        self.guides = None;
    }

    /// What a command is given to filter (`Alt+Shift+P`): the marked block, or with nothing
    /// marked the whole text, its lines parted by `\n` whatever the file's own ending is.
    /// `None` for a column block, which has no shape a rewritten text could go back into.
    pub fn filter_input(&self) -> Option<String> {
        match self.block {
            Some(block) if block.kind == BlockKind::Column => None,
            Some(block) if !block.range().is_empty() => Some(self.buffer.text_in(block.range())),
            _ => Some(self.buffer.lines().join("\n")),
        }
    }

    /// Put `text` where [`Editor::filter_input`] was read from: over the block, which stays
    /// marked on the new text, or over the whole buffer. **One** undo step either way, so a
    /// single `Ctrl+Z` brings back what the command was given.
    pub fn filter_replace(&mut self, text: &str) -> Result<(), EditorError> {
        if self.block.is_some_and(|b| !b.range().is_empty()) {
            self.replace_block(text)?;
            return Ok(());
        }
        let was = self.cursor;
        self.block = None;
        self.buffer.begin_group(was);
        let done = self
            .buffer
            .delete(Range::new(Position::default(), self.buffer.end()))
            .and_then(|_| self.buffer.insert(Position::default(), text));
        // The cursor stays on the line it was on, as near as the new text allows: a file
        // run through a formatter is still being read at the same place.
        self.cursor = self.clamp(was);
        self.buffer.end_group(self.cursor);
        done.map(|_| ()).map_err(Into::into)
    }

    /// `text` at the cursor, as one undo step, with the cursor left after it — a file or a
    /// command's output read in. A marked block is unmarked and left alone: reading
    /// something in is not typing over what is selected.
    pub fn insert_at_cursor(&mut self, text: &str) -> Result<(), EditorError> {
        self.block = None;
        self.insert_text(text)
    }

    /// The marked block as the bytes a save would write for it: the file's own end of line
    /// and code page, and no byte order mark — it is a piece of a file, not a file's start.
    /// `None` when nothing is marked.
    pub fn block_bytes(&self) -> Option<Vec<u8>> {
        let text = self.block_text().filter(|t| !t.is_empty())?;
        let text = text.replace('\n', self.buffer.eol.bytes());
        Some(self.buffer.codepage.encode(&text))
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
            SearchMode::Hex => hex_pattern(query, self.buffer.codepage)?,
            SearchMode::Plain => regex::escape(query),
        };
        regex::RegexBuilder::new(&pattern)
            .case_insensitive(self.search_case_insensitive)
            .build()
            .map_err(|e| EditorError::Search(e.to_string()))
    }

    /// Every hit of `re` in one line, as byte ranges, left to right and not overlapping.
    ///
    /// Whole words is decided here and not in the pattern. `\b…\b` round the pattern asks
    /// for a word character on one side of each end, so it never finds `++` or `-x` at all;
    /// what the box means is the other thing — that the hit is not *inside* a longer word,
    /// which is a question about the two characters next to it. A hit refused for its
    /// neighbours is tried again one character on, since a later start may stand alone.
    fn hits_in(&self, re: &Regex, text: &str) -> Vec<(usize, usize)> {
        let mut hits = Vec::new();
        let mut from = 0;
        // Where the last hit ended: an empty hit right against it is not another hit, which
        // is the rule the regex engine itself follows when it iterates.
        let mut last_end = None;
        while from <= text.len() {
            let Some(m) = re.find_at(text, from) else {
                break;
            };
            let one_on = |at: usize| at + text[at..].chars().next().map_or(1, char::len_utf8);
            let alone = !self.search_whole_words || stands_alone(text, m.start(), m.end());
            if !alone || (m.is_empty() && last_end == Some(m.start())) {
                from = one_on(m.start());
                continue;
            }
            hits.push((m.start(), m.end()));
            last_end = Some(m.end());
            from = if m.is_empty() {
                one_on(m.end())
            } else {
                m.end()
            };
        }
        hits
    }

    /// What goes in place of the hit at `hit` (bytes of `text`).
    ///
    /// A regular expression's replacement may refer to what its groups caught — `$1`,
    /// `${name}`, and `$$` for a dollar — which is the reason to search by pattern at all.
    /// A plain search puts in the text as typed, dollars included, and with *preserve style*
    /// gives it the capitals of what was found.
    fn replacement_for(&self, re: &Regex, text: &str, hit: (usize, usize), with: &str) -> String {
        if self.search_mode == SearchMode::Regex {
            // The groups of the very hit being replaced: the search is run again from its
            // start, and anything else it finds there is not what the user is looking at.
            return match re.captures_at(text, hit.0) {
                Some(caps) if caps.get(0).is_some_and(|m| (m.start(), m.end()) == hit) => {
                    let mut out = String::new();
                    caps.expand(with, &mut out);
                    out
                }
                _ => with.to_string(),
            };
        }
        if self.replace_preserve_style {
            in_the_style_of(&text[hit.0..hit.1], with)
        } else {
            with.to_string()
        }
    }

    /// Put the cursor on a hit and select it, the way every search leaves it.
    pub fn select_hit(&mut self, hit: Hit) {
        self.buffer.end_group(self.cursor);
        self.cursor = if self.cursor_after_match {
            Position::new(hit.line, hit.end)
        } else {
            Position::new(hit.line, hit.start)
        };
        self.block = Some(Block {
            anchor: Position::new(hit.line, hit.start),
            head: Position::new(hit.line, hit.end),
            kind: BlockKind::Stream,
        });
    }

    /// Find `query` from just after the cursor (or before it, going backwards).
    pub fn find(&mut self, query: &str, backwards: bool) -> Result<bool, EditorError> {
        let re = self.compile(query)?;
        self.last_search = Some(query.to_string());
        let lines = self.buffer.len();

        // A match *at* the cursor counts — unless the block already covers it, which means a
        // previous find put us here and the user asked for the next one.
        let on_a_hit = self.block.is_some_and(|b| {
            b.kind == BlockKind::Stream
                && (b.anchor == self.cursor || (self.cursor_after_match && b.head == self.cursor))
        });
        // With the cursor left after the match, "here" is where the match began.
        let here = match self.block {
            Some(b) if on_a_hit => b.anchor.col,
            _ => self.cursor.col,
        };

        for step in 0..=lines {
            let line = if backwards {
                (self.cursor.line + lines - step) % lines
            } else {
                (self.cursor.line + step) % lines
            };
            let text = self.buffer.line(line);
            let mut hits: Vec<(usize, usize)> = self
                .hits_in(&re, text)
                .into_iter()
                .map(|(start, end)| (text[..start].chars().count(), text[..end].chars().count()))
                .collect();
            if backwards {
                hits.reverse();
            }
            for (start, end) in hits {
                let is_current = line == self.cursor.line && step == 0;
                let ok = if backwards {
                    !is_current || end <= here
                } else if !is_current {
                    true
                } else if on_a_hit {
                    start > here
                } else {
                    start >= here
                };
                if ok {
                    self.select_hit(Hit { line, start, end });
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// `Ctrl+F3` and `Ctrl+Shift+F3`: the next, or the previous, place the word under the
    /// cursor is written — as a whole word and in the same capitals, whatever the search
    /// dialog was last set to, and without touching what `Shift+F7` will look for. It goes
    /// round the end of the file. `None` when the cursor is not on a word, `Some(false)`
    /// when this is the only place it is written.
    pub fn find_word(&mut self, backwards: bool) -> Option<bool> {
        let (start, end) = self.word_at_cursor()?;
        let here = self.cursor.line;
        let word: String = self
            .buffer
            .line(here)
            .chars()
            .skip(start)
            .take(end - start)
            .collect();
        let lines = self.buffer.len();
        // The cursor's own line is looked at twice: first for what lies beyond the word in
        // the direction asked for, and after every other line for what lies behind it.
        for step in 0..=lines {
            let line = if backwards {
                (here + lines - step % lines) % lines
            } else {
                (here + step) % lines
            };
            let text = self.buffer.line(line);
            let mut hits: Vec<usize> = text
                .match_indices(word.as_str())
                .filter(|(at, found)| stands_alone(text, *at, at + found.len()))
                .map(|(at, _)| text[..at].chars().count())
                .collect();
            if backwards {
                hits.reverse();
            }
            let found = hits.into_iter().find(|&at| match (step, backwards) {
                (0, false) => at > start,
                (0, true) => at < start,
                (n, false) if n == lines => at < start,
                (n, true) if n == lines => at > start,
                _ => true,
            });
            if let Some(at) = found {
                self.select_hit(Hit {
                    line,
                    start: at,
                    end: at + (end - start),
                });
                return Some(true);
            }
        }
        Some(false)
    }

    /// The first hit on every line that has one, top to bottom: what the list of all
    /// matches shows. One row per line, because a line is what the list has room to show
    /// and `Shift+F7` walks the rest of it.
    pub fn matching_lines(&mut self, query: &str) -> Result<Vec<Hit>, EditorError> {
        let re = self.compile(query)?;
        self.last_search = Some(query.to_string());
        let mut found = Vec::new();
        for line in 0..self.buffer.len() {
            let text = self.buffer.line(line);
            if let Some((start, end)) = self.hits_in(&re, text).into_iter().next() {
                found.push(Hit {
                    line,
                    start: text[..start].chars().count(),
                    end: text[..end].chars().count(),
                });
            }
        }
        Ok(found)
    }

    /// Replace the current match (if the block holds one) and find the next.
    pub fn replace_once(&mut self, query: &str, with: &str) -> Result<bool, EditorError> {
        let re = self.compile(query)?;
        self.last_search = Some(query.to_string());
        self.last_replace = Some(with.to_string());
        let found = match self.block {
            Some(block) if block.kind == BlockKind::Stream && !block.range().is_empty() => true,
            _ => self.find(query, false)?,
        };
        if !found {
            return Ok(false);
        }
        // What is selected is a hit on one line when a search put it there, and then its
        // groups and its capitals are known. A block the user marked by hand is replaced
        // too, as it always was, by the text as typed.
        let text = match self.block.map(|b| b.range()) {
            Some(range) if range.start.line == range.end.line => {
                let line = self.buffer.line(range.start.line);
                let byte = |col: usize| line.char_indices().nth(col).map_or(line.len(), |(i, _)| i);
                self.replacement_for(
                    &re,
                    line,
                    (byte(range.start.col), byte(range.end.col)),
                    with,
                )
            }
            _ => with.to_string(),
        };
        // One step, not the two that typing over a block is: what is undone is a replacement.
        if let Some(range) = self.block.take().map(|b| b.range()) {
            self.buffer.begin_group(self.cursor);
            self.buffer.delete(range)?;
            self.cursor = self.buffer.insert(range.start, &text)?;
            self.buffer.end_group(self.cursor);
        }
        // Leave the next one selected, so the same key goes through them one at a time.
        self.find(query, false)?;
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
            let hits = self.hits_in(&re, &text);
            if hits.is_empty() {
                continue;
            }
            let mut replaced = String::with_capacity(text.len());
            let mut done = 0;
            for hit in &hits {
                replaced.push_str(&text[done..hit.0]);
                replaced.push_str(&self.replacement_for(&re, &text, *hit, with));
                done = hit.1;
            }
            replaced.push_str(&text[done..]);
            self.buffer.set_line(line, &replaced)?;
            count += hits.len();
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

    // ── conversions ────────────────────────────────────────────────────────────────────────
    //
    // Each is one undo step and none touches the text: they change what the next save
    // writes. `Ok(false)` is "it is already written that way".

    /// End every line with `eol` from the next save on.
    pub fn convert_eol(&mut self, eol: crate::Eol) -> Result<bool, EditorError> {
        self.burst = None;
        Ok(self.buffer.convert_eol(eol, self.cursor)?)
    }

    /// Write the byte order mark, or leave it out.
    pub fn convert_bom(&mut self, bom: bool) -> Result<bool, EditorError> {
        self.burst = None;
        Ok(self.buffer.convert_bom(bom, self.cursor)?)
    }

    /// Save in another code page from now on.
    pub fn convert_codepage(&mut self, codepage: fcmd_view::Codepage) -> Result<bool, EditorError> {
        self.burst = None;
        Ok(self.buffer.convert_codepage(codepage, self.cursor)?)
    }

    /// Can `codepage` hold every character of the text? Asked before converting to it, so
    /// that a letter is never turned into a question mark by a save.
    pub fn fits_codepage(&self, codepage: fcmd_view::Codepage) -> bool {
        self.buffer
            .lines()
            .iter()
            .all(|line| codepage.can_encode(line))
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
        let mut buffer = Buffer::open(path.as_path())?;
        // The depth is a setting of the editor, not of the file that was just read again.
        buffer.set_undo_depth(self.buffer.undo_depth());
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
        // Said only when it is there: most files have none, and the ones that do are the
        // ones somebody is about to ask why a script will not start.
        if self.buffer.bom {
            text.push_str(" BOM");
        }
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

/// What a command printed, made ready to go into a buffer: its line endings turned into the
/// buffer's own `\n`, and the newline a program ends its last line with taken off when
/// `keep_last_newline` is false — which is when what it was given did not end with one, or
/// when the text goes in at the cursor, where that newline would break the line in two.
pub fn settle_output(output: &str, keep_last_newline: bool) -> String {
    let mut text = output.replace("\r\n", "\n");
    if !keep_last_newline && text.ends_with('\n') {
        text.pop();
    }
    text
}

/// Is the text between `start` and `end` (bytes) a word of its own — with no letter, digit
/// or underscore right before it or right after it?
fn stands_alone(text: &str, start: usize, end: usize) -> bool {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    !text[..start].chars().next_back().is_some_and(is_word)
        && !text[end..].chars().next().is_some_and(is_word)
}

/// FAR's *preserve style*: `with`, wearing the capitals of `found`.
///
/// Three shapes are recognised and nothing cleverer: all lower case, all upper case, and a
/// capital first with the rest lower. Anything else — `camelCase`, `mIxEd` — has no rule
/// that would not be a guess, so the replacement goes in as it was typed. One capital
/// letter on its own reads as "a capital first", not as "all capitals": replacing `x` by
/// `value` should make `X` into `Value`, not `VALUE`.
fn in_the_style_of(found: &str, with: &str) -> String {
    let cased: Vec<char> = found
        .chars()
        .filter(|c| c.is_lowercase() || c.is_uppercase())
        .collect();
    let upper = cased.iter().filter(|c| c.is_uppercase()).count();
    if cased.is_empty() {
        with.to_string()
    } else if upper == 0 {
        with.to_lowercase()
    } else if upper == cased.len() && cased.len() > 1 {
        with.to_uppercase()
    } else if upper == 1 && cased[0].is_uppercase() {
        let mut chars = with.chars();
        match chars.next() {
            Some(first) => first.to_uppercase().chain(chars).collect(),
            None => String::new(),
        }
    } else {
        with.to_string()
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

/// A word completion between two presses of `Alt+/`.
#[derive(Debug, Clone)]
struct Completion {
    line: usize,
    /// Where the word starts, in characters.
    start: usize,
    /// What was typed before the first press.
    prefix: String,
    /// What is in the buffer now, from `start`.
    shown: String,
    candidates: Vec<String>,
    next: usize,
}

/// A hex search in the editor: the bytes as the file's own code page reads them.
///
/// The editor holds text, not bytes, so `4D 5A` is looked for as the characters those bytes
/// are in the file's code page — `C3 A9` is `é` in a UTF-8 file and `Ã©` in a CP1252 one,
/// which is what a hex search of that file on disk would find. `??` stands for any one
/// character: in a code page of one byte per character that is exactly the viewer's rule, in
/// UTF-8 it is a character rather than a byte, which is as near as text can come.
fn hex_pattern(query: &str, codepage: fcmd_view::Codepage) -> Result<String, EditorError> {
    let mut pattern = String::new();
    let mut run: Vec<u8> = Vec::new();
    let mut any = false;
    for token in query.split_whitespace() {
        for pair in token.as_bytes().chunks(2) {
            let text = std::str::from_utf8(pair).unwrap_or("");
            if text == "??" {
                pattern.push_str(&regex::escape(&codepage.decode(&run)));
                run.clear();
                pattern.push('.');
                any = true;
                continue;
            }
            let byte = u8::from_str_radix(text, 16)
                .map_err(|_| EditorError::Search(format!("`{text}` is not a hex byte")))?;
            run.push(byte);
            any = true;
        }
    }
    if !any {
        return Err(EditorError::Search("empty search".to_string()));
    }
    pattern.push_str(&regex::escape(&codepage.decode(&run)));
    Ok(pattern)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::buffer::Eol;

    fn pascal(text: &str) -> Editor {
        let mut editor = Editor::new(Buffer::from_text(text), None);
        editor.highlight = crate::Highlight::for_name("Unit1.pas");
        editor
    }

    #[test]
    fn the_guides_of_a_window_know_the_blocks_opened_above_it() {
        let mut e = pascal("begin\n  try\n    x;\n  finally\n    y;\n  end;\nend;\n");
        // Lines 2..5 only: `begin` and `try` are above the window and still count.
        let g = e.block_guides(2..5).expect("Pascal has guides").to_vec();
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].through, [0, 2]);
        assert_eq!(g[1].through, [0], "`finally` stands in try's column");
        assert_eq!(g[2].through, [0, 2]);
        // A window past the end is cut to the buffer, not padded.
        assert_eq!(e.block_guides(5..99).unwrap().len(), 2);
    }

    #[test]
    fn the_guides_follow_an_edit_in_or_above_the_window_and_ignore_one_below() {
        let mut e = pascal("begin\n  x;\n  y;\nend;\nz;\n");
        assert_eq!(e.block_guides(1..3).unwrap()[0].through, [0]);
        // Below the window: the answer that was kept is still the answer.
        e.buffer.insert(Position::new(4, 0), "w").unwrap();
        assert!(e.guides.is_some(), "an edit below threw the guides away");
        assert_eq!(e.block_guides(1..3).unwrap()[0].through, [0]);
        // Above it: `begin` becomes a comment, and the block is no longer one.
        e.buffer.insert(Position::new(0, 0), "// ").unwrap();
        assert!(e.block_guides(1..3).unwrap()[0].through.is_empty());
        e.undo();
        assert_eq!(e.block_guides(1..3).unwrap()[0].through, [0]);
    }

    #[test]
    fn the_guides_stand_in_the_columns_tabs_are_drawn_in() {
        let mut e = pascal("begin\n\ttry\n\t\tx;\n\tend;\nend;\n");
        e.tab_width = 4;
        assert_eq!(e.block_guides(2..3).unwrap()[0].through, [0, 4]);
        // Another tab width is another answer, with no edit in between.
        e.tab_width = 8;
        assert_eq!(e.block_guides(2..3).unwrap()[0].through, [0, 8]);
    }

    #[test]
    fn a_comment_opened_above_the_window_keeps_its_words_from_opening_blocks() {
        let mut e = pascal("{ a note\nbegin in the note\n}\nbegin\n  x;\nend;\n");
        let g = e.block_guides(4..5).unwrap();
        assert_eq!(g[0].through, [0], "one block, the real one");
    }

    #[test]
    fn a_language_without_block_words_has_no_guides() {
        let mut e = Editor::new(Buffer::from_text("fn main() {\n}\n"), None);
        e.highlight = crate::Highlight::for_name("main.rs");
        assert!(e.block_guides(0..2).is_none());
        let mut plain = Editor::new(Buffer::from_text("begin\nend\n"), None);
        assert!(plain.block_guides(0..2).is_none(), "no syntax, no guides");
    }

    #[test]
    fn lines_matching_a_pattern_go_or_stay_in_one_undo() {
        let mut e = editor("keep 1\ndrop a\nkeep 2\ndrop b");
        let re = Regex::new("^drop").unwrap();
        assert_eq!(e.delete_matching(&re, false).unwrap(), 2);
        assert_eq!(e.buffer.len(), 2);
        assert_eq!(e.buffer.line(1), "keep 2");
        assert!(e.undo());
        assert_eq!(e.buffer.len(), 4, "one undo puts them all back");
        assert_eq!(
            e.delete_matching(&re, true).unwrap(),
            2,
            "keep only what matches"
        );
        assert_eq!(e.buffer.line(0), "drop a");
    }

    #[test]
    fn alt_slash_completes_with_the_nearest_word_and_cycles() {
        let mut e = editor("alphabet\nbeta\nal\nalpine almond\n");
        e.cursor = Position::new(2, 2);
        assert!(e.complete_word().unwrap());
        // The nearest line with a word that fits: the one below, then two lines up.
        assert_eq!(e.buffer.line(2), "alpine");
        assert_eq!(e.cursor, Position::new(2, 6));
        e.complete_word().unwrap();
        assert_eq!(e.buffer.line(2), "almond");
        e.complete_word().unwrap();
        assert_eq!(e.buffer.line(2), "alphabet");
        e.complete_word().unwrap();
        assert_eq!(e.buffer.line(2), "al", "and back to what was typed");
        e.cursor = Position::new(1, 4);
        assert!(
            !e.complete_word().unwrap(),
            "beta: nothing longer starts like it"
        );
    }

    fn editor(text: &str) -> Editor {
        Editor::new(Buffer::from_text(text), None)
    }

    /// The four cases, with what Unicode says about a letter: a sharp s grows, an accent
    /// keeps its letter, and a digit or a sign is left as it was.
    #[test]
    fn the_four_cases_follow_unicode_and_leave_what_has_no_case() {
        assert_eq!(
            Case::Upper.apply("stra\u{df}e 1, \u{e9}t\u{e9}"),
            "STRASSE 1, \u{c9}T\u{c9}"
        );
        assert_eq!(
            Case::Lower.apply("\u{c9}T\u{c9} ABC_9"),
            "\u{e9}t\u{e9} abc_9"
        );
        assert_eq!(
            Case::Title.apply("the QUICK brown-fox, don't stop_here"),
            "The Quick Brown-Fox, Don't Stop_Here"
        );
        assert_eq!(Case::Toggle.apply("Hello World 42"), "hELLO wORLD 42");
        for case in Case::ALL {
            assert_eq!(Case::parse(case.as_str()), Some(case));
        }
        assert_eq!(Case::parse("sideways"), None);
    }

    /// On a stream block the block stays marked and the cursor stays put; on a column block
    /// only the columns marked change; with nothing marked it is the word under the cursor,
    /// or the one just before it. Each is one undo step, and nothing to act on says so.
    #[test]
    fn the_case_of_a_block_a_column_or_the_word_under_the_cursor_changes() {
        let mut e = editor("alpha beta\ngamma delta");
        e.move_to(Position::new(0, 6), false, BlockKind::Stream);
        e.move_to(Position::new(1, 5), true, BlockKind::Stream);
        assert!(e.change_case(Case::Upper).unwrap());
        assert_eq!(e.buffer.to_text(), "alpha BETA\nGAMMA delta");
        assert_eq!(
            e.block_text().as_deref(),
            Some("BETA\nGAMMA"),
            "still marked"
        );
        assert_eq!(e.cursor, Position::new(1, 5));
        assert!(e.change_case(Case::Toggle).unwrap());
        assert_eq!(e.buffer.to_text(), "alpha beta\ngamma delta");
        assert!(e.undo() && e.undo());
        assert_eq!(e.buffer.to_text(), "alpha beta\ngamma delta", "a step each");
        assert!(!e.undo(), "and no more than that");

        // A column: the same two columns of every line, and a line too short is left alone.
        let mut e = editor("abcdef\nab\nabcdef");
        e.move_to(Position::new(0, 2), false, BlockKind::Column);
        e.move_to(Position::new(2, 4), true, BlockKind::Column);
        assert!(e.change_case(Case::Upper).unwrap());
        assert_eq!(e.buffer.to_text(), "abCDef\nab\nabCDef");
        assert!(e.undo());
        assert_eq!(
            e.buffer.to_text(),
            "abcdef\nab\nabcdef",
            "one step for all the lines"
        );

        // Nothing marked: the word the cursor is in, or stands right after.
        let mut e = editor("say hello_there now");
        e.cursor = Position::new(0, 7);
        assert_eq!(e.word_at_cursor(), Some((4, 15)));
        assert!(e.change_case(Case::Title).unwrap());
        assert_eq!(e.buffer.to_text(), "say Hello_There now");
        assert_eq!(e.cursor, Position::new(0, 7), "the cursor has not moved");
        e.cursor = Position::new(0, 3);
        assert!(e.change_case(Case::Upper).unwrap(), "right after `say`");
        assert_eq!(e.buffer.to_text(), "SAY Hello_There now");
        let mut e = editor("a  b");
        e.cursor = Position::new(0, 2);
        assert_eq!(e.word_at_cursor(), None);
        assert!(!e.change_case(Case::Upper).unwrap(), "between two blanks");
        assert_eq!(e.buffer.to_text(), "a  b");
    }

    /// A syntax chosen by hand colours a file whose name says nothing, and taking it away
    /// again leaves plain text.
    #[test]
    fn a_syntax_chosen_by_hand_colours_a_file_whose_name_says_nothing() {
        let mut e = editor("fn main() {} // hi");
        assert!(e.highlight.language().is_none());
        assert!(e.spans(0).is_empty());
        e.set_syntax(fcmd_syntax::of_language("Rust"));
        assert_eq!(e.highlight.language(), Some("Rust"));
        assert!(!e.spans(0).is_empty(), "now it is read as Rust");
        e.set_syntax(None);
        assert!(e.spans(0).is_empty());
    }

    /// The word under the cursor is looked for as a word and in its own capitals: `set` is
    /// not found inside `offset` nor as `Set`, the search goes round the end of the file in
    /// both directions, and the dialog's own search is left as it was.
    #[test]
    fn the_word_under_the_cursor_is_found_again_as_a_whole_word() {
        let mut e = editor("set a\noffset set Set\nx = set;\nlast");
        e.last_search = Some("kept".into());
        e.cursor = Position::new(0, 1);
        assert_eq!(e.find_word(false), Some(true));
        assert_eq!(e.cursor, Position::new(1, 7), "not the one inside `offset`");
        assert_eq!(e.block_text().as_deref(), Some("set"), "and it is marked");
        assert_eq!(e.find_word(false), Some(true));
        assert_eq!(e.cursor, Position::new(2, 4), "not `Set` either");
        assert_eq!(e.find_word(false), Some(true));
        assert_eq!(e.cursor, Position::new(0, 0), "round the end of the file");
        assert_eq!(e.find_word(true), Some(true));
        assert_eq!(e.cursor, Position::new(2, 4), "and back round the start");
        assert_eq!(e.find_word(true), Some(true));
        assert_eq!(e.cursor, Position::new(1, 7));
        assert_eq!(e.last_search.as_deref(), Some("kept"));

        // Two on one line, from the cursor left after the first by the setting.
        let mut e = editor("ab ab");
        e.cursor_after_match = true;
        assert_eq!(e.find_word(false), Some(true));
        assert_eq!(e.cursor, Position::new(0, 5), "after the second");
        assert_eq!(e.find_word(false), Some(true));
        assert_eq!(e.cursor, Position::new(0, 2), "and round to the first");

        let mut e = editor("only once\n  ");
        assert_eq!(e.find_word(false), Some(false), "nowhere else");
        assert_eq!(e.cursor, Position::new(0, 0), "so the cursor stays");
        e.cursor = Position::new(1, 1);
        assert_eq!(e.find_word(false), None, "no word here");
    }

    /// A paragraph is laid out again to a width: as many words to a line as fit, the first
    /// line's indent on every line, the paragraphs either side left alone. A block may hold
    /// several, and the empty lines between them stay.
    #[test]
    fn a_paragraph_is_laid_out_again_to_a_width_and_keeps_its_indent() {
        let mut e = editor("before\n\n  one two three\nfour\n  five   six\n\nafter stays as it is");
        e.cursor = Position::new(3, 0);
        assert!(e.reflow(12).unwrap());
        assert_eq!(
            e.buffer.to_text(),
            "before\n\n  one two\n  three four\n  five six\n\nafter stays as it is"
        );
        assert_eq!(e.cursor, Position::new(2, 0));
        assert!(e.undo());
        assert_eq!(
            e.buffer.to_text(),
            "before\n\n  one two three\nfour\n  five   six\n\nafter stays as it is",
            "one step"
        );
        // Wide enough, it is one line; a word wider than the width has a line to itself.
        e.cursor = Position::new(2, 0);
        assert!(e.reflow(80).unwrap());
        assert_eq!(e.buffer.line(2), "  one two three four five six");
        let mut e = editor("a extraordinarily b");
        assert!(e.reflow(5).unwrap());
        assert_eq!(e.buffer.to_text(), "a\nextraordinarily\nb");

        // On an empty line with nothing marked there is no paragraph.
        let mut e = editor("a b\n\nc d");
        e.cursor = Position::new(1, 0);
        assert!(!e.reflow(1).unwrap());
        assert_eq!(e.buffer.to_text(), "a b\n\nc d");
        // A block over both: each is laid out, and the line between them is kept.
        e.select_all();
        assert!(e.reflow(1).unwrap());
        assert_eq!(e.buffer.to_text(), "a\nb\n\nc\nd");
        assert!(e.block.is_none());
        assert!(e.undo());
        assert_eq!(
            e.buffer.to_text(),
            "a b\n\nc d",
            "one step for the whole block"
        );
    }

    /// What a filter is given and where its answer goes: the block when one is marked, and
    /// the block stays marked on the new text; the whole text when none is. Either way it is
    /// one step to undo, and a column block is refused before anything runs.
    #[test]
    fn a_filter_takes_the_block_or_the_whole_text_and_is_one_undo_step() {
        let mut e = editor("c\nb\na\ntail");
        assert_eq!(e.filter_input().as_deref(), Some("c\nb\na\ntail"));
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(3, 0), true, BlockKind::Stream);
        assert_eq!(e.filter_input().as_deref(), Some("c\nb\na\n"));
        e.filter_replace("a\nb\nc\n").unwrap();
        assert_eq!(e.buffer.to_text(), "a\nb\nc\ntail");
        assert_eq!(e.block_text().as_deref(), Some("a\nb\nc\n"), "still marked");
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), "c\nb\na\ntail", "one step back");

        // Nothing marked: the whole text, and the cursor kept on its line.
        e.clear_block();
        e.cursor = Position::new(2, 1);
        e.filter_replace("one\ntwo").unwrap();
        assert_eq!(e.buffer.to_text(), "one\ntwo");
        assert_eq!(
            e.cursor,
            Position::new(1, 1),
            "clamped onto the text there is"
        );
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), "c\nb\na\ntail");

        e.move_to(Position::new(0, 0), false, BlockKind::Column);
        e.move_to(Position::new(1, 1), true, BlockKind::Column);
        assert_eq!(
            e.filter_input(),
            None,
            "a column has no shape to come back into"
        );
    }

    /// A command ends its last line with a newline whether or not what it was given did.
    #[test]
    fn a_commands_output_loses_the_newline_it_was_not_given() {
        assert_eq!(settle_output("a\r\nb\r\n", true), "a\nb\n");
        assert_eq!(settle_output("a\nb\n", false), "a\nb");
        assert_eq!(settle_output("a\n\n", false), "a\n", "only the last one");
        assert_eq!(settle_output("", false), "");
    }

    /// Text read in at the cursor leaves a marked block alone and is one step to undo.
    #[test]
    fn text_read_in_at_the_cursor_does_not_type_over_the_block() {
        let mut e = editor("alpha beta");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(0, 5), true, BlockKind::Stream);
        e.insert_at_cursor("X\nY").unwrap();
        assert_eq!(e.buffer.to_text(), "alphaX\nY beta");
        assert_eq!(e.cursor, Position::new(1, 1));
        assert!(e.block.is_none());
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), "alpha beta");
    }

    /// A block written out is written the way the file is: its end of line and its code
    /// page, and never a byte order mark.
    #[test]
    fn a_block_is_written_in_the_files_own_end_of_line_and_code_page() {
        let mut e = editor("caf\u{e9}\nbar\nbaz");
        assert_eq!(e.block_bytes(), None, "nothing marked");
        e.buffer.eol = crate::Eol::Crlf;
        e.buffer.codepage = fcmd_view::Codepage::Cp1252;
        e.buffer.bom = true;
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(2, 0), true, BlockKind::Stream);
        assert_eq!(e.block_bytes().as_deref(), Some(&b"caf\xe9\r\nbar\r\n"[..]));
    }

    #[test]
    fn a_bracket_finds_its_match_across_lines_and_back() {
        let mut e = editor("fn x(a: [u8; 2]) {\n    if (b) { c }\n}");
        e.cursor = Position::new(0, 17); // the `{` that opens the body
        assert!(e.match_bracket());
        assert_eq!(e.cursor, Position::new(2, 0));
        assert!(e.match_bracket(), "and back from the closing one");
        assert_eq!(e.cursor, Position::new(0, 17));
        // Just after a bracket counts too, which is where typing leaves the cursor.
        e.cursor = Position::new(0, 16); // after `)`
        assert!(e.match_bracket());
        assert_eq!(e.cursor, Position::new(0, 4));
        // The nested pair inside is skipped over, not taken.
        e.cursor = Position::new(0, 8); // `[`
        assert!(e.match_bracket());
        assert_eq!(e.cursor, Position::new(0, 14));
        e.cursor = Position::new(1, 1);
        assert!(!e.match_bracket(), "no bracket here");
        let mut open = editor("(never closed");
        assert!(!open.match_bracket());
    }

    #[test]
    fn joining_takes_the_indent_away_and_leaves_one_space() {
        let mut e = editor("let x =\n        42;\nnext");
        assert!(e.join_line().unwrap());
        assert_eq!(e.buffer.line(0), "let x = 42;");
        assert_eq!(e.cursor, Position::new(0, 8));
        e.undo();
        assert_eq!(e.buffer.line(1), "        42;", "one undo puts it back");
        e.cursor = Position::new(2, 0);
        assert!(!e.join_line().unwrap(), "nothing below the last line");
    }

    #[test]
    fn sorting_orders_the_lines_of_the_block_and_nothing_else() {
        let mut e = editor("keep\ncherry\napple\nbanana\nalso keep");
        e.block = Some(Block {
            anchor: Position::new(1, 0),
            head: Position::new(4, 0),
            kind: BlockKind::Stream,
        });
        assert!(e.sort_lines().unwrap());
        assert_eq!(
            e.buffer.lines(),
            &["keep", "apple", "banana", "cherry", "also keep"]
        );
        e.undo();
        assert_eq!(e.buffer.line(1), "cherry", "one undo for the whole sort");
        e.block = None;
        assert!(!e.sort_lines().unwrap(), "a sort needs a block");
    }

    fn block_of(e: &mut Editor, top: usize, below: usize) {
        e.block = Some(Block {
            anchor: Position::new(top, 0),
            head: Position::new(below, 0),
            kind: BlockKind::Stream,
        });
    }

    #[test]
    fn unique_keeps_the_first_of_each_line_and_one_undo_brings_them_back() {
        let mut e = editor("keep\nb\na\nb\nc\na\nalso keep");
        block_of(&mut e, 1, 6);
        assert_eq!(e.unique_lines().unwrap(), Some(2));
        assert_eq!(e.buffer.lines(), &["keep", "b", "a", "c", "also keep"]);
        // The block now covers what is left, so there is nothing more to take.
        assert_eq!(e.unique_lines().unwrap(), Some(0));
        e.undo();
        assert_eq!(
            e.buffer.lines(),
            &["keep", "b", "a", "b", "c", "a", "also keep"]
        );
        e.block = None;
        assert_eq!(e.unique_lines().unwrap(), None, "it needs a block");
    }

    #[test]
    fn reverse_turns_the_block_upside_down() {
        let mut e = editor("one\ntwo\nthree\nrest");
        block_of(&mut e, 0, 3);
        assert!(e.reverse_lines().unwrap());
        assert_eq!(e.buffer.lines(), &["three", "two", "one", "rest"]);
        e.undo();
        assert_eq!(e.buffer.line(0), "one");
    }

    #[test]
    fn numbers_line_up_however_many_lines_there_are() {
        let text: Vec<String> = (0..10).map(|i| format!("line {i}")).collect();
        let mut e = editor(&text.join("\n"));
        block_of(&mut e, 0, 9);
        // The last line is not in the block: it ends at column 0 of it.
        assert!(e.number_lines().unwrap());
        assert_eq!(e.buffer.line(0), "1 line 0");
        assert_eq!(e.buffer.line(8), "9 line 8");
        assert_eq!(e.buffer.line(9), "line 9");
        let mut e = editor(&text.join("\n"));
        e.block = Some(Block {
            anchor: Position::new(0, 0),
            head: Position::new(9, 6),
            kind: BlockKind::Stream,
        });
        e.number_lines().unwrap();
        assert_eq!(e.buffer.line(0), " 1 line 0", "padded to the width of 10");
        assert_eq!(e.buffer.line(9), "10 line 9");
    }

    #[test]
    fn trailing_blanks_go_and_a_clean_file_is_left_alone() {
        let mut e = editor("a  \nb\t\nc");
        assert_eq!(e.trim_trailing().unwrap(), 2);
        assert_eq!(e.buffer.lines(), &["a", "b", "c"]);
        assert_eq!(e.trim_trailing().unwrap(), 0);
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

    /// The search dialog offers hex in the editor too, and it used to look for the text
    /// `4D 5A` itself. The bytes are the characters they are in the file's code page.
    #[test]
    fn a_hex_search_finds_the_bytes_as_the_file_reads_them() {
        let mut e = editor("MZ header\ncaf\u{e9} au lait\n");
        e.search_mode = SearchMode::Hex;
        assert!(e.find("4D 5A", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 0));
        // `é` is two bytes in UTF-8, the editor's page here.
        e.move_file_start(false);
        assert!(e.find("66 C3A9", false).unwrap());
        assert_eq!(e.cursor.line, 1);
        // `??` is any one character.
        e.move_file_start(false);
        assert!(e.find("68 ?? 61", false).unwrap(), "h?a in `header`");
        // Not hex at all is said, not searched for.
        assert!(e.find("4G", false).is_err());
        assert!(e.find("   ", false).is_err());
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
    fn whole_words_refuses_a_hit_inside_a_longer_word() {
        let mut e = editor("offset = set + reset_set; set\n");
        e.search_whole_words = true;
        assert!(e.find("set", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 9), "not the one inside offset");
        assert!(e.find("set", false).unwrap());
        assert_eq!(
            e.cursor,
            Position::new(0, 26),
            "nor the one after an underscore"
        );
        e.move_file_start(false);
        assert_eq!(e.replace_all("set", "put").unwrap(), 2);
        assert_eq!(e.buffer.line(0), "offset = put + reset_set; put");
    }

    #[test]
    fn whole_words_also_finds_what_is_not_made_of_word_characters() {
        // `\b++\b` finds nothing at all; the box means "not inside a longer word".
        let mut e = editor("a ++ b; c++x; é+é\n");
        e.search_whole_words = true;
        assert_eq!(e.replace_all("++", "--").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "a -- b; c++x; é+é");
        // An accented letter is a letter: `+` between two of them is not alone.
        assert_eq!(e.replace_all("+", "#").unwrap(), 0);
    }

    #[test]
    fn a_hit_refused_for_its_neighbours_does_not_hide_one_that_overlaps_it() {
        let mut e = editor("xab ab\n");
        e.search_mode = SearchMode::Regex;
        e.search_whole_words = true;
        // The `ab` inside `xab` is refused, and the search goes on to the one that is alone.
        assert_eq!(e.replace_all("ab", "-").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "xab -");
        // `a?b` first finds the `ab` inside `aab`; refusing it must not skip to the end.
        let mut e = editor("aab ab\n");
        e.search_mode = SearchMode::Regex;
        e.search_whole_words = true;
        assert_eq!(e.replace_all("a?b", "-").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "aab -");
    }

    #[test]
    fn a_pattern_replacement_refers_to_what_its_groups_caught() {
        let mut e = editor("width=10 height=20\n");
        e.search_mode = SearchMode::Regex;
        assert_eq!(
            e.replace_all(r"(\w+)=(?P<n>\d+)", "${n}:$1 ($$)").unwrap(),
            2
        );
        assert_eq!(e.buffer.line(0), "10:width ($) 20:height ($)");
    }

    #[test]
    fn replacing_one_hit_of_a_pattern_uses_that_hits_groups_and_selects_the_next() {
        let mut e = editor("a1 b2 c3\n");
        e.search_mode = SearchMode::Regex;
        assert!(e.replace_once(r"([a-z])(\d)", "$2$1").unwrap());
        assert_eq!(e.buffer.line(0), "1a b2 c3");
        let block = e.block.expect("the next hit is selected");
        assert_eq!((block.anchor.col, block.head.col), (3, 5));
        assert!(e.replace_once(r"([a-z])(\d)", "$2$1").unwrap());
        assert_eq!(e.buffer.line(0), "1a 2b c3");
        // One step each, so one undo takes back one replacement.
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "1a b2 c3");
    }

    #[test]
    fn a_plain_replacement_keeps_its_dollars() {
        let mut e = editor("price cost\n");
        assert_eq!(e.replace_all("price", "$1 ${x} $$").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "$1 ${x} $$ cost");
        assert!(e.replace_once("cost", "$0").unwrap());
        assert_eq!(e.buffer.line(0), "$1 ${x} $$ $0");
    }

    #[test]
    fn preserve_style_gives_the_replacement_the_capitals_of_what_it_replaces() {
        let mut e = editor("foo Foo FOO fOO F\n");
        e.replace_preserve_style = true;
        assert_eq!(e.replace_all("foo", "bar").unwrap(), 4);
        // All lower, a capital first, all upper — and a shape with no rule, as typed.
        assert_eq!(e.buffer.line(0), "bar Bar BAR bar F");
        // One capital letter on its own is a capital first, not shouting.
        assert_eq!(e.replace_all("f", "value").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "bar Bar BAR bar Value");

        // As typed means as typed: capitals in the replacement survive a mixed match.
        let mut e = editor("fooBar foobar\n");
        e.replace_preserve_style = true;
        assert_eq!(e.replace_all("foobar", "newName").unwrap(), 2);
        assert_eq!(e.buffer.line(0), "newName newname");

        // Off, which is the default, nothing is touched.
        let mut e = editor("Foo\n");
        assert_eq!(e.replace_all("foo", "bar").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "bar");
    }

    #[test]
    fn preserve_style_is_for_plain_text_and_leaves_a_pattern_alone() {
        let mut e = editor("Foo\n");
        e.replace_preserve_style = true;
        e.search_mode = SearchMode::Regex;
        assert_eq!(e.replace_all("f(o+)", "b$1").unwrap(), 1);
        assert_eq!(e.buffer.line(0), "boo");
    }

    #[test]
    fn the_list_of_matches_has_one_row_for_each_line_that_has_one() {
        let mut e = editor("alpha beta\ngamma\nbeta beta\nBETA\n");
        let hits = e.matching_lines("beta").unwrap();
        assert_eq!(
            hits,
            vec![
                Hit {
                    line: 0,
                    start: 6,
                    end: 10
                },
                Hit {
                    line: 2,
                    start: 0,
                    end: 4
                },
                Hit {
                    line: 3,
                    start: 0,
                    end: 4
                },
            ]
        );
        e.select_hit(hits[1]);
        assert_eq!(e.cursor, Position::new(2, 0));
        // The list is a search like any other: the next one goes on from the row chosen.
        assert!(e.find("beta", false).unwrap());
        assert_eq!(e.cursor, Position::new(2, 5));
        assert!(e.matching_lines("zeta").unwrap().is_empty());
        assert!(
            e.matching_lines("(unclosed").is_ok(),
            "plain text, not a pattern"
        );
        e.search_mode = SearchMode::Regex;
        assert!(e.matching_lines("(unclosed").is_err());
    }

    #[test]
    fn an_empty_hit_can_be_replaced_too() {
        let mut e = editor("one\ntwo\n");
        e.search_mode = SearchMode::Regex;
        assert_eq!(e.replace_all("^", "> ").unwrap(), 2);
        assert_eq!(e.buffer.to_text(), "> one\n> two\n");
        assert_eq!(e.replace_all("o*", "-").unwrap(), 10);
        assert_eq!(e.buffer.line(0), "->- -n-e-");
    }

    #[test]
    fn searching_again_goes_on_from_a_hit_the_cursor_was_left_after() {
        let mut e = editor("ab ab ab\n");
        e.cursor_after_match = true;
        assert!(e.find("ab", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 2));
        assert!(e.find("ab", false).unwrap());
        assert_eq!(e.cursor, Position::new(0, 5));
        assert!(e.find("ab", true).unwrap());
        assert_eq!(e.cursor, Position::new(0, 2), "and back to the one before");
    }

    fn type_in(e: &mut Editor, text: &str) {
        for c in text.chars() {
            e.insert_char(c).unwrap();
        }
    }

    #[test]
    fn a_word_typed_is_undone_in_one_step() {
        let mut e = editor("\n");
        type_in(&mut e, "hello world");
        assert_eq!(e.buffer.line(0), "hello world");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "");
        assert_eq!(e.cursor, Position::new(0, 0));
        assert!(!e.undo(), "it was one step");
        assert!(e.redo());
        assert_eq!(e.buffer.line(0), "hello world");
        assert_eq!(e.cursor, Position::new(0, 11));
    }

    #[test]
    fn typing_and_rubbing_out_are_two_steps_and_so_are_backspace_and_del() {
        let mut e = editor("\n");
        type_in(&mut e, "abcdef");
        e.backspace().unwrap();
        e.backspace().unwrap();
        assert_eq!(e.buffer.line(0), "abcd");
        e.move_line_start(false);
        e.delete_forward().unwrap();
        e.delete_forward().unwrap();
        assert_eq!(e.buffer.line(0), "cd");

        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "abcd", "both Del went back together");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "abcdef", "and both Backspace");
        assert_eq!(e.cursor, Position::new(0, 6));
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "");
    }

    #[test]
    fn backspace_straight_after_typing_does_not_join_the_typing() {
        let mut e = editor("\n");
        type_in(&mut e, "abc");
        e.backspace().unwrap();
        type_in(&mut e, "d");
        assert_eq!(e.buffer.line(0), "abd");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "ab");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "abc");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "");
    }

    #[test]
    fn moving_away_and_back_breaks_a_burst() {
        let mut e = editor("\n");
        type_in(&mut e, "ab");
        e.move_by(0, -1, false, BlockKind::Stream);
        e.move_by(0, 1, false, BlockKind::Stream);
        type_in(&mut e, "cd");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "ab");
        assert!(e.undo());
        assert_eq!(e.buffer.line(0), "");
    }

    #[test]
    fn a_new_line_and_a_command_each_break_a_burst() {
        let mut e = editor("\n");
        type_in(&mut e, "one");
        e.insert_newline().unwrap();
        type_in(&mut e, "two");
        e.duplicate_line().unwrap();
        type_in(&mut e, "!");
        assert_eq!(e.buffer.to_text(), "one\ntwo\ntwo!\n");
        // Back through them in order: the mark, the copy, the word, the Enter, the word.
        for expected in ["one\ntwo\ntwo\n", "one\ntwo\n", "one\n\n", "one\n", "\n"] {
            assert!(e.undo());
            assert_eq!(e.buffer.to_text(), expected);
        }
        assert!(!e.undo());
    }

    #[test]
    fn a_key_refused_by_a_read_only_buffer_leaves_no_burst_behind() {
        let mut e = editor("x\ny\n");
        e.buffer.read_only = true;
        assert!(e.insert_char('a').is_err());
        e.buffer.read_only = false;
        e.delete_line().unwrap();
        type_in(&mut e, "b");
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), "y\n", "the typing alone");
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), "x\ny\n");
    }

    #[test]
    fn reloading_keeps_the_depth_the_editor_was_given() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.txt");
        std::fs::write(&path, "one\n").unwrap();
        let mut e = Editor::open(&path).unwrap();
        e.buffer.set_undo_depth(7);
        e.reload().unwrap();
        assert_eq!(e.buffer.undo_depth(), 7);
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

    /// A block replaced by a text of another length: the new text stands where the old one
    /// was, is the block now, and one undo — not one for the taking out and one for the
    /// putting in — brings the old text back.
    #[test]
    fn a_block_is_replaced_as_one_step_that_one_undo_takes_back() {
        let before = "alpha\nbeta\ngamma\ndelta\n";
        let mut e = editor(before);
        // A change of its own first, so the undo has something behind it to stop at.
        e.cursor = Position::new(0, 5);
        e.insert_text("!").unwrap();
        let with_mark = "alpha!\nbeta\ngamma\ndelta\n";
        // Lines 2 and 3, whole.
        e.move_to(Position::new(1, 0), false, BlockKind::Stream);
        e.move_to(Position::new(3, 0), true, BlockKind::Stream);
        assert_eq!(e.block_text().as_deref(), Some("beta\ngamma\n"));

        assert!(e.replace_block("one\ntwo\nthree\n").unwrap());
        assert_eq!(e.buffer.to_text(), "alpha!\none\ntwo\nthree\ndelta\n");
        assert_eq!(e.block_text().as_deref(), Some("one\ntwo\nthree\n"));
        assert_eq!(e.cursor, Position::new(4, 0), "after what was put in");

        assert!(e.undo());
        assert_eq!(
            e.buffer.to_text(),
            with_mark,
            "one step back is the block back"
        );
        assert!(e.redo());
        assert_eq!(e.buffer.to_text(), "alpha!\none\ntwo\nthree\ndelta\n");
        assert!(e.undo());
        assert!(e.undo());
        assert_eq!(e.buffer.to_text(), before);

        // A piece of a line, replaced by nothing at all.
        let mut e = editor("let x = old_name;\n");
        e.move_to(Position::new(0, 8), false, BlockKind::Stream);
        e.move_to(Position::new(0, 16), true, BlockKind::Stream);
        assert!(e.replace_block("new").unwrap());
        assert_eq!(e.buffer.to_text(), "let x = new;\n");

        // No block, or a column block: nothing is touched and it says so.
        let mut e = editor("ab\ncd\n");
        assert!(!e.replace_block("x").unwrap());
        e.move_to(Position::new(0, 0), false, BlockKind::Column);
        e.move_to(Position::new(1, 1), true, BlockKind::Column);
        assert!(!e.replace_block("x").unwrap());
        assert_eq!(e.buffer.to_text(), "ab\ncd\n");

        // A buffer that takes no edits refuses, and leaves no step half open.
        let mut e = editor("ab\ncd\n");
        e.move_to(Position::new(0, 0), false, BlockKind::Stream);
        e.move_to(Position::new(1, 0), true, BlockKind::Stream);
        e.buffer.read_only = true;
        assert!(e.replace_block("x\n").is_err());
        e.buffer.read_only = false;
        assert_eq!(e.buffer.to_text(), "ab\ncd\n");
        assert!(!e.undo(), "nothing was done, so nothing is undone");
    }
}
