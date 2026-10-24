//! Syntax recognition for the viewer and the editor.
//!
//! A file is highlighted one line at a time, because that is how both of them draw: the
//! viewer never reads more of a file than fits on screen, and the editor redraws a window of
//! a buffer that may be millions of lines long. So the unit here is
//! [`Syntax::line`] — a line in, the coloured runs of it out, plus the [`State`] the next
//! line starts in. A block comment or an unterminated string is exactly that carried state,
//! and nothing else has to be remembered.
//!
//! What this is not: a parser. It does not know that `class` means something different in
//! Rust and in Java, and it will colour the word `if` inside an identifier-free string as a
//! string, which is right, but it will also colour a keyword in a language that merely
//! borrowed the word. That is the trade an editor makes everywhere, and it is why the tables
//! below are deliberately small: the common words of a language, the shape of its comments,
//! the quotes it uses, and nothing that needs a grammar.

mod tables;

pub use tables::{of_name, Syntax, SYNTAXES};

/// What a run of characters is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    /// `fn`, `if`, `SELECT`.
    Keyword,
    /// `u32`, `String`, `boolean` — and in markup, an element's name.
    Type,
    /// `"text"`, `'c'`, a here-document, a Markdown code span.
    Str,
    /// `42`, `0xff`, `1.5e3`.
    Number,
    /// Anything the language throws away.
    Comment,
    /// `#include`, `#!/bin/sh`, a Markdown heading.
    Preproc,
    /// `<div`, `</div>`.
    Tag,
    /// An attribute's name, a key in JSON, TOML, YAML or an INI file.
    Attr,
    /// `&amp;`, `\n` inside a string.
    Entity,
    /// A name being called or defined.
    Function,
    /// Everything that is none of the above.
    Text,
}

impl Class {
    /// The theme key this class is drawn with.
    pub fn theme_key(self) -> &'static str {
        match self {
            Class::Keyword => "syntax.keyword",
            Class::Type => "syntax.type",
            Class::Str => "syntax.string",
            Class::Number => "syntax.number",
            Class::Comment => "syntax.comment",
            Class::Preproc => "syntax.preproc",
            Class::Tag => "syntax.tag",
            Class::Attr => "syntax.attr",
            Class::Entity => "syntax.entity",
            Class::Function => "syntax.function",
            Class::Text => "syntax.text",
        }
    }
}

/// A run of one class, as byte offsets into the line it came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub class: Class,
}

/// What the next line starts inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct State {
    pub carry: Carry,
    /// Nesting of block comments, for the languages that nest them (Rust does; C does not).
    pub depth: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Carry {
    #[default]
    None,
    /// Inside `/* … */`, `<!-- … -->`, `=begin … =end`.
    Block,
    /// Inside a string that is allowed to span lines, with the quote that will end it.
    Str(char),
    /// Inside a Markdown fence, or a here-document.
    Fence,
}

/// How a language is shaped. Everything the scanner needs and nothing it does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flavour {
    /// Comments, strings, numbers, words.
    Code,
    /// Tags, attributes, entities: HTML, XML, SVG.
    Markup,
    /// Headings, fences, emphasis, links.
    Markdown,
}

impl Syntax {
    /// Colour one line, given the state the line before left behind.
    ///
    /// The spans come out in order, they never overlap, and the stretches between them are
    /// ordinary text: a caller draws the line in its base colour and then paints the spans
    /// over it, which is both the simplest thing to write and the fastest to draw.
    pub fn line(&self, text: &str, state: State) -> (Vec<Span>, State) {
        match self.flavour {
            Flavour::Code => self.scan_code(text, state),
            Flavour::Markup => self.scan_markup(text, state),
            Flavour::Markdown => scan_markdown(text, state),
        }
    }

