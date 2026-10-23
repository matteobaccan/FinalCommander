//! .NET assemblies: the CLI metadata we need to find methods, and their IL.
//!
//! Scope, stated plainly: this reads the metadata root, the `#~` table stream and the string
//! heaps far enough to list methods with their names and decode their IL bodies
//! (ECMA-335 §II.22–§II.25, §III). It does **not** parse signatures, generics or custom
//! attributes, and it is not a decompiler: `ilspycmd` through a `[[decompile]]` rule is still
//! the way to get C# back. Tokens we cannot name are printed as `Table(row)`, never guessed.

use std::fmt::Write as _;

#[derive(Debug, thiserror::Error)]
pub enum IlError {
    #[error("not a .NET assembly: no CLI header")]
    NotManaged,
    #[error("truncated metadata at byte {0}")]
    Truncated(usize),
    #[error("unexpected metadata: {0}")]
    Bad(&'static str),
}

/// Index sizes depend on how big the heaps and tables are, so everything is computed once.
#[derive(Debug, Clone, Copy)]
struct Sizes {
    string: usize,
    guid: usize,
    blob: usize,
}

#[derive(Debug, Clone)]
pub struct Method {
    pub name: String,
    pub type_name: String,
    pub rva: u32,
    pub flags: u16,
    pub impl_flags: u16,
}

/// What we read out of an assembly.
pub struct Assembly {
    pub runtime_version: String,
    pub methods: Vec<Method>,
    strings: Vec<u8>,
    user_strings: Vec<u8>,
    type_names: Vec<String>,
    type_ref_names: Vec<String>,
    member_refs: Vec<String>,
    method_names: Vec<String>,
}

impl Assembly {
    /// Resolve a metadata token the way a listing wants to read it.
    pub fn token(&self, token: u32) -> String {
        let table = (token >> 24) as u8;
        let row = (token & 0x00FF_FFFF) as usize;
        let name = |list: &Vec<String>, label: &str| match list.get(row.wrapping_sub(1)) {
            Some(name) if !name.is_empty() => name.clone(),
            _ => format!("{label}({row})"),
        };
        match table {
            0x01 => name(&self.type_ref_names, "TypeRef"),
            0x02 => name(&self.type_names, "TypeDef"),
            0x04 => format!("Field({row})"),
            0x06 => name(&self.method_names, "MethodDef"),
            0x0A => name(&self.member_refs, "MemberRef"),
            0x11 => format!("TypeSpec({row})"),
            0x2B => format!("MethodSpec({row})"),
            0x70 => match self.user_string(row) {
                Some(text) => format!("\"{}\"", text.replace('\n', "\\n")),
                None => format!("String({row})"),
            },
            other => format!("token(0x{other:02X}:{row})"),
        }
    }

    /// A literal from the `#US` heap: `ldstr` is the one token worth resolving to its text.
    fn user_string(&self, offset: usize) -> Option<String> {
        let (len, header) = compressed_u32(&self.user_strings, offset)?;
        let start = offset + header;
        let end = start + len as usize;
        let bytes = self
            .user_strings
            .get(start..end.min(self.user_strings.len()))?;
        // UTF-16LE, with a trailing byte that flags non-ASCII content.
        let mut units: Vec<u16> = Vec::with_capacity(bytes.len() / 2);
        let mut rest = bytes;
        while let [low, high, tail @ ..] = rest {
            units.push(u16::from_le_bytes([*low, *high]));
            rest = tail;
        }
        Some(String::from_utf16_lossy(&units))
    }

