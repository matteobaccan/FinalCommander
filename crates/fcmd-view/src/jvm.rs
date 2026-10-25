//! Java class files: the constant pool, the methods, and their bytecode.
//!
//! Written here rather than pulled in as a dependency because the format is small, stable and
//! completely specified (JVMS §4), and because the interesting part — turning the code array
//! into readable instructions with the constant pool resolved — is work no parser crate does
//! for us. The output is deliberately close to `javap -c`, so anyone who knows that tool can
//! read ours.

use std::fmt::Write as _;

#[derive(Debug, thiserror::Error)]
pub enum ClassError {
    #[error("not a class file")]
    NotAClass,
    #[error("truncated class file at byte {0}")]
    Truncated(usize),
    #[error("unsupported constant pool tag {0}")]
    BadTag(u8),
}

/// The entries of the constant pool we need in order to print instructions.
#[derive(Debug, Clone, PartialEq)]
enum Constant {
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    String(u16),
    /// field, method or interface-method reference: `(class, name_and_type)`
    Ref(u16, u16),
    NameAndType(u16, u16),
    MethodHandle(u8, u16),
    MethodType(u16),
    Dynamic(u16, u16),
    Module(u16),
    Package(u16),
    /// The unused slot after a long or a double.
    Gap,
}

pub struct Class {
    pool: Vec<Constant>,
    pub major: u16,
    pub minor: u16,
    pub access_flags: u16,
    pub this_class: String,
    pub super_class: String,
    pub interfaces: Vec<String>,
    pub fields: Vec<Member>,
    pub methods: Vec<Member>,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Member {
    pub name: String,
    pub descriptor: String,
    pub access_flags: u16,
    /// Present for methods that are neither abstract nor native.
    pub code: Option<Code>,
}

#[derive(Debug, Clone)]
pub struct Code {
    pub max_stack: u16,
    pub max_locals: u16,
    pub bytes: Vec<u8>,
    pub exceptions: Vec<Handler>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handler {
    pub start: u16,
    pub end: u16,
    pub handler: u16,
    pub catch_type: u16,
}

/// A byte reader that never panics and says where it ran out.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Reader { bytes, at: 0 }
    }

    fn u1(&mut self) -> Result<u8, ClassError> {
        let b = *self
            .bytes
            .get(self.at)
            .ok_or(ClassError::Truncated(self.at))?;
        self.at += 1;
        Ok(b)
    }

    fn u2(&mut self) -> Result<u16, ClassError> {
        Ok(u16::from_be_bytes([self.u1()?, self.u1()?]))
    }

    fn u4(&mut self) -> Result<u32, ClassError> {
        Ok(u32::from_be_bytes([
            self.u1()?,
            self.u1()?,
            self.u1()?,
            self.u1()?,
        ]))
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], ClassError> {
        let end = self
            .at
            .checked_add(len)
            .ok_or(ClassError::Truncated(self.at))?;
        let slice = self
            .bytes
            .get(self.at..end)
            .ok_or(ClassError::Truncated(self.at))?;
        self.at = end;
        Ok(slice)
    }
}

impl Class {
    pub fn parse(bytes: &[u8]) -> Result<Class, ClassError> {
        let mut r = Reader::new(bytes);
        if r.u4()? != 0xCAFE_BABE {
            return Err(ClassError::NotAClass);
        }
        let minor = r.u2()?;
        let major = r.u2()?;

        let count = r.u2()? as usize;
        let mut pool = Vec::with_capacity(count);
        pool.push(Constant::Gap); // the pool is 1-based
        let mut index = 1;
        while index < count {
            let tag = r.u1()?;
            let constant = match tag {
                1 => {
                    let len = r.u2()? as usize;
                    let raw = r.take(len)?;
                    Constant::Utf8(decode_modified_utf8(raw))
                }
                3 => Constant::Integer(r.u4()? as i32),
                4 => Constant::Float(f32::from_bits(r.u4()?)),
                5 => Constant::Long(((r.u4()? as u64) << 32 | r.u4()? as u64) as i64),
                6 => Constant::Double(f64::from_bits((r.u4()? as u64) << 32 | r.u4()? as u64)),
                7 => Constant::Class(r.u2()?),
                8 => Constant::String(r.u2()?),
                9..=11 => Constant::Ref(r.u2()?, r.u2()?),
                12 => Constant::NameAndType(r.u2()?, r.u2()?),
                15 => Constant::MethodHandle(r.u1()?, r.u2()?),
                16 => Constant::MethodType(r.u2()?),
                17 | 18 => Constant::Dynamic(r.u2()?, r.u2()?),
                19 => Constant::Module(r.u2()?),
                20 => Constant::Package(r.u2()?),
                other => return Err(ClassError::BadTag(other)),
            };
            let wide = matches!(constant, Constant::Long(_) | Constant::Double(_));
            pool.push(constant);
            index += 1;
            if wide {
                // JVMS §4.4.5: longs and doubles take two slots, "a poor choice".
                pool.push(Constant::Gap);
                index += 1;
            }
        }

        let access_flags = r.u2()?;
        let this_index = r.u2()?;
        let super_index = r.u2()?;

        let interface_count = r.u2()? as usize;
        let mut interface_indices = Vec::with_capacity(interface_count);
        for _ in 0..interface_count {
            interface_indices.push(r.u2()?);
        }

        let mut class = Class {
            pool,
            major,
            minor,
            access_flags,
            this_class: String::new(),
            super_class: String::new(),
            interfaces: Vec::new(),
            fields: Vec::new(),
            methods: Vec::new(),
            source_file: None,
        };
        class.this_class = class.class_name(this_index);
        class.super_class = if super_index == 0 {
            String::new()
        } else {
            class.class_name(super_index)
        };
        class.interfaces = interface_indices
            .into_iter()
            .map(|i| class.class_name(i))
            .collect();

        let field_count = r.u2()? as usize;
        for _ in 0..field_count {
            class.fields.push(class.read_member(&mut r, false)?);
        }
        let method_count = r.u2()? as usize;
        for _ in 0..method_count {
            class.methods.push(class.read_member(&mut r, true)?);
        }

        // Class attributes: we only care about SourceFile.
        let attr_count = r.u2().unwrap_or(0) as usize;
        for _ in 0..attr_count {
            let Ok(name_index) = r.u2() else { break };
            let Ok(len) = r.u4() else { break };
            let Ok(body) = r.take(len as usize) else {
                break;
            };
            if class.utf8(name_index) == "SourceFile" && body.len() >= 2 {
                let index = u16::from_be_bytes([body[0], body[1]]);
                class.source_file = Some(class.utf8(index));
            }
        }
        Ok(class)
    }

