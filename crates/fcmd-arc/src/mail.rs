//! Mail as folders (FARMail's and MailView's idea, SPEC 04 §4d; their code is GPL and none
//! of it is here).
//!
//! An `mbox` is a folder of its messages, `0001 Subject.eml`, in the order they were
//! appended. An `.eml` — or a message inside an mbox, which the archive cache opens the same
//! way — is a folder of what it carries: `headers.txt` with the headers decoded,
//! `message.txt` and `message.html` for the text, and each attachment under its own name,
//! decoded from base64 or quoted-printable. Read only.

use std::path::Path;

use fcmd_core::{CancelToken, EntryKind};

use crate::archive::ArcEntry;
use crate::limits::Limits;
use crate::{ArcError, Result};

/// The largest mail file opened as a folder.
pub const LIMIT: u64 = 512 * 1024 * 1024;

fn read(path: &Path, name: &str) -> Result<Vec<u8>> {
    let len = std::fs::metadata(path)
        .map_err(|e| ArcError::Io {
            path: name.to_string(),
            source: e,
        })?
        .len();
    if len > LIMIT {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!("it is over {} MiB", LIMIT >> 20),
        });
    }
    std::fs::read(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })
}

// ---------------------------------------------------------------------------------------
// mbox

/// The messages of an mbox: each starts at a line beginning `From ` at the top of the file or
/// after a blank line, and the `From ` line itself is the separator, not part of the message.
/// `>From ` quoting is undone, as mboxrd writes it.
pub fn split_mbox(bytes: &[u8]) -> Vec<Vec<u8>> {
    let mut out: Vec<Vec<u8>> = Vec::new();
    let mut current: Option<Vec<u8>> = None;
    let mut prev_blank = true;
    for line in bytes.split_inclusive(|&b| b == b'\n') {
        if prev_blank && line.starts_with(b"From ") {
            if let Some(m) = current.take() {
                out.push(m);
            }
            current = Some(Vec::new());
            prev_blank = false;
            continue;
        }
        let trimmed = line.strip_suffix(b"\n").unwrap_or(line);
        let trimmed = trimmed.strip_suffix(b"\r").unwrap_or(trimmed);
        prev_blank = trimmed.is_empty();
        if let Some(m) = current.as_mut() {
            let unquoted = match line.iter().position(|&b| b != b'>') {
                Some(n) if n > 0 && line[n..].starts_with(b"From ") => &line[1..],
                _ => line,
            };
            m.extend_from_slice(unquoted);
        }
    }
    if let Some(m) = current {
        out.push(m);
    }
    // The blank line before each separator belongs to mbox, not to the message.
    for m in &mut out {
        while m.ends_with(b"\n\n") || m.ends_with(b"\r\n\r\n") {
            m.pop();
            if m.ends_with(b"\r") {
                m.pop();
            }
        }
    }
    out
}

fn mbox_name(n: usize, message: &[u8]) -> String {
    let (headers, _) = split_head(message);
    let subject = header(&headers, "subject").unwrap_or_default();
    let clean: String = subject
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .take(60)
        .collect();
    let clean = clean.trim();
    if clean.is_empty() {
        format!("{n:04}.eml")
    } else {
        format!("{n:04} {clean}.eml")
    }
}

pub fn index_mbox(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let bytes = read(path, name)?;
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let messages = split_mbox(&bytes);
    if messages.len() > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!("it holds more than {} messages", limits.entries),
        });
    }
    let mut out = Vec::with_capacity(messages.len());
    for (i, m) in messages.iter().enumerate() {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        out.push(file_entry(
            format!("/{}", mbox_name(i + 1, m)),
            m.len() as u64,
            mtime,
            Some(i as u64),
        ));
    }
    Ok(out)
}

pub fn copy_out_mbox(
    path: &Path,
    name: &str,
    at: Option<u64>,
    entry: &str,
    out: &mut dyn std::io::Write,
) -> Result<u64> {
    let bytes = read(path, name)?;
    let messages = split_mbox(&bytes);
    let found = at
        .and_then(|i| messages.get(i as usize))
        .or_else(|| {
            messages
                .iter()
                .enumerate()
                .find(|(i, m)| format!("/{}", mbox_name(i + 1, m)) == entry)
                .map(|(_, m)| m)
        })
        .ok_or_else(|| ArcError::NoEntry {
            path: name.to_string(),
            entry: entry.to_string(),
        })?;
    write_all(out, found, name)
}