    fn scan_code(&self, text: &str, mut state: State) -> (Vec<Span>, State) {
        let bytes = text.as_bytes();
        let mut spans: Vec<Span> = Vec::new();
        let mut i = 0usize;

        // A line that begins inside something started earlier: finish that first.
        match state.carry {
            Carry::Block => {
                let (end, done) = self.close_block(text, 0, &mut state);
                spans.push(Span {
                    start: 0,
                    end,
                    class: Class::Comment,
                });
                if !done {
                    return (spans, state);
                }
                i = end;
            }
            Carry::Str(quote) => {
                let (end, closed) = scan_string_body(text, 0, quote, self.escape);
                spans.push(Span {
                    start: 0,
                    end,
                    class: Class::Str,
                });
                if !closed {
                    return (spans, state);
                }
                state.carry = Carry::None;
                i = end;
            }
            Carry::Fence | Carry::None => {}
        }

        // The shebang is a comment, but only on the first line and only at its very start.
        if i == 0 && text.starts_with("#!") {
            spans.push(Span {
                start: 0,
                end: text.len(),
                class: Class::Preproc,
            });
            return (spans, state);
        }

        while i < bytes.len() {
            let rest = &text[i..];

            // A line comment runs to the end and nothing in it matters.
            if let Some(marker) = self.line_comment.iter().find(|m| rest.starts_with(**m)) {
                let _ = marker;
                spans.push(Span {
                    start: i,
                    end: text.len(),
                    class: Class::Comment,
                });
                return (spans, state);
            }

            if let Some((open, _)) = self.block_comment {
                if rest.starts_with(open) {
                    state.carry = Carry::Block;
                    state.depth = 1;
                    let (end, done) = self.close_block(text, i + open.len(), &mut state);
                    spans.push(Span {
                        start: i,
                        end,
                        class: Class::Comment,
                    });
                    if !done {
                        return (spans, state);
                    }
                    i = end;
                    continue;
                }
            }

            let ch = rest.chars().next().unwrap_or('\0');

            if self.quotes.contains(ch) {
                let (end, closed) = scan_string_body(text, i + ch.len_utf8(), ch, self.escape);
                spans.push(Span {
                    start: i,
                    end,
                    class: Class::Str,
                });
                if !closed {
                    // Only some languages let a string run on; in the rest an unclosed quote
                    // is a mistake on this line and the next line starts clean.
                    state.carry = if self.multiline_strings {
                        Carry::Str(ch)
                    } else {
                        Carry::None
                    };
                    return (spans, state);
                }
                i = end;
                continue;
            }

            if self.preproc && ch == '#' && text[..i].trim().is_empty() {
                spans.push(Span {
                    start: i,
                    end: text.len(),
                    class: Class::Preproc,
                });
                return (spans, state);
            }

            if ch.is_ascii_digit() && !is_word_byte(prev_byte(bytes, i)) {
                let end = scan_number(text, i);
                spans.push(Span {
                    start: i,
                    end,
                    class: Class::Number,
                });
                i = end;
                continue;
            }

            if is_word_start(ch) {
                let end = scan_word(text, i);
                let word = &text[i..end];
                let class = if self.keywords.contains(&word) {
                    Some(Class::Keyword)
                } else if self.types.contains(&word) {
                    Some(Class::Type)
                } else if is_call(text, end) {
                    Some(Class::Function)
                } else {
                    None
                };
                if let Some(class) = class {
                    spans.push(Span {
                        start: i,
                        end,
                        class,
                    });
                }
                i = end;
                continue;
            }

            i += ch.len_utf8();
        }
        (spans, state)
    }

    /// Where the block comment that is open ends, and whether it ended at all.
    fn close_block(&self, text: &str, from: usize, state: &mut State) -> (usize, bool) {
        let Some((open, close)) = self.block_comment else {
            state.carry = Carry::None;
            return (text.len(), true);
        };
        let bytes = text.as_bytes();
        let mut i = from;
        while i < bytes.len() {
            if text[i..].starts_with(close) {
                state.depth = state.depth.saturating_sub(1);
                i += close.len();
                if state.depth == 0 {
                    state.carry = Carry::None;
                    return (i, true);
                }
                continue;
            }
            if self.nested_comments && text[i..].starts_with(open) {
                state.depth = state.depth.saturating_add(1);
                i += open.len();
                continue;
            }
            i = step(text, i);
        }
        state.carry = Carry::Block;
        if state.depth == 0 {
            state.depth = 1;
        }
        (text.len(), false)
    }

