//! The RTF body of an Outlook message: unpacked (MS-OXRTFCP), and reduced to the text or the
//! HTML it wraps (MS-OXRTFEX).
//!
//! Outlook keeps a message's body as RTF whatever it was written in. A message that arrived
//! as HTML carries the HTML *inside* the RTF, tag by tag in groups a word processor skips,
//! and one that has neither a plain body nor an HTML one has only this. So the reader here
//! does two things with it: take the HTML back out when the RTF says it wraps some, and
//! otherwise read the RTF as far as a mail needs — its text, its paragraphs, its characters
//! in whatever code page it declares. It is not a word processor: tables come out as tabs,
//! pictures and fonts not at all.

use encoding_rs::Encoding;

/// What the dictionary of the decompressor starts as: the RTF every message begins with, so
/// that the first reference can already point into it.
const PRESET: &[u8] = b"{\\rtf1\\ansi\\mac\\deff0\\deftab720{\\fonttbl;}{\\f0\\fnil \\froman \\fswiss \\fmodern \\fscript \\fdecor MS Sans SerifSymbolArialTimes New RomanCourier{\\colortbl\\red0\\green0\\blue0\r\n\\par \\pard\\plain\\f0\\fs20\\b\\i\\u\\tab\\tx";

/// `LZFu`: compressed.
const COMPRESSED: u32 = 0x7546_5A4C;
/// `MELA`: stored as it is.
const STORED: u32 = 0x414C_454D;

fn le32(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// The CRC of MS-OXRTFCP §2.1.3.1.3: the usual CRC-32 table, started at zero and not
/// inverted at either end.
fn crc(bytes: &[u8]) -> u32 {
    let mut crc = 0u32;
    for &b in bytes {
        let mut v = (crc ^ u32::from(b)) & 0xFF;
        for _ in 0..8 {
            v = if v & 1 != 0 {
                0xEDB8_8320 ^ (v >> 1)
            } else {
                v >> 1
            };
        }
        crc = v ^ (crc >> 8);
    }
    crc
}

/// Unpack a `PR_RTF_COMPRESSED` stream. `cap` bounds what comes out, whatever the header
/// says: the size it gives is the writer's claim and is only used to stop early.
pub fn decompress(input: &[u8], cap: usize) -> Result<Vec<u8>, String> {
    const SHORT: &str = "the RTF header is cut short";
    let comp_size = le32(input, 0).ok_or(SHORT)? as usize;
    let raw_size = le32(input, 4).ok_or(SHORT)? as usize;
    let kind = le32(input, 8).ok_or(SHORT)?;
    let sum = le32(input, 12).ok_or(SHORT)?;
    // COMPSIZE counts everything after itself.
    let end = comp_size.saturating_add(4).min(input.len());
    let body = input.get(16..end).unwrap_or_default();
    if raw_size > cap {
        return Err("the RTF body is over the size limit".into());
    }
    match kind {
        STORED => Ok(body[..raw_size.min(body.len())].to_vec()),
        COMPRESSED => {
            if crc(body) != sum {
                return Err("the compressed RTF fails its checksum".into());
            }
            let mut dict = [0u8; 4096];
            dict[..PRESET.len()].copy_from_slice(PRESET);
            let mut write = PRESET.len();
            // A reference of two bytes gives seventeen at most, so the claim is believed
            // only as far as the input could deliver it.
            let mut out = Vec::with_capacity(raw_size.min(body.len().saturating_mul(9)));
            let mut at = 0;
            'run: while at < body.len() {
                let control = body[at];
                at += 1;
                for bit in 0..8 {
                    if out.len() >= raw_size {
                        break 'run;
                    }
                    if control & (1 << bit) == 0 {
                        let Some(&b) = body.get(at) else { break 'run };
                        at += 1;
                        out.push(b);
                        dict[write] = b;
                        write = (write + 1) % 4096;
                    } else {
                        let (Some(&hi), Some(&lo)) = (body.get(at), body.get(at + 1)) else {
                            break 'run;
                        };
                        at += 2;
                        let offset = (usize::from(hi) << 4) | usize::from(lo >> 4);
                        let length = usize::from(lo & 0x0F) + 2;
                        // A reference to where the next byte would go is the end mark.
                        if offset == write {
                            break 'run;
                        }
                        for k in 0..length {
                            if out.len() >= raw_size {
                                break 'run;
                            }
                            let b = dict[(offset + k) % 4096];
                            out.push(b);
                            dict[write] = b;
                            write = (write + 1) % 4096;
                        }
                    }
                }
            }
            Ok(out)
        }
        other => Err(format!("an RTF compression called {other:08X}")),
    }
}

/// The decoder for a Windows code page number, as `\ansicpg` and the MAPI properties give
/// it. One this program has no table for reads as Windows-1252, which is what most of them
/// are a variation of.
pub fn code_page(cp: u32) -> &'static Encoding {
    let label = match cp {
        65001 => "utf-8".to_string(),
        932 => "shift_jis".to_string(),
        936 => "gbk".to_string(),
        949 => "euc-kr".to_string(),
        950 => "big5".to_string(),
        10000 => "macintosh".to_string(),
        20866 => "koi8-r".to_string(),
        21866 => "koi8-u".to_string(),
        28591..=28605 => format!("iso-8859-{}", cp - 28590),
        n => format!("windows-{n}"),
    };
    Encoding::for_label(label.as_bytes()).unwrap_or(encoding_rs::WINDOWS_1252)
}

