use fcmd_core::{CancelToken, VPath};
use unicode_width::UnicodeWidthChar;

use crate::decode::{detect, looks_binary, Codepage};
use crate::disasm::{self, Arch, Binary, DisasmError, Format};
use crate::nav::{next_line_start, prev_line_start, read_line, snap_to_line};
use crate::search::{find, Match, Pattern, SearchError, SearchMode};
use crate::source::Source;
use crate::structure::{self, Structure};
use crate::{il, jvm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Text,
    /// Raw bytes as characters, fixed width: for files where decoding lies.
    Dump,
    Hex,
    /// Machine code as instructions.
    Disasm,
    /// Markdown with the marks taken off and the space they held put to use: headings
    /// ruled, lists bulleted, tables drawn. `F2` goes back to the source.
    Markdown,
    /// A tabular file as a table: a heading that stays put, columns, a row at a time
    /// (SPEC 05 §1.11). `F2` goes back to the source, which is sometimes what one came for.
    Table,
}

impl Mode {
    /// The letter the status line shows, as FAR does.
    pub fn letter(self) -> char {
        match self {
            Mode::Text => 't',
            Mode::Dump => 'd',
            Mode::Hex => 'h',
            Mode::Disasm => 'a',
            Mode::Markdown => 'm',
            Mode::Table => 'b',
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Text => "text",
            Mode::Dump => "dump",
            Mode::Hex => "hex",
            Mode::Disasm => "disasm",
            Mode::Markdown => "markdown",
            Mode::Table => "table",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wrap {
    Off,
    Char,
    Word,
}

impl Wrap {
    pub fn as_str(self) -> &'static str {
        match self {
            Wrap::Off => "no wrap",
            Wrap::Char => "wrap",
            Wrap::Word => "word wrap",
        }
    }
}

/// What `start_disasm` did: rows of instructions, or a generated listing shown as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    Instructions,
    Derived(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    Text,
    Hex,
    Dump,
    /// A disassembled instruction.
    Code,
    /// A heading inside the disassembly: a symbol, a section, the file summary.
    Label,
}

/// One rendered row, ready for the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// Offset in the file this row starts at.
    pub offset: u64,
    pub text: String,
    /// Character ranges to highlight (search hits), as `(start, len)` in characters.
    pub hits: Vec<(usize, usize)>,
    pub kind: RowKind,
    /// How the row is to be coloured, where the row was rendered rather than read: Markdown
    /// mode fills this, and every other mode leaves it empty and is coloured by its kind.
    pub spans: Vec<crate::markdown::Span>,
}

/// What the status line needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub name: String,
    pub mode: Mode,
    pub wrap: Wrap,
    pub codepage: &'static str,
    pub offset: u64,
    pub len: u64,
    pub percent: u16,
    pub follow: bool,
    pub search: Option<String>,
    pub bookmarks: Vec<usize>,
    /// What this file is, when we recognised a compiled format.
    pub binary: Option<String>,
    /// Set when we are looking at generated text (WAT, a decompiler's output).
    pub derived: Option<String>,
    /// In table mode: what it was read as, the delimiter that was guessed, the shape of the
    /// table and what was left out (SPEC 05 §1.11).
    pub table: Option<String>,
}

/// Tabs expand to the next multiple of this unless the configuration says otherwise. Eight is
/// what a terminal, `cat` and FAR all use, so a file whose author lined something up with tabs
/// looks here as it looks everywhere else.
pub const TAB_WIDTH: usize = 8;

/// The widths a hex dump is drawn at: blocks of eight, which is the group the dump itself
/// puts a gap between. The arrows in hex mode step through these (SPEC 05 §2).
pub const HEX_STEPS: [usize; 8] = [8, 16, 24, 32, 40, 48, 56, 64];

/// The largest file the Markdown view will render. Rendering needs the whole document, so
/// this is the one place the viewer reads more than a screenful — four megabytes is a very
/// long README and a very short log.
pub const MARKDOWN_LIMIT: u64 = 4 * 1024 * 1024;

/// The largest file table mode will read whole (SPEC 05 §1.11).
///
/// Larger than the Markdown limit, because a table is data and a README is prose: a 16 MB CSV
/// is an ordinary export, while a 16 MB Markdown file is somebody's mistake. Above this the
/// file opens as text, which is the viewer's usual promise — a window at a time, whatever the
/// size.
pub const TABLE_LIMIT: u64 = 16 * 1024 * 1024;

pub struct Viewer {
    source: Box<dyn Source>,
    pub path: VPath,
    pub mode: Mode,
    /// What `F4` toggles back to from hex.
    pub base_mode: Mode,
    pub wrap: Wrap,
    pub codepage: Codepage,
    /// Offset of the first visible row (always a line start in text mode).
    pub top: u64,
    /// Which wrapped segment of that line is on the first row.
    pub top_sub: usize,
    /// Horizontal scroll, in characters (text mode without wrap only).
    pub col: usize,
    pub bytes_per_row: usize,
    pub follow: bool,
    pub bookmarks: [Option<u64>; 10],
    /// Is the key bar drawn below the content? FAR's `Ctrl+B`.
    ///
    /// This and the two below are the chrome the reader is allowed to take away one piece at a
    /// time, to give a long line or a wide hex dump the whole window. The viewer only keeps the
    /// answer — it draws nothing itself, and a caller that has not asked about these fields
    /// behaves exactly as it did before they existed. The defaults are therefore what the
    /// screen shows today: a key bar and a status line, and no scrollbar, which the viewer has
    /// never had.
    pub show_key_bar: bool,
    /// Is the status line drawn? FAR's `Ctrl+Shift+B`.
    pub show_status_line: bool,
    /// Is the scrollbar drawn? FAR's `Ctrl+S`.
    pub show_scrollbar: bool,
    /// What a tab expands to, in columns. Eight unless the configuration says otherwise.
    pub tab_width: usize,
    /// Structure of the compiled file, once we have looked.
    pub binary: Option<Binary>,
    /// Is the structure pane open (SPEC 05 §1.8)?
    pub show_structure: bool,
    /// The decoded shape of the file: `None` = not looked yet, `Some(None)` = looked and it is
    /// not a format the pane knows, which is the common case.
    structure: Option<Option<Structure>>,
    /// What each visible disassembly row can follow: (row offset, target).
    pub disasm_targets: Vec<(u64, disasm::Target)>,
    /// Offsets we followed a branch from, so there is a way back (SPEC 05 §1.9).
    pub branch_stack: Vec<u64>,
    /// The rendered Markdown and the width it was rendered at. Unlike every other mode this
    /// one needs the whole document — a table's columns cannot be laid out from the part of
    /// it that is on screen — so it is rendered once and kept, and rendered again when the
    /// window changes width. That is also why it is limited to files under
    /// [`MARKDOWN_LIMIT`]: a reader of a 40 GB log gets the viewer's usual promise, and a
    /// reader of a README gets its columns lined up.
    markdown: Option<(usize, Vec<crate::markdown::Line>)>,
    /// The parsed table, in table mode. Like the rendered Markdown above it needs the whole
    /// file — a column's width cannot be known from the part of it that is on screen — so it
    /// is parsed once and kept, and the mode is only offered for files under
    /// [`TABLE_LIMIT`].
    table: Option<crate::table::Table>,
    /// The memo file beside a dBase table, when there is one and it could be opened: a
    /// `.dbt` or an `.fpt`, which is where the text of a memo field actually lives. The
    /// caller opens it and hands it over, because what a path means — on disk, inside an
    /// archive, on a server — is not something the viewer decides.
    memo: Option<Box<dyn Source>>,
    /// Which row of the table is on the first row of the screen. The table's own position:
    /// `top` carries the byte offset of that record, for the search and the bookmarks.
    pub table_row: usize,
    /// Cursor and scroll inside the pane, and whether the keys go to it.
    pub struct_cursor: usize,
    struct_top: usize,
    pub struct_focus: bool,
    /// Name of the producer when the content is generated (`WAT`, `javap`, …).
    pub derived: Option<String>,
    /// Set when the file was too large to parse and we are sweeping raw bytes; carries the
    /// limit, for the status line.
    raw_disasm: Option<u64>,
    /// Offsets the disassembly scrolled down through, so scrolling up is exact instead of
    /// heuristic: backward decoding of a variable-length instruction set can only guess.
    disasm_trail: Vec<u64>,
    pattern: Option<Pattern>,
    query: Option<String>,
    pub last_match: Option<Match>,
    /// Offsets the user came from, for "undo position".
    history: Vec<(u64, usize)>,
}

/// The structure reader asks for bytes at an offset; the viewer already has something that
/// does exactly that, whatever the file is sitting on.
struct SourceBytes<'a>(&'a mut dyn Source);

impl structure::Bytes for SourceBytes<'_> {
    fn at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        self.0.read_vec(offset, len).unwrap_or_default()
    }

    fn len(&mut self) -> u64 {
        self.0.len()
    }
}

impl std::fmt::Debug for Viewer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Viewer")
            .field("path", &self.path)
            .field("mode", &self.mode)
            .field("wrap", &self.wrap)
            .field("codepage", &self.codepage)
            .field("top", &self.top)
            .field("query", &self.query)
            .finish_non_exhaustive()
    }
}

impl Viewer {
    /// Open a source: the codepage is detected from the first bytes, and a file that looks
    /// binary opens in dump mode instead of pretending to be text.
    pub fn new(mut source: Box<dyn Source>, path: VPath) -> Self {
        let head = source.read_vec(0, 4096).unwrap_or_default();
        let codepage = detect(&head);
        let binary = looks_binary(&head);
        let base_mode = if binary { Mode::Dump } else { Mode::Text };
        Viewer {
            source,
            path,
            mode: base_mode,
            base_mode,
            wrap: Wrap::Off,
            codepage,
            top: 0,
            top_sub: 0,
            col: 0,
            bytes_per_row: 16,
            follow: false,
            bookmarks: [None; 10],
            show_key_bar: true,
            show_status_line: true,
            show_scrollbar: false,
            tab_width: TAB_WIDTH,
            binary: None,
            show_structure: false,
            structure: None,
            disasm_targets: Vec::new(),
            branch_stack: Vec::new(),
            markdown: None,
            table: None,
            memo: None,
            table_row: 0,
            struct_cursor: 0,
            struct_top: 0,
            struct_focus: false,
            derived: None,
            raw_disasm: None,
            disasm_trail: Vec::new(),
            pattern: None,
            query: None,
            last_match: None,
            history: Vec::new(),
        }
    }

    /// Hand the viewer the memo file beside a dBase table — the `.dbt` or `.fpt` whose
    /// blocks its memo fields point at. Without it a memo cell says `memo:42`, which is the
    /// block number and all there is to say; with it the cell says what is in the block.
    pub fn attach_memo(&mut self, memo: Box<dyn Source>) {
        self.memo = Some(memo);
    }

    pub fn len(&mut self) -> u64 {
        self.source.len()
    }

    pub fn is_empty(&mut self) -> bool {
        self.source.len() == 0
    }

    /// `Enter` in the disassembly: the branch on the top row, if it has one. The viewer has no
    /// cursor of its own in this mode, so the row at the top is the one being read.
    pub fn follow_branch(&mut self) -> Option<u64> {
        let from = self.top;
        let target = self
            .disasm_targets
            .iter()
            .find(|(offset, _)| *offset == from)
            .and_then(|(_, t)| t.offset)?;
        self.branch_stack.push(from);
        self.jump(target);
        Some(target)
    }

    /// `Backspace`: back where the last branch was followed from.
    pub fn unfollow_branch(&mut self) -> bool {
        match self.branch_stack.pop() {
            Some(offset) => {
                self.jump(offset);
                true
            }
            None => false,
        }
    }

    /// What the top row can follow, for the status line and the key bar.
    pub fn branch_target(&self) -> Option<&disasm::Target> {
        self.disasm_targets
            .iter()
            .find(|(offset, _)| *offset == self.top)
            .map(|(_, t)| t)
    }

    // ── chrome ─────────────────────────────────────────────────────────────────────────────

    /// `Ctrl+B`: show or hide the key bar. Returns the state it left, which is what the caller
    /// has to redraw for.
    pub fn toggle_key_bar(&mut self) -> bool {
        self.show_key_bar = !self.show_key_bar;
        self.show_key_bar
    }

