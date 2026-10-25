//! Markdown laid out to be read, rather than coloured to be edited.
//!
//! The highlighter in `fcmd-syntax` paints the marks: a heading keeps its `#`, a link keeps
//! its brackets, a table keeps its pipes. That is right for the editor, where the marks are
//! what you are typing. For reading it is the wrong thing, so this module throws the marks
//! away and uses the space they held: a heading is bold with a rule under it, a list is
//! bullets and hanging indents, a paragraph is wrapped to the window, and a table is drawn
//! with box-drawing characters and its columns lined up.
//!
//! It is a renderer, not a parser with an AST. The document is walked a block at a time and
//! each block emits its lines, which is all a reader needs and keeps the whole thing in one
//! pass. The subset is CommonMark plus the GitHub extensions people actually write: ATX and
//! setext headings, thematic breaks, fenced and indented code, blockquotes, bulleted and
//! numbered lists, task lists, tables, and the inline marks (emphasis, strong, code, strike,
//! links, images, autolinks).
//!
//! Every rendered line remembers the byte offset of the source line it came from, and its
//! index among the lines that offset produced. That is the same `(offset, sub)` pair the
//! viewer already uses for wrapped text, so scrolling, `Home`, `End` and switching back to
//! the source all work without a line index and without a second notion of position.

use unicode_width::UnicodeWidthStr;

/// What a run of a rendered line is, so the screen can colour it.
///
/// The keys are the syntax theme's, not a set of its own: a reader who has chosen a theme has
/// already said what a heading and a string should look like, and a Markdown view that
/// invented its own colours would be the one thing on screen that ignored them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// A heading's text, and the rule under it.
    Heading,
    /// `**strong**` — bold, and drawn in the ordinary colour.
    Strong,
    /// `*emphasis*`, which a terminal cell cannot slant, so it changes colour instead.
    Emphasis,
    /// `~~struck out~~`, drawn in the colour comments get: present, and clearly not current.
    Strike,
    /// A code span or a line of a code block with no language we recognise.
    Code,
    /// A link's or an image's text.
    Link,
    /// The URL after it.
    Url,
    /// A bullet, a number, a checkbox, a quote bar — everything the renderer adds.
    Marker,
    /// A thematic break, and the frame of a table.
    Frame,
    /// Ordinary text.
    Text,
    /// A line inside a fenced block whose language we know: the fence said `rust`, so the
    /// code inside it is coloured as Rust is.
    Syntax(fcmd_syntax::Class),
}

impl Class {
    /// The theme key this class is drawn with.
    pub fn theme_key(self) -> &'static str {
        match self {
            Class::Heading => "syntax.preproc",
            Class::Strong | Class::Text => "syntax.text",
            Class::Emphasis => "syntax.type",
            Class::Strike | Class::Url | Class::Frame => "syntax.comment",
            Class::Code => "syntax.string",
            Class::Link => "syntax.function",
            Class::Marker => "syntax.keyword",
            Class::Syntax(class) => class.theme_key(),
        }
    }

    /// Is this run drawn bold?
    fn bold(self) -> bool {
        matches!(self, Class::Heading | Class::Strong)
    }
}

/// A run of one class on a rendered line, as byte offsets into its text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub class: Class,
    pub bold: bool,
}

/// One rendered line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub spans: Vec<Span>,
    /// Byte offset of the source line this came from.
    pub source: u64,
    /// Which of that offset's rendered lines this is.
    pub sub: usize,
}

/// One character, with what it is to be drawn as. Inline parsing produces these, and the
/// wrapper turns runs of them back into lines with spans: doing it the other way round means
/// splitting spans at wrap points, which is the same work with more ways to get it wrong.
/// They never outlive one block, so a long document costs no more than its longest paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Atom {
    ch: char,
    class: Class,
    bold: bool,
}

impl Atom {
    fn new(ch: char, class: Class, bold: bool) -> Atom {
        Atom { ch, class, bold }
    }
}

/// Where the rendered lines are collected, keeping the `(source, sub)` pair in step.
#[derive(Debug, Default)]
struct Out {
    lines: Vec<Line>,
    source: u64,
    sub: usize,
}

impl Out {
    /// Everything pushed from here on comes from this source offset.
    ///
    /// The index only restarts when the offset actually changes, because `(source, sub)` is
    /// the viewer's notion of position: two lines that shared a pair would be the same place,
    /// and scrolling to the end of a table would land on the row above its last rule.
    fn at(&mut self, source: u64) {
        if source != self.source {
            self.source = source;
            self.sub = 0;
        }
    }

    fn push(&mut self, text: String, spans: Vec<Span>) {
        self.lines.push(Line {
            text,
            spans,
            source: self.source,
            sub: self.sub,
        });
        self.sub += 1;
    }

    fn blank(&mut self) {
        // One blank line between blocks, never two, and never one before the first block: a
        // document written with generous spacing and one written tightly should read the same
        // once the marks are gone.
        match self.lines.last() {
            None => return,
            Some(last) if last.text.trim().is_empty() => return,
            Some(_) => {}
        }
        self.push(String::new(), Vec::new());
    }
}

/// Render `text` as Markdown, wrapped to `width` columns.
///
/// `width` is the window's, so the result is cached against it and rendered again when the
/// window changes size — a table's columns and a paragraph's wrapping both depend on it.
pub fn render(text: &str, width: usize) -> Vec<Line> {
    let width = width.clamp(8, 1024);
    let lines = split(text);
    let mut out = Out::default();
    blocks(&lines, width, "", &mut out);
    // A document that rendered to nothing still has to answer `rows`, and an empty vector
    // reads as "this file is empty", which is a different thing from "this file is blank".
    while out.lines.last().is_some_and(|l| l.text.is_empty()) {
        out.lines.pop();
    }
    out.lines
}

/// The source lines, each with the byte offset it starts at and no line ending.
fn split(text: &str) -> Vec<(u64, &str)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for line in text.split_inclusive('\n') {
        let body = line.trim_end_matches('\n').trim_end_matches('\r');
        out.push((at as u64, body));
        at += line.len();
    }
    if out.is_empty() {
        out.push((0, ""));
    }
    out
}

