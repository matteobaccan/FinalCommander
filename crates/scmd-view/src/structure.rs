//! What a binary *is*, and where anything inside it lives — `docs/spec/05-viewer-editor.md`
//! §1.8.
//!
//! This is not `object`'s job and deliberately does not use it. The pane needs three things
//! `object` does not give: the raw bytes of each header field beside its decoded meaning, a
//! parse that survives a truncated or hostile file, and reads that go straight to the offsets
//! that matter instead of a buffer holding the whole file. An ELF keeps its section table at
//! the end, so a three-gigabyte binary costs four reads here, not three gigabytes of memory.
//!
//! Everything in a file being looked at is hostile input: a section count of 0xFFFF, an offset
//! past the end, a size that overflows when added to its offset. Every field is checked against
//! the real length before it is used, and a file that stops early fills the rows it can and
//! says `(truncated)` for the rest. Anyone opening a corrupt binary is opening it *because* it
//! is corrupt.

use std::fmt::Write as _;

/// What a row is, for drawing and for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
    /// A heading: `ELF header`, `Sections`.
    Group,
    /// A header field: label, raw bytes, meaning.
    Field,
    /// A row of a table (a section, a segment, an import) — these are what `Enter` follows.
    Entry,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub kind: RowKind,
    pub label: String,
    /// The raw bytes, as hex, where the row has any.
    pub raw: String,
    /// What those bytes mean.
    pub value: String,
    /// Where in the file this row points, for `Enter`.
    pub offset: Option<u64>,
}

impl Row {
    fn group(label: impl Into<String>) -> Row {
        Row {
            kind: RowKind::Group,
            label: label.into(),
            raw: String::new(),
            value: String::new(),
            offset: None,
        }
    }

    fn field(label: impl Into<String>, raw: impl Into<String>, value: impl Into<String>) -> Row {
        Row {
            kind: RowKind::Field,
            label: label.into(),
            raw: raw.into(),
            value: value.into(),
            offset: None,
        }
    }

    fn entry(label: impl Into<String>, value: impl Into<String>, offset: Option<u64>) -> Row {
        Row {
            kind: RowKind::Entry,
            label: label.into(),
            raw: String::new(),
            value: value.into(),
            offset,
        }
    }

    /// Can `Enter` do something with this row?
    pub fn is_target(&self) -> bool {
        self.offset.is_some()
    }
}

/// A function symbol: the same three fields as `disasm::SymbolInfo`, so whichever of the two a
/// caller managed to read can label a disassembly line (SPEC 05 §1.9).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    /// Where it loads, not where it is in the file.
    pub address: u64,
    /// 0 when the format does not record one — Mach-O never does.
    pub size: u64,
}

/// A stretch of the file that loads at a virtual address. This is what turns a symbol's
/// address or a branch target into a place the other pane can go to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub address: u64,
    pub size: u64,
    pub offset: u64,
}

/// The decoded shape of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    /// What we decided the file is.
    pub format: &'static str,
    /// One line of identity, for the pane's title.
    pub summary: String,
    pub rows: Vec<Row>,
    /// Function symbols, sorted by address, so a lookup is a binary search.
    pub symbols: Vec<Symbol>,
    /// What loads where, sorted by address.
    pub maps: Vec<Mapping>,
    /// The file ended inside something we were reading.
    pub truncated: bool,
}

impl Structure {
    /// The rows `Enter` can act on, for the tests and for "jump to the next table".
    pub fn targets(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|r| r.is_target())
    }

    /// The name of the symbol covering `address`, so a disassembly line can read
    /// `call 0000001F80 <scmd_main>` instead of a bare number — SPEC 05 §1.9.
    pub fn symbol_at(&self, address: u64) -> Option<&str> {
        self.symbol_entry(address).map(|s| s.name.as_str())
    }

    /// The symbol covering `address`: the last one at or below it, and only while `address` is
    /// inside the size it declares. A size of 0 means the file never said, so the symbol owns
    /// everything after it — the same rule `disasm::Binary::function_at` follows.
    pub fn symbol_entry(&self, address: u64) -> Option<&Symbol> {
        let at = match self.symbols.binary_search_by(|s| s.address.cmp(&address)) {
            Ok(i) => i,
            Err(0) => return None,
            Err(i) => i - 1,
        };
        let s = &self.symbols[at];
        (s.size == 0 || address < s.address.saturating_add(s.size)).then_some(s)
    }

    /// File offset for a virtual address, following whatever the file says it loads where.
    /// `None` for an address that is not backed by bytes in this file.
    pub fn offset_for_va(&self, va: u64) -> Option<u64> {
        map_offset(&self.maps, va)
    }
}

/// How the pane gets bytes: the viewer hands in its own source, so this works the same on a
/// local file, a file inside an archive, or a temporary copy fetched over SFTP.
pub trait Bytes {
    /// Up to `len` bytes at `offset`; fewer (or none) at the end of the file.
    fn at(&mut self, offset: u64, len: usize) -> Vec<u8>;
    fn len(&mut self) -> u64;
    fn is_empty(&mut self) -> bool {
        self.len() == 0
    }
}

impl Bytes for &[u8] {
    fn at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        let total = <[u8]>::len(self);
        let start = (offset as usize).min(total);
        let end = start.saturating_add(len).min(total);
        self[start..end].to_vec()
    }

    fn len(&mut self) -> u64 {
        <[u8]>::len(self) as u64
    }
}

/// A window of bytes with its own position, which answers 0 past its end and remembers that it
/// was asked. That is how a truncated file stops being a special case: the fields that are
/// there decode, the ones that are not read as zero and the row says so.
struct Cur<'a> {
    bytes: &'a [u8],
    pos: usize,
    /// Set when a read went past the end.
    short: bool,
    big_endian: bool,
}

impl<'a> Cur<'a> {
    fn new(bytes: &'a [u8]) -> Cur<'a> {
        Cur {
            bytes,
            pos: 0,
            short: false,
            big_endian: false,
        }
    }

    fn seek(&mut self, pos: usize) -> &mut Self {
        self.pos = pos;
        self
    }

    fn take(&mut self, n: usize) -> &'a [u8] {
        let start = self.pos.min(self.bytes.len());
        let end = (start + n).min(self.bytes.len());
        if end - start < n {
            self.short = true;
        }
        self.pos += n;
        &self.bytes[start..end]
    }

    fn u16(&mut self) -> u16 {
        let b = self.take(2);
        let mut v = [0u8; 2];
        v[..b.len()].copy_from_slice(b);
        if self.big_endian {
            u16::from_be_bytes(v)
        } else {
            u16::from_le_bytes(v)
        }
    }

    fn u32(&mut self) -> u32 {
        let b = self.take(4);
        let mut v = [0u8; 4];
        v[..b.len()].copy_from_slice(b);
        if self.big_endian {
            u32::from_be_bytes(v)
        } else {
            u32::from_le_bytes(v)
        }
    }

    fn u64(&mut self) -> u64 {
        let b = self.take(8);
        let mut v = [0u8; 8];
        v[..b.len()].copy_from_slice(b);
        if self.big_endian {
            u64::from_be_bytes(v)
        } else {
            u64::from_le_bytes(v)
        }
    }

    /// A field: its raw bytes as hex, and whatever they mean.
    fn hex(&self, from: usize, n: usize) -> String {
        let start = from.min(self.bytes.len());
        let end = (start + n).min(self.bytes.len());
        let mut out = String::with_capacity(n * 3);
        for b in &self.bytes[start..end] {
            let _ = write!(out, "{b:02X} ");
        }
        if end - start < n {
            out.push_str("··");
        }
        out.trim_end().to_string()
    }
}

/// How many rows of a table we are willing to show. A file can claim 65 535 sections; a pane
/// that tries to list them all is a pane that hangs.
const MAX_ENTRIES: usize = 512;
/// The most we read for a name table.
const MAX_STRTAB: usize = 1 << 20;
/// The most we read looking for the end of one name. A name table with no NUL in it is a
/// corrupt file, not a reason to read the whole file looking.
const MAX_NAME: usize = 256;

/// Decode what we can. `None` means "not a format this pane knows", which is not an error:
/// most files are not executables.
pub fn read(src: &mut dyn Bytes) -> Option<Structure> {
    let len = src.len();
    let head = src.at(0, 4096);
    if head.len() >= 4 && head[..4] == *b"\x7fELF" {
        return Some(elf(src, &head, len));
    }
    if head.len() >= 2 && head[..2] == *b"MZ" {
        return Some(pe(src, &head, len));
    }
    match head.first_chunk::<4>().map(|b| u32::from_le_bytes(*b)) {
        Some(0xFEED_FACE) | Some(0xFEED_FACF) => return Some(macho(src, &head, len)),
        // A fat Mach-O is big-endian by definition; a Java class file shares the magic and is
        // told apart by what follows (its major version is at least 45).
        Some(0xBEBA_FECA) if head.len() >= 8 => {
            let count = u32::from_be_bytes([head[4], head[5], head[6], head[7]]);
            if count < 64 {
                return Some(fat_macho(&head, count, len));
            }
        }
        _ => {}
    }
    if head.len() >= 8 && head[..8] == *b"!<arch>\n" {
        return Some(ar(src, len));
    }
    None
}

// ── ELF ─────────────────────────────────────────────────────────────────────────────────────