    /// `Ctrl+Shift+B`: show or hide the status line.
    pub fn toggle_status_line(&mut self) -> bool {
        self.show_status_line = !self.show_status_line;
        self.show_status_line
    }

    /// `Ctrl+S`: show or hide the scrollbar.
    pub fn toggle_scrollbar(&mut self) -> bool {
        self.show_scrollbar = !self.show_scrollbar;
        self.show_scrollbar
    }

    // ── structure pane (SPEC 05 §1.8) ──────────────────────────────────────────────────────

    /// What this file is, decoded. Read once, lazily: a file that is not an executable costs
    /// one 4 KiB read to find out.
    pub fn structure(&mut self) -> Option<&Structure> {
        if self.structure.is_none() {
            let decoded = structure::read(&mut SourceBytes(self.source.as_mut()));
            self.structure = Some(decoded);
        }
        self.structure.as_ref().and_then(|s| s.as_ref())
    }

    /// `F11`: open or close the pane. Returns false when there is nothing to show, so the
    /// caller can say so instead of opening an empty frame.
    pub fn toggle_structure(&mut self) -> bool {
        if self.show_structure {
            self.show_structure = false;
            self.struct_focus = false;
            return true;
        }
        if self.structure().is_none() {
            return false;
        }
        self.show_structure = true;
        self.struct_focus = true;
        true
    }

    /// `Tab`: move the keys between the content and the pane.
    pub fn toggle_struct_focus(&mut self) {
        if self.show_structure {
            self.struct_focus = !self.struct_focus;
        }
    }

    /// The pane's visible rows, with the cursor's index among them.
    pub fn structure_rows(&mut self, height: u16) -> (Vec<structure::Row>, usize) {
        let top = self.struct_top;
        let cursor = self.struct_cursor;
        let Some(s) = self.structure() else {
            return (Vec::new(), 0);
        };
        let rows: Vec<structure::Row> = s
            .rows
            .iter()
            .skip(top)
            .take(height as usize)
            .cloned()
            .collect();
        (rows, cursor.saturating_sub(top))
    }

    pub fn structure_len(&mut self) -> usize {
        self.structure().map(|s| s.rows.len()).unwrap_or(0)
    }

    /// Move the pane's cursor, keeping it on screen. Group headings are skipped on the way,
    /// because they are labels, not destinations.
    pub fn structure_move(&mut self, delta: isize, height: u16) {
        let len = self.structure_len();
        if len == 0 {
            return;
        }
        let last = len - 1;
        let mut at = self.struct_cursor.min(last) as isize + delta;
        at = at.clamp(0, last as isize);
        self.struct_cursor = at as usize;
        let height = height.max(1) as usize;
        if self.struct_cursor < self.struct_top {
            self.struct_top = self.struct_cursor;
        } else if self.struct_cursor >= self.struct_top + height {
            self.struct_top = self.struct_cursor + 1 - height;
        }
    }

    pub fn structure_home(&mut self) {
        self.struct_cursor = 0;
        self.struct_top = 0;
    }

    pub fn structure_end(&mut self, height: u16) {
        let len = self.structure_len();
        self.struct_cursor = len.saturating_sub(1);
        self.struct_top = len.saturating_sub(height.max(1) as usize);
    }

    /// `Enter` in the pane: where the row points, if anywhere.
    pub fn structure_target(&mut self) -> Option<u64> {
        let at = self.struct_cursor;
        self.structure()?.rows.get(at)?.offset
    }

    /// One line of identity for the pane's title.
    pub fn structure_summary(&mut self) -> Option<String> {
        self.structure().map(|s| s.summary.clone())
    }

    // ── modes ──────────────────────────────────────────────────────────────────────────────

    /// `F2`: in text mode toggle wrapping; in dump mode switch to text; in hex go to the base.
    pub fn toggle_wrap(&mut self) {
        match self.mode {
            Mode::Text => {
                self.wrap = match self.wrap {
                    Wrap::Off => Wrap::Char,
                    Wrap::Char => Wrap::Word,
                    Wrap::Word => Wrap::Off,
                };
                if self.wrap != Wrap::Off {
                    self.col = 0;
                }
            }
            // From a rendered document or a dump, `F2` is the way to the source.
            Mode::Dump | Mode::Markdown | Mode::Table => self.set_mode(Mode::Text),
            // From hex or disassembly, F2 goes back to whatever text/dump mode we came from.
            Mode::Hex | Mode::Disasm => {
                let base = self.base_mode;
                self.set_mode(base);
            }
        }
    }

    /// `F4`: hex and back.
    pub fn toggle_hex(&mut self) {
        if self.mode == Mode::Hex {
            let base = self.base_mode;
            self.set_mode(base);
        } else {
            self.set_mode(Mode::Hex);
        }
    }

    pub fn set_mode(&mut self, mode: Mode) {
        if mode != Mode::Hex {
            self.base_mode = mode;
        }
        self.mode = mode;
        self.col = 0;
        // Keep the position: snap to what the new mode can show.
        let top = self.top;
        self.goto(top);
    }

    /// Read the file's structure (format, architecture, sections, symbols) if we have not yet.
    ///
    /// Files above [`disasm::MAX_PARSE`] are parsed from their first bytes only, and the result
    /// says so, rather than pulling a gigabyte into memory.
    pub fn analyze(&mut self) -> Result<&Binary, DisasmError> {
        if self.binary.is_none() {
            let len = self.source.len();
            let cap = disasm::MAX_PARSE as u64;
            if len > cap {
                return Err(DisasmError::TooLarge {
                    limit: cap / (1024 * 1024),
                });
            }
            let bytes = self
                .source
                .read_vec(0, len as usize)
                .map_err(|e| DisasmError::Parse(e.to_string()))?;
            self.binary = Some(disasm::analyze(&bytes, true)?);
        }
        Ok(self.binary.as_ref().expect("just analysed"))
    }

    /// Switch to disassembly, jumping to the code section the first time.
    ///
    /// Bytecode formats produce a listing instead of instruction rows, because that is how
    /// they are read: a `.class` file becomes a `javap`-style listing, a managed PE becomes an
    /// IL listing, a `.wasm` becomes its text format — all in process. A file too large to
    /// parse structurally is still disassembled, as a raw sweep from where the cursor already
    /// is, using the host architecture; the status line says so.
    pub fn start_disasm(&mut self) -> Result<Started, DisasmError> {
        match self.format() {
            Format::JavaClass => {
                let bytes = self.whole_file()?;
                let text = jvm::listing(&bytes).map_err(|e| DisasmError::Parse(e.to_string()))?;
                self.show_derived("bytecode", text);
                return Ok(Started::Derived("bytecode"));
            }
            Format::Wasm => {
                let text = self.wasm_text()?;
                self.show_derived("WAT", text);
                return Ok(Started::Derived("WAT"));
            }
            Format::Pe => {
                let bytes = self.whole_file()?;
                if il::is_managed(&bytes) {
                    let text =
                        il::listing(&bytes).map_err(|e| DisasmError::Parse(e.to_string()))?;
                    self.show_derived("IL", text);
                    return Ok(Started::Derived("IL"));
                }
            }
            _ => {}
        }
        self.start_native_disasm().map(|()| Started::Instructions)
    }

    /// The whole file, for the parsers that need it. Bounded by the parse limit.
    fn whole_file(&mut self) -> Result<Vec<u8>, DisasmError> {
        let len = self.source.len();
        if len > disasm::MAX_PARSE as u64 {
            return Err(DisasmError::TooLarge {
                limit: disasm::MAX_PARSE as u64 / (1024 * 1024),
            });
        }
        self.source
            .read_vec(0, len as usize)
            .map_err(|e| DisasmError::Parse(e.to_string()))
    }

    fn start_native_disasm(&mut self) -> Result<(), DisasmError> {
        let start = match self.analyze() {
            Ok(binary) => {
                if !binary.arch.disassemblable() {
                    return Err(DisasmError::NoDisassembler(binary.arch.as_str()));
                }
                binary
                    .offset_for_va(binary.entry)
                    .or_else(|| binary.text_section().and_then(|s| s.file_offset))
            }
            Err(DisasmError::TooLarge { limit }) => {
                // Too big to parse structurally: sweep the bytes. There is no architecture to
                // read, so the host's is used, or x86-64 where the host has no decoder.
                self.raw_disasm = Some(limit);
                None
            }
            Err(other) => return Err(other),
        };
        self.mode = Mode::Disasm;
        self.col = 0;
        if let Some(offset) = start {
            self.jump(offset);
        }
        Ok(())
    }

    /// Point the viewer at generated text — WebAssembly's text format, or a decompiler's
    /// output — keeping the original path in the title.
    pub fn show_derived(&mut self, producer: &str, text: String) {
        self.source = Box::new(crate::source::BytesSource::new(text.into_bytes()));
        self.derived = Some(producer.to_string());
        self.codepage = Codepage::Utf8;
        self.mode = Mode::Text;
        self.base_mode = Mode::Text;
        self.binary = None;
        self.history.clear();
        self.disasm_trail.clear();
        self.top = 0;
        self.top_sub = 0;
        self.col = 0;
        self.clear_search();
    }

    /// WebAssembly to its canonical text format, in process.
    pub fn wasm_text(&mut self) -> Result<String, DisasmError> {
        let len = self.source.len();
        let bytes = self
            .source
            .read_vec(0, len.min(disasm::MAX_PARSE as u64) as usize)
            .map_err(|e| DisasmError::Parse(e.to_string()))?;
        disasm::wasm_text(&bytes)
    }

    /// What this file is, for the decompile command to decide what to do.
    pub fn format(&mut self) -> disasm::Format {
        let head = self.source.read_vec(0, 64).unwrap_or_default();
        disasm::probe(&head)
    }

    pub fn next_codepage(&mut self) {
        self.codepage = self.codepage.next();
    }

    pub fn set_codepage(&mut self, codepage: Codepage) {
        self.codepage = codepage;
    }

    /// Widen or narrow the dump by one step, within what the window can draw.
    ///
    /// **The steps are blocks of eight** — 8, 16, 24, 32, 40, 48, 56, 64 — because eight
    /// bytes is the group a hex dump already draws a gap between, so every step adds one
    /// whole group and the columns stay where the eye expects them. They used to be the
    /// powers of two, which doubled: 16 to 32 to 64 is three widths on a wide screen where
    /// there is room for eight.
    ///
    /// They are steps rather than a free number because [`Self::hex_fit`] draws the widest
    /// step that fits: a setting the window cannot hold would mean one press doing nothing,
    /// the next doing the work of two, and the ones past the widest that fits counting up
    /// invisibly — so coming back would take as many presses as had been wasted. One press,
    /// one step one can see, and the widest step is the widest the window can hold rather
    /// than a number stored for a window nobody has.
    pub fn set_bytes_per_row(&mut self, delta: i32, width: u16) {
        let fits = |bpr: usize| width == 0 || hex_row_width(bpr) <= width;
        let next = match delta.signum() {
            // The next step above what we have, and the next below it: written this way
            // rather than as an index so that a width from anywhere — an old session, a
            // narrow window — still steps to a neighbour rather than jumping.
            1 => HEX_STEPS
                .iter()
                .copied()
                .find(|&step| step > self.bytes_per_row && fits(step))
                .unwrap_or(self.bytes_per_row),
            -1 => HEX_STEPS
                .iter()
                .rev()
                .copied()
                .find(|&step| step < self.bytes_per_row)
                .unwrap_or(HEX_STEPS[0]),
            _ => self.bytes_per_row,
        };
        // A window too narrow even for where we are still draws what it can (`hex_fit`),
        // and the setting follows it down rather than hanging above it.
        let next = if fits(next) {
            next
        } else {
            HEX_STEPS
                .iter()
                .rev()
                .copied()
                .find(|&step| fits(step))
                .unwrap_or(HEX_STEPS[0])
        };
        self.bytes_per_row = next;
        let top = self.top;
        self.goto(top);
    }