/// Walk the blocks of a document (or of a blockquote's contents, which is the same thing one
/// level in) and render each one. `prefix` is what every line is to start with, which is how
/// a quote bar reaches the lines inside it.
fn blocks(lines: &[(u64, &str)], width: usize, prefix: &str, out: &mut Out) {
    let mut i = 0usize;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        out.at(offset);

        if raw.trim().is_empty() {
            i += 1;
            continue;
        }
        if let Some(len) = fence_at(raw) {
            i = fenced(lines, i, len, width, prefix, out);
            continue;
        }
        if is_break(raw) {
            out.blank();
            let rule: String = "─".repeat(width.saturating_sub(prefix.width()));
            let text = format!("{prefix}{rule}");
            let spans = vec![span(prefix.len(), text.len(), Class::Frame)];
            out.push(text, spans);
            out.blank();
            i += 1;
            continue;
        }
        if let Some((level, body)) = atx_at(raw) {
            heading(level, body, width, prefix, out);
            i += 1;
            continue;
        }
        if raw.starts_with('>') {
            i = quote(lines, i, width, prefix, out);
            continue;
        }
        if let Some(cols) = table_at(lines, i) {
            i = table(lines, i, cols, width, prefix, out);
            continue;
        }
        if let Some((indent, marker, body)) = item_at(raw) {
            i = list(lines, i, indent, marker, body, width, prefix, out);
            continue;
        }
        if raw.starts_with("    ") || raw.starts_with('\t') {
            i = indented(lines, i, prefix, out);
            continue;
        }
        if is_html(raw.trim_start()) {
            // Raw HTML. There is nothing to render it as, so it is shown as it was written,
            // in the colour comments get: a reader can see it is there and see it is not prose.
            out.blank();
            while i < lines.len() && !lines[i].1.trim().is_empty() {
                out.at(lines[i].0);
                let text = format!("{prefix}{}", lines[i].1);
                let spans = vec![span(prefix.len(), text.len(), Class::Code)];
                out.push(text, spans);
                i += 1;
            }
            out.blank();
            continue;
        }
        i = paragraph(lines, i, width, prefix, out);
    }
}

/// A span of one class, taking its boldness from the class.
fn span(start: usize, end: usize, class: Class) -> Span {
    Span {
        start,
        end,
        class,
        bold: class.bold(),
    }
}

/// The length of the fence this line opens, if it opens one.
fn fence_at(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    for mark in ['`', '~'] {
        let run = trimmed.chars().take_while(|c| *c == mark).count();
        if run >= 3 {
            return Some(run);
        }
    }
    None
}

/// Is this the start of a raw HTML block? A tag is `<` and then a name, a slash or a bang.
/// `<https://example.com>` is an autolink and belongs to the inline parser, which is the
/// distinction this predicate exists to make.
fn is_html(line: &str) -> bool {
    if line.contains("://") {
        return false;
    }
    let mut chars = line.chars();
    chars.next() == Some('<')
        && chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '/' || c == '!')
}

/// `---`, `***`, `___`: three or more of one mark and nothing else.
fn is_break(line: &str) -> bool {
    let trimmed = line.trim();
    for mark in ['-', '*', '_'] {
        let kept: Vec<char> = trimmed.chars().filter(|c| !c.is_whitespace()).collect();
        if kept.len() >= 3 && kept.iter().all(|c| *c == mark) {
            return true;
        }
    }
    false
}

/// `## Heading` — the level and the text, with the closing `###` taken off.
fn atx_at(line: &str) -> Option<(usize, &str)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if level == 0 || level > 6 {
        return None;
    }
    let rest = &trimmed[level..];
    if !rest.is_empty() && !rest.starts_with(' ') && !rest.starts_with('\t') {
        return None;
    }
    Some((level, rest.trim().trim_end_matches('#').trim_end()))
}

/// A heading: its text, bold, with a rule under the top two levels. The rule is what tells a
/// reader scrolling past that a section started, which a colour alone does not.
fn heading(level: usize, body: &str, width: usize, prefix: &str, out: &mut Out) {
    out.blank();
    let atoms = inline(body, Class::Heading, true);
    let indent = prefix.to_string();
    let lines = flow(&atoms, width, &indent, &indent);
    let mut widest = 0usize;
    for (text, spans) in lines {
        widest = widest.max(text.width().saturating_sub(prefix.width()));
        out.push(text, spans);
    }
    if level <= 2 {
        let mark = if level == 1 { '═' } else { '─' };
        let room = width.saturating_sub(prefix.width());
        let rule: String = mark.to_string().repeat(widest.clamp(1, room));
        let text = format!("{prefix}{rule}");
        let spans = vec![span(prefix.len(), text.len(), Class::Heading)];
        out.push(text, spans);
    }
    out.blank();
}

/// A fenced block, from its opening fence to the matching close or the end of the document.
fn fenced(
    lines: &[(u64, &str)],
    start: usize,
    len: usize,
    width: usize,
    prefix: &str,
    out: &mut Out,
) -> usize {
    let open = lines[start].1.trim_start();
    let mark = open.chars().next().unwrap_or('`');
    // The info string names the language, and where we have a highlighter for it the code
    // inside the fence is coloured as that language rather than as one undifferentiated block.
    let info = open.trim_start_matches(mark).trim();
    let language = info.split_whitespace().next().unwrap_or("");
    let syntax = fcmd_syntax::of_language(language);
    let mut state = fcmd_syntax::State::default();

    out.blank();
    let mut i = start + 1;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        let trimmed = raw.trim_start();
        if trimmed.chars().take_while(|c| *c == mark).count() >= len
            && trimmed.trim_end().len() == trimmed.chars().take_while(|c| *c == mark).count()
        {
            i += 1;
            break;
        }
        out.at(offset);
        let text = format!(
            "{prefix}    {}",
            clip(raw, width.saturating_sub(prefix.width() + 4))
        );
        let mut spans = Vec::new();
        match syntax {
            Some(syntax) => {
                let base = prefix.len() + 4;
                let body = &text[base..];
                let (runs, next) = syntax.line(body, state);
                state = next;
                // The spans must not overlap — the drawing code paints them in order and the
                // last one wins — so what the language did not claim is filled in as code.
                let mut at = 0usize;
                for run in runs {
                    if run.start < at || run.end > body.len() {
                        continue;
                    }
                    if run.start > at {
                        spans.push(span(base + at, base + run.start, Class::Code));
                    }
                    spans.push(span(
                        base + run.start,
                        base + run.end,
                        Class::Syntax(run.class),
                    ));
                    at = run.end;
                }
                if at < body.len() {
                    spans.push(span(base + at, text.len(), Class::Code));
                }
            }
            None => spans.push(span(prefix.len() + 4, text.len(), Class::Code)),
        }
        out.push(text, spans);
        i += 1;
    }
    out.blank();
    i
}

