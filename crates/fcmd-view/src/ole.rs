//! The files written as OLE compound files: an Outlook message, and the Office documents
//! from before the zip-based formats — `.doc`, `.xls`, `.ppt` (SPEC 05 §1.8).
//!
//! The container is read by `fcmd_arc::cfb`, which the `.msg` reader was written on; here
//! it is asked for two things. A message gives the same rows an `.eml` does (`mime.rs`). A
//! document gives what it says about itself in the two property streams every Office
//! program wrote, `\x05SummaryInformation` and `\x05DocumentSummaryInformation` (MS-OLEPS):
//! the title, the author, the dates, the counts — the rows `office.rs` gives for a `.docx`,
//! under the same names, so a search by property finds both. Either way the streams of the
//! file are listed after, since that is what a compound file *is*.

use fcmd_arc::cfb::{self, Cfb, Kind, ReadAt};

use crate::structure::{Bytes, Row, Structure, MAX_ENTRIES};

/// The most of a property stream that is read: a real one is a few hundred bytes, and a
/// thumbnail saved in it a few tens of kilobytes.
const STREAM_LIMIT: u64 = 1 << 20;

/// The pane's source as the compound file reader wants it.
struct Source<'a>(&'a mut dyn Bytes);

impl ReadAt for Source<'_> {
    fn read_at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        self.0.at(offset, len)
    }

    fn size(&mut self) -> u64 {
        self.0.len()
    }
}

pub(crate) fn read(src: &mut dyn Bytes, head: &[u8], _len: u64) -> Option<Structure> {
    if !head.starts_with(&cfb::MAGIC) {
        return None;
    }
    let mut src = Source(src);
    let file = Cfb::open(&mut src, 1 << 16).ok()?;
    if fcmd_arc::msg::is_message(&file) {
        let outline = fcmd_arc::msg::outline(&mut src).ok()?;
        return Some(crate::mime::shape(
            "msg",
            "Outlook message",
            &outline.headline,
            &outline.tree,
            false,
        ));
    }

    let root = file.root();
    let mut set = |name: &str| {
        file.child(root, name)
            .and_then(|i| file.read(&mut src, i, STREAM_LIMIT).ok())
            .map(|bytes| property_set(&bytes))
            .unwrap_or_default()
    };
    let summary = set("\u{5}SummaryInformation");
    let document = set("\u{5}DocumentSummaryInformation");

    let has = |name: &str| file.child(root, name).is_some();
    let kind = if has("WordDocument") {
        "Word document"
    } else if has("Workbook") || has("Book") {
        "Excel workbook"
    } else if has("PowerPoint Document") {
        "PowerPoint presentation"
    } else if has("VisioDocument") {
        "Visio drawing"
    } else {
        "OLE compound file"
    };

    let field = |label: &str, value: String| Row::field(label, String::new(), value);
    let text = |set: &[(u32, Value)], id: u32| {
        set.iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, v)| match v {
                Value::Text(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
                Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
    };
    let time = |set: &[(u32, Value)], id: u32| {
        set.iter()
            .find(|(i, _)| *i == id)
            .and_then(|(_, v)| match v {
                Value::Time(t) => Some(*t),
                _ => None,
            })
    };

    let mut rows = Vec::new();
    let mut properties = Vec::new();
    for (id, label) in [
        (2, "title"),
        (3, "subject"),
        (4, "author"),
        (5, "keywords"),
        (6, "comments"),
    ] {
        if let Some(v) = text(&summary, id) {
            properties.push(field(label, v));
        }
    }
    if let Some(v) = text(&document, 2) {
        properties.push(field("category", v));
    }
    for (id, label) in [(8, "last saved by"), (9, "revision")] {
        if let Some(v) = text(&summary, id) {
            properties.push(field(label, v));
        }
    }
    for (id, label) in [(12, "created"), (13, "modified"), (11, "printed")] {
        if let Some(v) = time(&summary, id).and_then(cfb::filetime_text) {
            properties.push(field(label, v));
        }
    }
    if !properties.is_empty() {
        rows.push(Row::group("Properties"));
        rows.extend(properties);
    }

    let mut stats = Vec::new();
    for (id, label) in [(14, "pages"), (15, "words"), (16, "characters")] {
        // A count of zero is what a program writes when it never counted.
        if let Some(v) = text(&summary, id).filter(|v| v != "0") {
            stats.push(field(label, v));
        }
    }
    for (id, label) in [
        (5, "lines"),
        (6, "paragraphs"),
        (7, "slides"),
        (8, "notes"),
        (9, "hidden slides"),
    ] {
        if let Some(v) = text(&document, id).filter(|v| v != "0") {
            stats.push(field(label, v));
        }
    }
    // The editing time is a FILETIME used as a length: hundreds of nanoseconds.
    if let Some(t) = time(&summary, 10).filter(|&t| t >= 600_000_000) {
        stats.push(field(
            "editing time (minutes)",
            (t / 600_000_000).to_string(),
        ));
    }
    for (id, label) in [(15, "company"), (14, "manager")] {
        if let Some(v) = text(&document, id) {
            stats.push(field(label, v));
        }
    }
    if let Some(v) = text(&summary, 7) {
        stats.push(field("template", v));
    }
    if let Some(v) = text(&summary, 18) {
        stats.push(field("written by", v));
    }
    if !stats.is_empty() {
        rows.push(Row::group("Document"));
        rows.extend(stats);
    }

    rows.push(Row::group("Streams"));
    let mut listed = 0;
    list(&file, root, 1, &mut listed, &mut rows);

    let title = rows
        .iter()
        .find(|r| r.label == "title")
        .map(|r| format!(" · {}", r.value))
        .unwrap_or_default();
    Some(Structure {
        format: "OLE",
        summary: format!("{kind}{title}"),
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: false,
    })
}

/// The streams and storages under `storage`, indented by how deep they are.
fn list(file: &Cfb, storage: usize, depth: usize, listed: &mut usize, rows: &mut Vec<Row>) {
    for i in file.children(storage) {
        if *listed >= MAX_ENTRIES || depth > 16 {
            return;
        }
        let Some(e) = file.entry(i) else { continue };
        *listed += 1;
        // The property streams start with a control character, which is part of the name.
        let name: String = e
            .name
            .chars()
            .map(|c| {
                if c.is_control() {
                    format!("\\x{:02X}", c as u32)
                } else {
                    c.to_string()
                }
            })
            .collect();
        let indent = "  ".repeat(depth);
        if e.kind == Kind::Storage {
            rows.push(Row::entry(format!("{indent}{name}/"), String::new(), None));
            list(file, i, depth + 1, listed, rows);
        } else {
            rows.push(Row::entry(
                format!("{indent}{name}"),
                fcmd_arc::mail::size_text(e.size),
                None,
            ));
        }
    }
}

/// One value of a property set, as far as the rows need it.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Value {
    Text(String),
    Number(i64),
    /// A FILETIME: a date, or a length of time.
    Time(u64),
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        b.get(at..at.checked_add(2)?)?.try_into().ok()?,
    ))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        b.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}

