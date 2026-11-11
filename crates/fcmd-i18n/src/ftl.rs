//! The part of Fluent our catalogues are written in, read and formatted without a library.
//!
//! What a catalogue may hold: comments (`#`, `##`, `###`), messages `id = pattern`, and in a
//! pattern text, variables `{ $name }`, string literals `{ "text" }` and select expressions
//! `{ $n -> [one] … *[other] … }` whose keys are plural categories, numbers or words. A
//! pattern goes on over the indented lines below its first. Terms, attributes and functions
//! are not read: no catalogue of ours has them, and the test over every catalogue would say
//! so if one did. What is written outside that is kept as text rather than refused, so a
//! slip in a translation shows on the screen instead of hiding the message.

use std::borrow::Cow;
use std::collections::HashMap;

/// A value given to a message: a piece of text or a number.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Str(Cow<'a, str>),
    Num(f64),
}

impl Value<'_> {
    fn text(&self) -> Cow<'_, str> {
        match self {
            Value::Str(s) => Cow::Borrowed(s),
            Value::Num(n) => Cow::Owned(n.to_string()),
        }
    }
}

impl<'a> From<&'a str> for Value<'a> {
    fn from(s: &'a str) -> Self {
        Value::Str(Cow::Borrowed(s))
    }
}

impl<'a> From<&'a String> for Value<'a> {
    fn from(s: &'a String) -> Self {
        Value::Str(Cow::Borrowed(s))
    }
}

impl From<String> for Value<'_> {
    fn from(s: String) -> Self {
        Value::Str(Cow::Owned(s))
    }
}

impl<'a> From<Cow<'a, str>> for Value<'a> {
    fn from(s: Cow<'a, str>) -> Self {
        Value::Str(s)
    }
}

macro_rules! number {
    ($($t:ty),*) => {$(
        impl From<$t> for Value<'_> {
            fn from(n: $t) -> Self {
                Value::Num(n as f64)
            }
        }
    )*};
}
number!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, f32, f64);

#[derive(Debug, Clone, PartialEq)]
enum Key {
    Word(String),
    Num(f64),
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    Text(String),
    Var(String),
    Select {
        on: String,
        variants: Vec<(Key, Pattern)>,
        default: usize,
    },
}

type Pattern = Vec<Part>;

/// One language's messages.
#[derive(Debug, Default)]
pub struct Catalogue {
    lang: String,
    messages: HashMap<String, Pattern>,
}

impl Catalogue {
    pub fn parse(lang: &str, src: &str) -> Self {
        let mut messages = HashMap::new();
        let lines: Vec<&str> = src.lines().collect();
        let mut n = 0;
        while n < lines.len() {
            let line = lines[n];
            n += 1;
            let Some((id, first)) = message_start(line) else {
                continue;
            };
            // The pattern is the rest of this line and every indented line below it, with
            // the blank lines between them; it stops at the first line that starts at the
            // margin.
            let mut body = vec![first];
            let mut end = n;
            let mut k = n;
            while k < lines.len() {
                let l = lines[k];
                if l.trim().is_empty() {
                    k += 1;
                } else if l.starts_with([' ', '\t']) {
                    body.extend_from_slice(&lines[end..=k]);
                    k += 1;
                    end = k;
                } else {
                    break;
                }
            }
            n = end;
            let text = body.join("\n");
            let mut p = Parser {
                s: &text,
                at: 0,
                indented: false,
            };
            messages.insert(id.to_string(), p.pattern(Stop::End));
        }
        Catalogue {
            lang: lang.to_string(),
            messages,
        }
    }

    pub fn has(&self, id: &str) -> bool {
        self.messages.contains_key(id)
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.messages.keys().map(String::as_str)
    }