    fn string_at(&self, offset: usize) -> String {
        let bytes = &self.strings;
        if offset >= bytes.len() {
            return String::new();
        }
        let end = bytes[offset..]
            .iter()
            .position(|b| *b == 0)
            .map(|p| offset + p)
            .unwrap_or(bytes.len());
        String::from_utf8_lossy(&bytes[offset..end]).into_owned()
    }
}

/// ECMA-335 compressed integer: `(value, bytes consumed)`.
fn compressed_u32(bytes: &[u8], at: usize) -> Option<(u32, usize)> {
    let first = *bytes.get(at)?;
    if first & 0x80 == 0 {
        Some((first as u32, 1))
    } else if first & 0xC0 == 0x80 {
        let second = *bytes.get(at + 1)? as u32;
        Some(((((first & 0x3F) as u32) << 8) | second, 2))
    } else {
        let b1 = *bytes.get(at + 1)? as u32;
        let b2 = *bytes.get(at + 2)? as u32;
        let b3 = *bytes.get(at + 3)? as u32;
        Some((
            (((first & 0x1F) as u32) << 24) | (b1 << 16) | (b2 << 8) | b3,
            4,
        ))
    }
}

fn u16_at(bytes: &[u8], at: usize) -> Result<u16, IlError> {
    Ok(u16::from_le_bytes([
        *bytes.get(at).ok_or(IlError::Truncated(at))?,
        *bytes.get(at + 1).ok_or(IlError::Truncated(at + 1))?,
    ]))
}

fn u32_at(bytes: &[u8], at: usize) -> Result<u32, IlError> {
    Ok(u32::from_le_bytes([
        *bytes.get(at).ok_or(IlError::Truncated(at))?,
        *bytes.get(at + 1).ok_or(IlError::Truncated(at + 1))?,
        *bytes.get(at + 2).ok_or(IlError::Truncated(at + 2))?,
        *bytes.get(at + 3).ok_or(IlError::Truncated(at + 3))?,
    ]))
}

fn index_at(bytes: &[u8], at: usize, size: usize) -> Result<u32, IlError> {
    if size == 2 {
        Ok(u16_at(bytes, at)? as u32)
    } else {
        u32_at(bytes, at)
    }
}

/// Parse the metadata starting at `metadata` (the region the CLI header points at).
///
/// `metadata` must be the whole metadata region; offsets inside it are relative to its start,
/// which is how the format is defined.
pub fn parse_metadata(metadata: &[u8]) -> Result<Assembly, IlError> {
    if metadata.get(..4) != Some(b"BSJB") {
        return Err(IlError::Bad("no BSJB signature"));
    }
    let version_len = u32_at(metadata, 12)? as usize;
    let runtime_version = String::from_utf8_lossy(
        metadata
            .get(16..16 + version_len)
            .ok_or(IlError::Truncated(16))?,
    )
    .trim_end_matches('\0')
    .to_string();

    let mut at = 16 + version_len.next_multiple_of(4);
    at += 2; // flags
    let stream_count = u16_at(metadata, at)?;
    at += 2;

    let mut tables: Option<&[u8]> = None;
    let mut strings: &[u8] = &[];
    let mut user_strings: &[u8] = &[];
    for _ in 0..stream_count {
        let offset = u32_at(metadata, at)? as usize;
        let size = u32_at(metadata, at + 4)? as usize;
        at += 8;
        let name_start = at;
        let name_end = metadata[name_start..]
            .iter()
            .position(|b| *b == 0)
            .map(|p| name_start + p)
            .ok_or(IlError::Truncated(name_start))?;
        let name = String::from_utf8_lossy(&metadata[name_start..name_end]).into_owned();
        at = (name_end + 1).next_multiple_of(4);
        let stream = metadata
            .get(offset..(offset + size).min(metadata.len()))
            .ok_or(IlError::Truncated(offset))?;
        match name.as_str() {
            "#~" | "#-" => tables = Some(stream),
            "#Strings" => strings = stream,
            "#US" => user_strings = stream,
            _ => {}
        }
    }

    let tables = tables.ok_or(IlError::Bad("no table stream"))?;
    let heap_sizes = *tables.get(6).ok_or(IlError::Truncated(6))?;
    let sizes = Sizes {
        string: if heap_sizes & 0x01 != 0 { 4 } else { 2 },
        guid: if heap_sizes & 0x02 != 0 { 4 } else { 2 },
        blob: if heap_sizes & 0x04 != 0 { 4 } else { 2 },
    };
    let valid = u64::from_le_bytes(
        tables
            .get(8..16)
            .ok_or(IlError::Truncated(8))?
            .try_into()
            .map_err(|_| IlError::Bad("short valid mask"))?,
    );

    // Row counts for the present tables, in table order.
    let mut rows = [0u32; 64];
    let mut cursor = 24; // reserved(4) major(1) minor(1) heapsizes(1) reserved(1) valid(8) sorted(8)
    for table in 0..64u8 {
        if valid & (1u64 << table) != 0 {
            rows[table as usize] = u32_at(tables, cursor)?;
            cursor += 4;
        }
    }

    let simple = |table: usize| -> usize {
        if rows[table] >= 1 << 16 {
            4
        } else {
            2
        }
    };
    // A coded index is 2 bytes unless one of its tables is too long for the remaining bits.
    let coded = |candidates: &[usize], bits: u32| -> usize {
        let limit = 1u32 << (16 - bits);
        if candidates.iter().any(|t| rows[*t] >= limit) {
            4
        } else {
            2
        }
    };
    let type_def_or_ref = coded(&[0x02, 0x01, 0x1B], 2);
    let resolution_scope = coded(&[0x00, 0x1A, 0x23, 0x01], 2);
    let member_ref_parent = coded(&[0x02, 0x01, 0x1A, 0x06, 0x1B], 3);

    // Row sizes of the tables we must skip or read.
    let row_size = |table: usize| -> usize {
        match table {
            0x00 => 2 + sizes.string + 3 * sizes.guid,   // Module
            0x01 => resolution_scope + 2 * sizes.string, // TypeRef
            0x02 => 4 + 2 * sizes.string + type_def_or_ref + simple(0x04) + simple(0x06), // TypeDef
            0x03 => simple(0x04),                        // FieldPtr
            0x04 => 2 + sizes.string + sizes.blob,       // Field
            0x05 => simple(0x06),                        // MethodPtr
            0x06 => 4 + 2 + 2 + sizes.string + sizes.blob + simple(0x08), // MethodDef
            0x07 => simple(0x08),                        // ParamPtr
            0x08 => 2 + 2 + sizes.string,                // Param
            0x09 => simple(0x02) + type_def_or_ref,      // InterfaceImpl
            0x0A => member_ref_parent + sizes.string + sizes.blob, // MemberRef
            _ => 0,
        }
    };

    let mut table_start = [0usize; 64];
    let mut offset = cursor;
    for table in 0..=0x0Ausize {
        table_start[table] = offset;
        offset += rows[table] as usize * row_size(table);
    }

    let mut assembly = Assembly {
        runtime_version,
        methods: Vec::new(),
        strings: strings.to_vec(),
        user_strings: user_strings.to_vec(),
        type_names: Vec::new(),
        type_ref_names: Vec::new(),
        member_refs: Vec::new(),
        method_names: Vec::new(),
    };

    // TypeRef names, for `call` targets into other assemblies.
    for row in 0..rows[0x01] as usize {
        let base = table_start[0x01] + row * row_size(0x01);
        let name = index_at(tables, base + resolution_scope, sizes.string)? as usize;
        let namespace =
            index_at(tables, base + resolution_scope + sizes.string, sizes.string)? as usize;
        let name = assembly.string_at(name);
        let namespace = assembly.string_at(namespace);
        assembly.type_ref_names.push(if namespace.is_empty() {
            name
        } else {
            format!("{namespace}.{name}")
        });
    }

    // TypeDef names, and the first method of each type so methods can be attributed.
    let mut type_method_start: Vec<(String, usize)> = Vec::new();
    for row in 0..rows[0x02] as usize {
        let base = table_start[0x02] + row * row_size(0x02);
        let name = index_at(tables, base + 4, sizes.string)? as usize;
        let namespace = index_at(tables, base + 4 + sizes.string, sizes.string)? as usize;
        let method_list = index_at(
            tables,
            base + 4 + 2 * sizes.string + type_def_or_ref + simple(0x04),
            simple(0x06),
        )? as usize;
        let name = assembly.string_at(name);
        let namespace = assembly.string_at(namespace);
        let full = if namespace.is_empty() {
            name
        } else {
            format!("{namespace}.{name}")
        };
        assembly.type_names.push(full.clone());
        type_method_start.push((full, method_list));
    }

    // MemberRef names: `call`/`callvirt` targets.
    for row in 0..rows[0x0A] as usize {
        let base = table_start[0x0A] + row * row_size(0x0A);
        let name = index_at(tables, base + member_ref_parent, sizes.string)? as usize;
        assembly.member_refs.push(assembly.string_at(name));
    }

    // MethodDef: RVA, flags and name.
    let method_rows = rows[0x06] as usize;
    for row in 0..method_rows {
        let base = table_start[0x06] + row * row_size(0x06);
        let rva = u32_at(tables, base)?;
        let impl_flags = u16_at(tables, base + 4)?;
        let flags = u16_at(tables, base + 6)?;
        let name = index_at(tables, base + 8, sizes.string)? as usize;
        let name = assembly.string_at(name);
        let type_name = type_method_start
            .iter()
            .rev()
            .find(|(_, start)| *start <= row + 1)
            .map(|(name, _)| name.clone())
            .unwrap_or_default();
        assembly.method_names.push(name.clone());
        assembly.methods.push(Method {
            name,
            type_name,
            rva,
            flags,
            impl_flags,
        });
    }

    Ok(assembly)
}

/// A method body: its header plus the IL bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub max_stack: u16,
    pub code: Vec<u8>,
    pub local_signature: u32,
    pub fat: bool,
}

