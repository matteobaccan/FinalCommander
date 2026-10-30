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
            order: None,
            sort: None,
            filter: None,
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
            order: None,
            sort: None,
            filter: None,
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
            order: None,
            sort: None,
            filter: None,
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
    fn record_of(&self, row: usize) -> Option<usize> {
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
            self.order = None;
            return;
        }
        let count = self.records_len();
        // The cells each record is judged by, read once: the whole record for a filter, the
        // one column for a sort. A DBF is a read apiece, which is why `MAX_VIEW` exists.
        let mut keep: Vec<(usize, String)> = Vec::with_capacity(count);
        let col = self.sort.map(|(col, _)| col);
        for rec in 0..count {
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
    fn cells_of(
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
    fn cells(
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
