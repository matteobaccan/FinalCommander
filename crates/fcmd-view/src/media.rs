//! What a media container *is*, and where anything inside it lives —
//! `docs/spec/05-viewer-editor.md` §1.8.
//!
//! A commander opens a video the way it opens an executable: not to play it, but to find out
//! what it is. How long, what resolution, which codec, how many tracks, who wrote it, and
//! where the payload starts. All of that is in the file's own header, and reading it is the
//! same job the structure pane already does for ELF and PE — a tree of records, each with a
//! length, each to be checked against the real end of the file before it is believed.
//!
//! Three families cover what turns up in a folder:
//!
//! - **ISO base media** — MP4, MOV, M4A, M4V, 3GP, HEIF: a tree of *boxes*, each `size` then
//!   `type`, big-endian throughout.
//! - **Matroska** — MKV, WebM, MKA: EBML, a tree of elements whose ids and lengths are
//!   variable-width integers.
//! - **RIFF** — AVI, WAV, WebP: chunks of `id` then `size`, little-endian, the ancestor of
//!   both of the above.
//!
//! Nothing here decodes a single frame: that wants a codec, which is a library this project
//! does not have and a licence it does not want. What it answers is the question somebody has
//! when they press `F11`, which is never "show me the picture" — `F3` is for that — but "what
//! am I looking at".
//!
//! Every length in these formats is a number some other program wrote, so every one of them is
//! clamped to what the file really holds before it is used as an offset or a size. A file that
//! stops in the middle fills the rows it can and says so, like every other reader in this pane.

use std::fmt::Write as _;

use crate::structure::{in_file, size_cell, unix_time, Bytes, Cur, Row, Structure, MAX_ENTRIES};

/// The most of one record we are willing to hold in memory. A `moov` box carries the whole
/// index of a film and can be tens of megabytes on a long one; past this the rows that need it
/// are the ones that go missing, and the file still gets its identity.
const MAX_BODY: usize = 8 << 20;
/// How deep a tree of records we follow. Both formats allow a container inside a container
/// without limit, and a file that nests eight hundred deep is a file built to make a reader
/// recurse until it dies.
const MAX_DEPTH: usize = 8;

/// Seconds between 1904-01-01 and 1970-01-01: the epoch every ISO base media file counts its
/// own dates from, QuickTime's own, which is 66 years and 17 leap days before Unix.
const EPOCH_1904: u64 = 2_082_844_800;
/// Seconds between 1970-01-01 and 2001-01-01, which is where Matroska counts from.
const EPOCH_2001: u64 = 978_307_200;

/// Decode a container, or decide this is not one. `None` is the normal answer: most files are
/// not videos.
pub fn read(src: &mut dyn Bytes, head: &[u8], len: u64) -> Option<Structure> {
    // ISO base media says what it is in its first box, which is `ftyp` in everything written
    // this century. QuickTime predates `ftyp`, so a `.mov` from the nineties opens with
    // whichever box the writer put first, and those are the ones below.
    if head.len() >= 8 {
        let kind = &head[4..8];
        // A box of size zero runs to the end of the file, which only the last box of one may
        // say — so a file opening with four zero bytes is four zero bytes, not a container.
        let sized = head[..4] != *b"\0\0\0\0";
        if kind == b"ftyp" && sized {
            return Some(isobmff(src, len));
        }
        if matches!(
            kind,
            b"moov" | b"mdat" | b"free" | b"skip" | b"wide" | b"pnot" | b"styp"
        ) && sized
        {
            return Some(isobmff(src, len));
        }
    }
    if head.len() >= 4 && head[..4] == [0x1A, 0x45, 0xDF, 0xA3] {
        return Some(matroska(src, len));
    }
    if head.len() >= 12 && head[..4] == *b"RIFF" {
        return Some(riff(src, head, len));
    }
    None
}

// ── ISO base media: MP4, MOV, M4A, 3GP, HEIF ────────────────────────────────────────────────

/// One box as the walk found it: where it is, how long its header was, and how long the whole
/// of it is. `at + header` is where its body starts, `at + size` is the next box.
#[derive(Debug, Clone, Copy)]
struct Bx {
    kind: [u8; 4],
    at: u64,
    header: u64,
    size: u64,
}

impl Bx {
    fn body(&self) -> u64 {
        self.at + self.header
    }
    fn body_len(&self) -> u64 {
        self.size.saturating_sub(self.header)
    }
}

fn isobmff(src: &mut dyn Bytes, len: u64) -> Structure {
    let mut rows = Vec::new();
    let (top, short) = boxes(src, 0, len);

    // `ftyp`: the brands, which is the file's own statement of what standard it follows.
    let mut brand = String::new();
    let mut compatible = String::new();
    if let Some(ftyp) = top.iter().find(|b| b.kind == *b"ftyp") {
        let body = body_of(src, ftyp, len);
        let mut c = Cur::new(&body);
        c.big_endian = true;
        brand = fourcc(c.take(4));
        let minor = c.u32();
        let mut brands = Vec::new();
        while c.pos + 4 <= body.len() && brands.len() < 16 {
            brands.push(fourcc(c.take(4)));
        }
        compatible = brands.join(" ");
        rows.push(Row::group("File type"));
        rows.push(Row::field("major_brand", hex_of(&body, 0, 4), &brand));
        rows.push(Row::field(
            "minor_version",
            format!("{minor:08X}"),
            format!("{minor}"),
        ));
        if !compatible.is_empty() {
            rows.push(Row::field("compatible_brands", "", &compatible));
        }
    } else if let Some(first) = top.first() {
        brand = fourcc(&first.kind);
    }

    // `moov` is the whole of the metadata; read it once and answer everything from it.
    let moov = top
        .iter()
        .find(|b| b.kind == *b"moov")
        .map(|b| (body_of(src, b, len), *b));
    let mut duration = 0.0f64;
    let mut tracks = Vec::new();
    if let Some((body, bx)) = &moov {
        if let Some(mvhd) = find_box(body, b"mvhd") {
            let m = movie_header(mvhd);
            duration = m.seconds();
            rows.push(Row::group("Movie"));
            rows.push(Row::field(
                "timescale",
                format!("{:08X}", m.timescale),
                format!("{} Hz", m.timescale),
            ));
            rows.push(Row::field(
                "duration",
                format!("{:X}", m.duration),
                clock(m.seconds()),
            ));
            rows.push(Row::field(
                "creation_time",
                format!("{:X}", m.created),
                mac_time(m.created),
            ));
            rows.push(Row::field(
                "modification_time",
                format!("{:X}", m.modified),
                mac_time(m.modified),
            ));
        }
        // Every `trak` in the order the file lists them, which is the order their numbers run.
        for (i, trak) in boxes_in(body)
            .into_iter()
            .filter(|(kind, _, _, _)| *kind == *b"trak")
            .enumerate()
        {
            let (_, _, start, end) = trak;
            let track = track_of(&body[start..end], duration);
            rows.push(Row::group(format!(
                "Track {} · {}",
                track.id.unwrap_or(i as u32 + 1),
                track.headline()
            )));
            rows.extend(track.rows(bx.body() + start as u64, len));
            tracks.push(track);
        }
        // The tags an audio player writes: `udta > meta > ilst`, iTunes' own layout, which is
        // what every encoder has written into an `.m4a` for twenty years.
        let tags = find_path(body, &[b"udta", b"meta"])
            .and_then(meta_children)
            .and_then(|meta| find_box(meta, b"ilst").map(ilst))
            .unwrap_or_default();
        if !tags.is_empty() {
            rows.push(Row::group("Tags"));
            for (name, value) in tags {
                rows.push(Row::field(name, "", value));
            }
        }
    }

    // The top-level boxes, which is the map of the file: where the metadata is, where the
    // payload is, and how much of the file each takes.
    rows.push(Row::group("Boxes"));
    for b in &top {
        rows.push(Row::entry(
            format!("  {}", fourcc(&b.kind)),
            format!("{:08X}  {}", b.at, size_cell(b.size)),
            in_file(b.at, len),
        ));
    }
    // One level inside `moov`, because that is where a reader is actually going.
    if let Some((body, bx)) = &moov {
        for (kind, start, _, end) in boxes_in(body) {
            let at = bx.body() + start as u64;
            rows.push(Row::entry(
                format!("    moov/{}", fourcc(&kind)),
                format!("{at:08X}  {}", size_cell((end - start) as u64)),
                in_file(at, len),
            ));
        }
    }

    let fragmented = top.iter().any(|b| b.kind == *b"moof")
        || moov
            .as_ref()
            .is_some_and(|(body, _)| find_box(body, b"mvex").is_some());
    let mut summary = String::new();
    let _ = write!(summary, "{}", container_name(&brand, &compatible));
    if duration > 0.0 {
        let _ = write!(summary, " · {}", clock(duration));
    }
    let shape: Vec<String> = tracks.iter().filter_map(|t| t.brief()).collect();
    if !shape.is_empty() {
        let _ = write!(summary, " · {}", shape.join(", "));
    }
    if tracks.len() > shape.len() {
        let _ = write!(summary, " · {} tracks", tracks.len());
    }
    if fragmented {
        summary.push_str(" · fragmented");
    }

    Structure {
        format: "ISO base media",
        summary,
        rows,
        // Nothing in a container loads at an address: there is no code here to label.
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: short,
    }
}

/// The boxes at one level of the file, read from the file itself: sixteen bytes per box, and
/// never the bodies. A film is three gigabytes of `mdat` and this walk does not read a byte
/// of it.
fn boxes(src: &mut dyn Bytes, from: u64, end: u64) -> (Vec<Bx>, bool) {
    let mut out = Vec::new();
    let mut at = from;
    let mut short = false;
    while at + 8 <= end && out.len() < MAX_ENTRIES {
        let head = src.at(at, 16);
        if head.len() < 8 {
            short = true;
            break;
        }
        let mut c = Cur::new(&head);
        c.big_endian = true;
        let size32 = c.u32();
        let mut kind = [0u8; 4];
        kind.copy_from_slice(c.take(4));
        let (size, header) = match size32 {
            // `1` means the real size is the 64-bit number that follows the type.
            1 => (c.u64(), 16),
            // `0` means "to the end of the file", which is legal for the last box only.
            0 => (end - at, 8),
            n => (u64::from(n), 8),
        };
        // A box shorter than its own header, or one that claims to run past the end, is a
        // corrupt file: take what it says as far as the file goes and stop walking.
        if size < header {
            short = true;
            break;
        }
        let clamped = size.min(end - at);
        short |= clamped < size;
        out.push(Bx {
            kind,
            at,
            header,
            size: clamped,
        });
        at += clamped;
    }
    (out, short)
}

