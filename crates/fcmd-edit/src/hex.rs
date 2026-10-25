//! Editing bytes: the model behind the editor's hex mode (SPEC 05 §2.5).
//!
//! The viewer is read-only by contract — that is what lets it open a 40 GB file by seeking —
//! so changing bytes lives here, where undo, an atomic save and the unsaved-changes question
//! already exist.
//!
//! **A piece table rather than a buffer.** The file is never read into memory: the content is
//! a list of pieces, each either a stretch of the original file or a stretch of bytes the user
//! has typed. Overwriting one byte of a gigabyte file splits a piece and adds one byte; saving
//! walks the list and copies. That also answers "which bytes have changed" exactly, because a
//! changed byte *is* a piece from somewhere else — and it keeps answering after an insertion,
//! where a map of offsets would have gone wrong by the length of the insert.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use fcmd_core::VPath;

/// Where a piece's bytes come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    /// The file as it was opened.
    Original,
    /// The bytes the user has typed, kept in one growing buffer.
    Added,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Piece {
    source: Source,
    at: u64,
    len: u64,
}

/// One run of bytes that differs from the file on disk, as the patch list shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    /// Where it starts in the file as it is now.
    pub at: u64,
    /// What was there, as the file on disk still has it. Empty for an insertion.
    pub old: Vec<u8>,
    /// What is there now. Empty for a deletion.
    pub new: Vec<u8>,
}

impl Change {
    pub fn is_insert(&self) -> bool {
        self.old.is_empty()
    }

    pub fn is_delete(&self) -> bool {
        self.new.is_empty()
    }
}

/// Which half of the window the keys type into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    #[default]
    Hex,
    Text,
}

#[derive(Debug, thiserror::Error)]
pub enum HexError {
    #[error("{path}: {source}")]
    Io {
        path: String,
        #[source]
        source: io::Error,
    },
    #[error("{0}: not a file we can edit in place")]
    NotAFile(String),
    #[error("nothing to save to: this buffer has no path")]
    NoPath,
    #[error("{0} is a device: write the bytes out to a file instead")]
    IsADevice(String),
}

type Result<T> = std::result::Result<T, HexError>;

/// How much of an edit the undo stack keeps. A snapshot is the piece list, which is small —
/// one entry per edit boundary, not per byte — so a long session costs kilobytes.
const UNDO_DEPTH: usize = 512;

#[derive(Debug, Clone)]
struct Snapshot {
    pieces: Vec<Piece>,
    len: u64,
    cursor: u64,
}

pub struct HexEdit {
    pub path: Option<VPath>,
    /// The file the original pieces are read from. `None` for a buffer built from bytes.
    file: Option<File>,
    memory: Vec<u8>,
    pieces: Vec<Piece>,
    added: Vec<u8>,
    len: u64,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    saved_at: usize,
    pub cursor: u64,
    pub pane: Pane,
    /// The high nibble of a byte has been typed and the low one is awaited.
    pub half: Option<u8>,
    /// Overwrite is the default: in a binary a shifted byte usually breaks the file.
    pub insert: bool,
    pub bytes_per_row: usize,
    /// First visible offset, always a multiple of `bytes_per_row`.
    pub top: u64,
    /// The marked range, `F3` to set and `F12` to operate on (SPEC 05 §2.6).
    pub block: Option<(u64, u64)>,
    /// Where a block was started, while it is being marked.
    pub mark: Option<u64>,
    pub read_only: bool,
    /// A raw device rather than a file (SPEC 05 §1.10). It opens, it is read, it is searched
    /// — and it is never written to from here: the atomic save writes a new file beside the
    /// old one and renames it over the top, which is not a thing one can do to a disk.
    pub device: bool,
}

impl std::fmt::Debug for HexEdit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HexEdit")
            .field("path", &self.path)
            .field("len", &self.len)
            .field("cursor", &self.cursor)
            .field("pieces", &self.pieces.len())
            .field("modified", &self.modified())
            .finish_non_exhaustive()
    }
}

