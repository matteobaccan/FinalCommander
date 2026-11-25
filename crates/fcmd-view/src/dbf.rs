//! dBase tables: the header, the field types, the code page and the memo files beside one
//! (SPEC 05 §1.11).
//!
//! In a module of its own because it is a feature of its own: the compact build
//! (`--no-default-features`) leaves it out, and `dbf_none.rs` stands in for it there — a
//! `.dbf` is then bytes like any other file nobody reads. What changes a table is in
//! `dbf_write.rs`; this is the half that only reads.

use crate::table::{Align, MAX_COL};

/// Is the dBase reader in this build? It is: the compact build has `dbf_none.rs` instead.
pub const BUILT_IN: bool = true;

/// A `.dbf` file's header: what the columns are, and where the records are.
///
/// This is the reader the format was waiting for. A DBF says in its first bytes how many
/// records it holds, how long one is and where they start, so row *n* is one multiplication
/// and one read — which is why this one is not loaded whole like the text formats. A two
/// gigabyte table opens as fast as a log does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dbf {
    /// Where the first record begins.
    pub first: u64,
    /// How long one record is, the deletion flag included.
    pub record_len: usize,
    /// How many records the header claims. Checked against the file's own length, because a
    /// truncated table is a thing that happens and reading past the end is not.
    pub records: usize,
    pub(crate) fields: Vec<Field>,
    /// What the header's language byte says the text is in.
    pub(crate) encoding: Charset,
    /// What the version byte says this is, for the status line.
    pub flavour: &'static str,
    /// Does the language byte name a code page this program holds exactly? A DOS page that
    /// is read through its Windows cousin, or a byte that names none, is good enough to
    /// read and not to write: the accented letters would land on other bytes than the
    /// programs that own the table expect (see `dbf_write.rs`).
    pub(crate) exact: bool,
    /// The header's language byte itself. Zero is a table that does not say what its text is
    /// in — what Clipper writes — and the one case in which the reader may be told
    /// ([`Dbf::set_page`]).
    pub(crate) language: u8,
    /// Where in a record Visual FoxPro's `_NullFlags` field sits, and how long it is. The
    /// field is the table's own bookkeeping — a bit for every field that may be empty of a
    /// value, and one for every field whose length varies — so it is no column here.
    pub(crate) nulls: Option<(usize, usize)>,
}

/// One field as the header declares it: `QTY`, `N`, 8 wide, 2 decimals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub kind: char,
    pub len: usize,
    pub decimals: u8,
    /// Where in the record the field starts, the deletion flag being byte zero.
    pub offset: usize,
    /// Visual FoxPro's flags, zero in every other table.
    pub flags: u8,
}

/// A code page a table can be told it is in, when its header does not say.
///
/// The ones a table that says nothing is ever found in: the DOS pages Clipper programs ran
/// under, and the Windows pages their successors moved to. Each is held exactly, so a table
/// read through one is written through it as well.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Page {
    Cp437,
    Cp850,
    Cp852,
    Cp865,
    Cp866,
    Cp1250,
    Cp1251,
    Cp1252,
}

impl Page {
    /// Every page that can be chosen, in the order the key walks them.
    pub const ALL: [Page; 8] = [
        Page::Cp437,
        Page::Cp850,
        Page::Cp852,
        Page::Cp865,
        Page::Cp866,
        Page::Cp1250,
        Page::Cp1251,
        Page::Cp1252,
    ];

