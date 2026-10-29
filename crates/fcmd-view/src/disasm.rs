//! Looking inside compiled files: what the binary *is*, and what its code says.
//!
//! Two levels, and the difference matters:
//!
//! - **Disassembly** — machine code to mnemonics. Done here, in Rust, for x86 and x86-64
//!   (`iced-x86`) and for AArch64 (`yaxpeax-arm`), with the file's own structure read by
//!   `object`; and WebAssembly to its canonical text format (`wasmprinter`), which is as
//!   close to source as a `.wasm` has.
//! - **Decompilation** — machine code back to something C- or Java-like. That is a research
//!   project in its own right (Ghidra, RetDec, ILSpy, javap, uncompyle6), so FinalCommander
//!   does not reimplement it: it *calls* those tools when they are installed and shows their
//!   output in the viewer. See `[[decompile]]` in `docs/spec/07-config-theming.md`.
//!
//! Nothing here circumvents any protection: it reads bytes the user can already read.

use std::fmt;

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter, Mnemonic, OpKind};
use object::read::{Object, ObjectSection, ObjectSymbol};
use object::{Architecture, BinaryFormat, SectionKind};
use yaxpeax_arch::{Decoder as _, U8Reader};
use yaxpeax_arm::armv8::a64::{
    InstDecoder as A64Decoder, Opcode as A64Opcode, Operand as A64Operand,
};

/// How much of a file we are willing to parse structurally. Beyond this the viewer still
/// disassembles, it just sweeps raw bytes instead of following sections.
pub const MAX_PARSE: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Elf,
    Pe,
    MachO,
    Wasm,
    JavaClass,
    PythonBytecode,
    Archive,
    /// Not a container we know: still disassemblable as raw code.
    Raw,
}

impl Format {
    pub fn as_str(self) -> &'static str {
        match self {
            Format::Elf => "ELF",
            Format::Pe => "PE",
            Format::MachO => "Mach-O",
            Format::Wasm => "WebAssembly",
            Format::JavaClass => "Java class",
            Format::PythonBytecode => "Python bytecode",
            Format::Archive => "archive",
            Format::Raw => "raw",
        }
    }

    /// Can we disassemble it ourselves, or does it need an external tool?
    pub fn native_support(self) -> bool {
        matches!(
            self,
            Format::Elf | Format::Pe | Format::MachO | Format::Wasm | Format::Raw
        )
    }
}

