//! An Outlook `.msg` as a folder (MS-OXMSG), the way `mail.rs` makes one of an `.eml`.
//!
//! A `.msg` is a compound file (`cfb.rs`) holding one message as MAPI properties. The small
//! fixed-size ones — a date, a number, a flag — sit in a table, the stream
//! `__properties_version1.0`; each of the others is a stream of its own named after it,
//! `__substg1.0_0037001F` being property 0x0037 (the subject) of type 0x001F (a UTF-16
//! string). Recipients and attachments are storages, `__recip_version1.0_#00000000` and
//! `__attach_version1.0_#00000000`, each with its own table and streams, and a message
//! attached to the message is a storage inside its attachment, shaped like the whole file.
//!
//! What comes out is the same folder an `.eml` gives: `headers.txt`, `structure.txt`,
//! `message.txt`, `message.html`, the attachments under their names, and an attached message
//! as a folder of the same. The body is taken from whichever form the message carries: the
//! plain one, the HTML one, or the compressed RTF Outlook always writes (`rtf.rs`), which is
//! read as text — and as HTML, when it wraps some — if it is all there is. Read only.

use std::path::Path;

use encoding_rs::Encoding;
use fcmd_core::{CancelToken, EntryKind};

use crate::archive::ArcEntry;
use crate::cfb::{self, Cfb, Kind, ReadAt};
use crate::limits::Limits;
use crate::mail::{self, Node};
use crate::{ArcError, Result};

/// The table of fixed-size properties, in every storage that is an object.
const PROPERTIES: &str = "__properties_version1.0";
/// What is in front of the table: 32 bytes in the file's own message, 24 in a message
/// attached to one, 8 in a recipient or an attachment.
const TOP: usize = 32;
const EMBEDDED: usize = 24;
const CHILD: usize = 8;

/// How deep messages attached to messages are followed.
const MAX_NESTING: usize = 16;

/// The largest table of properties read: a real one is a few hundred entries of 16 bytes.
const TABLE_LIMIT: u64 = 16 << 20;

// The properties read, by their MAPI numbers.
const SUBJECT: u16 = 0x0037;
const SUBMIT_TIME: u16 = 0x0039;
const SENT_FOR_NAME: u16 = 0x0042;
const SENT_FOR_EMAIL: u16 = 0x0065;
const TRANSPORT_HEADERS: u16 = 0x007D;
const SENDER_NAME: u16 = 0x0C1A;
const SENDER_EMAIL: u16 = 0x0C1F;
const RECIPIENT_TYPE: u16 = 0x0C15;
const DISPLAY_CC: u16 = 0x0E03;
const DISPLAY_TO: u16 = 0x0E04;
const DELIVERY_TIME: u16 = 0x0E06;
const BODY: u16 = 0x1000;
const RTF: u16 = 0x1009;
const HTML: u16 = 0x1013;
const DISPLAY_NAME: u16 = 0x3001;
const EMAIL: u16 = 0x3003;
const ATTACH_DATA: u16 = 0x3701;
const ATTACH_FILENAME: u16 = 0x3704;
const ATTACH_LONG_FILENAME: u16 = 0x3707;
const ATTACH_MIME: u16 = 0x370E;
const SMTP_ADDRESS: u16 = 0x39FE;
const INTERNET_CODE_PAGE: u16 = 0x3FDE;
const MESSAGE_CODE_PAGE: u16 = 0x3FFD;
const SENDER_SMTP: u16 = 0x5D01;

/// What `structure.txt` calls a message of this kind.
const MIME: &str = "application/vnd.ms-outlook";

/// One object of the file — the message, a recipient, an attachment — with its table.
struct Props {
    storage: usize,
    /// The fixed-size properties: the tag (number and type) and its eight bytes.
    fixed: Vec<(u32, [u8; 8])>,
}

impl Props {
    fn value(&self, id: u16, kind: u16) -> Option<[u8; 8]> {
        let tag = (u32::from(id) << 16) | u32::from(kind);
        self.fixed.iter().find(|(t, _)| *t == tag).map(|(_, v)| *v)
    }

    /// A 32-bit number (`PT_LONG`).
    fn long(&self, id: u16) -> Option<u32> {
        self.value(id, 0x0003)
            .map(|v| u32::from_le_bytes([v[0], v[1], v[2], v[3]]))
    }

    /// A time (`PT_SYSTIME`), as a FILETIME.
    fn time(&self, id: u16) -> Option<u64> {
        self.value(id, 0x0040)
            .map(u64::from_le_bytes)
            .filter(|&t| t != 0)
    }
}

struct Reader<'a> {
    cfb: Cfb,
    src: &'a mut dyn ReadAt,
    /// What a stream may be at most.
    cap: u64,
}