    /// The bytes per row that actually fit in `width`: never more than the setting, and a
    /// whole number of the eight-byte groups the dump draws gaps between, so an offset
    /// aligned to the setting stays aligned to what we draw. A narrow window drops a group
    /// instead of cutting the printable gutter in half, which is the one part of a hex dump a
    /// reader scans.
    ///
    /// Four is the floor and the one width that is not a block of eight: a window too narrow
    /// for eight bytes — under forty-five columns — still has to draw something, and half a
    /// group is better than half a row.
    pub fn hex_fit(&self, width: u16) -> usize {
        let mut bpr = 4;
        for candidate in std::iter::once(4).chain(HEX_STEPS) {
            if candidate > self.bytes_per_row {
                break;
            }
            if hex_row_width(candidate) <= width {
                bpr = candidate;
            }
        }
        bpr
    }

    /// Columns a hex row needs: the offset, two spaces, the pairs with a gap every eight,
    /// and the printable gutter between its bars.
    pub fn hex_row_cols(&self, width: u16) -> u16 {
        hex_row_width(self.hex_fit(width))
    }

    // ── position ───────────────────────────────────────────────────────────────────────────

    pub fn goto(&mut self, offset: u64) {
        self.disasm_trail.clear();
        let len = self.source.len();
        let offset = offset.min(len);
        self.top = match self.mode {
            Mode::Text => snap_to_line(&mut *self.source, offset),
            Mode::Hex => offset - (offset % self.bytes_per_row as u64),
            Mode::Dump | Mode::Disasm | Mode::Markdown => offset,
            // The position of a table is which row is at the top, and an offset names the
            // record that holds it: anything else would put half a row on the first line.
            Mode::Table => {
                self.table_row = self.table_at(offset);
                offset
            }
        };
        self.top_sub = 0;
    }

    /// Move the view by whole bytes, *without* snapping to the row width.
    ///
    /// A hex dump starts each row on a multiple of the row width, which is what keeps the
    /// offsets round and the columns in line. A structure inside a file does not care about
    /// that: a record that begins at an odd offset is read diagonally, one byte further along
    /// on every row. Nudging the whole dump by a byte puts it back in line, and that is all
    /// this does — the alignment of the rows to each other is kept, only their alignment to
    /// the start of the file changes.
    pub fn shift(&mut self, delta: i64) {
        let len = self.source.len() as i64;
        let next = (self.top as i64).saturating_add(delta).clamp(0, len);
        self.top = next as u64;
        self.top_sub = 0;
        self.disasm_trail.clear();
    }

    /// `Ctrl` and an arrow in a table: the next column boundary, in the direction asked for.
    ///
    /// A table has a unit for sideways that text has not — the column — so the big step is
    /// one of those rather than a number of characters nobody chose. At the ends it stops.
    pub fn scroll_h_column(&mut self, forward: bool) {
        if self.mode != Mode::Table {
            // Everywhere else the big step is what it always was: a screenful of characters.
            self.scroll_h(if forward { 20 } else { -20 });
            return;
        }
        self.parse_table();
        let Some(table) = &self.table else {
            return;
        };
        let starts = table.column_starts();
        let at = self.col;
        let next = if forward {
            starts.iter().copied().find(|start| *start > at)
        } else {
            starts.iter().rev().copied().find(|start| *start < at)
        };
        self.col = next.unwrap_or(if forward { at } else { 0 });
    }

    /// Remember where we are, then jump: `Ctrl+Z` in the viewer comes back here.
    pub fn jump(&mut self, offset: u64) {
        self.history.push((self.top, self.top_sub));
        self.goto(offset);
    }

    pub fn undo_jump(&mut self) -> bool {
        match self.history.pop() {
            Some((top, sub)) => {
                self.top = top;
                self.top_sub = sub;
                true
            }
            None => false,
        }
    }

    pub fn to_start(&mut self) {
        self.history.push((self.top, self.top_sub));
        self.top = 0;
        self.top_sub = 0;
        self.col = 0;
        // A table's position is which row is at the top, and nothing else moves it: leaving
        // that where it was is what made `Home` look as if it had done nothing.
        self.table_row = 0;
    }

    /// The last screenful, so `End` shows the end of the file rather than an empty screen.
    pub fn to_end(&mut self, width: u16, height: u16) {
        self.history.push((self.top, self.top_sub));
        // A table counts in rows, so the last screenful is the last rows — worked out from
        // the record count rather than from an offset, which in a table means nothing.
        if self.mode == Mode::Table {
            self.parse_table();
            let rows = self.table.as_ref().map(|t| t.len()).unwrap_or(0);
            // Two of the window's rows are the heading and its rule.
            let body = (height as usize).saturating_sub(2).max(1);
            self.table_row = rows.saturating_sub(body);
            self.top = self
                .table
                .as_ref()
                .map(|t| t.offset(self.table_row))
                .unwrap_or(0);
            return;
        }
        let len = self.source.len();
        self.goto(len);
        self.scroll(-(height as i64), width, height);
    }

    pub fn goto_percent(&mut self, percent: u16) {
        let len = self.source.len();
        let offset = len * percent.min(100) as u64 / 100;
        self.jump(offset);
    }

    /// Move the window sideways, in display columns.
    ///
    /// Text without wrapping and a table both have something wider than the window to scroll
    /// — a long line in the one, a row of columns in the other — and a table stops where its
    /// last column ends rather than scrolling into empty space, because there is nothing out
    /// there and a reader who has lost the table has to scroll all the way back.
    pub fn scroll_h(&mut self, delta: i64) {
        match self.mode {
            Mode::Text if self.wrap == Wrap::Off => {
                self.col = (self.col as i64 + delta).max(0) as usize;
            }
            Mode::Table => {
                self.parse_table();
                let widest = self.table.as_ref().map(|t| t.total_width()).unwrap_or(0);
                let at = (self.col as i64 + delta).max(0) as usize;
                self.col = at.min(widest.saturating_sub(1));
            }
            _ => {}
        }
    }

    pub fn set_bookmark(&mut self, slot: usize) -> bool {
        if slot < self.bookmarks.len() {
            self.bookmarks[slot] = Some(self.top);
            true
        } else {
            false
        }
    }

    pub fn goto_bookmark(&mut self, slot: usize) -> bool {
        match self.bookmarks.get(slot).copied().flatten() {
            Some(offset) => {
                self.jump(offset);
                true
            }
            None => false,
        }
    }

    /// Scroll by `delta` display rows (negative is up).
    pub fn scroll(&mut self, delta: i64, width: u16, height: u16) {
        if delta == 0 {
            return;
        }
        match self.mode {
            Mode::Text => self.scroll_text(delta, width, height),
            Mode::Disasm => self.scroll_disasm(delta),
            Mode::Hex => {
                let bpr = self.hex_fit(width) as u64;
                let step = bpr as i64 * delta;
                self.top = clamp_offset(self.top, step, self.source.len());
                self.top -= self.top % bpr;
            }
            Mode::Dump => {
                let step = width.max(1) as i64 * delta;
                self.top = clamp_offset(self.top, step, self.source.len());
            }
            Mode::Markdown => self.scroll_markdown(delta, width),
            Mode::Table => self.scroll_table(delta),
        }
    }

    /// Scrolling a rendered document is scrolling its rendered lines, and the `(offset, sub)`
    /// pair of the line we land on is the position — the same pair wrapped text already uses,
    /// so `Ctrl+Z`, the bookmarks and the search all keep working without knowing about this.
    fn scroll_markdown(&mut self, delta: i64, width: u16) {
        self.render_markdown(width as usize);
        let Some((_, lines)) = &self.markdown else {
            return;
        };
        if lines.is_empty() {
            return;
        }
        let at = Self::markdown_index(lines, self.top, self.top_sub) as i64;
        let last = lines.len() as i64 - 1;
        let next = (at + delta).clamp(0, last) as usize;
        self.top = lines[next].source;
        self.top_sub = lines[next].sub;
    }

    /// Which rendered line `(top, sub)` names.
    ///
    /// Scrolling sets the pair from a line, so it is found exactly. A search or a bookmark
    /// sets it from an offset in the file, which may be the middle of a paragraph or of a
    /// table — then the block that offset belongs to is shown from its first line, because
    /// half a table is not a reading position.
    fn markdown_index(lines: &[crate::markdown::Line], top: u64, sub: usize) -> usize {
        let Some(at) = lines.iter().position(|l| (l.source, l.sub) >= (top, sub)) else {
            return lines.len();
        };
        if at == 0 || lines[at].source == top {
            return at;
        }
        let block = lines[at - 1].source;
        lines
            .iter()
            .position(|l| l.source == block)
            .unwrap_or(at - 1)
    }

    /// Render the document if it is not rendered at this width already.
    fn render_markdown(&mut self, width: usize) {
        if self.markdown.as_ref().is_some_and(|(at, _)| *at == width) {
            return;
        }
        let len = self.source.len().min(MARKDOWN_LIMIT) as usize;
        let bytes = self.source.read_vec(0, len).unwrap_or_default();
        let text = self.codepage.decode(&bytes);
        self.markdown = Some((width, crate::markdown::render(&text, width)));
    }

    /// Parse the table, if it has not been parsed already. Unlike the Markdown render this
    /// does not depend on the width: the columns are as wide as what is in them, and the
    /// window moves over the table rather than the table being laid out again for it.
    fn parse_table(&mut self) {
        if self.table.is_some() {
            return;
        }
        let whole = self.source.len();
        let name = self.path.display_name();
        // A dBase table says in its first bytes where its records are and how long they are,
        // so there is nothing to read here but the header: the rows are fetched a window at
        // a time, however large the file is.
        if name.to_ascii_lowercase().ends_with(".dbf") {
            let head = self.source.read_vec(0, 64 * 1024).unwrap_or_default();
            if let Some(table) = crate::table::Table::dbase(&head, whole) {
                self.table = Some(table);
                return;
            }
        }
        let len = whole.min(TABLE_LIMIT) as usize;
        let bytes = self.source.read_vec(0, len).unwrap_or_default();
        let text = self.codepage.decode(&bytes);
        let mut table = crate::table::Table::of(&name, &text)
            .unwrap_or_else(|| crate::table::Table::separated(&text, None));
        table.cut = whole > TABLE_LIMIT;
        self.table = Some(table);
    }

    /// The table on screen, parsing it if this is the first look. `None` outside table mode.
    pub fn table(&mut self) -> Option<&crate::table::Table> {
        if self.mode != Mode::Table {
            return None;
        }
        self.parse_table();
        self.table.as_ref()
    }

    /// Which row of the table a byte offset falls in: the last record that starts at or
    /// before it, because an offset in the middle of a record is still that record.
    fn table_at(&mut self, offset: u64) -> usize {
        self.parse_table();
        let Some(table) = &self.table else {
            return 0;
        };
        table.row_at(offset)
    }

    /// Scrolling a table is scrolling its rows, and `top` follows so that a search, a
    /// bookmark and `Ctrl+Z` keep meaning what they mean everywhere else.
    fn scroll_table(&mut self, delta: i64) {
        self.parse_table();
        let Some(table) = &self.table else {
            return;
        };
        if table.is_empty() {
            return;
        }
        let last = table.len() as i64 - 1;
        let next = (self.table_row as i64 + delta).clamp(0, last) as usize;
        self.table_row = next;
        self.top = table.offset(next);
    }

    /// The rows of a table: the heading and its rule first, and they stay there — a table
    /// whose heading scrolled away with the data would be a wall of numbers again — then as
    /// many records as the window has room for.
    fn rows_table(&mut self, width: u16, height: u16) -> Vec<Row> {
        self.parse_table();
        let (col, from) = (self.col, self.table_row);
        let pattern = self.pattern.is_some();
        // Two fields, borrowed apart: the table says what to draw and the source is where a
        // windowed table's records come from.
        let (table, source, memo) = (&self.table, &mut self.source, &mut self.memo);
        let Some(table) = table else {
            return Vec::new();
        };
        let width = width as usize;
        let mut out = Vec::with_capacity(height as usize);
        let first = table.offset(0);
        out.push(Row {
            offset: first,
            text: table.header(col, width),
            hits: Vec::new(),
            kind: RowKind::Label,
            spans: Vec::new(),
        });
        if height >= 2 {
            out.push(Row {
                offset: first,
                text: table.rule(col, width),
                hits: Vec::new(),
                kind: RowKind::Label,
                spans: Vec::new(),
            });
        }
        for at in from..table.len() {
            if out.len() >= height as usize {
                break;
            }
            out.push(Row {
                offset: table.offset(at),
                text: table.line(&mut **source, memo.as_deref_mut(), at, col, width),
                hits: Vec::new(),
                kind: RowKind::Text,
                spans: Vec::new(),
            });
        }
        // As in a rendered document, a search here finds what is on the screen: somebody
        // looking for a value means the cell, not the commas behind it.
        if pattern {
            for row in &mut out {
                row.hits = self.hits_in_text(&row.text);
            }
        }
        out
    }

