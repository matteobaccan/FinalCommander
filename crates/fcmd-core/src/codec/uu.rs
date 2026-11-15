//! UUencode and XXencode: three bytes in four characters, a count in front of every line,
//! and `begin 644 name` … `end` round the lot.
//!
//! The two differ only in the sixty-four characters they use. UU's run from the space to
//! the underscore, which is every punctuation mark a gateway ever mangled; XX's are `+`,
//! `-`, the digits and the letters, which was the answer to that. A file says which it is by
//! what its lines are made of, never by its name.

use super::{Error, Format};

const XX: &[u8; 64] = b"+-0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// How many bytes a full line carries. Forty-five is what every encoder writes and the
/// most a count character can say in the readers that take the format literally.
pub const FULL_LINE: usize = 45;

fn character(value: u8, xx: bool) -> u8 {
    match (xx, value) {
        (true, value) => XX[usize::from(value & 0x3f)],
        // A zero is written as a backquote rather than a space: the space is the one
        // character that does not survive being at the end of a line.
        (false, 0) => b'`',
        (false, value) => (value & 0x3f) + 32,
    }
}

fn value(byte: u8, xx: bool) -> Option<u8> {
    if xx {
        XX.iter().position(|c| *c == byte).map(|at| at as u8)
    } else {
        (0x20..=0x60).contains(&byte).then(|| (byte - 32) & 0x3f)
    }
}

/// The lines of the body: `per_line` bytes in each, the last one shorter.
pub fn encode_body(data: &[u8], xx: bool, per_line: usize, out: &mut Vec<u8>) {
    let per_line = per_line.clamp(3, FULL_LINE) / 3 * 3;
    for row in data.chunks(per_line) {
        out.push(character(row.len() as u8, xx));
        for triple in row.chunks(3) {
            let mut group = 0u32;
            for (i, byte) in triple.iter().enumerate() {
                group |= u32::from(*byte) << (16 - 8 * i);
            }
            for i in 0..4 {
                out.push(character(((group >> (18 - 6 * i)) & 0x3f) as u8, xx));
            }
        }
        out.push(b'\n');
    }
}

/// A whole section: the `begin` line with the name, the body, the empty line that closes
/// it, and `end`.
pub fn encode_section(name: &str, data: &[u8], xx: bool, per_line: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() / 3 * 4 + data.len() / 45 * 2 + name.len() + 32);
    out.extend_from_slice(format!("begin 644 {name}\n").as_bytes());
    encode_body(data, xx, per_line, &mut out);
    out.push(character(0, xx));
    out.extend_from_slice(b"\nend\n");
    out
}

/// One line of a body, onto `out`. A UU line may be shorter than its count says — the
/// spaces at its end are what mail took away, and they stood for zeros; a longer one, or a
/// character outside the alphabet, is not a line of this format.
fn decode_line(line: &[u8], xx: bool, out: &mut Vec<u8>) -> Result<(), &'static str> {
    let Some((first, chars)) = line.split_first() else {
        return Ok(());
    };
    let count = usize::from(value(*first, xx).ok_or("a count that is not in the alphabet")?);
    let need = count.div_ceil(3) * 4;
    if chars.len() > need {
        return Err("a line longer than its count says");
    }
    if chars.len() < need && xx {
        return Err("a line shorter than its count says");
    }
    let mut written = 0;
    for at in (0..need).step_by(4) {
        let mut group = 0u32;
        for i in 0..4 {
            let six = match chars.get(at + i) {
                Some(byte) => value(*byte, xx).ok_or("a character that is not in the alphabet")?,
                None => 0,
            };
            group = (group << 6) | u32::from(six);
        }
        for byte in &group.to_be_bytes()[1..] {
            if written < count {
                out.push(*byte);
                written += 1;
            }
        }
    }
    Ok(())
}

fn decode_lines(lines: &[&[u8]], xx: bool) -> Result<Vec<u8>, (usize, &'static str)> {
    let mut out = Vec::with_capacity(lines.len() * FULL_LINE);
    for (n, line) in lines.iter().enumerate() {
        decode_line(line, xx, &mut out).map_err(|why| (n, why))?;
    }
    Ok(out)
}

