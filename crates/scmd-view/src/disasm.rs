//! Looking inside compiled files: what the binary *is*, and what its code says.
//!
//! Two levels, and the difference matters:
//!
//! - **Disassembly** — machine code to mnemonics. Done here, in Rust, for x86 and x86-64
//!   (`iced-x86`), with the file's own structure read by `object`; and WebAssembly to its
//!   canonical text format (`wasmprinter`), which is as close to source as a `.wasm` has.
//! - **Decompilation** — machine code back to something C- or Java-like. That is a research
//!   project in its own right (Ghidra, RetDec, ILSpy, javap, uncompyle6), so SupremeCommander
//!   does not reimplement it: it *calls* those tools when they are installed and shows their
//!   output in the viewer. See `[[decompile]]` in `docs/spec/07-config-theming.md`.
//!
//! Nothing here circumvents any protection: it reads bytes the user can already read.

use std::fmt;

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, IntelFormatter};
use object::read::{Object, ObjectSection, ObjectSymbol};
use object::{Architecture, BinaryFormat, SectionKind};

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
        | [0xcf, 0xfa, 0xed, 0xfe, ..]
        | [0xca, 0xfe, 0xba, 0xbe, 0x00, ..] => Format::MachO,
        [0x00, b'a', b's', b'm', ..] => Format::Wasm,
        [0xca, 0xfe, 0xba, 0xbe, ..] => Format::JavaClass,
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

    /// We disassemble x86 ourselves; anything else needs `objdump` or a plugin.
    pub fn disassemblable(self) -> bool {
        matches!(self, Arch::X86 | Arch::X86_64)
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
    #[error("no disassembler for {0}; configure an external tool in scmd.toml")]
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
        });
    }
    Ok(out)
}

/// Where the instruction before `target` starts.
///
/// x86 instructions have no fixed length, so the only honest way back is to decode forward from
/// a little earlier and take the last boundary that lands on `target`. 32 bytes of lead-in is
/// enough for any x86 instruction (15 bytes is the architectural maximum).
pub fn previous_instruction(code_before: &[u8], target_offset: u64, arch: Arch) -> u64 {
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

/// The architecture this build of SupremeCommander is running on — the only sensible guess
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
        assert_eq!(probe(b"\xca\xfe\xba\xbe\x00\x00\x00\x34"), Format::MachO);
        assert_eq!(probe(b"\xca\xfe\xba\xbe\x34\x00"), Format::JavaClass);
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
            binary.arch.disassemblable() || binary.arch == Arch::Aarch64,
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

    #[test]
    fn an_unsupported_architecture_says_what_to_do() {
        let err = disassemble(&[0; 8], 0, 0, Arch::Aarch64, 4).unwrap_err();
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
    }
}