// ---------------------------------------------------------------------------------------
// One message

fn file_entry(
    path: String,
    size: u64,
    mtime: Option<std::time::SystemTime>,
    at: Option<u64>,
) -> ArcEntry {
    ArcEntry {
        path,
        size,
        packed: None,
        mtime,
        mode: Some(0o444),
        kind: EntryKind::File,
        link_target: None,
        at,
    }
}

fn write_all(out: &mut dyn std::io::Write, bytes: &[u8], name: &str) -> Result<u64> {
    out.write_all(bytes).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    Ok(bytes.len() as u64)
}

/// The files a message is made of: the decoded headers, the text, the HTML, the attachments.
pub fn files(message: &[u8]) -> Vec<(String, Vec<u8>)> {
    let (headers, body) = split_head(message);
    let mut out: Vec<(String, Vec<u8>)> = Vec::new();
    let mut shown = String::new();
    for (k, v) in &headers {
        shown.push_str(k);
        shown.push_str(": ");
        shown.push_str(&decode_words(v));
        shown.push('\n');
    }
    out.push(("headers.txt".into(), shown.into_bytes()));
    let mut parts = Vec::new();
    walk_parts(&headers, body, &mut parts, 0);
    let mut seen_text = false;
    let mut seen_html = false;
    let mut n = 0;
    for part in parts {
        let name = match (&part.filename, part.mime.as_str()) {
            (Some(f), _) => f.clone(),
            (None, "text/plain") if !seen_text => {
                seen_text = true;
                "message.txt".into()
            }
            (None, "text/html") if !seen_html => {
                seen_html = true;
                "message.html".into()
            }
            (None, mime) => {
                n += 1;
                format!("part{n}.{}", extension(mime))
            }
        };
        let mut unique = name.replace(['/', '\\', '\0'], "_");
        let mut k = 2;
        while out.iter().any(|(m, _)| *m == unique) {
            unique = format!("{k} {name}");
            k += 1;
        }
        out.push((unique, part.bytes));
    }
    out
}

pub fn index_eml(
    path: &Path,
    name: &str,
    _limits: &Limits,
    _cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let bytes = read(path, name)?;
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    Ok(files(&bytes)
        .into_iter()
        .map(|(n, b)| file_entry(format!("/{n}"), b.len() as u64, mtime, None))
        .collect())
}

pub fn copy_out_eml(
    path: &Path,
    name: &str,
    entry: &str,
    out: &mut dyn std::io::Write,
) -> Result<u64> {
    let bytes = read(path, name)?;
    let want = entry.trim_start_matches('/');
    let (_, b) = files(&bytes)
        .into_iter()
        .find(|(n, _)| n == want)
        .ok_or_else(|| ArcError::NoEntry {
            path: name.to_string(),
            entry: entry.to_string(),
        })?;
    write_all(out, &b, name)
}

struct Part {
    mime: String,
    filename: Option<String>,
    bytes: Vec<u8>,
}

/// Headers (unfolded, in order) and the body after the blank line.
fn split_head(message: &[u8]) -> (Vec<(String, String)>, &[u8]) {
    let mut headers: Vec<(String, String)> = Vec::new();
    let mut at = 0;
    while at < message.len() {
        let end = message[at..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(message.len(), |e| at + e + 1);
        let line = &message[at..end];
        let text = String::from_utf8_lossy(line);
        let text = text.trim_end_matches(['\r', '\n']);
        at = end;
        if text.is_empty() {
            break;
        }
        if text.starts_with([' ', '\t']) {
            if let Some((_, v)) = headers.last_mut() {
                v.push(' ');
                v.push_str(text.trim());
            }
            continue;
        }
        if let Some((k, v)) = text.split_once(':') {
            headers.push((k.trim().to_string(), v.trim().to_string()));
        }
    }
    (headers, &message[at.min(message.len())..])
}

fn header(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| decode_words(v))
}

