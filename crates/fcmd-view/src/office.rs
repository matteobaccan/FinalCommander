//! What an office document says about itself: title, author, dates, counts — for the
//! OOXML family (`.docx`, `.xlsx`, `.pptx`) and the OpenDocument one (`.odt`, `.ods`, `.odp`)
//! (SPEC 05 §1.8, 0.39.0).
//!
//! Both are zip files with the properties in a small XML part — `docProps/core.xml` and
//! `docProps/app.xml`, or `meta.xml` — so the reader here is the least of a zip reader: find
//! the central directory from the end of the file, look those names up in it, and inflate
//! just those entries. Nothing else of the archive is read, and a zip that holds neither part
//! is not a document and is left to whatever else wants it.

use std::io::Read;

use crate::structure::{Bytes, Row, Structure};

/// The most of one properties part that is inflated: they are a few kilobytes in every file
/// written by a real program.
const PART_LIMIT: usize = 1 << 20;

pub(crate) fn read(src: &mut dyn Bytes, head: &[u8], len: u64) -> Option<Structure> {
    if !head.starts_with(b"PK\x03\x04") {
        return None;
    }
    let dir = central_directory(src, len)?;
    let find = |name: &str| dir.iter().find(|e| e.name == name);
    if let Some(core) = find("docProps/core.xml") {
        let core = inflate(src, core, len).unwrap_or_default();
        let app = find("docProps/app.xml")
            .and_then(|e| inflate(src, e, len))
            .unwrap_or_default();
        let kind = if dir.iter().any(|e| e.name.starts_with("word/")) {
            "Word document"
        } else if dir.iter().any(|e| e.name.starts_with("xl/")) {
            "Excel workbook"
        } else if dir.iter().any(|e| e.name.starts_with("ppt/")) {
            "PowerPoint presentation"
        } else {
            "Office Open XML"
        };
        return Some(ooxml(kind, &core, &app));
    }
    if let Some(meta) = find("meta.xml") {
        let mime = find("mimetype")
            .and_then(|e| inflate(src, e, len))
            .unwrap_or_default();
        let meta = inflate(src, meta, len).unwrap_or_default();
        return Some(odf(&mime, &meta));
    }
    None
}

/// One entry of the central directory: its name and where its local header is.
struct Entry {
    name: String,
    method: u16,
    compressed: u64,
    local: u64,
}

fn le16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

/// The end-of-central-directory record is in the last 64 kB and change, behind a comment of
/// up to 65 535 bytes; the directory it points at is read whole, up to a cap.
fn central_directory(src: &mut dyn Bytes, len: u64) -> Option<Vec<Entry>> {
    let tail_len = len.min(65_557) as usize;
    let tail = src.at(len - tail_len as u64, tail_len);
    let eocd = tail.windows(4).rposition(|w| w == b"PK\x05\x06")?;
    let count = le16(&tail, eocd + 10)? as usize;
    let size = le32(&tail, eocd + 12)? as usize;
    let offset = le32(&tail, eocd + 16)? as u64;
    if offset >= len || size > 16 << 20 {
        return None;
    }
    let dir = src.at(offset, size);
    let mut out = Vec::new();
    let mut at = 0usize;
    for _ in 0..count.min(65_535) {
        if le32(&dir, at) != Some(0x0201_4B50) {
            break;
        }
        let method = le16(&dir, at + 10)?;
        let compressed = le32(&dir, at + 20)? as u64;
        let name_len = le16(&dir, at + 28)? as usize;
        let extra_len = le16(&dir, at + 30)? as usize;
        let comment_len = le16(&dir, at + 32)? as usize;
        let local = le32(&dir, at + 42)? as u64;
        let name = String::from_utf8_lossy(dir.get(at + 46..at + 46 + name_len)?).into_owned();
        out.push(Entry {
            name,
            method,
            compressed,
            local,
        });
        at += 46 + name_len + extra_len + comment_len;
    }
    Some(out)
}

