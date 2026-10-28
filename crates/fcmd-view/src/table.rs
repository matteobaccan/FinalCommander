//! Table mode: the tabular formats read as a table (SPEC 05 §1.11).
//!
//! A commander opens a `.csv` and a `.jsonl` every day, and both read in a text viewer as what
//! they are not — a wall of commas, a wall of braces. What they want is not two viewers but
//! one mode: the screen is the same in either case (a header, columns, a row under the
//! cursor) and all that differs is how a record is read out of the bytes. So the parsing ends
//! here, in one type, and the viewer above knows only about rows and columns.
//!
//! What this is not, and deliberately: a query tool and a spreadsheet. No editing, no sorting,
//! no filtering — each of those wants an answer about writing the file back, which a viewer
//! does not have.

use unicode_width::UnicodeWidthStr;

/// The widest a column is allowed to be. A single 4 kB cell must not push every other column
/// off the screen; what it holds is read with the horizontal scroll, as a long line is.
const MAX_COL: usize = 40;

/// How many records decide the columns of a JSONL file. The spec's *N*: enough to catch the
/// optional keys of a well-formed log, few enough that opening is instant.
const HEADER_RECORDS: usize = 200;

/// Where a table's records come from.
///
/// The text formats are read whole and kept — a column's width cannot be known from the part
/// of the file on screen, which is the same reason a laid-out Markdown document is rendered
/// whole. A DBF is the opposite and says so in its first bytes: the header gives the column
/// widths and the record length, so row *n* is one multiplication and one read, and nothing
/// but the window on screen is ever touched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Records {
    /// Parsed up front: CSV, TSV, JSON Lines.
    Read(Vec<Row>),
    /// Read a window at a time out of the file.
    Dbf(Dbf),
}

/// Which side of its column a cell sits on. A number reads down its last digit, which is why
/// every table tool right-aligns one and why the panel's size column is right-aligned too.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    Left,
    Right,
}

/// A parsed table: what the columns are called, and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<String>,
    pub records: Records,
    /// Display width of each column, from the header and the rows that were read.
    pub widths: Vec<usize>,
    /// Which side of its column each cell sits on. A text format has no types to go by, so
    /// everything is left; a dBase table knows which of its fields are numbers.
    pub aligns: Vec<Align>,
    /// `CSV`, `TSV`, `JSONL` — what the status line says this was read as.
    pub format: &'static str,
    /// The delimiter that was *guessed*, for the status line to admit. `None` for a format
    /// that has no delimiter to guess.
    pub delimiter: Option<char>,
    /// The file was longer than the mode reads whole, so the rows are the start of it.
    pub cut: bool,
    /// Lines that were not records: a JSONL line that is not an object, a CSV row with more
    /// fields than the header. Counted rather than announced one at a time.
    pub skipped: usize,
}

/// One record, and where in the file it began — which is what lets a search or a bookmark in
/// table mode mean the same thing it means in every other mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub offset: u64,
    pub cells: Vec<String>,
}

impl Table {
    /// A `.dbf`, from its header. The records are not read: the header says where they are.
    pub fn dbase(head: &[u8], file_len: u64) -> Option<Table> {
        let mut dbf = Dbf::parse(head)?;
        // A header that claims more records than the file can hold is a truncated table, and
        // what is there is what can be read.
        let room = file_len.saturating_sub(dbf.first) / dbf.record_len.max(1) as u64;
        let claimed = dbf.records;
        dbf.records = dbf.records.min(room as usize);
        let cut = dbf.records < claimed;
        let mut table = Table {
            columns: dbf.columns(),
            records: Records::Dbf(dbf.clone()),
            widths: dbf.widths(),
            aligns: dbf.aligns(),
            format: dbf.flavour,
            delimiter: None,
            cut,
            skipped: 0,
        };
        // The declared widths, with the headings allowed to widen a column.
        for (i, name) in table.columns.iter().enumerate() {
            table.widths[i] = table.widths[i].max(name.width().min(MAX_COL)).max(1);
        }
        Some(table)
    }

