//! The IMAP response grammar this client needs, as plain functions over bytes.
//!
//! A response is a sequence of atoms, quoted strings, literals (`{n}` and n bytes) and
//! parenthesised lists. `BODY[...]` is one atom however many spaces and parentheses its
//! section holds, which is how the protocol means it to be read.

/// One element of a response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Atom(String),
    Str(Vec<u8>),
    List(Vec<Item>),
}

impl Item {
    fn text(&self) -> Option<String> {
        match self {
            Item::Atom(a) if a.eq_ignore_ascii_case("NIL") => None,
            Item::Atom(a) => Some(a.clone()),
            Item::Str(s) => Some(String::from_utf8_lossy(s).into_owned()),
            Item::List(_) => None,
        }
    }
}

/// `{123}` (or `{123+}`) at the end of a line: a literal of that many bytes follows.
pub fn literal_length(line: &str) -> Option<usize> {
    let open = line.rfind('{')?;
    let inner = line[open + 1..].strip_suffix('}')?;
    inner.trim_end_matches('+').parse().ok()
}

/// A string as a quoted IMAP string.
pub fn quote(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

/// Split one response into its items.
pub fn items(raw: &[u8]) -> Vec<Item> {
    let mut at = 0;
    let mut stack: Vec<Vec<Item>> = vec![Vec::new()];
    while at < raw.len() {
        let b = raw[at];
        match b {
            b' ' | b'\r' | b'\n' => at += 1,
            b'(' => {
                stack.push(Vec::new());
                at += 1;
            }
            b')' => {
                at += 1;
                if stack.len() > 1 {
                    let list = stack.pop().unwrap_or_default();
                    if let Some(top) = stack.last_mut() {
                        top.push(Item::List(list));
                    }
                }
            }
            b'"' => {
                at += 1;
                let mut s = Vec::new();
                while at < raw.len() && raw[at] != b'"' {
                    if raw[at] == b'\\' && at + 1 < raw.len() {
                        at += 1;
                    }
                    s.push(raw[at]);
                    at += 1;
                }
                at += 1;
                if let Some(top) = stack.last_mut() {
                    top.push(Item::Str(s));
                }
            }
            b'{' => {
                let close = raw[at..].iter().position(|&c| c == b'}').map(|p| at + p);
                let Some(close) = close else { break };
                let n: usize = String::from_utf8_lossy(&raw[at + 1..close])
                    .trim_end_matches('+')
                    .parse()
                    .unwrap_or(0);
                // The literal starts after the CRLF that ends the line.
                let mut start = close + 1;
                if raw.get(start) == Some(&b'\r') {
                    start += 1;
                }
                if raw.get(start) == Some(&b'\n') {
                    start += 1;
                }
                let end = (start + n).min(raw.len());
                if let Some(top) = stack.last_mut() {
                    top.push(Item::Str(raw[start..end].to_vec()));
                }
                at = end;
            }
            _ => {
                let start = at;
                let mut depth = 0usize;
                while at < raw.len() {
                    let c = raw[at];
                    if c == b'[' {
                        depth += 1;
                    } else if c == b']' {
                        depth = depth.saturating_sub(1);
                    } else if depth == 0 && matches!(c, b' ' | b'(' | b')' | b'\r' | b'\n') {
                        break;
                    }
                    at += 1;
                }
                if let Some(top) = stack.last_mut() {
                    top.push(Item::Atom(
                        String::from_utf8_lossy(&raw[start..at]).into_owned(),
                    ));
                }
            }
        }
    }
    while stack.len() > 1 {
        let list = stack.pop().unwrap_or_default();
        if let Some(top) = stack.last_mut() {
            top.push(Item::List(list));
        }
    }
    stack.pop().unwrap_or_default()
}

/// One row of `LIST`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mailbox {
    pub name: String,
    pub delimiter: Option<char>,
    /// `\Noselect` says the name is only a level of the hierarchy, holding no messages.
    pub selectable: bool,
}