impl HexEdit {
    /// Open a file for editing. Nothing is read here but its length: the bytes come in as the
    /// window needs them.
    pub fn open(path: &Path) -> Result<HexEdit> {
        let name = path.display().to_string();
        let meta = std::fs::metadata(path).map_err(|e| HexError::Io {
            path: name.clone(),
            source: e,
        })?;
        if meta.is_dir() {
            return Err(HexError::NotAFile(name));
        }
        // A device is opened read-only whatever its permissions say.
        let device = fcmd_core::is_device(&meta);
        let read_only = device || meta.permissions().readonly();
        let mut file = File::open(path).map_err(|e| HexError::Io {
            path: name,
            source: e,
        })?;
        // A block device's metadata says it is zero bytes long; its length has to be measured.
        let len = fcmd_core::device::length(&mut file, &meta).bytes;
        Ok(HexEdit {
            path: Some(VPath::local(path)),
            file: Some(file),
            memory: Vec::new(),
            pieces: if len == 0 {
                Vec::new()
            } else {
                vec![Piece {
                    source: Source::Original,
                    at: 0,
                    len,
                }]
            },
            added: Vec::new(),
            len,
            undo: Vec::new(),
            redo: Vec::new(),
            saved_at: 0,
            cursor: 0,
            pane: Pane::default(),
            half: None,
            insert: false,
            bytes_per_row: 16,
            top: 0,
            block: None,
            mark: None,
            read_only,
            device,
        })
    }

    /// A buffer over bytes we already have — for a test, and for an entry read out of an
    /// archive or off a server.
    pub fn from_bytes(bytes: Vec<u8>, path: Option<VPath>) -> HexEdit {
        let len = bytes.len() as u64;
        HexEdit {
            path,
            file: None,
            memory: bytes,
            pieces: if len == 0 {
                Vec::new()
            } else {
                vec![Piece {
                    source: Source::Original,
                    at: 0,
                    len,
                }]
            },
            added: Vec::new(),
            len,
            undo: Vec::new(),
            redo: Vec::new(),
            saved_at: 0,
            cursor: 0,
            pane: Pane::default(),
            half: None,
            insert: false,
            bytes_per_row: 16,
            top: 0,
            block: None,
            mark: None,
            read_only: false,
            device: false,
        }
    }

    pub fn len(&self) -> u64 {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn modified(&self) -> bool {
        self.undo.len() != self.saved_at
    }

    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .map(|p| p.display_name())
            .unwrap_or_else(|| "untitled".to_string())
    }

    // ── reading ────────────────────────────────────────────────────────────────────────────

    /// `len` bytes from `at`, or as many as there are.
    pub fn read(&mut self, at: u64, len: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(len);
        let mut pos = 0u64;
        let end = at.saturating_add(len as u64);
        // The pieces are in order, so one pass over them fills the window.
        for i in 0..self.pieces.len() {
            let piece = self.pieces[i];
            let piece_end = pos + piece.len;
            if piece_end <= at {
                pos = piece_end;
                continue;
            }
            if pos >= end {
                break;
            }
            let skip = at.saturating_sub(pos);
            let take = (piece.len - skip).min(end - pos.max(at));
            self.read_piece(&piece, skip, take, &mut out);
            pos = piece_end;
        }
        out
    }

    fn read_piece(&mut self, piece: &Piece, skip: u64, take: u64, out: &mut Vec<u8>) {
        let from = piece.at + skip;
        match piece.source {
            Source::Added => {
                let a = from as usize;
                let b = (from + take) as usize;
                out.extend_from_slice(
                    &self.added[a.min(self.added.len())..b.min(self.added.len())],
                );
            }
            Source::Original => {
                if let Some(file) = self.file.as_mut() {
                    let mut buf = vec![0u8; take as usize];
                    if file.seek(SeekFrom::Start(from)).is_ok() {
                        // A short read means the file shrank under us; what we got is what
                        // there is, and the editor shows that rather than guessing.
                        let mut filled = 0;
                        while filled < buf.len() {
                            match file.read(&mut buf[filled..]) {
                                Ok(0) | Err(_) => break,
                                Ok(n) => filled += n,
                            }
                        }
                        buf.truncate(filled);
                        out.extend_from_slice(&buf);
                    }
                } else {
                    let a = from as usize;
                    let b = (from + take) as usize;
                    out.extend_from_slice(
                        &self.memory[a.min(self.memory.len())..b.min(self.memory.len())],
                    );
                }
            }
        }
    }

    /// The byte at `at`, if there is one.
    pub fn byte(&mut self, at: u64) -> Option<u8> {
        self.read(at, 1).first().copied()
    }

    // ── editing ────────────────────────────────────────────────────────────────────────────