/// A box's body, read once and capped. Past the cap the rows that need it go missing and the
/// file still gets its identity, which is the trade this pane makes everywhere.
fn body_of(src: &mut dyn Bytes, b: &Bx, len: u64) -> Vec<u8> {
    let start = b.body();
    if start >= len {
        return Vec::new();
    }
    let want = b.body_len().min(len - start).min(MAX_BODY as u64) as usize;
    src.at(start, want)
}

/// The boxes inside a buffer: kind, where the box starts, and the half-open range of its body.
fn boxes_in(buf: &[u8]) -> Vec<([u8; 4], usize, usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 8 <= buf.len() && out.len() < MAX_ENTRIES {
        let size32 = u32::from_be_bytes([buf[at], buf[at + 1], buf[at + 2], buf[at + 3]]);
        let mut kind = [0u8; 4];
        kind.copy_from_slice(&buf[at + 4..at + 8]);
        let (size, header) = match size32 {
            1 if at + 16 <= buf.len() => {
                let mut v = [0u8; 8];
                v.copy_from_slice(&buf[at + 8..at + 16]);
                (u64::from_be_bytes(v) as usize, 16)
            }
            0 => (buf.len() - at, 8),
            n => (n as usize, 8),
        };
        if size < header {
            break;
        }
        let end = (at + size).min(buf.len());
        out.push((kind, at, at + header, end.max(at + header)));
        at = (at + size).max(at + header);
    }
    out
}

/// A box by type at this level.
fn find_box<'a>(buf: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    boxes_in(buf)
        .into_iter()
        .find(|(k, _, _, _)| k == kind)
        .map(|(_, _, start, end)| &buf[start..end])
}

/// A box down a path of container boxes: `udta/meta`, `mdia/minf`.
fn find_path<'a>(buf: &'a [u8], path: &[&[u8; 4]]) -> Option<&'a [u8]> {
    let mut here = buf;
    for kind in path {
        here = find_box(here, kind)?;
    }
    Some(here)
}

/// The children of a `meta` box, which is the one box in this format whose layout depends on
/// who wrote it: ISO made it a full box, with a version and flags in front of its children,
/// and QuickTime did not. Deciding by *trying* is the only honest way — the first child's own
/// size says which of the two guesses parsed.
fn meta_children(meta: &[u8]) -> Option<&[u8]> {
    let plausible = |buf: &[u8]| {
        buf.len() >= 8
            && u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize <= buf.len()
            && u32::from_be_bytes([buf[0], buf[1], buf[2], buf[3]]) >= 8
    };
    if meta.len() > 4 && plausible(&meta[4..]) {
        return Some(&meta[4..]);
    }
    plausible(meta).then_some(meta)
}

/// `ilst`: one box per tag, named by a four-character code, each holding a `data` box with
/// four bytes of type, four of locale, and then the value.
fn ilst(list: &[u8]) -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    for (kind, _, start, end) in boxes_in(list) {
        let Some(name) = tag_name(&kind) else {
            continue;
        };
        let Some(data) = find_box(&list[start..end], b"data") else {
            continue;
        };
        if data.len() < 8 {
            continue;
        }
        let kind_of_value = u32::from_be_bytes([data[0], data[1], data[2], data[3]]) & 0xFF_FFFF;
        let value = &data[8..];
        let text = match kind_of_value {
            // 1 is UTF-8; 0 and 21 are binary, and the only binary tags worth a row are the
            // numbers, which are big-endian and one or two bytes.
            1 => String::from_utf8_lossy(value).to_string(),
            _ if value.len() == 1 => format!("{}", value[0]),
            _ if value.len() == 2 => {
                format!("{}", u16::from_be_bytes([value[0], value[1]]))
            }
            // `trkn` and `disk`: a pair of 16-bit numbers inside eight bytes of padding.
            _ if value.len() >= 6 => format!(
                "{} of {}",
                u16::from_be_bytes([value[2], value[3]]),
                u16::from_be_bytes([value[4], value[5]])
            ),
            _ => continue,
        };
        let text = text.trim().to_string();
        if !text.is_empty() {
            out.push((name, text));
        }
    }
    out
}

fn tag_name(kind: &[u8; 4]) -> Option<&'static str> {
    // The copyright sign is `0xA9` in the one-byte encoding these codes were written in.
    Some(match kind {
        b"\xA9nam" => "name",
        b"\xA9ART" => "artist",
        b"\xA9alb" => "album",
        b"\xA9day" => "year",
        b"\xA9gen" | b"gnre" => "genre",
        b"\xA9cmt" => "comment",
        b"\xA9wrt" => "writer",
        b"\xA9too" => "encoder",
        b"desc" => "description",
        b"trkn" => "track",
        b"disk" => "disc",
        _ => return None,
    })
}

struct MovieHeader {
    created: u64,
    modified: u64,
    timescale: u32,
    duration: u64,
}

impl MovieHeader {
    fn seconds(&self) -> f64 {
        if self.timescale == 0 {
            return 0.0;
        }
        self.duration as f64 / f64::from(self.timescale)
    }
}

/// `mvhd` and `mdhd` share this shape: a version that says whether the times and the duration
/// are 32- or 64-bit, and then the four numbers.
fn movie_header(buf: &[u8]) -> MovieHeader {
    let mut c = Cur::new(buf);
    c.big_endian = true;
    let version = c.u32() >> 24;
    let (created, modified, timescale, duration) = if version == 1 {
        (c.u64(), c.u64(), c.u32(), c.u64())
    } else {
        (
            u64::from(c.u32()),
            u64::from(c.u32()),
            c.u32(),
            u64::from(c.u32()),
        )
    };
    MovieHeader {
        created,
        modified,
        timescale,
        duration,
    }
}

#[derive(Default)]
struct Track {
    id: Option<u32>,
    /// `vide`, `soun`, `subt`: what the handler box says this track carries.
    handler: String,
    /// The sample entry's own four-character code: `avc1`, `mp4a`, `hvc1`.
    codec: String,
    width: u32,
    height: u32,
    channels: u16,
    sample_rate: u32,
    bits: u16,
    language: String,
    seconds: f64,
    /// Where the sample description is inside the `trak`, for the row `Enter` follows. Only
    /// ISO base media has one; the field below is what the other two formats call theirs.
    stsd_at: Option<usize>,
    codec_label: &'static str,
}

impl Track {
    /// What goes in the group heading: the kind, the codec and the one number that identifies
    /// the track — its size for a picture, its rate for sound.
    fn headline(&self) -> String {
        let mut out = String::new();
        let _ = write!(out, "{}", kind_name(&self.handler));
        if !self.codec.is_empty() {
            let _ = write!(out, " · {}", codec_name(&self.codec));
        }
        if self.width > 0 && self.height > 0 {
            let _ = write!(out, " · {}×{}", self.width, self.height);
        } else if self.sample_rate > 0 {
            let _ = write!(out, " · {} Hz", self.sample_rate);
            if self.channels > 0 {
                let _ = write!(out, " · {}", channel_name(self.channels));
            }
        }
        out
    }

    /// The same thing in one phrase for the pane's title, for the tracks that have a shape
    /// worth putting there.
    fn brief(&self) -> Option<String> {
        if self.width > 0 && self.height > 0 {
            return Some(format!(
                "{} {}×{}",
                codec_name(&self.codec),
                self.width,
                self.height
            ));
        }
        if self.sample_rate > 0 {
            return Some(format!(
                "{} {} kHz {}",
                codec_name(&self.codec),
                self.sample_rate / 1000,
                channel_name(self.channels)
            ));
        }
        None
    }

    fn rows(&self, trak_at: u64, len: u64) -> Vec<Row> {
        let mut rows = Vec::new();
        if let Some(id) = self.id {
            rows.push(Row::field("track_ID", format!("{id:08X}"), format!("{id}")));
        }
        if !self.handler.is_empty() {
            rows.push(Row::field(
                "handler_type",
                hex_of(self.handler.as_bytes(), 0, 4),
                format!("{} ({})", kind_name(&self.handler), self.handler),
            ));
        }
        if self.seconds > 0.0 {
            rows.push(Row::field("duration", "", clock(self.seconds)));
        }
        if !self.language.is_empty() && self.language != "und" {
            rows.push(Row::field("language", "", &self.language));
        }
        if self.width > 0 || self.height > 0 {
            rows.push(Row::field(
                "width × height",
                "",
                format!("{} × {}", self.width, self.height),
            ));
        }
        if self.sample_rate > 0 {
            rows.push(Row::field(
                "samplerate",
                "",
                format!("{} Hz", self.sample_rate),
            ));
        }
        if self.channels > 0 {
            rows.push(Row::field(
                "channelcount",
                format!("{:04X}", self.channels),
                format!("{} ({})", self.channels, channel_name(self.channels)),
            ));
        }
        if self.bits > 0 {
            rows.push(Row::field(
                "samplesize",
                format!("{:04X}", self.bits),
                format!("{} bit", self.bits),
            ));
        }
        if !self.codec.is_empty() {
            match self.stsd_at {
                Some(o) => rows.push(Row::entry(
                    "  sample description",
                    format!("{}  {}", self.codec, codec_name(&self.codec)),
                    in_file(trak_at + o as u64, len),
                )),
                None => rows.push(Row::field(
                    if self.codec_label.is_empty() {
                        "codec"
                    } else {
                        self.codec_label
                    },
                    &self.codec,
                    codec_name(&self.codec),
                )),
            }
        }
        rows
    }
}