    /// Is this a file table mode *opens* by itself? Its name has to say so and it has to be
    /// small enough to parse whole.
    ///
    /// This is about the first screen, and about the mode cycle: `Shift+F3` is less fussy
    /// (see [`Self::as_table`]), because a file whose name says nothing can still be a table
    /// and the reader can see that it is.
    pub fn reads_table(&mut self) -> bool {
        let name = self.path.display_name();
        if !crate::table::Table::reads(&name) || self.derived.is_some() {
            return false;
        }
        // The size limit is about reading a file whole, which a dBase table is not: it is
        // read a window at a time however large it is.
        name.to_ascii_lowercase().ends_with(".dbf") || self.source.len() <= TABLE_LIMIT
    }

    /// Show the file as a table. `false` when there is no table in it, so the caller can say
    /// why rather than leaving the key looking broken.
    ///
    /// The name is not the test. Tab-separated data in a `.txt` is as common as in a `.tsv`,
    /// and a log with a column layout has no extension that says so: what decides is whether
    /// the file *reads* as a table — two columns or more, with the delimiter guessed the
    /// usual way. The same key goes back to the source, so a wrong guess costs one keystroke.
    pub fn as_table(&mut self) -> bool {
        if self.derived.is_some() || self.source.len() > TABLE_LIMIT && !self.reads_table() {
            return false;
        }
        // Columns need text to be in. A file whose first bytes are not text at all is not a
        // table however many times a delimiter happens to turn up in it — unless its name
        // says what it is, which is how a `.dbf` gets past here.
        if !self.reads_table() {
            let head = self.source.read_vec(0, 8 * 1024).unwrap_or_default();
            if crate::decode::looks_binary(&head) {
                return false;
            }
        }
        self.parse_table();
        let columns = self.table.as_ref().map(|t| t.columns.len()).unwrap_or(0);
        if columns < 2 && !self.reads_table() {
            // Nothing in it comes apart into columns: a file of prose is not a table, and
            // drawing it as one would be a worse answer than saying so.
            self.table = None;
            return false;
        }
        self.set_mode(Mode::Table);
        self.table_row = 0;
        self.col = 0;
        true
    }

    /// What the status line says about the table: the format, the delimiter that was guessed,
    /// how many rows there are, and what was left out. `None` outside table mode.
    pub fn table_note(&mut self) -> Option<String> {
        if self.mode != Mode::Table {
            return None;
        }
        self.parse_table();
        let table = self.table.as_ref()?;
        let mut note = table.format.to_string();
        if let Some(delimiter) = table.delimiter {
            let shown = match delimiter {
                '\t' => "tab".to_string(),
                c => c.to_string(),
            };
            note.push_str(&format!(" `{shown}`"));
        }
        note.push_str(&format!(" · {} × {}", table.len(), table.fields()));
        if table.skipped > 0 {
            note.push_str(&format!(" · {} skipped", table.skipped));
        }
        if table.cut {
            note.push_str(" · cut");
        }
        Some(note)
    }

    /// Is this a file the Markdown view can show? Its name has to say Markdown and it has to
    /// be small enough to render whole.
    pub fn renders_markdown(&mut self) -> bool {
        let name = self.path.display_name().to_ascii_lowercase();
        let markdown = name.ends_with(".md")
            || name.ends_with(".markdown")
            || name.ends_with(".mdown")
            || name.ends_with(".mkd");
        markdown && self.source.len() <= MARKDOWN_LIMIT && self.derived.is_none()
    }

    /// Show the file rendered, if it is one we can render. `false` when it is not, so the
    /// caller can say why rather than leaving the key looking broken.
    pub fn as_markdown(&mut self) -> bool {
        if !self.renders_markdown() {
            return false;
        }
        self.set_mode(Mode::Markdown);
        true
    }

    fn scroll_text(&mut self, delta: i64, width: u16, height: u16) {
        let mut remaining = delta.abs();
        let down = delta > 0;
        let len = self.source.len();

        while remaining > 0 {
            if down {
                let segments = self.segment_count(self.top, width);
                if self.top_sub + 1 < segments {
                    self.top_sub += 1;
                } else {
                    match next_line_start(&mut *self.source, self.top) {
                        Some(next) => {
                            self.top = next;
                            self.top_sub = 0;
                        }
                        None => break,
                    }
                }
            } else if self.top_sub > 0 {
                self.top_sub -= 1;
            } else {
                match prev_line_start(&mut *self.source, self.top) {
                    Some(prev) => {
                        self.top = prev;
                        self.top_sub = self.segment_count(prev, width).saturating_sub(1);
                    }
                    None => break,
                }
            }
            remaining -= 1;
        }

        // Never scroll past the end: keep at least one row of content on screen.
        if self.top >= len && len > 0 {
            self.goto(len);
            let _ = height;
        }
    }

    /// Instructions have no fixed length, so scrolling decodes: forwards by one instruction,
    /// backwards by re-decoding a lead-in window (see `disasm::previous_instruction`).
    fn scroll_disasm(&mut self, delta: i64) {
        let arch = self.disasm_arch();
        let len = self.source.len();
        let mut remaining = delta.abs();
        while remaining > 0 {
            if delta > 0 {
                let window = self.source.read_vec(self.top, 32).unwrap_or_default();
                let step = disasm::disassemble(&window, self.top, self.top, arch, 1)
                    .ok()
                    .and_then(|lines| lines.first().map(|l| l.bytes.len() as u64))
                    .unwrap_or(1)
                    .max(1);
                if self.top + step >= len {
                    break;
                }
                self.disasm_trail.push(self.top);
                if self.disasm_trail.len() > 8192 {
                    self.disasm_trail.remove(0);
                }
                self.top += step;
            } else {
                if self.top == 0 {
                    break;
                }
                // Exact when we came down this way; a decoded guess when we did not.
                match self.disasm_trail.pop() {
                    Some(previous) if previous < self.top => self.top = previous,
                    _ => {
                        let lead = self.top.min(32);
                        let window = self
                            .source
                            .read_vec(self.top - lead, lead as usize)
                            .unwrap_or_default();
                        let previous = disasm::previous_instruction(&window, self.top, arch);
                        self.top = if previous >= self.top {
                            self.top.saturating_sub(1)
                        } else {
                            previous
                        };
                    }
                }
            }
            remaining -= 1;
        }
    }

    fn segment_count(&mut self, line_start: u64, width: u16) -> usize {
        if self.wrap == Wrap::Off {
            return 1;
        }
        let (bytes, _) = read_line(&mut *self.source, line_start);
        let text = expand_tabs(&self.codepage.decode(&bytes), self.tab_width);
        wrap_segments(&text, width.max(1) as usize, self.wrap)
            .len()
            .max(1)
    }

    // ── rendering ──────────────────────────────────────────────────────────────────────────

    /// The rows to put on screen, at most `height` of them.
    pub fn rows(&mut self, width: u16, height: u16) -> Vec<Row> {
        match self.mode {
            Mode::Text => self.rows_text(width, height),
            Mode::Hex => self.rows_hex(width, height),
            Mode::Dump => self.rows_dump(width, height),
            Mode::Disasm => self.rows_disasm(width, height),
            Mode::Markdown => self.rows_markdown(width, height),
            Mode::Table => self.rows_table(width, height),
        }
    }

    fn rows_markdown(&mut self, width: u16, height: u16) -> Vec<Row> {
        self.render_markdown(width as usize);
        let (top, sub) = (self.top, self.top_sub);
        let pattern = self.pattern.is_some();
        let Some((_, lines)) = &self.markdown else {
            return Vec::new();
        };
        let start = Self::markdown_index(lines, top, sub);
        let mut out = Vec::with_capacity(height as usize);
        for line in lines.iter().skip(start).take(height as usize) {
            out.push(Row {
                offset: line.source,
                text: line.text.clone(),
                hits: Vec::new(),
                kind: RowKind::Text,
                spans: line.spans.clone(),
            });
        }
        // A search in a rendered document searches what is on screen, not the marks behind
        // it: somebody looking for `size` in a table means the word in the cell, and the
        // offsets of the source line it came from would land on the pipes.
        if pattern {
            for row in &mut out {
                row.hits = self.hits_in_text(&row.text);
            }
        }
        out
    }

    /// The architecture to decode with: the file's, or the host's for a raw sweep.
    /// Which instruction set the bytes are decoded with: the file's own when we could read a
    /// container, otherwise the host's — and when the host has no decoder (an ARM Mac, say)
    /// x86-64, which the status line calls an assumption rather than a fact.
    fn disasm_arch(&self) -> Arch {
        if let Some(binary) = &self.binary {
            return binary.arch;
        }
        let host = disasm::host_arch();
        if host.disassemblable() {
            host
        } else {
            Arch::X86_64
        }
    }

    fn rows_disasm(&mut self, width: u16, height: u16) -> Vec<Row> {
        let arch = self.disasm_arch();
        // 15 bytes is the longest x86 instruction, so this window always covers the screen.
        let window_len = height as usize * 16 + 32;
        let window = self
            .source
            .read_vec(self.top, window_len)
            .unwrap_or_default();
        if window.is_empty() {
            return Vec::new();
        }
        let base_va = self
            .binary
            .as_ref()
            .map(|b| b.va_for_offset(self.top))
            .unwrap_or(self.top);
        let mut lines = disasm::disassemble(&window, self.top, base_va, arch, height as usize)
            .unwrap_or_default();

        // Where each branch points, in file offsets, so `Enter` can follow one. The section
        // map comes from whichever reader has it: the disassembler's own parse, or the
        // structure pane's, which works on files too large for the first.
        if let Some(binary) = self.binary.as_ref() {
            binary.resolve_targets(&mut lines);
        } else if let Some(structure) = self.structure.as_ref().and_then(|s| s.as_ref()) {
            let map: Vec<structure::Mapping> = structure.maps.clone();
            disasm::resolve_targets(&mut lines, |va| {
                map.iter().find_map(|m| {
                    (va >= m.address && va < m.address.saturating_add(m.size))
                        .then(|| m.offset + (va - m.address))
                })
            });
        }
        // Remember what each row can follow, for `Enter` (SPEC 05 §1.9).
        self.disasm_targets = lines
            .iter()
            .filter_map(|l| l.target.as_ref().map(|t| (l.offset, *t)))
            .collect();

        let mut out = Vec::with_capacity(lines.len());
        for line in lines {
            if out.len() >= height as usize {
                break;
            }
            // A symbol that starts exactly here gets a heading of its own.
            if let Some(symbol) = self
                .binary
                .as_ref()
                .and_then(|b| b.functions.iter().find(|f| f.address == line.address))
            {
                if out.len() + 1 < height as usize {
                    out.push(Row {
                        offset: line.offset,
                        text: format!("{:016X} <{}>:", line.address, symbol.name),
                        hits: Vec::new(),
                        kind: RowKind::Label,
                        spans: Vec::new(),
                    });
                }
            }
            let hex: String = line
                .bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            // A branch to a symbol reads as the symbol, which is the whole difference between
            // reading bytes and reading a program.
            let binary = self.binary.as_ref();
            let structure = self.structure.as_ref().and_then(|s| s.as_ref());
            let body = line.annotated(|va| {
                binary
                    .and_then(|b| b.symbol_at(va))
                    .or_else(|| structure.and_then(|s| s.symbol_at(va)))
            });
            let text = format!("{:016X}  {:<23}  {}", line.address, hex, body);
            out.push(Row {
                offset: line.offset,
                text: truncate_to(&text, width as usize),
                hits: Vec::new(),
                kind: if line.invalid {
                    RowKind::Dump
                } else {
                    RowKind::Code
                },
                spans: Vec::new(),
            });
        }
        out
    }

