use encoding_rs::{UTF_16BE, UTF_16LE, WINDOWS_1252};

/// Code page 437 (the DOS box-drawing one): what a dump of an old file should look like.
/// `encoding_rs` does not ship it, so the high half is a table.
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ',
    'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ',
    'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕',
    '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦',
    '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐',
    '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±',
    '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', ' ',
];

/// Which encoding the viewer is decoding with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codepage {
    Utf8,
    Utf16Le,
    Utf16Be,
    Cp1252,
    Cp437,
}

impl Codepage {
    pub fn name(self) -> &'static str {
        match self {
            Codepage::Utf8 => "UTF-8",
            Codepage::Utf16Le => "UTF-16LE",
            Codepage::Utf16Be => "UTF-16BE",
            Codepage::Cp1252 => "CP1252",
            Codepage::Cp437 => "CP437",
        }
    }

    pub const ALL: [Codepage; 5] = [
        Codepage::Utf8,
        Codepage::Utf16Le,
        Codepage::Utf16Be,
        Codepage::Cp1252,
        Codepage::Cp437,
    ];

    pub fn next(self) -> Codepage {
        let i = Codepage::ALL.iter().position(|c| *c == self).unwrap_or(0);
        Codepage::ALL[(i + 1) % Codepage::ALL.len()]
    }

    /// Decode bytes, replacing what does not decode rather than failing.
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Codepage::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Codepage::Utf16Le => UTF_16LE.decode(bytes).0.into_owned(),
            Codepage::Utf16Be => UTF_16BE.decode(bytes).0.into_owned(),
            Codepage::Cp1252 => WINDOWS_1252.decode(bytes).0.into_owned(),
            Codepage::Cp437 => bytes
                .iter()
                .map(|b| match b {
                    0x00 => ' ',
                    0x20..=0x7e => *b as char,
                    0x80..=0xff => CP437_HIGH[(*b - 0x80) as usize],
                    _ => '.',
                })
                .collect(),
        }
    }

    /// Encode a search query the way the file is encoded, so searching for `café` finds it in
    /// a CP1252 file as well as in a UTF-8 one.
    pub fn encode(self, text: &str) -> Vec<u8> {
        match self {
            Codepage::Utf8 => text.as_bytes().to_vec(),
            Codepage::Utf16Le => text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
            Codepage::Utf16Be => text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect(),
            Codepage::Cp1252 => WINDOWS_1252.encode(text).0.into_owned(),
            Codepage::Cp437 => text
                .chars()
                .map(|c| match c {
                    ' '..='~' => c as u8,
                    other => CP437_HIGH
                        .iter()
                        .position(|h| *h == other)
                        .map(|i| 0x80 + i as u8)
                        .unwrap_or(b'?'),
                })
                .collect(),
        }
    }

    /// How many bytes per code unit — the hex view uses it to keep rows aligned.
    pub fn unit(self) -> usize {
        match self {
            Codepage::Utf16Le | Codepage::Utf16Be => 2,
            _ => 1,
        }
    }
}

/// What a BOM says, if there is one: `(encoding, bom length)`.
pub fn sniff_bom(head: &[u8]) -> Option<(Codepage, usize)> {
    match head {
        [0xEF, 0xBB, 0xBF, ..] => Some((Codepage::Utf8, 3)),
        [0xFF, 0xFE, ..] => Some((Codepage::Utf16Le, 2)),
        [0xFE, 0xFF, ..] => Some((Codepage::Utf16Be, 2)),
        _ => None,
    }
}

