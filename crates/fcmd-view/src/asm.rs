//! An assembler for the instructions a patcher actually types (SPEC 05 §2.8).
//!
//! The point of `F9` in the hex editor is not to compile a program. It is to turn a `jz` into
//! a `jnz`, to nop out a call, to make a function return early, to put a constant in a
//! register — each one instruction at a time, in place, with the bytes shown before anything
//! is written. That is a small language, and this assembles it: registers and immediates,
//! with a branch target written as the address one wants to end up at.
//!
//! Memory operands are here too, because half of what a patcher reads is `mov eax, [rbx+8]`
//! and retyping it is the whole job: `[base]`, `[base+disp]`, `[base+index*scale+disp]`,
//! `[disp]` on its own, and `[rip+disp]` with the displacement as the disassembler prints it
//! — counted from the end of the instruction, so what one reads is what one types back. A
//! size word says how wide an address is read when no register says it: `mov dword [rbx], 1`,
//! or `byte ptr` for somebody who writes masm.
//!
//! What it still does not do: segment prefixes (`fs:[0x30]`), a 32-bit address in 64-bit code
//! (the `0x67` prefix), far pointers, and the floating-point and vector registers. Each is
//! refused by name rather than guessed at. The hex pane is still there for bytes nobody can
//! spell.
//!
//! Everything that comes out of here is decoded again by the disassembler before it is
//! returned, so the dialog shows the instruction the processor will see rather than the one
//! the user meant, and a bug in this encoder shows up as a refusal rather than as a corrupted
//! binary.

use crate::disasm::{self, Arch};
use fcmd_core::calc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assembled {
    pub bytes: Vec<u8>,
    /// How the disassembler reads the bytes we just made: the proof, for the dialog.
    pub text: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum AsmError {
    #[error("nothing to assemble")]
    Empty,
    #[error("{0}: not an instruction this assembler knows")]
    Unknown(String),
    #[error("{0}: not a register or a number")]
    BadOperand(String),
    #[error("{0}")]
    Unsupported(String),
    #[error("{0}")]
    OutOfRange(String),
    #[error("{0} cannot be assembled here")]
    Arch(&'static str),
}

type Result<T> = std::result::Result<T, AsmError>;

/// A general-purpose register: its number and how wide it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Reg {
    num: u8,
    size: u8,
    /// `ah`, `ch`, `dh`, `bh`: the high byte of the first four, which cannot coexist with a
    /// REX prefix and so cannot be mixed with `r8`–`r15`.
    high_byte: bool,
}

const NAMES64: [&str; 16] = [
    "rax", "rcx", "rdx", "rbx", "rsp", "rbp", "rsi", "rdi", "r8", "r9", "r10", "r11", "r12", "r13",
    "r14", "r15",
];
const NAMES32: [&str; 16] = [
    "eax", "ecx", "edx", "ebx", "esp", "ebp", "esi", "edi", "r8d", "r9d", "r10d", "r11d", "r12d",
    "r13d", "r14d", "r15d",
];
const NAMES16: [&str; 16] = [
    "ax", "cx", "dx", "bx", "sp", "bp", "si", "di", "r8w", "r9w", "r10w", "r11w", "r12w", "r13w",
    "r14w", "r15w",
];
const NAMES8: [&str; 16] = [
    "al", "cl", "dl", "bl", "spl", "bpl", "sil", "dil", "r8b", "r9b", "r10b", "r11b", "r12b",
    "r13b", "r14b", "r15b",
];
const HIGH8: [&str; 4] = ["ah", "ch", "dh", "bh"];

fn register(name: &str) -> Option<Reg> {
    let name = name.trim();
    for (size, table) in [(64, NAMES64), (32, NAMES32), (16, NAMES16), (8, NAMES8)] {
        if let Some(num) = table.iter().position(|n| *n == name) {
            return Some(Reg {
                num: num as u8,
                size,
                high_byte: false,
            });
        }
    }
    HIGH8.iter().position(|n| *n == name).map(|num| Reg {
        num: num as u8 + 4,
        size: 8,
        high_byte: true,
    })
}

/// One operand, as written.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Operand {
    Reg(Reg),
    Imm(i64),
    Mem(Mem),
}

/// An address: `[base + index*scale + disp]`, or the two degenerate forms — a bare
/// displacement, and `[rip+disp]`.
///
/// `size` is how wide the thing at that address is, and it is `None` until something says:
/// the register on the other side of the instruction, or a size word in front of the
/// brackets. An instruction where neither says is refused rather than guessed at — `mov
/// [rbx], 1` could write one byte or eight, and the difference is somebody's binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mem {
    base: Option<Reg>,
    index: Option<Reg>,
    scale: u8,
    disp: i64,
    /// `[rip+disp]`: no base, no index, and the displacement is relative to the end of the
    /// instruction. 64-bit code only.
    rip: bool,
    size: Option<u8>,
}

impl Mem {
    /// How wide the operand is, given what the other side of the instruction says.
    fn width(&self, other: Option<Reg>) -> Result<u8> {
        match (self.size, other) {
            (Some(size), Some(reg)) if size != reg.size => Err(AsmError::Unsupported(
                "the size word and the register disagree about how wide this is".to_string(),
            )),
            (Some(size), _) => Ok(size),
            (None, Some(reg)) => Ok(reg.size),
            (None, None) => Err(AsmError::Unsupported(
                "say how wide: byte, word, dword or qword in front of the brackets".to_string(),
            )),
        }
    }
}

fn operand(text: &str) -> Result<Operand> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AsmError::BadOperand(text.to_string()));
    }
    // A size word, with or without masm's `ptr`: it belongs to the brackets that follow it.
    let (size, rest) = size_word(text);
    let rest = rest.trim();
    if rest.starts_with('[') {
        return Ok(Operand::Mem(memory(rest, size)?));
    }
    if size.is_some() {
        // `mov dword eax, 1`: the register already says how wide it is, so the word is noise
        // and the register is the operand.
        return operand(rest);
    }
    if text.contains('[') || text.contains(']') {
        return Err(AsmError::BadOperand(text.to_string()));
    }
    if let Some(reg) = register(text) {
        return Ok(Operand::Reg(reg));
    }
    // The calculator parses every base the rest of the program speaks, and the arithmetic
    // with it, so `0x401000`, `401000h` and `0x1000+8` all work as a number.
    match calc::evaluate(text) {
        Ok(answer) => Ok(Operand::Imm(answer.0)),
        Err(_) => Err(AsmError::BadOperand(text.to_string())),
    }
}

/// `byte`, `word`, `dword`, `qword`, each with an optional `ptr` after it.
fn size_word(text: &str) -> (Option<u8>, &str) {
    let (first, rest) = match text.split_once(char::is_whitespace) {
        Some((a, b)) => (a, b),
        None => (text, ""),
    };
    let size = match first.trim() {
        "byte" => 8,
        "word" => 16,
        "dword" => 32,
        "qword" => 64,
        _ => return (None, text),
    };
    let rest = rest.trim();
    let rest = rest.strip_prefix("ptr").map(str::trim).unwrap_or(rest);
    (Some(size), rest)
}