    fn rows_text(&mut self, width: u16, height: u16) -> Vec<Row> {
        let width = width.max(1) as usize;
        let len = self.source.len();
        if len == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(height as usize);
        let mut offset = self.top;
        let mut skip = self.top_sub;

        while out.len() < height as usize {
            if offset > len || (offset == len && offset != 0) {
                break;
            }
            let (bytes, next) = read_line(&mut *self.source, offset);
            let decoded = expand_tabs(&self.codepage.decode(&bytes), self.tab_width);
            let hits = self.hits_for(&bytes, &decoded);

            if self.wrap == Wrap::Off {
                let text: String = decoded.chars().skip(self.col).take(width).collect();
                let shifted = shift_hits(&hits, self.col, width);
                out.push(Row {
                    offset,
                    text,
                    hits: shifted,
                    kind: RowKind::Text,
                    spans: Vec::new(),
                });
            } else {
                let segments = wrap_segments(&decoded, width, self.wrap);
                let mut char_base = 0usize;
                for (i, segment) in segments.iter().enumerate() {
                    let chars = segment.chars().count();
                    if i >= skip && out.len() < height as usize {
                        out.push(Row {
                            offset,
                            text: segment.clone(),
                            hits: shift_hits(&hits, char_base, chars),
                            kind: RowKind::Text,
                            spans: Vec::new(),
                        });
                    }
                    char_base += chars;
                }
                skip = 0;
            }

            if next <= offset {
                break;
            }
            offset = next;
            if offset >= len {
                break;
            }
        }
        out
    }

    fn rows_hex(&mut self, width: u16, height: u16) -> Vec<Row> {
        let bpr = self.hex_fit(width);
        let len = self.source.len();
        let mut out = Vec::with_capacity(height as usize);
        let mut offset = self.top;
        while out.len() < height as usize && offset < len {
            let bytes = self.source.read_vec(offset, bpr).unwrap_or_default();
            if bytes.is_empty() {
                break;
            }
            let mut hex = String::with_capacity(bpr * 3);
            for i in 0..bpr {
                match bytes.get(i) {
                    Some(b) => hex.push_str(&format!("{b:02X} ")),
                    None => hex.push_str("   "),
                }
                if i % 8 == 7 {
                    hex.push(' ');
                }
            }
            let printable: String = bytes
                .iter()
                .map(|b| match b {
                    0x20..=0x7e => *b as char,
                    _ => '.',
                })
                .collect();
            out.push(Row {
                offset,
                text: format!("{offset:08X}  {hex}|{printable}|"),
                hits: Vec::new(),
                kind: RowKind::Hex,
                spans: Vec::new(),
            });
            offset += bytes.len() as u64;
        }
        out
    }

    fn rows_dump(&mut self, width: u16, height: u16) -> Vec<Row> {
        let per_row = width.max(1) as usize;
        let len = self.source.len();
        let mut out = Vec::with_capacity(height as usize);
        let mut offset = self.top;
        while out.len() < height as usize && offset < len {
            let bytes = self.source.read_vec(offset, per_row).unwrap_or_default();
            if bytes.is_empty() {
                break;
            }
            let text: String = self
                .codepage
                .decode(&bytes)
                .chars()
                .map(|c| {
                    if c == '\n' || c == '\r' || c == '\t' || c.is_control() {
                        '.'
                    } else {
                        c
                    }
                })
                .collect();
            out.push(Row {
                offset,
                text,
                hits: Vec::new(),
                kind: RowKind::Dump,
                spans: Vec::new(),
            });
            offset += bytes.len() as u64;
        }
        out
    }

    /// Search hits inside one line, as character ranges in the decoded text.
    fn hits_for(&self, bytes: &[u8], decoded: &str) -> Vec<(usize, usize)> {
        let Some(pattern) = &self.pattern else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for m in pattern.matches_in(bytes) {
            let start_bytes = m.offset as usize;
            let end_bytes = (start_bytes + m.len).min(bytes.len());
            let start = self.codepage.decode(&bytes[..start_bytes]).chars().count();
            let end = self.codepage.decode(&bytes[..end_bytes]).chars().count();
            // Tab expansion shifts the columns; recompute against the expanded text.
            let start = expand_shift(decoded, bytes, start, self.tab_width);
            let end = expand_shift(decoded, bytes, end, self.tab_width);
            if end > start {
                out.push((start, end - start));
            }
        }
        out
    }

    /// Where the query matches a string we rendered ourselves, in characters.
    fn hits_in_text(&self, text: &str) -> Vec<(usize, usize)> {
        let Some(pattern) = &self.pattern else {
            return Vec::new();
        };
        let bytes = text.as_bytes();
        let mut out = Vec::new();
        for m in pattern.matches_in(bytes) {
            let from = m.offset as usize;
            let to = (from + m.len).min(bytes.len());
            if !text.is_char_boundary(from) || !text.is_char_boundary(to) {
                continue;
            }
            let start = text[..from].chars().count();
            let end = text[..to].chars().count();
            if end > start {
                out.push((start, end - start));
            }
        }
        out
    }

    // ── search ─────────────────────────────────────────────────────────────────────────────

    /// Compile a query and jump to the first match from the top of the screen.
    pub fn search(
        &mut self,
        query: &str,
        mode: SearchMode,
        case_insensitive: bool,
        backwards: bool,
        cancel: &CancelToken,
    ) -> Result<bool, SearchError> {
        self.search_with(query, mode, case_insensitive, false, backwards, cancel)
    }

    /// The same, with whole words as the dialog offers them.
    pub fn search_with(
        &mut self,
        query: &str,
        mode: SearchMode,
        case_insensitive: bool,
        whole_words: bool,
        backwards: bool,
        cancel: &CancelToken,
    ) -> Result<bool, SearchError> {
        let pattern =
            crate::search::compile_with(query, mode, case_insensitive, whole_words, self.codepage)?;
        self.pattern = Some(pattern);
        self.query = Some(query.to_string());
        Ok(self.step(backwards, cancel))
    }

    /// Search the *instructions* rather than the bytes (SPEC 05 §2.7's other half, and what
    /// `docs/02-features.md` §5b calls searching by instruction pattern).
    ///
    /// In disassembly mode this is the only search that means anything: nobody looking for a
    /// `call` wants the four letters of it to appear in the file. The query matches the text
    /// the disassembler prints — `call`, `jmp 0x401000`, `mov eax` — case-insensitively,
    /// with `*` standing for anything, so `mov *, eax` finds every move into that register.
    ///
    /// It walks instruction by instruction from where it is, decoding a window at a time,
    /// because an instruction stream has no index to jump about in.
    pub fn search_instructions(
        &mut self,
        query: &str,
        backwards: bool,
        cancel: &CancelToken,
    ) -> bool {
        let needle = query.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return false;
        }
        self.query = Some(query.to_string());
        self.pattern = None;
        let arch = self.disasm_arch();
        if !arch.disassemblable() {
            return false;
        }
        let len = self.len();
        let from = match (self.last_match, backwards) {
            (Some(m), false) => m.offset + 1,
            (Some(m), true) => m.offset.saturating_sub(1),
            (None, _) => self.top,
        };

        // A window at a time, forwards from here or backwards to the start.
        const WINDOW: usize = 64 * 1024;
        let mut at = if backwards {
            from.saturating_sub(WINDOW as u64)
        } else {
            from
        };
        loop {
            if cancel.is_cancelled() {
                return false;
            }
            let window = self.source.read_vec(at, WINDOW).unwrap_or_default();
            if window.is_empty() {
                return false;
            }
            let base = self
                .binary
                .as_ref()
                .map(|b| b.va_for_offset(at))
                .unwrap_or(at);
            // No limit: every instruction in the window is a candidate.
            let lines =
                disasm::disassemble(&window, at, base, arch, usize::MAX).unwrap_or_default();
            let hit = if backwards {
                lines
                    .iter()
                    .rev()
                    .find(|l| l.offset < from && matches(&l.text, &needle))
            } else {
                lines
                    .iter()
                    .find(|l| l.offset >= from && matches(&l.text, &needle))
            };
            if let Some(line) = hit {
                self.last_match = Some(Match {
                    offset: line.offset,
                    len: line.bytes.len(),
                });
                self.jump(line.offset);
                return true;
            }
            if backwards {
                if at == 0 {
                    return false;
                }
                at = at.saturating_sub(WINDOW as u64);
            } else {
                // Overlap by the longest instruction, so one straddling the edge is still
                // found — and never read past the end.
                at += WINDOW as u64 - 16;
                if at >= len {
                    return false;
                }
            }
        }
    }

    /// `Shift+F7` / `Alt+F7`: the same query again. In disassembly mode that means the next
    /// instruction matching the pattern, because that is what the search was.
    pub fn find_again(&mut self, backwards: bool, cancel: &CancelToken) -> bool {
        if self.pattern.is_none() {
            return match (self.mode, self.query.clone()) {
                (Mode::Disasm, Some(query)) => self.search_instructions(&query, backwards, cancel),
                _ => false,
            };
        }
        self.step(backwards, cancel)
    }

    fn step(&mut self, backwards: bool, cancel: &CancelToken) -> bool {
        let Some(pattern) = self.pattern.clone() else {
            return false;
        };
        let from = match (self.last_match, backwards) {
            (Some(m), false) => m.offset + 1,
            (Some(m), true) => m.offset,
            (None, false) => self.top,
            (None, true) => self.top,
        };
        match find(&mut *self.source, &pattern, from, backwards, cancel) {
            Some(m) => {
                self.last_match = Some(m);
                self.jump(m.offset);
                true
            }
            None => false,
        }
    }

    pub fn clear_search(&mut self) {
        self.pattern = None;
        self.query = None;
        self.last_match = None;
    }

    pub fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    // ── follow ─────────────────────────────────────────────────────────────────────────────

    pub fn toggle_follow(&mut self) {
        self.follow = !self.follow;
    }

    /// Called on a timer: if the file grew and we are following, show the new end.
    pub fn follow_tick(&mut self, width: u16, height: u16) -> bool {
        if !self.follow {
            return false;
        }
        let len = self.source.len();
        let last = self.top;
        self.to_end(width, height);
        self.history.pop(); // following is not a jump the user wants to undo
        let _ = len;
        self.top != last
    }

    // ── status ─────────────────────────────────────────────────────────────────────────────

    pub fn status(&mut self) -> Status {
        let table = self.table_note();
        let len = self.source.len();
        let offset = self.top;
        let percent = match len {
            0 => 100,
            len => ((offset.min(len) * 100) / len) as u16,
        };
        Status {
            name: self.path.display_name(),
            mode: self.mode,
            wrap: self.wrap,
            codepage: self.codepage.name(),
            offset,
            len,
            percent,
            follow: self.follow,
            search: self.query.clone(),
            bookmarks: self
                .bookmarks
                .iter()
                .enumerate()
                .filter_map(|(i, b)| b.map(|_| i))
                .collect(),
            binary: match (&self.binary, self.raw_disasm) {
                (Some(binary), _) => Some(disasm::summary(binary)),
                (None, Some(limit)) => Some(format!(
                    "raw {}{} sweep (over the {limit} MiB parse limit)",
                    self.disasm_arch().as_str(),
                    if disasm::host_arch().disassemblable() {
                        ""
                    } else {
                        " (assumed)"
                    }
                )),
                _ => None,
            },
            derived: self.derived.clone(),
            table,
        }
    }
}

fn clamp_offset(current: u64, delta: i64, len: u64) -> u64 {
    if delta >= 0 {
        current.saturating_add(delta as u64).min(len)
    } else {
        current.saturating_sub((-delta) as u64)
    }
}

/// Columns one hex row occupies for `bpr` bytes.
fn hex_row_width(bpr: usize) -> u16 {
    // `00000000  ` + `XX ` each + a space every eight + `|` + the printables + `|`.
    (8 + 2 + bpr * 3 + bpr / 8 + 1 + bpr + 1) as u16
}