    pub fn name(self) -> &'static str {
        self.charset().name()
    }

    fn charset(self) -> Charset {
        match self {
            Page::Cp437 => Charset::Cp437,
            Page::Cp850 => Charset::Dos("CP850", &CP850_HIGH),
            Page::Cp852 => Charset::Dos("CP852", &CP852_HIGH),
            Page::Cp865 => Charset::Dos("CP865", &CP865_HIGH),
            Page::Cp866 => Charset::Table(encoding_rs::IBM866),
            Page::Cp1250 => Charset::Table(encoding_rs::WINDOWS_1250),
            Page::Cp1251 => Charset::Table(encoding_rs::WINDOWS_1251),
            Page::Cp1252 => Charset::Table(encoding_rs::WINDOWS_1252),
        }
    }

    /// The page after `now` on the wheel: the first one from the header's own reading, and
    /// the header's own reading again (`None`) after the last.
    pub fn after(now: Option<Page>) -> Option<Page> {
        match now {
            None => Some(Page::ALL[0]),
            Some(page) => {
                let at = Page::ALL.iter().position(|p| *p == page).unwrap_or(0);
                Page::ALL.get(at + 1).copied()
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) kind: u8,
    /// Where in the record the field starts, the deletion flag included.
    pub(crate) at: usize,
    pub(crate) len: usize,
    pub(crate) decimals: u8,
    /// Visual FoxPro's flags: `0x01` a system field, `0x02` one that may hold no value,
    /// `0x04` binary, `0x0C` a field the table numbers by itself.
    pub(crate) flags: u8,
    /// Which bit of `_NullFlags` says this record holds no value in the field.
    pub(crate) null_bit: Option<usize>,
    /// Which bit of `_NullFlags` says a `V` or `Q` field is shorter than its width, the
    /// length then being the field's last byte.
    pub(crate) var_bit: Option<usize>,
}

impl Dbf {
    /// Read the header. `None` for anything that is not a table we know how to read: the
    /// first byte says what it is, and a byte that says something else is not a DBF.
    pub fn parse(head: &[u8]) -> Option<Dbf> {
        if head.len() < 33 {
            return None;
        }
        let version = head[0];
        let flavour = match version {
            0x02 => "dBase II",
            0x03 => "dBase III",
            0x04 | 0x7b => "dBase IV",
            0x05 => "dBase V",
            0x30..=0x32 => "Visual FoxPro",
            0x43 | 0x63 | 0xcb => "dBase IV SQL",
            0x83 => "dBase III + memo",
            0x8b | 0x8e => "dBase IV + memo",
            0xf5 => "FoxPro 2 + memo",
            0xfb => "FoxPro",
            _ => return None,
        };
        let records = u32::from_le_bytes([head[4], head[5], head[6], head[7]]) as usize;
        let header_len = u16::from_le_bytes([head[8], head[9]]) as usize;
        let record_len = u16::from_le_bytes([head[10], head[11]]) as usize;
        if header_len < 33 || record_len == 0 {
            return None;
        }
        let encoding = encoding_for(head[29]);

        let mut fields = Vec::new();
        let mut nulls = None;
        let mut at = 1usize; // the deletion flag is the first byte of every record
        let mut cursor = 32usize;
        while cursor + 32 <= head.len() && cursor + 32 <= header_len {
            if head[cursor] == 0x0d {
                break;
            }
            let raw = &head[cursor..cursor + 32];
            let name: String = raw[..11]
                .iter()
                .take_while(|b| **b != 0)
                .map(|b| *b as char)
                .collect();
            let len = raw[16] as usize;
            // The length of a `C` field over 255 is the two bytes 16 and 17 together, which
            // is how FoxPro writes a long character field; for every other type byte 17 is
            // the decimal count.
            let (len, decimals) = if raw[11] == b'C' && raw[17] != 0 && len < 255 {
                (len + ((raw[17] as usize) << 8), 0)
            } else {
                (len, raw[17])
            };
            // Visual FoxPro's own field, hidden from its own users too: it is where the
            // bits below live, and it takes its room in the record like any other.
            if raw[11] == b'0' && raw[18] & 0x01 != 0 {
                nulls = Some((at, len));
                at += len;
                cursor += 32;
                continue;
            }
            fields.push(Field {
                name: name.trim().to_string(),
                kind: raw[11],
                at,
                len,
                decimals,
                flags: raw[18],
                null_bit: None,
                var_bit: None,
            });
            at += len;
            cursor += 32;
        }
        if fields.is_empty() {
            return None;
        }
        // The bits are dealt out in the order of the fields, the lowest of the first byte
        // first: a field of varying length takes one for "shorter than its width", and a
        // field that may be empty takes one for "no value" — a field that is both takes
        // two, in that order.
        if nulls.is_some() {
            let mut bit = 0usize;
            for f in &mut fields {
                if matches!(f.kind, b'V' | b'Q') {
                    f.var_bit = Some(bit);
                    bit += 1;
                }
                if f.flags & 0x02 != 0 {
                    f.null_bit = Some(bit);
                    bit += 1;
                }
            }
        }
        Some(Dbf {
            first: header_len as u64,
            record_len,
            records,
            fields,
            encoding,
            flavour,
            exact: is_exact(head[29]),
            language: head[29],
            nulls,
        })
    }

    /// Does the header say which code page the text is in? A table that does is read as it
    /// says; one that does not can be told ([`Dbf::set_page`]).
    pub fn names_its_page(&self) -> bool {
        self.language != 0
    }

    /// What the text is read as, for the status line and the messages.
    pub fn page_name(&self) -> &'static str {
        self.encoding.name()
    }

    /// Read — and write — the text of a table whose header names no code page as `page`, or
    /// as the header's own silence would have it again (`None`). `false` for a table whose
    /// header does name one: that is not a thing to argue with from here.
    ///
    /// A page chosen is a page known: the table is then written in it in full, where a
    /// table nobody has spoken for takes ASCII alone.
    pub fn set_page(&mut self, page: Option<Page>) -> bool {
        if self.names_its_page() {
            return false;
        }
        match page {
            Some(page) => {
                self.encoding = page.charset();
                self.exact = true;
            }
            None => {
                self.encoding = encoding_for(0);
                self.exact = false;
            }
        }
        true
    }

    /// Is bit `n` of this record's `_NullFlags` set?
    fn flag(&self, record: &[u8], n: Option<usize>) -> bool {
        let (Some(n), Some((at, len))) = (n, self.nulls) else {
            return false;
        };
        n / 8 < len
            && record
                .get(at + n / 8)
                .is_some_and(|byte| byte & (1 << (n % 8)) != 0)
    }

    /// Where record `n` begins in the file.
    pub fn offset(&self, n: usize) -> u64 {
        self.first + (n as u64) * (self.record_len as u64)
    }

    /// How many fields a record has, the deletion flag not among them.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }

    /// What the header says about field `n`: what a dialog that edits one has to show, and
    /// what it is checked against.
    pub fn field(&self, n: usize) -> Option<FieldInfo> {
        self.fields.get(n).map(|f| FieldInfo {
            name: f.name.clone(),
            kind: f.kind as char,
            len: f.len,
            decimals: f.decimals,
            offset: f.at,
            flags: f.flags,
        })
    }

    /// The columns: the deletion flag first, then the fields as the header names them.
    pub(crate) fn columns(&self) -> Vec<String> {
        let mut out = vec!["*".to_string()];
        out.extend(self.fields.iter().map(|f| f.name.clone()));
        out
    }

    /// Which side of its column a cell sits on. A number is read down its last digit, so the
    /// numeric types go to the right and everything else stays where it was written.
    pub(crate) fn aligns(&self) -> Vec<Align> {
        let mut out = vec![Align::Left];
        out.extend(self.fields.iter().map(|f| match f.kind {
            b'N' | b'F' | b'I' | b'Y' | b'B' | b'O' => Align::Right,
            _ => Align::Left,
        }));
        out
    }

    /// How wide each column is: what the header declares, capped. A DBF is the one format
    /// that does not have to be read to be laid out.
    pub(crate) fn widths(&self) -> Vec<usize> {
        let mut out = vec![1usize]; // the deletion flag's own column
        out.extend(self.fields.iter().map(|f| {
            let declared = match f.kind {
                b'D' => 10, // `YYYY-MM-DD`, which is wider than the stored 8
                b'L' => 5,  // `false`
                b'I' => 11,
                b'T' => 19,
                b'Y' | b'B' | b'O' => 20,
                b'M' | b'G' => 10,
                // Two hex digits a byte.
                b'Q' => f.len * 2,
                _ => f.len,
            };
            declared.max(f.name.chars().count()).clamp(1, MAX_COL)
        }));
        out
    }

    /// One record's cells, out of the record's own bytes.
    ///
    /// A record whose deletion flag is set is still a record: the first column says so with
    /// a `*`, because a table where deleted rows simply vanish is a table whose row count
    /// does not add up — and finding the deleted ones is half of why anybody opens a DBF.
    /// The flag has a column of its own rather than a mark on the first cell, which in a
    /// column of numbers would read as part of the number.
    pub(crate) fn cells(
        &self,
        record: &[u8],
        mut memo: Option<&mut (dyn crate::source::Source + '_)>,
    ) -> Vec<String> {
        let deleted = record.first() == Some(&b'*');
        let mut out = Vec::with_capacity(self.fields.len() + 1);
        out.push(if deleted {
            "*".to_string()
        } else {
            String::new()
        });
        for f in &self.fields {
            let end = (f.at + f.len).min(record.len());
            let raw = if f.at < end {
                &record[f.at..end]
            } else {
                &[][..]
            };
            // No value at all is not an empty one, and Visual FoxPro's own word for it is
            // the one a reader of such a table expects.
            if self.flag(record, f.null_bit) {
                out.push("NULL".to_string());
                continue;
            }
            let short = self.flag(record, f.var_bit);
            out.push(self.cell(f, raw, short, memo.as_deref_mut()));
        }
        out
    }

    /// The bytes of field `field` in `record`, cut to the length a `V` or `Q` field says it
    /// holds. `None` for a field that is not there, or one that holds no value.
    fn value<'a>(&self, record: &'a [u8], field: usize) -> Option<&'a [u8]> {
        let f = self.fields.get(field)?;
        if self.flag(record, f.null_bit) {
            return None;
        }
        let raw = record.get(f.at..f.at + f.len)?;
        Some(varying(raw, self.flag(record, f.var_bit)))
    }

    /// The whole text of the memo that field `field` of `record` points at: what a cell has
    /// one line of. `None` for a field that is not a text memo, an empty one, and one whose
    /// block is not in the file beside the table.
    pub(crate) fn memo_whole(
        &self,
        record: &[u8],
        field: usize,
        memo: &mut dyn crate::source::Source,
    ) -> Option<String> {
        let f = self.fields.get(field)?;
        if f.kind != b'M' {
            return None;
        }
        let block = self.memo_block(f, self.value(record, field)?);
        if block == 0 {
            return None;
        }
        memo_text(memo, block, self.encoding, MEMO_WHOLE)
    }

    /// Is field `field` a memo of text — one whose whole text can be read?
    pub fn is_memo(&self, field: usize) -> bool {
        self.fields.get(field).is_some_and(|f| f.kind == b'M')
    }

    /// Which block of the memo file a memo field's bytes name: written as text in the dBase
    /// tables and as four binary bytes in the FoxPro ones, where the field is four long.
    fn memo_block(&self, f: &Field, raw: &[u8]) -> usize {
        if f.len == 4 && raw.len() >= 4 {
            u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize
        } else {
            let text = self.encoding.decode(raw);
            text.trim().parse::<usize>().unwrap_or(0)
        }
    }

    /// The fields that hold numbers, by index: the ones a sum is taken of.
    pub(crate) fn numeric_fields(&self) -> Vec<usize> {
        (0..self.fields.len())
            .filter(|i| matches!(self.fields[*i].kind, b'N' | b'F' | b'I' | b'Y' | b'B'))
            .collect()
    }

    fn cell(
        &self,
        f: &Field,
        raw: &[u8],
        short: bool,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
    ) -> String {
        let text = |bytes: &[u8]| self.encoding.decode(bytes);
        match f.kind {
            // A field of varying length: text, or bytes that are nobody's text and are
            // shown as what they are.
            b'V' => text(varying(raw, short)),
            b'Q' => varying(raw, short)
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect(),
            // Numbers are written as text, right-aligned in the field with spaces.
            b'N' | b'F' => text(raw).trim().to_string(),
            // Eight digits, `YYYYMMDD`, or spaces for a date that was never set.
            b'D' => {
                let s = text(raw);
                let s = s.trim();
                if s.len() == 8 && s.chars().all(|c| c.is_ascii_digit()) {
                    format!("{}-{}-{}", &s[0..4], &s[4..6], &s[6..8])
                } else {
                    String::new()
                }
            }
            b'L' => match raw.first() {
                Some(b'T') | Some(b't') | Some(b'Y') | Some(b'y') => "true".to_string(),
                Some(b'F') | Some(b'f') | Some(b'N') | Some(b'n') => "false".to_string(),
                _ => String::new(),
            },
            // A memo is a block number into the `.dbt` or `.fpt` beside the table. The
            // number is written as text in the dBase tables and as four binary bytes in the
            // FoxPro ones, where the field is four long.
            b'M' | b'G' | b'P' => {
                let block = self.memo_block(f, raw);
                if block == 0 {
                    return String::new(); // where an empty memo points
                }
                // What it points at, when the file beside the table was there to open; the
                // block number otherwise, which is all there was to say before.
                match memo.and_then(|file| memo_text(file, block, self.encoding, MEMO_CELL)) {
                    // A picture or an object is not text, and a cell is no place for it.
                    Some(text) if f.kind == b'M' => one_line(&text),
                    _ => format!("memo:{block}"),
                }
            }
            b'I' if raw.len() >= 4 => {
                i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]).to_string()
            }
            b'Y' if raw.len() >= 8 => {
                let n = i64::from_le_bytes([
                    raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                ]);
                // The sign apart: minus half a unit has no whole part to carry it.
                let sign = if n < 0 { "-" } else { "" };
                let n = n.unsigned_abs();
                format!("{sign}{}.{:04}", n / 10_000, n % 10_000)
            }
            b'B' | b'O' if raw.len() >= 8 => {
                let n = f64::from_le_bytes([
                    raw[0], raw[1], raw[2], raw[3], raw[4], raw[5], raw[6], raw[7],
                ]);
                format!("{n}")
            }
            // A `T` is two 32-bit halves: a Julian day and milliseconds past midnight. The
            // day is the part worth reading in a panel, and the arithmetic is the one from
            // the format's own documentation.
            b'T' if raw.len() >= 8 => {
                let day = i32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]);
                let ms = i32::from_le_bytes([raw[4], raw[5], raw[6], raw[7]]);
                if day == 0 {
                    String::new()
                } else {
                    let (y, m, d) = from_julian(day);
                    let (h, min, s) = (ms / 3_600_000, (ms / 60_000) % 60, (ms / 1000) % 60);
                    format!("{y:04}-{m:02}-{d:02} {h:02}:{min:02}:{s:02}")
                }
            }
            // `C` and anything we do not know: the bytes as text, with the padding off.
            _ => {
                let s = text(raw);
                s.trim_end().trim_start_matches('\0').to_string()
            }
        }
        .replace(['\r', '\n', '\t'], " ")
    }
}