    /// The one primitive: take `remove` bytes out at `at` and put `bytes` in their place.
    /// Everything else — overwrite, insert, delete, fill, a block operation — is this.
    pub fn splice(&mut self, at: u64, remove: u64, bytes: &[u8]) {
        if self.read_only {
            return;
        }
        let at = at.min(self.len);
        let remove = remove.min(self.len - at);
        if remove == 0 && bytes.is_empty() {
            return;
        }
        self.remember();

        let mut out: Vec<Piece> = Vec::with_capacity(self.pieces.len() + 2);
        let mut pos = 0u64;
        let end = at + remove;
        for piece in &self.pieces {
            let piece_end = pos + piece.len;
            // Wholly before, or wholly after, the stretch being replaced.
            if piece_end <= at || pos >= end {
                out.push(*piece);
                pos = piece_end;
                continue;
            }
            // The head of this piece survives.
            if pos < at {
                out.push(Piece {
                    source: piece.source,
                    at: piece.at,
                    len: at - pos,
                });
            }
            // And the tail.
            if piece_end > end {
                let cut = end - pos;
                out.push(Piece {
                    source: piece.source,
                    at: piece.at + cut,
                    len: piece.len - cut,
                });
            }
            pos = piece_end;
        }

        if !bytes.is_empty() {
            let at_in_added = self.added.len() as u64;
            self.added.extend_from_slice(bytes);
            let new = Piece {
                source: Source::Added,
                at: at_in_added,
                len: bytes.len() as u64,
            };
            // Put it where the removed stretch was.
            let mut pos = 0u64;
            let mut index = out.len();
            for (i, piece) in out.iter().enumerate() {
                if pos >= at {
                    index = i;
                    break;
                }
                pos += piece.len;
            }
            out.insert(index, new);
        }

        self.pieces = out;
        self.len = self.len - remove + bytes.len() as u64;
        self.cursor = self.cursor.min(self.len);
        self.redo.clear();
    }

    /// Overwrite the byte at `at`, which is what typing does unless insert mode is on.
    pub fn set_byte(&mut self, at: u64, byte: u8) {
        if at >= self.len {
            // Past the end: typing extends the file rather than refusing, as FAR's and
            // Hiew's do, and the length is shown so it is never a surprise.
            self.splice(self.len, 0, &[byte]);
            return;
        }
        self.splice(at, 1, &[byte]);
    }

    pub fn insert_bytes(&mut self, at: u64, bytes: &[u8]) {
        self.splice(at, 0, bytes);
    }

    pub fn delete(&mut self, at: u64, len: u64) {
        self.splice(at, len, &[]);
    }

    /// Type one hex digit into the byte under the cursor: the first makes the high nibble,
    /// the second completes the byte and moves on.
    pub fn type_hex(&mut self, digit: char) -> bool {
        let Some(value) = digit.to_digit(16) else {
            return false;
        };
        let value = value as u8;
        match self.half.take() {
            None => {
                let low = self.byte(self.cursor).unwrap_or(0) & 0x0f;
                let byte = (value << 4) | if self.insert { 0 } else { low };
                if self.insert {
                    self.insert_bytes(self.cursor, &[byte]);
                } else {
                    self.set_byte(self.cursor, byte);
                }
                self.half = Some(value << 4);
            }
            Some(high) => {
                self.set_byte(self.cursor, high | value);
                self.cursor = (self.cursor + 1).min(self.len);
            }
        }
        true
    }

    /// Type a character into the text pane: one byte, one character, no re-encoding.
    pub fn type_char(&mut self, c: char) -> bool {
        if !c.is_ascii() {
            // A byte is a byte: anything that is not one character of one byte would have to
            // guess an encoding, and this is the pane where guessing is not allowed.
            return false;
        }
        self.half = None;
        let byte = c as u8;
        if self.insert {
            self.insert_bytes(self.cursor, &[byte]);
        } else {
            self.set_byte(self.cursor, byte);
        }
        self.cursor = (self.cursor + 1).min(self.len);
        true
    }

    // ── undo ───────────────────────────────────────────────────────────────────────────────

    fn remember(&mut self) {
        self.undo.push(Snapshot {
            pieces: self.pieces.clone(),
            len: self.len,
            cursor: self.cursor,
        });
        if self.undo.len() > UNDO_DEPTH {
            self.undo.remove(0);
            self.saved_at = self.saved_at.saturating_sub(1);
        }
    }

