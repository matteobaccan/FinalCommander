//! What a *file* of each format looks like: written, told apart, and put back together.
//!
//! **Written**: the encoded text under the file's own name with the format's extension
//! after it — `holiday.jpg.b64` — and, when a part size is asked for, one file per part,
//! `holiday.jpg.part1.uue`, `holiday.jpg.part2.uue`, …. Every part stands on its own: a UU
//! part is a whole `begin` … `end`, a yEnc part says its byte range and carries its CRC, a
//! base64 part is the base64 of its own bytes. So the parts can be joined by anything that
//! reads the format, and joining them by hand — `cat` — gives a file this reads too.
//!
//! **Told apart**: by what is in it, never by its name alone — `=ybegin`, a `begin 644 name`
//! line, the headers of a MIME part, PEM armour, a `data:` URI, and then the alphabet the
//! bytes are made of. The name is a hint only where two alphabets overlap: every hex file is
//! also valid base64, and it is the `.hex` that says which was meant.
//!
//! **Put back together**: under the name the file carries inside it, or its own without the
//! extension when it carries none; the pieces of one file — several sections of one name,
//! `part1`/`part2`, yEnc's `=ypart` — joined in order and checked against whatever the
//! format says about the whole.

use super::{base64, text, uu, yenc, Error, Format};

/// How a file is to be written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EncodeOptions {
    pub format: Format,
    /// Characters to a line; `None` is what the format usually has, and nothing is "do not
    /// break it" for the formats that are one run of characters.
    pub line: Option<usize>,
    /// How many of the file's bytes go into each part; `None` is one part.
    pub part_size: Option<u64>,
}

/// One file to write: its name, with no folder, and what goes in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Output {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// A file being encoded, one part at a time — so that whoever writes the parts can say how
/// far it has got and stop between two of them.
#[derive(Debug)]
pub struct Encoder<'a> {
    name: &'a str,
    data: &'a [u8],
    format: Format,
    line: usize,
    part: usize,
    /// The CRC32 of the whole file, which every yEnc part carries. Worked out once.
    crc: u32,
}

impl<'a> Encoder<'a> {
    pub fn new(name: &'a str, data: &'a [u8], options: &EncodeOptions) -> Result<Self, Error> {
        let format = options.format;
        if format.extension().is_none() {
            return Err(Error::NotFor(format));
        }
        let part = match options.part_size {
            Some(size) if size > 0 && size < data.len() as u64 => size as usize,
            _ => data.len().max(1),
        };
        let asked = options.line.unwrap_or_else(|| format.default_line());
        // A line holds whole groups: four characters of base64, eight of base32, a pair of
        // hex digits. Anything else would be a line that cannot be read on its own.
        let whole = |unit: usize| {
            if asked == 0 {
                0
            } else {
                (asked / unit).max(1) * unit
            }
        };
        let line = match format {
            Format::Base64 => whole(4),
            Format::Base32 => whole(8),
            Format::Hex => whole(2),
            // A count character and four characters for every three bytes.
            Format::Uu | Format::Xx if asked == 0 => uu::FULL_LINE,
            Format::Uu | Format::Xx => asked.saturating_sub(1) / 4 * 3,
            _ => asked,
        };
        let crc = if format == Format::Yenc && part < data.len() {
            crc32fast::hash(data)
        } else {
            0
        };
        Ok(Encoder {
            name,
            data,
            format,
            line,
            part,
            crc,
        })
    }

    /// How many files will be written.
    pub fn count(&self) -> usize {
        self.data.len().div_ceil(self.part).max(1)
    }

    /// The `index`th of them, from nothing.
    pub fn part(&self, index: usize) -> Output {
        let count = self.count();
        let start = (index * self.part).min(self.data.len());
        let end = (start + self.part).min(self.data.len());
        let chunk = &self.data[start..end];
        let bytes = match self.format {
            Format::Base64 => base64::encode(chunk, self.line),
            Format::Base32 => text::base32_encode(chunk, self.line),
            Format::Hex => text::hex_encode(chunk, self.line),
            Format::Url => text::url_encode(chunk, self.line),
            Format::Qp => text::qp_encode(chunk, self.line),
            Format::Uu => uu::encode_section(self.name, chunk, false, self.line),
            Format::Xx => uu::encode_section(self.name, chunk, true, self.line),
            Format::Yenc => {
                let place = (count > 1).then_some(yenc::Place {
                    part: index + 1,
                    total: count,
                    offset: start as u64,
                });
                yenc::encode(
                    self.name,
                    chunk,
                    self.line,
                    self.data.len() as u64,
                    self.crc,
                    place,
                )
            }
            // Refused in `new`.
            Format::Html | Format::Rot13 => Vec::new(),
        };
        let ext = self.format.extension().unwrap_or_default();
        let name = if count == 1 {
            format!("{}.{ext}", self.name)
        } else {
            // Padded to the width of the last number, so the parts sort as they are read.
            let width = count.to_string().len();
            format!("{}.part{:0width$}.{ext}", self.name, index + 1)
        };
        Output { name, bytes }
    }
}