/// `[rbx]`, `[rbx+8]`, `[rbx+rcx*4-0x10]`, `[0x401000]`, `[rip+0x2e35]`.
///
/// The pieces are added up in any order, which is what one does when retyping a line read off
/// a disassembly; everything that is not a register is a number, and the calculator reads it,
/// so `[rbx+8*4]` is an address eight times four past `rbx` rather than a scaled index nobody
/// asked for — a scale is written on a register (`rcx*4`) and only there.
fn memory(text: &str, size: Option<u8>) -> Result<Mem> {
    let inside = text
        .trim()
        .strip_prefix('[')
        .and_then(|t| t.strip_suffix(']'))
        .ok_or_else(|| AsmError::BadOperand(text.trim().to_string()))?;
    let inside = inside.trim();
    if inside.is_empty() {
        return Err(AsmError::BadOperand(text.trim().to_string()));
    }
    if inside.contains(':') {
        return Err(AsmError::Unsupported(format!(
            "{}: a segment prefix is not assembled here",
            inside.trim()
        )));
    }
    let mut mem = Mem {
        base: None,
        index: None,
        scale: 1,
        disp: 0,
        rip: false,
        size,
    };
    // Split keeping each sign, so `[rbx-0x10]` is `rbx` and `-0x10`.
    let mut pieces: Vec<(bool, String)> = Vec::new();
    let mut piece = String::new();
    let mut minus = false;
    for ch in inside.chars() {
        match ch {
            '+' | '-' if !piece.trim().is_empty() => {
                pieces.push((minus, piece.trim().to_string()));
                piece.clear();
                minus = ch == '-';
            }
            '-' if piece.trim().is_empty() => minus = !minus,
            '+' => {}
            other => piece.push(other),
        }
    }
    if !piece.trim().is_empty() {
        pieces.push((minus, piece.trim().to_string()));
    }

    for (negative, piece) in pieces {
        let lower = piece.to_ascii_lowercase();
        // A scaled index: `rcx*4`, or `4*rcx`, which some assemblers print.
        if let Some((a, b)) = lower.split_once('*') {
            let (reg, scale) = match (register(a.trim()), register(b.trim())) {
                (Some(reg), None) => (reg, b.trim()),
                (None, Some(reg)) => (reg, a.trim()),
                _ => {
                    // Neither side is a register: it is arithmetic, and the calculator does
                    // it. Falls through to the displacement below.
                    let value = calc::evaluate(&lower)
                        .map(|a| a.0)
                        .map_err(|_| AsmError::BadOperand(piece.clone()))?;
                    mem.disp += if negative { -value } else { value };
                    continue;
                }
            };
            if negative {
                return Err(AsmError::Unsupported(
                    "an index register is added, not subtracted".to_string(),
                ));
            }
            let scale: u8 = match scale {
                "1" => 1,
                "2" => 2,
                "4" => 4,
                "8" => 8,
                other => {
                    return Err(AsmError::Unsupported(format!(
                        "{other}: an index is scaled by 1, 2, 4 or 8"
                    )))
                }
            };
            if mem.index.is_some() {
                return Err(AsmError::Unsupported(
                    "an address has one index register".to_string(),
                ));
            }
            mem.index = Some(reg);
            mem.scale = scale;
            continue;
        }
        if lower == "rip" || lower == "eip" {
            if negative {
                return Err(AsmError::Unsupported(
                    "the instruction pointer is added, not subtracted".to_string(),
                ));
            }
            mem.rip = true;
            continue;
        }
        if let Some(reg) = register(&lower) {
            if negative {
                return Err(AsmError::Unsupported(
                    "a register in an address is added, not subtracted".to_string(),
                ));
            }
            // The first register is the base; a second one is the index, as every assembler
            // reads `[rbx+rcx]`.
            if mem.base.is_none() {
                mem.base = Some(reg);
            } else if mem.index.is_none() {
                mem.index = Some(reg);
            } else {
                return Err(AsmError::Unsupported(
                    "an address has a base and one index".to_string(),
                ));
            }
            continue;
        }
        let value = calc::evaluate(&lower)
            .map(|a| a.0)
            .map_err(|_| AsmError::BadOperand(piece.clone()))?;
        mem.disp += if negative { -value } else { value };
    }

    if mem.rip && (mem.base.is_some() || mem.index.is_some()) {
        return Err(AsmError::Unsupported(
            "an address relative to the instruction pointer has nothing else in it".to_string(),
        ));
    }
    Ok(mem)
}

/// The condition of a `jcc`, by every name it goes by.
fn condition(suffix: &str) -> Option<u8> {
    Some(match suffix {
        "o" => 0,
        "no" => 1,
        "b" | "c" | "nae" => 2,
        "ae" | "nb" | "nc" => 3,
        "e" | "z" => 4,
        "ne" | "nz" => 5,
        "be" | "na" => 6,
        "a" | "nbe" => 7,
        "s" => 8,
        "ns" => 9,
        "p" | "pe" => 10,
        "np" | "po" => 11,
        "l" | "nge" => 12,
        "ge" | "nl" => 13,
        "le" | "ng" => 14,
        "g" | "nle" => 15,
        _ => return None,
    })
}

/// The `/digit` of the group-1 arithmetic instructions, and their register-to-register opcode.
fn arith(mnemonic: &str) -> Option<(u8, u8)> {
    Some(match mnemonic {
        "add" => (0, 0x01),
        "or" => (1, 0x09),
        "adc" => (2, 0x11),
        "sbb" => (3, 0x19),
        "and" => (4, 0x21),
        "sub" => (5, 0x29),
        "xor" => (6, 0x31),
        "cmp" => (7, 0x39),
        _ => return None,
    })
}

/// What is being written, and for which processor.
struct Out {
    bytes: Vec<u8>,
    bits: u32,
}

impl Out {
    fn byte(&mut self, b: u8) -> &mut Self {
        self.bytes.push(b);
        self
    }

    /// The prefixes an operand size needs: `0x66` for 16 bits, REX for 64 or for the
    /// registers that only exist with one.
    fn prefixes(&mut self, size: u8, reg: Option<Reg>, rm: Option<Reg>) -> Result<()> {
        if size == 16 {
            self.bytes.push(0x66);
        }
        let wide = size == 64;
        let r = reg.map(|r| r.num >= 8).unwrap_or(false);
        let b = rm.map(|r| r.num >= 8).unwrap_or(false);
        // A low byte of rsp/rbp/rsi/rdi exists only with a REX prefix.
        let low_byte = [reg, rm]
            .iter()
            .flatten()
            .any(|r| r.size == 8 && !r.high_byte && r.num >= 4);
        let high_byte = [reg, rm].iter().flatten().any(|r| r.high_byte);
        if self.bits == 64 {
            if high_byte && (wide || r || b || low_byte) {
                return Err(AsmError::Unsupported(
                    "ah, ch, dh and bh cannot be used with the registers that need a REX prefix"
                        .to_string(),
                ));
            }
            if wide || r || b || low_byte {
                let rex = 0x40 | ((wide as u8) << 3) | ((r as u8) << 2) | (b as u8);
                self.bytes.push(rex);
            }
        } else {
            if wide {
                return Err(AsmError::Arch("a 64-bit register in 32-bit code"));
            }
            if r || b {
                return Err(AsmError::Arch("r8 to r15 in 32-bit code"));
            }
        }
        Ok(())
    }

    /// Register-direct ModRM: both operands are registers, so there is no address to encode.
    fn modrm_rr(&mut self, reg: u8, rm: u8) -> &mut Self {
        self.byte(0xC0 | ((reg & 7) << 3) | (rm & 7))
    }