    pub fn undo(&mut self) -> bool {
        let Some(previous) = self.undo.pop() else {
            return false;
        };
        self.redo.push(Snapshot {
            pieces: std::mem::replace(&mut self.pieces, previous.pieces),
            len: self.len,
            cursor: self.cursor,
        });
        self.len = previous.len;
        self.cursor = previous.cursor.min(self.len);
        self.half = None;
        true
    }

    pub fn redo(&mut self) -> bool {
        let Some(next) = self.redo.pop() else {
            return false;
        };
        self.undo.push(Snapshot {
            pieces: std::mem::replace(&mut self.pieces, next.pieces),
            len: self.len,
            cursor: self.cursor,
        });
        self.len = next.len;
        self.cursor = next.cursor.min(self.len);
        self.half = None;
        true
    }

    // ── what has changed ───────────────────────────────────────────────────────────────────

    /// Is the byte at `at` one the user has changed? The piece it is in answers that without
    /// looking at the file, and keeps answering after an insertion.
    pub fn is_changed(&self, at: u64) -> bool {
        let mut pos = 0u64;
        for piece in &self.pieces {
            let end = pos + piece.len;
            if at < end {
                return piece.source == Source::Added;
            }
            pos = end;
        }
        false
    }

    /// The patch list of SPEC 05 §2.5: every run that differs from the file on disk, with the
    /// bytes that were there and the bytes that are there now.
    pub fn changes(&mut self) -> Vec<Change> {
        let mut out: Vec<Change> = Vec::new();
        let mut pos = 0u64; // where we are in the content as it is now
        let mut original = 0u64; // where we are in the file as it is on disk
        let pieces = self.pieces.clone();
        let mut i = 0;
        while i < pieces.len() {
            let piece = pieces[i];
            match piece.source {
                Source::Original => {
                    if piece.at > original {
                        // The original bytes between where we were and where this piece
                        // starts are gone: a deletion.
                        let gap = piece.at - original;
                        let old = self.read_original(original, gap);
                        out.push(Change {
                            at: pos,
                            old,
                            new: Vec::new(),
                        });
                    }
                    original = piece.at + piece.len;
                    pos += piece.len;
                    i += 1;
                }
                Source::Added => {
                    // A run of typed bytes, however many pieces it is in.
                    let start = pos;
                    let mut new = Vec::new();
                    while i < pieces.len() && pieces[i].source == Source::Added {
                        let p = pieces[i];
                        let a = p.at as usize;
                        let b = (p.at + p.len) as usize;
                        new.extend_from_slice(&self.added[a..b]);
                        pos += p.len;
                        i += 1;
                    }
                    // What it replaced: the original bytes up to where the next original
                    // piece picks up.
                    let next = pieces[i..]
                        .iter()
                        .find(|p| p.source == Source::Original)
                        .map(|p| p.at)
                        .unwrap_or(self.original_len());
                    let old = if next > original {
                        let old = self.read_original(original, next - original);
                        original = next;
                        old
                    } else {
                        Vec::new()
                    };
                    out.push(Change {
                        at: start,
                        old,
                        new,
                    });
                }
            }
        }
        // Anything left of the original after the last piece was cut off the end.
        let total = self.original_len();
        if original < total {
            let old = self.read_original(original, total - original);
            out.push(Change {
                at: pos,
                old,
                new: Vec::new(),
            });
        }
        out
    }

    fn original_len(&self) -> u64 {
        match self.file.as_ref() {
            Some(file) => file.metadata().map(|m| m.len()).unwrap_or(0),
            None => self.memory.len() as u64,
        }
    }

    fn read_original(&mut self, at: u64, len: u64) -> Vec<u8> {
        // Capped: a patch list is for reading, and nobody reads a megabyte of it.
        let len = len.min(4096);
        let piece = Piece {
            source: Source::Original,
            at,
            len,
        };
        let mut out = Vec::new();
        self.read_piece(&piece, 0, len, &mut out);
        out
    }

    // ── saving ─────────────────────────────────────────────────────────────────────────────