/// Recognise a file from its first bytes, without parsing it.
pub fn probe(head: &[u8]) -> Format {
    match head {
        [0x7f, b'E', b'L', b'F', ..] => Format::Elf,
        [b'M', b'Z', ..] => Format::Pe,
        // Mach-O, 32- and 64-bit, both endiannesses, and the universal binary.
        [0xfe, 0xed, 0xfa, 0xce, ..]
        | [0xce, 0xfa, 0xed, 0xfe, ..]
        | [0xfe, 0xed, 0xfa, 0xcf, ..]
        | [0xcf, 0xfa, 0xed, 0xfe, ..] => Format::MachO,
        [0x00, b'a', b's', b'm', ..] => Format::Wasm,
        // `CAFEBABE` is both a Java class and a Mach-O universal binary. They are told apart
        // by what follows: a class file has a major version of at least 45 (Java 1.1) where a
        // fat Mach-O has a small architecture count.
        [0xca, 0xfe, 0xba, 0xbe, _, _, major_high, major_low, ..] => {
            let major = u16::from_be_bytes([*major_high, *major_low]);
            if (45..=200).contains(&major) {
                Format::JavaClass
            } else {
                Format::MachO
            }
        }
        [0xca, 0xfe, 0xba, 0xbe, ..] => Format::MachO,
        [b'!', b'<', b'a', b'r', b'c', b'h', b'>', ..] => Format::Archive,
        _ => {
            // `.pyc`: a 16-byte header whose magic ends in \r\n.
            if head.len() > 4 && head[2] == 0x0d && head[3] == 0x0a {
                Format::PythonBytecode
            } else {
                Format::Raw
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    Aarch64,
    Other,
}

impl Arch {
    pub fn as_str(self) -> &'static str {
        match self {
            Arch::X86 => "x86",
            Arch::X86_64 => "x86-64",
            Arch::Aarch64 => "aarch64",
            Arch::Other => "unknown",
        }
    }

    /// We disassemble x86 and AArch64 ourselves; anything else needs `objdump` or a plugin.
    pub fn disassemblable(self) -> bool {
        matches!(self, Arch::X86 | Arch::X86_64 | Arch::Aarch64)
    }

    /// What the assembler writes, which is a shorter list than what the disassembler reads:
    /// `F9` in the hex pane is x86 only (SPEC 05 §2.8). The two are asked separately so that
    /// reading AArch64 — the point of the decoder — cannot be mistaken for writing it, which
    /// would put x86 bytes into an ARM binary.
    pub fn assemblable(self) -> bool {
        matches!(self, Arch::X86 | Arch::X86_64)
    }

    /// How long an instruction is, where that is one number. AArch64 is four bytes, always,
    /// which is what makes stepping backwards through it arithmetic rather than a search.
    pub fn instruction_len(self) -> Option<usize> {
        match self {
            Arch::Aarch64 => Some(4),
            _ => None,
        }
    }

    pub fn bitness(self) -> u32 {
        match self {
            Arch::X86 => 32,
            _ => 64,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
    /// Offset in the file, when the section has bytes there.
    pub file_offset: Option<u64>,
    pub executable: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolInfo {
    pub name: String,
    pub address: u64,
    pub size: u64,
    pub is_function: bool,
}

/// What we learned about a compiled file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binary {
    pub format: Format,
    pub arch: Arch,
    pub bits: u8,
    pub entry: u64,
    pub sections: Vec<SectionInfo>,
    /// Function symbols, sorted by address — used to label the disassembly.
    pub functions: Vec<SymbolInfo>,
    /// True when the file was too big to parse and this is a best effort.
    pub truncated: bool,
}

impl Binary {
    /// The first executable section: where a disassembly should start.
    pub fn text_section(&self) -> Option<&SectionInfo> {
        self.sections
            .iter()
            .find(|s| s.executable && s.file_offset.is_some())
    }

    /// The name of the function containing `address` — the same question
    /// `structure::Structure::symbol_at` answers, asked the same way, so a caller can use
    /// whichever of the two it managed to read (SPEC 05 §1.9).
    pub fn symbol_at(&self, address: u64) -> Option<&str> {
        self.function_at(address).map(|f| f.name.as_str())
    }

    /// The function containing `address`, if any.
    pub fn function_at(&self, address: u64) -> Option<&SymbolInfo> {
        self.functions
            .iter()
            .rev()
            .find(|f| f.address <= address && (f.size == 0 || address < f.address + f.size))
    }

    /// Virtual address for a file offset, following the section it falls in.
    pub fn va_for_offset(&self, offset: u64) -> u64 {
        for s in &self.sections {
            if let Some(start) = s.file_offset {
                if offset >= start && offset < start + s.size {
                    return s.address + (offset - start);
                }
            }
        }
        offset
    }

    /// File offset for a virtual address, if it is mapped from the file.
    pub fn offset_for_va(&self, va: u64) -> Option<u64> {
        for s in &self.sections {
            if let Some(start) = s.file_offset {
                if va >= s.address && va < s.address + s.size {
                    return Some(start + (va - s.address));
                }
            }
        }
        None
    }

    /// Fill in the file offset of each line's branch target, so `Enter` on a `call` has
    /// somewhere to go.
    pub fn resolve_targets(&self, lines: &mut [Line]) {
        resolve_targets(lines, |va| self.offset_for_va(va));
    }
}

/// What kind of address an instruction's operand is: the pane follows a code target in the
/// disassembly and a data one in the hex view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetKind {
    Call,
    /// An unconditional `jmp`.
    Jump,
    /// A conditional branch, which may or may not be taken.
    Branch,
    /// A `lea` or a memory operand relative to the instruction pointer: the address of
    /// something, not of code to run.
    Data,
}

/// Where an instruction's operand points — SPEC 05 §1.9.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    /// The address the operand names, as the loaded program sees it.
    pub address: u64,
    /// Where that is in the file, when something could map it. `None` for an address outside
    /// this file: a `jmp` into a library, or a nonsense address in data decoded as code.
    pub offset: Option<u64>,
    pub kind: TargetKind,
}

/// Fill in the file offset of every branch target `map` can place. A caller with the structure
/// pane's mapping rather than a parsed [`Binary`] passes its own lookup.
pub fn resolve_targets(lines: &mut [Line], map: impl Fn(u64) -> Option<u64>) {
    for line in lines {
        if let Some(target) = line.target.as_mut() {
            target.offset = map(target.address);
        }
    }
}

/// The address an instruction's operand names, where it names one.
///
/// This asks `iced-x86` what the operands *are* rather than reading the formatted text: the
/// text is for people, and a branch target that has to survive a formatter option is not a
/// branch target.
fn branch_target(instruction: &Instruction) -> Option<Target> {
    for i in 0..instruction.op_count() {
        if matches!(
            instruction.op_kind(i),
            OpKind::NearBranch16 | OpKind::NearBranch32 | OpKind::NearBranch64
        ) {
            let kind = match instruction.mnemonic() {
                Mnemonic::Call => TargetKind::Call,
                Mnemonic::Jmp => TargetKind::Jump,
                // Everything else that takes a branch operand is conditional: `jcc`, `loop`,
                // `jrcxz`, `xbegin`.
                _ => TargetKind::Branch,
            };
            return Some(Target {
                address: instruction.near_branch_target(),
                offset: None,
                kind,
            });
        }
    }
    // A RIP-relative operand is an address computed from this instruction's own position: a
    // `lea` of a string, or the import slot a `call [rip+…]` reads its destination out of.
    // Either way the address names where the bytes are, not code to run, so it is data.
    if instruction.is_ip_rel_memory_operand() {
        return Some(Target {
            address: instruction.ip_rel_memory_address(),
            offset: None,
            kind: TargetKind::Data,
        });
    }
    None
}

#[derive(Debug, thiserror::Error)]
pub enum DisasmError {
    #[error("not a format we can parse: {0}")]
    Unsupported(&'static str),
    /// ELF and PE keep their section tables at the *end* of the file, so a prefix cannot be
    /// parsed: above the limit the viewer sweeps raw bytes instead. A debug build of a Rust
    /// program is routinely 100 MB, so this is the normal case, not an exotic one.
    #[error("file larger than the {limit} MiB parse limit: disassembling raw bytes")]
    TooLarge { limit: u64 },
    #[error("{0}")]
    Parse(String),
    #[error("no disassembler for {0}; configure an external tool in fcmd.toml")]
    NoDisassembler(&'static str),
}

/// Read a compiled file's structure. `bytes` may be a prefix of a very large file; then the
/// result is marked `truncated`.
pub fn analyze(bytes: &[u8], complete: bool) -> Result<Binary, DisasmError> {
    let format = probe(bytes);
    if format == Format::Wasm {
        return Ok(Binary {
            format,
            arch: Arch::Other,
            bits: 32,
            entry: 0,
            sections: Vec::new(),
            functions: Vec::new(),
            truncated: !complete,
        });
    }
    if !matches!(format, Format::Elf | Format::Pe | Format::MachO) {
        return Err(DisasmError::Unsupported(format.as_str()));
    }

    let file = object::read::File::parse(bytes).map_err(|e| DisasmError::Parse(e.to_string()))?;
    let arch = match file.architecture() {
        Architecture::I386 => Arch::X86,
        Architecture::X86_64 | Architecture::X86_64_X32 => Arch::X86_64,
        Architecture::Aarch64 => Arch::Aarch64,
        _ => Arch::Other,
    };
    let bits = if file.is_64() { 64 } else { 32 };

    let mut sections: Vec<SectionInfo> = file
        .sections()
        .map(|s| SectionInfo {
            name: s.name().unwrap_or("<unnamed>").to_string(),
            address: s.address(),
            size: s.size(),
            file_offset: s.file_range().map(|(start, _)| start),
            executable: s.kind() == SectionKind::Text,
        })
        .collect();
    sections.sort_by_key(|s| s.address);

    let mut functions: Vec<SymbolInfo> = file
        .symbols()
        .filter(|s| s.kind() == object::SymbolKind::Text)
        .filter_map(|s| {
            let name = s.name().ok()?;
            (!name.is_empty()).then(|| SymbolInfo {
                name: name.to_string(),
                address: s.address(),
                size: s.size(),
                is_function: true,
            })
        })
        .collect();
    functions.sort_by_key(|f| f.address);

    let _ = BinaryFormat::Elf; // keep the import honest across platforms
    Ok(Binary {
        format,
        arch,
        bits,
        entry: file.entry(),
        sections,
        functions,
        truncated: !complete,
    })
}

/// One disassembled instruction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Offset in the file.
    pub offset: u64,
    /// Virtual address, when we know the mapping.
    pub address: u64,
    pub bytes: Vec<u8>,
    pub text: String,
    /// `true` when the decoder could not make sense of the bytes.
    pub invalid: bool,
    /// Where this instruction's operand points, when it points anywhere — SPEC 05 §1.9. An
    /// `Option`, so everything that read a `Line` before this existed still does.
    pub target: Option<Target>,
}

impl Line {
    /// `call 0000001F80 <fcmd_main>`: the text with its target named, where something can
    /// name it. The viewer never invents a name, so a target nothing knows changes nothing.
    pub fn annotated<'a>(&self, name: impl Fn(u64) -> Option<&'a str>) -> String {
        match self.target.and_then(|t| name(t.address)) {
            Some(symbol) => format!("{} <{}>", self.text, symbol),
            None => self.text.clone(),
        }
    }
}

impl fmt::Display for Line {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hex: String = self
            .bytes
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect::<Vec<_>>()
            .join("");
        write!(f, "{:016X}  {:<20}  {}", self.address, hex, self.text)
    }
}

/// Disassemble `code`, which starts at file offset `offset` and virtual address `address`.
///
/// Stops after `limit` instructions. Invalid bytes become one `(bad)` line each, so a sweep
/// through data never loses sync for long.
pub fn disassemble(
    code: &[u8],
    offset: u64,
    address: u64,
    arch: Arch,
    limit: usize,
) -> Result<Vec<Line>, DisasmError> {
    if !arch.disassemblable() {
        return Err(DisasmError::NoDisassembler(arch.as_str()));
    }
    if arch == Arch::Aarch64 {
        return Ok(aarch64(code, offset, address, limit));
    }
    let mut decoder = Decoder::with_ip(arch.bitness(), code, address, DecoderOptions::NONE);
    let mut formatter = IntelFormatter::new();
    formatter.options_mut().set_first_operand_char_index(8);
    let mut instruction = Instruction::default();
    let mut out = Vec::with_capacity(limit.min(1024));

    while decoder.can_decode() && out.len() < limit {
        let position = decoder.position();
        decoder.decode_out(&mut instruction);

        if instruction.is_invalid() {
            // The decoder may swallow several bytes in one invalid instruction, which would
            // hide real code right after it. Emit one `(bad)` byte and resync one byte on, the
            // way a linear-sweep disassembler is expected to behave.
            out.push(Line {
                offset: offset + position as u64,
                address: address + position as u64,
                bytes: vec![code[position]],
                text: "(bad)".to_string(),
                invalid: true,
                target: None,
            });
            let next = position + 1;
            if next >= code.len() {
                break;
            }
            decoder.set_position(next).ok();
            decoder.set_ip(address + next as u64);
            continue;
        }

        let len = instruction.len().max(1);
        let bytes = code[position..(position + len).min(code.len())].to_vec();
        let mut text = String::new();
        formatter.format(&instruction, &mut text);
        out.push(Line {
            offset: offset + position as u64,
            address: address + position as u64,
            bytes,
            text,
            invalid: false,
            target: branch_target(&instruction),
        });
    }
    Ok(out)
}

/// AArch64: one instruction is one 32-bit word, and that is the whole decoder loop.
///
/// A word nothing decodes is one `(bad)` line of four bytes rather than one of a byte: there is
/// no sync to lose in a fixed-width instruction set, so stepping a byte at a time — which is
/// right for x86 — would turn one unallocated encoding into four lines of noise.
///
/// The text is written the way the x86 side writes it: the mnemonic in a column eight wide, and
/// a branch shown as the address it arrives at rather than as the displacement in the encoding.
/// `yaxpeax` prints `b $+0x14`, which is what the bytes say; `b 401014h` is what the reader
/// wants, and it is what `Enter` then follows.
fn aarch64(code: &[u8], offset: u64, address: u64, limit: usize) -> Vec<Line> {
    let decoder = A64Decoder::default();
    let mut out = Vec::with_capacity(limit.min(1024));
    let mut position = 0usize;
    while position < code.len() && out.len() < limit {
        let at = position;
        // The tail of a section that is not a whole word is not an instruction, and saying so
        // is better than decoding whatever happens to follow it in memory.
        let Some(word) = code.get(at..at + 4) else {
            out.push(Line {
                offset: offset + at as u64,
                address: address + at as u64,
                bytes: code[at..].to_vec(),
                text: "(bad)".to_string(),
                invalid: true,
                target: None,
            });
            break;
        };
        position += 4;
        let mut reader = U8Reader::new(word);
        let (text, target, invalid) = match decoder.decode(&mut reader) {
            Ok(instruction) if instruction.opcode != A64Opcode::Invalid => {
                let here = address + at as u64;
                (
                    a64_text(&instruction, here),
                    a64_target(&instruction, here),
                    false,
                )
            }
            _ => ("(bad)".to_string(), None, true),
        };
        out.push(Line {
            offset: offset + at as u64,
            address: address + at as u64,
            bytes: word.to_vec(),
            text,
            invalid,
            target,
        });
    }
    out
}

/// One AArch64 instruction as a line of text, with any branch displacement turned into the
/// address it names.
fn a64_text(instruction: &yaxpeax_arm::armv8::a64::Instruction, here: u64) -> String {
    let printed = instruction.to_string();
    let (mnemonic, operands) = match printed.split_once(' ') {
        Some((m, rest)) => (m, rest.trim().to_string()),
        None => (printed.as_str(), String::new()),
    };
    let operands = match a64_offset(instruction) {
        // The displacement as `yaxpeax` prints it, and the address it adds up to. Replacing
        // the one with the other keeps everything else the decoder wrote.
        Some(delta) => {
            let was = if delta < 0 {
                format!("$-{:#x}", delta.wrapping_neg())
            } else {
                format!("$+{delta:#x}")
            };
            let now = format!("{:X}h", here.wrapping_add(delta as u64));
            operands.replace(&was, &now)
        }
        None => operands,
    };
    if operands.is_empty() {
        mnemonic.to_string()
    } else {
        format!("{mnemonic:<8}{operands}")
    }
}

/// The displacement of the one operand that carries one, if the instruction has it.
fn a64_offset(instruction: &yaxpeax_arm::armv8::a64::Instruction) -> Option<i64> {
    instruction
        .operands
        .iter()
        .find_map(|operand| match operand {
            A64Operand::PCOffset(delta) => Some(*delta),
            _ => None,
        })
}

/// Where an AArch64 instruction points — the same question as `branch_target`, asked of the
/// decoder's own operands rather than of its text.
fn a64_target(instruction: &yaxpeax_arm::armv8::a64::Instruction, here: u64) -> Option<Target> {
    let delta = a64_offset(instruction)?;
    let kind = match instruction.opcode {
        A64Opcode::BL | A64Opcode::BLR => TargetKind::Call,
        A64Opcode::B | A64Opcode::BR => TargetKind::Jump,
        // `adr` and `adrp` compute the address of something — a string, a page of data —
        // rather than of code to run, which is the same distinction x86 makes for `lea`.
        A64Opcode::ADR | A64Opcode::ADRP => TargetKind::Data,
        // What is left taking a displacement is conditional: `b.cc`, `cbz`, `cbnz`, `tbz`,
        // `tbnz`, and the load-literal forms.
        _ => TargetKind::Branch,
    };
    let kind = match (kind, instruction.opcode) {
        // A literal load reads the bytes at the address, so it is data wherever it appears.
        (TargetKind::Branch, A64Opcode::LDR | A64Opcode::LDRSW | A64Opcode::PRFM) => {
            TargetKind::Data
        }
        (kind, _) => kind,
    };
    Some(Target {
        address: here.wrapping_add(delta as u64),
        offset: None,
        kind,
    })
}

/// Where the instruction before `target` starts.
///
/// x86 instructions have no fixed length, so the only honest way back is to decode forward from
/// a little earlier and take the last boundary that lands on `target`. 32 bytes of lead-in is
/// enough for any x86 instruction (15 bytes is the architectural maximum). AArch64 needs none
/// of that: every instruction is four bytes, so the one before is four bytes back.
pub fn previous_instruction(code_before: &[u8], target_offset: u64, arch: Arch) -> u64 {
    if let Some(len) = arch.instruction_len() {
        return target_offset.saturating_sub(len as u64);
    }
    if code_before.is_empty() || !arch.disassemblable() {
        return target_offset.saturating_sub(1);
    }
    let start_offset = target_offset - code_before.len() as u64;
    let lines = disassemble(code_before, start_offset, start_offset, arch, 64).unwrap_or_default();
    lines
        .iter()
        .rev()
        .find(|l| l.offset + l.bytes.len() as u64 <= target_offset)
        .map(|l| l.offset)
        .unwrap_or_else(|| target_offset.saturating_sub(1))
}

/// WebAssembly's canonical text format: as close to source as a `.wasm` file gets.
pub fn wasm_text(bytes: &[u8]) -> Result<String, DisasmError> {
    wasmprinter::print_bytes(bytes).map_err(|e| DisasmError::Parse(e.to_string()))
}

/// The architecture this build of FinalCommander is running on — the only sensible guess
/// when a file is too large to parse but the user still wants to see instructions.
pub fn host_arch() -> Arch {
    if cfg!(target_arch = "x86_64") {
        Arch::X86_64
    } else if cfg!(target_arch = "x86") {
        Arch::X86
    } else if cfg!(target_arch = "aarch64") {
        Arch::Aarch64
    } else {
        Arch::Other
    }
}

/// A short, honest summary line for the viewer's header.
pub fn summary(binary: &Binary) -> String {
    let mut text = format!("{} {}", binary.format.as_str(), binary.arch.as_str());
    if binary.bits != 0 {
        text.push_str(&format!(" {}-bit", binary.bits));
    }
    if binary.entry != 0 {
        text.push_str(&format!("  entry {:#x}", binary.entry));
    }
    if !binary.sections.is_empty() {
        text.push_str(&format!("  {} sections", binary.sections.len()));
    }
    if !binary.functions.is_empty() {
        text.push_str(&format!("  {} symbols", binary.functions.len()));
    }
    if binary.truncated {
        text.push_str("  (partial: file larger than the parse limit)");
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The test binary itself is a real native executable: the best possible fixture.
    fn own_binary() -> Vec<u8> {
        let path = std::env::current_exe().expect("the test binary has a path");
        std::fs::read(path).expect("and it is readable")
    }

    #[test]
    fn formats_are_recognised_from_their_magic() {
        assert_eq!(probe(b"\x7fELF\x02\x01"), Format::Elf);
        assert_eq!(probe(b"MZ\x90\x00"), Format::Pe);
        assert_eq!(probe(b"\x00asm\x01\x00\x00\x00"), Format::Wasm);
        // CAFEBABE is ambiguous: a class file's major version tells it from a fat Mach-O.
        assert_eq!(
            probe(b"\xca\xfe\xba\xbe\x00\x00\x00\x34"),
            Format::JavaClass,
            "minor 0, major 52 is a Java 8 class"
        );
        assert_eq!(
            probe(b"\xca\xfe\xba\xbe\x00\x00\x00\x02"),
            Format::MachO,
            "two architectures is a universal binary"
        );
        assert_eq!(probe(b"!<arch>\n"), Format::Archive);
        assert_eq!(probe(b"plain text"), Format::Raw);
        assert_eq!(
            probe(&[0x6f, 0x0d, 0x0d, 0x0a, 0, 0]),
            Format::PythonBytecode
        );
    }

    #[test]
    fn our_own_executable_is_analysed() {
        let bytes = own_binary();
        let binary = analyze(&bytes, true).expect("the test binary parses");
        assert!(binary.format.native_support());
        assert!(
            binary.arch.disassemblable(),
            "unexpected architecture {}",
            binary.arch.as_str()
        );
        assert!(!binary.sections.is_empty());
        let text = binary.text_section().expect("there is a code section");
        assert!(text.executable && text.size > 0);
        assert!(summary(&binary).contains(binary.format.as_str()));
        assert!(!binary.truncated);
    }

    #[test]
    fn the_code_section_disassembles_into_instructions() {
        let bytes = own_binary();
        let binary = analyze(&bytes, true).unwrap();
        if !binary.arch.disassemblable() {
            return; // on aarch64 CI this is objdump's job, see DisasmError::NoDisassembler
        }
        let text = binary.text_section().unwrap();
        let start = text.file_offset.unwrap() as usize;
        let code = &bytes[start..(start + 4096).min(bytes.len())];
        let lines = disassemble(code, start as u64, text.address, binary.arch, 50).unwrap();
        assert_eq!(lines.len(), 50);
        let bad = lines.iter().filter(|l| l.invalid).count();
        assert!(bad < 10, "too many invalid instructions: {bad}/50");
        assert!(lines[0].address >= text.address);
        assert!(lines.iter().any(|l| !l.text.is_empty()));
    }

    #[test]
    fn known_x86_bytes_decode_to_the_expected_mnemonics() {
        // push rbp; mov rbp,rsp; xor eax,eax; ret
        let code = [0x55, 0x48, 0x89, 0xE5, 0x31, 0xC0, 0xC3];
        let lines = disassemble(&code, 0x1000, 0x401000, Arch::X86_64, 10).unwrap();
        let text: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(text[0].starts_with("push"), "{text:?}");
        assert!(text[1].starts_with("mov"), "{text:?}");
        assert!(text[2].starts_with("xor"), "{text:?}");
        assert!(text[3].starts_with("ret"), "{text:?}");
        assert_eq!(lines[0].offset, 0x1000);
        assert_eq!(lines[1].address, 0x401001);
        assert_eq!(lines[1].bytes, vec![0x48, 0x89, 0xE5]);
        assert!(lines.iter().all(|l| !l.invalid));
    }

    #[test]
    fn thirty_two_bit_code_decodes_differently() {
        let code = [0x55, 0x89, 0xE5, 0xC3];
        let lines = disassemble(&code, 0, 0, Arch::X86, 10).unwrap();
        assert!(lines[1].text.contains("ebp"), "{:?}", lines[1].text);
        let lines64 = disassemble(&code, 0, 0, Arch::X86_64, 10).unwrap();
        assert!(lines64[1].text.contains("ebp") || lines64[1].text.contains("rbp"));
    }

    #[test]
    fn invalid_bytes_become_bad_lines_instead_of_stopping_the_sweep() {
        let code = [0xff, 0xff, 0xff, 0x90, 0xc3];
        let lines = disassemble(&code, 0, 0, Arch::X86_64, 10).unwrap();
        assert!(lines.iter().any(|l| l.invalid), "{lines:?}");
        assert!(
            lines.iter().any(|l| l.text.starts_with("nop")),
            "the sweep did not recover: {lines:?}"
        );
    }

    #[test]
    fn stepping_backwards_lands_on_an_instruction_boundary() {
        // nop nop nop; mov rbp,rsp  → the instruction before offset 3 starts at 2
        let code = [0x90, 0x90, 0x90, 0x48, 0x89, 0xE5];
        let before = &code[..3];
        assert_eq!(previous_instruction(before, 3, Arch::X86_64), 2);
        assert_eq!(previous_instruction(&code[..6], 6, Arch::X86_64), 3);
        assert_eq!(previous_instruction(&[], 10, Arch::X86_64), 9);
    }

    /// A hand-assembled AArch64 prologue, decoded. The words are little-endian in the file
    /// and the bytes below are in file order, which is how anybody checking this against a
    /// hex dump will read them.
    #[test]
    fn aarch64_words_decode_to_the_instructions_they_are() {
        // stp x29, x30, [sp, #-16]!   mov x29, sp   bl $+8   ldp x29, x30, [sp], #16   ret
        let code: Vec<u8> = [
            0xA9BF7BFDu32,
            0x910003FD,
            0x94000002,
            0xA8C17BFD,
            0xD65F03C0,
        ]
        .iter()
        .flat_map(|word| word.to_le_bytes())
        .collect();
        let lines = disassemble(&code, 0, 0x4000, Arch::Aarch64, 16).expect("aarch64 decodes");
        assert_eq!(lines.len(), 5);
        // Every instruction is four bytes, and the addresses step by four.
        for (i, line) in lines.iter().enumerate() {
            assert_eq!(line.bytes.len(), 4, "{line}");
            assert_eq!(line.address, 0x4000 + 4 * i as u64);
            assert!(!line.invalid, "{line}");
        }
        let text: Vec<&str> = lines.iter().map(|l| l.text.as_str()).collect();
        assert!(text[0].starts_with("stp"), "{:?}", text[0]);
        assert!(text[1].starts_with("mov"), "{:?}", text[1]);
        assert!(text[3].starts_with("ldp"), "{:?}", text[3]);
        assert_eq!(text[4], "ret");
        // The mnemonic sits in a column eight wide, as it does for x86.
        assert!(text[1].starts_with("mov     "), "{:?}", text[1]);
        // A branch is shown as where it goes, not as the displacement in the encoding, and
        // the target is the one `Enter` follows.
        assert!(text[2].contains("4010h"), "{:?}", text[2]);
        assert!(!text[2].contains('$'), "{:?}", text[2]);
        let target = lines[2].target.expect("a call carries its target");
        assert_eq!(target.address, 0x4010);
        assert_eq!(target.kind, TargetKind::Call);
    }

    #[test]
    fn aarch64_branches_name_their_kind_and_data_is_not_code() {
        let at = |word: u32| {
            let bytes = word.to_le_bytes();
            disassemble(&bytes, 0, 0x1000, Arch::Aarch64, 1)
                .expect("aarch64 decodes")
                .remove(0)
        };
        // b $+0x20, b.eq $+0x10, cbz x0 $+8, adrp x0, $+0x1000
        assert_eq!(at(0x14000008).target.unwrap().kind, TargetKind::Jump);
        assert_eq!(at(0x54000080).target.unwrap().kind, TargetKind::Branch);
        assert_eq!(at(0xB4000040).target.unwrap().kind, TargetKind::Branch);
        assert_eq!(at(0x90000000).target.unwrap().kind, TargetKind::Data);
        // An unconditional branch backwards, which is the one a loop ends with.
        let back = at(0x17FFFFFF);
        assert_eq!(back.target.unwrap().address, 0x0FFC);
        assert!(back.text.contains("FFCh"), "{}", back.text);
    }

    /// A word nothing decodes is one line of four bytes: there is no sync to lose in a
    /// fixed-width instruction set, so a byte-at-a-time resync would be four lines of noise.
    #[test]
    fn an_aarch64_word_that_decodes_to_nothing_is_one_bad_line() {
        // `00 00 80 00` is unallocated; `FD 7B BF A9` right after it is the `stp` above,
        // and it still decodes, which is the part that matters.
        let code = [0x00, 0x00, 0x80, 0x00, 0xFD, 0x7B, 0xBF, 0xA9];
        let lines = disassemble(&code, 0, 0, Arch::Aarch64, 8).expect("aarch64 decodes");
        assert_eq!(lines.len(), 2);
        assert!(lines[0].invalid);
        assert_eq!(lines[0].bytes.len(), 4);
        assert_eq!(lines[0].text, "(bad)");
        assert!(!lines[1].invalid, "{}", lines[1]);
        // And the tail that is not a whole word is not an instruction either.
        let short = disassemble(&[0xFD, 0x7B], 0, 0, Arch::Aarch64, 8).expect("decodes");
        assert_eq!(short.len(), 1);
        assert!(short[0].invalid);
        assert_eq!(short[0].bytes.len(), 2);
    }

    /// Stepping backwards is arithmetic here, not a search: four bytes, always.
    #[test]
    fn stepping_back_through_aarch64_is_four_bytes() {
        assert_eq!(previous_instruction(&[], 0x40, Arch::Aarch64), 0x3C);
        assert_eq!(previous_instruction(&[0; 32], 0x40, Arch::Aarch64), 0x3C);
        assert_eq!(previous_instruction(&[], 2, Arch::Aarch64), 0);
    }

    /// Reading AArch64 and writing it are different questions, and the assembler answers only
    /// the x86 one: an ARM binary must not get x86 bytes written into it by a dialog that
    /// assumed too much.
    #[test]
    fn aarch64_is_read_but_not_assembled() {
        assert!(Arch::Aarch64.disassemblable());
        assert!(!Arch::Aarch64.assemblable());
        assert!(Arch::X86_64.assemblable());
        assert!(Arch::X86.assemblable());
        assert!(!Arch::Other.disassemblable());
    }

    #[test]
    fn an_unsupported_architecture_says_what_to_do() {
        // AArch64 is ours now; what is left is everything else, and it names the way out.
        let err = disassemble(&[0; 8], 0, 0, Arch::Other, 4).unwrap_err();
        assert!(err.to_string().contains("external tool"), "{err}");
    }

    #[test]
    fn a_wasm_module_prints_as_text() {
        // (module (func (export "answer") (result i32) i32.const 42))
        let wasm: &[u8] = &[
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, // header
            0x01, 0x05, 0x01, 0x60, 0x00, 0x01, 0x7f, // type: () -> i32
            0x03, 0x02, 0x01, 0x00, // function section
            0x07, 0x0a, 0x01, 0x06, b'a', b'n', b's', b'w', b'e', b'r', 0x00, 0x00, // export
            0x0a, 0x06, 0x01, 0x04, 0x00, 0x41, 0x2a, 0x0b, // code: i32.const 42
        ];
        assert_eq!(probe(wasm), Format::Wasm);
        let text = wasm_text(wasm).expect("the module prints");
        assert!(text.contains("(module"), "{text}");
        assert!(text.contains("i32.const 42"), "{text}");
        assert!(text.contains("answer"), "{text}");

        let binary = analyze(wasm, true).unwrap();
        assert_eq!(binary.format, Format::Wasm);
    }

    #[test]
    fn text_files_and_rubbish_are_refused_politely() {
        let err = analyze(b"#!/bin/sh\necho hello\n", true).unwrap_err();
        assert!(matches!(err, DisasmError::Unsupported(_)), "{err}");
        let err = analyze(b"\x7fELF but truncated", true).unwrap_err();
        assert!(matches!(err, DisasmError::Parse(_)), "{err}");
    }

    #[test]
    fn addresses_and_offsets_translate_both_ways() {
        let binary = Binary {
            format: Format::Elf,
            arch: Arch::X86_64,
            bits: 64,
            entry: 0x1040,
            sections: vec![SectionInfo {
                name: ".text".into(),
                address: 0x1000,
                size: 0x200,
                file_offset: Some(0x400),
                executable: true,
            }],
            functions: vec![
                SymbolInfo {
                    name: "main".into(),
                    address: 0x1040,
                    size: 0x20,
                    is_function: true,
                },
                SymbolInfo {
                    name: "helper".into(),
                    address: 0x1100,
                    size: 0,
                    is_function: true,
                },
            ],
            truncated: false,
        };
        assert_eq!(binary.va_for_offset(0x410), 0x1010);
        assert_eq!(binary.offset_for_va(0x1010), Some(0x410));
        assert_eq!(binary.offset_for_va(0x9999), None);
        assert_eq!(binary.function_at(0x1045).unwrap().name, "main");
        assert!(binary.function_at(0x1070).is_none(), "outside main's size");
        assert_eq!(binary.function_at(0x2000).unwrap().name, "helper");
        assert_eq!(binary.symbol_at(0x1045), Some("main"));
        assert_eq!(binary.symbol_at(0x1070), None);
    }

    // ── following a branch (SPEC 05 §1.9) ───────────────────────────────────────────────────

    /// `call +0xB`, `jmp -2`, `je +5`, `lea rax,[rip+0x10]`, `ret` — one of each kind of
    /// operand §1.9 names, and one instruction with no address in it at all.
    fn branching_code() -> [u8; 16] {
        [
            0xE8, 0x0B, 0x00, 0x00, 0x00, // call 401010h
            0xEB, 0xFE, // jmp 401005h
            0x74, 0x05, // je 40100Eh
            0x48, 0x8D, 0x05, 0x10, 0x00, 0x00, 0x00, // lea rax,[rip+10h]
        ]
    }

    #[test]
    fn a_call_carries_the_address_it_calls() {
        let lines = disassemble(&branching_code(), 0x200, 0x401000, Arch::X86_64, 10).unwrap();
        let target = lines[0].target.expect("a call has a target");
        assert_eq!(target.kind, TargetKind::Call);
        assert_eq!(target.address, 0x401010, "{}", lines[0].text);
        assert_eq!(target.offset, None, "nothing has mapped it yet");
    }

    #[test]
    fn a_conditional_branch_and_a_jump_are_both_targets() {
        let lines = disassemble(&branching_code(), 0, 0x401000, Arch::X86_64, 10).unwrap();
        let jmp = lines[1].target.expect("jmp");
        assert_eq!(jmp.kind, TargetKind::Jump);
        assert_eq!(jmp.address, 0x401005, "a backwards jump: {}", lines[1].text);
        let jcc = lines[2].target.expect("je");
        assert_eq!(jcc.kind, TargetKind::Branch, "it may not be taken");
        assert_eq!(jcc.address, 0x40100E);
    }

    #[test]
    fn a_rip_relative_lea_points_at_data() {
        let code = [0x48, 0x8D, 0x05, 0x10, 0x00, 0x00, 0x00, 0xC3];
        let lines = disassemble(&code, 0, 0x401000, Arch::X86_64, 10).unwrap();
        let lea = lines[0].target.expect("a lea of an address has one");
        assert_eq!(lea.kind, TargetKind::Data, "it is an address, not code");
        // RIP is the *next* instruction: 0x401007 + 0x10.
        assert_eq!(lea.address, 0x401017, "{}", lines[0].text);
        assert_eq!(lines[1].target, None, "ret goes nowhere nameable");
    }

    #[test]
    fn an_instruction_without_an_address_operand_has_no_target() {
        // push rbp; mov rbp,rsp; xor eax,eax; ret
        let code = [0x55, 0x48, 0x89, 0xE5, 0x31, 0xC0, 0xC3];
        let lines = disassemble(&code, 0, 0x401000, Arch::X86_64, 10).unwrap();
        assert!(lines.iter().all(|l| l.target.is_none()), "{lines:?}");
        // And invalid bytes certainly do not acquire one.
        let bad = disassemble(&[0xff, 0xff, 0xff], 0, 0, Arch::X86_64, 4).unwrap();
        assert!(bad.iter().all(|l| l.target.is_none()));
    }

    #[test]
    fn a_branch_target_gets_a_file_offset_from_the_binary() {
        let binary = Binary {
            format: Format::Elf,
            arch: Arch::X86_64,
            bits: 64,
            entry: 0x401000,
            sections: vec![SectionInfo {
                name: ".text".into(),
                address: 0x401000,
                size: 0x200,
                file_offset: Some(0x200),
                executable: true,
            }],
            functions: vec![SymbolInfo {
                name: "fcmd_main".into(),
                address: 0x401010,
                size: 0x10,
                is_function: true,
            }],
            truncated: false,
        };
        let mut lines = disassemble(&branching_code(), 0x200, 0x401000, Arch::X86_64, 10).unwrap();
        binary.resolve_targets(&mut lines);
        assert_eq!(lines[0].target.unwrap().offset, Some(0x210));
        // The lea's address is data in the same section, so it has an offset too.
        assert_eq!(lines[3].target.unwrap().offset, Some(0x220));
        // An address no section maps gets no offset invented for it.
        let mut away = disassemble(
            &[0xE8, 0x00, 0x00, 0x10, 0x00],
            0x200,
            0x401000,
            Arch::X86_64,
            1,
        )
        .unwrap();
        binary.resolve_targets(&mut away);
        assert_eq!(away[0].target.unwrap().address, 0x501005);
        assert_eq!(away[0].target.unwrap().offset, None);

        // And with a symbol table, the line can say whose code that is.
        assert_eq!(
            lines[0].annotated(|va| binary.symbol_at(va)),
            format!("{} <fcmd_main>", lines[0].text)
        );
        assert_eq!(lines[1].annotated(|va| binary.symbol_at(va)), lines[1].text);
    }
}
