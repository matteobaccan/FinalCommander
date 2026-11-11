//! What a picture or a piece of music says about itself: the camera, the date and the place in
//! a JPEG's EXIF, the text a PNG carries, the title and artist of an MP3, a FLAC or an Ogg
//! file (SPEC 05 §1.8).
//!
//! These are the files a commander is most often asked "what is this one" about, and the answer
//! is in a few hundred bytes near the start of each. The readers below read only those bytes —
//! the headers and the tag blocks, never the picture or the sound — and stop at the first
//! length that does not fit, so a damaged file gives fewer rows rather than wrong ones.

use crate::structure::{Bytes, Row, Structure};

/// How much of the start of a file the tag readers look at. EXIF, ID3 and the FLAC and Ogg
/// comment blocks all sit inside it in every file one meets; a tag block that does not is
/// reported as cut rather than chased.
const HEAD: usize = 512 * 1024;

pub(crate) fn read(src: &mut dyn Bytes, head: &[u8], len: u64) -> Option<Structure> {
    if head.starts_with(&[0xFF, 0xD8, 0xFF]) {
        let b = src.at(0, HEAD);
        return Some(jpeg(&b, len));
    }
    if head.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Some(png(src, len));
    }
    if head.starts_with(b"RIFF") && head.get(8..12) == Some(b"WEBP") {
        return Some(webp(src, len));
    }
    if head.starts_with(b"GIF87a") || head.starts_with(b"GIF89a") {
        return Some(gif(head));
    }
    if head.starts_with(b"ID3") {
        let b = src.at(0, HEAD);
        return Some(id3(&b, len));
    }
    if head.starts_with(b"fLaC") {
        let b = src.at(0, HEAD);
        return Some(flac(&b));
    }
    if head.starts_with(b"OggS") {
        let b = src.at(0, HEAD.min(256 * 1024));
        return ogg(&b);
    }
    // An MP3 with no ID3v2 in front may still carry the 128-byte ID3v1 tag at its end.
    if head.len() >= 2 && head[0] == 0xFF && head[1] & 0xE0 == 0xE0 && len >= 128 {
        let tail = src.at(len - 128, 128);
        if tail.starts_with(b"TAG") {
            return Some(id3v1(&tail));
        }
    }
    None
}

fn structure(format: &'static str, summary: String, rows: Vec<Row>, truncated: bool) -> Structure {
    Structure {
        format,
        summary,
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated,
    }
}

fn be16(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn text(b: &[u8]) -> String {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    String::from_utf8_lossy(&b[..end]).trim().to_string()
}

fn field(label: impl Into<String>, value: impl Into<String>) -> Row {
    Row::field(label, String::new(), value)
}

// ── JPEG and EXIF ───────────────────────────────────────────────────────────────────────────

fn jpeg(b: &[u8], len: u64) -> Structure {
    let mut rows = vec![Row::group("JPEG")];
    let mut exif = Vec::new();
    let mut texts = Vec::new();
    let mut c2pa: Vec<u8> = Vec::new();
    let mut size = None;
    let mut at = 2usize;
    let mut cut = false;
    while at + 4 <= b.len() {
        if b[at] != 0xFF {
            break;
        }
        let marker = b[at + 1];
        // Fill bytes and the markers that carry no length.
        if marker == 0xFF {
            at += 1;
            continue;
        }
        if matches!(marker, 0x01 | 0xD0..=0xD7) {
            at += 2;
            continue;
        }
        let Some(seg) = be16(b, at + 2).map(|l| l as usize) else {
            break;
        };
        if seg < 2 {
            break;
        }
        let Some(body) = b.get(at + 4..at + 2 + seg) else {
            cut = true;
            break;
        };
        match marker {
            0xC0..=0xCF if !matches!(marker, 0xC4 | 0xC8 | 0xCC) && body.len() >= 6 => {
                let height = be16(body, 1).unwrap_or(0);
                let width = be16(body, 3).unwrap_or(0);
                let kind = if marker == 0xC2 {
                    "progressive"
                } else {
                    "baseline"
                };
                size = Some((width, height));
                rows.push(field("size", format!("{width} × {height}")));
                rows.push(field(
                    "encoding",
                    format!("{kind}, {} components, {} bits", body[5], body[0]),
                ));
            }
            0xE0 if body.starts_with(b"JFIF\0") && body.len() >= 12 => {
                let unit = match body[7] {
                    1 => "dpi",
                    2 => "dots per cm",
                    _ => "",
                };
                let (x, y) = (be16(body, 8).unwrap_or(0), be16(body, 10).unwrap_or(0));
                if !unit.is_empty() && x > 0 {
                    rows.push(field("density", format!("{x} × {y} {unit}")));
                }
            }
            0xE1 if body.starts_with(b"Exif\0\0") => {
                exif = exif_rows(&body[6..]);
                texts = crate::aigen::exif_texts(&body[6..], (at + 4 + 6) as u64);
            }
            // APP11 is JUMBF, which is where C2PA keeps its manifest, in as many segments as
            // it takes.
            0xEB if body.starts_with(b"JP") && body.len() > 8 => {
                c2pa.extend_from_slice(&body[8..]);
            }
            0xFE => {
                let c = text(body);
                if !c.is_empty() {
                    rows.push(field("comment", c));
                }
            }
            // Start of scan: the picture itself, which is not read.
            0xDA => break,
            _ => {}
        }
        at += 2 + seg;
    }
    if !exif.is_empty() {
        rows.push(Row::group("EXIF"));
        rows.extend(exif);
    }
    let c2pa = (find(&c2pa, b"c2pa").is_some()).then_some(c2pa);
    rows.extend(crate::aigen::rows(&texts, c2pa.as_deref()));
    let summary = match size {
        Some((w, h)) => format!("JPEG · {w} × {h}"),
        None => "JPEG".into(),
    };
    structure("JPEG", summary, rows, cut && (b.len() as u64) < len)
}

/// A TIFF structure, as EXIF keeps one: the byte order, then IFDs of twelve-byte entries.
struct Tiff<'a> {
    b: &'a [u8],
    little: bool,
}