    /// Read a table out of text, by what the name says it is. `None` for a name that says
    /// nothing we can read.
    pub fn of(name: &str, text: &str) -> Option<Table> {
        let name = name.to_ascii_lowercase();
        if name.ends_with(".csv") {
            return Some(Table::separated(text, None));
        }
        if name.ends_with(".tsv") || name.ends_with(".tab") {
            return Some(Table::separated(text, Some('\t')));
        }
        if name.ends_with(".jsonl") || name.ends_with(".ndjson") {
            return Some(Table::json_lines(text));
        }
        None
    }

    /// Is this a name table mode can read? Asked before the file is fetched, so it takes the
    /// name alone.
    pub fn reads(name: &str) -> bool {
        let name = name.to_ascii_lowercase();
        [".csv", ".tsv", ".tab", ".jsonl", ".ndjson", ".dbf"]
            .iter()
            .any(|ext| name.ends_with(ext))
    }

    /// Delimiter-separated text, RFC 4180 quoting. `delimiter` is `None` to guess.
    pub fn separated(text: &str, delimiter: Option<char>) -> Table {
        let guessed = delimiter.is_none();
        let delimiter = delimiter.unwrap_or_else(|| sniff(text));
        let mut records = parse_separated(text, delimiter);
        let header = if records.is_empty() {
            Vec::new()
        } else {
            records.remove(0).cells
        };
        let mut table = Table {
            columns: header,
            records: Records::Read(records),
            widths: Vec::new(),
            aligns: Vec::new(),
            format: if delimiter == '\t' { "TSV" } else { "CSV" },
            delimiter: guessed.then_some(delimiter),
            cut: false,
            skipped: 0,
        };
        // A row with more fields than the header has them: a header short of a column is a
        // common export, and dropping the data would be worse than naming the column after
        // the number it is.
        let widest = table
            .rows()
            .iter()
            .map(|r| r.cells.len())
            .max()
            .unwrap_or(0);
        while table.columns.len() < widest {
            table.columns.push(format!("#{}", table.columns.len() + 1));
        }
        table.measure();
        table
    }

    /// One JSON object per line. The columns are the union of the keys of the first
    /// [`HEADER_RECORDS`] records, in the order they were first seen — which is what makes a
    /// log whose later lines carry an extra field read as a table at all.
    pub fn json_lines(text: &str) -> Table {
        let mut columns: Vec<String> = Vec::new();
        let mut values: Vec<(u64, Vec<(String, String)>)> = Vec::new();
        let mut skipped = 0usize;
        let mut at = 0u64;
        for line in text.split_inclusive('\n') {
            let offset = at;
            at += line.len() as u64;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let Ok(serde_json::Value::Object(map)) = serde_json::from_str(trimmed) else {
                // A line that is not an object is not a record. A `[` on its own means
                // somebody has a JSON array rather than JSON Lines, and the count says so.
                skipped += 1;
                continue;
            };
            let mut cells = Vec::with_capacity(map.len());
            for (key, value) in map {
                if values.len() < HEADER_RECORDS && !columns.contains(&key) {
                    columns.push(key.clone());
                }
                cells.push((key, flatten(&value)));
            }
            values.push((offset, cells));
        }
        // A key no column was made for — it turned up after the header was decided — goes in
        // one overflow column rather than being dropped without a word.
        let mut overflow = false;
        let mut rows = Vec::with_capacity(values.len());
        for (offset, cells) in values {
            let mut row = vec![String::new(); columns.len()];
            let mut extra: Vec<String> = Vec::new();
            for (key, value) in cells {
                match columns.iter().position(|c| *c == key) {
                    Some(at) => row[at] = value,
                    None => extra.push(format!("{key}={value}")),
                }
            }
            if !extra.is_empty() {
                overflow = true;
                row.push(extra.join(" "));
            }
            rows.push(Row { offset, cells: row });
        }
        if overflow {
            columns.push("…".to_string());
            for row in &mut rows {
                row.cells.resize(columns.len(), String::new());
            }
        }
        let mut table = Table {
            columns,
            records: Records::Read(rows),
            widths: Vec::new(),
            aligns: Vec::new(),
            format: "JSONL",
            delimiter: None,
            cut: false,
            skipped,
        };
        table.measure();
        table
    }

