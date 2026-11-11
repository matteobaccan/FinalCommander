//! Taking a picture's metadata out without touching the picture (SPEC 05 §1.8, 0.40).
//!
//! PNG, WebP and JPEG keep what they say about themselves in blocks of their own beside the
//! image data, so the image data is copied byte for byte and only those blocks are left out
//! or rewritten — nothing is decoded, nothing is encoded again, and the picture is the same
//! picture. Three ways to do it:
//!
//! - [`Strip::All`] — every text, EXIF, XMP, IPTC, comment and C2PA block. What stays is what
//!   changes how the picture looks: the colour profile, gamma, transparency, density, the
//!   animation.
//! - [`Strip::Ai`] — only what an image model wrote: ComfyUI's graph and workflow,
//!   A1111's `parameters`, InvokeAI's, Fooocus's and SwarmUI's settings, NovelAI's, and the
//!   same things kept in EXIF fields. A camera's EXIF and everything else stays.
//! - [`Strip::KeepCamera`] — everything but the camera's EXIF (maker, model, lens, exposure,
//!   date) and the author and copyright; the GPS position, serial numbers, the maker's
//!   private notes and the thumbnail go with the rest.
//!
//! A C2PA manifest signs the bytes it was made for, so any change makes it say nothing true;
//! only [`Strip::Ai`] leaves one in, and then only because that is what was asked.

/// Which metadata to take out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strip {
    All,
    Ai,
    KeepCamera,
}

/// The picture without what was taken out, and the names of what went.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stripped {
    pub bytes: Vec<u8>,
    pub removed: Vec<String>,
}

/// Take the metadata `how` says out of a PNG, a WebP or a JPEG.
pub fn strip(b: &[u8], how: Strip) -> Result<Stripped, String> {
    if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        png(b, how)
    } else if b.starts_with(b"RIFF") && b.get(8..12) == Some(b"WEBP") {
        webp(b, how)
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        jpeg(b, how)
    } else {
        Err("not a PNG, a WebP or a JPEG".into())
    }
}

/// The keys of PNG text an image model writes.
const AI_KEYS: &[&str] = &[
    "prompt",
    "workflow",
    "parameters",
    "invokeai_metadata",
    "invokeai_graph",
    "sd-metadata",
    "dream",
    "generation_data",
    "fooocus_scheme",
];

/// The text keys that say who made a picture and who owns it, kept by `KeepCamera`.
const OWNER_KEYS: &[&str] = &["Author", "Copyright", "Artist"];

fn looks_ai(value: &str) -> bool {
    let v = value.trim_start();
    v.starts_with("prompt:")
        || v.starts_with("workflow:")
        || v.contains("Steps:") && v.contains("Seed:")
        || v.starts_with('{') && (v.contains("\"class_type\"") || v.contains("sui_image_params"))
}

// ── PNG ──────────────────────────────────────────────────────────────────────────────────────