/// Read a method body header (ECMA-335 §II.25.4). `at` is the file offset of the body.
pub fn parse_body(bytes: &[u8], at: usize) -> Result<Body, IlError> {
    let first = *bytes.get(at).ok_or(IlError::Truncated(at))?;
    match first & 0x03 {
        0x02 => {
            let size = (first >> 2) as usize;
            let code = bytes
                .get(at + 1..at + 1 + size)
                .ok_or(IlError::Truncated(at + 1))?
                .to_vec();
            Ok(Body {
                max_stack: 8,
                code,
                local_signature: 0,
                fat: false,
            })
        }
        0x03 => {
            let max_stack = u16_at(bytes, at + 2)?;
            let size = u32_at(bytes, at + 4)? as usize;
            let local_signature = u32_at(bytes, at + 8)?;
            let header_size = ((u16_at(bytes, at)? >> 12) as usize) * 4;
            let start = at + header_size.max(12);
            let code = bytes
                .get(start..start + size)
                .ok_or(IlError::Truncated(start))?
                .to_vec();
            Ok(Body {
                max_stack,
                code,
                local_signature,
                fat: true,
            })
        }
        _ => Err(IlError::Bad("not a method body header")),
    }
}

/// What follows an IL opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Arg {
    None,
    I1,
    U1,
    I4,
    I8,
    R4,
    R8,
    Var1,
    Var2,
    Branch1,
    Branch4,
    Token,
    Switch,
    Unknown,
}