    fn read_member(&self, r: &mut Reader<'_>, method: bool) -> Result<Member, ClassError> {
        let access_flags = r.u2()?;
        let name = self.utf8(r.u2()?);
        let descriptor = self.utf8(r.u2()?);
        let attr_count = r.u2()? as usize;
        let mut code = None;
        for _ in 0..attr_count {
            let attr_name = self.utf8(r.u2()?);
            let len = r.u4()? as usize;
            let body = r.take(len)?;
            if method && attr_name == "Code" {
                code = parse_code(body).ok();
            }
        }
        Ok(Member {
            name,
            descriptor,
            access_flags,
            code,
        })
    }

    fn get(&self, index: u16) -> Option<&Constant> {
        self.pool.get(index as usize)
    }

    fn utf8(&self, index: u16) -> String {
        match self.get(index) {
            Some(Constant::Utf8(text)) => text.clone(),
            _ => format!("#{index}"),
        }
    }

    fn class_name(&self, index: u16) -> String {
        match self.get(index) {
            Some(Constant::Class(name)) => self.utf8(*name),
            _ => format!("#{index}"),
        }
    }

    /// How a constant pool entry reads in a comment next to an instruction.
    pub fn describe(&self, index: u16) -> String {
        match self.get(index) {
            Some(Constant::Utf8(text)) => format!("\"{}\"", escape(text)),
            Some(Constant::Integer(v)) => format!("int {v}"),
            Some(Constant::Float(v)) => format!("float {v}"),
            Some(Constant::Long(v)) => format!("long {v}"),
            Some(Constant::Double(v)) => format!("double {v}"),
            Some(Constant::Class(name)) => self.utf8(*name),
            Some(Constant::String(text)) => format!("String \"{}\"", escape(&self.utf8(*text))),
            Some(Constant::Ref(class, nat)) => {
                let (name, descriptor) = self.name_and_type(*nat);
                format!("{}.{}:{}", self.class_name(*class), name, descriptor)
            }
            Some(Constant::NameAndType(name, descriptor)) => {
                format!("{}:{}", self.utf8(*name), self.utf8(*descriptor))
            }
            Some(Constant::MethodHandle(kind, reference)) => {
                format!("MethodHandle kind {kind} {}", self.describe(*reference))
            }
            Some(Constant::MethodType(descriptor)) => {
                format!("MethodType {}", self.utf8(*descriptor))
            }
            Some(Constant::Dynamic(bootstrap, nat)) => {
                let (name, descriptor) = self.name_and_type(*nat);
                format!("dynamic[{bootstrap}] {name}:{descriptor}")
            }
            Some(Constant::Module(name)) => format!("module {}", self.utf8(*name)),
            Some(Constant::Package(name)) => format!("package {}", self.utf8(*name)),
            _ => format!("#{index}"),
        }
    }

    fn name_and_type(&self, index: u16) -> (String, String) {
        match self.get(index) {
            Some(Constant::NameAndType(name, descriptor)) => {
                (self.utf8(*name), self.utf8(*descriptor))
            }
            _ => (format!("#{index}"), String::new()),
        }
    }

    /// `java/lang/String` → `java.lang.String`, as a reader expects.
    pub fn pretty_class(&self) -> String {
        self.this_class.replace('/', ".")
    }

    pub fn java_version(&self) -> String {
        // 45 is Java 1.1; 52 is Java 8; 65 is Java 21.
        let java = match self.major {
            0..=44 => None,
            45 => Some("1.1".to_string()),
            46..=48 => Some(format!("1.{}", self.major - 44)),
            v => Some(format!("{}", v - 44)),
        };
        match java {
            Some(v) => format!("class file {}.{} (Java {v})", self.major, self.minor),
            None => format!("class file {}.{}", self.major, self.minor),
        }
    }
}

fn parse_code(body: &[u8]) -> Result<Code, ClassError> {
    let mut r = Reader::new(body);
    let max_stack = r.u2()?;
    let max_locals = r.u2()?;
    let len = r.u4()? as usize;
    let bytes = r.take(len)?.to_vec();
    let handler_count = r.u2()? as usize;
    let mut exceptions = Vec::with_capacity(handler_count);
    for _ in 0..handler_count {
        exceptions.push(Handler {
            start: r.u2()?,
            end: r.u2()?,
            handler: r.u2()?,
            catch_type: r.u2()?,
        });
    }
    Ok(Code {
        max_stack,
        max_locals,
        bytes,
        exceptions,
    })
}

/// JVMS §4.4.7: almost UTF-8, except for NUL and the surrogate encoding of astral characters.
fn decode_modified_utf8(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) if !bytes.contains(&0xC0) => text.to_string(),
        _ => {
            let mut out = String::with_capacity(bytes.len());
            let mut i = 0;
            while i < bytes.len() {
                let b = bytes[i];
                if b < 0x80 {
                    out.push(b as char);
                    i += 1;
                } else if b == 0xC0 && bytes.get(i + 1) == Some(&0x80) {
                    out.push('\0');
                    i += 2;
                } else {
                    // Fall back to lossy decoding for the rest of the run.
                    let rest = String::from_utf8_lossy(&bytes[i..]).into_owned();
                    out.push_str(&rest);
                    break;
                }
            }
            out
        }
    }
}