    /// The width of each column: the widest thing in it, the heading included, capped.
    fn measure(&mut self) {
        self.widths = self
            .columns
            .iter()
            .map(|c| c.width().min(MAX_COL))
            .collect();
        if let Records::Read(rows) = &self.records {
            for row in rows {
                for (i, cell) in row.cells.iter().enumerate() {
                    if i >= self.widths.len() {
                        continue;
                    }
                    self.widths[i] = self.widths[i].max(cell.width()).min(MAX_COL);
                }
            }
        }
        // A column with nothing in it and no name still needs a cell to sit in.
        for w in &mut self.widths {
            *w = (*w).max(1);
        }
        // A text format carries no types, so nothing is claimed about what a column holds.
        if self.aligns.len() != self.columns.len() {
            self.aligns = vec![Align::Left; self.columns.len()];
        }
    }

    /// The heading, as one line, scrolled right by `col` display columns. A heading sits on
    /// the same side as what is under it, so a number column reads as one block.
    pub fn header(&self, col: usize, width: usize) -> String {
        let cells: Vec<&str> = self.columns.iter().map(String::as_str).collect();
        lay_out(&cells, &self.widths, &self.aligns, col, width)
    }

    /// The rule under the heading: the same layout, in box-drawing characters, so the eye
    /// follows a column down the screen.
    pub fn rule(&self, col: usize, width: usize) -> String {
        let mut line = String::new();
        for (i, w) in self.widths.iter().enumerate() {
            if i > 0 {
                line.push_str("─┼─");
            }
            line.push_str(&"─".repeat(*w));
        }
        shift(&line, col, width)
    }

    /// The records that were read up front. Empty for a table that is read a window at a
    /// time, which is why everything that draws goes through `line` instead.
    pub fn rows(&self) -> &[Row] {
        match &self.records {
            Records::Read(rows) => rows,
            Records::Dbf(_) => &[],
        }
    }

    /// How many columns the file itself has: for a DBF, the fields, without the deletion
    /// flag this view puts in front of them — a four-field table says four.
    pub fn fields(&self) -> usize {
        match self.records {
            Records::Dbf(_) => self.columns.len().saturating_sub(1),
            Records::Read(_) => self.columns.len(),
        }
    }

