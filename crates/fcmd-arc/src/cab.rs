//! Microsoft cabinets (`.cab`), read here.
//!
//! The format is from 1996 and documented by its author (`[MS-CAB]`): a header, a table of
//! *folders*, a table of files, and then each folder's data as a run of blocks. A folder is
//! not a directory — it is a compression unit, one stream that several files share, so a
//! file is "these many bytes, this far into that folder". Reading the last file of a folder
//! means unpacking the folder up to it, which is a solid archive's bargain and the reason
//! [`crate::Format::seekable`] says no.
//!
//! A block unpacks to at most 32 KiB and carries a checksum of its packed bytes, which is
//! what makes testing a cabinet mean something. How it is packed is the folder's choice of
//! four:
//!
//! * **stored** — nothing to do;
//! * **MSZIP** — deflate, each block a complete deflate stream after the letters `CK`,
//!   with the one twist that a block may refer back into the 32 KiB before it. That is the
//!   whole of the format's difficulty, and it is answered here without a decompressor of our
//!   own: see [`inflate_block`];
//! * **Quantum** and **LZX** — two compressors nothing in this workspace implements. A
//!   cabinet packed with either is still *listed* here, since the tables are not compressed,
//!   and reading an entry out of it says which compressor it wants; where a program on the
//!   machine reads cabinets (`7z`, by the shipped table), the whole archive is handed to that
//!   instead (`Cache::read`).
//!
//! A cabinet can also be one of a *set*, with a file running off the end of one into the
//! start of the next. The files wholly inside this cabinet are read; one that crosses is
//! listed and refused by name, for the same program to deal with.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, SystemTime};

use fcmd_core::{CancelToken, EntryKind};

use crate::archive::{days_from_civil, normalise, ArcEntry};
use crate::{ArcError, Limits, Result};

/// What a cabinet starts with.
pub(crate) const MAGIC: &[u8] = b"MSCF";

const STORED: u16 = 0;
const MSZIP: u16 = 1;
const QUANTUM: u16 = 2;
const LZX: u16 = 3;

/// The most a block unpacks to, and so the most a later block can reach back.
const WINDOW: usize = 32 * 1024;

/// One folder: where its blocks start, how many there are, and how they are packed.
#[derive(Debug, Clone, Copy)]
struct Folder {
    start: u64,
    blocks: u16,
    method: u16,
}

impl Folder {
    /// The compressor this folder wants and this crate has not got.
    fn beyond(&self) -> Option<&'static str> {
        match self.method & 0x000f {
            STORED | MSZIP => None,
            QUANTUM => Some("Quantum"),
            LZX => Some("LZX"),
            _ => Some("a compressor nobody documented"),
        }
    }
}

/// The header and the folder table: everything needed to read a folder's data.
struct Tables {
    folders: Vec<Folder>,
    /// Bytes each block carries between its header and its data, for whoever made the
    /// cabinet to use (a signature, usually). Skipped.
    block_reserve: usize,
    files_at: u64,
    files: u16,
}

fn broken(name: &str, reason: impl std::fmt::Display) -> ArcError {
    ArcError::Broken {
        path: name.to_string(),
        reason: reason.to_string(),
    }
}

fn u16_at(bytes: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([bytes[at], bytes[at + 1]])
}

fn u32_at(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// A string that ends at a zero byte, as the names in a cabinet do. Bounded: the format
/// allows 256 bytes and a file that goes on past that is not a cabinet.
fn c_string(file: &mut impl Read) -> io::Result<Vec<u8>> {
    let mut out = Vec::new();
    let mut byte = [0u8; 1];
    loop {
        file.read_exact(&mut byte)?;
        if byte[0] == 0 {
            return Ok(out);
        }
        if out.len() >= 1024 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "a name that never ends",
            ));
        }
        out.push(byte[0]);
    }
}