    /// `<tag attr="value">text</tag>`, `<!-- … -->`, `&entity;`.
    fn scan_markup(&self, text: &str, mut state: State) -> (Vec<Span>, State) {
        let mut spans = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0usize;

        if state.carry == Carry::Block {
            match text.find("-->") {
                Some(at) => {
                    spans.push(Span {
                        start: 0,
                        end: at + 3,
                        class: Class::Comment,
                    });
                    state.carry = Carry::None;
                    i = at + 3;
                }
                None => {
                    spans.push(Span {
                        start: 0,
                        end: text.len(),
                        class: Class::Comment,
                    });
                    return (spans, state);
                }
            }
        }

        while i < bytes.len() {
            let rest = &text[i..];
            if rest.starts_with("<!--") {
                match rest.find("-->") {
                    Some(at) => {
                        spans.push(Span {
                            start: i,
                            end: i + at + 3,
                            class: Class::Comment,
                        });
                        i += at + 3;
                    }
                    None => {
                        spans.push(Span {
                            start: i,
                            end: text.len(),
                            class: Class::Comment,
                        });
                        state.carry = Carry::Block;
                        return (spans, state);
                    }
                }
                continue;
            }
            if bytes[i] == b'<' {
                let end = text[i..]
                    .find('>')
                    .map(|at| i + at + 1)
                    .unwrap_or(text.len());
                spans.extend(scan_tag(text, i, end));
                i = end;
                continue;
            }
            if bytes[i] == b'&' {
                if let Some(at) = text[i..].find(';') {
                    if at <= 10 {
                        spans.push(Span {
                            start: i,
                            end: i + at + 1,
                            class: Class::Entity,
                        });
                        i += at + 1;
                        continue;
                    }
                }
            }
            i = step(text, i);
        }
        (spans, state)
    }
}

/// The inside of a tag: its name, then the attribute names and the quoted values.
fn scan_tag(text: &str, start: usize, end: usize) -> Vec<Span> {
    let mut spans = vec![Span {
        start,
        end,
        class: Class::Tag,
    }];
    let inner = &text[start..end];
    // `<`, then an optional `/` or `!` or `?`, then the name.
    let name_from = 1 + inner[1..]
        .find(|c: char| c.is_alphanumeric() || c == '_')
        .unwrap_or(0);
    let name_to = name_from
        + inner[name_from..]
            .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == ':'))
            .unwrap_or(inner.len() - name_from);
    if name_to > name_from {
        spans.push(Span {
            start: start + name_from,
            end: start + name_to,
            class: Class::Type,
        });
    }

    let bytes = inner.as_bytes();
    let mut i = name_to;
    while i < bytes.len() {
        if !inner.is_char_boundary(i) {
            i += 1;
            continue;
        }
        let ch = bytes[i] as char;
        if ch == '"' || ch == '\'' {
            let (stop, _) = scan_string_body(inner, i + 1, ch, Some('\\'));
            spans.push(Span {
                start: start + i,
                end: start + stop,
                class: Class::Str,
            });
            i = stop;
            continue;
        }
        if ch.is_alphabetic() || ch == '_' {
            let stop = i + inner[i..]
                .find(|c: char| !(c.is_alphanumeric() || c == '_' || c == '-' || c == ':'))
                .unwrap_or(inner.len() - i);
            spans.push(Span {
                start: start + i,
                end: start + stop,
                class: Class::Attr,
            });
            i = stop;
            continue;
        }
        i = step(inner, i);
    }
    spans
}

