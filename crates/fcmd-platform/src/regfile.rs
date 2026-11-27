//! A registry value as bytes and as something a person reads, and the `.reg` file regedit
//! writes and reads back (SPEC 06 §17).
//!
//! Nothing here calls Windows: a value is a name, a type number and the bytes the registry
//! keeps for it, and a `.reg` file is text. That is on purpose — the format is the one place
//! where a mistake loses somebody's settings, so it is plain Rust that every platform's
//! tests run, and the calls that need Windows ([`crate::registry`]) only carry these types
//! to and from the system.
//!
//! The file is the one `regedit` calls *version 5*: `Windows Registry Editor Version 5.00`
//! on the first line, UTF-16 little endian with a byte order mark, `\r\n` line ends, a key
//! in square brackets and its values under it. A string is written in quotes with `\` and
//! `"` escaped; a `REG_DWORD` as `dword:` and eight hex digits; everything else as `hex:`
//! or `hex(<type>):` and its bytes, wrapped with a `\` at the end of the line the way
//! regedit wraps them. The older `REGEDIT4` file, which is the same in the machine's ANSI
//! code page, is read as well and never written.

use std::fmt;

pub const REG_NONE: u32 = 0;
pub const REG_SZ: u32 = 1;
pub const REG_EXPAND_SZ: u32 = 2;
pub const REG_BINARY: u32 = 3;
pub const REG_DWORD: u32 = 4;
pub const REG_DWORD_BIG_ENDIAN: u32 = 5;
pub const REG_LINK: u32 = 6;
pub const REG_MULTI_SZ: u32 = 7;
pub const REG_QWORD: u32 = 11;

/// The first line of the file this module writes.
pub const HEADER: &str = "Windows Registry Editor Version 5.00";
/// The first line of the file Windows 95 and NT 4 wrote, which is read and never written.
const HEADER_4: &str = "REGEDIT4";

/// One value of a key: its name — empty for the one regedit shows as `(Default)` —, the
/// type as the number the registry keeps, and the bytes exactly as the registry keeps them.
///
/// The bytes are kept raw because the type is a label and not a promise: a `REG_SZ` with no
/// terminator, a `REG_DWORD` of three bytes and a `REG_MULTI_SZ` that is not a list all
/// exist on real machines, and a panel that normalised them on the way through would write
/// back something other than what it read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Value {
    pub name: String,
    pub kind: u32,
    pub data: Vec<u8>,
}

/// Why a value's text could not become its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DataError {
    /// Not a number, in decimal or after `0x` in hex.
    NotANumber,
    /// A number, but more than the type holds.
    TooLarge,
    /// Not pairs of hex digits.
    NotHex,
}

fn utf16(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

fn units(data: &[u8]) -> Vec<u16> {
    data.as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u16::from_le_bytes(*pair))
        .collect()
}

impl Value {
    /// A `REG_SZ`: the text and its terminator.
    pub fn sz(name: &str, text: &str) -> Value {
        Value::text_of(name, REG_SZ, text)
    }

    /// A `REG_EXPAND_SZ`: text with `%VARIABLES%` in it for whoever reads it to expand.
    pub fn expand_sz(name: &str, text: &str) -> Value {
        Value::text_of(name, REG_EXPAND_SZ, text)
    }

    fn text_of(name: &str, kind: u32, text: &str) -> Value {
        let mut data = utf16(text);
        data.extend_from_slice(&[0, 0]);
        Value {
            name: name.to_string(),
            kind,
            data,
        }
    }

    /// A `REG_MULTI_SZ`: each string with its terminator, and one more to end the list.
    pub fn multi_sz<S: AsRef<str>>(name: &str, strings: &[S]) -> Value {
        let mut data = Vec::new();
        for s in strings {
            data.extend(utf16(s.as_ref()));
            data.extend_from_slice(&[0, 0]);
        }
        data.extend_from_slice(&[0, 0]);
        Value {
            name: name.to_string(),
            kind: REG_MULTI_SZ,
            data,
        }
    }

    pub fn dword(name: &str, number: u32) -> Value {
        Value {
            name: name.to_string(),
            kind: REG_DWORD,
            data: number.to_le_bytes().to_vec(),
        }
    }

    pub fn qword(name: &str, number: u64) -> Value {
        Value {
            name: name.to_string(),
            kind: REG_QWORD,
            data: number.to_le_bytes().to_vec(),
        }
    }

    pub fn binary(name: &str, data: &[u8]) -> Value {
        Value {
            name: name.to_string(),
            kind: REG_BINARY,
            data: data.to_vec(),
        }
    }

