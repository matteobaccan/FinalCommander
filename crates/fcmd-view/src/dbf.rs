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
}

/// One field as the header declares it: `QTY`, `N`, 8 wide, 2 decimals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldInfo {
    pub name: String,
    pub kind: char,
    pub len: usize,
    pub decimals: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Field {
    pub(crate) name: String,
    pub(crate) kind: u8,
    /// Where in the record the field starts, the deletion flag included.
    pub(crate) at: usize,
    pub(crate) len: usize,
    pub(crate) decimals: u8,
    /// Visual FoxPro's flags: `0x0C` is a field the table numbers by itself.
    pub(crate) flags: u8,
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
            fields.push(Field {
                name: name.trim().to_string(),
                kind: raw[11],
                at,
                len,
                decimals,
                flags: raw[18],
            });
            at += len;
            cursor += 32;
        }
        if fields.is_empty() {
            return None;
        }
        Some(Dbf {
            first: header_len as u64,
            record_len,
            records,
            fields,
            encoding,
            flavour,
            exact: is_exact(head[29]),
        })
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
            out.push(self.cell(f, raw, memo.as_deref_mut()));
        }
        out
    }

    fn cell(
        &self,
        f: &Field,
        raw: &[u8],
        memo: Option<&mut (dyn crate::source::Source + '_)>,
    ) -> String {
        let text = |bytes: &[u8]| self.encoding.decode(bytes);
        match f.kind {
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
                let block = if f.len == 4 && raw.len() >= 4 {
                    u32::from_le_bytes([raw[0], raw[1], raw[2], raw[3]]) as usize
                } else {
                    text(raw).trim().parse::<usize>().unwrap_or(0)
                };
                if block == 0 {
                    return String::new(); // where an empty memo points
                }
                // What it points at, when the file beside the table was there to open; the
                // block number otherwise, which is all there was to say before.
                match memo.and_then(|file| memo_text(file, block, self.encoding)) {
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
                format!("{}.{:04}", n / 10_000, (n % 10_000).abs())
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

/// The text encoding a DBF's language byte names. The ones that are actually met; anything
/// else is read as Windows-1252, which at least never fails and is right for most of Europe.
fn encoding_for(byte: u8) -> Charset {
    match byte {
        // The 437 family: the US driver, and the western-European OEM drivers that all
        // wrote plain 437 — the German one among them, which is the byte a table written
        // under DOS in Germany carries.
        0x01 | 0x09 | 0x0b | 0x0d | 0x0f | 0x11 | 0x15 | 0x18 | 0x19 | 0x1b => Charset::Cp437,
        // 850, 860, 861, 863, 865 and the other western DOS tables: `encoding_rs` has none
        // of them, and 1252 agrees with all of them on the accented letters, which is what
        // a field holds. They differ in the box-drawing half, which a field does not.
        0x02 | 0x0a | 0x0e | 0x10 | 0x12 | 0x14 | 0x16 | 0x17 | 0x1a | 0x1d | 0x24 | 0x25
        | 0x37 | 0x66 | 0x67 | 0x6c => Charset::Table(encoding_rs::WINDOWS_1252),
        // 852 and the eastern DOS tables, likewise against 1250.
        0x1f | 0x22 | 0x23 | 0x40 | 0x64 | 0x68 | 0x69 | 0xc8 => {
            Charset::Table(encoding_rs::WINDOWS_1250)
        }
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
/// Reading forgives an approximation and writing does not. A table marked 850 is read as
/// Windows-1252 — and whatever that makes of its accented letters is at least shown as it
/// stands; written through the same approximation an `è` would land on the byte that is `Þ`
/// to the program the table belongs to. So only the pages held exactly are written in full,
/// and the others take ASCII alone. A Clipper table, whose language byte is zero, is one of
/// the others: its header does not say what its text is in.
fn is_exact(byte: u8) -> bool {
    matches!(
        byte,
        // 437 and the national drivers that wrote plain 437.
        0x01 | 0x09 | 0x0b | 0x0d | 0x0f | 0x11 | 0x15 | 0x18 | 0x19 | 0x1b
            // The Windows pages, 866 and the far-eastern ones: each is the table it names.
            | 0x03 | 0x57 | 0x58 | 0x59 | 0xc8 | 0xc9 | 0xca | 0xcb | 0x7d | 0x7e
            | 0x26 | 0x65 | 0x13 | 0x4d | 0x7a | 0x4e | 0x79 | 0x4f | 0x78 | 0x50 | 0x7c
    )
}

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
) -> Option<String> {
    /// No cell is forty characters wide, let alone this; it is a bound on the read, so that
    /// a length field with nonsense in it cannot ask for the whole machine.
    const MAX_MEMO: usize = 64 * 1024;

    let len = source.len();
    let head = source.read_vec(0, 32).ok()?;
    if head.len() < 32 {
        return None;
    }
    let read = |source: &mut dyn crate::source::Source, at: u64, n: usize| -> Option<Vec<u8>> {
        if at >= len {
            return None;
        }
        let n = n.min(MAX_MEMO).min((len - at) as usize);
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
    let body = read(source, at, MAX_MEMO)?;
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
    Table(&'static encoding_rs::Encoding),
}

impl Charset {
    pub(crate) fn decode(self, bytes: &[u8]) -> String {
        match self {
            Charset::Cp437 => crate::decode::Codepage::Cp437.decode(bytes),
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
        };
        match text.chars().find(|c| !page.can_encode(&c.to_string())) {
            Some(bad) => Err(bad),
            None => Ok(page.encode(text)),
        }
    }
}

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
            memo_text(&mut source, 1, Charset::Cp437).as_deref(),
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
            memo_text(&mut source, 1, Charset::Cp437).as_deref(),
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
            memo_text(&mut source, 1, Charset::Cp437).as_deref(),
            Some("fox")
        );

        // A block past the end of the file is not a block: the cell says the number again.
        assert_eq!(memo_text(&mut source, 99, Charset::Cp437), None);
    }
}
