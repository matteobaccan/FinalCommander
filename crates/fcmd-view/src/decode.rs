use encoding_rs::{Encoding, UTF_16BE, UTF_16LE, WINDOWS_1252};

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
///
/// Five are handled here, because the program has to: UTF-8 and the two UTF-16 byte orders
/// are what a BOM names, and CP437 is the DOS box-drawing page that `encoding_rs` does not
/// ship. Everything else is one of `encoding_rs`'s own encodings, which is what makes the
/// list in `Shift+F8` as long as FAR's without a table per page (SPEC 05 §1.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codepage {
    Utf8,
    Utf16Le,
    Utf16Be,
    Cp1252,
    Cp437,
    /// One of `encoding_rs`'s: single-byte pages, and the East Asian multi-byte ones.
    Enc(&'static Encoding),
}

/// The code pages offered beyond the five above, each with the label the list shows.
///
/// The order is the order the list shows them in: the Windows pages, the DOS and Macintosh
/// ones, the ISO family, the two KOI8s, then the East Asian multi-byte pages. The label is
/// ours rather than the WHATWG name, because `windows-1251` is a worse answer than `CP1251`
/// to somebody looking for the Cyrillic page.
const MORE: &[(&Encoding, &str)] = &[
    (encoding_rs::WINDOWS_1250, "CP1250 (Central European)"),
    (encoding_rs::WINDOWS_1251, "CP1251 (Cyrillic)"),
    (encoding_rs::WINDOWS_1253, "CP1253 (Greek)"),
    (encoding_rs::WINDOWS_1254, "CP1254 (Turkish)"),
    (encoding_rs::WINDOWS_1255, "CP1255 (Hebrew)"),
    (encoding_rs::WINDOWS_1256, "CP1256 (Arabic)"),
    (encoding_rs::WINDOWS_1257, "CP1257 (Baltic)"),
    (encoding_rs::WINDOWS_1258, "CP1258 (Vietnamese)"),
    (encoding_rs::WINDOWS_874, "CP874 (Thai)"),
    (encoding_rs::IBM866, "CP866 (DOS Cyrillic)"),
    (encoding_rs::MACINTOSH, "Mac Roman"),
    (encoding_rs::X_MAC_CYRILLIC, "Mac Cyrillic"),
    (encoding_rs::ISO_8859_2, "ISO-8859-2 (Central European)"),
    (encoding_rs::ISO_8859_3, "ISO-8859-3 (South European)"),
    (encoding_rs::ISO_8859_4, "ISO-8859-4 (Baltic)"),
    (encoding_rs::ISO_8859_5, "ISO-8859-5 (Cyrillic)"),
    (encoding_rs::ISO_8859_6, "ISO-8859-6 (Arabic)"),
    (encoding_rs::ISO_8859_7, "ISO-8859-7 (Greek)"),
    (encoding_rs::ISO_8859_8, "ISO-8859-8 (Hebrew)"),
    (encoding_rs::ISO_8859_10, "ISO-8859-10 (Nordic)"),
    (encoding_rs::ISO_8859_13, "ISO-8859-13 (Baltic)"),
    (encoding_rs::ISO_8859_14, "ISO-8859-14 (Celtic)"),
    (encoding_rs::ISO_8859_15, "ISO-8859-15 (Latin-9)"),
    (encoding_rs::ISO_8859_16, "ISO-8859-16 (Romanian)"),
    (encoding_rs::KOI8_R, "KOI8-R (Russian)"),
    (encoding_rs::KOI8_U, "KOI8-U (Ukrainian)"),
    (encoding_rs::SHIFT_JIS, "Shift_JIS (Japanese)"),
    (encoding_rs::EUC_JP, "EUC-JP (Japanese)"),
    (encoding_rs::ISO_2022_JP, "ISO-2022-JP (Japanese)"),
    (encoding_rs::GBK, "GBK (Simplified Chinese)"),
    (encoding_rs::GB18030, "GB18030 (Simplified Chinese)"),
    (encoding_rs::BIG5, "Big5 (Traditional Chinese)"),
    (encoding_rs::EUC_KR, "EUC-KR (Korean)"),
];

