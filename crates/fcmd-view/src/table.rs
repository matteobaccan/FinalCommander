//! Table mode: the tabular formats read as a table (SPEC 05 §1.11).
//!
//! A commander opens a `.csv` and a `.jsonl` every day, and both read in a text viewer as what
//! they are not — a wall of commas, a wall of braces. What they want is not two viewers but
//! one mode: the screen is the same in either case (a header, columns, a row under the
//! cursor) and all that differs is how a record is read out of the bytes. So the parsing ends
//! here, in one type, and the viewer above knows only about rows and columns.
//!
//! What this is not, and deliberately: a query tool and a spreadsheet. Sorting and filtering
//! are an index over the records and write nothing. The one format that is written is the
//! dBase table, a record at a time, and that lives apart in `dbf_write.rs`.

use unicode_width::UnicodeWidthStr;

pub use crate::dbf::Dbf;

/// The widest a column is allowed to be. A single 4 kB cell must not push every other column
/// off the screen; what it holds is read with the horizontal scroll, as a long line is.
pub(crate) const MAX_COL: usize = 40;

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
    /// The first line is the heading. Always so for a format that names its fields (JSON
    /// Lines, dBase); for delimited text it is *guessed*, because an export without a heading
    /// is as common as one with, and a guess the reader can turn round with one key.
    pub headed: bool,
    /// The delimiter the text was split on, guessed or not; `None` for a format that has none.
    pub separator: Option<char>,
    /// The rows as the *view* shows them: one record index per visible row, and `None` while
    /// the view is the file's own order, so a table of a million records costs no index at
    /// all until somebody sorts or filters it (SPEC 05 §1.11).
    ///
    /// This is the whole of sorting and filtering a table: an index, rebuilt from the
    /// records. Nothing is written, the records are untouched, and the view goes back to the
    /// file's own order by throwing the index away.
    order: Option<Vec<usize>>,
    /// Which column the view is sorted by and which way, for the status line to say.
    sort: Option<(usize, bool)>,
    /// What the rows are filtered on, for the status line to say.
    filter: Option<String>,
    /// The records the view is kept to, when it shows the marked rows only: the marks as
    /// they stood when that was asked for. An index like the other two, and gone as easily.
    only: Option<std::collections::BTreeSet<usize>>,
}

/// The extensions a dBase table goes by. `.dbf`, and the files Visual FoxPro keeps its own
/// things in, which are tables under other names: a database container, a form, a report,
/// a class library, a menu, a project, a label.
const DBASE_NAMES: [&str; 8] = [
    ".dbf", ".dbc", ".scx", ".frx", ".vcx", ".mnx", ".pjx", ".lbx",
];

/// Does this name say dBase table? What decides whether a file is read by its header a
/// record at a time rather than whole as text.
pub fn dbase_name(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    DBASE_NAMES.iter().any(|ext| name.ends_with(ext))
}

/// What sorting a column did, which is what the caller has to say out loud.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sorted {
    Ascending,
    Descending,
    /// Back to the order the records are in the file.
    Natural,
    /// More records than [`MAX_VIEW`]: nothing was changed.
    TooMany,
}