pub fn list_line(raw: &[u8]) -> Option<Mailbox> {
    let items = items(raw);
    match items.as_slice() {
        [Item::Atom(star), Item::Atom(list), Item::List(flags), delim, name, ..]
            if star == "*" && list.eq_ignore_ascii_case("LIST") =>
        {
            let selectable = !flags.iter().any(|f| {
                matches!(f, Item::Atom(a) if a.eq_ignore_ascii_case("\\Noselect")
                    || a.eq_ignore_ascii_case("\\NonExistent"))
            });
            Some(Mailbox {
                name: name.text()?,
                delimiter: delim.text().and_then(|d| d.chars().next()),
                selectable,
            })
        }
        _ => None,
    }
}

/// What the panel shows of one message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub uid: u32,
    pub size: u64,
    pub arrived: Option<u32>,
    pub subject: String,
}

impl Summary {
    /// `<uid> <subject>.eml`, the subject made safe for a file name and kept short.
    pub fn file_name(&self) -> String {
        let clean: String = self
            .subject
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
                c if c.is_control() => ' ',
                c => c,
            })
            .collect();
        let clean = clean.split_whitespace().collect::<Vec<_>>().join(" ");
        let short: String = clean.chars().take(80).collect();
        if short.is_empty() {
            format!("{}.eml", self.uid)
        } else {
            format!("{} {}.eml", self.uid, short.trim_end_matches('.'))
        }
    }
}

/// The pairs of a `* n FETCH (...)` response.
fn fetch_pairs(raw: &[u8]) -> Option<Vec<Item>> {
    let items = items(raw);
    match items.as_slice() {
        [Item::Atom(star), Item::Atom(_), Item::Atom(fetch), Item::List(pairs), ..]
            if star == "*" && fetch.eq_ignore_ascii_case("FETCH") =>
        {
            Some(pairs.clone())
        }
        _ => None,
    }
}

pub fn summary(raw: &[u8]) -> Option<Summary> {
    let pairs = fetch_pairs(raw)?;
    let mut out = Summary {
        uid: 0,
        size: 0,
        arrived: None,
        subject: String::new(),
    };
    for pair in pairs.chunks(2) {
        let [Item::Atom(key), value] = pair else {
            continue;
        };
        let key = key.to_ascii_uppercase();
        match key.as_str() {
            "UID" => out.uid = value.text()?.parse().ok()?,
            "RFC822.SIZE" => out.size = value.text()?.parse().ok()?,
            "INTERNALDATE" => out.arrived = value.text().and_then(|d| internal_date(&d)),
            k if k.starts_with("BODY[HEADER.FIELDS") => {
                out.subject = subject_of(&value.text().unwrap_or_default());
            }
            _ => {}
        }
    }
    (out.uid != 0).then_some(out)
}

/// The message bytes of a `BODY[]` fetch.
pub fn body(raw: &[u8]) -> Option<Vec<u8>> {
    let pairs = fetch_pairs(raw)?;
    pairs.chunks(2).find_map(|pair| match pair {
        [Item::Atom(key), Item::Str(bytes)] if key.eq_ignore_ascii_case("BODY[]") => {
            Some(bytes.clone())
        }
        _ => None,
    })
}

/// The subject out of a `Subject:` header block, unfolded and with its encoded words read.
fn subject_of(headers: &str) -> String {
    let mut value = String::new();
    let mut inside = false;
    for line in headers.lines() {
        if let Some(rest) = line
            .get(..8)
            .filter(|h| h.eq_ignore_ascii_case("subject:"))
            .map(|_| &line[8..])
        {
            inside = true;
            value.push_str(rest.trim());
        } else if inside && line.starts_with([' ', '\t']) {
            value.push(' ');
            value.push_str(line.trim());
        } else {
            inside = false;
        }
    }
    fcmd_arc::mail::decode_words(value.trim())
}

/// The uid at the front of a message's file name.
pub fn uid_of_name(name: &str) -> Option<u32> {
    let digits: String = name.chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().ok().filter(|uid| *uid > 0)
}