/// Does this RTF wrap an HTML body (`\fromhtml1`)? The mark is in the header, before any
/// text, so only the front is looked at.
pub fn wraps_html(rtf: &[u8]) -> bool {
    let head = &rtf[..rtf.len().min(4096)];
    head.windows(9).any(|w| w == b"\\fromhtml")
}

/// Does it wrap a plain text body (`\fromtext`)?
pub fn wraps_text(rtf: &[u8]) -> bool {
    let head = &rtf[..rtf.len().min(4096)];
    head.windows(9).any(|w| w == b"\\fromtext")
}

/// The text of an RTF document: paragraphs as lines, characters decoded, everything a
/// reader of mail does not see — fonts, colours, styles, pictures — left out.
pub fn to_text(rtf: &[u8]) -> String {
    let text = run(rtf, false);
    // RTF ends a paragraph where a text file would only end a line, and a body taken out
    // of its wrapping often closes with a run of empty ones.
    let mut out = String::with_capacity(text.len());
    let mut blank = 0;
    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        out.push_str(line);
        out.push('\n');
    }
    out.trim_matches('\n').to_string() + "\n"
}

/// The HTML an RTF body wraps, or `None` when it wraps none.
pub fn to_html(rtf: &[u8]) -> Option<String> {
    wraps_html(rtf).then(|| run(rtf, true))
}

/// What a group carries, restored when it closes.
#[derive(Clone, Copy)]
struct Group {
    /// Nothing in this group is text: a font table, a picture, an unknown `\*` destination.
    skip: bool,
    /// `{\*\htmltag …}`: a piece of the original HTML.
    tag: bool,
    /// `\htmlrtf`: RTF written for a word processor, which the HTML does not contain.
    rtf_only: bool,
    /// How many characters after a `\uN` are its fallback, to be dropped.
    uc: usize,
}

/// The destinations whose contents are not the document's text.
const NOT_TEXT: [&str; 22] = [
    "fonttbl",
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "object",
    "header",
    "headerl",
    "headerr",
    "headerf",
    "footer",
    "footerl",
    "footerr",
    "footerf",
    "footnote",
    "themedata",
    "colorschememapping",
    "latentstyles",
    "datastore",
    "generator",
    "listtable",
    "revtbl",
];