fn elf(src: &mut dyn Bytes, head: &[u8], len: u64) -> Structure {
    let mut rows = Vec::new();
    let mut c = Cur::new(head);
    let class = head.get(4).copied().unwrap_or(0);
    let data = head.get(5).copied().unwrap_or(0);
    let is64 = class == 2;
    c.big_endian = data == 2;

    let ident = c.hex(0, 16);
    c.seek(16);
    let e_type = c.u16();
    let e_machine = c.u16();
    let e_version = c.u32();
    let (entry, phoff, shoff) = if is64 {
        (c.u64(), c.u64(), c.u64())
    } else {
        (c.u32() as u64, c.u32() as u64, c.u32() as u64)
    };
    let _flags = c.u32();
    let _ehsize = c.u16();
    let phentsize = c.u16();
    let phnum = c.u16();
    let shentsize = c.u16();
    let shnum = c.u16();
    let shstrndx = c.u16();

    let kind = match e_type {
        0 => "NONE",
        1 => "REL (object file)",
        2 => "EXEC (executable)",
        3 => "DYN (shared object or PIE)",
        4 => "CORE (core dump)",
        other => return truncated_elf(other, len),
    };
    let machine = elf_machine(e_machine);
    let summary = format!(
        "ELF{} {} {} · {}",
        if is64 { "64" } else { "32" },
        if c.big_endian {
            "big-endian"
        } else {
            "little-endian"
        },
        machine,
        kind
    );

    rows.push(Row::group("ELF header"));
    rows.push(Row::field("e_ident", ident, "\x7fELF"));
    rows.push(Row::field(
        "class",
        format!("{class:02X}"),
        if is64 { "ELF64" } else { "ELF32" },
    ));
    rows.push(Row::field(
        "data",
        format!("{data:02X}"),
        if c.big_endian {
            "2's complement, big endian"
        } else {
            "2's complement, little endian"
        },
    ));
    rows.push(Row::field(
        "osabi",
        format!("{:02X}", head.get(7).copied().unwrap_or(0)),
        elf_osabi(head.get(7).copied().unwrap_or(0)),
    ));
    rows.push(Row::field("e_type", format!("{e_type:04X}"), kind));
    rows.push(Row::field("e_machine", format!("{e_machine:04X}"), machine));
    rows.push(Row::field("e_version", format!("{e_version:08X}"), "1"));
    let mut entry_row = Row::field("e_entry", format!("{entry:016X}"), "entry point");
    entry_row.kind = RowKind::Entry;
    entry_row.offset = None; // a virtual address; resolved below if a section maps it
    let entry_at = rows.len();
    rows.push(entry_row);
    rows.push(Row::entry(
        "e_phoff",
        format!("{phnum} segments × {phentsize} B"),
        in_file(phoff, len),
    ));
    rows.push(Row::entry(
        "e_shoff",
        format!("{shnum} sections × {shentsize} B"),
        in_file(shoff, len),
    ));

    // Section names live in a section of their own, so that one is read first.
    let mut truncated = c.short;
    let strtab = elf_strtab(
        src,
        shoff,
        shentsize,
        shnum,
        shstrndx,
        is64,
        c.big_endian,
        len,
    );

    // The section table is decoded once and kept: the symbol, string and dynamic tables all
    // have to find each other through it, and re-reading it per name is a read per name.
    let (sections, cut) = elf_section_headers(
        src,
        shoff,
        shentsize,
        shnum,
        is64,
        c.big_endian,
        &strtab,
        len,
    );
    truncated |= cut;
    let mut maps = Vec::new();

    if shnum > 0 && shentsize as usize >= 40 {
        rows.push(Row::group(format!("Sections ({shnum})")));
        for (i, s) in sections.iter().enumerate() {
            // SHT_NOBITS (8) occupies no file space, so it has no offset to go to.
            let offset = if s.kind == 8 {
                None
            } else {
                add_map(&mut maps, s.addr, s.size, s.offset, len);
                in_file(s.offset, len)
            };
            let name = if s.name.is_empty() {
                format!("[{i}]")
            } else {
                s.name.clone()
            };
            rows.push(Row::entry(
                format!("  {name}"),
                format!(
                    "{:08X}  {} {}  {}",
                    s.offset,
                    size_cell(s.size),
                    elf_flags(s.flags),
                    if s.addr != 0 {
                        format!("@ {:X}", s.addr)
                    } else {
                        String::new()
                    }
                ),
                offset,
            ));
        }
        if shnum as usize > sections.len() {
            rows.push(Row::entry(
                "  …",
                format!(
                    "{} more sections not shown",
                    shnum as usize - sections.len()
                ),
                None,
            ));
        }
    }

    if phnum > 0 && phentsize as usize >= 32 {
        rows.push(Row::group(format!("Segments ({phnum})")));
        let count = (phnum as usize).min(MAX_ENTRIES);
        let table = read_table(src, phoff, phentsize as usize, count, len);
        for i in 0..count {
            let from = i * phentsize as usize;
            if from + phentsize as usize > table.len() {
                truncated = true;
                break;
            }
            let mut s = Cur::new(&table[from..from + phentsize as usize]);
            s.big_endian = c.big_endian;
            let p_type = s.u32();
            let (p_flags, p_offset, p_vaddr, _p_paddr, p_filesz) = if is64 {
                let flags = s.u32();
                (flags, s.u64(), s.u64(), s.u64(), s.u64())
            } else {
                let offset = s.u32() as u64;
                let vaddr = s.u32() as u64;
                let paddr = s.u32() as u64;
                let filesz = s.u32() as u64;
                let _memsz = s.u32();
                (s.u32(), offset, vaddr, paddr, filesz)
            };
            // A PT_LOAD is the authoritative answer to "what loads where"; a stripped file has
            // these when it has no section addresses left.
            if p_type == 1 {
                add_map(&mut maps, p_vaddr, p_filesz, p_offset, len);
            }
            rows.push(Row::entry(
                format!("  {}", elf_ptype(p_type)),
                format!(
                    "{p_offset:08X}  {}  {}  @ {p_vaddr:X}",
                    size_cell(p_filesz),
                    elf_pflags(p_flags)
                ),
                in_file(p_offset, len),
            ));
        }
    }

    maps.sort_by_key(|m| m.address);
    // The entry point is a virtual address; now that we know what loads where, it can be a
    // place to jump to rather than a number to read.
    if let Some(row) = rows.get_mut(entry_at) {
        row.offset = map_offset(&maps, entry);
    }

    // Symbols, imports and exports — SPEC 05 §1.8. `.symtab` is the whole table and `.dynsym`
    // only what the loader needs, so the fuller one names the code while the dynamic one says
    // what crosses a library boundary.
    let (static_syms, static_claim) = match elf_find(&sections, ".symtab") {
        Some(s) => elf_symbol_table(src, &sections, s, is64, c.big_endian, len),
        None => (Vec::new(), 0),
    };
    let (dynamic_syms, dynamic_claim) = match elf_find(&sections, ".dynsym") {
        Some(s) => elf_symbol_table(src, &sections, s, is64, c.big_endian, len),
        None => (Vec::new(), 0),
    };

    // The lookup takes functions from both tables, because a symbol stripped from one may
    // still be in the other; the pane lists only the fuller table, to say each name once.
    let mut symbols: Vec<Symbol> = static_syms
        .iter()
        .chain(dynamic_syms.iter())
        .filter(|s| s.is_function())
        .map(|s| Symbol {
            name: s.name.clone(),
            address: s.value,
            size: s.size,
        })
        .collect();

    let (listed, claimed) = if static_syms.is_empty() {
        (&dynamic_syms, dynamic_claim)
    } else {
        (&static_syms, static_claim)
    };
    let functions: Vec<&ElfSym> = listed.iter().filter(|s| s.is_function()).collect();
    if !functions.is_empty() {
        rows.push(Row::group(format!("Symbols ({})", functions.len())));
        for f in &functions {
            let at = map_offset(&maps, f.value);
            rows.push(Row::entry(
                format!("  {}", f.name),
                sym_cell(at, f.value, f.size),
                at,
            ));
        }
        if claimed > listed.len() {
            rows.push(Row::entry(
                "  …",
                format!("{} more entries not read", claimed - listed.len()),
                None,
            ));
        }
    }

    let dynamic = elf_dynamic(src, &sections, is64, c.big_endian, len);
    // A file with no `.dynsym` is usually an object file, whose undefined symbols are what it
    // will need at link time — the same question, answered one stage earlier.
    let dynamic_table = if dynamic_syms.is_empty() {
        &static_syms
    } else {
        &dynamic_syms
    };
    let undefined: Vec<&ElfSym> = dynamic_table
        .iter()
        .filter(|s| s.shndx == 0 && !s.name.is_empty())
        .collect();
    if !dynamic.needed.is_empty() || !undefined.is_empty() {
        rows.push(Row::group(format!(
            "Imports ({} libraries, {} symbols)",
            dynamic.needed.len(),
            undefined.len()
        )));
        for lib in &dynamic.needed {
            rows.push(Row::entry(format!("  {lib}"), "needed library", None));
        }
        // ELF has one flat namespace: nothing in the file says which of the libraries above
        // will supply a given symbol, so none is claimed here. The offset is the symbol's own
        // entry, which is the only place it exists in this file.
        for s in &undefined {
            rows.push(Row::entry(
                format!("  {}", s.name),
                format!("{}  undefined", off_cell(s.at)),
                s.at,
            ));
        }
    }

    let exported: Vec<&ElfSym> = dynamic_table.iter().filter(|s| s.is_export()).collect();
    if !exported.is_empty() {
        rows.push(Row::group(format!("Exports ({})", exported.len())));
        for e in &exported {
            let at = map_offset(&maps, e.value);
            rows.push(Row::entry(
                format!("  {}", e.name),
                sym_cell(at, e.value, e.size),
                at,
            ));
        }
    }

    // The interpreter, the SONAME, the run path and the build id are the strings worth reading
    // the bytes for: they say how the file will be loaded and which build it came from.
    let mut notes = Vec::new();
    if let Some(interp) = elf_find(&sections, ".interp").and_then(|s| elf_data(src, s, len, 256)) {
        let text = String::from_utf8_lossy(&interp)
            .trim_end_matches('\0')
            .to_string();
        if !text.is_empty() {
            notes.push(Row::field("interpreter", String::new(), text));
        }
    }
    if let Some(soname) = dynamic.soname {
        notes.push(Row::field("soname", String::new(), soname));
    }
    if let Some(runpath) = dynamic.runpath {
        notes.push(Row::field("run path", String::new(), runpath));
    }
    if let Some(note) = elf_find(&sections, ".note.gnu.build-id")
        .and_then(|s| elf_data(src, s, len, 64))
        .filter(|n| !n.is_empty())
    {
        // A note is namesz, descsz, type, then the padded name, then the descriptor.
        if note.len() > 16 {
            let id: String = note[16..]
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join("");
            if !id.is_empty() {
                notes.push(Row::field("build id", String::new(), id));
            }
        }
    }
    if !notes.is_empty() {
        rows.push(Row::group("Of note"));
        rows.extend(notes);
    }

    sort_symbols(&mut symbols);
    Structure {
        format: "ELF",
        summary,
        rows,
        symbols,
        maps,
        truncated,
    }
}

fn truncated_elf(e_type: u16, len: u64) -> Structure {
    Structure {
        format: "ELF",
        summary: format!("ELF, type {e_type:#06X} (not one of the five standard kinds)"),
        rows: vec![
            Row::group("ELF header"),
            Row::field("e_type", format!("{e_type:04X}"), "unknown"),
            Row::field("file size", String::new(), size_cell(len)),
        ],
        symbols: Vec::new(),
        maps: Vec::new(),
        truncated: false,
    }
}

#[allow(clippy::too_many_arguments)]
fn elf_strtab(
    src: &mut dyn Bytes,
    shoff: u64,
    shentsize: u16,
    shnum: u16,
    shstrndx: u16,
    is64: bool,
    big_endian: bool,
    len: u64,
) -> Vec<u8> {
    if shstrndx >= shnum || shentsize == 0 {
        return Vec::new();
    }
    let at = shoff.saturating_add(shstrndx as u64 * shentsize as u64);
    let header = src.at(at, shentsize as usize);
    if header.len() < shentsize as usize {
        return Vec::new();
    }
    let mut s = Cur::new(&header);
    s.big_endian = big_endian;
    s.seek(8); // past sh_name and sh_type
    let (offset, size) = if is64 {
        s.seek(24);
        (s.u64(), s.u64())
    } else {
        s.seek(16);
        (s.u32() as u64, s.u32() as u64)
    };
    if offset >= len {
        return Vec::new();
    }
    let size = size.min(MAX_STRTAB as u64).min(len - offset) as usize;
    src.at(offset, size)
}

/// A section header, decoded. Only the fields a later pass asks about are kept.
#[derive(Debug, Clone)]
struct ElfSection {
    name: String,
    /// `sh_type`: 2 `SYMTAB`, 3 `STRTAB`, 6 `DYNAMIC`, 8 `NOBITS`, 11 `DYNSYM`.
    kind: u32,
    flags: u64,
    addr: u64,
    offset: u64,
    size: u64,
    /// `sh_link`: for a symbol table, the section holding its names.
    link: u32,
    entsize: u64,
}

/// Decode the section table once. The second value is set when the table stopped early, which
/// is the file saying it was cut off mid-header.
#[allow(clippy::too_many_arguments)]
fn elf_section_headers(
    src: &mut dyn Bytes,
    shoff: u64,
    shentsize: u16,
    shnum: u16,
    is64: bool,
    big_endian: bool,
    strtab: &[u8],
    len: u64,
) -> (Vec<ElfSection>, bool) {
    if shentsize == 0 || shnum == 0 {
        return (Vec::new(), false);
    }
    let count = (shnum as usize).min(MAX_ENTRIES);
    let table = read_table(src, shoff, shentsize as usize, count, len);
    let mut out = Vec::with_capacity(count.min(64));
    for i in 0..count {
        let from = i * shentsize as usize;
        if from + shentsize as usize > table.len() {
            return (out, true);
        }
        let mut s = Cur::new(&table[from..from + shentsize as usize]);
        s.big_endian = big_endian;
        let sh_name = s.u32();
        let kind = s.u32();
        let (flags, addr, offset, size) = if is64 {
            (s.u64(), s.u64(), s.u64(), s.u64())
        } else {
            (
                s.u32() as u64,
                s.u32() as u64,
                s.u32() as u64,
                s.u32() as u64,
            )
        };
        let link = s.u32();
        let _info = s.u32();
        let entsize = if is64 {
            s.seek(56);
            s.u64()
        } else {
            s.seek(36);
            s.u32() as u64
        };
        out.push(ElfSection {
            name: str_at(strtab, sh_name as usize),
            kind,
            flags,
            addr,
            offset,
            size,
            link,
            entsize,
        });
    }
    (out, false)
}

fn elf_find<'a>(sections: &'a [ElfSection], name: &str) -> Option<&'a ElfSection> {
    sections.iter().find(|s| s.name == name)
}