/// The body in whichever of the two alphabets reads it. The line that closes an XX body is
/// a lone `+`, so that one is tried as XX first; anything else as UU first — a file of
/// small letters cannot be UU, and one of punctuation cannot be XX.
fn decode_either(lines: &[&[u8]]) -> Result<(Vec<u8>, bool), (usize, &'static str)> {
    let xx_first = lines.last().is_some_and(|last| *last == b"+");
    match decode_lines(lines, xx_first) {
        Ok(bytes) => Ok((bytes, xx_first)),
        Err(first) => match decode_lines(lines, !xx_first) {
            Ok(bytes) => Ok((bytes, !xx_first)),
            // The refusal of the reading that was the likelier one.
            Err(_) => Err(first),
        },
    }
}

/// One `begin` … `end` of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub name: String,
    pub bytes: Vec<u8>,
    pub xx: bool,
}

fn lines_of(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    input
        .split(|b| *b == b'\n')
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
}

/// The name on a `begin` line, if this is one: `begin`, a mode of three or four octal
/// digits, and the name, which is everything after and may have spaces in it.
pub fn begin_name(line: &[u8]) -> Option<String> {
    let rest = line.strip_prefix(b"begin ")?;
    let space = rest.iter().position(|b| *b == b' ')?;
    let (mode, name) = (&rest[..space], &rest[space + 1..]);
    if !(3..=4).contains(&mode.len()) || !mode.iter().all(|b| (b'0'..=b'7').contains(b)) {
        return None;
    }
    let name = String::from_utf8_lossy(name).trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Is there a `begin` line in the first lines of this file? Prose may come before it — a
/// mail's headers, a note from whoever posted it — but not pages of it.
pub fn has_begin(input: &[u8]) -> bool {
    lines_of(&input[..input.len().min(64 * 1024)])
        .take(400)
        .any(|line| begin_name(line).is_some())
}

/// Every section of a file, in the order they are in. What stands between them is read
/// past; a section that does not close, or a line of one that is in neither alphabet, is
/// the whole file refused — half of an attachment is not something to write to disk.
pub fn sections(input: &[u8]) -> Result<Vec<Section>, Error> {
    let bad = |format: Format, why: String| Error::Malformed { format, why };
    let mut out = Vec::new();
    let mut open: Option<(String, usize, Vec<&[u8]>)> = None;
    for (n, line) in lines_of(input).enumerate() {
        match &mut open {
            None => {
                if let Some(name) = begin_name(line) {
                    open = Some((name, n + 1, Vec::new()));
                }
            }
            Some((_, _, body)) if line != b"end" => body.push(line),
            Some(_) => {
                let (name, first, body) = open.take().expect("matched as open");
                let (bytes, xx) = decode_either(&body).map_err(|(at, why)| {
                    bad(Format::Uu, format!("line {}: {why}", first + at + 1))
                })?;
                out.push(Section { name, bytes, xx });
            }
        }
    }
    if let Some((name, _, _)) = open {
        return Err(bad(
            Format::Uu,
            format!("{name} has no end line: the file stops before it does"),
        ));
    }
    Ok(out)
}

/// The body alone, with no `begin` and no `end` round it: what is left of a block somebody
/// marked between the two. An `end` line, if it was marked too, closes it.
pub fn decode_bare(input: &[u8], xx: bool) -> Result<Vec<u8>, Error> {
    let lines: Vec<&[u8]> = lines_of(input).take_while(|line| *line != b"end").collect();
    decode_lines(&lines, xx).map_err(|(at, why)| Error::Malformed {
        format: if xx { Format::Xx } else { Format::Uu },
        why: format!("line {}: {why}", at + 1),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bytes(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 13 + i / 256) as u8).collect()
    }

    #[test]
    fn both_alphabets_come_back_at_every_length_and_say_which_they_are() {
        for xx in [false, true] {
            for len in [0, 1, 2, 3, 44, 45, 46, 90, 1000] {
                let data = bytes(len);
                let coded = encode_section("blob.bin", &data, xx, FULL_LINE);
                assert!(coded
                    .iter()
                    .all(|b| b.is_ascii_graphic() || *b == b' ' || *b == b'\n'));
                assert!(coded.split(|b| *b == b'\n').all(|l| l.len() <= 61));
                let read = sections(&coded).unwrap();
                assert_eq!(read.len(), 1);
                assert_eq!(read[0].name, "blob.bin");
                assert_eq!(read[0].bytes, data, "{len} bytes, xx {xx}");
                // Which alphabet is read off the lines; with nothing in it there is only the
                // closing line to go by, and that says it too.
                assert_eq!(read[0].xx, xx, "{len} bytes");
            }
        }
    }

    #[test]
    fn the_classic_uuencode_of_three_letters() {
        // What `uuencode` itself writes for "Cat".
        assert_eq!(
            encode_section("cat.txt", b"Cat", false, FULL_LINE),
            b"begin 644 cat.txt\n#0V%T\n`\nend\n"
        );
        // And the older spelling, with a space for the zero and DOS line ends.
        let old = b"begin 644 cat.txt\r\n#0V%T\r\n \r\nend\r\n";
        assert_eq!(sections(old).unwrap()[0].bytes, b"Cat");
    }

    #[test]
    fn a_shorter_line_is_a_line_that_lost_its_spaces_and_a_longer_one_is_not_uu() {
        // "\0\0\0" is three zeros: with spaces for zeros the line is `#` and four spaces,
        // and a mailer that trims them leaves the count alone.
        let trimmed = b"begin 644 z\n#\n`\nend\n";
        assert_eq!(sections(trimmed).unwrap()[0].bytes, [0, 0, 0]);
        assert!(sections(b"begin 644 z\n#0V%T0V%T\n`\nend\n").is_err());
        assert!(sections(b"begin 644 z\n#0V%T\n").is_err(), "no end line");
        assert!(sections(b"begin 644 z\n#0V~T\n`\nend\n").is_err());
    }

    #[test]
    fn several_sections_and_what_stands_between_them() {
        let mut file = b"From: somebody\n\nhere it is\n".to_vec();
        file.extend(encode_section("a b.txt", b"first", false, 45));
        file.extend_from_slice(b"\n-- \nand the other one\n");
        file.extend(encode_section("c.bin", b"second", true, 45));
        let read = sections(&file).unwrap();
        assert_eq!(read.len(), 2);
        assert_eq!((read[0].name.as_str(), read[0].xx), ("a b.txt", false));
        assert_eq!(read[0].bytes, b"first");
        assert_eq!((read[1].name.as_str(), read[1].xx), ("c.bin", true));
        assert_eq!(read[1].bytes, b"second");
        assert!(has_begin(&file));
        assert!(!has_begin(b"begin the beguine\nbegin 9 x\n"));
        assert!(sections(b"no sections here\n").unwrap().is_empty());
    }

    #[test]
    fn a_body_with_nothing_round_it_is_read_as_the_alphabet_asked_for() {
        let mut body = Vec::new();
        encode_body(b"hello, world", false, 45, &mut body);
        assert_eq!(decode_bare(&body, false).unwrap(), b"hello, world");
        let mut body = Vec::new();
        encode_body(b"hello, world", true, 45, &mut body);
        body.extend_from_slice(b"+\nend\n");
        assert_eq!(decode_bare(&body, true).unwrap(), b"hello, world");
        assert!(decode_bare(b"{{{{\n", false).is_err());
    }

    #[test]
    fn the_line_length_asked_for_is_kept_to_whole_groups() {
        let data = bytes(100);
        let mut body = Vec::new();
        encode_body(&data, false, 30, &mut body);
        assert!(body.split(|b| *b == b'\n').all(|l| l.len() <= 41));
        assert_eq!(decode_bare(&body, false).unwrap(), data);
        // Not a multiple of three, and more than a count can say.
        for per_line in [1, 31, 500] {
            let mut body = Vec::new();
            encode_body(&data, true, per_line, &mut body);
            assert_eq!(decode_bare(&body, true).unwrap(), data);
        }
    }
}