fn escape(text: &str) -> String {
    text.chars()
        .flat_map(|c| match c {
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect(),
            '\t' => "\\t".chars().collect(),
            '"' => "\\\"".chars().collect(),
            other => vec![other],
        })
        .collect()
}

/// Access flags, the way `javap` prints them.
pub fn access(flags: u16, is_class: bool) -> String {
    let mut parts = Vec::new();
    if flags & 0x0001 != 0 {
        parts.push("public");
    }
    if flags & 0x0002 != 0 {
        parts.push("private");
    }
    if flags & 0x0004 != 0 {
        parts.push("protected");
    }
    if flags & 0x0008 != 0 {
        parts.push("static");
    }
    if flags & 0x0010 != 0 {
        parts.push("final");
    }
    if is_class && flags & 0x0020 != 0 {
        parts.push("super");
    }
    if !is_class && flags & 0x0020 != 0 {
        parts.push("synchronized");
    }
    if flags & 0x0040 != 0 && !is_class {
        parts.push("volatile/bridge");
    }
    if flags & 0x0080 != 0 && !is_class {
        parts.push("transient/varargs");
    }
    if flags & 0x0100 != 0 {
        parts.push("native");
    }
    if flags & 0x0200 != 0 {
        parts.push("interface");
    }
    if flags & 0x0400 != 0 {
        parts.push("abstract");
    }
    if flags & 0x1000 != 0 {
        parts.push("synthetic");
    }
    if flags & 0x2000 != 0 {
        parts.push("annotation");
    }
    if flags & 0x4000 != 0 {
        parts.push("enum");
    }
    parts.join(" ")
}

/// What follows an opcode in the code array.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operands {
    None,
    /// unsigned byte (a local index, or `bipush`)
    U1,
    /// signed byte (`bipush`)
    S1,
    /// constant pool index, one byte (`ldc`)
    Cp1,
    /// constant pool index, two bytes
    Cp2,
    /// signed short (`sipush`)
    S2,
    /// branch offset, two bytes
    Branch2,
    /// branch offset, four bytes
    Branch4,
    IInc,
    NewArray,
    MultiANewArray,
    InvokeInterface,
    InvokeDynamic,
    Wide,
    TableSwitch,
    LookupSwitch,
    /// not a defined opcode
    Unknown,
}

