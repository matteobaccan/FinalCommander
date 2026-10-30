//! What a PDF *is*, and where inside it anything lives — `docs/spec/05-viewer-editor.md` §1.8.
//!
//! A PDF is not a document in the way a text file is one: it is a heap of numbered objects
//! with an index at the end, and what a reader wants to know about it — how many pages, what
//! size, who made it, whether it is encrypted, whether it has been edited since it was written
//! — is in that heap and nowhere on the first page. So this is the same work the structure
//! pane does for an executable: find the index, follow it to the records that matter, and
//! report them with the offsets they are at.
//!
//! It stops firmly short of *rendering*. Pulling the text out of a page means the content
//! stream's operators, the font's `ToUnicode` map and the glyph names, which is a typesetting
//! engine's worth of work and still gets a scanned page wrong; `[[decompile]]` with
//! `pdftotext` is the honest answer to that and is already there (SPEC 06). What is here is
//! the catalogue.
//!
//! Three things make this harder than a header:
//!
//! - **The index is at the end**, and may be a chain of them: every incremental save adds a
//!   new one pointing back at the one before. The newest wins, and this follows the chain only
//!   far enough to say how many there are.
//! - **Since 1.5 the index is itself a deflated object**, and so is most of the catalogue: the
//!   objects live inside *object streams*, compressed together. Reading a modern PDF's page
//!   count therefore means inflating two streams before anything can be looked up.
//! - **A broken index is the normal case.** Files get truncated, concatenated, and written by
//!   programs that count bytes wrongly. When the index does not lead anywhere this falls back
//!   to reading the file for `N G obj` and believing what it finds, which is what every repair
//!   tool does.
//!
//! Everything in the file is hostile input: a `/Length` longer than the file, a page tree that
//! points at itself, an object number of four billion. Every read is clamped, every walk is
//! capped, and nothing here recurses without a depth that runs out.

use std::collections::HashMap;
use std::fmt::Write as _;
use std::io::Read as _;

use crate::structure::{in_file, Bytes, Row, Structure, MAX_ENTRIES};

/// How far into the file the `%PDF-` marker may be. The specification says the first line;
/// Acrobat accepts it in the first kilobyte, and so do the files people actually have.
const HEADER_WINDOW: usize = 1024;
/// How much of the tail to read looking for `startxref`. The last line of a PDF is
/// `%%EOF`, and above it `startxref` and a number, but files pad, comment and append.
const TAIL: u64 = 4096;
/// The most of one stream we inflate. An object stream holding a catalogue is kilobytes; a
/// page's content is megabytes and is not read here at all.
const MAX_STREAM: usize = 16 << 20;
/// How many index sections of an incremental chain to follow.
const MAX_CHAIN: usize = 32;
/// How deep into the page tree to walk, and how many pages to list.
const MAX_PAGE_DEPTH: usize = 32;
const MAX_PAGES_LISTED: usize = 64;
/// How much of the file the fallback scan reads when the index does not work.
const MAX_SCAN: u64 = 64 << 20;

/// One parsed object. Enough of PDF's type system to read a catalogue, and no more: this does
/// not need to *write* a PDF, so there is nothing here about preserving what it did not parse.
#[derive(Debug, Clone, PartialEq)]
pub enum Obj {
    Null,
    Bool(bool),
    Num(f64),
    /// A literal or hexadecimal string, as bytes: a PDF string is not text until something
    /// says which of the two encodings it is in.
    Str(Vec<u8>),
    Name(String),
    Array(Vec<Obj>),
    Dict(Dict),
    /// `12 0 R`: the object number and its generation.
    Ref(u32, u16),
    /// A dictionary with bytes after it. `data` is where in the *file* those bytes start, so
    /// nothing is held in memory until somebody asks for the stream.
    Stream {
        dict: Dict,
        data: u64,
    },
}

/// A dictionary, in the order the file writes its keys — which is the order to show them in,
/// and cheaper to build than a map for the handful of keys any of these hold.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Dict(Vec<(String, Obj)>);

impl Dict {
    pub fn get(&self, key: &str) -> Option<&Obj> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    fn has(&self, key: &str) -> bool {
        self.get(key).is_some()
    }
}

impl Obj {
    fn as_dict(&self) -> Option<&Dict> {
        match self {
            Obj::Dict(d) => Some(d),
            Obj::Stream { dict, .. } => Some(dict),
            _ => None,
        }
    }
    fn as_num(&self) -> Option<f64> {
        match self {
            Obj::Num(n) => Some(*n),
            _ => None,
        }
    }
    fn as_int(&self) -> Option<i64> {
        self.as_num().map(|n| n as i64)
    }
    fn as_name(&self) -> Option<&str> {
        match self {
            Obj::Name(n) => Some(n),
            _ => None,
        }
    }
    fn as_array(&self) -> Option<&[Obj]> {
        match self {
            Obj::Array(a) => Some(a),
            _ => None,
        }
    }
}

// ── the lexer ───────────────────────────────────────────────────────────────────────────────

struct Lex<'a> {
    b: &'a [u8],
    pos: usize,
    /// Where in the file byte 0 of this buffer is, so a stream's data offset is a file offset.
    base: u64,
}

fn is_space(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\r' | b'\n' | b'\x0c' | b'\0')
}

fn is_delim(c: u8) -> bool {
    matches!(
        c,
        b'(' | b')' | b'<' | b'>' | b'[' | b']' | b'{' | b'}' | b'/' | b'%'
    )
}

impl<'a> Lex<'a> {
    fn new(b: &'a [u8], base: u64) -> Lex<'a> {
        Lex { b, pos: 0, base }
    }

    fn skip_space(&mut self) {
        while self.pos < self.b.len() {
            let c = self.b[self.pos];
            if is_space(c) {
                self.pos += 1;
            } else if c == b'%' {
                // A comment runs to the end of the line, and a PDF's own markers — `%%EOF`,
                // the binary marker after the header — are comments.
                while self.pos < self.b.len() && !matches!(self.b[self.pos], b'\n' | b'\r') {
                    self.pos += 1;
                }
            } else {
                return;
            }
        }
    }

    fn peek(&self) -> Option<u8> {
        self.b.get(self.pos).copied()
    }

    /// A bare word: `obj`, `stream`, `trailer`, `R`, `true`.
    fn word(&mut self) -> &'a [u8] {
        let start = self.pos;
        while self.pos < self.b.len() && !is_space(self.b[self.pos]) && !is_delim(self.b[self.pos])
        {
            self.pos += 1;
        }
        &self.b[start..self.pos]
    }

    fn expect_word(&mut self, want: &[u8]) -> bool {
        self.skip_space();
        let save = self.pos;
        if self.word() == want {
            return true;
        }
        self.pos = save;
        false
    }

    /// One object, with a depth that runs out: an array of arrays of arrays is a file built to
    /// make this recurse.
    fn obj(&mut self, depth: usize) -> Obj {
        if depth > 24 {
            return Obj::Null;
        }
        self.skip_space();
        let Some(c) = self.peek() else {
            return Obj::Null;
        };
        match c {
            b'/' => {
                self.pos += 1;
                Obj::Name(self.name())
            }
            b'(' => {
                self.pos += 1;
                Obj::Str(self.literal())
            }
            b'[' => {
                self.pos += 1;
                let mut items = Vec::new();
                loop {
                    self.skip_space();
                    match self.peek() {
                        None => break,
                        Some(b']') => {
                            self.pos += 1;
                            break;
                        }
                        _ => {}
                    }
                    let before = self.pos;
                    items.push(self.obj(depth + 1));
                    // An object that consumed nothing would loop for ever on a corrupt array.
                    if self.pos == before {
                        self.pos += 1;
                    }
                    if items.len() > 4096 {
                        break;
                    }
                }
                Obj::Array(items)
            }
            b'<' if self.b.get(self.pos + 1) == Some(&b'<') => {
                self.pos += 2;
                let dict = self.dict(depth);
                // `stream` right after a dictionary means the bytes follow, after the one
                // end-of-line the specification allows and nothing else.
                let save = self.pos;
                self.skip_space();
                if self.word() == b"stream" {
                    if self.peek() == Some(b'\r') {
                        self.pos += 1;
                    }
                    if self.peek() == Some(b'\n') {
                        self.pos += 1;
                    }
                    return Obj::Stream {
                        dict,
                        data: self.base + self.pos as u64,
                    };
                }
                self.pos = save;
                Obj::Dict(dict)
            }
            b'<' => {
                self.pos += 1;
                Obj::Str(self.hex_string())
            }
            b'+' | b'-' | b'.' | b'0'..=b'9' => self.number_or_ref(),
            _ => match self.word() {
                b"true" => Obj::Bool(true),
                b"false" => Obj::Bool(false),
                // `null`, `endobj`, `R` on its own, or a keyword this reader does not know:
                // all of them are "nothing here", which is what a corrupt file gives.
                _ => Obj::Null,
            },
        }
    }

