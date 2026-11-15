//! The encodings that are one alphabet and no framing: base32, hex, percent-encoding,
//! quoted-printable — and the two that only make sense on text, HTML entities and ROT13.
//!
//! Each is a pair of functions over bytes. What a file of them looks like — the line length,
//! the extension, the parts — is [`super::files`]'s business, and what the editor does with
//! a block is [`super::encode_text`]'s.

use super::{Error, Format};

fn bad(format: Format, why: impl Into<String>) -> Error {
    Error::Malformed {
        format,
        why: why.into(),
    }
}

/// Break `payload` after every `line` characters and after the last one; a `line` of nothing
/// leaves it as one run.
pub(crate) fn wrap(payload: Vec<u8>, line: usize) -> Vec<u8> {
    if line == 0 || payload.is_empty() {
        return payload;
    }
    let mut out = Vec::with_capacity(payload.len() + payload.len() / line + 1);
    for chunk in payload.chunks(line) {
        out.extend_from_slice(chunk);
        out.push(b'\n');
    }
    out
}

// ── base32 (RFC 4648) ───────────────────────────────────────────────────────────────────

const BASE32: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";

pub fn base32_encode(data: &[u8], line: usize) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len().div_ceil(5) * 8);
    for chunk in data.chunks(5) {
        let mut group = 0u64;
        for (i, byte) in chunk.iter().enumerate() {
            group |= u64::from(*byte) << (32 - 8 * i);
        }
        // Five bytes are eight characters; a shorter tail is as many as hold its bits, and
        // `=` up to the eight.
        let chars = (chunk.len() * 8).div_ceil(5);
        for i in 0..8 {
            if i < chars {
                out.push(BASE32[((group >> (35 - 5 * i)) & 0x1f) as usize]);
            } else {
                out.push(b'=');
            }
        }
    }
    wrap(out, line)
}

/// Either case is read, and the padding may be missing: both are how base32 is typed by
/// hand, which is what it was made for.
pub fn base32_decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len() / 8 * 5);
    let mut group = 0u64;
    let mut bits = 0u32;
    let mut chars = 0usize;
    let mut padded = false;
    for (at, byte) in input.iter().enumerate() {
        if byte.is_ascii_whitespace() {
            continue;
        }
        if *byte == b'=' {
            padded = true;
            continue;
        }
        let five = match byte.to_ascii_uppercase() {
            upper @ b'A'..=b'Z' => upper - b'A',
            digit @ b'2'..=b'7' => digit - b'2' + 26,
            _ => 0xff,
        };
        if five == 0xff || padded {
            return Err(bad(
                Format::Base32,
                format!("byte {at} is {byte:#04x}, which is not base32"),
            ));
        }
        chars += 1;
        group = (group << 5) | u64::from(five);
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push((group >> bits) as u8);
            group &= (1 << bits) - 1;
        }
    }
    if chars == 0 {
        return Err(bad(Format::Base32, "there is no base32 here"));
    }
    // One, three or six characters past a group of eight hold less than a byte more than
    // the one before them: no encoder writes them.
    if matches!(chars % 8, 1 | 3 | 6) {
        return Err(bad(
            Format::Base32,
            "the base32 payload stops in the middle of a group",
        ));
    }
    Ok(out)
}

// ── hex ─────────────────────────────────────────────────────────────────────────────────

pub fn hex_encode(data: &[u8], line: usize) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = Vec::with_capacity(data.len() * 2);
    for byte in data {
        out.push(DIGITS[usize::from(byte >> 4)]);
        out.push(DIGITS[usize::from(byte & 0x0f)]);
    }
    wrap(out, line)
}