impl Codepage {
    pub fn name(self) -> &'static str {
        match self {
            Codepage::Utf8 => "UTF-8",
            Codepage::Utf16Le => "UTF-16LE",
            Codepage::Utf16Be => "UTF-16BE",
            Codepage::Cp1252 => "CP1252 (Windows Latin-1)",
            Codepage::Cp437 => "CP437 (DOS Latin US)",
            Codepage::Enc(enc) => MORE
                .iter()
                .find(|(e, _)| std::ptr::eq(*e, enc))
                .map(|(_, label)| *label)
                // An encoding nobody listed still has a name, which beats saying nothing.
                .unwrap_or_else(|| enc.name()),
        }
    }

    /// What the configuration writes, and what [`Codepage::parse`] reads back.
    ///
    /// The WHATWG name for everything `encoding_rs` knows, and our own word for the rest: an
    /// id has to survive a list being reordered, so it cannot be a number in that list.
    pub fn id(self) -> &'static str {
        match self {
            Codepage::Utf8 => "utf-8",
            Codepage::Utf16Le => "utf-16le",
            Codepage::Utf16Be => "utf-16be",
            Codepage::Cp1252 => "windows-1252",
            Codepage::Cp437 => "cp437",
            Codepage::Enc(enc) => enc.name(),
        }
    }

    /// Read an id back.
    ///
    /// Case does not matter, every label `encoding_rs` knows works, and so do the two
    /// spellings somebody is likely to type instead: `CP-1251` with the hyphen, and the bare
    /// number `1251`. A page is written down by its id, so reading one has to be generous
    /// about how it was typed and exact about what it means.
    pub fn parse(id: &str) -> Option<Codepage> {
        let want = id.trim().to_ascii_lowercase();
        match want.as_str() {
            "utf-8" | "utf8" => return Some(Codepage::Utf8),
            "utf-16le" | "utf-16" | "utf16le" => return Some(Codepage::Utf16Le),
            "utf-16be" | "utf16be" => return Some(Codepage::Utf16Be),
            "cp437" | "cp-437" | "437" | "ibm437" => return Some(Codepage::Cp437),
            _ => {}
        }
        // `cp-1251` is nobody's label but everybody's spelling.
        let tries = [
            want.clone(),
            want.replacen("cp-", "cp", 1),
            format!("windows-{want}"),
            format!("cp{want}"),
            format!("ibm{want}"),
        ];
        let numeric = want.chars().all(|c| c.is_ascii_digit());
        for (n, label) in tries.iter().enumerate() {
            // The number forms are tried only for a number: `windows-utf8` is not a page.
            if n >= 2 && !numeric {
                break;
            }
            if let Some(enc) = Encoding::for_label(label.as_bytes()) {
                return Some(Codepage::of(enc));
            }
        }
        None
    }

    /// One of `encoding_rs`'s encodings as a code page, using the variant of its own where
    /// there is one: a page must have one spelling, or two rows of the list would be it.
    pub fn of(enc: &'static Encoding) -> Codepage {
        if std::ptr::eq(enc, encoding_rs::UTF_8) {
            Codepage::Utf8
        } else if std::ptr::eq(enc, UTF_16LE) {
            Codepage::Utf16Le
        } else if std::ptr::eq(enc, UTF_16BE) {
            Codepage::Utf16Be
        } else if std::ptr::eq(enc, WINDOWS_1252) {
            Codepage::Cp1252
        } else {
            Codepage::Enc(enc)
        }
    }

    pub const ALL: [Codepage; 5] = [
        Codepage::Utf8,
        Codepage::Utf16Le,
        Codepage::Utf16Be,
        Codepage::Cp1252,
        Codepage::Cp437,
    ];

    /// Every code page the program offers, in the order the list shows them.
    pub fn every() -> Vec<Codepage> {
        let mut all: Vec<Codepage> = Codepage::ALL.to_vec();
        all.extend(MORE.iter().map(|(enc, _)| Codepage::Enc(enc)));
        all
    }

    pub fn next(self) -> Codepage {
        self.next_in(&Codepage::ALL)
    }

    /// The next one of `among`, which is how `F8` walks the favourites rather than all of
    /// them. A page that is not in the list lands on its first row, because the point of
    /// `F8` is to reach the pages one actually uses.
    pub fn next_in(self, among: &[Codepage]) -> Codepage {
        if among.is_empty() {
            return self;
        }
        match among.iter().position(|c| *c == self) {
            Some(i) => among[(i + 1) % among.len()],
            None => among[0],
        }
    }

    /// Decode bytes, replacing what does not decode rather than failing.
    pub fn decode(self, bytes: &[u8]) -> String {
        match self {
            Codepage::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
            Codepage::Utf16Le => UTF_16LE.decode(bytes).0.into_owned(),
            Codepage::Utf16Be => UTF_16BE.decode(bytes).0.into_owned(),
            Codepage::Cp1252 => WINDOWS_1252.decode(bytes).0.into_owned(),
            Codepage::Enc(enc) => enc.decode(bytes).0.into_owned(),
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
            Codepage::Enc(enc) => enc.encode(text).0.into_owned(),
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

    #[test]
    fn every_code_page_has_one_spelling_and_survives_being_written_down() {
        let every = Codepage::every();
        // The five the program handles itself, and the rest from `encoding_rs`.
        assert!(every.len() > 30, "{}", every.len());
        for cp in &every {
            assert_eq!(
                Codepage::parse(cp.id()),
                Some(*cp),
                "{} does not read back",
                cp.id()
            );
            assert!(!cp.name().is_empty());
        }
        // One page, one spelling: `windows-1252` is the variant of its own, not an `Enc`.
        assert_eq!(Codepage::parse("windows-1252"), Some(Codepage::Cp1252));
        assert_eq!(Codepage::parse("utf8"), Some(Codepage::Utf8));
        assert_eq!(Codepage::parse("CP-1251"), Codepage::parse("windows-1251"));
        assert_eq!(Codepage::parse("1251"), Codepage::parse("windows-1251"));
        assert_eq!(Codepage::parse("866"), Codepage::parse("ibm866"));
        assert_eq!(Codepage::parse("437"), Some(Codepage::Cp437));
        assert_eq!(Codepage::parse("nonesuch"), None);
        let ids: Vec<&str> = every.iter().map(|cp| cp.id()).collect();
        let mut sorted = ids.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), ids.len(), "a page is listed twice");
    }

    #[test]
    fn a_cyrillic_page_decodes_and_encodes_what_it_should() {
        let koi8 = Codepage::parse("koi8-r").expect("KOI8-R is offered");
        let bytes = koi8.encode("Привет");
        assert_ne!(bytes, "Привет".as_bytes(), "it is not UTF-8");
        assert_eq!(koi8.decode(&bytes), "Привет");
        assert_eq!(koi8.unit(), 1);
    }

    /// `F8` walks the pages one has marked, and a page outside that list lands on its first
    /// row rather than nowhere.
    #[test]
    fn the_cycle_walks_the_list_it_is_given() {
        let favourites = [Codepage::Utf8, Codepage::Cp437];
        assert_eq!(Codepage::Utf8.next_in(&favourites), Codepage::Cp437);
        assert_eq!(Codepage::Cp437.next_in(&favourites), Codepage::Utf8);
        assert_eq!(Codepage::Cp1252.next_in(&favourites), Codepage::Utf8);
        assert_eq!(Codepage::Cp1252.next_in(&[]), Codepage::Cp1252);
    }
}