    fn dict(&mut self, depth: usize) -> Dict {
        let mut out = Vec::new();
        loop {
            self.skip_space();
            match self.peek() {
                None => break,
                Some(b'>') => {
                    self.pos += 1;
                    if self.peek() == Some(b'>') {
                        self.pos += 1;
                    }
                    break;
                }
                Some(b'/') => {}
                // A key that is not a name: skip the byte and try again rather than give up on
                // the whole dictionary, because the keys after it are usually fine.
                _ => {
                    self.pos += 1;
                    continue;
                }
            }
            self.pos += 1;
            let key = self.name();
            let value = self.obj(depth + 1);
            out.push((key, value));
            if out.len() > 512 {
                break;
            }
        }
        Dict(out)
    }

    fn name(&mut self) -> String {
        let raw = self.word();
        if !raw.contains(&b'#') {
            return String::from_utf8_lossy(raw).to_string();
        }
        // `#20` is a space: a name may spell any byte that way.
        let mut out = Vec::with_capacity(raw.len());
        let mut i = 0;
        while i < raw.len() {
            if raw[i] == b'#' && i + 2 < raw.len() {
                let hi = (raw[i + 1] as char).to_digit(16);
                let lo = (raw[i + 2] as char).to_digit(16);
                if let (Some(hi), Some(lo)) = (hi, lo) {
                    out.push((hi * 16 + lo) as u8);
                    i += 3;
                    continue;
                }
            }
            out.push(raw[i]);
            i += 1;
        }
        String::from_utf8_lossy(&out).to_string()
    }

    fn literal(&mut self) -> Vec<u8> {
        let mut out = Vec::new();
        let mut nesting = 1usize;
        while let Some(c) = self.peek() {
            self.pos += 1;
            match c {
                b'\\' => {
                    let Some(esc) = self.peek() else { break };
                    self.pos += 1;
                    out.push(match esc {
                        b'n' => b'\n',
                        b'r' => b'\r',
                        b't' => b'\t',
                        b'b' => 8,
                        b'f' => 12,
                        b'\n' => continue,
                        b'\r' => {
                            if self.peek() == Some(b'\n') {
                                self.pos += 1;
                            }
                            continue;
                        }
                        b'0'..=b'7' => {
                            // Up to three octal digits.
                            let mut v = u32::from(esc - b'0');
                            for _ in 0..2 {
                                match self.peek() {
                                    Some(d @ b'0'..=b'7') => {
                                        v = v * 8 + u32::from(d - b'0');
                                        self.pos += 1;
                                    }
                                    _ => break,
                                }
                            }
                            (v & 0xFF) as u8
                        }
                        other => other,
                    });
                }
                b'(' => {
                    nesting += 1;
                    out.push(c);
                }
                b')' => {
                    nesting -= 1;
                    if nesting == 0 {
                        break;
                    }
                    out.push(c);
                }
                _ => out.push(c),
            }
            if out.len() > 1 << 16 {
                break;
            }
        }
        out
    }

    fn hex_string(&mut self) -> Vec<u8> {
        let mut digits = Vec::new();
        while let Some(c) = self.peek() {
            self.pos += 1;
            if c == b'>' {
                break;
            }
            if let Some(d) = (c as char).to_digit(16) {
                digits.push(d as u8);
            }
        }
        // An odd number of digits is padded with a zero, which the specification says to do.
        if digits.len() % 2 == 1 {
            digits.push(0);
        }
        digits.chunks(2).map(|p| p[0] * 16 + p[1]).collect()
    }

    /// A number, or the `12 0 R` that three of them in a row spell.
    fn number_or_ref(&mut self) -> Obj {
        let save = self.pos;
        let first = self.read_number();
        // A reference is two non-negative integers and an `R`.
        if let Some(n) = first.filter(|n| *n >= 0.0 && n.fract() == 0.0) {
            let after = self.pos;
            self.skip_space();
            if let Some(g) = self.read_number().filter(|g| *g >= 0.0 && g.fract() == 0.0) {
                self.skip_space();
                let save_word = self.pos;
                if self.word() == b"R" {
                    return Obj::Ref(n as u32, g as u16);
                }
                self.pos = save_word;
            }
            self.pos = after;
            return Obj::Num(n);
        }
        match first {
            Some(n) => Obj::Num(n),
            None => {
                self.pos = save + 1;
                Obj::Null
            }
        }
    }

    fn read_number(&mut self) -> Option<f64> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let mut digits = false;
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                digits = true;
                self.pos += 1;
            } else if c == b'.' {
                self.pos += 1;
            } else {
                break;
            }
        }
        if !digits {
            self.pos = start;
            return None;
        }
        String::from_utf8_lossy(&self.b[start..self.pos])
            .parse::<f64>()
            .ok()
    }
}

// ── the index ───────────────────────────────────────────────────────────────────────────────

#[derive(Default)]
struct Xref {
    /// Object number to where in the file it is written.
    at: HashMap<u32, u64>,
    /// Object number to the stream that holds it, and its place in that stream.
    inside: HashMap<u32, (u32, u32)>,
    trailer: Dict,
    /// `table` for the classic one, `stream` for the deflated one every file since 1.5 has.
    kind: &'static str,
    /// How many sections the chain had: one for a file written once, more for every save.
    sections: usize,
    /// Where the newest section is, for the row `Enter` follows.
    start: Option<u64>,
    /// The index did not lead anywhere and the file was read looking for objects instead.
    rebuilt: bool,
}

/// The whole of the reader's state: the index, and a cache of the object streams already
/// inflated, because the catalogue, the page tree and the first page are usually in one.
struct Pdf {
    xref: Xref,
    streams: HashMap<u32, Vec<u8>>,
    len: u64,
}

impl Pdf {
    /// The object with this number, and where in the file it is — `None` for one that lives
    /// inside an object stream, which is a place in the file no offset describes.
    fn object(&mut self, src: &mut dyn Bytes, num: u32) -> (Obj, Option<u64>) {
        if let Some(at) = self.xref.at.get(&num).copied() {
            return (self.at_offset(src, at, num), Some(at));
        }
        if let Some((container, index)) = self.xref.inside.get(&num).copied() {
            return (self.in_stream(src, container, index, num), None);
        }
        (Obj::Null, None)
    }

    /// Where to send `Enter` for an object, and what the row should say: its own offset, or —
    /// for one packed into an object stream — that stream's, because that is where its bytes
    /// really are. A reader who cannot see the catalogue at all learns nothing; one sent to
    /// the stream that holds it can at least see the file admit it is compressed.
    fn place(&self, num: u32) -> (Option<u64>, String) {
        if let Some(at) = self.xref.at.get(&num).copied() {
            return (Some(at), format!("{at:08X}"));
        }
        if let Some((container, _)) = self.xref.inside.get(&num).copied() {
            return match self.xref.at.get(&container).copied() {
                Some(at) => (Some(at), format!("{at:08X}  in object stream {container}")),
                None => (None, format!("in object stream {container}")),
            };
        }
        (None, "not in the index".to_string())
    }

    /// `N G obj` at an offset, which is what the index points at.
    fn at_offset(&mut self, src: &mut dyn Bytes, at: u64, want: u32) -> Obj {
        if at >= self.len {
            return Obj::Null;
        }
        // Enough for any dictionary worth reading here; a stream's bytes are not in it.
        let room = (self.len - at).min(64 << 10) as usize;
        let buf = src.at(at, room);
        let mut lex = Lex::new(&buf, at);
        lex.skip_space();
        let Some(num) = lex.read_number() else {
            return Obj::Null;
        };
        lex.skip_space();
        let _gen = lex.read_number();
        if !lex.expect_word(b"obj") {
            return Obj::Null;
        }
        // An index that points at the wrong object is an index to stop believing.
        if num as u32 != want {
            return Obj::Null;
        }
        lex.obj(0)
    }

    fn in_stream(&mut self, src: &mut dyn Bytes, container: u32, index: u32, want: u32) -> Obj {
        let Some(at) = self.xref.at.get(&container).copied() else {
            return Obj::Null;
        };
        let holder = self.at_offset(src, at, container);
        let Obj::Stream { dict, data } = &holder else {
            return Obj::Null;
        };
        // `N` objects, whose numbers and offsets are written in the `First` bytes in front.
        let n = dict.get("N").and_then(|o| o.as_int()).unwrap_or(0);
        let first = dict.get("First").and_then(|o| o.as_int()).unwrap_or(-1);
        if n <= 0 || first < 0 {
            return Obj::Null;
        }
        let (n, first) = (n as usize, first as usize);
        let bytes = match self.streams.get(&container) {
            Some(cached) => cached.clone(),
            None => {
                let decoded = self.stream_bytes(src, dict, *data);
                self.streams.insert(container, decoded.clone());
                decoded
            }
        };
        if first > bytes.len() {
            return Obj::Null;
        }
        let mut lex = Lex::new(&bytes[..first], 0);
        let mut pairs = Vec::with_capacity(n.min(1 << 16));
        for _ in 0..n.min(1 << 16) {
            lex.skip_space();
            let Some(num) = lex.read_number() else { break };
            lex.skip_space();
            let Some(off) = lex.read_number() else { break };
            pairs.push((num as u32, off as usize));
        }
        // The index said which slot; the slot says which object is in it. Where the two
        // disagree — and in files written by broken tools they do — the number wins.
        let slot = pairs
            .get(index as usize)
            .filter(|(num, _)| *num == want)
            .or_else(|| pairs.iter().find(|(num, _)| *num == want));
        let Some((_, off)) = slot else {
            return Obj::Null;
        };
        let start = first.saturating_add(*off);
        if start >= bytes.len() {
            return Obj::Null;
        }
        Lex::new(&bytes[start..], 0).obj(0)
    }