/// The one-byte IL opcodes (ECMA-335 Partition III).
const ONE: [(&str, Arg); 256] = {
    let mut t = [("", Arg::Unknown); 256];
    t[0x00] = ("nop", Arg::None);
    t[0x01] = ("break", Arg::None);
    t[0x02] = ("ldarg.0", Arg::None);
    t[0x03] = ("ldarg.1", Arg::None);
    t[0x04] = ("ldarg.2", Arg::None);
    t[0x05] = ("ldarg.3", Arg::None);
    t[0x06] = ("ldloc.0", Arg::None);
    t[0x07] = ("ldloc.1", Arg::None);
    t[0x08] = ("ldloc.2", Arg::None);
    t[0x09] = ("ldloc.3", Arg::None);
    t[0x0a] = ("stloc.0", Arg::None);
    t[0x0b] = ("stloc.1", Arg::None);
    t[0x0c] = ("stloc.2", Arg::None);
    t[0x0d] = ("stloc.3", Arg::None);
    t[0x0e] = ("ldarg.s", Arg::Var1);
    t[0x0f] = ("ldarga.s", Arg::Var1);
    t[0x10] = ("starg.s", Arg::Var1);
    t[0x11] = ("ldloc.s", Arg::Var1);
    t[0x12] = ("ldloca.s", Arg::Var1);
    t[0x13] = ("stloc.s", Arg::Var1);
    t[0x14] = ("ldnull", Arg::None);
    t[0x15] = ("ldc.i4.m1", Arg::None);
    t[0x16] = ("ldc.i4.0", Arg::None);
    t[0x17] = ("ldc.i4.1", Arg::None);
    t[0x18] = ("ldc.i4.2", Arg::None);
    t[0x19] = ("ldc.i4.3", Arg::None);
    t[0x1a] = ("ldc.i4.4", Arg::None);
    t[0x1b] = ("ldc.i4.5", Arg::None);
    t[0x1c] = ("ldc.i4.6", Arg::None);
    t[0x1d] = ("ldc.i4.7", Arg::None);
    t[0x1e] = ("ldc.i4.8", Arg::None);
    t[0x1f] = ("ldc.i4.s", Arg::I1);
    t[0x20] = ("ldc.i4", Arg::I4);
    t[0x21] = ("ldc.i8", Arg::I8);
    t[0x22] = ("ldc.r4", Arg::R4);
    t[0x23] = ("ldc.r8", Arg::R8);
    t[0x25] = ("dup", Arg::None);
    t[0x26] = ("pop", Arg::None);
    t[0x27] = ("jmp", Arg::Token);
    t[0x28] = ("call", Arg::Token);
    t[0x29] = ("calli", Arg::Token);
    t[0x2a] = ("ret", Arg::None);
    t[0x2b] = ("br.s", Arg::Branch1);
    t[0x2c] = ("brfalse.s", Arg::Branch1);
    t[0x2d] = ("brtrue.s", Arg::Branch1);
    t[0x2e] = ("beq.s", Arg::Branch1);
    t[0x2f] = ("bge.s", Arg::Branch1);
    t[0x30] = ("bgt.s", Arg::Branch1);
    t[0x31] = ("ble.s", Arg::Branch1);
    t[0x32] = ("blt.s", Arg::Branch1);
    t[0x33] = ("bne.un.s", Arg::Branch1);
    t[0x34] = ("bge.un.s", Arg::Branch1);
    t[0x35] = ("bgt.un.s", Arg::Branch1);
    t[0x36] = ("ble.un.s", Arg::Branch1);
    t[0x37] = ("blt.un.s", Arg::Branch1);
    t[0x38] = ("br", Arg::Branch4);
    t[0x39] = ("brfalse", Arg::Branch4);
    t[0x3a] = ("brtrue", Arg::Branch4);
    t[0x3b] = ("beq", Arg::Branch4);
    t[0x3c] = ("bge", Arg::Branch4);
    t[0x3d] = ("bgt", Arg::Branch4);
    t[0x3e] = ("ble", Arg::Branch4);
    t[0x3f] = ("blt", Arg::Branch4);
    t[0x40] = ("bne.un", Arg::Branch4);
    t[0x41] = ("bge.un", Arg::Branch4);
    t[0x42] = ("bgt.un", Arg::Branch4);
    t[0x43] = ("ble.un", Arg::Branch4);
    t[0x44] = ("blt.un", Arg::Branch4);
    t[0x45] = ("switch", Arg::Switch);
    t[0x46] = ("ldind.i1", Arg::None);
    t[0x47] = ("ldind.u1", Arg::None);
    t[0x48] = ("ldind.i2", Arg::None);
    t[0x49] = ("ldind.u2", Arg::None);
    t[0x4a] = ("ldind.i4", Arg::None);
    t[0x4b] = ("ldind.u4", Arg::None);
    t[0x4c] = ("ldind.i8", Arg::None);
    t[0x4d] = ("ldind.i", Arg::None);
    t[0x4e] = ("ldind.r4", Arg::None);
    t[0x4f] = ("ldind.r8", Arg::None);
    t[0x50] = ("ldind.ref", Arg::None);
    t[0x51] = ("stind.ref", Arg::None);
    t[0x52] = ("stind.i1", Arg::None);
    t[0x53] = ("stind.i2", Arg::None);
    t[0x54] = ("stind.i4", Arg::None);
    t[0x55] = ("stind.i8", Arg::None);
    t[0x56] = ("stind.r4", Arg::None);
    t[0x57] = ("stind.r8", Arg::None);
    t[0x58] = ("add", Arg::None);
    t[0x59] = ("sub", Arg::None);
    t[0x5a] = ("mul", Arg::None);
    t[0x5b] = ("div", Arg::None);
    t[0x5c] = ("div.un", Arg::None);
    t[0x5d] = ("rem", Arg::None);
    t[0x5e] = ("rem.un", Arg::None);
    t[0x5f] = ("and", Arg::None);
    t[0x60] = ("or", Arg::None);
    t[0x61] = ("xor", Arg::None);
    t[0x62] = ("shl", Arg::None);
    t[0x63] = ("shr", Arg::None);
    t[0x64] = ("shr.un", Arg::None);
    t[0x65] = ("neg", Arg::None);
    t[0x66] = ("not", Arg::None);
    t[0x67] = ("conv.i1", Arg::None);
    t[0x68] = ("conv.i2", Arg::None);
    t[0x69] = ("conv.i4", Arg::None);
    t[0x6a] = ("conv.i8", Arg::None);
    t[0x6b] = ("conv.r4", Arg::None);
    t[0x6c] = ("conv.r8", Arg::None);
    t[0x6d] = ("conv.u4", Arg::None);
    t[0x6e] = ("conv.u8", Arg::None);
    t[0x6f] = ("callvirt", Arg::Token);
    t[0x70] = ("cpobj", Arg::Token);
    t[0x71] = ("ldobj", Arg::Token);
    t[0x72] = ("ldstr", Arg::Token);
    t[0x73] = ("newobj", Arg::Token);
    t[0x74] = ("castclass", Arg::Token);
    t[0x75] = ("isinst", Arg::Token);
    t[0x76] = ("conv.r.un", Arg::None);
    t[0x79] = ("unbox", Arg::Token);
    t[0x7a] = ("throw", Arg::None);
    t[0x7b] = ("ldfld", Arg::Token);
    t[0x7c] = ("ldflda", Arg::Token);
    t[0x7d] = ("stfld", Arg::Token);
    t[0x7e] = ("ldsfld", Arg::Token);
    t[0x7f] = ("ldsflda", Arg::Token);
    t[0x80] = ("stsfld", Arg::Token);
    t[0x81] = ("stobj", Arg::Token);
    t[0x82] = ("conv.ovf.i1.un", Arg::None);
    t[0x83] = ("conv.ovf.i2.un", Arg::None);
    t[0x84] = ("conv.ovf.i4.un", Arg::None);
    t[0x85] = ("conv.ovf.i8.un", Arg::None);
    t[0x86] = ("conv.ovf.u1.un", Arg::None);
    t[0x87] = ("conv.ovf.u2.un", Arg::None);
    t[0x88] = ("conv.ovf.u4.un", Arg::None);
    t[0x89] = ("conv.ovf.u8.un", Arg::None);
    t[0x8a] = ("conv.ovf.i.un", Arg::None);
    t[0x8b] = ("conv.ovf.u.un", Arg::None);
    t[0x8c] = ("box", Arg::Token);
    t[0x8d] = ("newarr", Arg::Token);
    t[0x8e] = ("ldlen", Arg::None);
    t[0x8f] = ("ldelema", Arg::Token);
    t[0x90] = ("ldelem.i1", Arg::None);
    t[0x91] = ("ldelem.u1", Arg::None);
    t[0x92] = ("ldelem.i2", Arg::None);
    t[0x93] = ("ldelem.u2", Arg::None);
    t[0x94] = ("ldelem.i4", Arg::None);
    t[0x95] = ("ldelem.u4", Arg::None);
    t[0x96] = ("ldelem.i8", Arg::None);
    t[0x97] = ("ldelem.i", Arg::None);
    t[0x98] = ("ldelem.r4", Arg::None);
    t[0x99] = ("ldelem.r8", Arg::None);
    t[0x9a] = ("ldelem.ref", Arg::None);
    t[0x9b] = ("stelem.i", Arg::None);
    t[0x9c] = ("stelem.i1", Arg::None);
    t[0x9d] = ("stelem.i2", Arg::None);
    t[0x9e] = ("stelem.i4", Arg::None);
    t[0x9f] = ("stelem.i8", Arg::None);
    t[0xa0] = ("stelem.r4", Arg::None);
    t[0xa1] = ("stelem.r8", Arg::None);
    t[0xa2] = ("stelem.ref", Arg::None);
    t[0xa3] = ("ldelem", Arg::Token);
    t[0xa4] = ("stelem", Arg::Token);
    t[0xa5] = ("unbox.any", Arg::Token);
    t[0xb3] = ("conv.ovf.i1", Arg::None);
    t[0xb4] = ("conv.ovf.u1", Arg::None);
    t[0xb5] = ("conv.ovf.i2", Arg::None);
    t[0xb6] = ("conv.ovf.u2", Arg::None);
    t[0xb7] = ("conv.ovf.i4", Arg::None);
    t[0xb8] = ("conv.ovf.u4", Arg::None);
    t[0xb9] = ("conv.ovf.i8", Arg::None);
    t[0xba] = ("conv.ovf.u8", Arg::None);
    t[0xc2] = ("refanyval", Arg::Token);
    t[0xc3] = ("ckfinite", Arg::None);
    t[0xc6] = ("mkrefany", Arg::Token);
    t[0xd0] = ("ldtoken", Arg::Token);
    t[0xd1] = ("conv.u2", Arg::None);
    t[0xd2] = ("conv.u1", Arg::None);
    t[0xd3] = ("conv.i", Arg::None);
    t[0xd4] = ("conv.ovf.i", Arg::None);
    t[0xd5] = ("conv.ovf.u", Arg::None);
    t[0xd6] = ("add.ovf", Arg::None);
    t[0xd7] = ("add.ovf.un", Arg::None);
    t[0xd8] = ("mul.ovf", Arg::None);
    t[0xd9] = ("mul.ovf.un", Arg::None);
    t[0xda] = ("sub.ovf", Arg::None);
    t[0xdb] = ("sub.ovf.un", Arg::None);
    t[0xdc] = ("endfinally", Arg::None);
    t[0xdd] = ("leave", Arg::Branch4);
    t[0xde] = ("leave.s", Arg::Branch1);
    t[0xdf] = ("stind.i", Arg::None);
    t[0xe0] = ("conv.u", Arg::None);
    t
};

