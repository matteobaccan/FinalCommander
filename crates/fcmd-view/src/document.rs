//! The text of an office document, laid out to be read (SPEC 05 §1.16).
//!
//! A `.docx` and an `.odt` are zip files with the text in one XML part — `word/document.xml`
//! and `content.xml` — and a `.pptx` and an `.odp` are the same with a part per slide or the
//! slides one after the other. What somebody pressing `F3` on one of them came for is the
//! words, so this module reads that part and writes the document out again as Markdown: a
//! heading as a heading, a list as a list, a table as a table. The viewer then shows it the
//! way it shows any other Markdown (`markdown.rs`), which is why nothing here knows about
//! widths, wrapping or colours.
//!
//! Markdown is the form in between and nobody reads it, so every mark a document's own text
//! could be taken for is escaped: an asterisk in a paragraph stays an asterisk.
//!
//! The zip is read with the reader the structure pane already has (`office.rs`) and the XML
//! with the few lines below, which is all the XML these formats need: elements, attributes,
//! text and the five entities. Neither trusts the file. An entry is inflated up to a count of
//! our own whatever size it declares, all the parts of one document share one allowance, the
//! XML is walked as a flat run of tags with counters instead of recursion — so nesting a
//! million elements deep costs a million small steps and no stack — and no entity a document
//! defines for itself is ever expanded.

use std::collections::HashMap;

use crate::office::{central_directory, part, Entry, Part};
use crate::structure::Bytes;

/// How much XML one document may inflate to, all its parts together. A novel is about two
/// megabytes of `document.xml`; sixteen is the table limit, and a part that goes past it is
/// either not prose or not honest.
pub const XML_LIMIT: usize = 16 * 1024 * 1024;

/// How long the text may come out: what the Markdown view lays out whole.
pub const TEXT_LIMIT: usize = 4 * 1024 * 1024;

/// How many slides of a presentation are read.
const SLIDES: usize = 2000;

/// The line separator: a line that ends inside a paragraph, which is what `w:br` and
/// `text:line-break` are. The Markdown view starts a new line at one.
const BREAK: char = '\u{2028}';

/// Why a document is not shown as its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentError {
    /// Its XML or its text is past the limit.
    TooLarge,
    /// Not a zip, no text part in it, or a part that cannot be read: encrypted, written
    /// with a method no office program uses, or not XML.
    Unreadable,
}

/// Does this name say "a document whose text we can show"?
pub fn is_document(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    [
        ".docx", ".docm", ".dotx", ".dotm", ".odt", ".ott", ".pptx", ".pptm", ".ppsx", ".potx",
        ".odp", ".otp",
    ]
    .iter()
    .any(|ext| name.ends_with(ext))
}

/// The document as Markdown. Which of the three families it is comes from the parts the zip
/// holds and not from its name, so a `.docx` that is really an `.odt` still reads.
pub fn to_markdown(src: &mut dyn Bytes) -> Result<String, DocumentError> {
    let mut zip = Zip::open(src).ok_or(DocumentError::Unreadable)?;
    let mut md = Md::default();
    if zip.has("word/document.xml") {
        docx(&mut zip, &mut md)?;
    } else if zip.has("ppt/presentation.xml") {
        pptx(&mut zip, &mut md)?;
    } else if zip.has("content.xml") {
        odf(&mut zip, &mut md)?;
    } else {
        return Err(DocumentError::Unreadable);
    }
    md.finish()
}

// ── the zip ────────────────────────────────────────────────────────────────────────────────

/// The archive, and how much of the allowance is left.
struct Zip<'a> {
    src: &'a mut dyn Bytes,
    len: u64,
    dir: Vec<Entry>,
    left: usize,
}