/// A Julian day number as a calendar date. The algorithm is the standard one (Fliegel and Van
/// Flandern), written out because it is four lines and pulling in a date library for the one
/// field type that needs it would not be a trade.
fn from_julian(jd: i32) -> (i32, i32, i32) {
    let l = jd + 68_569;
    let n = (4 * l) / 146_097;
    let l = l - (146_097 * n + 3) / 4;
    let i = (4000 * (l + 1)) / 1_461_001;
    let l = l - (1461 * i) / 4 + 31;
    let j = (80 * l) / 2447;
    let d = l - (2447 * j) / 80;
    let l = j / 11;
    let m = j + 2 - (12 * l);
    let y = 100 * (n - 49) + i + l;
    (y, m, d)
}

/// The value a `V` or `Q` field holds: all of its width, or — when its bit in `_NullFlags`
/// says it is shorter — as many bytes as its last byte counts.
fn varying(raw: &[u8], short: bool) -> &[u8] {
    match raw.last() {
        Some(len) if short => &raw[..(*len as usize).min(raw.len() - 1)],
        _ => raw,
    }
}

/// The text encoding a DBF's language byte names. The ones that are actually met; anything
/// else is read as Windows-1252, which at least never fails and is right for most of Europe.
fn encoding_for(byte: u8) -> Charset {
    match byte {
        // The 437 family: the US driver, and the western-European OEM drivers that all
        // wrote plain 437 — the German one among them, which is the byte a table written
        // under DOS in Germany carries.
        0x01 | 0x09 | 0x0b | 0x0d | 0x0f | 0x11 | 0x15 | 0x18 | 0x19 | 0x1b => Charset::Cp437,
        // 850, the "multilingual" page DOS moved western Europe to, under each country's
        // driver; 865, which is 437 with the Nordic letters; 852 for the centre and east.
        // `encoding_rs` has none of the three, so each is a table of its own here: read
        // through a Windows cousin they showed most letters right and could not be written.
        0x02 | 0x0a | 0x0e | 0x10 | 0x12 | 0x14 | 0x16 | 0x1a | 0x1d | 0x25 | 0x37 => {
            Charset::Dos("CP850", &CP850_HIGH)
        }
        0x08 | 0x17 | 0x66 => Charset::Dos("CP865", &CP865_HIGH),
        0x1f | 0x22 | 0x23 | 0x40 | 0x64 | 0x87 => Charset::Dos("CP852", &CP852_HIGH),
        // 860, 861 and 863 are still read through 1252, which agrees with them on most of
        // the accented letters a field holds and on none of the rest.
        0x1c | 0x24 | 0x67 | 0x6c => Charset::Table(encoding_rs::WINDOWS_1252),
        // Kamenický and Mazovia, likewise against 1250.
        0x68 | 0x69 | 0xc8 => Charset::Table(encoding_rs::WINDOWS_1250),
        0x26 | 0x65 => Charset::Table(encoding_rs::IBM866),
        0xc9 => Charset::Table(encoding_rs::WINDOWS_1251),
        0x13 => Charset::Table(encoding_rs::SHIFT_JIS),
        0x4d | 0x7a | 0x7b => Charset::Table(encoding_rs::GBK),
        0x4e | 0x79 => Charset::Table(encoding_rs::EUC_KR),
        0x4f | 0x78 => Charset::Table(encoding_rs::BIG5),
        0x50 | 0x7c => Charset::Table(encoding_rs::WINDOWS_874),
        0x6a | 0xcb => Charset::Table(encoding_rs::WINDOWS_1253),
        0x6b | 0xca => Charset::Table(encoding_rs::WINDOWS_1254),
        0x7d | 0xcc => Charset::Table(encoding_rs::WINDOWS_1255),
        0x7e | 0xcd => Charset::Table(encoding_rs::WINDOWS_1256),
        _ => Charset::Table(encoding_rs::WINDOWS_1252),
    }
}

/// Is the page [`encoding_for`] gives this byte the page the byte really names?
///
/// Reading forgives an approximation and writing does not. A table marked 860 is read as
/// Windows-1252 — and whatever that makes of its accented letters is at least shown as it
/// stands; written through the same approximation a letter would land on the byte that is
/// another to the program the table belongs to. So only the pages held exactly are written
/// in full, and the others take ASCII alone. A Clipper table, whose language byte is zero,
/// is one of the others until somebody says what it is in ([`Dbf::set_page`]): its header
/// does not.
fn is_exact(byte: u8) -> bool {
    matches!(
        byte,
        // 437 and the national drivers that wrote plain 437.
        0x01 | 0x09 | 0x0b | 0x0d | 0x0f | 0x11 | 0x15 | 0x18 | 0x19 | 0x1b
            // 850, 865 and 852, each a table of this program's own.
            | 0x02 | 0x0a | 0x0e | 0x10 | 0x12 | 0x14 | 0x16 | 0x1a | 0x1d | 0x25 | 0x37
            | 0x08 | 0x17 | 0x66
            | 0x1f | 0x22 | 0x23 | 0x40 | 0x64 | 0x87
            // The Windows pages, 866 and the far-eastern ones: each is the table it names.
            | 0x03 | 0x57 | 0x58 | 0x59 | 0xc8 | 0xc9 | 0xca | 0xcb | 0x7d | 0x7e
            | 0x26 | 0x65 | 0x13 | 0x4d | 0x7a | 0x4e | 0x79 | 0x4f | 0x78 | 0x50 | 0x7c
    )
}

/// The most of a memo a cell reads: no cell is forty characters wide, let alone this.
const MEMO_CELL: usize = 64 * 1024;

/// The most of a memo that is read to be shown whole. A bound all the same, so that a
/// length field with nonsense in it cannot ask for the whole machine.
const MEMO_WHOLE: usize = 16 * 1024 * 1024;