/// An entry's bytes, stored or deflated — the two methods every office program writes.
fn inflate(src: &mut dyn Bytes, e: &Entry, len: u64) -> Option<String> {
    let head = src.at(e.local, 30);
    if le32(&head, 0) != Some(0x0403_4B50) {
        return None;
    }
    let skip = 30 + le16(&head, 26)? as u64 + le16(&head, 28)? as u64;
    let start = e.local + skip;
    if start >= len {
        return None;
    }
    let raw = src.at(start, (e.compressed as usize).min(PART_LIMIT));
    let bytes = match e.method {
        0 => raw,
        8 => {
            let mut out = Vec::new();
            flate2::read::DeflateDecoder::new(&raw[..])
                .take(PART_LIMIT as u64)
                .read_to_end(&mut out)
                .ok()?;
            out
        }
        _ => return None,
    };
    Some(String::from_utf8_lossy(&bytes).into_owned())
}

/// The text of the first element called `tag` (with its namespace prefix), entities read.
fn element(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}");
    let mut from = 0;
    while let Some(pos) = xml[from..].find(&open) {
        let at = from + pos + open.len();
        // `<dc:title>` and `<dc:title xml:lang="…">`, but not `<dc:titlebar>`.
        match xml[at..].chars().next() {
            Some('>') | Some(' ') | Some('/') => {}
            _ => {
                from = at;
                continue;
            }
        }
        let gt = at + xml[at..].find('>')?;
        if xml[..gt].ends_with('/') {
            return None;
        }
        let close = format!("</{tag}>");
        let end = gt + 1 + xml[gt + 1..].find(&close)?;
        let text = unescape(xml[gt + 1..end].trim());
        return (!text.is_empty()).then_some(text);
    }
    None
}

/// An attribute of the first element called `tag`.
fn attribute(xml: &str, tag: &str, name: &str) -> Option<String> {
    let at = xml.find(&format!("<{tag}"))?;
    let end = at + xml[at..].find('>')?;
    let head = &xml[at..end];
    let key = format!("{name}=\"");
    let v = head.find(&key)? + key.len();
    let close = head[v..].find('"')?;
    Some(unescape(&head[v..v + close]))
}

fn unescape(s: &str) -> String {
    s.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&amp;", "&")
}

/// `2026-11-06T14:20:00Z` as `2026-11-06 14:20:00 UTC`, and anything else as it is.
fn when(s: String) -> String {
    match s.split_once('T') {
        Some((date, time)) => {
            let time = time.trim_end_matches('Z');
            let time = time.split('.').next().unwrap_or(time);
            if s.ends_with('Z') {
                format!("{date} {time} UTC")
            } else {
                format!("{date} {time}")
            }
        }
        None => s,
    }
}

fn field(label: &str, value: String) -> Row {
    Row::field(label, String::new(), value)
}

fn ooxml(kind: &'static str, core: &str, app: &str) -> Structure {
    let mut rows = vec![Row::group("Properties")];
    for (tag, label) in [
        ("dc:title", "title"),
        ("dc:subject", "subject"),
        ("dc:creator", "author"),
        ("cp:keywords", "keywords"),
        ("dc:description", "comments"),
        ("cp:category", "category"),
        ("cp:lastModifiedBy", "last saved by"),
        ("cp:revision", "revision"),
    ] {
        if let Some(v) = element(core, tag) {
            rows.push(field(label, v));
        }
    }
    for (tag, label) in [
        ("dcterms:created", "created"),
        ("dcterms:modified", "modified"),
        ("cp:lastPrinted", "printed"),
    ] {
        if let Some(v) = element(core, tag) {
            rows.push(field(label, when(v)));
        }
    }
    let mut stats = Vec::new();
    for (tag, label) in [
        ("Pages", "pages"),
        ("Words", "words"),
        ("Characters", "characters"),
        ("Lines", "lines"),
        ("Paragraphs", "paragraphs"),
        ("Slides", "slides"),
        ("Notes", "notes"),
        ("HiddenSlides", "hidden slides"),
        ("TotalTime", "editing time (minutes)"),
        ("Company", "company"),
        ("Manager", "manager"),
        ("Template", "template"),
    ] {
        if let Some(v) = element(app, tag) {
            stats.push(field(label, v));
        }
    }
    let program = match (element(app, "Application"), element(app, "AppVersion")) {
        (Some(a), Some(v)) => Some(format!("{a} {v}")),
        (Some(a), None) => Some(a),
        _ => None,
    };
    if let Some(p) = &program {
        stats.push(field("written by", p.clone()));
    }
    if !stats.is_empty() {
        rows.push(Row::group("Document"));
        rows.extend(stats);
    }
    let title = rows
        .iter()
        .find(|r| r.label == "title")
        .map(|r| format!(" · {}", r.value))
        .unwrap_or_default();
    structure("OOXML", format!("{kind}{title}"), rows)
}