    /// The prefixes for an instruction whose r/m operand is an address.
    ///
    /// Three bits come from three different places here: `W` from how wide the *operand* is,
    /// `R` from the register on the other side, and `B` and `X` from the base and the index
    /// of the address — which is why a register r/m cannot share this code.
    fn prefixes_mem(&mut self, size: u8, reg: Option<Reg>, mem: &Mem) -> Result<()> {
        let want = if self.bits == 64 { 64 } else { 32 };
        for part in [mem.base, mem.index].into_iter().flatten() {
            if part.size != want {
                return Err(AsmError::Unsupported(format!(
                    "an address is built from {want}-bit registers in this code"
                )));
            }
        }
        if mem.rip && self.bits != 64 {
            return Err(AsmError::Arch(
                "an address relative to the instruction pointer",
            ));
        }
        if let Some(index) = mem.index {
            if index.num == 4 {
                return Err(AsmError::Unsupported(
                    "rsp cannot be an index register — the encoding has no room for it".to_string(),
                ));
            }
        }
        if size == 16 {
            self.bytes.push(0x66);
        }
        let wide = size == 64;
        let r = reg.map(|r| r.num >= 8).unwrap_or(false);
        let x = mem.index.map(|r| r.num >= 8).unwrap_or(false);
        let b = mem.base.map(|r| r.num >= 8).unwrap_or(false);
        let low_byte = reg
            .map(|r| r.size == 8 && !r.high_byte && r.num >= 4)
            .unwrap_or(false);
        let high_byte = reg.map(|r| r.high_byte).unwrap_or(false);
        if self.bits == 64 {
            if high_byte && (wide || r || x || b || low_byte) {
                return Err(AsmError::Unsupported(
                    "ah, ch, dh and bh cannot be used with the registers that need a REX prefix"
                        .to_string(),
                ));
            }
            if wide || r || x || b || low_byte {
                let rex =
                    0x40 | ((wide as u8) << 3) | ((r as u8) << 2) | ((x as u8) << 1) | (b as u8);
                self.bytes.push(rex);
            }
        } else {
            if wide {
                return Err(AsmError::Arch("a 64-bit operand in 32-bit code"));
            }
            if r || x || b {
                return Err(AsmError::Arch("r8 to r15 in 32-bit code"));
            }
        }
        Ok(())
    }

    /// ModRM, the SIB byte where one is needed, and the displacement.
    ///
    /// The special cases are the encoding's, not ours, and each one is the reason a
    /// hand-rolled ModRM gets it wrong: `rsp` and `r12` as a base need a SIB byte to be said
    /// at all; `rbp` and `r13` as a base cannot use the no-displacement form, because that
    /// bit pattern already means something else (`[rip+disp]` in 64-bit code, a bare address
    /// in 32-bit); and an address with no base at all is that same pattern, so it carries a
    /// four-byte displacement whether it needs one or not.
    fn modrm_mem(&mut self, reg: u8, mem: &Mem) -> Result<()> {
        let reg = (reg & 7) << 3;
        if mem.rip {
            // `mod=00, rm=101`: the displacement is counted from the end of the instruction,
            // which is what the disassembler prints and therefore what was typed back.
            if !fits32(mem.disp) {
                return Err(AsmError::OutOfRange(
                    "a displacement is at most 32 bits".to_string(),
                ));
            }
            self.byte(reg | 5);
            self.imm32(mem.disp);
            return Ok(());
        }
        let base = mem.base;
        let need_sib = mem.index.is_some() || base.map(|b| b.num & 7 == 4).unwrap_or(true);
        // No base: the bare-displacement form, which is four bytes of displacement and, in
        // 64-bit code, a SIB byte that says "no base and no index".
        let (mode, disp_bytes) = match base {
            None => (0u8, 4),
            Some(b) if mem.disp == 0 && b.num & 7 != 5 => (0, 0),
            Some(_) if (-128..=127).contains(&mem.disp) => (1, 1),
            Some(_) => {
                if !fits32(mem.disp) {
                    return Err(AsmError::OutOfRange(
                        "a displacement is at most 32 bits".to_string(),
                    ));
                }
                (2, 4)
            }
        };
        let rm = if need_sib {
            4
        } else {
            base.expect("a base").num & 7
        };
        self.byte((mode << 6) | reg | rm);
        if need_sib {
            let scale = match mem.scale {
                1 => 0u8,
                2 => 1,
                4 => 2,
                8 => 3,
                _ => {
                    return Err(AsmError::Unsupported(
                        "an index is scaled by 1, 2, 4 or 8".to_string(),
                    ))
                }
            };
            let index = mem.index.map(|i| i.num & 7).unwrap_or(4);
            let base_field = base.map(|b| b.num & 7).unwrap_or(5);
            self.byte((scale << 6) | (index << 3) | base_field);
        }
        match disp_bytes {
            1 => self.imm8(mem.disp),
            4 => self.imm32(mem.disp),
            _ => {}
        }
        Ok(())
    }

    fn imm8(&mut self, v: i64) {
        self.bytes.push(v as u8);
    }
    fn imm16(&mut self, v: i64) {
        self.bytes.extend_from_slice(&(v as u16).to_le_bytes());
    }
    fn imm32(&mut self, v: i64) {
        self.bytes.extend_from_slice(&(v as u32).to_le_bytes());
    }
    fn imm64(&mut self, v: i64) {
        self.bytes.extend_from_slice(&(v as u64).to_le_bytes());
    }
}

fn fits8(v: i64) -> bool {
    (-128..=255).contains(&v)
}
fn fits32(v: i64) -> bool {
    (-(1i64 << 31)..(1i64 << 32)).contains(&v)
}

/// Assemble one instruction for `arch`, as if it sat at `address`.
///
/// The address is what makes a branch writable the way a reader thinks of it: `jmp 0x401000`
/// is the address to end up at, and the displacement is worked out from here.
pub fn assemble(line: &str, arch: Arch, address: u64) -> Result<Assembled> {
    if !arch.disassemblable() {
        return Err(AsmError::Arch(match arch {
            Arch::Aarch64 => "aarch64",
            _ => "this architecture",
        }));
    }
    let line = line.trim();
    if line.is_empty() {
        return Err(AsmError::Empty);
    }
    let lower = line.to_ascii_lowercase();
    let (mnemonic, rest) = match lower.split_once(char::is_whitespace) {
        Some((m, r)) => (m, r.trim()),
        None => (lower.as_str(), ""),
    };
    // `jmp short 0x401000` and `mov dword eax, 1`: the size words are noise here, because the
    // operands already say how wide they are. `short` and `near` do pick a branch form.
    let (forced, rest) = match rest.split_once(char::is_whitespace) {
        Some(("short", r)) => (Some("short"), r.trim()),
        Some(("near", r)) => (Some("near"), r.trim()),
        _ => (None, rest),
    };
    let args: Vec<&str> = if rest.is_empty() {
        Vec::new()
    } else {
        rest.split(',').collect()
    };

    let mut out = Out {
        bytes: Vec::new(),
        bits: arch.bitness(),
    };
    encode(&mut out, mnemonic, forced, &args, address)?;

    // Decode what we made. The dialog shows this, and a disagreement between the two is a bug
    // here rather than something to write into a file.
    let decoded = disasm::disassemble(&out.bytes, 0, address, arch, 2).unwrap_or_default();
    match decoded.first() {
        Some(line) if !line.invalid && line.bytes.len() == out.bytes.len() => Ok(Assembled {
            bytes: out.bytes,
            text: line.text.clone(),
        }),
        _ => Err(AsmError::Unsupported(format!(
            "{line}: assembled to bytes the disassembler does not agree with"
        ))),
    }
}