/// `data`, which is the file called `name`, written as the options say: one [`Output`], or
/// one for every part.
pub fn encode_file(name: &str, data: &[u8], options: &EncodeOptions) -> Result<Vec<Output>, Error> {
    let encoder = Encoder::new(name, data, options)?;
    Ok((0..encoder.count()).map(|i| encoder.part(i)).collect())
}

/// The extensions an encoded file goes by, ours and the ones other programs write.
const EXTENSIONS: &[&str] = &[
    "b64", "base64", "b32", "uue", "uu", "xxe", "xx", "yenc", "ync", "qp", "hex", "urlenc",
];

/// How short a run of characters can be and still be taken for an encoding on the strength
/// of its alphabet alone: a word of eight letters is not a file somebody encoded.
const MIN_PAYLOAD: usize = 16;

/// `name.partN.ext` taken apart: what comes before `.partN`, the number, and the extension
/// in small letters. `None` for a name that is not one part of several.
pub fn part_of(file: &str) -> Option<(String, usize, String)> {
    let (stem, ext) = file.rsplit_once('.')?;
    if !EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
        return None;
    }
    let (before, tail) = stem.rsplit_once('.')?;
    let digits = tail
        .get(..4)
        .filter(|p| p.eq_ignore_ascii_case("part"))
        .map(|_| &tail[4..])?;
    if before.is_empty() || digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((
        before.to_string(),
        digits.parse().ok()?,
        ext.to_ascii_lowercase(),
    ))
}

/// What an encoded file is the encoding *of*, going by its own name: the name with the
/// extension off it, and with `.partN` off that. A name with no extension this knows gets
/// `.decoded` after it instead — `key.pem` decoded is not `key`, and must not be written
/// over `key.pem` either.
pub fn carried_name(file: &str) -> String {
    if let Some((before, _, _)) = part_of(file) {
        return before;
    }
    match file.rsplit_once('.') {
        Some((stem, ext))
            if !stem.is_empty() && EXTENSIONS.iter().any(|e| e.eq_ignore_ascii_case(ext)) =>
        {
            stem.to_string()
        }
        _ => format!("{file}.decoded"),
    }
}