fn tables(file: &mut (impl Read + Seek), name: &str) -> Result<Tables> {
    let cut = |e: io::Error| broken(name, format!("the header is cut short ({e})"));
    let mut head = [0u8; 36];
    file.read_exact(&mut head).map_err(cut)?;
    if &head[..4] != MAGIC {
        return Err(ArcError::Unknown(name.to_string()));
    }
    let files_at = u64::from(u32_at(&head, 16));
    let folders = u16_at(&head, 26);
    let files = u16_at(&head, 28);
    let flags = u16_at(&head, 30);
    const PREV: u16 = 1;
    const NEXT: u16 = 2;
    const RESERVE: u16 = 4;

    let (mut folder_reserve, mut block_reserve) = (0usize, 0usize);
    if flags & RESERVE != 0 {
        let mut sizes = [0u8; 4];
        file.read_exact(&mut sizes).map_err(cut)?;
        let header_reserve = i64::from(u16_at(&sizes, 0));
        folder_reserve = usize::from(sizes[2]);
        block_reserve = usize::from(sizes[3]);
        file.seek(SeekFrom::Current(header_reserve)).map_err(cut)?;
    }
    // The names of the cabinets and disks on either side, which nothing here follows.
    for flag in [PREV, NEXT] {
        if flags & flag != 0 {
            c_string(file).map_err(cut)?;
            c_string(file).map_err(cut)?;
        }
    }
    let mut table = Vec::with_capacity(usize::from(folders));
    for _ in 0..folders {
        let mut entry = [0u8; 8];
        file.read_exact(&mut entry).map_err(cut)?;
        file.seek(SeekFrom::Current(folder_reserve as i64))
            .map_err(cut)?;
        table.push(Folder {
            start: u64::from(u32_at(&entry, 0)),
            blocks: u16_at(&entry, 4),
            method: u16_at(&entry, 6),
        });
    }
    Ok(Tables {
        folders: table,
        block_reserve,
        files_at,
        files,
    })
}

/// What an index of a cabinet is: its entries, and — when this crate cannot read all of
/// them — why not, in words for the reader.
pub(crate) struct Index {
    pub(crate) entries: Vec<ArcEntry>,
    pub(crate) beyond: Option<String>,
}

/// Where an entry's bytes are: which folder, and how far into what it unpacks to. Packed
/// into the one number an [`ArcEntry`] keeps.
fn place(folder: usize, offset: u32) -> u64 {
    ((folder as u64) << 32) | u64::from(offset)
}

/// And back: which folder an entry is in, and how far into it.
pub(crate) fn folder_of(at: u64) -> (usize, u64) {
    ((at >> 32) as usize, at & 0xffff_ffff)
}

/// A file's date and time, which a cabinet keeps the way DOS did: two-second steps, no
/// zone. Read as UTC, as a zip's are here.
fn dos_time(date: u16, time: u16) -> Option<SystemTime> {
    let (year, month, day) = (
        1980 + i64::from(date >> 9),
        u32::from((date >> 5) & 0x0f),
        u32::from(date & 0x1f),
    );
    let (hour, minute, second) = (
        i64::from(time >> 11),
        i64::from((time >> 5) & 0x3f),
        i64::from(time & 0x1f) * 2,
    );
    if hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day)?;
    let secs = days * 86_400 + hour * 3_600 + minute * 60 + second;
    (secs >= 0).then(|| SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64))
}