impl Reader<'_> {
    fn props(&mut self, storage: usize, header: usize) -> Props {
        let mut fixed = Vec::new();
        if let Some(table) = self
            .cfb
            .child(storage, PROPERTIES)
            .and_then(|i| self.cfb.read(self.src, i, TABLE_LIMIT).ok())
        {
            for e in table.get(header..).unwrap_or_default().as_chunks::<16>().0 {
                let tag = u32::from_le_bytes([e[0], e[1], e[2], e[3]]);
                let mut v = [0u8; 8];
                v.copy_from_slice(&e[8..16]);
                fixed.push((tag, v));
            }
        }
        Props { storage, fixed }
    }

    /// The stream a property of this number and type is in.
    fn stream(&self, p: &Props, id: u16, kind: u16) -> Option<usize> {
        self.cfb
            .child(p.storage, &format!("__substg1.0_{id:04X}{kind:04X}"))
            .filter(|&i| self.cfb.entry(i).is_some_and(|e| e.kind == Kind::Stream))
    }

    /// How large a property is as stored, in any of the types given.
    fn stored(&self, p: &Props, id: u16, kinds: &[u16]) -> Option<u64> {
        kinds
            .iter()
            .find_map(|&k| self.stream(p, id, k))
            .and_then(|i| self.cfb.entry(i))
            .map(|e| e.size)
    }

    fn binary(&mut self, p: &Props, id: u16) -> Option<Vec<u8>> {
        let i = self.stream(p, id, 0x0102)?;
        self.cfb.read(self.src, i, self.cap).ok()
    }

    /// A string property: UTF-16 when the message was saved as Unicode, and bytes in the
    /// message's code page when it was not.
    fn string(&mut self, p: &Props, id: u16, page: &'static Encoding) -> Option<String> {
        let text = if let Some(i) = self.stream(p, id, 0x001F) {
            let bytes = self.cfb.read(self.src, i, self.cap).ok()?;
            let units: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| u16::from_le_bytes(*c))
                .collect();
            String::from_utf16_lossy(&units)
        } else {
            let i = self.stream(p, id, 0x001E)?;
            let bytes = self.cfb.read(self.src, i, self.cap).ok()?;
            page.decode_without_bom_handling(&bytes).0.into_owned()
        };
        // Writers differ on whether the terminating zero is part of the stream.
        Some(text.trim_end_matches('\0').to_string())
    }

    /// The storages of one kind under an object — its recipients or its attachments — in the
    /// order their numbers give.
    fn numbered(&self, storage: usize, prefix: &str) -> Vec<usize> {
        let mut found: Vec<(String, usize)> = self
            .cfb
            .children(storage)
            .into_iter()
            .filter_map(|i| {
                let e = self.cfb.entry(i)?;
                (e.kind == Kind::Storage
                    && e.name
                        .get(..prefix.len())
                        .is_some_and(|p| p.eq_ignore_ascii_case(prefix)))
                .then(|| (e.name.to_ascii_uppercase(), i))
            })
            .collect();
        found.sort();
        found.into_iter().map(|(_, i)| i).collect()
    }

    /// The message attached through this attachment, when it is one: a storage in the place
    /// of the data, holding a table of its own.
    fn embedded(&self, attachment: usize) -> Option<usize> {
        let inner = self
            .cfb
            .child(attachment, &format!("__substg1.0_{ATTACH_DATA:04X}000D"))?;
        self.cfb.child(inner, PROPERTIES).map(|_| inner)
    }

    fn page(&self, p: &Props) -> &'static Encoding {
        let cp = p
            .long(MESSAGE_CODE_PAGE)
            .or_else(|| p.long(INTERNET_CODE_PAGE))
            .unwrap_or(1252);
        crate::rtf::code_page(cp)
    }
}

/// `Name <address>`, or whichever of the two there is.
fn mailbox(name: Option<String>, email: Option<String>) -> String {
    let name = name.unwrap_or_default();
    let email = email.unwrap_or_default();
    match (name.trim(), email.trim()) {
        ("", e) => e.to_string(),
        (n, "") => n.to_string(),
        (n, e) if n.eq_ignore_ascii_case(e) => e.to_string(),
        (n, e) => format!("{n} <{e}>"),
    }
}

