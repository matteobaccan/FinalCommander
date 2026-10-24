use scmd_core::{CancelToken, VPath};
use unicode_width::UnicodeWidthChar;

use crate::decode::{detect, looks_binary, Codepage};
use crate::disasm::{self, Arch, Binary, DisasmError, Format};
use crate::nav::{next_line_start, prev_line_start, read_line, snap_to_line};
use crate::search::{compile, find, Match, Pattern, SearchError, SearchMode};
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
}

impl Mode {
    /// The letter the status line shows, as FAR does.
    pub fn letter(self) -> char {
        match self {
            Mode::Text => 't',
            Mode::Dump => 'd',
            Mode::Hex => 'h',
            Mode::Disasm => 'a',
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Text => "text",
            Mode::Dump => "dump",
            Mode::Hex => "hex",
            Mode::Disasm => "disasm",
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
}

/// Tabs expand to the next multiple of this.
const TAB_WIDTH: usize = 8;

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
    /// Structure of the compiled file, once we have looked.
    pub binary: Option<Binary>,
    /// Is the structure pane open (SPEC 05 §1.8)?
    pub show_structure: bool,
    /// The decoded shape of the file: `None` = not looked yet, `Some(None)` = looked and it is
    /// not a format the pane knows, which is the common case.
    structure: Option<Option<Structure>>,
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
            binary: None,
            show_structure: false,
            structure: None,
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

    pub fn len(&mut self) -> u64 {
        self.source.len()
    }

    pub fn is_empty(&mut self) -> bool {
        self.source.len() == 0
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
            Mode::Dump => self.set_mode(Mode::Text),
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

    pub fn set_bytes_per_row(&mut self, delta: i32) {
        let next = (self.bytes_per_row as i32 + delta).clamp(4, 64) as usize;
        self.bytes_per_row = next;
        let top = self.top;
        self.goto(top);
    }

    // ── position ───────────────────────────────────────────────────────────────────────────

    pub fn goto(&mut self, offset: u64) {
        self.disasm_trail.clear();
        let len = self.source.len();
        let offset = offset.min(len);
        self.top = match self.mode {
            Mode::Text => snap_to_line(&mut *self.source, offset),
            Mode::Hex => offset - (offset % self.bytes_per_row as u64),
            Mode::Dump | Mode::Disasm => offset,
        };
        self.top_sub = 0;
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
    }

    /// The last screenful, so `End` shows the end of the file rather than an empty screen.
    pub fn to_end(&mut self, width: u16, height: u16) {
        self.history.push((self.top, self.top_sub));
        let len = self.source.len();
        self.goto(len);
        self.scroll(-(height as i64), width, height);
    }

    pub fn goto_percent(&mut self, percent: u16) {
        let len = self.source.len();
        let offset = len * percent.min(100) as u64 / 100;
        self.jump(offset);
    }

    pub fn scroll_h(&mut self, delta: i64) {
        if self.mode == Mode::Text && self.wrap == Wrap::Off {
            self.col = (self.col as i64 + delta).max(0) as usize;
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
                let step = self.bytes_per_row as i64 * delta;
                self.top = clamp_offset(self.top, step, self.source.len());
                self.top -= self.top % self.bytes_per_row as u64;
            }
            Mode::Dump => {
                let step = width.max(1) as i64 * delta;
                self.top = clamp_offset(self.top, step, self.source.len());
            }
        }
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
        let text = expand_tabs(&self.codepage.decode(&bytes));
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
        }
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
        let lines = disasm::disassemble(&window, self.top, base_va, arch, height as usize)
            .unwrap_or_default();

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
                    });
                }
            }
            let hex: String = line
                .bytes
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<Vec<_>>()
                .join(" ");
            let text = format!("{:016X}  {:<23}  {}", line.address, hex, line.text);
            out.push(Row {
                offset: line.offset,
                text: truncate_to(&text, width as usize),
                hits: Vec::new(),
                kind: if line.invalid {
                    RowKind::Dump
                } else {
                    RowKind::Code
                },
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
            let decoded = expand_tabs(&self.codepage.decode(&bytes));
            let hits = self.hits_for(&bytes, &decoded);

            if self.wrap == Wrap::Off {
                let text: String = decoded.chars().skip(self.col).take(width).collect();
                let shifted = shift_hits(&hits, self.col, width);
                out.push(Row {
                    offset,
                    text,
                    hits: shifted,
                    kind: RowKind::Text,
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

    fn rows_hex(&mut self, _width: u16, height: u16) -> Vec<Row> {
        let bpr = self.bytes_per_row;
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
            let start = expand_shift(decoded, bytes, start);
            let end = expand_shift(decoded, bytes, end);
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
        let pattern = compile(query, mode, case_insensitive, self.codepage)?;
        self.pattern = Some(pattern);
        self.query = Some(query.to_string());
        Ok(self.step(backwards, cancel))
    }

    /// `Shift+F7` / `Alt+F7`: the same query again.
    pub fn find_again(&mut self, backwards: bool, cancel: &CancelToken) -> bool {
        if self.pattern.is_none() {
            return false;
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
pub fn expand_tabs(text: &str) -> String {
    if !text.contains('\t') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 8);
    let mut column = 0usize;
    for ch in text.chars() {
        if ch == '\t' {
            let spaces = TAB_WIDTH - (column % TAB_WIDTH);
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
fn expand_shift(expanded: &str, raw_bytes: &[u8], char_index: usize) -> usize {
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
            column += TAB_WIDTH - (column % TAB_WIDTH);
        } else {
            column += ch.width().unwrap_or(0);
        }
    }
    column.min(expanded.chars().count())
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
    fn hex_rows_follow_the_bytes_per_row_setting() {
        let mut v = viewer(&[0u8; 64]);
        v.set_mode(Mode::Hex);
        assert_eq!(v.bytes_per_row, 16);
        v.set_bytes_per_row(8);
        assert_eq!(v.bytes_per_row, 24);
        v.set_bytes_per_row(-100);
        assert_eq!(v.bytes_per_row, 4, "clamped, not negative");
        v.scroll(1, 80, 4);
        assert_eq!(v.top, 4);
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