    /// Write the bytes out through a sibling temporary file and rename it into place, so an
    /// interrupted save never leaves half a binary where a whole one was.
    pub fn save(&mut self, to: Option<&Path>) -> Result<u64> {
        let target: PathBuf = match (to, self.path.as_ref()) {
            (Some(path), _) => path.to_path_buf(),
            (None, Some(path)) => path.as_path().to_path_buf(),
            (None, None) => return Err(HexError::NoPath),
        };
        let name = target.display().to_string();
        // Saving in place writes a new file beside the old one and renames it over the top.
        // There is no such move onto a disk, so a device is written out to a file or not at
        // all — which is also the only way to be sure nobody loses a partition by habit.
        if self.device && to.is_none() {
            return Err(HexError::IsADevice(name));
        }
        let part = target.with_extension(format!(
            "{}.fcmd-part",
            target
                .extension()
                .map(|e| e.to_string_lossy())
                .unwrap_or_default()
        ));

        let written = {
            let file = File::create(&part).map_err(|e| HexError::Io {
                path: part.display().to_string(),
                source: e,
            })?;
            let mut out = io::BufWriter::new(file);
            let mut written = 0u64;
            let mut at = 0u64;
            while at < self.len {
                let chunk = self.read(at, 256 * 1024);
                if chunk.is_empty() {
                    break;
                }
                out.write_all(&chunk).map_err(|e| HexError::Io {
                    path: part.display().to_string(),
                    source: e,
                })?;
                at += chunk.len() as u64;
                written += chunk.len() as u64;
            }
            // The bytes have to be on the device before the rename, or a crash between the
            // two leaves an empty file where the original was.
            out.flush()
                .and_then(|()| out.get_ref().sync_all())
                .map_err(|e| HexError::Io {
                    path: part.display().to_string(),
                    source: e,
                })?;
            written
        };

        // The file we were reading from has to let go before Windows will rename over it.
        self.file = None;
        std::fs::rename(&part, &target).map_err(|e| {
            let _ = std::fs::remove_file(&part);
            HexError::Io {
                path: name.clone(),
                source: e,
            }
        })?;

        // Start again from the file as it now is: every piece is original, nothing is changed.
        let file = File::open(&target).map_err(|e| HexError::Io {
            path: name,
            source: e,
        })?;
        self.file = Some(file);
        self.memory = Vec::new();
        self.added = Vec::new();
        self.pieces = if self.len == 0 {
            Vec::new()
        } else {
            vec![Piece {
                source: Source::Original,
                at: 0,
                len: self.len,
            }]
        };
        self.undo.clear();
        self.redo.clear();
        self.saved_at = 0;
        self.path = Some(VPath::local(&target));
        Ok(written)
    }

    // ── the window ─────────────────────────────────────────────────────────────────────────

    /// Keep the cursor on screen for a window of `rows` rows.
    pub fn scroll_into_view(&mut self, rows: usize) {
        let per = self.bytes_per_row.max(1) as u64;
        let row = self.cursor / per;
        let top_row = self.top / per;
        let rows = rows.max(1) as u64;
        let top_row = if row < top_row {
            row
        } else if row >= top_row + rows {
            row + 1 - rows
        } else {
            top_row
        };
        self.top = top_row * per;
    }

    /// Move the cursor by whole bytes, clamped to the file.
    pub fn move_cursor(&mut self, delta: i64) {
        self.half = None;
        let at = self.cursor as i64 + delta;
        self.cursor = at.clamp(0, self.len as i64) as u64;
        if let Some(mark) = self.mark {
            self.block = Some((mark.min(self.cursor), mark.max(self.cursor)));
        }
    }

    /// `F3`: start marking a block here, or finish the one being marked.
    pub fn toggle_mark(&mut self) {
        match self.mark.take() {
            Some(start) => {
                let (a, b) = (start.min(self.cursor), start.max(self.cursor));
                self.block = (a != b).then_some((a, b));
            }
            None => {
                self.mark = Some(self.cursor);
                self.block = None;
            }
        }
    }

    pub fn clear_block(&mut self) {
        self.block = None;
        self.mark = None;
    }

    /// The block as bytes, for the operations that carve one out.
    pub fn block_bytes(&mut self) -> Option<Vec<u8>> {
        let (a, b) = self.block?;
        Some(self.read(a, (b - a) as usize))
    }
}

#[cfg(test)]
mod tests {