/// The bytes of a section, up to `cap`, range-checked against the real file length.
fn elf_data(src: &mut dyn Bytes, s: &ElfSection, len: u64, cap: usize) -> Option<Vec<u8>> {
    if s.kind == 8 || s.offset >= len {
        return None;
    }
    let size = s.size.min(cap as u64).min(len - s.offset) as usize;
    (size > 0).then(|| src.at(s.offset, size))
}

/// One entry of an ELF symbol table, and where that entry sits in the file.
#[derive(Debug, Clone)]
struct ElfSym {
    name: String,
    value: u64,
    size: u64,
    /// `STT_*`: 2 is a function.
    kind: u8,
    /// `STB_*`: 1 global, 2 weak.
    bind: u8,
    /// `SHN_UNDEF` (0) is a symbol this file needs from somewhere else.
    shndx: u16,
    at: Option<u64>,
}

impl ElfSym {
    /// A named function with an address: what a disassembly can be labelled with.
    fn is_function(&self) -> bool {
        self.kind == 2 && self.shndx != 0 && self.value != 0 && !self.name.is_empty()
    }

    /// A defined symbol anything else can bind to.
    fn is_export(&self) -> bool {
        self.shndx != 0 && (self.bind == 1 || self.bind == 2) && !self.name.is_empty()
    }
}

/// Read a symbol table, capped like every other table here. The second value is how many
/// entries the table claimed, which is what a capped one has to own up to.
fn elf_symbol_table(
    src: &mut dyn Bytes,
    sections: &[ElfSection],
    s: &ElfSection,
    is64: bool,
    big_endian: bool,
    len: u64,
) -> (Vec<ElfSym>, usize) {
    let natural = if is64 { 24usize } else { 16 };
    // A table that claims an entry size of its own is believed, up to the point where the
    // number stops being a size at all.
    let entry = if s.entsize == 0 {
        natural
    } else {
        s.entsize.min(1 << 16) as usize
    };
    // An entry smaller than the format's own layout cannot be a symbol table at all.
    if entry < natural || s.offset >= len {
        return (Vec::new(), 0);
    }
    let claimed = (s.size / entry as u64) as usize;
    let count = claimed.min(MAX_ENTRIES);
    let table = read_table(src, s.offset, entry, count, len);
    let names = elf_symbol_names(src, sections, s, len);
    let mut out = Vec::with_capacity(count.min(64));
    for i in 0..count {
        let from = i * entry;
        let to = (from + entry).min(table.len());
        if to - from < natural {
            break;
        }
        let mut e = Cur::new(&table[from..to]);
        e.big_endian = big_endian;
        let st_name = e.u32();
        let (value, size, info, shndx) = if is64 {
            let info = e.take(1).first().copied().unwrap_or(0);
            let _other = e.take(1);
            let shndx = e.u16();
            (e.u64(), e.u64(), info, shndx)
        } else {
            let value = e.u32() as u64;
            let size = e.u32() as u64;
            let info = e.take(1).first().copied().unwrap_or(0);
            let _other = e.take(1);
            (value, size, info, e.u16())
        };
        out.push(ElfSym {
            // A name index past the end of the string table names nothing, and nothing is
            // invented for it.
            name: str_at(&names, st_name as usize),
            value,
            size,
            kind: info & 0x0F,
            bind: info >> 4,
            shndx,
            at: in_file(s.offset + from as u64, len),
        });
    }
    (out, claimed)
}

/// Where a symbol table's names live. `sh_link` says; a link out of range is a corrupt file,
/// and then the conventional name is the next best guess.
fn elf_symbol_names(
    src: &mut dyn Bytes,
    sections: &[ElfSection],
    s: &ElfSection,
    len: u64,
) -> Vec<u8> {
    if let Some(linked) = sections.get(s.link as usize) {
        if linked.kind == 3 {
            if let Some(data) = elf_data(src, linked, len, MAX_STRTAB) {
                return data;
            }
        }
    }
    let fallback = if s.name == ".dynsym" {
        ".dynstr"
    } else {
        ".strtab"
    };
    elf_find(sections, fallback)
        .and_then(|l| elf_data(src, l, len, MAX_STRTAB))
        .unwrap_or_default()
}

/// What `.dynamic` says about the libraries this file needs and the names it goes by.
#[derive(Debug, Default)]
struct ElfDynamic {
    needed: Vec<String>,
    soname: Option<String>,
    runpath: Option<String>,
}

fn elf_dynamic(
    src: &mut dyn Bytes,
    sections: &[ElfSection],
    is64: bool,
    big_endian: bool,
    len: u64,
) -> ElfDynamic {
    let mut out = ElfDynamic::default();
    let Some(dynamic) = elf_find(sections, ".dynamic") else {
        return out;
    };
    let entry = if is64 { 16usize } else { 8 };
    if dynamic.offset >= len {
        return out;
    }
    let count = ((dynamic.size / entry as u64) as usize).min(MAX_ENTRIES);
    let table = read_table(src, dynamic.offset, entry, count, len);
    let strings = elf_find(sections, ".dynstr")
        .and_then(|s| elf_data(src, s, len, MAX_STRTAB))
        .unwrap_or_default();
    for i in 0..count {
        let from = i * entry;
        if from + entry > table.len() {
            break;
        }
        let mut e = Cur::new(&table[from..from + entry]);
        e.big_endian = big_endian;
        let (tag, val) = if is64 {
            (e.u64(), e.u64())
        } else {
            (e.u32() as u64, e.u32() as u64)
        };
        // DT_NULL ends the list, however much room the section claims.
        match tag {
            0 => break,
            1 => {
                let name = str_at(&strings, val as usize);
                if !name.is_empty() {
                    out.needed.push(name);
                }
            }
            14 => out.soname = Some(str_at(&strings, val as usize)).filter(|s| !s.is_empty()),
            // DT_RPATH (15) and DT_RUNPATH (29) answer the same question.
            15 | 29 => out.runpath = Some(str_at(&strings, val as usize)).filter(|s| !s.is_empty()),
            _ => {}
        }
    }
    out
}

fn elf_machine(m: u16) -> &'static str {
    match m {
        0x03 => "x86",
        0x08 => "MIPS",
        0x14 => "PowerPC",
        0x15 => "PowerPC64",
        0x16 => "S390",
        0x28 => "ARM",
        0x2A => "SuperH",
        0x32 => "IA-64",
        0x3E => "x86-64",
        0xB7 => "AArch64",
        0xF3 => "RISC-V",
        0x102 => "LoongArch",
        _ => "unknown machine",
    }
}

fn elf_osabi(v: u8) -> &'static str {
    match v {
        0 => "System V",
        1 => "HP-UX",
        2 => "NetBSD",
        3 => "Linux",
        6 => "Solaris",
        9 => "FreeBSD",
        12 => "OpenBSD",
        _ => "other",
    }
}

fn elf_ptype(t: u32) -> &'static str {
    match t {
        0 => "NULL",
        1 => "LOAD",
        2 => "DYNAMIC",
        3 => "INTERP",
        4 => "NOTE",
        6 => "PHDR",
        7 => "TLS",
        0x6474_E550 => "GNU_EH_FRAME",
        0x6474_E551 => "GNU_STACK",
        0x6474_E552 => "GNU_RELRO",
        0x6474_E553 => "GNU_PROPERTY",
        _ => "other",
    }
}

fn elf_flags(f: u64) -> String {
    let mut s = String::new();
    s.push(if f & 0x2 != 0 { 'a' } else { '-' }); // ALLOC
    s.push(if f & 0x1 != 0 { 'w' } else { '-' }); // WRITE
    s.push(if f & 0x4 != 0 { 'x' } else { '-' }); // EXECINSTR
    s
}

fn elf_pflags(f: u32) -> String {
    let mut s = String::new();
    s.push(if f & 0x4 != 0 { 'r' } else { '-' });
    s.push(if f & 0x2 != 0 { 'w' } else { '-' });
    s.push(if f & 0x1 != 0 { 'x' } else { '-' });
    s
}

// ── PE ──────────────────────────────────────────────────────────────────────────────────────

fn pe(src: &mut dyn Bytes, head: &[u8], len: u64) -> Structure {
    let mut rows = Vec::new();
    let mut c = Cur::new(head);
    c.seek(0x3C);
    let lfanew = c.u32() as u64;

    rows.push(Row::group("DOS header"));
    rows.push(Row::field("e_magic", "4D 5A", "MZ"));
    rows.push(Row::entry(
        "e_lfanew",
        format!("PE header at {lfanew:08X}"),
        in_file(lfanew, len),
    ));

    let nt = src.at(lfanew, 256);
    if nt.len() < 24 || nt[..4] != *b"PE\0\0" {
        return Structure {
            format: "MZ",
            summary: "DOS executable (no PE header)".to_string(),
            rows,
            symbols: Vec::new(),
            maps: Vec::new(),
            truncated: nt.len() < 24,
        };
    }
    let mut n = Cur::new(&nt);
    n.seek(4);
    let machine = n.u16();
    let nsections = n.u16();
    let timestamp = n.u32();
    let _sym = n.u32();
    let _nsym = n.u32();
    let opt_size = n.u16();
    let characteristics = n.u16();
    let opt_magic = n.u16();

    let machine_name = pe_machine(machine);
    let plus = opt_magic == 0x20B;
    let kind = if characteristics & 0x2000 != 0 {
        "DLL"
    } else if characteristics & 0x0002 != 0 {
        "executable"
    } else {
        "object"
    };
    let summary = format!(
        "PE{} {} · {}",
        if plus { "32+" } else { "32" },
        machine_name,
        kind
    );

    rows.push(Row::group("PE header"));
    rows.push(Row::field("Signature", "50 45 00 00", "PE\\0\\0"));
    rows.push(Row::field(
        "Machine",
        format!("{machine:04X}"),
        machine_name,
    ));
    rows.push(Row::field(
        "NumberOfSections",
        format!("{nsections:04X}"),
        format!("{nsections}"),
    ));
    rows.push(Row::field(
        "TimeDateStamp",
        format!("{timestamp:08X}"),
        unix_time(timestamp as u64),
    ));
    rows.push(Row::field(
        "Characteristics",
        format!("{characteristics:04X}"),
        kind,
    ));

    // The optional header, which is not optional in an image.
    let mut image_base = 0u64;
    let mut dirs = Vec::new();
    if opt_size >= 24 {
        n.seek(24 + 16);
        let entry = n.u32();
        let base_of_code = n.u32();
        // PE32 has a BaseOfData before ImageBase; PE32+ drops it and widens ImageBase, so the
        // field sits eight bytes earlier there.
        image_base = if plus {
            n.u64()
        } else {
            let _base_of_data = n.u32();
            n.u32() as u64
        };
        rows.push(Row::group("Optional header"));
        rows.push(Row::field(
            "Magic",
            format!("{opt_magic:04X}"),
            if plus { "PE32+" } else { "PE32" },
        ));
        rows.push(Row::field(
            "AddressOfEntryPoint",
            format!("{entry:08X}"),
            format!("RVA {entry:X}"),
        ));
        rows.push(Row::field(
            "BaseOfCode",
            format!("{base_of_code:08X}"),
            format!("RVA {base_of_code:X}"),
        ));
        rows.push(Row::field(
            "ImageBase",
            format!("{image_base:X}"),
            format!("loads at {image_base:X}"),
        ));
        // Subsystem sits 68 bytes into the optional header (PE32) or 68 as well for PE32+,
        // because the fields that differ come after it.
        n.seek(24 + 68);
        let subsystem = n.u16();
        rows.push(Row::field(
            "Subsystem",
            format!("{subsystem:04X}"),
            pe_subsystem(subsystem),
        ));
        // The data directories are the index to everything else in the image. They sit at the
        // end of the optional header, whose two shapes put them in different places.
        let dd_base = if plus { 112u64 } else { 96 };
        n.seek(24 + dd_base as usize - 4);
        let count = n.u32().min(16) as usize;
        if count > 0 && opt_size as u64 >= dd_base {
            dirs = src.at(lfanew + 24 + dd_base, count * 8);
        }
    }

    let mut truncated = n.short;
    let sec_at = lfanew + 24 + opt_size as u64;
    let mut maps = Vec::new();
    let mut pe_sections: Vec<PeSection> = Vec::new();
    if nsections > 0 {
        rows.push(Row::group(format!("Sections ({nsections})")));
        let count = (nsections as usize).min(MAX_ENTRIES);
        let table = read_table(src, sec_at, 40, count, len);
        for i in 0..count {
            let from = i * 40;
            if from + 40 > table.len() {
                truncated = true;
                break;
            }
            let mut s = Cur::new(&table[from..from + 40]);
            let name_bytes = s.take(8);
            let name = String::from_utf8_lossy(name_bytes)
                .trim_end_matches('\0')
                .to_string();
            let virtual_size = s.u32();
            let virtual_address = s.u32();
            let raw_size = s.u32();
            let raw_ptr = s.u32();
            s.seek(36);
            let flags = s.u32();
            // Only the bytes the section really has in the file can be mapped: the rest of a
            // larger virtual size is zero-filled by the loader, not read from here.
            let mapped = if virtual_size == 0 {
                raw_size
            } else {
                raw_size.min(virtual_size)
            };
            pe_sections.push(PeSection {
                rva: virtual_address,
                raw_ptr,
                raw_size,
            });
            add_map(
                &mut maps,
                image_base.saturating_add(virtual_address as u64),
                mapped as u64,
                raw_ptr as u64,
                len,
            );
            rows.push(Row::entry(
                format!("  {name}"),
                format!(
                    "{raw_ptr:08X}  {}  {}  RVA {virtual_address:X}/{virtual_size:X}",
                    size_cell(raw_size as u64),
                    pe_section_flags(flags)
                ),
                in_file(raw_ptr as u64, len),
            ));
        }
    }
    maps.sort_by_key(|m| m.address);

    // Imports and exports — SPEC 05 §1.8. An image with neither is unusual but legal, and a
    // directory that points nowhere simply contributes no rows.
    let (import_rva, _) = pe_dir(&dirs, 1);
    if import_rva != 0 {
        rows.extend(pe_imports(src, &pe_sections, import_rva, plus, len));
    }
    let (export_rva, export_size) = pe_dir(&dirs, 0);
    let mut symbols = Vec::new();
    if export_rva != 0 {
        let (export_rows, exported) =
            pe_exports(src, &pe_sections, export_rva, export_size, image_base, len);
        rows.extend(export_rows);
        symbols = exported;
    }

    sort_symbols(&mut symbols);
    Structure {
        format: "PE",
        summary,
        rows,
        symbols,
        maps,
        truncated,
    }
}