/// A header parameter: `boundary`, `charset`, `filename`, `name`, quoted or not.
fn param(value: &str, key: &str) -> Option<String> {
    for piece in value.split(';').skip(1) {
        let (k, v) = piece.split_once('=')?;
        let k = k.trim().to_ascii_lowercase();
        if k == key || k == format!("{key}*") {
            let v = v.trim().trim_matches('"');
            // RFC 2231: `utf-8''name%20here`.
            if let Some((_, rest)) = v.split_once("''") {
                return Some(percent(rest));
            }
            return Some(decode_words(v));
        }
    }
    None
}

fn percent(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(n) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(n);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn walk_parts(headers: &[(String, String)], body: &[u8], out: &mut Vec<Part>, depth: usize) {
    let ctype = headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("content-type"))
        .map(|(_, v)| v.clone())
        .unwrap_or_else(|| "text/plain".into());
    let mime = ctype
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    if mime.starts_with("multipart/") && depth < 16 {
        if let Some(boundary) = param(&ctype, "boundary") {
            for piece in split_multipart(body, &boundary) {
                let (h, b) = split_head(piece);
                walk_parts(&h, b, out, depth + 1);
            }
            return;
        }
    }
    let encoding = header(headers, "content-transfer-encoding")
        .unwrap_or_default()
        .to_ascii_lowercase();
    let mut bytes = match encoding.trim() {
        "base64" => base64(body),
        "quoted-printable" => quoted_printable(body),
        _ => body.to_vec(),
    };
    let disposition = header(headers, "content-disposition").unwrap_or_default();
    let filename = param(&disposition, "filename").or_else(|| param(&ctype, "name"));
    if mime.starts_with("text/") && filename.is_none() {
        if let Some(cs) = param(&ctype, "charset") {
            bytes = to_utf8(&bytes, &cs).into_bytes();
        }
    }
    out.push(Part {
        mime,
        filename,
        bytes,
    });
}

fn split_multipart<'a>(body: &'a [u8], boundary: &str) -> Vec<&'a [u8]> {
    let marker = format!("--{boundary}");
    let marker = marker.as_bytes();
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let mut at = 0;
    while at < body.len() {
        let end = body[at..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(body.len(), |e| at + e + 1);
        let line = &body[at..end];
        let trimmed = line.strip_suffix(b"\n").unwrap_or(line);
        let trimmed = trimmed.strip_suffix(b"\r").unwrap_or(trimmed);
        if trimmed.starts_with(marker) {
            if let Some(s) = start {
                let mut e = at;
                // The line break before a boundary belongs to the boundary.
                if e > s && body[e - 1] == b'\n' {
                    e -= 1;
                    if e > s && body[e - 1] == b'\r' {
                        e -= 1;
                    }
                }
                out.push(&body[s..e]);
            }
            if trimmed[marker.len()..].starts_with(b"--") {
                return out;
            }
            start = Some(end);
        }
        at = end;
    }
    if let Some(s) = start {
        out.push(&body[s..]);
    }
    out
}

fn to_utf8(bytes: &[u8], charset: &str) -> String {
    match encoding_rs::Encoding::for_label(charset.trim().as_bytes()) {
        Some(enc) => enc.decode(bytes).0.into_owned(),
        None => String::from_utf8_lossy(bytes).into_owned(),
    }
}