impl Tiff<'_> {
    fn u16(&self, at: usize) -> Option<u16> {
        let s: [u8; 2] = self.b.get(at..at + 2)?.try_into().ok()?;
        Some(if self.little {
            u16::from_le_bytes(s)
        } else {
            u16::from_be_bytes(s)
        })
    }

    fn u32(&self, at: usize) -> Option<u32> {
        let s: [u8; 4] = self.b.get(at..at + 4)?.try_into().ok()?;
        Some(if self.little {
            u32::from_le_bytes(s)
        } else {
            u32::from_be_bytes(s)
        })
    }

    /// The entries of the IFD at `at`: `(tag, type, count, value-or-offset field position)`.
    fn ifd(&self, at: usize) -> Vec<(u16, u16, u32, usize)> {
        let Some(n) = self.u16(at) else {
            return Vec::new();
        };
        (0..n.min(512) as usize)
            .map(|i| at + 2 + i * 12)
            .filter_map(|e| Some((self.u16(e)?, self.u16(e + 2)?, self.u32(e + 4)?, e + 8)))
            .collect()
    }

    /// Where an entry's data is: in the entry itself when it fits in four bytes.
    fn data(&self, kind: u16, count: u32, field: usize) -> Option<&[u8]> {
        let unit = match kind {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 | 11 => 4,
            5 | 10 | 12 => 8,
            _ => return None,
        };
        let size = unit * count as usize;
        let at = if size <= 4 {
            field
        } else {
            self.u32(field)? as usize
        };
        self.b.get(at..at + size)
    }

    fn ascii(&self, kind: u16, count: u32, field: usize) -> Option<String> {
        (kind == 2)
            .then(|| self.data(kind, count, field).map(text))
            .flatten()
            .filter(|s| !s.is_empty())
    }

    fn number(&self, kind: u16, count: u32, field: usize) -> Option<u32> {
        match kind {
            3 => self.u16(field).map(u32::from),
            4 => self.u32(field),
            _ => None,
        }
        .filter(|_| count >= 1)
    }

    fn rationals(&self, kind: u16, count: u32, field: usize) -> Vec<f64> {
        if kind != 5 && kind != 10 {
            return Vec::new();
        }
        let Some(at) = self.u32(field).map(|a| a as usize) else {
            return Vec::new();
        };
        (0..count.min(8) as usize)
            .filter_map(|i| {
                let n = self.u32(at + i * 8)? as f64;
                let d = self.u32(at + i * 8 + 4)? as f64;
                (d != 0.0).then_some(n / d)
            })
            .collect()
    }
}