impl<'a> Zip<'a> {
    fn open(src: &'a mut dyn Bytes) -> Option<Zip<'a>> {
        let len = src.len();
        if src.at(0, 4) != b"PK\x03\x04" {
            return None;
        }
        let dir = central_directory(src, len)?;
        Some(Zip {
            src,
            len,
            dir,
            left: XML_LIMIT,
        })
    }

    fn has(&self, name: &str) -> bool {
        self.dir.iter().any(|e| e.name == name)
    }

    /// A part as text, or `None` when the document has no such part.
    fn text(&mut self, name: &str) -> Result<Option<String>, DocumentError> {
        let Some(entry) = self.dir.iter().find(|e| e.name == name) else {
            return Ok(None);
        };
        match part(&mut *self.src, entry, self.len, self.left) {
            Part::Read(bytes) => {
                self.left -= bytes.len();
                Ok(Some(decode(&bytes)))
            }
            Part::Cut(_) => Err(DocumentError::TooLarge),
            Part::Unreadable => Err(DocumentError::Unreadable),
        }
    }

    /// A part the document reads better with and reads without: the styles, the numbering,
    /// the notes. One that is missing or broken is as good as empty; one that is too large
    /// still stops the document, because the allowance is the document's and not the part's.
    fn extra(&mut self, name: &str) -> Result<String, DocumentError> {
        match self.text(name) {
            Ok(text) => Ok(text.unwrap_or_default()),
            Err(DocumentError::Unreadable) => Ok(String::new()),
            Err(e) => Err(e),
        }
    }
}

/// XML is UTF-8 unless it starts by saying otherwise, and the only otherwise a word processor
/// has ever written is UTF-16 with its byte order mark.
fn decode(bytes: &[u8]) -> String {
    match encoding_rs::Encoding::for_bom(bytes) {
        Some((encoding, _)) => encoding.decode(bytes).0.into_owned(),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

// ── the XML ────────────────────────────────────────────────────────────────────────────────

/// One piece of a document's XML.
enum Tok<'a> {
    /// `<name attrs>`, or `<name attrs/>` when `empty`.
    Open {
        name: &'a str,
        attrs: &'a str,
        empty: bool,
    },
    /// `</name>`
    Close(&'a str),
    /// What is between two tags, entities not yet read.
    Text(&'a str),
    /// A `CDATA` section, which is text with nothing in it to read.
    Raw(&'a str),
}

/// The tags and the text of an XML string, in order. Comments, processing instructions and
/// the document type are passed over; a tag that never closes ends the walk, which is how a
/// file cut short gives back the text it had.
struct Xml<'a> {
    text: &'a str,
    at: usize,
}

impl<'a> Xml<'a> {
    fn new(text: &'a str) -> Xml<'a> {
        Xml { text, at: 0 }
    }

    /// Past the next `end`, or to the end of the text when there is none.
    fn skip_past(&mut self, end: &str) {
        self.at = match self.text[self.at..].find(end) {
            Some(pos) => self.at + pos + end.len(),
            None => self.text.len(),
        };
    }
}

impl<'a> Iterator for Xml<'a> {
    type Item = Tok<'a>;

    fn next(&mut self) -> Option<Tok<'a>> {
        loop {
            let rest = &self.text[self.at..];
            if rest.is_empty() {
                return None;
            }
            if !rest.starts_with('<') {
                let end = rest.find('<').unwrap_or(rest.len());
                self.at += end;
                return Some(Tok::Text(&rest[..end]));
            }
            if rest.starts_with("<!--") {
                self.skip_past("-->");
                continue;
            }
            if let Some(body) = rest.strip_prefix("<![CDATA[") {
                let end = body.find("]]>").unwrap_or(body.len());
                self.skip_past("]]>");
                return Some(Tok::Raw(&body[..end]));
            }
            if rest.starts_with("<?") {
                self.skip_past("?>");
                continue;
            }
            if rest.starts_with("<!") {
                self.skip_past(">");
                continue;
            }
            // The end of the tag is the first `>` outside a quoted attribute value.
            let mut quote = 0u8;
            let end = rest.bytes().position(|b| {
                match (quote, b) {
                    (0, b'"') | (0, b'\'') => quote = b,
                    (q, b) if q == b => quote = 0,
                    _ => {}
                }
                quote == 0 && b == b'>'
            });
            let Some(end) = end else {
                self.at = self.text.len();
                return None;
            };
            self.at += end + 1;
            let inner = &rest[1..end];
            if let Some(name) = inner.strip_prefix('/') {
                return Some(Tok::Close(name.trim()));
            }
            let (inner, empty) = match inner.strip_suffix('/') {
                Some(inner) => (inner, true),
                None => (inner, false),
            };
            let cut = inner.find(char::is_whitespace).unwrap_or(inner.len());
            return Some(Tok::Open {
                name: &inner[..cut],
                attrs: &inner[cut..],
                empty,
            });
        }
    }
}

/// The value of one attribute of a tag.
fn attr(attrs: &str, name: &str) -> Option<String> {
    let mut rest = attrs;
    loop {
        let eq = rest.find('=')?;
        let key = rest[..eq].trim();
        let after = rest[eq + 1..].trim_start();
        let quote = after.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let close = after[1..].find(quote)?;
        if key == name {
            return Some(unescape(&after[1..1 + close]));
        }
        rest = &after[close + 2..];
    }
}

/// An attribute that is a small number.
fn attr_number(attrs: &str, name: &str) -> Option<usize> {
    attr(attrs, name)?.trim().parse().ok()
}

/// Is a switch such as `<w:b/>` on? It is unless it says it is not.
fn switched_on(attrs: &str, name: &str) -> bool {
    !matches!(
        attr(attrs, name).as_deref(),
        Some("0") | Some("false") | Some("off") | Some("none")
    )
}

/// The five entities XML has and the numbered ones. Anything else that starts with an
/// ampersand is left as it was written: an entity a document declares for itself is exactly
/// the thing that is not expanded here.
fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(amp) = rest.find('&') {
        out.push_str(&rest[..amp]);
        rest = &rest[amp..];
        let read = rest
            .find(';')
            .filter(|end| *end <= 10)
            .and_then(|end| Some((entity(&rest[1..end])?, end)));
        match read {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn entity(name: &str) -> Option<char> {
    match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => {
            let number = name.strip_prefix('#')?;
            let code = match number
                .strip_prefix('x')
                .or_else(|| number.strip_prefix('X'))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => number.parse().ok()?,
            };
            char::from_u32(code)
        }
    }
}

// ── the Markdown ───────────────────────────────────────────────────────────────────────────

/// Bold, italic, struck out: what a run of text is drawn as.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Marks {
    bold: bool,
    italic: bool,
    strike: bool,
}

/// A paragraph being read: its text, already escaped, and what it will turn out to be.
#[derive(Debug, Default)]
struct Para {
    text: String,
    /// The marks that are open at the end of `text`.
    open: Marks,
    /// A heading or a table cell, where the text is drawn one way throughout and a mark
    /// would only switch that off.
    plain: bool,
    heading: Option<usize>,
    /// Docx: the list this paragraph is an item of, and how deep.
    list: Option<(String, usize)>,
}

impl Para {
    /// Open and close marks so that what follows is drawn as `want`.
    fn mark(&mut self, want: Marks) {
        if self.plain {
            return;
        }
        if self.open.strike != want.strike {
            self.text.push_str("~~");
        }
        if self.open.bold != want.bold {
            self.text.push_str("**");
        }
        if self.open.italic != want.italic {
            self.text.push('*');
        }
        self.open = want;
    }

    /// Text of the document's own: every character that Markdown reads as a mark is escaped,
    /// and what cannot be on a line is taken off it.
    fn say(&mut self, marks: Marks, text: &str) {
        // Marks around nothing but spaces would open and close for no reader.
        if !text.trim().is_empty() {
            self.mark(marks);
        }
        for c in text.chars() {
            match c {
                '\t' => self.text.push_str("    "),
                '\n' | '\r' | '\u{2028}' | '\u{2029}' => self.text.push(' '),
                c if c.is_control() => {}
                c if c.is_ascii_punctuation() => {
                    self.text.push('\\');
                    self.text.push(c);
                }
                c => self.text.push(c),
            }
        }
    }

    /// Something of ours: a line break, the spaces of a tab, a note's number.
    fn add(&mut self, text: &str) {
        self.text.push_str(text);
    }

    /// `[image: name]`, where a picture was.
    fn image(&mut self, name: &str) {
        let marks = self.open;
        let name = name.trim();
        if name.is_empty() {
            self.say(marks, "[image]");
        } else {
            self.say(marks, &format!("[image: {name}]"));
        }
    }

    /// The finished text, with every mark closed and nothing around it.
    fn finish(mut self) -> Para {
        self.mark(Marks::default());
        self.text = self
            .text
            .trim_matches(|c: char| c.is_whitespace())
            .to_string();
        self
    }
}

/// The document being written.
#[derive(Debug, Default)]
struct Md {
    out: String,
    /// Was the last thing written a list item? Items follow each other without a blank line;
    /// everything else has one before it.
    item: bool,
    /// The notes, in the order they are referred to.
    notes: Vec<String>,
    /// Past the limit: nothing more is taken, and the document is refused at the end.
    over: bool,
}

impl Md {
    fn block(&mut self, text: &str) {
        if self.over || text.is_empty() {
            return;
        }
        if !self.out.is_empty() {
            self.out.push('\n');
        }
        self.out.push_str(text);
        self.out.push('\n');
        self.item = false;
        self.over = self.out.len() > TEXT_LIMIT;
    }

    fn heading(&mut self, level: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let text = text.replace(BREAK, " ");
        self.block(&format!("{} {text}", "#".repeat(level.clamp(1, 6))));
    }

    fn para(&mut self, text: &str) {
        self.block(text);
    }

    /// A list item `level` lists deep, numbered when it has a number.
    fn list_item(&mut self, level: usize, number: Option<usize>, text: &str) {
        if self.over || text.is_empty() {
            return;
        }
        if !self.item && !self.out.is_empty() {
            self.out.push('\n');
        }
        let pad = "  ".repeat(level.min(8));
        match number {
            Some(n) => self
                .out
                .push_str(&format!("{pad}{}. {text}\n", n % 1_000_000_000)),
            None => self.out.push_str(&format!("{pad}- {text}\n")),
        }
        self.item = true;
        self.over = self.out.len() > TEXT_LIMIT;
    }

    /// A second paragraph of the item just written: on a line of its own under the first.
    fn list_more(&mut self, text: &str) {
        if self.over || text.is_empty() {
            return;
        }
        if !self.item {
            self.block(text);
            return;
        }
        self.out.pop();
        self.out.push(BREAK);
        self.out.push_str(text);
        self.out.push('\n');
        self.over = self.out.len() > TEXT_LIMIT;
    }

    /// A table, the first row as its heading: a document says which rows are headings in
    /// more ways than are worth reading, and the first one is the heading in most of them.
    fn table(&mut self, rows: Vec<Vec<String>>) {
        let cols = rows.iter().map(Vec::len).max().unwrap_or(0);
        if cols == 0 || rows.iter().flatten().all(|c| c.is_empty()) {
            return;
        }
        let mut text = String::new();
        for (i, row) in rows.iter().enumerate() {
            text.push('|');
            for col in 0..cols {
                text.push(' ');
                text.push_str(row.get(col).map(String::as_str).unwrap_or(""));
                text.push_str(" |");
            }
            text.push('\n');
            if i == 0 {
                text.push('|');
                text.push_str(&" --- |".repeat(cols));
                text.push('\n');
            }
        }
        text.pop();
        self.block(&text);
    }

    /// The number of a new note, counted from one.
    fn note(&mut self) -> usize {
        self.notes.push(String::new());
        self.notes.len()
    }

    /// More text for note `n`.
    fn note_text(&mut self, n: usize, text: &str) {
        if let Some(note) = n.checked_sub(1).and_then(|i| self.notes.get_mut(i)) {
            if !note.is_empty() && !text.is_empty() {
                note.push(' ');
            }
            note.push_str(&text.replace(BREAK, " "));
        }
    }

    /// The document, with its notes under a rule at the end.
    fn finish(mut self) -> Result<String, DocumentError> {
        let notes = std::mem::take(&mut self.notes);
        if notes.iter().any(|n| !n.is_empty()) {
            self.block("---");
            for (i, note) in notes.iter().enumerate() {
                if !note.is_empty() {
                    self.block(&format!("\\[{}\\] {note}", i + 1));
                }
            }
        }
        if self.over {
            return Err(DocumentError::TooLarge);
        }
        Ok(self.out)
    }
}

/// More text for a table cell, where a paragraph is a few words and a line of its own for
/// each would be a table nobody could draw.
fn cell_text(cell: &mut String, text: &str) {
    if text.is_empty() {
        return;
    }
    if !cell.is_empty() {
        cell.push(' ');
    }
    cell.push_str(&text.replace(BREAK, " "));
}

/// A table being read. Only the outermost one is a table; one inside a cell gives the cell
/// its words, because a table in a table is layout and not data.
#[derive(Debug, Default)]
struct Grid {
    depth: usize,
    rows: Vec<Vec<String>>,
}

impl Grid {
    fn open(&mut self) {
        self.depth += 1;
        if self.depth == 1 {
            self.rows.clear();
        }
    }

    fn row(&mut self) {
        if self.depth == 1 {
            self.rows.push(Vec::new());
        }
    }

    fn cell(&mut self) {
        if self.depth == 1 {
            if self.rows.is_empty() {
                self.rows.push(Vec::new());
            }
            if let Some(row) = self.rows.last_mut() {
                row.push(String::new());
            }
        }
    }

    /// Text for the cell being read. `false` when no table is open, or none has a cell yet.
    fn say(&mut self, text: &str) -> bool {
        if self.depth == 0 {
            return false;
        }
        if let Some(cell) = self.rows.last_mut().and_then(|r| r.last_mut()) {
            cell_text(cell, text);
        }
        true
    }

    /// The table, when its last level closes.
    fn close(&mut self) -> Option<Vec<Vec<String>>> {
        self.depth = self.depth.saturating_sub(1);
        (self.depth == 0).then(|| std::mem::take(&mut self.rows))
    }

    /// The table a part broke off inside of, which never closed.
    fn rest(&mut self) -> Option<Vec<Vec<String>>> {
        if self.depth == 0 {
            return None;
        }
        self.depth = 1;
        self.close()
    }
}

// ── Word ───────────────────────────────────────────────────────────────────────────────────

fn docx(zip: &mut Zip, md: &mut Md) -> Result<(), DocumentError> {
    let body = zip.text("word/document.xml")?.unwrap_or_default();
    if !body.contains("<w:body") {
        return Err(DocumentError::Unreadable);
    }
    let styles = docx_headings(&zip.extra("word/styles.xml")?);
    let numbered = docx_numbering(&zip.extra("word/numbering.xml")?);
    let mut word = Word {
        styles,
        numbered,
        ..Word::default()
    };
    word.walk(&body, md, None);
    // The notes are parts of their own; only the ones the text refers to are read.
    for (file, kind) in [("word/footnotes.xml", 'f'), ("word/endnotes.xml", 'e')] {
        if word.refs.keys().any(|(k, _)| *k == kind) {
            let notes = zip.extra(file)?;
            word.walk(&notes, md, Some(kind));
        }
    }
    Ok(())
}

/// Which paragraph styles are headings, and of what level.
///
/// A style's id is whatever the program that made the document calls it — `Heading1` from an
/// English Word, `Titolo1` from an Italian one, `berschrift1` from a German one — but its
/// *name* is `heading 1` in every one of them, which is how Word itself knows.
fn docx_headings(styles: &str) -> HashMap<String, usize> {
    let mut out = HashMap::new();
    let mut current: Option<String> = None;
    for tok in Xml::new(styles) {
        match tok {
            Tok::Open { name, attrs, .. } => match name {
                "w:style" => current = attr(attrs, "w:styleId"),
                "w:name" => {
                    let level = attr(attrs, "w:val").and_then(|n| heading_name(&n));
                    if let (Some(id), Some(level)) = (&current, level) {
                        out.insert(id.clone(), level);
                    }
                }
                "w:outlineLvl" => {
                    let level = attr_number(attrs, "w:val").filter(|l| *l < 9);
                    if let (Some(id), Some(level)) = (&current, level) {
                        out.entry(id.clone()).or_insert(level + 1);
                    }
                }
                _ => {}
            },
            Tok::Close("w:style") => current = None,
            _ => {}
        }
    }
    out
}

/// `heading 2` is level two, and a title is the heading above them all.
fn heading_name(name: &str) -> Option<usize> {
    let name = name.to_ascii_lowercase();
    if name == "title" {
        return Some(1);
    }
    let level: usize = name.strip_prefix("heading")?.trim().parse().ok()?;
    (1..=9).contains(&level).then_some(level)
}

/// Which lists count: `(numId, level)` for every level that is numbered rather than
/// bulleted. A list instance (`w:num`) points at the definition (`w:abstractNum`) that says.
fn docx_numbering(numbering: &str) -> HashMap<(String, usize), bool> {
    let mut formats: HashMap<(String, usize), bool> = HashMap::new();
    let mut instances: Vec<(String, String)> = Vec::new();
    let mut definition: Option<String> = None;
    let mut instance: Option<String> = None;
    let mut level = 0usize;
    for tok in Xml::new(numbering) {
        match tok {
            Tok::Open { name, attrs, .. } => match name {
                "w:abstractNum" => definition = attr(attrs, "w:abstractNumId"),
                "w:num" => instance = attr(attrs, "w:numId"),
                "w:abstractNumId" => {
                    if let (Some(num), Some(id)) = (&instance, attr(attrs, "w:val")) {
                        instances.push((num.clone(), id));
                    }
                }
                "w:lvl" if instance.is_none() => {
                    level = attr_number(attrs, "w:ilvl").unwrap_or(0);
                }
                "w:numFmt" if instance.is_none() => {
                    if let (Some(id), Some(format)) = (&definition, attr(attrs, "w:val")) {
                        let counts = format != "bullet" && format != "none";
                        formats.insert((id.clone(), level), counts);
                    }
                }
                _ => {}
            },
            Tok::Close("w:abstractNum") => definition = None,
            Tok::Close("w:num") => instance = None,
            _ => {}
        }
    }
    let mut out = HashMap::new();
    for (num, id) in instances {
        for ((definition, level), counts) in &formats {
            if *definition == id {
                out.insert((num.clone(), *level), *counts);
            }
        }
    }
    out
}

/// The reader of `word/document.xml`, and of the notes after it.
#[derive(Debug, Default)]
struct Word {
    styles: HashMap<String, usize>,
    numbered: HashMap<(String, usize), bool>,
    /// The paragraphs that are open. More than one when a text box sits inside a paragraph.
    paras: Vec<Para>,
    grid: Grid,
    /// How far each numbered list has counted, by list and level.
    counters: HashMap<(String, usize), usize>,
    /// The notes referred to — `f` or `e` and the id — and the number each was given.
    refs: HashMap<(char, String), usize>,
    /// The note whose text is being read, in the notes' own parts.
    note: Option<usize>,
    marks: Marks,
    in_text: bool,
    in_para_props: bool,
    in_run_props: bool,
    /// The level of the list the paragraph is in, until its `w:numId` says which list.
    level: usize,
    /// The name the last drawing gave itself, for the picture inside it.
    drawing: String,
    /// How deep inside something that is not read.
    skip: usize,
}

impl Word {
    fn walk(&mut self, xml: &str, md: &mut Md, notes: Option<char>) {
        for tok in Xml::new(xml) {
            if self.skip > 0 {
                match tok {
                    Tok::Open { empty: false, .. } => self.skip += 1,
                    Tok::Close(_) => self.skip -= 1,
                    _ => {}
                }
                continue;
            }
            match tok {
                Tok::Open { name, attrs, empty } => self.open(name, attrs, empty, md, notes),
                Tok::Close(name) => self.close(name, md),
                Tok::Text(text) if self.in_text => {
                    let marks = self.marks;
                    if let Some(para) = self.paras.last_mut() {
                        para.say(marks, &unescape(text));
                    }
                }
                Tok::Raw(text) if self.in_text => {
                    let marks = self.marks;
                    if let Some(para) = self.paras.last_mut() {
                        para.say(marks, text);
                    }
                }
                _ => {}
            }
        }
        // A part that broke off in the middle of a paragraph still said what it said.
        while let Some(para) = self.paras.pop() {
            self.emit(para, md);
        }
        if let Some(rows) = self.grid.rest() {
            md.table(rows);
        }
        self.in_text = false;
        self.in_para_props = false;
        self.in_run_props = false;
        self.note = None;
        self.skip = 0;
    }

    fn open(&mut self, name: &str, attrs: &str, empty: bool, md: &mut Md, notes: Option<char>) {
        match name {
            // What a program that cannot draw the real thing is to draw instead: the same
            // words a second time.
            "mc:Fallback" if !empty => self.skip = 1,
            "w:footnote" | "w:endnote" => {
                // The rule above the notes on a page is written as a note of its own.
                let rule = attr(attrs, "w:type").is_some_and(|t| t != "normal");
                let number = notes
                    .zip(attr(attrs, "w:id"))
                    .and_then(|key| self.refs.get(&key).copied());
                match number {
                    Some(n) if !rule => self.note = Some(n),
                    _ if !empty => self.skip = 1,
                    _ => {}
                }
            }
            "w:p" if !empty => {
                self.level = 0;
                self.paras.push(Para {
                    plain: self.grid.depth > 0,
                    ..Para::default()
                });
            }
            "w:pPr" => self.in_para_props = !empty,
            "w:rPr" => self.in_run_props = !empty && !self.in_para_props,
            "w:r" => self.marks = Marks::default(),
            "w:pStyle" if self.in_para_props => {
                let id = attr(attrs, "w:val").unwrap_or_default();
                // Without a styles part the id is all there is, and an English Word's ids
                // say what they are.
                let level = self.styles.get(&id).copied().or_else(|| {
                    let digits = id.to_ascii_lowercase();
                    let digits = digits.strip_prefix("heading")?;
                    digits.parse().ok().filter(|l| (1..=9).contains(l))
                });
                self.heading(level);
            }
            "w:outlineLvl" if self.in_para_props => {
                let level = attr_number(attrs, "w:val").filter(|l| *l < 9);
                if self.paras.last().is_some_and(|p| p.heading.is_none()) {
                    self.heading(level.map(|l| l + 1));
                }
            }
            "w:ilvl" if self.in_para_props => {
                self.level = attr_number(attrs, "w:val").unwrap_or(0);
            }
            "w:numId" if self.in_para_props => {
                let id = attr(attrs, "w:val").unwrap_or_default();
                let level = self.level;
                if let Some(para) = self.paras.last_mut() {
                    // List nought is Word's way of saying "not in a list after all".
                    para.list = (id != "0" && !id.is_empty()).then_some((id, level));
                }
            }
            "w:b" if self.in_run_props => self.marks.bold = switched_on(attrs, "w:val"),
            "w:i" if self.in_run_props => self.marks.italic = switched_on(attrs, "w:val"),
            "w:strike" if self.in_run_props => self.marks.strike = switched_on(attrs, "w:val"),
            "w:t" | "m:t" => self.in_text = !empty,
            // In a paragraph's properties a `w:tab` is a tab stop, not a tab.
            "w:tab" if !self.in_para_props => self.add("    "),
            "w:br" | "w:cr" => {
                let text = if self.grid.depth > 0 { " " } else { "\u{2028}" };
                self.add(text);
            }
            "w:noBreakHyphen" => self.add("\\-"),
            "wp:docPr" => {
                self.drawing = attr(attrs, "descr")
                    .filter(|d| !d.trim().is_empty())
                    .or_else(|| attr(attrs, "name"))
                    .unwrap_or_default();
            }
            "pic:pic" => {
                let name = std::mem::take(&mut self.drawing);
                if let Some(para) = self.paras.last_mut() {
                    para.image(&name);
                }
            }
            "v:imagedata" => {
                let name = attr(attrs, "o:title").unwrap_or_default();
                if let Some(para) = self.paras.last_mut() {
                    para.image(&name);
                }
            }
            "w:footnoteReference" | "w:endnoteReference" => {
                let kind = if name == "w:footnoteReference" {
                    'f'
                } else {
                    'e'
                };
                if let Some(id) = attr(attrs, "w:id") {
                    let n = match self.refs.get(&(kind, id.clone())) {
                        Some(n) => *n,
                        None => {
                            let n = md.note();
                            self.refs.insert((kind, id), n);
                            n
                        }
                    };
                    self.add(&format!("\\[{n}\\]"));
                }
            }
            "w:tbl" if !empty => self.grid.open(),
            "w:tr" => self.grid.row(),
            "w:tc" => self.grid.cell(),
            _ => {}
        }
    }

    fn close(&mut self, name: &str, md: &mut Md) {
        match name {
            "w:t" | "m:t" => self.in_text = false,
            "w:pPr" => self.in_para_props = false,
            "w:rPr" => self.in_run_props = false,
            "w:p" => {
                if let Some(para) = self.paras.pop() {
                    self.emit(para, md);
                }
            }
            "w:tbl" => {
                if let Some(rows) = self.grid.close() {
                    md.table(rows);
                }
            }
            "w:footnote" | "w:endnote" => self.note = None,
            _ => {}
        }
    }

    fn heading(&mut self, level: Option<usize>) {
        if let Some(para) = self.paras.last_mut() {
            para.heading = level;
            para.plain = para.plain || level.is_some();
        }
    }

    fn add(&mut self, text: &str) {
        if let Some(para) = self.paras.last_mut() {
            para.add(text);
        }
    }

    /// A finished paragraph goes where it belongs: a note, a cell, or the page.
    fn emit(&mut self, para: Para, md: &mut Md) {
        let para = para.finish();
        if let Some(n) = self.note {
            md.note_text(n, &para.text);
            return;
        }
        if self.grid.say(&para.text) {
            return;
        }
        if let Some(level) = para.heading {
            md.heading(level, &para.text);
        } else if let Some((list, level)) = para.list {
            let counts = self
                .numbered
                .get(&(list.clone(), level))
                .copied()
                .unwrap_or(false);
            let number = counts.then(|| {
                // An item starts the count again for every list nested under it.
                self.counters
                    .retain(|(id, deeper), _| *id != list || *deeper <= level);
                let n = self.counters.entry((list, level)).or_insert(0);
                *n += 1;
                *n
            });
            md.list_item(level, number, &para.text);
        } else {
            md.para(&para.text);
        }
    }
}

// ── OpenDocument ───────────────────────────────────────────────────────────────────────────

fn odf(zip: &mut Zip, md: &mut Md) -> Result<(), DocumentError> {
    let content = zip.text("content.xml")?.unwrap_or_default();
    if !content.contains("<office:body") {
        return Err(DocumentError::Unreadable);
    }
    let mut open = Open::default();
    // The lists a document's styles define are in a part of their own; the ones it made up
    // as it went are at the head of the content, and are read on the way to the text.
    let styles = zip.extra("styles.xml")?;
    open.styles_only = true;
    open.walk(&styles, md);
    open.styles_only = false;
    open.walk(&content, md);
    Ok(())
}

/// A list that is open.
#[derive(Debug, Default)]
struct OpenList {
    style: String,
    count: usize,
    /// Has the item being read written its first paragraph?
    started: bool,
    /// A `text:list-header`: text in the list that is not one of its items.
    header: bool,
}

/// The reader of an OpenDocument `content.xml`: text and presentations alike, which differ
/// in what holds the paragraphs and not in the paragraphs.
#[derive(Debug, Default)]
struct Open {
    /// Reading `styles.xml`, for what it defines: its text is page headers and footers.
    styles_only: bool,
    /// Which text styles are bold, italic or struck out.
    text_styles: HashMap<String, Marks>,
    /// Which levels of which list styles are numbered.
    numbered: HashMap<(String, usize), bool>,
    style: Option<String>,
    list_style: Option<String>,
    paras: Vec<Para>,
    /// The marks of the spans that are open, the innermost last.
    spans: Vec<Marks>,
    lists: Vec<OpenList>,
    grid: Grid,
    /// The note whose body is being read.
    note: Option<usize>,
    /// The number of the note that is open, for its body.
    noted: usize,
    frame: String,
    slides: usize,
    skip: usize,
}

impl Open {
    fn walk(&mut self, xml: &str, md: &mut Md) {
        for tok in Xml::new(xml) {
            if self.skip > 0 {
                match tok {
                    Tok::Open { empty: false, .. } => self.skip += 1,
                    Tok::Close(_) => self.skip -= 1,
                    _ => {}
                }
                continue;
            }
            match tok {
                Tok::Open { name, attrs, empty } => self.open(name, attrs, empty, md),
                Tok::Close(name) => self.close(name, md),
                Tok::Text(text) => self.say(&unescape(text)),
                Tok::Raw(text) => self.say(text),
            }
        }
        while let Some(para) = self.paras.pop() {
            self.emit(para, md);
        }
        if let Some(rows) = self.grid.rest() {
            md.table(rows);
        }
        self.spans.clear();
        self.lists.clear();
        self.style = None;
        self.list_style = None;
        self.note = None;
        self.skip = 0;
    }

    fn say(&mut self, text: &str) {
        let marks = self.spans.last().copied().unwrap_or_default();
        if let Some(para) = self.paras.last_mut() {
            para.say(marks, text);
        }
    }

    fn open(&mut self, name: &str, attrs: &str, empty: bool, md: &mut Md) {
        match name {
            "style:style" => self.style = attr(attrs, "style:name").filter(|_| !empty),
            "style:text-properties" => {
                let marks = Marks {
                    bold: attr(attrs, "fo:font-weight").is_some_and(|w| w == "bold"),
                    italic: attr(attrs, "fo:font-style").is_some_and(|s| s == "italic"),
                    strike: attr(attrs, "style:text-line-through-style")
                        .is_some_and(|s| s != "none"),
                };
                if let Some(style) = self.style.clone().filter(|_| marks != Marks::default()) {
                    self.text_styles.insert(style, marks);
                }
            }
            "text:list-style" => self.list_style = attr(attrs, "style:name").filter(|_| !empty),
            "text:list-level-style-number" => {
                // A level with no number format is numbered in name only: it is how a
                // document's outline is hung on a list without a number being drawn.
                let drawn = attr(attrs, "style:num-format").is_some_and(|f| !f.is_empty());
                let level = attr_number(attrs, "text:level").unwrap_or(1);
                if let Some(style) = self.list_style.clone().filter(|_| drawn) {
                    self.numbered.insert((style, level), true);
                }
            }
            _ if self.styles_only => {}
            // Text that is in the file and not in the document: what was deleted while
            // changes were tracked, a reviewer's comments, a slide's speaker notes, the
            // label of a note (its number is ours) and the descriptions of a drawing.
            "text:tracked-changes"
            | "office:annotation"
            | "presentation:notes"
            | "text:note-citation"
            | "svg:title"
            | "svg:desc"
                if !empty =>
            {
                self.skip = 1;
            }
            "text:h" | "text:p" if !empty => {
                let heading = (name == "text:h")
                    .then(|| attr_number(attrs, "text:outline-level").unwrap_or(1));
                self.paras.push(Para {
                    plain: heading.is_some() || self.grid.depth > 0,
                    heading,
                    ..Para::default()
                });
            }
            "text:span" if !empty => {
                let mut marks = self.spans.last().copied().unwrap_or_default();
                let own = attr(attrs, "text:style-name").and_then(|s| self.text_styles.get(&s));
                if let Some(own) = own {
                    marks.bold |= own.bold;
                    marks.italic |= own.italic;
                    marks.strike |= own.strike;
                }
                self.spans.push(marks);
            }
            "text:s" => {
                // As many spaces as it says, within reason: the count is the file's.
                let count = attr_number(attrs, "text:c").unwrap_or(1).clamp(1, 16);
                self.add(&" ".repeat(count));
            }
            "text:tab" => self.add("    "),
            "text:line-break" => {
                let text = if self.grid.depth > 0 { " " } else { "\u{2028}" };
                self.add(text);
            }
            "text:note" if !empty => {
                self.noted = md.note();
                self.add(&format!("\\[{}\\]", self.noted));
            }
            "text:note-body" if !empty => self.note = Some(self.noted),
            "text:list" if !empty => {
                let style = attr(attrs, "text:style-name")
                    .or_else(|| self.lists.last().map(|l| l.style.clone()))
                    .unwrap_or_default();
                self.lists.push(OpenList {
                    style,
                    ..OpenList::default()
                });
            }
            "text:list-item" | "text:list-header" => {
                if let Some(list) = self.lists.last_mut() {
                    list.started = false;
                    list.header = name == "text:list-header";
                }
            }
            "table:table" if !empty => self.grid.open(),
            "table:table-row" => self.grid.row(),
            "table:table-cell" | "table:covered-table-cell" => self.grid.cell(),
            "draw:frame" => self.frame = attr(attrs, "draw:name").unwrap_or_default(),
            "draw:image" => {
                let name = std::mem::take(&mut self.frame);
                match self.paras.last_mut() {
                    Some(para) => para.image(&name),
                    // A picture on a slide stands on the page, in no paragraph.
                    None => {
                        let mut para = Para::default();
                        para.image(&name);
                        self.emit(para, md);
                    }
                }
            }
            "draw:page" => {
                self.slides += 1;
                md.heading(2, &format!("Slide {}", self.slides));
            }
            _ => {}
        }
    }

    fn close(&mut self, name: &str, md: &mut Md) {
        match name {
            "style:style" => self.style = None,
            "text:list-style" => self.list_style = None,
            _ if self.styles_only => {}
            "text:h" | "text:p" => {
                if let Some(para) = self.paras.pop() {
                    self.emit(para, md);
                }
            }
            "text:span" => {
                self.spans.pop();
            }
            "text:note-body" => self.note = None,
            "text:list" => {
                self.lists.pop();
            }
            "table:table" => {
                if let Some(rows) = self.grid.close() {
                    md.table(rows);
                }
            }
            _ => {}
        }
    }

    fn add(&mut self, text: &str) {
        if let Some(para) = self.paras.last_mut() {
            para.add(text);
        }
    }

    fn emit(&mut self, para: Para, md: &mut Md) {
        let para = para.finish();
        if let Some(n) = self.note {
            md.note_text(n, &para.text);
            return;
        }
        if self.grid.say(&para.text) || para.text.is_empty() {
            return;
        }
        if let Some(level) = para.heading {
            md.heading(level, &para.text);
            return;
        }
        let level = self.lists.len().saturating_sub(1);
        let Some(list) = self.lists.last_mut() else {
            md.para(&para.text);
            return;
        };
        if list.header {
            md.para(&para.text);
        } else if list.started {
            md.list_more(&para.text);
        } else {
            list.started = true;
            let counts = self
                .numbered
                .get(&(list.style.clone(), level + 1))
                .copied()
                .unwrap_or(false);
            let number = counts.then(|| {
                list.count += 1;
                list.count
            });
            md.list_item(level, number, &para.text);
        }
    }
}

// ── PowerPoint ─────────────────────────────────────────────────────────────────────────────

fn pptx(zip: &mut Zip, md: &mut Md) -> Result<(), DocumentError> {
    let slides = pptx_slides(zip)?;
    if slides.is_empty() {
        return Err(DocumentError::Unreadable);
    }
    if slides.len() > SLIDES {
        return Err(DocumentError::TooLarge);
    }
    for (i, name) in slides.iter().enumerate() {
        let xml = zip.extra(name)?;
        md.heading(2, &format!("Slide {}", i + 1));
        Slide::default().walk(&xml, md);
    }
    Ok(())
}

/// The slides' parts in the order they are shown, which is the presentation's to say: the
/// numbers in the file names are the order the slides were *made* in.
fn pptx_slides(zip: &mut Zip) -> Result<Vec<String>, DocumentError> {
    let mut targets: HashMap<String, String> = HashMap::new();
    let rels = zip.extra("ppt/_rels/presentation.xml.rels")?;
    for tok in Xml::new(&rels) {
        if let Tok::Open {
            name: "Relationship",
            attrs,
            ..
        } = tok
        {
            if let (Some(id), Some(target)) = (attr(attrs, "Id"), attr(attrs, "Target")) {
                let path = match target.strip_prefix('/') {
                    Some(absolute) => absolute.to_string(),
                    None => format!("ppt/{target}"),
                };
                targets.insert(id, path);
            }
        }
    }
    let mut out = Vec::new();
    let presentation = zip.extra("ppt/presentation.xml")?;
    for tok in Xml::new(&presentation) {
        if let Tok::Open {
            name: "p:sldId",
            attrs,
            ..
        } = tok
        {
            let path = attr(attrs, "r:id").and_then(|id| targets.get(&id).cloned());
            if let Some(path) = path.filter(|p| zip.has(p)) {
                out.push(path);
            }
        }
    }
    if out.is_empty() {
        // A presentation that does not say: the parts that are there, by their number.
        let mut found: Vec<(usize, String)> = zip
            .dir
            .iter()
            .filter_map(|e| {
                let number = e
                    .name
                    .strip_prefix("ppt/slides/slide")?
                    .strip_suffix(".xml")?;
                Some((number.parse().ok()?, e.name.clone()))
            })
            .collect();
        found.sort();
        out = found.into_iter().map(|(_, name)| name).collect();
    }
    Ok(out)
}

/// What a shape on a slide holds, which is what its paragraphs are drawn as.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Shape {
    /// The slide's title.
    Title,
    /// The body: an outline, a bullet for each paragraph.
    Body,
    /// A text box: paragraphs.
    #[default]
    Other,
    /// The date, the footer, the slide number: the same on every slide, and not its text.
    Furniture,
}

/// The reader of one slide.
#[derive(Debug, Default)]
struct Slide {
    shape: Shape,
    para: Option<Para>,
    grid: Grid,
    marks: Marks,
    in_text: bool,
    level: usize,
    bullets: bool,
    numbered: bool,
    /// How far the numbered paragraphs of this shape have counted, by level.
    counts: Vec<usize>,
    /// The next `p:cNvPr` names a picture.
    picture: bool,
}

impl Slide {
    fn walk(mut self, xml: &str, md: &mut Md) {
        for tok in Xml::new(xml) {
            match tok {
                Tok::Open { name, attrs, empty } => self.open(name, attrs, empty, md),
                Tok::Close(name) => self.close(name, md),
                Tok::Text(text) if self.in_text => {
                    let marks = self.marks;
                    if let Some(para) = &mut self.para {
                        para.say(marks, &unescape(text));
                    }
                }
                _ => {}
            }
        }
        if let Some(para) = self.para.take() {
            self.emit(para, md);
        }
        if let Some(rows) = self.grid.rest() {
            md.table(rows);
        }
    }

    fn open(&mut self, name: &str, attrs: &str, empty: bool, md: &mut Md) {
        match name {
            "p:sp" => {
                self.shape = Shape::Other;
                self.counts.clear();
            }
            "p:ph" => {
                self.shape = match attr(attrs, "type").as_deref() {
                    Some("title") | Some("ctrTitle") => Shape::Title,
                    Some("dt") | Some("ftr") | Some("sldNum") | Some("hdr") => Shape::Furniture,
                    Some("subTitle") => Shape::Other,
                    // A placeholder that does not say what it is for is the body.
                    Some("body") | Some("obj") | None => Shape::Body,
                    Some(_) => Shape::Other,
                };
            }
            "p:pic" => self.picture = true,
            "p:cNvPr" if self.picture => {
                self.picture = false;
                let mut para = Para::default();
                let name = attr(attrs, "descr")
                    .filter(|d| !d.trim().is_empty())
                    .or_else(|| attr(attrs, "name"))
                    .unwrap_or_default();
                para.image(&name);
                md.para(&para.finish().text);
            }
            "a:p" if !empty => {
                self.level = 0;
                self.bullets = self.shape == Shape::Body && self.grid.depth == 0;
                self.numbered = false;
                self.para = Some(Para {
                    plain: self.shape == Shape::Title || self.grid.depth > 0,
                    ..Para::default()
                });
            }
            "a:pPr" => self.level = attr_number(attrs, "lvl").unwrap_or(0),
            "a:buNone" => self.bullets = false,
            "a:buAutoNum" => {
                self.bullets = true;
                self.numbered = true;
            }
            "a:r" => self.marks = Marks::default(),
            "a:rPr" => {
                self.marks = Marks {
                    bold: attr(attrs, "b").is_some_and(|b| b == "1" || b == "true"),
                    italic: attr(attrs, "i").is_some_and(|i| i == "1" || i == "true"),
                    strike: attr(attrs, "strike").is_some_and(|s| s != "noStrike"),
                };
            }
            "a:t" => self.in_text = !empty,
            "a:br" => {
                let text = if self.grid.depth > 0 { " " } else { "\u{2028}" };
                if let Some(para) = &mut self.para {
                    para.add(text);
                }
            }
            "a:tbl" if !empty => self.grid.open(),
            "a:tr" => self.grid.row(),
            "a:tc" => self.grid.cell(),
            _ => {}
        }
    }

    fn close(&mut self, name: &str, md: &mut Md) {
        match name {
            "a:t" => self.in_text = false,
            "a:p" => {
                if let Some(para) = self.para.take() {
                    self.emit(para, md);
                }
            }
            "a:tbl" => {
                if let Some(rows) = self.grid.close() {
                    md.table(rows);
                }
            }
            _ => {}
        }
    }

    fn emit(&mut self, para: Para, md: &mut Md) {
        let para = para.finish();
        if self.grid.say(&para.text) || self.shape == Shape::Furniture {
            return;
        }
        match self.shape {
            Shape::Title => md.heading(3, &para.text),
            _ if self.bullets => {
                let level = self.level.min(8);
                let number = (self.numbered && !para.text.is_empty()).then(|| {
                    self.counts.resize(level + 1, 0);
                    self.counts[level] += 1;
                    self.counts[level]
                });
                md.list_item(level, number, &para.text);
            }
            _ => md.para(&para.text),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// A zip of these parts, written by the crate everything else here writes zips with.
    fn zip(parts: &[(&str, &str)]) -> Vec<u8> {
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, text) in parts {
            zip.start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(text.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    fn markdown(parts: &[(&str, &str)]) -> Result<String, DocumentError> {
        let bytes = zip(parts);
        to_markdown(&mut &bytes[..])
    }

    /// The document as the viewer draws it, eighty columns wide.
    fn shown(parts: &[(&str, &str)]) -> Vec<String> {
        let text = markdown(parts).expect("a document");
        crate::markdown::render(&text, 80)
            .into_iter()
            .map(|l| l.text)
            .collect()
    }

    fn body(inner: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="w"><w:body>{inner}</w:body></w:document>"#
        )
    }

    const STYLES: &str = r#"<w:styles>
        <w:style w:type="paragraph" w:styleId="Titolo1"><w:name w:val="heading 1"/></w:style>
        <w:style w:type="paragraph" w:styleId="Titolo2"><w:name w:val="heading 2"/></w:style>
        <w:style w:type="paragraph" w:styleId="Normale"><w:name w:val="Normal"/></w:style>
        </w:styles>"#;

    const NUMBERING: &str = r#"<w:numbering>
        <w:abstractNum w:abstractNumId="0"><w:lvl w:ilvl="0"><w:numFmt w:val="bullet"/></w:lvl>
          <w:lvl w:ilvl="1"><w:numFmt w:val="bullet"/></w:lvl></w:abstractNum>
        <w:abstractNum w:abstractNumId="1"><w:lvl w:ilvl="0"><w:numFmt w:val="decimal"/></w:lvl></w:abstractNum>
        <w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>
        <w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>
        </w:numbering>"#;

    fn item(list: u32, level: u32, text: &str) -> String {
        format!(
            r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="{level}"/><w:numId w:val="{list}"/></w:numPr></w:pPr><w:r><w:t>{text}</w:t></w:r></w:p>"#
        )
    }

    #[test]
    fn a_docx_shows_its_headings_and_its_paragraphs() {
        let xml = body(
            r#"<w:p><w:pPr><w:pStyle w:val="Titolo1"/></w:pPr><w:r><w:t>Relazione</w:t></w:r></w:p>
            <w:p><w:r><w:t xml:space="preserve">Il primo </w:t></w:r><w:r><w:rPr><w:b/></w:rPr><w:t>paragrafo</w:t></w:r><w:r><w:t>, con R&amp;D.</w:t></w:r></w:p>
            <w:p><w:pPr><w:pStyle w:val="Titolo2"/></w:pPr><w:r><w:t>Dettagli</w:t></w:r></w:p>
            <w:p/>"#,
        );
        let lines = shown(&[("word/document.xml", &xml), ("word/styles.xml", STYLES)]);
        assert_eq!(
            lines,
            [
                "Relazione",
                "═════════",
                "",
                "Il primo paragrafo, con R&D.",
                "",
                "Dettagli",
                "────────",
            ]
        );
    }

    #[test]
    fn a_heading_is_known_by_its_id_when_there_are_no_styles() {
        let xml = body(
            r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>Two</w:t></w:r></w:p>
            <w:p><w:pPr><w:outlineLvl w:val="0"/></w:pPr><w:r><w:t>One</w:t></w:r></w:p>"#,
        );
        let text = markdown(&[("word/document.xml", &xml)]).unwrap();
        assert_eq!(text, "## Two\n\n# One\n");
    }

    #[test]
    fn the_marks_a_document_could_be_taken_for_stay_text() {
        let xml = body(
            r#"<w:p><w:r><w:t># not a heading, *not* emphasis, a|b, 1. no list, `x` and [y](z)</w:t></w:r></w:p>
            <w:p><w:r><w:t>- nor this</w:t></w:r></w:p>"#,
        );
        let lines = shown(&[("word/document.xml", &xml)]);
        assert_eq!(
            lines,
            [
                "# not a heading, *not* emphasis, a|b, 1. no list, `x` and [y](z)",
                "",
                "- nor this",
            ]
        );
    }

    #[test]
    fn bulleted_and_numbered_lists_are_lists() {
        let xml = body(&format!(
            "{}{}{}{}{}{}",
            item(1, 0, "apples"),
            item(1, 1, "green ones"),
            item(1, 0, "pears"),
            r#"<w:p><w:r><w:t>Then:</w:t></w:r></w:p>"#,
            item(2, 0, "wash"),
            item(2, 0, "eat"),
        ));
        let lines = shown(&[
            ("word/document.xml", &xml),
            ("word/numbering.xml", NUMBERING),
        ]);
        assert_eq!(
            lines,
            [
                "• apples",
                "  ◦ green ones",
                "• pears",
                "",
                "Then:",
                "",
                "1. wash",
                "2. eat",
            ]
        );
    }

    #[test]
    fn a_table_is_drawn_as_a_table() {
        let cell = |t: &str| format!("<w:tc><w:p><w:r><w:t>{t}</w:t></w:r></w:p></w:tc>");
        let xml = body(&format!(
            "<w:tbl><w:tr>{}{}</w:tr><w:tr>{}{}</w:tr></w:tbl>",
            cell("name"),
            cell("size"),
            cell("a|b"),
            cell("12"),
        ));
        let lines = shown(&[("word/document.xml", &xml)]);
        assert_eq!(lines.len(), 5, "{lines:?}");
        assert!(
            lines[0].starts_with('┌') || lines[0].starts_with('╭'),
            "{lines:?}"
        );
        assert!(lines[1].contains("name") && lines[1].contains("size"));
        assert!(lines[3].contains("a|b") && lines[3].contains("12"));
    }

    #[test]
    fn breaks_tabs_pictures_and_notes_are_shown() {
        let xml = body(
            r#"<w:p><w:r><w:t>one</w:t><w:br/><w:t>two</w:t><w:tab/><w:t>three</w:t></w:r>
            <w:r><w:footnoteReference w:id="2"/></w:r></w:p>
            <w:p><w:r><w:drawing><wp:inline><wp:docPr id="1" name="Picture 1"/><a:graphic><pic:pic/></a:graphic></wp:inline></w:drawing></w:r></w:p>
            <w:p><w:r><mc:AlternateContent><mc:Choice><w:t>once</w:t></mc:Choice><mc:Fallback><w:t>twice</w:t></mc:Fallback></mc:AlternateContent></w:r></w:p>"#,
        );
        let notes = r#"<w:footnotes><w:footnote w:type="separator" w:id="0"><w:p><w:r><w:t>rule</w:t></w:r></w:p></w:footnote>
            <w:footnote w:id="2"><w:p><w:r><w:t>See the appendix.</w:t></w:r></w:p></w:footnote></w:footnotes>"#;
        let lines = shown(&[("word/document.xml", &xml), ("word/footnotes.xml", notes)]);
        assert_eq!(
            lines[..7],
            [
                "one",
                "two    three[1]",
                "",
                "[image: Picture 1]",
                "",
                "once",
                "",
            ]
        );
        assert_eq!(lines.last().unwrap(), "[1] See the appendix.");
        assert!(!lines
            .iter()
            .any(|l| l.contains("rule") || l.contains("twice")));
    }

    #[test]
    fn an_odt_shows_headings_lists_a_table_and_a_note() {
        let xml = r#"<office:document-content><office:automatic-styles>
            <style:style style:name="T1" style:family="text"><style:text-properties fo:font-weight="bold"/></style:style>
            <text:list-style style:name="L1"><text:list-level-style-number text:level="1" style:num-format="1"/></text:list-style>
            <text:list-style style:name="L2"><text:list-level-style-bullet text:level="1" text:bullet-char="•"/></text:list-style>
            </office:automatic-styles><office:body><office:text>
            <text:tracked-changes><text:changed-region><text:deletion><text:p>deleted</text:p></text:deletion></text:changed-region></text:tracked-changes>
            <text:h text:outline-level="1">Titolo</text:h>
            <text:p>Uno<text:s text:c="2"/>due<text:tab/>tre<text:line-break/>quattro <text:span text:style-name="T1">forte</text:span><text:note text:note-class="footnote"><text:note-citation>1</text:note-citation><text:note-body><text:p>La nota.</text:p></text:note-body></text:note></text:p>
            <text:list text:style-name="L1"><text:list-item><text:p>primo</text:p><text:p>ancora</text:p></text:list-item><text:list-item><text:p>secondo</text:p>
              <text:list><text:list-item><text:p>dentro</text:p></text:list-item></text:list></text:list-item></text:list>
            <text:list text:style-name="L2"><text:list-item><text:p>punto</text:p></text:list-item></text:list>
            <table:table><table:table-row><table:table-cell><text:p>a</text:p></table:table-cell><table:table-cell><text:p>b</text:p></table:table-cell></table:table-row>
            <table:table-row><table:table-cell><text:p>1</text:p></table:table-cell><table:table-cell><text:p>2</text:p></table:table-cell></table:table-row></table:table>
            <text:p><draw:frame draw:name="Image1"><draw:image xlink:href="Pictures/1.png"/><svg:title>alt</svg:title></draw:frame></text:p>
            </office:text></office:body></office:document-content>"#;
        let parts = [
            ("mimetype", "application/vnd.oasis.opendocument.text"),
            ("content.xml", xml),
        ];
        let lines = shown(&parts);
        assert_eq!(
            lines[..12],
            [
                "Titolo",
                "══════",
                "",
                "Uno  due    tre",
                "quattro forte[1]",
                "",
                "1. primo",
                "   ancora",
                "2. secondo",
                "  ◦ dentro",
                "• punto",
                "",
            ],
            "{lines:?}"
        );
        assert!(lines
            .iter()
            .any(|l| l.contains('a') && l.contains('b') && l.contains('│')));
        assert!(lines.contains(&"[image: Image1]".to_string()));
        assert_eq!(lines.last().unwrap(), "[1] La nota.");
        assert!(!lines
            .iter()
            .any(|l| l.contains("deleted") || l.contains("alt")));
    }

    #[test]
    fn a_presentation_shows_the_text_of_each_slide_in_the_order_it_is_shown() {
        let slide = |title: &str, points: &[&str]| {
            let points: String = points
                .iter()
                .map(|p| format!("<a:p><a:r><a:t>{p}</a:t></a:r></a:p>"))
                .collect();
            format!(
                r#"<p:sld><p:cSld><p:spTree>
                <p:sp><p:nvSpPr><p:nvPr><p:ph type="title"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:r><a:t>{title}</a:t></a:r></a:p></p:txBody></p:sp>
                <p:sp><p:nvSpPr><p:nvPr><p:ph idx="1"/></p:nvPr></p:nvSpPr><p:txBody>{points}</p:txBody></p:sp>
                <p:sp><p:nvSpPr><p:nvPr><p:ph type="sldNum"/></p:nvPr></p:nvSpPr><p:txBody><a:p><a:fld><a:t>7</a:t></a:fld></a:p></p:txBody></p:sp>
                </p:spTree></p:cSld></p:sld>"#
            )
        };
        let first = slide("Opening", &["why", "how"]);
        let second = slide("Closing", &["thanks"]);
        // The second slide made is the first one shown.
        let rels = r#"<Relationships><Relationship Id="rId1" Target="slides/slide1.xml"/><Relationship Id="rId2" Target="slides/slide2.xml"/></Relationships>"#;
        let presentation = r#"<p:presentation><p:sldIdLst><p:sldId id="256" r:id="rId2"/><p:sldId id="257" r:id="rId1"/></p:sldIdLst></p:presentation>"#;
        let lines = shown(&[
            ("ppt/presentation.xml", presentation),
            ("ppt/_rels/presentation.xml.rels", rels),
            ("ppt/slides/slide1.xml", &second),
            ("ppt/slides/slide2.xml", &first),
        ]);
        assert_eq!(
            lines,
            [
                "Slide 1",
                "───────",
                "",
                "Opening",
                "",
                "• why",
                "• how",
                "",
                "Slide 2",
                "───────",
                "",
                "Closing",
                "",
                "• thanks",
            ]
        );
    }

    #[test]
    fn an_odp_is_read_slide_by_slide() {
        let xml = r#"<office:document-content><office:body><office:presentation>
            <draw:page draw:name="page1"><draw:frame><draw:text-box><text:p>Hello</text:p></draw:text-box></draw:frame>
              <presentation:notes><draw:frame><draw:text-box><text:p>say hello</text:p></draw:text-box></draw:frame></presentation:notes></draw:page>
            <draw:page draw:name="page2"><draw:frame draw:name="Logo"><draw:image xlink:href="Pictures/l.png"/></draw:frame></draw:page>
            </office:presentation></office:body></office:document-content>"#;
        let text = markdown(&[("content.xml", xml)]).unwrap();
        assert_eq!(
            text,
            "## Slide 1\n\nHello\n\n## Slide 2\n\n\\[image\\: Logo\\]\n"
        );
    }

    #[test]
    fn a_malformed_document_is_refused_or_read_as_far_as_it_goes() {
        // Not a zip at all, and a zip that is not a document.
        assert_eq!(
            to_markdown(&mut &b"PK\x03\x04 and then nothing"[..]),
            Err(DocumentError::Unreadable)
        );
        assert_eq!(
            to_markdown(&mut &b"plain text"[..]),
            Err(DocumentError::Unreadable)
        );
        assert_eq!(
            markdown(&[("a.txt", "hello")]),
            Err(DocumentError::Unreadable)
        );
        // The right part with something else in it.
        assert_eq!(
            markdown(&[("word/document.xml", "\u{0}\u{1} < not > xml << at all")]),
            Err(DocumentError::Unreadable)
        );
        // XML that breaks off: what was there before the break is the document.
        let cut = r#"<w:document><w:body><w:p><w:r><w:t>whole</w:t></w:r></w:p><w:p><w:r><w:t>half a para"#;
        assert_eq!(
            markdown(&[("word/document.xml", cut)]).unwrap(),
            "whole\n\nhalf a para\n"
        );
        // Tags closed that were never opened, and attributes with no end.
        let wild = r#"<w:document><w:body></w:p></w:tbl></w:tc><w:p><w:pPr><w:pStyle w:val="x></w:pPr><w:r><w:t>still here</w:t></w:r></w:p></w:body>"#;
        assert!(markdown(&[("word/document.xml", wild)]).is_ok());
        // A zip cut short in the middle of the part.
        let whole = zip(&[(
            "word/document.xml",
            &body("<w:p><w:r><w:t>x</w:t></w:r></w:p>"),
        )]);
        for cut in [10, 40, whole.len() - 30, whole.len() - 1] {
            let _ = to_markdown(&mut &whole[..cut]);
        }
    }

    #[test]
    fn a_part_that_inflates_past_the_limit_is_refused_and_not_inflated() {
        // A few kilobytes in the file, far more than the allowance out of it: the classic.
        let para = "<w:p><w:r><w:t>          </w:t></w:r></w:p>";
        let bomb = body(&para.repeat(XML_LIMIT / para.len() + 16));
        let bytes = zip(&[("word/document.xml", &bomb)]);
        assert!(bytes.len() < XML_LIMIT / 50, "{} bytes", bytes.len());
        assert_eq!(to_markdown(&mut &bytes[..]), Err(DocumentError::TooLarge));

        // The allowance is the document's: two parts that each fit are still too much.
        let half = body(&para.repeat(XML_LIMIT * 2 / 3 / para.len()));
        let styles = format!("<w:styles>{}</w:styles>", " ".repeat(XML_LIMIT * 2 / 3));
        assert_eq!(
            markdown(&[("word/document.xml", &half), ("word/styles.xml", &styles)]),
            Err(DocumentError::TooLarge)
        );
    }

    #[test]
    fn text_past_the_limit_is_refused() {
        // Under the XML limit and over the text limit: short tags around long words.
        let word = "x".repeat(4000);
        let para = format!("<w:p><w:r><w:t>{word}</w:t></w:r></w:p>");
        let xml = body(&para.repeat(TEXT_LIMIT / 4000 + 2));
        assert!(xml.len() < XML_LIMIT);
        assert_eq!(
            markdown(&[("word/document.xml", &xml)]),
            Err(DocumentError::TooLarge)
        );
    }

    #[test]
    fn nesting_a_million_deep_costs_no_stack() {
        let deep = format!(
            "<w:document><w:body>{}<w:p><w:r><w:t>bottom</w:t></w:r></w:p>",
            "<w:tbl><w:tr><w:tc>".repeat(200_000)
        );
        let text = markdown(&[("word/document.xml", &deep)]).unwrap();
        assert!(text.contains("bottom"), "{}", &text[..text.len().min(80)]);
        let spans = format!(
            "<office:document-content><office:body><text:p>{}deep</text:p>",
            "<text:span>".repeat(500_000)
        );
        assert_eq!(markdown(&[("content.xml", &spans)]).unwrap(), "deep\n");
    }

    #[test]
    fn an_entity_a_document_defines_is_not_expanded() {
        let xml = r#"<?xml version="1.0"?><!DOCTYPE d [<!ENTITY a "aaaaaaaaaa"><!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;">]>
            <w:document><w:body><w:p><w:r><w:t>&b;&b; &#65;&#x42;&lt;</w:t></w:r></w:p></w:body></w:document>"#;
        let lines = shown(&[("word/document.xml", xml)]);
        assert_eq!(lines.last().unwrap(), "&b;&b; AB<");
    }

    #[test]
    fn the_names_that_open_as_documents() {
        for name in ["a.docx", "A.DOCX", "b.odt", "c.pptx", "d.odp", "e.docm"] {
            assert!(is_document(name), "{name}");
        }
        for name in ["a.doc", "b.xlsx", "c.ods", "d.txt", "docx"] {
            assert!(!is_document(name), "{name}");
        }
    }
}
