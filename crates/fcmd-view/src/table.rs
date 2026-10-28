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

/// A parsed table: what the columns are called, and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Row>,
    /// Display width of each column, from the header and the rows that were read.
    pub widths: Vec<usize>,
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
        [".csv", ".tsv", ".tab", ".jsonl", ".ndjson"]
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
            rows: records,
            widths: Vec::new(),
            format: if delimiter == '\t' { "TSV" } else { "CSV" },
            delimiter: guessed.then_some(delimiter),
            cut: false,
            skipped: 0,
        };
        // A row with more fields than the header has them: a header short of a column is a
        // common export, and dropping the data would be worse than naming the column after
        // the number it is.
        let widest = table.rows.iter().map(|r| r.cells.len()).max().unwrap_or(0);
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
            rows,
            widths: Vec::new(),
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
        for row in &self.rows {
            for (i, cell) in row.cells.iter().enumerate() {
                if i >= self.widths.len() {
                    continue;
                }
                self.widths[i] = self.widths[i].max(cell.width()).min(MAX_COL);
            }
        }
        // A column with nothing in it and no name still needs a cell to sit in.
        for w in &mut self.widths {
            *w = (*w).max(1);
        }
    }

    /// The heading, as one line, scrolled right by `col` display columns.
    pub fn header(&self, col: usize, width: usize) -> String {
        let cells: Vec<&str> = self.columns.iter().map(String::as_str).collect();
        lay_out(&cells, &self.widths, col, width)
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

    /// One row of cells, laid out in the columns.
    pub fn line(&self, row: usize, col: usize, width: usize) -> String {
        let Some(row) = self.rows.get(row) else {
            return String::new();
        };
        let cells: Vec<&str> = row.cells.iter().map(String::as_str).collect();
        lay_out(&cells, &self.widths, col, width)
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
fn lay_out(cells: &[&str], widths: &[usize], col: usize, width: usize) -> String {
    let mut line = String::new();
    for (i, w) in widths.iter().enumerate() {
        if i > 0 {
            line.push_str(" │ ");
        }
        let cell = cells.get(i).copied().unwrap_or("");
        let cell = truncate(cell, *w);
        line.push_str(&cell);
        for _ in cell.width()..*w {
            line.push(' ');
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_comma_separated_file_reads_as_a_table_with_its_first_line_as_the_header() {
        let table = Table::of("people.csv", "name,age\nmb,51\nada,36\n").expect("a table");
        assert_eq!(table.columns, ["name", "age"]);
        assert_eq!(table.rows.len(), 2);
        assert_eq!(table.rows[0].cells, ["mb", "51"]);
        assert_eq!(table.format, "CSV");
        // The delimiter was guessed, so the status line has something to admit.
        assert_eq!(table.delimiter, Some(','));
        // Each row knows where it began, so a search can say where it found something.
        assert_eq!(table.rows[0].offset, "name,age\n".len() as u64);
    }

    #[test]
    fn a_quoted_field_may_hold_the_delimiter_and_a_newline() {
        let text = "a,b\n\"one,two\",\"line\nbreak\"\n\"say \"\"hi\"\"\",last\n";
        let table = Table::separated(text, Some(','));
        assert_eq!(table.rows[0].cells, ["one,two", "line\nbreak"]);
        assert_eq!(table.rows[1].cells, ["say \"hi\"", "last"]);
    }

    #[test]
    fn the_delimiter_is_guessed_from_what_gives_a_table() {
        // Semicolons: the European export. The commas inside the numbers must not win.
        let table = Table::separated("name;price\nbread;1,20\nmilk;0,99\n", None);
        assert_eq!(table.delimiter, Some(';'));
        assert_eq!(table.columns, ["name", "price"]);
        assert_eq!(table.rows[0].cells, ["bread", "1,20"]);

        // Tabs, which a `.tsv` says outright but a `.csv` may still hold.
        let table = Table::separated("a\tb\n1\t2\n", None);
        assert_eq!(table.delimiter, Some('\t'));

        // One column and nothing to separate: no guess is better than a wrong one.
        let table = Table::separated("just one line\n", None);
        assert_eq!(table.columns, ["just one line"]);
        assert!(table.rows.is_empty());
    }

    #[test]
    fn a_row_with_more_fields_than_the_header_keeps_them() {
        let table = Table::separated("a,b\n1,2,3\n", Some(','));
        assert_eq!(table.columns, ["a", "b", "#3"]);
        assert_eq!(table.rows[0].cells, ["1", "2", "3"]);
    }

    #[test]
    fn json_lines_take_their_columns_from_the_keys_in_the_order_they_appear() {
        let text = "{\"when\":\"now\",\"what\":\"start\"}\n{\"what\":\"stop\",\"code\":2}\n";
        let table = Table::json_lines(text);
        assert_eq!(table.columns, ["when", "what", "code"]);
        assert_eq!(table.rows[0].cells, ["now", "start", ""]);
        assert_eq!(table.rows[1].cells, ["", "stop", "2"]);
        assert_eq!(table.format, "JSONL");

        // A line that is not an object is counted, not shown as a row of nonsense.
        let table = Table::json_lines("[\n{\"a\":1}\nnot json\n");
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.skipped, 2);

        // Nested values are their own compact JSON, and null is an empty cell.
        let table = Table::json_lines("{\"a\":{\"b\":1},\"c\":[1,2],\"d\":null}\n");
        assert_eq!(table.rows[0].cells, ["{\"b\":1}", "[1,2]", ""]);
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
        let last = table.rows.last().expect("a row");
        assert_eq!(last.cells, ["999", "late=yes"]);
    }

    #[test]
    fn the_layout_pads_the_columns_and_the_window_scrolls_sideways() {
        let table = Table::of("t.csv", "name,age\nmb,51\n").expect("a table");
        assert_eq!(table.widths, [4, 3]);
        assert_eq!(table.header(0, 40), "name │ age");
        assert_eq!(table.rule(0, 40), "──── │ ───".replace(" │ ", "─┼─"));
        assert_eq!(table.line(0, 0, 40), "mb   │ 51 ");

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
        let line = table.line(0, 0, 100);
        assert!(line.ends_with('…'), "{line}");
        assert_eq!(line.width(), MAX_COL);
    }

    #[test]
    fn only_the_names_table_mode_can_read_are_offered() {
        assert!(Table::reads("a.csv") && Table::reads("A.TSV") && Table::reads("log.jsonl"));
        assert!(Table::reads("log.ndjson") && Table::reads("x.tab"));
        assert!(!Table::reads("a.json") && !Table::reads("a.txt") && !Table::reads("a.dbf"));
        assert!(Table::of("a.txt", "x").is_none());
    }
}