/// Markdown: the shape of the page rather than the language of a program.
fn scan_markdown(text: &str, mut state: State) -> (Vec<Span>, State) {
    let trimmed = text.trim_start();
    let indent = text.len() - trimmed.len();

    if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
        state.carry = match state.carry {
            Carry::Fence => Carry::None,
            _ => Carry::Fence,
        };
        return (
            vec![Span {
                start: 0,
                end: text.len(),
                class: Class::Comment,
            }],
            state,
        );
    }
    if state.carry == Carry::Fence {
        return (
            vec![Span {
                start: 0,
                end: text.len(),
                class: Class::Str,
            }],
            state,
        );
    }
    if trimmed.starts_with('#') {
        return (
            vec![Span {
                start: 0,
                end: text.len(),
                class: Class::Preproc,
            }],
            state,
        );
    }
    if trimmed.starts_with('>') {
        return (
            vec![Span {
                start: 0,
                end: text.len(),
                class: Class::Comment,
            }],
            state,
        );
    }

    let mut spans = Vec::new();
    // A list bullet or a numbered item: the marker only.
    if let Some(rest) = trimmed.strip_prefix(['-', '*', '+']) {
        if rest.starts_with(' ') {
            spans.push(Span {
                start: indent,
                end: indent + 1,
                class: Class::Keyword,
            });
        }
    }

    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        match bytes[i] {
            b'`' => {
                let end = text[i + 1..]
                    .find('`')
                    .map(|at| i + at + 2)
                    .unwrap_or(text.len());
                spans.push(Span {
                    start: i,
                    end,
                    class: Class::Str,
                });
                i = end;
            }
            b'[' => {
                // `[text](target)`: the target is the part worth finding again.
                let close = text[i..].find(']').map(|at| i + at);
                match close {
                    Some(close) if text[close + 1..].starts_with('(') => {
                        let end = text[close..]
                            .find(')')
                            .map(|at| close + at + 1)
                            .unwrap_or(text.len());
                        spans.push(Span {
                            start: i,
                            end: close + 1,
                            class: Class::Type,
                        });
                        spans.push(Span {
                            start: close + 1,
                            end,
                            class: Class::Function,
                        });
                        i = end;
                    }
                    _ => i = step(text, i),
                }
            }
            b'*' | b'_' if emphasis_at(text, i) => {
                let marker = bytes[i];
                let double = bytes.get(i + 1) == Some(&marker);
                let width = if double { 2 } else { 1 };
                let needle = if double {
                    String::from_utf8(vec![marker, marker]).unwrap_or_default()
                } else {
                    (marker as char).to_string()
                };
                match text[i + width..].find(&needle) {
                    Some(at) => {
                        let end = i + width + at + width;
                        spans.push(Span {
                            start: i,
                            end,
                            class: Class::Type,
                        });
                        i = end;
                    }
                    None => i = step(text, i),
                }
            }
            _ => i = step(text, i),
        }
    }
    spans.sort_by_key(|s| s.start);
    (spans, state)
}

/// `*` starts emphasis only where a word could start: `a*b` in the middle of a word is not
/// emphasis, and neither is a `*` with a space after it, which is a bullet.
fn emphasis_at(text: &str, at: usize) -> bool {
    let bytes = text.as_bytes();
    let after = bytes.get(at + 1).copied().unwrap_or(b' ');
    let after = if after == bytes[at] {
        bytes.get(at + 2).copied().unwrap_or(b' ')
    } else {
        after
    };
    !after.is_ascii_whitespace()
}

/// From just past the opening quote to just past the closing one. The `bool` says whether the
/// closing quote was really there.
fn scan_string_body(text: &str, from: usize, quote: char, escape: Option<char>) -> (usize, bool) {
    let mut chars = text[from..].char_indices();
    while let Some((offset, ch)) = chars.next() {
        if Some(ch) == escape {
            // The escaped character, whatever it is, is part of the string.
            if chars.next().is_none() {
                return (text.len(), false);
            }
            continue;
        }
        if ch == quote {
            return (from + offset + ch.len_utf8(), true);
        }
    }
    (text.len(), false)
}

fn scan_number(text: &str, from: usize) -> usize {
    let mut end = from;
    for (offset, ch) in text[from..].char_indices() {
        let keep = ch.is_ascii_alphanumeric()
            || ch == '.'
            || ch == '_'
            // An exponent's sign, but only straight after an `e`.
            || ((ch == '+' || ch == '-')
                && text[..from + offset].ends_with(['e', 'E'])
                && text[from..from + offset].len() > 1);
        if !keep {
            break;
        }
        end = from + offset + ch.len_utf8();
    }
    end
}

fn scan_word(text: &str, from: usize) -> usize {
    let mut end = text.len();
    for (offset, ch) in text[from..].char_indices() {
        if !is_word_char(ch) {
            end = from + offset;
            break;
        }
    }
    end
}

/// A name immediately followed by `(` is being called, which is worth seeing.
fn is_call(text: &str, after: usize) -> bool {
    text[after..].starts_with('(')
}

fn is_word_start(ch: char) -> bool {
    ch.is_alphabetic() || ch == '_' || ch == '$' || ch == '@'
}

fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_' || ch == '$'
}

fn is_word_byte(byte: Option<u8>) -> bool {
    byte.map(|b| b.is_ascii_alphanumeric() || b == b'_')
        .unwrap_or(false)
}

fn prev_byte(bytes: &[u8], at: usize) -> Option<u8> {
    at.checked_sub(1).map(|i| bytes[i])
}

/// One character on, from a byte offset that is on a boundary.
fn step(text: &str, at: usize) -> usize {
    at + text[at..].chars().next().map(char::len_utf8).unwrap_or(1)
}