/// What `headers.txt` is made of.
#[derive(Default)]
struct Head {
    lines: Vec<(&'static str, String)>,
    /// The headers the message arrived with, when Outlook kept them.
    transport: Option<String>,
}

impl Reader<'_> {
    fn head(&mut self, p: &Props, page: &'static Encoding) -> Head {
        let mut head = Head::default();
        // The address the message was sent from: the SMTP one when the message has it —
        // inside an Exchange organisation the plain property is a directory path, not an
        // address anybody could write to.
        let smtp = self.string(p, SENDER_SMTP, page);
        let sender = self.string(p, SENDER_EMAIL, page);
        let sent_for = self.string(p, SENT_FOR_EMAIL, page);
        let email = [smtp, sender.clone(), sent_for]
            .into_iter()
            .flatten()
            .find(|e| e.contains('@'))
            .or(sender);
        let name = self
            .string(p, SENDER_NAME, page)
            .or_else(|| self.string(p, SENT_FOR_NAME, page));
        let from = mailbox(name, email);
        if !from.is_empty() {
            head.lines.push(("From", from));
        }

        let mut lists: [Vec<String>; 3] = Default::default();
        for r in self.numbered(p.storage, "__recip_version1.0_#") {
            let rp = self.props(r, CHILD);
            let name = self.string(&rp, DISPLAY_NAME, page);
            let email = self
                .string(&rp, SMTP_ADDRESS, page)
                .or_else(|| self.string(&rp, EMAIL, page));
            let who = mailbox(name, email);
            if who.is_empty() {
                continue;
            }
            // 1 is To, 2 is Cc, 3 is Bcc; a recipient that does not say is a To.
            let list = match rp.long(RECIPIENT_TYPE).map(|t| t & 0xF) {
                Some(2) => 1,
                Some(3) => 2,
                _ => 0,
            };
            lists[list].push(who);
        }
        // A message with no table of recipients still has the lines Outlook displays.
        if lists.iter().all(|l| l.is_empty()) {
            for (list, id) in [(0, DISPLAY_TO), (1, DISPLAY_CC)] {
                if let Some(shown) = self.string(p, id, page).filter(|s| !s.trim().is_empty()) {
                    lists[list].push(shown);
                }
            }
        }
        for (name, list) in ["To", "Cc", "Bcc"].into_iter().zip(lists) {
            if !list.is_empty() {
                head.lines.push((name, list.join("; ")));
            }
        }
        if let Some(subject) = self.string(p, SUBJECT, page).filter(|s| !s.is_empty()) {
            head.lines.push(("Subject", subject));
        }
        let sent = p.time(SUBMIT_TIME);
        let delivered = p.time(DELIVERY_TIME);
        if let Some(text) = sent.or(delivered).and_then(cfb::filetime_text) {
            head.lines.push(("Date", text));
        }
        if let (Some(_), Some(text)) = (sent, delivered.and_then(cfb::filetime_text)) {
            if sent != delivered {
                head.lines.push(("Delivered", text));
            }
        }
        head.transport = self
            .string(p, TRANSPORT_HEADERS, page)
            .filter(|s| !s.trim().is_empty());
        head
    }

    /// The name an attachment goes under: the long one, the 8.3 one, the one displayed.
    fn attachment_name(&mut self, ap: &Props, page: &'static Encoding) -> Option<String> {
        [ATTACH_LONG_FILENAME, ATTACH_FILENAME, DISPLAY_NAME]
            .into_iter()
            .find_map(|id| self.string(ap, id, page).filter(|s| !s.trim().is_empty()))
    }

    /// The shape of a message: its body in each form it has, and what is attached.
    fn tree(&mut self, storage: usize, header: usize, name: Option<String>, depth: usize) -> Node {
        let p = self.props(storage, header);
        let page = self.page(&p);
        let leaf = |mime: &str, encoding: &str, size: Option<u64>, name: Option<String>| Node {
            mime: mime.to_string(),
            encoding: encoding.to_string(),
            size,
            name,
            offset: None,
            children: Vec::new(),
        };
        let mut children = Vec::new();
        if let Some(size) = self.stored(&p, BODY, &[0x001F, 0x001E]) {
            children.push(leaf("text/plain", "", Some(size), None));
        }
        if let Some(size) = self.stored(&p, HTML, &[0x0102, 0x001F, 0x001E]) {
            children.push(leaf("text/html", "", Some(size), None));
        }
        if let Some(size) = self.stored(&p, RTF, &[0x0102]) {
            children.push(leaf("application/rtf", "compressed", Some(size), None));
        }
        for a in self.numbered(storage, "__attach_version1.0_#") {
            let ap = self.props(a, CHILD);
            let name = self.attachment_name(&ap, page);
            match self.embedded(a) {
                Some(inner) if depth < MAX_NESTING => {
                    children.push(self.tree(inner, EMBEDDED, name, depth + 1));
                }
                Some(_) => children.push(leaf(MIME, "", None, name)),
                None => {
                    let mime = self
                        .string(&ap, ATTACH_MIME, page)
                        .filter(|m| !m.trim().is_empty())
                        .unwrap_or_else(|| "application/octet-stream".into());
                    let size = self.stored(&ap, ATTACH_DATA, &[0x0102]);
                    children.push(leaf(mime.trim(), "", size, name));
                }
            }
        }
        Node {
            mime: MIME.to_string(),
            encoding: String::new(),
            size: None,
            name,
            offset: None,
            children,
        }
    }