    /// A stream's bytes, inflated when they are deflated and handed back as they are when the
    /// filter is one this reader does not do — a row saying "not decoded" is better than a row
    /// of rubbish.
    fn stream_bytes(&mut self, src: &mut dyn Bytes, dict: &Dict, data: u64) -> Vec<u8> {
        let declared = match dict.get("Length") {
            Some(Obj::Num(n)) => *n as u64,
            // `/Length 12 0 R` is legal and common: the length is an object of its own.
            Some(Obj::Ref(num, _)) => {
                let num = *num;
                match self.object(src, num).0 {
                    Obj::Num(n) => n as u64,
                    _ => 0,
                }
            }
            _ => 0,
        };
        if data >= self.len {
            return Vec::new();
        }
        let room = (self.len - data).min(MAX_STREAM as u64);
        // A `/Length` of zero or a wrong one happens; `endstream` is the other witness, and
        // the shorter of the two is the safe answer.
        let want = if declared == 0 || declared > room {
            room
        } else {
            declared
        };
        let raw = src.at(data, want as usize);
        let filter = match dict.get("Filter") {
            Some(Obj::Name(n)) => n.clone(),
            Some(Obj::Array(a)) => a
                .first()
                .and_then(|o| o.as_name())
                .unwrap_or_default()
                .to_string(),
            _ => String::new(),
        };
        let out = match filter.as_str() {
            "" => raw,
            "FlateDecode" | "Fl" => inflate(&raw),
            // LZW, RunLength, the image codecs: not this reader's job, and a page's content
            // stream is not read here at all.
            _ => return Vec::new(),
        };
        predictor(dict, out)
    }
}

/// zlib, and then raw deflate for the files whose two-byte header is wrong — which happens,
/// and which every PDF reader tolerates.
fn inflate(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut zlib = flate2::read::ZlibDecoder::new(raw);
    if zlib.read_to_end(&mut out).is_ok() && !out.is_empty() {
        return out;
    }
    out.clear();
    let mut deflate = flate2::read::DeflateDecoder::new(raw);
    let _ = deflate.read_to_end(&mut out);
    out
}

/// Undo the PNG predictor a cross-reference stream is almost always written with: each row of
/// the table is stored as the difference from the row above it, which compresses far better
/// and has to be added back before the numbers mean anything.
fn predictor(dict: &Dict, data: Vec<u8>) -> Vec<u8> {
    let parms = match dict.get("DecodeParms") {
        Some(Obj::Dict(d)) => d.clone(),
        Some(Obj::Array(a)) => match a.iter().find_map(|o| o.as_dict()) {
            Some(d) => d.clone(),
            None => return data,
        },
        _ => return data,
    };
    let kind = parms.get("Predictor").and_then(|o| o.as_int()).unwrap_or(1);
    if kind < 10 {
        return data;
    }
    let columns = parms
        .get("Columns")
        .and_then(|o| o.as_int())
        .unwrap_or(1)
        .max(1) as usize;
    let colors = parms
        .get("Colors")
        .and_then(|o| o.as_int())
        .unwrap_or(1)
        .max(1) as usize;
    let depth = parms
        .get("BitsPerComponent")
        .and_then(|o| o.as_int())
        .unwrap_or(8)
        .max(1) as usize;
    let row = columns * colors * depth / 8;
    if row == 0 {
        return data;
    }
    let bpp = (colors * depth / 8).max(1);
    let mut out = Vec::with_capacity(data.len());
    let mut previous = vec![0u8; row];
    for chunk in data.chunks(row + 1) {
        if chunk.len() < 2 {
            break;
        }
        let tag = chunk[0];
        let mut line = chunk[1..].to_vec();
        line.resize(row, 0);
        for i in 0..row {
            let left = if i >= bpp { line[i - bpp] } else { 0 };
            let up = previous[i];
            let up_left = if i >= bpp { previous[i - bpp] } else { 0 };
            line[i] = match tag {
                0 => line[i],
                1 => line[i].wrapping_add(left),
                2 => line[i].wrapping_add(up),
                3 => line[i].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => {
                    // Paeth: whichever of the three neighbours the gradient is nearest.
                    let p = i16::from(left) + i16::from(up) - i16::from(up_left);
                    let (a, b, c) = (
                        (p - i16::from(left)).abs(),
                        (p - i16::from(up)).abs(),
                        (p - i16::from(up_left)).abs(),
                    );
                    let near = if a <= b && a <= c {
                        left
                    } else if b <= c {
                        up
                    } else {
                        up_left
                    };
                    line[i].wrapping_add(near)
                }
                _ => line[i],
            };
        }
        out.extend_from_slice(&line);
        previous = line;
    }
    out
}

/// Read the index, following the chain of earlier ones. Everything an older section says that
/// a newer one also says is already answered, so the newest is loaded first and nothing
/// overwrites it.
fn read_xref(src: &mut dyn Bytes, len: u64, start: u64) -> Xref {
    let mut xref = Xref {
        start: Some(start),
        ..Default::default()
    };
    let mut at = Some(start);
    let mut seen = Vec::new();
    while let Some(here) = at {
        if here >= len || seen.contains(&here) || seen.len() >= MAX_CHAIN {
            break;
        }
        seen.push(here);
        xref.sections += 1;
        let room = (len - here).min(64 << 10) as usize;
        let head = src.at(here, room);
        let mut lex = Lex::new(&head, here);
        lex.skip_space();
        let trailer = if lex.expect_word(b"xref") {
            if xref.kind.is_empty() {
                xref.kind = "table";
            }
            classic(&mut lex, &mut xref)
        } else {
            if xref.kind.is_empty() {
                xref.kind = "stream";
            }
            stream_section(src, len, here, &mut xref)
        };
        // The newest section's trailer is the file's trailer.
        if xref.trailer.0.is_empty() {
            xref.trailer = trailer.clone();
        }
        // A hybrid file keeps the old table for old readers and a stream beside it for new
        // ones; both are followed, the stream first because it is the complete one.
        if let Some(stm) = trailer.get("XRefStm").and_then(|o| o.as_int()) {
            if stm > 0 && (stm as u64) < len && !seen.contains(&(stm as u64)) {
                seen.push(stm as u64);
                stream_section(src, len, stm as u64, &mut xref);
            }
        }
        at = trailer
            .get("Prev")
            .and_then(|o| o.as_int())
            .filter(|p| *p > 0)
            .map(|p| p as u64);
    }
    xref
}

/// The classic table: subsections of twenty-byte entries, then `trailer` and a dictionary.
fn classic(lex: &mut Lex, xref: &mut Xref) -> Dict {
    loop {
        lex.skip_space();
        if lex.expect_word(b"trailer") {
            return match lex.obj(0) {
                Obj::Dict(d) => d,
                _ => Dict::default(),
            };
        }
        let Some(first) = lex.read_number() else {
            return Dict::default();
        };
        lex.skip_space();
        let Some(count) = lex.read_number() else {
            return Dict::default();
        };
        let first = first as u32;
        let count = (count as usize).min(1 << 20);
        for i in 0..count {
            lex.skip_space();
            let Some(offset) = lex.read_number() else {
                return Dict::default();
            };
            lex.skip_space();
            let Some(_gen) = lex.read_number() else {
                return Dict::default();
            };
            lex.skip_space();
            let kind = lex.word();
            let num = first.saturating_add(i as u32);
            // `n` is in use, `f` is free. A free entry is a hole, not an object.
            if kind == b"n" && offset > 0.0 {
                xref.at.entry(num).or_insert(offset as u64);
            }
        }
    }
}

/// The deflated index: an object whose stream is a table of fixed-width fields.
fn stream_section(src: &mut dyn Bytes, len: u64, at: u64, xref: &mut Xref) -> Dict {
    let mut pdf = Pdf {
        xref: Xref::default(),
        streams: HashMap::new(),
        len,
    };
    let room = (len - at).min(64 << 10) as usize;
    let buf = src.at(at, room);
    let mut lex = Lex::new(&buf, at);
    lex.skip_space();
    let Some(_num) = lex.read_number() else {
        return Dict::default();
    };
    lex.skip_space();
    let _gen = lex.read_number();
    if !lex.expect_word(b"obj") {
        return Dict::default();
    }
    let Obj::Stream { dict, data } = lex.obj(0) else {
        return Dict::default();
    };
    let bytes = pdf.stream_bytes(src, &dict, data);
    let widths: Vec<usize> = dict
        .get("W")
        .and_then(|o| o.as_array())
        .map(|a| {
            a.iter()
                .map(|o| o.as_int().unwrap_or(0).max(0) as usize)
                .collect()
        })
        .unwrap_or_default();
    if widths.len() < 3 {
        return dict;
    }
    let row: usize = widths.iter().sum();
    if row == 0 {
        return dict;
    }
    // `/Index` is pairs of (first object number, how many); without it the table starts at 0
    // and runs for `/Size` entries.
    let index: Vec<i64> = dict
        .get("Index")
        .and_then(|o| o.as_array())
        .map(|a| a.iter().filter_map(|o| o.as_int()).collect())
        .unwrap_or_else(|| vec![0, dict.get("Size").and_then(|o| o.as_int()).unwrap_or(0)]);

    let mut pos = 0usize;
    for pair in index.chunks(2) {
        let (first, count) = match pair {
            [a, b] => (*a as u32, (*b).max(0) as usize),
            _ => break,
        };
        for i in 0..count {
            if pos + row > bytes.len() {
                return dict;
            }
            let mut field = [0u64; 3];
            let mut p = pos;
            for (f, width) in field.iter_mut().zip(&widths) {
                let mut v = 0u64;
                for b in &bytes[p..p + width] {
                    v = (v << 8) | u64::from(*b);
                }
                *f = v;
                p += width;
            }
            pos += row;
            // A zero-width type field means type 1, which is what the specification says.
            let kind = if widths[0] == 0 { 1 } else { field[0] };
            let num = first.saturating_add(i as u32);
            match kind {
                1 => {
                    xref.at.entry(num).or_insert(field[1]);
                }
                2 => {
                    xref.inside
                        .entry(num)
                        .or_insert((field[1] as u32, field[2] as u32));
                }
                // 0 is a free object: a hole.
                _ => {}
            }
        }
    }
    dict
}