fn be32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (n, slot) in table.iter_mut().enumerate() {
        let mut c = n as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &byte in data {
        c = table[((c ^ byte as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn png_chunk(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = (body.len() as u32).to_be_bytes().to_vec();
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    let mut crc_in = kind.to_vec();
    crc_in.extend_from_slice(body);
    out.extend(crc32(&crc_in).to_be_bytes());
    out
}

fn png(b: &[u8], how: Strip) -> Result<Stripped, String> {
    let mut out = b[..8].to_vec();
    let mut removed = Vec::new();
    let mut at = 8;
    // NovelAI's settings are in keys anyone might use; they are its own only when it signed
    // the picture as NovelAI.
    let mut novelai = false;
    let mut chunks: Vec<(&[u8], &[u8], &[u8])> = Vec::new();
    while at + 12 <= b.len() {
        let size = be32(b, at).ok_or("a damaged chunk")? as usize;
        let end = at + 12 + size;
        let whole = b.get(at..end).ok_or("a chunk that runs past the end")?;
        let kind = &whole[4..8];
        let body = &whole[8..8 + size];
        if matches!(kind, b"tEXt" | b"iTXt" | b"zTXt") {
            if let Some((k, v)) = crate::aigen::chunk_text(kind, body) {
                novelai |= k == "Software" && v.contains("NovelAI");
            }
        }
        chunks.push((kind, body, whole));
        at = end;
        if kind == b"IEND" {
            break;
        }
    }
    for (kind, body, whole) in chunks {
        let go = match kind {
            b"tEXt" | b"iTXt" | b"zTXt" => {
                let (key, value) = crate::aigen::chunk_text(kind, body).unwrap_or_default();
                let ai = AI_KEYS.contains(&key.as_str())
                    || novelai
                        && matches!(
                            key.as_str(),
                            "Description" | "Comment" | "Source" | "Software"
                        )
                    || looks_ai(&value);
                let go = match how {
                    Strip::All => true,
                    Strip::Ai => ai,
                    Strip::KeepCamera => ai || !OWNER_KEYS.contains(&key.as_str()),
                };
                if go {
                    removed.push(format!("{} {key}", String::from_utf8_lossy(kind)));
                }
                go
            }
            b"eXIf" => match how {
                Strip::All => {
                    removed.push("eXIf".into());
                    true
                }
                Strip::Ai | Strip::KeepCamera => {
                    let (rebuilt, gone) = exif_filter(body, how);
                    if gone.is_empty() {
                        false
                    } else {
                        removed.extend(gone.into_iter().map(|g| format!("EXIF {g}")));
                        if let Some(e) = rebuilt {
                            out.extend(png_chunk(b"eXIf", &e));
                        }
                        true
                    }
                }
            },
            b"tIME" | b"dSIG" if how != Strip::Ai => {
                removed.push(String::from_utf8_lossy(kind).into_owned());
                true
            }
            b"caBX" if how != Strip::Ai => {
                removed.push("C2PA".into());
                true
            }
            _ => false,
        };
        if !go {
            out.extend_from_slice(whole);
        }
    }
    Ok(Stripped {
        bytes: out,
        removed,
    })
}

// ── WebP ─────────────────────────────────────────────────────────────────────────────────────

fn le32(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn webp(b: &[u8], how: Strip) -> Result<Stripped, String> {
    let mut chunks: Vec<([u8; 4], Vec<u8>)> = Vec::new();
    let mut removed = Vec::new();
    let mut at = 12;
    while at + 8 <= b.len() {
        let kind: [u8; 4] = b[at..at + 4].try_into().map_err(|_| "a damaged chunk")?;
        let size = le32(b, at + 4).ok_or("a damaged chunk")? as usize;
        let body = b
            .get(at + 8..at + 8 + size)
            .ok_or("a chunk that runs past the end")?
            .to_vec();
        at += 8 + size + (size & 1);
        match &kind {
            b"EXIF" => {
                if how == Strip::All {
                    removed.push("EXIF".into());
                    continue;
                }
                let (tiff, prefix) = match body.strip_prefix(b"Exif\0\0") {
                    Some(t) => (t, true),
                    None => (&body[..], false),
                };
                let (rebuilt, gone) = exif_filter(tiff, how);
                if gone.is_empty() {
                    chunks.push((kind, body));
                } else {
                    removed.extend(gone.into_iter().map(|g| format!("EXIF {g}")));
                    if let Some(mut e) = rebuilt {
                        if prefix {
                            e.splice(0..0, b"Exif\0\0".iter().copied());
                        }
                        chunks.push((kind, e));
                    }
                }
            }
            b"XMP " if how != Strip::Ai => removed.push("XMP".into()),
            b"C2PA" if how != Strip::Ai => removed.push("C2PA".into()),
            _ => chunks.push((kind, body)),
        }
    }
    // The extended header says which of the two metadata chunks there are; it has to keep
    // telling the truth.
    let has = |k: &[u8; 4]| chunks.iter().any(|(c, _)| c == k);
    let (exif, xmp) = (has(b"EXIF"), has(b"XMP "));
    if let Some((_, vp8x)) = chunks.iter_mut().find(|(c, _)| c == b"VP8X") {
        if let Some(flags) = vp8x.first_mut() {
            *flags &= !(0x08 | 0x04);
            if exif {
                *flags |= 0x08;
            }
            if xmp {
                *flags |= 0x04;
            }
        }
    }
    let mut body = b"WEBP".to_vec();
    for (kind, data) in &chunks {
        body.extend_from_slice(kind);
        body.extend((data.len() as u32).to_le_bytes());
        body.extend_from_slice(data);
        if data.len() & 1 == 1 {
            body.push(0);
        }
    }
    let mut out = b"RIFF".to_vec();
    out.extend((body.len() as u32).to_le_bytes());
    out.extend(body);
    Ok(Stripped {
        bytes: out,
        removed,
    })
}

// ── JPEG ─────────────────────────────────────────────────────────────────────────────────────

fn jpeg(b: &[u8], how: Strip) -> Result<Stripped, String> {
    let mut out = vec![0xFF, 0xD8];
    let mut removed = Vec::new();
    let mut at = 2;
    loop {
        // Fill bytes between segments.
        while b.get(at) == Some(&0xFF) && b.get(at + 1) == Some(&0xFF) {
            at += 1;
        }
        let (Some(&0xFF), Some(&marker)) = (b.get(at), b.get(at + 1)) else {
            return Err("a damaged JPEG: no marker where one should be".into());
        };
        // The start of the scan: from here to the end is the picture, and it is copied whole.
        if marker == 0xDA || marker == 0xD9 {
            out.extend_from_slice(&b[at..]);
            break;
        }
        let len = u16::from_be_bytes([
            *b.get(at + 2).ok_or("a damaged JPEG")?,
            *b.get(at + 3).ok_or("a damaged JPEG")?,
        ]) as usize;
        let seg = b
            .get(at..at + 2 + len)
            .ok_or("a segment that runs past the end")?;
        let body = &seg[4..];
        let mut keep = true;
        match marker {
            0xE1 if body.starts_with(b"Exif\0\0") => {
                if how == Strip::All {
                    removed.push("EXIF".into());
                    keep = false;
                } else {
                    let (rebuilt, gone) = exif_filter(&body[6..], how);
                    if !gone.is_empty() {
                        removed.extend(gone.into_iter().map(|g| format!("EXIF {g}")));
                        keep = false;
                        if let Some(e) = rebuilt {
                            let mut data = b"Exif\0\0".to_vec();
                            data.extend(e);
                            if data.len() + 2 <= 0xFFFF {
                                out.extend([0xFF, 0xE1]);
                                out.extend(((data.len() + 2) as u16).to_be_bytes());
                                out.extend(data);
                            }
                        }
                    }
                }
            }
            // XMP, and the extended XMP that carries what does not fit.
            0xE1 if how != Strip::Ai => {
                removed.push("XMP".into());
                keep = false;
            }
            // Photoshop's IPTC block.
            0xED if how != Strip::Ai => {
                removed.push("IPTC".into());
                keep = false;
            }
            // JUMBF: C2PA's manifest.
            0xEB if how != Strip::Ai => {
                if !removed.iter().any(|r| r == "C2PA") {
                    removed.push("C2PA".into());
                }
                keep = false;
            }
            0xFE => {
                let text = String::from_utf8_lossy(body);
                if how != Strip::Ai || looks_ai(&text) {
                    removed.push("comment".into());
                    keep = false;
                }
            }
            _ => {}
        }
        if keep {
            out.extend_from_slice(seg);
        }
        at += 2 + len;
    }
    Ok(Stripped {
        bytes: out,
        removed,
    })
}

// ── EXIF, rewritten ──────────────────────────────────────────────────────────────────────────

/// The camera's own tags, kept by [`Strip::KeepCamera`].
const CAMERA_IFD0: &[u16] = &[
    0x010F, 0x0110, 0x0112, 0x011A, 0x011B, 0x0128, 0x0132, 0x013B, 0x8298,
];
const CAMERA_EXIF: &[u16] = &[
    0x829A, 0x829D, 0x8822, 0x8827, 0x8830, 0x8832, 0x9000, 0x9003, 0x9004, 0x9010, 0x9011, 0x9201,
    0x9202, 0x9204, 0x9205, 0x9207, 0x9208, 0x9209, 0x920A, 0xA001, 0xA002, 0xA003, 0xA402, 0xA403,
    0xA404, 0xA405, 0xA406, 0xA432, 0xA433, 0xA434,
];

struct Entry {
    tag: u16,
    kind: u16,
    count: u32,
    data: Vec<u8>,
}

struct Ifd {
    little: bool,
}

impl Ifd {
    fn u16(&self, b: &[u8], at: usize) -> Option<u16> {
        let s: [u8; 2] = b.get(at..at + 2)?.try_into().ok()?;
        Some(if self.little {
            u16::from_le_bytes(s)
        } else {
            u16::from_be_bytes(s)
        })
    }

    fn u32(&self, b: &[u8], at: usize) -> Option<u32> {
        let s: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
        Some(if self.little {
            u32::from_le_bytes(s)
        } else {
            u32::from_be_bytes(s)
        })
    }

    fn entries(&self, b: &[u8], at: usize) -> Vec<Entry> {
        let Some(n) = self.u16(b, at) else {
            return Vec::new();
        };
        (0..n.min(1024) as usize)
            .filter_map(|i| {
                let e = at + 2 + i * 12;
                let tag = self.u16(b, e)?;
                let kind = self.u16(b, e + 2)?;
                let count = self.u32(b, e + 4)?;
                let unit = match kind {
                    1 | 2 | 6 | 7 => 1,
                    3 | 8 => 2,
                    4 | 9 | 11 | 13 => 4,
                    5 | 10 | 12 => 8,
                    _ => return None,
                };
                let size = unit * count as usize;
                let from = if size <= 4 {
                    e + 8
                } else {
                    self.u32(b, e + 8)? as usize
                };
                let data = b.get(from..from.checked_add(size)?)?.to_vec();
                Some(Entry {
                    tag,
                    kind,
                    count,
                    data,
                })
            })
            .collect()
    }
}

/// An EXIF block with only the tags `how` keeps, and the names of those that went. `None` for
/// the block when nothing is left in it; the names empty when nothing had to go, in which
/// case the block is not rewritten at all.
fn exif_filter(b: &[u8], how: Strip) -> (Option<Vec<u8>>, Vec<String>) {
    let little = match b.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return (None, vec!["unreadable".into()]),
    };
    let r = Ifd { little };
    let Some(first) = r.u32(b, 4) else {
        return (None, vec!["unreadable".into()]);
    };
    let ifd0 = r.entries(b, first as usize);
    let sub = |tag: u16| {
        ifd0.iter()
            .find(|e| e.tag == tag)
            .and_then(|e| r.u32(&e.data, 0).map(|a| r.entries(b, a as usize)))
            .unwrap_or_default()
    };
    let exif = sub(0x8769);
    let gps = sub(0x8825);
    let text = |e: &Entry| {
        crate::aigen::exif_value(
            if e.tag == 0x9286 {
                "UserComment"
            } else if e.tag == 0x9C9C {
                "XPComment"
            } else {
                "text"
            },
            &e.data,
        )
    };
    let mut gone = Vec::new();
    let mut keep0 = Vec::new();
    let mut keep_exif = Vec::new();
    let mut keep_gps = Vec::new();
    let next_ifd = r.u32(b, first as usize + 2 + ifd0.len() * 12).unwrap_or(0);
    for e in ifd0 {
        if matches!(e.tag, 0x8769 | 0x8825 | 0xA005) {
            continue;
        }
        let ai = matches!(e.tag, 0x010F | 0x0110 | 0x010E | 0x9C9C) && looks_ai(&text(&e));
        let stays = match how {
            Strip::All => false,
            Strip::Ai => !ai,
            Strip::KeepCamera => !ai && CAMERA_IFD0.contains(&e.tag),
        };
        if stays {
            keep0.push(e);
        } else {
            gone.push(tag_name(e.tag));
        }
    }
    for e in exif {
        let ai = matches!(e.tag, 0x9286) && looks_ai(&text(&e));
        // The maker's notes point into themselves by absolute offsets, which a rewrite moves.
        let stays = match how {
            Strip::All => false,
            Strip::Ai => !ai && e.tag != 0x927C && e.tag != 0xA005,
            Strip::KeepCamera => CAMERA_EXIF.contains(&e.tag),
        };
        if stays {
            keep_exif.push(e);
        } else {
            gone.push(tag_name(e.tag));
        }
    }
    for e in gps {
        if how == Strip::Ai {
            keep_gps.push(e);
        } else {
            gone.push("GPS".into());
        }
    }
    gone.dedup();
    // The thumbnail in IFD1: with anything else gone it goes too, and without it nothing in
    // the rewrite points at it.
    if next_ifd != 0 {
        gone.push("thumbnail".into());
    }
    // Nothing to take out — except the thumbnail `Ai` would keep: leave the block as it is.
    let only_thumbnail = gone.len() == 1 && next_ifd != 0;
    if gone.is_empty() || (how == Strip::Ai && only_thumbnail) {
        return (None, Vec::new());
    }
    if keep0.is_empty() && keep_exif.is_empty() && keep_gps.is_empty() {
        return (None, gone);
    }
    (Some(write_tiff(little, keep0, keep_exif, keep_gps)), gone)
}

fn tag_name(tag: u16) -> String {
    match tag {
        0x010E => "description".into(),
        0x010F => "make".into(),
        0x0110 => "model".into(),
        0x0131 => "software".into(),
        0x9286 => "user comment".into(),
        0x927C => "maker notes".into(),
        0x9C9C => "comment".into(),
        0xA431 => "body serial number".into(),
        0xA435 => "lens serial number".into(),
        _ => format!("tag {tag:#06X}"),
    }
}

/// A TIFF with IFD0, and the Exif and GPS IFDs when they have anything, in the byte order it
/// came in.
fn write_tiff(little: bool, mut ifd0: Vec<Entry>, exif: Vec<Entry>, gps: Vec<Entry>) -> Vec<u8> {
    let w16 = |v: u16| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    let w32 = |v: u32| {
        if little {
            v.to_le_bytes()
        } else {
            v.to_be_bytes()
        }
    };
    // The pointers to the sub-IFDs are entries of IFD0 whose value is filled in below.
    if !exif.is_empty() {
        ifd0.push(Entry {
            tag: 0x8769,
            kind: 4,
            count: 1,
            data: vec![0; 4],
        });
    }
    if !gps.is_empty() {
        ifd0.push(Entry {
            tag: 0x8825,
            kind: 4,
            count: 1,
            data: vec![0; 4],
        });
    }
    let mut ifds = vec![ifd0, exif, gps];
    for ifd in &mut ifds {
        ifd.sort_by_key(|e| e.tag);
    }
    let size = |ifd: &[Entry]| {
        let table = 2 + ifd.len() * 12 + 4;
        let data: usize = ifd
            .iter()
            .filter(|e| e.data.len() > 4)
            .map(|e| e.data.len() + (e.data.len() & 1))
            .sum();
        table + data
    };
    let at0 = 8usize;
    let at_exif = at0 + size(&ifds[0]);
    let at_gps = at_exif
        + if ifds[1].is_empty() {
            0
        } else {
            size(&ifds[1])
        };
    for e in ifds[0].iter_mut() {
        if e.tag == 0x8769 {
            e.data = w32(at_exif as u32).to_vec();
        } else if e.tag == 0x8825 {
            e.data = w32(at_gps as u32).to_vec();
        }
    }
    let mut out = Vec::new();
    out.extend(if little { *b"II" } else { *b"MM" });
    out.extend(w16(42));
    out.extend(w32(at0 as u32));
    for (n, ifd) in ifds.iter().enumerate() {
        if ifd.is_empty() && n > 0 {
            continue;
        }
        let start = out.len();
        let mut data_at = start + 2 + ifd.len() * 12 + 4;
        let mut data = Vec::new();
        out.extend(w16(ifd.len() as u16));
        for e in ifd {
            out.extend(w16(e.tag));
            out.extend(w16(e.kind));
            out.extend(w32(e.count));
            if e.data.len() <= 4 {
                let mut v = e.data.clone();
                v.resize(4, 0);
                out.extend(v);
            } else {
                out.extend(w32(data_at as u32));
                data.extend(&e.data);
                if e.data.len() & 1 == 1 {
                    data.push(0);
                }
                data_at += e.data.len() + (e.data.len() & 1);
            }
        }
        out.extend(w32(0));
        out.extend(data);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png_with(chunks: &[(&[u8; 4], Vec<u8>)]) -> Vec<u8> {
        let mut p = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut ihdr = 2u32.to_be_bytes().to_vec();
        ihdr.extend(2u32.to_be_bytes());
        ihdr.extend([8, 2, 0, 0, 0]);
        p.extend(png_chunk(b"IHDR", &ihdr));
        for (k, b) in chunks {
            p.extend(png_chunk(k, b));
        }
        p.extend(png_chunk(b"IDAT", b"pixels-are-not-touched"));
        p.extend(png_chunk(b"IEND", b""));
        p
    }

    fn text(key: &str, value: &str) -> Vec<u8> {
        let mut t = key.as_bytes().to_vec();
        t.push(0);
        t.extend(value.as_bytes());
        t
    }

    fn keys(png: &[u8]) -> Vec<String> {
        let mut out = Vec::new();
        let mut at = 8;
        while at + 12 <= png.len() {
            let size = be32(png, at).unwrap() as usize;
            let kind = &png[at + 4..at + 8];
            let body = &png[at + 8..at + 8 + size];
            // Every chunk written has a right checksum.
            let mut crc_in = kind.to_vec();
            crc_in.extend(body);
            assert_eq!(be32(png, at + 8 + size).unwrap(), crc32(&crc_in));
            out.push(match crate::aigen::chunk_text(kind, body) {
                Some((k, _)) => k,
                None => String::from_utf8_lossy(kind).into_owned(),
            });
            at += 12 + size;
        }
        out
    }

    #[test]
    fn png_all_ai_and_camera_keep_what_they_say() {
        let p = png_with(&[
            (
                b"tEXt",
                text("prompt", r#"{"1":{"class_type":"KSampler"}}"#),
            ),
            (b"tEXt", text("workflow", "{}")),
            (b"tEXt", text("Copyright", "Matteo")),
            (b"tEXt", text("Title", "lake")),
            (b"tIME", vec![7, 234, 11, 11, 10, 0, 0]),
            (b"gAMA", 45455u32.to_be_bytes().to_vec()),
        ]);
        let all = strip(&p, Strip::All).unwrap();
        assert_eq!(keys(&all.bytes), ["IHDR", "gAMA", "IDAT", "IEND"]);
        let ai = strip(&p, Strip::Ai).unwrap();
        assert_eq!(
            keys(&ai.bytes),
            ["IHDR", "Copyright", "Title", "tIME", "gAMA", "IDAT", "IEND"]
        );
        assert_eq!(ai.removed, ["tEXt prompt", "tEXt workflow"]);
        let camera = strip(&p, Strip::KeepCamera).unwrap();
        assert_eq!(
            keys(&camera.bytes),
            ["IHDR", "Copyright", "gAMA", "IDAT", "IEND"]
        );
        // The picture itself is the same bytes.
        assert!(find(&all.bytes, b"pixels-are-not-touched"));
        // And the result reads as a PNG with no AI section left.
        let s =
            crate::tags::read(&mut &all.bytes[..], &all.bytes[..], all.bytes.len() as u64).unwrap();
        assert!(!s.rows.iter().any(|r| r.label == "AI generation"));
    }

    fn find(hay: &[u8], needle: &[u8]) -> bool {
        hay.windows(needle.len()).any(|w| w == needle)
    }

    /// A little-endian TIFF: IFD0 with Make, Model and a GPS pointer, the GPS IFD with one
    /// entry, and an Exif IFD with the exposure and a user comment.
    fn tiff(model: &str, comment: &str) -> Vec<u8> {
        let mut ifd0 = vec![
            Entry {
                tag: 0x010F,
                kind: 2,
                count: 6,
                data: b"Canon\0".to_vec(),
            },
            Entry {
                tag: 0x0110,
                kind: 2,
                count: model.len() as u32 + 1,
                data: [model.as_bytes(), b"\0"].concat(),
            },
        ];
        ifd0.push(Entry {
            tag: 0x0131,
            kind: 2,
            count: 4,
            data: b"GIM\0".to_vec(),
        });
        let mut uc = b"ASCII\0\0\0".to_vec();
        uc.extend(comment.as_bytes());
        let exif = vec![
            Entry {
                tag: 0x829A,
                kind: 5,
                count: 1,
                data: [1u32.to_le_bytes(), 250u32.to_le_bytes()].concat(),
            },
            Entry {
                tag: 0x9286,
                kind: 7,
                count: uc.len() as u32,
                data: uc,
            },
        ];
        let gps = vec![Entry {
            tag: 0x0002,
            kind: 5,
            count: 3,
            data: vec![1; 24],
        }];
        write_tiff(true, ifd0, exif, gps)
    }

    fn tags_of(t: &[u8]) -> Vec<u16> {
        let r = Ifd { little: true };
        let ifd0 = r.entries(t, r.u32(t, 4).unwrap() as usize);
        let mut out: Vec<u16> = ifd0.iter().map(|e| e.tag).collect();
        for e in &ifd0 {
            if matches!(e.tag, 0x8769 | 0x8825) {
                let sub = r.entries(t, r.u32(&e.data, 0).unwrap() as usize);
                out.extend(sub.iter().map(|e| e.tag));
            }
        }
        out
    }

    #[test]
    fn exif_keeps_the_camera_and_loses_the_place_and_the_prompt() {
        let t = tiff("EOS R5", "a castle\nSteps: 20, Seed: 3");
        let (camera, gone) = exif_filter(&t, Strip::KeepCamera);
        let camera = camera.unwrap();
        assert_eq!(tags_of(&camera), [0x010F, 0x0110, 0x8769, 0x829A]);
        assert!(gone.contains(&"GPS".to_string()), "{gone:?}");
        let (ai, gone) = exif_filter(&t, Strip::Ai);
        let ai = ai.unwrap();
        assert_eq!(gone, ["user comment"]);
        assert_eq!(
            tags_of(&ai),
            [0x010F, 0x0110, 0x0131, 0x8769, 0x8825, 0x829A, 0x0002]
        );
        // A camera's EXIF with nothing of an image model in it is left as it is.
        let plain = tiff("EOS R5", "holiday");
        assert_eq!(exif_filter(&plain, Strip::Ai), (None, Vec::new()));
        // And the rewrite reads back with the same make.
        let texts = crate::aigen::exif_texts(&camera, 0);
        assert_eq!(texts[0].value, "Canon");
    }

    #[test]
    fn a_comfyui_webp_loses_its_graph_and_keeps_its_flags_true() {
        let t = {
            let ifd0 = vec![
                Entry {
                    tag: 0x010F,
                    kind: 2,
                    count: 12,
                    data: b"workflow:{}\0".to_vec(),
                },
                Entry {
                    tag: 0x0110,
                    kind: 2,
                    count: 10,
                    data: b"prompt:{}\0".to_vec(),
                },
            ];
            write_tiff(true, ifd0, Vec::new(), Vec::new())
        };
        let mut body = b"WEBP".to_vec();
        let mut chunk = |k: &[u8; 4], d: &[u8]| {
            body.extend(k);
            body.extend((d.len() as u32).to_le_bytes());
            body.extend(d);
            if d.len() & 1 == 1 {
                body.push(0);
            }
        };
        chunk(b"VP8X", &[0x08 | 0x04, 0, 0, 0, 1, 0, 0, 1, 0, 0]);
        chunk(b"VP8L", b"\x2f\0\0\0\0image");
        chunk(b"EXIF", &t);
        chunk(b"XMP ", b"<x/>");
        let mut w = b"RIFF".to_vec();
        w.extend((body.len() as u32).to_le_bytes());
        w.extend(body);
        let ai = strip(&w, Strip::Ai).unwrap();
        assert_eq!(ai.removed, ["EXIF make", "EXIF model"]);
        let flags = ai.bytes[20];
        assert_eq!(flags & 0x08, 0, "no EXIF left, and the header says so");
        assert_eq!(flags & 0x04, 0x04, "the XMP stays with Ai");
        let all = strip(&w, Strip::All).unwrap();
        assert_eq!(all.bytes[20] & 0x0C, 0);
        assert_eq!(le32(&all.bytes, 4).unwrap() as usize, all.bytes.len() - 8);
        assert!(find(&all.bytes, b"image"));
    }

    #[test]
    fn a_jpeg_keeps_its_scan_and_its_colour_and_loses_the_rest() {
        let mut j = vec![0xFF, 0xD8];
        let seg = |m: u8, body: &[u8]| {
            let mut s = vec![0xFF, m];
            s.extend(((body.len() + 2) as u16).to_be_bytes());
            s.extend(body);
            s
        };
        j.extend(seg(0xE0, b"JFIF\0\x01\x01\0\0\x01\0\x01\0\0"));
        let mut exif = b"Exif\0\0".to_vec();
        exif.extend(tiff("EOS R5", "Steps: 5, Seed: 1"));
        j.extend(seg(0xE1, &exif));
        j.extend(seg(0xE1, b"http://ns.adobe.com/xap/1.0/\0<x/>"));
        j.extend(seg(0xE2, b"ICC_PROFILE\0\x01\x01profile"));
        j.extend(seg(0xEB, b"JP\0\0\0\0\0\x01jumbc2pa"));
        j.extend(seg(0xFE, b"a note"));
        j.extend([0xFF, 0xDA, 0, 2, 1, 2, 3, 0xFF, 0xD9]);
        let all = strip(&j, Strip::All).unwrap();
        assert_eq!(all.removed, ["EXIF", "XMP", "C2PA", "comment"]);
        assert!(find(&all.bytes, b"ICC_PROFILE"), "the colour stays");
        assert!(find(&all.bytes, b"JFIF"));
        assert!(all
            .bytes
            .ends_with(&[0xFF, 0xDA, 0, 2, 1, 2, 3, 0xFF, 0xD9]));
        let ai = strip(&j, Strip::Ai).unwrap();
        assert_eq!(ai.removed, ["EXIF user comment"]);
        assert!(find(&ai.bytes, b"a note") && find(&ai.bytes, b"xap/1.0"));
        let camera = strip(&j, Strip::KeepCamera).unwrap();
        assert!(
            camera.removed.contains(&"EXIF GPS".to_string()),
            "{:?}",
            camera.removed
        );
        assert!(find(&camera.bytes, b"Canon"));
    }

    #[test]
    fn anything_else_is_refused() {
        assert!(strip(b"GIF89a....", Strip::All).is_err());
    }
}
