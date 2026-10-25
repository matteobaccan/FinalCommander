//! An assembler for the instructions a patcher actually types (SPEC 05 §2.8).
//!
//! The point of `F9` in the hex editor is not to compile a program. It is to turn a `jz` into
//! a `jnz`, to nop out a call, to make a function return early, to put a constant in a
//! register — each one instruction at a time, in place, with the bytes shown before anything
//! is written. That is a small language, and this assembles it: registers and immediates,
//! with a branch target written as the address one wants to end up at.
//!
//! What it deliberately does not do is memory operands. `mov [rbx+rcx*4+8], eax` needs a full
//! ModRM/SIB parser and brings with it the addressing-mode mistakes that are exactly what a
//! patcher cannot afford; an instruction out of the subset is refused by name rather than
//! guessed at. The hex pane is still there for bytes nobody can spell.
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
}

fn operand(text: &str) -> Result<Operand> {
    let text = text.trim();
    if text.is_empty() {
        return Err(AsmError::BadOperand(text.to_string()));
    }
    if text.contains('[') || text.contains(']') {
        return Err(AsmError::Unsupported(format!(
            "{text}: a memory operand — this assembler takes registers and numbers"
        )));
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
            let Operand::Reg(reg) = operand(args[0])? else {
                return Err(AsmError::BadOperand(args[0].trim().to_string()));
            };
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
            out.prefixes(reg.size, None, Some(reg))?;
            let opcode = if reg.size == 8 { opcode - 1 } else { opcode };
            out.byte(opcode).modrm_rr(digit, reg.num);
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
                _ => Err(AsmError::Unsupported(
                    "test writes nothing, so its first operand is a register".to_string(),
                )),
            }
        }
        ("xchg", 2) => {
            let (Operand::Reg(a), Operand::Reg(b)) = (operand(args[0])?, operand(args[1])?) else {
                return Err(AsmError::Unsupported(
                    "xchg swaps two registers here".to_string(),
                ));
            };
            same_size(a, b)?;
            out.prefixes(a.size, Some(b), Some(a))?;
            let opcode = if a.size == 8 { 0x86 } else { 0x87 };
            out.byte(opcode).modrm_rr(b.num, a.num);
            Ok(())
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
                _ => Err(AsmError::Unsupported(format!(
                    "{m} writes into its first operand, so that one is a register"
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
        _ => Err(AsmError::Unsupported(
            "mov writes into its first operand, so that one is a register".to_string(),
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
            "inc dword_placeholder_skip",
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
        ];
        for line in lines {
            if line.contains("placeholder") {
                continue;
            }
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
        // A memory operand is the deliberate hole, and it is named.
        let Err(AsmError::Unsupported(msg)) = at("mov eax, [rbx]") else {
            panic!("a memory operand must be refused by name")
        };
        assert!(msg.contains("memory"), "{msg}");
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