/// A section, reduced to what mapping an RVA needs.
#[derive(Debug, Clone, Copy)]
struct PeSection {
    rva: u32,
    raw_ptr: u32,
    raw_size: u32,
}

/// One data directory: its RVA and its size. Each entry is eight bytes, and an index past the
/// end of the array is a directory the file does not have.
fn pe_dir(dirs: &[u8], index: usize) -> (u32, u32) {
    let at = index * 8;
    if at + 8 > dirs.len() {
        return (0, 0);
    }
    let w = |k: usize| u32::from_le_bytes([dirs[k], dirs[k + 1], dirs[k + 2], dirs[k + 3]]);
    (w(at), w(at + 4))
}

/// File offset for an RVA, through the section that covers it. An RVA inside a section's
/// virtual size but past the bytes it has in the file maps nowhere, and says so.
fn pe_offset(sections: &[PeSection], rva: u32, len: u64) -> Option<u64> {
    for s in sections {
        if rva >= s.rva && rva - s.rva < s.raw_size {
            return in_file(s.raw_ptr as u64 + (rva - s.rva) as u64, len);
        }
    }
    None
}

/// Up to `want` bytes at an RVA, and nothing at all for an RVA that is not in the file.
fn pe_bytes(
    src: &mut dyn Bytes,
    sections: &[PeSection],
    rva: u32,
    want: usize,
    len: u64,
) -> Vec<u8> {
    match pe_offset(sections, rva, len) {
        Some(at) => src.at(at, (want as u64).min(len - at) as usize),
        None => Vec::new(),
    }
}

/// A NUL-terminated name at an RVA, capped: a name is not a reason to read a megabyte.
fn pe_name(src: &mut dyn Bytes, sections: &[PeSection], rva: u32, len: u64) -> String {
    let bytes = pe_bytes(src, sections, rva, MAX_NAME, len);
    str_at(&bytes, 0)
}

/// The import directory: a library per descriptor, and a symbol per thunk under it.
fn pe_imports(
    src: &mut dyn Bytes,
    sections: &[PeSection],
    rva: u32,
    plus: bool,
    len: u64,
) -> Vec<Row> {
    // The descriptor array ends with an all-zero entry. A file that forgets one would walk
    // forever, so the read is as long as the walk is allowed to be and no longer.
    let table = pe_bytes(src, sections, rva, MAX_ENTRIES * 20, len);
    let step = if plus { 8usize } else { 4 };
    let ordinal_flag = if plus { 1u64 << 63 } else { 1 << 31 };
    let mut rows = Vec::new();
    let mut libraries = 0usize;
    // One budget for the symbols of all libraries together: 512 libraries of 512 symbols is a
    // pane that hangs just as surely as 65 535 sections.
    let mut budget = MAX_ENTRIES;
    let mut at = 0usize;
    while at + 20 <= table.len() && libraries < MAX_ENTRIES {
        let w = |k: usize| {
            u32::from_le_bytes([
                table[at + k],
                table[at + k + 1],
                table[at + k + 2],
                table[at + k + 3],
            ])
        };
        let lookup = w(0);
        let name_rva = w(12);
        let iat = w(16);
        if lookup == 0 && name_rva == 0 && iat == 0 {
            break;
        }
        let library = pe_name(src, sections, name_rva, len);
        let library = if library.is_empty() {
            format!("[{libraries}]")
        } else {
            library
        };
        let descriptor = pe_offset(sections, rva.saturating_add(at as u32), len);

        // The lookup table holds the names; the address table holds what the loader patches,
        // and that is the slot a `call [rip+…]` in the disassembly actually names — so the
        // row points at the slot and the name comes from the lookup.
        let from = if lookup != 0 { lookup } else { iat };
        let thunks = pe_bytes(src, sections, from, MAX_ENTRIES * step, len);
        let mut names: Vec<(String, Option<u64>)> = Vec::new();
        let mut k = 0usize;
        while (k + 1) * step <= thunks.len() && names.len() < budget {
            let v = if plus {
                let mut b = [0u8; 8];
                b.copy_from_slice(&thunks[k * step..k * step + 8]);
                u64::from_le_bytes(b)
            } else {
                let mut b = [0u8; 4];
                b.copy_from_slice(&thunks[k * step..k * step + 4]);
                u32::from_le_bytes(b) as u64
            };
            if v == 0 {
                break;
            }
            let slot = pe_offset(sections, iat.saturating_add((k * step) as u32), len);
            if v & ordinal_flag != 0 {
                names.push((format!("ordinal {}", v & 0xFFFF), slot));
            } else {
                // IMAGE_IMPORT_BY_NAME: a two-byte hint, then the name.
                let name_at = (v as u32 & 0x7FFF_FFFF).saturating_add(2);
                let name = pe_name(src, sections, name_at, len);
                if !name.is_empty() {
                    names.push((name, slot));
                }
            }
            k += 1;
        }

        rows.push(Row::entry(
            format!("  {library}"),
            format!("{}  {} symbols", off_cell(descriptor), names.len()),
            descriptor,
        ));
        for (name, slot) in &names {
            rows.push(Row::entry(format!("    {name}"), off_cell(*slot), *slot));
        }
        budget = budget.saturating_sub(names.len());
        libraries += 1;
        at += 20;
    }
    if budget == 0 {
        rows.push(Row::entry("  …", "more imports not shown", None));
    }
    if !rows.is_empty() {
        rows.insert(0, Row::group(format!("Imports ({libraries} libraries)")));
    }
    rows
}

/// The export directory: the names a DLL offers, and where each one is.
fn pe_exports(
    src: &mut dyn Bytes,
    sections: &[PeSection],
    rva: u32,
    dir_size: u32,
    image_base: u64,
    len: u64,
) -> (Vec<Row>, Vec<Symbol>) {
    let dir = pe_bytes(src, sections, rva, 40, len);
    if dir.len() < 40 {
        return (Vec::new(), Vec::new());
    }
    let w = |k: usize| u32::from_le_bytes([dir[k], dir[k + 1], dir[k + 2], dir[k + 3]]);
    let library = pe_name(src, sections, w(12), len);
    let ordinal_base = w(16);
    let n_functions = w(20) as usize;
    let n_names = w(24) as usize;
    let shown_names = n_names.min(MAX_ENTRIES);
    let shown_functions = n_functions.min(MAX_ENTRIES);
    let functions = pe_bytes(src, sections, w(28), shown_functions * 4, len);
    let names = pe_bytes(src, sections, w(32), shown_names * 4, len);
    let ordinals = pe_bytes(src, sections, w(36), shown_names * 2, len);

    let mut rows = vec![Row::group(format!("Exports ({n_names})"))];
    if !library.is_empty() {
        rows.push(Row::field("library name", String::new(), library));
    }
    let mut symbols = Vec::new();
    for i in 0..shown_names {
        if (i + 1) * 4 > names.len() || (i + 1) * 2 > ordinals.len() {
            break;
        }
        let name = pe_name(
            src,
            sections,
            u32::from_le_bytes([
                names[i * 4],
                names[i * 4 + 1],
                names[i * 4 + 2],
                names[i * 4 + 3],
            ]),
            len,
        );
        if name.is_empty() {
            continue;
        }
        // The name list and the address list are only parallel through the ordinal table.
        let ordinal = u16::from_le_bytes([ordinals[i * 2], ordinals[i * 2 + 1]]) as usize;
        let function = if (ordinal + 1) * 4 <= functions.len() {
            u32::from_le_bytes([
                functions[ordinal * 4],
                functions[ordinal * 4 + 1],
                functions[ordinal * 4 + 2],
                functions[ordinal * 4 + 3],
            ])
        } else {
            0
        };
        // An address inside the export directory itself is not code: it is the name of the
        // DLL that really has the symbol, which this one only forwards to.
        let forwarded = dir_size > 0 && function >= rva && function - rva < dir_size;
        if forwarded {
            let to = pe_name(src, sections, function, len);
            rows.push(Row::entry(
                format!("  {name}"),
                format!("forwarded to {to}"),
                None,
            ));
            continue;
        }
        let at = pe_offset(sections, function, len);
        let address = image_base.saturating_add(function as u64);
        rows.push(Row::entry(
            format!("  {name}"),
            format!(
                "{}  ordinal {}",
                sym_cell(at, address, 0),
                ordinal as u32 + ordinal_base
            ),
            at,
        ));
        if function != 0 {
            symbols.push(Symbol {
                name,
                address,
                size: 0,
            });
        }
    }
    if n_names > shown_names {
        rows.push(Row::entry(
            "  …",
            format!("{} more exports not shown", n_names - shown_names),
            None,
        ));
    }
    (rows, symbols)
}

fn pe_machine(m: u16) -> &'static str {
    match m {
        0x014C => "i386",
        0x8664 => "x86-64",
        0x01C0 | 0x01C4 => "ARM",
        0xAA64 => "ARM64",
        0x0200 => "IA-64",
        0x5032 => "RISC-V 32",
        0x5064 => "RISC-V 64",
        _ => "unknown machine",
    }
}

fn pe_subsystem(s: u16) -> &'static str {
    match s {
        1 => "native",
        2 => "Windows GUI",
        3 => "Windows console",
        7 => "POSIX",
        9 => "Windows CE",
        10 => "EFI application",
        11 => "EFI boot service driver",
        12 => "EFI runtime driver",
        13 => "EFI ROM",
        14 => "Xbox",
        _ => "other",
    }
}

fn pe_section_flags(f: u32) -> String {
    let mut s = String::new();
    s.push(if f & 0x4000_0000 != 0 { 'r' } else { '-' });
    s.push(if f & 0x8000_0000 != 0 { 'w' } else { '-' });
    s.push(if f & 0x2000_0000 != 0 { 'x' } else { '-' });
    s
}

// ── Mach-O ──────────────────────────────────────────────────────────────────────────────────