/// The index of a cabinet: its file table, which is not compressed.
pub(crate) fn index(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Index> {
    let io = |e: io::Error| ArcError::Io {
        path: name.to_string(),
        source: e,
    };
    let mut file = BufReader::new(File::open(path).map_err(io)?);
    let tables = tables(&mut file, name)?;
    if usize::from(tables.files) > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!(
                "{} entries, over the limit of {}",
                tables.files, limits.entries
            ),
        });
    }
    let cut = |e: io::Error| broken(name, format!("the file table is cut short ({e})"));
    file.seek(SeekFrom::Start(tables.files_at)).map_err(cut)?;

    let mut beyond: Option<String> = None;
    let mut entries = Vec::with_capacity(usize::from(tables.files));
    for _ in 0..tables.files {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let mut entry = [0u8; 16];
        file.read_exact(&mut entry).map_err(cut)?;
        let size = u32_at(&entry, 0);
        let offset = u32_at(&entry, 4);
        let folder = u16_at(&entry, 8);
        let (date, time, attributes) = (u16_at(&entry, 10), u16_at(&entry, 12), u16_at(&entry, 14));
        let raw = c_string(&mut file).map_err(cut)?;
        // Bit 7 of the attributes says the name is UTF-8. Otherwise it is in whatever code
        // page the machine that made the cabinet was set to, which the cabinet does not
        // record; the Western one is the commonest and reads every byte as something.
        const NAME_IS_UTF8: u16 = 0x80;
        let text = if attributes & NAME_IS_UTF8 != 0 {
            String::from_utf8_lossy(&raw).into_owned()
        } else {
            encoding_rs::WINDOWS_1252.decode(&raw).0.into_owned()
        };
        let inner = normalise(&text);
        if inner == "/" {
            continue;
        }
        // The three highest folder numbers are not folders: they say the file runs in from
        // the cabinet before this one, out into the one after, or both.
        let at = match folder {
            0xfffd..=0xffff => {
                beyond.get_or_insert_with(|| {
                    "it is one cabinet of a set, and a file that runs from one cabinet into \
                     the next is not put together here"
                        .to_string()
                });
                None
            }
            n => {
                let Some(found) = tables.folders.get(usize::from(n)) else {
                    return Err(broken(
                        name,
                        format!("{text} is in folder {n}, which is not there"),
                    ));
                };
                if let Some(compressor) = found.beyond() {
                    beyond.get_or_insert_with(|| {
                        format!("it is packed with {compressor}, which is not unpacked here")
                    });
                }
                Some(place(usize::from(n), offset))
            }
        };
        entries.push(ArcEntry {
            path: inner,
            size: u64::from(size),
            // A folder is packed as one stream: no file in it has a packed size of its own.
            packed: None,
            mtime: dos_time(date, time),
            // DOS attributes, not permission bits, and nothing here invents the one from
            // the other.
            mode: None,
            kind: EntryKind::File,
            link_target: None,
            at,
        });
    }
    Ok(Index { entries, beyond })
}

/// The checksum a block carries, as `[MS-CAB]` gives it: the bytes taken four at a time as
/// little-endian words and XORed together, the one to three left over packed the other way
/// round — the first of them highest.
fn checksum(bytes: &[u8], seed: u32) -> u32 {
    let mut sum = seed;
    let (words, rest) = bytes.as_chunks::<4>();
    for word in words {
        sum ^= u32::from_le_bytes(*word);
    }
    let mut last = 0u32;
    for &byte in rest {
        last = (last << 8) | u32::from(byte);
    }
    sum ^ last
}

/// One MSZIP block unpacked, given the bytes that came out before it.
///
/// A block is a complete deflate stream — it ends with a final block of its own — but its
/// matches may reach back into what the block before it unpacked to, as if the two were one
/// stream. A decompressor wants to be told that history as a preset dictionary, which the
/// pure-Rust one under `flate2` cannot be. So the history is *shown* to it instead: a deflate
/// stream may begin with a stored block, which is its bytes copied as they are, and one
/// holding the history, put in front of the real block, leaves the decompressor's window
/// exactly as the format assumes it to be. What comes out is the history again followed by
/// the block, and the first part is dropped. The history is at most 32 KiB and a stored block
/// holds up to 64, so one always fits.
fn inflate_block(packed: &[u8], history: &[u8], expect: usize) -> io::Result<Vec<u8>> {
    let bad = |reason: &str| io::Error::new(io::ErrorKind::InvalidData, reason.to_string());
    let Some(deflated) = packed.strip_prefix(b"CK") else {
        return Err(bad("a block does not begin with MSZIP's `CK`"));
    };
    let len = history.len() as u16;
    // Not the last block (bit 0 clear), stored (the next two clear), then its length and the
    // length's complement.
    let [lo, hi] = len.to_le_bytes();
    let [nlo, nhi] = (!len).to_le_bytes();
    let lead = [0u8, lo, hi, nlo, nhi];
    let stream = lead[..].chain(history).chain(deflated);
    let mut out = Vec::with_capacity(history.len() + expect);
    // One byte more than there should be is enough to know there are too many.
    flate2::read::DeflateDecoder::new(stream)
        .take((history.len() + expect + 1) as u64)
        .read_to_end(&mut out)?;
    if out.len() != history.len() + expect {
        return Err(bad("a block does not unpack to the size its header gives"));
    }
    out.drain(..history.len());
    Ok(out)
}

