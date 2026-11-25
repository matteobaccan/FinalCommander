//! Writing a dBase table: a cell, a new record, the deletion flag, `Pack` (SPEC 05 §1.11).
//!
//! The tables this is for are still opened every day by programs written in Clipper and
//! Harbour, so every rule here is on the side of the table:
//!
//! * **The file is opened for writing only for the moment of a write**, and closed again. A
//!   table another program holds is a refusal that says so, not a wait.
//! * **Nothing is written behind an index nobody was asked about.** An index holds a copy
//!   of the keys; a record changed behind its back leaves it pointing at what is no longer
//!   there, and the program that trusts it then finds the wrong record or none. This
//!   program does not keep an index right, so before it writes a table that has any it
//!   names them and is told to go on — the reindexing afterwards being the business of the
//!   program the indexes belong to — and an index that turns up later is asked about again
//!   ([`check_indexes`]).
//! * **A value is written exactly or not at all**: too long, not a number, a date that is
//!   not one, a letter the table's code page has no byte for — each is refused by name.
//!   Nothing is cut, rounded or replaced with a `?`.
//! * **The original changes only by these record writes.** `Pack`, which is the one
//!   operation that moves every record, writes a new file beside the table and renames it
//!   into place, so there is no moment at which half a table is on disk.
//! * **The backup is a copy with the date in its name**, `CLIENTI.DBF` →
//!   `CLIENTI.20261125-083015.DBF`, the extension kept last so the copy still opens as a
//!   table, and the memo file and the indexes copied with it.
//!
//! What only reads the table and writes another file is here too: the export of some of its
//! records to CSV or to a table of the same structure, which never writes over a file that
//! is there.
//!
//! Every write leaves the header saying the truth: the record count, the date of the last
//! change in bytes 1–3, and the `1A` that ends the records.

use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use crate::dbf::Dbf;

/// The byte dBase puts after the last record.
const END_OF_FILE: u8 = 0x1a;

/// A moment, as the caller's clock reads it in local time. Passed in rather than read here,
/// so that what a backup is called and what date a header carries can be tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl Stamp {
    /// Read `2026-11-25 08:30:15`, which is how the rest of the program writes a moment.
    pub fn parse(text: &str) -> Option<Stamp> {
        let (date, time) = text.trim().split_once(' ')?;
        let mut date = date.split('-');
        let mut time = time.split(':');
        let stamp = Stamp {
            year: date.next()?.parse().ok()?,
            month: date.next()?.parse().ok()?,
            day: date.next()?.parse().ok()?,
            hour: time.next()?.parse().ok()?,
            minute: time.next()?.parse().ok()?,
            second: time.next()?.parse().ok()?,
        };
        (date.next().is_none() && time.next().is_none()).then_some(stamp)
    }

    /// `20261125-083015`: year, month, day, then the time, so the copies of one table sort
    /// by name in the order they were made.
    pub fn in_a_name(&self) -> String {
        format!(
            "{:04}{:02}{:02}-{:02}{:02}{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// The three bytes of the header's last-update date: the year counted from 1900, which
    /// is what dBase III wrote and what Clipper and Harbour still write.
    fn in_a_header(&self) -> [u8; 3] {
        [
            self.year.saturating_sub(1900).min(255) as u8,
            self.month,
            self.day,
        ]
    }
}

/// Why a value typed for a field is not written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// A memo, a picture, a binary type: not written by this program yet.
    ReadOnly { kind: char },
    /// More bytes than the field has. Never cut to fit.
    TooLong { bytes: usize, room: usize },
    /// A character the table's code page has no byte for. `exact` is false when the table's
    /// header names no page this program holds exactly, and only ASCII is written to it.
    NoByte { ch: char, exact: bool },
    /// Not a number at all.
    Number,
    /// A number, but not one the field's width and decimals can hold.
    NumberFit { width: usize, decimals: u8 },
    /// Not a date of the calendar.
    Date,
    /// Neither true nor false.
    Logical,
}

/// Why a table was not written.
#[derive(Debug)]
pub enum WriteError {
    /// Index files sit beside the table that nobody has agreed to leave stale: all of the
    /// table's indexes as they are now, for the question to name.
    Indexes(Vec<PathBuf>),
    /// The file an export would write is already there, and nothing is written over.
    Exists(PathBuf),
    /// The table could not be opened for writing: read-only, or held by another program.
    Open(io::Error),
    /// A read or a write failed part of the way.
    Io(io::Error),
    /// What is on disk is not a table this program writes.
    NotATable,
    /// The fields on disk are not the fields on screen: another program restructured it.
    Changed,
    /// The file is shorter or longer than its header says it should be.
    Length,
    /// No such record, which is what a table packed by another program looks like from here.
    NoRecord,
    /// The value does not go in the field.
    Cell(Refused),
    /// The table numbers a field by itself, and a record appended here would not be numbered.
    Numbered,
    /// A file is already where `Pack` writes its new table: a `Pack` that did not finish.
    Leftover(PathBuf),
}

impl From<io::Error> for WriteError {
    fn from(e: io::Error) -> Self {
        WriteError::Io(e)
    }
}

/// What `Pack` did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Packed {
    pub removed: usize,
    pub left: usize,
}

// ── what sits beside a table ───────────────────────────────────────────────────────────────

/// The files of `table`'s folder whose name passes `wanted(stem, extension)`, both given in
/// lower case — DOS names come in capitals as often as not. Sorted, so a message that lists
/// them says the same thing twice running.
fn beside(table: &Path, wanted: impl Fn(&str, &str) -> bool) -> Vec<PathBuf> {
    let Some(own) = table.file_name() else {
        return Vec::new();
    };
    let dir = match table.parent() {
        Some(dir) if !dir.as_os_str().is_empty() => dir,
        _ => Path::new("."),
    };
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name == own {
            continue;
        }
        let name = name.to_string_lossy().to_lowercase();
        let Some((stem, ext)) = name.rsplit_once('.') else {
            continue;
        };
        if wanted(stem, ext) && entry.path().is_file() {
            out.push(entry.path());
        }
    }
    out.sort();
    out
}

fn stem_of(table: &Path) -> String {
    let name = table
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match name.rsplit_once('.') {
        Some((stem, _)) => stem.to_string(),
        None => name,
    }
}

/// The extensions an index goes by: Clipper's, FoxPro's (and the one of its database
/// container), dBase IV's, Harbour's and FoxBase's.
const INDEX_EXTENSIONS: [&str; 6] = ["ntx", "cdx", "mdx", "nsx", "idx", "dcx"];

/// The memo file's extension for a table's: what dBase and FoxPro write beside a `.dbf`,
/// and what Visual FoxPro writes beside each of the tables it keeps its own things in.
/// Lower case in, lower case out; empty for a name that is no table's.
pub fn memo_extensions(table_extension: &str) -> &'static [&'static str] {
    match table_extension {
        "dbf" => &["dbt", "fpt"],
        "dbc" => &["dct"],
        "scx" => &["sct"],
        "frx" => &["frt"],
        "vcx" => &["vct"],
        "mnx" => &["mnt"],
        "pjx" => &["pjt"],
        "lbx" => &["lbt"],
        _ => &[],
    }
}

fn extension_of(table: &Path) -> String {
    let name = table
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    match name.rsplit_once('.') {
        Some((_, ext)) => ext.to_string(),
        None => String::new(),
    }
}

/// Is this the stem of a copy [`backup`] made — `clienti.20261125-083015`? A backup of an
/// index is not an index of the table: nothing opens it, and nothing is owed to it.
fn is_backup(stem: &str) -> bool {
    let Some((_, stamp)) = stem.rsplit_once('.') else {
        return false;
    };
    let bytes = stamp.as_bytes();
    bytes.len() == 15
        && bytes[8] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 8 || b.is_ascii_digit())
}

/// The index files of `table`: the files beside it whose name begins with the table's own
/// and whose extension is an index's.
///
/// `PIPPO.DBF` is indexed by `PIPPO.CDX`, by `pippo1.ntx`, by `PIPPO_NOME.NTX`: a program
/// names its indexes after the table and whatever tells them apart, and the name is all
/// there is to go by — nothing in an index file says which table it belongs to. An index
/// named otherwise is another table's, or nobody's, and is left out of this.
pub fn index_files(table: &Path) -> Vec<PathBuf> {
    let own = stem_of(table);
    beside(table, |stem, ext| {
        INDEX_EXTENSIONS.contains(&ext) && stem.starts_with(&own) && !is_backup(stem)
    })
}

/// Is every index now beside `table` one of `agreed` — the ones whoever asked for a write
/// has been told of and has agreed to leave to be rebuilt? Otherwise all of the indexes as
/// they are now, for the question to be asked again.
///
/// Asked on the worker at the moment of every write: an index made by another program
/// between the question and the write is an index nobody was asked about.
pub fn check_indexes(table: &Path, agreed: &[PathBuf]) -> Result<(), WriteError> {
    let now = index_files(table);
    if now.iter().all(|index| agreed.contains(index)) {
        Ok(())
    } else {
        Err(WriteError::Indexes(now))
    }
}