    /// The files of a message, pushed on `out` under `prefix`.
    fn unpack(
        &mut self,
        storage: usize,
        header: usize,
        prefix: &str,
        depth: usize,
        limits: &Limits,
        out: &mut Vec<(String, Vec<u8>)>,
    ) -> std::result::Result<(), String> {
        let p = self.props(storage, header);
        let page = self.page(&p);
        // The names taken in this folder, so two attachments called the same are both kept.
        let mut taken: Vec<String> = Vec::new();
        let mut put = |name: &str, bytes: Vec<u8>, out: &mut Vec<(String, Vec<u8>)>| {
            let name = unique(name, &mut taken);
            out.push((format!("{prefix}{name}"), bytes));
        };

        let head = self.head(&p, page);
        let mut shown = String::new();
        for (k, v) in &head.lines {
            shown.push_str(k);
            shown.push_str(": ");
            shown.push_str(&v.replace(['\r', '\n'], " "));
            shown.push('\n');
        }
        if let Some(transport) = &head.transport {
            shown.push('\n');
            shown.push_str(&transport.replace("\r\n", "\n"));
            if !shown.ends_with('\n') {
                shown.push('\n');
            }
        }
        put("headers.txt", shown.into_bytes(), out);
        let tree = self.tree(storage, header, None, depth);
        put("structure.txt", mail::tree_text(&tree).into_bytes(), out);

        let text = self.string(&p, BODY, page);
        let html = match self.binary(&p, HTML) {
            Some(bytes) => Some(bytes),
            None => self.string(&p, HTML, page).map(String::into_bytes),
        };
        // The RTF is unpacked whenever it is there, and what it is used for depends on what
        // else the message has.
        let cap = usize::try_from(self.cap).unwrap_or(usize::MAX);
        let rtf = self
            .binary(&p, RTF)
            .and_then(|packed| crate::rtf::decompress(&packed, cap).ok())
            .filter(|rtf| !rtf.is_empty());
        let wraps = rtf
            .as_deref()
            .is_some_and(|r| crate::rtf::wraps_html(r) || crate::rtf::wraps_text(r));
        match (&text, &rtf) {
            (Some(text), _) => put("message.txt", text.clone().into_bytes(), out),
            (None, Some(rtf)) => put("message.txt", crate::rtf::to_text(rtf).into_bytes(), out),
            (None, None) => {}
        }
        match (html, &rtf) {
            (Some(html), _) => put("message.html", html, out),
            (None, Some(rtf)) => {
                if let Some(html) = crate::rtf::to_html(rtf) {
                    put("message.html", html.into_bytes(), out);
                }
            }
            (None, None) => {}
        }
        // RTF that only wraps another form is that form again; RTF the message was written
        // in is a body of its own.
        if let (Some(rtf), false) = (rtf, wraps) {
            put("message.rtf", rtf, out);
        }

        let mut nameless = 0;
        for a in self.numbered(storage, "__attach_version1.0_#") {
            let ap = self.props(a, CHILD);
            let name = self.attachment_name(&ap, page);
            if let Some(inner) = self.embedded(a) {
                if depth >= MAX_NESTING {
                    continue;
                }
                let mut name = clean(&name.unwrap_or_else(|| "message".into()));
                if !name.to_ascii_lowercase().ends_with(".msg") {
                    name.push_str(".msg");
                }
                let name = unique(&name, &mut taken);
                self.unpack(
                    inner,
                    EMBEDDED,
                    &format!("{prefix}{name}/"),
                    depth + 1,
                    limits,
                    out,
                )?;
            } else if let Some(bytes) = self.binary(&ap, ATTACH_DATA) {
                let name = match name {
                    Some(n) => clean(&n),
                    None => {
                        nameless += 1;
                        format!("attachment{nameless}.bin")
                    }
                };
                let name = unique(&name, &mut taken);
                out.push((format!("{prefix}{name}"), bytes));
            }
            if out.len() > limits.entries {
                return Err(format!("it holds more than {} files", limits.entries));
            }
            let total: u64 = out.iter().map(|(_, b)| b.len() as u64).sum();
            if total > limits.total_bytes {
                return Err(format!(
                    "it unpacks to more than {} MiB",
                    limits.total_bytes >> 20
                ));
            }
        }
        Ok(())
    }
}

/// A name that can be a file's: no separators, no control characters, not empty.
fn clean(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | '\0') || c.is_control() {
                '_'
            } else {
                c
            }
        })
        .collect();
    let cleaned = cleaned.trim();
    if cleaned.is_empty() || cleaned == "." || cleaned == ".." {
        "attachment".into()
    } else {
        cleaned.to_string()
    }
}

/// `name`, or `2 name`, `3 name`… when the folder already has one: the same rule `mail.rs`
/// follows, so the two kinds of message name their duplicates alike.
fn unique(name: &str, taken: &mut Vec<String>) -> String {
    let mut candidate = name.to_string();
    let mut k = 2;
    while taken.iter().any(|t| t.eq_ignore_ascii_case(&candidate)) {
        candidate = format!("{k} {name}");
        k += 1;
    }
    taken.push(candidate.clone());
    candidate
}

fn open<'a>(src: &'a mut dyn ReadAt, entries: usize) -> std::result::Result<Reader<'a>, String> {
    let cfb = Cfb::open(src, entries)?;
    if cfb.child(cfb.root(), PROPERTIES).is_none() {
        return Err("a compound file, but not an Outlook message".into());
    }
    let cap = src.size().min(mail::LIMIT);
    Ok(Reader { cfb, src, cap })
}

