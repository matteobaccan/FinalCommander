//! A condition on what a file says about itself (SPEC 06 §1d): the camera of a photograph,
//! the artist of a song, the author of a document.
//!
//! Nothing is parsed here. The viewer's `F11` already reads all of it — EXIF and PNG text,
//! ID3, FLAC and Ogg comments, the properties of a PDF and of an office document, the version
//! resource of a program — into rows of a label and a value
//! ([`fcmd_view::structure::read`]), and a search condition is a question asked of those same
//! rows. So what `F11` shows about a file is, by construction, exactly what this can find it
//! by, and a format the pane learns to read is a format the search can be asked about.

use std::io::{self, Read};

use fcmd_core::VPath;
use fcmd_view::structure::{self, Bytes, RowKind, Structure};
use fcmd_view::Source;

use crate::fs::FindFs;

/// The most that is read into memory to look at a file that cannot be read at an offset — an
/// entry of an archive. A file on the disk is never read whole: the readers go to the few
/// places they need.
pub const WHOLE_FILE_LIMIT: u64 = 64 << 20;

/// One thing a row has to say: a label that contains `field` and a value that contains
/// `value`, both without regard to case. Either may be empty — "any row" and "anything".
#[derive(Debug, Clone, PartialEq, Eq)]
struct Term {
    field: String,
    value: String,
}

/// What the dialog's *metadata* row asks for.
///
/// The grammar is small enough to guess: `artist=bowie` is a row whose label contains
/// "artist" and whose value contains "bowie"; `=canon` is "canon" in any row; `gps=` is a
/// file that has a GPS row at all; a bare `canon` is the same as `=canon`. Several of them
/// separated by `;` must all hold. `format` is a row every file the readers know has: what
/// it is (`JPEG`, `PDF`, `MP3`), with its one-line summary.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetaCond {
    terms: Vec<Term>,
}

impl MetaCond {
    /// Read the row as typed. Any text is a condition; an empty one asks for nothing.
    pub fn parse(text: &str) -> MetaCond {
        let terms = text
            .split(';')
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(|part| {
                let (field, value) = match part.split_once('=') {
                    Some((field, value)) => (field.trim(), value.trim()),
                    None => ("", part),
                };
                Term {
                    field: field.to_lowercase(),
                    value: value.to_lowercase(),
                }
            })
            .filter(|term| !(term.field.is_empty() && term.value.is_empty()))
            .collect();
        MetaCond { terms }
    }

    /// Does it ask for anything?
    pub fn is_empty(&self) -> bool {
        self.terms.is_empty()
    }

    /// Do the rows of one file answer every term?
    pub fn matches(&self, shape: &Structure) -> bool {
        // The row every known file has, so "what kind of file" can be asked like the rest.
        let format = format!("{} {}", shape.format, shape.summary).to_lowercase();
        self.terms.iter().all(|term| {
            let holds = |label: &str, value: &str| {
                !value.is_empty() && label.contains(&term.field) && value.contains(&term.value)
            };
            holds("format", &format)
                || shape.rows.iter().any(|row| {
                    // A heading is not something a file says about itself.
                    row.kind != RowKind::Group
                        && holds(&row.label.to_lowercase(), &row.value.to_lowercase())
                })
        })
    }

    /// Ask about one file. `Ok(false)` for a file the readers do not know — which is most
    /// files, and not a problem to report.
    pub fn matches_file(&self, fs: &dyn FindFs, path: &VPath) -> io::Result<bool> {
        let shape = match fs.open_at(path) {
            Some(source) => structure::read(&mut At(source?)),
            None => {
                let mut bytes = Vec::new();
                fs.open(path)?
                    .take(WHOLE_FILE_LIMIT)
                    .read_to_end(&mut bytes)?;
                structure::read(&mut bytes.as_slice())
            }
        };
        Ok(shape.is_some_and(|shape| self.matches(&shape)))
    }
}

/// The readers ask for bytes at an offset; a [`Source`] is something that has them.
struct At(Box<dyn Source>);

