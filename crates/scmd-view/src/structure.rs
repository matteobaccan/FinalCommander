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

/// The decoded shape of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Structure {
    /// What we decided the file is.
    pub format: &'static str,
    /// One line of identity, for the pane's title.
    pub summary: String,
    pub rows: Vec<Row>,
    /// The file ended inside something we were reading.
    pub truncated: bool,
}

impl Structure {
    /// The rows `Enter` can act on, for the tests and for "jump to the next table".
    pub fn targets(&self) -> impl Iterator<Item = &Row> {
        self.rows.iter().filter(|r| r.is_target())
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

    if shnum > 0 && shentsize as usize >= 40 {
        rows.push(Row::group(format!("Sections ({shnum})")));
        let count = (shnum as usize).min(MAX_ENTRIES);
        let table = read_table(src, shoff, shentsize as usize, count, len);
        if table.is_empty() && shnum > 0 {
            truncated = true;
        }
        for i in 0..count {
            let from = i * shentsize as usize;
            if from + shentsize as usize > table.len() {
                truncated = true;
                break;
            }
            let mut s = Cur::new(&table[from..from + shentsize as usize]);
            s.big_endian = c.big_endian;
            let sh_name = s.u32();
            let sh_type = s.u32();
            let (sh_flags, sh_addr, sh_offset, sh_size) = if is64 {
                (s.u64(), s.u64(), s.u64(), s.u64())
            } else {
                (
                    s.u32() as u64,
                    s.u32() as u64,
                    s.u32() as u64,
                    s.u32() as u64,
                )
            };
            let name = str_at(&strtab, sh_name as usize);
            let name = if name.is_empty() {
                format!("[{i}]")
            } else {
                name
            };
            // SHT_NOBITS (8) occupies no file space, so it has no offset to go to.
            let offset = if sh_type == 8 {
                None
            } else {
                in_file(sh_offset, len)
            };
            rows.push(Row::entry(
                format!("  {name}"),
                format!(
                    "{sh_offset:08X}  {} {}  {}",
                    size_cell(sh_size),
                    elf_flags(sh_flags),
                    if sh_addr != 0 {
                        format!("@ {sh_addr:X}")
                    } else {
                        String::new()
                    }
                ),
                offset,
            ));
        }
        if shnum as usize > count {
            rows.push(Row::entry(
                "  …",
                format!("{} more sections not shown", shnum as usize - count),
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

    // The interpreter and the build id are the two things worth reading the bytes for: they say
    // how the file will be loaded and which build it came from.
    let mut notes = Vec::new();
    if let Some(interp) = elf_section_data(
        src,
        shoff,
        shentsize,
        shnum,
        is64,
        c.big_endian,
        &strtab,
        ".interp",
        len,
        256,
    ) {
        let text = String::from_utf8_lossy(&interp)
            .trim_end_matches('\0')
            .to_string();
        if !text.is_empty() {
            notes.push(Row::field("interpreter", String::new(), text));
        }
    }
    if let Some(note) = elf_section_data(
        src,
        shoff,
        shentsize,
        shnum,
        is64,
        c.big_endian,
        &strtab,
        ".note.gnu.build-id",
        len,
        64,
    ) {
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

    Structure {
        format: "ELF",
        summary,
        rows,
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

/// The bytes of a named section, up to `cap`.
#[allow(clippy::too_many_arguments)]
fn elf_section_data(
    src: &mut dyn Bytes,
    shoff: u64,
    shentsize: u16,
    shnum: u16,
    is64: bool,
    big_endian: bool,
    strtab: &[u8],
    want: &str,
    len: u64,
    cap: usize,
) -> Option<Vec<u8>> {
    if shentsize == 0 || shnum == 0 {
        return None;
    }
    let count = (shnum as usize).min(MAX_ENTRIES);
    let table = read_table(src, shoff, shentsize as usize, count, len);
    for i in 0..count {
        let from = i * shentsize as usize;
        if from + shentsize as usize > table.len() {
            break;
        }
        let mut s = Cur::new(&table[from..from + shentsize as usize]);
        s.big_endian = big_endian;
        let sh_name = s.u32();
        if str_at(strtab, sh_name as usize) != want {
            continue;
        }
        let (offset, size) = if is64 {
            s.seek(24);
            (s.u64(), s.u64())
        } else {
            s.seek(16);
            (s.u32() as u64, s.u32() as u64)
        };
        if offset >= len {
            return None;
        }
        let size = size.min(cap as u64).min(len - offset) as usize;
        return Some(src.at(offset, size));
    }
    None
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
    if opt_size >= 24 {
        n.seek(24 + 16);
        let entry = n.u32();
        let base_of_code = n.u32();
        let _base_of_data = n.u32();
        let image_base = if plus { n.u64() } else { n.u32() as u64 };
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
    }

    let mut truncated = n.short;
    let sec_at = lfanew + 24 + opt_size as u64;
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

    Structure {
        format: "PE",
        summary,
        rows,
        truncated,
    }
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
                    rows.push(Row::entry(
                        if cmd == 0x0D { "  id" } else { "  dylib" },
                        name,
                        None,
                    ));
                }
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

    Structure {
        format: "Mach-O",
        summary,
        rows,
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
        truncated,
    }
}

// ── helpers ─────────────────────────────────────────────────────────────────────────────────

/// An offset is only a target if it is really inside the file.
fn in_file(offset: u64, len: u64) -> Option<u64> {
    (offset < len).then_some(offset)
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
}