/// One `trak`: its own header, the media header under it, and the first sample description,
/// which is where the codec and the picture's real size are written.
fn track_of(trak: &[u8], movie_seconds: f64) -> Track {
    let mut t = Track::default();
    if let Some(tkhd) = find_box(trak, b"tkhd") {
        let mut c = Cur::new(tkhd);
        c.big_endian = true;
        let version = c.u32() >> 24;
        if version == 1 {
            let _created = c.u64();
            let _modified = c.u64();
            t.id = Some(c.u32());
            let _reserved = c.u32();
            let _duration = c.u64();
        } else {
            let _created = c.u32();
            let _modified = c.u32();
            t.id = Some(c.u32());
            let _reserved = c.u32();
            let _duration = c.u32();
        }
        // Then the presentation matrix, and after it the track's size on screen as two 16.16
        // fixed-point numbers. That is the *display* size, which a stretched film states
        // differently from its stored size; the sample description below wins when it has one.
        c.seek(c.pos + 8 + 2 + 2 + 2 + 2 + 36);
        let w = c.u32() >> 16;
        let h = c.u32() >> 16;
        t.width = w;
        t.height = h;
    }
    if let Some(mdhd) = find_path(trak, &[b"mdia", b"mdhd"]) {
        let m = movie_header(mdhd);
        t.seconds = if m.timescale == 0 {
            movie_seconds
        } else {
            m.seconds()
        };
        // The language is three five-bit letters, each biased by 0x60, in one 16-bit field.
        let tail = if (mdhd.first().copied().unwrap_or(0)) == 1 {
            32
        } else {
            20
        };
        if mdhd.len() >= tail + 2 {
            let packed = u16::from_be_bytes([mdhd[tail], mdhd[tail + 1]]);
            t.language = iso639(packed);
        }
    }
    if let Some(hdlr) = find_path(trak, &[b"mdia", b"hdlr"]) {
        if hdlr.len() >= 12 {
            t.handler = fourcc(&hdlr[8..12]);
        }
    }
    if let Some((stsd, at)) = find_stsd(trak) {
        t.stsd_at = Some(at);
        // A full box, then the number of entries, then the entries themselves.
        let entries = &stsd[8.min(stsd.len())..];
        if let Some((kind, _, start, end)) = boxes_in(entries).into_iter().next() {
            t.codec = fourcc(&kind);
            let e = &entries[start..end];
            match t.handler.as_str() {
                // A video sample entry: the stored size, in pixels, at a fixed place after the
                // common part of the entry. These offsets count from the entry's body, which
                // is why the eight bytes of `size` and `type` are already gone.
                "vide" | "pict" => {
                    if e.len() >= 28 {
                        let w = u16::from_be_bytes([e[24], e[25]]);
                        let h = u16::from_be_bytes([e[26], e[27]]);
                        if w > 0 && h > 0 {
                            t.width = u32::from(w);
                            t.height = u32::from(h);
                        }
                    }
                }
                "soun" if e.len() >= 28 => {
                    t.channels = u16::from_be_bytes([e[16], e[17]]);
                    t.bits = u16::from_be_bytes([e[18], e[19]]);
                    // 16.16 fixed point, so the rate is the high half.
                    t.sample_rate = u32::from(u16::from_be_bytes([e[24], e[25]]));
                }
                _ => {}
            }
        }
    }
    t
}

/// `mdia/minf/stbl/stsd`, with where it starts inside the `trak` so `Enter` can go there.
fn find_stsd(trak: &[u8]) -> Option<(&[u8], usize)> {
    let mut here = trak;
    let mut base = 0usize;
    for kind in [b"mdia", b"minf", b"stbl"] {
        let (_, _, start, end) = boxes_in(here).into_iter().find(|(k, _, _, _)| k == kind)?;
        base += start;
        here = &here[start..end];
    }
    let (_, at, start, end) = boxes_in(here)
        .into_iter()
        .find(|(k, _, _, _)| k == b"stsd")?;
    Some((&here[start..end], base + at))
}

// ── Matroska and WebM ───────────────────────────────────────────────────────────────────────

/// The element ids worth naming. Matroska's ids carry their own length in their leading bits,
/// so these are the bytes as they appear in the file, read as one number.
mod ebml {
    pub const HEADER: u64 = 0x1A45_DFA3;
    pub const DOCTYPE: u64 = 0x4282;
    pub const DOCTYPE_VERSION: u64 = 0x4287;
    pub const SEGMENT: u64 = 0x1853_8067;
    pub const INFO: u64 = 0x1549_A966;
    pub const TIMECODE_SCALE: u64 = 0x002A_D7B1;
    pub const DURATION: u64 = 0x4489;
    pub const TITLE: u64 = 0x7BA9;
    pub const MUXING_APP: u64 = 0x4D80;
    pub const WRITING_APP: u64 = 0x5741;
    pub const DATE_UTC: u64 = 0x4461;
    pub const TRACKS: u64 = 0x1654_AE6B;
    pub const TRACK_ENTRY: u64 = 0xAE;
    pub const TRACK_NUMBER: u64 = 0xD7;
    pub const TRACK_TYPE: u64 = 0x83;
    pub const CODEC_ID: u64 = 0x86;
    pub const CODEC_NAME: u64 = 0x0025_8688;
    pub const TRACK_NAME: u64 = 0x536E;
    pub const LANGUAGE: u64 = 0x0022_B59C;
    pub const VIDEO: u64 = 0xE0;
    pub const PIXEL_WIDTH: u64 = 0xB0;
    pub const PIXEL_HEIGHT: u64 = 0xBA;
    pub const AUDIO: u64 = 0xE1;
    pub const SAMPLING: u64 = 0xB5;
    pub const CHANNELS: u64 = 0x9F;
    pub const BIT_DEPTH: u64 = 0x6264;
    pub const CLUSTER: u64 = 0x1F43_B675;
}

/// One EBML element: its id, where its body is, and how long the body is. A length of `None`
/// is the "unknown" every bit set, which is how a stream that is still being written says
/// "to the end of what there is".
#[derive(Debug, Clone, Copy)]
struct El {
    id: u64,
    at: u64,
    header: u64,
    size: Option<u64>,
}

fn matroska(src: &mut dyn Bytes, len: u64) -> Structure {
    let mut rows = Vec::new();
    let mut short = false;
    let (top, s) = elements(src, 0, len);
    short |= s;

    let mut doctype = String::from("matroska");
    if let Some(h) = top.iter().find(|e| e.id == ebml::HEADER) {
        let body = element_body(src, h, len);
        let fields = children(&body);
        if let Some(v) = fields.iter().find(|(id, _, _)| *id == ebml::DOCTYPE) {
            doctype = text(&body[v.1..v.2]);
        }
        rows.push(Row::group("EBML header"));
        rows.push(Row::field("DocType", "", &doctype));
        if let Some(v) = fields
            .iter()
            .find(|(id, _, _)| *id == ebml::DOCTYPE_VERSION)
        {
            rows.push(Row::field(
                "DocTypeVersion",
                "",
                format!("{}", uint(&body[v.1..v.2])),
            ));
        }
    }

    // The Segment is the file. Its children are read from the file, not from memory: one of
    // them is every frame of the film.
    let segment = top.iter().find(|e| e.id == ebml::SEGMENT);
    let mut duration = 0.0f64;
    let mut shape: Vec<String> = Vec::new();
    let mut count = 0usize;
    if let Some(seg) = segment {
        let end = seg
            .size
            .map(|n| (seg.at + seg.header + n).min(len))
            .unwrap_or(len);
        let (kids, s) = elements(src, seg.at + seg.header, end);
        short |= s;

        let mut scale = 1_000_000u64;
        if let Some(info) = kids.iter().find(|e| e.id == ebml::INFO) {
            let body = element_body(src, info, len);
            let fields = children(&body);
            let get = |id: u64| {
                fields
                    .iter()
                    .find(|(i, _, _)| *i == id)
                    .map(|(_, a, b)| &body[*a..*b])
            };
            if let Some(v) = get(ebml::TIMECODE_SCALE) {
                scale = uint(v).max(1);
            }
            rows.push(Row::group("Segment info"));
            rows.push(Row::field(
                "TimecodeScale",
                "",
                format!("{scale} ns per tick"),
            ));
            if let Some(v) = get(ebml::DURATION) {
                // The duration is a float, in ticks of the scale above.
                duration = float(v) * scale as f64 / 1e9;
                rows.push(Row::field("Duration", "", clock(duration)));
            }
            for (id, label) in [
                (ebml::TITLE, "Title"),
                (ebml::MUXING_APP, "MuxingApp"),
                (ebml::WRITING_APP, "WritingApp"),
            ] {
                if let Some(v) = get(id) {
                    let t = text(v);
                    if !t.is_empty() {
                        rows.push(Row::field(label, "", t));
                    }
                }
            }
            if let Some(v) = get(ebml::DATE_UTC) {
                // Nanoseconds since 2001, signed, which no file in a folder is ever negative.
                let ns = uint(v);
                rows.push(Row::field(
                    "DateUTC",
                    "",
                    unix_time(ns / 1_000_000_000 + EPOCH_2001),
                ));
            }
        }

        if let Some(tracks) = kids.iter().find(|e| e.id == ebml::TRACKS) {
            let body = element_body(src, tracks, len);
            for (i, (_, start, end)) in children(&body)
                .into_iter()
                .filter(|(id, _, _)| *id == ebml::TRACK_ENTRY)
                .enumerate()
            {
                let t = mkv_track(&body[start..end]);
                rows.push(Row::group(format!(
                    "Track {} · {}",
                    t.id.unwrap_or(i as u32 + 1),
                    t.headline()
                )));
                rows.extend(t.rows(0, 0));
                if let Some(b) = t.brief() {
                    shape.push(b);
                }
                count += 1;
            }
        }

        // The top of the segment, which is the map: where the index is, where the tags are,
        // and the first cluster, which is where the frames start. Clusters are listed once
        // and then counted — a two-hour film has tens of thousands of them.
        rows.push(Row::group("Segment"));
        let mut clusters = 0usize;
        for e in &kids {
            if e.id == ebml::CLUSTER {
                clusters += 1;
                if clusters > 1 {
                    continue;
                }
            }
            rows.push(Row::entry(
                format!("  {}", el_name(e.id)),
                format!(
                    "{:08X}  {}",
                    e.at,
                    e.size.map(size_cell).unwrap_or_else(|| "unknown".into())
                ),
                in_file(e.at, len),
            ));
        }
        if clusters > 1 {
            rows.push(Row::field("  Cluster", "", format!("{clusters} in all")));
        }
    }

    let mut summary = String::new();
    let _ = write!(
        summary,
        "{}",
        if doctype == "webm" {
            "WebM"
        } else {
            "Matroska"
        }
    );
    if duration > 0.0 {
        let _ = write!(summary, " · {}", clock(duration));
    }
    if !shape.is_empty() {
        let _ = write!(summary, " · {}", shape.join(", "));
    }
    if count > shape.len() {
        let _ = write!(summary, " · {count} tracks");
    }

    Structure {
        format: "Matroska",
        summary,
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: short,
    }
}