/// Is this compound file a message? What tells one from a `.doc` is the table of properties
/// in its root.
pub fn is_message(cfb: &Cfb) -> bool {
    cfb.child(cfb.root(), PROPERTIES).is_some()
}

/// The files a message is made of, with an attached message's under `Name.msg/`.
pub fn files(
    src: &mut dyn ReadAt,
    limits: &Limits,
) -> std::result::Result<Vec<(String, Vec<u8>)>, String> {
    let mut reader = open(src, limits.entries)?;
    let mut out = Vec::new();
    let root = reader.cfb.root();
    reader.unpack(root, TOP, "", 0, limits, &mut out)?;
    Ok(out)
}

/// What the viewer's pane shows of a message: who, to whom, about what and when, and the
/// tree of what it carries. Nothing of the bodies or the attachments is read for it.
pub struct Outline {
    pub headline: Vec<(&'static str, String)>,
    pub tree: Node,
}

pub fn outline(src: &mut dyn ReadAt) -> std::result::Result<Outline, String> {
    let mut reader = open(src, Limits::default().entries)?;
    let root = reader.cfb.root();
    let p = reader.props(root, TOP);
    let page = reader.page(&p);
    let headline = reader.head(&p, page).lines;
    let tree = reader.tree(root, TOP, None, 0);
    Ok(Outline { headline, tree })
}

fn read(path: &Path, name: &str) -> Result<Vec<u8>> {
    let io = |e| ArcError::Io {
        path: name.to_string(),
        source: e,
    };
    let len = std::fs::metadata(path).map_err(io)?.len();
    if len > mail::LIMIT {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!("it is over {} MiB", mail::LIMIT >> 20),
        });
    }
    std::fs::read(path).map_err(io)
}

fn unpacked(path: &Path, name: &str, limits: &Limits) -> Result<Vec<(String, Vec<u8>)>> {
    let bytes = read(path, name)?;
    let mut src = bytes.as_slice();
    files(&mut src, limits).map_err(|reason| ArcError::Broken {
        path: name.to_string(),
        reason,
    })
}

pub fn index(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
    let files = unpacked(path, name, limits)?;
    if cancel.is_cancelled() {
        return Err(ArcError::Cancelled);
    }
    Ok(files
        .into_iter()
        .map(|(n, b)| ArcEntry {
            path: format!("/{n}"),
            size: b.len() as u64,
            packed: None,
            mtime,
            mode: Some(0o444),
            kind: EntryKind::File,
            link_target: None,
            at: None,
        })
        .collect())
}

pub fn copy_out(path: &Path, name: &str, entry: &str, out: &mut dyn std::io::Write) -> Result<u64> {
    let want = entry.trim_start_matches('/');
    // The index was made under the user's limits; reading one entry back needs none.
    let (_, bytes) = unpacked(path, name, &Limits::unlimited())?
        .into_iter()
        .find(|(n, _)| n == want)
        .ok_or_else(|| ArcError::NoEntry {
            path: name.to_string(),
            entry: entry.to_string(),
        })?;
    out.write_all(&bytes).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    Ok(bytes.len() as u64)
}

/// Messages written for the tests, on the compound file writer of `cfb::build`: no `.msg`
/// fixture exists, and one built here says in the test what it holds.
#[doc(hidden)]
pub mod build {
    use super::*;
    use crate::cfb::build::{storage, stream, Item};

    #[derive(Default, Clone)]
    pub struct Draft {
        pub subject: String,
        pub from: (String, String),
        /// Kind (1 To, 2 Cc, 3 Bcc), name, address.
        pub to: Vec<(u32, String, String)>,
        /// A FILETIME.
        pub sent: Option<u64>,
        pub text: Option<String>,
        pub html: Option<Vec<u8>>,
        /// The RTF as written; it is packed on the way in.
        pub rtf: Option<Vec<u8>>,
        pub headers: Option<String>,
        pub attachments: Vec<Attach>,
        /// Strings as bytes in this code page (type 001E) instead of UTF-16 (001F).
        pub code_page: Option<u32>,
    }

    #[derive(Clone)]
    pub enum Attach {
        File {
            name: String,
            mime: Option<String>,
            bytes: Vec<u8>,
        },
        Message {
            name: String,
            draft: Draft,
        },
    }

    fn text(d: &Draft, id: u16, value: &str) -> Item {
        match d.code_page {
            Some(cp) => {
                let bytes = crate::rtf::code_page(cp).encode(value).0.into_owned();
                stream(&format!("__substg1.0_{id:04X}001E"), &bytes)
            }
            None => {
                let bytes: Vec<u8> = value.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
                stream(&format!("__substg1.0_{id:04X}001F"), &bytes)
            }
        }
    }