fn encode(
    out: &mut Out,
    mnemonic: &str,
    forced: Option<&str>,
    args: &[&str],
    address: u64,
) -> Result<()> {
    // The instructions with no operands at all.
    if args.is_empty() {
        let one = |out: &mut Out, b: u8| {
            out.byte(b);
            Ok(())
        };
        return match mnemonic {
            "nop" => one(out, 0x90),
            "ret" | "retn" => one(out, 0xC3),
            "int3" => one(out, 0xCC),
            "hlt" => one(out, 0xF4),
            "leave" => one(out, 0xC9),
            "pushf" | "pushfq" => one(out, 0x9C),
            "popf" | "popfq" => one(out, 0x9D),
            "cld" => one(out, 0xFC),
            "std" => one(out, 0xFD),
            "clc" => one(out, 0xF8),
            "stc" => one(out, 0xF9),
            "cmc" => one(out, 0xF5),
            "cdq" => one(out, 0x99),
            "cqo" => {
                out.byte(0x48).byte(0x99);
                Ok(())
            }
            "ud2" => {
                out.byte(0x0F).byte(0x0B);
                Ok(())
            }
            "syscall" => {
                out.byte(0x0F).byte(0x05);
                Ok(())
            }
            _ => Err(AsmError::Unknown(mnemonic.to_string())),
        };
    }

    // A branch: one operand, and it is an address to arrive at rather than a displacement.
    let branch = mnemonic == "jmp"
        || mnemonic == "call"
        || mnemonic.strip_prefix('j').and_then(condition).is_some();
    if branch && args.len() == 1 {
        return branch_to(out, mnemonic, forced, args[0], address);
    }

    match (mnemonic, args.len()) {
        // `ret 8`: pop that many bytes of arguments with it.
        ("ret" | "retn", 1) => {
            let Operand::Imm(v) = operand(args[0])? else {
                return Err(AsmError::BadOperand(args[0].trim().to_string()));
            };
            out.byte(0xC2).imm16(v);
            Ok(())
        }
        ("int", 1) => match operand(args[0])? {
            Operand::Imm(3) => {
                out.byte(0xCC);
                Ok(())
            }
            Operand::Imm(v) if fits8(v) => {
                out.byte(0xCD).imm8(v);
                Ok(())
            }
            _ => Err(AsmError::OutOfRange(
                "an interrupt number is one byte".to_string(),
            )),
        },
        ("push", 1) | ("pop", 1) => {
            let pushing = mnemonic == "push";
            match operand(args[0])? {
                Operand::Mem(mem) => {
                    // The stack moves in machine words, so the size word is noise here and a
                    // wrong one is a mistake worth saying out loud.
                    let want = if out.bits == 64 { 64 } else { 32 };
                    if mem.size.is_some_and(|s| s != want) {
                        return Err(AsmError::Unsupported(format!(
                            "push and pop move {want} bits at a time in this code"
                        )));
                    }
                    // No REX.W: the 64-bit operand is this instruction's default, and asking
                    // for it is an encoding every disassembler refuses.
                    out.prefixes_mem(32, None, &mem)?;
                    out.byte(if pushing { 0xFF } else { 0x8F });
                    out.modrm_mem(if pushing { 6 } else { 0 }, &mem)?;
                    Ok(())
                }
                Operand::Reg(reg) => {
                    // The stack always moves in machine words: 64 bits here, 32 in 32-bit code.
                    let want = if out.bits == 64 { 64 } else { 32 };
                    if reg.size != want && reg.size != 16 {
                        return Err(AsmError::Unsupported(format!(
                            "push and pop move {want}-bit registers in this code"
                        )));
                    }
                    if reg.size == 16 {
                        out.byte(0x66);
                    }
                    if reg.num >= 8 {
                        if out.bits != 64 {
                            return Err(AsmError::Arch("r8 to r15 in 32-bit code"));
                        }
                        out.byte(0x41);
                    }
                    out.byte(if pushing { 0x50 } else { 0x58 } + (reg.num & 7));
                    Ok(())
                }
                Operand::Imm(v) if pushing => {
                    if fits8(v) && (-128..=127).contains(&v) {
                        out.byte(0x6A).imm8(v);
                    } else if fits32(v) {
                        out.byte(0x68).imm32(v);
                    } else {
                        return Err(AsmError::OutOfRange(
                            "a pushed number is at most 32 bits".to_string(),
                        ));
                    }
                    Ok(())
                }
                Operand::Imm(_) => Err(AsmError::Unsupported(
                    "pop takes a register, not a number".to_string(),
                )),
            }
        }
        // `inc`, `dec`, `not`, `neg`, `mul`, `imul r`, `div`, `idiv`: one register.
        ("inc" | "dec" | "not" | "neg" | "mul" | "imul" | "div" | "idiv", 1) => {
            let what = operand(args[0])?;
            let (opcode, digit) = match mnemonic {
                "inc" => (0xFF, 0),
                "dec" => (0xFF, 1),
                "not" => (0xF7, 2),
                "neg" => (0xF7, 3),
                "mul" => (0xF7, 4),
                "imul" => (0xF7, 5),
                "div" => (0xF7, 6),
                _ => (0xF7, 7),
            };
            match what {
                Operand::Reg(reg) => {
                    out.prefixes(reg.size, None, Some(reg))?;
                    let opcode = if reg.size == 8 { opcode - 1 } else { opcode };
                    out.byte(opcode).modrm_rr(digit, reg.num);
                    Ok(())
                }
                Operand::Mem(mem) => {
                    let size = mem.width(None)?;
                    out.prefixes_mem(size, None, &mem)?;
                    let opcode = if size == 8 { opcode - 1 } else { opcode };
                    out.byte(opcode);
                    out.modrm_mem(digit, &mem)?;
                    Ok(())
                }
                Operand::Imm(_) => Err(AsmError::Unsupported(format!(
                    "{mnemonic} changes something, so its operand is a register or an address"
                ))),
            }
        }
        // `lea`: the one instruction whose operand is the *address* rather than what is at it.
        ("lea", 2) => {
            let (Operand::Reg(dst), Operand::Mem(mem)) = (operand(args[0])?, operand(args[1])?)
            else {
                return Err(AsmError::Unsupported(
                    "lea takes a register and an address in brackets".to_string(),
                ));
            };
            if dst.size == 8 {
                return Err(AsmError::Unsupported(
                    "lea computes an address, which does not fit in a byte".to_string(),
                ));
            }
            out.prefixes_mem(dst.size, Some(dst), &mem)?;
            out.byte(0x8D);
            out.modrm_mem(dst.num, &mem)?;
            Ok(())
        }
        ("mov", 2) => two_mov(out, args),
        ("test", 2) => {
            let (a, b) = (operand(args[0])?, operand(args[1])?);
            match (a, b) {
                (Operand::Reg(dst), Operand::Reg(src)) => {
                    same_size(dst, src)?;
                    out.prefixes(dst.size, Some(src), Some(dst))?;
                    let opcode = if dst.size == 8 { 0x84 } else { 0x85 };
                    out.byte(opcode).modrm_rr(src.num, dst.num);
                    Ok(())
                }
                (Operand::Reg(dst), Operand::Imm(v)) => {
                    if !fits32(v) {
                        return Err(AsmError::OutOfRange(
                            "an immediate here is at most 32 bits".to_string(),
                        ));
                    }
                    out.prefixes(dst.size, None, Some(dst))?;
                    if dst.size == 8 {
                        out.byte(0xF6).modrm_rr(0, dst.num).imm8(v);
                    } else {
                        out.byte(0xF7).modrm_rr(0, dst.num);
                        if dst.size == 16 {
                            out.imm16(v)
                        } else {
                            out.imm32(v)
                        }
                    }
                    Ok(())
                }
                // `test [rbx], eax` and `test eax, [rbx]` are the one instruction: it reads
                // both sides and writes neither, so the order they were typed in does not
                // reach the bytes.
                (Operand::Mem(mem), Operand::Reg(reg)) | (Operand::Reg(reg), Operand::Mem(mem)) => {
                    let size = mem.width(Some(reg))?;
                    out.prefixes_mem(size, Some(reg), &mem)?;
                    out.byte(if size == 8 { 0x84 } else { 0x85 });
                    out.modrm_mem(reg.num, &mem)?;
                    Ok(())
                }
                (Operand::Mem(mem), Operand::Imm(v)) => {
                    let size = mem.width(None)?;
                    if !fits32(v) {
                        return Err(AsmError::OutOfRange(
                            "an immediate here is at most 32 bits".to_string(),
                        ));
                    }
                    out.prefixes_mem(size, None, &mem)?;
                    out.byte(if size == 8 { 0xF6 } else { 0xF7 });
                    out.modrm_mem(0, &mem)?;
                    match size {
                        8 => out.imm8(v),
                        16 => out.imm16(v),
                        _ => out.imm32(v),
                    };
                    Ok(())
                }
                _ => Err(AsmError::Unsupported(
                    "test compares two things, and at most one of them is a number".to_string(),
                )),
            }
        }
        ("xchg", 2) => {
            match (operand(args[0])?, operand(args[1])?) {
                (Operand::Reg(a), Operand::Reg(b)) => {
                    same_size(a, b)?;
                    out.prefixes(a.size, Some(b), Some(a))?;
                    let opcode = if a.size == 8 { 0x86 } else { 0x87 };
                    out.byte(opcode).modrm_rr(b.num, a.num);
                    Ok(())
                }
                // The swap is symmetrical, and so is the encoding: one of the two sides goes
                // in the ModRM byte whichever way round it was written.
                (Operand::Mem(mem), Operand::Reg(reg)) | (Operand::Reg(reg), Operand::Mem(mem)) => {
                    let size = mem.width(Some(reg))?;
                    out.prefixes_mem(size, Some(reg), &mem)?;
                    out.byte(if size == 8 { 0x86 } else { 0x87 });
                    out.modrm_mem(reg.num, &mem)?;
                    Ok(())
                }
                _ => Err(AsmError::Unsupported(
                    "xchg swaps two places, and a number is not one".to_string(),
                )),
            }
        }
        (m, 2) if arith(m).is_some() => {
            let (digit, rr) = arith(m).unwrap();
            match (operand(args[0])?, operand(args[1])?) {
                (Operand::Reg(dst), Operand::Reg(src)) => {
                    same_size(dst, src)?;
                    out.prefixes(dst.size, Some(src), Some(dst))?;
                    let opcode = if dst.size == 8 { rr - 1 } else { rr };
                    out.byte(opcode).modrm_rr(src.num, dst.num);
                    Ok(())
                }
                (Operand::Reg(dst), Operand::Imm(v)) => {
                    out.prefixes(dst.size, None, Some(dst))?;
                    if dst.size == 8 {
                        if !fits8(v) {
                            return Err(AsmError::OutOfRange("that will not fit in a byte".into()));
                        }
                        out.byte(0x80).modrm_rr(digit, dst.num).imm8(v);
                    } else if (-128..=127).contains(&v) {
                        // The one-byte form, sign-extended: what an assembler picks for a
                        // small number, and what a disassembler shows back.
                        out.byte(0x83).modrm_rr(digit, dst.num).imm8(v);
                    } else if fits32(v) {
                        out.byte(0x81).modrm_rr(digit, dst.num);
                        if dst.size == 16 {
                            out.imm16(v)
                        } else {
                            out.imm32(v)
                        }
                    } else {
                        return Err(AsmError::OutOfRange(
                            "an immediate here is at most 32 bits".to_string(),
                        ));
                    }
                    Ok(())
                }
                // Reading from memory into a register, and the other way about: the same
                // opcode pair two apart, which is why both live in one arm.
                (Operand::Reg(dst), Operand::Mem(mem)) => {
                    let size = mem.width(Some(dst))?;
                    out.prefixes_mem(size, Some(dst), &mem)?;
                    out.byte(if size == 8 { rr + 1 } else { rr + 2 });
                    out.modrm_mem(dst.num, &mem)?;
                    Ok(())
                }
                (Operand::Mem(mem), Operand::Reg(src)) => {
                    let size = mem.width(Some(src))?;
                    out.prefixes_mem(size, Some(src), &mem)?;
                    out.byte(if size == 8 { rr - 1 } else { rr });
                    out.modrm_mem(src.num, &mem)?;
                    Ok(())
                }
                (Operand::Mem(mem), Operand::Imm(v)) => {
                    let size = mem.width(None)?;
                    out.prefixes_mem(size, None, &mem)?;
                    if size == 8 {
                        if !fits8(v) {
                            return Err(AsmError::OutOfRange("that will not fit in a byte".into()));
                        }
                        out.byte(0x80);
                        out.modrm_mem(digit, &mem)?;
                        out.imm8(v);
                    } else if (-128..=127).contains(&v) {
                        out.byte(0x83);
                        out.modrm_mem(digit, &mem)?;
                        out.imm8(v);
                    } else if fits32(v) {
                        out.byte(0x81);
                        out.modrm_mem(digit, &mem)?;
                        if size == 16 {
                            out.imm16(v)
                        } else {
                            out.imm32(v)
                        };
                    } else {
                        return Err(AsmError::OutOfRange(
                            "an immediate here is at most 32 bits".to_string(),
                        ));
                    }
                    Ok(())
                }
                _ => Err(AsmError::Unsupported(format!(
                    "{m} writes into its first operand, so that one is a register or an address"
                ))),
            }
        }
        (m, n) => Err(AsmError::Unknown(format!(
            "{m} with {n} operand{}",
            if n == 1 { "" } else { "s" }
        ))),
    }
}