    /// The value a new one of this type starts as: an empty string, zero, no bytes.
    pub fn empty(name: &str, kind: u32) -> Value {
        match kind {
            REG_SZ | REG_EXPAND_SZ => Value::text_of(name, kind, ""),
            REG_MULTI_SZ => Value::multi_sz::<&str>(name, &[]),
            REG_DWORD => Value::dword(name, 0),
            REG_QWORD => Value::qword(name, 0),
            _ => Value {
                name: name.to_string(),
                kind,
                data: Vec::new(),
            },
        }
    }

    /// Whether the type says the bytes are text.
    pub fn is_text(&self) -> bool {
        matches!(self.kind, REG_SZ | REG_EXPAND_SZ | REG_LINK)
    }

    /// The bytes read as one string: UTF-16, up to the terminator if there is one.
    pub fn text(&self) -> String {
        let units = units(&self.data);
        let end = units.iter().position(|u| *u == 0).unwrap_or(units.len());
        String::from_utf16_lossy(&units[..end])
    }

    /// The bytes read as a list of strings, the way a `REG_MULTI_SZ` keeps one.
    pub fn strings(&self) -> Vec<String> {
        let mut units = units(&self.data);
        // The list ends with an empty string, and each string with a terminator: without
        // the last two a split on the terminator gives exactly the strings.
        for _ in 0..2 {
            if units.last() == Some(&0) {
                units.pop();
            }
        }
        if units.is_empty() {
            return Vec::new();
        }
        units
            .split(|u| *u == 0)
            .map(String::from_utf16_lossy)
            .collect()
    }

    /// The number a `REG_DWORD` or a `REG_QWORD` holds, when it has the bytes for one.
    pub fn number(&self) -> Option<u64> {
        match (self.kind, self.data.len()) {
            (REG_DWORD, 4) => Some(u64::from(u32::from_le_bytes(
                self.data[..].try_into().ok()?,
            ))),
            (REG_DWORD_BIG_ENDIAN, 4) => Some(u64::from(u32::from_be_bytes(
                self.data[..].try_into().ok()?,
            ))),
            (REG_QWORD, 8) => Some(u64::from_le_bytes(self.data[..].try_into().ok()?)),
            _ => None,
        }
    }

    /// The value on one line, for a panel row: the text, the strings of a list with a bar
    /// between them, a number in hex and decimal, or the first bytes.
    pub fn summary(&self) -> String {
        if self.is_text() {
            return self.text();
        }
        if self.kind == REG_MULTI_SZ {
            return self.strings().join(" | ");
        }
        if let Some(n) = self.number() {
            return if self.kind == REG_QWORD {
                format!("0x{n:016x} ({n})")
            } else {
                format!("0x{n:08x} ({n})")
            };
        }
        const SHOWN: usize = 24;
        let mut out = hex_text(&self.data[..self.data.len().min(SHOWN)]);
        if self.data.len() > SHOWN {
            out.push_str(" …");
        }
        out
    }

    /// The value in full, a line at a time, for `F3`: the text, a list one string per line,
    /// a number in decimal and in hex, anything else as a dump with its offsets.
    pub fn full(&self) -> Vec<String> {
        if self.is_text() {
            return self.text().lines().map(str::to_string).collect();
        }
        if self.kind == REG_MULTI_SZ {
            return self.strings();
        }
        if let Some(n) = self.number() {
            return vec![if self.kind == REG_QWORD {
                format!("{n}  (0x{n:016x})")
            } else {
                format!("{n}  (0x{n:08x})")
            }];
        }
        hex_dump(&self.data)
    }

    /// What the editing dialog opens on: the text, the number in decimal, or the bytes as
    /// pairs of hex digits. A list is edited a string a row, from [`Value::strings`].
    pub fn edit_text(&self) -> String {
        if self.is_text() {
            return self.text();
        }
        match self.number() {
            Some(n) if self.kind != REG_DWORD_BIG_ENDIAN => n.to_string(),
            _ => hex_text(&self.data),
        }
    }
}

/// `REG_SZ` and its kin by name; a type nobody named by its number.
pub fn type_name(kind: u32) -> String {
    match kind {
        REG_NONE => "REG_NONE".into(),
        REG_SZ => "REG_SZ".into(),
        REG_EXPAND_SZ => "REG_EXPAND_SZ".into(),
        REG_BINARY => "REG_BINARY".into(),
        REG_DWORD => "REG_DWORD".into(),
        REG_DWORD_BIG_ENDIAN => "REG_DWORD_BE".into(),
        REG_LINK => "REG_LINK".into(),
        REG_MULTI_SZ => "REG_MULTI_SZ".into(),
        REG_QWORD => "REG_QWORD".into(),
        other => format!("REG_{other}"),
    }
}