/// Four spaces or a tab: a code block without a fence, so there is no language to read.
fn indented(lines: &[(u64, &str)], start: usize, prefix: &str, out: &mut Out) -> usize {
    out.blank();
    let mut i = start;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        if raw.trim().is_empty() {
            // A blank line inside an indented block belongs to it only if more indented
            // lines follow; otherwise the block has ended.
            let more = lines[i + 1..]
                .iter()
                .find(|(_, l)| !l.trim().is_empty())
                .is_some_and(|(_, l)| l.starts_with("    ") || l.starts_with('\t'));
            if !more {
                break;
            }
            out.at(offset);
            out.push(prefix.to_string(), Vec::new());
            i += 1;
            continue;
        }
        if !raw.starts_with("    ") && !raw.starts_with('\t') {
            break;
        }
        out.at(offset);
        let body = raw
            .strip_prefix("    ")
            .unwrap_or(raw.trim_start_matches('\t'));
        let text = format!("{prefix}    {body}");
        let spans = vec![span(prefix.len() + 4, text.len(), Class::Code)];
        out.push(text, spans);
        i += 1;
    }
    out.blank();
    i
}

/// A blockquote: the `>` marks come off, a bar goes on, and what is inside is a document in
/// its own right — which is how a quoted list or a quoted quote renders as what it is.
fn quote(lines: &[(u64, &str)], start: usize, width: usize, prefix: &str, out: &mut Out) -> usize {
    let mut inner: Vec<(u64, &str)> = Vec::new();
    let mut i = start;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        let trimmed = raw.trim_start();
        if let Some(rest) = trimmed.strip_prefix('>') {
            inner.push((offset, rest.strip_prefix(' ').unwrap_or(rest)));
            i += 1;
            continue;
        }
        // A lazy continuation line: no `>`, but part of the quoted paragraph all the same.
        if !trimmed.is_empty() && !inner.is_empty() {
            inner.push((offset, raw));
            i += 1;
            continue;
        }
        break;
    }
    let bar = format!("{prefix}│ ");
    out.blank();
    let mut quoted = Out {
        source: out.source,
        ..Out::default()
    };
    blocks(&inner, width, &bar, &mut quoted);
    while quoted
        .lines
        .last()
        .is_some_and(|l| l.text.trim().is_empty())
    {
        quoted.lines.pop();
    }
    for line in quoted.lines {
        out.source = line.source;
        out.sub = line.sub + 1;
        // The bar is drawn even on the blank lines between the quoted blocks, so the quote
        // reads as one thing rather than as several.
        let text = if line.text.is_empty() {
            bar.trim_end().to_string()
        } else {
            line.text
        };
        let mut spans = line.spans;
        spans.insert(
            0,
            span(prefix.len(), prefix.len() + "│".len(), Class::Marker),
        );
        out.lines.push(Line {
            text,
            spans,
            source: line.source,
            sub: line.sub,
        });
    }
    out.blank();
    i
}

/// A list item's indentation, its marker as it is to be drawn, and what follows it.
fn item_at(line: &str) -> Option<(usize, String, &str)> {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    let rest = &line[indent..];
    if let Some(body) = rest
        .strip_prefix("- ")
        .or_else(|| rest.strip_prefix("* "))
        .or_else(|| rest.strip_prefix("+ "))
    {
        // A task list is a list whose markers are boxes, which is the one case where the
        // mark in the source is worth keeping — as a box rather than as `[ ]`.
        let (mark, body) = match body.trim_start() {
            b if b.starts_with("[ ] ") => ("☐", &b[4..]),
            b if b.to_ascii_lowercase().starts_with("[x] ") => ("☑", &b[4..]),
            _ => (bullet(indent), body),
        };
        return Some((indent, mark.to_string(), body));
    }
    // `1.` or `1)`, up to nine digits, which is more than any list anybody reads has.
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if (1..=9).contains(&digits) {
        let after = &rest[digits..];
        if let Some(body) = after
            .strip_prefix(". ")
            .or_else(|| after.strip_prefix(") "))
        {
            return Some((indent, format!("{}.", &rest[..digits]), body));
        }
    }
    None
}

/// Which bullet a nesting level gets. Three are enough: a document that nests deeper than
/// that is not being read off a screen anyway.
fn bullet(indent: usize) -> &'static str {
    match indent / 2 {
        0 => "•",
        1 => "◦",
        _ => "▪",
    }
}

/// A list, from its first item to the first line that is not part of one.
#[allow(clippy::too_many_arguments)]
fn list(
    lines: &[(u64, &str)],
    start: usize,
    _indent: usize,
    _marker: String,
    _body: &str,
    width: usize,
    prefix: &str,
    out: &mut Out,
) -> usize {
    out.blank();
    let mut i = start;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        if raw.trim().is_empty() {
            // A blank line inside a list is kept only if the list goes on after it.
            let more = lines[i + 1..]
                .iter()
                .find(|(_, l)| !l.trim().is_empty())
                .is_some_and(|(_, l)| item_at(l).is_some() || l.starts_with("  "));
            if !more {
                break;
            }
            i += 1;
            continue;
        }
        let Some((indent, marker, body)) = item_at(raw) else {
            break;
        };
        // Everything that follows an item without a marker of its own continues it.
        let mut text = body.trim().to_string();
        let mut last = i;
        for (j, (_, cont)) in lines.iter().enumerate().skip(i + 1) {
            if cont.trim().is_empty() || item_at(cont).is_some() {
                break;
            }
            let deeper = cont.chars().take_while(|c| *c == ' ').count();
            if deeper <= indent && indent > 0 {
                break;
            }
            text.push(' ');
            text.push_str(cont.trim());
            last = j;
        }

        out.at(offset);
        let pad = " ".repeat(indent);
        let first = format!("{prefix}{pad}{marker} ");
        let cont = " ".repeat(first.width());
        let atoms = inline(&text, Class::Text, false);
        let flowed = flow(&atoms, width, &first, &cont);
        for (n, (line, mut spans)) in flowed.into_iter().enumerate() {
            if n == 0 {
                // The marker is the renderer's, not the author's, so it is coloured as such.
                spans.insert(
                    0,
                    span(prefix.len() + pad.len(), first.len() - 1, Class::Marker),
                );
            }
            out.push(line, spans);
        }
        i = last + 1;
    }
    out.blank();
    i
}