impl Bytes for At {
    fn at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        self.0.read_vec(offset, len).unwrap_or_default()
    }

    fn len(&mut self) -> u64 {
        self.0.len()
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use fcmd_vfs::LocalVfs;

    /// A PNG with one `tEXt` chunk, which is the smallest file the readers have a row for.
    pub(crate) fn png_with_text(key: &str, value: &str) -> Vec<u8> {
        fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], body: &[u8]) {
            out.extend_from_slice(&(body.len() as u32).to_be_bytes());
            out.extend_from_slice(kind);
            out.extend_from_slice(body);
            // The readers do not check the CRC, and neither does this test.
            out.extend_from_slice(&[0; 4]);
        }
        let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = Vec::new();
        header.extend_from_slice(&4u32.to_be_bytes());
        header.extend_from_slice(&3u32.to_be_bytes());
        header.extend_from_slice(&[8, 2, 0, 0, 0]);
        chunk(&mut out, b"IHDR", &header);
        let mut text = key.as_bytes().to_vec();
        text.push(0);
        text.extend_from_slice(value.as_bytes());
        chunk(&mut out, b"tEXt", &text);
        chunk(&mut out, b"IEND", &[]);
        out
    }

    fn shape_of(bytes: &[u8]) -> Structure {
        let mut bytes = bytes;
        structure::read(&mut bytes).expect("the readers know a PNG")
    }

    #[test]
    fn the_row_is_read_as_fields_and_values() {
        assert!(MetaCond::parse("").is_empty());
        assert!(MetaCond::parse(" ; = ; ").is_empty());
        let cond = MetaCond::parse("Artist = Bowie; =1977 ; gps= ; canon");
        assert_eq!(
            cond.terms,
            vec![
                Term {
                    field: "artist".into(),
                    value: "bowie".into()
                },
                Term {
                    field: String::new(),
                    value: "1977".into()
                },
                Term {
                    field: "gps".into(),
                    value: String::new()
                },
                Term {
                    field: String::new(),
                    value: "canon".into()
                },
            ]
        );
    }

    #[test]
    fn a_condition_is_a_question_asked_of_the_rows_f11_shows() {
        let shape = shape_of(&png_with_text("Author", "Ada Lovelace"));
        // The reader's own row, whatever it chose to call it: the value is what was written.
        assert!(
            shape.rows.iter().any(|r| r.value.contains("Ada Lovelace")),
            "{:?}",
            shape.rows
        );
        assert!(MetaCond::parse("author=ada").matches(&shape));
        assert!(MetaCond::parse("AUTHOR=LOVELACE").matches(&shape));
        assert!(MetaCond::parse("lovelace").matches(&shape));
        assert!(MetaCond::parse("author=").matches(&shape));
        assert!(!MetaCond::parse("author=babbage").matches(&shape));
        assert!(!MetaCond::parse("camera=").matches(&shape));
        // Every term has to hold.
        assert!(MetaCond::parse("author=ada; format=png").matches(&shape));
        assert!(!MetaCond::parse("author=ada; format=jpeg").matches(&shape));
        // And what kind of file it is can be asked by itself.
        assert!(MetaCond::parse("format=png").matches(&shape));
    }

    #[test]
    fn a_file_on_the_disk_is_asked_through_the_filesystem() {
        let dir = tempfile::tempdir().unwrap();
        let picture = dir.path().join("a.png");
        let text = dir.path().join("a.txt");
        std::fs::write(&picture, png_with_text("Author", "Ada Lovelace")).unwrap();
        std::fs::write(&text, b"Author: Ada Lovelace").unwrap();

        let cond = MetaCond::parse("author=ada");
        assert!(cond
            .matches_file(&LocalVfs, &VPath::local(&picture))
            .unwrap());
        // A text file that happens to say the same thing has no rows: it is not metadata.
        assert!(!cond.matches_file(&LocalVfs, &VPath::local(&text)).unwrap());
        // A file that is not there is an error, for the walk to report.
        assert!(cond
            .matches_file(&LocalVfs, &VPath::local(dir.path().join("gone.png")))
            .is_err());
    }
}