fn mkv_track(entry: &[u8]) -> Track {
    let mut t = Track::default();
    let fields = children(entry);
    let get = |id: u64| {
        fields
            .iter()
            .find(|(i, _, _)| *i == id)
            .map(|(_, a, b)| &entry[*a..*b])
    };
    if let Some(v) = get(ebml::TRACK_NUMBER) {
        t.id = Some(uint(v) as u32);
    }
    if let Some(v) = get(ebml::TRACK_TYPE) {
        t.handler = match uint(v) {
            1 => "vide",
            2 => "soun",
            0x11 => "subt",
            0x12 => "butn",
            _ => "",
        }
        .to_string();
    }
    t.codec_label = "CodecID";
    if let Some(v) = get(ebml::CODEC_ID) {
        t.codec = text(v);
    }
    if t.codec.is_empty() {
        if let Some(v) = get(ebml::CODEC_NAME) {
            t.codec = text(v);
        }
    }
    if let Some(v) = get(ebml::LANGUAGE) {
        t.language = text(v);
    }
    if let Some(v) = get(ebml::TRACK_NAME) {
        let name = text(v);
        if !name.is_empty() {
            t.language = if t.language.is_empty() {
                name
            } else {
                format!("{} · {}", t.language, name)
            };
        }
    }
    if let Some(video) = get(ebml::VIDEO) {
        let kids = children(video);
        for (id, a, b) in kids {
            match id {
                ebml::PIXEL_WIDTH => t.width = uint(&video[a..b]) as u32,
                ebml::PIXEL_HEIGHT => t.height = uint(&video[a..b]) as u32,
                _ => {}
            }
        }
    }
    if let Some(audio) = get(ebml::AUDIO) {
        let kids = children(audio);
        for (id, a, b) in kids {
            match id {
                ebml::SAMPLING => t.sample_rate = float(&audio[a..b]) as u32,
                ebml::CHANNELS => t.channels = uint(&audio[a..b]) as u16,
                ebml::BIT_DEPTH => t.bits = uint(&audio[a..b]) as u16,
                _ => {}
            }
        }
    }
    t
}

/// The elements at one level, read from the file: a dozen bytes each, never a body.
fn elements(src: &mut dyn Bytes, from: u64, end: u64) -> (Vec<El>, bool) {
    let mut out = Vec::new();
    let mut at = from;
    let mut short = false;
    while at < end && out.len() < MAX_ENTRIES {
        let head = src.at(at, 16);
        if head.len() < 2 {
            short = head.len() == 1;
            break;
        }
        let Some((id, id_len)) = vint(&head, true) else {
            short = true;
            break;
        };
        let Some((size, size_len)) = vint(&head[id_len..], false) else {
            short = true;
            break;
        };
        let header = (id_len + size_len) as u64;
        // All ones is "unknown": the writer did not know how long this would be.
        let unknown = size == u64::MAX;
        let body = if unknown {
            end.saturating_sub(at + header)
        } else {
            size.min(end.saturating_sub(at + header))
        };
        short |= !unknown && body < size;
        out.push(El {
            id,
            at,
            header,
            size: if unknown { None } else { Some(body) },
        });
        // An element of unknown length is the last thing we can walk at this level: where the
        // next one starts is a question only its children answer.
        if unknown {
            break;
        }
        at += header + body;
    }
    (out, short)
}

fn element_body(src: &mut dyn Bytes, e: &El, len: u64) -> Vec<u8> {
    let start = e.at + e.header;
    if start >= len {
        return Vec::new();
    }
    let want = e
        .size
        .unwrap_or(len - start)
        .min(len - start)
        .min(MAX_BODY as u64) as usize;
    src.at(start, want)
}

/// The children of an element already in memory: id, and the body's range in the buffer.
fn children(buf: &[u8]) -> Vec<(u64, usize, usize)> {
    children_at(buf, 0)
}

fn children_at(buf: &[u8], depth: usize) -> Vec<(u64, usize, usize)> {
    if depth > MAX_DEPTH {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < buf.len() && out.len() < MAX_ENTRIES {
        let Some((id, id_len)) = vint(&buf[at..], true) else {
            break;
        };
        let Some((size, size_len)) = vint(&buf[at + id_len..], false) else {
            break;
        };
        let start = at + id_len + size_len;
        let size = if size == u64::MAX {
            buf.len() - start.min(buf.len())
        } else {
            size as usize
        };
        let end = start.saturating_add(size).min(buf.len());
        if start > buf.len() {
            break;
        }
        out.push((id, start, end));
        at = end.max(start);
        if size == 0 && start == at {
            at += 1;
        }
    }
    out
}

/// A variable-width integer. An id keeps the marker bits that say how long it is — that is
/// what makes `0x1A45DFA3` the number everyone writes for the EBML header — and a length
/// drops them, because what is wanted there is the number.
fn vint(buf: &[u8], keep_marker: bool) -> Option<(u64, usize)> {
    let first = *buf.first()?;
    if first == 0 {
        // No marker bit in the first byte: this is a number eight bytes or longer, which no
        // real file uses and which this reader declines rather than guesses.
        return None;
    }
    let width = first.leading_zeros() as usize + 1;
    if width > 8 || buf.len() < width {
        return None;
    }
    let mut value = if keep_marker {
        u64::from(first)
    } else {
        u64::from(first & (0x7F >> (width - 1)))
    };
    for b in &buf[1..width] {
        value = (value << 8) | u64::from(*b);
    }
    // All data bits set is the "unknown length" marker; say so with a value no length has.
    if !keep_marker {
        let bits = width * 7;
        let all_ones = if bits >= 64 {
            u64::MAX
        } else {
            (1u64 << bits) - 1
        };
        if value == all_ones {
            return Some((u64::MAX, width));
        }
    }
    Some((value, width))
}

/// A big-endian unsigned integer of whatever width the element was written in.
fn uint(buf: &[u8]) -> u64 {
    buf.iter()
        .take(8)
        .fold(0u64, |a, b| (a << 8) | u64::from(*b))
}

/// Matroska floats are 4 or 8 bytes, big-endian, in IEEE 754.
fn float(buf: &[u8]) -> f64 {
    match buf.len() {
        4 => f64::from(f32::from_bits(uint(buf) as u32)),
        8 => f64::from_bits(uint(buf)),
        _ => uint(buf) as f64,
    }
}

fn text(buf: &[u8]) -> String {
    let end = buf.iter().position(|b| *b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).trim().to_string()
}

fn el_name(id: u64) -> String {
    match id {
        ebml::HEADER => "EBML".into(),
        ebml::SEGMENT => "Segment".into(),
        ebml::INFO => "Info".into(),
        ebml::TRACKS => "Tracks".into(),
        ebml::CLUSTER => "Cluster".into(),
        0x1C53_BB6B => "Cues".into(),
        0x1043_A770 => "Chapters".into(),
        0x1254_C367 => "Tags".into(),
        0x1941_A469 => "Attachments".into(),
        0x114D_9B74 => "SeekHead".into(),
        0xEC => "Void".into(),
        other => format!("{other:X}"),
    }
}

// ── RIFF: AVI, WAV, WebP ────────────────────────────────────────────────────────────────────

fn riff(src: &mut dyn Bytes, head: &[u8], len: u64) -> Structure {
    let form = fourcc(&head[8..12]);
    // The declared length counts from after `RIFF` and its own size field, so eight bytes of
    // the file are outside it. A file whose real length disagrees was cut short or padded.
    let declared = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]])) + 8;
    let end = declared.min(len);
    let (top, short) = chunks(src, 12, end);

    let mut rows = vec![Row::group("RIFF")];
    rows.push(Row::field("formType", hex_of(head, 8, 4), &form));
    rows.push(Row::field(
        "size",
        format!("{:08X}", declared - 8),
        format!("{} of {}", size_cell(declared), size_cell(len)),
    ));

    let mut summary = form_name(&form).to_string();
    match form.as_str() {
        "AVI " | "AVIX" => avi(src, &top, &mut rows, &mut summary, len),
        "WAVE" => wave(src, &top, &mut rows, &mut summary, len),
        "WEBP" => webp(src, &top, &mut rows, &mut summary, len),
        _ => {}
    }

    // The `INFO` list, which is where a RIFF file of any kind writes who made it.
    if let Some(info) = top
        .iter()
        .find(|c| c.kind == *b"LIST" && c.form == Some(*b"INFO"))
    {
        let body = chunk_body(src, info, len);
        let mut tags = Vec::new();
        for (kind, start, end) in chunks_in(&body[4.min(body.len())..]) {
            if let Some(name) = info_name(&kind) {
                let value = text(&body[4 + start..4 + end]);
                if !value.is_empty() {
                    tags.push((name, value));
                }
            }
        }
        if !tags.is_empty() {
            rows.push(Row::group("INFO"));
            for (name, value) in tags {
                rows.push(Row::field(name, "", value));
            }
        }
    }

    rows.push(Row::group("Chunks"));
    for c in &top {
        let label = match c.form {
            Some(form) => format!("  LIST {}", fourcc(&form)),
            None => format!("  {}", fourcc(&c.kind)),
        };
        rows.push(Row::entry(
            label,
            format!("{:08X}  {}", c.at, size_cell(c.size + 8)),
            in_file(c.at, len),
        ));
    }

    Structure {
        format: "RIFF",
        summary,
        rows,
        symbols: Vec::new(),
        maps: Vec::new(),
        clr: None,
        truncated: short || declared > len,
    }
}

#[derive(Debug, Clone, Copy)]
struct Chunk {
    kind: [u8; 4],
    /// For a `LIST`, the kind it is a list of, which is what names it.
    form: Option<[u8; 4]>,
    at: u64,
    size: u64,
}

fn chunks(src: &mut dyn Bytes, from: u64, end: u64) -> (Vec<Chunk>, bool) {
    let mut out = Vec::new();
    let mut at = from;
    let mut short = false;
    while at + 8 <= end && out.len() < MAX_ENTRIES {
        let head = src.at(at, 12);
        if head.len() < 8 {
            short = true;
            break;
        }
        let mut kind = [0u8; 4];
        kind.copy_from_slice(&head[..4]);
        let size = u64::from(u32::from_le_bytes([head[4], head[5], head[6], head[7]]));
        let form = (matches!(&kind, b"LIST" | b"RIFF") && head.len() >= 12).then(|| {
            let mut f = [0u8; 4];
            f.copy_from_slice(&head[8..12]);
            f
        });
        let clamped = size.min(end - at - 8);
        short |= clamped < size;
        out.push(Chunk {
            kind,
            form,
            at,
            size: clamped,
        });
        // Chunks are padded to an even length, and the pad byte is not counted in the size.
        at += 8 + clamped + (clamped & 1);
    }
    (out, short)
}

fn chunk_body(src: &mut dyn Bytes, c: &Chunk, len: u64) -> Vec<u8> {
    let start = c.at + 8;
    if start >= len {
        return Vec::new();
    }
    let want = c.size.min(len - start).min(MAX_BODY as u64) as usize;
    src.at(start, want)
}

fn chunks_in(buf: &[u8]) -> Vec<([u8; 4], usize, usize)> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 8 <= buf.len() && out.len() < MAX_ENTRIES {
        let mut kind = [0u8; 4];
        kind.copy_from_slice(&buf[at..at + 4]);
        let size =
            u32::from_le_bytes([buf[at + 4], buf[at + 5], buf[at + 6], buf[at + 7]]) as usize;
        let start = at + 8;
        let end = start.saturating_add(size).min(buf.len());
        out.push((kind, start, end));
        at = start + size + (size & 1);
    }
    out
}