fn exif_rows(b: &[u8]) -> Vec<Row> {
    let little = match b.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Vec::new(),
    };
    let t = Tiff { b, little };
    let Some(first) = t.u32(4) else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    let mut exif_ifd = None;
    let mut gps_ifd = None;
    for (tag, kind, count, f) in t.ifd(first as usize) {
        let label = match tag {
            0x010F => "camera maker",
            0x0110 => "camera",
            0x0131 => "software",
            0x0132 => "modified",
            0x013B => "artist",
            0x8298 => "copyright",
            0x010E => "description",
            0x0112 => {
                if let Some(o) = t.number(kind, count, f) {
                    let said = match o {
                        1 => "normal",
                        3 => "upside down",
                        6 => "turned right",
                        8 => "turned left",
                        2 | 4 | 5 | 7 => "mirrored",
                        _ => "",
                    };
                    if !said.is_empty() {
                        rows.push(field("orientation", said));
                    }
                }
                continue;
            }
            0x8769 => {
                exif_ifd = t.number(kind, count, f);
                continue;
            }
            0x8825 => {
                gps_ifd = t.number(kind, count, f);
                continue;
            }
            _ => continue,
        };
        if let Some(v) = t.ascii(kind, count, f) {
            rows.push(field(label, v));
        }
    }
    if let Some(at) = exif_ifd {
        for (tag, kind, count, f) in t.ifd(at as usize) {
            match tag {
                0x9003 => {
                    if let Some(v) = t.ascii(kind, count, f) {
                        rows.push(field("taken", v));
                    }
                }
                0xA434 => {
                    if let Some(v) = t.ascii(kind, count, f) {
                        rows.push(field("lens", v));
                    }
                }
                0x829A => {
                    if let Some(&s) = t.rationals(kind, count, f).first() {
                        let shown = if s > 0.0 && s < 1.0 {
                            format!("1/{:.0} s", 1.0 / s)
                        } else {
                            format!("{s} s")
                        };
                        rows.push(field("exposure", shown));
                    }
                }
                0x829D => {
                    if let Some(&n) = t.rationals(kind, count, f).first() {
                        rows.push(field("aperture", format!("f/{n:.1}")));
                    }
                }
                0x8827 => {
                    if let Some(n) = t.number(kind, count, f) {
                        rows.push(field("ISO", n.to_string()));
                    }
                }
                0x920A => {
                    if let Some(&n) = t.rationals(kind, count, f).first() {
                        rows.push(field("focal length", format!("{n:.0} mm")));
                    }
                }
                _ => {}
            }
        }
    }
    if let Some(at) = gps_ifd {
        let mut lat = None;
        let mut lon = None;
        let mut lat_ref = 'N';
        let mut lon_ref = 'E';
        for (tag, kind, count, f) in t.ifd(at as usize) {
            match tag {
                1 => {
                    lat_ref = t
                        .data(kind, count, f)
                        .and_then(|d| d.first())
                        .map_or('N', |&c| c as char)
                }
                3 => {
                    lon_ref = t
                        .data(kind, count, f)
                        .and_then(|d| d.first())
                        .map_or('E', |&c| c as char)
                }
                2 => lat = degrees(&t.rationals(kind, count, f)),
                4 => lon = degrees(&t.rationals(kind, count, f)),
                _ => {}
            }
        }
        if let (Some(lat), Some(lon)) = (lat, lon) {
            let lat = if lat_ref == 'S' { -lat } else { lat };
            let lon = if lon_ref == 'W' { -lon } else { lon };
            // A picture that says where it was taken is the one thing in EXIF people most
            // often do not know they are sending, so it gets a row of its own.
            rows.push(field("location", format!("{lat:.6}, {lon:.6}")));
        }
    }
    rows
}

fn degrees(parts: &[f64]) -> Option<f64> {
    match parts {
        [d, m, s, ..] => Some(d + m / 60.0 + s / 3600.0),
        _ => None,
    }
}

// ── PNG and GIF ─────────────────────────────────────────────────────────────────────────────

fn png(src: &mut dyn Bytes, len: u64) -> Structure {
    let mut rows = vec![Row::group("PNG")];
    let mut tags = Vec::new();
    let mut texts = Vec::new();
    let mut c2pa: Option<Vec<u8>> = None;
    let mut summary = "PNG".to_string();
    let mut at = 8u64;
    let mut truncated = false;
    for _ in 0..4096 {
        if at + 8 > len {
            break;
        }
        let head = src.at(at, 8);
        let (Some(size), Some(kind)) = (be32(&head, 0), head.get(4..8)) else {
            truncated = true;
            break;
        };
        let kind: [u8; 4] = kind.try_into().unwrap_or([0; 4]);
        let size = size as u64;
        if at + 12 + size > len {
            truncated = true;
            break;
        }
        let small = || size <= 64 * 1024;
        match &kind {
            b"IHDR" => {
                let b = src.at(at + 8, 13);
                let (w, h) = (be32(&b, 0).unwrap_or(0), be32(&b, 4).unwrap_or(0));
                let colour = match b.get(9) {
                    Some(0) => "greyscale",
                    Some(2) => "RGB",
                    Some(3) => "palette",
                    Some(4) => "greyscale with alpha",
                    Some(6) => "RGBA",
                    _ => "unknown colour type",
                };
                let depth = b.get(8).copied().unwrap_or(0);
                summary = format!("PNG · {w} × {h}");
                rows.push(field("size", format!("{w} × {h}")));
                rows.push(field("colour", format!("{colour}, {depth} bits")));
                if b.get(12) == Some(&1) {
                    rows.push(field("interlaced", "Adam7"));
                }
            }
            b"pHYs" => {
                let b = src.at(at + 8, 9);
                if b.get(8) == Some(&1) {
                    let x = be32(&b, 0).unwrap_or(0) as f64 * 0.0254;
                    rows.push(field("density", format!("{x:.0} dpi")));
                }
            }
            b"tIME" => {
                let b = src.at(at + 8, 7);
                if b.len() == 7 {
                    rows.push(field(
                        "modified",
                        format!(
                            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
                            be16(&b, 0).unwrap_or(0),
                            b[2],
                            b[3],
                            b[4],
                            b[5],
                            b[6]
                        ),
                    ));
                }
            }
            // The text chunks, compressed or not, up to the size of a workflow: an image
            // model's settings are kept here, and a graph can run to hundreds of kilobytes.
            b"tEXt" | b"zTXt" | b"iTXt" if size as usize <= crate::aigen::TEXT_LIMIT => {
                let b = src.at(at + 8, size as usize);
                if let Some((key, value)) = crate::aigen::chunk_text(&kind, &b) {
                    let shown = value.trim();
                    if key == "XML:com.adobe.xmp" {
                        // XMP is a page of XML: it is there, and that is what to say.
                        tags.push(field("XMP", format!("{} bytes", value.len())));
                    } else if !shown.is_empty() {
                        let line = if shown.len() > 200 || shown.contains('\n') {
                            let head: String = shown
                                .split_whitespace()
                                .collect::<Vec<_>>()
                                .join(" ")
                                .chars()
                                .take(120)
                                .collect();
                            format!("{head}… ({} bytes)", shown.len())
                        } else {
                            shown.to_string()
                        };
                        tags.push(field(key.clone(), line));
                    }
                    texts.push(crate::aigen::Text { key, value, at });
                }
            }
            // C2PA's manifest store.
            b"caBX" if size <= 16 * 1024 * 1024 => {
                c2pa = Some(src.at(at + 8, size as usize));
            }
            b"eXIf" if small() => {
                let b = src.at(at + 8, size as usize);
                let e = exif_rows(&b);
                if !e.is_empty() {
                    tags.push(Row::group("EXIF"));
                    tags.extend(e);
                }
            }
            b"acTL" => {
                let b = src.at(at + 8, 8);
                rows.push(field(
                    "animated",
                    format!("{} frames", be32(&b, 0).unwrap_or(0)),
                ));
            }
            b"IEND" => break,
            _ => {}
        }
        at += 12 + size;
    }
    if !tags.is_empty() {
        if !matches!(tags.first(), Some(r) if r.label == "EXIF") {
            rows.push(Row::group("Text"));
        }
        rows.extend(tags);
    }
    rows.extend(crate::aigen::rows(&texts, c2pa.as_deref()));
    structure("PNG", summary, rows, truncated)
}