pub(crate) fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Pairs of digits in either case. What people put between them is read past — spaces, line
/// breaks, the colons of a fingerprint, the commas and `0x` of a C array — because hex is
/// written in all of those ways and means the same bytes in each.
pub fn hex_decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len() / 2);
    let mut high: Option<u8> = None;
    let mut i = 0;
    while i < input.len() {
        let byte = input[i];
        let between = byte.is_ascii_whitespace() || matches!(byte, b':' | b',' | b'-');
        if between {
            i += 1;
            continue;
        }
        if high.is_none() && byte == b'0' && matches!(input.get(i + 1), Some(b'x' | b'X')) {
            i += 2;
            continue;
        }
        let Some(value) = hex_value(byte) else {
            return Err(bad(
                Format::Hex,
                format!("byte {i} is {byte:#04x}, which is not a hex digit"),
            ));
        };
        match high.take() {
            Some(high) => out.push((high << 4) | value),
            None => high = Some(value),
        }
        i += 1;
    }
    if high.is_some() {
        return Err(bad(Format::Hex, "an odd number of hex digits"));
    }
    if out.is_empty() {
        return Err(bad(Format::Hex, "there are no hex digits here"));
    }
    Ok(out)
}

// ── percent-encoding (RFC 3986) ─────────────────────────────────────────────────────────

/// Everything but the unreserved characters is written as `%XX`: what comes out can go into
/// any part of a URL, which is the only safe answer when one does not know which part.
pub fn url_encode(data: &[u8], line: usize) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = Vec::with_capacity(data.len() * 3);
    let mut col = 0usize;
    for byte in data {
        let plain = byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~');
        let need = if plain { 1 } else { 3 };
        // A line is broken between two characters and never inside a `%XX`.
        if line > 0 && col > 0 && col + need > line {
            out.push(b'\n');
            col = 0;
        }
        if plain {
            out.push(*byte);
        } else {
            out.push(b'%');
            out.push(DIGITS[usize::from(byte >> 4)]);
            out.push(DIGITS[usize::from(byte & 0x0f)]);
        }
        col += need;
    }
    if line > 0 && col > 0 {
        out.push(b'\n');
    }
    out
}

/// `%XX` back to the byte. A line break is read past — a real one is always written `%0A`,
/// so one that is there was put in by whoever wrapped the text — and a `+` stays a `+`:
/// only a form turns it into a space, and a path with a plus in it is not a form.
pub fn url_decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        match input[i] {
            b'\r' | b'\n' => i += 1,
            b'%' => {
                let pair = input
                    .get(i + 1)
                    .and_then(|h| hex_value(*h))
                    .zip(input.get(i + 2).and_then(|l| hex_value(*l)));
                let Some((high, low)) = pair else {
                    return Err(bad(
                        Format::Url,
                        format!("the % at byte {i} is not followed by two hex digits"),
                    ));
                };
                out.push((high << 4) | low);
                i += 3;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }
    Ok(out)
}

// ── quoted-printable (RFC 2045 §6.7) ────────────────────────────────────────────────────

/// A line break is left as the line break it is — `\r\n` or `\n`, whichever the text has —
/// so a text file stays readable and comes back byte for byte; a `\r` on its own is not a
/// line break and is written `=0D`. Lines longer than `line` are folded with a soft break.
pub fn qp_encode(data: &[u8], line: usize) -> Vec<u8> {
    const DIGITS: &[u8; 16] = b"0123456789ABCDEF";
    let max = if line == 0 { 76 } else { line.clamp(8, 76) };
    let mut out = Vec::with_capacity(data.len() + data.len() / 8);
    let mut col = 0usize;
    let mut i = 0;
    let breaks_at = |i: usize| match data.get(i) {
        None | Some(b'\n') => true,
        Some(b'\r') => data.get(i + 1) == Some(&b'\n'),
        _ => false,
    };
    while i < data.len() {
        let byte = data[i];
        if byte == b'\r' && data.get(i + 1) == Some(&b'\n') {
            out.extend_from_slice(b"\r\n");
            col = 0;
            i += 2;
            continue;
        }
        if byte == b'\n' {
            out.push(b'\n');
            col = 0;
            i += 1;
            continue;
        }
        // White space at the end of a line is what mail transports eat, so there it is
        // written out in full.
        let literal = match byte {
            b'=' => false,
            b' ' | b'\t' => !breaks_at(i + 1),
            33..=126 => true,
            _ => false,
        };
        let need = if literal { 1 } else { 3 };
        // One column is kept for the `=` of a soft break.
        if col + need > max - 1 {
            out.extend_from_slice(b"=\n");
            col = 0;
        }
        if literal {
            out.push(byte);
        } else {
            out.push(b'=');
            out.push(DIGITS[usize::from(byte >> 4)]);
            out.push(DIGITS[usize::from(byte & 0x0f)]);
        }
        col += need;
        i += 1;
    }
    out
}