fn avi(src: &mut dyn Bytes, top: &[Chunk], rows: &mut Vec<Row>, summary: &mut String, len: u64) {
    let Some(hdrl) = top
        .iter()
        .find(|c| c.kind == *b"LIST" && c.form == Some(*b"hdrl"))
    else {
        return;
    };
    let body = chunk_body(src, hdrl, len);
    let inner = &body[4.min(body.len())..];
    let kids = chunks_in(inner);
    let mut seconds = 0.0f64;
    let mut shape: Vec<String> = Vec::new();

    if let Some((_, start, end)) = kids.iter().find(|(k, _, _)| k == b"avih") {
        let h = &inner[*start..*end];
        let mut c = Cur::new(h);
        let micros = c.u32();
        let _max_rate = c.u32();
        let _padding = c.u32();
        let flags = c.u32();
        let frames = c.u32();
        let _initial = c.u32();
        let streams = c.u32();
        let _buffer = c.u32();
        let width = c.u32();
        let height = c.u32();
        let fps = if micros > 0 {
            1e6 / f64::from(micros)
        } else {
            0.0
        };
        seconds = f64::from(frames) * f64::from(micros) / 1e6;
        rows.push(Row::group("Main AVI header"));
        rows.push(Row::field(
            "dwMicroSecPerFrame",
            format!("{micros:08X}"),
            format!("{micros} µs ({fps:.3} fps)"),
        ));
        rows.push(Row::field(
            "dwTotalFrames",
            format!("{frames:08X}"),
            format!("{frames} ({})", clock(seconds)),
        ));
        rows.push(Row::field(
            "dwStreams",
            format!("{streams:08X}"),
            format!("{streams}"),
        ));
        rows.push(Row::field(
            "dwWidth × dwHeight",
            "",
            format!("{width} × {height}"),
        ));
        rows.push(Row::field(
            "dwFlags",
            format!("{flags:08X}"),
            avi_flags(flags),
        ));
        if width > 0 && height > 0 {
            shape.push(format!("{width}×{height}"));
        }
    }

    // One `strl` list per stream: its header says what kind it is and how fast it runs, and
    // the format chunk after it says in which codec.
    for (i, (_, start, end)) in kids
        .iter()
        .filter(|(k, s, e)| {
            k == b"LIST" && inner.len() >= *e && *e - *s >= 4 && &inner[*s..*s + 4] == b"strl"
        })
        .enumerate()
    {
        let strl = &inner[*start + 4..*end];
        let parts = chunks_in(strl);
        let mut kind = String::new();
        let mut handler = String::new();
        let mut rate = 0.0f64;
        for (k, a, b) in &parts {
            let part = &strl[*a..*b];
            if k == b"strh" && part.len() >= 40 {
                kind = fourcc(&part[0..4]);
                handler = fourcc(&part[4..8]);
                let scale = u32::from_le_bytes([part[20], part[21], part[22], part[23]]);
                let r = u32::from_le_bytes([part[24], part[25], part[26], part[27]]);
                if scale > 0 {
                    rate = f64::from(r) / f64::from(scale);
                }
            }
        }
        let mut width = 0u32;
        let mut height = 0u32;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits = 0u16;
        let mut codec = handler.clone();
        let mut codec_label = "fccHandler";
        for (k, a, b) in &parts {
            let part = &strl[*a..*b];
            if k != b"strf" {
                continue;
            }
            match kind.as_str() {
                // A video stream's format chunk is a BITMAPINFOHEADER, whose `biCompression`
                // is the real four-character code — `strh`'s handler is often zero.
                "vids" if part.len() >= 20 => {
                    width = u32::from_le_bytes([part[4], part[5], part[6], part[7]]);
                    height = u32::from_le_bytes([part[8], part[9], part[10], part[11]]);
                    let comp = fourcc(&part[16..20]);
                    if comp.chars().any(|c| c.is_ascii_alphanumeric()) {
                        codec = comp;
                        codec_label = "biCompression";
                    }
                }
                // An audio stream's is a WAVEFORMATEX, with a numeric tag instead of a code.
                "auds" if part.len() >= 16 => {
                    let tag = u16::from_le_bytes([part[0], part[1]]);
                    channels = u16::from_le_bytes([part[2], part[3]]);
                    sample_rate = u32::from_le_bytes([part[4], part[5], part[6], part[7]]);
                    bits = u16::from_le_bytes([part[14], part[15]]);
                    codec = wave_format(tag).to_string();
                    codec_label = "wFormatTag";
                }
                _ => {}
            }
        }
        let t = Track {
            id: Some(i as u32 + 1),
            handler: match kind.as_str() {
                "vids" => "vide".into(),
                "auds" => "soun".into(),
                "txts" => "subt".into(),
                _ => String::new(),
            },
            codec,
            width,
            height,
            channels,
            sample_rate,
            bits,
            language: String::new(),
            seconds: if rate > 0.0 && kind == "vids" {
                seconds
            } else {
                0.0
            },
            stsd_at: None,
            codec_label,
        };
        rows.push(Row::group(format!("Stream {} · {}", i + 1, t.headline())));
        rows.extend(t.rows(0, 0));
        if let Some(b) = t.brief() {
            if !shape.iter().any(|s| b.contains(s)) {
                shape.push(b);
            }
        }
    }

    if seconds > 0.0 {
        let _ = write!(summary, " · {}", clock(seconds));
    }
    if !shape.is_empty() {
        let _ = write!(summary, " · {}", shape.join(", "));
    }
}

fn wave(src: &mut dyn Bytes, top: &[Chunk], rows: &mut Vec<Row>, summary: &mut String, len: u64) {
    let Some(fmt) = top.iter().find(|c| c.kind == *b"fmt ") else {
        return;
    };
    let body = chunk_body(src, fmt, len);
    if body.len() < 16 {
        return;
    }
    let mut c = Cur::new(&body);
    let tag = c.u16();
    let channels = c.u16();
    let rate = c.u32();
    let bytes_per_second = c.u32();
    let _block = c.u16();
    let bits = c.u16();
    rows.push(Row::group("Wave format"));
    rows.push(Row::field(
        "wFormatTag",
        format!("{tag:04X}"),
        wave_format(tag),
    ));
    rows.push(Row::field(
        "nChannels",
        format!("{channels:04X}"),
        format!("{channels} ({})", channel_name(channels)),
    ));
    rows.push(Row::field(
        "nSamplesPerSec",
        format!("{rate:08X}"),
        format!("{rate} Hz"),
    ));
    rows.push(Row::field(
        "wBitsPerSample",
        format!("{bits:04X}"),
        format!("{bits} bit"),
    ));
    // The length of the sound is the length of its data, which is a different chunk.
    let data = top.iter().find(|c| c.kind == *b"data");
    let seconds = match (data, bytes_per_second) {
        (Some(d), rate) if rate > 0 => d.size as f64 / f64::from(rate),
        _ => 0.0,
    };
    if let Some(d) = data {
        rows.push(Row::field(
            "data",
            "",
            format!("{} ({})", size_cell(d.size), clock(seconds)),
        ));
    }
    let _ = write!(
        summary,
        " · {} · {} Hz · {} bit · {}",
        wave_format(tag),
        rate,
        bits,
        channel_name(channels)
    );
    if seconds > 0.0 {
        let _ = write!(summary, " · {}", clock(seconds));
    }
}

fn webp(src: &mut dyn Bytes, top: &[Chunk], rows: &mut Vec<Row>, summary: &mut String, len: u64) {
    // Three shapes: a plain lossy frame, a lossless one, and the extended header that an
    // animated or transparent file starts with. Each writes its size differently.
    for c in top {
        let body = chunk_body(src, c, len);
        let (w, h, kind) = match (&c.kind, body.len()) {
            (b"VP8X", n) if n >= 10 => (
                1 + (u32::from(body[4]) | u32::from(body[5]) << 8 | u32::from(body[6]) << 16),
                1 + (u32::from(body[7]) | u32::from(body[8]) << 8 | u32::from(body[9]) << 16),
                "extended",
            ),
            (b"VP8 ", n) if n >= 10 => (
                u32::from(u16::from_le_bytes([body[6], body[7]]) & 0x3FFF),
                u32::from(u16::from_le_bytes([body[8], body[9]]) & 0x3FFF),
                "lossy",
            ),
            (b"VP8L", n) if n >= 5 => {
                let bits = u32::from_le_bytes([body[1], body[2], body[3], body[4]]);
                ((bits & 0x3FFF) + 1, ((bits >> 14) & 0x3FFF) + 1, "lossless")
            }
            _ => continue,
        };
        rows.push(Row::group("WebP"));
        rows.push(Row::field("kind", fourcc(&c.kind), kind));
        rows.push(Row::field("canvas", "", format!("{w} × {h}")));
        let _ = write!(summary, " · {kind} · {w}×{h}");
        if top.iter().any(|c| c.kind == *b"ANMF") {
            summary.push_str(" · animated");
        }
        return;
    }
}

// ── names ───────────────────────────────────────────────────────────────────────────────────

fn container_name(brand: &str, compatible: &str) -> &'static str {
    let has = |b: &str| brand == b || compatible.split(' ').any(|c| c == b);
    if has("qt  ") {
        return "QuickTime movie";
    }
    if has("M4A ") {
        return "MPEG-4 audio";
    }
    if has("M4V ") {
        return "MPEG-4 video";
    }
    if has("mif1") || has("heic") || has("heix") {
        return "HEIF image";
    }
    if has("avif") {
        return "AVIF image";
    }
    if has("3gp4") || has("3gp5") || has("3gp6") || has("3g2a") {
        return "3GPP";
    }
    if has("dash") {
        return "MPEG-4 (DASH)";
    }
    "MPEG-4"
}

fn form_name(form: &str) -> &'static str {
    match form {
        "AVI " | "AVIX" => "AVI",
        "WAVE" => "WAVE",
        "WEBP" => "WebP",
        "ACON" => "Windows animated cursor",
        "RMID" => "MIDI",
        _ => "RIFF",
    }
}

fn kind_name(handler: &str) -> &'static str {
    match handler {
        "vide" => "video",
        "soun" => "audio",
        "subt" | "sbtl" | "text" | "clcp" => "subtitle",
        "hint" => "hint",
        "meta" => "metadata",
        "pict" => "picture",
        "butn" => "buttons",
        "" => "track",
        _ => "other",
    }
}