/// A WebP: the size from its first image chunk, animation, the EXIF and XMP it carries, and
/// what an image model wrote into the EXIF.
fn webp(src: &mut dyn Bytes, len: u64) -> Structure {
    let mut rows = vec![Row::group("WebP")];
    let mut summary = "WebP".to_string();
    let mut texts = Vec::new();
    let mut exif = Vec::new();
    let mut c2pa: Option<Vec<u8>> = None;
    let mut at = 12u64;
    let mut truncated = false;
    let mut sized = false;
    for _ in 0..4096 {
        if at + 8 > len {
            break;
        }
        let head = src.at(at, 8);
        let (Some(kind), Some(size)) = (head.get(..4), le32(&head, 4)) else {
            truncated = true;
            break;
        };
        let kind: [u8; 4] = kind.try_into().unwrap_or([0; 4]);
        let size = size as u64;
        if at + 8 + size > len {
            truncated = true;
            break;
        }
        let body_at = at + 8;
        let mut say_size = |rows: &mut Vec<Row>, w: u32, h: u32, how: &str| {
            if !sized {
                sized = true;
                summary = format!("WebP · {w} × {h}");
                rows.push(field("size", format!("{w} × {h}")));
                rows.push(field("encoding", how.to_string()));
            }
        };
        match &kind {
            b"VP8X" => {
                let b = src.at(body_at, 10);
                if b.len() == 10 {
                    let w = u32::from_le_bytes([b[4], b[5], b[6], 0]) + 1;
                    let h = u32::from_le_bytes([b[7], b[8], b[9], 0]) + 1;
                    let how = if b[0] & 0x02 != 0 {
                        "extended, animated"
                    } else {
                        "extended"
                    };
                    say_size(&mut rows, w, h, how);
                }
            }
            b"VP8 " => {
                let b = src.at(body_at, 10);
                if b.len() == 10 && b[3..6] == [0x9D, 0x01, 0x2A] {
                    let w = u32::from(u16::from_le_bytes([b[6], b[7]]) & 0x3FFF);
                    let h = u32::from(u16::from_le_bytes([b[8], b[9]]) & 0x3FFF);
                    say_size(&mut rows, w, h, "lossy");
                }
            }
            b"VP8L" => {
                let b = src.at(body_at, 5);
                if b.len() == 5 && b[0] == 0x2F {
                    let bits = u32::from_le_bytes([b[1], b[2], b[3], b[4]]);
                    say_size(
                        &mut rows,
                        (bits & 0x3FFF) + 1,
                        ((bits >> 14) & 0x3FFF) + 1,
                        "lossless",
                    );
                }
            }
            b"ANIM" => rows.push(field("animated", "yes")),
            b"EXIF" if size <= 16 * 1024 * 1024 => {
                let mut b = src.at(body_at, size as usize);
                let mut base = body_at;
                // Some writers put the JPEG marker's `Exif\0\0` in front; the spec says not to.
                if b.starts_with(b"Exif\0\0") {
                    b.drain(..6);
                    base += 6;
                }
                exif = exif_rows(&b);
                texts = crate::aigen::exif_texts(&b, base);
            }
            b"XMP " => rows.push(field("XMP", format!("{size} bytes"))),
            b"C2PA" if size <= 16 * 1024 * 1024 => c2pa = Some(src.at(body_at, size as usize)),
            _ => {}
        }
        // Chunks are padded to an even length.
        at = body_at + size + (size & 1);
    }
    // The camera fields of a WebP from ComfyUI hold its graphs, not a camera.
    exif.retain(|r| !r.value.starts_with("prompt:") && !r.value.starts_with("workflow:"));
    if !exif.is_empty() {
        rows.push(Row::group("EXIF"));
        rows.extend(exif);
    }
    rows.extend(crate::aigen::rows(&texts, c2pa.as_deref()));
    structure("WebP", summary, rows, truncated)
}