    fn table(header: usize, fixed: &[(u32, u64)]) -> Item {
        let mut bytes = vec![0u8; header];
        for (tag, value) in fixed {
            bytes.extend_from_slice(&tag.to_le_bytes());
            bytes.extend_from_slice(&6u32.to_le_bytes());
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        stream(PROPERTIES, &bytes)
    }

    /// The streams and storages of one message.
    pub fn items(d: &Draft, header: usize) -> Vec<Item> {
        let mut fixed: Vec<(u32, u64)> = Vec::new();
        if let Some(sent) = d.sent {
            fixed.push(((u32::from(SUBMIT_TIME) << 16) | 0x0040, sent));
        }
        if let Some(cp) = d.code_page {
            fixed.push(((u32::from(MESSAGE_CODE_PAGE) << 16) | 0x0003, u64::from(cp)));
        }
        let mut out = vec![table(header, &fixed)];
        out.push(text(d, SUBJECT, &d.subject));
        if !d.from.0.is_empty() {
            out.push(text(d, SENDER_NAME, &d.from.0));
        }
        if !d.from.1.is_empty() {
            out.push(text(d, SENDER_EMAIL, &d.from.1));
        }
        if let Some(body) = &d.text {
            out.push(text(d, BODY, body));
        }
        if let Some(html) = &d.html {
            out.push(stream(&format!("__substg1.0_{HTML:04X}0102"), html));
        }
        if let Some(rtf) = &d.rtf {
            out.push(stream(
                &format!("__substg1.0_{RTF:04X}0102"),
                &crate::rtf::pack_literals(rtf),
            ));
        }
        if let Some(headers) = &d.headers {
            out.push(text(d, TRANSPORT_HEADERS, headers));
        }
        for (i, (kind, name, email)) in d.to.iter().enumerate() {
            out.push(storage(
                &format!("__recip_version1.0_#{i:08X}"),
                vec![
                    table(
                        CHILD,
                        &[((u32::from(RECIPIENT_TYPE) << 16) | 0x0003, u64::from(*kind))],
                    ),
                    text(d, DISPLAY_NAME, name),
                    text(d, EMAIL, email),
                ],
            ));
        }
        for (i, a) in d.attachments.iter().enumerate() {
            let inner = match a {
                Attach::File { name, mime, bytes } => {
                    let mut v = vec![
                        table(CHILD, &[]),
                        text(d, ATTACH_LONG_FILENAME, name),
                        stream(&format!("__substg1.0_{ATTACH_DATA:04X}0102"), bytes),
                    ];
                    if let Some(mime) = mime {
                        v.push(text(d, ATTACH_MIME, mime));
                    }
                    v
                }
                Attach::Message { name, draft } => vec![
                    table(CHILD, &[]),
                    text(d, DISPLAY_NAME, name),
                    storage(
                        &format!("__substg1.0_{ATTACH_DATA:04X}000D"),
                        items(draft, EMBEDDED),
                    ),
                ],
            };
            out.push(storage(&format!("__attach_version1.0_#{i:08X}"), inner));
        }
        out
    }

    /// The `.msg` file: `major` 3 for 512-byte sectors, 4 for 4096.
    pub fn file(d: &Draft, major: u16) -> Vec<u8> {
        crate::cfb::build::file(major, items(d, TOP))
    }
}

#[cfg(test)]
mod tests {
    use super::build::{file, Attach, Draft};
    use super::*;

    /// 2024-03-05 14:30:00 UTC.
    const SENT: u64 = (1_709_649_000 + 11_644_473_600) * 10_000_000;

    fn plain() -> Draft {
        Draft {
            subject: "Caffè domani".into(),
            from: ("Matteo".into(), "m@example.com".into()),
            to: vec![
                (1, "Anna".into(), "anna@example.com".into()),
                (2, "Luca".into(), "luca@example.com".into()),
            ],
            sent: Some(SENT),
            text: Some("Ciao, un caffè per te.\r\n".into()),
            ..Draft::default()
        }
    }

    fn unpack(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
        let mut src = bytes;
        files(&mut src, &Limits::default()).expect("a message")
    }

    fn text_of(files: &[(String, Vec<u8>)], name: &str) -> String {
        let (_, bytes) = files
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no {name} among {:?}", names(files)));
        String::from_utf8(bytes.clone()).unwrap()
    }

    fn names(files: &[(String, Vec<u8>)]) -> Vec<&str> {
        files.iter().map(|(n, _)| n.as_str()).collect()
    }

    #[test]
    fn a_plain_message_is_its_headers_and_its_text() {
        let files = unpack(&file(&plain(), 3));
        assert_eq!(
            names(&files),
            ["headers.txt", "structure.txt", "message.txt"]
        );
        assert_eq!(
            text_of(&files, "headers.txt"),
            "From: Matteo <m@example.com>\n\
             To: Anna <anna@example.com>\n\
             Cc: Luca <luca@example.com>\n\
             Subject: Caffè domani\n\
             Date: 2024-03-05 14:30:00 UTC\n"
        );
        assert_eq!(text_of(&files, "message.txt"), "Ciao, un caffè per te.\r\n");
        assert_eq!(
            text_of(&files, "structure.txt"),
            "application/vnd.ms-outlook\n└ text/plain  48 B\n"
        );
    }