fn odf(mime: &str, meta: &str) -> Structure {
    let kind = match mime.trim() {
        "application/vnd.oasis.opendocument.text" => "OpenDocument text",
        "application/vnd.oasis.opendocument.spreadsheet" => "OpenDocument spreadsheet",
        "application/vnd.oasis.opendocument.presentation" => "OpenDocument presentation",
        "application/vnd.oasis.opendocument.graphics" => "OpenDocument drawing",
        _ => "OpenDocument",
    };
    let mut rows = vec![Row::group("Properties")];
    for (tag, label) in [
        ("dc:title", "title"),
        ("dc:subject", "subject"),
        ("meta:initial-creator", "author"),
        ("meta:keyword", "keywords"),
        ("dc:description", "comments"),
        ("dc:creator", "last saved by"),
        ("meta:editing-cycles", "revision"),
    ] {
        if let Some(v) = element(meta, tag) {
            rows.push(field(label, v));
        }
    }
    for (tag, label) in [("meta:creation-date", "created"), ("dc:date", "modified")] {
        if let Some(v) = element(meta, tag) {
            rows.push(field(label, when(v)));
        }
    }
    let mut stats = Vec::new();
    for (attr, label) in [
        ("meta:page-count", "pages"),
        ("meta:word-count", "words"),
        ("meta:character-count", "characters"),
        ("meta:paragraph-count", "paragraphs"),
        ("meta:table-count", "tables"),
        ("meta:image-count", "images"),
        ("meta:cell-count", "cells"),
    ] {
        if let Some(v) = attribute(meta, "meta:document-statistic", attr) {
            stats.push(field(label, v));
        }
    }
    if let Some(g) = element(meta, "meta:generator") {
        stats.push(field("written by", g));
    }
    if !stats.is_empty() {
        rows.push(Row::group("Document"));
        rows.extend(stats);
    }
    let title = rows
        .iter()
        .find(|r| r.label == "title")
        .map(|r| format!(" · {}", r.value))
        .unwrap_or_default();
    structure("ODF", format!("{kind}{title}"), rows)
}