fn gif(head: &[u8]) -> Structure {
    let w = u16::from_le_bytes([
        head.get(6).copied().unwrap_or(0),
        head.get(7).copied().unwrap_or(0),
    ]);
    let h = u16::from_le_bytes([
        head.get(8).copied().unwrap_or(0),
        head.get(9).copied().unwrap_or(0),
    ]);
    let rows = vec![
        Row::group("GIF"),
        field("version", String::from_utf8_lossy(&head[3..6]).into_owned()),
        field("size", format!("{w} × {h}")),
    ];
    structure("GIF", format!("GIF · {w} × {h}"), rows, false)
}

// ── ID3 ─────────────────────────────────────────────────────────────────────────────────────

fn syncsafe(b: &[u8], at: usize) -> Option<u32> {
    let s = b.get(at..at + 4)?;
    Some(
        (u32::from(s[0] & 0x7F) << 21)
            | (u32::from(s[1] & 0x7F) << 14)
            | (u32::from(s[2] & 0x7F) << 7)
            | u32::from(s[3] & 0x7F),
    )
}

/// An ID3v2 text value: the encoding byte, then the text in it.
fn id3_text(b: &[u8]) -> String {
    let Some((&enc, rest)) = b.split_first() else {
        return String::new();
    };
    let s = match enc {
        1 | 2 => {
            let (body, big) = match rest {
                [0xFE, 0xFF, tail @ ..] => (tail, true),
                [0xFF, 0xFE, tail @ ..] => (tail, false),
                _ => (rest, enc == 2),
            };
            let units: Vec<u16> = body
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| {
                    if big {
                        u16::from_be_bytes([c[0], c[1]])
                    } else {
                        u16::from_le_bytes([c[0], c[1]])
                    }
                })
                .take_while(|&u| u != 0)
                .collect();
            String::from_utf16_lossy(&units)
        }
        3 => text(rest),
        _ => rest
            .iter()
            .take_while(|&&c| c != 0)
            .map(|&c| c as char)
            .collect(),
    };
    s.trim().to_string()
}

fn id3_label(id: &str) -> Option<&'static str> {
    Some(match id {
        "TIT2" | "TT2" => "title",
        "TPE1" | "TP1" => "artist",
        "TPE2" | "TP2" => "album artist",
        "TALB" | "TAL" => "album",
        "TYER" | "TYE" | "TDRC" => "year",
        "TRCK" | "TRK" => "track",
        "TPOS" | "TPA" => "disc",
        "TCON" | "TCO" => "genre",
        "TCOM" | "TCM" => "composer",
        "TENC" | "TEN" => "encoded by",
        "TSSE" | "TSS" => "encoder",
        "TBPM" | "TBP" => "BPM",
        "TCOP" | "TCR" => "copyright",
        _ => return None,
    })
}

fn id3(b: &[u8], len: u64) -> Structure {
    let major = b.get(3).copied().unwrap_or(0);
    let flags = b.get(5).copied().unwrap_or(0);
    let size = syncsafe(b, 6).unwrap_or(0) as usize;
    let mut rows = vec![
        Row::group("ID3"),
        field("version", format!("ID3v2.{major}")),
        field("tag size", format!("{size} bytes")),
    ];
    let mut tags = Vec::new();
    let end = (10 + size).min(b.len());
    let mut at = 10usize;
    // An extended header is skipped by the size it states.
    if flags & 0x40 != 0 {
        let ext = if major == 4 {
            syncsafe(b, at).unwrap_or(0) as usize
        } else {
            be32(b, at).unwrap_or(0) as usize + 4
        };
        at += ext;
    }
    let short = major == 2;
    while at + if short { 6 } else { 10 } <= end && tags.len() < 64 {
        let (id, fsize, header) = if short {
            let id = String::from_utf8_lossy(&b[at..at + 3]).into_owned();
            let s = (usize::from(b[at + 3]) << 16)
                | (usize::from(b[at + 4]) << 8)
                | usize::from(b[at + 5]);
            (id, s, 6)
        } else {
            let id = String::from_utf8_lossy(&b[at..at + 4]).into_owned();
            let s = if major == 4 {
                syncsafe(b, at + 4).unwrap_or(0)
            } else {
                be32(b, at + 4).unwrap_or(0)
            } as usize;
            (id, s, 10)
        };
        // Padding: the rest of the tag is zeros.
        if id.starts_with('\0') || fsize == 0 {
            break;
        }
        let Some(body) = b.get(at + header..at + header + fsize) else {
            break;
        };
        if let Some(label) = id3_label(&id) {
            let v = id3_text(body);
            if !v.is_empty() {
                tags.push(field(label, v));
            }
        } else if id == "COMM" || id == "COM" {
            // encoding, a three-letter language, a description, then the comment.
            if body.len() > 4 {
                let enc = body[0];
                let rest = &body[4..];
                let wide = enc == 1 || enc == 2;
                let skip = if wide {
                    rest.as_chunks::<2>()
                        .0
                        .iter()
                        .position(|c| *c == [0, 0])
                        .map(|p| p * 2 + 2)
                } else {
                    rest.iter().position(|&c| c == 0).map(|p| p + 1)
                };
                if let Some(skip) = skip {
                    let mut v = vec![enc];
                    v.extend(&rest[skip..]);
                    let c = id3_text(&v);
                    if !c.is_empty() {
                        tags.push(field("comment", c));
                    }
                }
            }
        } else if id == "APIC" || id == "PIC" {
            tags.push(field("cover", format!("a picture, {fsize} bytes")));
        }
        at += header + fsize;
    }
    if !tags.is_empty() {
        rows.push(Row::group("Tags"));
        rows.extend(tags);
    }
    let title = rows
        .iter()
        .find(|r| r.label == "title")
        .map(|r| format!(" · {}", r.value))
        .unwrap_or_default();
    let truncated = 10 + size > b.len() && (b.len() as u64) < len;
    structure("MP3", format!("ID3v2.{major}{title}"), rows, truncated)
}