pub fn qp_decode(input: &[u8]) -> Result<Vec<u8>, Error> {
    let mut out = Vec::with_capacity(input.len());
    let mut i = 0;
    while i < input.len() {
        let byte = input[i];
        if byte != b'=' {
            out.push(byte);
            i += 1;
            continue;
        }
        // A soft break: `=` and the end of the line, with whatever white space a transport
        // added in between.
        let mut end = i + 1;
        while matches!(input.get(end), Some(b' ' | b'\t')) {
            end += 1;
        }
        match input.get(end) {
            None => {
                i = end;
                continue;
            }
            Some(b'\n') => {
                i = end + 1;
                continue;
            }
            Some(b'\r') if input.get(end + 1) == Some(&b'\n') => {
                i = end + 2;
                continue;
            }
            _ => {}
        }
        let pair = input
            .get(i + 1)
            .and_then(|h| hex_value(*h))
            .zip(input.get(i + 2).and_then(|l| hex_value(*l)));
        let Some((high, low)) = pair else {
            return Err(bad(
                Format::Qp,
                format!("the = at byte {i} is not followed by two hex digits"),
            ));
        };
        out.push((high << 4) | low);
        i += 3;
    }
    Ok(out)
}

// ── HTML entities ───────────────────────────────────────────────────────────────────────