/// One pass over the RTF, for either of its two readings. `html` is the unwrapping: the
/// `\*\htmltag` groups are the output and the `\htmlrtf` stretches are dropped; without it
/// every `\*` group is dropped and the rest is the text.
fn run(rtf: &[u8], html: bool) -> String {
    let mut out = String::new();
    // Bytes of the current code page waiting to be decoded together, so a two-byte
    // character split over two `\'hh` is one character.
    let mut bytes: Vec<u8> = Vec::new();
    // And UTF-16 units from `\uN`, for the same reason with surrogates.
    let mut units: Vec<u16> = Vec::new();
    let mut encoding = encoding_rs::WINDOWS_1252;
    let mut stack: Vec<Group> = Vec::new();
    let mut now = Group {
        skip: false,
        tag: false,
        rtf_only: false,
        uc: 1,
    };
    // `\*` seen, and the control word it applies to not yet.
    let mut starred = false;
    // Fallback characters still to drop after a `\uN`.
    let mut drop = 0usize;
    let mut i = 0;

    macro_rules! flush {
        () => {
            if !bytes.is_empty() {
                out.push_str(&encoding.decode_without_bom_handling(&bytes).0);
                bytes.clear();
            }
            if !units.is_empty() {
                out.push_str(&String::from_utf16_lossy(&units));
                units.clear();
            }
        };
    }
    let shown = |g: &Group| {
        if html {
            (g.tag || !g.skip) && !g.rtf_only
        } else {
            !g.skip
        }
    };
    let newline = if html { "\r\n" } else { "\n" };

    while i < rtf.len() {
        let c = rtf[i];
        match c {
            b'{' => {
                flush!();
                // A file of nothing but `{` must not cost memory for each of them.
                if stack.len() < 10_000 {
                    stack.push(now);
                }
                starred = false;
                drop = 0;
                i += 1;
            }
            b'}' => {
                flush!();
                if let Some(g) = stack.pop() {
                    now = g;
                }
                starred = false;
                drop = 0;
                i += 1;
            }
            b'\\' => {
                i += 1;
                let Some(&n) = rtf.get(i) else { break };
                if n.is_ascii_alphabetic() {
                    let start = i;
                    while i < rtf.len() && rtf[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    let word = std::str::from_utf8(&rtf[start..i]).unwrap_or("");
                    let num_start = i;
                    if rtf.get(i) == Some(&b'-') {
                        i += 1;
                    }
                    while i < rtf.len() && rtf[i].is_ascii_digit() && i - num_start < 11 {
                        i += 1;
                    }
                    let param: Option<i64> = std::str::from_utf8(&rtf[num_start..i])
                        .ok()
                        .and_then(|s| s.parse().ok());
                    if param.is_none() {
                        i = num_start;
                    }
                    // One space after a control word belongs to it.
                    if rtf.get(i) == Some(&b' ') {
                        i += 1;
                    }
                    if starred {
                        starred = false;
                        if html && word == "htmltag" {
                            now.tag = true;
                        } else {
                            now.skip = true;
                        }
                        continue;
                    }
                    match word {
                        "u" => {
                            if drop > 0 {
                                drop -= 1;
                                continue;
                            }
                            if shown(&now) {
                                if !bytes.is_empty() {
                                    flush!();
                                }
                                units.push(param.unwrap_or(0) as u16);
                            }
                            drop = now.uc;
                            continue;
                        }
                        "uc" => now.uc = param.unwrap_or(1).clamp(0, 16) as usize,
                        "ansicpg" => {
                            flush!();
                            encoding = code_page(param.unwrap_or(1252).clamp(0, 70_000) as u32);
                        }
                        "bin" => {
                            // Raw bytes follow, as many as the parameter says.
                            let n = usize::try_from(param.unwrap_or(0)).unwrap_or(0);
                            i = i.saturating_add(n).min(rtf.len());
                        }
                        "htmlrtf" => {
                            flush!();
                            now.rtf_only = param != Some(0);
                        }
                        w if NOT_TEXT.contains(&w) => now.skip = true,
                        _ => {
                            let text = match word {
                                "par" | "line" | "sect" | "page" | "row" => newline,
                                "tab" | "cell" => "\t",
                                "emdash" => "\u{2014}",
                                "endash" => "\u{2013}",
                                "lquote" => "\u{2018}",
                                "rquote" => "\u{2019}",
                                "ldblquote" => "\u{201C}",
                                "rdblquote" => "\u{201D}",
                                "bullet" => "\u{2022}",
                                "emspace" | "enspace" | "qmspace" => " ",
                                _ => "",
                            };
                            if !text.is_empty() && shown(&now) {
                                flush!();
                                out.push_str(text);
                            }
                        }
                    }
                    drop = 0;
                } else {
                    i += 1;
                    match n {
                        b'\'' => {
                            let hex = rtf
                                .get(i..i + 2)
                                .and_then(|h| std::str::from_utf8(h).ok())
                                .and_then(|h| u8::from_str_radix(h, 16).ok());
                            i = (i + 2).min(rtf.len());
                            if drop > 0 {
                                drop -= 1;
                            } else if let (Some(b), true) = (hex, shown(&now)) {
                                if !units.is_empty() {
                                    flush!();
                                }
                                bytes.push(b);
                            }
                        }
                        b'*' => starred = true,
                        b'\\' | b'{' | b'}' => {
                            if drop > 0 {
                                drop -= 1;
                            } else if shown(&now) {
                                flush!();
                                out.push(n as char);
                            }
                        }
                        b'~' if shown(&now) => {
                            flush!();
                            out.push('\u{a0}');
                        }
                        b'_' if shown(&now) => {
                            flush!();
                            out.push('-');
                        }
                        // A backslash before a line break is a paragraph.
                        b'\r' | b'\n' if shown(&now) => {
                            flush!();
                            out.push_str(newline);
                        }
                        _ => {}
                    }
                }
            }
            // Line breaks in the source are layout of the file, not of the text.
            b'\r' | b'\n' => i += 1,
            _ => {
                i += 1;
                if drop > 0 {
                    drop -= 1;
                } else if shown(&now) {
                    if !units.is_empty() {
                        flush!();
                    }
                    bytes.push(c);
                }
            }
        }
    }
    flush!();
    out
}

/// Pack RTF the way the tests' messages carry it: every byte a literal, then the end mark.
/// A real writer finds repeats; a reader only has to be given something valid. Here rather
/// than in a test module because the viewer's tests build messages too.
#[doc(hidden)]
pub fn pack_literals(rtf: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    let mut write = PRESET.len();
    let mut chunks = rtf.chunks(8).peekable();
    let mut ended = false;
    while let Some(chunk) = chunks.next() {
        write = (write + chunk.len()) % 4096;
        if chunk.len() == 8 || chunks.peek().is_some() {
            body.push(0);
            body.extend_from_slice(chunk);
        } else {
            // The last, short run shares its control byte with the end mark.
            body.push(1 << chunk.len());
            body.extend_from_slice(chunk);
            body.push((write >> 4) as u8);
            body.push(((write & 0x0F) << 4) as u8);
            ended = true;
        }
    }
    if !ended {
        body.push(1);
        body.push((write >> 4) as u8);
        body.push(((write & 0x0F) << 4) as u8);
    }
    let mut out = Vec::new();
    out.extend_from_slice(&(body.len() as u32 + 12).to_le_bytes());
    out.extend_from_slice(&(rtf.len() as u32).to_le_bytes());
    out.extend_from_slice(&COMPRESSED.to_le_bytes());
    out.extend_from_slice(&crc(&body).to_le_bytes());
    out.extend_from_slice(&body);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_two_examples_of_the_specification_unpack_to_what_it_says() {
        assert_eq!(PRESET.len(), 207);
        // MS-OXRTFCP §3.1: a simple one, with one reference into the preset dictionary…
        let simple = [
            0x2d, 0x00, 0x00, 0x00, 0x2b, 0x00, 0x00, 0x00, 0x4c, 0x5a, 0x46, 0x75, 0xf1, 0xc5,
            0xc7, 0xa7, 0x03, 0x00, 0x0a, 0x00, 0x72, 0x63, 0x70, 0x67, 0x31, 0x32, 0x35, 0x42,
            0x32, 0x0a, 0xf3, 0x20, 0x68, 0x65, 0x6c, 0x09, 0x00, 0x20, 0x62, 0x77, 0x05, 0xb0,
            0x6c, 0x64, 0x7d, 0x0a, 0x80, 0x0f, 0xa0,
        ];
        assert_eq!(
            String::from_utf8(decompress(&simple, 1 << 20).unwrap()).unwrap(),
            "{\\rtf1\\ansi\\ansicpg1252\\pard hello world}\r\n"
        );
        // …and §3.2: one that crosses the write position, copying what it has just written.
        let crossing = [
            0x1a, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x00, 0x00, 0x4c, 0x5a, 0x46, 0x75, 0xe2, 0xd4,
            0x4b, 0x51, 0x41, 0x00, 0x04, 0x20, 0x57, 0x58, 0x59, 0x5a, 0x0d, 0x6e, 0x7d, 0x01,
            0x0e, 0xb0,
        ];
        assert_eq!(
            String::from_utf8(decompress(&crossing, 1 << 20).unwrap()).unwrap(),
            "{\\rtf1 WXYZWXYZWXYZWXYZWXYZ}"
        );
    }

    #[test]
    fn a_damaged_stream_fails_its_checksum_and_a_short_one_does_not_panic() {
        let rtf = b"{\\rtf1\\ansi some text that is long enough to take several runs}";
        let packed = pack_literals(rtf);
        assert_eq!(decompress(&packed, 1 << 20).unwrap(), rtf);
        // A length that is a multiple of eight ends on a control byte of its own.
        assert_eq!(
            decompress(&pack_literals(&rtf[..16]), 1 << 20).unwrap(),
            &rtf[..16]
        );
        let mut bad = packed.clone();
        *bad.last_mut().unwrap() ^= 0x10;
        assert!(decompress(&bad, 1 << 20).unwrap_err().contains("checksum"));
        for cut in 0..packed.len() {
            let _ = decompress(&packed[..cut], 1 << 20);
        }
        // The size the header claims is refused when it is over the cap.
        assert!(decompress(&packed, 8).is_err());
        // Stored, not compressed.
        let mut stored = Vec::new();
        stored.extend_from_slice(&(rtf.len() as u32 + 12).to_le_bytes());
        stored.extend_from_slice(&(rtf.len() as u32).to_le_bytes());
        stored.extend_from_slice(&STORED.to_le_bytes());
        stored.extend_from_slice(&0u32.to_le_bytes());
        stored.extend_from_slice(rtf);
        assert_eq!(decompress(&stored, 1 << 20).unwrap(), rtf);
    }

    #[test]
    fn rtf_is_read_as_its_text() {
        let rtf =
            br"{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0 Arial;}}{\colortbl;\red0\green0\blue0;}
{\*\generator Riched20;}\pard\f0\fs20 Caff\'e8 e t\'e8\par
Second \b bold\b0  line\tab tabbed\line
\u8364?uro \{braces\} \\ and \u-10179?\u-8704? done\par}";
        assert_eq!(
            to_text(rtf),
            "Caffè e tè\nSecond bold line\ttabbed\n€uro {braces} \\ and \u{1F600} done\n"
        );
    }

    #[test]
    fn the_code_page_decides_what_an_escaped_byte_is() {
        // Cyrillic in 1251, and a two-byte Shift-JIS character split over two escapes.
        assert_eq!(
            to_text(br"{\rtf1\ansi\ansicpg1251 \'cf\'f0\'e8\'e2\'e5\'f2}"),
            "Привет\n"
        );
        assert_eq!(
            to_text(br"{\rtf1\ansi\ansicpg932 \'93\'fa\'96\'7b}"),
            "日本\n"
        );
        // \uc2 drops two fallback characters, escaped or not.
        assert_eq!(to_text(br"{\rtf1\uc2 a\u233\'82\'a0b}"), "a\u{e9}b\n");
    }

    #[test]
    fn html_wrapped_in_rtf_comes_back_out() {
        let rtf = br#"{\rtf1\ansi\ansicpg1252\fromhtml1 \deff0{\fonttbl{\f0\fswiss Arial;}}
{\*\htmltag19 <html>}{\*\htmltag34 <head>}{\*\htmltag50 <body>}\htmlrtf \par\htmlrtf0
{\*\htmltag64 <p>}\htmlrtf {\htmlrtf0 Caff\'e8 & {\*\htmltag84 <a href="http://x/\{a\}">}\htmlrtf {\field{\*\fldinst{HYPERLINK "http://x/"}}{\fldrslt\cf1\ul \htmlrtf0 link\htmlrtf }\htmlrtf0 \htmlrtf }\htmlrtf0 {\*\htmltag92 </a>}\htmlrtf\par}\htmlrtf0
{\*\htmltag72 </p>}{\*\htmltag58 </body>}{\*\htmltag27 </html>}}"#;
        assert!(wraps_html(rtf));
        assert_eq!(
            to_html(rtf).unwrap(),
            "<html><head><body><p>Caffè & <a href=\"http://x/{a}\">link</a></p></body></html>"
        );
        // The same body read as text: the tags go, the paragraphs stay.
        assert_eq!(to_text(rtf), "Caffè & link\n");
        assert!(to_html(br"{\rtf1 plain}").is_none());
    }

    #[test]
    fn rubbish_is_read_without_a_panic() {
        for bad in [
            &b"{{{{{{{{"[..],
            b"}}}}}}}",
            b"\\",
            b"\\'",
            b"\\'z",
            b"\\u",
            b"\\u99999999999999999999",
            b"\\bin99999999999 x",
            b"{\\*",
            b"\\uc-5\\u1 abc",
            &[0xFF, 0xFE, b'\\', 0xFF],
        ] {
            let _ = to_text(bad);
            let _ = run(bad, true);
        }
    }
}