/// When the index does not lead anywhere, read the file for `N G obj` and believe that. This
/// is what a repair tool does, and a PDF that has been edited by hand or concatenated needs
/// it. A later definition of an object wins, which is what an incremental save means.
fn rebuild(src: &mut dyn Bytes, len: u64) -> Xref {
    let mut xref = Xref {
        rebuilt: true,
        kind: "rebuilt",
        ..Default::default()
    };
    const CHUNK: u64 = 1 << 20;
    // An `N G obj` may straddle a chunk boundary, so the chunks overlap by more than one can
    // be long.
    const LAP: u64 = 64;
    let mut at = 0u64;
    while at < len.min(MAX_SCAN) {
        let room = ((len - at).min(CHUNK + LAP)) as usize;
        let buf = src.at(at, room);
        let mut i = 0usize;
        while let Some(found) = memchr::memmem::find(&buf[i..], b"obj") {
            let hit = i + found;
            i = hit + 3;
            // Walk back over `G` and `N`, which is the only way to know this `obj` is the
            // keyword and not the middle of `endobj` or of a name.
            let mut p = hit;
            let back = |p: &mut usize, want_digits: bool| -> Option<u64> {
                while *p > 0 && is_space(buf[*p - 1]) {
                    *p -= 1;
                }
                let end = *p;
                while *p > 0 && buf[*p - 1].is_ascii_digit() {
                    *p -= 1;
                }
                if *p == end && want_digits {
                    return None;
                }
                String::from_utf8_lossy(&buf[*p..end]).parse::<u64>().ok()
            };
            if back(&mut p, true).is_none() {
                continue;
            }
            let Some(num) = back(&mut p, true) else {
                continue;
            };
            // And what comes before the number must be whitespace or the start: `12 0 obj`
            // inside a string is not an object.
            if p > 0 && !is_space(buf[p - 1]) && buf[p - 1] != b'>' {
                continue;
            }
            if num > 0 && num < u64::from(u32::MAX) {
                xref.at.insert(num as u32, at + p as u64);
            }
            if xref.at.len() > 1 << 20 {
                break;
            }
        }
        at += CHUNK;
    }
    // The trailer is wherever the last one is: with no index to read, the file's own
    // `trailer` keyword is the only place `/Root` is written.
    let tail_at = len.saturating_sub(TAIL * 4);
    let tail = src.at(tail_at, (len - tail_at).min(TAIL * 4) as usize);
    if let Some(found) = memchr::memmem::rfind(&tail, b"trailer") {
        let mut lex = Lex::new(&tail[found + 7..], tail_at + found as u64 + 7);
        if let Obj::Dict(d) = lex.obj(0) {
            xref.trailer = d;
        }
    }
    xref
}

// ── the pane ────────────────────────────────────────────────────────────────────────────────

