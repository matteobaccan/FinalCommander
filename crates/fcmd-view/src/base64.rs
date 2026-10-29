//! Base64, for a file that is one (SPEC 05 §1.13).
//!
//! A certificate, a key, an attachment pulled out of a mail, a `data:` URI pasted into a
//! file: all of them are bytes somebody wrote down in sixty-four printable characters, and
//! all of them are unreadable until something decodes them. The viewer is where one looks at
//! a file one did not write, so this is where the decoding belongs.
//!
//! Three shapes are read, because all three turn up:
//!
//! - the bare payload, with or without padding and with or without line breaks;
//! - PEM armour — `-----BEGIN CERTIFICATE-----`, the payload, `-----END …-----` — where the
//!   armour lines are skipped and the payload between them is what is decoded;
//! - a `data:` URI, where everything up to and including `;base64,` is a label rather than
//!   data.
//!
//! Both alphabets are accepted. The standard one ends `+/` and the URL-safe one `-_`, they
//! never appear in the same file, and refusing the second would mean refusing every token a
//! web application ever issued.

/// The characters a payload may hold, beyond the letters and the digits.
const STANDARD: (u8, u8) = (b'+', b'/');
const URL_SAFE: (u8, u8) = (b'-', b'_');

/// Why a file is not base64, in the words the viewer puts on the screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A character that is in neither alphabet, with the byte offset it was found at — the
    /// one thing worth knowing, because it says *where* the file stops being base64.
    NotBase64 { at: usize, byte: u8 },
    /// The payload's length cannot be right: base64 encodes three bytes in four characters,
    /// so a tail of exactly one character encodes nothing.
    Truncated,
    /// Nothing to decode: an empty file, or armour with no payload inside it.
    Empty,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::NotBase64 { at, byte } => {
                write!(f, "byte {at} is {byte:#04x}, which is not base64")
            }
            Error::Truncated => f.write_str("the base64 payload stops in the middle of a group"),
            Error::Empty => f.write_str("there is no base64 in this file"),
        }
    }
}

/// Decode a whole file's worth of base64.
///
/// Whitespace is skipped wherever it falls, which is what every encoder's line wrapping
/// needs; everything else has to be the payload. A file that is *mostly* base64 is not
/// decoded half way — the answer would be bytes that are not what anybody wrote.
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let payload = payload_of(input);
    if payload.is_empty() {
        return Err(Error::Empty);
    }
    // Which alphabet, decided once by what the payload actually holds: a file with both
    // `+` and `-` in it is not base64 in either, and falls out of the loop below as the
    // character that is in neither.
    let alphabet = if payload
        .iter()
        .any(|(_, b)| *b == URL_SAFE.0 || *b == URL_SAFE.1)
    {
        URL_SAFE
    } else {
        STANDARD
    };

    let mut out = Vec::with_capacity(payload.len() / 4 * 3);
    let mut group: u32 = 0;
    let mut have = 0u32;
    let mut padded = false;
    for (at, byte) in payload {
        if byte == b'=' {
            // Padding only ever closes a group, and nothing follows it but more padding.
            padded = true;
            continue;
        }
        if padded {
            return Err(Error::NotBase64 { at, byte });
        }
        let six = match sextet(byte, alphabet) {
            Some(six) => six,
            None => return Err(Error::NotBase64 { at, byte }),
        };
        group = (group << 6) | u32::from(six);
        have += 6;
        if have == 24 {
            out.extend_from_slice(&group.to_be_bytes()[1..]);
            group = 0;
            have = 0;
        }
    }
    // What is left of an unclosed group: 18 bits are two bytes, 12 bits are one, and 6 bits
    // are a single character that encodes nothing at all.
    match have {
        0 => {}
        12 => out.push((group >> 4) as u8),
        18 => {
            out.push((group >> 10) as u8);
            out.push((group >> 2) as u8);
        }
        _ => return Err(Error::Truncated),
    }
    Ok(out)
}

/// Would [`decode`] read this file? The same walk, without the bytes it would produce.
///
/// `min` is how much payload is worth calling base64: three characters that happen to be
/// letters are not a file somebody encoded, and offering to decode every short text file
/// would make the key look broken.
pub fn reads(input: &[u8], min: usize) -> bool {
    let payload = payload_of(input);
    if payload.len() < min {
        return false;
    }
    decode(input).is_ok()
}