/// The text of one memo block, out of the `.dbt` or `.fpt` beside the table.
///
/// Three formats and no extension to tell them apart by, so each is recognised by what it
/// actually holds — and each says enough about itself to be sure:
///
/// * **FoxPro `.fpt`**: the block size is two big-endian bytes at offset 6, and a block opens
///   with its type and its length, both big-endian. A type of 0, 1 or 2 and a length that
///   fits in the file is a signature that nothing else matches by accident.
/// * **dBase IV `.dbt`**: the block size is two little-endian bytes at offset 20, and a block
///   opens with `FF FF 08 00` and then its length, the eight-byte opening included.
/// * **dBase III `.dbt`**: blocks of 512 with nothing in front of them; the text runs to the
///   first `1A`, which is the end-of-file byte the format puts after every memo.
///
/// `None` for a block that is not there — a number past the end of the file, a memo file that
/// belongs to another table — and the cell then says what it said before: the block number.
fn memo_text(
    source: &mut dyn crate::source::Source,
    block: usize,
    encoding: Charset,
    most: usize,
) -> Option<String> {
    let len = source.len();
    let head = source.read_vec(0, 32).ok()?;
    if head.len() < 32 {
        return None;
    }
    let read = |source: &mut dyn crate::source::Source, at: u64, n: usize| -> Option<Vec<u8>> {
        if at >= len {
            return None;
        }
        let n = n.min(most).min((len - at) as usize);
        source.read_vec(at, n).ok()
    };

    // FoxPro, by the two signatures its block carries.
    let fox_block = u16::from_be_bytes([head[6], head[7]]) as usize;
    if fox_block >= 32 {
        let at = (block as u64).saturating_mul(fox_block as u64);
        if let Some(opening) = read(source, at, 8) {
            if opening.len() == 8 {
                let kind = u32::from_be_bytes([opening[0], opening[1], opening[2], opening[3]]);
                let size =
                    u32::from_be_bytes([opening[4], opening[5], opening[6], opening[7]]) as usize;
                if kind <= 2 && size > 0 && at + 8 + size as u64 <= len {
                    let body = read(source, at + 8, size)?;
                    return Some(encoding.decode(&body));
                }
            }
        }
    }

    // dBase IV, by the `FF FF 08 00` its block opens with.
    let dbase_block = match u16::from_le_bytes([head[20], head[21]]) as usize {
        0 => 512,
        n => n,
    };
    let at = (block as u64).saturating_mul(dbase_block as u64);
    let opening = read(source, at, 8).unwrap_or_default();
    if opening.len() == 8 && opening[..4] == [0xff, 0xff, 0x08, 0x00] {
        let size = u32::from_le_bytes([opening[4], opening[5], opening[6], opening[7]]) as usize;
        let size = size.saturating_sub(8);
        if size > 0 {
            let body = read(source, at + 8, size)?;
            return Some(encoding.decode(&body));
        }
    }

    // dBase III: blocks of 512, and the text runs to the end-of-file byte after it.
    let at = (block as u64).saturating_mul(512);
    let body = read(source, at, most)?;
    if body.is_empty() {
        return None;
    }
    let end = body.iter().position(|b| *b == 0x1a).unwrap_or(body.len());
    Some(encoding.decode(&body[..end]))
}

/// A memo on one line: it is written with newlines in it, and a cell has one line to give.
fn one_line(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut space = false;
    for ch in text.chars() {
        if ch.is_whitespace() {
            space = !out.is_empty();
            continue;
        }
        if space {
            out.push(' ');
            space = false;
        }
        out.push(ch);
    }
    out
}

/// Which table a DBF's character fields are in.
///
/// Two kinds, because the one that matters most is the one `encoding_rs` does not carry: a
/// DBF written under DOS is in code page 437, and reading it as Windows-1252 turns every
/// accented letter into a different one — `temporäre` into `tempor„re`. The viewer has had
/// a 437 table since the day it learned to show a DOS text file; this is that table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Charset {
    Cp437,
    /// One of the other DOS pages, by its name and the upper half of its table; the lower
    /// half is ASCII in all of them.
    Dos(&'static str, &'static [char; 128]),
    Table(&'static encoding_rs::Encoding),
}

impl Charset {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Charset::Cp437 => "CP437",
            Charset::Dos(name, _) => name,
            Charset::Table(table) => {
                let known: [(&encoding_rs::Encoding, &str); 4] = [
                    (encoding_rs::IBM866, "CP866"),
                    (encoding_rs::WINDOWS_1250, "CP1250"),
                    (encoding_rs::WINDOWS_1251, "CP1251"),
                    (encoding_rs::WINDOWS_1252, "CP1252"),
                ];
                known
                    .iter()
                    .find(|(enc, _)| std::ptr::eq(*enc, table))
                    .map(|(_, name)| *name)
                    .unwrap_or_else(|| table.name())
            }
        }
    }

    pub(crate) fn decode(self, bytes: &[u8]) -> String {
        match self {
            Charset::Cp437 => crate::decode::Codepage::Cp437.decode(bytes),
            // As the 437 table reads them: a NUL is padding, and the other control bytes
            // are not letters of any page.
            Charset::Dos(_, high) => bytes
                .iter()
                .map(|b| match b {
                    0x00 => ' ',
                    0x20..=0x7e => *b as char,
                    0x80..=0xff => high[(*b - 0x80) as usize],
                    _ => '.',
                })
                .collect(),
            Charset::Table(table) => table.decode(bytes).0.into_owned(),
        }
    }

    /// The bytes of `text` in this page, or the first character the page has no byte for.
    ///
    /// Never a `?` in its place: a field written with a letter quietly swapped is a record
    /// somebody will trust.
    pub(crate) fn encode(self, text: &str) -> Result<Vec<u8>, char> {
        let page = match self {
            Charset::Cp437 => crate::decode::Codepage::Cp437,
            Charset::Table(table) => crate::decode::Codepage::Enc(table),
            Charset::Dos(_, high) => {
                return text
                    .chars()
                    .map(|c| match c {
                        c if (c as u32) < 0x80 => Ok(c as u8),
                        other => high
                            .iter()
                            .position(|h| *h == other)
                            .map(|i| 0x80 + i as u8)
                            .ok_or(other),
                    })
                    .collect();
            }
        };
        match text.chars().find(|c| !page.can_encode(&c.to_string())) {
            Some(bad) => Err(bad),
            None => Ok(page.encode(text)),
        }
    }
}

// ── the structure pane ─────────────────────────────────────────────────────────────────────

/// A field's type as a dBase programmer writes it: `C 10`, `N 7.2`, `D 8`.
fn type_of(kind: u8, len: usize, decimals: u8) -> String {
    match kind {
        b'N' | b'F' | b'B' if decimals > 0 => format!("{} {len}.{decimals}", kind as char),
        _ => format!("{} {len}", kind as char),
    }
}

/// A dBase table for the structure pane: what the header says of the table, and every field
/// with its type, width, decimals, where it sits in the record and its flags.
///
/// A DBF has no signature — its first byte is a version number, and `0x30` is also the
/// digit a text file may well begin with — so this asks more than [`Dbf::parse`] does
/// before it believes one: the header has to fit in the file, the fields have to be of
/// types somebody writes, and their widths have to add up to the record length the header
/// declares. A file that passes all three by accident has yet to be found.
pub(crate) fn structure(
    src: &mut dyn crate::structure::Bytes,
    head: &[u8],
    len: u64,
) -> Option<crate::structure::Structure> {
    use crate::structure::Row;
    if head.len() < 33 {
        return None;
    }
    let header_len = u16::from_le_bytes([head[8], head[9]]) as usize;
    if header_len as u64 > len {
        return None;
    }
    let whole;
    let head = if header_len > head.len() {
        whole = src.at(0, header_len);
        &whole[..]
    } else {
        head
    };
    let dbf = Dbf::parse(head)?;
    let known = |f: &Field| f.kind.is_ascii_uppercase() || matches!(f.kind, b'0' | b'@' | b'+');
    let named = |f: &Field| !f.name.is_empty() && f.name.bytes().all(|b| b.is_ascii_graphic());
    if !dbf.fields.iter().all(|f| known(f) && named(f)) {
        return None;
    }
    let end = dbf
        .fields
        .iter()
        .map(|f| f.at + f.len)
        .chain(dbf.nulls.map(|(at, len)| at + len))
        .max()?;
    if end != dbf.record_len {
        return None;
    }

    let mut rows = vec![Row::group("Header")];
    rows.push(Row::field(
        "version",
        format!("{:02x}", head[0]),
        dbf.flavour,
    ));
    rows.push(Row::field("records", "", dbf.records.to_string()));
    rows.push(Row::field(
        "last update",
        "",
        format!("{:04}-{:02}-{:02}", 1900 + head[1] as u32, head[2], head[3]),
    ));
    rows.push(Row::field(
        "header length",
        "",
        format!("{} bytes", dbf.first),
    ));
    rows.push(Row::field(
        "record length",
        "",
        format!("{} bytes", dbf.record_len),
    ));
    rows.push(Row::field(
        "code page",
        format!("{:02x}", dbf.language),
        if dbf.names_its_page() {
            dbf.page_name().to_string()
        } else {
            "not named".to_string()
        },
    ));

    rows.push(Row::group(format!("Fields ({})", dbf.fields.len())));
    for f in &dbf.fields {
        let mut notes = vec![format!("offset {}", f.at)];
        for (bit, name) in [(0x01, "system"), (0x02, "nullable"), (0x04, "binary")] {
            if f.flags & bit != 0 {
                notes.push(name.to_string());
            }
        }
        if f.flags & 0x0c == 0x0c {
            notes.push("autoincrement".to_string());
        }
        rows.push(Row::field(
            f.name.clone(),
            type_of(f.kind, f.len, f.decimals),
            notes.join(" · "),
        ));
    }
    if let Some((at, len)) = dbf.nulls {
        // Not a column of the table, and all the same a part of its record.
        rows.push(Row::field(
            "_NullFlags",
            type_of(b'0', len, 0),
            format!("offset {at} · system"),
        ));
    }
    let room = len.saturating_sub(dbf.first) / dbf.record_len.max(1) as u64;
    Some(crate::structure::Structure {
        format: "DBF",
        summary: format!(
            "{}, {} records of {} fields",
            dbf.flavour,
            dbf.records,
            dbf.fields.len()
        ),
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: (dbf.records as u64) > room,
    })
}