/// The two-byte opcodes, after the `0xFE` prefix.
const TWO: [(&str, Arg); 32] = {
    let mut t = [("", Arg::Unknown); 32];
    t[0x00] = ("arglist", Arg::None);
    t[0x01] = ("ceq", Arg::None);
    t[0x02] = ("cgt", Arg::None);
    t[0x03] = ("cgt.un", Arg::None);
    t[0x04] = ("clt", Arg::None);
    t[0x05] = ("clt.un", Arg::None);
    t[0x06] = ("ldftn", Arg::Token);
    t[0x07] = ("ldvirtftn", Arg::Token);
    t[0x09] = ("ldarg", Arg::Var2);
    t[0x0a] = ("ldarga", Arg::Var2);
    t[0x0b] = ("starg", Arg::Var2);
    t[0x0c] = ("ldloc", Arg::Var2);
    t[0x0d] = ("ldloca", Arg::Var2);
    t[0x0e] = ("stloc", Arg::Var2);
    t[0x0f] = ("localloc", Arg::None);
    t[0x11] = ("endfilter", Arg::None);
    t[0x12] = ("unaligned.", Arg::U1);
    t[0x13] = ("volatile.", Arg::None);
    t[0x14] = ("tail.", Arg::None);
    t[0x15] = ("initobj", Arg::Token);
    t[0x16] = ("constrained.", Arg::Token);
    t[0x17] = ("cpblk", Arg::None);
    t[0x18] = ("initblk", Arg::None);
    t[0x19] = ("no.", Arg::U1);
    t[0x1a] = ("rethrow", Arg::None);
    t[0x1c] = ("sizeof", Arg::Token);
    t[0x1d] = ("refanytype", Arg::None);
    t[0x1e] = ("readonly.", Arg::None);
    t
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub offset: usize,
    pub len: usize,
    pub text: String,
}

/// Decode IL. `assembly` resolves tokens to names; pass `None` to print them as `Table(row)`.
pub fn disassemble(code: &[u8], assembly: Option<&Assembly>) -> Vec<Instruction> {
    let mut out = Vec::new();
    let mut at = 0usize;

    while at < code.len() {
        let start = at;
        let first = code[at];
        at += 1;
        let (name, arg) = if first == 0xFE {
            let second = code.get(at).copied().unwrap_or(0);
            at += 1;
            TWO.get(second as usize)
                .copied()
                .unwrap_or(("", Arg::Unknown))
        } else {
            ONE[first as usize]
        };

        let mut text = if name.is_empty() {
            format!("unknown opcode 0x{first:02X}")
        } else {
            name.to_string()
        };

        macro_rules! read {
            ($n:expr) => {{
                let end = (at + $n).min(code.len());
                let slice = &code[at.min(code.len())..end];
                at += $n;
                slice.to_vec()
            }};
        }

        match arg {
            Arg::None | Arg::Unknown => {}
            Arg::U1 => {
                let v = read!(1);
                let _ = write!(text, " {}", v.first().copied().unwrap_or(0));
            }
            Arg::I1 => {
                let v = read!(1);
                let _ = write!(text, " {}", v.first().copied().unwrap_or(0) as i8);
            }
            Arg::Var1 => {
                let v = read!(1);
                let _ = write!(text, " {}", v.first().copied().unwrap_or(0));
            }
            Arg::Var2 => {
                let v = read!(2);
                let _ = write!(text, " {}", le16(&v));
            }
            Arg::I4 => {
                let v = read!(4);
                let _ = write!(text, " {}", le32(&v) as i32);
            }
            Arg::I8 => {
                let v = read!(8);
                let _ = write!(text, " {}", le64(&v) as i64);
            }
            Arg::R4 => {
                let v = read!(4);
                let _ = write!(text, " {}", f32::from_bits(le32(&v)));
            }
            Arg::R8 => {
                let v = read!(8);
                let _ = write!(text, " {}", f64::from_bits(le64(&v)));
            }
            Arg::Branch1 => {
                let v = read!(1);
                let delta = v.first().copied().unwrap_or(0) as i8 as isize;
                let _ = write!(text, " IL_{:04X}", (at as isize + delta).max(0));
            }
            Arg::Branch4 => {
                let v = read!(4);
                let delta = le32(&v) as i32 as isize;
                let _ = write!(text, " IL_{:04X}", (at as isize + delta).max(0));
            }
            Arg::Token => {
                let v = read!(4);
                let token = le32(&v);
                match assembly {
                    Some(assembly) => {
                        let _ = write!(text, " {}", assembly.token(token));
                    }
                    None => {
                        let _ = write!(text, " 0x{token:08X}");
                    }
                }
            }
            Arg::Switch => {
                let v = read!(4);
                let count = le32(&v) as usize;
                let mut targets = Vec::new();
                for _ in 0..count.min(4096) {
                    let v = read!(4);
                    targets.push(le32(&v) as i32);
                }
                let base = at as isize;
                let list: Vec<String> = targets
                    .iter()
                    .map(|d| format!("IL_{:04X}", (base + *d as isize).max(0)))
                    .collect();
                let _ = write!(text, " ({})", list.join(", "));
            }
        }

        out.push(Instruction {
            offset: start,
            len: at.min(code.len()).saturating_sub(start).max(1),
            text,
        });
    }
    out
}

