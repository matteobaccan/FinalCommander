//! yEnc: every byte plus forty-two, and only the four that a news server cannot carry
//! escaped — which is why it is two per cent larger than the file where base64 is a third.
//!
//! It is also the only one of these formats that says what it should come out as: the size
//! in `=ybegin`, the CRC32 in `=yend`, and for a file posted in pieces the byte range and
//! the CRC of each (`=ypart`, `pcrc32`). All of them are checked here, because a format
//! that carries a checksum nobody reads might as well not carry it.

use super::{Error, Format};

/// What every encoder writes; the format's own draft says 128 and allows up to 997.
pub const DEFAULT_LINE: usize = 128;

fn bad(why: impl Into<String>) -> Error {
    Error::Malformed {
        format: Format::Yenc,
        why: why.into(),
    }
}

/// The lines of the body, `\r\n` after each as the format is written.
pub fn encode_body(data: &[u8], line: usize, out: &mut Vec<u8>) {
    let line = if line == 0 {
        DEFAULT_LINE
    } else {
        line.clamp(16, 997)
    };
    let mut col = 0usize;
    for (i, byte) in data.iter().enumerate() {
        let coded = byte.wrapping_add(42);
        let last = col + 1 >= line || i + 1 == data.len();
        let escape = match coded {
            0 | b'\n' | b'\r' | b'=' => true,
            // White space at either end of a line is what gets trimmed on the way, and a
            // dot at the start of one is what NNTP doubles.
            b'\t' | b' ' => col == 0 || last,
            b'.' => col == 0,
            _ => false,
        };
        if escape {
            out.push(b'=');
            out.push(coded.wrapping_add(64));
            col += 2;
        } else {
            out.push(coded);
            col += 1;
        }
        if col >= line {
            out.extend_from_slice(b"\r\n");
            col = 0;
        }
    }
    if col > 0 {
        out.extend_from_slice(b"\r\n");
    }
}

/// Where one piece of a file sits in the whole: its number and how many there are, from
/// one, and the offset its bytes start at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Place {
    pub part: usize,
    pub total: usize,
    pub offset: u64,
}

/// A whole article: `=ybegin`, `=ypart` when it is one piece of several, the body, `=yend`
/// with the size and the CRC32 of what the body holds — and, on a piece, the CRC of the
/// whole file too, so the join can be checked from any one of them.
pub fn encode(
    name: &str,
    data: &[u8],
    line: usize,
    whole_size: u64,
    whole_crc: u32,
    place: Option<Place>,
) -> Vec<u8> {
    let line = if line == 0 {
        DEFAULT_LINE
    } else {
        line.clamp(16, 997)
    };
    let mut out = Vec::with_capacity(data.len() + data.len() / 40 + name.len() + 128);
    let crc = crc32fast::hash(data);
    match place {
        Some(place) => {
            out.extend_from_slice(
                format!(
                    "=ybegin part={} total={} line={line} size={whole_size} name={name}\r\n",
                    place.part, place.total
                )
                .as_bytes(),
            );
            // The range is counted from one and names its last byte, as the format has it.
            out.extend_from_slice(
                format!(
                    "=ypart begin={} end={}\r\n",
                    place.offset + 1,
                    place.offset + data.len() as u64
                )
                .as_bytes(),
            );
            encode_body(data, line, &mut out);
            out.extend_from_slice(
                format!(
                    "=yend size={} part={} pcrc32={crc:08x} crc32={whole_crc:08x}\r\n",
                    data.len(),
                    place.part
                )
                .as_bytes(),
            );
        }
        None => {
            out.extend_from_slice(
                format!("=ybegin line={line} size={whole_size} name={name}\r\n").as_bytes(),
            );
            encode_body(data, line, &mut out);
            out.extend_from_slice(
                format!("=yend size={} crc32={crc:08x}\r\n", data.len()).as_bytes(),
            );
        }
    }
    out
}

/// One `=ybegin` … `=yend` of a file, already checked against what its own `=yend` says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub name: String,
    /// The piece's number, when the article is one of several.
    pub part: Option<usize>,
    pub total: Option<usize>,
    /// Where its bytes start in the whole, from nothing.
    pub offset: Option<u64>,
    /// How long the whole file is.
    pub size: Option<u64>,
    /// The CRC32 of the whole file, when the article says it.
    pub crc: Option<u32>,
    pub bytes: Vec<u8>,
    /// Whether a CRC of these bytes was there to check — and matched, or the read failed.
    pub checked: bool,
}