/// Decode a PDF, or decide this is not one.
pub fn read(src: &mut dyn Bytes, head: &[u8], len: u64) -> Option<Structure> {
    let window = &head[..head.len().min(HEADER_WINDOW)];
    let marker = memchr::memmem::find(window, b"%PDF-")?;
    let mut rows = Vec::new();
    let mut truncated = false;

    // The version, which is the first thing on the first line.
    let version_end = window[marker..]
        .iter()
        .position(|c| matches!(c, b'\r' | b'\n' | b' '))
        .map(|p| marker + p)
        .unwrap_or(window.len());
    let version = String::from_utf8_lossy(&window[marker + 5..version_end]).to_string();
    rows.push(Row::group("Header"));
    rows.push(Row::field(
        "%PDF-",
        format!("{marker:08X}"),
        if version.is_empty() {
            "no version on the first line".to_string()
        } else {
            format!("PDF {version}")
        },
    ));
    if marker > 0 {
        rows.push(Row::field(
            "junk before the header",
            "",
            format!("{marker} bytes — every offset in this file is shifted by that much"),
        ));
    }
    // The second line of a PDF holding binary data is a comment with four bytes over 127 in
    // it, which is what tells a text-mode transfer that it must not touch this file.
    let binary = window[version_end..]
        .iter()
        .take(16)
        .filter(|c| **c >= 0x80)
        .count()
        >= 4;
    rows.push(Row::field(
        "binary marker",
        "",
        if binary { "present" } else { "absent" },
    ));

    // The index, from the last `startxref`.
    let tail_at = len.saturating_sub(TAIL);
    let tail = src.at(tail_at, (len - tail_at).min(TAIL) as usize);
    let eofs = count(&tail, b"%%EOF");
    let start = memchr::memmem::rfind(&tail, b"startxref").and_then(|found| {
        let mut lex = Lex::new(&tail[found + 9..], 0);
        lex.skip_space();
        lex.read_number().map(|n| n as u64)
    });
    if memchr::memmem::rfind(&tail, b"%%EOF").is_none() {
        truncated = true;
    }

    let mut xref = match start.filter(|at| *at < len) {
        Some(at) => read_xref(src, len, at),
        None => Xref::default(),
    };
    // No index, or one that knows nothing, or one with no `/Root`: read the file instead.
    if xref.trailer.get("Root").is_none() || xref.at.is_empty() && xref.inside.is_empty() {
        let rebuilt = rebuild(src, len);
        if !rebuilt.at.is_empty() {
            let had = xref.start;
            let sections = xref.sections;
            let kind = xref.kind;
            xref = rebuilt;
            xref.start = had;
            xref.sections = sections;
            if !kind.is_empty() {
                xref.kind = "rebuilt";
            }
        }
    }
    let objects = xref.at.len() + xref.inside.len();
    let sections = xref.sections;
    let mut pdf = Pdf {
        xref,
        streams: HashMap::new(),
        len,
    };

    // The trailer, which is the one dictionary that says where everything else is.
    rows.push(Row::group("Trailer"));
    rows.push(Row::field(
        "cross-reference",
        "",
        match pdf.xref.kind {
            "table" => "table".to_string(),
            "stream" => "stream (deflated)".to_string(),
            "rebuilt" => "unreadable — rebuilt by reading the file".to_string(),
            _ => "none found".to_string(),
        },
    ));
    if let Some(at) = pdf.xref.start {
        rows.push(Row::entry(
            "startxref",
            format!("{at:08X}"),
            in_file(at, len),
        ));
    }
    if sections > 1 {
        rows.push(Row::field(
            "/Prev chain",
            "",
            format!(
                "{sections} sections — the file has been saved {} times since it was written",
                sections - 1
            ),
        ));
    }
    if let Some(size) = pdf.xref.trailer.get("Size").and_then(|o| o.as_int()) {
        rows.push(Row::field("/Size", "", format!("{size} objects")));
    }
    rows.push(Row::field("objects indexed", "", format!("{objects}")));
    if eofs > 1 {
        rows.push(Row::field(
            "%%EOF",
            "",
            format!("{eofs} — appended revisions"),
        ));
    }

    // Encryption decides what everything below is worth: the strings of an encrypted file are
    // ciphertext, and printing them would be printing rubbish.
    let encrypted = pdf.xref.trailer.has("Encrypt");
    if encrypted {
        let which = pdf.xref.trailer.get("Encrypt").cloned();
        let enc = follow(&mut pdf, src, which);
        rows.push(Row::group("Encryption"));
        if let Some(d) = enc.as_dict() {
            let filter = d.get("Filter").and_then(|o| o.as_name()).unwrap_or("?");
            let v = d.get("V").and_then(|o| o.as_int()).unwrap_or(0);
            let r = d.get("R").and_then(|o| o.as_int()).unwrap_or(0);
            let bits = d.get("Length").and_then(|o| o.as_int()).unwrap_or(40);
            rows.push(Row::field("/Filter", "", filter));
            rows.push(Row::field("/V, /R", "", format!("{v}, {r}")));
            rows.push(Row::field("/Length", "", format!("{bits} bit")));
            if let Some(p) = d.get("P").and_then(|o| o.as_int()) {
                rows.push(Row::field(
                    "/P",
                    format!("{:08X}", p as i32),
                    permissions(p as i32),
                ));
            }
            // An empty owner password is the usual case: the file opens, and only the
            // permissions above are claimed to be enforced.
            rows.push(Row::field(
                "strings and streams",
                "",
                "encrypted — the text below is not decrypted here",
            ));
        }
    }

    // The catalogue, and the page tree under it.
    let root_ref = pdf.xref.trailer.get("Root").cloned();
    let root = follow(&mut pdf, src, root_ref.clone());
    let catalog = root.as_dict().cloned().unwrap_or_default();
    let pages_ref = catalog.get("Pages").cloned();
    let pages = follow(&mut pdf, src, pages_ref.clone());
    let count_pages = pages
        .as_dict()
        .and_then(|d| d.get("Count"))
        .and_then(|o| o.as_int());

    rows.push(Row::group("Document"));
    match count_pages {
        Some(n) => rows.push(Row::field("/Pages /Count", "", format!("{n}"))),
        None if !catalog.0.is_empty() => {
            rows.push(Row::field("/Pages /Count", "", "(unreadable)"));
        }
        None => {}
    }
    if let Some(v) = catalog.get("Version").and_then(|o| o.as_name()) {
        // A catalogue may say a later version than the header does, and then it wins.
        rows.push(Row::field("/Version", "", format!("PDF {v}")));
    }
    if let Some(lang) = catalog.get("Lang") {
        rows.push(Row::field("/Lang", "", text_of(lang)));
    }
    if let Some(mode) = catalog.get("PageMode").and_then(|o| o.as_name()) {
        rows.push(Row::field("/PageMode", "", page_mode(mode)));
    }
    if catalog.has("AcroForm") {
        rows.push(Row::field("/AcroForm", "", "a form to fill in"));
    }
    if catalog.has("Outlines") {
        rows.push(Row::field("/Outlines", "", "bookmarks"));
    }
    let tagged = follow(&mut pdf, src, catalog.get("MarkInfo").cloned())
        .as_dict()
        .and_then(|d| d.get("Marked").cloned())
        == Some(Obj::Bool(true));
    if tagged {
        rows.push(Row::field(
            "/MarkInfo /Marked",
            "",
            "tagged for accessibility",
        ));
    }
    if catalog.has("Names") {
        let names = follow(&mut pdf, src, catalog.get("Names").cloned());
        if let Some(d) = names.as_dict() {
            if d.has("EmbeddedFiles") {
                rows.push(Row::field(
                    "/Names /EmbeddedFiles",
                    "",
                    "the file carries attachments",
                ));
            }
        }
    }

    // The first page's size, which is the question somebody looking at a PDF has after the
    // page count. `/MediaBox` is inherited, so the first page that states one wins.
    let first = first_page(&mut pdf, src, pages_ref.clone());
    if let Some((page, at)) = &first {
        if let Some(size) = page
            .get("MediaBox")
            .cloned()
            .or_else(|| pages.as_dict().and_then(|d| d.get("MediaBox").cloned()))
            .map(|b| follow(&mut pdf, src, Some(b)))
            .and_then(|b| media_box(&b))
        {
            rows.push(Row::field("/MediaBox", "", size));
        }
        if let Some(rotate) = page
            .get("Rotate")
            .and_then(|o| o.as_int())
            .filter(|r| *r != 0)
        {
            rows.push(Row::field("/Rotate", "", format!("{rotate}°")));
        }
        let _ = at;
    }

    // `/Info`, the old place for the title and the producer. The newer one is an XMP packet,
    // which is a whole XML document and is only reported as present.
    let info_ref = pdf.xref.trailer.get("Info").cloned();
    let info = follow(&mut pdf, src, info_ref.clone());
    if let Some(d) = info.as_dict() {
        let fields: Vec<(&str, String)> = [
            "Title",
            "Author",
            "Subject",
            "Keywords",
            "Creator",
            "Producer",
            "CreationDate",
            "ModDate",
        ]
        .into_iter()
        .filter_map(|key| {
            let value = d.get(key)?;
            let shown = if encrypted {
                "(encrypted)".to_string()
            } else if key.ends_with("Date") {
                pdf_date(&text_of(value))
            } else {
                text_of(value)
            };
            (!shown.is_empty()).then_some((key, shown))
        })
        .collect();
        if !fields.is_empty() {
            rows.push(Row::group("Info"));
            for (key, value) in fields {
                rows.push(Row::field(format!("/{key}"), "", value));
            }
        }
    }
    if catalog.has("Metadata") {
        rows.push(Row::field("/Metadata", "", "an XMP packet (not read here)"));
    }

    // What `Enter` can go to: the objects a reader would want to look at in the hex pane.
    let number_of = |obj: &Option<Obj>| match obj {
        Some(Obj::Ref(num, _)) => Some(*num),
        _ => None,
    };
    let named = [
        ("/Root (catalogue)", number_of(&root_ref)),
        ("/Info", number_of(&info_ref)),
        ("/Pages (tree root)", number_of(&pages_ref)),
    ];
    let named: Vec<(&str, u32)> = named
        .into_iter()
        .filter_map(|(label, num)| num.map(|n| (label, n)))
        .collect();
    if !named.is_empty() {
        rows.push(Row::group("Objects"));
        for (label, num) in named {
            let (at, cell) = pdf.place(num);
            rows.push(Row::entry(
                format!("  {label}"),
                cell,
                at.and_then(|at| in_file(at, len)),
            ));
        }
    }

    // And the pages themselves, which is the navigation this pane is for: the row is the page,
    // the offset is where its object is written.
    let listed = page_numbers(&mut pdf, src, pages_ref);
    if !listed.is_empty() {
        rows.push(Row::group("Pages"));
        for (i, num) in listed.iter().copied().enumerate() {
            let (at, cell) = match num {
                Some(num) => pdf.place(num),
                // A page written inline in its parent's `/Kids` has no object of its own.
                None => (None, "written inline".to_string()),
            };
            rows.push(Row::entry(
                format!("  page {}", i + 1),
                cell,
                at.and_then(|at| in_file(at, len)),
            ));
        }
        if let Some(n) = count_pages {
            if n as usize > listed.len() {
                rows.push(Row::field(
                    "  …",
                    "",
                    format!("{} more", n as usize - listed.len()),
                ));
            }
        }
    }

    let mut summary = format!("PDF {version}");
    if let Some(n) = count_pages {
        let _ = write!(summary, " · {n} page{}", if n == 1 { "" } else { "s" });
    }
    if let Some((page, _)) = &first {
        if let Some(name) = page
            .get("MediaBox")
            .cloned()
            .or_else(|| pages.as_dict().and_then(|d| d.get("MediaBox").cloned()))
            .map(|b| follow(&mut pdf, src, Some(b)))
            .and_then(|b| paper_name(&b))
        {
            let _ = write!(summary, " · {name}");
        }
    }
    if encrypted {
        summary.push_str(" · encrypted");
    }
    if pdf.xref.rebuilt {
        summary.push_str(" · index rebuilt");
    }
    if sections > 1 {
        let _ = write!(summary, " · {sections} revisions");
    }

    Some(Structure {
        format: "PDF",
        summary,
        rows,
        // A document has no code and loads at no address.
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated,
    })
}

/// A value, with a reference followed once. Everything in a PDF is either the thing or a
/// pointer to it, and a reader that forgets this reads half a file.
fn follow(pdf: &mut Pdf, src: &mut dyn Bytes, obj: Option<Obj>) -> Obj {
    match obj {
        Some(Obj::Ref(num, _)) => pdf.object(src, num).0,
        Some(other) => other,
        None => Obj::Null,
    }
}

/// The first page of the tree: down the first kid that is not empty, until a `/Page`.
fn first_page(
    pdf: &mut Pdf,
    src: &mut dyn Bytes,
    root: Option<Obj>,
) -> Option<(Dict, Option<u64>)> {
    let mut here = root;
    let mut at = None;
    for _ in 0..MAX_PAGE_DEPTH {
        if let Some(Obj::Ref(num, _)) = &here {
            at = pdf.xref.at.get(num).copied();
        }
        let node = follow(pdf, src, here.clone());
        let dict = node.as_dict()?.clone();
        match dict.get("Type").and_then(|o| o.as_name()) {
            Some("Page") => return Some((dict, at)),
            _ => {
                let kids = follow(pdf, src, dict.get("Kids").cloned());
                let first = kids.as_array()?.first()?.clone();
                here = Some(first);
            }
        }
    }
    None
}

/// Which object each page is, in order, up to the cap. A tree of a thousand nodes is walked
/// only as far as the pages this pane will show.
fn page_numbers(pdf: &mut Pdf, src: &mut dyn Bytes, root: Option<Obj>) -> Vec<Option<u32>> {
    let mut out = Vec::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((node, depth)) = stack.pop() {
        if out.len() >= MAX_PAGES_LISTED || depth > MAX_PAGE_DEPTH {
            break;
        }
        let num = match &node {
            Some(Obj::Ref(num, _)) => Some(*num),
            _ => None,
        };
        let resolved = follow(pdf, src, node);
        let Some(dict) = resolved.as_dict() else {
            continue;
        };
        match dict.get("Type").and_then(|o| o.as_name()) {
            Some("Page") => out.push(num),
            _ => {
                let kids = follow(pdf, src, dict.get("Kids").cloned());
                let Some(list) = kids.as_array() else {
                    continue;
                };
                // Pushed in reverse so the pages come out in the order the file has them.
                for kid in list.iter().rev().take(MAX_ENTRIES) {
                    stack.push((Some(kid.clone()), depth + 1));
                }
            }
        }
    }
    out
}