/// What a four-character code or a Matroska codec id is, in the words somebody would use.
/// An unknown one is handed back as it is: a code this table has not met is still an answer.
fn codec_name(code: &str) -> String {
    let known = match code {
        "avc1" | "avc3" | "h264" | "H264" | "x264" | "V_MPEG4/ISO/AVC" => "H.264/AVC",
        "hvc1" | "hev1" | "hevc" | "V_MPEGH/ISO/HEVC" => "H.265/HEVC",
        "vvc1" | "vvi1" => "H.266/VVC",
        "av01" | "V_AV1" => "AV1",
        "vp08" | "VP80" | "V_VP8" => "VP8",
        "vp09" | "VP90" | "V_VP9" => "VP9",
        "mp4v" | "V_MPEG4/ISO/ASP" | "DIVX" | "divx" | "XVID" | "xvid" | "DX50" | "FMP4" => {
            "MPEG-4 Visual"
        }
        "mpeg" | "V_MPEG2" | "mp2v" => "MPEG-2 video",
        "V_MPEG1" => "MPEG-1 video",
        "mjpa" | "mjpb" | "MJPG" | "jpeg" => "Motion JPEG",
        "apcn" | "apch" | "apcs" | "apco" | "ap4h" => "Apple ProRes",
        "dvc " | "dvcp" | "dvhp" => "DV",
        "mp4a" | "A_AAC" | "A_AAC/MPEG4/LC" => "AAC",
        "ac-3" | "A_AC3" => "Dolby Digital",
        "ec-3" | "A_EAC3" => "Dolby Digital Plus",
        "mlpa" | "A_TRUEHD" => "Dolby TrueHD",
        "dtsc" | "dtse" | "dtsh" | "A_DTS" => "DTS",
        "Opus" | "opus" | "A_OPUS" => "Opus",
        "A_VORBIS" => "Vorbis",
        "A_FLAC" | "fLaC" => "FLAC",
        "alac" => "Apple Lossless",
        ".mp3" | "mp4a-6B" | "A_MPEG/L3" => "MP3",
        "A_MPEG/L2" => "MP2",
        "twos" | "sowt" | "in24" | "in32" | "lpcm" | "A_PCM/INT/LIT" | "A_PCM/INT/BIG" => "PCM",
        "samr" | "sawb" => "AMR",
        "tx3g" | "S_TEXT/UTF8" => "timed text",
        "S_TEXT/ASS" | "S_TEXT/SSA" => "SSA/ASS subtitles",
        "S_VOBSUB" => "VobSub",
        "S_HDMV/PGS" => "PGS subtitles",
        "c608" | "c708" => "closed captions",
        _ => "",
    };
    if known.is_empty() {
        code.to_string()
    } else {
        known.to_string()
    }
}

fn wave_format(tag: u16) -> &'static str {
    match tag {
        0x0001 => "PCM",
        0x0002 => "ADPCM",
        0x0003 => "IEEE float",
        0x0006 => "A-law",
        0x0007 => "µ-law",
        0x0011 => "IMA ADPCM",
        0x0031 | 0x0032 => "GSM 6.10",
        0x0050 => "MPEG audio",
        0x0055 => "MP3",
        0x0160..=0x0163 => "Windows Media Audio",
        0x2000 => "Dolby AC-3",
        0x2001 => "DTS",
        0xFFFE => "extensible",
        _ => "unknown",
    }
}

fn channel_name(n: u16) -> &'static str {
    match n {
        0 => "no channels",
        1 => "mono",
        2 => "stereo",
        3 => "2.1",
        6 => "5.1",
        8 => "7.1",
        _ => "multichannel",
    }
}

fn info_name(kind: &[u8; 4]) -> Option<&'static str> {
    Some(match kind {
        b"INAM" => "INAM (title)",
        b"IART" => "IART (artist)",
        b"IPRD" => "IPRD (album)",
        b"ICRD" => "ICRD (date)",
        b"IGNR" => "IGNR (genre)",
        b"ICMT" => "ICMT (comment)",
        b"ISFT" => "ISFT (software)",
        b"ICOP" => "ICOP (copyright)",
        b"IENG" => "IENG (engineer)",
        b"ITRK" => "ITRK (track)",
        _ => return None,
    })
}

fn avi_flags(f: u32) -> String {
    let mut out = Vec::new();
    if f & 0x0010 != 0 {
        out.push("has index");
    }
    if f & 0x0020 != 0 {
        out.push("must use index");
    }
    if f & 0x0100 != 0 {
        out.push("interleaved");
    }
    if f & 0x0800 != 0 {
        out.push("capture");
    }
    if f & 0x0002_0000 != 0 {
        out.push("copyrighted");
    }
    if out.is_empty() {
        return format!("{f:#X}");
    }
    out.join(", ")
}

// ── small conversions ───────────────────────────────────────────────────────────────────────

/// A four-character code as text, with anything unprintable spelled in hex so a corrupt one
/// cannot write control characters into the pane.
fn fourcc(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(4);
    for b in bytes.iter().take(4) {
        if b.is_ascii_graphic() || *b == b' ' {
            out.push(*b as char);
        } else {
            let _ = write!(out, "\\x{b:02X}");
        }
    }
    out
}

fn hex_of(buf: &[u8], from: usize, n: usize) -> String {
    let start = from.min(buf.len());
    let end = (start + n).min(buf.len());
    buf[start..end]
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A length of time as somebody would read it: hours only when there are any, and the
/// thousandths, because the difference between two cuts of the same film is often that small.
fn clock(seconds: f64) -> String {
    if !seconds.is_finite() || seconds <= 0.0 {
        return "0.000".to_string();
    }
    let total = seconds.floor() as u64;
    let ms = ((seconds - total as f64) * 1000.0).round() as u64;
    let (h, m, s) = (total / 3600, (total % 3600) / 60, total % 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}.{ms:03}")
    } else {
        format!("{m:02}:{s:02}.{ms:03}")
    }
}

/// A QuickTime date: seconds since 1904, which is the one epoch nothing else uses.
fn mac_time(secs: u64) -> String {
    if secs == 0 {
        return "not set".to_string();
    }
    if secs < EPOCH_1904 {
        // Before 1970: a real date, but not one this project's formatter does, and far more
        // often a file whose writer left the field as something other than a date.
        return format!("{secs} (1904 epoch)");
    }
    unix_time(secs - EPOCH_1904)
}