/// `key=value` out of a control line. `name` is the last one and runs to the end of the
/// line, spaces and all; the others end at the next space.
fn field<'a>(line: &'a [u8], key: &str) -> Option<&'a [u8]> {
    let pattern = format!(" {key}=");
    let at = line
        .windows(pattern.len())
        .position(|w| w == pattern.as_bytes())?;
    let value = &line[at + pattern.len()..];
    if key == "name" {
        return Some(value);
    }
    let end = value.iter().position(|b| *b == b' ').unwrap_or(value.len());
    Some(&value[..end])
}

fn number(line: &[u8], key: &str) -> Option<u64> {
    std::str::from_utf8(field(line, key)?).ok()?.parse().ok()
}

fn crc_field(line: &[u8], key: &str) -> Option<u32> {
    u32::from_str_radix(std::str::from_utf8(field(line, key)?).ok()?, 16).ok()
}

/// Everything before `name=` on a `=ybegin` line: a name may itself hold ` size=` and must
/// not be read as one.
fn before_name(line: &[u8]) -> &[u8] {
    let at = line
        .windows(6)
        .position(|w| w == b" name=")
        .unwrap_or(line.len());
    &line[..at]
}

pub fn has_begin(input: &[u8]) -> bool {
    input.starts_with(b"=ybegin ")
        || input[..input.len().min(64 * 1024)]
            .windows(9)
            .any(|w| w == b"\n=ybegin ")
}