/// `[0 0 595 842]` as something a reader recognises: the size in points, in millimetres, and
/// the name of the paper when it is one.
fn media_box(obj: &Obj) -> Option<String> {
    let (w, h) = box_size(obj)?;
    let mm = |pt: f64| pt * 25.4 / 72.0;
    let mut out = format!("{:.0} × {:.0} pt  ({:.0} × {:.0} mm)", w, h, mm(w), mm(h));
    if let Some(name) = paper_name(obj) {
        let _ = write!(out, " — {name}");
    }
    Some(out)
}

fn box_size(obj: &Obj) -> Option<(f64, f64)> {
    let a = obj.as_array()?;
    if a.len() < 4 {
        return None;
    }
    let n: Vec<f64> = a.iter().take(4).filter_map(|o| o.as_num()).collect();
    if n.len() < 4 {
        return None;
    }
    Some(((n[2] - n[0]).abs(), (n[3] - n[1]).abs()))
}

/// The paper a page is, when it is a paper somebody has a name for. Two points of tolerance,
/// because every generator rounds the millimetres differently.
fn paper_name(obj: &Obj) -> Option<String> {
    let (w, h) = box_size(obj)?;
    let (short, long) = if w <= h { (w, h) } else { (h, w) };
    let landscape = w > h;
    const PAPERS: &[(&str, f64, f64)] = &[
        ("A3", 841.89, 1190.55),
        ("A4", 595.28, 841.89),
        ("A5", 419.53, 595.28),
        ("A6", 297.64, 419.53),
        ("B5", 498.90, 708.66),
        ("Letter", 612.0, 792.0),
        ("Legal", 612.0, 1008.0),
        ("Tabloid", 792.0, 1224.0),
        ("Executive", 521.86, 756.0),
    ];
    for (name, pw, ph) in PAPERS {
        if (short - pw).abs() <= 2.0 && (long - ph).abs() <= 2.0 {
            return Some(if landscape {
                format!("{name} landscape")
            } else {
                (*name).to_string()
            });
        }
    }
    None
}