fn le64(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        b.get(at..at.checked_add(8)?)?.try_into().ok()?,
    ))
}

/// The first section of a property set stream: its properties by number. Anything that does
/// not fit in the stream is left out, and a stream that is not a property set is empty.
fn property_set(bytes: &[u8]) -> Vec<(u32, Value)> {
    let mut out = Vec::new();
    let parse = || -> Option<(usize, usize)> {
        if le16(bytes, 0)? != 0xFFFE || le32(bytes, 24)? == 0 {
            return None;
        }
        let section = le32(bytes, 44)? as usize;
        let count = le32(bytes, section.checked_add(4)?)? as usize;
        Some((section, count.min(1024)))
    };
    let Some((section, count)) = parse() else {
        return out;
    };
    let entry = |k: usize| -> Option<(u32, usize)> {
        let at = section + 8 + k * 8;
        let id = le32(bytes, at)?;
        let offset = section.checked_add(le32(bytes, at + 4)? as usize)?;
        Some((id, offset))
    };
    // Property 1 is the code page the strings are in, and has to be known before them.
    let page = (0..count)
        .filter_map(entry)
        .find(|(id, _)| *id == 1)
        .and_then(|(_, at)| le16(bytes, at + 4))
        .unwrap_or(1252);
    for (id, at) in (0..count).filter_map(entry) {
        let Some(kind) = le32(bytes, at) else {
            continue;
        };
        let value = match kind & 0xFFFF {
            // VT_I2, VT_I4.
            2 => le16(bytes, at + 4).map(|v| Value::Number(i64::from(v as i16))),
            3 => le32(bytes, at + 4).map(|v| Value::Number(i64::from(v as i32))),
            // VT_LPSTR: a length in bytes, then the bytes in the set's code page — which is
            // UTF-16 when the page says 1200.
            0x1E => le32(bytes, at + 4).and_then(|n| {
                let raw = bytes.get(at + 8..(at + 8).checked_add(n as usize)?)?;
                Some(Value::Text(if page == 1200 {
                    utf16(raw)
                } else {
                    fcmd_arc::rtf::code_page(u32::from(page))
                        .decode_without_bom_handling(raw)
                        .0
                        .trim_end_matches('\0')
                        .to_string()
                }))
            }),
            // VT_LPWSTR: a length in characters.
            0x1F => le32(bytes, at + 4).and_then(|n| {
                let raw = bytes.get(at + 8..(at + 8).checked_add((n as usize).checked_mul(2)?)?)?;
                Some(Value::Text(utf16(raw)))
            }),
            // VT_FILETIME.
            0x40 => le64(bytes, at + 4).map(Value::Time),
            _ => None,
        };
        if let Some(value) = value {
            out.push((id, value));
        }
    }
    out
}