fn id3v1(t: &[u8]) -> Structure {
    let get = |from: usize, n: usize| text(&t[from..from + n]);
    let mut rows = vec![Row::group("ID3v1")];
    for (label, v) in [
        ("title", get(3, 30)),
        ("artist", get(33, 30)),
        ("album", get(63, 30)),
        ("year", get(93, 4)),
    ] {
        if !v.is_empty() {
            rows.push(field(label, v));
        }
    }
    if t[125] == 0 && t[126] != 0 {
        rows.push(field("track", t[126].to_string()));
    }
    structure("MP3", "MP3 · ID3v1".into(), rows, false)
}

// ── FLAC and Ogg ────────────────────────────────────────────────────────────────────────────

/// A Vorbis comment block — FLAC's and Ogg's — as rows: the vendor, then every `KEY=value`.
fn vorbis_comments(b: &[u8]) -> Vec<Row> {
    let mut rows = Vec::new();
    let Some(vendor_len) = le32(b, 0).map(|n| n as usize) else {
        return rows;
    };
    let Some(vendor) = b.get(4..4 + vendor_len) else {
        return rows;
    };
    let vendor = String::from_utf8_lossy(vendor).trim().to_string();
    if !vendor.is_empty() {
        rows.push(field("encoder", vendor));
    }
    let mut at = 4 + vendor_len;
    let count = le32(b, at).unwrap_or(0).min(256);
    at += 4;
    for _ in 0..count {
        let Some(n) = le32(b, at).map(|n| n as usize) else {
            break;
        };
        let Some(c) = b.get(at + 4..at + 4 + n) else {
            break;
        };
        let c = String::from_utf8_lossy(c);
        if let Some((k, v)) = c.split_once('=') {
            // Cover art in a comment is base64 of a whole picture: say it is there.
            if k.eq_ignore_ascii_case("METADATA_BLOCK_PICTURE") {
                rows.push(field(
                    "cover",
                    format!("a picture, {} bytes of base64", v.len()),
                ));
            } else {
                rows.push(field(k.to_ascii_lowercase(), v.trim().to_string()));
            }
        }
        at += 4 + n;
    }
    rows
}

fn flac(b: &[u8]) -> Structure {
    let mut rows = vec![Row::group("FLAC")];
    let mut tags = Vec::new();
    let mut summary = "FLAC".to_string();
    let mut at = 4usize;
    let mut truncated = false;
    for _ in 0..128 {
        let Some(&h) = b.get(at) else {
            truncated = true;
            break;
        };
        let size = (usize::from(*b.get(at + 1).unwrap_or(&0)) << 16)
            | (usize::from(*b.get(at + 2).unwrap_or(&0)) << 8)
            | usize::from(*b.get(at + 3).unwrap_or(&0));
        let Some(body) = b.get(at + 4..at + 4 + size) else {
            truncated = true;
            break;
        };
        match h & 0x7F {
            0 if body.len() >= 18 => {
                let rate = (u32::from(body[10]) << 12)
                    | (u32::from(body[11]) << 4)
                    | (u32::from(body[12]) >> 4);
                let channels = ((body[12] >> 1) & 0x07) + 1;
                let bits = (((body[12] & 1) << 4) | (body[13] >> 4)) + 1;
                let samples = (u64::from(body[13] & 0x0F) << 32)
                    | u64::from(u32::from_be_bytes([body[14], body[15], body[16], body[17]]));
                rows.push(field(
                    "stream",
                    format!("{rate} Hz, {channels} channels, {bits} bits"),
                ));
                if rate > 0 && samples > 0 {
                    let secs = samples / u64::from(rate);
                    let length = format!("{}:{:02}", secs / 60, secs % 60);
                    summary = format!("FLAC · {length}");
                    rows.push(field("length", length));
                }
            }
            4 => tags.extend(vorbis_comments(body)),
            6 => tags.push(field("cover", format!("a picture, {size} bytes"))),
            _ => {}
        }
        at += 4 + size;
        if h & 0x80 != 0 {
            break;
        }
    }
    if !tags.is_empty() {
        rows.push(Row::group("Tags"));
        rows.extend(tags);
    }
    structure("FLAC", summary, rows, truncated)
}