/// The files a backup copies with the table: its memo file, and its indexes.
fn companions(table: &Path) -> Vec<PathBuf> {
    let own = stem_of(table);
    let memos = memo_extensions(&extension_of(table));
    let mut out = beside(table, |stem, ext| stem == own && memos.contains(&ext));
    out.extend(index_files(table));
    out
}

/// What the copy of `file` made at `at` is called: the moment goes before the extension,
/// which stays last so the copy is still a `.DBF` to everything that looks at names.
pub fn backup_name(file: &Path, at: Stamp) -> PathBuf {
    let name = file
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stamped = match name.rsplit_once('.') {
        Some((stem, ext)) => format!("{stem}.{}.{ext}", at.in_a_name()),
        None => format!("{name}.{}", at.in_a_name()),
    };
    file.with_file_name(stamped)
}

/// Copy the table, its memo file and its indexes beside themselves, each under its
/// [`backup_name`]. The copies made, the table's first.
///
/// A name that is already taken is an error and nothing is overwritten: a backup is the one
/// file that must never be the thing that got damaged. A copy that fails part of the way
/// takes its half-written files away again, so what is left in the folder is a whole backup
/// or none.
pub fn backup(table: &Path, at: Stamp) -> io::Result<Vec<PathBuf>> {
    let mut sources = vec![table.to_path_buf()];
    sources.extend(companions(table));
    let mut made = Vec::new();
    for source in &sources {
        let target = backup_name(source, at);
        match copy_new(source, &target) {
            Ok(()) => made.push(target),
            Err(e) => {
                // `AlreadyExists` is somebody else's file: it stays. Ours go.
                if e.kind() != io::ErrorKind::AlreadyExists {
                    let _ = std::fs::remove_file(&target);
                }
                for done in &made {
                    let _ = std::fs::remove_file(done);
                }
                return Err(io::Error::new(
                    e.kind(),
                    format!("{}: {e}", target.display()),
                ));
            }
        }
    }
    Ok(made)
}

/// Copy to a name nothing has yet, and keep the original's time: a backup dated by when it
/// was copied would say nothing about what is in it.
fn copy_new(source: &Path, target: &Path) -> io::Result<()> {
    let mut from = File::open(source)?;
    let mut to = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    io::copy(&mut from, &mut to)?;
    to.sync_all()?;
    if let Ok(modified) = from.metadata().and_then(|m| m.modified()) {
        let _ = to.set_modified(modified);
    }
    Ok(())
}

// ── a value, as the field wants it ─────────────────────────────────────────────────────────

/// The bytes of `text` for field `field`, exactly as long as the field: what is written over
/// the old value, or why nothing is.
///
/// The text is what table mode shows and what is typed over it: a date as `1975-03-12`, a
/// logical as `true` or `false`, a number with a point. Empty is the field's own empty —
/// spaces, which is what a record that was only appended holds.
pub fn encode(dbf: &Dbf, field: usize, text: &str) -> Result<Vec<u8>, Refused> {
    let Some(f) = dbf.fields.get(field) else {
        return Err(Refused::ReadOnly { kind: '?' });
    };
    let padded = |bytes: Vec<u8>, right: bool| -> Result<Vec<u8>, Refused> {
        if bytes.len() > f.len {
            return Err(Refused::TooLong {
                bytes: bytes.len(),
                room: f.len,
            });
        }
        let mut out = vec![b' '; f.len];
        let at = if right { f.len - bytes.len() } else { 0 };
        out[at..at + bytes.len()].copy_from_slice(&bytes);
        Ok(out)
    };
    match f.kind {
        b'C' => {
            // What the reader takes off the end is not put back on it.
            let text = text.trim_end();
            // A control character has no business in a field, and a page that is only
            // approximately known takes ASCII alone (see `Dbf::exact`).
            if let Some(ch) = text
                .chars()
                .find(|c| c.is_control() || (!dbf.exact && !c.is_ascii()))
            {
                return Err(Refused::NoByte {
                    ch,
                    exact: dbf.exact,
                });
            }
            let bytes = dbf
                .encoding
                .encode(text)
                .map_err(|ch| Refused::NoByte { ch, exact: true })?;
            padded(bytes, false)
        }
        b'N' | b'F' => {
            let number = number(text.trim(), f.len, f.decimals)?;
            padded(number.into_bytes(), true)
        }
        b'D' => {
            let text = text.trim();
            if text.is_empty() {
                return Ok(vec![b' '; f.len]);
            }
            let digits = date(text).ok_or(Refused::Date)?;
            padded(digits.into_bytes(), false)
        }
        b'L' => {
            let byte = match text.trim().to_ascii_lowercase().as_str() {
                "" => b' ',
                "true" | "t" | "y" | "yes" | ".t." => b'T',
                "false" | "f" | "n" | "no" | ".f." => b'F',
                _ => return Err(Refused::Logical),
            };
            padded(vec![byte], false)
        }
        other => Err(Refused::ReadOnly {
            kind: other as char,
        }),
    }
}

/// A typed number as an `N` or `F` field holds it: digits, a point, exactly `decimals`
/// digits after it. Empty stays empty.
///
/// Digits past the field's decimals are an error unless they are zeros: `12.50` goes into
/// two decimals and `12.505` does not, because the half cent it would lose is somebody's.
fn number(text: &str, width: usize, decimals: u8) -> Result<String, Refused> {
    if text.is_empty() {
        return Ok(String::new());
    }
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    let all_digits = |s: &str| s.chars().all(|c| c.is_ascii_digit());
    if !all_digits(whole) || !all_digits(fraction) || (whole.is_empty() && fraction.is_empty()) {
        return Err(Refused::Number);
    }
    let fits = Refused::NumberFit { width, decimals };
    let places = decimals as usize;
    let mut fraction = fraction.to_string();
    while fraction.len() > places && fraction.ends_with('0') {
        fraction.pop();
    }
    if fraction.len() > places {
        return Err(fits);
    }
    while fraction.len() < places {
        fraction.push('0');
    }
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let zero = whole == "0" && fraction.chars().all(|c| c == '0');
    let mut out = String::new();
    if negative && !zero {
        out.push('-');
    }
    out.push_str(whole);
    if places > 0 {
        out.push('.');
        out.push_str(&fraction);
    }
    if out.len() > width {
        return Err(fits);
    }
    Ok(out)
}

/// `1975-03-12`, or the eight digits themselves, as the eight digits a `D` field holds —
/// when it is a day there was.
fn date(text: &str) -> Option<String> {
    let digits: String = match text.len() {
        10 => {
            let bytes = text.as_bytes();
            if bytes[4] != b'-' || bytes[7] != b'-' {
                return None;
            }
            text.chars().filter(|c| *c != '-').collect()
        }
        8 => text.to_string(),
        _ => return None,
    };
    if digits.len() != 8 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let year: u32 = digits[0..4].parse().ok()?;
    let month: u32 = digits[4..6].parse().ok()?;
    let day: u32 = digits[6..8].parse().ok()?;
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    (day >= 1 && day <= days).then_some(digits)
}

// ── the writes ─────────────────────────────────────────────────────────────────────────────

/// A table open for a write: the file, its header as it is on disk now, and those bytes.
struct Open {
    file: File,
    dbf: Dbf,
    head: Vec<u8>,
}

impl Open {
    /// Where the records end, by the header's count.
    fn end(&self) -> u64 {
        self.dbf.offset(self.dbf.records)
    }
}

/// Open `table` for a write, having checked what would make the write wrong: a file that
/// is no longer the table on screen, a length that does not agree with the header. (The
/// indexes beside it are [`check_indexes`]'s to ask about, before this.)
///
/// `shown` is the header the reader has. The count may have moved — another program appends
/// while this one looks — but the fields may not: a write placed by a layout the file no
/// longer has would land in the middle of some other field.
fn open(table: &Path, shown: &Dbf) -> Result<Open, WriteError> {
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(table)
        .map_err(WriteError::Open)?;
    let len = file.metadata()?.len();
    let mut start = [0u8; 32];
    if len < 32 {
        return Err(WriteError::NotATable);
    }
    file.read_exact(&mut start)?;
    // dBase II keeps its header in another shape altogether, and is read here only as far
    // as its name.
    if start[0] == 0x02 {
        return Err(WriteError::NotATable);
    }
    let header_len = u16::from_le_bytes([start[8], start[9]]) as u64;
    if header_len < 33 || header_len > len {
        return Err(WriteError::NotATable);
    }
    let mut head = vec![0u8; header_len as usize];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut head)?;
    let dbf = Dbf::parse(&head).ok_or(WriteError::NotATable)?;
    if dbf.first != shown.first || dbf.record_len != shown.record_len || dbf.fields != shown.fields
    {
        return Err(WriteError::Changed);
    }
    let open = Open { file, dbf, head };
    if len < open.end() {
        return Err(WriteError::Length);
    }
    Ok(open)
}