fn utf16(raw: &[u8]) -> String {
    let units: Vec<u16> = raw
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    String::from_utf16_lossy(&units)
        .trim_end_matches('\0')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure;
    use fcmd_arc::cfb::build::{file, storage, stream};
    use fcmd_arc::msg::build::{Attach, Draft};

    /// A property set of one section, as an Office program writes it.
    fn set(page: u16, properties: &[(u32, Value)]) -> Vec<u8> {
        let mut values: Vec<(u32, Vec<u8>)> = vec![(1, {
            let mut v = 2u32.to_le_bytes().to_vec();
            v.extend_from_slice(&u32::from(page).to_le_bytes());
            v
        })];
        for (id, value) in properties {
            let mut v = Vec::new();
            match value {
                Value::Text(s) => {
                    let mut raw = if page == 1200 {
                        s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
                    } else {
                        fcmd_arc::rtf::code_page(u32::from(page))
                            .encode(s)
                            .0
                            .into_owned()
                    };
                    raw.push(0);
                    v.extend_from_slice(&0x1Eu32.to_le_bytes());
                    v.extend_from_slice(&(raw.len() as u32).to_le_bytes());
                    v.extend_from_slice(&raw);
                    v.resize(v.len().div_ceil(4) * 4, 0);
                }
                Value::Number(n) => {
                    v.extend_from_slice(&3u32.to_le_bytes());
                    v.extend_from_slice(&(*n as i32).to_le_bytes());
                }
                Value::Time(t) => {
                    v.extend_from_slice(&0x40u32.to_le_bytes());
                    v.extend_from_slice(&t.to_le_bytes());
                }
            }
            values.push((*id, v));
        }
        let mut section = Vec::new();
        let mut body = Vec::new();
        let table = 8 + values.len() * 8;
        for (id, v) in &values {
            section.extend_from_slice(&id.to_le_bytes());
            section.extend_from_slice(&((table + body.len()) as u32).to_le_bytes());
            body.extend_from_slice(v);
        }
        let mut out = vec![0u8; 48];
        out[0..2].copy_from_slice(&0xFFFEu16.to_le_bytes());
        out[24..28].copy_from_slice(&1u32.to_le_bytes());
        out[44..48].copy_from_slice(&48u32.to_le_bytes());
        out.extend_from_slice(&((table + body.len()) as u32).to_le_bytes());
        out.extend_from_slice(&(values.len() as u32).to_le_bytes());
        out.extend_from_slice(&section);
        out.extend_from_slice(&body);
        out
    }

    fn get<'a>(s: &'a Structure, label: &str) -> &'a str {
        s.rows
            .iter()
            .find(|r| r.label == label)
            .map(|r| r.value.as_str())
            .unwrap_or_else(|| panic!("no row {label}"))
    }

    /// 2024-03-05 14:30:00 UTC.
    const WHEN: u64 = (1_709_649_000 + 11_644_473_600) * 10_000_000;

    fn doc(major: u16) -> Vec<u8> {
        let summary = set(
            1252,
            &[
                (2, Value::Text("Relazione & bilancio".into())),
                (4, Value::Text("Matteo".into())),
                (5, Value::Text("caffè, conti".into())),
                (8, Value::Text("Ada".into())),
                (9, Value::Text("7".into())),
                (10, Value::Time(90 * 600_000_000)),
                (12, Value::Time(WHEN)),
                (14, Value::Number(12)),
                (15, Value::Number(3400)),
                (16, Value::Number(0)),
                (18, Value::Text("Microsoft Office Word".into())),
            ],
        );
        let document = set(1200, &[(15, Value::Text("Acme".into()))]);
        file(
            major,
            vec![
                stream("WordDocument", &[0u8; 5000]),
                stream("\u{5}SummaryInformation", &summary),
                stream("\u{5}DocumentSummaryInformation", &document),
                storage("ObjectPool", vec![stream("inner", b"x")]),
            ],
        )
    }

    #[test]
    fn an_old_word_document_says_what_a_docx_says() {
        for major in [3, 4] {
            let bytes = doc(major);
            let s = structure::read(&mut bytes.as_slice()).expect("a document");
            assert_eq!(s.format, "OLE");
            assert_eq!(s.summary, "Word document · Relazione & bilancio");
            assert_eq!(get(&s, "author"), "Matteo");
            assert_eq!(get(&s, "keywords"), "caffè, conti");
            assert_eq!(get(&s, "last saved by"), "Ada");
            assert_eq!(get(&s, "revision"), "7");
            assert_eq!(get(&s, "created"), "2024-03-05 14:30:00 UTC");
            assert_eq!(get(&s, "pages"), "12");
            assert_eq!(get(&s, "words"), "3400");
            assert_eq!(get(&s, "editing time (minutes)"), "90");
            assert_eq!(get(&s, "company"), "Acme");
            assert_eq!(get(&s, "written by"), "Microsoft Office Word");
            // A count the program never made is not a row.
            assert!(!s.rows.iter().any(|r| r.label == "characters"));
            // And the file's own streams follow, the folders with what is in them.
            assert_eq!(get(&s, "  WordDocument"), "4.9 KB");
            assert!(s
                .rows
                .iter()
                .any(|r| r.label == "  \\x05SummaryInformation"));
            assert!(s.rows.iter().any(|r| r.label == "  ObjectPool/"));
            assert_eq!(get(&s, "    inner"), "1 B");
        }
    }

    #[test]
    fn a_compound_file_with_no_properties_still_lists_its_streams() {
        let bytes = file(3, vec![stream("Workbook", b"cells")]);
        let s = structure::read(&mut bytes.as_slice()).expect("a compound file");
        assert_eq!(s.summary, "Excel workbook");
        assert_eq!(s.rows[0].label, "Streams");
        assert_eq!(get(&s, "  Workbook"), "5 B");
    }

    #[test]
    fn an_outlook_message_shows_the_rows_an_eml_shows() {
        let draft = Draft {
            subject: "Caffè domani".into(),
            from: ("Matteo".into(), "m@example.com".into()),
            to: vec![(1, "Anna".into(), "anna@example.com".into())],
            text: Some("Ciao".into()),
            html: Some(b"<p>Ciao</p>".to_vec()),
            attachments: vec![Attach::File {
                name: "invoice.pdf".into(),
                mime: Some("application/pdf".into()),
                bytes: vec![0; 2048],
            }],
            ..Draft::default()
        };
        let bytes = fcmd_arc::msg::build::file(&draft, 3);
        let s = structure::read(&mut bytes.as_slice()).expect("a message");
        assert_eq!(s.format, "msg");
        assert_eq!(s.summary, "Outlook message · Caffè domani");
        let text: Vec<String> = s
            .rows
            .iter()
            .map(|r| format!("{}|{}", r.label, r.value))
            .collect();
        assert_eq!(
            text,
            [
                "Message|",
                "from|Matteo <m@example.com>",
                "to|Anna <anna@example.com>",
                "subject|Caffè domani",
                "Structure|",
                "  application/vnd.ms-outlook|",
                "  ├ text/plain|8 B",
                "  ├ text/html|11 B",
                "  └ application/pdf|2.0 KB  \"invoice.pdf\"",
            ]
        );
    }

    #[test]
    fn a_property_set_cut_short_or_filled_with_rubbish_gives_what_it_can() {
        let good = set(
            1252,
            &[(2, Value::Text("Title".into())), (14, Value::Number(3))],
        );
        for cut in 0..good.len() {
            let _ = property_set(&good[..cut]);
        }
        for at in 0..good.len() {
            let mut bad = good.clone();
            bad[at] = 0xFF;
            let _ = property_set(&bad);
        }
        assert!(property_set(b"").is_empty());
        // And a compound file cut short is no pane, or a shorter one, and not a panic.
        let bytes = doc(3);
        for cut in (0..bytes.len()).step_by(97) {
            let _ = structure::read(&mut &bytes[..cut]);
        }
    }
}