/// The five characters that mean something to a parser. Everything else is left as it is:
/// a page written in UTF-8 needs no more, and `&egrave;` for every accented letter is how
/// a text becomes unreadable in its own source.
pub fn html_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + text.len() / 8);
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// The named entities one meets: the markup five, Latin-1, and the punctuation, arrows,
/// Greek and mathematics of ordinary pages. An entity that is not here is left as written
/// rather than guessed at.
const ENTITIES: &[(&str, char)] = &[
    ("amp", '&'),
    ("lt", '<'),
    ("gt", '>'),
    ("quot", '"'),
    ("apos", '\''),
    ("nbsp", '\u{a0}'),
    ("iexcl", '¡'),
    ("cent", '¢'),
    ("pound", '£'),
    ("curren", '¤'),
    ("yen", '¥'),
    ("brvbar", '¦'),
    ("sect", '§'),
    ("uml", '¨'),
    ("copy", '©'),
    ("ordf", 'ª'),
    ("laquo", '«'),
    ("not", '¬'),
    ("shy", '\u{ad}'),
    ("reg", '®'),
    ("macr", '¯'),
    ("deg", '°'),
    ("plusmn", '±'),
    ("sup2", '²'),
    ("sup3", '³'),
    ("acute", '´'),
    ("micro", 'µ'),
    ("para", '¶'),
    ("middot", '·'),
    ("cedil", '¸'),
    ("sup1", '¹'),
    ("ordm", 'º'),
    ("raquo", '»'),
    ("frac14", '¼'),
    ("frac12", '½'),
    ("frac34", '¾'),
    ("iquest", '¿'),
    ("Agrave", 'À'),
    ("Aacute", 'Á'),
    ("Acirc", 'Â'),
    ("Atilde", 'Ã'),
    ("Auml", 'Ä'),
    ("Aring", 'Å'),
    ("AElig", 'Æ'),
    ("Ccedil", 'Ç'),
    ("Egrave", 'È'),
    ("Eacute", 'É'),
    ("Ecirc", 'Ê'),
    ("Euml", 'Ë'),
    ("Igrave", 'Ì'),
    ("Iacute", 'Í'),
    ("Icirc", 'Î'),
    ("Iuml", 'Ï'),
    ("ETH", 'Ð'),
    ("Ntilde", 'Ñ'),
    ("Ograve", 'Ò'),
    ("Oacute", 'Ó'),
    ("Ocirc", 'Ô'),
    ("Otilde", 'Õ'),
    ("Ouml", 'Ö'),
    ("times", '×'),
    ("Oslash", 'Ø'),
    ("Ugrave", 'Ù'),
    ("Uacute", 'Ú'),
    ("Ucirc", 'Û'),
    ("Uuml", 'Ü'),
    ("Yacute", 'Ý'),
    ("THORN", 'Þ'),
    ("szlig", 'ß'),
    ("agrave", 'à'),
    ("aacute", 'á'),
    ("acirc", 'â'),
    ("atilde", 'ã'),
    ("auml", 'ä'),
    ("aring", 'å'),
    ("aelig", 'æ'),
    ("ccedil", 'ç'),
    ("egrave", 'è'),
    ("eacute", 'é'),
    ("ecirc", 'ê'),
    ("euml", 'ë'),
    ("igrave", 'ì'),
    ("iacute", 'í'),
    ("icirc", 'î'),
    ("iuml", 'ï'),
    ("eth", 'ð'),
    ("ntilde", 'ñ'),
    ("ograve", 'ò'),
    ("oacute", 'ó'),
    ("ocirc", 'ô'),
    ("otilde", 'õ'),
    ("ouml", 'ö'),
    ("divide", '÷'),
    ("oslash", 'ø'),
    ("ugrave", 'ù'),
    ("uacute", 'ú'),
    ("ucirc", 'û'),
    ("uuml", 'ü'),
    ("yacute", 'ý'),
    ("thorn", 'þ'),
    ("yuml", 'ÿ'),
    ("OElig", 'Œ'),
    ("oelig", 'œ'),
    ("Scaron", 'Š'),
    ("scaron", 'š'),
    ("Yuml", 'Ÿ'),
    ("fnof", 'ƒ'),
    ("circ", 'ˆ'),
    ("tilde", '˜'),
    ("Alpha", 'Α'),
    ("Beta", 'Β'),
    ("Gamma", 'Γ'),
    ("Delta", 'Δ'),
    ("Theta", 'Θ'),
    ("Lambda", 'Λ'),
    ("Pi", 'Π'),
    ("Sigma", 'Σ'),
    ("Phi", 'Φ'),
    ("Psi", 'Ψ'),
    ("Omega", 'Ω'),
    ("alpha", 'α'),
    ("beta", 'β'),
    ("gamma", 'γ'),
    ("delta", 'δ'),
    ("epsilon", 'ε'),
    ("theta", 'θ'),
    ("lambda", 'λ'),
    ("mu", 'μ'),
    ("pi", 'π'),
    ("rho", 'ρ'),
    ("sigma", 'σ'),
    ("tau", 'τ'),
    ("phi", 'φ'),
    ("chi", 'χ'),
    ("psi", 'ψ'),
    ("omega", 'ω'),
    ("ensp", '\u{2002}'),
    ("emsp", '\u{2003}'),
    ("thinsp", '\u{2009}'),
    ("zwnj", '\u{200c}'),
    ("zwj", '\u{200d}'),
    ("ndash", '–'),
    ("mdash", '—'),
    ("lsquo", '‘'),
    ("rsquo", '’'),
    ("sbquo", '‚'),
    ("ldquo", '“'),
    ("rdquo", '”'),
    ("bdquo", '„'),
    ("dagger", '†'),
    ("Dagger", '‡'),
    ("bull", '•'),
    ("hellip", '…'),
    ("permil", '‰'),
    ("prime", '′'),
    ("Prime", '″'),
    ("lsaquo", '‹'),
    ("rsaquo", '›'),
    ("euro", '€'),
    ("trade", '™'),
    ("larr", '←'),
    ("uarr", '↑'),
    ("rarr", '→'),
    ("darr", '↓'),
    ("harr", '↔'),
    ("rArr", '⇒'),
    ("hArr", '⇔'),
    ("forall", '∀'),
    ("part", '∂'),
    ("exist", '∃'),
    ("empty", '∅'),
    ("isin", '∈'),
    ("sum", '∑'),
    ("minus", '−'),
    ("radic", '√'),
    ("infin", '∞'),
    ("cap", '∩'),
    ("cup", '∪'),
    ("int", '∫'),
    ("asymp", '≈'),
    ("ne", '≠'),
    ("equiv", '≡'),
    ("le", '≤'),
    ("ge", '≥'),
    ("hearts", '♥'),
];