/// The bytes a value of this type holds for what was typed (the inverse of
/// [`Value::edit_text`]): text for a string, a number in decimal or `0x` hex for a
/// `REG_DWORD` or `REG_QWORD` — refused when it does not fit —, pairs of hex digits for
/// everything else.
pub fn parse_data(kind: u32, text: &str) -> Result<Vec<u8>, DataError> {
    match kind {
        REG_SZ | REG_EXPAND_SZ | REG_LINK => {
            let mut data = utf16(text);
            data.extend_from_slice(&[0, 0]);
            Ok(data)
        }
        REG_DWORD => {
            let n = parse_number(text)?;
            let n = u32::try_from(n).map_err(|_| DataError::TooLarge)?;
            Ok(n.to_le_bytes().to_vec())
        }
        REG_QWORD => Ok(parse_number(text)?.to_le_bytes().to_vec()),
        _ => parse_hex(text),
    }
}

fn parse_number(text: &str) -> Result<u64, DataError> {
    let text = text.trim();
    let (digits, radix) = match text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        Some(hex) => (hex, 16),
        None => (text, 10),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return Err(DataError::NotANumber);
    }
    // Only digits are left, so the one way this fails is by being too long for 64 bits.
    u64::from_str_radix(digits, radix).map_err(|_| DataError::TooLarge)
}

/// `01 02 0a`, `01,02,0a` or `01020a`: pairs of hex digits, however they are spaced.
fn parse_hex(text: &str) -> Result<Vec<u8>, DataError> {
    let digits: Vec<u32> = text
        .chars()
        .filter(|c| !c.is_whitespace() && *c != ',')
        .map(|c| c.to_digit(16).ok_or(DataError::NotHex))
        .collect::<Result<_, _>>()?;
    if !digits.len().is_multiple_of(2) {
        return Err(DataError::NotHex);
    }
    Ok(digits
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| (pair[0] * 16 + pair[1]) as u8)
        .collect())
}

fn hex_text(data: &[u8]) -> String {
    data.iter()
        .map(|b| format!("{b:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Sixteen bytes a line: the offset, the bytes, and the characters they are where they are
/// printable.
fn hex_dump(data: &[u8]) -> Vec<String> {
    data.chunks(16)
        .enumerate()
        .map(|(row, chunk)| {
            let text: String = chunk
                .iter()
                .map(|b| {
                    if b.is_ascii_graphic() || *b == b' ' {
                        *b as char
                    } else {
                        '.'
                    }
                })
                .collect();
            format!("{:08x}  {:<47}  {text}", row * 16, hex_text(chunk))
        })
        .collect()
}

// ── the file ────────────────────────────────────────────────────────────────────────────

/// One line under a key: a value to write, or the name of one to take away (`"name"=-`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    Set(Value),
    Delete(String),
}

/// One key of a `.reg` file and what the file says about it. `delete` is the `[-key]` form:
/// the key and everything under it go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyBlock {
    pub path: String,
    pub delete: bool,
    pub lines: Vec<Line>,
}

/// Why a file could not be read as a `.reg` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegFileError {
    /// The first line is neither of the two a `.reg` file starts with.
    NotARegFile,
    /// Something the format has no meaning for, and the line it is on (from 1).
    Syntax(usize),
}