/// A folder's data, unpacked as it is read: the stream its files are slices of.
pub(crate) struct FolderReader {
    file: BufReader<File>,
    method: u16,
    blocks_left: u16,
    block_reserve: usize,
    /// The last 32 KiB that came out, which an MSZIP block may refer back to.
    history: Vec<u8>,
    /// The block being handed out, and how much of it has been.
    block: Vec<u8>,
    given: usize,
}

/// The stream of folder `at` in the cabinet at `path`.
///
/// Refused, with the compressor's name, for a folder this crate cannot unpack.
pub(crate) fn folder(path: &Path, name: &str, at: u64) -> Result<(FolderReader, u64)> {
    let io = |e: io::Error| ArcError::Io {
        path: name.to_string(),
        source: e,
    };
    let (index, offset) = folder_of(at);
    let mut file = BufReader::with_capacity(64 * 1024, File::open(path).map_err(io)?);
    let tables = tables(&mut file, name)?;
    let folder = *tables
        .folders
        .get(index)
        .ok_or_else(|| broken(name, format!("folder {index} is not there")))?;
    if let Some(compressor) = folder.beyond() {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!("it is packed with {compressor}, which is not unpacked here"),
        });
    }
    file.seek(SeekFrom::Start(folder.start)).map_err(io)?;
    Ok((
        FolderReader {
            file,
            method: folder.method & 0x000f,
            blocks_left: folder.blocks,
            block_reserve: tables.block_reserve,
            history: Vec::new(),
            block: Vec::new(),
            given: 0,
        },
        offset,
    ))
}

impl FolderReader {
    fn next_block(&mut self) -> io::Result<()> {
        let bad = |reason: &str| io::Error::new(io::ErrorKind::InvalidData, reason.to_string());
        let mut head = [0u8; 8];
        self.file.read_exact(&mut head)?;
        let written = u32_at(&head, 0);
        let packed_len = usize::from(u16_at(&head, 4));
        let unpacked_len = usize::from(u16_at(&head, 6));
        self.file
            .seek(SeekFrom::Current(self.block_reserve as i64))?;
        let mut packed = vec![0u8; packed_len];
        self.file.read_exact(&mut packed)?;
        // Zero is "no checksum was written", which the format allows.
        if written != 0 && checksum(&head[4..8], checksum(&packed, 0)) != written {
            return Err(bad("a block's checksum is wrong"));
        }
        if unpacked_len == 0 {
            return Err(bad("a block is finished in the next cabinet of the set"));
        }
        if unpacked_len > WINDOW {
            return Err(bad("a block claims to unpack to more than 32 KiB"));
        }
        self.block = match self.method {
            STORED => {
                if packed.len() != unpacked_len {
                    return Err(bad("a stored block is not the size its header gives"));
                }
                packed
            }
            _ => {
                let block = inflate_block(&packed, &self.history, unpacked_len)?;
                // What the next block may reach back into: the last 32 KiB of everything so
                // far, which is this block alone when it is a full one.
                self.history.extend_from_slice(&block);
                let over = self.history.len().saturating_sub(WINDOW);
                self.history.drain(..over);
                block
            }
        };
        self.given = 0;
        self.blocks_left -= 1;
        Ok(())
    }
}

impl Read for FolderReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        while self.given == self.block.len() {
            if self.blocks_left == 0 {
                return Ok(0);
            }
            self.next_block()?;
        }
        let n = buf.len().min(self.block.len() - self.given);
        buf[..n].copy_from_slice(&self.block[self.given..self.given + n]);
        self.given += n;
        Ok(n)
    }
}

/// A reader that must give exactly this many bytes: a folder that ends before one of its
/// files does is a damaged cabinet, not a short file.
pub(crate) struct Exactly<R> {
    pub(crate) inner: R,
    pub(crate) left: u64,
}