/// Leave the header true after a write: `records` of them, last changed on `at`, and the
/// end-of-file byte after the last one where the file stops without it.
///
/// A byte that is already there and is not `1A` is left alone: it is not ours, and a file
/// with something after its records is a file to look at, not one to tidy.
fn close(mut open: Open, records: usize, at: Stamp) -> Result<(), WriteError> {
    let end = open.dbf.offset(records);
    if open.file.metadata()?.len() <= end {
        open.file.seek(SeekFrom::Start(end))?;
        open.file.write_all(&[END_OF_FILE])?;
    }
    let mut bytes = [0u8; 7];
    bytes[..3].copy_from_slice(&at.in_a_header());
    bytes[3..].copy_from_slice(&(records as u32).to_le_bytes());
    open.file.seek(SeekFrom::Start(1))?;
    open.file.write_all(&bytes)?;
    open.file.sync_all()?;
    Ok(())
}

/// Write `text` into field `field` of record `record`.
pub fn set_cell(
    table: &Path,
    shown: &Dbf,
    record: usize,
    field: usize,
    text: &str,
    at: Stamp,
) -> Result<(), WriteError> {
    // The value first: a refusal for what was typed should not need the file at all.
    let bytes = encode(shown, field, text).map_err(WriteError::Cell)?;
    let mut open = open(table, shown)?;
    if record >= open.dbf.records {
        return Err(WriteError::NoRecord);
    }
    let within = open.dbf.fields[field].at as u64;
    let start = open.dbf.offset(record);
    open.file.seek(SeekFrom::Start(start + within))?;
    open.file.write_all(&bytes)?;
    // A field that held no value holds one now: Visual FoxPro keeps "no value" as a bit of
    // its own beside the record's bytes, and a value written under a bit left set would
    // still read as none.
    if let (Some(bit), Some((flags, len))) = (open.dbf.fields[field].null_bit, open.dbf.nulls) {
        if bit / 8 < len {
            let place = start + (flags + bit / 8) as u64;
            let mut byte = [0u8; 1];
            open.file.seek(SeekFrom::Start(place))?;
            open.file.read_exact(&mut byte)?;
            if byte[0] & (1 << (bit % 8)) != 0 {
                byte[0] &= !(1 << (bit % 8));
                open.file.seek(SeekFrom::Start(place))?;
                open.file.write_all(&byte)?;
            }
        }
    }
    let records = open.dbf.records;
    close(open, records, at)
}

/// Set or clear the deletion flag of a record: `*` deletes, a space recalls. The record
/// stays where it is either way, which is the whole of what dBase means by deleting.
pub fn set_deleted(
    table: &Path,
    shown: &Dbf,
    record: usize,
    deleted: bool,
    at: Stamp,
) -> Result<(), WriteError> {
    let mut open = open(table, shown)?;
    if record >= open.dbf.records {
        return Err(WriteError::NoRecord);
    }
    open.file.seek(SeekFrom::Start(open.dbf.offset(record)))?;
    open.file.write_all(&[if deleted { b'*' } else { b' ' }])?;
    let records = open.dbf.records;
    close(open, records, at)
}

/// Set or clear the deletion flag of every one of `records` in one go, and say how many of
/// them it changed — a record that already was what is asked of it is left as it is.
///
/// All of them or none: a number that is no record of the table is found before the first
/// flag is written.
pub fn set_deleted_many(
    table: &Path,
    shown: &Dbf,
    records: &[usize],
    deleted: bool,
    at: Stamp,
) -> Result<usize, WriteError> {
    let mut open = open(table, shown)?;
    if records.iter().any(|record| *record >= open.dbf.records) {
        return Err(WriteError::NoRecord);
    }
    let want = if deleted { b'*' } else { b' ' };
    let mut changed = 0usize;
    for &record in records {
        let place = open.dbf.offset(record);
        let mut flag = [0u8; 1];
        open.file.seek(SeekFrom::Start(place))?;
        open.file.read_exact(&mut flag)?;
        // `*` is deleted and anything else is not, which is how every reader takes it.
        if (flag[0] == b'*') != deleted {
            open.file.seek(SeekFrom::Start(place))?;
            open.file.write_all(&[want])?;
            changed += 1;
        }
    }
    let count = open.dbf.records;
    close(open, count, at)?;
    Ok(changed)
}

/// Append an empty record, and say which one it is.
///
/// The record goes where the end-of-file byte was and the byte goes after it; the header's
/// count is written last, so a write cut short leaves a table that still says what it said.
pub fn append(table: &Path, shown: &Dbf, at: Stamp) -> Result<usize, WriteError> {
    let mut open = open(table, shown)?;
    // Visual FoxPro numbers such a field from a counter in the header, and a record that
    // arrives without its number breaks the one promise the field makes.
    let fox = matches!(open.head[0], 0x30..=0x32);
    if fox && open.dbf.fields.iter().any(|f| f.flags & 0x0c == 0x0c) {
        return Err(WriteError::Numbered);
    }
    let end = open.end();
    let len = open.file.metadata()?.len();
    // Past the records there is the end-of-file byte or nothing. Anything else is data the
    // header does not count, and a record written there would be written over it.
    if len > end + 1 {
        return Err(WriteError::Length);
    }
    if len == end + 1 {
        let mut last = [0u8; 1];
        open.file.seek(SeekFrom::Start(end))?;
        open.file.read_exact(&mut last)?;
        if last[0] != END_OF_FILE {
            return Err(WriteError::Length);
        }
    }
    let mut record = vec![b' '; open.dbf.record_len];
    for f in &open.dbf.fields {
        // The types kept as text are empty as spaces; the binary ones — an integer, a
        // currency, a FoxPro memo's four-byte block number — are empty as zeros.
        let text = match f.kind {
            b'C' | b'N' | b'F' | b'D' | b'L' => true,
            b'M' | b'G' | b'P' => f.len != 4,
            _ => false,
        };
        if !text {
            let to = (f.at + f.len).min(record.len());
            if f.at < to {
                record[f.at..to].fill(0);
            }
        }
    }
    // Visual FoxPro's flags are bits, not text: none set, but for the fields of varying
    // length, which are empty by being zero long — the bit that says "shorter than the
    // width", and the length in the last byte, which is the zero already there.
    if let Some((flags, len)) = open.dbf.nulls {
        let to = (flags + len).min(record.len());
        if flags < to {
            record[flags..to].fill(0);
        }
        for bit in open.dbf.fields.iter().filter_map(|f| f.var_bit) {
            if bit / 8 < len && flags + bit / 8 < record.len() {
                record[flags + bit / 8] |= 1 << (bit % 8);
            }
        }
    }
    record.push(END_OF_FILE);
    open.file.seek(SeekFrom::Start(end))?;
    open.file.write_all(&record)?;
    let new = open.dbf.records;
    close(open, new + 1, at)?;
    Ok(new)
}

/// How many records carry the deletion flag: what `Pack` is about to remove, counted before
/// anybody is asked. Reads, and writes nothing.
pub fn count_deleted(table: &Path, shown: &Dbf) -> Result<usize, WriteError> {
    let file = File::open(table).map_err(WriteError::Open)?;
    let mut reader = BufReader::new(file);
    let mut head = vec![0u8; shown.first as usize];
    reader
        .read_exact(&mut head)
        .map_err(|_| WriteError::NotATable)?;
    let dbf = Dbf::parse(&head).ok_or(WriteError::NotATable)?;
    if dbf.record_len != shown.record_len || dbf.fields != shown.fields {
        return Err(WriteError::Changed);
    }
    let mut record = vec![0u8; dbf.record_len];
    let mut deleted = 0usize;
    for _ in 0..dbf.records {
        reader
            .read_exact(&mut record)
            .map_err(|_| WriteError::Length)?;
        if record[0] == b'*' {
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Where `Pack` builds the new table: beside the old one, so the rename at the end is a
/// rename and not a copy across disks.
pub fn pack_file(table: &Path) -> PathBuf {
    let mut name = table
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_default();
    name.push(".fcmd-pack");
    table.with_file_name(name)
}

/// Take the deleted records out of the table for good.
///
/// The table is not rewritten where it stands: the records that stay are written to a new
/// file beside it, with its header, and only when that file is whole and on disk is it
/// renamed over the original. Until that instant the original is untouched, and after it the
/// new table is all there — there is no half-packed table to find after a power cut.
pub fn pack(table: &Path, shown: &Dbf, at: Stamp) -> Result<Packed, WriteError> {
    let open = open(table, shown)?;
    let target = pack_file(table);
    let new = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => WriteError::Leftover(target.clone()),
            _ => WriteError::Io(e),
        })?;
    let permissions = open.file.metadata().map(|m| m.permissions()).ok();
    let done = copy_kept(open, new, at);
    let done = done.and_then(|packed| {
        if let Some(permissions) = permissions {
            let _ = std::fs::set_permissions(&target, permissions);
        }
        std::fs::rename(&target, table)?;
        Ok(packed)
    });
    if done.is_err() {
        let _ = std::fs::remove_file(&target);
    }
    done
}

/// The body of [`pack`]: the header and the records that are not deleted, from the open
/// table into the new file, which is closed and on disk when this returns.
fn copy_kept(open: Open, new: File, at: Stamp) -> Result<Packed, WriteError> {
    let Open {
        mut file,
        dbf,
        mut head,
    } = open;
    file.seek(SeekFrom::Start(dbf.first))?;
    let mut reader = BufReader::new(file);
    let mut writer = BufWriter::new(new);
    // The header goes first with the old count and is put right at the end, when the count
    // is known: the table is read once.
    writer.write_all(&head)?;
    let mut record = vec![0u8; dbf.record_len];
    let mut left = 0usize;
    for _ in 0..dbf.records {
        reader
            .read_exact(&mut record)
            .map_err(|_| WriteError::Length)?;
        if record[0] == b'*' {
            continue;
        }
        writer.write_all(&record)?;
        left += 1;
    }
    writer.write_all(&[END_OF_FILE])?;
    head[1..4].copy_from_slice(&at.in_a_header());
    head[4..8].copy_from_slice(&(left as u32).to_le_bytes());
    let mut new = writer.into_inner().map_err(|e| e.into_error())?;
    new.seek(SeekFrom::Start(0))?;
    new.write_all(&head[..8])?;
    new.sync_all()?;
    Ok(Packed {
        removed: dbf.records - left,
        left,
    })
}

// ── export ─────────────────────────────────────────────────────────────────────────────────

/// What an export wrote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exported {
    pub rows: usize,
    /// The memo file written beside an exported table, when the table has one.
    pub memo: Option<PathBuf>,
}