impl fmt::Display for RegFileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegFileError::NotARegFile => write!(f, "not a .reg file"),
            RegFileError::Syntax(line) => write!(f, "line {line}"),
        }
    }
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    for c in text.chars() {
        if c == '\\' || c == '"' {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The string a `REG_SZ` can be written as in quotes: well-formed UTF-16 with exactly its
/// terminator. Anything else — no terminator, a NUL in the middle, half a character — goes
/// out as bytes, so that reading the file back gives the same bytes.
fn quotable(data: &[u8]) -> Option<String> {
    if data.len() < 2 || !data.len().is_multiple_of(2) {
        return None;
    }
    let units = units(data);
    let (last, text) = units.split_last()?;
    if *last != 0 || text.contains(&0) {
        return None;
    }
    String::from_utf16(text).ok()
}

/// The text of a `.reg` file holding these keys, in the order given.
pub fn write(blocks: &[KeyBlock]) -> String {
    let mut out = String::from(HEADER);
    out.push_str("\r\n\r\n");
    for block in blocks {
        out.push('[');
        if block.delete {
            out.push('-');
        }
        out.push_str(&block.path);
        out.push_str("]\r\n");
        for line in &block.lines {
            let (name, value) = match line {
                Line::Set(value) => (&value.name, Some(value)),
                Line::Delete(name) => (name, None),
            };
            let mut text = if name.is_empty() {
                "@".to_string()
            } else {
                format!("\"{}\"", escape(name))
            };
            text.push('=');
            match value {
                None => text.push('-'),
                Some(value) => write_data(&mut text, value),
            }
            out.push_str(&text);
            out.push_str("\r\n");
        }
        out.push_str("\r\n");
    }
    out
}

fn write_data(text: &mut String, value: &Value) {
    if value.kind == REG_SZ {
        if let Some(s) = quotable(&value.data) {
            text.push('"');
            text.push_str(&escape(&s));
            text.push('"');
            return;
        }
    }
    if value.kind == REG_DWORD && value.data.len() == 4 {
        let n = u32::from_le_bytes([value.data[0], value.data[1], value.data[2], value.data[3]]);
        text.push_str(&format!("dword:{n:08x}"));
        return;
    }
    if value.kind == REG_BINARY {
        text.push_str("hex:");
    } else {
        text.push_str(&format!("hex({:x}):", value.kind));
    }
    // Regedit's own wrapping: a line is cut with a `\` before it would pass eighty columns,
    // and goes on two spaces in, which is twenty-five bytes a line from the second on.
    let mut column = text.chars().count();
    for (i, byte) in value.data.iter().enumerate() {
        let last = i + 1 == value.data.len();
        let width = if last { 2 } else { 3 };
        if column + width > 79 {
            text.push_str("\\\r\n  ");
            column = 2;
        }
        text.push_str(&format!("{byte:02x}"));
        if !last {
            text.push(',');
        }
        column += width;
    }
}

/// The bytes of a `.reg` file with this text: UTF-16 little endian after its byte order
/// mark, which is the only encoding regedit takes a version 5 file in.
pub fn encode(text: &str) -> Vec<u8> {
    let mut out = vec![0xff, 0xfe];
    out.extend(utf16(text));
    out
}

/// The text of a `.reg` file's bytes: UTF-16 where the byte order mark says so, otherwise
/// UTF-8, and otherwise one character a byte — what a `REGEDIT4` file from an editor is.
pub fn decode(bytes: &[u8]) -> String {
    if let Some(rest) = bytes.strip_prefix(&[0xff, 0xfe]) {
        return String::from_utf16_lossy(&units(rest));
    }
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    }
}

/// The keys a `.reg` file's text names, in the file's order.
///
/// Strict on purpose: a line that is not a key, a value or a comment is an error with its
/// line number rather than something skipped, because the caller is about to write what
/// this returns into the registry and half a file is not what anybody meant to import.
pub fn parse(text: &str) -> Result<Vec<KeyBlock>, RegFileError> {
    let chars: Vec<char> = text.trim_start_matches('\u{feff}').chars().collect();
    let mut p = Parser { chars, at: 0 };
    let header = p.rest_of_line();
    let ansi = match header.trim() {
        HEADER => false,
        HEADER_4 => true,
        _ => return Err(RegFileError::NotARegFile),
    };
    let mut blocks: Vec<KeyBlock> = Vec::new();
    loop {
        p.skip_blank();
        let Some(c) = p.peek() else { break };
        match c {
            ';' => {
                p.rest_of_line();
            }
            '[' => {
                let at = p.at;
                let line = p.rest_of_line();
                let line = line.trim_end();
                let inner = line
                    .strip_prefix('[')
                    .and_then(|l| l.strip_suffix(']'))
                    .ok_or_else(|| p.error(at))?;
                let (delete, path) = match inner.strip_prefix('-') {
                    Some(path) => (true, path),
                    None => (false, inner),
                };
                if path.is_empty() {
                    return Err(p.error(at));
                }
                blocks.push(KeyBlock {
                    path: path.to_string(),
                    delete,
                    lines: Vec::new(),
                });
            }
            '@' | '"' => {
                let at = p.at;
                let name = if c == '@' {
                    p.at += 1;
                    String::new()
                } else {
                    p.quoted()?
                };
                p.skip_spaces();
                if p.peek() != Some('=') {
                    return Err(p.error(p.at));
                }
                p.at += 1;
                p.skip_spaces();
                let line = p.data(name, ansi)?;
                // A value before any key belongs to nothing.
                let block = blocks.last_mut().ok_or_else(|| p.error(at))?;
                block.lines.push(line);
            }
            _ => return Err(p.error(p.at)),
        }
    }
    Ok(blocks)
}

struct Parser {
    chars: Vec<char>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.at).copied()
    }

    fn error(&self, at: usize) -> RegFileError {
        let line = self.chars[..at.min(self.chars.len())]
            .iter()
            .filter(|c| **c == '\n')
            .count();
        RegFileError::Syntax(line + 1)
    }

    fn skip_blank(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.at += 1;
        }
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(' ') | Some('\t')) {
            self.at += 1;
        }
    }

    /// What is left of this line, without its end, leaving the cursor on the next one.
    fn rest_of_line(&mut self) -> String {
        let start = self.at;
        while self.peek().is_some_and(|c| c != '\n') {
            self.at += 1;
        }
        let line: String = self.chars[start..self.at].iter().collect();
        self.at = (self.at + 1).min(self.chars.len());
        line.trim_end_matches('\r').to_string()
    }

    /// A string in quotes, from its opening quote: `\` makes the next character itself, and
    /// a line end inside it is part of the string, which is how regedit writes one.
    fn quoted(&mut self) -> Result<String, RegFileError> {
        let start = self.at;
        self.at += 1;
        let mut out = String::new();
        loop {
            match self.peek() {
                None => return Err(self.error(start)),
                Some('"') => {
                    self.at += 1;
                    return Ok(out);
                }
                Some('\\') => {
                    self.at += 1;
                    match self.peek() {
                        Some(c) => out.push(c),
                        None => return Err(self.error(start)),
                    }
                    self.at += 1;
                }
                Some(c) => {
                    out.push(c);
                    self.at += 1;
                }
            }
        }
    }

    /// What follows the `=`.
    fn data(&mut self, name: String, ansi: bool) -> Result<Line, RegFileError> {
        let start = self.at;
        if self.peek() == Some('"') {
            let text = self.quoted()?;
            self.end_of_value(start)?;
            return Ok(Line::Set(Value::sz(&name, &text)));
        }
        if self.peek() == Some('-') {
            self.at += 1;
            self.end_of_value(start)?;
            return Ok(Line::Delete(name));
        }
        if self.word("dword:") {
            let digits = self.rest_of_line();
            let digits = digits.split(';').next().unwrap_or("").trim();
            if digits.is_empty() || digits.len() > 8 {
                return Err(self.error(start));
            }
            let n = u32::from_str_radix(digits, 16).map_err(|_| self.error(start))?;
            return Ok(Line::Set(Value::dword(&name, n)));
        }
        if !self.word("hex") {
            return Err(self.error(start));
        }
        let mut kind = REG_BINARY;
        if self.peek() == Some('(') {
            self.at += 1;
            let mut digits = String::new();
            while let Some(c) = self.peek().filter(|c| *c != ')' && *c != '\n') {
                digits.push(c);
                self.at += 1;
            }
            if self.peek() != Some(')') {
                return Err(self.error(start));
            }
            self.at += 1;
            kind = u32::from_str_radix(digits.trim(), 16).map_err(|_| self.error(start))?;
        }
        if self.peek() != Some(':') {
            return Err(self.error(start));
        }
        self.at += 1;
        let mut data = self.bytes(start)?;
        // A version 4 file keeps its strings in the ANSI code page, a byte a character;
        // the registry keeps them in UTF-16, so they are widened on the way in.
        if ansi && matches!(kind, REG_SZ | REG_EXPAND_SZ | REG_MULTI_SZ) {
            data = data.iter().flat_map(|b| [*b, 0]).collect();
        }
        Ok(Line::Set(Value { name, kind, data }))
    }

    /// The word, if the text goes on with it (in either case).
    fn word(&mut self, word: &str) -> bool {
        let end = self.at + word.len();
        let found = self.chars.get(self.at..end).is_some_and(|have| {
            have.iter()
                .zip(word.chars())
                .all(|(a, b)| a.eq_ignore_ascii_case(&b))
        });
        if found {
            self.at = end;
        }
        found
    }

    /// Nothing but blanks or a comment may follow a value on its line.
    fn end_of_value(&mut self, start: usize) -> Result<(), RegFileError> {
        let rest = self.rest_of_line();
        let rest = rest.trim();
        if rest.is_empty() || rest.starts_with(';') {
            Ok(())
        } else {
            Err(self.error(start))
        }
    }

    /// Pairs of hex digits with commas between them, going on over a `\` at a line's end.
    fn bytes(&mut self, start: usize) -> Result<Vec<u8>, RegFileError> {
        let mut out = Vec::new();
        let mut digits = String::new();
        let mut push = |digits: &mut String, p: &Parser| -> Result<(), RegFileError> {
            if digits.is_empty() {
                return Ok(());
            }
            if digits.len() > 2 {
                return Err(p.error(start));
            }
            out.push(u8::from_str_radix(digits, 16).map_err(|_| p.error(start))?);
            digits.clear();
            Ok(())
        };
        loop {
            match self.peek() {
                None | Some('\n') => {
                    self.at = (self.at + 1).min(self.chars.len());
                    break;
                }
                Some(';') => {
                    self.rest_of_line();
                    break;
                }
                Some('\\') => {
                    // The line goes on: whatever is left of this one is its end.
                    let rest = self.rest_of_line();
                    if rest.trim() != "\\" {
                        return Err(self.error(start));
                    }
                }
                Some(',') => {
                    push(&mut digits, self)?;
                    self.at += 1;
                }
                Some(c) if c.is_whitespace() => {
                    self.at += 1;
                }
                Some(c) if c.is_ascii_hexdigit() => {
                    digits.push(c);
                    self.at += 1;
                }
                Some(_) => return Err(self.error(start)),
            }
        }
        push(&mut digits, self)?;
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(values: Vec<Value>) -> Vec<KeyBlock> {
        vec![KeyBlock {
            path: r"HKEY_CURRENT_USER\Software\Example".into(),
            delete: false,
            lines: values.into_iter().map(Line::Set).collect(),
        }]
    }

    #[test]
    fn every_type_comes_back_from_the_file_as_the_bytes_it_went_in_as() {
        let long: Vec<u8> = (0..=255u8).collect();
        let blocks = block(vec![
            Value::sz("", "the default"),
            Value::sz("Text", "plain"),
            Value::expand_sz("Path", r"%SystemRoot%\system32"),
            Value::multi_sz("List", &["one", "two", "three"]),
            Value::multi_sz::<&str>("Empty list", &[]),
            Value::dword("Count", 42),
            Value::dword("All", u32::MAX),
            Value::qword("Big", 0x0102_0304_0506_0708),
            Value::binary("Bytes", &long),
            Value::binary("No bytes", &[]),
            Value::empty("Nothing", REG_NONE),
            Value {
                name: "Odd".into(),
                kind: 0x1234,
                data: vec![1, 2, 3],
            },
            // What the type promises and the bytes do not keep: these go out as bytes.
            Value {
                name: "Short dword".into(),
                kind: REG_DWORD,
                data: vec![1, 2, 3],
            },
            Value {
                name: "Unterminated".into(),
                kind: REG_SZ,
                data: utf16("abc"),
            },
            Value {
                name: "Empty string bytes".into(),
                kind: REG_SZ,
                data: Vec::new(),
            },
        ]);
        let text = write(&blocks);
        assert_eq!(parse(&text).unwrap(), blocks);
        // And through the bytes on disk as well.
        assert_eq!(parse(&decode(&encode(&text))).unwrap(), blocks);
    }

    #[test]
    fn the_file_is_written_the_way_regedit_writes_it() {
        let text = write(&block(vec![
            Value::sz("", "x"),
            Value::sz("Name", "a"),
            Value::dword("Count", 42),
            Value::qword("Big", 1),
            Value::expand_sz("P", "%A%"),
            Value::multi_sz("L", &["a"]),
            Value::binary("B", &[1, 0xab]),
            Value::empty("N", REG_NONE),
        ]));
        let want = [
            "Windows Registry Editor Version 5.00",
            "",
            r"[HKEY_CURRENT_USER\Software\Example]",
            "@=\"x\"",
            "\"Name\"=\"a\"",
            "\"Count\"=dword:0000002a",
            "\"Big\"=hex(b):01,00,00,00,00,00,00,00",
            "\"P\"=hex(2):25,00,41,00,25,00,00,00",
            "\"L\"=hex(7):61,00,00,00,00,00",
            "\"B\"=hex:01,ab",
            "\"N\"=hex(0):",
            "",
            "",
        ]
        .join("\r\n");
        assert_eq!(text, want);
    }

    #[test]
    fn a_backslash_and_a_quote_are_escaped_in_names_and_in_strings() {
        let blocks = block(vec![Value::sz(
            r#"a "quoted" \name"#,
            r#"C:\Program Files\"x" \\server\share"#,
        )]);
        let text = write(&blocks);
        assert!(
            text.contains(r#""a \"quoted\" \\name"="C:\\Program Files\\\"x\" \\\\server\\share""#),
            "{text}"
        );
        assert_eq!(parse(&text).unwrap(), blocks);
    }

    #[test]
    fn a_string_of_several_lines_survives_the_file() {
        let blocks = block(vec![Value::sz("Note", "first\r\nsecond\nthird")]);
        assert_eq!(parse(&write(&blocks)).unwrap(), blocks);
    }

    #[test]
    fn long_bytes_are_wrapped_with_a_backslash_before_eighty_columns() {
        let data: Vec<u8> = (0..200u8).collect();
        let blocks = block(vec![Value::binary("Long", &data)]);
        let text = write(&blocks);
        let lines: Vec<&str> = text.split("\r\n").collect();
        let wrapped: Vec<&&str> = lines.iter().filter(|l| l.ends_with('\\')).collect();
        assert!(wrapped.len() > 5, "{text}");
        for line in &lines {
            assert!(line.chars().count() <= 80, "{line}");
        }
        // Every line that goes on starts two spaces in and holds twenty-five bytes.
        let second = lines
            .iter()
            .position(|l| l.ends_with('\\'))
            .map(|i| lines[i + 1])
            .unwrap();
        assert!(second.starts_with("  ") && !second.starts_with("   "));
        assert_eq!(second.matches(',').count(), 25);
        assert_eq!(parse(&text).unwrap(), blocks);
    }

    #[test]
    fn the_bytes_on_disk_are_utf16_little_endian_after_a_byte_order_mark() {
        let bytes = encode("Wi\u{e8}");
        assert_eq!(bytes, [0xff, 0xfe, b'W', 0, b'i', 0, 0xe8, 0]);
        assert_eq!(decode(&bytes), "Wi\u{e8}");
        // A file somebody saved as UTF-8, with or without a mark, reads the same.
        assert_eq!(decode("Wi\u{e8}".as_bytes()), "Wi\u{e8}");
        assert_eq!(decode(b"\xef\xbb\xbfWi"), "Wi");
    }

    #[test]
    fn deletions_and_comments_are_read() {
        let text = format!(
            "{HEADER}\r\n; a comment\r\n\r\n[-HKEY_CURRENT_USER\\Gone]\r\n\r\n\
             [HKEY_CURRENT_USER\\Kept]\r\n\"Old\"=-\r\n@=-\r\n\"n\"=dword:1 ; one\r\n\
             \"h\"=hex:01,\\\r\n  02 ; two\r\n"
        );
        let blocks = parse(&text).unwrap();
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].delete && blocks[0].path == r"HKEY_CURRENT_USER\Gone");
        assert_eq!(
            blocks[1].lines,
            vec![
                Line::Delete("Old".into()),
                Line::Delete(String::new()),
                Line::Set(Value::dword("n", 1)),
                Line::Set(Value::binary("h", &[1, 2])),
            ]
        );
        // And written back, the deletions are still deletions.
        assert_eq!(parse(&write(&blocks)).unwrap(), blocks);
    }

    #[test]
    fn a_version_4_file_has_its_strings_widened() {
        let text =
            "REGEDIT4\r\n\r\n[HKEY_CURRENT_USER\\K]\r\n\"p\"=hex(2):41,42,00\r\n\"s\"=\"x\"\r\n";
        let blocks = parse(text).unwrap();
        assert_eq!(
            blocks[0].lines,
            vec![
                Line::Set(Value::expand_sz("p", "AB")),
                Line::Set(Value::sz("s", "x")),
            ]
        );
    }

    #[test]
    fn what_is_not_a_reg_file_is_refused_with_the_line() {
        assert_eq!(parse("hello\r\n"), Err(RegFileError::NotARegFile));
        assert_eq!(parse(""), Err(RegFileError::NotARegFile));
        let bad = |body: &str| parse(&format!("{HEADER}\r\n{body}"));
        // A value with no key above it.
        assert_eq!(bad("\"a\"=\"b\"\r\n"), Err(RegFileError::Syntax(2)));
        assert_eq!(
            bad("[HKEY_CURRENT_USER\\K\r\n"),
            Err(RegFileError::Syntax(2))
        );
        assert_eq!(
            bad("[HKEY_CURRENT_USER\\K]\r\nnonsense\r\n"),
            Err(RegFileError::Syntax(3))
        );
        assert_eq!(
            bad("[HKEY_CURRENT_USER\\K]\r\n\"a\"=dword:xyz\r\n"),
            Err(RegFileError::Syntax(3))
        );
        assert_eq!(
            bad("[HKEY_CURRENT_USER\\K]\r\n\"a\"=hex:0g\r\n"),
            Err(RegFileError::Syntax(3))
        );
        assert_eq!(
            bad("[HKEY_CURRENT_USER\\K]\r\n\"a\"=\"never closed\r\n"),
            Err(RegFileError::Syntax(3))
        );
    }

    #[test]
    fn a_value_reads_as_what_its_type_says() {
        assert_eq!(Value::sz("a", "text").text(), "text");
        assert_eq!(
            Value::multi_sz("a", &["x", "", "y"]).strings(),
            ["x", "", "y"]
        );
        assert!(Value::multi_sz::<&str>("a", &[]).strings().is_empty());
        assert_eq!(Value::dword("a", 42).number(), Some(42));
        assert_eq!(Value::qword("a", u64::MAX).number(), Some(u64::MAX));
        assert_eq!(Value::dword("a", 42).summary(), "0x0000002a (42)");
        assert_eq!(Value::multi_sz("a", &["x", "y"]).summary(), "x | y");
        assert_eq!(Value::binary("a", &[1, 0xff]).summary(), "01 ff");
        assert!(Value::binary("a", &[0; 100]).summary().ends_with('…'));
        assert_eq!(type_name(REG_EXPAND_SZ), "REG_EXPAND_SZ");
        assert_eq!(type_name(99), "REG_99");
    }

    #[test]
    fn f3_shows_text_lists_numbers_and_a_dump() {
        assert_eq!(Value::multi_sz("a", &["x", "y"]).full(), ["x", "y"]);
        assert_eq!(Value::dword("a", 255).full(), ["255  (0x000000ff)"]);
        assert_eq!(Value::qword("a", 1).full(), ["1  (0x0000000000000001)"]);
        let dump = Value::binary("a", b"0123456789abcdefXY\x00").full();
        assert_eq!(dump.len(), 2);
        assert_eq!(
            dump[0],
            "00000000  30 31 32 33 34 35 36 37 38 39 61 62 63 64 65 66  0123456789abcdef"
        );
        assert!(dump[1].starts_with("00000010  58 59 00 ") && dump[1].ends_with("XY."));
    }

    #[test]
    fn what_is_typed_becomes_the_bytes_of_the_type_or_is_refused() {
        assert_eq!(parse_data(REG_SZ, "ab"), Ok(vec![b'a', 0, b'b', 0, 0, 0]));
        assert_eq!(parse_data(REG_DWORD, "42"), Ok(vec![42, 0, 0, 0]));
        assert_eq!(parse_data(REG_DWORD, " 0x2A "), Ok(vec![42, 0, 0, 0]));
        assert_eq!(parse_data(REG_DWORD, "4294967295"), Ok(vec![255; 4]));
        assert_eq!(
            parse_data(REG_DWORD, "4294967296"),
            Err(DataError::TooLarge)
        );
        assert_eq!(parse_data(REG_DWORD, "-1"), Err(DataError::NotANumber));
        assert_eq!(parse_data(REG_DWORD, ""), Err(DataError::NotANumber));
        assert_eq!(parse_data(REG_DWORD, "12abc"), Err(DataError::NotANumber));
        assert_eq!(
            parse_data(REG_QWORD, "0xffffffffffffffff"),
            Ok(vec![255; 8])
        );
        assert_eq!(
            parse_data(REG_QWORD, "18446744073709551616"),
            Err(DataError::TooLarge)
        );
        assert_eq!(
            parse_data(REG_BINARY, "01 0a,FF\n10"),
            Ok(vec![1, 10, 255, 16])
        );
        assert_eq!(parse_data(REG_BINARY, "010aff"), Ok(vec![1, 10, 255]));
        assert_eq!(parse_data(REG_BINARY, ""), Ok(vec![]));
        assert_eq!(parse_data(REG_BINARY, "1"), Err(DataError::NotHex));
        assert_eq!(parse_data(REG_BINARY, "zz"), Err(DataError::NotHex));
        // What the dialog opens on is what it reads back.
        for value in [
            Value::sz("a", "text"),
            Value::dword("a", 7),
            Value::qword("a", 1 << 40),
            Value::binary("a", &[0, 1, 2]),
        ] {
            assert_eq!(parse_data(value.kind, &value.edit_text()), Ok(value.data));
        }
    }
}