/// Does a table start here? If so, how many columns has it got?
///
/// A table is a row of cells and then the delimiter row that says so. Without that second
/// row a line with pipes in it is a line with pipes in it, which is what CommonMark says and
/// what anybody who has written `a | b` in prose expects.
fn table_at(lines: &[(u64, &str)], at: usize) -> Option<usize> {
    let header = lines.get(at)?.1;
    if !header.contains('|') {
        return None;
    }
    let delim = lines.get(at + 1)?.1.trim();
    if delim.is_empty() || !delim.contains('-') {
        return None;
    }
    if !delim
        .chars()
        .all(|c| matches!(c, '|' | '-' | ':' | ' ' | '\t'))
    {
        return None;
    }
    let cols = cells(header).len();
    (cols > 0 && cells(delim).len() == cols).then_some(cols)
}

/// A table row split into its cells, with the outer pipes dropped and `\|` left alone.
fn cells(row: &str) -> Vec<String> {
    let trimmed = row.trim();
    let trimmed = trimmed.strip_prefix('|').unwrap_or(trimmed);
    let trimmed = trimmed.strip_suffix('|').unwrap_or(trimmed);
    let mut out = Vec::new();
    let mut cell = String::new();
    let mut escaped = false;
    for c in trimmed.chars() {
        match c {
            '\\' if !escaped => {
                escaped = true;
                cell.push(c);
            }
            '|' if !escaped => {
                out.push(cell.trim().to_string());
                cell = String::new();
            }
            _ => {
                escaped = false;
                cell.push(c);
            }
        }
    }
    out.push(cell.trim().to_string());
    out
}

/// How a column's cells sit in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Centre,
    Right,
}

/// The alignments the delimiter row asks for.
fn alignments(delim: &str, cols: usize) -> Vec<Align> {
    let mut out = Vec::with_capacity(cols);
    for cell in cells(delim) {
        let cell = cell.trim();
        out.push(match (cell.starts_with(':'), cell.ends_with(':')) {
            (true, true) => Align::Centre,
            (false, true) => Align::Right,
            _ => Align::Left,
        });
    }
    out.resize(cols, Align::Left);
    out
}

/// A table, drawn with its columns lined up and a frame around them. This is the thing a
/// highlighted source cannot give you: a column of numbers you can read down.
fn table(
    lines: &[(u64, &str)],
    start: usize,
    cols: usize,
    width: usize,
    prefix: &str,
    out: &mut Out,
) -> usize {
    let align = alignments(lines[start + 1].1, cols);
    let mut rows: Vec<(u64, Vec<String>)> = vec![(lines[start].0, cells(lines[start].1))];
    let mut i = start + 2;
    while i < lines.len() {
        let (offset, raw) = lines[i];
        if raw.trim().is_empty() || !raw.contains('|') {
            break;
        }
        rows.push((offset, cells(raw)));
        i += 1;
    }
    for (_, row) in rows.iter_mut() {
        row.resize(cols, String::new());
    }

    // The columns are as wide as their widest cell, measured on the text a reader will see
    // rather than on the source: `**no**` is two columns wide, not six.
    let rendered: Vec<Vec<String>> = rows
        .iter()
        .map(|(_, row)| row.iter().map(|c| plain(c)).collect())
        .collect();
    let mut widths: Vec<usize> = (0..cols)
        .map(|c| {
            rendered
                .iter()
                .map(|r| r[c].width())
                .max()
                .unwrap_or(0)
                .max(1)
        })
        .collect();
    // Too wide for the window: the widest column gives a column back until it fits, so a
    // table of one long description and three short numbers loses it from the description.
    let room = width.saturating_sub(prefix.width());
    let frame = |widths: &[usize]| widths.iter().sum::<usize>() + 3 * widths.len() + 1;
    while frame(&widths) > room {
        let Some(widest) = (0..cols).max_by_key(|c| widths[*c]) else {
            break;
        };
        if widths[widest] <= 3 {
            break;
        }
        widths[widest] -= 1;
    }

    let rule = |left: char, mid: char, right: char| {
        let mut text = String::from(prefix);
        text.push(left);
        for (n, w) in widths.iter().enumerate() {
            if n > 0 {
                text.push(mid);
            }
            text.push_str(&"─".repeat(w + 2));
        }
        text.push(right);
        text
    };

    out.blank();
    out.at(lines[start].0);
    let top = rule('┌', '┬', '┐');
    let spans = vec![span(prefix.len(), top.len(), Class::Frame)];
    out.push(top, spans);

    for (n, (offset, row)) in rows.iter().enumerate() {
        out.at(*offset);
        let mut text = String::from(prefix);
        let mut spans = vec![span(prefix.len(), prefix.len() + "│".len(), Class::Frame)];
        text.push('│');
        for (c, cell) in row.iter().enumerate() {
            // The header is the one row that is bold, which is what makes it the header.
            let atoms = inline(cell, Class::Text, n == 0);
            let content = pad(&atoms, widths[c], align[c]);
            text.push(' ');
            let base = text.len();
            for atom in &content {
                let start = text.len();
                text.push(atom.ch);
                spans.push(Span {
                    start,
                    end: text.len(),
                    class: atom.class,
                    bold: atom.bold,
                });
            }
            let _ = base;
            text.push(' ');
            let bar = text.len();
            text.push('│');
            spans.push(span(bar, text.len(), Class::Frame));
        }
        out.push(text, merge(spans));
        if n == 0 {
            let under = rule('├', '┼', '┤');
            let spans = vec![span(prefix.len(), under.len(), Class::Frame)];
            out.push(under, spans);
        }
    }

    out.at(rows.last().map(|(o, _)| *o).unwrap_or(lines[start].0));
    let bottom = rule('└', '┴', '┘');
    let spans = vec![span(prefix.len(), bottom.len(), Class::Frame)];
    out.push(bottom, spans);
    out.blank();
    i
}