/// The state every file starts in.
pub fn start() -> State {
    State::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classes(syntax: &Syntax, line: &str) -> Vec<(String, Class)> {
        let (spans, _) = syntax.line(line, State::default());
        spans
            .into_iter()
            .map(|s| (line[s.start..s.end].to_string(), s.class))
            .collect()
    }

    #[test]
    fn a_file_is_recognised_by_its_name() {
        assert_eq!(of_name("main.rs").map(|s| s.name), Some("Rust"));
        assert_eq!(of_name("index.html").map(|s| s.name), Some("HTML"));
        assert_eq!(of_name("README.md").map(|s| s.name), Some("Markdown"));
        assert_eq!(of_name("Cargo.toml").map(|s| s.name), Some("TOML"));
        assert_eq!(of_name("Makefile").map(|s| s.name), Some("Make"));
        assert_eq!(of_name("Dockerfile").map(|s| s.name), Some("Dockerfile"));
        assert_eq!(of_name(".gitignore"), None);
        assert_eq!(of_name("photo.png"), None);
        // The name is matched without regard to case, as a filesystem may store it.
        assert_eq!(of_name("MAIN.RS").map(|s| s.name), Some("Rust"));
    }

    #[test]
    fn the_pieces_of_a_line_of_code_are_told_apart() {
        let rust = of_name("main.rs").unwrap();
        let got = classes(rust, r#"    let n: u32 = count(3) + 0xff; // why"#);
        assert_eq!(
            got,
            vec![
                ("let".to_string(), Class::Keyword),
                ("u32".to_string(), Class::Type),
                ("count".to_string(), Class::Function),
                ("3".to_string(), Class::Number),
                ("0xff".to_string(), Class::Number),
                ("// why".to_string(), Class::Comment),
            ]
        );
    }

    #[test]
    fn a_string_swallows_what_looks_like_code() {
        let rust = of_name("main.rs").unwrap();
        let got = classes(rust, r#"let s = "fn main() // not a comment";"#);
        assert_eq!(got[0], ("let".to_string(), Class::Keyword));
        assert_eq!(
            got[1],
            (r#""fn main() // not a comment""#.to_string(), Class::Str)
        );
        assert_eq!(got.len(), 2, "{got:?}");

        // An escaped quote does not end it.
        let got = classes(rust, r#""a \" b" + 1"#);
        assert_eq!(got[0], (r#""a \" b""#.to_string(), Class::Str));
        assert_eq!(got[1], ("1".to_string(), Class::Number));
    }

    #[test]
    fn a_block_comment_carries_to_the_lines_after_it() {
        let c = of_name("main.c").unwrap();
        let (spans, state) = c.line("int x; /* here", State::default());
        assert_eq!(spans.last().unwrap().class, Class::Comment);
        assert_eq!(state.carry, Carry::Block);

        let (spans, state) = c.line("still a comment", state);
        assert_eq!(
            spans,
            vec![Span {
                start: 0,
                end: 15,
                class: Class::Comment
            }]
        );
        assert_eq!(state.carry, Carry::Block);

        let (spans, state) = c.line("*/ int y = 2;", state);
        assert_eq!(spans[0].class, Class::Comment);
        assert_eq!(spans[0].end, 2);
        assert_eq!(state.carry, Carry::None);
        assert!(spans.iter().any(|s| s.class == Class::Number));
    }

    /// Rust nests block comments and C does not, which is a real difference and the one
    /// place where getting it wrong colours the rest of a file.
    #[test]
    fn rust_nests_block_comments_and_c_does_not() {
        let rust = of_name("main.rs").unwrap();
        let (_, state) = rust.line("/* outer /* inner */", State::default());
        assert_eq!(state.carry, Carry::Block, "rust should still be inside");
        let (_, state) = rust.line("*/ done", state);
        assert_eq!(state.carry, Carry::None);

        let c = of_name("main.c").unwrap();
        let (_, state) = c.line("/* outer /* inner */", State::default());
        assert_eq!(state.carry, Carry::None, "C closes at the first `*/`");
    }

    #[test]
    fn markup_separates_the_tag_from_its_attributes() {
        let html = of_name("index.html").unwrap();
        let line = r#"<a href="x.html" id=main>&amp;</a>"#;
        let (spans, _) = html.line(line, State::default());
        let named: Vec<(&str, Class)> = spans
            .iter()
            .map(|s| (&line[s.start..s.end], s.class))
            .collect();
        assert!(named.contains(&("a", Class::Type)), "{named:?}");
        assert!(named.contains(&("href", Class::Attr)), "{named:?}");
        assert!(named.contains(&(r#""x.html""#, Class::Str)), "{named:?}");
        assert!(named.contains(&("&amp;", Class::Entity)), "{named:?}");
    }

    #[test]
    fn an_html_comment_carries_too() {
        let html = of_name("page.html").unwrap();
        let (spans, state) = html.line("<p>x</p> <!-- todo", State::default());
        assert_eq!(spans.last().unwrap().class, Class::Comment);
        assert_eq!(state.carry, Carry::Block);
        let (spans, state) = html.line("still --> <b>", state);
        assert_eq!(spans[0].class, Class::Comment);
        assert_eq!(state.carry, Carry::None);
        assert!(spans.iter().any(|s| s.class == Class::Tag));
    }

    #[test]
    fn markdown_knows_a_heading_from_a_fence() {
        let md = of_name("README.md").unwrap();
        let (spans, _) = md.line("# Title", State::default());
        assert_eq!(spans[0].class, Class::Preproc);

        let (_, state) = md.line("```rust", State::default());
        assert_eq!(state.carry, Carry::Fence);
        let (spans, state) = md.line("fn main() {}", state);
        assert_eq!(spans[0].class, Class::Str, "inside a fence it is all code");
        assert_eq!(state.carry, Carry::Fence);
        let (_, state) = md.line("```", state);
        assert_eq!(state.carry, Carry::None);

        let line = "a `code` and [text](http://x) and *loud*";
        let (spans, _) = md.line(line, State::default());
        let named: Vec<(&str, Class)> = spans
            .iter()
            .map(|s| (&line[s.start..s.end], s.class))
            .collect();
        assert!(named.contains(&("`code`", Class::Str)), "{named:?}");
        assert!(
            named.contains(&("(http://x)", Class::Function)),
            "{named:?}"
        );
        assert!(named.contains(&("*loud*", Class::Type)), "{named:?}");
    }

    #[test]
    fn a_hash_comment_language_does_not_see_a_c_comment() {
        let py = of_name("script.py").unwrap();
        let got = classes(py, "x = 1  # really a comment");
        assert_eq!(got[0], ("1".to_string(), Class::Number));
        assert_eq!(got[1].1, Class::Comment);

        let sh = of_name("run.sh").unwrap();
        let got = classes(sh, "#!/bin/sh");
        assert_eq!(got[0].1, Class::Preproc);
    }

    #[test]
    fn a_key_in_a_data_file_is_a_key() {
        let toml = of_name("scmd.toml").unwrap();
        let line = r#"name = "scmd" # the crate"#;
        let got = classes(toml, line);
        assert!(got
            .iter()
            .any(|(t, c)| t == r#""scmd""# && *c == Class::Str));
        assert!(got.iter().any(|(_, c)| *c == Class::Comment));

        let json = of_name("package.json").unwrap();
        let got = classes(json, r#"{"name": "scmd", "count": 3, "ok": true}"#);
        assert!(got.iter().any(|(t, c)| t == "true" && *c == Class::Keyword));
        assert!(got.iter().any(|(t, c)| t == "3" && *c == Class::Number));
    }

    /// Every syntax has to survive the odd shapes a real file is full of, without panicking
    /// and without running away with the rest of the line.
    #[test]
    fn nothing_panics_on_the_awkward_lines() {
        let lines = [
            "",
            " ",
            "\"unterminated",
            "'",
            "/*",
            "*/",
            "<",
            "<<<",
            "&",
            "&nosemicolon",
            "```",
            "#",
            "0x",
            "1e+",
            "é à ü — a line of plain prose",
            "/* nested /* /* /*",
        ];
        for syntax in SYNTAXES {
            let mut state = State::default();
            for line in lines {
                let (spans, next) = syntax.line(line, state);
                for span in &spans {
                    assert!(
                        span.end <= line.len() && span.start <= span.end,
                        "{}: {line:?} gave {span:?}",
                        syntax.name
                    );
                    // The offsets must land on character boundaries, or the caller slicing
                    // the line with them would panic.
                    assert!(line.is_char_boundary(span.start) && line.is_char_boundary(span.end));
                }
                for pair in spans.windows(2) {
                    assert!(
                        pair[0].end <= pair[1].start,
                        "{}: overlapping spans in {line:?}",
                        syntax.name
                    );
                }
                state = next;
            }
        }
    }
}