/// `&name;`, `&#123;` and `&#x7b;` back to the character. What is not an entity — a bare
/// ampersand, a name nobody defined, a number that is no character — stays as it was
/// written, which is what a browser does with it too.
pub fn html_decode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let found = tail
            .char_indices()
            .take(32)
            .find(|(_, c)| *c == ';')
            .and_then(|(end, _)| {
                let body = &tail[..end];
                let ch = if let Some(number) = body.strip_prefix('#') {
                    let code = match number.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => number.parse::<u32>().ok(),
                    };
                    code.and_then(char::from_u32)
                } else {
                    ENTITIES
                        .iter()
                        .find(|(name, _)| *name == body)
                        .map(|(_, ch)| *ch)
                };
                ch.map(|ch| (ch, end))
            });
        match found {
            Some((ch, end)) => {
                out.push(ch);
                rest = &tail[end + 1..];
            }
            None => {
                out.push('&');
                rest = tail;
            }
        }
    }
    out.push_str(rest);
    out
}

// ── ROT13 ───────────────────────────────────────────────────────────────────────────────

/// Thirteen places round the alphabet, which is its own way back.
pub fn rot13(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            'a'..='z' => (((ch as u8 - b'a') + 13) % 26 + b'a') as char,
            'A'..='Z' => (((ch as u8 - b'A') + 13) % 26 + b'A') as char,
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every byte value, and enough of them that every line length is crossed.
    fn bytes(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 7 + i / 256) as u8).collect()
    }

    #[test]
    fn base32_comes_back_at_every_length_and_reads_what_is_typed_by_hand() {
        // The vectors of RFC 4648 §10.
        for (plain, coded) in [
            ("f", "MY======"),
            ("fo", "MZXQ===="),
            ("foo", "MZXW6==="),
            ("foob", "MZXW6YQ="),
            ("fooba", "MZXW6YTB"),
            ("foobar", "MZXW6YTBOI======"),
        ] {
            assert_eq!(base32_encode(plain.as_bytes(), 0), coded.as_bytes());
            assert_eq!(base32_decode(coded.as_bytes()).unwrap(), plain.as_bytes());
        }
        for len in [1, 4, 5, 6, 99, 700] {
            let data = bytes(len);
            let coded = base32_encode(&data, 64);
            assert!(coded.split(|b| *b == b'\n').all(|l| l.len() <= 64));
            assert_eq!(base32_decode(&coded).unwrap(), data);
        }
        // Small letters, no padding, a space in the middle.
        assert_eq!(base32_decode(b"mzxw 6ytb oi").unwrap(), b"foobar");
        assert!(base32_decode(b"MZXW1").is_err(), "1 is not in the alphabet");
        assert!(
            base32_decode(b"MZX").is_err(),
            "three characters are no group"
        );
        assert!(
            base32_decode(b"MY==MY==").is_err(),
            "nothing follows padding"
        );
        assert!(base32_decode(b"").is_err());
    }

    #[test]
    fn hex_comes_back_and_reads_past_what_people_write_between_the_pairs() {
        let data = bytes(300);
        let coded = hex_encode(&data, 64);
        assert!(coded.split(|b| *b == b'\n').all(|l| l.len() <= 64));
        assert_eq!(hex_decode(&coded).unwrap(), data);
        assert_eq!(hex_encode(&[0xde, 0xad, 0xbe, 0xef], 0), b"deadbeef");
        assert_eq!(
            hex_decode(b"DE:AD:be:ef").unwrap(),
            [0xde, 0xad, 0xbe, 0xef]
        );
        assert_eq!(hex_decode(b"0xDE, 0xAD,\n0x00").unwrap(), [0xde, 0xad, 0]);
        assert_eq!(hex_decode(b"de ad\r\nbe ef\n").unwrap().len(), 4);
        assert!(hex_decode(b"dea").is_err(), "half a byte");
        assert!(hex_decode(b"dexd").is_err());
        assert!(hex_decode(b" \n").is_err());
    }

    #[test]
    fn percent_encoding_comes_back_and_leaves_a_plus_alone() {
        let data = bytes(300);
        assert_eq!(url_decode(&url_encode(&data, 0)).unwrap(), data);
        assert_eq!(url_decode(&url_encode(&data, 60)).unwrap(), data);
        assert_eq!(
            url_encode("a b/c?d=è~".as_bytes(), 0),
            b"a%20b%2Fc%3Fd%3D%C3%A8~"
        );
        assert_eq!(url_decode(b"a+b%2fc").unwrap(), b"a+b/c");
        assert!(url_decode(b"100%").is_err());
        assert!(url_decode(b"%zz").is_err());
    }

    #[test]
    fn quoted_printable_comes_back_byte_for_byte_with_its_line_ends() {
        for data in [
            bytes(1000),
            b"plain text\r\nwith DOS ends \r\nand a tab\t\r\n".to_vec(),
            b"unix\nends = and a lone \r in it \n".to_vec(),
            b"ends on a space ".to_vec(),
            "città, perché no — ".repeat(20).into_bytes(),
            Vec::new(),
        ] {
            for line in [0, 20, 76] {
                let coded = qp_encode(&data, line);
                let max = if line == 0 { 76 } else { line };
                for row in coded.split(|b| *b == b'\n') {
                    let row = row.strip_suffix(b"\r").unwrap_or(row);
                    assert!(row.len() <= max, "{} > {max}", row.len());
                    assert!(!row.ends_with(b" ") && !row.ends_with(b"\t"));
                }
                assert!(coded.iter().all(|b| b.is_ascii()));
                assert_eq!(qp_decode(&coded).unwrap(), data);
            }
        }
        assert_eq!(qp_encode("è=1".as_bytes(), 0), b"=C3=A8=3D1");
        // A soft break with a transport's padding after it, and small hex digits.
        assert_eq!(
            qp_decode(b"caf=c3=a9 lon= \r\ngue").unwrap(),
            "café longue".as_bytes()
        );
        assert!(qp_decode(b"a=zz").is_err());
    }

    #[test]
    fn html_entities_come_back_and_what_is_not_one_is_left_alone() {
        let text = "<a href=\"x?a=1&b=2\">it's</a>";
        let coded = html_encode(text);
        assert_eq!(
            coded,
            "&lt;a href=&quot;x?a=1&amp;b=2&quot;&gt;it&#39;s&lt;/a&gt;"
        );
        assert_eq!(html_decode(&coded), text);
        assert_eq!(
            html_decode("caf&eacute; &#8212; &#x20AC;5&nbsp;"),
            "café — €5\u{a0}"
        );
        // A bare ampersand, an unknown name, a number that is no character.
        assert_eq!(
            html_decode("R&D &nosuch; &#xD800; &"),
            "R&D &nosuch; &#xD800; &"
        );
        // Every name in the table is spelled once.
        for (i, (name, _)) in ENTITIES.iter().enumerate() {
            assert!(!ENTITIES[..i].iter().any(|(n, _)| n == name), "{name}");
        }
    }

    #[test]
    fn rot13_is_its_own_way_back() {
        assert_eq!(rot13("Hello, World! 123 è"), "Uryyb, Jbeyq! 123 è");
        assert_eq!(rot13(&rot13("Hello, World!")), "Hello, World!");
    }
}