/// The packets of the first pages of an Ogg stream, joined: enough for the identification
/// and the comment header, which is all the tags are.
fn ogg_stream(b: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    let mut at = 0usize;
    for _ in 0..64 {
        if b.get(at..at + 4) != Some(b"OggS") {
            break;
        }
        let Some(&segments) = b.get(at + 26) else {
            break;
        };
        let Some(table) = b.get(at + 27..at + 27 + segments as usize) else {
            break;
        };
        let body: usize = table.iter().map(|&s| s as usize).sum();
        let from = at + 27 + segments as usize;
        let Some(data) = b.get(from..from + body) else {
            out.extend(&b[from.min(b.len())..]);
            break;
        };
        out.extend(data);
        at = from + body;
    }
    out
}

fn ogg(b: &[u8]) -> Option<Structure> {
    let s = ogg_stream(b);
    let mut rows = Vec::new();
    let format;
    let mut summary;
    if let Some(p) = find(&s, b"\x01vorbis") {
        format = "Ogg Vorbis";
        let id = &s[p + 7..];
        let channels = id.get(4).copied().unwrap_or(0);
        let rate = le32(id, 5).unwrap_or(0);
        summary = "Ogg Vorbis".to_string();
        rows.push(Row::group("Vorbis"));
        rows.push(field("stream", format!("{rate} Hz, {channels} channels")));
        if let Some(c) = find(&s, b"\x03vorbis") {
            let tags = vorbis_comments(&s[c + 7..]);
            if !tags.is_empty() {
                rows.push(Row::group("Tags"));
                rows.extend(tags);
            }
        }
    } else {
        let p = find(&s, b"OpusHead")?;
        format = "Ogg Opus";
        let id = &s[p + 8..];
        let channels = id.get(1).copied().unwrap_or(0);
        let rate = le32(id, 4).unwrap_or(0);
        summary = "Ogg Opus".to_string();
        rows.push(Row::group("Opus"));
        rows.push(field(
            "stream",
            format!("{channels} channels, recorded at {rate} Hz"),
        ));
        if let Some(c) = find(&s, b"OpusTags") {
            let tags = vorbis_comments(&s[c + 8..]);
            if !tags.is_empty() {
                rows.push(Row::group("Tags"));
                rows.extend(tags);
            }
        }
    }
    if let Some(t) = rows.iter().find(|r| r.label == "title") {
        summary = format!("{summary} · {}", t.value);
    }
    Some(structure(format, summary, rows, false))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get(s: &Structure, label: &str) -> String {
        s.rows
            .iter()
            .find(|r| r.label == label)
            .map(|r| r.value.clone())
            .unwrap_or_default()
    }

    #[test]
    fn a_jpeg_says_its_size_its_camera_and_where_it_was_taken() {
        // EXIF, little-endian: IFD0 with Make, Model and the two IFD pointers.
        let mut t = b"II\x2A\x00\x08\x00\x00\x00".to_vec();
        let entry = |tag: u16, kind: u16, count: u32, value: u32| {
            let mut e = tag.to_le_bytes().to_vec();
            e.extend(kind.to_le_bytes());
            e.extend(count.to_le_bytes());
            e.extend(value.to_le_bytes());
            e
        };
        // IFD0 at 8: 4 entries → ends at 8 + 2 + 48 + 4 = 62; strings from 62.
        t.extend(4u16.to_le_bytes());
        t.extend(entry(0x010F, 2, 5, 62)); // "Acme\0"
        t.extend(entry(0x0110, 2, 3, u32::from_le_bytes(*b"X1\0\0")));
        t.extend(entry(0x8769, 4, 1, 72));
        t.extend(entry(0x8825, 4, 1, 120));
        t.extend(0u32.to_le_bytes());
        t.extend(b"Acme\0");
        t.resize(72, 0);
        // Exif IFD at 72: ISO 200, then the next-IFD word: ends at 72+2+12+4 = 90.
        t.extend(1u16.to_le_bytes());
        t.extend(entry(0x8827, 3, 1, 200));
        t.extend(0u32.to_le_bytes());
        t.resize(120, 0);
        // GPS IFD at 120: N, 45/1 7/1 0/1, E, 7/1 41/1 0/1 — rationals from 120+2+48+4=174.
        t.extend(4u16.to_le_bytes());
        t.extend(entry(1, 2, 2, u32::from_le_bytes(*b"N\0\0\0")));
        t.extend(entry(2, 5, 3, 174));
        t.extend(entry(3, 2, 2, u32::from_le_bytes(*b"E\0\0\0")));
        t.extend(entry(4, 5, 3, 198));
        t.extend(0u32.to_le_bytes());
        for (n, d) in [(45u32, 1u32), (6, 1), (0, 1), (7, 1), (40, 1), (30, 1)] {
            t.extend(n.to_le_bytes());
            t.extend(d.to_le_bytes());
        }
        let mut j = vec![0xFF, 0xD8, 0xFF, 0xE1];
        let app1_len = (2 + 6 + t.len()) as u16;
        j.extend(app1_len.to_be_bytes());
        j.extend(b"Exif\0\0");
        j.extend(&t);
        j.extend([0xFF, 0xC0, 0x00, 0x11, 8, 0x01, 0xE0, 0x02, 0x80, 3]);
        j.extend([0u8; 9]);
        j.extend([0xFF, 0xDA, 0x00, 0x02]);
        let s = read(&mut &j[..], &j, j.len() as u64).expect("a JPEG");
        assert_eq!(s.summary, "JPEG · 640 × 480");
        assert_eq!(get(&s, "camera maker"), "Acme");
        assert_eq!(get(&s, "camera"), "X1");
        assert_eq!(get(&s, "ISO"), "200");
        assert_eq!(get(&s, "location"), "45.100000, 7.675000");
    }

    #[test]
    fn a_png_says_its_size_and_its_text() {
        fn chunk(kind: &[u8], body: &[u8]) -> Vec<u8> {
            let mut c = (body.len() as u32).to_be_bytes().to_vec();
            c.extend(kind);
            c.extend(body);
            c.extend([0, 0, 0, 0]);
            c
        }
        let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = 300u32.to_be_bytes().to_vec();
        ihdr.extend(200u32.to_be_bytes());
        ihdr.extend([8, 6, 0, 0, 0]);
        p.extend(chunk(b"IHDR", &ihdr));
        p.extend(chunk(b"tEXt", b"Software\0Shotwell"));
        p.extend(chunk(b"iTXt", b"Title\0\0\0it\0Titolo\0Il lago"));
        p.extend(chunk(b"IEND", b""));
        let s = read(&mut &p[..], &p, p.len() as u64).expect("a PNG");
        assert_eq!(s.summary, "PNG · 300 × 200");
        assert_eq!(get(&s, "colour"), "RGBA, 8 bits");
        assert_eq!(get(&s, "Software"), "Shotwell");
        assert_eq!(get(&s, "Title"), "Il lago");
    }

    #[test]
    fn an_mp3_says_its_title_and_artist() {
        let frame = |id: &[u8], body: &[u8]| {
            let mut f = id.to_vec();
            f.extend((body.len() as u32).to_be_bytes());
            f.extend([0, 0]);
            f.extend(body);
            f
        };
        let mut frames = frame(b"TIT2", b"\x03Volare");
        frames.extend(frame(b"TPE1", b"\x01\xFF\xFEM\0o\0d\0u\0g\0n\0o\0"));
        frames.extend(frame(b"COMM", b"\x03itanel blu\0dipinto di blu"));
        frames.extend([0u8; 16]);
        let mut m = b"ID3\x03\x00\x00".to_vec();
        let n = frames.len() as u32;
        m.extend([
            (n >> 21) as u8 & 0x7F,
            (n >> 14) as u8 & 0x7F,
            (n >> 7) as u8 & 0x7F,
            n as u8 & 0x7F,
        ]);
        m.extend(frames);
        let s = read(&mut &m[..], &m, m.len() as u64).expect("an MP3");
        assert_eq!(s.summary, "ID3v2.3 · Volare");
        assert_eq!(get(&s, "artist"), "Modugno");
        assert_eq!(get(&s, "comment"), "dipinto di blu");
    }

    #[test]
    fn a_flac_says_its_stream_and_its_comments() {
        let mut f = b"fLaC".to_vec();
        // STREAMINFO: 44100 Hz, 2 channels, 16 bits, 441000 samples (10 s).
        let mut si = vec![0u8; 34];
        let rate = 44100u32;
        si[10] = (rate >> 12) as u8;
        si[11] = (rate >> 4) as u8;
        si[12] = ((rate & 0xF) as u8) << 4 | (1 << 1);
        si[13] = (15 << 4) as u8;
        si[14..18].copy_from_slice(&441_000u32.to_be_bytes());
        f.extend([0x00, 0, 0, 34]);
        f.extend(&si);
        let mut vc = 4u32.to_le_bytes().to_vec();
        vc.extend(b"test");
        vc.extend(1u32.to_le_bytes());
        let c = b"TITLE=Nel blu";
        vc.extend((c.len() as u32).to_le_bytes());
        vc.extend(c);
        f.extend([0x84, 0, 0, vc.len() as u8]);
        f.extend(&vc);
        let s = read(&mut &f[..], &f, f.len() as u64).expect("a FLAC");
        assert_eq!(get(&s, "stream"), "44100 Hz, 2 channels, 16 bits");
        assert_eq!(s.summary, "FLAC · 0:10");
        assert_eq!(get(&s, "title"), "Nel blu");
        assert_eq!(get(&s, "encoder"), "test");
    }

    #[test]
    fn nothing_is_read_out_of_garbage() {
        for junk in [
            &[0xFF, 0xD8, 0xFF, 0xE1, 0xFF, 0xFF][..],
            b"\x89PNG\r\n\x1a\n\xFF\xFF\xFF\xFFIHDR",
            b"ID3\x04\x00\x40\x7F\x7F\x7F\x7F",
            b"fLaC\x7F\xFF\xFF\xFF",
            b"OggS\0\0\0\0",
        ] {
            let _ = read(&mut &junk[..], junk, junk.len() as u64);
        }
    }
}