/// A new file that must not be there yet. The name being taken is an answer of its own.
fn create_new(target: &Path) -> Result<File, WriteError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)
        .map_err(|e| match e.kind() {
            io::ErrorKind::AlreadyExists => WriteError::Exists(target.to_path_buf()),
            _ => WriteError::Io(e),
        })
}

/// A table's header and one record at a time, for the two exports to read.
fn open_to_read(table: &Path, shown: &Dbf) -> Result<(File, Dbf, Vec<u8>), WriteError> {
    let mut file = File::open(table).map_err(WriteError::Open)?;
    let mut head = vec![0u8; shown.first as usize];
    file.read_exact(&mut head)
        .map_err(|_| WriteError::NotATable)?;
    let dbf = Dbf::parse(&head).ok_or(WriteError::NotATable)?;
    if dbf.first != shown.first || dbf.record_len != shown.record_len || dbf.fields != shown.fields
    {
        return Err(WriteError::Changed);
    }
    Ok((file, dbf, head))
}

/// One cell as a CSV field (RFC 4180): quoted when it holds the comma, a quote or a line
/// break, with the quotes inside it doubled.
fn csv_field(cell: &str) -> String {
    if cell.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

/// Write `records` of the table, in the order given, as comma-separated text in UTF-8: the
/// field names first, then a line a record, each cell as table mode shows it — a date as
/// `1975-03-12`, a memo as its text on one line — with the deletion mark in the first
/// column, where table mode has it. `memo` is the table's memo file, where it has one.
///
/// Read through `shown`, so in the code page the reader was told the table is in. The file
/// is new or the export is refused, and a write that fails takes its half away.
pub fn export_csv(
    table: &Path,
    memo: Option<&Path>,
    shown: &Dbf,
    records: &[usize],
    target: &Path,
) -> Result<Exported, WriteError> {
    let (mut file, dbf, _) = open_to_read(table, shown)?;
    if records.iter().any(|record| *record >= dbf.records) {
        return Err(WriteError::NoRecord);
    }
    let mut memo = memo.and_then(|path| crate::source::FileSource::open(path).ok());
    let new = create_new(target)?;
    let written = (|| -> Result<(), WriteError> {
        let mut out = BufWriter::new(new);
        let line = |cells: Vec<String>| -> String {
            let fields: Vec<String> = cells.iter().map(|cell| csv_field(cell)).collect();
            format!("{}\r\n", fields.join(","))
        };
        out.write_all(line(shown.columns()).as_bytes())?;
        let mut record = vec![0u8; dbf.record_len];
        for &n in records {
            file.seek(SeekFrom::Start(dbf.offset(n)))?;
            file.read_exact(&mut record)
                .map_err(|_| WriteError::Length)?;
            let memo = memo.as_mut().map(|m| m as &mut dyn crate::source::Source);
            out.write_all(line(shown.cells(&record, memo)).as_bytes())?;
        }
        let new = out.into_inner().map_err(|e| e.into_error())?;
        new.sync_all()?;
        Ok(())
    })();
    match written {
        Ok(()) => Ok(Exported {
            rows: records.len(),
            memo: None,
        }),
        Err(e) => {
            let _ = std::fs::remove_file(target);
            Err(e)
        }
    }
}

/// Write `records` of the table, in the order given, as a new table of the same structure:
/// the header as it is, with the new count and `at` as its date, and each record byte for
/// byte — its deletion mark with it.
///
/// A memo field holds the number of a block in the memo file, and a record copied as it is
/// keeps its numbers; so the memo file is copied whole beside the new table, under the new
/// table's name, and every number still means what it meant. That carries the memos of the
/// records left behind as well, which costs room and loses nothing. `memo` is that file.
///
/// A Visual FoxPro table that belongs to a database says so after its fields, with the path
/// of the container; the copy is a table of its own and that path is blanked in it.
///
/// Both files are new or the export is refused before anything is written, and a write
/// that fails takes its files away.
pub fn export_dbf(
    table: &Path,
    memo: Option<&Path>,
    shown: &Dbf,
    records: &[usize],
    target: &Path,
    at: Stamp,
) -> Result<Exported, WriteError> {
    let (mut file, dbf, mut head) = open_to_read(table, shown)?;
    if records.iter().any(|record| *record >= dbf.records) {
        return Err(WriteError::NoRecord);
    }
    // The memo file keeps its own extension, in the case the target's is written in.
    let memo_target = memo.map(|memo| {
        let ext = memo
            .extension()
            .map(|e| e.to_string_lossy().into_owned())
            .unwrap_or_default();
        let upper = target
            .extension()
            .is_some_and(|e| e.to_string_lossy().chars().all(|c| !c.is_lowercase()));
        target.with_extension(if upper {
            ext.to_uppercase()
        } else {
            ext.to_lowercase()
        })
    });
    if let Some(taken) = memo_target.iter().find(|path| path.exists()) {
        return Err(WriteError::Exists(taken.clone()));
    }
    let new = create_new(target)?;
    let written = (|| -> Result<(), WriteError> {
        head[1..4].copy_from_slice(&at.in_a_header());
        head[4..8].copy_from_slice(&(records.len() as u32).to_le_bytes());
        if matches!(head[0], 0x30..=0x32) {
            // After the terminator of the fields, to the end of the header.
            let fields = dbf.fields.len() + usize::from(dbf.nulls.is_some());
            let from = (32 + fields * 32 + 1).min(head.len());
            head[from..].fill(0);
        }
        let mut out = BufWriter::new(new);
        out.write_all(&head)?;
        let mut record = vec![0u8; dbf.record_len];
        for &n in records {
            file.seek(SeekFrom::Start(dbf.offset(n)))?;
            file.read_exact(&mut record)
                .map_err(|_| WriteError::Length)?;
            out.write_all(&record)?;
        }
        out.write_all(&[END_OF_FILE])?;
        let new = out.into_inner().map_err(|e| e.into_error())?;
        new.sync_all()?;
        if let (Some(from), Some(to)) = (memo, &memo_target) {
            copy_new(from, to).map_err(|e| match e.kind() {
                io::ErrorKind::AlreadyExists => WriteError::Exists(to.clone()),
                _ => WriteError::Io(e),
            })?;
        }
        Ok(())
    })();
    match written {
        Ok(()) => Ok(Exported {
            rows: records.len(),
            memo: memo_target,
        }),
        Err(e) => {
            let _ = std::fs::remove_file(target);
            // A memo file that turned up in the way is somebody else's, and stays.
            if let (Some(memo), false) = (&memo_target, matches!(e, WriteError::Exists(_))) {
                let _ = std::fs::remove_file(memo);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::BytesSource;
    use crate::table::Table;

    /// 25 November 2026, half past eight and fifteen seconds: the moment in the author's
    /// own example of a backup's name.
    const THEN: Stamp = Stamp {
        year: 2026,
        month: 11,
        day: 25,
        hour: 8,
        minute: 30,
        second: 15,
    };

    /// A dBase III table with one field of each kind this program writes, and a memo:
    /// `NAME C(10)`, `QTY N(7,2)`, `RATE F(6,3)`, `BORN D`, `OK L`, `NOTE M`. Three records,
    /// the second deleted. `language` is the header's code page byte.
    fn table_bytes(language: u8) -> Vec<u8> {
        let fields: [(&str, u8, u8, u8); 6] = [
            ("NAME", b'C', 10, 0),
            ("QTY", b'N', 7, 2),
            ("RATE", b'F', 6, 3),
            ("BORN", b'D', 8, 0),
            ("OK", b'L', 1, 0),
            ("NOTE", b'M', 10, 0),
        ];
        let header_len = 32 + fields.len() * 32 + 1;
        let record_len = 1 + fields.iter().map(|f| f.2 as usize).sum::<usize>();
        let mut out = vec![0x83]; // dBase III with a memo
        out.extend_from_slice(&[99, 1, 15]); // last written 1999-01-15
        out.extend_from_slice(&3u32.to_le_bytes());
        out.extend_from_slice(&(header_len as u16).to_le_bytes());
        out.extend_from_slice(&(record_len as u16).to_le_bytes());
        let mut rest = [0u8; 20];
        rest[29 - 12] = language;
        out.extend_from_slice(&rest);
        for (name, kind, len, decimals) in fields {
            let mut raw = [0u8; 32];
            raw[..name.len()].copy_from_slice(name.as_bytes());
            raw[11] = kind;
            raw[16] = len;
            raw[17] = decimals;
            out.extend_from_slice(&raw);
        }
        out.push(0x0d);
        for (flag, record) in [
            (b' ', "mb           1.50 0.25019750312T         1"),
            (b'*', "gone                                      "),
            (b' ', "ada        -12.00      18151210F          "),
        ] {
            assert_eq!(record.len() + 1, record_len);
            out.push(flag);
            out.extend_from_slice(record.as_bytes());
        }
        out.push(END_OF_FILE);
        out
    }

    /// The table on disk, in a folder of its own, and its header as the reader has it.
    fn on_disk(name: &str, language: u8) -> (tempfile::TempDir, PathBuf, Dbf) {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join(name);
        let bytes = table_bytes(language);
        std::fs::write(&path, &bytes).expect("written");
        let dbf = Dbf::parse(&bytes).expect("a table");
        (dir, path, dbf)
    }

    /// What table mode shows of a record, read back from the file.
    fn shown(path: &Path, record: usize) -> Vec<String> {
        let bytes = std::fs::read(path).expect("read");
        let table = Table::dbase(&bytes, bytes.len() as u64).expect("a table");
        table.cells(&mut BytesSource::new(bytes), None, record)
    }

    /// The header's record count and last-update bytes.
    fn header(path: &Path) -> (u32, [u8; 3]) {
        let bytes = std::fs::read(path).expect("read");
        (
            u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
            [bytes[1], bytes[2], bytes[3]],
        )
    }

    #[test]
    fn a_cell_of_each_type_is_written_as_the_field_holds_it() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();

        set_cell(&path, &dbf, 0, 0, "Matteo", THEN).expect("a character field");
        set_cell(&path, &dbf, 0, 1, "42.5", THEN).expect("a number");
        set_cell(&path, &dbf, 0, 2, "-1.125", THEN).expect("a float");
        set_cell(&path, &dbf, 0, 3, "2026-11-25", THEN).expect("a date");
        set_cell(&path, &dbf, 0, 4, "false", THEN).expect("a logical");
        assert_eq!(
            shown(&path, 0),
            [
                "",
                "Matteo",
                "42.50",
                "-1.125",
                "2026-11-25",
                "false",
                "memo:1"
            ]
        );

        // On disk: text to the left and padded, numbers to the right with the field's own
        // decimals, a date as eight digits, a logical as one letter.
        let after = std::fs::read(&path).unwrap();
        let at = dbf.offset(0) as usize;
        assert_eq!(
            &after[at..at + dbf.record_len],
            b" Matteo      42.50-1.12520261125F         1"
        );
        // And nothing else moved: the other records and the length are what they were.
        let next = dbf.offset(1) as usize;
        assert_eq!(after[next..], before[next..]);
        assert_eq!(after.len(), before.len());
    }

    #[test]
    fn a_value_that_does_not_fit_the_field_is_refused_and_nothing_is_written() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        let refused = |field: usize, text: &str| match set_cell(&path, &dbf, 0, field, text, THEN) {
            Err(WriteError::Cell(why)) => why,
            other => panic!("{text:?} into field {field}: {other:?}"),
        };
        // Eleven letters do not go into ten, and are not cut to ten.
        assert_eq!(
            refused(0, "abcdefghijk"),
            Refused::TooLong {
                bytes: 11,
                room: 10
            }
        );
        assert_eq!(refused(1, "twelve"), Refused::Number);
        assert_eq!(refused(1, "1,5"), Refused::Number);
        assert_eq!(refused(1, "."), Refused::Number);
        let fit = Refused::NumberFit {
            width: 7,
            decimals: 2,
        };
        // A third decimal would have to be rounded away; five whole digits leave no room
        // for the point and the two decimals.
        assert_eq!(refused(1, "1.505"), fit);
        assert_eq!(refused(1, "12345"), fit);
        assert_eq!(refused(1, "-1234.5"), fit);
        assert_eq!(refused(3, "2026-02-30"), Refused::Date);
        assert_eq!(refused(3, "25/11/2026"), Refused::Date);
        assert_eq!(refused(3, "2026-13-01"), Refused::Date);
        assert_eq!(refused(4, "maybe"), Refused::Logical);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn a_number_is_laid_out_by_the_fields_width_and_decimals() {
        assert_eq!(number("7", 7, 2), Ok("7.00".into()));
        assert_eq!(number("+7.5", 7, 2), Ok("7.50".into()));
        assert_eq!(number("007.50000", 7, 2), Ok("7.50".into()));
        assert_eq!(number(".5", 7, 2), Ok("0.50".into()));
        assert_eq!(number("-0.00", 7, 2), Ok("0.00".into()));
        assert_eq!(number("-9999.99", 8, 2), Ok("-9999.99".into()));
        assert_eq!(number("12.0", 4, 0), Ok("12".into()));
        assert_eq!(number("", 4, 0), Ok(String::new()));
        // A leap day is a day; the same day a year later is not.
        assert_eq!(date("2024-02-29"), Some("20240229".into()));
        assert_eq!(date("20240229"), Some("20240229".into()));
        assert_eq!(date("2025-02-29"), None);
        assert_eq!(date("1900-02-29"), None);
    }

    #[test]
    fn an_empty_value_is_the_fields_own_empty() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        for field in 0..5 {
            set_cell(&path, &dbf, 0, field, "", THEN).expect("emptied");
        }
        assert_eq!(shown(&path, 0), ["", "", "", "", "", "", "memo:1"]);
        let bytes = std::fs::read(&path).unwrap();
        let at = dbf.offset(0) as usize;
        assert_eq!(&bytes[at..at + 33], [b' '; 33]);
    }

    #[test]
    fn a_memo_field_is_refused_as_not_written_yet() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        assert_eq!(encode(&dbf, 5, "x"), Err(Refused::ReadOnly { kind: 'M' }));
        assert!(matches!(
            set_cell(&path, &dbf, 0, 5, "a note", THEN),
            Err(WriteError::Cell(Refused::ReadOnly { kind: 'M' }))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    #[test]
    fn text_goes_in_the_tables_own_code_page_or_not_at_all() {
        // Code page 437, as a DOS program wrote it: `ä` is the byte 0x84.
        let (_dir, path, dbf) = on_disk("dos.dbf", 0x01);
        set_cell(&path, &dbf, 0, 0, "Käse", THEN).expect("437 has an ä");
        let bytes = std::fs::read(&path).unwrap();
        let at = dbf.offset(0) as usize + 1;
        assert_eq!(&bytes[at..at + 10], b"K\x84se      ");
        assert_eq!(shown(&path, 0)[1], "Käse");
        // The euro sign came after 437 did. It is refused by name, never written as `?`.
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(
            set_cell(&path, &dbf, 0, 0, "5 €", THEN),
            Err(WriteError::Cell(Refused::NoByte {
                ch: '€',
                exact: true
            }))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);

        // Windows-1252 has both, and the euro is one byte there.
        let (_dir, path, dbf) = on_disk("win.dbf", 0x03);
        set_cell(&path, &dbf, 0, 0, "5 €", THEN).expect("1252 has a euro");
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[at..at + 3], b"5 \x80");
        assert!(matches!(
            encode(&dbf, 0, "Ж"),
            Err(Refused::NoByte {
                ch: 'Ж',
                exact: true
            })
        ));
    }

    #[test]
    fn a_table_that_does_not_name_its_code_page_takes_ascii_alone() {
        // A Clipper table: the language byte is zero, and the text is in whatever page the
        // machine that wrote it was set to. Guessing would put an `è` on the wrong byte.
        let (_dir, path, dbf) = on_disk("CLIENTI.DBF", 0x00);
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(
            set_cell(&path, &dbf, 0, 0, "caffè", THEN),
            Err(WriteError::Cell(Refused::NoByte {
                ch: 'è',
                exact: false
            }))
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        set_cell(&path, &dbf, 0, 0, "caffe'", THEN).expect("ASCII is the same everywhere");
        assert_eq!(shown(&path, 0)[1], "caffe'");
        // Code page 860 is read through 1252, which is near enough to read and not to write.
        let (_dir, _path, dbf) = on_disk("porto.dbf", 0x24);
        assert!(matches!(
            encode(&dbf, 0, "è"),
            Err(Refused::NoByte { exact: false, .. })
        ));
    }

    #[test]
    fn a_table_told_its_code_page_is_written_in_it_exactly() {
        // The same Clipper table, once somebody has said it is in 850: `è` is the byte 8A
        // there, and that is the byte written.
        let (_dir, path, mut dbf) = on_disk("CLIENTI.DBF", 0x00);
        assert!(dbf.set_page(Some(crate::dbf::Page::Cp850)));
        set_cell(&path, &dbf, 0, 0, "caffè", THEN).expect("850 has an è");
        let bytes = std::fs::read(&path).unwrap();
        let at = dbf.offset(0) as usize + 1;
        assert_eq!(&bytes[at..at + 10], b"caff\x8a     ");
        // The header still says nothing: the choice is this program's, not the table's.
        assert_eq!(bytes[29], 0);
        // What 850 has not got is refused as a page's own refusal, by name.
        assert!(matches!(
            set_cell(&path, &dbf, 0, 0, "5 €", THEN),
            Err(WriteError::Cell(Refused::NoByte {
                ch: '€',
                exact: true
            }))
        ));
        // And a table whose header does name 850 is written in full with nobody asked.
        let (_dir, path, dbf) = on_disk("latin.dbf", 0x02);
        set_cell(&path, &dbf, 0, 0, "Ávila", THEN).expect("850 has an Á");
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[at..at + 10], b"\xb5vila     ");
        assert_eq!(shown(&path, 0)[1], "Ávila");
    }

    #[test]
    fn every_write_updates_the_headers_date_and_keeps_the_count() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        assert_eq!(header(&path), (3, [99, 1, 15]));
        set_cell(&path, &dbf, 2, 0, "Ada", THEN).unwrap();
        // 2026 is 126 years after 1900, which is how the format counts.
        assert_eq!(header(&path), (3, [126, 11, 25]));
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.last(), Some(&END_OF_FILE));
        assert_eq!(bytes.len() as u64, dbf.offset(3) + 1);
    }

    #[test]
    fn a_table_without_its_end_of_file_byte_gets_one() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let mut bytes = std::fs::read(&path).unwrap();
        assert_eq!(bytes.pop(), Some(END_OF_FILE));
        std::fs::write(&path, &bytes).unwrap();
        set_deleted(&path, &dbf, 0, true, THEN).unwrap();
        let after = std::fs::read(&path).unwrap();
        assert_eq!(after.len(), bytes.len() + 1);
        assert_eq!(after.last(), Some(&END_OF_FILE));
    }

    #[test]
    fn a_record_is_deleted_and_recalled_by_its_flag_alone() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        set_deleted(&path, &dbf, 0, true, THEN).unwrap();
        assert_eq!(shown(&path, 0)[0], "*");
        assert_eq!(shown(&path, 0)[1], "mb", "the record is still there");
        let after = std::fs::read(&path).unwrap();
        let at = dbf.offset(0) as usize;
        assert_eq!(after[at], b'*');
        assert_eq!(after[at + 1..], before[at + 1..]);
        assert_eq!(count_deleted(&path, &dbf).unwrap(), 2);

        set_deleted(&path, &dbf, 0, false, THEN).unwrap();
        set_deleted(&path, &dbf, 1, false, THEN).unwrap();
        assert_eq!(shown(&path, 1), ["", "gone", "", "", "", "", ""]);
        assert_eq!(count_deleted(&path, &dbf).unwrap(), 0);
        assert_eq!(header(&path).0, 3, "deleting and recalling moves no record");

        assert!(matches!(
            set_deleted(&path, &dbf, 3, true, THEN),
            Err(WriteError::NoRecord)
        ));
    }

    #[test]
    fn an_appended_record_is_empty_and_counted() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        assert_eq!(append(&path, &dbf, THEN).unwrap(), 3);
        let after = std::fs::read(&path).unwrap();
        assert_eq!(header(&path), (4, [126, 11, 25]));
        assert_eq!(after.len(), before.len() + dbf.record_len);
        assert_eq!(after.last(), Some(&END_OF_FILE));
        // The records that were there are the bytes they were.
        let end = dbf.offset(3) as usize;
        assert_eq!(after[8..end], before[8..end]);
        assert_eq!(after[end..end + dbf.record_len], vec![b' '; dbf.record_len]);
        assert_eq!(shown(&path, 3), ["", "", "", "", "", "", ""]);

        // The header the reader has is a record behind now, and a second append still
        // lands after the first: the count is read from the file each time.
        assert_eq!(append(&path, &dbf, THEN).unwrap(), 4);
        set_cell(&path, &dbf, 4, 0, "new", THEN).expect("and it is edited");
        assert_eq!(shown(&path, 4)[1], "new");
        assert_eq!(header(&path).0, 5);
    }

    #[test]
    fn a_file_longer_than_its_header_counts_is_not_appended_to() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes.extend_from_slice(b"something the header does not count");
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(append(&path, &dbf, THEN), Err(WriteError::Length)));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        // And one shorter than its header claims is not written at all.
        std::fs::write(&path, &bytes[..dbf.offset(2) as usize]).unwrap();
        assert!(matches!(
            set_cell(&path, &dbf, 0, 0, "x", THEN),
            Err(WriteError::Length)
        ));
    }

    #[test]
    fn pack_takes_the_deleted_records_out_through_a_new_file() {
        let (dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        assert_eq!(count_deleted(&path, &dbf).unwrap(), 1);
        assert_eq!(
            pack(&path, &dbf, THEN).unwrap(),
            Packed {
                removed: 1,
                left: 2
            }
        );
        assert_eq!(header(&path), (2, [126, 11, 25]));
        let after = std::fs::read(&path).unwrap();
        assert_eq!(after.len(), before.len() - dbf.record_len);
        assert_eq!(after.last(), Some(&END_OF_FILE));
        // The field descriptors are the same bytes, and the records that stayed are whole.
        let first = dbf.first as usize;
        assert_eq!(after[8..first], before[8..first]);
        assert_eq!(shown(&path, 0)[1], "mb");
        assert_eq!(shown(&path, 1)[1], "ada");
        // Nothing is left beside the table.
        let names: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .collect();
        assert_eq!(names, ["stock.dbf"]);

        // A table with nothing deleted packs to itself.
        let again = Dbf::parse(&after).unwrap();
        assert_eq!(
            pack(&path, &again, THEN).unwrap(),
            Packed {
                removed: 0,
                left: 2
            }
        );
        assert_eq!(std::fs::read(&path).unwrap(), after);
    }

    #[test]
    fn a_pack_that_cannot_finish_leaves_the_table_as_it_was() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let before = std::fs::read(&path).unwrap();
        // The file a pack that was cut short left behind: not ours to overwrite.
        let leftover = pack_file(&path);
        std::fs::write(&leftover, b"half a table").unwrap();
        assert!(matches!(
            pack(&path, &dbf, THEN),
            Err(WriteError::Leftover(file)) if file == leftover
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert_eq!(std::fs::read(&leftover).unwrap(), b"half a table");
        std::fs::remove_file(&leftover).unwrap();

        // A table cut short under its header: the new file is taken away again.
        let mut short = before.clone();
        short[4] = 9; // nine records claimed, three there
        std::fs::write(&path, &short).unwrap();
        assert!(matches!(pack(&path, &dbf, THEN), Err(WriteError::Length)));
        assert!(!leftover.exists());
        assert_eq!(std::fs::read(&path).unwrap(), short);
    }

    #[test]
    fn an_index_is_the_tables_when_its_name_begins_with_the_tables() {
        let (dir, path, _dbf) = on_disk("PIPPO.DBF", 0x03);
        let at = |name: &str| dir.path().join(name);
        // Other tables' indexes, and files that are no index at all.
        for name in [
            "FORNIT.CDX",
            "PIPPO.TXT",
            "xpippo.ntx",
            "PIPP.NTX",
            "ordini.idx",
        ] {
            std::fs::write(at(name), b"x").unwrap();
        }
        assert!(index_files(&path).is_empty());
        check_indexes(&path, &[]).expect("no index of its own");

        // Its own: the same name, and the name with something after it, in either case.
        let own = [
            "PIPPO.CDX",
            "PIPPO_NOME.NTX",
            "pippo.mdx",
            "pippo1.ntx",
            "pippo2.NSX",
            "pippox.idx",
        ];
        for name in own {
            std::fs::write(at(name), b"x").unwrap();
        }
        let mut expected: Vec<PathBuf> = own.iter().map(|name| at(name)).collect();
        expected.sort();
        assert_eq!(index_files(&path), expected);

        // A copy a backup made of an index is not an index of the table.
        std::fs::write(at("PIPPO.20261125-083015.CDX"), b"x").unwrap();
        std::fs::write(at("pippo1.20261125-083015.ntx"), b"x").unwrap();
        assert_eq!(index_files(&path), expected);
        // But a name that only looks a little like one is.
        std::fs::write(at("pippo.2026.ntx"), b"x").unwrap();
        assert!(index_files(&path).contains(&at("pippo.2026.ntx")));
    }

    #[test]
    fn a_write_goes_on_only_past_the_indexes_somebody_agreed_to() {
        let (dir, path, dbf) = on_disk("CLIENTI.DBF", 0x03);
        let cdx = dir.path().join("clienti.cdx");
        std::fs::write(&cdx, b"the index").unwrap();
        // Nobody was asked: all of the indexes come back, for the question.
        match check_indexes(&path, &[]) {
            Err(WriteError::Indexes(files)) => assert_eq!(files, std::slice::from_ref(&cdx)),
            other => panic!("{other:?}"),
        }
        // Agreed to: the write is this program's to make, and the index is not touched.
        check_indexes(&path, std::slice::from_ref(&cdx)).expect("agreed");
        set_cell(&path, &dbf, 0, 0, "x", THEN).expect("the write itself does not look");
        assert_eq!(std::fs::read(&cdx).unwrap(), b"the index");
        // One that turns up afterwards was not agreed to, and both are named.
        let ntx = dir.path().join("CLIENTI2.NTX");
        std::fs::write(&ntx, b"x").unwrap();
        match check_indexes(&path, std::slice::from_ref(&cdx)) {
            Err(WriteError::Indexes(files)) => assert_eq!(files, [ntx.clone(), cdx.clone()]),
            other => panic!("{other:?}"),
        }
        // An index that went away is nothing to ask about.
        std::fs::remove_file(&ntx).unwrap();
        check_indexes(&path, &[cdx.clone(), ntx]).expect("fewer is fine");
    }

    #[test]
    fn the_marked_records_are_deleted_or_recalled_in_one_write() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        // Three records, the second already deleted: two change.
        assert_eq!(
            set_deleted_many(&path, &dbf, &[0, 1, 2], true, THEN).unwrap(),
            2
        );
        for record in 0..3 {
            assert_eq!(shown(&path, record)[0], "*");
        }
        assert_eq!(header(&path), (3, [126, 11, 25]));
        assert_eq!(
            set_deleted_many(&path, &dbf, &[2, 0], false, THEN).unwrap(),
            2
        );
        assert_eq!(shown(&path, 0)[0], "");
        assert_eq!(shown(&path, 1)[0], "*");
        assert_eq!(shown(&path, 2)[0], "");
        // Nothing but the flags moved.
        let bytes = std::fs::read(&path).unwrap();
        let original = table_bytes(0x03);
        assert_eq!(bytes[8..], original[8..]);
        // A record that is not there stops all of it, before the first flag.
        let before = std::fs::read(&path).unwrap();
        assert!(matches!(
            set_deleted_many(&path, &dbf, &[0, 7], true, THEN),
            Err(WriteError::NoRecord)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), before);
        // And none at all is a write of nothing.
        assert_eq!(set_deleted_many(&path, &dbf, &[], true, THEN).unwrap(), 0);
    }

    /// The Visual FoxPro table of the reader's tests, on disk.
    fn fox_on_disk() -> (tempfile::TempDir, PathBuf, Dbf) {
        let dir = tempfile::tempdir().expect("a folder");
        let path = dir.path().join("fox.dbf");
        let bytes = crate::dbf::tests::fox_bytes();
        std::fs::write(&path, &bytes).expect("written");
        let dbf = Dbf::parse(&bytes).expect("a table");
        (dir, path, dbf)
    }

    #[test]
    fn a_value_written_over_a_null_is_a_value_and_a_new_record_has_none_set() {
        let (_dir, path, dbf) = fox_on_disk();
        // Record 1 has no `QTY` and no `NAME`; its `NOTE` is an ordinary field.
        assert_eq!(shown(&path, 1), ["", "NULL", "ABCD", "NULL", ""]);
        set_cell(&path, &dbf, 1, 3, "hi", THEN).expect("a C field");
        assert_eq!(shown(&path, 1), ["", "NULL", "ABCD", "NULL", "hi"]);
        // The fields of varying length and the integers are shown and not written yet.
        for field in 0..3 {
            assert!(matches!(
                set_cell(&path, &dbf, 1, field, "1", THEN),
                Err(WriteError::Cell(Refused::ReadOnly { .. }))
            ));
        }

        // An appended record: the flags are bits and not spaces, the fields of varying
        // length are empty by being zero long, and nothing in it is null.
        assert_eq!(append(&path, &dbf, THEN).unwrap(), 3);
        assert_eq!(shown(&path, 3), ["", "", "", "0", ""]);
        let bytes = std::fs::read(&path).unwrap();
        let flags = dbf.offset(3) as usize + 24;
        assert_eq!(
            bytes[flags], 0b0101,
            "`NAME` and `CODE` are short, nothing is null"
        );
    }

    #[test]
    fn a_null_text_field_written_over_loses_its_null() {
        // `NAME C(4)` that may be null, and is, in the one record.
        let bytes = crate::dbf::tests::build(
            0x30,
            0x03,
            &[("NAME", b'C', 4, 0, 0x02), ("_NullFlags", b'0', 1, 0, 0x05)],
            &[(b' ', b"    \x01".to_vec())],
        );
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("n.dbf");
        std::fs::write(&path, &bytes).unwrap();
        let dbf = Dbf::parse(&bytes).unwrap();
        assert_eq!(shown(&path, 0), ["", "NULL"]);
        set_cell(&path, &dbf, 0, 0, "ada", THEN).unwrap();
        assert_eq!(shown(&path, 0), ["", "ada"]);
    }

    #[test]
    fn the_export_to_csv_is_the_table_as_shown_in_utf_8_with_its_heading() {
        let (dir, path, dbf) = on_disk("dos.dbf", 0x01);
        set_cell(&path, &dbf, 0, 0, "Kä,\"a\"", THEN).unwrap();
        let target = dir.path().join("out.csv");
        // In the order asked for, which is the view's.
        let done = export_csv(&path, None, &dbf, &[2, 0], &target).unwrap();
        assert_eq!(
            done,
            Exported {
                rows: 2,
                memo: None
            }
        );
        let text = std::fs::read_to_string(&target).expect("UTF-8");
        assert_eq!(
            text,
            "*,NAME,QTY,RATE,BORN,OK,NOTE\r\n\
             ,ada,-12.00,,1815-12-10,false,\r\n\
             ,\"Kä,\"\"a\"\"\",1.50,0.250,1975-03-12,true,memo:1\r\n"
        );
        // A file that is there is never written over, and is left as it was.
        assert!(matches!(
            export_csv(&path, None, &dbf, &[1], &target),
            Err(WriteError::Exists(taken)) if taken == target
        ));
        assert_eq!(std::fs::read_to_string(&target).unwrap(), text);
        // A record that is not there writes no file at all.
        let other = dir.path().join("other.csv");
        assert!(matches!(
            export_csv(&path, None, &dbf, &[9], &other),
            Err(WriteError::NoRecord)
        ));
        assert!(!other.exists());
        // The table itself was only read.
        assert_eq!(shown(&path, 1)[0], "*");
    }

    #[test]
    fn the_export_to_a_table_copies_the_records_as_they_are_and_the_memo_file_whole() {
        let (dir, path, dbf) = on_disk("STOCK.DBF", 0x03);
        let memo = dir.path().join("STOCK.DBT");
        std::fs::write(&memo, b"the memos, every block of them").unwrap();
        let original = std::fs::read(&path).unwrap();
        let target = dir.path().join("PART.DBF");
        let done = export_dbf(&path, Some(&memo), &dbf, &[2, 1], &target, THEN).unwrap();
        let memo_copy = dir.path().join("PART.DBT");
        assert_eq!(
            done,
            Exported {
                rows: 2,
                memo: Some(memo_copy.clone())
            }
        );
        // A table of the same structure: two records, dated now, the deleted one still so.
        assert_eq!(header(&target), (2, [126, 11, 25]));
        let new = std::fs::read(&target).unwrap();
        let first = dbf.first as usize;
        assert_eq!(new[8..first], original[8..first]);
        let record = |bytes: &[u8], n: usize| {
            let at = first + n * dbf.record_len;
            bytes[at..at + dbf.record_len].to_vec()
        };
        assert_eq!(record(&new, 0), record(&original, 2));
        assert_eq!(record(&new, 1), record(&original, 1));
        assert_eq!(new.len(), first + 2 * dbf.record_len + 1);
        assert_eq!(new.last(), Some(&END_OF_FILE));
        assert_eq!(shown(&target, 0)[1], "ada");
        assert_eq!(shown(&target, 1)[0], "*");
        // The memo file whole, so the block numbers in the records still mean something.
        assert_eq!(
            std::fs::read(&memo_copy).unwrap(),
            b"the memos, every block of them"
        );
        assert_eq!(std::fs::read(&path).unwrap(), original);

        // Neither file is written over: a memo file in the way stops the table too.
        let again = dir.path().join("AGAIN.DBF");
        std::fs::write(dir.path().join("AGAIN.DBT"), b"somebody's").unwrap();
        assert!(matches!(
            export_dbf(&path, Some(&memo), &dbf, &[0], &again, THEN),
            Err(WriteError::Exists(_))
        ));
        assert!(!again.exists());
        assert_eq!(
            std::fs::read(dir.path().join("AGAIN.DBT")).unwrap(),
            b"somebody's"
        );
        assert!(matches!(
            export_dbf(&path, None, &dbf, &[0], &target, THEN),
            Err(WriteError::Exists(_))
        ));
        // A table with no memo file is one file, and none of them is a table of none.
        let lone = dir.path().join("lone.dbf");
        let done = export_dbf(&path, None, &dbf, &[], &lone, THEN).unwrap();
        assert_eq!(
            done,
            Exported {
                rows: 0,
                memo: None
            }
        );
        assert_eq!(header(&lone).0, 0);
        assert_eq!(std::fs::read(&lone).unwrap().len(), first + 1);
    }

    #[test]
    fn an_exported_visual_foxpro_table_is_a_table_of_its_own() {
        let (dir, path, dbf) = fox_on_disk();
        let target = dir.path().join("free.dbf");
        export_dbf(&path, None, &dbf, &[1, 0], &target, THEN).unwrap();
        let new = std::fs::read(&target).unwrap();
        let original = std::fs::read(&path).unwrap();
        // The fields as they were, and no path of a database after them.
        let fields = 32 + 5 * 32 + 1;
        assert_eq!(new[8..fields], original[8..fields]);
        assert!(original[fields..dbf.first as usize].iter().any(|b| *b != 0));
        assert!(new[fields..dbf.first as usize].iter().all(|b| *b == 0));
        // The nulls and the lengths went with the records.
        assert_eq!(shown(&target, 0), ["", "NULL", "ABCD", "NULL", ""]);
        assert_eq!(shown(&target, 1), ["", "ada", "01020304", "7", "x"]);
    }

    #[test]
    fn a_backup_takes_the_indexes_named_after_the_table_and_the_memo_of_a_container() {
        let dir = tempfile::tempdir().unwrap();
        let at = |name: &str| dir.path().join(name);
        let path = at("shop.dbc");
        std::fs::write(&path, table_bytes(0x03)).unwrap();
        for name in [
            "shop.dct",
            "shop.dcx",
            "shop_by_name.cdx",
            "other.dct",
            "shop.dbt",
        ] {
            std::fs::write(at(name), name.as_bytes()).unwrap();
        }
        let mut made = backup(&path, THEN).unwrap();
        made.sort();
        assert_eq!(
            made,
            [
                at("shop.20261125-083015.dbc"),
                at("shop.20261125-083015.dct"),
                at("shop.20261125-083015.dcx"),
                at("shop_by_name.20261125-083015.cdx"),
            ]
        );
        // The copies are no indexes of the table: a second backup copies as many files.
        let later = Stamp { second: 16, ..THEN };
        assert_eq!(backup(&path, later).unwrap().len(), 4);
    }

    #[test]
    fn a_backup_is_named_by_the_moment_with_the_extension_kept_last() {
        assert_eq!(THEN.in_a_name(), "20261125-083015");
        assert_eq!(Stamp::parse("2026-11-25 08:30:15"), Some(THEN));
        assert_eq!(Stamp::parse("2026-11-25"), None);
        let dir = Path::new("data");
        assert_eq!(
            backup_name(&dir.join("CLIENTI.DBF"), THEN),
            dir.join("CLIENTI.20261125-083015.DBF")
        );
        assert_eq!(
            backup_name(&dir.join("CLIENTI.DBT"), THEN),
            dir.join("CLIENTI.20261125-083015.DBT")
        );
        assert_eq!(
            backup_name(&dir.join("my.old.table.dbf"), THEN),
            dir.join("my.old.table.20261125-083015.dbf")
        );
        assert_eq!(
            backup_name(&dir.join("TABLE"), THEN),
            dir.join("TABLE.20261125-083015")
        );
    }

    #[test]
    fn a_backup_copies_the_table_with_its_memo_and_its_own_indexes() {
        let (dir, path, _dbf) = on_disk("CLIENTI.DBF", 0x03);
        let at = |name: &str| dir.path().join(name);
        std::fs::write(at("CLIENTI.DBT"), b"the memos").unwrap();
        std::fs::write(at("clienti.cdx"), b"the index").unwrap();
        std::fs::write(at("FORNIT.DBT"), b"another table's").unwrap();
        let table = std::fs::read(&path).unwrap();

        let made = backup(&path, THEN).expect("copied");
        assert_eq!(made[0], at("CLIENTI.20261125-083015.DBF"));
        let mut rest = made[1..].to_vec();
        rest.sort();
        assert_eq!(
            rest,
            [
                at("CLIENTI.20261125-083015.DBT"),
                at("clienti.20261125-083015.cdx")
            ]
        );
        assert_eq!(std::fs::read(&made[0]).unwrap(), table);
        assert_eq!(
            std::fs::read(at("CLIENTI.20261125-083015.DBT")).unwrap(),
            b"the memos"
        );
        assert!(!at("FORNIT.20261125-083015.DBT").exists());
        // The originals are where they were, as they were.
        assert_eq!(std::fs::read(&path).unwrap(), table);
        assert_eq!(std::fs::read(at("CLIENTI.DBT")).unwrap(), b"the memos");

        // A second backup in the same second would overwrite the first, so it is refused —
        // and the copy that is there is not harmed by the refusal.
        let again = backup(&path, THEN).expect_err("the name is taken");
        assert_eq!(again.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(std::fs::read(&made[0]).unwrap(), table);
        // A second later it is another backup.
        let later = Stamp { second: 16, ..THEN };
        assert_eq!(backup(&path, later).unwrap().len(), 3);
    }

    #[test]
    fn a_backup_that_fails_half_way_leaves_no_half_of_itself() {
        let (dir, path, _dbf) = on_disk("CLIENTI.DBF", 0x03);
        std::fs::write(dir.path().join("CLIENTI.DBT"), b"the memos").unwrap();
        // The memo's copy cannot be made: its name is taken.
        let taken = dir.path().join("CLIENTI.20261125-083015.DBT");
        std::fs::write(&taken, b"somebody's").unwrap();
        assert!(backup(&path, THEN).is_err());
        assert!(!dir.path().join("CLIENTI.20261125-083015.DBF").exists());
        assert_eq!(std::fs::read(&taken).unwrap(), b"somebody's");
    }

    #[test]
    fn a_table_that_cannot_be_opened_for_writing_says_so() {
        // A folder is the one thing no platform opens for writing, whoever runs the test.
        let dir = tempfile::tempdir().unwrap();
        let (_other, _path, dbf) = on_disk("stock.dbf", 0x03);
        let not_a_file = dir.path().join("locked.dbf");
        std::fs::create_dir(&not_a_file).unwrap();
        assert!(matches!(
            set_cell(&not_a_file, &dbf, 0, 0, "x", THEN),
            Err(WriteError::Open(_))
        ));
        assert!(matches!(
            append(&dir.path().join("missing.dbf"), &dbf, THEN),
            Err(WriteError::Open(_))
        ));
    }

    #[test]
    fn a_table_restructured_by_another_program_is_not_written_by_the_old_layout() {
        let (_dir, path, dbf) = on_disk("stock.dbf", 0x03);
        let mut bytes = std::fs::read(&path).unwrap();
        // `NAME C(10)` became `NAME C(9)` and `QTY` grew by one: the same record length,
        // and every offset after the first field different.
        bytes[32 + 16] = 9;
        bytes[64 + 16] = 8;
        std::fs::write(&path, &bytes).unwrap();
        assert!(matches!(
            set_cell(&path, &dbf, 0, 1, "1", THEN),
            Err(WriteError::Changed)
        ));
        assert!(matches!(pack(&path, &dbf, THEN), Err(WriteError::Changed)));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);

        // And a file that stopped being a table is not one.
        std::fs::write(&path, b"not a table any more, just some text in its place").unwrap();
        assert!(matches!(
            set_deleted(&path, &dbf, 0, true, THEN),
            Err(WriteError::NotATable)
        ));
    }

    #[test]
    fn a_table_that_numbers_its_own_records_is_not_appended_to() {
        let (_dir, path, _dbf) = on_disk("fox.dbf", 0x03);
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] = 0x31; // Visual FoxPro with an autoincrement field
        bytes[64 + 18] = 0x0c;
        std::fs::write(&path, &bytes).unwrap();
        let dbf = Dbf::parse(&bytes).unwrap();
        assert!(matches!(
            append(&path, &dbf, THEN),
            Err(WriteError::Numbered)
        ));
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}