    /// The message filled in with `args`, or `None` when the catalogue does not have it. A
    /// variable that was not given is shown as `{$name}`, as Fluent shows it.
    pub fn format(&self, id: &str, args: &[(&str, Value<'_>)]) -> Option<String> {
        let pattern = self.messages.get(id)?;
        let mut out = String::new();
        self.write(pattern, args, &mut out);
        Some(out)
    }

    fn write(&self, pattern: &Pattern, args: &[(&str, Value<'_>)], out: &mut String) {
        let arg = |name: &str| args.iter().find(|(k, _)| *k == name).map(|(_, v)| v);
        for part in pattern {
            match part {
                Part::Text(t) => out.push_str(t),
                Part::Var(name) => match arg(name) {
                    Some(v) => out.push_str(&v.text()),
                    None => {
                        out.push_str("{$");
                        out.push_str(name);
                        out.push('}');
                    }
                },
                Part::Select {
                    on,
                    variants,
                    default,
                } => {
                    let chosen = arg(on)
                        .and_then(|v| variants.iter().position(|(k, _)| self.matches(k, v)))
                        .unwrap_or(*default);
                    self.write(&variants[chosen].1, args, out);
                }
            }
        }
    }

    fn matches(&self, key: &Key, value: &Value<'_>) -> bool {
        match (key, value) {
            (Key::Word(w), Value::Str(s)) => w == s,
            (Key::Num(k), Value::Num(n)) => k == n,
            (Key::Word(w), Value::Num(n)) => w == plural(&self.lang, *n),
            (Key::Num(k), Value::Str(s)) => s.parse::<f64>().is_ok_and(|n| n == *k),
        }
    }
}

/// `id = …` at the margin: the id, and what follows the `=`.
fn message_start(line: &str) -> Option<(&str, &str)> {
    if !line.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let (id, rest) = line.split_once('=')?;
    let id = id.trim_end();
    id.chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        .then_some((id, rest))
}

/// The CLDR cardinal category of `n` in `lang`, for the languages we ship.
fn plural(lang: &str, n: f64) -> &'static str {
    let abs = n.abs();
    let whole = abs.fract() == 0.0;
    let i = abs.trunc() as u64;
    match lang {
        "zh" => "other",
        // French counts 0 and 1, and their fractions, as one.
        "fr" => {
            if i < 2 {
                "one"
            } else {
                "other"
            }
        }
        "ru" => {
            if !whole {
                "other"
            } else if i % 10 == 1 && i % 100 != 11 {
                "one"
            } else if (2..=4).contains(&(i % 10)) && !(12..=14).contains(&(i % 100)) {
                "few"
            } else {
                "many"
            }
        }
        // English, Italian, German, Spanish: one is the whole number 1.
        _ => {
            if whole && i == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Stop {
    /// A message's own pattern: it runs to the end.
    End,
    /// A variant's pattern: it ends at the line that starts the next variant or closes the
    /// select.
    Variant,
}

struct Parser<'s> {
    s: &'s str,
    at: usize,
    /// Whether the text read so far starts on a line of its own, below the `=` or the `]`.
    indented: bool,
}

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.s[self.at..]
    }

    fn skip_blank(&mut self) {
        let r = self.rest();
        self.at += r.len() - r.trim_start().len();
    }

    fn skip_inline(&mut self) {
        let r = self.rest();
        self.at += r.len() - r.trim_start_matches([' ', '\t']).len();
    }

    /// A pattern, with Fluent's rules for where it starts and ends: the spaces after `=` are
    /// not part of it, nor are blank lines before its first text or after its last; each line
    /// below the first loses the indent the lines have in common.
    fn pattern(&mut self, stop: Stop) -> Pattern {
        self.skip_inline();
        if self.rest().starts_with('\n') {
            self.indented = true;
        }
        let first_line_inline = !self.indented;
        if self.indented {
            // The text starts below: blank lines before it are not part of it.
            self.skip_blank_lines();
        }
        let mut lines: Vec<Vec<Piece>> = vec![Vec::new()];
        loop {
            let r = self.rest();
            let Some(c) = r.chars().next() else { break };
            match c {
                '\n' => {
                    if stop == Stop::Variant {
                        let next = r[1..].trim_start();
                        if next.starts_with('[') || next.starts_with("*[") || next.starts_with('}')
                        {
                            break;
                        }
                    }
                    self.at += 1;
                    lines.push(Vec::new());
                }
                '{' => {
                    self.at += 1;
                    let part = self.placeable();
                    lines.last_mut().expect("a line").push(Piece::Part(part));
                }
                '}' if stop == Stop::Variant => break,
                _ => {
                    let len = r.find(['\n', '{', '}']).unwrap_or(r.len());
                    // A `}` outside a placeable is text in a message's own pattern.
                    let len = if len == 0 { c.len_utf8() } else { len };
                    lines
                        .last_mut()
                        .expect("a line")
                        .push(Piece::Text(r[..len].to_string()));
                    self.at += len;
                }
            }
        }
        dedent(lines, first_line_inline)
    }

    fn skip_blank_lines(&mut self) {
        loop {
            let r = self.rest();
            let line_end = r.find('\n');
            match line_end {
                Some(e) if r[..e].trim().is_empty() => self.at += e + 1,
                _ => break,
            }
        }
    }

    /// What is between `{` and its `}`.
    fn placeable(&mut self) -> Part {
        self.skip_blank();
        let r = self.rest();
        let part = if let Some(lit) = r.strip_prefix('"') {
            let end = lit.find('"').unwrap_or(lit.len());
            let text = unescape(&lit[..end]);
            self.at += 1 + end + 1;
            Part::Text(text)
        } else {
            let name_start = usize::from(r.starts_with('$'));
            let name_len = r[name_start..]
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
                .unwrap_or(r.len() - name_start);
            let name = r[name_start..name_start + name_len].to_string();
            self.at += name_start + name_len;
            self.skip_blank();
            if self.rest().starts_with("->") {
                self.at += 2;
                return self.select(name);
            }
            Part::Var(name)
        };
        self.skip_blank();
        if self.rest().starts_with('}') {
            self.at += 1;
        }
        part
    }

    fn select(&mut self, on: String) -> Part {
        let mut variants = Vec::new();
        let mut default = 0;
        loop {
            self.skip_blank();
            let r = self.rest();
            let star = r.starts_with('*');
            if star {
                self.at += 1;
            }
            if !self.rest().starts_with('[') {
                break;
            }
            let close = self.rest().find(']').unwrap_or(self.rest().len());
            let key = self.rest()[1..close].trim().to_string();
            self.at += close + 1;
            let key = match key.parse::<f64>() {
                Ok(n) => Key::Num(n),
                Err(_) => Key::Word(key),
            };
            if star {
                default = variants.len();
            }
            self.indented = false;
            let value = self.pattern(Stop::Variant);
            variants.push((key, value));
        }
        if self.rest().starts_with('}') {
            self.at += 1;
        }
        Part::Select {
            on,
            variants,
            default,
        }
    }
}

enum Piece {
    Text(String),
    Part(Part),
}

/// The lines of a pattern put together: the indent the lines below the first have in common
/// taken off them, blank lines at the end dropped, the spaces at its very end trimmed.
fn dedent(mut lines: Vec<Vec<Piece>>, first_line_inline: bool) -> Pattern {
    let indent_of = |line: &[Piece]| match line.first() {
        Some(Piece::Text(t)) => t.len() - t.trim_start_matches([' ', '\t']).len(),
        _ => 0,
    };
    let blank = |line: &[Piece]| {
        line.iter()
            .all(|p| matches!(p, Piece::Text(t) if t.trim().is_empty()))
    };
    while lines.len() > 1 && lines.last().is_some_and(|l| blank(l)) {
        lines.pop();
    }
    let skip = usize::from(first_line_inline);
    let common = lines
        .iter()
        .skip(skip)
        .filter(|l| !blank(l))
        .map(|l| indent_of(l))
        .min()
        .unwrap_or(0);
    let mut out: Pattern = Vec::new();
    let push_text = |out: &mut Pattern, t: &str| {
        if t.is_empty() {
            return;
        }
        if let Some(Part::Text(prev)) = out.last_mut() {
            prev.push_str(t);
        } else {
            out.push(Part::Text(t.to_string()));
        }
    };
    for (n, line) in lines.into_iter().enumerate() {
        if n > 0 {
            push_text(&mut out, "\n");
        }
        let cut = if n >= skip { common } else { 0 };
        for (k, piece) in line.into_iter().enumerate() {
            match piece {
                Piece::Text(t) if k == 0 => {
                    let cut = cut.min(t.len() - t.trim_start_matches([' ', '\t']).len());
                    push_text(&mut out, &t[cut..]);
                }
                Piece::Text(t) => push_text(&mut out, &t),
                Piece::Part(Part::Text(t)) => {
                    // A literal is text, but it is not trimmed with the text around it.
                    out.push(Part::Text(t));
                }
                Piece::Part(p) => out.push(p),
            }
        }
    }
    if let Some(Part::Text(t)) = out.last_mut() {
        let trimmed = t.trim_end().len();
        t.truncate(trimmed);
        if t.is_empty() {
            out.pop();
        }
    }
    out
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some('U') => {
                let hex: String = chars.by_ref().take(6).collect();
                out.extend(u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(lang: &str, src: &str, args: &[(&str, Value<'_>)]) -> String {
        Catalogue::parse(lang, src).format("m", args).expect("m")
    }

    #[test]
    fn text_and_variables() {
        assert_eq!(one("en", "m = hello", &[]), "hello");
        assert_eq!(
            one("en", "m = to { $who }!  ", &[("who", "you".into())]),
            "to you!"
        );
        assert_eq!(one("en", "m = { $nobody }", &[]), "{$nobody}");
        assert_eq!(one("en", "m = 5 { \"{\" } x", &[]), "5 { x");
    }

    #[test]
    fn a_pattern_goes_on_over_indented_lines() {
        let src = "m =\n    one\n      two\n\n    three\n\nnext = x\n";
        assert_eq!(one("en", src, &[]), "one\n  two\n\nthree");
        let src = "m = first\n    second\n";
        assert_eq!(one("en", src, &[]), "first\nsecond");
    }

    #[test]
    fn plurals_follow_each_language() {
        let src =
            "m = { $n ->\n    [one] one\n    [few] few\n    [many] many\n   *[other] other\n}";
        let cat = |lang: &str, n: f64| one(lang, src, &[("n", n.into())]);
        assert_eq!(cat("en", 1.0), "one");
        assert_eq!(cat("en", 0.0), "other");
        assert_eq!(cat("en", 1.5), "other");
        assert_eq!(cat("fr", 0.0), "one");
        assert_eq!(cat("fr", 1.5), "one");
        assert_eq!(cat("fr", 2.0), "other");
        assert_eq!(cat("ru", 21.0), "one");
        assert_eq!(cat("ru", 11.0), "many");
        assert_eq!(cat("ru", 23.0), "few");
        assert_eq!(cat("ru", 13.0), "many");
        assert_eq!(cat("ru", 25.0), "many");
        assert_eq!(cat("ru", 2.5), "other");
        assert_eq!(cat("zh", 1.0), "other");
    }

    #[test]
    fn a_select_sits_inside_its_text() {
        let src =
            "m = Drop { $n ->\n        [one] { $n } file\n       *[other] { $n } files\n    }?\n";
        assert_eq!(one("en", src, &[("n", 1.into())]), "Drop 1 file?");
        assert_eq!(one("en", src, &[("n", 3.into())]), "Drop 3 files?");
        assert_eq!(one("en", src, &[]), "Drop {$n} files?");
        let words = "m = { $k ->\n    [a] alpha\n    [3] three\n   *[b] beta\n}";
        assert_eq!(one("en", words, &[("k", "a".into())]), "alpha");
        assert_eq!(one("en", words, &[("k", "z".into())]), "beta");
        assert_eq!(one("en", words, &[("k", 3.into())]), "three");
    }

    /// Checked against the `fluent-bundle` crate when it was replaced, with every message of
    /// every catalogue and these.
    #[test]
    fn variants_over_lines_and_escapes() {
        let src =
            "m = { $k ->\n    [a]\n        alpha\n          more\n   *[b] beta\n         two\n}";
        assert_eq!(one("en", src, &[("k", "a".into())]), "alpha\n  more");
        assert_eq!(one("en", src, &[]), "beta\ntwo");
        let src = "m =\n\n\n    lead { $k } tail\n    x\n";
        assert_eq!(one("en", src, &[("k", 1.into())]), "lead 1 tail\nx");
        assert_eq!(
            one("en", "m = a \"q\" \\u0041 { \"\\u0041\\\\\" }", &[]),
            "a \"q\" \\u0041 A\\"
        );
    }

    #[test]
    fn comments_and_the_margin_end_a_message() {
        let cat = Catalogue::parse("en", "# c\n## group\na = 1\n# about b\nb = 2\n");
        assert_eq!(cat.format("a", &[]).as_deref(), Some("1"));
        assert_eq!(cat.format("b", &[]).as_deref(), Some("2"));
        assert_eq!(cat.format("c", &[]), None);
    }

    #[test]
    fn numbers_print_as_fluent_prints_them() {
        assert_eq!(one("en", "m = { $n }", &[("n", 1234.into())]), "1234");
        assert_eq!(one("en", "m = { $n }", &[("n", 2.5.into())]), "2.5");
        assert_eq!(one("en", "m = { $n }", &[("n", (-3i64).into())]), "-3");
    }
}