    #[test]
    fn strings_in_a_code_page_read_the_same_as_unicode_ones() {
        let mut draft = plain();
        draft.code_page = Some(1252);
        draft.headers = Some("Received: from a\r\nX-Mailer: test\r\n".into());
        let files = unpack(&file(&draft, 3));
        let headers = text_of(&files, "headers.txt");
        assert!(headers.contains("Subject: Caffè domani\n"), "{headers}");
        // The headers the message travelled with follow the ones read from its properties.
        assert!(
            headers.ends_with("\n\nReceived: from a\nX-Mailer: test\n"),
            "{headers}"
        );
        assert_eq!(text_of(&files, "message.txt"), "Ciao, un caffè per te.\r\n");
        // And another page is another alphabet.
        let mut draft = plain();
        draft.code_page = Some(1251);
        draft.subject = "Привет".into();
        let files = unpack(&file(&draft, 3));
        assert!(text_of(&files, "headers.txt").contains("Subject: Привет\n"));
    }

    #[test]
    fn an_html_message_keeps_both_bodies() {
        let mut draft = plain();
        draft.html = Some(b"<html><body><p>Ciao</p></body></html>".to_vec());
        let files = unpack(&file(&draft, 3));
        assert_eq!(
            names(&files),
            [
                "headers.txt",
                "structure.txt",
                "message.txt",
                "message.html"
            ]
        );
        assert_eq!(
            text_of(&files, "message.html"),
            "<html><body><p>Ciao</p></body></html>"
        );
    }

    #[test]
    fn a_message_with_only_rtf_is_read_as_text() {
        let mut draft = plain();
        draft.text = None;
        draft.rtf = Some(
            br"{\rtf1\ansi\ansicpg1252{\fonttbl{\f0 Arial;}}\pard Caff\'e8 alle dieci\par A presto\par}"
                .to_vec(),
        );
        let files = unpack(&file(&draft, 3));
        assert_eq!(
            names(&files),
            ["headers.txt", "structure.txt", "message.txt", "message.rtf"]
        );
        assert_eq!(
            text_of(&files, "message.txt"),
            "Caffè alle dieci\nA presto\n"
        );
        assert!(text_of(&files, "message.rtf").starts_with("{\\rtf1"));
        assert!(text_of(&files, "structure.txt").contains("└ application/rtf  compressed  "));
    }

    #[test]
    fn rtf_that_wraps_html_gives_the_html_back_and_no_rtf() {
        let mut draft = plain();
        draft.text = None;
        draft.rtf = Some(
            br"{\rtf1\ansi\fromhtml1 {\*\htmltag64 <p>}\htmlrtf {\htmlrtf0 Ciao{\*\htmltag72 </p>}\htmlrtf\par}\htmlrtf0 }"
                .to_vec(),
        );
        let files = unpack(&file(&draft, 3));
        assert_eq!(
            names(&files),
            [
                "headers.txt",
                "structure.txt",
                "message.txt",
                "message.html"
            ]
        );
        assert_eq!(text_of(&files, "message.html"), "<p>Ciao</p>");
        assert_eq!(text_of(&files, "message.txt"), "Ciao\n");
    }

    #[test]
    fn attachments_come_out_under_their_names_from_either_kind_of_sector() {
        let big: Vec<u8> = (0..20_000u32).map(|i| (i % 253) as u8).collect();
        let mut draft = plain();
        draft.attachments = vec![
            Attach::File {
                name: "notes.txt".into(),
                mime: Some("text/plain".into()),
                bytes: b"short, so it lives in the mini-stream".to_vec(),
            },
            Attach::File {
                name: "scan.bin".into(),
                mime: None,
                bytes: big.clone(),
            },
            Attach::File {
                name: "notes.txt".into(),
                mime: None,
                bytes: b"the second of that name".to_vec(),
            },
            Attach::File {
                name: "../up/and\\away".into(),
                mime: None,
                bytes: b"x".to_vec(),
            },
        ];
        for major in [3, 4] {
            let files = unpack(&file(&draft, major));
            assert_eq!(
                names(&files),
                [
                    "headers.txt",
                    "structure.txt",
                    "message.txt",
                    "notes.txt",
                    "scan.bin",
                    "2 notes.txt",
                    ".._up_and_away"
                ],
                "v{major}"
            );
            assert_eq!(
                text_of(&files, "notes.txt"),
                "short, so it lives in the mini-stream"
            );
            assert_eq!(files[4].1, big, "v{major}");
            assert_eq!(
                text_of(&files, "structure.txt"),
                "application/vnd.ms-outlook\n\
                 ├ text/plain  48 B\n\
                 ├ text/plain  37 B  \"notes.txt\"\n\
                 ├ application/octet-stream  19.5 KB  \"scan.bin\"\n\
                 ├ application/octet-stream  23 B  \"notes.txt\"\n\
                 └ application/octet-stream  1 B  \"../up/and\\away\"\n"
            );
        }
    }