fn macho(src: &mut dyn Bytes, head: &[u8], len: u64) -> Structure {
    let mut rows = Vec::new();
    let mut c = Cur::new(head);
    let magic = c.u32();
    let is64 = magic == 0xFEED_FACF;
    let cputype = c.u32();
    let _cpusubtype = c.u32();
    let filetype = c.u32();
    let ncmds = c.u32();
    let sizeofcmds = c.u32();
    let flags = c.u32();

    let cpu = macho_cpu(cputype);
    let kind = macho_filetype(filetype);
    let summary = format!(
        "Mach-O {} {} · {}",
        if is64 { "64" } else { "32" },
        cpu,
        kind
    );

    rows.push(Row::group("Mach-O header"));
    rows.push(Row::field(
        "magic",
        format!("{magic:08X}"),
        if is64 { "MH_MAGIC_64" } else { "MH_MAGIC" },
    ));
    rows.push(Row::field("cputype", format!("{cputype:08X}"), cpu));
    rows.push(Row::field("filetype", format!("{filetype:08X}"), kind));
    rows.push(Row::field(
        "ncmds",
        format!("{ncmds:08X}"),
        format!("{ncmds} load commands"),
    ));
    rows.push(Row::field(
        "flags",
        format!("{flags:08X}"),
        macho_flags(flags),
    ));

    // Load commands follow the header; each carries its own size, so a bad one stops the walk
    // rather than sending it anywhere.
    let start = if is64 { 32u64 } else { 28 };
    let table = src.at(start, (sizeofcmds as usize).min(1 << 20));
    let mut at = 0usize;
    let mut shown = 0usize;
    let mut truncated = c.short;
    let mut maps = Vec::new();
    // The dylibs in the order their commands appear: a two-level-namespace symbol names its
    // library by its position in exactly this list.
    let mut dylibs: Vec<String> = Vec::new();
    let mut symtab: Option<(u64, u32, u64, u32)> = None;
    rows.push(Row::group("Load commands"));
    for _ in 0..ncmds.min(MAX_ENTRIES as u32) {
        if at + 8 > table.len() {
            truncated = ncmds > 0 && at < sizeofcmds as usize;
            break;
        }
        let cmd = u32::from_le_bytes([table[at], table[at + 1], table[at + 2], table[at + 3]]);
        let size = u32::from_le_bytes([table[at + 4], table[at + 5], table[at + 6], table[at + 7]])
            as usize;
        if size < 8 || at + size > table.len() {
            truncated = true;
            break;
        }
        let body = &table[at..at + size];
        match cmd {
            // LC_SEGMENT (1) and LC_SEGMENT_64 (0x19)
            1 | 0x19 => {
                let name = String::from_utf8_lossy(&body[8..8 + 16.min(body.len() - 8)])
                    .trim_end_matches('\0')
                    .to_string();
                let mut s = Cur::new(body);
                s.seek(24);
                let (vmaddr, _vmsize, fileoff, filesize) = if cmd == 0x19 {
                    (s.u64(), s.u64(), s.u64(), s.u64())
                } else {
                    (
                        s.u32() as u64,
                        s.u32() as u64,
                        s.u32() as u64,
                        s.u32() as u64,
                    )
                };
                add_map(&mut maps, vmaddr, filesize, fileoff, len);
                rows.push(Row::entry(
                    format!("  {name}"),
                    format!("{fileoff:08X}  {}  @ {vmaddr:X}", size_cell(filesize)),
                    in_file(fileoff, len),
                ));
            }
            // LC_LOAD_DYLIB (0x0C), LC_ID_DYLIB (0x0D), LC_LOAD_WEAK_DYLIB (0x80000018)
            0x0C | 0x0D | 0x8000_0018 => {
                if body.len() > 24 {
                    let name = String::from_utf8_lossy(&body[24..])
                        .trim_end_matches('\0')
                        .to_string();
                    // LC_ID_DYLIB is this file's own name, not a dependency, so it takes no
                    // place in the ordinals.
                    if cmd != 0x0D {
                        dylibs.push(name.clone());
                    }
                    rows.push(Row::entry(
                        if cmd == 0x0D { "  id" } else { "  dylib" },
                        name,
                        None,
                    ));
                }
            }
            // LC_SYMTAB says where the symbol table and its names are.
            2 => {
                let mut s = Cur::new(body);
                s.seek(8);
                let symoff = s.u32() as u64;
                let nsyms = s.u32();
                let stroff = s.u32() as u64;
                let strsize = s.u32();
                symtab = Some((symoff, nsyms, stroff, strsize));
                rows.push(Row::entry(
                    format!("  {}", macho_cmd(cmd)),
                    format!("{symoff:08X}  {nsyms} symbols"),
                    in_file(symoff, len),
                ));
            }
            _ => {
                rows.push(Row::entry(
                    format!("  {}", macho_cmd(cmd)),
                    format!("{size} B"),
                    None,
                ));
            }
        }
        shown += 1;
        at += size;
    }
    if (ncmds as usize) > shown {
        rows.push(Row::entry(
            "  …",
            format!("{} more commands not shown", ncmds as usize - shown),
            None,
        ));
    }
    maps.sort_by_key(|m| m.address);

    // The symbol table is one bounded read, and it answers all three questions at once: what
    // is defined here, what is wanted from elsewhere, and what the rest of the world may bind
    // to. Mach-O records no symbol sizes, so every size here is 0 — "we were not told".
    let mut symbols = Vec::new();
    if let Some((symoff, nsyms, stroff, strsize)) = symtab {
        let entry = if is64 { 16usize } else { 12 };
        let count = (nsyms as usize).min(MAX_ENTRIES);
        let table = read_table(src, symoff, entry, count, len);
        let strings = if stroff < len {
            src.at(
                stroff,
                (strsize as u64).min(MAX_STRTAB as u64).min(len - stroff) as usize,
            )
        } else {
            Vec::new()
        };
        let mut defined: Vec<(String, u64)> = Vec::new();
        let mut undefined: Vec<(String, Option<String>, Option<u64>)> = Vec::new();
        let mut exported: Vec<(String, u64)> = Vec::new();
        for i in 0..count {
            let from = i * entry;
            if from + entry > table.len() {
                break;
            }
            let mut e = Cur::new(&table[from..from + entry]);
            let n_strx = e.u32();
            let n_type = e.take(1).first().copied().unwrap_or(0);
            let _n_sect = e.take(1);
            let n_desc = e.u16();
            let n_value = if is64 { e.u64() } else { e.u32() as u64 };
            let name = str_at(&strings, n_strx as usize);
            // N_STAB: a debugger's entry, not a symbol anything binds to.
            if name.is_empty() || n_type & 0xE0 != 0 {
                continue;
            }
            match n_type & 0x0E {
                // N_SECT: defined in a section of this file.
                0x0E if n_value != 0 => {
                    defined.push((name.clone(), n_value));
                    // N_EXT: and visible outside it.
                    if n_type & 0x01 != 0 {
                        exported.push((name, n_value));
                    }
                }
                // N_UNDF: wanted from a library, which the ordinal in n_desc names.
                0x00 => {
                    let library = ((n_desc >> 8) as usize)
                        .checked_sub(1)
                        .and_then(|at| dylibs.get(at))
                        .cloned();
                    undefined.push((name, library, in_file(symoff + from as u64, len)));
                }
                _ => {}
            }
        }

        if !defined.is_empty() {
            rows.push(Row::group(format!("Symbols ({})", defined.len())));
            for (name, address) in &defined {
                let at = map_offset(&maps, *address);
                rows.push(Row::entry(
                    format!("  {name}"),
                    sym_cell(at, *address, 0),
                    at,
                ));
                symbols.push(Symbol {
                    name: name.clone(),
                    address: *address,
                    size: 0,
                });
            }
            if nsyms as usize > count {
                rows.push(Row::entry(
                    "  …",
                    format!("{} more entries not read", nsyms as usize - count),
                    None,
                ));
            }
        }
        if !undefined.is_empty() {
            rows.push(Row::group(format!("Imports ({})", undefined.len())));
            for (name, library, entry_at) in &undefined {
                rows.push(Row::entry(
                    format!("  {name}"),
                    match library {
                        Some(lib) => format!("{}  {lib}", off_cell(*entry_at)),
                        None => format!("{}  undefined", off_cell(*entry_at)),
                    },
                    *entry_at,
                ));
            }
        }
        if !exported.is_empty() {
            rows.push(Row::group(format!("Exports ({})", exported.len())));
            for (name, address) in &exported {
                let at = map_offset(&maps, *address);
                rows.push(Row::entry(
                    format!("  {name}"),
                    sym_cell(at, *address, 0),
                    at,
                ));
            }
        }
    }

    sort_symbols(&mut symbols);
    Structure {
        format: "Mach-O",
        summary,
        rows,
        symbols,
        maps,
        truncated,
    }
}

fn fat_macho(head: &[u8], count: u32, len: u64) -> Structure {
    let mut rows = vec![Row::group("Universal binary")];
    let mut c = Cur::new(head);
    c.big_endian = true;
    c.seek(8);
    for _ in 0..count.min(32) {
        let cputype = c.u32();
        let _sub = c.u32();
        let offset = c.u32() as u64;
        let size = c.u32() as u64;
        let _align = c.u32();
        rows.push(Row::entry(
            format!("  {}", macho_cpu(cputype)),
            format!("{offset:08X}  {}", size_cell(size)),
            in_file(offset, len),
        ));
    }
    Structure {
        format: "Mach-O",
        summary: format!("Mach-O universal binary · {count} slices"),
        rows,
        // A slice is a whole file of its own: opening one is what answers for it.
        symbols: Vec::new(),
        maps: Vec::new(),
        truncated: c.short,
    }
}

fn macho_cpu(t: u32) -> &'static str {
    match t {
        7 => "i386",
        0x0100_0007 => "x86-64",
        12 => "ARM",
        0x0100_000C => "ARM64",
        0x0200_000C => "ARM64_32",
        18 => "PowerPC",
        0x0100_0012 => "PowerPC64",
        _ => "unknown cpu",
    }
}

fn macho_filetype(t: u32) -> &'static str {
    match t {
        1 => "object file",
        2 => "executable",
        4 => "core dump",
        5 => "preload",
        6 => "dylib",
        7 => "dylinker",
        8 => "bundle",
        9 => "dylib stub",
        10 => "debug symbols",
        11 => "kext",
        _ => "other",
    }
}

fn macho_flags(f: u32) -> String {
    let mut parts = Vec::new();
    if f & 0x1 != 0 {
        parts.push("NOUNDEFS");
    }
    if f & 0x80 != 0 {
        parts.push("TWOLEVEL");
    }
    if f & 0x0020_0000 != 0 {
        parts.push("PIE");
    }
    if parts.is_empty() {
        format!("{f:#X}")
    } else {
        parts.join(" ")
    }
}

fn macho_cmd(c: u32) -> String {
    match c {
        2 => "LC_SYMTAB".into(),
        0x0B => "LC_DYSYMTAB".into(),
        0x1B => "LC_UUID".into(),
        0x1D => "LC_CODE_SIGNATURE".into(),
        0x21 => "LC_ENCRYPTION_INFO".into(),
        0x22 => "LC_DYLD_INFO".into(),
        0x8000_0022 => "LC_DYLD_INFO_ONLY".into(),
        0x8000_001C => "LC_RPATH".into(),
        0x8000_0028 => "LC_MAIN".into(),
        0x32 => "LC_BUILD_VERSION".into(),
        other => format!("LC_{other:#X}"),
    }
}

// ── ar archives ─────────────────────────────────────────────────────────────────────────────