/// Guess from the first chunk of a file: BOM first, then valid UTF-8, then a UTF-16 pattern,
/// then CP1252 as the "it is some 8-bit text" fallback.
pub fn detect(head: &[u8]) -> Codepage {
    if let Some((cp, _)) = sniff_bom(head) {
        return cp;
    }

    // A UTF-16 file is valid UTF-8 as far as `from_utf8` is concerned (NUL is a legal byte),
    // so the NUL pattern has to be checked *before* the UTF-8 test.
    let pairs = head.len() / 2;
    if pairs > 8 {
        let nuls_odd = head.iter().skip(1).step_by(2).filter(|b| **b == 0).count();
        let nuls_even = head.iter().step_by(2).filter(|b| **b == 0).count();
        if nuls_odd > pairs * 3 / 4 && nuls_even < pairs / 4 {
            return Codepage::Utf16Le;
        }
        if nuls_even > pairs * 3 / 4 && nuls_odd < pairs / 4 {
            return Codepage::Utf16Be;
        }
    }

    match std::str::from_utf8(head) {
        Ok(_) => return Codepage::Utf8,
        Err(e) => {
            // The *window* can cut a multi-byte character in half; that is not a reason to
            // call the file 8-bit. But a short sample with an invalid byte in the middle is.
            if head.len() > 8 && e.valid_up_to() + 3 >= head.len() {
                return Codepage::Utf8;
            }
        }
    }
    Codepage::Cp1252
}

/// Does this look like something a text view would mangle? (FAR's "detect dump mode".)
pub fn looks_binary(head: &[u8]) -> bool {
    if head.is_empty() {
        return false;
    }
    if sniff_bom(head).is_some() {
        return false;
    }
    let nul = head.iter().filter(|b| **b == 0).count();
    if nul * 100 / head.len() > 10 {
        return true;
    }
    let control = head
        .iter()
        .filter(|b| **b < 0x09 || (**b > 0x0d && **b < 0x20))
        .count();
    control * 100 / head.len() > 5
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_text_is_detected_and_decoded() {
        let bytes = "ciao, mondo — こんにちは".as_bytes();
        assert_eq!(detect(bytes), Codepage::Utf8);
        assert_eq!(Codepage::Utf8.decode(bytes), "ciao, mondo — こんにちは");
        assert!(!looks_binary(bytes));
    }

    #[test]
    fn boms_win_over_heuristics() {
        let mut utf16 = vec![0xFF, 0xFE];
        utf16.extend_from_slice(&[b'h', 0, b'i', 0]);
        assert_eq!(detect(&utf16), Codepage::Utf16Le);
        assert_eq!(sniff_bom(&utf16), Some((Codepage::Utf16Le, 2)));
        assert_eq!(Codepage::Utf16Le.decode(&utf16[2..]), "hi");
    }

    #[test]
    fn utf16_without_a_bom_is_guessed_from_the_nul_pattern() {
        let mut bytes = Vec::new();
        for c in "hello world, this is wide text".chars() {
            bytes.push(c as u8);
            bytes.push(0);
        }
        assert_eq!(detect(&bytes), Codepage::Utf16Le);
    }

    #[test]
    fn eight_bit_text_falls_back_to_cp1252() {
        let bytes = b"caf\xe9 cr\xe8me";
        assert_eq!(detect(bytes), Codepage::Cp1252);
        assert_eq!(Codepage::Cp1252.decode(bytes), "café crème");
    }

    #[test]
    fn cp437_draws_boxes() {
        let bytes = [0xc9, 0xcd, 0xbb];
        assert_eq!(Codepage::Cp437.decode(&bytes), "╔═╗");
    }

    #[test]
    fn binaries_are_recognised_but_text_is_not() {
        let mut exe = vec![0x7f, b'E', b'L', b'F'];
        exe.extend(std::iter::repeat_n(0u8, 60));
        assert!(looks_binary(&exe));
        assert!(!looks_binary(b"plain text\nwith lines\n"));
        assert!(!looks_binary(b""));
    }

    #[test]
    fn invalid_bytes_decode_to_replacements_instead_of_failing() {
        let bytes = b"ok \xff\xfe bad";
        let text = Codepage::Utf8.decode(bytes);
        assert!(text.contains('\u{fffd}'), "{text}");
    }

    #[test]
    fn codepages_cycle() {
        assert_eq!(Codepage::Utf8.next(), Codepage::Utf16Le);
        assert_eq!(Codepage::Cp437.next(), Codepage::Utf8);
    }
}
