//! Writing bytes down as text and reading them back (SPEC 03 §9a, feature 12.16).
//!
//! Base64, base32, UUencode, XXencode, yEnc, quoted-printable, hex and percent-encoding —
//! and, for text only, HTML entities and ROT13. One module, because three places need it and
//! must agree: the command on files (`Files → Encode`, `Files → Decode`), the editor's
//! conversion of a block, and `Alt+B` in the viewer. A format the file command can read and
//! the viewer cannot would be a file one has to copy somewhere to look at.
//!
//! The alphabets live in [`base64`], [`text`], [`uu`] and [`yenc`]; what a *file* of each
//! looks like — its extension, its line length, its parts, how it is told from the others
//! and put back together — is [`files`]. This crate knows nothing of folders: everything
//! here is bytes in and bytes out, and the caller reads and writes them.

pub mod base64;
pub mod files;
pub mod text;
pub mod uu;
pub mod yenc;

pub use files::{
    carried_name, decode_files, detect, encode_file, part_of, Decoded, EncodeOptions, Encoder,
    Input, Output,
};

/// The ways of writing bytes down that this program knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Base64,
    Base32,
    Uu,
    Xx,
    Yenc,
    Qp,
    Hex,
    Url,
    /// HTML entities: text to text, so the editor's and nobody else's.
    Html,
    /// And ROT13, for the same reason.
    Rot13,
}

impl Format {
    /// What a file can be written as, in the order the dialog offers them.
    pub const FILES: [Format; 8] = [
        Format::Base64,
        Format::Base32,
        Format::Uu,
        Format::Xx,
        Format::Yenc,
        Format::Qp,
        Format::Hex,
        Format::Url,
    ];

    /// What a block of text can be turned into and out of. yEnc is not among them: what it
    /// writes is eight-bit bytes, which a text buffer would read through a code page and
    /// hand back as something else.
    pub const TEXT: [Format; 9] = [
        Format::Base64,
        Format::Base32,
        Format::Hex,
        Format::Url,
        Format::Qp,
        Format::Uu,
        Format::Xx,
        Format::Html,
        Format::Rot13,
    ];

    /// The name configuration, commands and tests know it by.
    pub fn id(self) -> &'static str {
        match self {
            Format::Base64 => "base64",
            Format::Base32 => "base32",
            Format::Uu => "uu",
            Format::Xx => "xx",
            Format::Yenc => "yenc",
            Format::Qp => "qp",
            Format::Hex => "hex",
            Format::Url => "url",
            Format::Html => "html",
            Format::Rot13 => "rot13",
        }
    }

    pub fn parse(id: &str) -> Option<Format> {
        Format::FILES
            .into_iter()
            .chain([Format::Html, Format::Rot13])
            .find(|format| format.id() == id)
    }

    /// What it is called. These are names, the same in every language.
    pub fn name(self) -> &'static str {
        match self {
            Format::Base64 => "Base64",
            Format::Base32 => "Base32",
            Format::Uu => "UUencode",
            Format::Xx => "XXencode",
            Format::Yenc => "yEnc",
            Format::Qp => "Quoted-printable",
            Format::Hex => "Hex",
            Format::Url => "URL (percent-encoding)",
            Format::Html => "HTML entities",
            Format::Rot13 => "ROT13",
        }
    }

    /// What is put after a file's name when it is written as this; nothing for the two that
    /// are not for files.
    pub fn extension(self) -> Option<&'static str> {
        match self {
            Format::Base64 => Some("b64"),
            Format::Base32 => Some("b32"),
            Format::Uu => Some("uue"),
            Format::Xx => Some("xxe"),
            Format::Yenc => Some("yenc"),
            Format::Qp => Some("qp"),
            Format::Hex => Some("hex"),
            // Not `.url`, which is an Internet shortcut wherever a file manager runs.
            Format::Url => Some("urlenc"),
            Format::Html | Format::Rot13 => None,
        }
    }

    /// The line length a file of it has when nobody asks for another, in characters;
    /// nothing is "not broken into lines at all".
    pub fn default_line(self) -> usize {
        match self {
            Format::Base64 | Format::Qp => 76,
            Format::Base32 | Format::Hex => 64,
            Format::Uu | Format::Xx => 61,
            Format::Yenc => yenc::DEFAULT_LINE,
            Format::Url | Format::Html | Format::Rot13 => 0,
        }
    }
}