/// The JVM instruction set (JVMS §6.5), by opcode.
const OPCODES: [(&str, Operands); 256] = {
    let mut table = [("", Operands::Unknown); 256];
    table[0x00] = ("nop", Operands::None);
    table[0x01] = ("aconst_null", Operands::None);
    table[0x02] = ("iconst_m1", Operands::None);
    table[0x03] = ("iconst_0", Operands::None);
    table[0x04] = ("iconst_1", Operands::None);
    table[0x05] = ("iconst_2", Operands::None);
    table[0x06] = ("iconst_3", Operands::None);
    table[0x07] = ("iconst_4", Operands::None);
    table[0x08] = ("iconst_5", Operands::None);
    table[0x09] = ("lconst_0", Operands::None);
    table[0x0a] = ("lconst_1", Operands::None);
    table[0x0b] = ("fconst_0", Operands::None);
    table[0x0c] = ("fconst_1", Operands::None);
    table[0x0d] = ("fconst_2", Operands::None);
    table[0x0e] = ("dconst_0", Operands::None);
    table[0x0f] = ("dconst_1", Operands::None);
    table[0x10] = ("bipush", Operands::S1);
    table[0x11] = ("sipush", Operands::S2);
    table[0x12] = ("ldc", Operands::Cp1);
    table[0x13] = ("ldc_w", Operands::Cp2);
    table[0x14] = ("ldc2_w", Operands::Cp2);
    table[0x15] = ("iload", Operands::U1);
    table[0x16] = ("lload", Operands::U1);
    table[0x17] = ("fload", Operands::U1);
    table[0x18] = ("dload", Operands::U1);
    table[0x19] = ("aload", Operands::U1);
    table[0x1a] = ("iload_0", Operands::None);
    table[0x1b] = ("iload_1", Operands::None);
    table[0x1c] = ("iload_2", Operands::None);
    table[0x1d] = ("iload_3", Operands::None);
    table[0x1e] = ("lload_0", Operands::None);
    table[0x1f] = ("lload_1", Operands::None);
    table[0x20] = ("lload_2", Operands::None);
    table[0x21] = ("lload_3", Operands::None);
    table[0x22] = ("fload_0", Operands::None);
    table[0x23] = ("fload_1", Operands::None);
    table[0x24] = ("fload_2", Operands::None);
    table[0x25] = ("fload_3", Operands::None);
    table[0x26] = ("dload_0", Operands::None);
    table[0x27] = ("dload_1", Operands::None);
    table[0x28] = ("dload_2", Operands::None);
    table[0x29] = ("dload_3", Operands::None);
    table[0x2a] = ("aload_0", Operands::None);
    table[0x2b] = ("aload_1", Operands::None);
    table[0x2c] = ("aload_2", Operands::None);
    table[0x2d] = ("aload_3", Operands::None);
    table[0x2e] = ("iaload", Operands::None);
    table[0x2f] = ("laload", Operands::None);
    table[0x30] = ("faload", Operands::None);
    table[0x31] = ("daload", Operands::None);
    table[0x32] = ("aaload", Operands::None);
    table[0x33] = ("baload", Operands::None);
    table[0x34] = ("caload", Operands::None);
    table[0x35] = ("saload", Operands::None);
    table[0x36] = ("istore", Operands::U1);
    table[0x37] = ("lstore", Operands::U1);
    table[0x38] = ("fstore", Operands::U1);
    table[0x39] = ("dstore", Operands::U1);
    table[0x3a] = ("astore", Operands::U1);
    table[0x3b] = ("istore_0", Operands::None);
    table[0x3c] = ("istore_1", Operands::None);
    table[0x3d] = ("istore_2", Operands::None);
    table[0x3e] = ("istore_3", Operands::None);
    table[0x3f] = ("lstore_0", Operands::None);
    table[0x40] = ("lstore_1", Operands::None);
    table[0x41] = ("lstore_2", Operands::None);
    table[0x42] = ("lstore_3", Operands::None);
    table[0x43] = ("fstore_0", Operands::None);
    table[0x44] = ("fstore_1", Operands::None);
    table[0x45] = ("fstore_2", Operands::None);
    table[0x46] = ("fstore_3", Operands::None);
    table[0x47] = ("dstore_0", Operands::None);
    table[0x48] = ("dstore_1", Operands::None);
    table[0x49] = ("dstore_2", Operands::None);
    table[0x4a] = ("dstore_3", Operands::None);
    table[0x4b] = ("astore_0", Operands::None);
    table[0x4c] = ("astore_1", Operands::None);
    table[0x4d] = ("astore_2", Operands::None);
    table[0x4e] = ("astore_3", Operands::None);
    table[0x4f] = ("iastore", Operands::None);
    table[0x50] = ("lastore", Operands::None);
    table[0x51] = ("fastore", Operands::None);
    table[0x52] = ("dastore", Operands::None);
    table[0x53] = ("aastore", Operands::None);
    table[0x54] = ("bastore", Operands::None);
    table[0x55] = ("castore", Operands::None);
    table[0x56] = ("sastore", Operands::None);
    table[0x57] = ("pop", Operands::None);
    table[0x58] = ("pop2", Operands::None);
    table[0x59] = ("dup", Operands::None);
    table[0x5a] = ("dup_x1", Operands::None);
    table[0x5b] = ("dup_x2", Operands::None);
    table[0x5c] = ("dup2", Operands::None);
    table[0x5d] = ("dup2_x1", Operands::None);
    table[0x5e] = ("dup2_x2", Operands::None);
    table[0x5f] = ("swap", Operands::None);
    table[0x60] = ("iadd", Operands::None);
    table[0x61] = ("ladd", Operands::None);
    table[0x62] = ("fadd", Operands::None);
    table[0x63] = ("dadd", Operands::None);
    table[0x64] = ("isub", Operands::None);
    table[0x65] = ("lsub", Operands::None);
    table[0x66] = ("fsub", Operands::None);
    table[0x67] = ("dsub", Operands::None);
    table[0x68] = ("imul", Operands::None);
    table[0x69] = ("lmul", Operands::None);
    table[0x6a] = ("fmul", Operands::None);
    table[0x6b] = ("dmul", Operands::None);
    table[0x6c] = ("idiv", Operands::None);
    table[0x6d] = ("ldiv", Operands::None);
    table[0x6e] = ("fdiv", Operands::None);
    table[0x6f] = ("ddiv", Operands::None);
    table[0x70] = ("irem", Operands::None);
    table[0x71] = ("lrem", Operands::None);
    table[0x72] = ("frem", Operands::None);
    table[0x73] = ("drem", Operands::None);
    table[0x74] = ("ineg", Operands::None);
    table[0x75] = ("lneg", Operands::None);
    table[0x76] = ("fneg", Operands::None);
    table[0x77] = ("dneg", Operands::None);
    table[0x78] = ("ishl", Operands::None);
    table[0x79] = ("lshl", Operands::None);
    table[0x7a] = ("ishr", Operands::None);
    table[0x7b] = ("lshr", Operands::None);
    table[0x7c] = ("iushr", Operands::None);
    table[0x7d] = ("lushr", Operands::None);
    table[0x7e] = ("iand", Operands::None);
    table[0x7f] = ("land", Operands::None);
    table[0x80] = ("ior", Operands::None);
    table[0x81] = ("lor", Operands::None);
    table[0x82] = ("ixor", Operands::None);
    table[0x83] = ("lxor", Operands::None);
    table[0x84] = ("iinc", Operands::IInc);
    table[0x85] = ("i2l", Operands::None);
    table[0x86] = ("i2f", Operands::None);
    table[0x87] = ("i2d", Operands::None);
    table[0x88] = ("l2i", Operands::None);
    table[0x89] = ("l2f", Operands::None);
    table[0x8a] = ("l2d", Operands::None);
    table[0x8b] = ("f2i", Operands::None);
    table[0x8c] = ("f2l", Operands::None);
    table[0x8d] = ("f2d", Operands::None);
    table[0x8e] = ("d2i", Operands::None);
    table[0x8f] = ("d2l", Operands::None);
    table[0x90] = ("d2f", Operands::None);
    table[0x91] = ("i2b", Operands::None);
    table[0x92] = ("i2c", Operands::None);
    table[0x93] = ("i2s", Operands::None);
    table[0x94] = ("lcmp", Operands::None);
    table[0x95] = ("fcmpl", Operands::None);
    table[0x96] = ("fcmpg", Operands::None);
    table[0x97] = ("dcmpl", Operands::None);
    table[0x98] = ("dcmpg", Operands::None);
    table[0x99] = ("ifeq", Operands::Branch2);
    table[0x9a] = ("ifne", Operands::Branch2);
    table[0x9b] = ("iflt", Operands::Branch2);
    table[0x9c] = ("ifge", Operands::Branch2);
    table[0x9d] = ("ifgt", Operands::Branch2);
    table[0x9e] = ("ifle", Operands::Branch2);
    table[0x9f] = ("if_icmpeq", Operands::Branch2);
    table[0xa0] = ("if_icmpne", Operands::Branch2);
    table[0xa1] = ("if_icmplt", Operands::Branch2);
    table[0xa2] = ("if_icmpge", Operands::Branch2);
    table[0xa3] = ("if_icmpgt", Operands::Branch2);
    table[0xa4] = ("if_icmple", Operands::Branch2);
    table[0xa5] = ("if_acmpeq", Operands::Branch2);
    table[0xa6] = ("if_acmpne", Operands::Branch2);
    table[0xa7] = ("goto", Operands::Branch2);
    table[0xa8] = ("jsr", Operands::Branch2);
    table[0xa9] = ("ret", Operands::U1);
    table[0xaa] = ("tableswitch", Operands::TableSwitch);
    table[0xab] = ("lookupswitch", Operands::LookupSwitch);
    table[0xac] = ("ireturn", Operands::None);
    table[0xad] = ("lreturn", Operands::None);
    table[0xae] = ("freturn", Operands::None);
    table[0xaf] = ("dreturn", Operands::None);
    table[0xb0] = ("areturn", Operands::None);
    table[0xb1] = ("return", Operands::None);
    table[0xb2] = ("getstatic", Operands::Cp2);
    table[0xb3] = ("putstatic", Operands::Cp2);
    table[0xb4] = ("getfield", Operands::Cp2);
    table[0xb5] = ("putfield", Operands::Cp2);
    table[0xb6] = ("invokevirtual", Operands::Cp2);
    table[0xb7] = ("invokespecial", Operands::Cp2);
    table[0xb8] = ("invokestatic", Operands::Cp2);
    table[0xb9] = ("invokeinterface", Operands::InvokeInterface);
    table[0xba] = ("invokedynamic", Operands::InvokeDynamic);
    table[0xbb] = ("new", Operands::Cp2);
    table[0xbc] = ("newarray", Operands::NewArray);
    table[0xbd] = ("anewarray", Operands::Cp2);
    table[0xbe] = ("arraylength", Operands::None);
    table[0xbf] = ("athrow", Operands::None);
    table[0xc0] = ("checkcast", Operands::Cp2);
    table[0xc1] = ("instanceof", Operands::Cp2);
    table[0xc2] = ("monitorenter", Operands::None);
    table[0xc3] = ("monitorexit", Operands::None);
    table[0xc4] = ("wide", Operands::Wide);
    table[0xc5] = ("multianewarray", Operands::MultiANewArray);
    table[0xc6] = ("ifnull", Operands::Branch2);
    table[0xc7] = ("ifnonnull", Operands::Branch2);
    table[0xc8] = ("goto_w", Operands::Branch4);
    table[0xc9] = ("jsr_w", Operands::Branch4);
    table[0xca] = ("breakpoint", Operands::None);
    table[0xfe] = ("impdep1", Operands::None);
    table[0xff] = ("impdep2", Operands::None);
    table
};