impl<R: Read> Read for Exactly<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 || buf.is_empty() {
            return Ok(0);
        }
        let want = buf
            .len()
            .min(usize::try_from(self.left).unwrap_or(usize::MAX));
        let n = self.inner.read(&mut buf[..want])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the cabinet's data ends before this entry does",
            ));
        }
        self.left -= n as u64;
        Ok(n)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::io::Write;

    /// One file to put in a cabinet.
    pub(crate) struct Member<'a> {
        pub(crate) name: &'a str,
        pub(crate) body: &'a [u8],
    }

    /// One folder of a cabinet under construction: how it is packed, and what is in it.
    pub(crate) struct Unit<'a> {
        pub(crate) method: u16,
        pub(crate) members: Vec<Member<'a>>,
        /// Blocks to use as they are instead of packing the members' bytes — for a block
        /// written by hand, and for a compressor that cannot be run here.
        pub(crate) blocks: Option<Vec<(Vec<u8>, u16)>>,
    }

    /// A cabinet, built by hand so that no test needs `makecab` on the machine running it.
    ///
    /// Stored and MSZIP folders are really packed: the bytes are cut into 32 KiB blocks and
    /// each MSZIP block is a deflate stream of its own after `CK` — which is a valid MSZIP
    /// cabinet, if one that never uses the history between blocks; the test that needs that
    /// writes its second block by hand. `7z` reads what this makes, and a test below asks it
    /// to wherever it is installed.
    pub(crate) fn cabinet(units: &[Unit<'_>]) -> Vec<u8> {
        let files: usize = units.iter().map(|u| u.members.len()).sum();
        let mut header = Vec::new();
        header.extend_from_slice(MAGIC);
        header.extend_from_slice(&[0; 4]);
        header.extend_from_slice(&[0; 4]); // the cabinet's size, filled in at the end
        header.extend_from_slice(&[0; 4]);
        header.extend_from_slice(&[0; 4]); // where the file table is, filled in below
        header.extend_from_slice(&[0; 4]);
        header.extend_from_slice(&[3, 1]); // version 1.3
        header.extend_from_slice(&(units.len() as u16).to_le_bytes());
        header.extend_from_slice(&(files as u16).to_le_bytes());
        header.extend_from_slice(&[0; 2]); // flags: alone, nothing reserved
        header.extend_from_slice(&[0x34, 0x12]); // the set's id
        header.extend_from_slice(&[0; 2]);
        assert_eq!(header.len(), 36);

        let mut table = Vec::new();
        for (folder, unit) in units.iter().enumerate() {
            let mut offset = 0u32;
            for member in &unit.members {
                table.extend_from_slice(&(member.body.len() as u32).to_le_bytes());
                table.extend_from_slice(&offset.to_le_bytes());
                table.extend_from_slice(&(folder as u16).to_le_bytes());
                // 2026-11-04 09:57:12, and the archive bit with "the name is UTF-8".
                table.extend_from_slice(&(((2026 - 1980) << 9 | 11 << 5 | 4) as u16).to_le_bytes());
                table.extend_from_slice(&((9 << 11 | 57 << 5 | 6) as u16).to_le_bytes());
                table.extend_from_slice(&0x00a0u16.to_le_bytes());
                table.extend_from_slice(member.name.as_bytes());
                table.push(0);
                offset += member.body.len() as u32;
            }
        }

        let mut data = Vec::new();
        let mut folders = Vec::new();
        let data_at = header.len() + units.len() * 8 + table.len();
        for unit in units {
            let start = data_at + data.len();
            let blocks: Vec<(Vec<u8>, u16)> = match &unit.blocks {
                Some(blocks) => blocks.clone(),
                None => {
                    let whole: Vec<u8> = unit
                        .members
                        .iter()
                        .flat_map(|m| m.body.iter().copied())
                        .collect();
                    whole
                        .chunks(WINDOW)
                        .map(|chunk| {
                            let packed = match unit.method {
                                STORED => chunk.to_vec(),
                                _ => mszip_block(chunk),
                            };
                            (packed, chunk.len() as u16)
                        })
                        .collect()
                }
            };
            for (packed, unpacked) in &blocks {
                let mut sizes = Vec::new();
                sizes.extend_from_slice(&(packed.len() as u16).to_le_bytes());
                sizes.extend_from_slice(&unpacked.to_le_bytes());
                data.extend_from_slice(&checksum(&sizes, checksum(packed, 0)).to_le_bytes());
                data.extend_from_slice(&sizes);
                data.extend_from_slice(packed);
            }
            folders.extend_from_slice(&(start as u32).to_le_bytes());
            folders.extend_from_slice(&(blocks.len() as u16).to_le_bytes());
            folders.extend_from_slice(&unit.method.to_le_bytes());
        }

        let files_at = (header.len() + folders.len()) as u32;
        header[16..20].copy_from_slice(&files_at.to_le_bytes());
        let mut out = header;
        out.extend_from_slice(&folders);
        out.extend_from_slice(&table);
        out.extend_from_slice(&data);
        let size = out.len() as u32;
        out[8..12].copy_from_slice(&size.to_le_bytes());
        out
    }

    /// One MSZIP block: `CK`, then the chunk deflated as a stream of its own.
    ///
    /// A chunk that does not compress is written as one stored block marked as the last,
    /// by hand. The compressor here writes such a chunk as a stored block followed by an
    /// empty final one, which is deflate and which this crate reads, but which 7-Zip
    /// refuses in a cabinet — it stops at the block's size and wants the stream finished
    /// there. `makecab` writes the single block, so that is what a fixture should hold.
    pub(crate) fn mszip_block(chunk: &[u8]) -> Vec<u8> {
        let mut enc =
            flate2::write::DeflateEncoder::new(b"CK".to_vec(), flate2::Compression::default());
        enc.write_all(chunk).unwrap();
        let packed = enc.finish().unwrap();
        if packed.len() < chunk.len() + 2 {
            return packed;
        }
        let len = chunk.len() as u16;
        let mut stored = b"CK\x01".to_vec();
        stored.extend_from_slice(&len.to_le_bytes());
        stored.extend_from_slice(&(!len).to_le_bytes());
        stored.extend_from_slice(chunk);
        stored
    }

    /// Bytes that do not compress to nothing and are not the same twice.
    pub(crate) fn noise(len: usize, seed: u64) -> Vec<u8> {
        let mut state = seed | 1;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                (state >> 24) as u8
            })
            .collect()
    }

    /// A deflate stream of one block, fixed codes, that is nothing but "copy ten bytes from
    /// a thousand back" — written bit by bit, because the point of it is a match that
    /// starts before the block does, and no compressor here can be made to emit one.
    pub(crate) fn a_block_that_only_looks_back() -> Vec<u8> {
        struct Bits {
            out: Vec<u8>,
            used: u8,
        }
        impl Bits {
            fn bit(&mut self, bit: u32) {
                if self.used == 0 {
                    self.out.push(0);
                }
                *self.out.last_mut().unwrap() |= (bit as u8 & 1) << self.used;
                self.used = (self.used + 1) % 8;
            }
            /// A field: its lowest bit first.
            fn field(&mut self, value: u32, width: u32) {
                for n in 0..width {
                    self.bit(value >> n);
                }
            }
            /// A Huffman code: its highest bit first.
            fn code(&mut self, value: u32, width: u32) {
                for n in (0..width).rev() {
                    self.bit(value >> n);
                }
            }
        }
        let mut bits = Bits {
            out: b"CK".to_vec(),
            used: 0,
        };
        bits.field(1, 1); // the last block
        bits.field(1, 2); // fixed codes
        bits.code(264 - 256, 7); // length 10: symbol 264, no extra bits
        bits.code(19, 5); // distance 769…1024, with eight bits to say which
        bits.field(1000 - 769, 8);
        bits.code(0, 7); // end of block
        bits.out
    }

    fn write(dir: &Path, name: &str, bytes: &[u8]) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn read_entry(path: &Path, entry: &ArcEntry) -> io::Result<Vec<u8>> {
        let (mut reader, offset) = folder(path, "test.cab", entry.at.unwrap()).unwrap();
        io::copy(&mut (&mut reader).take(offset), &mut io::sink())?;
        let mut out = Vec::new();
        Exactly {
            inner: reader,
            left: entry.size,
        }
        .read_to_end(&mut out)?;
        Ok(out)
    }

    #[test]
    fn a_cabinet_lists_its_files_and_gives_their_bytes_back() {
        let dir = tempfile::tempdir().unwrap();
        // Longer than two blocks, so a file starts in one block and ends in another.
        let big = noise(80_000, 7);
        let text = b"leggimi\n".repeat(300);
        let bytes = cabinet(&[
            Unit {
                method: MSZIP,
                members: vec![
                    Member {
                        name: "readme.md",
                        body: &text,
                    },
                    Member {
                        name: "src\\deep\\big.bin",
                        body: &big,
                    },
                    Member {
                        name: "src\\empty.txt",
                        body: b"",
                    },
                ],
                blocks: None,
            },
            Unit {
                method: STORED,
                members: vec![Member {
                    name: "perché.txt",
                    body: b"stored as it is",
                }],
                blocks: None,
            },
        ]);
        let path = write(dir.path(), "tree.cab", &bytes);
        let found = index(&path, "tree.cab", &Limits::default(), &CancelToken::new()).unwrap();
        assert_eq!(found.beyond, None);
        let names: Vec<&str> = found.entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            names,
            [
                "/readme.md",
                "/src/deep/big.bin",
                "/src/empty.txt",
                "/perché.txt"
            ],
            "a backslash is the separator, and the names are UTF-8 when they say so"
        );
        assert_eq!(found.entries[1].size, 80_000);
        // 2026-11-04 09:57:12 as seconds since the epoch.
        let when = days_from_civil(2026, 11, 4).unwrap() as u64 * 86_400 + 9 * 3_600 + 57 * 60 + 12;
        assert_eq!(
            found.entries[0].mtime,
            Some(SystemTime::UNIX_EPOCH + Duration::from_secs(when))
        );
        assert_eq!(read_entry(&path, &found.entries[0]).unwrap(), text);
        assert_eq!(read_entry(&path, &found.entries[1]).unwrap(), big);
        assert_eq!(read_entry(&path, &found.entries[2]).unwrap(), b"");
        assert_eq!(
            read_entry(&path, &found.entries[3]).unwrap(),
            b"stored as it is"
        );
    }

    /// The one thing MSZIP adds to deflate: a block may copy from the block before it.
    #[test]
    fn an_mszip_block_reaches_back_into_the_one_before_it() {
        let dir = tempfile::tempdir().unwrap();
        let first = noise(WINDOW, 99);
        let mut body = first.clone();
        body.extend_from_slice(&first[WINDOW - 1000..WINDOW - 990]);
        let bytes = cabinet(&[Unit {
            method: MSZIP,
            members: vec![Member {
                name: "two-blocks.bin",
                body: &body,
            }],
            blocks: Some(vec![
                (mszip_block(&first), WINDOW as u16),
                (a_block_that_only_looks_back(), 10),
            ]),
        }]);
        let path = write(dir.path(), "back.cab", &bytes);
        let found = index(&path, "back.cab", &Limits::default(), &CancelToken::new()).unwrap();
        assert_eq!(read_entry(&path, &found.entries[0]).unwrap(), body);
    }

    #[test]
    fn a_damaged_block_is_caught_by_its_checksum_and_a_short_folder_by_its_length() {
        let dir = tempfile::tempdir().unwrap();
        let body = noise(50_000, 3);
        let unit = || Unit {
            method: STORED,
            members: vec![Member {
                name: "a.bin",
                body: &body,
            }],
            blocks: None,
        };
        let mut bytes = cabinet(&[unit()]);
        let last = bytes.len() - 1;
        bytes[last] ^= 0x40;
        let path = write(dir.path(), "bad.cab", &bytes);
        let found = index(&path, "bad.cab", &Limits::default(), &CancelToken::new()).unwrap();
        let error = read_entry(&path, &found.entries[0]).unwrap_err();
        assert!(error.to_string().contains("checksum"), "{error}");

        // A file table that promises more than the folder holds.
        let mut bytes = cabinet(&[unit()]);
        let files_at = u32_at(&bytes, 16) as usize;
        bytes[files_at..files_at + 4].copy_from_slice(&60_000u32.to_le_bytes());
        let path = write(dir.path(), "short.cab", &bytes);
        let found = index(&path, "short.cab", &Limits::default(), &CancelToken::new()).unwrap();
        let error = read_entry(&path, &found.entries[0]).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::UnexpectedEof, "{error}");
    }

    /// A cabinet packed with LZX is listed all the same — the tables are not compressed —
    /// and says which compressor it wants, both at once and when an entry is asked for.
    #[test]
    fn a_compressor_that_is_not_here_is_named_rather_than_read_as_rubble() {
        let dir = tempfile::tempdir().unwrap();
        for (method, word) in [(LZX | 0x1500, "LZX"), (QUANTUM | 0x0a40, "Quantum")] {
            let bytes = cabinet(&[Unit {
                method,
                members: vec![Member {
                    name: "driver.sys",
                    body: &[0u8; 1000],
                }],
                blocks: Some(vec![(vec![0x5b, 0x80, 0x80, 0x8d], 1000)]),
            }]);
            let path = write(dir.path(), "packed.cab", &bytes);
            let found =
                index(&path, "packed.cab", &Limits::default(), &CancelToken::new()).unwrap();
            assert_eq!(found.entries.len(), 1);
            assert_eq!(found.entries[0].size, 1000);
            assert!(
                found.beyond.as_deref().unwrap().contains(word),
                "{:?}",
                found.beyond
            );
            match folder(&path, "packed.cab", found.entries[0].at.unwrap()) {
                Err(ArcError::Refused { reason, .. }) => assert!(reason.contains(word), "{reason}"),
                Err(other) => panic!("{other:?}"),
                Ok(_) => panic!("a folder nothing here can unpack was opened"),
            }
        }
    }

    #[test]
    fn something_that_is_not_a_cabinet_or_stops_half_way_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let whole = cabinet(&[Unit {
            method: STORED,
            members: vec![Member {
                name: "a.txt",
                body: b"abc",
            }],
            blocks: None,
        }]);
        for cut in [10, 40, whole.len() - 8] {
            let path = write(dir.path(), "cut.cab", &whole[..cut]);
            match index(&path, "cut.cab", &Limits::default(), &CancelToken::new()) {
                Err(ArcError::Broken { .. }) => {}
                Err(other) => panic!("{cut}: {other:?}"),
                // The tables are whole and the data is not: found when the entry is read.
                Ok(found) => assert!(read_entry(&path, &found.entries[0]).is_err(), "{cut}"),
            }
        }
        let path = write(dir.path(), "not.cab", &[b'x'; 64]);
        assert!(matches!(
            index(&path, "not.cab", &Limits::default(), &CancelToken::new()),
            Err(ArcError::Unknown(_))
        ));
        // The limit on entries is the archive's own count, asked before anything is read.
        let path = write(dir.path(), "whole.cab", &whole);
        let tight = Limits {
            entries: 0,
            ..Limits::default()
        };
        assert!(matches!(
            index(&path, "whole.cab", &tight, &CancelToken::new()),
            Err(ArcError::Refused { .. })
        ));
    }

    /// The checksum is `[MS-CAB]`'s, and the left-over bytes go in the other way round from
    /// the words: the numbers here are worked out by hand from its definition.
    #[test]
    fn the_block_checksum_is_the_documented_one() {
        assert_eq!(checksum(&[], 0), 0);
        assert_eq!(checksum(&[1, 2, 3, 4], 0), 0x0403_0201);
        assert_eq!(checksum(&[1, 2, 3, 4, 5], 0), 0x0403_0201 ^ 0x05);
        assert_eq!(
            checksum(&[1, 2, 3, 4, 5, 6, 7], 0),
            0x0403_0201 ^ 0x0005_0607
        );
        assert_eq!(checksum(&[0xff], 0x0f), 0xf0);
    }
}