/// Every article of a file, in the order they are in, each checked against its own
/// `=yend`: the size, and the CRC32 when there is one. What stands between them is read
/// past, as it is in the news posting such a file is cut from.
pub fn articles(input: &[u8]) -> Result<Vec<Article>, Error> {
    let mut out = Vec::new();
    let mut open: Option<Article> = None;
    for line in input.split(|b| *b == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        let Some(article) = &mut open else {
            if line.starts_with(b"=ybegin ") {
                let name = field(line, "name")
                    .map(|n| String::from_utf8_lossy(n).trim().to_string())
                    .filter(|n| !n.is_empty())
                    .ok_or_else(|| bad("an =ybegin line with no name"))?;
                let head = before_name(line);
                open = Some(Article {
                    name,
                    part: number(head, "part").map(|n| n as usize),
                    total: number(head, "total").map(|n| n as usize),
                    offset: None,
                    size: number(head, "size"),
                    crc: None,
                    bytes: Vec::new(),
                    checked: false,
                });
            }
            continue;
        };
        if line.starts_with(b"=ypart ") {
            article.offset = number(line, "begin").map(|begin| begin.saturating_sub(1));
            continue;
        }
        if !line.starts_with(b"=yend") {
            let mut bytes = line.iter();
            while let Some(byte) = bytes.next() {
                let coded = if *byte == b'=' {
                    bytes
                        .next()
                        .ok_or_else(|| bad("a line that ends on an escape"))?
                        .wrapping_sub(64)
                } else {
                    *byte
                };
                article.bytes.push(coded.wrapping_sub(42));
            }
            continue;
        }
        let mut article = open.take().expect("matched as open");
        let found = article.bytes.len() as u64;
        if let Some(expected) = number(line, "size") {
            if expected != found {
                return Err(Error::Size {
                    name: article.name,
                    expected,
                    found,
                });
            }
        }
        // On a piece `pcrc32` is the piece's and `crc32` the whole file's; on a file in one
        // article `crc32` is both.
        let whole = crc_field(line, "crc32");
        let own = if article.part.is_some() {
            crc_field(line, "pcrc32")
        } else {
            whole
        };
        if let Some(expected) = own {
            let found = crc32fast::hash(&article.bytes);
            if expected != found {
                return Err(Error::Checksum {
                    name: article.name,
                    expected,
                    found,
                });
            }
            article.checked = true;
        }
        article.crc = whole;
        out.push(article);
    }
    if let Some(article) = open {
        return Err(bad(format!(
            "{} has no =yend line: the file stops before it does",
            article.name
        )));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 31 + i / 256) as u8).collect()
    }

    #[test]
    fn every_byte_comes_back_at_every_line_length() {
        for len in [0, 1, 127, 128, 129, 256, 5000] {
            for line in [0, 16, 128, 997] {
                let data = bytes(len);
                let coded = encode("blob.bin", &data, line, len as u64, 0, None);
                let read = articles(&coded).unwrap();
                assert_eq!(read.len(), 1);
                assert_eq!(read[0].name, "blob.bin");
                assert_eq!(read[0].bytes, data, "{len} at {line}");
                assert_eq!(read[0].size, Some(len as u64));
                assert_eq!(read[0].crc, Some(crc32fast::hash(&data)));
                assert!(read[0].checked);
                assert_eq!(read[0].part, None);
                // No line is longer than asked, give or take the escape that may end it.
                let max = if line == 0 { DEFAULT_LINE } else { line } + 1;
                for row in coded
                    .split(|b| *b == b'\n')
                    .filter(|r| !r.starts_with(b"=y"))
                {
                    assert!(row.len() <= max + 1, "{} > {max}", row.len());
                }
            }
        }
    }

    #[test]
    fn what_a_line_cannot_carry_is_escaped() {
        // 0xd6 + 42 is NUL, 0xe0 is LF, 0xe3 is CR, 0x13 is `=`; 0x04 is a dot and 0xf6 a
        // space, which only matter where they fall.
        let data = [0xd6, 0xe0, 0xe3, 0x13, 0x04, 0xf6, 0x04, 0xf6];
        let mut body = Vec::new();
        encode_body(&data, 128, &mut body);
        assert_eq!(body, b"=@=J=M=}. .=`\r\n");
        // The same dot at the start of a line is escaped, and so is a space there.
        let mut body = Vec::new();
        encode_body(&[0x04, 0x00, 0xf6], 128, &mut body);
        assert_eq!(body, b"=n*=`\r\n");
        assert!(!body.contains(&0));
    }

    #[test]
    fn a_piece_says_where_it_goes_and_a_damaged_one_is_caught() {
        let whole = bytes(300);
        let crc = crc32fast::hash(&whole);
        let place = Place {
            part: 2,
            total: 3,
            offset: 100,
        };
        let coded = encode("a b.bin", &whole[100..200], 128, 300, crc, Some(place));
        let text = String::from_utf8_lossy(&coded).into_owned();
        assert!(text.starts_with("=ybegin part=2 total=3 line=128 size=300 name=a b.bin\r\n"));
        assert!(text.contains("=ypart begin=101 end=200\r\n"));
        let read = articles(&coded).unwrap();
        assert_eq!(read[0].name, "a b.bin");
        assert_eq!((read[0].part, read[0].total), (Some(2), Some(3)));
        assert_eq!(read[0].offset, Some(100));
        assert_eq!(read[0].size, Some(300));
        assert_eq!(read[0].crc, Some(crc));
        assert_eq!(read[0].bytes, &whole[100..200]);

        // One byte of the body changed: the piece's own CRC says so.
        let mut damaged = coded.clone();
        let at = text.find("=ypart").unwrap() + 30;
        damaged[at] = if damaged[at] == b'A' { b'B' } else { b'A' };
        assert!(matches!(articles(&damaged), Err(Error::Checksum { .. })));
        // A line gone: the size says so before the CRC is looked at.
        let mut short = coded.clone();
        let at = coded
            .windows(13)
            .position(|w| w == b"size=100 part")
            .unwrap();
        short[at + 5..at + 8].copy_from_slice(b"099");
        assert!(matches!(
            articles(&short),
            Err(Error::Size {
                expected: 99,
                found: 100,
                ..
            })
        ));
        // And one that stops before its end is not a file.
        let cut = &coded[..coded.len() - 50];
        assert!(articles(cut).is_err());
    }

    #[test]
    fn a_name_is_not_read_for_the_numbers_in_it_and_unix_line_ends_are_read() {
        let coded = b"junk before\n=ybegin line=128 size=3 name=x size=9 part=4.bin\n\x8b\x8c\x8d\n=yend size=3\n";
        let read = articles(coded).unwrap();
        assert_eq!(read[0].name, "x size=9 part=4.bin");
        assert_eq!(read[0].size, Some(3));
        assert_eq!(read[0].part, None);
        assert_eq!(read[0].bytes, b"abc");
        assert!(!read[0].checked, "there was no CRC to check");
        assert!(has_begin(coded));
        assert!(!has_begin(b"about =ybegin lines\n"));
    }
}