    /// A raw device (SPEC 05 §1.10). A disk needs permissions CI does not have, but
    /// `/dev/null` is the same kind of thing — a character device whose metadata says zero —
    /// and it proves the two rules that matter: it opens, and it cannot be saved over.
    #[test]
    #[cfg(unix)]
    fn a_device_opens_read_only_and_refuses_to_be_saved_over() {
        let path = Path::new("/dev/null");
        if !path.exists() {
            return;
        }
        let Ok(mut editor) = HexEdit::open(path) else {
            return; // a build environment without /dev is not a failure of this code
        };
        assert!(editor.device, "a character device must be recognised");
        assert!(
            editor.read_only,
            "a device opens read-only whatever its mode"
        );

        match editor.save(None) {
            Err(HexError::IsADevice(name)) => assert!(name.contains("null"), "{name}"),
            other => panic!("saving over a device must be refused, got {other:?}"),
        }

        // Writing the bytes out somewhere else is allowed: that is how a device is dumped.
        let dir = tempfile::tempdir().unwrap();
        let out = dir.path().join("dump.bin");
        assert!(editor.save(Some(&out)).is_ok());
        assert!(out.exists());
    }

    use super::*;

    fn edit(bytes: &[u8]) -> HexEdit {
        HexEdit::from_bytes(bytes.to_vec(), None)
    }

    fn all(edit: &mut HexEdit) -> Vec<u8> {
        let len = edit.len() as usize;
        edit.read(0, len)
    }

    #[test]
    fn overwriting_a_byte_changes_that_byte_and_nothing_else() {
        let mut e = edit(b"hello world");
        e.set_byte(4, b'O');
        assert_eq!(all(&mut e), b"hellO world");
        assert_eq!(e.len(), 11);
        assert!(e.is_changed(4));
        assert!(!e.is_changed(3) && !e.is_changed(5));
        assert!(e.modified());
    }

    #[test]
    fn inserting_and_deleting_move_everything_after_them() {
        let mut e = edit(b"hello world");
        e.insert_bytes(5, b", dear");
        assert_eq!(all(&mut e), b"hello, dear world");
        assert_eq!(e.len(), 17);
        // The inserted run is what changed; the text after it is still the file's own.
        assert!(e.is_changed(5) && e.is_changed(10));
        assert!(!e.is_changed(11));

        e.delete(0, 6);
        assert_eq!(all(&mut e), b" dear world");
        assert_eq!(e.len(), 11);
    }

    #[test]
    fn a_window_reads_the_same_bytes_whatever_the_pieces_look_like() {
        let mut e = edit(b"0123456789abcdef");
        e.set_byte(2, b'X');
        e.insert_bytes(8, b"--");
        e.delete(0, 1);
        let whole = all(&mut e);
        assert_eq!(whole, b"1X34567--89abcdef");
        // Every window of it agrees with the whole.
        for at in 0..whole.len() {
            for len in 0..(whole.len() - at) {
                assert_eq!(e.read(at as u64, len), whole[at..at + len], "{at}+{len}");
            }
        }
        // And past the end is empty rather than an error.
        assert!(e.read(e.len(), 16).is_empty());
    }

    #[test]
    fn undo_takes_back_one_edit_at_a_time_and_redo_puts_it_again() {
        let mut e = edit(b"abcd");
        e.set_byte(0, b'X');
        e.set_byte(3, b'Y');
        assert_eq!(all(&mut e), b"XbcY");

        assert!(e.undo());
        assert_eq!(all(&mut e), b"Xbcd");
        assert!(e.undo());
        assert_eq!(all(&mut e), b"abcd");
        assert!(!e.undo(), "nothing left to take back");
        assert!(!e.modified(), "back where it started");

        assert!(e.redo());
        assert_eq!(all(&mut e), b"Xbcd");
        assert!(e.modified());
        // A new edit forgets the redo, as everywhere else.
        e.set_byte(1, b'Z');
        assert!(!e.redo());
        assert_eq!(all(&mut e), b"XZcd");
    }

    #[test]
    fn typing_hex_takes_two_digits_and_then_moves_on() {
        let mut e = edit(b"\x00\x00\x00");
        assert!(e.type_hex('4'));
        assert_eq!(e.byte(0), Some(0x40), "the high nibble lands first");
        assert_eq!(e.cursor, 0, "and the cursor waits for the other half");
        assert!(e.type_hex('1'));
        assert_eq!(e.byte(0), Some(0x41));
        assert_eq!(e.cursor, 1);
        assert!(!e.type_hex('q'), "not a hex digit");

        // The text pane writes the byte whole.
        e.pane = Pane::Text;
        assert!(e.type_char('B'));
        assert_eq!(e.byte(1), Some(b'B'));
        assert!(!e.type_char('è'), "one byte per character, or nothing");
    }