/// Cut a string to `cols` display columns without the ellipsis a user would mistake for data.
fn truncate_to(text: &str, cols: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if used + w > cols {
            break;
        }
        out.push(ch);
        used += w;
    }
    out
}

/// Expand tabs to the next tab stop, so columns line up as the file's author intended.
pub fn expand_tabs(text: &str, tab_width: usize) -> String {
    let tab_width = tab_width.max(1);
    if !text.contains('\t') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut column = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let spaces = tab_width - (column % tab_width);
            out.extend(std::iter::repeat_n(' ', spaces));
            column += spaces;
        } else {
            out.push(ch);
            column += ch.width().unwrap_or(0);
        }
    }
    out
}

/// Split a line into segments of at most `width` display columns.
pub fn wrap_segments(text: &str, width: usize, wrap: Wrap) -> Vec<String> {
    if wrap == Wrap::Off || width == 0 {
        return vec![text.to_string()];
    }
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut columns = 0usize;
    let mut last_space: Option<usize> = None;

    for ch in text.chars() {
        let w = ch.width().unwrap_or(0);
        if columns + w > width && !current.is_empty() {
            if wrap == Wrap::Word {
                if let Some(space) = last_space {
                    // Break at the last space instead of mid-word.
                    let (head, tail) = current.split_at(space);
                    let tail = tail.trim_start().to_string();
                    segments.push(head.trim_end().to_string());
                    current = tail;
                    columns = current.chars().filter_map(|c| c.width()).sum();
                    last_space = None;
                } else {
                    segments.push(std::mem::take(&mut current));
                    columns = 0;
                }
            } else {
                segments.push(std::mem::take(&mut current));
                columns = 0;
            }
        }
        if ch == ' ' {
            last_space = Some(current.len());
        }
        current.push(ch);
        columns += w;
    }
    segments.push(current);
    if segments.is_empty() {
        segments.push(String::new());
    }
    segments
}

/// Move highlight ranges into a window that starts at `from` and is `len` characters wide.
fn shift_hits(hits: &[(usize, usize)], from: usize, len: usize) -> Vec<(usize, usize)> {
    hits.iter()
        .filter_map(|(start, count)| {
            let end = start + count;
            if end <= from || *start >= from + len {
                return None;
            }
            let s = start.saturating_sub(from);
            let e = (end - from).min(len);
            (e > s).then_some((s, e - s))
        })
        .collect()
}

/// Map a character index in the raw decode onto the tab-expanded text.
fn expand_shift(expanded: &str, raw_bytes: &[u8], char_index: usize, tab_width: usize) -> usize {
    if !raw_bytes.contains(&b'\t') {
        return char_index;
    }
    // Count how many expanded characters correspond to the first `char_index` raw characters.
    let raw = String::from_utf8_lossy(raw_bytes);
    let mut column = 0usize;
    for (i, ch) in raw.chars().enumerate() {
        if i >= char_index {
            break;
        }
        if ch == '\t' {
            column += tab_width - (column % tab_width);
        } else {
            column += ch.width().unwrap_or(0);
        }
    }
    column.min(expanded.chars().count())
}