fn structure(format: &'static str, summary: String, rows: Vec<Row>) -> Structure {
    Structure {
        format,
        summary,
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A zip of these entries, deflated, the way a word processor writes one.
    fn zip(entries: &[(&str, &str)]) -> Vec<u8> {
        use std::io::Write;
        let mut out = Vec::new();
        let mut central = Vec::new();
        for (name, text) in entries {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(text.as_bytes()).unwrap();
            let data = enc.finish().unwrap();
            let local = out.len() as u32;
            out.extend(0x0403_4B50u32.to_le_bytes());
            out.extend([20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            out.extend(0u32.to_le_bytes()); // crc, not checked here
            out.extend((data.len() as u32).to_le_bytes());
            out.extend((text.len() as u32).to_le_bytes());
            out.extend((name.len() as u16).to_le_bytes());
            out.extend(0u16.to_le_bytes());
            out.extend(name.as_bytes());
            out.extend(&data);
            central.extend(0x0201_4B50u32.to_le_bytes());
            central.extend([20, 0, 20, 0, 0, 0, 8, 0, 0, 0, 0, 0]);
            central.extend(0u32.to_le_bytes());
            central.extend((data.len() as u32).to_le_bytes());
            central.extend((text.len() as u32).to_le_bytes());
            central.extend((name.len() as u16).to_le_bytes());
            central.extend([0u8; 12]);
            central.extend(local.to_le_bytes());
            central.extend(name.as_bytes());
        }
        let at = out.len() as u32;
        out.extend(&central);
        out.extend(0x0605_4B50u32.to_le_bytes());
        out.extend([0, 0, 0, 0]);
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((entries.len() as u16).to_le_bytes());
        out.extend((central.len() as u32).to_le_bytes());
        out.extend(at.to_le_bytes());
        out.extend(0u16.to_le_bytes());
        out
    }

    fn get(s: &Structure, label: &str) -> String {
        s.rows
            .iter()
            .find(|r| r.label == label)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_docx_says_its_title_its_author_and_how_long_it_is() {
        let core = r#"<?xml version="1.0"?><cp:coreProperties xmlns:cp="x" xmlns:dc="y" xmlns:dcterms="z">
            <dc:title>Relazione &amp; bilancio</dc:title><dc:creator>Matteo</dc:creator>
            <cp:lastModifiedBy>Ada</cp:lastModifiedBy><cp:revision>7</cp:revision>
            <dcterms:created xsi:type="dcterms:W3CDTF">2026-11-01T09:30:00Z</dcterms:created>
            </cp:coreProperties>"#;
        let app = "<Properties><Application>Microsoft Office Word</Application><AppVersion>16.0000</AppVersion><Pages>12</Pages><Words>3400</Words><Company>Acme</Company></Properties>";
        let z = zip(&[
            ("[Content_Types].xml", "<Types/>"),
            ("word/document.xml", "<w:document/>"),
            ("docProps/core.xml", core),
            ("docProps/app.xml", app),
        ]);
        let s = read(&mut &z[..], &z, z.len() as u64).expect("a document");
        assert_eq!(s.summary, "Word document · Relazione & bilancio");
        assert_eq!(get(&s, "author"), "Matteo");
        assert_eq!(get(&s, "last saved by"), "Ada");
        assert_eq!(get(&s, "revision"), "7");
        assert_eq!(get(&s, "created"), "2026-11-01 09:30:00 UTC");
        assert_eq!(get(&s, "pages"), "12");
        assert_eq!(get(&s, "words"), "3400");
        assert_eq!(get(&s, "company"), "Acme");
        assert_eq!(get(&s, "written by"), "Microsoft Office Word 16.0000");
    }

    #[test]
    fn an_odt_says_the_same_in_its_own_words() {
        let meta = r#"<office:document-meta><office:meta>
            <meta:generator>LibreOffice/24.8</meta:generator><dc:title>Note</dc:title>
            <meta:initial-creator>Matteo</meta:initial-creator><meta:editing-cycles>3</meta:editing-cycles>
            <meta:document-statistic meta:page-count="2" meta:word-count="512"/>
            </office:meta></office:document-meta>"#;
        let z = zip(&[
            ("mimetype", "application/vnd.oasis.opendocument.text"),
            ("content.xml", "<x/>"),
            ("meta.xml", meta),
        ]);
        let s = read(&mut &z[..], &z, z.len() as u64).expect("a document");
        assert_eq!(s.summary, "OpenDocument text · Note");
        assert_eq!(get(&s, "author"), "Matteo");
        assert_eq!(get(&s, "pages"), "2");
        assert_eq!(get(&s, "words"), "512");
        assert_eq!(get(&s, "written by"), "LibreOffice/24.8");
    }

    #[test]
    fn a_plain_zip_is_not_a_document() {
        let z = zip(&[("a.txt", "hello")]);
        assert!(read(&mut &z[..], &z, z.len() as u64).is_none());
        let junk = b"PK\x03\x04garbage";
        assert!(read(&mut &junk[..], junk, junk.len() as u64).is_none());
    }
}