// ── sums ───────────────────────────────────────────────────────────────────────────────────

/// What one numeric column adds up to, over the records it was asked about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnSum {
    pub name: String,
    /// The sum as the column writes its numbers: with the decimals the values had.
    pub sum: String,
    /// How many of the records held a number there — an empty cell and a `NULL` add
    /// nothing and are not counted.
    pub count: usize,
}

/// A running total. Kept in whole units of the smallest decimal met, so that a column of
/// prices adds up to the cent: a sum of money that is one cent out is worse than none. A
/// `B` field is a double already, and a number written with an exponent has left decimal
/// arithmetic behind; either turns the total into a double too.
enum Total {
    Exact { units: i128, scale: u32 },
    Float(f64),
}

impl Total {
    fn as_float(&self) -> f64 {
        match self {
            Total::Exact { units, scale } => *units as f64 / 10f64.powi(*scale as i32),
            Total::Float(f) => *f,
        }
    }

    /// Add what a cell shows. `false` for a cell that is not a number.
    fn add(&mut self, cell: &str) -> bool {
        let cell = cell.trim();
        if let (Total::Exact { units, scale }, Some((value, places))) = (&mut *self, exact(cell)) {
            let wider = places.max(*scale);
            let sum = 10i128
                .checked_pow(wider - *scale)
                .and_then(|by| units.checked_mul(by))
                .zip(
                    10i128
                        .checked_pow(wider - places)
                        .and_then(|by| value.checked_mul(by)),
                )
                .and_then(|(a, b)| a.checked_add(b));
            if let Some(sum) = sum {
                *units = sum;
                *scale = wider;
                return true;
            }
        }
        match cell.parse::<f64>() {
            Ok(value) if value.is_finite() => {
                *self = Total::Float(self.as_float() + value);
                true
            }
            _ => false,
        }
    }

    fn show(&self) -> String {
        match self {
            Total::Float(f) => format!("{f}"),
            Total::Exact { units, scale: 0 } => units.to_string(),
            Total::Exact { units, scale } => {
                let one = 10u128.pow(*scale);
                let whole = units.unsigned_abs() / one;
                let part = units.unsigned_abs() % one;
                let sign = if *units < 0 { "-" } else { "" };
                format!("{sign}{whole}.{part:0width$}", width = *scale as usize)
            }
        }
    }
}

/// `-12.50` as 1250 hundredths: the digits as one integer, and how many of them were after
/// the point. `None` for anything that is not written that plainly.
fn exact(cell: &str) -> Option<(i128, u32)> {
    let (negative, digits) = match cell.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cell.strip_prefix('+').unwrap_or(cell)),
    };
    let (whole, part) = digits.split_once('.').unwrap_or((digits, ""));
    let plain = |s: &str| s.bytes().all(|b| b.is_ascii_digit());
    if !plain(whole) || !plain(part) || whole.len() + part.len() == 0 || part.len() > 18 {
        return None;
    }
    let value: i128 = format!("{whole}{part}").parse().ok()?;
    Some((if negative { -value } else { value }, part.len() as u32))
}

impl crate::table::Table {
    /// The sum of every numeric column of a dBase table — `N`, `F`, `I`, `Y` and `B` — over
    /// `records`, which are numbers of records in the file. `None` for a table that is not
    /// a dBase one; empty for one with no numbers in it.
    pub fn sums(
        &self,
        source: &mut dyn crate::source::Source,
        records: &[usize],
    ) -> Option<Vec<ColumnSum>> {
        let crate::table::Records::Dbf(dbf) = &self.records else {
            return None;
        };
        let fields = dbf.numeric_fields();
        let mut totals: Vec<(Total, usize)> = fields
            .iter()
            .map(|_| (Total::Exact { units: 0, scale: 0 }, 0))
            .collect();
        if !fields.is_empty() {
            for &record in records {
                // Without the memo file: no number is kept there.
                let cells = self.cells_of(source, None, record);
                for (slot, field) in totals.iter_mut().zip(&fields) {
                    // The deletion flag's own column comes first.
                    if cells.get(field + 1).is_some_and(|cell| slot.0.add(cell)) {
                        slot.1 += 1;
                    }
                }
            }
        }
        Some(
            fields
                .iter()
                .zip(totals)
                .map(|(field, (total, count))| ColumnSum {
                    name: dbf.fields[*field].name.clone(),
                    sum: total.show(),
                    count,
                })
                .collect(),
        )
    }
}

/// The upper half of code page 850, "DOS Latin 1": what western Europe's DOS wrote after
/// 437, with the accented capitals where 437 had half of its box-drawing.
#[rustfmt::skip]
const CP850_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '×', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '®', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'À', '©',
    '╣', '║', '╗', '╝', '¢', '¥', '┐', '└', '┴', '┬', '├', '─', '┼', 'ã', 'Ã', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '¤', 'ð', 'Ð', 'Ê', 'Ë', 'È', 'ı', 'Í', 'Î', 'Ï', '┘', '┌', '█', '▄', '¦', 'Ì',
    '▀', 'Ó', 'ß', 'Ô', 'Ò', 'õ', 'Õ', 'µ', 'þ', 'Þ', 'Ú', 'Û', 'Ù', 'ý', 'Ý', '¯', '´', '\u{ad}',
    '±', '‗', '¾', '¶', '§', '÷', '¸', '°', '¨', '·', '¹', '³', '²', '■', '\u{a0}',
];

/// The upper half of code page 852, "DOS Latin 2": Polish, Czech, Slovak, Hungarian,
/// Romanian and the languages of the former Yugoslavia in Latin letters.
#[rustfmt::skip]
const CP852_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'ů', 'ć', 'ç', 'ł', 'ë', 'Ő', 'ő', 'î', 'Ź', 'Ä', 'Ć', 'É', 'Ĺ', 'ĺ',
    'ô', 'ö', 'Ľ', 'ľ', 'Ś', 'ś', 'Ö', 'Ü', 'Ť', 'ť', 'Ł', '×', 'č', 'á', 'í', 'ó', 'ú', 'Ą', 'ą',
    'Ž', 'ž', 'Ę', 'ę', '¬', 'ź', 'Č', 'ş', '«', '»', '░', '▒', '▓', '│', '┤', 'Á', 'Â', 'Ě', 'Ş',
    '╣', '║', '╗', '╝', 'Ż', 'ż', '┐', '└', '┴', '┬', '├', '─', '┼', 'Ă', 'ă', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '¤', 'đ', 'Đ', 'Ď', 'Ë', 'ď', 'Ň', 'Í', 'Î', 'ě', '┘', '┌', '█', '▄', 'Ţ', 'Ů',
    '▀', 'Ó', 'ß', 'Ô', 'Ń', 'ń', 'ň', 'Š', 'š', 'Ŕ', 'Ú', 'ŕ', 'Ű', 'ý', 'Ý', 'ţ', '´', '\u{ad}',
    '˝', '˛', 'ˇ', '˘', '§', '÷', '¸', '°', '¨', '˙', 'ű', 'Ř', 'ř', '■', '\u{a0}',
];