    /// How many records the table has.
    pub fn len(&self) -> usize {
        match &self.records {
            Records::Read(rows) => rows.len(),
            Records::Dbf(dbf) => dbf.records,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Where record `n` begins in the file — what a search or a bookmark in table mode means.
    pub fn offset(&self, n: usize) -> u64 {
        match &self.records {
            Records::Read(rows) => rows.get(n).map(|r| r.offset).unwrap_or(0),
            Records::Dbf(dbf) => dbf.offset(n),
        }
    }

    /// Which record a byte offset falls in: the last one that starts at or before it.
    pub fn row_at(&self, offset: u64) -> usize {
        match &self.records {
            Records::Read(rows) => rows
                .iter()
                .rposition(|r| r.offset <= offset)
                .unwrap_or_default(),
            Records::Dbf(dbf) => {
                let past = offset.saturating_sub(dbf.first) / dbf.record_len.max(1) as u64;
                (past as usize).min(dbf.records.saturating_sub(1))
            }
        }
    }

    /// The cells of one record. The source is only touched for a table that is read a window
    /// at a time; for the text formats it is already in memory and is not read again.
    pub fn cells(&self, source: &mut dyn crate::source::Source, row: usize) -> Vec<String> {
        match &self.records {
            Records::Read(rows) => rows.get(row).map(|r| r.cells.clone()).unwrap_or_default(),
            Records::Dbf(dbf) => {
                if row >= dbf.records {
                    return Vec::new();
                }
                let bytes = source
                    .read_vec(dbf.offset(row), dbf.record_len)
                    .unwrap_or_default();
                if bytes.len() < dbf.record_len {
                    // A table shorter than its header claims: the rest is not there to read.
                    return Vec::new();
                }
                dbf.cells(&bytes)
            }
        }
    }

    /// One row of cells, laid out in the columns.
    pub fn line(
        &self,
        source: &mut dyn crate::source::Source,
        row: usize,
        col: usize,
        width: usize,
    ) -> String {
        let cells = self.cells(source, row);
        if cells.is_empty() {
            return String::new();
        }
        let cells: Vec<&str> = cells.iter().map(String::as_str).collect();
        lay_out(&cells, &self.widths, &self.aligns, col, width)
    }

    /// Where each column begins, in display columns: what a jump sideways lands on, so that
    /// `Ctrl` and an arrow in a table moves by a whole column rather than by a guess.
    pub fn column_starts(&self) -> Vec<usize> {
        let mut out = Vec::with_capacity(self.widths.len());
        let mut at = 0usize;
        for (i, w) in self.widths.iter().enumerate() {
            if i > 0 {
                at += 3; // ` │ `
            }
            out.push(at);
            at += w;
        }
        out
    }

    /// How wide the whole table is, in display columns: what the horizontal scroll may reach.
    pub fn total_width(&self) -> usize {
        let gaps = self.widths.len().saturating_sub(1) * 3;
        self.widths.iter().sum::<usize>() + gaps
    }
}

/// A JSON value as one cell of a table. A string is itself — quoting it again would double
/// the quotes of every cell on the screen — and anything nested is its own compact JSON,
/// which is at least true and readable.
fn flatten(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

/// Lay cells out in columns of the given widths, then take the window the screen shows.
fn lay_out(cells: &[&str], widths: &[usize], aligns: &[Align], col: usize, width: usize) -> String {
    let mut line = String::new();
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            line.push_str(" │ ");
        }
        let cell = cells.get(i).copied().unwrap_or("");
        let cell = truncate(cell, *w);
        let pad = w.saturating_sub(cell.width());
        let right = aligns.get(i).copied().unwrap_or(Align::Left) == Align::Right;
        if right {
            for _ in 0..pad {
                line.push(' ');
            }
        }
        line.push_str(&cell);
        if !right {
            for _ in 0..pad {
                line.push(' ');
            }
        }
    }
    shift(&line, col, width)
}

/// The window of a laid-out line that the screen shows: `col` display columns in, `width`
/// wide. Written in display columns rather than characters because a table of CJK text would
/// otherwise come apart.
fn shift(line: &str, col: usize, width: usize) -> String {
    let mut out = String::new();
    let mut at = 0usize;
    for ch in line.chars() {
        let w = ch.to_string().width();
        if at + w > col && out.width() + w <= width {
            out.push(ch);
        }
        at += w;
        if at >= col + width {
            break;
        }
    }
    out
}

/// Cut a cell to a width, with a `…` where something was cut off.
fn truncate(cell: &str, width: usize) -> String {
    if cell.width() <= width {
        return cell.to_string();
    }
    if width == 0 {
        return String::new();
    }
    let mut out = String::new();
    for ch in cell.chars() {
        let w = ch.to_string().width();
        if out.width() + w > width.saturating_sub(1) {
            break;
        }
        out.push(ch);
    }
    out.push('…');
    out
}

/// Which delimiter this looks separated by.
///
/// The count per line of each candidate, outside quotes, over the first few kilobytes: the
/// winner is the one that gives the same non-zero count on the most lines, because that is
/// what a table *is*. Ties go to the order below, which is the order they are common in.
fn sniff(text: &str) -> char {
    const CANDIDATES: [char; 4] = [',', ';', '\t', '|'];
    let sample: String = text.chars().take(64 * 1024).collect();
    let mut best = (0usize, ',');
    for candidate in CANDIDATES {
        let mut counts: Vec<usize> = Vec::new();
        for record in parse_separated(&sample, candidate).iter().take(100) {
            counts.push(record.cells.len());
        }
        if counts.len() < 2 {
            continue;
        }
        // Lines that agree with the first one, which is the header.
        let fields = counts[0];
        if fields < 2 {
            continue;
        }
        let agree = counts.iter().filter(|c| **c == fields).count();
        if agree > best.0 {
            best = (agree, candidate);
        }
    }
    best.1
}

/// Records out of delimiter-separated text, RFC 4180: a field may be quoted with `"`, a
/// doubled `""` is one quote, and a quoted field may hold the delimiter and a newline — which
/// is why a record is not a line.
fn parse_separated(text: &str, delimiter: char) -> Vec<Row> {
    let mut rows = Vec::new();
    let mut cells: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut start = 0u64;
    let mut at = 0u64;
    let mut chars = text.chars().peekable();
    let mut any = false;
    while let Some(ch) = chars.next() {
        at += ch.len_utf8() as u64;
        any = true;
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    at += 1;
                    cell.push('"');
                } else {
                    quoted = false;
                }
            } else {
                cell.push(ch);
            }
            continue;
        }
        match ch {
            '"' if cell.is_empty() => quoted = true,
            c if c == delimiter => {
                cells.push(std::mem::take(&mut cell));
            }
            '\r' => {}
            '\n' => {
                cells.push(std::mem::take(&mut cell));
                rows.push(Row {
                    offset: start,
                    cells: std::mem::take(&mut cells),
                });
                start = at;
                any = false;
            }
            c => cell.push(c),
        }
    }
    if any || !cell.is_empty() || !cells.is_empty() {
        cells.push(cell);
        rows.push(Row {
            offset: start,
            cells,
        });
    }
    // A trailing newline leaves one empty record, which is not a row of anything.
    if rows
        .last()
        .is_some_and(|r| r.cells.len() == 1 && r.cells[0].is_empty())
    {
        rows.pop();
    }
    rows
}