const ARRAY_TYPES: [&str; 12] = [
    "", "", "", "", "boolean", "char", "float", "double", "byte", "short", "int", "long",
];

/// One decoded instruction: its offset in the code array and how it reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub offset: usize,
    pub len: usize,
    pub text: String,
}

/// Decode a method's code array. Unknown opcodes become one line each, so the listing cannot
/// run away on a malformed class.
pub fn disassemble(class: &Class, code: &Code) -> Vec<Instruction> {
    let bytes = &code.bytes;
    let mut out = Vec::new();
    let mut at = 0usize;

    while at < bytes.len() {
        let start = at;
        let opcode = bytes[at];
        let (name, operands) = OPCODES[opcode as usize];
        at += 1;

        let mut text = if name.is_empty() {
            format!("unknown opcode 0x{opcode:02X}")
        } else {
            name.to_string()
        };

        macro_rules! u1 {
            () => {{
                let v = bytes.get(at).copied().unwrap_or(0);
                at += 1;
                v
            }};
        }
        macro_rules! u2 {
            () => {{
                let v = u16::from_be_bytes([
                    bytes.get(at).copied().unwrap_or(0),
                    bytes.get(at + 1).copied().unwrap_or(0),
                ]);
                at += 2;
                v
            }};
        }
        macro_rules! i4 {
            () => {{
                let v = i32::from_be_bytes([
                    bytes.get(at).copied().unwrap_or(0),
                    bytes.get(at + 1).copied().unwrap_or(0),
                    bytes.get(at + 2).copied().unwrap_or(0),
                    bytes.get(at + 3).copied().unwrap_or(0),
                ]);
                at += 4;
                v
            }};
        }

        match operands {
            Operands::None | Operands::Unknown => {}
            Operands::U1 => {
                let _ = write!(text, " {}", u1!());
            }
            Operands::S1 => {
                let _ = write!(text, " {}", u1!() as i8);
            }
            Operands::S2 => {
                let _ = write!(text, " {}", u2!() as i16);
            }
            Operands::Cp1 => {
                let index = u1!() as u16;
                let _ = write!(text, " #{index:<10} // {}", class.describe(index));
            }
            Operands::Cp2 => {
                let index = u2!();
                let _ = write!(text, " #{index:<10} // {}", class.describe(index));
            }
            Operands::Branch2 => {
                let delta = u2!() as i16 as i32;
                let _ = write!(text, " {}", start as i32 + delta);
            }
            Operands::Branch4 => {
                let delta = i4!();
                let _ = write!(text, " {}", start as i32 + delta);
            }
            Operands::IInc => {
                let index = u1!();
                let constant = u1!() as i8;
                let _ = write!(text, " {index}, {constant}");
            }
            Operands::NewArray => {
                let kind = u1!();
                let name = ARRAY_TYPES.get(kind as usize).copied().unwrap_or("?");
                let _ = write!(text, " {kind} // {name}");
            }
            Operands::MultiANewArray => {
                let index = u2!();
                let dims = u1!();
                let _ = write!(text, " #{index}, {dims} // {}", class.describe(index));
            }
            Operands::InvokeInterface => {
                let index = u2!();
                let count = u1!();
                let _ = u1!(); // the mandated zero byte
                let _ = write!(text, " #{index}, {count} // {}", class.describe(index));
            }
            Operands::InvokeDynamic => {
                let index = u2!();
                let _ = u2!(); // two zero bytes
                let _ = write!(text, " #{index} // {}", class.describe(index));
            }
            Operands::Wide => {
                // `wide` widens the next instruction's local index to two bytes.
                let next = u1!();
                let (next_name, _) = OPCODES[next as usize];
                let index = u2!();
                if next == 0x84 {
                    let constant = u2!() as i16;
                    let _ = write!(text, " iinc {index}, {constant}");
                } else {
                    let _ = write!(text, " {next_name} {index}");
                }
            }
            Operands::TableSwitch => {
                at += (4 - (at % 4)) % 4; // padded to a four-byte boundary
                let default = i4!();
                let low = i4!();
                let high = i4!();
                let mut targets = Vec::new();
                for key in low..=high.max(low) {
                    let offset = i4!();
                    targets.push(format!("{key}: {}", start as i32 + offset));
                }
                let _ = write!(
                    text,
                    " {{ {}, default: {} }}",
                    targets.join(", "),
                    start as i32 + default
                );
            }
            Operands::LookupSwitch => {
                at += (4 - (at % 4)) % 4;
                let default = i4!();
                let pairs = i4!();
                let mut targets = Vec::new();
                for _ in 0..pairs.clamp(0, 4096) {
                    let key = i4!();
                    let offset = i4!();
                    targets.push(format!("{key}: {}", start as i32 + offset));
                }
                let _ = write!(
                    text,
                    " {{ {}, default: {} }}",
                    targets.join(", "),
                    start as i32 + default
                );
            }
        }

        out.push(Instruction {
            offset: start,
            len: at - start,
            text,
        });
    }
    out
}