/// The upper half of code page 865, "DOS Nordic": 437 with `ø`, `Ø` and `¤` in three of its
/// places, which is all Danish and Norwegian asked of it.
#[rustfmt::skip]
const CP865_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', 'ø', '£', 'Ø', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '¤', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::table::Table;

    /// A dBase III table, built by hand: two fields, three records, one of them deleted.
    pub(crate) fn dbase_bytes() -> Vec<u8> {
        let mut out = Vec::new();
        let header_len = 32 + 2 * 32 + 1; // header, two field descriptors, the terminator
        let record_len = 1 + 10 + 8; // the deletion flag, a `C(10)`, a date
        out.push(0x03); // dBase III, no memo
        out.extend_from_slice(&[126, 1, 15]); // last written 2026-01-15
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(&(record_len as u16).to_le_bytes());
        out.extend_from_slice(&[0u8; 20]);
        let mut field = |name: &str, kind: u8, len: u8| {
            let mut raw = [0u8; 32];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            raw[11] = kind;
            raw[16] = len;
            out.extend_from_slice(&raw);
        };
        field("NAME", b'C', 10);
        field("BORN", b'D', 8);
        out.push(0x0d);
        for (flag, name, born) in [
            (b' ', "mb        ", "19750312"),
            (b'*', "gone      ", "        "),
            (b' ', "ada       ", "18151210"),
        ] {
            out.push(flag);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(born.as_bytes());
        }
        out.push(0x1a); // the end-of-file byte a dBase writer puts there
        out
    }

    #[test]
    fn a_dbase_table_reads_its_columns_from_the_header_and_its_rows_by_seeking() {
        let bytes = dbase_bytes();
        let len = bytes.len() as u64;
        let table = Table::dbase(&bytes, len).expect("a dBase table");
        // The deletion flag is a column of its own, in front of the fields.
        assert_eq!(table.columns, ["*", "NAME", "BORN"]);
        assert_eq!(table.fields(), 2);
        assert_eq!(table.format, "dBase III");
        assert_eq!(table.len(), 3);
        // The widths come from the header, not from the records: that is the whole point.
        assert_eq!(table.widths, [1, 10, 10]);
        // Text is read, numbers are read down their last digit.
        assert_eq!(table.aligns, [Align::Left, Align::Left, Align::Left]);
        // Nothing was read to get this far.
        assert!(table.rows().is_empty());

        let mut source = crate::source::BytesSource::new(bytes.clone());
        assert_eq!(table.cells(&mut source, None, 0), ["", "mb", "1975-03-12"]);
        // A deleted record is still a record, with a `*` in the first column: a table whose
        // deleted rows vanish is one whose count does not add up.
        assert_eq!(table.cells(&mut source, None, 1), ["*", "gone", ""]);
        assert_eq!(table.cells(&mut source, None, 2), ["", "ada", "1815-12-10"]);
        // Row n is one seek, and the offsets say where.
        assert_eq!(table.offset(1), table.offset(0) + 19);
        assert_eq!(table.row_at(table.offset(2) + 3), 2);
        assert_eq!(
            table.line(&mut source, None, 2, 0, 40),
            "  │ ada        │ 1815-12-10"
        );

        // A header that claims more records than the file holds is a truncated table, and
        // what is there is what can be read.
        let short = Table::dbase(&bytes, (bytes.len() - 20) as u64).expect("still a table");
        assert_eq!(short.len(), 2);
        assert!(short.cut);

        // Anything that is not a table we know is not one.
        assert!(Table::dbase(b"not a dbf at all, really", 24).is_none());
        assert!(Table::dbase(&[], 0).is_none());
    }

    /// A table as a DOS program in Germany would have written it: language byte `0x0F`, text
    /// in code page 437, and a numeric field.
    fn german_dos_dbf() -> Vec<u8> {
        let mut out = Vec::new();
        let header_len = 32 + 2 * 32 + 1;
        let record_len = 1 + 12 + 8;
        out.push(0x03);
        out.extend_from_slice(&[126, 1, 15]);
        out.extend_from_slice(&1u32.to_le_bytes());
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(&(record_len as u16).to_le_bytes());
        let mut rest = [0u8; 20];
        rest[29 - 12] = 0x0f; // the language byte: German OEM, which means 437
        out.extend_from_slice(&rest);
        let mut field = |name: &str, kind: u8, len: u8, decimals: u8| {
            let mut raw = [0u8; 32];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            raw[11] = kind;
            raw[16] = len;
            raw[17] = decimals;
            out.extend_from_slice(&raw);
        };
        field("LEISTUNG", b'C', 12, 0);
        field("PREIS", b'N', 8, 2);
        out.push(0x0d);
        out.push(b' ');
        // `temporäre` with the 437 byte for the umlaut, which is not the 1252 one.
        out.extend_from_slice(&[
            b't', b'e', b'm', b'p', b'o', b'r', 0x84, b'r', b'e', b' ', b' ', b' ',
        ]);
        out.extend_from_slice(b"   44.00");
        out.push(0x1a);
        out
    }

    #[test]
    fn a_dos_table_is_read_in_its_own_code_page_and_its_numbers_sit_to_the_right() {
        let bytes = german_dos_dbf();
        let len = bytes.len() as u64;
        let table = Table::dbase(&bytes, len).expect("a dBase table");
        assert_eq!(table.columns, ["*", "LEISTUNG", "PREIS"]);
        // The numeric field is read down its last digit; the flag and the text are not.
        assert_eq!(table.aligns, [Align::Left, Align::Left, Align::Right]);

        let mut source = crate::source::BytesSource::new(bytes.clone());
        // Read as Windows-1252 this would be `tempor„re`, which is what the bug looked like.
        assert_eq!(
            table.cells(&mut source, None, 0),
            ["", "temporäre", "44.00"]
        );
        // `PREIS` is eight wide, and `44.00` ends where the column does.
        assert_eq!(
            table.line(&mut source, None, 0, 0, 40),
            "  │ temporäre    │    44.00"
        );
        // The heading sits over its own column, on the same side as the cells under it.
        assert_eq!(table.header(0, 40), "* │ LEISTUNG     │    PREIS");
    }

    /// A dBase III table with a memo field, and the `.dbt` that goes with it.
    fn dbase_with_memo() -> (Vec<u8>, Vec<u8>) {
        let mut out = Vec::new();
        let header_len = 32 + 2 * 32 + 1;
        let record_len = 1 + 10 + 10;
        out.push(0x83); // dBase III with a memo file
        out.extend_from_slice(&[126, 1, 15]);
        out.extend_from_slice(&2u32.to_le_bytes());
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(&(record_len as u16).to_le_bytes());
        out.extend_from_slice(&[0u8; 20]);
        let mut field = |name: &str, kind: u8, len: u8| {
            let mut raw = [0u8; 32];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            raw[11] = kind;
            raw[16] = len;
            out.extend_from_slice(&raw);
        };
        field("NAME", b'C', 10);
        field("NOTE", b'M', 10);
        out.push(0x0d);
        // The deletion flag, the name, and the block number written as text and
        // right-aligned in its field; a memo that was never written points at block 0.
        out.push(b' ');
        out.extend_from_slice(b"mb        ");
        out.extend_from_slice(b"         1");
        out.push(b' ');
        out.extend_from_slice(b"ada       ");
        out.extend_from_slice(b"         0");
        out.push(0x1a);

        // The memo file: blocks of 512, the text ending at the end-of-file byte.
        let mut dbt = vec![0u8; 512];
        let mut block = vec![0u8; 512];
        let text = b"a note\nover two lines";
        block[..text.len()].copy_from_slice(text);
        block[text.len()] = 0x1a;
        dbt.extend_from_slice(&block);
        (out, dbt)
    }

    #[test]
    fn a_memo_field_says_what_the_file_beside_the_table_holds() {
        let (bytes, dbt) = dbase_with_memo();
        let table = Table::dbase(&bytes, bytes.len() as u64).expect("a dBase table");
        let mut source = crate::source::BytesSource::new(bytes);
        let mut memo = crate::source::BytesSource::new(dbt);

        // Without the file there is only the block number, which is what there was before.
        assert_eq!(table.cells(&mut source, None, 0), ["", "mb", "memo:1"]);
        // With it, what the block holds — on one line, because a cell has one line to give.
        assert_eq!(
            table.cells(&mut source, Some(&mut memo), 0),
            ["", "mb", "a note over two lines"]
        );
        // Block 0 is where an empty memo points, and an empty cell is what it says.
        assert_eq!(
            table.cells(&mut source, Some(&mut memo), 1),
            ["", "ada", ""]
        );
    }

    #[test]
    fn the_three_memo_formats_are_each_recognised_by_what_they_hold() {
        let text = |s: &str| s.as_bytes().to_vec();

        // dBase III: blocks of 512, the text ending at the end-of-file byte.
        let mut three = vec![0u8; 512];
        three.extend_from_slice(&[0u8; 512]);
        three[512..512 + 5].copy_from_slice(&text("three"));
        three[512 + 5] = 0x1a;
        let mut source = crate::source::BytesSource::new(three);
        assert_eq!(
            memo_text(&mut source, 1, Charset::Cp437, MEMO_CELL).as_deref(),
            Some("three")
        );

        // dBase IV: the block size in the header, `FF FF 08 00` and the length in the block.
        let mut four = vec![0u8; 1024];
        four[20] = 0x00;
        four[21] = 0x04; // 1024, little-endian
        let mut block = vec![0xff, 0xff, 0x08, 0x00];
        block.extend_from_slice(&(8u32 + 4).to_le_bytes());
        block.extend_from_slice(&text("four"));
        block.resize(1024, 0);
        four.extend_from_slice(&block);
        let mut source = crate::source::BytesSource::new(four);
        assert_eq!(
            memo_text(&mut source, 1, Charset::Cp437, MEMO_CELL).as_deref(),
            Some("four")
        );

        // FoxPro: the block size big-endian at 6, and the type and length in the block.
        let mut fox = vec![0u8; 64];
        fox[6] = 0x00;
        fox[7] = 0x40; // 64, big-endian
        let mut block = 1u32.to_be_bytes().to_vec(); // a text memo
        block.extend_from_slice(&3u32.to_be_bytes());
        block.extend_from_slice(&text("fox"));
        block.resize(64, 0);
        fox.extend_from_slice(&block);
        let mut source = crate::source::BytesSource::new(fox);
        assert_eq!(
            memo_text(&mut source, 1, Charset::Cp437, MEMO_CELL).as_deref(),
            Some("fox")
        );

        // A block past the end of the file is not a block: the cell says the number again.
        assert_eq!(memo_text(&mut source, 99, Charset::Cp437, MEMO_CELL), None);
    }

    /// A table from its parts: the version and language bytes, the fields as
    /// `(name, type, width, decimals, flags)`, and the records, each without its deletion
    /// flag. A Visual FoxPro version gets the 263 bytes that format keeps after its fields.
    pub(crate) fn build(
        version: u8,
        language: u8,
        fields: &[(&str, u8, u8, u8, u8)],
        records: &[(u8, Vec<u8>)],
    ) -> Vec<u8> {
        let fox = matches!(version, 0x30..=0x32);
        let header_len = 32 + fields.len() * 32 + 1 + if fox { 263 } else { 0 };
        let record_len = 1 + fields.iter().map(|f| f.2 as usize).sum::<usize>();
        let mut out = vec![version, 126, 1, 15];
        out.extend_from_slice(&(records.len() as u32).to_le_bytes());
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(&(record_len as u16).to_le_bytes());
        let mut rest = [0u8; 20];
        rest[29 - 12] = language;
        out.extend_from_slice(&rest);
        for (name, kind, len, decimals, flags) in fields {
            let mut raw = [0u8; 32];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            raw[11] = *kind;
            raw[16] = *len;
            raw[17] = *decimals;
            raw[18] = *flags;
            out.extend_from_slice(&raw);
        }
        out.push(0x0d);
        if fox {
            // Where a table of a database keeps the path of its container.
            let mut link = [0u8; 263];
            link[..9].copy_from_slice(b"shop.dbc\0");
            out.extend_from_slice(&link);
        }
        for (flag, record) in records {
            assert_eq!(
                record.len() + 1,
                record_len,
                "a record of the declared length"
            );
            out.push(*flag);
            out.extend_from_slice(record);
        }
        out.push(0x1a);
        out
    }

    /// A Visual FoxPro table with what only that format has: `NAME V(10)` that may be
    /// empty of a value, `CODE Q(4)`, `QTY I` that may be, `NOTE C(5)` — and `_NullFlags`,
    /// whose bits are, lowest first: `NAME` is short, `NAME` is null, `CODE` is short,
    /// `QTY` is null.
    pub(crate) fn fox_bytes() -> Vec<u8> {
        let record = |name: &[u8; 10], code: [u8; 4], qty: i32, note: &[u8; 5], flags: u8| {
            let mut out = name.to_vec();
            out.extend_from_slice(&code);
            out.extend_from_slice(&qty.to_le_bytes());
            out.extend_from_slice(note);
            out.push(flags);
            out
        };
        build(
            0x30,
            0x03,
            &[
                ("NAME", b'V', 10, 0, 0x02),
                ("CODE", b'Q', 4, 0, 0x04),
                ("QTY", b'I', 4, 0, 0x02),
                ("NOTE", b'C', 5, 0, 0),
                ("_NullFlags", b'0', 1, 0, 0x05),
            ],
            &[
                // `ada`, three long by the last byte; a code of all four bytes.
                (
                    b' ',
                    record(b"ada\0\0\0\0\0\0\x03", [1, 2, 3, 4], 7, b"x    ", 0b0001),
                ),
                // No name and no quantity; a code of two bytes.
                (
                    b' ',
                    record(
                        b"\0\0\0\0\0\0\0\0\0\0",
                        [0xab, 0xcd, 0, 2],
                        0,
                        b"     ",
                        0b1110,
                    ),
                ),
                // A name of the whole width, which needs no length written.
                (
                    b'*',
                    record(b"abcdefghij", [0, 0, 0, 0], -5, b"     ", 0b0100),
                ),
            ],
        )
    }

    #[test]
    fn a_visual_foxpro_table_reads_its_nulls_and_its_fields_of_varying_length() {
        let bytes = fox_bytes();
        let table = Table::dbase(&bytes, bytes.len() as u64).expect("a table");
        assert_eq!(table.format, "Visual FoxPro");
        // The field the flags live in is the table's own business, and no column.
        assert_eq!(table.columns, ["*", "NAME", "CODE", "QTY", "NOTE"]);
        assert_eq!(table.fields(), 4);
        let mut source = crate::source::BytesSource::new(bytes.clone());
        assert_eq!(
            table.cells(&mut source, None, 0),
            ["", "ada", "01020304", "7", "x"]
        );
        // No value is not an empty value, and a short code is as long as it says.
        assert_eq!(
            table.cells(&mut source, None, 1),
            ["", "NULL", "ABCD", "NULL", ""]
        );
        assert_eq!(
            table.cells(&mut source, None, 2),
            ["*", "abcdefghij", "", "-5", ""]
        );
        // The hidden field still takes its room: the fields after it would be misplaced
        // otherwise, and so would the record length.
        let dbf = Dbf::parse(&bytes).unwrap();
        assert_eq!(dbf.nulls, Some((24, 1)));
        assert_eq!(dbf.record_len, 25);
        let bits: Vec<_> = dbf.fields.iter().map(|f| (f.var_bit, f.null_bit)).collect();
        assert_eq!(
            bits,
            [
                (Some(0), Some(1)),
                (Some(2), None),
                (None, Some(3)),
                (None, None)
            ]
        );
    }

    #[test]
    fn a_table_without_null_flags_has_no_nulls_whatever_its_fields_say() {
        // The flag on the field and no `_NullFlags` to keep its bit in: nothing is null.
        let bytes = build(
            0x30,
            0x03,
            &[("NAME", b'C', 3, 0, 0x02)],
            &[(b' ', b"abc".to_vec())],
        );
        let table = Table::dbase(&bytes, bytes.len() as u64).unwrap();
        let mut source = crate::source::BytesSource::new(bytes);
        assert_eq!(table.cells(&mut source, None, 0), ["", "abc"]);
    }

    #[test]
    fn the_containers_of_visual_foxpro_are_tables_by_their_names() {
        for name in [
            "shop.dbc",
            "FORM.SCX",
            "report.frx",
            "lib.vcx",
            "main.mnx",
            "app.PJX",
            "label.lbx",
            "people.dbf",
        ] {
            assert_eq!(Table::reads(name), BUILT_IN, "{name}");
            assert!(crate::table::dbase_name(name), "{name}");
        }
        // Their memo files are not tables, and neither is a name that only ends alike.
        for name in ["shop.dct", "form.sct", "a.dbfx", "dbf"] {
            assert!(!crate::table::dbase_name(name), "{name}");
        }
    }

    #[test]
    fn a_memo_is_read_whole_where_a_cell_has_one_line_of_it() {
        let (table, memo) = dbase_with_memo();
        let dbf = Dbf::parse(&table).unwrap();
        let field = (0..dbf.field_count())
            .find(|n| dbf.is_memo(*n))
            .expect("a memo field");
        let record = |n: usize| {
            let at = dbf.offset(n) as usize;
            table[at..at + dbf.record_len].to_vec()
        };
        let mut source = crate::source::BytesSource::new(memo);
        let whole = (0..dbf.records)
            .find_map(|n| dbf.memo_whole(&record(n), field, &mut source))
            .expect("one record has a memo");
        // The line breaks the cell had to flatten are there.
        let cell = Table::dbase(&table, table.len() as u64)
            .unwrap()
            .cells(
                &mut crate::source::BytesSource::new(table.clone()),
                Some(&mut source),
                0,
            )
            .join("|");
        assert_eq!(whole, "a note\nover two lines");
        assert!(cell.contains("a note over two lines"), "{cell}");
        // A field that is not a memo has no memo to read.
        let other = (0..dbf.field_count()).find(|n| !dbf.is_memo(*n)).unwrap();
        assert_eq!(dbf.memo_whole(&record(0), other, &mut source), None);
    }

    #[test]
    fn the_structure_pane_lists_every_field_with_its_type_offset_and_flags() {
        let bytes = fox_bytes();
        let len = bytes.len() as u64;
        let found = crate::structure::read(&mut &bytes[..]).expect("a table has a structure");
        assert_eq!(found.format, "DBF");
        assert_eq!(found.summary, "Visual FoxPro, 3 records of 4 fields");
        let row = |label: &str| {
            let row = found
                .rows
                .iter()
                .find(|r| r.label == label)
                .unwrap_or_else(|| panic!("no row {label}"));
            (row.raw.clone(), row.value.clone())
        };
        assert_eq!(row("records"), (String::new(), "3".into()));
        assert_eq!(row("record length"), (String::new(), "25 bytes".into()));
        assert_eq!(row("last update"), (String::new(), "2026-01-15".into()));
        assert_eq!(row("code page"), ("03".into(), "CP1252".into()));
        assert_eq!(row("NAME"), ("V 10".into(), "offset 1 · nullable".into()));
        assert_eq!(row("CODE"), ("Q 4".into(), "offset 11 · binary".into()));
        assert_eq!(row("QTY"), ("I 4".into(), "offset 15 · nullable".into()));
        assert_eq!(row("NOTE"), ("C 5".into(), "offset 19".into()));
        // The hidden field is part of the record, and the pane is where that is said.
        assert_eq!(
            row("_NullFlags"),
            ("0 1".into(), "offset 24 · system".into())
        );
        assert!(!found.truncated);

        // Decimals, and a table that does not name its code page.
        let bytes = build(
            0x03,
            0,
            &[("QTY", b'N', 7, 2, 0)],
            &[(b' ', b"   1.50".to_vec())],
        );
        let found = crate::structure::read(&mut &bytes[..]).unwrap();
        let qty = found.rows.iter().find(|r| r.label == "QTY").unwrap();
        assert_eq!(
            (qty.raw.as_str(), qty.value.as_str()),
            ("N 7.2", "offset 1")
        );
        let page = found.rows.iter().find(|r| r.label == "code page").unwrap();
        assert_eq!(page.value, "not named");

        // A version byte is not a signature: text that begins with a `0`, and a header
        // whose fields do not add up to its record, are not tables.
        assert!(structure(&mut &b"0123456789"[..], b"0123456789", 10).is_none());
        let mut wrong = fox_bytes();
        wrong[10] = 99;
        assert!(structure(&mut &wrong[..], &wrong, len).is_none());
        let text = "0 is where this file starts, and it goes on for a good while after that.";
        assert!(crate::structure::read(&mut text.as_bytes()).is_none_or(|s| s.format != "DBF"));
    }

    #[test]
    fn the_numeric_columns_are_summed_to_the_last_decimal() {
        let record = |qty: &str, n: i32, price: i64, weight: f64, name: &str| {
            let mut out = qty.as_bytes().to_vec();
            out.extend_from_slice(&n.to_le_bytes());
            out.extend_from_slice(&price.to_le_bytes());
            out.extend_from_slice(&weight.to_le_bytes());
            out.extend_from_slice(name.as_bytes());
            (b' ', out)
        };
        let bytes = build(
            0x30,
            0x03,
            &[
                ("QTY", b'N', 7, 2, 0),
                ("N", b'I', 4, 0, 0),
                ("PRICE", b'Y', 8, 4, 0),
                ("WEIGHT", b'B', 8, 2, 0),
                ("NAME", b'C', 3, 0, 0),
            ],
            &[
                record("   0.10", 1, 10_000, 0.5, "a  "),
                record("   0.20", 2, 25_000, 0.25, "b  "),
                // An empty number adds nothing and is not counted.
                record("       ", -4, -5_000, 1.0, "c  "),
                record("  -1.05", 0, 1, 2.0, "d  "),
            ],
        );
        let table = Table::dbase(&bytes, bytes.len() as u64).unwrap();
        let mut source = crate::source::BytesSource::new(bytes);
        let sums = table
            .sums(&mut source, &[0, 1, 2, 3])
            .expect("a dBase table");
        let shown: Vec<(&str, &str, usize)> = sums
            .iter()
            .map(|s| (s.name.as_str(), s.sum.as_str(), s.count))
            .collect();
        // 0.10 + 0.20 is 0.30 here, which in doubles it is not.
        assert_eq!(
            shown,
            [
                ("QTY", "-0.75", 3),
                ("N", "-1", 4),
                ("PRICE", "3.0001", 4),
                ("WEIGHT", "3.75", 4)
            ]
        );
        // Over some of the records only, and over none.
        let some = table.sums(&mut source, &[0, 1]).unwrap();
        assert_eq!((some[0].sum.as_str(), some[0].count), ("0.30", 2));
        let none = table.sums(&mut source, &[]).unwrap();
        assert_eq!((none[0].sum.as_str(), none[0].count), ("0", 0));
        // A table of text has nothing to sum, and a CSV is not asked.
        let text = dbase_bytes();
        let table = Table::dbase(&text, text.len() as u64).unwrap();
        let mut source = crate::source::BytesSource::new(text);
        assert_eq!(table.sums(&mut source, &[0]), Some(Vec::new()));
        assert_eq!(
            Table::separated("a,b\n1,2\n", None).sums(&mut source, &[0]),
            None
        );
    }

    #[test]
    fn the_dos_pages_are_tables_of_their_own_and_go_both_ways() {
        // One letter each page puts where no other does: the byte, and what it reads as.
        for (language, byte, letter, name) in [
            (0x02u8, 0xb5u8, 'Á', "CP850"),
            (0x64, 0x88, 'ł', "CP852"),
            (0x66, 0x9b, 'ø', "CP865"),
            (0x17, 0x9d, 'Ø', "CP865"),
            (0x1f, 0xfd, 'ř', "CP852"),
            (0x10, 0xd5, 'ı', "CP850"),
        ] {
            let bytes = build(
                0x03,
                language,
                &[("T", b'C', 2, 0, 0)],
                &[(b' ', vec![byte, b' '])],
            );
            let dbf = Dbf::parse(&bytes).unwrap();
            assert_eq!(dbf.page_name(), name);
            assert!(dbf.exact, "{name} is held exactly");
            assert_eq!(
                dbf.cells(&bytes[dbf.first as usize..], None)[1],
                letter.to_string()
            );
            assert_eq!(dbf.encoding.encode(&letter.to_string()), Ok(vec![byte]));
        }
        // Every byte of the upper half is a character, and each character one byte: what
        // is read from a table is written back as the same bytes.
        for high in [&CP850_HIGH, &CP852_HIGH, &CP865_HIGH] {
            let page = Charset::Dos("page", high);
            let all: Vec<u8> = (0x80..=0xff).collect();
            let text = page.decode(&all);
            assert_eq!(text.chars().count(), 128);
            assert_eq!(page.encode(&text), Ok(all));
            assert_eq!(page.encode("plain ASCII"), Ok(b"plain ASCII".to_vec()));
        }
        // What a page has not got is refused by name, never written as another letter.
        assert_eq!(Charset::Dos("CP852", &CP852_HIGH).encode("è"), Err('è'));
        assert_eq!(Charset::Dos("CP850", &CP850_HIGH).encode("5 €"), Err('€'));
    }

    #[test]
    fn a_table_that_names_no_code_page_can_be_told_one() {
        // Clipper's header: the language byte is zero. `8a` is `è` in 437 and 850, and `Š`
        // in Windows-1252, which is how it is read until somebody says.
        let bytes = build(
            0x03,
            0,
            &[("T", b'C', 5, 0, 0)],
            &[(b' ', b"caff\x8a".to_vec())],
        );
        let mut dbf = Dbf::parse(&bytes).unwrap();
        let record = &bytes[dbf.first as usize..];
        assert!(!dbf.names_its_page());
        assert!(!dbf.exact);
        assert_eq!(dbf.cells(record, None)[1], "caffŠ");
        assert!(dbf.set_page(Some(Page::Cp850)));
        assert_eq!(dbf.cells(record, None)[1], "caffè");
        assert_eq!(dbf.page_name(), "CP850");
        assert!(dbf.exact, "a page that was chosen is written in full");
        // And back to saying nothing.
        assert!(dbf.set_page(None));
        assert!(!dbf.exact);
        assert_eq!(dbf.cells(record, None)[1], "caffŠ");
        // The wheel goes through every page once and then back to the header's silence.
        let mut seen = Vec::new();
        let mut at = None;
        loop {
            at = Page::after(at);
            match at {
                Some(page) => seen.push(page),
                None => break,
            }
        }
        assert_eq!(seen, Page::ALL);
        // A header that names its page is not argued with.
        let named = build(
            0x03,
            0x01,
            &[("T", b'C', 5, 0, 0)],
            &[(b' ', b"caff\x8a".to_vec())],
        );
        let mut dbf = Dbf::parse(&named).unwrap();
        assert!(dbf.names_its_page());
        assert!(!dbf.set_page(Some(Page::Cp1252)));
        assert_eq!(dbf.page_name(), "CP437");
    }
}