/// A cell's atoms, cut or padded to the column's width and placed in it.
fn pad(atoms: &[Atom], width: usize, align: Align) -> Vec<Atom> {
    let mut content: Vec<Atom> = atoms.to_vec();
    let shown: usize = content.iter().map(|a| char_width(a.ch)).sum();
    if shown > width {
        // Cut it, and say so with an ellipsis rather than leaving a word half written.
        let mut kept = Vec::new();
        let mut used = 0usize;
        for atom in content {
            let w = char_width(atom.ch);
            if used + w > width.saturating_sub(1) {
                break;
            }
            used += w;
            kept.push(atom);
        }
        kept.push(Atom::new('…', Class::Frame, false));
        return kept;
    }
    let slack = width - shown;
    let (before, after) = match align {
        Align::Left => (0, slack),
        Align::Right => (slack, 0),
        Align::Centre => (slack / 2, slack - slack / 2),
    };
    let mut out = Vec::with_capacity(width);
    out.extend(std::iter::repeat_n(
        Atom::new(' ', Class::Text, false),
        before,
    ));
    out.append(&mut content);
    out.extend(std::iter::repeat_n(
        Atom::new(' ', Class::Text, false),
        after,
    ));
    out
}

/// A paragraph: every line up to the next blank one, joined and wrapped to the window.
fn paragraph(
    lines: &[(u64, &str)],
    start: usize,
    width: usize,
    prefix: &str,
    out: &mut Out,
) -> usize {
    let mut text = String::new();
    let mut i = start;
    while i < lines.len() {
        let raw = lines[i].1;
        if raw.trim().is_empty() || is_break(raw) || atx_at(raw).is_some() || item_at(raw).is_some()
        {
            break;
        }
        if fence_at(raw).is_some() || raw.trim_start().starts_with('>') {
            break;
        }
        // `Heading` over a line of `===` or `---` — the other way of writing a heading, and
        // the reason a paragraph has to look one line ahead.
        if i > start {
            let marks = raw.trim();
            let setext = !marks.is_empty()
                && (marks.chars().all(|c| c == '=') || marks.chars().all(|c| c == '-'));
            if setext {
                break;
            }
        }
        if i + 1 < lines.len() {
            let next = lines[i + 1].1.trim();
            let level = match next.chars().next() {
                Some('=') if next.chars().all(|c| c == '=') => Some(1),
                Some('-') if next.len() > 1 && next.chars().all(|c| c == '-') => Some(2),
                _ => None,
            };
            if let Some(level) = level {
                if text.is_empty() {
                    out.at(lines[i].0);
                    heading(level, raw.trim(), width, prefix, out);
                    return i + 2;
                }
                break;
            }
        }
        if !text.is_empty() {
            text.push(' ');
        }
        text.push_str(raw.trim());
        i += 1;
    }
    if text.is_empty() {
        return i.max(start + 1);
    }
    out.at(lines[start].0);
    out.blank();
    let atoms = inline(&text, Class::Text, false);
    for (line, spans) in flow(&atoms, width, prefix, prefix) {
        out.push(line, spans);
    }
    out.blank();
    i
}

/// What a run of inline Markdown reads as, with the marks gone.
fn plain(text: &str) -> String {
    inline(text, Class::Text, false)
        .iter()
        .map(|a| a.ch)
        .collect()
}

/// Parse inline marks into atoms. `base` and `bold` are what plain text in this context is:
/// the same function renders a paragraph, a heading and a table cell.
fn inline(text: &str, base: Class, bold: bool) -> Vec<Atom> {
    let chars: Vec<char> = text.chars().collect();
    let mut out: Vec<Atom> = Vec::with_capacity(chars.len());
    let mut class = base;
    let mut strong = bold;
    let mut emphasis = false;
    let mut strike = false;
    let mut i = 0usize;

    let current = |class: Class, emphasis: bool, strike: bool, base: Class| {
        if strike {
            Class::Strike
        } else if emphasis {
            Class::Emphasis
        } else if class != base {
            class
        } else {
            base
        }
    };

    while i < chars.len() {
        let c = chars[i];
        // An escape puts the next character in as itself, which is the only way to write a
        // literal asterisk and the reason `\*` is not emphasis.
        if c == '\\' && i + 1 < chars.len() && chars[i + 1].is_ascii_punctuation() {
            out.push(Atom::new(
                chars[i + 1],
                current(class, emphasis, strike, base),
                strong,
            ));
            i += 2;
            continue;
        }
        if c == '`' {
            let run = chars[i..].iter().take_while(|c| **c == '`').count();
            // A code span ends at a run of the same length; one that never does is not a
            // code span, so the backticks stay as text.
            if let Some(end) = find_run(&chars, i + run, '`', run) {
                for ch in &chars[i + run..end] {
                    out.push(Atom::new(*ch, Class::Code, strong));
                }
                i = end + run;
                continue;
            }
        }
        if c == '~' && i + 1 < chars.len() && chars[i + 1] == '~' {
            strike = !strike;
            i += 2;
            continue;
        }
        if c == '*' || c == '_' {
            let run = chars[i..].iter().take_while(|ch| **ch == c).count();
            // `_` inside a word is a word, not emphasis: `snake_case_names` is one name and
            // nobody writing one meant to turn the middle of it bold.
            let boundary = c == '*' || {
                let before = i.checked_sub(1).map(|p| chars[p]);
                let after = chars.get(i + run).copied();
                !before.is_some_and(|c| c.is_alphanumeric())
                    || !after.is_some_and(|c| c.is_alphanumeric())
            };
            if boundary {
                if run >= 2 {
                    strong = !strong;
                    i += 2;
                } else {
                    emphasis = !emphasis;
                    i += 1;
                }
                continue;
            }
        }
        if c == '!' && chars.get(i + 1) == Some(&'[') {
            if let Some((label, url, next)) = link_at(&chars, i + 1) {
                // An image cannot be drawn in a cell, so it is announced and its text shown.
                for ch in "▣ ".chars() {
                    out.push(Atom::new(ch, Class::Marker, strong));
                }
                push_label(&mut out, &label, strong);
                push_url(&mut out, &url, strong);
                i = next;
                continue;
            }
        }
        if c == '[' {
            if let Some((label, url, next)) = link_at(&chars, i) {
                push_label(&mut out, &label, strong);
                push_url(&mut out, &url, strong);
                i = next;
                continue;
            }
        }
        if c == '<' {
            // `<https://example.com>`: a URL and nothing else between the angle brackets.
            if let Some(end) = chars[i..].iter().position(|c| *c == '>') {
                let inner: String = chars[i + 1..i + end].iter().collect();
                if inner.contains("://") || inner.contains('@') {
                    for ch in inner.chars() {
                        out.push(Atom::new(ch, Class::Url, strong));
                    }
                    i += end + 1;
                    continue;
                }
            }
        }
        out.push(Atom::new(c, current(class, emphasis, strike, base), strong));
        class = base;
        i += 1;
    }
    out
}