fn le16(bytes: &[u8]) -> u16 {
    u16::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
    ])
}

fn le32(bytes: &[u8]) -> u32 {
    u32::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
        bytes.get(2).copied().unwrap_or(0),
        bytes.get(3).copied().unwrap_or(0),
    ])
}

fn le64(bytes: &[u8]) -> u64 {
    let mut value = [0u8; 8];
    for (i, slot) in value.iter_mut().enumerate() {
        *slot = bytes.get(i).copied().unwrap_or(0);
    }
    u64::from_le_bytes(value)
}

/// Method flags, as `ildasm` would show them.
pub fn method_flags(flags: u16) -> String {
    let mut parts = Vec::new();
    match flags & 0x0007 {
        0x0001 => parts.push("private"),
        0x0002 => parts.push("famandassem"),
        0x0003 => parts.push("assembly"),
        0x0004 => parts.push("family"),
        0x0005 => parts.push("famorassem"),
        0x0006 => parts.push("public"),
        _ => {}
    }
    if flags & 0x0010 != 0 {
        parts.push("static");
    }
    if flags & 0x0020 != 0 {
        parts.push("final");
    }
    if flags & 0x0040 != 0 {
        parts.push("virtual");
    }
    if flags & 0x0400 != 0 {
        parts.push("abstract");
    }
    if flags & 0x0800 != 0 {
        parts.push("specialname");
    }
    if flags & 0x2000 != 0 {
        parts.push("pinvoke");
    }
    parts.join(" ")
}

/// Is this PE a managed (.NET) image? Checked by looking for the CLI data directory.
pub fn is_managed(bytes: &[u8]) -> bool {
    cli_metadata(bytes).is_ok()
}

/// The metadata region of a .NET assembly, found through the PE headers.
fn cli_metadata(bytes: &[u8]) -> Result<(Vec<u8>, SectionMap), IlError> {
    fn extract<Pe: object::read::pe::ImageNtHeaders>(
        bytes: &[u8],
    ) -> Result<(Vec<u8>, SectionMap), IlError> {
        let file = object::read::pe::PeFile::<Pe>::parse(bytes).map_err(|_| IlError::NotManaged)?;
        let directory = file
            .data_directory(object::pe::IMAGE_DIRECTORY_ENTRY_COM_DESCRIPTOR)
            .ok_or(IlError::NotManaged)?;
        let sections = file.section_table();
        let cli = directory
            .data(bytes, &sections)
            .map_err(|_| IlError::NotManaged)?;
        // CLI header (ECMA-335 §II.25.3.3): the MetaData directory is at offset 8.
        let rva = u32_at(cli, 8)?;
        let size = u32_at(cli, 12)? as usize;
        let map = SectionMap::new(&sections);
        let start = map
            .offset(rva)
            .ok_or(IlError::Bad("metadata outside the sections"))?;
        let metadata = bytes
            .get(start..(start + size).min(bytes.len()))
            .ok_or(IlError::Truncated(start))?
            .to_vec();
        Ok((metadata, map))
    }

    if bytes.len() < 0x40 {
        return Err(IlError::NotManaged);
    }
    extract::<object::pe::ImageNtHeaders64>(bytes)
        .or_else(|_| extract::<object::pe::ImageNtHeaders32>(bytes))
}

/// RVA → file offset, from the PE section table.
#[derive(Debug, Clone, Default)]
pub struct SectionMap {
    /// `(virtual_address, virtual_size, raw_pointer)`
    ranges: Vec<(u32, u32, u32)>,
}

impl SectionMap {
    fn new(sections: &object::read::pe::SectionTable<'_>) -> SectionMap {
        use object::LittleEndian as LE;
        let ranges = sections
            .iter()
            .map(|section| {
                (
                    section.virtual_address.get(LE),
                    section
                        .virtual_size
                        .get(LE)
                        .max(section.size_of_raw_data.get(LE)),
                    section.pointer_to_raw_data.get(LE),
                )
            })
            .collect();
        SectionMap { ranges }
    }

    /// For tests and for callers that already know the layout.
    pub fn identity() -> SectionMap {
        SectionMap {
            ranges: vec![(0, u32::MAX, 0)],
        }
    }

    pub fn offset(&self, rva: u32) -> Option<usize> {
        for (address, size, raw) in &self.ranges {
            if rva >= *address && rva < address.saturating_add(*size) {
                return Some((raw + (rva - address)) as usize);
            }
        }
        None
    }
}

/// An `ildasm`-flavoured listing of a .NET assembly: what `F5` shows for a managed PE.
pub fn listing(bytes: &[u8]) -> Result<String, IlError> {
    let (metadata, map) = cli_metadata(bytes)?;
    let assembly = parse_metadata(&metadata)?;
    Ok(render(bytes, &assembly, &map))
}