/// The most records a sort or a filter will walk.
///
/// Both read the key out of every record, which for a DBF is a read apiece. The text formats
/// are already bounded by what the mode reads whole, so this only ever bites a dBase table of
/// millions — and a table of millions is one to query rather than to sort in a viewer.
pub const MAX_VIEW: usize = 200_000;

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
            headed: true,
            separator: None,
            order: None,
            sort: None,
            filter: None,
            only: None,
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
        Table::of_with(name, text, None)
    }

    /// [`Table::of`], with the heading of delimited text decided rather than guessed.
    pub fn of_with(name: &str, text: &str, header: Option<bool>) -> Option<Table> {
        let name = name.to_ascii_lowercase();
        if name.ends_with(".csv") {
            return Some(Table::separated_with(text, None, header));
        }
        if name.ends_with(".tsv") || name.ends_with(".tab") {
            return Some(Table::separated_with(text, Some('\t'), header));
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
        [".csv", ".tsv", ".tab", ".jsonl", ".ndjson"]
            .iter()
            .any(|ext| name.ends_with(ext))
            // A dBase table is read where the build has the reader for it; in the compact
            // build it is bytes, like any other file nobody here reads.
            || (crate::dbf::BUILT_IN && dbase_name(&name))
    }

    /// Delimiter-separated text, RFC 4180 quoting. `delimiter` is `None` to guess; whether
    /// the first line is a heading is guessed too (see [`Table::separated_with`]).
    pub fn separated(text: &str, delimiter: Option<char>) -> Table {
        Table::separated_with(text, delimiter, None)
    }

    /// Delimiter-separated text, with the heading decided rather than guessed when `header`
    /// is `Some`. Without one the columns are called by their number and the first line is a
    /// record like the others.
    pub fn separated_with(text: &str, delimiter: Option<char>, header: Option<bool>) -> Table {
        let guessed = delimiter.is_none();
        let delimiter = delimiter.unwrap_or_else(|| sniff(text));
        let mut records = parse_separated(text, delimiter);
        let headed = header.unwrap_or_else(|| has_header(&records));
        let header = if records.is_empty() || !headed {
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
            headed,
            separator: Some(delimiter),
            order: None,
            sort: None,
            filter: None,
            only: None,
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
            headed: true,
            separator: None,
            order: None,
            sort: None,
            filter: None,
            only: None,
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
                    // The width of the cell *as it will be drawn*: one line, so a value
                    // with a newline in it is not measured a column wider than it appears.
                    let shown = as_drawn(cell);
                    self.widths[i] = self.widths[i].max(shown.width()).min(MAX_COL);
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

    /// How many records the file has, whatever the view is showing of them.
    pub fn records_len(&self) -> usize {
        match &self.records {
            Records::Read(rows) => rows.len(),
            Records::Dbf(dbf) => dbf.records,
        }
    }

    /// How many rows the view shows: every record, until a filter leaves fewer.
    pub fn len(&self) -> usize {
        match &self.order {
            Some(order) => order.len(),
            None => self.records_len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Which record a view row is, following whatever the view has been sorted or filtered
    /// to. The two are the same number until something sorts or filters.
    pub fn record_of(&self, row: usize) -> Option<usize> {
        match &self.order {
            Some(order) => order.get(row).copied(),
            None => (row < self.records_len()).then_some(row),
        }
    }

    /// Where the record a view row shows begins in the file — what a search or a bookmark in
    /// table mode means.
    pub fn offset(&self, n: usize) -> u64 {
        let Some(rec) = self.record_of(n) else {
            return 0;
        };
        self.record_offset(rec)
    }

    fn record_offset(&self, rec: usize) -> u64 {
        match &self.records {
            Records::Read(rows) => rows.get(rec).map(|r| r.offset).unwrap_or(0),
            Records::Dbf(dbf) => dbf.offset(rec),
        }
    }

    /// Which *view row* a byte offset falls in: the record that holds it, found where the
    /// view happens to be showing it. A sorted view is not in file order, so this is a
    /// lookup rather than arithmetic there — but every offset the viewer asks about came
    /// from [`Table::offset`], so the record it names is one the view is showing.
    pub fn row_at(&self, offset: u64) -> usize {
        let rec = self.record_at(offset);
        match &self.order {
            None => rec,
            Some(order) => order.iter().position(|i| *i == rec).unwrap_or_default(),
        }
    }

    /// Which record a byte offset falls in: the last one that starts at or before it.
    fn record_at(&self, offset: u64) -> usize {
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

    /// Which column the view is sorted by, and whether it is the other way round.
    pub fn sorted_by(&self) -> Option<(usize, bool)> {
        self.sort
    }

    /// What the rows are filtered on, where they are.
    pub fn filtered_on(&self) -> Option<&str> {
        self.filter.as_deref()
    }

    /// `Ctrl+F3` on a column: ascending, then descending, then the file's own order again
    /// (SPEC 05 §1.11). A third press is what puts a table back the way it was read, which a
    /// sort with no way out would not.
    ///
    /// Nothing is written. What changes is the index the view reads the records through, so
    /// the file is untouched and the rows keep their own offsets — a search, a bookmark and
    /// `Ctrl+F10` still mean the record, not the place in the sorted list.
    pub fn cycle_sort(
        &mut self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        col: usize,
    ) -> Sorted {
        let next = match self.sort {
            Some((was, false)) if was == col => Some((col, true)),
            Some((was, true)) if was == col => None,
            _ => Some((col, false)),
        };
        if self.records_len() > MAX_VIEW {
            return Sorted::TooMany;
        }
        self.sort = next;
        self.rebuild(source, memo);
        match next {
            Some((_, false)) => Sorted::Ascending,
            Some((_, true)) => Sorted::Descending,
            None => Sorted::Natural,
        }
    }

    /// Is the view kept to the rows that were marked?
    pub fn only_marked(&self) -> bool {
        self.only.is_some()
    }

    /// Keep the view to these records — the marked ones — or to all of them again (`None`).
    /// The sort and the filter stay as they are and apply within what is kept. `false` when
    /// those would have to walk more records than they will, in which case nothing changed.
    pub fn set_only(
        &mut self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        only: Option<std::collections::BTreeSet<usize>>,
    ) -> bool {
        let walks = self.sort.is_some() || self.filter.is_some();
        if walks && self.records_len() > MAX_VIEW {
            return false;
        }
        self.only = only;
        self.rebuild(source, memo);
        true
    }

    /// Keep only the rows that hold `needle`, or all of them again when it is empty. `false`
    /// when there are more records than this will walk, in which case nothing changed.
    pub fn set_filter(
        &mut self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        needle: &str,
    ) -> bool {
        if self.records_len() > MAX_VIEW {
            return false;
        }
        let needle = needle.trim();
        self.filter = (!needle.is_empty()).then(|| needle.to_lowercase());
        self.rebuild(source, memo);
        true
    }

    /// Build the index the view reads through: the filter decides which records are in it,
    /// the sort decides the order they are in. Both from the records, so applying one does
    /// not depend on what the other did last.
    fn rebuild(
        &mut self,
        source: &mut dyn crate::source::Source,
        mut memo: Option<&mut (dyn crate::source::Source + '_)>,
    ) {
        if self.sort.is_none() && self.filter.is_none() {
            // The marked rows alone need no record read: the marks are the index, in the
            // order of the file.
            let count = self.records_len();
            self.order = self
                .only
                .as_ref()
                .map(|only| only.iter().copied().filter(|rec| *rec < count).collect());
            return;
        }
        let count = self.records_len();
        // The cells each record is judged by, read once: the whole record for a filter, the
        // one column for a sort. A DBF is a read apiece, which is why `MAX_VIEW` exists.
        let mut keep: Vec<(usize, String)> = Vec::with_capacity(count);
        let col = self.sort.map(|(col, _)| col);
        for rec in 0..count {
            if self.only.as_ref().is_some_and(|only| !only.contains(&rec)) {
                continue;
            }
            let cells = self.cells_of(source, memo.as_deref_mut(), rec);
            if let Some(needle) = &self.filter {
                if !cells.iter().any(|c| c.to_lowercase().contains(needle)) {
                    continue;
                }
            }
            let key = col
                .and_then(|col| cells.get(col))
                .cloned()
                .unwrap_or_default();
            keep.push((rec, key));
        }
        if let Some((_, reverse)) = self.sort {
            // A column whose every cell reads as a number sorts as numbers: a column of
            // sizes must not put 1000 before 9. Anything else sorts as text, with the same
            // comparison the panels sort names with.
            let numeric = keep
                .iter()
                .all(|(_, key)| key.trim().is_empty() || as_number(key).is_some());
            keep.sort_by(|(_, a), (_, b)| {
                let order = if numeric {
                    let (x, y) = (as_number(a).unwrap_or(0.0), as_number(b).unwrap_or(0.0));
                    x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
                } else {
                    fcmd_core::sort::natural_cmp(a, b, false)
                };
                if reverse {
                    order.reverse()
                } else {
                    order
                }
            });
        }
        self.order = Some(keep.into_iter().map(|(rec, _)| rec).collect());
    }

    /// The cells of one *record*, by its own index rather than the view's.
    pub(crate) fn cells_of(
        &self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        rec: usize,
    ) -> Vec<String> {
        match &self.records {
            Records::Read(rows) => rows.get(rec).map(|r| r.cells.clone()).unwrap_or_default(),
            Records::Dbf(dbf) => {
                if rec >= dbf.records {
                    return Vec::new();
                }
                let bytes = source
                    .read_vec(dbf.offset(rec), dbf.record_len)
                    .unwrap_or_default();
                if bytes.len() < dbf.record_len {
                    // A table shorter than its header claims: the rest is not there to read.
                    return Vec::new();
                }
                dbf.cells(&bytes, memo)
            }
        }
    }

    /// The cells of one record. The source is only touched for a table that is read a window
    /// at a time; for the text formats it is already in memory and is not read again.
    ///
    /// `memo` is the file beside the table that holds what its memo fields point at — a
    /// `.dbt` or an `.fpt` — when there is one and it could be opened.
    pub fn cells(
        &self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        row: usize,
    ) -> Vec<String> {
        let Some(rec) = self.record_of(row) else {
            return Vec::new();
        };
        self.cells_of(source, memo, rec)
    }

    /// One row of cells, laid out in the columns.
    pub fn line(
        &self,
        source: &mut dyn crate::source::Source,
        memo: Option<&mut (dyn crate::source::Source + '_)>,
        row: usize,
        col: usize,
        width: usize,
    ) -> String {
        let cells = self.cells(source, memo, row);
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

/// A cell as a number, where it reads as one: what decides whether a column sorts as numbers.
///
/// A thousands separator is taken off and a decimal comma is taken as a decimal point, which
/// is what a cell written by a spreadsheet in half of Europe looks like. Anything else is
/// text, and text is what it sorts as.
fn as_number(cell: &str) -> Option<f64> {
    let cell = cell.trim();
    if cell.is_empty() {
        return None;
    }
    // A comma is a decimal point only when it is the one comma in the cell and three digits
    // do not follow it: `1,20` is a price, `1,200` is a thousand and two hundred.
    let decimal_comma = cell.matches(',').count() == 1
        && cell
            .split_once(',')
            .is_some_and(|(_, rest)| rest.len() != 3 || rest.contains(['.', ' ']));
    let mut text = String::with_capacity(cell.len());
    for ch in cell.chars() {
        match ch {
            ',' if decimal_comma => text.push('.'),
            ',' | '\'' | ' ' | '_' => {}
            other => text.push(other),
        }
    }
    text.parse::<f64>().ok().filter(|n| n.is_finite())
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

/// A cell's text as the screen will draw it: every control character replaced by a space.
///
/// A table row *is* one line, and a value with a newline in it — which a CSV field may hold
/// between quotes and a JSON string holds whenever somebody wrote a paragraph — cannot be
/// drawn as one. Replacing rather than dropping is what keeps the words apart: a description
/// reading `Attività:` then a list would otherwise come out as `Attività:- Eseguire`.
///
/// It is also what keeps the columns lined up, and that is the sharper reason. The two halves
/// of the program disagree about a control character: `UnicodeWidthStr::width` counts one as
/// **one** column, while the screen buffer skips it and draws **none**. A cell holding a
/// newline was therefore measured one column wider than it was drawn, and every column to the
/// right of it slid one place left on that row and on no other. With the controls gone the
/// two measures cannot disagree, which is a better answer than teaching each of them the
/// other's rule.
///
/// Borrowed where there is nothing to replace, which is nearly every cell of nearly every
/// table.
fn as_drawn(text: &str) -> std::borrow::Cow<'_, str> {
    if !text.chars().any(|c| c.is_control()) {
        return std::borrow::Cow::Borrowed(text);
    }
    std::borrow::Cow::Owned(
        text.chars()
            .map(|c| if c.is_control() { ' ' } else { c })
            .collect(),
    )
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
    use unicode_width::UnicodeWidthChar;
    let mut out = String::new();
    let mut at = 0usize;
    for ch in line.chars() {
        // Per character and not per string: `UnicodeWidthStr::width` gives a control
        // character one column and the screen buffer gives it none, and this has to agree
        // with the screen.
        let w = ch.width().unwrap_or(0);
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
///
/// The cell is flattened to one line first ([`as_drawn`]), so what is measured here is what
/// the screen will draw.
fn truncate(cell: &str, width: usize) -> String {
    let cell = as_drawn(cell);
    let cell: &str = &cell;
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
    const CANDIDATES: [char; 5] = [',', ';', '\t', '|', '^'];
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

/// Is the first record a heading, or the first row of data?
///
/// A heading is words over the data, so the test is the columns whose data has a shape a
/// word would break: where every cell below reads as a number, a first cell that is not one
/// votes for a heading and a first cell that is one votes against; where the cells below all
/// have one length (codes, dates, flags), a first cell of another length votes for it. A
/// first line that repeats a value found under it is data. With no vote either way the first
/// line is the heading, which is what nearly every export writes and what this mode always
/// assumed.
fn has_header(records: &[Row]) -> bool {
    let Some((first, rest)) = records.split_first() else {
        return true;
    };
    let rest: Vec<&Row> = rest.iter().take(HEADER_RECORDS).collect();
    if rest.is_empty() {
        return true;
    }
    // An empty cell in the first line is a column nobody named, which a heading rarely has
    // and data often does.
    let mut votes = 0i32;
    for (c, head) in first.cells.iter().enumerate() {
        let below: Vec<&str> = rest
            .iter()
            .filter_map(|r| r.cells.get(c).map(String::as_str))
            .filter(|s| !s.trim().is_empty())
            .collect();
        if below.is_empty() {
            continue;
        }
        let head = head.trim();
        if head.is_empty() {
            votes -= 1;
            continue;
        }
        if below.contains(&head) {
            votes -= 1;
            continue;
        }
        if below.iter().all(|s| as_number(s).is_some()) {
            votes += if as_number(head).is_some() { -1 } else { 1 };
            continue;
        }
        let len = below[0].chars().count();
        if below.len() > 1 && below.iter().all(|s| s.chars().count() == len) {
            votes += if head.chars().count() == len { -1 } else { 1 };
        }
    }
    votes >= 0
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

#[cfg(test)]
pub(crate) mod tests {

    #[test]
    fn the_heading_is_guessed_from_what_is_under_it() {
        // Words over numbers: a heading.
        let t = Table::separated("name,size\na,10\nb,2000\n", None);
        assert!(t.headed);
        assert_eq!(t.columns, vec!["name", "size"]);
        // Numbers over numbers: data, and the columns are called by their number.
        let t = Table::separated("1,10\n2,20\n3,30\n", None);
        assert!(!t.headed);
        assert_eq!(t.columns, vec!["#1", "#2"]);
        assert_eq!(t.len(), 3);
        // Codes of one length under a longer word: a heading.
        let t = Table::separated("country;code\nItaly;IT\nFrance;FR\n", None);
        assert!(t.headed);
        // A first line whose value turns up again below is data.
        let t = Table::separated("red,apple\nred,cherry\ngreen,pear\n", None);
        assert!(!t.headed);
        // Text over text with nothing to go by: the heading every export writes.
        let t = Table::separated("first,last\nAda,Lovelace\nAlan,Turing\n", None);
        assert!(t.headed);
        // And told rather than guessed.
        let t = Table::separated_with("1,10\n2,20\n", None, Some(true));
        assert!(t.headed);
        assert_eq!(t.columns, vec!["1", "10"]);
    }

    #[test]
    fn a_caret_is_a_delimiter() {
        let t = Table::separated("name^size\na^10\nb^20\n", None);
        assert_eq!(t.delimiter, Some('^'));
        assert_eq!(t.separator, Some('^'));
        assert_eq!(t.columns, vec!["name", "size"]);
    }

    use super::*;

    /// A source with nothing in it: the text formats never read from it, and the tests that
    /// do read — the dBase ones — build their own.
    fn src() -> crate::source::BytesSource {
        crate::source::BytesSource::new(Vec::new())
    }

    /// Sorting a table is an index and nothing else: the rows come out in order, the records
    /// keep their own offsets, and a third press puts the file's own order back.
    #[test]
    fn sorting_a_column_reorders_the_view_and_goes_back() {
        let text = "name,size\nzoe,9\nadam,1000\nmia,20\n";
        let mut table = Table::of("x.csv", text).expect("a CSV");
        let mut src = src();
        assert_eq!(table.len(), 3);
        let names = |t: &Table, src: &mut crate::source::BytesSource| -> Vec<String> {
            (0..t.len())
                .map(|row| t.cells(src, None, row)[0].clone())
                .collect()
        };
        assert_eq!(names(&table, &mut src), ["zoe", "adam", "mia"]);

        // By name, ascending.
        assert_eq!(table.cycle_sort(&mut src, None, 0), Sorted::Ascending);
        assert_eq!(names(&table, &mut src), ["adam", "mia", "zoe"]);
        assert_eq!(table.sorted_by(), Some((0, false)));
        // The offsets are still the records': the first row is where `adam` is in the file.
        assert_eq!(table.offset(0), text.find("adam").unwrap() as u64);
        // And a byte offset still finds the row that shows it, wherever the sort put it.
        assert_eq!(table.row_at(table.offset(2)), 2);

        // Again, the other way; a third press is the file's own order.
        assert_eq!(table.cycle_sort(&mut src, None, 0), Sorted::Descending);
        assert_eq!(names(&table, &mut src), ["zoe", "mia", "adam"]);
        assert_eq!(table.cycle_sort(&mut src, None, 0), Sorted::Natural);
        assert_eq!(names(&table, &mut src), ["zoe", "adam", "mia"]);
        assert_eq!(table.sorted_by(), None);

        // The size column is numbers, so 1000 sorts after 20 rather than before it.
        assert_eq!(table.cycle_sort(&mut src, None, 1), Sorted::Ascending);
        assert_eq!(names(&table, &mut src), ["zoe", "mia", "adam"]);
    }

    /// A filter keeps the rows that hold what was typed, and an empty one gives them back.
    #[test]
    fn a_filter_keeps_the_rows_that_hold_it() {
        let text = "name,town\nmia,Torino\nadam,Asti\nzoe,Trento\n";
        let mut table = Table::of("x.csv", text).expect("a CSV");
        let mut src = src();

        assert!(table.set_filter(&mut src, None, "t"));
        // Three rows hold a `t`: Torino, Asti and Trento.
        assert_eq!(table.len(), 3);
        assert!(table.set_filter(&mut src, None, "tor"));
        assert_eq!(table.len(), 1);
        assert_eq!(table.cells(&mut src, None, 0)[0], "mia");
        assert_eq!(table.filtered_on(), Some("tor"));
        // It composes with a sort, and each is undone on its own.
        assert_eq!(table.cycle_sort(&mut src, None, 0), Sorted::Ascending);
        assert_eq!(table.len(), 1, "the filter still holds");
        assert!(table.set_filter(&mut src, None, "  "));
        assert_eq!(table.len(), 3, "and the sort still holds over all of them");
        assert_eq!(table.cells(&mut src, None, 0)[0], "adam");
        assert_eq!(table.filtered_on(), None);
    }

    /// A cell reads as a number where it is one, including the way half of Europe writes a
    /// price — and `1,200` is a thousand and two hundred rather than one and a fifth.
    #[test]
    fn the_marked_rows_alone_are_a_view_that_works_with_the_sort_and_the_filter() {
        let mut table = Table::separated("name,size\nb,10\na,9\nc,1000\nab,5\n", None);
        let mut source = src();
        let names = |table: &Table, source: &mut crate::source::BytesSource| -> Vec<String> {
            (0..table.len())
                .map(|row| table.cells(source, None, row)[0].clone())
                .collect()
        };
        // Records 3, 0 and 2, in the order of the file — and one that is not a record.
        let only: std::collections::BTreeSet<usize> = [3, 0, 2, 99].into_iter().collect();
        assert!(table.set_only(&mut source, None, Some(only)));
        assert!(table.only_marked());
        assert_eq!(names(&table, &mut source), ["b", "c", "ab"]);
        assert_eq!(table.record_of(1), Some(2));
        // Sorted and filtered within what is kept.
        table.cycle_sort(&mut source, None, 1);
        assert_eq!(names(&table, &mut source), ["ab", "b", "c"]);
        assert!(table.set_filter(&mut source, None, "b"));
        assert_eq!(names(&table, &mut source), ["ab", "b"]);
        // Every row again, with the sort and the filter as they were.
        assert!(table.set_only(&mut source, None, None));
        assert!(!table.only_marked());
        assert_eq!(names(&table, &mut source), ["ab", "b"]);
        assert!(table.set_filter(&mut source, None, ""));
        assert_eq!(names(&table, &mut source), ["ab", "a", "b", "c"]);
    }

    #[test]
    fn a_number_is_a_number_however_it_is_written() {
        assert_eq!(as_number("42"), Some(42.0));
        assert_eq!(as_number(" -3.5 "), Some(-3.5));
        assert_eq!(as_number("1,20"), Some(1.2));
        assert_eq!(as_number("1,200"), Some(1200.0));
        assert_eq!(as_number("1 000"), Some(1000.0));
        assert_eq!(as_number("mia"), None);
        assert_eq!(as_number(""), None);
        assert_eq!(as_number("12 euro"), None);
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

    /// A value with a newline in it is still one row, and every column to the right of it
    /// stays where the heading says it is.
    ///
    /// The two halves of the program used to disagree about a control character —
    /// `UnicodeWidthStr::width` gives one a column and the screen buffer draws none — so a
    /// cell holding one was measured a column wider than it appeared, and the rest of that
    /// row slid one place left. A job description written as a paragraph and a list did it
    /// on every record that had one.
    #[test]
    fn a_cell_with_a_newline_in_it_does_not_move_the_columns() {
        let text = concat!(
            r#"{"titolo":"Ortopedico","descrizione":"Attivita:\n- Eseguire visite","citta":"Fidenza"}"#,
            "\n",
            r#"{"titolo":"Ecografista","descrizione":"Indagini diagnostiche","citta":"Savona"}"#,
            "\n",
        );
        let table = Table::json_lines(text);
        assert_eq!(table.records_len(), 2, "both records were read");
        let header = table.header(0, 200);
        let a = table.line(
            &mut crate::source::BytesSource::new(Vec::new()),
            None,
            0,
            0,
            200,
        );
        let b = table.line(
            &mut crate::source::BytesSource::new(Vec::new()),
            None,
            1,
            0,
            200,
        );

        // Measured the way the screen measures it: a control character is worth nothing,
        // which is what `Buffer::put_str` draws.
        let drawn = |line: &str| -> usize {
            use unicode_width::UnicodeWidthChar;
            line.chars().map(|c| c.width().unwrap_or(0)).sum()
        };
        assert_eq!(drawn(&a), drawn(&b), "{a:?}\n{b:?}");
        assert_eq!(drawn(&a), drawn(&header), "{a:?}\n{header:?}");

        // The separators therefore stand in the same columns on every row.
        let bars = |line: &str| -> Vec<usize> {
            use unicode_width::UnicodeWidthChar;
            let mut at = 0;
            let mut out = Vec::new();
            for ch in line.chars() {
                if ch == '│' {
                    out.push(at);
                }
                at += ch.width().unwrap_or(0);
            }
            out
        };
        assert_eq!(bars(&a), bars(&header));
        assert_eq!(bars(&b), bars(&header));

        // And the newline itself is a space, so the words do not run together.
        assert!(a.contains("Attivita: - Eseguire"), "{a:?}");
        assert!(!a.contains('\n'));
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
        assert_eq!(table.line(&mut src(), None, 0, 0, 40), "mb   │ 51 ");

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
        let line = table.line(&mut src(), None, 0, 0, 100);
        assert!(line.ends_with('…'), "{line}");
        assert_eq!(line.width(), MAX_COL);
    }

    #[test]
    fn only_the_names_table_mode_can_read_are_offered() {
        assert!(Table::reads("a.csv") && Table::reads("A.TSV") && Table::reads("log.jsonl"));
        assert!(Table::reads("log.ndjson") && Table::reads("x.tab"));
        assert_eq!(Table::reads("people.DBF"), crate::dbf::BUILT_IN);
        assert!(!Table::reads("a.json") && !Table::reads("a.txt"));
        assert!(Table::of("a.txt", "x").is_none());
        // A `.dbf` is not read out of text at all: it has its own constructor, because its
        // header says where the records are rather than the bytes being lines.
        assert!(Table::of("a.dbf", "x").is_none());
    }
}