/// Why something was not encoded or decoded, in the words that go on the status line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Base64(base64::Error),
    /// The bytes are not what the format's own rules allow.
    Malformed {
        format: Format,
        why: String,
    },
    /// None of the formats: nothing in the file says what it is.
    Unknown,
    /// What came out is not as long as the file said it would be.
    Size {
        name: String,
        expected: u64,
        found: u64,
    },
    /// What came out does not have the CRC32 the file said it would have.
    Checksum {
        name: String,
        expected: u32,
        found: u32,
    },
    /// A file in pieces with one of them not there.
    Parts {
        name: String,
        missing: usize,
    },
    /// A format asked to do what it is not for: text for yEnc, a file for HTML entities.
    NotFor(Format),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::Base64(e) => e.fmt(f),
            Error::Malformed { format, why } => write!(f, "{}: {why}", format.name()),
            Error::Unknown => f.write_str("nothing in it says what it is encoded as"),
            Error::Size {
                name,
                expected,
                found,
            } => write!(
                f,
                "{name} should be {expected} bytes and what was decoded is {found}"
            ),
            Error::Checksum {
                name,
                expected,
                found,
            } => write!(
                f,
                "{name} is damaged: its CRC32 should be {expected:08x} and is {found:08x}"
            ),
            Error::Parts { name, missing } => {
                write!(f, "{name} is in pieces and piece {missing} is not here")
            }
            Error::NotFor(format) => {
                write!(
                    f,
                    "{} is not something this can be written as",
                    format.name()
                )
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<base64::Error> for Error {
    fn from(e: base64::Error) -> Self {
        Error::Base64(e)
    }
}

/// A block of text written as `format`, for the editor: the text's UTF-8 bytes, encoded,
/// with no line breaks added except where the format is made of lines.
pub fn encode_text(format: Format, input: &str) -> Result<String, Error> {
    let bytes = input.as_bytes();
    let coded = match format {
        Format::Base64 => base64::encode(bytes, 0),
        Format::Base32 => text::base32_encode(bytes, 0),
        Format::Hex => text::hex_encode(bytes, 0),
        Format::Url => text::url_encode(bytes, 0),
        Format::Qp => text::qp_encode(bytes, 0),
        Format::Uu | Format::Xx => {
            let mut section =
                uu::encode_section("text", bytes, format == Format::Xx, uu::FULL_LINE);
            // The block that was marked did not end in a line break, and neither does what
            // takes its place.
            section.pop();
            section
        }
        Format::Html => return Ok(text::html_encode(input)),
        Format::Rot13 => return Ok(text::rot13(input)),
        Format::Yenc => return Err(Error::NotFor(format)),
    };
    // Every one of these writes ASCII and nothing else.
    Ok(String::from_utf8_lossy(&coded).into_owned())
}

/// A block of text read back out of `format`: the bytes it stood for, which may or may not
/// be text — [`as_text`] says which.
pub fn decode_text(format: Format, input: &str) -> Result<Vec<u8>, Error> {
    let bytes = input.as_bytes();
    match format {
        Format::Base64 => Ok(base64::decode(bytes)?),
        Format::Base32 => text::base32_decode(bytes),
        Format::Hex => text::hex_decode(bytes),
        Format::Url => text::url_decode(bytes),
        Format::Qp => text::qp_decode(bytes),
        Format::Uu | Format::Xx => {
            // With its `begin` and `end` the block says which alphabet it is itself; a body
            // marked without them is read as the one that was asked for.
            let mut sections = uu::sections(bytes)?;
            if sections.is_empty() {
                uu::decode_bare(bytes, format == Format::Xx)
            } else {
                Ok(sections.remove(0).bytes)
            }
        }
        Format::Html => Ok(text::html_decode(input).into_bytes()),
        Format::Rot13 => Ok(text::rot13(input).into_bytes()),
        Format::Yenc => Err(Error::NotFor(format)),
    }
}

/// What was decoded, as text — or nothing, when it is not text: bytes that are not UTF-8,
/// or that hold control characters no text has. A picture pasted into a source file as
/// mojibake helps nobody, so the editor asks this before it inserts anything.
pub fn as_text(bytes: Vec<u8>) -> Option<String> {
    let text = String::from_utf8(bytes).ok()?;
    let readable = |c: char| !c.is_control() || matches!(c, '\t' | '\n' | '\r' | '\u{c}');
    text.chars().all(readable).then_some(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_format_has_a_name_an_id_and_a_way_back_from_it() {
        for format in Format::FILES
            .into_iter()
            .chain([Format::Html, Format::Rot13])
        {
            assert_eq!(Format::parse(format.id()), Some(format));
            assert!(!format.name().is_empty());
        }
        assert_eq!(Format::parse("nonsense"), None);
        // What is for files has an extension, and no two share one.
        for (i, format) in Format::FILES.iter().enumerate() {
            let ext = format.extension().expect("a file format has an extension");
            assert!(Format::FILES[..i]
                .iter()
                .all(|f| f.extension() != Some(ext)));
        }
        assert_eq!(Format::Html.extension(), None);
        assert!(!Format::TEXT.contains(&Format::Yenc));
    }

    #[test]
    fn a_block_of_text_goes_into_every_format_and_comes_back_out() {
        let block = "Città: “perché no?”\n\tsecond line & <third> = 100%\n";
        for format in Format::TEXT {
            let coded = encode_text(format, block).unwrap();
            assert_ne!(coded, block, "{format:?} changed nothing");
            let back = decode_text(format, &coded).unwrap();
            assert_eq!(as_text(back).as_deref(), Some(block), "{format:?}");
        }
        assert_eq!(encode_text(Format::Base64, "hello").unwrap(), "aGVsbG8=");
        assert_eq!(encode_text(Format::Hex, "hello").unwrap(), "68656c6c6f");
        assert_eq!(encode_text(Format::Url, "a b").unwrap(), "a%20b");
        assert_eq!(encode_text(Format::Rot13, "hello").unwrap(), "uryyb");
        assert_eq!(
            encode_text(Format::Uu, "Cat").unwrap(),
            "begin 644 text\n#0V%T\n`\nend"
        );
        // yEnc is bytes, not text, in both directions.
        assert_eq!(
            encode_text(Format::Yenc, "x"),
            Err(Error::NotFor(Format::Yenc))
        );
        assert!(decode_text(Format::Yenc, "x").is_err());
    }

    #[test]
    fn a_uu_body_marked_without_its_begin_is_read_as_what_was_asked() {
        assert_eq!(decode_text(Format::Uu, "#0V%T\n`").unwrap(), b"Cat");
        let xx = encode_text(Format::Xx, "Cat").unwrap();
        let body: Vec<&str> = xx.lines().skip(1).collect();
        assert_eq!(decode_text(Format::Xx, &body.join("\n")).unwrap(), b"Cat");
        // And with its begin, the block says what it is whatever was asked.
        assert_eq!(decode_text(Format::Uu, &xx).unwrap(), b"Cat");
    }

    #[test]
    fn what_is_decoded_is_text_only_when_it_is_text() {
        assert_eq!(
            as_text(b"plain\r\n\ttext".to_vec()).as_deref(),
            Some("plain\r\n\ttext")
        );
        assert_eq!(as_text(vec![0xff, 0xfe, 0x00]), None, "not UTF-8");
        assert_eq!(as_text(b"a\0b".to_vec()), None, "a NUL is not text");
        assert_eq!(as_text(vec![0x1b, b'[', b'm']), None, "nor is an escape");
        // A PNG's first bytes, out of base64.
        let png = decode_text(Format::Base64, "iVBORw0KGgo=").unwrap();
        assert_eq!(as_text(png), None);
    }

    #[test]
    fn a_refusal_says_what_and_where() {
        let e = decode_text(Format::Hex, "zz").unwrap_err();
        assert_eq!(
            e.to_string(),
            "Hex: byte 0 is 0x7a, which is not a hex digit"
        );
        let e = decode_text(Format::Base64, "a!").unwrap_err();
        assert_eq!(e.to_string(), "byte 1 is 0x21, which is not base64");
        let e = Error::Checksum {
            name: "a.bin".into(),
            expected: 0xdead_beef,
            found: 1,
        };
        assert_eq!(
            e.to_string(),
            "a.bin is damaged: its CRC32 should be deadbeef and is 00000001"
        );
    }
}