fn ar(src: &mut dyn Bytes, len: u64) -> Structure {
    let mut rows = vec![Row::group("Archive members")];
    let mut at = 8u64;
    let mut count = 0usize;
    let mut truncated = false;
    while at < len && count < MAX_ENTRIES {
        let header = src.at(at, 60);
        if header.len() < 60 {
            truncated = !header.is_empty();
            break;
        }
        let name = String::from_utf8_lossy(&header[0..16]).trim().to_string();
        let size: u64 = String::from_utf8_lossy(&header[48..58])
            .trim()
            .parse()
            .unwrap_or(0);
        if size == 0 && name.is_empty() {
            break;
        }
        rows.push(Row::entry(
            format!("  {name}"),
            format!("{:08X}  {}", at + 60, size_cell(size)),
            in_file(at + 60, len),
        ));
        // Members are two-byte aligned.
        at = at + 60 + size + (size & 1);
        count += 1;
    }
    Structure {
        format: "ar",
        summary: format!("ar archive · {count} members"),
        rows,
        // The members are the binaries; the archive itself defines nothing.
        symbols: Vec::new(),
        maps: Vec::new(),
        truncated,
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────────────────────

/// An offset is only a target if it is really inside the file.
fn in_file(offset: u64, len: u64) -> Option<u64> {
    (offset < len).then_some(offset)
}

/// Remember that `size` bytes at `offset` load at `address` — but only the bytes the file
/// really has, so every offset this mapping ever produces is inside the file.
fn add_map(maps: &mut Vec<Mapping>, address: u64, size: u64, offset: u64, len: u64) {
    if address == 0 || size == 0 || offset >= len {
        return;
    }
    let size = size.min(len - offset);
    if size > 0 {
        maps.push(Mapping {
            address,
            size,
            offset,
        });
    }
}

/// File offset for a virtual address. Mappings may overlap — a segment covers the sections
/// inside it — and any of them answers correctly, so the first hit wins.
fn map_offset(maps: &[Mapping], va: u64) -> Option<u64> {
    maps.iter()
        .find(|m| va >= m.address && va - m.address < m.size)
        .map(|m| m.offset + (va - m.address))
}

/// Symbols are sorted by address so that a lookup is a binary search, and the duplicates two
/// tables of the same file are bound to agree on are dropped.
fn sort_symbols(symbols: &mut Vec<Symbol>) {
    symbols.sort_by(|a, b| a.address.cmp(&b.address).then_with(|| a.name.cmp(&b.name)));
    symbols.dedup();
}

/// An offset cell: where the bytes are, or the dots the rest of this file uses to admit that
/// they are nowhere.
fn off_cell(offset: Option<u64>) -> String {
    match offset {
        Some(at) => format!("{at:08X}"),
        None => "········".to_string(),
    }
}

/// The value cell of a row that names an address: where it is in the file, how big it is where
/// the format says, and where it loads.
fn sym_cell(offset: Option<u64>, address: u64, size: u64) -> String {
    if size == 0 {
        format!("{}  @ {address:X}", off_cell(offset))
    } else {
        format!("{}  {}  @ {address:X}", off_cell(offset), size_cell(size))
    }
}

fn read_table(src: &mut dyn Bytes, at: u64, entry: usize, count: usize, len: u64) -> Vec<u8> {
    if entry == 0 || count == 0 || at >= len {
        return Vec::new();
    }
    let want = entry.saturating_mul(count);
    let room = (len - at).min(want as u64) as usize;
    src.at(at, room)
}

fn str_at(table: &[u8], at: usize) -> String {
    if at >= table.len() {
        return String::new();
    }
    let end = table[at..]
        .iter()
        .position(|b| *b == 0)
        .map(|p| at + p)
        .unwrap_or(table.len());
    String::from_utf8_lossy(&table[at..end]).to_string()
}

fn size_cell(size: u64) -> String {
    scmd_core::format_size(size)
}

/// A timestamp as a date, or the raw number when it is obviously not one.
fn unix_time(secs: u64) -> String {
    if secs == 0 || secs > 4_000_000_000 {
        return format!("{secs}");
    }
    // Days since the epoch, then the civil date — no chrono needed for one field.
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil, the standard trick.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal but valid ELF64: header, two section headers (null + .text) and a string
    /// table, built by hand so the test does not depend on a binary in the tree.
    fn elf64() -> Vec<u8> {
        let mut f = vec![0u8; 0x200];
        f[..4].copy_from_slice(b"\x7fELF");
        f[4] = 2; // ELF64
        f[5] = 1; // little endian
        f[6] = 1; // version
        f[7] = 3; // Linux
        f[16..18].copy_from_slice(&3u16.to_le_bytes()); // e_type = DYN
        f[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // x86-64
        f[20..24].copy_from_slice(&1u32.to_le_bytes());
        f[24..32].copy_from_slice(&0x1B50u64.to_le_bytes()); // e_entry
        f[32..40].copy_from_slice(&0x40u64.to_le_bytes()); // e_phoff
        f[40..48].copy_from_slice(&0x80u64.to_le_bytes()); // e_shoff
        f[54..56].copy_from_slice(&56u16.to_le_bytes()); // e_phentsize
        f[56..58].copy_from_slice(&0u16.to_le_bytes()); // e_phnum
        f[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        f[60..62].copy_from_slice(&3u16.to_le_bytes()); // e_shnum
        f[62..64].copy_from_slice(&2u16.to_le_bytes()); // e_shstrndx

        // section 1: .text
        let s1 = 0x80 + 64;
        f[s1..s1 + 4].copy_from_slice(&1u32.to_le_bytes()); // sh_name -> ".text"
        f[s1 + 4..s1 + 8].copy_from_slice(&1u32.to_le_bytes()); // PROGBITS
        f[s1 + 8..s1 + 16].copy_from_slice(&6u64.to_le_bytes()); // ALLOC|EXEC
        f[s1 + 16..s1 + 24].copy_from_slice(&0x1000u64.to_le_bytes()); // sh_addr
        f[s1 + 24..s1 + 32].copy_from_slice(&0x180u64.to_le_bytes()); // sh_offset
        f[s1 + 32..s1 + 40].copy_from_slice(&0x40u64.to_le_bytes()); // sh_size

        // section 2: .shstrtab
        let s2 = 0x80 + 128;
        f[s2..s2 + 4].copy_from_slice(&7u32.to_le_bytes()); // sh_name -> ".shstrtab"
        f[s2 + 4..s2 + 8].copy_from_slice(&3u32.to_le_bytes()); // STRTAB
        f[s2 + 24..s2 + 32].copy_from_slice(&0x1C0u64.to_le_bytes()); // sh_offset
        f[s2 + 32..s2 + 40].copy_from_slice(&17u64.to_le_bytes()); // sh_size

        let strtab = b"\0.text\0.shstrtab\0";
        f[0x1C0..0x1C0 + strtab.len()].copy_from_slice(strtab);
        f
    }

    #[test]
    fn an_elf_says_what_it_is_and_where_its_sections_are() {
        let bytes = elf64();
        let s = read(&mut bytes.as_slice()).expect("ELF recognised");
        assert_eq!(s.format, "ELF");
        assert!(s.summary.contains("ELF64"), "{}", s.summary);
        assert!(s.summary.contains("x86-64"), "{}", s.summary);
        assert!(s.summary.contains("DYN"), "{}", s.summary);
        assert!(!s.truncated);

        let text = s
            .rows
            .iter()
            .find(|r| r.label.trim() == ".text")
            .expect("the .text section is listed");
        assert_eq!(text.offset, Some(0x180), "Enter must go to its bytes");
        assert!(text.value.contains("a-x"), "{}", text.value);

        // The header fields carry their raw bytes as well as their meaning.
        let machine = s
            .rows
            .iter()
            .find(|r| r.label == "e_machine")
            .expect("e_machine");
        assert_eq!(machine.raw, "003E");
        assert_eq!(machine.value, "x86-64");
    }

    #[test]
    fn a_file_that_stops_early_fills_what_it_can() {
        let full = elf64();
        // Cut it off inside the section table: the header still decodes.
        let short = &full[..0x90];
        let s = read(&mut (short as &[u8])).expect("still an ELF");
        assert!(s.truncated, "a short read has to be reported");
        assert!(s.rows.iter().any(|r| r.label == "e_type"));
    }

    #[test]
    fn a_hostile_section_count_does_not_run_away() {
        let mut bytes = elf64();
        // Claim 65 535 sections, each 56 bytes, from an offset near the end.
        bytes[60..62].copy_from_slice(&0xFFFFu16.to_le_bytes());
        let s = read(&mut bytes.as_slice()).expect("still parses");
        let entries = s.rows.iter().filter(|r| r.kind == RowKind::Entry).count();
        assert!(entries <= MAX_ENTRIES + 8, "{entries} rows is too many");
        // And no row points outside the file.
        for row in s.targets() {
            assert!(
                row.offset.unwrap() < bytes.len() as u64,
                "{:?} points past the end",
                row
            );
        }
    }

    #[test]
    fn an_offset_past_the_end_is_not_a_target() {
        let mut bytes = elf64();
        // .text claims to start past the end of the file.
        let s1 = 0x80 + 64;
        bytes[s1 + 24..s1 + 32].copy_from_slice(&0xFFFF_FFFFu64.to_le_bytes());
        let s = read(&mut bytes.as_slice()).unwrap();
        let text = s.rows.iter().find(|r| r.label.trim() == ".text").unwrap();
        assert_eq!(text.offset, None, "Enter must not offer to jump nowhere");
    }

    #[test]
    fn a_pe_decodes_its_two_headers() {
        let mut f = vec![0u8; 0x400];
        f[..2].copy_from_slice(b"MZ");
        f[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        f[0x80..0x84].copy_from_slice(b"PE\0\0");
        f[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes()); // x86-64
        f[0x86..0x88].copy_from_slice(&1u16.to_le_bytes()); // one section
        f[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes()); // SizeOfOptionalHeader
        f[0x96..0x98].copy_from_slice(&0x2022u16.to_le_bytes()); // DLL
        f[0x98..0x9A].copy_from_slice(&0x20Bu16.to_le_bytes()); // PE32+
                                                                // section table at 0x80 + 24 + 0xF0
        let sec = 0x80 + 24 + 0xF0;
        f[sec..sec + 5].copy_from_slice(b".text");
        f[sec + 16..sec + 20].copy_from_slice(&0x40u32.to_le_bytes()); // SizeOfRawData
        f[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes()); // PointerToRawData
        f[sec + 36..sec + 40].copy_from_slice(&0x6000_0020u32.to_le_bytes()); // r-x

        let s = read(&mut f.as_slice()).expect("PE recognised");
        assert_eq!(s.format, "PE");
        assert!(s.summary.contains("PE32+"), "{}", s.summary);
        assert!(s.summary.contains("x86-64"), "{}", s.summary);
        assert!(s.summary.contains("DLL"), "{}", s.summary);
        let text = s.rows.iter().find(|r| r.label.trim() == ".text").unwrap();
        assert_eq!(text.offset, Some(0x200));
    }

    #[test]
    fn a_macho_lists_its_segments() {
        let mut f = vec![0u8; 0x200];
        f[..4].copy_from_slice(&0xFEED_FACFu32.to_le_bytes());
        f[4..8].copy_from_slice(&0x0100_000Cu32.to_le_bytes()); // ARM64
        f[12..16].copy_from_slice(&2u32.to_le_bytes()); // executable
        f[16..20].copy_from_slice(&1u32.to_le_bytes()); // one command
        f[20..24].copy_from_slice(&72u32.to_le_bytes()); // sizeofcmds
        let lc = 32;
        f[lc..lc + 4].copy_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
        f[lc + 4..lc + 8].copy_from_slice(&72u32.to_le_bytes());
        f[lc + 8..lc + 15].copy_from_slice(b"__TEXT\0");
        f[lc + 24..lc + 32].copy_from_slice(&0x1_0000_0000u64.to_le_bytes()); // vmaddr
        f[lc + 40..lc + 48].copy_from_slice(&0u64.to_le_bytes()); // fileoff
        f[lc + 48..lc + 56].copy_from_slice(&0x100u64.to_le_bytes()); // filesize

        let s = read(&mut f.as_slice()).expect("Mach-O recognised");
        assert!(s.summary.contains("ARM64"), "{}", s.summary);
        assert!(s.summary.contains("executable"), "{}", s.summary);
        let seg = s.rows.iter().find(|r| r.label.trim() == "__TEXT").unwrap();
        assert_eq!(seg.offset, Some(0));
    }

    #[test]
    fn plain_text_is_not_a_binary_and_says_nothing() {
        let text = b"#!/bin/sh\necho hello\n";
        assert!(read(&mut (text as &[u8])).is_none());
    }

    #[test]
    fn a_timestamp_becomes_a_date() {
        assert_eq!(unix_time(0), "0");
        assert_eq!(unix_time(1_000_000_000), "2001-09-09 01:46:40 UTC");
    }

    // ── symbols, imports and exports (SPEC 05 §1.8, §1.9) ───────────────────────────────

    #[allow(clippy::too_many_arguments)]
    fn sh(
        f: &mut [u8],
        i: usize,
        name: u32,
        kind: u32,
        flags: u64,
        addr: u64,
        off: u64,
        size: u64,
        link: u32,
        entsize: u64,
    ) {
        let at = 0x80 + i * 64;
        f[at..at + 4].copy_from_slice(&name.to_le_bytes());
        f[at + 4..at + 8].copy_from_slice(&kind.to_le_bytes());
        f[at + 8..at + 16].copy_from_slice(&flags.to_le_bytes());
        f[at + 16..at + 24].copy_from_slice(&addr.to_le_bytes());
        f[at + 24..at + 32].copy_from_slice(&off.to_le_bytes());
        f[at + 32..at + 40].copy_from_slice(&size.to_le_bytes());
        f[at + 40..at + 44].copy_from_slice(&link.to_le_bytes());
        f[at + 56..at + 64].copy_from_slice(&entsize.to_le_bytes());
    }

    fn sym(f: &mut [u8], at: usize, name: u32, info: u8, shndx: u16, value: u64, size: u64) {
        f[at..at + 4].copy_from_slice(&name.to_le_bytes());
        f[at + 4] = info;
        f[at + 6..at + 8].copy_from_slice(&shndx.to_le_bytes());
        f[at + 8..at + 16].copy_from_slice(&value.to_le_bytes());
        f[at + 16..at + 24].copy_from_slice(&size.to_le_bytes());
    }

    /// The offsets this fixture puts its tables at, so a test can corrupt one by name.
    const SYMTAB: usize = 0x380;
    const DYNSYM: usize = 0x3D0;

    /// An ELF64 with the five tables that say what it defines and what it needs: `.symtab`
    /// with `.strtab`, `.dynsym` with `.dynstr`, and `.dynamic`. Built by hand for the same
    /// reason as `elf64()` above — a test should not depend on a binary in the tree.
    ///
    /// `.text` loads at 0x1000 from offset 0x300, so every address here has a known offset.
    fn elf64_with_symbols() -> Vec<u8> {
        let mut f = vec![0u8; 0x600];
        f[..4].copy_from_slice(b"\x7fELF");
        f[4] = 2; // ELF64
        f[5] = 1; // little endian
        f[6] = 1;
        f[7] = 3;
        f[16..18].copy_from_slice(&3u16.to_le_bytes()); // DYN
        f[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // x86-64
        f[20..24].copy_from_slice(&1u32.to_le_bytes());
        f[24..32].copy_from_slice(&0x1000u64.to_le_bytes()); // e_entry, inside .text
        f[40..48].copy_from_slice(&0x80u64.to_le_bytes()); // e_shoff
        f[58..60].copy_from_slice(&64u16.to_le_bytes()); // e_shentsize
        f[60..62].copy_from_slice(&8u16.to_le_bytes()); // e_shnum
        f[62..64].copy_from_slice(&1u16.to_le_bytes()); // e_shstrndx

        // The section names, in section order, so each index is the one its header needs.
        let names = [
            ".shstrtab",
            ".text",
            ".symtab",
            ".strtab",
            ".dynsym",
            ".dynstr",
            ".dynamic",
        ];
        let mut shstr = vec![0u8];
        let mut idx = Vec::new();
        for n in names {
            idx.push(shstr.len() as u32);
            shstr.extend_from_slice(n.as_bytes());
            shstr.push(0);
        }
        f[0x480..0x480 + shstr.len()].copy_from_slice(&shstr);

        sh(&mut f, 1, idx[0], 3, 0, 0, 0x480, shstr.len() as u64, 0, 0);
        sh(&mut f, 2, idx[1], 1, 6, 0x1000, 0x300, 0x40, 0, 0); // .text
        sh(&mut f, 3, idx[2], 2, 0, 0, SYMTAB as u64, 72, 4, 24); // .symtab → .strtab
        sh(&mut f, 4, idx[3], 3, 0, 0, 0x500, 18, 0, 0); // .strtab
        sh(&mut f, 5, idx[4], 11, 2, 0, DYNSYM as u64, 72, 6, 24); // .dynsym → .dynstr
        sh(&mut f, 6, idx[5], 3, 0, 0, 0x540, 40, 0, 0); // .dynstr
        sh(&mut f, 7, idx[6], 6, 3, 0, 0x420, 48, 6, 16); // .dynamic

        f[0x500..0x512].copy_from_slice(b"\0scmd_main\0helper\0");
        sym(&mut f, SYMTAB + 24, 1, 0x12, 2, 0x1000, 0x20); // global function
        sym(&mut f, SYMTAB + 48, 11, 0x02, 2, 0x1030, 0x10); // local function

        f[0x540..0x568].copy_from_slice(b"\0printf\0libc.so.6\0scmd_run\0libscmd.so.1\0");
        sym(&mut f, DYNSYM + 24, 1, 0x12, 0, 0, 0); // printf, undefined
        sym(&mut f, DYNSYM + 48, 18, 0x12, 2, 0x1020, 0x10); // scmd_run, exported

        // .dynamic: DT_NEEDED libc.so.6, DT_SONAME libscmd.so.1, DT_NULL
        f[0x420..0x428].copy_from_slice(&1u64.to_le_bytes());
        f[0x428..0x430].copy_from_slice(&8u64.to_le_bytes());
        f[0x430..0x438].copy_from_slice(&14u64.to_le_bytes());
        f[0x438..0x440].copy_from_slice(&27u64.to_le_bytes());
        f
    }

    fn row<'a>(s: &'a Structure, label: &str) -> Option<&'a Row> {
        s.rows.iter().find(|r| r.label.trim() == label)
    }

    /// No row may ever offer to jump outside the file.
    fn targets_stay_inside(s: &Structure, len: u64) {
        for r in s.targets() {
            assert!(r.offset.unwrap() < len, "{r:?} points past the end");
        }
    }

    #[test]
    fn an_elf_lists_its_functions_imports_and_exports() {
        let bytes = elf64_with_symbols();
        let s = read(&mut bytes.as_slice()).expect("ELF recognised");
        assert!(!s.truncated);

        // `.symtab` is the fuller table, so it is the one the pane lists.
        assert!(
            s.rows.iter().any(|r| r.label == "Symbols (2)"),
            "{:?}",
            s.rows
        );
        let main = row(&s, "scmd_main").expect("scmd_main is listed");
        assert_eq!(
            main.offset,
            Some(0x300),
            "Enter goes to the function's bytes"
        );
        assert!(main.value.contains("@ 1000"), "{}", main.value);
        assert!(row(&s, "helper").is_some());

        assert!(
            s.rows
                .iter()
                .any(|r| r.label == "Imports (1 libraries, 1 symbols)"),
            "{:?}",
            s.rows
        );
        let lib = row(&s, "libc.so.6").expect("the needed library is listed");
        assert_eq!(lib.value, "needed library");
        let printf = row(&s, "printf").expect("the undefined symbol is listed");
        assert_eq!(printf.offset, Some(DYNSYM as u64 + 24), "its own entry");
        assert!(printf.value.contains("undefined"), "{}", printf.value);

        assert!(
            s.rows.iter().any(|r| r.label == "Exports (1)"),
            "{:?}",
            s.rows
        );
        let exported = row(&s, "scmd_run").expect("the exported symbol is listed");
        assert_eq!(exported.offset, Some(0x320));

        // The SONAME comes from the same two sections, and the pane says it.
        let soname = row(&s, "soname").expect("soname");
        assert_eq!(soname.value, "libscmd.so.1");

        // Nothing was capped, so nothing claims to have been.
        assert!(row(&s, "…").is_none(), "{:?}", s.rows);
        // The entry point is a virtual address until something maps it.
        assert_eq!(row(&s, "e_entry").unwrap().offset, Some(0x300));
        targets_stay_inside(&s, bytes.len() as u64);
    }

    #[test]
    fn a_symbol_resolves_an_address_to_a_name() {
        let bytes = elf64_with_symbols();
        let s = read(&mut bytes.as_slice()).unwrap();
        // Both tables feed the lookup, so a symbol only one of them has is still found.
        assert_eq!(s.symbols.len(), 3, "{:?}", s.symbols);
        assert_eq!(s.symbol_at(0x1000), Some("scmd_main"));
        assert_eq!(s.symbol_at(0x1005), Some("scmd_main"));
        assert_eq!(s.symbol_at(0x1025), Some("scmd_run"));
        assert_eq!(s.symbol_at(0x1035), Some("helper"));
        assert_eq!(s.symbol_at(0x900), None, "below every symbol");
        assert_eq!(s.symbol_at(0x1045), None, "past the end of the last one");
        assert_eq!(s.symbol_entry(0x1000).unwrap().size, 0x20);

        assert_eq!(s.offset_for_va(0x1000), Some(0x300));
        assert_eq!(s.offset_for_va(0x103F), Some(0x33F));
        assert_eq!(s.offset_for_va(0x2000), None, "nothing loads there");
    }

    #[test]
    fn a_symbol_table_pointing_past_the_end_adds_no_rows() {
        let mut bytes = elf64_with_symbols();
        // .symtab claims to live past the end of the file.
        let at = 0x80 + 3 * 64 + 24;
        bytes[at..at + 8].copy_from_slice(&0xFFFF_FFFFu64.to_le_bytes());
        let s = read(&mut bytes.as_slice()).expect("still an ELF");
        assert!(row(&s, "scmd_main").is_none(), "it was never read");
        assert!(row(&s, "helper").is_none());
        // `.dynsym` is still readable, so the pane falls back to what it has.
        assert_eq!(s.symbols.len(), 1, "{:?}", s.symbols);
        assert_eq!(s.symbol_at(0x1025), Some("scmd_run"));
        targets_stay_inside(&s, bytes.len() as u64);
    }

    #[test]
    fn a_string_table_index_out_of_range_leaves_the_symbol_nameless() {
        let mut bytes = elf64_with_symbols();
        // scmd_main's name is somewhere beyond the end of .strtab.
        bytes[SYMTAB + 24..SYMTAB + 28].copy_from_slice(&0xFFFF_FFFFu32.to_le_bytes());
        let s = read(&mut bytes.as_slice()).unwrap();
        assert!(row(&s, "scmd_main").is_none(), "no name is invented for it");
        assert!(row(&s, "helper").is_some(), "the next symbol still decodes");
        assert_eq!(s.symbol_at(0x1005), None);
        targets_stay_inside(&s, bytes.len() as u64);
    }

    #[test]
    fn a_symbol_count_of_ffff_is_capped_like_any_other_table() {
        let mut bytes = elf64_with_symbols();
        // .symtab claims 65 535 symbols from an offset near the end of the file.
        let at = 0x80 + 3 * 64 + 32;
        bytes[at..at + 8].copy_from_slice(&(0xFFFFu64 * 24).to_le_bytes());
        let s = read(&mut bytes.as_slice()).unwrap();
        let entries = s.rows.iter().filter(|r| r.kind == RowKind::Entry).count();
        assert!(entries <= MAX_ENTRIES + 16, "{entries} rows is too many");
        assert!(s.symbols.len() <= MAX_ENTRIES * 2);
        // And the pane owns up to the entries it did not read, rather than pretending the
        // table was that short.
        let capped = row(&s, "…").expect("a capped table says so");
        assert!(capped.value.contains("not read"), "{}", capped.value);
        targets_stay_inside(&s, bytes.len() as u64);
    }

    /// A PE32+ DLL with one section, an export directory and an import directory. `.text`
    /// loads at RVA 0x1000 from offset 0x200, on an image base of 0x1_4000_0000.
    fn pe64() -> Vec<u8> {
        let mut f = vec![0u8; 0x600];
        f[..2].copy_from_slice(b"MZ");
        f[0x3C..0x40].copy_from_slice(&0x80u32.to_le_bytes());
        f[0x80..0x84].copy_from_slice(b"PE\0\0");
        f[0x84..0x86].copy_from_slice(&0x8664u16.to_le_bytes()); // x86-64
        f[0x86..0x88].copy_from_slice(&1u16.to_le_bytes()); // one section
        f[0x94..0x96].copy_from_slice(&0xF0u16.to_le_bytes()); // SizeOfOptionalHeader
        f[0x96..0x98].copy_from_slice(&0x2022u16.to_le_bytes()); // DLL
        let opt = 0x98;
        f[opt..opt + 2].copy_from_slice(&0x20Bu16.to_le_bytes()); // PE32+
        f[opt + 24..opt + 32].copy_from_slice(&0x1_4000_0000u64.to_le_bytes()); // ImageBase
        f[opt + 108..opt + 112].copy_from_slice(&16u32.to_le_bytes()); // NumberOfRvaAndSizes
        let dd = opt + 112;
        f[dd..dd + 4].copy_from_slice(&0x1100u32.to_le_bytes()); // export directory
        f[dd + 4..dd + 8].copy_from_slice(&40u32.to_le_bytes());
        f[dd + 8..dd + 12].copy_from_slice(&0x1300u32.to_le_bytes()); // import directory
        f[dd + 12..dd + 16].copy_from_slice(&40u32.to_le_bytes());

        let sec = 0x80 + 24 + 0xF0;
        f[sec..sec + 5].copy_from_slice(b".text");
        f[sec + 8..sec + 12].copy_from_slice(&0x400u32.to_le_bytes()); // VirtualSize
        f[sec + 12..sec + 16].copy_from_slice(&0x1000u32.to_le_bytes()); // VirtualAddress
        f[sec + 16..sec + 20].copy_from_slice(&0x400u32.to_le_bytes()); // SizeOfRawData
        f[sec + 20..sec + 24].copy_from_slice(&0x200u32.to_le_bytes()); // PointerToRawData
        f[sec + 36..sec + 40].copy_from_slice(&0x6000_0020u32.to_le_bytes()); // r-x

        // The export directory at RVA 0x1100 (offset 0x300).
        let ed = 0x300;
        f[ed + 12..ed + 16].copy_from_slice(&0x1200u32.to_le_bytes()); // Name
        f[ed + 16..ed + 20].copy_from_slice(&1u32.to_le_bytes()); // Base
        f[ed + 20..ed + 24].copy_from_slice(&2u32.to_le_bytes()); // NumberOfFunctions
        f[ed + 24..ed + 28].copy_from_slice(&2u32.to_le_bytes()); // NumberOfNames
        f[ed + 28..ed + 32].copy_from_slice(&0x1140u32.to_le_bytes()); // AddressOfFunctions
        f[ed + 32..ed + 36].copy_from_slice(&0x1150u32.to_le_bytes()); // AddressOfNames
        f[ed + 36..ed + 40].copy_from_slice(&0x1160u32.to_le_bytes()); // AddressOfNameOrdinals
        f[0x340..0x344].copy_from_slice(&0x1000u32.to_le_bytes());
        f[0x344..0x348].copy_from_slice(&0x1050u32.to_le_bytes());
        f[0x350..0x354].copy_from_slice(&0x1210u32.to_le_bytes());
        f[0x354..0x358].copy_from_slice(&0x1220u32.to_le_bytes());
        f[0x362..0x364].copy_from_slice(&1u16.to_le_bytes()); // the second name is ordinal 1
        f[0x400..0x40B].copy_from_slice(b"scmdlib.dll");
        f[0x410..0x418].copy_from_slice(b"scmd_run");
        f[0x420..0x429].copy_from_slice(b"scmd_quit");

        // The import directory at RVA 0x1300 (offset 0x500), one descriptor and a terminator.
        let id = 0x500;
        f[id..id + 4].copy_from_slice(&0x1360u32.to_le_bytes()); // OriginalFirstThunk
        f[id + 12..id + 16].copy_from_slice(&0x1340u32.to_le_bytes()); // Name
        f[id + 16..id + 20].copy_from_slice(&0x13A0u32.to_le_bytes()); // FirstThunk
        f[0x540..0x54C].copy_from_slice(b"kernel32.dll");
        f[0x572..0x580].copy_from_slice(b"GetProcAddress"); // past a two-byte hint
        f[0x560..0x568].copy_from_slice(&0x1370u64.to_le_bytes());
        f[0x568..0x570].copy_from_slice(&0x8000_0000_0000_0007u64.to_le_bytes());
        f
    }

    #[test]
    fn a_pe_lists_its_imports_and_exports() {
        let bytes = pe64();
        let s = read(&mut bytes.as_slice()).expect("PE recognised");
        assert!(s.rows.iter().any(|r| r.label == "Imports (1 libraries)"));
        let lib = row(&s, "kernel32.dll").expect("the library is listed");
        assert_eq!(lib.offset, Some(0x500), "its descriptor");
        assert!(lib.value.contains("2 symbols"), "{}", lib.value);
        let import = row(&s, "GetProcAddress").expect("the imported symbol is listed");
        assert_eq!(
            import.offset,
            Some(0x5A0),
            "the slot the code calls through"
        );
        // An import by ordinal has no name, and none is invented for it.
        assert!(row(&s, "ordinal 7").is_some(), "{:?}", s.rows);

        assert!(s.rows.iter().any(|r| r.label == "Exports (2)"));
        assert_eq!(row(&s, "library name").unwrap().value, "scmdlib.dll");
        let run = row(&s, "scmd_run").expect("the export is listed");
        assert_eq!(run.offset, Some(0x200));
        assert!(run.value.contains("ordinal 1"), "{}", run.value);
        assert!(row(&s, "scmd_quit").unwrap().value.contains("ordinal 2"));

        assert_eq!(s.symbols.len(), 2, "{:?}", s.symbols);
        assert_eq!(s.symbol_at(0x1_4000_1004), Some("scmd_run"));
        assert_eq!(s.symbol_at(0x1_4000_1050), Some("scmd_quit"));
        assert_eq!(s.offset_for_va(0x1_4000_1000), Some(0x200));
        targets_stay_inside(&s, bytes.len() as u64);
    }

    #[test]
    fn an_import_with_a_bogus_offset_is_not_a_target() {
        let mut bytes = pe64();
        // The descriptor's name and thunk list both point outside every section.
        bytes[0x500..0x504].copy_from_slice(&0x9999_9999u32.to_le_bytes());
        bytes[0x50C..0x510].copy_from_slice(&0x9999_9999u32.to_le_bytes());
        bytes[0x510..0x514].copy_from_slice(&0x9999_9999u32.to_le_bytes());
        let s = read(&mut bytes.as_slice()).unwrap();
        assert!(row(&s, "kernel32.dll").is_none(), "no name is invented");
        assert!(row(&s, "GetProcAddress").is_none(), "nor any symbol");
        assert!(
            row(&s, "[0]").is_some(),
            "the descriptor itself is still shown"
        );
        targets_stay_inside(&s, bytes.len() as u64);

        // And a directory that is nowhere at all contributes nothing.
        let mut bytes = pe64();
        bytes[0x98 + 120..0x98 + 124].copy_from_slice(&0x9999_9999u32.to_le_bytes());
        let s = read(&mut bytes.as_slice()).unwrap();
        assert!(!s.rows.iter().any(|r| r.label.starts_with("Imports")));
        targets_stay_inside(&s, bytes.len() as u64);
    }

    #[test]
    fn an_import_directory_that_loops_still_ends() {
        let mut bytes = pe64();
        // A descriptor array with no terminator, repeating over a whole page.
        bytes[0x98 + 120..0x98 + 124].copy_from_slice(&0x1200u32.to_le_bytes());
        let mut at = 0x400;
        while at + 20 <= 0x500 {
            bytes[at..at + 4].copy_from_slice(&0x1360u32.to_le_bytes());
            bytes[at + 12..at + 16].copy_from_slice(&0x1340u32.to_le_bytes());
            bytes[at + 16..at + 20].copy_from_slice(&0x13A0u32.to_le_bytes());
            at += 20;
        }
        let s = read(&mut bytes.as_slice()).unwrap();
        let libraries = s
            .rows
            .iter()
            .filter(|r| r.label.trim() == "kernel32.dll")
            .count();
        assert!(libraries > 1 && libraries <= MAX_ENTRIES, "{libraries}");
        assert!(s.rows.len() < 2048, "{} rows is too many", s.rows.len());
        targets_stay_inside(&s, bytes.len() as u64);
    }

    /// A 64-bit Mach-O with a segment, a dylib and a symbol table: `__TEXT` loads at
    /// 0x1_0000_0000 from offset 0.
    fn macho64_with_symbols() -> Vec<u8> {
        let mut f = vec![0u8; 0x400];
        f[..4].copy_from_slice(&0xFEED_FACFu32.to_le_bytes());
        f[4..8].copy_from_slice(&0x0100_0007u32.to_le_bytes()); // x86-64
        f[12..16].copy_from_slice(&6u32.to_le_bytes()); // dylib
        f[16..20].copy_from_slice(&3u32.to_le_bytes()); // three commands
        f[20..24].copy_from_slice(&152u32.to_le_bytes()); // sizeofcmds

        let lc = 32;
        f[lc..lc + 4].copy_from_slice(&0x19u32.to_le_bytes()); // LC_SEGMENT_64
        f[lc + 4..lc + 8].copy_from_slice(&72u32.to_le_bytes());
        f[lc + 8..lc + 15].copy_from_slice(b"__TEXT\0");
        f[lc + 24..lc + 32].copy_from_slice(&0x1_0000_0000u64.to_le_bytes()); // vmaddr
        f[lc + 32..lc + 40].copy_from_slice(&0x400u64.to_le_bytes()); // vmsize
        f[lc + 48..lc + 56].copy_from_slice(&0x400u64.to_le_bytes()); // filesize

        let lc = 104;
        f[lc..lc + 4].copy_from_slice(&2u32.to_le_bytes()); // LC_SYMTAB
        f[lc + 4..lc + 8].copy_from_slice(&24u32.to_le_bytes());
        f[lc + 8..lc + 12].copy_from_slice(&0x200u32.to_le_bytes()); // symoff
        f[lc + 12..lc + 16].copy_from_slice(&2u32.to_le_bytes()); // nsyms
        f[lc + 16..lc + 20].copy_from_slice(&0x280u32.to_le_bytes()); // stroff
        f[lc + 20..lc + 24].copy_from_slice(&20u32.to_le_bytes()); // strsize

        let lc = 128;
        f[lc..lc + 4].copy_from_slice(&0x0Cu32.to_le_bytes()); // LC_LOAD_DYLIB
        f[lc + 4..lc + 8].copy_from_slice(&56u32.to_le_bytes());
        f[lc + 24..lc + 50].copy_from_slice(b"/usr/lib/libSystem.B.dylib");

        // nlist_64: a defined external symbol, and one wanted from dylib ordinal 1.
        f[0x200..0x204].copy_from_slice(&1u32.to_le_bytes());
        f[0x204] = 0x0F; // N_SECT | N_EXT
        f[0x205] = 1;
        f[0x208..0x210].copy_from_slice(&0x1_0000_0100u64.to_le_bytes());
        f[0x210..0x214].copy_from_slice(&12u32.to_le_bytes());
        f[0x214] = 0x01; // N_UNDF | N_EXT
        f[0x216..0x218].copy_from_slice(&(1u16 << 8).to_le_bytes()); // library ordinal 1
        f[0x280..0x294].copy_from_slice(b"\0_scmd_main\0_printf\0");
        f
    }

    #[test]
    fn a_macho_reads_its_symbol_table_and_names_each_import() {
        let bytes = macho64_with_symbols();
        let s = read(&mut bytes.as_slice()).expect("Mach-O recognised");
        let main = row(&s, "_scmd_main").expect("the defined symbol is listed");
        assert_eq!(main.offset, Some(0x100), "Enter goes to its bytes");
        // Two-level namespace: the ordinal in the entry names the library.
        let printf = row(&s, "_printf").expect("the undefined symbol is listed");
        assert_eq!(
            printf.value.split_whitespace().last().unwrap(),
            "/usr/lib/libSystem.B.dylib"
        );
        assert_eq!(printf.offset, Some(0x210), "its own entry");
        assert!(
            s.rows.iter().any(|r| r.label == "Exports (1)"),
            "{:?}",
            s.rows
        );

        // Mach-O records no sizes, so the symbol owns everything after it.
        assert_eq!(s.symbols.len(), 1);
        assert_eq!(s.symbol_at(0x1_0000_0180), Some("_scmd_main"));
        assert_eq!(s.symbol_at(0x1_0000_0000), None, "below the only symbol");
        assert_eq!(s.offset_for_va(0x1_0000_0100), Some(0x100));
        targets_stay_inside(&s, bytes.len() as u64);
    }
}