/// A name out of a file, made safe to write: its last component only — a `begin` line that
/// says `../../etc/passwd` is not obeyed — and nothing in it that a file system refuses.
fn safe_name(name: &str) -> Option<String> {
    let last = name.rsplit(['/', '\\']).next()?.trim();
    let cleaned: String = last
        .chars()
        .map(|c| match c {
            ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    (!cleaned.is_empty() && cleaned != "." && cleaned != "..").then_some(cleaned)
}

/// The headers of a single MIME part, when the file starts with them: which encoding the
/// body is in, the name it gives, and the body. A multipart message is a folder of parts
/// and is read as one elsewhere; this is the attachment saved on its own.
fn mime_part(input: &[u8]) -> Option<(Format, Option<String>, &[u8])> {
    let head = &input[..input.len().min(16 * 1024)];
    let (end, body) = match (find(head, b"\r\n\r\n"), find(head, b"\n\n")) {
        (Some(crlf), Some(lf)) if crlf < lf => (crlf, crlf + 4),
        (_, Some(lf)) => (lf, lf + 2),
        (Some(crlf), None) => (crlf, crlf + 4),
        (None, None) => return None,
    };
    let headers = String::from_utf8_lossy(&head[..end]).replace("\r\n", "\n");
    let mut format = None;
    for line in headers.lines() {
        // A continuation of the header above it.
        if line.starts_with([' ', '\t']) {
            continue;
        }
        let (key, value) = line.split_once(':')?;
        if key.is_empty() || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
            return None;
        }
        if key.eq_ignore_ascii_case("content-transfer-encoding") {
            format = match value.trim().to_ascii_lowercase().as_str() {
                "base64" => Some(Format::Base64),
                "quoted-printable" => Some(Format::Qp),
                _ => return None,
            };
        }
    }
    let lower = headers.to_ascii_lowercase();
    let name = ["filename=", "name="].iter().find_map(|key| {
        let at = lower.find(key)? + key.len();
        let value = &headers[at..];
        let value = match value.strip_prefix('"') {
            Some(quoted) => quoted.split('"').next()?,
            None => value.split([';', '\n', ' ', '\t']).next()?,
        };
        safe_name(value)
    });
    Some((format?, name, &input[body..]))
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Is this hex and nothing else — pairs of digits, with only white space between them?
fn looks_hex(input: &[u8]) -> bool {
    let mut digits = 0usize;
    for byte in input {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if text::hex_value(*byte).is_none() {
            return false;
        }
        digits += 1;
    }
    digits >= MIN_PAYLOAD && digits.is_multiple_of(2)
}

/// Is this base32 as an encoder writes it: capitals and `2`–`7`, in groups of eight?
fn looks_base32(input: &[u8]) -> bool {
    let mut chars = 0usize;
    for byte in input {
        match byte {
            b'A'..=b'Z' | b'2'..=b'7' | b'=' => chars += 1,
            byte if byte.is_ascii_whitespace() => {}
            _ => return false,
        }
    }
    chars >= MIN_PAYLOAD && chars.is_multiple_of(8) && text::base32_decode(input).is_ok()
}

/// Is this quoted-printable? Plain text is, by the letter of the format, so the question is
/// whether it shows the one thing only an encoder writes: a line folded with a `=` at its
/// end — and nothing that an encoder would not have written.
fn looks_qp(input: &[u8]) -> bool {
    let printable = input
        .iter()
        .all(|b| matches!(b, b'\t' | b'\r' | b'\n' | 32..=126));
    let folded = find(input, b"=\n").is_some() || find(input, b"=\r\n").is_some();
    let short = input.split(|b| *b == b'\n').all(|line| line.len() <= 78);
    printable && folded && short && text::qp_decode(input).is_ok()
}

/// Is this percent-encoded: one run of characters that may stand in a URL, with a `%XX` in
/// it?
fn looks_url(input: &[u8]) -> bool {
    let body = input.trim_ascii_end();
    body.len() >= 3
        && body.contains(&b'%')
        && body.iter().all(|b| b.is_ascii_graphic())
        && text::url_decode(body).is_ok()
}

/// Which alphabet the first line after a `begin` is in: small letters and a lone `+` are
/// XXencode's, and never UUencode's.
fn section_is_xx(input: &[u8]) -> bool {
    let mut lines = input
        .split(|b| *b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        .skip_while(|line| uu::begin_name(line).is_none())
        .skip(1);
    lines
        .next()
        .is_some_and(|line| line == b"+" || line.iter().any(|b| *b > 0x60))
}

/// What a file is encoded as, read from what is in it; `None` when nothing says.
///
/// `name` is the file's own, and is only a hint: it decides between two formats the bytes
/// would both pass for, and never makes a file something its bytes are not.
pub fn detect(name: Option<&str>, input: &[u8]) -> Option<Format> {
    // The name first, where it names one of the formats that are a bare alphabet and the
    // bytes agree: a text that quotes a `begin 644` line, written as quoted-printable, is
    // quoted-printable.
    let ext = name
        .and_then(|n| n.rsplit_once('.'))
        .map(|(_, ext)| ext.to_ascii_lowercase());
    match ext.as_deref() {
        Some("b32") if text::base32_decode(input).is_ok() => return Some(Format::Base32),
        Some("hex") if text::hex_decode(input).is_ok() => return Some(Format::Hex),
        Some("qp") if text::qp_decode(input).is_ok() => return Some(Format::Qp),
        Some("urlenc") if text::url_decode(input).is_ok() => return Some(Format::Url),
        Some("b64" | "base64") if base64::decode(input).is_ok() => return Some(Format::Base64),
        _ => {}
    }
    if yenc::has_begin(input) {
        return Some(Format::Yenc);
    }
    if uu::has_begin(input) {
        return Some(if section_is_xx(input) {
            Format::Xx
        } else {
            Format::Uu
        });
    }
    if let Some((format, _, _)) = mime_part(input) {
        return Some(format);
    }
    // Narrowest alphabet first: sixteen digits are hex before they are base32, and base32
    // before base64, because each of those is also valid in the next.
    if looks_hex(input) {
        Some(Format::Hex)
    } else if looks_base32(input) {
        Some(Format::Base32)
    } else if base64::reads(input, MIN_PAYLOAD) {
        Some(Format::Base64)
    } else if looks_qp(input) {
        Some(Format::Qp)
    } else if looks_url(input) {
        Some(Format::Url)
    } else {
        None
    }
}

/// One encoded file to read: its name, with no folder, and what is in it.
#[derive(Debug, Clone, Copy)]
pub struct Input<'a> {
    pub name: &'a str,
    pub bytes: &'a [u8],
}

/// One file that came out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoded {
    /// What to call it: the name it carried, made safe, or the encoded file's own without
    /// its extension.
    pub name: String,
    pub bytes: Vec<u8>,
    pub format: Format,
    /// How many pieces it was joined from; one for a file that was whole.
    pub parts: usize,
    /// Whether the format carried a checksum to hold the result against. When it did, the
    /// result matched, or there would be no result.
    pub checked: bool,
}

/// A stretch of one file, before it is known which others it belongs with.
struct Piece {
    name: String,
    format: Format,
    part: Option<usize>,
    total: Option<usize>,
    offset: Option<u64>,
    size: Option<u64>,
    crc: Option<u32>,
    bytes: Vec<u8>,
    checked: bool,
}

fn pieces_of(input: &Input<'_>) -> Result<Vec<Piece>, Error> {
    let format = detect(Some(input.name), input.bytes).ok_or(Error::Unknown)?;
    let own = carried_name(input.name);
    let numbered = part_of(input.name).map(|(_, n, _)| n);
    let plain = |format: Format, name: String, bytes: Vec<u8>| Piece {
        name,
        format,
        part: numbered,
        total: None,
        offset: None,
        size: None,
        crc: None,
        bytes,
        checked: false,
    };
    match format {
        Format::Yenc => {
            let articles = yenc::articles(input.bytes)?;
            let alone = articles.len() == 1;
            Ok(articles
                .into_iter()
                .map(|a| Piece {
                    name: safe_name(&a.name).unwrap_or_else(|| own.clone()),
                    format,
                    part: a.part.or(numbered.filter(|_| alone)),
                    total: a.total,
                    offset: a.offset,
                    size: a.size,
                    crc: a.crc,
                    bytes: a.bytes,
                    checked: a.checked,
                })
                .collect())
        }
        Format::Uu | Format::Xx => {
            let sections = uu::sections(input.bytes)?;
            let alone = sections.len() == 1;
            Ok(sections
                .into_iter()
                .map(|s| {
                    let format = if s.xx { Format::Xx } else { Format::Uu };
                    let name = safe_name(&s.name).unwrap_or_else(|| own.clone());
                    let mut piece = plain(format, name, s.bytes);
                    // A file's own number counts only when the file is one section: two
                    // sections of one file are in the order they are written in.
                    piece.part = numbered.filter(|_| alone);
                    piece
                })
                .collect())
        }
        _ => {
            let (label, body) = match mime_part(input.bytes) {
                Some((_, label, body)) => (label, body),
                None => (None, input.bytes),
            };
            let bytes = match format {
                Format::Base64 => base64::decode(body)?,
                Format::Base32 => text::base32_decode(body)?,
                Format::Hex => text::hex_decode(body)?,
                Format::Url => text::url_decode(body.trim_ascii_end())?,
                Format::Qp => text::qp_decode(body)?,
                _ => return Err(Error::NotFor(format)),
            };
            Ok(vec![plain(format, label.unwrap_or(own), bytes)])
        }
    }
}

/// The pieces of one file, in order, as the file.
fn join(name: String, mut pieces: Vec<Piece>) -> Result<Decoded, Error> {
    let format = pieces[0].format;
    if pieces.iter().all(|p| p.part.is_some()) {
        pieces.sort_by_key(|p| p.part);
        for (i, piece) in pieces.iter().enumerate() {
            if piece.part != Some(i + 1) {
                return Err(Error::Parts {
                    name,
                    missing: i + 1,
                });
            }
        }
        let total = pieces.iter().find_map(|p| p.total).unwrap_or(0);
        if pieces.len() < total {
            return Err(Error::Parts {
                name,
                missing: pieces.len() + 1,
            });
        }
    }
    let parts = pieces.len();
    let size = pieces.iter().find_map(|p| p.size);
    let crc = pieces.iter().find_map(|p| p.crc);
    let mut checked = pieces.iter().all(|p| p.checked);
    let mut bytes = Vec::with_capacity(pieces.iter().map(|p| p.bytes.len()).sum());
    for piece in pieces {
        // A piece that says where it starts has to start where the one before it ended.
        if piece.offset.is_some_and(|at| at != bytes.len() as u64) {
            return Err(Error::Malformed {
                format,
                why: format!(
                    "{name}: piece {} does not start where the one before it ends",
                    piece.part.unwrap_or(1)
                ),
            });
        }
        bytes.extend_from_slice(&piece.bytes);
    }
    if let Some(expected) = size.filter(|size| *size != bytes.len() as u64) {
        return Err(Error::Size {
            name,
            expected,
            found: bytes.len() as u64,
        });
    }
    // The CRC of the whole, where the pieces carry it: each piece matching its own says
    // nothing about one that belongs to another posting of the same name.
    if let Some(expected) = crc.filter(|_| parts > 1 || !checked) {
        let found = crc32fast::hash(&bytes);
        if expected != found {
            return Err(Error::Checksum {
                name,
                expected,
                found,
            });
        }
        checked = true;
    }
    Ok(Decoded {
        name,
        bytes,
        format,
        parts,
        checked,
    })
}

/// Decode these files — one, or the parts of one, or several that have nothing to do with
/// each other — into the files they are the encoding of.
///
/// Pieces that carry the same name are one file: joined by their numbers when they have
/// them, in the order given when they do not. Nothing comes out half done: a piece that is
/// missing, a size or a checksum that does not match, a line that is not the format's, and
/// the answer is the reason and no file at all.
pub fn decode_files(inputs: &[Input<'_>]) -> Result<Vec<Decoded>, Error> {
    let mut groups: Vec<(String, Vec<Piece>)> = Vec::new();
    for input in inputs {
        for piece in pieces_of(input)? {
            match groups.iter_mut().find(|(name, _)| *name == piece.name) {
                Some((_, group)) => group.push(piece),
                None => groups.push((piece.name.clone(), vec![piece])),
            }
        }
    }
    if groups.is_empty() {
        return Err(Error::Unknown);
    }
    groups
        .into_iter()
        .map(|(name, pieces)| join(name, pieces))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every byte value, in an order that is not the alphabet's, and long enough to cross
    /// every line length many times.
    fn bytes(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 37 + i / 256 + 5) as u8).collect()
    }

    fn options(format: Format) -> EncodeOptions {
        EncodeOptions {
            format,
            line: None,
            part_size: None,
        }
    }

    fn inputs(outputs: &[Output]) -> Vec<Input<'_>> {
        outputs
            .iter()
            .map(|o| Input {
                name: &o.name,
                bytes: &o.bytes,
            })
            .collect()
    }

    #[test]
    fn a_file_comes_back_from_every_format_under_its_own_name() {
        for format in Format::FILES {
            for len in [1, 2, 3, 100, 4000] {
                let data = bytes(len);
                let written = encode_file("holiday.jpg", &data, &options(format)).unwrap();
                assert_eq!(written.len(), 1);
                let ext = format.extension().unwrap();
                assert_eq!(written[0].name, format!("holiday.jpg.{ext}"));
                let read = decode_files(&inputs(&written)).unwrap();
                assert_eq!(read.len(), 1, "{format:?}");
                assert_eq!(read[0].name, "holiday.jpg");
                assert_eq!(read[0].format, format, "{len} bytes");
                assert_eq!(read[0].bytes, data, "{format:?}, {len} bytes");
                assert_eq!(read[0].parts, 1);
                // Only yEnc says what it should come out as.
                assert_eq!(read[0].checked, format == Format::Yenc);
            }
        }
        // The two that are text to text are not for files.
        assert_eq!(
            encode_file("a", b"x", &options(Format::Html)),
            Err(Error::NotFor(Format::Html))
        );
    }

    #[test]
    fn the_line_length_asked_for_is_the_line_length_written() {
        let data = bytes(2000);
        for (format, asked, longest) in [
            (Format::Base64, 40, 40),
            (Format::Base64, 43, 40),
            (Format::Base32, 20, 16),
            (Format::Hex, 33, 32),
            (Format::Qp, 40, 40),
            (Format::Uu, 41, 41),
            (Format::Xx, 30, 29),
            (Format::Url, 50, 50),
            // An escape may end a yEnc line one character past it.
            (Format::Yenc, 64, 65),
        ] {
            let mut opts = options(format);
            opts.line = Some(asked);
            let written = encode_file("f", &data, &opts).unwrap();
            let rows = written[0]
                .bytes
                .split(|b| *b == b'\n')
                .map(|row| row.strip_suffix(b"\r").unwrap_or(row))
                .filter(|row| !row.starts_with(b"begin ") && !row.starts_with(b"=y"));
            let widest = rows.map(|row| row.len()).max().unwrap();
            assert!(
                widest <= longest && widest >= longest - 1,
                "{format:?} at {asked}: {widest}"
            );
            assert_eq!(decode_files(&inputs(&written)).unwrap()[0].bytes, data);
        }
        // Nothing is "one run": no line break anywhere, and it still reads.
        let mut opts = options(Format::Base64);
        opts.line = Some(0);
        let written = encode_file("f.bin", &data, &opts).unwrap();
        assert!(!written[0].bytes.contains(&b'\n'));
        assert_eq!(decode_files(&inputs(&written)).unwrap()[0].bytes, data);
    }

    #[test]
    fn a_file_in_parts_is_joined_from_them_in_any_order() {
        let data = bytes(10_000);
        for format in Format::FILES {
            let mut opts = options(format);
            opts.part_size = Some(900);
            let mut written = encode_file("big.iso", &data, &opts).unwrap();
            assert_eq!(written.len(), 12);
            let ext = format.extension().unwrap();
            assert_eq!(written[0].name, format!("big.iso.part01.{ext}"));
            assert_eq!(written[11].name, format!("big.iso.part12.{ext}"));
            // Marked in whatever order the panel happens to be sorted in.
            written.reverse();
            written.swap(3, 7);
            let read = decode_files(&inputs(&written)).unwrap();
            assert_eq!(read.len(), 1, "{format:?}");
            assert_eq!(read[0].name, "big.iso");
            assert_eq!(read[0].parts, 12);
            assert_eq!(read[0].bytes, data, "{format:?}");
            assert_eq!(read[0].checked, format == Format::Yenc);

            // One of them not there is said, and nothing comes out.
            written.remove(5);
            assert!(
                matches!(decode_files(&inputs(&written)), Err(Error::Parts { .. })),
                "{format:?}"
            );
        }
        // A part size the file fits in is one file, with no number in its name.
        let mut opts = options(Format::Base64);
        opts.part_size = Some(10_000);
        let written = encode_file("big.iso", &data, &opts).unwrap();
        assert_eq!(written.len(), 1);
        assert_eq!(written[0].name, "big.iso.b64");
    }

    #[test]
    fn parts_put_end_to_end_in_one_file_are_still_the_file() {
        let data = bytes(5_000);
        for format in [Format::Uu, Format::Xx, Format::Yenc] {
            let mut opts = options(format);
            opts.part_size = Some(1_200);
            let written = encode_file("a.bin", &data, &opts).unwrap();
            // `cat a.bin.part*.uue > all.uue`, with a mail's chatter between two of them.
            let mut all = Vec::new();
            for part in &written {
                all.extend_from_slice(b"\nSubject: more of the same\n\n");
                all.extend_from_slice(&part.bytes);
            }
            let read = decode_files(&[Input {
                name: "all.txt",
                bytes: &all,
            }])
            .unwrap();
            assert_eq!(read.len(), 1, "{format:?}");
            assert_eq!(read[0].name, "a.bin");
            assert_eq!(read[0].parts, 5);
            assert_eq!(read[0].bytes, data);
        }
    }

    #[test]
    fn a_yenc_file_in_parts_is_held_against_its_checksums() {
        let data = bytes(3_000);
        let mut opts = options(Format::Yenc);
        opts.part_size = Some(1_000);
        let written = encode_file("a.bin", &data, &opts).unwrap();
        assert!(decode_files(&inputs(&written)).unwrap()[0].checked);

        // The second part swapped for the second part of another file of the same name
        // and size: each part matches its own CRC, and the whole does not.
        let other: Vec<u8> = data.iter().map(|b| b ^ 0x55).collect();
        let foreign = encode_file("a.bin", &other, &opts).unwrap();
        let mut mixed = written.clone();
        // Its own claim about the whole is kept, so that only the bytes differ.
        let own_crc = format!("crc32={:08x}", crc32fast::hash(&data));
        let their_crc = format!("crc32={:08x}", crc32fast::hash(&other));
        let text = foreign[1].bytes.clone();
        let at = text
            .windows(their_crc.len())
            .rposition(|w| w == their_crc.as_bytes())
            .unwrap();
        let mut patched = text.clone();
        patched[at..at + own_crc.len()].copy_from_slice(own_crc.as_bytes());
        mixed[1].bytes = patched;
        assert!(matches!(
            decode_files(&inputs(&mixed)),
            Err(Error::Checksum { .. })
        ));

        // A part that says it starts somewhere else.
        let mut shifted = written.clone();
        let moved =
            String::from_utf8_lossy(&shifted[2].bytes[..80]).replace("begin=2001", "begin=2002");
        shifted[2].bytes.splice(..80, moved.into_bytes());
        assert!(matches!(
            decode_files(&inputs(&shifted)),
            Err(Error::Malformed { .. })
        ));

        // And only two of three: the header says how many there should be.
        assert_eq!(
            decode_files(&inputs(&written[..2])),
            Err(Error::Parts {
                name: "a.bin".into(),
                missing: 3
            })
        );
    }

    #[test]
    fn what_a_file_is_is_read_from_what_is_in_it() {
        let cases: [(&[u8], Option<Format>); 16] = [
            (b"begin 644 cat.txt\n#0V%T\n`\nend\n", Some(Format::Uu)),
            (b"begin 644 cat.txt\n1Eq3o\n+\nend\n", Some(Format::Xx)),
            (
                b"=ybegin line=128 size=3 name=a\r\n\x8b\x8c\x8d\r\n=yend size=3\r\n",
                Some(Format::Yenc),
            ),
            (
                b"-----BEGIN CERTIFICATE-----\naGVsbG8sIHdvcmxkIQ==\n-----END CERTIFICATE-----\n",
                Some(Format::Base64),
            ),
            (b"data:text/plain;base64,aGVsbG8sIHdvcmxkIQ==", Some(Format::Base64)),
            (b"aGVsbG8sIHdvcmxkIQ==\n", Some(Format::Base64)),
            (b"48656c6c6f2c20776f726c6421\n", Some(Format::Hex)),
            (b"48 65 6c 6c 6f 2c 20 77\n6f 72 6c 64 21 0a 00 ff\n", Some(Format::Hex)),
            (b"JBSWY3DPFQQHO33SNRSCC===\n", Some(Format::Base32)),
            (
                b"Content-Type: text/plain; name=\"note.txt\"\r\nContent-Transfer-Encoding: base64\r\n\r\naGVsbG8=\r\n",
                Some(Format::Base64),
            ),
            (
                b"a long line that an encoder folded with a soft break at its e=\nnd, and caf=C3=A9\n",
                Some(Format::Qp),
            ),
            (b"hello%2C%20world%21\n", Some(Format::Url)),
            // And what is none of them.
            (b"This is a sentence, not a payload.\n", None),
            (b"PORT=80\nNAME=cafe\n", None),
            (b"deadbeef\n", None),
            (b"", None),
        ];
        for (bytes, expected) in cases {
            assert_eq!(
                detect(None, bytes),
                expected,
                "{}",
                String::from_utf8_lossy(bytes)
            );
        }
    }

    #[test]
    fn the_name_decides_only_between_two_things_the_bytes_could_both_be() {
        // Sixteen hex digits are hex by themselves, and base64 when the name says so.
        let hexish = b"deadbeefdeadbeef\n";
        assert_eq!(detect(Some("x.bin"), hexish), Some(Format::Hex));
        assert_eq!(detect(Some("x.b64"), hexish), Some(Format::Base64));
        // Plain text with an equals sign in it is quoted-printable only by name.
        let text = b"caf=C3=A9\n";
        assert_eq!(detect(Some("note.txt"), text), None);
        assert_eq!(detect(Some("note.qp"), text), Some(Format::Qp));
        // And a short one is base32 by name where the alphabet alone is not enough.
        assert_eq!(detect(Some("k.b32"), b"MZXW6==="), Some(Format::Base32));
        assert_eq!(detect(None, b"MZXW6==="), None);
        // But a name never makes a file what its bytes are not.
        assert_eq!(detect(Some("prose.hex"), b"not hex at all\n"), None);
        assert_eq!(
            detect(Some("a.b64"), b"begin 644 a\n#0V%T\n`\nend\n"),
            Some(Format::Uu)
        );
    }

    #[test]
    fn the_name_a_file_carries_is_the_name_it_is_written_under_and_never_a_path() {
        let read = |name: &str, bytes: &[u8]| {
            decode_files(&[Input { name, bytes }])
                .unwrap()
                .remove(0)
                .name
        };
        assert_eq!(
            read("x.uue", b"begin 644 cat.txt\n#0V%T\n`\nend\n"),
            "cat.txt"
        );
        // A path in the file is not obeyed: its last component is the name.
        assert_eq!(
            read("x.uue", b"begin 644 ../../etc/passwd\n#0V%T\n`\nend\n"),
            "passwd"
        );
        assert_eq!(
            read("x.uue", b"begin 644 C:\\Windows\\win.ini\n#0V%T\n`\nend\n"),
            "win.ini"
        );
        assert_eq!(read("x.uue", b"begin 644 ..\n#0V%T\n`\nend\n"), "x");
        // A MIME part's file name.
        let mime = b"Content-Type: application/octet-stream\nContent-Disposition: attachment; filename=\"photo one.jpg\"\nContent-Transfer-Encoding: base64\n\naGVsbG8=\n";
        assert_eq!(read("part.eml", mime), "photo one.jpg");
        // No name inside: the file's own, without what says it is encoded.
        assert_eq!(read("token.b64", b"aGVsbG8sIHdvcmxkIQ=="), "token");
        assert_eq!(
            read("token.tar.gz.B64", b"aGVsbG8sIHdvcmxkIQ=="),
            "token.tar.gz"
        );
        // And one whose name says nothing is not written over itself.
        assert_eq!(read("key.pem", b"aGVsbG8sIHdvcmxkIQ=="), "key.pem.decoded");
        assert_eq!(carried_name("a.bin.part03.uue"), "a.bin");
        assert_eq!(
            part_of("a.bin.Part03.UUE"),
            Some(("a.bin".to_string(), 3, "uue".to_string()))
        );
        assert_eq!(part_of("a.bin.uue"), None);
        assert_eq!(part_of("a.part3.txt"), None);
        assert_eq!(part_of(".part3.uue"), None);
    }

    #[test]
    fn several_files_in_one_come_out_as_several() {
        let mut both = uu::encode_section("one.txt", b"first", false, 45);
        both.extend(uu::encode_section("two.txt", b"second", false, 45));
        let read = decode_files(&[Input {
            name: "both.uue",
            bytes: &both,
        }])
        .unwrap();
        let names: Vec<&str> = read.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, ["one.txt", "two.txt"]);
        assert_eq!(read[0].bytes, b"first");
        assert_eq!(read[1].bytes, b"second");
    }

    #[test]
    fn what_is_not_encoded_and_what_is_damaged_both_give_a_reason_and_no_file() {
        let prose = Input {
            name: "notes.txt",
            bytes: b"just some prose\n",
        };
        assert_eq!(decode_files(&[prose]), Err(Error::Unknown));
        assert_eq!(decode_files(&[]), Err(Error::Unknown));
        let cut = Input {
            name: "x.uue",
            bytes: b"begin 644 cat.txt\n#0V%T\n",
        };
        assert!(matches!(decode_files(&[cut]), Err(Error::Malformed { .. })));
        let damaged = Input {
            name: "x.yenc",
            bytes: b"=ybegin line=128 size=3 name=a\n\x8b\x8c\x8d\n=yend size=3 crc32=00000000\n",
        };
        assert!(matches!(
            decode_files(&[damaged]),
            Err(Error::Checksum { .. })
        ));
    }
}