    fn forwarded() -> Draft {
        let mut inner = plain();
        inner.subject = "The original".into();
        inner.text = Some("Forwarded text".into());
        inner.attachments = vec![Attach::File {
            name: "inner.txt".into(),
            mime: None,
            bytes: b"deep".to_vec(),
        }];
        let mut outer = plain();
        outer.subject = "Fwd: The original".into();
        outer.attachments = vec![Attach::Message {
            name: "The original".into(),
            draft: inner,
        }];
        outer
    }

    #[test]
    fn a_message_attached_to_a_message_is_a_folder_in_turn() {
        let files = unpack(&file(&forwarded(), 3));
        assert_eq!(
            names(&files),
            [
                "headers.txt",
                "structure.txt",
                "message.txt",
                "The original.msg/headers.txt",
                "The original.msg/structure.txt",
                "The original.msg/message.txt",
                "The original.msg/inner.txt",
            ]
        );
        assert!(text_of(&files, "The original.msg/headers.txt").contains("Subject: The original\n"));
        assert_eq!(text_of(&files, "The original.msg/inner.txt"), "deep");
        assert_eq!(
            text_of(&files, "structure.txt"),
            "application/vnd.ms-outlook\n\
             ├ text/plain  48 B\n\
             └ application/vnd.ms-outlook  \"The original\"\n\
             \x20 ├ text/plain  28 B\n\
             \x20 └ application/octet-stream  4 B  \"inner.txt\"\n"
        );
    }

    #[test]
    fn the_archive_lists_a_message_as_a_tree_and_reads_its_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fwd.msg");
        std::fs::write(&path, file(&forwarded(), 4)).unwrap();
        let archive =
            crate::Archive::open(&path, &Limits::default(), &CancelToken::new()).expect("a msg");
        assert_eq!(archive.format(), Some(crate::Format::Msg));
        let folder = archive.entry("/The original.msg").expect("the folder");
        assert_eq!(folder.kind, EntryKind::Dir);
        let read = archive
            .read("/The original.msg/inner.txt", &CancelToken::new())
            .unwrap();
        assert_eq!(read, b"deep");
        assert!(archive.read("/nothing.txt", &CancelToken::new()).is_err());
    }

    #[test]
    fn the_outline_reads_no_body_and_names_everything() {
        let bytes = file(&forwarded(), 3);
        let mut src = bytes.as_slice();
        let outline = outline(&mut src).unwrap();
        assert_eq!(
            outline.headline[0],
            ("From", "Matteo <m@example.com>".into())
        );
        assert!(outline
            .headline
            .contains(&("Subject", "Fwd: The original".into())));
        assert_eq!(outline.tree.children.len(), 2);
        assert_eq!(outline.tree.children[1].children.len(), 2);
    }

    #[test]
    fn a_compound_file_that_is_not_a_message_says_so() {
        let doc = crate::cfb::build::file(3, vec![crate::cfb::build::stream("WordDocument", b"x")]);
        let mut src = doc.as_slice();
        let err = files(&mut src, &Limits::default()).unwrap_err();
        assert!(err.contains("not an Outlook message"), "{err}");
        let mut src = &b"not even a compound file"[..];
        assert!(files(&mut src, &Limits::default()).is_err());
    }

    #[test]
    fn the_limits_refuse_a_message_that_unpacks_to_too_much() {
        let mut draft = plain();
        draft.attachments = (0..5)
            .map(|i| Attach::File {
                name: format!("a{i}.bin"),
                mime: None,
                bytes: vec![0; 5000],
            })
            .collect();
        let bytes = file(&draft, 3);
        let limits = Limits {
            total_bytes: 10_000,
            ..Limits::default()
        };
        let mut src = bytes.as_slice();
        assert!(files(&mut src, &limits).unwrap_err().contains("MiB"));
    }

    #[test]
    fn a_damaged_message_is_an_error_or_a_shorter_message_and_never_a_panic() {
        let mut draft = forwarded();
        draft.rtf = Some(br"{\rtf1 body\par}".to_vec());
        draft.html = Some(b"<p>x</p>".to_vec());
        for major in [3, 4] {
            let bytes = file(&draft, major);
            // Cut short anywhere…
            for cut in (0..bytes.len()).step_by(101) {
                let mut src = &bytes[..cut];
                let _ = files(&mut src, &Limits::default());
                let mut src = &bytes[..cut];
                let _ = outline(&mut src);
            }
            // …and with rubbish written over each stretch of it in turn.
            for at in (0..bytes.len()).step_by(61) {
                for v in [0x00u8, 0xFF, 0x7F] {
                    let mut bad = bytes.clone();
                    for b in bad.iter_mut().skip(at).take(7) {
                        *b = v;
                    }
                    let mut src = bad.as_slice();
                    let _ = files(&mut src, &Limits::default());
                    let mut src = bad.as_slice();
                    let _ = outline(&mut src);
                }
            }
        }
    }
}