/// `05-Nov-2026 10:00:00 +0100` as seconds since 1970.
pub fn internal_date(text: &str) -> Option<u32> {
    let text = text.trim();
    let (date, rest) = text.split_once(' ')?;
    let (time, zone) = rest.split_once(' ')?;
    let mut d = date.split('-');
    let day: i64 = d.next()?.trim().parse().ok()?;
    let month = match d.next()?.to_ascii_lowercase().as_str() {
        "jan" => 1,
        "feb" => 2,
        "mar" => 3,
        "apr" => 4,
        "may" => 5,
        "jun" => 6,
        "jul" => 7,
        "aug" => 8,
        "sep" => 9,
        "oct" => 10,
        "nov" => 11,
        "dec" => 12,
        _ => return None,
    };
    let year: i64 = d.next()?.parse().ok()?;
    let mut t = time.split(':');
    let (h, m, s): (i64, i64, i64) = (
        t.next()?.parse().ok()?,
        t.next()?.parse().ok()?,
        t.next()?.parse().ok()?,
    );
    let sign = if zone.starts_with('-') { -1 } else { 1 };
    let zone = zone.trim_start_matches(['+', '-']);
    let offset =
        zone.get(..2)?.parse::<i64>().ok()? * 3600 + zone.get(2..4)?.parse::<i64>().ok()? * 60;
    // Days from 1970-01-01 to the date, by the civil-from-days algorithm turned round.
    let (y, mo) = if month <= 2 {
        (year - 1, month + 9)
    } else {
        (year, month - 3)
    };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * mo + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    let secs = days * 86_400 + h * 3600 + m * 60 + s - sign * offset;
    u32::try_from(secs).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_line_is_read_with_its_flags_and_separator() {
        let m = list_line(b"* LIST (\\HasNoChildren) \"/\" \"INBOX\"\r\n").unwrap();
        assert_eq!(m.name, "INBOX");
        assert_eq!(m.delimiter, Some('/'));
        assert!(m.selectable);
        let m = list_line(b"* LIST (\\Noselect \\HasChildren) \".\" Work\r\n").unwrap();
        assert_eq!(m.name, "Work");
        assert_eq!(m.delimiter, Some('.'));
        assert!(!m.selectable);
        let m = list_line(b"* LIST () NIL {6}\r\nOdd \")\r\n").unwrap();
        assert_eq!(m.name, "Odd \")", "a name in a literal, whatever it holds");
        assert_eq!(m.delimiter, None);
    }

    #[test]
    fn a_fetch_gives_the_uid_size_date_and_subject() {
        let raw = b"* 2 FETCH (UID 41 RFC822.SIZE 1234 INTERNALDATE \"05-Nov-2026 10:00:00 +0100\" BODY[HEADER.FIELDS (SUBJECT)] {44}\r\nSubject: =?UTF-8?B?Q2FmZsOo?=\r\n and more\r\n\r\n)\r\n";
        let s = summary(raw).unwrap();
        assert_eq!(s.uid, 41);
        assert_eq!(s.size, 1234);
        assert_eq!(s.subject, "Caffè and more");
        assert_eq!(s.arrived, Some(1_793_872_800 - 3600));
        assert_eq!(s.file_name(), "41 Caffè and more.eml");
    }

    #[test]
    fn a_body_comes_out_of_its_literal_whole() {
        let raw = b"* 1 FETCH (UID 9 BODY[] {11}\r\nab)c\r\n(d)\r\n)\r\n";
        assert_eq!(body(raw).unwrap(), b"ab)c\r\n(d)\r\n");
    }

    #[test]
    fn names_dates_and_quoting() {
        assert_eq!(uid_of_name("123 Hello.eml"), Some(123));
        assert_eq!(uid_of_name("Hello.eml"), None);
        assert_eq!(quote("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(literal_length("* 1 FETCH (BODY[] {42}"), Some(42));
        assert_eq!(literal_length("* OK done"), None);
        assert_eq!(internal_date("01-Jan-1970 00:00:00 +0000"), Some(0));
        let s = Summary {
            uid: 7,
            size: 1,
            arrived: None,
            subject: "a/b: c?".into(),
        };
        assert_eq!(s.file_name(), "7 a_b_ c_.eml");
        let blank = Summary {
            subject: String::new(),
            ..s
        };
        assert_eq!(blank.file_name(), "7.eml");
    }
}