fn same_size(a: Reg, b: Reg) -> Result<()> {
    if a.size == b.size {
        Ok(())
    } else {
        Err(AsmError::Unsupported(
            "both registers have to be the same width".to_string(),
        ))
    }
}

fn two_mov(out: &mut Out, args: &[&str]) -> Result<()> {
    match (operand(args[0])?, operand(args[1])?) {
        (Operand::Reg(dst), Operand::Reg(src)) => {
            same_size(dst, src)?;
            out.prefixes(dst.size, Some(src), Some(dst))?;
            let opcode = if dst.size == 8 { 0x88 } else { 0x89 };
            out.byte(opcode).modrm_rr(src.num, dst.num);
            Ok(())
        }
        (Operand::Reg(dst), Operand::Imm(v)) => {
            match dst.size {
                8 => {
                    if !fits8(v) {
                        return Err(AsmError::OutOfRange("that will not fit in a byte".into()));
                    }
                    out.prefixes(8, None, Some(dst))?;
                    out.byte(0xB0 + (dst.num & 7)).imm8(v);
                }
                16 => {
                    out.prefixes(16, None, Some(dst))?;
                    out.byte(0xB8 + (dst.num & 7)).imm16(v);
                }
                32 => {
                    out.prefixes(32, None, Some(dst))?;
                    out.byte(0xB8 + (dst.num & 7)).imm32(v);
                }
                _ => {
                    // A number that fits in 32 bits sign-extends, which is three bytes shorter
                    // than carrying a whole word — and is what every assembler emits.
                    out.prefixes(64, None, Some(dst))?;
                    if (-(1i64 << 31)..(1i64 << 31)).contains(&v) {
                        out.byte(0xC7).modrm_rr(0, dst.num).imm32(v);
                    } else {
                        out.byte(0xB8 + (dst.num & 7)).imm64(v);
                    }
                }
            }
            Ok(())
        }
        // A load and a store: the same opcode pair as the register form, one bit apart.
        (Operand::Reg(dst), Operand::Mem(mem)) => {
            let size = mem.width(Some(dst))?;
            out.prefixes_mem(size, Some(dst), &mem)?;
            out.byte(if size == 8 { 0x8A } else { 0x8B });
            out.modrm_mem(dst.num, &mem)?;
            Ok(())
        }
        (Operand::Mem(mem), Operand::Reg(src)) => {
            let size = mem.width(Some(src))?;
            out.prefixes_mem(size, Some(src), &mem)?;
            out.byte(if size == 8 { 0x88 } else { 0x89 });
            out.modrm_mem(src.num, &mem)?;
            Ok(())
        }
        // Writing a number straight into memory: this is where the size word earns its
        // keep, because nothing else in the line says how many bytes to put down.
        (Operand::Mem(mem), Operand::Imm(v)) => {
            let size = mem.width(None)?;
            if size == 8 && !fits8(v) {
                return Err(AsmError::OutOfRange("that will not fit in a byte".into()));
            }
            if !fits32(v) {
                // There is no form that writes a whole word of immediate into memory: it
                // goes through a register, and saying so is better than a wrong guess.
                return Err(AsmError::OutOfRange(
                    "an immediate written to memory is at most 32 bits — load it into a register first"
                        .to_string(),
                ));
            }
            out.prefixes_mem(size, None, &mem)?;
            out.byte(if size == 8 { 0xC6 } else { 0xC7 });
            out.modrm_mem(0, &mem)?;
            match size {
                8 => out.imm8(v),
                16 => out.imm16(v),
                _ => out.imm32(v),
            };
            Ok(())
        }
        _ => Err(AsmError::Unsupported(
            "mov writes into its first operand, so that one is a register or an address"
                .to_string(),
        )),
    }
}