/// RFC 2047 encoded words: `=?utf-8?B?…?=` and `=?iso-8859-1?Q?…?=`, with the blanks
/// between two of them dropped, as the RFC says.
pub fn decode_words(value: &str) -> String {
    let mut out = String::new();
    let mut rest = value;
    let mut last_was_word = false;
    while let Some(start) = rest.find("=?") {
        let (before, tail) = rest.split_at(start);
        let parts: Vec<&str> = tail[2..].splitn(3, '?').collect();
        let decoded = if parts.len() == 3 {
            let (charset, enc, more) = (parts[0], parts[1], parts[2]);
            more.find("?=").map(|end| {
                let text = &more[..end];
                let bytes = match enc.to_ascii_uppercase().as_str() {
                    "B" => base64(text.as_bytes()),
                    _ => quoted_printable(text.replace('_', " ").as_bytes()),
                };
                (
                    to_utf8(&bytes, charset),
                    2 + charset.len() + 1 + enc.len() + 1 + end + 2,
                )
            })
        } else {
            None
        };
        match decoded {
            Some((text, used)) => {
                if !(last_was_word && before.trim().is_empty()) {
                    out.push_str(before);
                }
                out.push_str(&text);
                rest = &tail[used..];
                last_was_word = true;
            }
            None => {
                out.push_str(before);
                out.push_str("=?");
                rest = &tail[2..];
                last_was_word = false;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn base64(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len() * 3 / 4);
    let mut acc = 0u32;
    let mut bits = 0;
    for &c in input {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    out
}

pub fn quoted_printable(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        if input[i] == b'=' {
            if input.get(i + 1) == Some(&b'\n') {
                i += 2;
                continue;
            }
            if input.get(i + 1) == Some(&b'\r') && input.get(i + 2) == Some(&b'\n') {
                i += 3;
                continue;
            }
            if let (Some(a), Some(b)) = (input.get(i + 1), input.get(i + 2)) {
                if let Ok(n) = u8::from_str_radix(&format!("{}{}", *a as char, *b as char), 16) {
                    out.push(n);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(input[i]);
        i += 1;
    }
    out
}

fn extension(mime: &str) -> &'static str {
    match mime {
        "text/plain" => "txt",
        "text/html" => "html",
        "text/calendar" => "ics",
        "application/pdf" => "pdf",
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "message/rfc822" => "eml",
        _ => "bin",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAIL: &str = "From: =?utf-8?B?TWF0dGVv?= <m@example.com>\r\n\
Subject: =?iso-8859-1?Q?Caff=E8?= e =?utf-8?Q?t=C3=A8?=\r\n\
Content-Type: multipart/mixed; boundary=\"XX\"\r\n\
\r\n\
preamble\r\n\
--XX\r\n\
Content-Type: text/plain; charset=iso-8859-1\r\n\
Content-Transfer-Encoding: quoted-printable\r\n\
\r\n\
Ciao, un caff=E8 =\r\n\
per te.\r\n\
--XX\r\n\
Content-Type: application/octet-stream; name=\"notes.bin\"\r\n\
Content-Disposition: attachment; filename=\"notes.txt\"\r\n\
Content-Transfer-Encoding: base64\r\n\
\r\n\
aGVsbG8gd29ybGQ=\r\n\
--XX--\r\n";

    #[test]
    fn a_message_is_its_headers_its_text_and_its_attachments() {
        let files = files(MAIL.as_bytes());
        let names: Vec<&str> = files.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(names, ["headers.txt", "message.txt", "notes.txt"]);
        let headers = String::from_utf8(files[0].1.clone()).unwrap();
        assert!(
            headers.contains("From: Matteo <m@example.com>"),
            "{headers}"
        );
        assert!(headers.contains("Subject: Caffè e tè"), "{headers}");
        assert_eq!(
            String::from_utf8(files[1].1.clone()).unwrap(),
            "Ciao, un caffè per te."
        );
        assert_eq!(files[2].1, b"hello world");
    }

    #[test]
    fn an_mbox_is_split_at_its_from_lines_and_unquoted() {
        let mbox = "From a@b Mon Jan  1 00:00:00 2024\nSubject: one\n\nbody\n>From here\n\nFrom c@d Tue Jan  2 00:00:00 2024\nSubject: two\n\nsecond\n";
        let messages = split_mbox(mbox.as_bytes());
        assert_eq!(messages.len(), 2);
        assert_eq!(
            String::from_utf8_lossy(&messages[0]),
            "Subject: one\n\nbody\nFrom here\n"
        );
        assert_eq!(mbox_name(2, &messages[1]), "0002 two.eml");
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("inbox.mbox");
        std::fs::write(&p, mbox).unwrap();
        let entries =
            index_mbox(&p, "inbox.mbox", &Limits::default(), &CancelToken::new()).unwrap();
        assert_eq!(entries[0].path, "/0001 one.eml");
        let mut out = Vec::new();
        copy_out_mbox(&p, "inbox.mbox", entries[1].at, &entries[1].path, &mut out).unwrap();
        assert!(String::from_utf8(out).unwrap().contains("second"));
    }
}