/// Does the instruction text match the pattern? The pieces of the pattern have to appear in
/// the text, in order; `*` is what stands between them, and a pattern with no `*` is simply a
/// substring. So `call` finds every call, `mov *, eax` every move into that register, and
/// `cmpsb` finds `repe cmpsb` too — a reader searching for an instruction is not thinking
/// about what prefixes it.
fn matches(text: &str, needle: &str) -> bool {
    // The disassembler pads its operands into columns; the pattern should not have to know.
    let text: String = text
        .to_ascii_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let mut at = 0usize;
    for part in needle.split('*') {
        if part.is_empty() {
            continue;
        }
        match text[at..].find(part) {
            Some(found) => at += found + part.len(),
            None => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::BytesSource;

    fn viewer(bytes: &[u8]) -> Viewer {
        Viewer::new(
            Box::new(BytesSource::new(bytes.to_vec())),
            VPath::local("/tmp/sample"),
        )
    }

    fn texts(v: &mut Viewer, w: u16, h: u16) -> Vec<String> {
        v.rows(w, h).into_iter().map(|r| r.text).collect()
    }

    /// A viewer on a Markdown file, which is what `renders_markdown` asks about.
    fn markdown_viewer(text: &str) -> Viewer {
        Viewer::new(
            Box::new(BytesSource::new(text.as_bytes().to_vec())),
            VPath::local("/tmp/read.md"),
        )
    }

    /// A viewer on a tabular file, which is what `reads_table` asks about.
    fn table_viewer(name: &str, text: &str) -> Viewer {
        Viewer::new(
            Box::new(BytesSource::new(text.as_bytes().to_vec())),
            VPath::local(format!("/tmp/{name}")),
        )
    }

    #[test]
    fn a_csv_reads_as_a_table_with_a_heading_that_stays_put() {
        let mut v = table_viewer("people.csv", "name,age\nmb,51\nada,36\nlee,8\n");
        assert!(v.reads_table());
        assert!(v.as_table());
        assert_eq!(v.mode, Mode::Table);
        assert_eq!(v.status().mode.letter(), 'b');

        let rows = texts(&mut v, 40, 5);
        assert_eq!(rows[0], "name │ age");
        assert!(rows[1].starts_with("────"), "{:?}", rows[1]);
        assert_eq!(rows[2], "mb   │ 51 ");
        assert_eq!(rows[4], "lee  │ 8  ");

        // Scrolling moves the rows under a heading that stays where it is.
        v.scroll(1, 40, 5);
        let rows = texts(&mut v, 40, 5);
        assert_eq!(rows[0], "name │ age");
        assert_eq!(rows[2], "ada  │ 36 ");
        // And `top` follows the record, so a bookmark or a search means the same thing here.
        assert_eq!(v.top, "name,age\nmb,51\n".len() as u64);

        // The status line says what it was read as, and admits the delimiter was a guess.
        let note = v.status().table.expect("a note");
        assert!(note.contains("CSV"), "{note}");
        assert!(note.contains('`'), "the delimiter should be named: {note}");
        assert!(note.contains("3 × 2"), "{note}");

        // `F2` goes back to the commas.
        v.toggle_wrap();
        assert_eq!(v.mode, Mode::Text);
    }

    #[test]
    fn json_lines_read_as_a_table_and_a_plain_text_file_does_not() {
        let mut v = table_viewer("log.jsonl", "{\"at\":\"now\",\"ok\":true}\n");
        assert!(v.as_table());
        let rows = texts(&mut v, 40, 3);
        assert_eq!(rows[0], "at  │ ok  ");
        assert_eq!(rows[2], "now │ true");

        // A name that says nothing tabular does not *open* as a table, but the key still
        // offers it: tab-separated columns in a `.txt` are as common as in a `.tsv`, and the
        // reader can see that the file is a table even where its name cannot.
        let mut v = table_viewer("notes.txt", "name\tage\nmb\t51\n");
        assert!(!v.reads_table(), "it does not open as one");
        assert!(v.as_table(), "but the key puts it in table mode");
        assert_eq!(v.mode, Mode::Table);
        assert_eq!(texts(&mut v, 40, 3)[2], "mb   │ 51 ");

        // A file of prose is not a table, and saying so is better than drawing one.
        let mut v = table_viewer("readme.txt", "This is a sentence.\nAnd another one.\n");
        assert!(!v.as_table());
        assert_eq!(v.mode, Mode::Text);
    }

    #[test]
    fn a_dbase_table_opens_as_a_table_however_large_it_says_it_is() {
        // The same hand-built table the reader's own test uses.
        let bytes = crate::table::tests::dbase_bytes();
        let mut v = Viewer::new(
            Box::new(BytesSource::new(bytes)),
            VPath::local("/tmp/people.dbf"),
        );
        assert!(v.reads_table());
        assert!(v.as_table());
        let rows = texts(&mut v, 40, 5);
        assert_eq!(rows[0], "* │ NAME       │ BORN      ");
        assert_eq!(rows[2], "  │ mb         │ 1975-03-12");
        assert_eq!(rows[3], "* │ gone       │           ");
        let note = v.status().table.expect("a note");
        assert!(note.contains("dBase III"), "{note}");
        assert!(note.contains("3 × 2"), "{note}");
        // No delimiter was guessed, because a dBase table has none to guess.
        assert!(!note.contains('`'), "{note}");
    }

    #[test]
    fn a_table_too_large_to_read_whole_is_not_offered_the_mode() {
        let mut v = table_viewer("small.csv", "a,b\nx,y\n");
        assert!(v.reads_table());
        let huge = "a,b\n".to_string() + &"1,2\n".repeat((TABLE_LIMIT as usize / 4) + 1);
        let mut v = table_viewer("huge.csv", &huge);
        assert!(!v.reads_table(), "a file over the limit opens as text");
    }

    #[test]
    fn a_markdown_file_can_be_laid_out_and_scrolls_by_rendered_lines() {
        let mut v = markdown_viewer("# One\n\nbody\n\n## Two\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
        assert!(v.renders_markdown());
        assert!(v.as_markdown());
        assert_eq!(v.mode, Mode::Markdown);
        assert_eq!(v.status().mode.letter(), 'm');
        assert_eq!(texts(&mut v, 40, 2), vec!["One", "═══"]);

        // Scrolling moves by rendered lines, and comes back to exactly where it was. The
        // blank line between the heading and the body is a line like any other.
        v.scroll(2, 40, 2);
        assert_eq!(texts(&mut v, 40, 2), vec!["", "body"]);
        v.scroll(1, 40, 2);
        assert_eq!(texts(&mut v, 40, 1), vec!["body"]);
        v.scroll(-3, 40, 2);
        assert_eq!(texts(&mut v, 40, 2), vec!["One", "═══"]);

        // Past the start and past the end both stop rather than going blank.
        v.scroll(-50, 40, 2);
        assert_eq!(texts(&mut v, 40, 1), vec!["One"]);
        v.scroll(500, 40, 2);
        assert_eq!(
            texts(&mut v, 40, 1),
            vec!["└───┴───┘"],
            "the last line of the table is the last line of the document"
        );

        // `End` shows the last screenful, as it does in text mode.
        v.to_start();
        v.to_end(40, 3);
        let tail = texts(&mut v, 40, 3);
        assert!(tail.iter().any(|l| l.starts_with('└')), "{tail:?}");

        // `F2` out of it is the source, marks and all.
        v.to_start();
        v.toggle_wrap();
        assert_eq!(v.mode, Mode::Text);
        assert!(texts(&mut v, 40, 1)[0].starts_with("# One"));
    }

    #[test]
    fn only_a_markdown_file_small_enough_to_render_is_laid_out() {
        // The name decides, as it does for the syntax colouring.
        let mut v = viewer(b"# not markdown\n");
        assert!(!v.renders_markdown());
        assert!(!v.as_markdown(), "a .txt stays text");
        assert_eq!(v.mode, Mode::Text);

        // And so does the size: rendering reads the whole file, so there is a limit on it.
        let big = format!("# big\n\n{}\n", "x".repeat(MARKDOWN_LIMIT as usize + 1));
        let mut v = markdown_viewer(&big);
        assert!(!v.renders_markdown());
        assert!(!v.as_markdown());
    }

    #[test]
    fn searching_a_laid_out_document_finds_what_is_on_screen() {
        let mut v = markdown_viewer("| name | size |\n|---|---|\n| a | 1 |\n");
        assert!(v.as_markdown());
        // `size` is in a cell. In the source it sits between pipes; on screen it is a word,
        // and the hit has to be marked where the word is.
        v.search("size", SearchMode::Plain, false, false, &CancelToken::new())
            .expect("the search compiles");
        let row = v
            .rows(40, 10)
            .into_iter()
            .find(|r| r.text.contains("size"))
            .expect("the header is on screen");
        let (start, len) = row.hits.first().copied().expect("the hit is marked");
        let hit: String = row.text.chars().skip(start).take(len).collect();
        assert_eq!(hit, "size");
    }

    #[test]
    fn text_mode_shows_lines_and_scrolls_by_rows() {
        let mut v = viewer(b"one\ntwo\nthree\nfour\nfive\n");
        assert_eq!(texts(&mut v, 20, 3), vec!["one", "two", "three"]);
        v.scroll(2, 20, 3);
        assert_eq!(texts(&mut v, 20, 3), vec!["three", "four", "five"]);
        v.scroll(-1, 20, 3);
        assert_eq!(texts(&mut v, 20, 1), vec!["two"]);
        v.scroll(-10, 20, 3);
        assert_eq!(
            v.top, 0,
            "scrolling up past the start must stop at the start"
        );
    }

    #[test]
    fn end_shows_the_last_screenful_not_an_empty_screen() {
        let mut v = viewer(b"1\n2\n3\n4\n5\n6\n7\n8\n");
        v.to_end(20, 3);
        let rows = texts(&mut v, 20, 3);
        assert_eq!(rows.last().unwrap(), "8");
        assert_eq!(rows.len(), 3, "{rows:?}");
        v.to_start();
        assert_eq!(v.top, 0);
    }

    #[test]
    fn horizontal_scrolling_works_only_without_wrapping() {
        let mut v = viewer(b"abcdefghij\n");
        v.scroll_h(3);
        assert_eq!(texts(&mut v, 4, 1), vec!["defg"]);
        v.toggle_wrap(); // Off -> Char
        assert_eq!(v.wrap, Wrap::Char);
        assert_eq!(v.col, 0, "turning wrapping on resets the horizontal offset");
        v.scroll_h(3);
        assert_eq!(
            v.col, 0,
            "with wrapping there is nothing to scroll sideways"
        );
    }

    #[test]
    fn character_wrapping_splits_a_long_line() {
        let mut v = viewer(b"abcdefghij\nnext\n");
        v.wrap = Wrap::Char;
        assert_eq!(texts(&mut v, 4, 4), vec!["abcd", "efgh", "ij", "next"]);
        // Scrolling moves one *display* row, staying inside the long line.
        v.scroll(1, 4, 4);
        assert_eq!(texts(&mut v, 4, 1), vec!["efgh"]);
    }

    #[test]
    fn word_wrapping_does_not_break_words() {
        let mut v = viewer(b"the quick brown fox jumps\n");
        v.wrap = Wrap::Word;
        let rows = texts(&mut v, 10, 4);
        assert!(rows.iter().all(|r| r.len() <= 10), "{rows:?}");
        assert!(
            rows[0].ends_with("quick") || rows[0].ends_with("the"),
            "{rows:?}"
        );
        assert!(!rows.join("|").contains("qui|"), "a word was cut: {rows:?}");
    }

    #[test]
    fn tabs_line_up_on_tab_stops() {
        let mut v = viewer(b"a\tb\n");
        assert_eq!(texts(&mut v, 20, 1), vec!["a       b"]);
    }

    #[test]
    fn a_binary_file_opens_in_dump_mode() {
        let mut bytes = vec![0x7f, b'E', b'L', b'F'];
        bytes.extend(std::iter::repeat_n(0u8, 200));
        let mut v = viewer(&bytes);
        assert_eq!(v.mode, Mode::Dump, "a binary should not pretend to be text");
        let rows = v.rows(16, 2);
        assert_eq!(rows.len(), 2);
        assert!(rows[0].text.chars().count() <= 16);
        assert!(
            rows[0].text.contains('.'),
            "control bytes should show as dots"
        );
    }

    #[test]
    fn hex_mode_shows_offsets_bytes_and_ascii() {
        let mut v = viewer(b"Hello, hex!");
        v.toggle_hex();
        assert_eq!(v.mode, Mode::Hex);
        let rows = v.rows(80, 2);
        assert!(
            rows[0].text.starts_with("00000000  48 65 6C 6C 6F"),
            "{}",
            rows[0].text
        );
        assert!(rows[0].text.contains("|Hello, hex!|"), "{}", rows[0].text);
        // F4 goes back to the mode we came from.
        v.toggle_hex();
        assert_eq!(v.mode, Mode::Text);
    }

    #[test]
    fn a_hex_row_never_overruns_the_window() {
        let mut v = viewer(&(0u8..=255).collect::<Vec<_>>());
        v.set_mode(Mode::Hex);
        for width in [30u16, 45, 60, 78, 100] {
            let rows = v.rows(width, 4);
            for row in &rows {
                assert!(
                    row.text.chars().count() <= width as usize,
                    "width {width} produced a {} column row: {}",
                    row.text.chars().count(),
                    row.text
                );
                assert!(
                    row.text.ends_with('|'),
                    "the printable gutter was cut: {}",
                    row.text
                );
            }
        }
        // Wide enough for the setting and it is honoured; too narrow and it drops a group
        // of eight at a time, with four as the floor for a window too narrow even for one.
        assert_eq!(v.hex_fit(100), 16);
        assert_eq!(v.hex_fit(60), 8);
        assert_eq!(v.hex_fit(30), 4);
        v.bytes_per_row = 40;
        assert_eq!(v.hex_fit(1000), 40);
        assert_eq!(v.hex_fit(hex_row_width(32)), 32);
        assert_eq!(v.hex_fit(hex_row_width(24) + 2), 24);
        v.bytes_per_row = 16;
        // Never more than the setting, however much room there is.
        v.set_bytes_per_row(-1, 200);
        assert_eq!(v.bytes_per_row, 8);
        assert_eq!(v.hex_fit(200), 8);
    }

    /// The arrows in hex mode set the row width; `Ctrl` plus the arrows nudges the dump off
    /// the grid so a structure that starts at an odd offset can be read in line.
    #[test]
    fn a_hex_dump_can_be_widened_and_nudged() {
        let mut v = viewer(&(0..=255u8).collect::<Vec<u8>>());
        v.set_mode(Mode::Hex);
        assert_eq!(v.bytes_per_row, 16);

        // One press, one block of eight — the group the dump already draws a gap between —
        // and the screen follows every press, which is what the steps are for.
        let wide = 1000;
        for want in [24, 32, 40, 48, 56, 64] {
            v.set_bytes_per_row(1, wide);
            assert_eq!(v.bytes_per_row, want);
            assert_eq!(v.hex_fit(wide), want, "the screen followed the setting");
        }
        v.set_bytes_per_row(1, wide);
        assert_eq!(v.bytes_per_row, 64, "it stops at the widest it draws");

        for want in [56, 48, 40, 32, 24, 16, 8] {
            v.set_bytes_per_row(-1, wide);
            assert_eq!(v.bytes_per_row, want);
        }
        v.set_bytes_per_row(-1, wide);
        assert_eq!(
            v.bytes_per_row, 8,
            "eight is the narrowest the arrows offer"
        );

        // And it never climbs past what the window can hold: a press that would need a
        // wider screen than there is leaves the width where it was.
        v.bytes_per_row = 16;
        let narrow = hex_row_width(16);
        v.set_bytes_per_row(1, narrow);
        assert_eq!(
            v.bytes_per_row, 16,
            "32 does not fit in this window, so the press changes nothing"
        );
        v.set_bytes_per_row(1, hex_row_width(32));
        assert_eq!(v.bytes_per_row, 24, "and it does when the room is there");

        for _ in 0..20 {
            v.set_bytes_per_row(1, wide);
        }
        assert_eq!(v.bytes_per_row, 64);

        // A jump snaps to the row width, which is what keeps the offsets round...
        v.bytes_per_row = 16;
        v.goto(33);
        assert_eq!(v.top, 32);
        // ...and a nudge does not, which is the whole point of it.
        v.shift(1);
        assert_eq!(v.top, 33);
        v.shift(1);
        assert_eq!(v.top, 34);
        v.shift(-4);
        assert_eq!(v.top, 30);
        // It stops at the ends rather than wrapping or going negative.
        v.shift(-1000);
        assert_eq!(v.top, 0);
        v.shift(-1);
        assert_eq!(v.top, 0);
        v.shift(100_000);
        assert_eq!(v.top, 256);
    }

    #[test]
    fn hex_rows_follow_the_bytes_per_row_setting() {
        let mut v = viewer(&[0u8; 64]);
        v.set_mode(Mode::Hex);
        assert_eq!(v.bytes_per_row, 16);
        v.set_bytes_per_row(1, 1000);
        assert_eq!(v.bytes_per_row, 24);
        for _ in 0..100 {
            v.set_bytes_per_row(-1, 1000);
        }
        assert_eq!(v.bytes_per_row, 8, "it stops at the narrowest it offers");
        v.scroll(1, 80, 4);
        assert_eq!(v.top, 8);
    }

    #[test]
    fn search_moves_to_the_hit_and_highlights_it() {
        let mut v = viewer(b"alpha\nbeta\ngamma beta\n");
        let found = v
            .search("beta", SearchMode::Plain, false, false, &CancelToken::new())
            .unwrap();
        assert!(found);
        assert_eq!(v.last_match.unwrap().offset, 6);
        let rows = v.rows(20, 2);
        assert_eq!(rows[0].text, "beta");
        assert_eq!(rows[0].hits, [(0, 4)]);

        // The next hit is on the following line, highlighted at its own column.
        assert!(v.find_again(false, &CancelToken::new()));
        let rows = v.rows(20, 1);
        assert_eq!(rows[0].text, "gamma beta");
        assert_eq!(rows[0].hits, [(6, 4)]);

        // And back.
        assert!(v.find_again(true, &CancelToken::new()));
        assert_eq!(v.last_match.unwrap().offset, 6);
        assert!(
            !v.find_again(true, &CancelToken::new()),
            "no hit before the first"
        );
    }

    #[test]
    fn a_missing_search_leaves_the_position_alone() {
        let mut v = viewer(b"alpha\nbeta\n");
        v.scroll(1, 20, 2);
        let before = v.top;
        let found = v
            .search("zeta", SearchMode::Plain, false, false, &CancelToken::new())
            .unwrap();
        assert!(!found);
        assert_eq!(v.top, before);
    }

    #[test]
    fn highlights_survive_horizontal_scrolling_and_wrapping() {
        let mut v = viewer(b"xxxxxxxxxxNEEDLExxxxx\n");
        v.search(
            "NEEDLE",
            SearchMode::Plain,
            false,
            false,
            &CancelToken::new(),
        )
        .unwrap();
        v.goto(0);
        v.scroll_h(8);
        let rows = v.rows(10, 1);
        assert_eq!(rows[0].hits, [(2, 6)], "{:?}", rows[0]);

        v.wrap = Wrap::Char;
        v.col = 0;
        let rows = v.rows(10, 3);
        // The needle starts exactly at the second segment, so the first has no hit.
        assert!(rows[0].hits.is_empty(), "{:?}", rows[0]);
        assert_eq!(rows[1].hits, [(0, 6)], "{:?}", rows[1]);
    }

    #[test]
    fn goto_percent_and_offset_are_remembered_for_undo() {
        let mut v = viewer(b"0123456789\nabcdefghij\n");
        v.jump(11);
        assert_eq!(v.top, 11);
        v.goto_percent(0);
        assert_eq!(v.top, 0);
        assert!(v.undo_jump());
        assert_eq!(v.top, 11, "undo should come back to the previous position");
        assert!(v.undo_jump());
        assert_eq!(v.top, 0);
        assert!(!v.undo_jump());
    }

    #[test]
    fn bookmarks_store_and_recall_positions() {
        let mut v = viewer(b"a\nb\nc\nd\n");
        v.scroll(2, 20, 2);
        assert!(v.set_bookmark(3));
        v.to_start();
        assert!(v.goto_bookmark(3));
        assert_eq!(v.top, 4);
        assert!(!v.goto_bookmark(7), "an unset bookmark does nothing");
        assert!(!v.set_bookmark(99));
        assert_eq!(v.status().bookmarks, [3]);
    }

    #[test]
    fn follow_mode_shows_what_was_appended() {
        let mut source = BytesSource::new(b"line 1\n".to_vec());
        source.append(b"line 2\n");
        let mut v = Viewer::new(Box::new(source), VPath::local("/tmp/log"));
        assert!(!v.follow);
        v.toggle_follow();
        assert!(v.follow_tick(20, 1) || v.top > 0);
        let rows = texts(&mut v, 20, 1);
        assert_eq!(rows, vec!["line 2"]);
    }

    #[test]
    fn the_status_line_reports_mode_codepage_and_position() {
        let mut v = viewer(b"hello\nworld\n");
        let s = v.status();
        assert_eq!(s.mode, Mode::Text);
        assert_eq!(s.codepage, "UTF-8");
        assert_eq!((s.offset, s.len, s.percent), (0, 12, 0));
        v.to_end(20, 1);
        let s = v.status();
        assert!(s.percent > 0, "{s:?}");
        v.next_codepage();
        assert_eq!(v.status().codepage, "UTF-16LE");
    }

    #[test]
    fn the_chrome_flags_start_as_what_the_viewer_draws_today() {
        let v = viewer(b"hello\n");
        assert!(v.show_key_bar, "the key bar is drawn today");
        assert!(v.show_status_line, "so is the status line");
        assert!(
            !v.show_scrollbar,
            "the viewer has never drawn a scrollbar, so asking for one is opting in"
        );
    }

    #[test]
    fn each_chrome_toggle_moves_only_its_own_flag() {
        let mut v = viewer(b"hello\n");
        assert!(!v.toggle_key_bar(), "Ctrl+B hides the key bar");
        assert!(v.show_status_line, "and leaves the status line alone");
        assert!(!v.show_scrollbar);
        assert!(v.toggle_key_bar(), "and brings it back");

        assert!(
            !v.toggle_status_line(),
            "Ctrl+Shift+B hides the status line"
        );
        assert!(v.show_key_bar);
        assert!(v.toggle_status_line());

        assert!(v.toggle_scrollbar(), "Ctrl+S asks for the scrollbar");
        assert!(v.show_key_bar);
        assert!(v.show_status_line);
        assert!(!v.toggle_scrollbar());
    }

    #[test]
    fn hiding_the_chrome_changes_nothing_the_viewer_itself_renders() {
        // The viewer keeps the answer and draws none of it: the rows are the rows either way,
        // so wiring the keys cannot change what the content pane shows.
        let mut v = viewer(b"one\ntwo\nthree\n");
        let before = texts(&mut v, 20, 3);
        v.show_key_bar = false;
        v.show_status_line = false;
        v.show_scrollbar = true;
        assert_eq!(texts(&mut v, 20, 3), before);
        assert_eq!(v.status().mode, Mode::Text);
    }

    #[test]
    fn a_huge_file_is_opened_and_navigated_without_reading_it_all() {
        // 8 MB of lines: opening, End and a page up must all be cheap.
        let line = b"the quick brown fox jumps over the lazy dog\n";
        let mut bytes = Vec::with_capacity(8 * 1024 * 1024);
        while bytes.len() < 8 * 1024 * 1024 {
            bytes.extend_from_slice(line);
        }
        let total = bytes.len() as u64;
        let mut v = viewer(&bytes);
        assert_eq!(v.len(), total);
        v.to_end(80, 25);
        assert!(v.top > total - 80 * 25, "End did not land near the end");
        v.scroll(-25, 80, 25);
        let rows = texts(&mut v, 80, 25);
        assert_eq!(rows.len(), 25);
        assert!(rows[0].starts_with("the quick"));
        assert_eq!(v.status().percent, 99);
    }

    #[test]
    fn an_empty_file_renders_nothing_and_says_so() {
        let mut v = viewer(b"");
        assert!(v.is_empty());
        assert!(v.rows(20, 5).is_empty());
        assert_eq!(v.status().percent, 100);
    }

    #[test]
    fn a_real_executable_disassembles_in_the_viewer() {
        let path = std::env::current_exe().unwrap();
        let mut v = Viewer::new(
            Box::new(crate::source::FileSource::open(&path).unwrap()),
            VPath::local(&path),
        );
        // A native binary opens in dump mode; asking for disassembly analyses it first.
        assert_eq!(v.mode, Mode::Dump);
        match v.start_disasm() {
            Ok(Started::Instructions) => {}
            Ok(Started::Derived(producer)) => {
                panic!("a native binary should give instructions, not {producer}")
            }
            Err(e) => {
                // On a non-x86 runner this is objdump's job, and the message says so.
                assert!(e.to_string().contains("external tool"), "{e}");
                return;
            }
        }
        assert_eq!(v.mode, Mode::Disasm);
        // A debug build of this test is often over the parse limit; then the viewer sweeps raw
        // bytes instead of following sections, and says so.
        match v.binary.as_ref() {
            Some(binary) => {
                assert!(binary.format.native_support());
                assert!(
                    v.top > 0,
                    "disassembly should start at the code, not the header"
                );
            }
            None => assert!(
                v.status().binary.unwrap_or_default().contains("raw"),
                "a raw sweep must be announced"
            ),
        }

        let rows = v.rows(100, 10);
        assert!(!rows.is_empty());
        let code_rows: Vec<&Row> = rows.iter().filter(|r| r.kind == RowKind::Code).collect();
        assert!(code_rows.len() >= 5, "{rows:?}");
        assert!(
            code_rows[0].text.contains("  "),
            "a row is address, bytes and mnemonic: {:?}",
            code_rows[0].text
        );
        for row in &rows {
            assert!(
                unicode_width::UnicodeWidthStr::width(row.text.as_str()) <= 100,
                "row wider than the screen: {:?}",
                row.text
            );
        }

        // Scrolling moves by whole instructions, forwards and back to where it started.
        let start = v.top;
        v.scroll(3, 100, 10);
        assert!(v.top > start);
        v.scroll(-3, 100, 10);
        assert_eq!(
            v.top, start,
            "scrolling back must land on the same instruction"
        );

        assert_eq!(v.status().mode, Mode::Disasm);
        assert!(
            v.status().binary.is_some(),
            "the status must say what the file is"
        );
    }

    #[test]
    fn an_instruction_pattern_matches_the_way_a_reader_writes_it() {
        assert!(matches("call rax", "call"));
        assert!(matches("CALL rax", "call"));
        // The disassembler pads its operands; the pattern should not have to.
        assert!(matches("mov    eax, 1", "mov eax, 1"));
        assert!(!matches("mov eax, 1", "call"));
        // `*` stands for anything, so a register can be asked for without its source.
        assert!(matches("mov ebx, eax", "mov *, eax"));
        assert!(!matches("mov eax, ebx", "mov *, eax"));
        assert!(matches("jne 0x401000", "jne *"));
        assert!(matches("lock xchg eax, ebx", "*xchg*"));
        assert!(!matches("xor eax, eax", "*xchg*"));
        // A plain query is a substring: someone looking for `cmpsb` means the one with a
        // `repe` in front of it as well.
        assert!(matches("repe cmpsb", "cmpsb"));
        // The pieces have to come in the pattern's own order, though.
        assert!(!matches("mov eax, ebx", "ebx*mov"));
    }

    #[test]
    fn searching_the_disassembly_finds_an_instruction_not_its_letters() {
        // push rbp / mov rbp,rsp / xor eax,eax / nop*8 / call rax / ret
        let mut code = vec![0x55, 0x48, 0x89, 0xE5, 0x31, 0xC0];
        code.extend(std::iter::repeat_n(0x90u8, 8));
        code.extend_from_slice(&[0xFF, 0xD0, 0xC3]);
        let mut v = viewer(&code);
        if v.start_disasm().is_err() {
            return; // no x86 decoder on this target: the external-tool path is tested elsewhere
        }
        assert_eq!(v.mode, Mode::Disasm);
        let cancel = CancelToken::new();

        // The bytes `call` appear nowhere in the file, but the instruction does.
        assert!(
            v.search_instructions("call", false, &cancel),
            "no call found"
        );
        assert_eq!(v.last_match.unwrap().offset, 14);

        // From the top again, a pattern with a wildcard.
        v.clear_search();
        v.jump(0);
        assert!(v.search_instructions("mov *, rsp", false, &cancel));
        assert_eq!(v.last_match.unwrap().offset, 1);

        // Repeating the search walks forwards: the eight nops, one after another.
        v.clear_search();
        v.jump(0);
        assert!(v.search_instructions("nop", false, &cancel));
        let first = v.last_match.unwrap().offset;
        assert_eq!(first, 6);
        assert!(
            v.find_again(false, &cancel),
            "Shift+F7 must find the next nop"
        );
        assert_eq!(v.last_match.unwrap().offset, 7);
        // And backwards, back to the first one.
        assert!(v.find_again(true, &cancel));
        assert_eq!(v.last_match.unwrap().offset, 6);

        // Something that is not in the code at all leaves the search empty.
        v.clear_search();
        v.jump(0);
        assert!(!v.search_instructions("syscall", false, &cancel));
        assert!(!v.search_instructions("", false, &cancel));
    }

    #[test]
    fn disassembly_of_known_bytes_shows_addresses_and_mnemonics() {
        // Not a container: a raw code fragment is disassembled from offset 0.
        let mut v = viewer(&[0x55, 0x48, 0x89, 0xE5, 0x31, 0xC0, 0x5D, 0xC3]);
        v.mode = Mode::Disasm;
        let rows = v.rows(80, 8);
        let joined = rows
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(joined.contains("push"), "{joined}");
        assert!(joined.contains("mov"), "{joined}");
        assert!(joined.contains("ret"), "{joined}");
        assert!(joined.contains("0000000000000000"), "{joined}");
        assert!(joined.contains("48 89 E5"), "{joined}");
    }

    #[test]
    fn raw_bytes_are_decoded_with_an_architecture_we_have_a_decoder_for() {
        // On a host we cannot decode (an ARM Mac, say) raw bytes still disassemble, as x86-64,
        // and the status line calls that an assumption. Without this, `F5` on a code fragment
        // would do nothing at all on Apple Silicon.
        let v = viewer(&[0x90]);
        assert!(
            v.disasm_arch().disassemblable(),
            "the fallback architecture must have a decoder"
        );
        if !disasm::host_arch().disassemblable() {
            assert_eq!(v.disasm_arch(), Arch::X86_64);
        }
    }

    #[test]
    fn a_wasm_module_is_shown_as_its_text_format() {
        let wasm: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b,
        ];
        let mut v = viewer(wasm);
        assert_eq!(v.format(), crate::disasm::Format::Wasm);
        let text = v.wasm_text().expect("prints");
        v.show_derived("WAT", text);

        assert_eq!(v.mode, Mode::Text);
        assert_eq!(v.derived.as_deref(), Some("WAT"));
        assert_eq!(v.status().derived.as_deref(), Some("WAT"));
        let rows = texts(&mut v, 80, 10);
        assert!(rows[0].contains("(module"), "{rows:?}");
        assert!(rows.iter().any(|r| r.contains("i32.const 42")), "{rows:?}");

        // Searching works on the generated text like on any file.
        assert!(v
            .search(
                "i32.const",
                SearchMode::Plain,
                false,
                false,
                &CancelToken::new()
            )
            .unwrap());
    }

    #[test]
    fn asking_for_disassembly_of_a_text_file_is_refused_with_a_reason() {
        let mut v = viewer(b"#!/bin/sh\necho hi\n");
        let err = v.start_disasm().unwrap_err();
        assert!(err.to_string().contains("parse"), "{err}");
        assert_eq!(v.mode, Mode::Text, "the mode must not change on failure");
    }

    #[test]
    fn mode_cycling_keeps_the_position() {
        let mut v = viewer(b"0123456789\nabcdefghij\nABCDEFGHIJ\n");
        v.goto(11);
        v.set_mode(Mode::Hex);
        assert_eq!(v.top, 0, "hex snaps to the row boundary (16 bytes)");
        v.goto(16);
        v.set_mode(Mode::Dump);
        assert_eq!(v.top, 16);
        v.set_mode(Mode::Text);
        assert_eq!(v.top, 11, "text snaps back to the start of the line");
    }

    #[test]
    fn utf16_and_cp1252_files_are_decoded_in_the_viewer() {
        let mut utf16 = vec![0xFF, 0xFE];
        for c in "wide\n".chars() {
            utf16.push(c as u8);
            utf16.push(0);
        }
        let v = viewer(&utf16);
        assert_eq!(v.codepage, Codepage::Utf16Le);

        let mut v = viewer(b"caf\xe9\n");
        assert_eq!(v.codepage, Codepage::Cp1252);
        assert_eq!(texts(&mut v, 20, 1), vec!["café"]);
    }
}