/// A jump or a call to an address, in the shortest form that reaches it.
fn branch_to(
    out: &mut Out,
    mnemonic: &str,
    forced: Option<&str>,
    arg: &str,
    address: u64,
) -> Result<()> {
    let target = match operand(arg)? {
        Operand::Imm(v) => v as u64,
        // `jmp rax` is a real instruction and a different one: an indirect branch.
        Operand::Reg(reg) => {
            if mnemonic != "jmp" && mnemonic != "call" {
                return Err(AsmError::Unsupported(
                    "a conditional jump goes to an address, not through a register".to_string(),
                ));
            }
            if reg.size != out.bits as u8 {
                return Err(AsmError::Unsupported(
                    "an indirect branch goes through a machine-word register".to_string(),
                ));
            }
            if reg.num >= 8 {
                out.byte(0x41);
            }
            let digit = if mnemonic == "jmp" { 4 } else { 2 };
            out.byte(0xFF).modrm_rr(digit, reg.num);
            return Ok(());
        }
        // `call [rip+8]` and friends: the address to branch to is in memory. The same
        // opcode as the register form, which is why it lives here rather than with the
        // other memory operands.
        Operand::Mem(mem) => {
            if mnemonic != "jmp" && mnemonic != "call" {
                return Err(AsmError::Unsupported(
                    "a conditional jump goes to an address, not through memory".to_string(),
                ));
            }
            let want = if out.bits == 64 { 64 } else { 32 };
            if mem.size.is_some_and(|s| s != want) {
                return Err(AsmError::Unsupported(format!(
                    "an indirect branch reads {want} bits"
                )));
            }
            // No REX.W: the machine word is this instruction's default operand size.
            out.prefixes_mem(32, None, &mem)?;
            out.byte(0xFF);
            out.modrm_mem(if mnemonic == "jmp" { 4 } else { 2 }, &mem)?;
            return Ok(());
        }
    };

    // The displacement depends on the length of what we are about to write, so each form is
    // measured from its own end.
    let rel = |len: u64| (target as i64) - (address as i64) - (len as i64);
    let cc = mnemonic.strip_prefix('j').and_then(condition);

    let short_ok = forced != Some("near");
    let near_ok = forced != Some("short");
    match (mnemonic, cc) {
        ("call", _) => {
            let d = rel(5);
            if !fits32(d) {
                return Err(AsmError::OutOfRange(format!(
                    "{target:#x} is too far from here for a call"
                )));
            }
            out.byte(0xE8).imm32(d);
            Ok(())
        }
        ("jmp", _) => {
            let d8 = rel(2);
            if short_ok && (-128..=127).contains(&d8) {
                out.byte(0xEB).imm8(d8);
                return Ok(());
            }
            let d32 = rel(5);
            if near_ok && fits32(d32) {
                out.byte(0xE9).imm32(d32);
                return Ok(());
            }
            Err(AsmError::OutOfRange(format!(
                "{target:#x} is too far from here for a jump of that form"
            )))
        }
        (_, Some(cc)) => {
            let d8 = rel(2);
            if short_ok && (-128..=127).contains(&d8) {
                out.byte(0x70 + cc).imm8(d8);
                return Ok(());
            }
            let d32 = rel(6);
            if near_ok && fits32(d32) {
                out.byte(0x0F).byte(0x80 + cc).imm32(d32);
                return Ok(());
            }
            Err(AsmError::OutOfRange(format!(
                "{target:#x} is too far from here for a conditional jump of that form"
            )))
        }
        _ => Err(AsmError::Unknown(mnemonic.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn asm(line: &str) -> Assembled {
        assemble(line, Arch::X86_64, 0x401000).unwrap_or_else(|e| panic!("{line}: {e}"))
    }

    fn bytes(line: &str) -> Vec<u8> {
        asm(line).bytes
    }

    #[test]
    fn the_instructions_a_patcher_types_assemble_to_the_bytes_they_should() {
        // No operands.
        assert_eq!(bytes("nop"), [0x90]);
        assert_eq!(bytes("ret"), [0xC3]);
        assert_eq!(bytes("int3"), [0xCC]);
        assert_eq!(bytes("leave"), [0xC9]);
        assert_eq!(bytes("ud2"), [0x0F, 0x0B]);
        assert_eq!(bytes("syscall"), [0x0F, 0x05]);
        assert_eq!(bytes("ret 8"), [0xC2, 0x08, 0x00]);

        // Registers, with and without the prefixes they need.
        assert_eq!(bytes("xor eax, eax"), [0x31, 0xC0]);
        assert_eq!(bytes("xor rax, rax"), [0x48, 0x31, 0xC0]);
        assert_eq!(bytes("mov rbp, rsp"), [0x48, 0x89, 0xE5]);
        assert_eq!(bytes("mov r12, r13"), [0x4D, 0x89, 0xEC]);
        assert_eq!(bytes("mov al, bl"), [0x88, 0xD8]);
        assert_eq!(bytes("mov ax, bx"), [0x66, 0x89, 0xD8]);
        assert_eq!(bytes("push rbp"), [0x55]);
        assert_eq!(bytes("pop r15"), [0x41, 0x5F]);
        assert_eq!(bytes("inc rcx"), [0x48, 0xFF, 0xC1]);
        assert_eq!(bytes("neg eax"), [0xF7, 0xD8]);
        assert_eq!(bytes("test eax, eax"), [0x85, 0xC0]);

        // Immediates pick the shortest form, the way an assembler does.
        assert_eq!(bytes("mov eax, 1"), [0xB8, 1, 0, 0, 0]);
        assert_eq!(bytes("mov rax, 1"), [0x48, 0xC7, 0xC0, 1, 0, 0, 0]);
        assert_eq!(
            bytes("mov rax, 0x123456789"),
            [0x48, 0xB8, 0x89, 0x67, 0x45, 0x23, 0x01, 0, 0, 0]
        );
        assert_eq!(bytes("add esp, 8"), [0x83, 0xC4, 0x08]);
        assert_eq!(
            bytes("sub rsp, 0x1000"),
            [0x48, 0x81, 0xEC, 0x00, 0x10, 0, 0]
        );
        assert_eq!(bytes("cmp al, 0x7f"), [0x80, 0xF8, 0x7F]);
        assert_eq!(bytes("push 1"), [0x6A, 0x01]);
        assert_eq!(bytes("push 0x1000"), [0x68, 0x00, 0x10, 0, 0]);

        // A number is whatever the calculator can read.
        assert_eq!(bytes("mov eax, 1Fh"), bytes("mov eax, 0x1f"));
        assert_eq!(bytes("mov eax, 0b1010"), bytes("mov eax, 10"));
        assert_eq!(bytes("mov eax, 0x10+6"), bytes("mov eax, 22"));
    }

    #[test]
    fn a_branch_is_written_as_the_address_to_arrive_at() {
        // Two bytes forwards from 0x401000 lands on 0x401002: a jump to itself-plus-two is
        // the shortest nothing there is.
        assert_eq!(bytes("jmp 0x401002"), [0xEB, 0x00]);
        assert_eq!(bytes("jmp 0x401010"), [0xEB, 0x0E]);
        // Backwards, the classic `jmp $-2`.
        assert_eq!(bytes("jmp 0x401000"), [0xEB, 0xFE]);
        // Too far for one byte: the near form, measured from its own end.
        assert_eq!(bytes("jmp 0x402000"), [0xE9, 0xFB, 0x0F, 0x00, 0x00]);
        // `near` and `short` pick the form by hand.
        assert_eq!(bytes("jmp near 0x401010"), [0xE9, 0x0B, 0x00, 0x00, 0x00]);
        assert!(assemble("jmp short 0x402000", Arch::X86_64, 0x401000).is_err());

        assert_eq!(bytes("call 0x401100"), [0xE8, 0xFB, 0x00, 0x00, 0x00]);
        assert_eq!(bytes("je 0x401010"), [0x74, 0x0E]);
        assert_eq!(bytes("jne 0x401010"), [0x75, 0x0E]);
        assert_eq!(bytes("jz 0x401010"), bytes("je 0x401010"));
        assert_eq!(bytes("jne 0x402000"), [0x0F, 0x85, 0xFA, 0x0F, 0x00, 0x00]);
        // Through a register.
        assert_eq!(bytes("call rax"), [0xFF, 0xD0]);
        assert_eq!(bytes("jmp r11"), [0x41, 0xFF, 0xE3]);
    }

    /// The property that matters more than any single encoding: what comes out is what the
    /// disassembler reads back. Every instruction here is assembled, then decoded, then
    /// assembled again from the decoder's own text — and the bytes must not move.
    #[test]
    fn every_instruction_round_trips_through_the_disassembler() {
        let lines = [
            "nop",
            "ret",
            "int3",
            "leave",
            "push rbp",
            "pop rbp",
            "mov rbp, rsp",
            "xor eax, eax",
            "mov eax, 0x2a",
            "mov rax, 0x123456789",
            "add rsp, 0x18",
            "sub esp, 4",
            "cmp eax, 1",
            "and al, 0x0f",
            "or r9, r10",
            "test rcx, rcx",
            "dec rsi",
            "not ebx",
            "neg rdx",
            "xchg eax, ecx",
            "jmp 0x401010",
            "je 0x401040",
            "jne 0x400f00",
            "call 0x402000",
            "call rax",
            "jmp r11",
            "syscall",
            "ud2",
            // Addresses, in every shape the encoding has a rule for.
            "mov eax, [rbx]",
            "mov [rbx], eax",
            "mov rax, [rbx+8]",
            "mov rax, [rbx+rcx*4+8]",
            "mov eax, [rsp+0x20]",
            "mov eax, [rbp]",
            "mov eax, [r12]",
            "mov eax, [r13+4]",
            "mov eax, [rcx*8+0x10]",
            "mov eax, [0x401000]",
            "mov byte [rbx], 1",
            "mov dword [rbx+4], 0x2a",
            "mov qword [rbp-8], -1",
            "add eax, [rbx]",
            "add [rbx], eax",
            "cmp dword [rbx], 0x10",
            "sub al, [rsi]",
            "and qword [rsp+8], 0x0f",
            "inc dword [rbx]",
            "neg qword [rbx+rcx*2]",
            "test byte [rdi], 1",
            "test [rbx], eax",
            "xchg [rbx], eax",
            "push qword [rbx]",
            "pop qword [rbx]",
            "lea rax, [rbx+rcx*8-0x10]",
            "jmp qword [rbx]",
            "call qword [rbx+8]",
            // `[rip+disp]` is not in this list on purpose: the disassembler resolves it to
            // the address it lands on, which is what one wants to read and is a different
            // text. The test below checks that pair instead.
        ];
        for line in lines {
            let first = asm(line);
            // The disassembler's own spelling, assembled again.
            let again = assemble(&first.text, Arch::X86_64, 0x401000)
                .unwrap_or_else(|e| panic!("{line} decoded as {:?}: {e}", first.text));
            assert_eq!(
                first.bytes, again.bytes,
                "{line} decoded as {:?} and came back different",
                first.text
            );
        }
    }

    /// Addresses, at the level of the bytes: the rules that are easy to get wrong are rsp
    /// and r12 needing a SIB byte, rbp and r13 needing a displacement, and an index with no
    /// base at all.
    #[test]
    fn the_shapes_an_address_can_take_encode_the_way_the_manual_says() {
        assert_eq!(bytes("mov eax, [rbx]"), [0x8B, 0x03]);
        assert_eq!(bytes("mov [rbx], eax"), [0x89, 0x03]);
        assert_eq!(bytes("mov rax, [rbx+8]"), [0x48, 0x8B, 0x43, 0x08]);
        assert_eq!(
            bytes("mov rax, [rbx+rcx*4+8]"),
            [0x48, 0x8B, 0x44, 0x8B, 0x08]
        );
        // rsp as a base has no register field of its own: it is spelled with a SIB byte.
        assert_eq!(bytes("mov eax, [rsp]"), [0x8B, 0x04, 0x24]);
        assert_eq!(bytes("mov eax, [rsp+0x20]"), [0x8B, 0x44, 0x24, 0x20]);
        // rbp as a base with no displacement is the encoding that means "no base at all",
        // so it is written with a displacement of zero.
        assert_eq!(bytes("mov eax, [rbp]"), [0x8B, 0x45, 0x00]);
        // The same two rules again for the registers that only exist in 64-bit code.
        assert_eq!(bytes("mov eax, [r12]"), [0x41, 0x8B, 0x04, 0x24]);
        assert_eq!(bytes("mov eax, [r13]"), [0x41, 0x8B, 0x45, 0x00]);
        // An index with no base: the base field says none, and the displacement is there
        // whether one wrote it or not.
        assert_eq!(
            bytes("mov eax, [rcx*8+0x10]"),
            [0x8B, 0x04, 0xCD, 0x10, 0, 0, 0]
        );
        // A bare address.
        assert_eq!(
            bytes("mov eax, [0x401000]"),
            [0x8B, 0x04, 0x25, 0x00, 0x10, 0x40, 0x00]
        );
        // Writing a number: the size word says how much of it reaches memory.
        assert_eq!(bytes("mov byte [rbx], 1"), [0xC6, 0x03, 0x01]);
        assert_eq!(bytes("mov word [rbx], 1"), [0x66, 0xC7, 0x03, 0x01, 0x00]);
        assert_eq!(bytes("mov dword [rbx], 1"), [0xC7, 0x03, 0x01, 0, 0, 0]);
        assert_eq!(
            bytes("mov qword [rbx], 1"),
            [0x48, 0xC7, 0x03, 0x01, 0, 0, 0]
        );
        // The stack moves a machine word at a time, and asking for it in the prefix is an
        // encoding nothing accepts — so there is no REX.W here.
        assert_eq!(bytes("push qword [rbx]"), [0xFF, 0x33]);
        assert_eq!(bytes("pop qword [rbx]"), [0x8F, 0x03]);
        // lea takes the address itself, which is why it has no size word.
        assert_eq!(
            bytes("lea rax, [rbx+rcx*8-0x10]"),
            [0x48, 0x8D, 0x44, 0xCB, 0xF0]
        );
        // The pieces may be written in any order, which is what one does when retyping a
        // line off a disassembly.
        assert_eq!(bytes("mov eax, [8+rbx]"), bytes("mov eax, [rbx+8]"));
        assert_eq!(bytes("mov eax, [rcx*4+rbx]"), bytes("mov eax, [rbx+rcx*4]"));
    }

    /// `[rip+disp]`: the displacement is counted from the end of the instruction, so the
    /// disassembler shows the address it lands on rather than the number that was typed.
    /// Both halves of that are worth pinning down.
    #[test]
    fn an_address_relative_to_the_instruction_pointer_is_counted_from_the_next_instruction() {
        let made = asm("mov eax, [rip+0x2e35]");
        assert_eq!(made.bytes, [0x8B, 0x05, 0x35, 0x2E, 0x00, 0x00]);
        // Six bytes long, so the address is 0x401006 + 0x2e35.
        assert!(
            made.text.to_ascii_lowercase().contains("403e3b"),
            "{}",
            made.text
        );
        assert_eq!(
            bytes("lea rdi, [rip+0x100]"),
            [0x48, 0x8D, 0x3D, 0x00, 0x01, 0x00, 0x00]
        );
        assert_eq!(bytes("call qword [rip+0x20]"), [0xFF, 0x15, 0x20, 0, 0, 0]);
        // 32-bit code has no such address, and says so.
        assert!(matches!(
            assemble("mov eax, [rip+8]", Arch::X86, 0x401000),
            Err(AsmError::Arch(_))
        ));
    }

    #[test]
    fn thirty_two_bit_code_refuses_what_it_cannot_hold() {
        let at = |line: &str| assemble(line, Arch::X86, 0x401000);
        assert_eq!(at("mov eax, 1").unwrap().bytes, [0xB8, 1, 0, 0, 0]);
        assert_eq!(at("push ebp").unwrap().bytes, [0x55]);
        // A 64-bit register and the registers that only exist in 64-bit code.
        assert!(matches!(at("mov rax, 1"), Err(AsmError::Arch(_))));
        assert!(matches!(at("mov r8d, 1"), Err(AsmError::Arch(_))));
        // aarch64 is not ours to assemble, and says so rather than emitting x86.
        assert!(matches!(
            assemble("nop", Arch::Aarch64, 0),
            Err(AsmError::Arch(_))
        ));
    }

    #[test]
    fn what_it_will_not_do_it_says_rather_than_guesses() {
        let at = |line: &str| assemble(line, Arch::X86_64, 0x401000);
        assert!(matches!(at(""), Err(AsmError::Empty)));
        assert!(matches!(at("   "), Err(AsmError::Empty)));
        // An address with nothing to say how wide the thing at it is: the one guess that
        // would quietly write the wrong number of bytes, so it is refused by name.
        let Err(AsmError::Unsupported(msg)) = at("mov [rbx], 1") else {
            panic!("an address with no width must be refused")
        };
        assert!(msg.contains("byte, word, dword or qword"), "{msg}");
        // A size word that contradicts the register on the other side.
        assert!(matches!(
            at("mov byte [rbx], eax"),
            Err(AsmError::Unsupported(_))
        ));
        // rsp cannot be an index, and a half-written bracket is not an address.
        assert!(matches!(
            at("mov eax, [rbx+rsp*2]"),
            Err(AsmError::Unsupported(_))
        ));
        assert!(matches!(at("mov eax, [rbx"), Err(AsmError::BadOperand(_))));
        // A whole word of immediate has no form that reaches memory.
        assert!(matches!(
            at("mov qword [rbx], 0x123456789"),
            Err(AsmError::OutOfRange(_))
        ));
        // `lea` wants an address: that is the whole point of it.
        assert!(matches!(at("lea rax, rbx"), Err(AsmError::Unsupported(_))));
        // An instruction nobody here knows.
        assert!(matches!(
            at("vfmadd132ps xmm0, xmm1, xmm2"),
            Err(AsmError::Unknown(_))
        ));
        assert!(matches!(at("mov eax"), Err(AsmError::Unknown(_))));
        // Mismatched widths, a number that does not fit, a target out of reach.
        assert!(matches!(at("mov eax, rbx"), Err(AsmError::Unsupported(_))));
        assert!(matches!(at("mov al, 0x100"), Err(AsmError::OutOfRange(_))));
        assert!(matches!(
            at("call 0x7fffffffffff"),
            Err(AsmError::OutOfRange(_))
        ));
        // ah and r8 cannot share a prefix, and that is said rather than silently mis-encoded.
        assert!(matches!(at("mov ah, r8b"), Err(AsmError::Unsupported(_))));
        assert!(at("mov ah, bl").is_ok());
        // Something that is neither a register nor a number.
        assert!(matches!(
            at("mov eax, wobble"),
            Err(AsmError::BadOperand(_))
        ));
    }
}