/// The whole class as a `javap -c`-style listing: what `F5` shows for a `.class` file.
pub fn listing(bytes: &[u8]) -> Result<String, ClassError> {
    let class = Class::parse(bytes)?;
    let mut out = String::with_capacity(4096);

    let _ = writeln!(out, "{}", class.java_version());
    if let Some(source) = &class.source_file {
        let _ = writeln!(out, "compiled from \"{source}\"");
    }
    let flags = access(class.access_flags, true);
    let _ = writeln!(out, "{flags} class {}", class.pretty_class());
    if !class.super_class.is_empty() && class.super_class != "java/lang/Object" {
        let _ = writeln!(out, "  extends {}", class.super_class.replace('/', "."));
    }
    for interface in &class.interfaces {
        let _ = writeln!(out, "  implements {}", interface.replace('/', "."));
    }
    let _ = writeln!(out);

    if !class.fields.is_empty() {
        for field in &class.fields {
            let flags = access(field.access_flags, false);
            let _ = writeln!(out, "  {flags} {} {};", field.descriptor, field.name);
        }
        let _ = writeln!(out);
    }

    for method in &class.methods {
        let flags = access(method.access_flags, false);
        let _ = writeln!(out, "  {flags} {}{};", method.name, method.descriptor);
        match &method.code {
            None => {
                let _ = writeln!(out, "    (no code: abstract or native)");
            }
            Some(code) => {
                let _ = writeln!(
                    out,
                    "    Code: stack={}, locals={}, {} bytes",
                    code.max_stack,
                    code.max_locals,
                    code.bytes.len()
                );
                for instruction in disassemble(&class, code) {
                    let _ = writeln!(out, "    {:>5}: {}", instruction.offset, instruction.text);
                }
                for handler in &code.exceptions {
                    let kind = if handler.catch_type == 0 {
                        "any".to_string()
                    } else {
                        class.describe(handler.catch_type)
                    };
                    let _ = writeln!(
                        out,
                        "      catch {kind} from {} to {} handled at {}",
                        handler.start, handler.end, handler.handler
                    );
                }
            }
        }
        let _ = writeln!(out);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tiny class-file builder, so the tests do not need a JDK.
    #[derive(Default)]
    struct Builder {
        pool: Vec<u8>,
        count: u16,
    }

    impl Builder {
        fn utf8(&mut self, text: &str) -> u16 {
            self.pool.push(1);
            self.pool
                .extend_from_slice(&(text.len() as u16).to_be_bytes());
            self.pool.extend_from_slice(text.as_bytes());
            self.next()
        }

        fn class(&mut self, name: u16) -> u16 {
            self.pool.push(7);
            self.pool.extend_from_slice(&name.to_be_bytes());
            self.next()
        }

        fn string(&mut self, utf8: u16) -> u16 {
            self.pool.push(8);
            self.pool.extend_from_slice(&utf8.to_be_bytes());
            self.next()
        }

        fn name_and_type(&mut self, name: u16, descriptor: u16) -> u16 {
            self.pool.push(12);
            self.pool.extend_from_slice(&name.to_be_bytes());
            self.pool.extend_from_slice(&descriptor.to_be_bytes());
            self.next()
        }

        fn methodref(&mut self, class: u16, nat: u16) -> u16 {
            self.pool.push(10);
            self.pool.extend_from_slice(&class.to_be_bytes());
            self.pool.extend_from_slice(&nat.to_be_bytes());
            self.next()
        }

        fn long(&mut self, value: i64) -> u16 {
            self.pool.push(5);
            self.pool.extend_from_slice(&value.to_be_bytes());
            let index = self.next();
            self.count += 1; // a long eats the next slot
            index
        }

        fn next(&mut self) -> u16 {
            self.count += 1;
            self.count
        }
    }

    fn sample_class() -> Vec<u8> {
        let mut b = Builder::default();
        let name_hello = b.utf8("Hello");
        let class_hello = b.class(name_hello);
        let name_object = b.utf8("java/lang/Object");
        let class_object = b.class(name_object);
        let name_main = b.utf8("main");
        let desc_main = b.utf8("([Ljava/lang/String;)V");
        let utf8_code = b.utf8("Code");
        let utf8_greeting = b.utf8("greeting");
        let const_greeting = b.string(utf8_greeting);
        let utf8_source = b.utf8("SourceFile");
        let utf8_file = b.utf8("Hello.java");
        let name_println = b.utf8("println");
        let desc_println = b.utf8("(Ljava/lang/String;)V");
        let nat_println = b.name_and_type(name_println, desc_println);
        let name_stream = b.utf8("java/io/PrintStream");
        let class_stream = b.class(name_stream);
        let ref_println = b.methodref(class_stream, nat_println);
        let big = b.long(1_234_567_890_123);
        let name_abstract = b.utf8("abstractThing");
        let desc_abstract = b.utf8("()V");

        // The code array: one instruction of every operand shape we care about.
        let mut code: Vec<u8> = vec![
            0x12,                 // ldc
            const_greeting as u8, //   the String constant
            0x10,                 // bipush
            42,
        ];
        code.extend_from_slice(&[0x11, 0x01, 0x00]); // sipush 256
        code.extend_from_slice(&[0x84, 0x00, 0x01]); // iinc 0, 1
        code.extend_from_slice(&[0xb6]); // invokevirtual
        code.extend_from_slice(&ref_println.to_be_bytes());
        code.extend_from_slice(&[0xbc, 0x0a]); // newarray int
        code.extend_from_slice(&[0xa7, 0x00, 0x03]); // goto +3 (to the next instruction)
        code.push(0xb1); // return
        let code_len = code.len() as u32;

        let mut code_attr: Vec<u8> = Vec::new();
        code_attr.extend_from_slice(&2u16.to_be_bytes()); // max_stack
        code_attr.extend_from_slice(&1u16.to_be_bytes()); // max_locals
        code_attr.extend_from_slice(&code_len.to_be_bytes());
        code_attr.extend_from_slice(&code);
        code_attr.extend_from_slice(&1u16.to_be_bytes()); // one exception handler
        code_attr.extend_from_slice(&0u16.to_be_bytes()); // start
        code_attr.extend_from_slice(&5u16.to_be_bytes()); // end
        code_attr.extend_from_slice(&12u16.to_be_bytes()); // handler
        code_attr.extend_from_slice(&class_stream.to_be_bytes()); // catch type
        code_attr.extend_from_slice(&0u16.to_be_bytes()); // no attributes

        let mut out: Vec<u8> = Vec::new();
        out.extend_from_slice(&0xCAFE_BABEu32.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // minor
        out.extend_from_slice(&52u16.to_be_bytes()); // major: Java 8
        out.extend_from_slice(&(b.count + 1).to_be_bytes());
        out.extend_from_slice(&b.pool);
        out.extend_from_slice(&0x0021u16.to_be_bytes()); // public super
        out.extend_from_slice(&class_hello.to_be_bytes());
        out.extend_from_slice(&class_object.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes()); // interfaces

        // one field: `static long counter`
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&0x0008u16.to_be_bytes()); // static
        out.extend_from_slice(&utf8_greeting.to_be_bytes()); // reuse as the field name
        out.extend_from_slice(&desc_println.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());

        // two methods: one with code, one abstract
        out.extend_from_slice(&2u16.to_be_bytes());
        out.extend_from_slice(&0x0009u16.to_be_bytes()); // public static
        out.extend_from_slice(&name_main.to_be_bytes());
        out.extend_from_slice(&desc_main.to_be_bytes());
        out.extend_from_slice(&1u16.to_be_bytes()); // one attribute
        out.extend_from_slice(&utf8_code.to_be_bytes());
        out.extend_from_slice(&(code_attr.len() as u32).to_be_bytes());
        out.extend_from_slice(&code_attr);

        out.extend_from_slice(&0x0401u16.to_be_bytes()); // public abstract
        out.extend_from_slice(&name_abstract.to_be_bytes());
        out.extend_from_slice(&desc_abstract.to_be_bytes());
        out.extend_from_slice(&0u16.to_be_bytes());

        // class attributes: SourceFile
        out.extend_from_slice(&1u16.to_be_bytes());
        out.extend_from_slice(&utf8_source.to_be_bytes());
        out.extend_from_slice(&2u32.to_be_bytes());
        out.extend_from_slice(&utf8_file.to_be_bytes());

        let _ = big;
        out
    }

    #[test]
    fn a_class_file_is_parsed_with_its_pool_and_members() {
        let class = Class::parse(&sample_class()).expect("parses");
        assert_eq!(class.this_class, "Hello");
        assert_eq!(class.pretty_class(), "Hello");
        assert_eq!(class.super_class, "java/lang/Object");
        assert_eq!(class.major, 52);
        assert!(
            class.java_version().contains("Java 8"),
            "{}",
            class.java_version()
        );
        assert_eq!(class.source_file.as_deref(), Some("Hello.java"));
        assert_eq!(class.fields.len(), 1);
        assert_eq!(class.methods.len(), 2);
        assert_eq!(class.methods[0].name, "main");
        assert_eq!(class.methods[0].descriptor, "([Ljava/lang/String;)V");
        assert!(class.methods[0].code.is_some());
        assert!(
            class.methods[1].code.is_none(),
            "an abstract method has no Code attribute"
        );
        assert_eq!(
            access(class.methods[0].access_flags, false),
            "public static"
        );
    }

    #[test]
    fn the_constant_pool_is_resolved_for_comments() {
        let bytes = sample_class();
        let class = Class::parse(&bytes).unwrap();
        let code = class.methods[0].code.as_ref().unwrap();
        let listing: Vec<String> = disassemble(&class, code)
            .into_iter()
            .map(|i| format!("{}: {}", i.offset, i.text))
            .collect();
        let joined = listing.join("\n");

        assert!(joined.contains("ldc"), "{joined}");
        assert!(joined.contains("String \"greeting\""), "{joined}");
        assert!(
            joined.contains("java/io/PrintStream.println:(Ljava/lang/String;)V"),
            "{joined}"
        );
        assert!(joined.contains("bipush 42"), "{joined}");
        assert!(joined.contains("sipush 256"), "{joined}");
        assert!(joined.contains("iinc 0, 1"), "{joined}");
        assert!(joined.contains("newarray 10 // int"), "{joined}");
        assert!(joined.contains("return"), "{joined}");
    }

    #[test]
    fn branch_targets_are_absolute_offsets_not_deltas() {
        let bytes = sample_class();
        let class = Class::parse(&bytes).unwrap();
        let code = class.methods[0].code.as_ref().unwrap();
        let instructions = disassemble(&class, code);
        let goto = instructions
            .iter()
            .find(|i| i.text.starts_with("goto"))
            .expect("there is a goto");
        // `goto +3` at offset N targets N+3, which is what a reader needs to see.
        assert_eq!(goto.text, format!("goto {}", goto.offset + 3));
        assert_eq!(goto.len, 3);
    }

    #[test]
    fn instruction_offsets_and_lengths_cover_the_code_array_exactly() {
        let bytes = sample_class();
        let class = Class::parse(&bytes).unwrap();
        let code = class.methods[0].code.as_ref().unwrap();
        let instructions = disassemble(&class, code);
        let mut expected = 0usize;
        for instruction in &instructions {
            assert_eq!(instruction.offset, expected, "{instructions:?}");
            expected += instruction.len;
        }
        assert_eq!(expected, code.bytes.len(), "the sweep must end at the end");
    }

    #[test]
    fn switches_decode_with_their_targets() {
        // tableswitch at offset 0: pad to 4, default, low=1, high=2, two targets
        let mut code = vec![0xaa, 0x00, 0x00, 0x00];
        code.extend_from_slice(&40i32.to_be_bytes()); // default
        code.extend_from_slice(&1i32.to_be_bytes()); // low
        code.extend_from_slice(&2i32.to_be_bytes()); // high
        code.extend_from_slice(&10i32.to_be_bytes()); // target for 1
        code.extend_from_slice(&20i32.to_be_bytes()); // target for 2
        let class = Class::parse(&sample_class()).unwrap();
        let instructions = disassemble(
            &class,
            &Code {
                max_stack: 1,
                max_locals: 1,
                bytes: code,
                exceptions: Vec::new(),
            },
        );
        assert_eq!(instructions.len(), 1, "{instructions:?}");
        let text = &instructions[0].text;
        assert!(text.starts_with("tableswitch"), "{text}");
        assert!(text.contains("1: 10") && text.contains("2: 20"), "{text}");
        assert!(text.contains("default: 40"), "{text}");
    }

    #[test]
    fn a_wide_instruction_widens_the_next_one() {
        let class = Class::parse(&sample_class()).unwrap();
        let code = Code {
            max_stack: 1,
            max_locals: 300,
            bytes: vec![0xc4, 0x15, 0x01, 0x2c, 0xc4, 0x84, 0x01, 0x2c, 0x00, 0x05],
            exceptions: Vec::new(),
        };
        let instructions = disassemble(&class, &code);
        assert_eq!(instructions[0].text, "wide iload 300");
        assert_eq!(instructions[0].len, 4);
        assert_eq!(instructions[1].text, "wide iinc 300, 5");
        assert_eq!(instructions[1].len, 6);
    }

    #[test]
    fn unknown_opcodes_do_not_derail_the_listing() {
        let class = Class::parse(&sample_class()).unwrap();
        let code = Code {
            max_stack: 0,
            max_locals: 0,
            bytes: vec![0xcb, 0x00, 0xb1],
            exceptions: Vec::new(),
        };
        let instructions = disassemble(&class, &code);
        assert!(
            instructions[0].text.contains("unknown opcode 0xCB"),
            "{instructions:?}"
        );
        assert_eq!(instructions[2].text, "return");
    }

    #[test]
    fn the_listing_reads_like_javap() {
        let text = listing(&sample_class()).expect("a listing");
        assert!(text.contains("Java 8"), "{text}");
        assert!(text.contains("public super class Hello"), "{text}");
        assert!(text.contains("compiled from \"Hello.java\""), "{text}");
        assert!(
            text.contains("public static main([Ljava/lang/String;)V;"),
            "{text}"
        );
        assert!(text.contains("Code: stack=2, locals=1"), "{text}");
        assert!(
            text.contains("catch java/io/PrintStream from 0 to 5"),
            "{text}"
        );
        assert!(
            text.contains("(no code: abstract or native)"),
            "an abstract method must say so: {text}"
        );
    }

    #[test]
    fn broken_input_is_reported_not_guessed() {
        assert!(matches!(
            Class::parse(b"not a class at all"),
            Err(ClassError::NotAClass)
        ));
        let truncated = &sample_class()[..20];
        assert!(matches!(
            Class::parse(truncated),
            Err(ClassError::Truncated(_)) | Err(ClassError::BadTag(_))
        ));
    }

    #[test]
    fn long_constants_take_two_pool_slots() {
        // If the gap after a long were missing, every later index would be wrong — and the
        // sample class puts a long in the middle of its pool.
        let class = Class::parse(&sample_class()).unwrap();
        assert_eq!(class.methods[1].name, "abstractThing");
        assert_eq!(class.methods[1].descriptor, "()V");
    }
}