/// Render a parsed assembly; split out so a test can drive it without a PE.
pub fn render(bytes: &[u8], assembly: &Assembly, map: &SectionMap) -> String {
    let mut out = String::with_capacity(4096);
    let _ = writeln!(
        out,
        ".NET assembly, runtime {}, {} methods",
        assembly.runtime_version,
        assembly.methods.len()
    );
    let _ = writeln!(
        out,
        "// IL as decoded in process; for C# use a [[decompile]] rule with ilspycmd"
    );
    let _ = writeln!(out);

    let mut current_type = String::new();
    for method in &assembly.methods {
        if method.type_name != current_type {
            current_type = method.type_name.clone();
            let _ = writeln!(out, ".class {current_type}");
        }
        let flags = method_flags(method.flags);
        let _ = writeln!(out, "  .method {flags} {}()", method.name);
        if method.rva == 0 {
            let _ = writeln!(out, "    // no body (abstract, extern or pinvoke)");
            let _ = writeln!(out);
            continue;
        }
        let Some(offset) = map.offset(method.rva) else {
            let _ = writeln!(
                out,
                "    // body at RVA {:#x} is outside the file",
                method.rva
            );
            let _ = writeln!(out);
            continue;
        };
        match parse_body(bytes, offset) {
            Ok(body) => {
                let _ = writeln!(
                    out,
                    "    // maxstack {}, {} bytes of IL{}",
                    body.max_stack,
                    body.code.len(),
                    if body.fat { ", fat header" } else { "" }
                );
                for instruction in disassemble(&body.code, Some(assembly)) {
                    let _ = writeln!(
                        out,
                        "    IL_{:04X}:  {}",
                        instruction.offset, instruction.text
                    );
                }
            }
            Err(e) => {
                let _ = writeln!(out, "    // unreadable body: {e}");
            }
        }
        let _ = writeln!(out);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn il_opcodes_decode_with_their_operands() {
        // ldarg.0; ldc.i4.s 42; add; ldstr <token>; call <token>; ret
        let code = [
            0x02, 0x1f, 0x2a, 0x58, 0x72, 0x01, 0x00, 0x00, 0x70, 0x28, 0x03, 0x00, 0x00, 0x0a,
            0x2a,
        ];
        let instructions = disassemble(&code, None);
        let text: Vec<&str> = instructions.iter().map(|i| i.text.as_str()).collect();
        assert_eq!(text[0], "ldarg.0");
        assert_eq!(text[1], "ldc.i4.s 42");
        assert_eq!(text[2], "add");
        assert_eq!(text[3], "ldstr 0x70000001");
        assert_eq!(text[4], "call 0x0A000003");
        assert_eq!(text[5], "ret");
        // Offsets and lengths must tile the code exactly.
        let mut expected = 0;
        for instruction in &instructions {
            assert_eq!(instruction.offset, expected);
            expected += instruction.len;
        }
        assert_eq!(expected, code.len());
    }

    #[test]
    fn two_byte_opcodes_and_prefixes_decode() {
        // fe 01 ceq; fe 09 ldarg 300; fe 16 constrained. <token>; fe 1a rethrow
        let code = [
            0xfe, 0x01, 0xfe, 0x09, 0x2c, 0x01, 0xfe, 0x16, 0x01, 0x00, 0x00, 0x1b, 0xfe, 0x1a,
        ];
        let text: Vec<String> = disassemble(&code, None)
            .into_iter()
            .map(|i| i.text)
            .collect();
        assert_eq!(text[0], "ceq");
        assert_eq!(text[1], "ldarg 300");
        assert_eq!(text[2], "constrained. 0x1B000001");
        assert_eq!(text[3], "rethrow");
    }

    #[test]
    fn branches_print_absolute_il_offsets() {
        // br.s +2 (from offset 2) then nop nop ret
        let code = [0x2b, 0x02, 0x00, 0x00, 0x2a];
        let instructions = disassemble(&code, None);
        assert_eq!(instructions[0].text, "br.s IL_0004");
        // switch with two targets
        let mut code = vec![0x45, 0x02, 0x00, 0x00, 0x00];
        code.extend_from_slice(&4i32.to_le_bytes());
        code.extend_from_slice(&8i32.to_le_bytes());
        code.push(0x2a);
        let instructions = disassemble(&code, None);
        assert!(
            instructions[0]
                .text
                .starts_with("switch (IL_0011, IL_0015)"),
            "{}",
            instructions[0].text
        );
    }

    #[test]
    fn a_tiny_method_body_header_is_read() {
        // (6 << 2) | 2 = 0x1a: six bytes of IL
        let bytes = [0x1a, 0x02, 0x03, 0x58, 0x0a, 0x06, 0x2a];
        let body = parse_body(&bytes, 0).unwrap();
        assert!(!body.fat);
        assert_eq!(body.max_stack, 8, "a tiny header implies maxstack 8");
        assert_eq!(body.code.len(), 6);
        assert_eq!(disassemble(&body.code, None)[0].text, "ldarg.0");
    }

    #[test]
    fn a_fat_method_body_header_is_read() {
        let mut bytes = vec![0u8; 12];
        bytes[0] = 0x33; // flags low byte: fat
        bytes[1] = 0x30; // header size 3 dwords
        bytes[2..4].copy_from_slice(&7u16.to_le_bytes()); // maxstack
        bytes[4..8].copy_from_slice(&2u32.to_le_bytes()); // code size
        bytes[8..12].copy_from_slice(&0x1100_0001u32.to_le_bytes()); // locals signature
        bytes.extend_from_slice(&[0x16, 0x2a]); // ldc.i4.0; ret
        let body = parse_body(&bytes, 0).unwrap();
        assert!(body.fat);
        assert_eq!(body.max_stack, 7);
        assert_eq!(body.local_signature, 0x1100_0001);
        let text: Vec<String> = disassemble(&body.code, None)
            .into_iter()
            .map(|i| i.text)
            .collect();
        assert_eq!(text, vec!["ldc.i4.0", "ret"]);
    }

    #[test]
    fn a_bad_body_header_is_refused() {
        assert!(parse_body(&[0x00, 0x00], 0).is_err());
        assert!(parse_body(&[], 0).is_err());
    }

    /// Build a metadata blob with one type, one method and one user string, so the table
    /// walker is exercised without needing a .NET SDK.
    fn metadata_blob() -> Vec<u8> {
        // #Strings: "" "Program" "Main" "Console"
        let mut strings = vec![0u8];
        let off_program = strings.len();
        strings.extend_from_slice(b"Program\0");
        let off_main = strings.len();
        strings.extend_from_slice(b"Main\0");
        let off_console = strings.len();
        strings.extend_from_slice(b"WriteLine\0");

        // #US: a single UTF-16 literal, length-prefixed, at offset 1
        let mut user = vec![0u8];
        let literal: Vec<u8> = "hi"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .chain(std::iter::once(0))
            .collect();
        user.push(literal.len() as u8);
        user.extend_from_slice(&literal);

        // #~ tables: Module(0x00), TypeDef(0x02), MethodDef(0x06), MemberRef(0x0A)
        let mut tables = Vec::new();
        tables.extend_from_slice(&0u32.to_le_bytes()); // reserved
        tables.push(2); // major
        tables.push(0); // minor
        tables.push(0); // heap sizes: all 2-byte indexes
        tables.push(0); // reserved
        let valid: u64 = (1 << 0x00) | (1 << 0x02) | (1 << 0x06) | (1 << 0x0A);
        tables.extend_from_slice(&valid.to_le_bytes());
        tables.extend_from_slice(&0u64.to_le_bytes()); // sorted
        for count in [1u32, 1, 1, 1] {
            tables.extend_from_slice(&count.to_le_bytes()); // rows per present table
        }

        // Module: generation(2) name(S) mvid(G) encid(G) encbaseid(G)
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&[0u8; 6]);
        // TypeDef: flags(4) name(S) namespace(S) extends(coded) fieldlist(2) methodlist(2)
        tables.extend_from_slice(&0u32.to_le_bytes());
        tables.extend_from_slice(&(off_program as u16).to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&1u16.to_le_bytes());
        tables.extend_from_slice(&1u16.to_le_bytes());
        // MethodDef: rva(4) implflags(2) flags(2) name(S) signature(B) paramlist(2)
        tables.extend_from_slice(&0x1000u32.to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&0x0016u16.to_le_bytes()); // public static
        tables.extend_from_slice(&(off_main as u16).to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&1u16.to_le_bytes());
        // MemberRef: class(coded 3 bits) name(S) signature(B)
        tables.extend_from_slice(&0u16.to_le_bytes());
        tables.extend_from_slice(&(off_console as u16).to_le_bytes());
        tables.extend_from_slice(&0u16.to_le_bytes());

        // Metadata root
        let version = b"v4.0.30319\0\0";
        let mut out = Vec::new();
        out.extend_from_slice(b"BSJB");
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&(version.len() as u32).to_le_bytes());
        out.extend_from_slice(version);
        out.extend_from_slice(&0u16.to_le_bytes()); // flags
        out.extend_from_slice(&3u16.to_le_bytes()); // three streams

        let header_len = out.len() + 3 * 8 + 4 + 12 + 4; // stream headers: "#~", "#Strings", "#US"
        let tables_offset = header_len.next_multiple_of(4);
        let strings_offset = tables_offset + tables.len();
        let user_offset = strings_offset + strings.len();

        out.extend_from_slice(&(tables_offset as u32).to_le_bytes());
        out.extend_from_slice(&(tables.len() as u32).to_le_bytes());
        out.extend_from_slice(b"#~\0\0");
        out.extend_from_slice(&(strings_offset as u32).to_le_bytes());
        out.extend_from_slice(&(strings.len() as u32).to_le_bytes());
        out.extend_from_slice(b"#Strings\0\0\0\0");
        out.extend_from_slice(&(user_offset as u32).to_le_bytes());
        out.extend_from_slice(&(user.len() as u32).to_le_bytes());
        out.extend_from_slice(b"#US\0");
        while out.len() < tables_offset {
            out.push(0);
        }
        out.extend_from_slice(&tables);
        out.extend_from_slice(&strings);
        out.extend_from_slice(&user);
        out
    }

    #[test]
    fn metadata_tables_and_heaps_are_walked() {
        let assembly = parse_metadata(&metadata_blob()).expect("parses");
        assert_eq!(assembly.runtime_version, "v4.0.30319");
        assert_eq!(assembly.methods.len(), 1);
        let method = &assembly.methods[0];
        assert_eq!(method.name, "Main");
        assert_eq!(method.type_name, "Program");
        assert_eq!(method.rva, 0x1000);
        assert_eq!(method_flags(method.flags), "public static");
    }

    #[test]
    fn tokens_resolve_to_names_and_literals() {
        let assembly = parse_metadata(&metadata_blob()).unwrap();
        assert_eq!(assembly.token(0x0600_0001), "Main");
        assert_eq!(assembly.token(0x0200_0001), "Program");
        assert_eq!(assembly.token(0x0A00_0001), "WriteLine");
        assert_eq!(assembly.token(0x7000_0001), "\"hi\"");
        // Anything we cannot name says what it is, instead of inventing a name.
        assert_eq!(assembly.token(0x0400_0009), "Field(9)");
        assert_eq!(assembly.token(0x0600_0099), "MethodDef(153)");
    }

    #[test]
    fn a_listing_names_the_type_the_method_and_its_il() {
        let assembly = parse_metadata(&metadata_blob()).unwrap();
        // A body at "RVA" 0x1000 with an identity section map.
        let mut image = vec![0u8; 0x1000];
        image.push((11 << 2) | 2); // tiny header: eleven bytes of IL follow
        image.extend_from_slice(&[0x72, 0x01, 0x00, 0x00, 0x70]); // ldstr "hi"
        image.extend_from_slice(&[0x28, 0x01, 0x00, 0x00, 0x0a]); // call WriteLine
        image.push(0x2a); // ret
        let text = render(&image, &assembly, &SectionMap::identity());

        assert!(text.contains(".NET assembly, runtime v4.0.30319"), "{text}");
        assert!(text.contains(".class Program"), "{text}");
        assert!(text.contains(".method public static Main()"), "{text}");
        assert!(text.contains("IL_0000:  ldstr \"hi\""), "{text}");
        assert!(text.contains("call WriteLine"), "{text}");
        assert!(text.contains("ret"), "{text}");
        assert!(
            text.contains("ilspycmd"),
            "the listing should point at a decompiler"
        );
    }

    #[test]
    fn a_method_without_a_body_says_so() {
        let mut assembly = parse_metadata(&metadata_blob()).unwrap();
        assembly.methods[0].rva = 0;
        let text = render(&[], &assembly, &SectionMap::identity());
        assert!(
            text.contains("no body (abstract, extern or pinvoke)"),
            "{text}"
        );
    }

    #[test]
    fn a_native_binary_is_not_managed() {
        let bytes = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        assert!(!is_managed(&bytes));
        assert!(matches!(listing(&bytes), Err(IlError::NotManaged)));
        assert!(matches!(parse_metadata(b"nope"), Err(IlError::Bad(_))));
    }

    #[test]
    fn compressed_integers_decode_in_all_three_widths() {
        assert_eq!(compressed_u32(&[0x03], 0), Some((3, 1)));
        assert_eq!(compressed_u32(&[0x80 | 0x01, 0x00], 0), Some((256, 2)));
        assert_eq!(
            compressed_u32(&[0xC0, 0x00, 0x40, 0x00], 0),
            Some((0x4000, 4))
        );
        assert_eq!(compressed_u32(&[], 0), None);
    }
}