/// A PDF string as text. Two encodings: UTF-16 when it starts with a byte order mark, and
/// PDFDocEncoding — which is Latin-1 for everything a title ever holds — otherwise.
fn text_of(obj: &Obj) -> String {
    let bytes = match obj {
        Obj::Str(b) => b.clone(),
        Obj::Name(n) => return n.clone(),
        Obj::Num(n) => return format!("{n}"),
        Obj::Bool(b) => return format!("{b}"),
        _ => return String::new(),
    };
    let text = if bytes.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|p| p.len() == 2)
            .map(|p| u16::from_be_bytes([p[0], p[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else if bytes.starts_with(&[0xFF, 0xFE]) {
        let units: Vec<u16> = bytes[2..]
            .chunks(2)
            .filter(|p| p.len() == 2)
            .map(|p| u16::from_le_bytes([p[0], p[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        bytes.iter().map(|b| *b as char).collect()
    };
    // A control character in a title would be drawn as a hole in the pane, and a title is
    // one line by definition.
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

/// `D:20240131142530+01'00'` as a date somebody can read. Anything that is not that shape is
/// handed back as it was written: a date this does not understand is still what the file says.
fn pdf_date(raw: &str) -> String {
    let s = raw.trim();
    let digits = s.strip_prefix("D:").unwrap_or(s);
    let num =
        |from: usize, n: usize| -> Option<u32> { digits.get(from..from + n)?.parse::<u32>().ok() };
    let Some(year) = num(0, 4) else {
        return s.to_string();
    };
    let mut out = format!("{year:04}");
    if let Some(month) = num(4, 2) {
        let _ = write!(out, "-{month:02}");
        if let Some(day) = num(6, 2) {
            let _ = write!(out, "-{day:02}");
        }
    }
    if let Some(hour) = num(8, 2) {
        let _ = write!(out, " {hour:02}");
        let _ = write!(out, ":{:02}", num(10, 2).unwrap_or(0));
        let _ = write!(out, ":{:02}", num(12, 2).unwrap_or(0));
    }
    // The zone, written with apostrophes: `+01'00'`.
    if let Some(rest) = digits.get(14..) {
        let zone: String = rest.chars().filter(|c| *c != '\'').collect();
        if !zone.is_empty() {
            let zone = if zone == "Z" {
                " UTC".to_string()
            } else if zone.len() >= 3 {
                format!(" {}:{}", &zone[..3], zone[3..].get(..2).unwrap_or("00"))
            } else {
                format!(" {zone}")
            };
            out.push_str(&zone);
        }
    }
    out
}

/// `/P`, the permissions an encrypted file claims. They are a bitfield of what is *allowed*,
/// counted from bit 3, and every bit this reader does not name is one nothing enforces.
fn permissions(p: i32) -> String {
    let mut allowed = Vec::new();
    for (bit, what) in [
        (3, "print"),
        (4, "modify"),
        (5, "copy text"),
        (6, "annotate"),
        (9, "fill in forms"),
        (10, "copy for accessibility"),
        (11, "assemble"),
        (12, "print at full resolution"),
    ] {
        if p & (1 << (bit - 1)) != 0 {
            allowed.push(what);
        }
    }
    if allowed.is_empty() {
        return "nothing allowed".to_string();
    }
    allowed.join(", ")
}

fn page_mode(mode: &str) -> String {
    match mode {
        "UseNone" => "open with no panel".to_string(),
        "UseOutlines" => "open with the bookmarks".to_string(),
        "UseThumbs" => "open with the thumbnails".to_string(),
        "FullScreen" => "open full screen".to_string(),
        "UseOC" => "open with the layers".to_string(),
        "UseAttachments" => "open with the attachments".to_string(),
        other => other.to_string(),
    }
}

fn count(haystack: &[u8], needle: &[u8]) -> usize {
    memchr::memmem::find_iter(haystack, needle).count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure;
    use std::io::Write as _;

    /// A PDF put together the way a writer does it: objects one after another, then the table
    /// of where each one landed. Nothing here hard-codes an offset, because the point of the
    /// tests below is that the reader follows the file's own arithmetic.
    #[derive(Default)]
    struct Build {
        out: Vec<u8>,
        at: Vec<(u32, u64)>,
    }

    impl Build {
        fn new(version: &str) -> Build {
            let mut b = Build::default();
            b.out
                .extend_from_slice(format!("%PDF-{version}\n").as_bytes());
            // The binary marker: four bytes over 127, as every real writer puts there.
            b.out.extend_from_slice(b"%\xE2\xE3\xCF\xD3\n");
            b
        }

        fn obj(&mut self, num: u32, body: &str) -> &mut Build {
            self.at.push((num, self.out.len() as u64));
            self.out
                .extend_from_slice(format!("{num} 0 obj\n{body}\nendobj\n").as_bytes());
            self
        }

        fn stream(&mut self, num: u32, dict: &str, data: &[u8]) -> &mut Build {
            self.at.push((num, self.out.len() as u64));
            self.out.extend_from_slice(
                format!("{num} 0 obj\n<< {dict} /Length {} >>\nstream\n", data.len()).as_bytes(),
            );
            self.out.extend_from_slice(data);
            self.out.extend_from_slice(b"\nendstream\nendobj\n");
            self
        }

        /// The classic table, and the trailer under it.
        fn table(&mut self, trailer: &str) -> Vec<u8> {
            let start = self.out.len();
            self.at.sort_by_key(|(num, _)| *num);
            let highest = self.at.last().map(|(n, _)| *n).unwrap_or(0);
            self.out.extend_from_slice(b"xref\n");
            self.out
                .extend_from_slice(format!("0 {}\n", highest + 1).as_bytes());
            // Object zero is always the head of the free list.
            self.out.extend_from_slice(b"0000000000 65535 f \n");
            for num in 1..=highest {
                match self.at.iter().find(|(n, _)| *n == num) {
                    Some((_, at)) => self
                        .out
                        .extend_from_slice(format!("{at:010} 00000 n \n").as_bytes()),
                    None => self.out.extend_from_slice(b"0000000000 65535 f \n"),
                }
            }
            self.out.extend_from_slice(
                format!(
                    "trailer\n<< /Size {} {trailer} >>\nstartxref\n{start}\n%%EOF\n",
                    highest + 1
                )
                .as_bytes(),
            );
            std::mem::take(&mut self.out)
        }

        fn offset_of(&self, num: u32) -> u64 {
            self.at
                .iter()
                .find(|(n, _)| *n == num)
                .map(|(_, at)| *at)
                .expect("the builder wrote this object")
        }
    }

    fn deflate(raw: &[u8]) -> Vec<u8> {
        let mut e = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        e.write_all(raw).unwrap();
        e.finish().unwrap()
    }

    /// A 1.4 file with a table: two pages, a title, A4.
    fn classic_pdf() -> Vec<u8> {
        let mut b = Build::new("1.4");
        b.obj(
            1,
            "<< /Type /Catalog /Pages 2 0 R /PageMode /UseOutlines >>",
        );
        b.obj(
            2,
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 595.28 841.89] >>",
        );
        b.obj(3, "<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>");
        b.obj(4, "<< /Type /Page /Parent 2 0 R >>");
        b.stream(5, "", b"BT (ciao) Tj ET");
        b.obj(
            6,
            "<< /Title (Relazione annuale) /Author (Matteo Baccan) \
             /Producer (FinalCommander) /CreationDate (D:20240131142530+01'00') >>",
        );
        b.table("/Root 1 0 R /Info 6 0 R")
    }

    fn structure_of(bytes: &[u8]) -> Structure {
        let mut src: &[u8] = bytes;
        structure::read(&mut src).expect("this should be read as a PDF")
    }

    fn value_of<'a>(s: &'a Structure, label: &str) -> &'a str {
        s.rows
            .iter()
            .find(|r| r.label.trim() == label)
            .map(|r| r.value.as_str())
            .unwrap_or_else(|| {
                panic!(
                    "no row {label:?} in {:?}",
                    s.rows.iter().map(|r| &r.label).collect::<Vec<_>>()
                )
            })
    }

    fn row<'a>(s: &'a Structure, label: &str) -> Option<&'a Row> {
        s.rows.iter().find(|r| r.label.trim() == label)
    }

    #[test]
    fn a_pdf_with_a_table_gives_its_pages_its_paper_and_who_made_it() {
        let s = structure_of(&classic_pdf());
        assert_eq!(s.format, "PDF");
        assert!(!s.truncated, "{}", s.summary);
        assert_eq!(s.summary, "PDF 1.4 · 2 pages · A4");

        assert_eq!(value_of(&s, "%PDF-"), "PDF 1.4");
        assert_eq!(value_of(&s, "binary marker"), "present");
        assert_eq!(value_of(&s, "cross-reference"), "table");
        assert_eq!(value_of(&s, "/Size"), "7 objects");
        assert_eq!(value_of(&s, "/Pages /Count"), "2");
        assert_eq!(value_of(&s, "/PageMode"), "open with the bookmarks");
        assert_eq!(
            value_of(&s, "/MediaBox"),
            "595 × 842 pt  (210 × 297 mm) — A4"
        );
        assert_eq!(value_of(&s, "/Title"), "Relazione annuale");
        assert_eq!(value_of(&s, "/Author"), "Matteo Baccan");
        assert_eq!(value_of(&s, "/CreationDate"), "2024-01-31 14:25:30 +01:00");
        // One revision: a file written once and not saved again says nothing about a chain.
        assert!(row(&s, "/Prev chain").is_none(), "{}", s.summary);
    }

    #[test]
    fn the_rows_enter_follows_point_at_the_objects_they_name() {
        let file = classic_pdf();
        let mut b = Build::new("1.4");
        // The same objects in the same order, to know where they are.
        b.obj(
            1,
            "<< /Type /Catalog /Pages 2 0 R /PageMode /UseOutlines >>",
        );
        b.obj(
            2,
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 595.28 841.89] >>",
        );
        b.obj(3, "<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>");
        b.obj(4, "<< /Type /Page /Parent 2 0 R >>");
        b.stream(5, "", b"BT (ciao) Tj ET");
        b.obj(
            6,
            "<< /Title (Relazione annuale) /Author (Matteo Baccan) \
             /Producer (FinalCommander) /CreationDate (D:20240131142530+01'00') >>",
        );
        let s = structure_of(&file);

        let catalogue = row(&s, "/Root (catalogue)").unwrap();
        assert_eq!(catalogue.offset, Some(b.offset_of(1)));
        assert_eq!(row(&s, "/Info").unwrap().offset, Some(b.offset_of(6)));
        assert_eq!(
            row(&s, "/Pages (tree root)").unwrap().offset,
            Some(b.offset_of(2))
        );
        // The pages, in the order the file lists them, each at its own object.
        assert_eq!(row(&s, "page 1").unwrap().offset, Some(b.offset_of(3)));
        assert_eq!(row(&s, "page 2").unwrap().offset, Some(b.offset_of(4)));
        // And the bytes there really are that object.
        let at = catalogue.offset.unwrap() as usize;
        assert!(
            file[at..].starts_with(b"1 0 obj"),
            "{:?}",
            String::from_utf8_lossy(&file[at..at + 16])
        );
    }

    /// A 1.5 file: the index is a deflated stream and the catalogue lives inside an object
    /// stream, which is how every PDF written in the last twenty years is put together.
    fn modern_pdf() -> Vec<u8> {
        let mut b = Build::new("1.5");
        // The three objects that go inside the object stream, and their offsets within it.
        let bodies = [
            (
                1u32,
                "<< /Type /Catalog /Pages 2 0 R /Lang (it-IT) >>".to_string(),
            ),
            (
                2u32,
                "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 612 792] >>".to_string(),
            ),
            (3u32, "<< /Type /Page /Parent 2 0 R >>".to_string()),
        ];
        let mut pairs = String::new();
        let mut payload = String::new();
        for (num, body) in &bodies {
            pairs.push_str(&format!("{num} {} ", payload.len()));
            payload.push_str(body);
            payload.push('\n');
        }
        let first = pairs.len();
        let objstm = deflate(format!("{pairs}{payload}").as_bytes());
        b.stream(
            4,
            &format!("/Type /ObjStm /N 3 /First {first} /Filter /FlateDecode"),
            &objstm,
        );
        let objstm_at = b.offset_of(4);

        // The index itself, as a stream of seven-byte rows: type, offset, extra.
        let xref_at = b.out.len() as u64;
        let mut table = Vec::new();
        let mut push = |kind: u8, two: u32, three: u16| {
            table.push(kind);
            table.extend_from_slice(&two.to_be_bytes());
            table.extend_from_slice(&three.to_be_bytes());
        };
        push(2, 4, 0); // object 1 is slot 0 of stream 4
        push(2, 4, 1); // object 2 is slot 1
        push(2, 4, 2); // object 3 is slot 2
        push(1, objstm_at as u32, 0); // object 4 is written in the file
        push(1, xref_at as u32, 0); // object 5 is this very stream
        let packed = deflate(&table);
        b.stream(
            5,
            "/Type /XRef /W [1 4 2] /Index [1 5] /Size 6 /Root 1 0 R /Filter /FlateDecode",
            &packed,
        );
        b.out
            .extend_from_slice(format!("startxref\n{xref_at}\n%%EOF\n").as_bytes());
        std::mem::take(&mut b.out)
    }

    #[test]
    fn a_modern_pdf_is_read_through_its_deflated_index_and_object_stream() {
        let s = structure_of(&modern_pdf());
        assert!(!s.truncated, "{}", s.summary);
        assert_eq!(value_of(&s, "cross-reference"), "stream (deflated)");
        // The page count is inside two layers of deflate, and this is the proof both were
        // undone: the number is only in the object stream.
        assert_eq!(value_of(&s, "/Pages /Count"), "1");
        assert_eq!(value_of(&s, "/Lang"), "it-IT");
        assert_eq!(s.summary, "PDF 1.5 · 1 page · Letter");
        // The catalogue is not written in the file: it is packed into object stream 4, and
        // that is where the row sends `Enter`, because that is where its bytes are.
        let catalogue = row(&s, "/Root (catalogue)").unwrap();
        assert!(
            catalogue.value.ends_with("  in object stream 4"),
            "{}",
            catalogue.value
        );
        assert!(catalogue.offset.is_some());
        // And so is the page, which is in the same stream.
        assert_eq!(row(&s, "page 1").unwrap().value, catalogue.value);
    }

    #[test]
    fn an_index_that_leads_nowhere_is_rebuilt_by_reading_the_file() {
        let mut file = classic_pdf();
        // Point `startxref` at the middle of the file, which is what a truncated download or
        // a file edited by hand looks like.
        let at = memchr::memmem::rfind(&file, b"startxref").unwrap();
        let eof = file.len();
        file.splice(at..eof, b"startxref\n9\n%%EOF\n".iter().copied());
        let s = structure_of(&file);
        assert_eq!(
            value_of(&s, "cross-reference"),
            "unreadable — rebuilt by reading the file"
        );
        assert!(s.summary.contains("index rebuilt"), "{}", s.summary);
        // And it still answers the question: the objects are all there, the index was not.
        assert_eq!(value_of(&s, "/Pages /Count"), "2");
        assert_eq!(value_of(&s, "/Title"), "Relazione annuale");
    }

    #[test]
    fn an_incremental_save_is_counted_not_hidden() {
        let base = classic_pdf();
        let mut file = base.clone();
        let first_xref = {
            let at = memchr::memmem::rfind(&base, b"startxref").unwrap();
            let mut lex = Lex::new(&base[at + 9..], 0);
            lex.skip_space();
            lex.read_number().unwrap() as u64
        };
        // A second generation of object 6, and a table of its own that points back.
        let at = file.len() as u64;
        file.extend_from_slice(
            b"6 0 obj\n<< /Title (Relazione annuale, seconda stesura) >>\nendobj\n",
        );
        let xref = file.len();
        file.extend_from_slice(format!("xref\n0 1\n0000000000 65535 f \n6 1\n{at:010} 00000 n \ntrailer\n<< /Size 7 /Root 1 0 R /Info 6 0 R /Prev {first_xref} >>\nstartxref\n{xref}\n%%EOF\n").as_bytes());

        let s = structure_of(&file);
        assert_eq!(
            value_of(&s, "/Prev chain"),
            "2 sections — the file has been saved 1 times since it was written"
        );
        assert!(s.summary.contains("2 revisions"), "{}", s.summary);
        // The newest definition of the object wins, which is the whole point of a chain.
        assert_eq!(value_of(&s, "/Title"), "Relazione annuale, seconda stesura");
        assert_eq!(value_of(&s, "%%EOF"), "2 — appended revisions");
    }

    #[test]
    fn an_encrypted_file_says_what_it_allows_and_does_not_print_ciphertext() {
        let mut b = Build::new("1.6");
        b.obj(1, "<< /Type /Catalog /Pages 2 0 R >>");
        b.obj(
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 /MediaBox [0 0 595 842] >>",
        );
        b.obj(3, "<< /Type /Page /Parent 2 0 R >>");
        // What `/Title` holds in an encrypted file is ciphertext, and showing it as a title
        // would be showing rubbish.
        b.obj(4, "<< /Title <DEADBEEF> >>");
        b.obj(5, "<< /Filter /Standard /V 4 /R 4 /Length 128 /P -1340 >>");
        let file = b.table("/Root 1 0 R /Info 4 0 R /Encrypt 5 0 R");
        let s = structure_of(&file);
        assert!(s.summary.contains("encrypted"), "{}", s.summary);
        assert_eq!(value_of(&s, "/Filter"), "Standard");
        assert_eq!(value_of(&s, "/V, /R"), "4, 4");
        assert_eq!(value_of(&s, "/Length"), "128 bit");
        // -1340 allows printing and copying for accessibility, and nothing else.
        assert_eq!(
            value_of(&s, "/P"),
            "print, copy for accessibility, print at full resolution"
        );
        assert_eq!(value_of(&s, "/Title"), "(encrypted)");
    }

    #[test]
    fn a_file_that_stops_before_its_end_marker_says_so() {
        let whole = classic_pdf();
        let cut = &whole[..whole.len() - 40];
        let s = structure_of(cut);
        assert!(s.truncated, "a PDF with no %%EOF has been cut");
    }

    #[test]
    fn junk_in_front_of_the_header_is_reported_because_every_offset_moves() {
        let mut file = b"Received: from a mail server\r\n\r\n".to_vec();
        let shift = file.len();
        file.extend_from_slice(&classic_pdf());
        let s = structure_of(&file);
        assert_eq!(
            value_of(&s, "junk before the header"),
            format!("{shift} bytes — every offset in this file is shifted by that much")
        );
        // The index's own offsets are now wrong by that much, so the reader falls back to
        // reading the file — and still answers.
        assert_eq!(value_of(&s, "/Pages /Count"), "2");
    }

    #[test]
    fn what_is_not_a_pdf_is_not_read_as_one() {
        for file in [
            &b"Just some prose.\n"[..],
            &b"%PD"[..],
            &b"\x7fELF"[..],
            // The marker, but past the window a PDF reader looks in.
            &[&[b'x'; 2048][..], b"%PDF-1.7\n"].concat()[..],
        ] {
            let mut src = file;
            assert!(
                read(&mut src, file, file.len() as u64).is_none(),
                "{:?}",
                String::from_utf8_lossy(&file[..file.len().min(8)])
            );
        }
    }

    // ── the small conversions, each with the case that caught it ───────────────────────────

    #[test]
    fn a_pdf_date_reads_as_a_date_and_anything_else_reads_as_itself() {
        assert_eq!(
            pdf_date("D:20240131142530+01'00'"),
            "2024-01-31 14:25:30 +01:00"
        );
        assert_eq!(pdf_date("D:20240131142530Z"), "2024-01-31 14:25:30 UTC");
        assert_eq!(pdf_date("D:202401"), "2024-01");
        assert_eq!(pdf_date("D:2024"), "2024");
        // A writer that put something else in the field: show what it says.
        assert_eq!(pdf_date("marted\u{ec} scorso"), "marted\u{ec} scorso");
        assert_eq!(pdf_date(""), "");
    }

    #[test]
    fn a_page_is_named_after_its_paper_when_it_is_a_paper() {
        let a4 = Obj::Array(vec![
            Obj::Num(0.0),
            Obj::Num(0.0),
            Obj::Num(595.28),
            Obj::Num(841.89),
        ]);
        assert_eq!(paper_name(&a4).as_deref(), Some("A4"));
        let landscape = Obj::Array(vec![
            Obj::Num(0.0),
            Obj::Num(0.0),
            Obj::Num(841.89),
            Obj::Num(595.28),
        ]);
        assert_eq!(paper_name(&landscape).as_deref(), Some("A4 landscape"));
        let odd = Obj::Array(vec![
            Obj::Num(0.0),
            Obj::Num(0.0),
            Obj::Num(100.0),
            Obj::Num(200.0),
        ]);
        assert_eq!(paper_name(&odd), None);
        // And a box written with an origin that is not zero is still its own size.
        let shifted = Obj::Array(vec![
            Obj::Num(10.0),
            Obj::Num(10.0),
            Obj::Num(622.0),
            Obj::Num(802.0),
        ]);
        assert_eq!(paper_name(&shifted).as_deref(), Some("Letter"));
    }

    #[test]
    fn a_title_in_utf16_is_read_as_text_and_not_as_bytes() {
        let utf16 = {
            let mut b = vec![0xFE, 0xFF];
            for u in "Perché".encode_utf16() {
                b.extend_from_slice(&u.to_be_bytes());
            }
            Obj::Str(b)
        };
        assert_eq!(text_of(&utf16), "Perché");
        // Latin-1, which is what PDFDocEncoding is for anything in a title.
        assert_eq!(
            text_of(&Obj::Str(vec![0x50, 0x65, 0x72, 0x63, 0x68, 0xE9])),
            "Perché"
        );
        // A control character would be drawn as a hole in the pane.
        assert_eq!(text_of(&Obj::Str(b"due\nrighe".to_vec())), "due righe");
    }

    #[test]
    fn the_permissions_are_the_bits_the_file_allows() {
        assert_eq!(permissions(-1), "print, modify, copy text, annotate, fill in forms, copy for accessibility, assemble, print at full resolution");
        assert_eq!(permissions(0), "nothing allowed");
        assert_eq!(permissions(1 << 2), "print");
    }

    #[test]
    fn the_png_predictor_is_undone_before_the_table_is_believed() {
        // Three rows of four bytes, each stored as the difference from the row above: the
        // layout every cross-reference stream since 1.5 is written in.
        let dict = Dict(vec![(
            "DecodeParms".to_string(),
            Obj::Dict(Dict(vec![
                ("Predictor".to_string(), Obj::Num(12.0)),
                ("Columns".to_string(), Obj::Num(4.0)),
            ])),
        )]);
        let data = vec![
            2, 1, 0, 0, 10, // up: 1 0 0 10
            2, 1, 0, 0, 10, // up: 2 0 0 20
            2, 0, 0, 0, 5, //  up: 2 0 0 25
        ];
        assert_eq!(
            predictor(&dict, data),
            vec![1, 0, 0, 10, 2, 0, 0, 20, 2, 0, 0, 25]
        );
        // With no parameters the bytes are the bytes.
        assert_eq!(predictor(&Dict::default(), vec![1, 2, 3]), vec![1, 2, 3]);
    }

    #[test]
    fn the_lexer_reads_what_a_pdf_writes() {
        let mut lex = Lex::new(
            br"<< /A 1 /B 2.5 /C (ciao \(mondo\)) /D /Name#20with#20spaces /E [1 2 R 3] /F << /G true >> /H <414243> >>",
            0,
        );
        let Obj::Dict(d) = lex.obj(0) else {
            panic!("a dictionary")
        };
        assert_eq!(d.get("A"), Some(&Obj::Num(1.0)));
        assert_eq!(d.get("B"), Some(&Obj::Num(2.5)));
        assert_eq!(d.get("C"), Some(&Obj::Str(b"ciao (mondo)".to_vec())));
        assert_eq!(d.get("D"), Some(&Obj::Name("Name with spaces".to_string())));
        // `1 2 R` is a reference, and the `3` after it is the array's second item.
        assert_eq!(
            d.get("E"),
            Some(&Obj::Array(vec![Obj::Ref(1, 2), Obj::Num(3.0)]))
        );
        assert_eq!(
            d.get("F")
                .and_then(|o| o.as_dict())
                .and_then(|d| d.get("G")),
            Some(&Obj::Bool(true))
        );
        assert_eq!(d.get("H"), Some(&Obj::Str(b"ABC".to_vec())));
    }

    #[test]
    fn the_lexer_does_not_hang_or_overrun_on_rubbish() {
        // Each of these used to be a way to loop for ever or read past the end.
        for bad in [
            &b"<< /A"[..],
            &b"<< /A [[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[[1"[..],
            &b"<< /A (unterminated"[..],
            &b"<< << << >>"[..],
            &b"[1 0 R"[..],
            &b"<"[..],
            &b"<< / /A 1 >>"[..],
            &b"999999999999999999999999 0 R"[..],
        ] {
            let mut lex = Lex::new(bad, 0);
            let _ = lex.obj(0);
            assert!(lex.pos <= bad.len() + 1);
        }
    }
}