/// The characters to decode: the armour and the `data:` label taken off, whitespace dropped,
/// each one with the offset it had in the file so an error can point at it.
fn payload_of(input: &[u8]) -> Vec<(usize, u8)> {
    let mut out = Vec::with_capacity(input.len());
    let mut start = 0;
    // A `data:` URI is a label and then the payload. Only the first line can carry one, and
    // only up to `;base64,`.
    if input.starts_with(b"data:") {
        if let Some(at) = find(input, b";base64,") {
            start = at + b";base64,".len();
        }
    }
    let mut at = start;
    for line in split_lines(&input[start..]) {
        let line_start = at;
        at += line.len() + 1;
        // PEM armour: `-----BEGIN …-----`. The dashes are not base64 in either alphabet, so
        // this is the one place they are allowed, and only at the start of a line.
        if line.starts_with(b"-----") {
            continue;
        }
        for (i, byte) in line.iter().enumerate() {
            if !byte.is_ascii_whitespace() {
                out.push((line_start + i, *byte));
            }
        }
    }
    out
}

fn split_lines(input: &[u8]) -> impl Iterator<Item = &[u8]> {
    input.split(|b| *b == b'\n').map(|line| match line {
        [rest @ .., b'\r'] => rest,
        whole => whole,
    })
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn sextet(byte: u8, alphabet: (u8, u8)) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b if b == alphabet.0 => Some(62),
        b if b == alphabet.1 => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_shapes_a_base64_file_comes_in() {
        // The bare payload, padded and not, wrapped and not.
        assert_eq!(decode(b"aGVsbG8=").unwrap(), b"hello");
        assert_eq!(decode(b"aGVsbG8").unwrap(), b"hello");
        assert_eq!(decode(b"aGVs\nbG8=\n").unwrap(), b"hello");
        assert_eq!(decode(b"aGVsbG8sIHdvcmxk").unwrap(), b"hello, world");

        // PEM armour, which is how a certificate or a key is written down.
        let pem = b"-----BEGIN CERTIFICATE-----\naGVsbG8=\n-----END CERTIFICATE-----\n";
        assert_eq!(decode(pem).unwrap(), b"hello");

        // A `data:` URI, which is how one lands in a file pasted from a browser.
        assert_eq!(
            decode(b"data:text/plain;base64,aGVsbG8=").unwrap(),
            b"hello"
        );
    }

    #[test]
    fn both_alphabets_and_neither() {
        // `+/` is the standard alphabet; `-_` is what a URL or a token uses.
        assert_eq!(decode(b"++//").unwrap(), [0xfb, 0xef, 0xff]);
        assert_eq!(decode(b"--__").unwrap(), [0xfb, 0xef, 0xff]);
        // One file cannot be both.
        assert!(matches!(
            decode(b"+-//"),
            Err(Error::NotBase64 { at: 0, byte: b'+' })
        ));
    }

    #[test]
    fn what_is_not_base64_says_where_it_stops_being_base64() {
        assert_eq!(decode(b""), Err(Error::Empty));
        assert_eq!(decode(b"   \n\n"), Err(Error::Empty));
        assert_eq!(decode(b"aGVsbG8="), Ok(b"hello".to_vec()));
        // A single character left over encodes nothing.
        assert_eq!(decode(b"aGVsbG8sIHdvcmxkZ"), Err(Error::Truncated));
        // And a character in neither alphabet is named, with where it is.
        assert_eq!(
            decode(b"aGVs!G8="),
            Err(Error::NotBase64 { at: 4, byte: b'!' })
        );
        // Padding closes the payload: nothing may follow it.
        assert!(matches!(decode(b"aGVs=bG8"), Err(Error::NotBase64 { .. })));
    }

    #[test]
    fn reads_says_no_to_a_file_that_merely_has_letters_in_it() {
        assert!(reads(b"aGVsbG8sIHdvcmxkIQ==", 8));
        // Too short to be worth calling base64.
        assert!(!reads(b"aGVs", 8));
        // A plain English sentence has spaces and a full stop in it.
        assert!(!reads(b"This file is just some prose.\n", 8));
        // And one that is all letters is still not base64 when its length cannot be.
        assert!(!reads(b"abcdefghijklmnopq", 8));
    }

    #[test]
    fn every_byte_value_survives_the_round_trip() {
        // The decoder against an encoding written here, so the test does not check the
        // decoder against itself.
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let bytes: Vec<u8> = (0..=255u8).collect();
        let mut text = String::new();
        for chunk in bytes.chunks(3) {
            let mut group = 0u32;
            for (i, byte) in chunk.iter().enumerate() {
                group |= u32::from(*byte) << (16 - 8 * i);
            }
            let chars = chunk.len() + 1;
            for i in 0..4 {
                if i < chars {
                    text.push(ALPHABET[((group >> (18 - 6 * i)) & 0x3f) as usize] as char);
                } else {
                    text.push('=');
                }
            }
        }
        assert_eq!(decode(text.as_bytes()).unwrap(), bytes);
    }
}