/// The next run of `mark` of at least `len`, from `from`.
fn find_run(chars: &[char], from: usize, mark: char, len: usize) -> Option<usize> {
    let mut i = from;
    while i < chars.len() {
        if chars[i] == mark {
            let run = chars[i..].iter().take_while(|c| **c == mark).count();
            if run >= len {
                return Some(i);
            }
            i += run;
            continue;
        }
        i += 1;
    }
    None
}

/// `[label](url)` starting at `at`, and where it ends.
fn link_at(chars: &[char], at: usize) -> Option<(String, String, usize)> {
    if chars.get(at) != Some(&'[') {
        return None;
    }
    let mut depth = 0usize;
    let mut close = None;
    for (i, c) in chars.iter().enumerate().skip(at) {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    close = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    if chars.get(close + 1) != Some(&'(') {
        return None;
    }
    let end = chars[close + 1..].iter().position(|c| *c == ')')? + close + 1;
    let label: String = chars[at + 1..close].iter().collect();
    let url: String = chars[close + 2..end].iter().collect();
    Some((label, url, end + 1))
}

/// A link's text, with its own inline marks rendered but all of it coloured as a link.
fn push_label(out: &mut Vec<Atom>, label: &str, strong: bool) {
    for atom in inline(label, Class::Link, strong) {
        out.push(Atom::new(atom.ch, Class::Link, atom.bold));
    }
}

/// The URL after it, in parentheses. A terminal cannot be clicked, so hiding the address
/// would lose the only part a reader can act on.
fn push_url(out: &mut Vec<Atom>, url: &str, strong: bool) {
    if url.is_empty() {
        return;
    }
    out.push(Atom::new(' ', Class::Url, strong));
    out.push(Atom::new('(', Class::Url, strong));
    for ch in url.chars() {
        out.push(Atom::new(ch, Class::Url, strong));
    }
    out.push(Atom::new(')', Class::Url, strong));
}

/// How wide a character is on screen; a control character takes no room.
fn char_width(c: char) -> usize {
    unicode_width::UnicodeWidthChar::width(c).unwrap_or(0)
}

/// Wrap atoms into lines, breaking at spaces, with `first` in front of the first line and
/// `cont` in front of the rest — which is what gives a list item its hanging indent.
fn flow(atoms: &[Atom], width: usize, first: &str, cont: &str) -> Vec<(String, Vec<Span>)> {
    let mut out = Vec::new();
    let mut line: Vec<Atom> = Vec::new();
    let mut word: Vec<Atom> = Vec::new();
    let mut indent = first;
    let mut room = width.saturating_sub(indent.width()).max(4);

    let flush = |line: &mut Vec<Atom>, indent: &str, out: &mut Vec<(String, Vec<Span>)>| {
        while line.last().is_some_and(|a| a.ch == ' ') {
            line.pop();
        }
        out.push(build(indent, line));
        line.clear();
    };

    for atom in atoms {
        // A word nobody can break — a long URL, a path, a hash — is broken anyway: letting
        // it run past the edge would lose its tail, and a reader needs the tail most.
        let word_w: usize = word.iter().map(|a| char_width(a.ch)).sum();
        if word_w >= room {
            let line_w: usize = line.iter().map(|a| char_width(a.ch)).sum();
            if line_w > 0 {
                flush(&mut line, indent, &mut out);
                indent = cont;
                room = width.saturating_sub(indent.width()).max(4);
            }
            if word.iter().map(|a| char_width(a.ch)).sum::<usize>() >= room {
                line.append(&mut word);
                flush(&mut line, indent, &mut out);
                indent = cont;
                room = width.saturating_sub(indent.width()).max(4);
            }
        }
        if atom.ch == ' ' || atom.ch == '\t' {
            let word_w: usize = word.iter().map(|a| char_width(a.ch)).sum();
            let line_w: usize = line.iter().map(|a| char_width(a.ch)).sum();
            if line_w + word_w > room && !line.is_empty() {
                flush(&mut line, indent, &mut out);
                indent = cont;
                room = width.saturating_sub(indent.width()).max(4);
            }
            line.append(&mut word);
            line.push(Atom::new(' ', atom.class, atom.bold));
            continue;
        }
        word.push(*atom);
    }
    let word_w: usize = word.iter().map(|a| char_width(a.ch)).sum();
    let line_w: usize = line.iter().map(|a| char_width(a.ch)).sum();
    if line_w + word_w > room && !line.is_empty() {
        flush(&mut line, indent, &mut out);
        indent = cont;
    }
    line.append(&mut word);
    if !line.is_empty() || out.is_empty() {
        flush(&mut line, indent, &mut out);
    }
    out
}

/// Atoms into a line of text and the spans over it.
fn build(indent: &str, atoms: &[Atom]) -> (String, Vec<Span>) {
    let mut text = String::from(indent);
    let mut spans = Vec::new();
    for atom in atoms {
        let start = text.len();
        text.push(atom.ch);
        spans.push(Span {
            start,
            end: text.len(),
            class: atom.class,
            bold: atom.bold,
        });
    }
    (text, merge(spans))
}

/// Runs of the same class and weight become one span, which is what the drawing code wants:
/// one `put_str` per run rather than one per character.
fn merge(spans: Vec<Span>) -> Vec<Span> {
    let mut out: Vec<Span> = Vec::with_capacity(spans.len());
    for span in spans {
        match out.last_mut() {
            Some(last)
                if last.end == span.start && last.class == span.class && last.bold == span.bold =>
            {
                last.end = span.end;
            }
            _ => out.push(span),
        }
    }
    out
}

/// Cut a string to `width` columns, on a character boundary.
fn clip(text: &str, width: usize) -> String {
    let mut out = String::new();
    let mut used = 0usize;
    for c in text.chars() {
        let w = char_width(c);
        if used + w > width {
            break;
        }
        used += w;
        out.push(c);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rendered text of a document, one string per line, for tests that are about layout.
    fn lines(src: &str, width: usize) -> Vec<String> {
        render(src, width).into_iter().map(|l| l.text).collect()
    }

    /// What class covers the first occurrence of `needle`?
    fn class_of(src: &str, width: usize, needle: &str) -> Option<Class> {
        for line in render(src, width) {
            if let Some(at) = line.text.find(needle) {
                return line
                    .spans
                    .iter()
                    .find(|s| s.start <= at && at < s.end)
                    .map(|s| s.class);
            }
        }
        None
    }

    #[test]
    fn a_heading_loses_its_hashes_and_gains_a_rule() {
        let out = lines("# Title\n\nbody\n", 40);
        assert_eq!(out[0], "Title");
        assert_eq!(out[1], "═════", "a first-level heading is ruled with ═");
        assert_eq!(out.last().unwrap(), "body");
        assert_eq!(class_of("# Title\n", 40, "Title"), Some(Class::Heading));

        // The second level gets the lighter rule, and the third none at all.
        let out = lines("## Two\n", 40);
        assert_eq!(out, vec!["Two", "───"]);
        let out = lines("### Three\n", 40);
        assert_eq!(out, vec!["Three"]);
    }

    #[test]
    fn a_setext_heading_is_a_heading_too() {
        let out = lines("Title\n=====\n\nbody\n", 40);
        assert_eq!(out[0], "Title");
        assert_eq!(out[1], "═════");
        assert_eq!(out.last().unwrap(), "body");
    }

    #[test]
    fn a_paragraph_is_joined_and_wrapped_to_the_window() {
        let src = "one two three\nfour five six\n";
        assert_eq!(lines(src, 20), vec!["one two three four", "five six"]);
        // A narrower window rewraps it; the text is the same text.
        assert_eq!(
            lines(src, 10).join(" ").split_whitespace().count(),
            6,
            "no word may be lost in the wrapping"
        );
    }

    #[test]
    fn the_marks_come_off_and_turn_into_colour_and_weight() {
        let out = lines("**bold** and *soft* and `code` and ~~gone~~\n", 60);
        assert_eq!(out[0], "bold and soft and code and gone");
        let src = "**bold** and *soft* and `code` and ~~gone~~\n";
        assert_eq!(class_of(src, 60, "soft"), Some(Class::Emphasis));
        assert_eq!(class_of(src, 60, "code"), Some(Class::Code));
        assert_eq!(class_of(src, 60, "gone"), Some(Class::Strike));
        let bold = render(src, 60)[0]
            .spans
            .iter()
            .find(|s| s.bold)
            .copied()
            .expect("something should be bold");
        assert_eq!(&render(src, 60)[0].text[bold.start..bold.end], "bold");
    }

    #[test]
    fn an_underscore_inside_a_word_is_part_of_the_word() {
        // The classic false positive: a snake_case name is one name, not emphasis.
        assert_eq!(
            lines("a snake_case_name here\n", 40),
            vec!["a snake_case_name here"]
        );
        // Between words it still emphasises.
        assert_eq!(lines("_soft_ here\n", 40), vec!["soft here"]);
    }

    #[test]
    fn an_escape_keeps_the_mark() {
        assert_eq!(lines(r"a \*literal\* star", 40), vec!["a *literal* star"]);
    }

    #[test]
    fn a_link_shows_its_text_and_its_address() {
        let src = "see [the docs](https://example.com/x) for more\n";
        assert_eq!(
            lines(src, 60),
            vec!["see the docs (https://example.com/x) for more"]
        );
        assert_eq!(class_of(src, 60, "the docs"), Some(Class::Link));
        assert_eq!(class_of(src, 60, "https"), Some(Class::Url));
        // An image says it is one, because a cell cannot hold it.
        assert!(lines("![a cat](cat.png)\n", 40)[0].starts_with('▣'));
        // An autolink is a URL on its own.
        assert_eq!(
            lines("<https://example.com>\n", 40),
            vec!["https://example.com"]
        );
    }

    #[test]
    fn a_list_gets_bullets_and_a_hanging_indent() {
        let out = lines("- one\n- two\n", 40);
        assert_eq!(out, vec!["• one", "• two"]);
        // Nesting changes the bullet, and the indent is kept.
        let out = lines("- one\n  - deep\n", 40);
        assert_eq!(out, vec!["• one", "  ◦ deep"]);
        // A numbered list keeps its numbers.
        assert_eq!(lines("1. one\n2. two\n", 40), vec!["1. one", "2. two"]);
        // A task list is boxes.
        assert_eq!(
            lines("- [ ] todo\n- [x] done\n", 40),
            vec!["☐ todo", "☑ done"]
        );
        // A long item wraps under its own text, not under the bullet.
        let out = lines("- one two three four five six seven\n", 16);
        assert_eq!(out[0], "• one two three");
        assert!(out[1].starts_with("  "), "{out:?}");
    }

    #[test]
    fn a_fence_is_kept_verbatim_and_indented() {
        let out = lines("text\n\n```\nlet x = 1;\n```\n\nmore\n", 40);
        assert_eq!(out[0], "text");
        assert!(
            out.iter().any(|l| l == "    let x = 1;"),
            "the code keeps its spacing: {out:?}"
        );
        assert!(!out.iter().any(|l| l.contains("```")), "{out:?}");
        assert_eq!(out.last().unwrap(), "more");
    }

    #[test]
    fn a_fence_with_a_language_is_coloured_as_that_language() {
        let src = "```rust\nfn main() {}\n```\n";
        // `fn` is a keyword in Rust, and the fence said Rust.
        assert_eq!(
            class_of(src, 40, "fn"),
            Some(Class::Syntax(fcmd_syntax::Class::Keyword))
        );
        // Without a language it is all one block.
        assert_eq!(
            class_of("```\nfn main() {}\n```\n", 40, "fn"),
            Some(Class::Code)
        );
    }

    #[test]
    fn a_quote_gets_a_bar_and_keeps_what_is_in_it() {
        let out = lines("> quoted\n", 40);
        assert_eq!(out, vec!["│ quoted"]);
        // A list inside a quote is a list inside a quote.
        let out = lines("> - one\n> - two\n", 40);
        assert_eq!(out, vec!["│ • one", "│ • two"]);
        // And a quote inside a quote nests its bars.
        let out = lines("> > deep\n", 40);
        assert_eq!(out, vec!["│ │ deep"]);
    }

    #[test]
    fn a_thematic_break_is_a_rule_across_the_window() {
        let out = lines("a\n\n---\n\nb\n", 10);
        assert_eq!(out, vec!["a", "", "──────────", "", "b"]);
    }

    #[test]
    fn a_table_is_drawn_with_its_columns_lined_up() {
        let src = "\
| name | size |
|------|-----:|
| a    | 1    |
| long | 200  |
";
        let out = lines(src, 40);
        assert_eq!(
            out,
            vec![
                "┌──────┬──────┐",
                "│ name │ size │",
                "├──────┼──────┤",
                "│ a    │    1 │",
                "│ long │  200 │",
                "└──────┴──────┘",
            ]
        );
    }

    #[test]
    fn a_table_centres_and_truncates_as_it_is_told() {
        let src = "\
| a | b |
|:-:|---|
| x | y |
";
        let out = lines(src, 40);
        assert!(out[1].contains("│ a │"), "{out:?}");
        // A table too wide for the window loses it from the widest column, with an ellipsis.
        let src = "| description | n |\n|---|---|\n| a very long cell indeed | 1 |\n";
        let out = lines(src, 24);
        assert!(out.iter().all(|l| l.width() <= 24), "{out:?}");
        assert!(out.iter().any(|l| l.contains('…')), "{out:?}");
    }

    #[test]
    fn pipes_in_prose_are_not_a_table() {
        // Without the delimiter row it is a sentence that happens to have a pipe in it.
        assert_eq!(lines("a | b is a pipe\n", 40), vec!["a | b is a pipe"]);
    }

    #[test]
    fn every_line_points_back_at_the_source_it_came_from() {
        let src = "# One\n\nbody\n\n## Two\n\nmore\n";
        let rendered = render(src, 40);
        // The offsets never go backwards, which is what the viewer's scrolling relies on.
        let offsets: Vec<u64> = rendered.iter().map(|l| l.source).collect();
        assert!(
            offsets.windows(2).all(|w| w[0] <= w[1]),
            "offsets must not go backwards: {offsets:?}"
        );
        // And the line for `more` points at the line `more` is on.
        let at = rendered
            .iter()
            .find(|l| l.text == "more")
            .expect("`more` is rendered")
            .source;
        assert_eq!(&src[at as usize..at as usize + 4], "more");
    }

    /// `(source, sub)` is the viewer's position, so no two lines may share a pair and the
    /// pairs have to run forwards. Everything about scrolling a rendered document rests on it.
    #[test]
    fn every_line_has_a_place_of_its_own() {
        let src = "\
# Title

A paragraph long enough to wrap at a narrow width, with words in it.

| a | b |
|---|---|
| 1 | 2 |
| 3 | 4 |

> a quote
> over two lines

- one
- two

```rust
fn main() {}
```

---

Last.
";
        for width in [16usize, 30, 60] {
            let lines = render(src, width);
            let places: Vec<(u64, usize)> = lines.iter().map(|l| (l.source, l.sub)).collect();
            for pair in places.windows(2) {
                assert!(
                    pair[0] < pair[1],
                    "at width {width}: {:?} does not come before {:?}",
                    pair[0],
                    pair[1]
                );
            }
        }
    }

    #[test]
    fn the_sub_index_counts_within_one_source_line() {
        // A paragraph that wraps comes from one source line and produces several, so the
        // viewer can tell them apart by `sub` alone.
        let rendered = render("one two three four five six seven eight\n", 12);
        let body: Vec<&Line> = rendered.iter().filter(|l| !l.text.is_empty()).collect();
        assert!(body.len() > 2, "{body:?}");
        assert!(body.iter().all(|l| l.source == 0));
        let subs: Vec<usize> = body.iter().map(|l| l.sub).collect();
        assert!(subs.windows(2).all(|w| w[0] < w[1]), "{subs:?}");
    }

    #[test]
    fn an_empty_or_blank_document_renders_to_nothing() {
        assert!(render("", 40).is_empty());
        assert!(render("\n\n\n", 40).is_empty());
    }

    #[test]
    fn no_rendered_line_is_wider_than_the_window() {
        let src = "\
# A heading that is quite long indeed and will not fit

Some prose with a [link](https://example.com/a/very/long/path/indeed) in it.

- a list item that is also rather long and will have to wrap somewhere

> a quote with enough words in it to need wrapping as well, which it does

| a | b |
|---|---|
| 1 | 2 |
";
        for width in [12usize, 20, 40, 80] {
            for line in render(src, width) {
                assert!(
                    line.text.width() <= width,
                    "`{}` is {} wide, window is {width}",
                    line.text,
                    line.text.width()
                );
            }
        }
    }

    #[test]
    fn the_spans_cover_the_text_they_claim_to() {
        // Every span has to be a real byte range of its line, or the drawing code panics on
        // a slice that falls inside a character.
        let src = "# Tést\n\n**grassetto** e *corsivo* e `codice` — à è ì ò ù\n\n| à | b |\n|---|---|\n| 1 | 2 |\n";
        for line in render(src, 30) {
            for span in &line.spans {
                assert!(span.start <= span.end, "{span:?} in `{}`", line.text);
                assert!(
                    line.text.is_char_boundary(span.start) && line.text.is_char_boundary(span.end),
                    "{span:?} is not on a character boundary in `{}`",
                    line.text
                );
            }
        }
    }
}