// ── dBase ──────────────────────────────────────────────────────────────────────────────────

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
    fields: Vec<Field>,
    /// What the header's language byte says the text is in.
    encoding: Charset,
    /// What the version byte says this is, for the status line.
    pub flavour: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Field {
    name: String,
    kind: u8,
    /// Where in the record the field starts, the deletion flag included.
    at: usize,
    len: usize,
    decimals: u8,
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
        })
    }

    /// Where record `n` begins in the file.
    pub fn offset(&self, n: usize) -> u64 {
        self.first + (n as u64) * (self.record_len as u64)
    }

    /// The columns: the deletion flag first, then the fields as the header names them.
    fn columns(&self) -> Vec<String> {
        let mut out = vec!["*".to_string()];
        out.extend(self.fields.iter().map(|f| f.name.clone()));
        out
    }

    /// Which side of its column a cell sits on. A number is read down its last digit, so the
    /// numeric types go to the right and everything else stays where it was written.
    fn aligns(&self) -> Vec<Align> {
        let mut out = vec![Align::Left];
        out.extend(self.fields.iter().map(|f| match f.kind {
            b'N' | b'F' | b'I' | b'Y' | b'B' | b'O' => Align::Right,
            _ => Align::Left,
        }));
        out
    }

    /// How wide each column is: what the header declares, capped. A DBF is the one format
    /// that does not have to be read to be laid out.
    fn widths(&self) -> Vec<usize> {
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
    fn cells(&self, record: &[u8]) -> Vec<String> {
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
            out.push(self.cell(f, raw));
        }
        out
    }

    fn cell(&self, f: &Field, raw: &[u8]) -> String {
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
            // A memo is a block number into the `.dbt`/`.fpt` beside the table, which is a
            // second file we do not open yet: the number is what there is to say.
            b'M' | b'G' | b'P' => {
                let s = text(raw);
                let s = s.trim().trim_start_matches('0');
                if s.is_empty() {
                    String::new()
                } else {
                    format!("memo:{s}")
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

/// Which table a DBF's character fields are in.
///
/// Two kinds, because the one that matters most is the one `encoding_rs` does not carry: a
/// DBF written under DOS is in code page 437, and reading it as Windows-1252 turns every
/// accented letter into a different one — `temporäre` into `tempor„re`. The viewer has had
/// a 437 table since the day it learned to show a DOS text file; this is that table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Charset {
    Cp437,
    Table(&'static encoding_rs::Encoding),
}

impl Charset {
    fn decode(self, bytes: &[u8]) -> String {
        match self {
            Charset::Cp437 => crate::decode::Codepage::Cp437.decode(bytes),
            Charset::Table(table) => table.decode(bytes).0.into_owned(),
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A source with nothing in it: the text formats never read from it, and the tests that
    /// do read — the dBase ones — build their own.
    fn src() -> crate::source::BytesSource {
        crate::source::BytesSource::new(Vec::new())
    }

    #[test]
    fn a_comma_separated_file_reads_as_a_table_with_its_first_line_as_the_header() {
        let table = Table::of("people.csv", "name,age\nmb,51\nada,36\n").expect("a table");
        assert_eq!(table.columns, ["name", "age"]);
        assert_eq!(table.rows().len(), 2);
        assert_eq!(table.rows()[0].cells, ["mb", "51"]);
        assert_eq!(table.format, "CSV");
        // The delimiter was guessed, so the status line has something to admit.
        assert_eq!(table.delimiter, Some(','));
        // Each row knows where it began, so a search can say where it found something.
        assert_eq!(table.rows()[0].offset, "name,age\n".len() as u64);
    }

    #[test]
    fn a_quoted_field_may_hold_the_delimiter_and_a_newline() {
        let text = "a,b\n\"one,two\",\"line\nbreak\"\n\"say \"\"hi\"\"\",last\n";
        let table = Table::separated(text, Some(','));
        assert_eq!(table.rows()[0].cells, ["one,two", "line\nbreak"]);
        assert_eq!(table.rows()[1].cells, ["say \"hi\"", "last"]);
    }

    #[test]
    fn the_delimiter_is_guessed_from_what_gives_a_table() {
        // Semicolons: the European export. The commas inside the numbers must not win.
        let table = Table::separated("name;price\nbread;1,20\nmilk;0,99\n", None);
        assert_eq!(table.delimiter, Some(';'));
        assert_eq!(table.columns, ["name", "price"]);
        assert_eq!(table.rows()[0].cells, ["bread", "1,20"]);

        // Tabs, which a `.tsv` says outright but a `.csv` may still hold.
        let table = Table::separated("a\tb\n1\t2\n", None);
        assert_eq!(table.delimiter, Some('\t'));

        // One column and nothing to separate: no guess is better than a wrong one.
        let table = Table::separated("just one line\n", None);
        assert_eq!(table.columns, ["just one line"]);
        assert!(table.rows().is_empty());
    }

    #[test]
    fn a_row_with_more_fields_than_the_header_keeps_them() {
        let table = Table::separated("a,b\n1,2,3\n", Some(','));
        assert_eq!(table.columns, ["a", "b", "#3"]);
        assert_eq!(table.rows()[0].cells, ["1", "2", "3"]);
    }

    #[test]
    fn json_lines_take_their_columns_from_the_keys_in_the_order_they_appear() {
        let text = "{\"when\":\"now\",\"what\":\"start\"}\n{\"what\":\"stop\",\"code\":2}\n";
        let table = Table::json_lines(text);
        assert_eq!(table.columns, ["when", "what", "code"]);
        assert_eq!(table.rows()[0].cells, ["now", "start", ""]);
        assert_eq!(table.rows()[1].cells, ["", "stop", "2"]);
        assert_eq!(table.format, "JSONL");

        // A line that is not an object is counted, not shown as a row of nonsense.
        let table = Table::json_lines("[\n{\"a\":1}\nnot json\n");
        assert_eq!(table.rows().len(), 1);
        assert_eq!(table.skipped, 2);

        // Nested values are their own compact JSON, and null is an empty cell.
        let table = Table::json_lines("{\"a\":{\"b\":1},\"c\":[1,2],\"d\":null}\n");
        assert_eq!(table.rows()[0].cells, ["{\"b\":1}", "[1,2]", ""]);
    }

    #[test]
    fn a_key_that_turns_up_after_the_header_was_decided_goes_in_the_overflow_column() {
        let mut text = String::new();
        for i in 0..HEADER_RECORDS {
            text.push_str(&format!("{{\"i\":{i}}}\n"));
        }
        text.push_str("{\"i\":999,\"late\":\"yes\"}\n");
        let table = Table::json_lines(&text);
        assert_eq!(table.columns, ["i", "…"]);
        let last = table.rows().last().expect("a row");
        assert_eq!(last.cells, ["999", "late=yes"]);
    }

    #[test]
    fn the_layout_pads_the_columns_and_the_window_scrolls_sideways() {
        let table = Table::of("t.csv", "name,age\nmb,51\n").expect("a table");
        assert_eq!(table.widths, [4, 3]);
        assert_eq!(table.header(0, 40), "name │ age");
        assert_eq!(table.rule(0, 40), "──── │ ───".replace(" │ ", "─┼─"));
        assert_eq!(table.line(&mut src(), 0, 0, 40), "mb   │ 51 ");

        // A window narrower than the table shows what fits, from where it was scrolled to.
        assert_eq!(table.header(0, 4), "name");
        assert_eq!(table.header(7, 3), "age");
        assert_eq!(table.total_width(), 4 + 3 + 3);
    }

    #[test]
    fn a_cell_wider_than_its_column_is_cut_and_says_so() {
        let long = "x".repeat(MAX_COL + 10);
        let table = Table::separated(&format!("a\n{long}\n"), Some(','));
        assert_eq!(table.widths, [MAX_COL]);
        let line = table.line(&mut src(), 0, 0, 100);
        assert!(line.ends_with('…'), "{line}");
        assert_eq!(line.width(), MAX_COL);
    }

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
        assert_eq!(table.cells(&mut source, 0), ["", "mb", "1975-03-12"]);
        // A deleted record is still a record, with a `*` in the first column: a table whose
        // deleted rows vanish is one whose count does not add up.
        assert_eq!(table.cells(&mut source, 1), ["*", "gone", ""]);
        assert_eq!(table.cells(&mut source, 2), ["", "ada", "1815-12-10"]);
        // Row n is one seek, and the offsets say where.
        assert_eq!(table.offset(1), table.offset(0) + 19);
        assert_eq!(table.row_at(table.offset(2) + 3), 2);
        assert_eq!(
            table.line(&mut source, 2, 0, 40),
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
        assert_eq!(table.cells(&mut source, 0), ["", "temporäre", "44.00"]);
        // `PREIS` is eight wide, and `44.00` ends where the column does.
        assert_eq!(
            table.line(&mut source, 0, 0, 40),
            "  │ temporäre    │    44.00"
        );
        // The heading sits over its own column, on the same side as the cells under it.
        assert_eq!(table.header(0, 40), "* │ LEISTUNG     │    PREIS");
    }

    #[test]
    fn only_the_names_table_mode_can_read_are_offered() {
        assert!(Table::reads("a.csv") && Table::reads("A.TSV") && Table::reads("log.jsonl"));
        assert!(Table::reads("log.ndjson") && Table::reads("x.tab"));
        assert!(Table::reads("people.DBF"));
        assert!(!Table::reads("a.json") && !Table::reads("a.txt"));
        assert!(Table::of("a.txt", "x").is_none());
        // A `.dbf` is not read out of text at all: it has its own constructor, because its
        // header says where the records are rather than the bytes being lines.
        assert!(Table::of("a.dbf", "x").is_none());
    }
}