    #[test]
    fn typing_past_the_end_grows_the_file_rather_than_refusing() {
        let mut e = edit(b"ab");
        e.cursor = 2;
        e.type_char('c');
        assert_eq!(all(&mut e), b"abc");
        assert_eq!(e.len(), 3);
    }

    /// The patch list describes the difference from the file on disk, not the keystrokes
    /// that got there: two edits that end up touching the same stretch are one line, because
    /// that is what someone checking a patched binary needs to read.
    #[test]
    fn the_patch_list_says_what_was_there_and_what_is_there_now() {
        let mut e = edit(b"hello world");
        e.set_byte(0, b'H');
        e.insert_bytes(5, b"!!");
        e.delete(7, 1); // the space, which is at 7 now that two bytes went in before it
        assert_eq!(all(&mut e), b"Hello!!world");

        let changes = e.changes();
        assert_eq!(changes.len(), 2, "{changes:?}");
        assert_eq!(
            changes[0],
            Change {
                at: 0,
                old: b"h".to_vec(),
                new: b"H".to_vec()
            }
        );
        // The insertion and the deletion met: the space became the two marks.
        assert_eq!(
            changes[1],
            Change {
                at: 5,
                old: b" ".to_vec(),
                new: b"!!".to_vec()
            }
        );

        // On their own they read as what they are.
        let mut inserted = edit(b"abcd");
        inserted.insert_bytes(2, b"XY");
        let changes = inserted.changes();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_insert() && changes[0].new == b"XY".to_vec());

        let mut deleted = edit(b"abcd");
        deleted.delete(1, 2);
        let changes = deleted.changes();
        assert_eq!(changes.len(), 1);
        assert!(changes[0].is_delete() && changes[0].old == b"bc".to_vec());

        // A file nobody has touched has nothing to list.
        let mut clean = edit(b"abc");
        assert!(clean.changes().is_empty());
    }

    #[test]
    fn saving_writes_the_bytes_through_a_temporary_file_and_starts_again_clean() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("binary.bin");
        std::fs::write(&path, b"\x00\x01\x02\x03").unwrap();

        let mut e = HexEdit::open(&path).expect("open");
        e.set_byte(1, 0xff);
        e.insert_bytes(4, b"\xaa\xbb");
        assert!(e.modified());

        let written = e.save(None).expect("save");
        assert_eq!(written, 6);
        assert_eq!(std::fs::read(&path).unwrap(), b"\x00\xff\x02\x03\xaa\xbb");
        assert!(!e.modified(), "a saved file is not modified");
        assert!(e.changes().is_empty(), "and has nothing left to patch");
        assert!(
            !dir.path().join("binary.bin.fcmd-part").exists(),
            "the temporary file was left behind"
        );

        // It can go on being edited from the file it just wrote.
        e.set_byte(0, 0x7f);
        assert_eq!(all(&mut e), b"\x7f\xff\x02\x03\xaa\xbb");
    }

    #[test]
    fn a_block_is_marked_between_two_presses_and_can_be_read_out() {
        let mut e = edit(b"0123456789");
        e.cursor = 2;
        e.toggle_mark();
        e.move_cursor(4);
        assert_eq!(e.block, Some((2, 6)), "it grows as the cursor moves");
        e.toggle_mark();
        assert_eq!(e.block, Some((2, 6)));
        assert_eq!(e.block_bytes(), Some(b"2345".to_vec()));

        e.clear_block();
        assert_eq!(e.block, None);
        assert_eq!(e.block_bytes(), None);
    }

    #[test]
    fn the_window_follows_the_cursor_a_row_at_a_time() {
        let mut e = edit(&vec![0u8; 1024]);
        e.bytes_per_row = 16;
        e.cursor = 600;
        e.scroll_into_view(10);
        assert_eq!(e.top % 16, 0);
        assert!(e.top <= 600 && 600 < e.top + 160);

        e.cursor = 0;
        e.scroll_into_view(10);
        assert_eq!(e.top, 0);
    }

    #[test]
    fn a_file_that_is_read_only_refuses_the_edit_rather_than_losing_it_at_the_save() {
        let mut e = edit(b"abc");
        e.read_only = true;
        e.set_byte(0, b'X');
        assert_eq!(all(&mut e), b"abc");
        assert!(!e.modified());
    }
}