/// Three letters packed five bits each, biased by 0x60 — ISO 639-2 as QuickTime wrote it.
fn iso639(packed: u16) -> String {
    // The top bit set means the field is a Macintosh language code instead, which is a number.
    if packed & 0x8000 != 0 {
        return format!("code {}", packed & 0x7FFF);
    }
    let letters = [
        ((packed >> 10) & 0x1F) as u8 + 0x60,
        ((packed >> 5) & 0x1F) as u8 + 0x60,
        (packed & 0x1F) as u8 + 0x60,
    ];
    if letters.iter().all(|c| c.is_ascii_lowercase()) {
        String::from_utf8_lossy(&letters).to_string()
    } else {
        String::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::structure;

    // ── builders, so each test says what the file holds and nothing else ────────────────────

    /// One ISO base media box: its length, its type, its body.
    fn bx(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = ((body.len() + 8) as u32).to_be_bytes().to_vec();
        v.extend_from_slice(kind);
        v.extend_from_slice(body);
        v
    }

    fn be32(n: u32) -> Vec<u8> {
        n.to_be_bytes().to_vec()
    }

    fn mvhd(timescale: u32, duration: u32, created: u32) -> Vec<u8> {
        let mut b = be32(0); // version 0, no flags
        b.extend(be32(created));
        b.extend(be32(created));
        b.extend(be32(timescale));
        b.extend(be32(duration));
        b.extend(std::iter::repeat_n(0u8, 80)); // rate, volume, matrix, next_track_ID
        bx(b"mvhd", &b)
    }

    fn tkhd(id: u32, width: u32, height: u32) -> Vec<u8> {
        let mut b = be32(0);
        b.extend(be32(0)); // created
        b.extend(be32(0)); // modified
        b.extend(be32(id));
        b.extend(be32(0)); // reserved
        b.extend(be32(0)); // duration
        b.extend(std::iter::repeat_n(0u8, 8 + 2 + 2 + 2 + 2 + 36));
        b.extend(be32(width << 16));
        b.extend(be32(height << 16));
        bx(b"tkhd", &b)
    }

    fn mdhd(timescale: u32, duration: u32, language: &str) -> Vec<u8> {
        let mut b = be32(0);
        b.extend(be32(0));
        b.extend(be32(0));
        b.extend(be32(timescale));
        b.extend(be32(duration));
        let l: Vec<u16> = language.bytes().map(|c| u16::from(c - 0x60)).collect();
        let packed = (l[0] << 10) | (l[1] << 5) | l[2];
        b.extend(packed.to_be_bytes());
        b.extend(be32(0));
        bx(b"mdhd", &b)
    }

    fn hdlr(kind: &[u8; 4]) -> Vec<u8> {
        let mut b = be32(0);
        b.extend(be32(0));
        b.extend_from_slice(kind);
        b.extend(std::iter::repeat_n(0u8, 12));
        bx(b"hdlr", &b)
    }

    fn video_stsd(codec: &[u8; 4], width: u16, height: u16) -> Vec<u8> {
        let mut e = vec![0u8; 24];
        e.extend(width.to_be_bytes());
        e.extend(height.to_be_bytes());
        e.extend(std::iter::repeat_n(0u8, 50));
        let entry = bx(codec, &e);
        let mut b = be32(0);
        b.extend(be32(1));
        b.extend(entry);
        bx(b"stsd", &b)
    }

    fn audio_stsd(codec: &[u8; 4], channels: u16, rate: u16, bits: u16) -> Vec<u8> {
        let mut e = vec![0u8; 16];
        e.extend(channels.to_be_bytes());
        e.extend(bits.to_be_bytes());
        e.extend(std::iter::repeat_n(0u8, 4));
        e.extend(rate.to_be_bytes());
        e.extend([0, 0]);
        let entry = bx(codec, &e);
        let mut b = be32(0);
        b.extend(be32(1));
        b.extend(entry);
        bx(b"stsd", &b)
    }

    fn trak(id: u32, kind: &[u8; 4], stsd: Vec<u8>, w: u32, h: u32, language: &str) -> Vec<u8> {
        let stbl = bx(b"stbl", &stsd);
        let minf = bx(b"minf", &stbl);
        let mut mdia = mdhd(600, 54_000, language);
        mdia.extend(hdlr(kind));
        mdia.extend(minf);
        let mut body = tkhd(id, w, h);
        body.extend(bx(b"mdia", &mdia));
        bx(b"trak", &body)
    }

    fn mp4() -> Vec<u8> {
        let mut ftyp = b"isom".to_vec();
        ftyp.extend(be32(512));
        ftyp.extend_from_slice(b"isomiso2avc1mp41");

        let mut moov = mvhd(1000, 90_000, (EPOCH_1904 + 1_700_000_000) as u32);
        moov.extend(trak(
            1,
            b"vide",
            video_stsd(b"avc1", 1920, 1080),
            1920,
            1080,
            "und",
        ));
        moov.extend(trak(
            2,
            b"soun",
            audio_stsd(b"mp4a", 2, 48_000, 16),
            0,
            0,
            "ita",
        ));

        // `udta > meta > ilst > ©nam > data`, which is where a title is written.
        let data = {
            let mut d = be32(1); // UTF-8
            d.extend(be32(0)); // locale
            d.extend_from_slice("Il nome del film".as_bytes());
            bx(b"data", &d)
        };
        let ilst = bx(b"ilst", &bx(b"\xA9nam", &data));
        let mut meta_body = be32(0); // meta is a full box in ISO's spelling
        meta_body.extend(hdlr(b"mdir"));
        meta_body.extend(ilst);
        moov.extend(bx(b"udta", &bx(b"meta", &meta_body)));

        let mut file = bx(b"ftyp", &ftyp);
        file.extend(bx(b"moov", &moov));
        file.extend(bx(b"mdat", &[0xAAu8; 64]));
        file
    }

    /// The whole file, the way the viewer hands it over.
    fn structure_of(bytes: &[u8]) -> Structure {
        let mut src: &[u8] = bytes;
        structure::read(&mut src).expect("this pane should recognise the file")
    }

    fn value_of<'a>(s: &'a Structure, label: &str) -> &'a str {
        s.rows
            .iter()
            .find(|r| r.label.trim() == label)
            .map(|r| r.value.as_str())
            .unwrap_or_else(|| panic!("no row {label:?} in {:?}", labels(s)))
    }

    fn labels(s: &Structure) -> Vec<&str> {
        s.rows.iter().map(|r| r.label.as_str()).collect()
    }

    fn has_group(s: &Structure, text: &str) -> bool {
        s.rows
            .iter()
            .any(|r| r.kind == structure::RowKind::Group && r.label.contains(text))
    }

    // ── ISO base media ─────────────────────────────────────────────────────────────────────

    #[test]
    fn an_mp4_says_what_it_is_how_long_it_is_and_what_is_in_it() {
        let s = structure_of(&mp4());
        assert_eq!(s.format, "ISO base media");
        assert!(!s.truncated, "{}", s.summary);
        // 90 000 ticks of a 1 000 Hz clock is a minute and a half.
        assert!(s.summary.starts_with("MPEG-4 · 01:30.000"), "{}", s.summary);
        assert!(s.summary.contains("H.264/AVC 1920×1080"), "{}", s.summary);
        assert!(s.summary.contains("AAC 48 kHz stereo"), "{}", s.summary);

        assert_eq!(value_of(&s, "major_brand"), "isom");
        assert_eq!(value_of(&s, "compatible_brands"), "isom iso2 avc1 mp41");
        assert_eq!(value_of(&s, "timescale"), "1000 Hz");
        assert_eq!(value_of(&s, "duration"), "01:30.000");
        // The creation time is counted from 1904, and nothing else in the program is.
        assert!(
            value_of(&s, "creation_time").starts_with("2023-11-14"),
            "{}",
            value_of(&s, "creation_time")
        );

        assert!(
            has_group(&s, "Track 1 · video · H.264/AVC · 1920×1080"),
            "{:?}",
            labels(&s)
        );
        assert!(
            has_group(&s, "Track 2 · audio · AAC · 48000 Hz · stereo"),
            "{:?}",
            labels(&s)
        );
        // The audio track said `ita`, and the video track said nothing, which is `und`.
        assert_eq!(value_of(&s, "language"), "ita");
        // And the tag that an audio player would show.
        assert_eq!(value_of(&s, "name"), "Il nome del film");
    }

    #[test]
    fn the_box_list_is_navigation_and_points_at_real_offsets() {
        let file = mp4();
        let s = structure_of(&file);
        // The box list is the last group, after the tracks: those have rows `Enter` follows
        // of their own, which is why this reads from the heading rather than from the top.
        let from = s
            .rows
            .iter()
            .position(|r| r.label == "Boxes")
            .expect("the file's own map of itself");
        let targets: Vec<(&str, u64)> = s.rows[from..]
            .iter()
            .filter(|r| r.is_target())
            .map(|r| (r.label.trim(), r.offset.unwrap()))
            .collect();
        // The three top-level boxes, in the order the file writes them, each at its own start.
        assert_eq!(targets[0], ("ftyp", 0));
        assert_eq!(targets[1].0, "moov");
        assert_eq!(targets[2].0, "mdat");
        // `moov` really starts where the row says, and the bytes there say `moov`.
        let at = targets[1].1 as usize;
        assert_eq!(&file[at + 4..at + 8], b"moov");
        // And one level inside it, which is where a reader is going next.
        let mvhd = targets
            .iter()
            .find(|(label, _)| *label == "moov/mvhd")
            .expect("the boxes inside moov are listed too");
        assert_eq!(&file[mvhd.1 as usize + 4..mvhd.1 as usize + 8], b"mvhd");
        // Every offset is inside the file: that is the only promise `Enter` needs.
        assert!(targets.iter().all(|(_, at)| *at < file.len() as u64));
    }

    #[test]
    fn a_quicktime_file_with_no_ftyp_is_still_read() {
        // Before `ftyp` existed, a `.mov` opened with whichever box came first.
        let mut file = bx(b"moov", &mvhd(600, 1200, 0));
        file.extend(bx(b"mdat", &[0u8; 16]));
        let s = structure_of(&file);
        assert_eq!(s.format, "ISO base media");
        assert_eq!(value_of(&s, "duration"), "00:02.000");
    }

    #[test]
    fn a_fragmented_file_says_so() {
        let mut moov = mvhd(1000, 0, 0);
        moov.extend(bx(b"mvex", &[0u8; 8]));
        let mut file = bx(b"ftyp", b"isomiso5");
        file.extend(bx(b"moov", &moov));
        file.extend(bx(b"moof", &[0u8; 8]));
        let s = structure_of(&file);
        assert!(s.summary.contains("fragmented"), "{}", s.summary);
    }

    #[test]
    fn a_box_that_claims_more_than_the_file_holds_is_clamped_not_believed() {
        // The size field says four gigabytes; the file is forty bytes.
        let mut file = b"\x00\x00\x00\x10ftypisomiso2".to_vec();
        file.extend_from_slice(b"\xFF\xFF\xFF\xF0mdat");
        file.extend_from_slice(&[0u8; 8]);
        let s = structure_of(&file);
        assert!(s.truncated, "a short read has to be admitted");
        // And the row still points somewhere real.
        assert!(s.targets().all(|r| r.offset.unwrap() < file.len() as u64));
    }

    #[test]
    fn a_box_shorter_than_its_own_header_stops_the_walk() {
        let mut file = b"\x00\x00\x00\x10ftypisomiso2".to_vec();
        file.extend_from_slice(b"\x00\x00\x00\x02moov");
        let s = structure_of(&file);
        assert!(s.truncated);
        assert_eq!(value_of(&s, "major_brand"), "isom");
    }

    // ── Matroska ───────────────────────────────────────────────────────────────────────────

    /// A length as EBML writes it: as few bytes as it fits in, with the marker bit.
    fn vint_of(n: u64) -> Vec<u8> {
        for width in 1..=8usize {
            let bits = width * 7;
            let max = (1u64 << bits) - 1;
            // All ones means "unknown", so a length that size needs one byte more.
            if n < max {
                let mut out = vec![0u8; width];
                let mut v = n;
                for i in (0..width).rev() {
                    out[i] = (v & 0xFF) as u8;
                    v >>= 8;
                }
                out[0] |= 0x80 >> (width - 1);
                return out;
            }
        }
        unreachable!("no EBML length is this big")
    }

    fn el(id: &[u8], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend(vint_of(body.len() as u64));
        v.extend_from_slice(body);
        v
    }

    fn uint_of(n: u64) -> Vec<u8> {
        let mut out = n.to_be_bytes().to_vec();
        while out.len() > 1 && out[0] == 0 {
            out.remove(0);
        }
        out
    }

    fn mkv(doctype: &str) -> Vec<u8> {
        let mut header = el(&[0x42, 0x82], doctype.as_bytes());
        header.extend(el(&[0x42, 0x87], &uint_of(4)));
        let mut file = el(&[0x1A, 0x45, 0xDF, 0xA3], &header);

        let mut info = el(&[0x2A, 0xD7, 0xB1], &uint_of(1_000_000));
        info.extend(el(&[0x44, 0x89], &90_000.0f64.to_be_bytes()));
        info.extend(el(&[0x4D, 0x80], b"libebml"));
        info.extend(el(&[0x57, 0x41], b"mkvmerge v81"));
        info.extend(el(&[0x7B, 0xA9], "Il titolo".as_bytes()));

        let mut video = el(&[0xB0], &uint_of(3840));
        video.extend(el(&[0xBA], &uint_of(2160)));
        let mut track1 = el(&[0xD7], &uint_of(1));
        track1.extend(el(&[0x83], &uint_of(1)));
        track1.extend(el(&[0x86], b"V_MPEGH/ISO/HEVC"));
        track1.extend(el(&[0x22, 0xB5, 0x9C], b"und"));
        track1.extend(el(&[0xE0], &video));

        let mut audio = el(&[0xB5], &48_000.0f64.to_be_bytes());
        audio.extend(el(&[0x9F], &uint_of(6)));
        let mut track2 = el(&[0xD7], &uint_of(2));
        track2.extend(el(&[0x83], &uint_of(2)));
        track2.extend(el(&[0x86], b"A_OPUS"));
        track2.extend(el(&[0x22, 0xB5, 0x9C], b"ita"));
        track2.extend(el(&[0xE1], &audio));

        let mut tracks = el(&[0xAE], &track1);
        tracks.extend(el(&[0xAE], &track2));

        let mut segment = el(&[0x15, 0x49, 0xA9, 0x66], &info);
        segment.extend(el(&[0x16, 0x54, 0xAE, 0x6B], &tracks));
        for _ in 0..3 {
            segment.extend(el(&[0x1F, 0x43, 0xB6, 0x75], &[0u8; 16]));
        }
        file.extend(el(&[0x18, 0x53, 0x80, 0x67], &segment));
        file
    }

    #[test]
    fn a_matroska_file_reads_its_header_its_info_and_its_tracks() {
        let s = structure_of(&mkv("matroska"));
        assert_eq!(s.format, "Matroska");
        assert!(!s.truncated, "{}", s.summary);
        assert!(
            s.summary.starts_with("Matroska · 01:30.000"),
            "{}",
            s.summary
        );
        assert!(s.summary.contains("H.265/HEVC 3840×2160"), "{}", s.summary);
        assert!(s.summary.contains("Opus 48 kHz 5.1"), "{}", s.summary);

        assert_eq!(value_of(&s, "DocType"), "matroska");
        assert_eq!(value_of(&s, "DocTypeVersion"), "4");
        assert_eq!(value_of(&s, "TimecodeScale"), "1000000 ns per tick");
        assert_eq!(value_of(&s, "Duration"), "01:30.000");
        assert_eq!(value_of(&s, "Title"), "Il titolo");
        assert_eq!(value_of(&s, "WritingApp"), "mkvmerge v81");
        assert!(
            has_group(&s, "Track 1 · video · H.265/HEVC · 3840×2160"),
            "{:?}",
            labels(&s)
        );
        assert!(has_group(&s, "Track 2 · audio · Opus"), "{:?}", labels(&s));
        assert_eq!(value_of(&s, "CodecID"), "H.265/HEVC");
    }

    #[test]
    fn webm_is_named_after_its_doctype() {
        let s = structure_of(&mkv("webm"));
        assert!(s.summary.starts_with("WebM · "), "{}", s.summary);
    }

    #[test]
    fn the_clusters_are_listed_once_and_then_counted() {
        let s = structure_of(&mkv("matroska"));
        let clusters: Vec<&Row> = s
            .rows
            .iter()
            .filter(|r| r.label.trim() == "Cluster")
            .collect();
        // One row that `Enter` follows, and one that says how many there are in all: a film
        // has tens of thousands of these and the pane is not a list of them.
        assert_eq!(clusters.len(), 2, "{:?}", labels(&s));
        assert!(clusters[0].offset.is_some());
        assert_eq!(clusters[1].value, "3 in all");
    }

    #[test]
    fn an_element_of_unknown_length_does_not_stop_the_file_being_read() {
        // A stream still being written: the Segment does not know how long it will be, and
        // says so with every data bit set.
        let mut file = el(&[0x1A, 0x45, 0xDF, 0xA3], &el(&[0x42, 0x82], b"matroska"));
        let info = el(&[0x2A, 0xD7, 0xB1], &uint_of(1_000_000));
        file.extend_from_slice(&[0x18, 0x53, 0x80, 0x67, 0xFF]);
        file.extend(el(&[0x15, 0x49, 0xA9, 0x66], &info));
        let s = structure_of(&file);
        assert_eq!(value_of(&s, "TimecodeScale"), "1000000 ns per tick");
        let segment = s
            .rows
            .iter()
            .find(|r| r.label.trim() == "Info")
            .expect("the segment's children are still walked");
        assert!(segment.offset.is_some());
    }

    // ── RIFF ───────────────────────────────────────────────────────────────────────────────

    fn ck(id: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut v = id.to_vec();
        v.extend((body.len() as u32).to_le_bytes());
        v.extend_from_slice(body);
        if body.len() % 2 == 1 {
            v.push(0);
        }
        v
    }

    fn list(form: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut inner = form.to_vec();
        inner.extend_from_slice(body);
        ck(b"LIST", &inner)
    }

    fn riff_file(form: &[u8; 4], body: &[u8]) -> Vec<u8> {
        let mut inner = form.to_vec();
        inner.extend_from_slice(body);
        let mut v = b"RIFF".to_vec();
        v.extend((inner.len() as u32).to_le_bytes());
        v.extend(inner);
        v
    }

    fn le32(n: u32) -> Vec<u8> {
        n.to_le_bytes().to_vec()
    }

    fn avi_file() -> Vec<u8> {
        let mut avih = le32(40_000); // 25 fps
        avih.extend(le32(0));
        avih.extend(le32(0));
        avih.extend(le32(0x10)); // has an index
        avih.extend(le32(1500)); // a minute at 25 fps
        avih.extend(le32(0));
        avih.extend(le32(2));
        avih.extend(le32(0));
        avih.extend(le32(720));
        avih.extend(le32(576));
        avih.extend(std::iter::repeat_n(0u8, 16));

        let mut strh_v = b"vids".to_vec();
        strh_v.extend_from_slice(b"XVID");
        strh_v.extend(std::iter::repeat_n(0u8, 12));
        strh_v.extend(le32(1)); // dwScale
        strh_v.extend(le32(25)); // dwRate
        strh_v.extend(std::iter::repeat_n(0u8, 24));
        let mut bih = le32(40);
        bih.extend(le32(720));
        bih.extend(le32(576));
        bih.extend([1, 0, 24, 0]);
        bih.extend_from_slice(b"XVID");
        bih.extend(std::iter::repeat_n(0u8, 20));
        let mut strl_v = ck(b"strh", &strh_v);
        strl_v.extend(ck(b"strf", &bih));

        let mut strh_a = b"auds".to_vec();
        strh_a.extend(std::iter::repeat_n(0u8, 52));
        let mut wfx = 0x0055u16.to_le_bytes().to_vec(); // MP3
        wfx.extend(2u16.to_le_bytes());
        wfx.extend(le32(44_100));
        wfx.extend(le32(16_000));
        wfx.extend(4u16.to_le_bytes());
        wfx.extend(16u16.to_le_bytes());
        let mut strl_a = ck(b"strh", &strh_a);
        strl_a.extend(ck(b"strf", &wfx));

        let mut hdrl = ck(b"avih", &avih);
        hdrl.extend(list(b"strl", &strl_v));
        hdrl.extend(list(b"strl", &strl_a));

        let mut body = list(b"hdrl", &hdrl);
        body.extend(list(b"INFO", &ck(b"ISFT", b"VirtualDub\0")));
        body.extend(list(b"movi", &ck(b"00dc", &[0u8; 32])));
        body.extend(ck(b"idx1", &[0u8; 16]));
        riff_file(b"AVI ", &body)
    }

    #[test]
    fn an_avi_reads_its_main_header_and_both_streams() {
        let s = structure_of(&avi_file());
        assert_eq!(s.format, "RIFF");
        assert!(!s.truncated, "{}", s.summary);
        assert!(s.summary.starts_with("AVI · 01:00.000"), "{}", s.summary);
        assert!(s.summary.contains("720×576"), "{}", s.summary);

        assert_eq!(value_of(&s, "formType"), "AVI ");
        assert_eq!(value_of(&s, "dwMicroSecPerFrame"), "40000 µs (25.000 fps)");
        assert_eq!(value_of(&s, "dwTotalFrames"), "1500 (01:00.000)");
        assert_eq!(value_of(&s, "dwStreams"), "2");
        assert_eq!(value_of(&s, "dwFlags"), "has index");
        // The codec comes from the format chunk, which is where the real four-character code
        // is: `strh`'s handler is zero in half the files ever written.
        assert!(
            has_group(&s, "Stream 1 · video · MPEG-4 Visual · 720×576"),
            "{:?}",
            labels(&s)
        );
        assert_eq!(value_of(&s, "biCompression"), "MPEG-4 Visual");
        assert!(
            has_group(&s, "Stream 2 · audio · MP3 · 44100 Hz · stereo"),
            "{:?}",
            labels(&s)
        );
        assert_eq!(value_of(&s, "wFormatTag"), "MP3");
        assert_eq!(value_of(&s, "ISFT (software)"), "VirtualDub");
    }

    #[test]
    fn a_wav_says_its_format_and_how_long_it_plays() {
        let mut fmt = 1u16.to_le_bytes().to_vec();
        fmt.extend(2u16.to_le_bytes());
        fmt.extend(le32(44_100));
        fmt.extend(le32(176_400));
        fmt.extend(4u16.to_le_bytes());
        fmt.extend(16u16.to_le_bytes());
        let mut body = ck(b"fmt ", &fmt);
        body.extend(ck(b"data", &vec![0u8; 176_400]));
        let s = structure_of(&riff_file(b"WAVE", &body));
        assert_eq!(value_of(&s, "wFormatTag"), "PCM");
        assert_eq!(value_of(&s, "nSamplesPerSec"), "44100 Hz");
        assert_eq!(value_of(&s, "nChannels"), "2 (stereo)");
        assert_eq!(value_of(&s, "data"), "172 K (00:01.000)");
        assert!(
            s.summary
                .starts_with("WAVE · PCM · 44100 Hz · 16 bit · stereo · 00:01.000"),
            "{}",
            s.summary
        );
    }

    #[test]
    fn a_webp_gives_its_canvas() {
        let mut vp8x = vec![0x10, 0, 0, 0]; // the alpha flag
                                            // Width and height are kept one less than they are, three bytes each.
        vp8x.extend([0x7F, 0x01, 0x00]); // 384
        vp8x.extend([0xDF, 0x00, 0x00]); // 224
        let s = structure_of(&riff_file(b"WEBP", &ck(b"VP8X", &vp8x)));
        assert_eq!(value_of(&s, "canvas"), "384 × 224");
        assert!(s.summary.contains("extended · 384×224"), "{}", s.summary);
    }

    #[test]
    fn a_riff_that_says_it_is_longer_than_it_is_admits_the_truncation() {
        let mut file = b"RIFF".to_vec();
        file.extend(le32(0x0010_0000));
        file.extend_from_slice(b"WAVE");
        file.extend(ck(
            b"fmt ",
            &[1, 0, 1, 0, 0x44, 0xAC, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0],
        ));
        let s = structure_of(&file);
        assert!(s.truncated);
        assert_eq!(value_of(&s, "wFormatTag"), "PCM");
    }

    // ── the files that are not containers ──────────────────────────────────────────────────

    #[test]
    fn nothing_else_is_mistaken_for_a_container() {
        for file in [
            &b"Just some prose in a text file.\n"[..],
            &b"\x7fELF\x02\x01\x01\x00"[..],
            &b"RIFF"[..],
            &b"\x1aE\xdf"[..],
            &[0u8; 64][..],
            &b"\x00\x00\x00\x00ftypisom"[..],
        ] {
            let mut src = file;
            let decoded = read(&mut src, file, file.len() as u64);
            assert!(
                decoded.is_none(),
                "this is not a media container: {:?}",
                &file[..file.len().min(8)]
            );
        }
    }

    #[test]
    fn a_four_character_code_can_never_write_control_characters_into_the_pane() {
        assert_eq!(fourcc(b"avc1"), "avc1");
        assert_eq!(fourcc(b"qt  "), "qt  ");
        assert_eq!(fourcc(&[0xA9, b'n', b'a', b'm']), "\\xA9nam");
        assert_eq!(fourcc(&[0, 1, 2, 3]), "\\x00\\x01\\x02\\x03");
    }

    #[test]
    fn a_length_of_time_reads_the_way_somebody_would_say_it() {
        assert_eq!(clock(0.0), "0.000");
        assert_eq!(clock(1.5), "00:01.500");
        assert_eq!(clock(90.0), "01:30.000");
        assert_eq!(clock(3661.25), "1:01:01.250");
        assert_eq!(clock(f64::INFINITY), "0.000");
    }
}
