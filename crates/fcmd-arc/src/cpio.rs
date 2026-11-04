//! `cpio`, and the `rpm` that carries one.
//!
//! A cpio archive is a run of members, each a header, a name and the member's bytes, ended by a
//! member called `TRAILER!!!`. Three header layouts are in use and all three are read here:
//!
//! * **newc** (`070701`) — thirteen fields of eight hex digits. What `cpio -H newc` writes, what
//!   the Linux kernel reads an initramfs in, and what every `rpm` carries.
//! * **crc** (`070702`) — the same layout with a checksum in the last field, which nothing
//!   checks against the bytes and this does not either: it is a sum, not a hash.
//! * **odc** (`070707`) — the POSIX one, octal fields of six and eleven digits and no padding.
//!
//! The fourth, the old *binary* header, is refused by name. It was written by machines whose
//! byte order is part of the format, nothing has produced it in thirty years, and guessing the
//! order of a header is how an index ends up showing a file of four gigabytes.
//!
//! A hard link is stored once. In newc the bytes go with the *last* name of a group of links and
//! the others say they are empty, so the index gives every name of the group the size and the
//! bytes of the one that holds them: an archive of two names for one file shows two files, not
//! a file and an empty one.
//!
//! An `rpm` is a 96-byte lead, a signature header, the package header, and a compressed cpio —
//! newc — as its payload. The payload's codec is told by its magic rather than by the header's
//! `PAYLOADCOMPRESSOR` tag, because the magic cannot be wrong about the bytes behind it.

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::time::{Duration, SystemTime};

use fcmd_core::{CancelToken, EntryKind};

use crate::archive::{normalise, ArcEntry};
use crate::{ArcError, Format, Limits, Result};

/// One member's header, as the walk hands it over.
#[derive(Debug, Clone)]
pub(crate) struct Member {
    pub name: String,
    pub mode: u32,
    pub size: u64,
    pub mtime: u64,
    /// The inode and device folded into one number, which is what groups hard links.
    pub ino: u64,
    pub nlink: u32,
    /// Where the member's bytes start, counted from the first byte of the cpio.
    pub data_at: u64,
}

impl Member {
    pub fn kind(&self) -> EntryKind {
        match self.mode & 0o170000 {
            0o040000 => EntryKind::Dir,
            0o100000 => EntryKind::File,
            0o120000 => EntryKind::Symlink,
            // A device, a fifo, a socket: an initramfs has them, and they have no bytes.
            _ => EntryKind::Other,
        }
    }
}

/// A reader that counts what has gone through it, so a header knows where it is.
struct Counted<R> {
    inner: R,
    at: u64,
}

impl<R: Read> Read for Counted<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let read = self.inner.read(buf)?;
        self.at += read as u64;
        Ok(read)
    }
}

/// The longest name a header may ask for. `PATH_MAX` is 4096; this is generous and still a
/// bound, so a broken header cannot ask for a gigabyte of name.
const NAME_MOST: u64 = 64 * 1024;

/// Walk the members of a cpio, handing each one to `visit` with a reader over its bytes.
///
/// `visit` may read as much or as little of the bytes as it likes; whatever it leaves is read
/// past here. It returns whether to go on. The trailer ends the walk, and so does the end of
/// the stream exactly where a header would start: some writers leave the trailer out.
pub(crate) fn walk<R: Read>(
    reader: R,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
    mut visit: impl FnMut(&Member, &mut dyn Read) -> Result<bool>,
) -> Result<()> {
    let broken = |reason: &str| ArcError::Broken {
        path: name.to_string(),
        reason: reason.to_string(),
    };
    let mut input = Counted {
        inner: reader,
        at: 0,
    };
    let mut members = 0usize;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let mut magic = [0u8; 6];
        match read_full(&mut input, &mut magic) {
            Ok(0) if members > 0 => return Ok(()),
            Ok(6) => {}
            Ok(_) => return Err(broken("it ends in the middle of a header")),
            Err(e) => {
                return Err(ArcError::Io {
                    path: name.to_string(),
                    source: e,
                })
            }
        }
        let (member, padded) = match &magic {
            b"070701" | b"070702" => (newc(&mut input, &broken)?, true),
            b"070707" => (odc(&mut input, &broken)?, false),
            _ if magic[..2] == [0xc7, 0x71] || magic[..2] == [0x71, 0xc7] => {
                return Err(broken(
                    "it is an old binary cpio, whose byte order is part of the format; \
                     it is not read here",
                ))
            }
            _ => return Err(broken("a member does not start with a cpio header")),
        };
        let (namesize, mut member) = member;
        if namesize == 0 || namesize > NAME_MOST {
            return Err(broken("a member's name is empty or impossibly long"));
        }
        let mut raw = vec![0u8; namesize as usize];
        input
            .read_exact(&mut raw)
            .map_err(|_| broken("it ends in the middle of a name"))?;
        let end = raw.iter().position(|b| *b == 0).unwrap_or(raw.len());
        member.name = String::from_utf8_lossy(&raw[..end]).into_owned();
        if padded {
            pad(&mut input, &broken)?;
        }
        if member.name == "TRAILER!!!" {
            return Ok(());
        }
        members += 1;
        if members > limits.entries {
            return Err(ArcError::Refused {
                path: name.to_string(),
                reason: format!("it holds more than {} members", limits.entries),
            });
        }
        member.data_at = input.at;
        let mut body = Read::by_ref(&mut input).take(member.size);
        let go = visit(&member, &mut body)?;
        io::copy(&mut body, &mut io::sink()).map_err(|e| ArcError::Io {
            path: name.to_string(),
            source: e,
        })?;
        if body.limit() != 0 {
            return Err(broken("it ends in the middle of a member"));
        }
        if padded {
            pad(&mut input, &broken)?;
        }
        if !go {
            return Ok(());
        }
    }
}

/// As many bytes as there are, up to the buffer: 0 means the stream had already ended.
fn read_full(input: &mut impl Read, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match input.read(&mut buf[filled..]) {
            Ok(0) => break,
            Ok(n) => filled += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
            Err(e) => return Err(e),
        }
    }
    Ok(filled)
}

/// newc and crc pad the header-and-name, and the bytes, to a multiple of four.
fn pad<R: Read>(input: &mut Counted<R>, broken: &impl Fn(&str) -> ArcError) -> Result<()> {
    let gap = (4 - input.at % 4) % 4;
    let mut skip = [0u8; 3];
    input
        .read_exact(&mut skip[..gap as usize])
        .map_err(|_| broken("it ends in the padding after a member"))
}

/// The rest of a newc header, after its magic: thirteen fields of eight hex digits.
fn newc<R: Read>(
    input: &mut Counted<R>,
    broken: &impl Fn(&str) -> ArcError,
) -> Result<(u64, Member)> {
    let mut raw = [0u8; 104];
    input
        .read_exact(&mut raw)
        .map_err(|_| broken("it ends in the middle of a header"))?;
    let field = |i: usize| -> Result<u64> {
        let text = std::str::from_utf8(&raw[i * 8..i * 8 + 8])
            .map_err(|_| broken("a header field is not hex"))?;
        u64::from_str_radix(text, 16).map_err(|_| broken("a header field is not hex"))
    };
    let ino = field(0)?;
    let mode = field(1)? as u32;
    let nlink = field(4)? as u32;
    let mtime = field(5)?;
    let size = field(6)?;
    let dev = (field(7)? << 16) | field(8)?;
    let namesize = field(11)?;
    Ok((
        namesize,
        Member {
            name: String::new(),
            mode,
            size,
            mtime,
            ino: ino ^ (dev << 32),
            nlink,
            data_at: 0,
        },
    ))
}

/// The rest of an odc header: octal fields, six digits each but for the time and the size.
fn odc<R: Read>(
    input: &mut Counted<R>,
    broken: &impl Fn(&str) -> ArcError,
) -> Result<(u64, Member)> {
    let mut raw = [0u8; 70];
    input
        .read_exact(&mut raw)
        .map_err(|_| broken("it ends in the middle of a header"))?;
    let field = |from: usize, len: usize| -> Result<u64> {
        let text = std::str::from_utf8(&raw[from..from + len])
            .map_err(|_| broken("a header field is not octal"))?;
        u64::from_str_radix(text, 8).map_err(|_| broken("a header field is not octal"))
    };
    // dev 6, ino 6, mode 6, uid 6, gid 6, nlink 6, rdev 6, mtime 11, namesize 6, filesize 11.
    let dev = field(0, 6)?;
    let ino = field(6, 6)?;
    let mode = field(12, 6)? as u32;
    let nlink = field(30, 6)? as u32;
    let mtime = field(42, 11)?;
    let namesize = field(53, 6)?;
    let size = field(59, 11)?;
    Ok((
        namesize,
        Member {
            name: String::new(),
            mode,
            size,
            mtime,
            ino: ino ^ (dev << 32),
            nlink,
            data_at: 0,
        },
    ))
}

/// The entries of a cpio read from `reader`. `at` says whether a member's offset means
/// anything — it does for a cpio on the disk, which is seeked into, and not for an rpm's
/// payload, which is a compressed stream read from the start every time; there the inode goes
/// in its place, so a hard link can find the member holding its bytes.
pub(crate) fn index<R: Read>(
    reader: R,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
    seekable: bool,
) -> Result<Vec<ArcEntry>> {
    let mut out: Vec<ArcEntry> = Vec::new();
    // For each group of links: who holds the bytes, and where.
    let mut holders: HashMap<u64, (u64, u64)> = HashMap::new();
    let mut links: Vec<(usize, u64)> = Vec::new();
    walk(reader, name, limits, cancel, |member, bytes| {
        let path = normalise(&member.name);
        if path == "/" {
            return Ok(true);
        }
        let kind = member.kind();
        let link_target = if kind == EntryKind::Symlink {
            // A symlink's target is its bytes. Bounded, because a header saying a link is a
            // gigabyte long is a broken archive and not a long link.
            let mut target = Vec::new();
            bytes
                .take(PATH_MOST)
                .read_to_end(&mut target)
                .map_err(|e| ArcError::Io {
                    path: name.to_string(),
                    source: e,
                })?;
            Some(String::from_utf8_lossy(&target).into_owned())
        } else {
            None
        };
        if kind == EntryKind::File && member.nlink > 1 {
            if member.size > 0 {
                holders.insert(member.ino, (member.size, member.data_at));
            }
            links.push((out.len(), member.ino));
        }
        out.push(ArcEntry {
            path,
            size: if kind == EntryKind::File {
                member.size
            } else {
                0
            },
            packed: None,
            mtime: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(member.mtime)),
            mode: Some(member.mode & 0o7777),
            kind,
            link_target,
            at: Some(if seekable { member.data_at } else { member.ino }),
        });
        Ok(true)
    })?;
    for (i, ino) in links {
        if let Some((size, data_at)) = holders.get(&ino) {
            out[i].size = *size;
            if seekable {
                out[i].at = Some(*data_at);
            }
        }
    }
    Ok(out)
}

/// The longest symlink target read: a path, so `PATH_MAX`.
const PATH_MOST: u64 = 4096;

/// The index of a cpio file on the disk.
pub(crate) fn index_file(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    index(
        BufReader::with_capacity(64 * 1024, file),
        name,
        limits,
        cancel,
        true,
    )
}

/// What an rpm's payload is compressed with, told by its first bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Codec {
    Plain,
    Gz,
    Bz2,
    Xz,
    Zst,
    Lzma,
}

/// Where an rpm's payload starts and what it is compressed with.
///
/// The lead is 96 bytes. Then two headers of one shape: an 8-byte magic-and-reserved, a count
/// of index entries and a length of data, both 32-bit big-endian, then sixteen bytes per index
/// entry and the data. The first of them, the signature, is padded to a multiple of eight; the
/// second, the package header, is not, and the payload starts where it ends.
fn payload(path: &Path, name: &str) -> Result<(u64, Codec)> {
    let broken = |reason: &str| ArcError::Broken {
        path: name.to_string(),
        reason: reason.to_string(),
    };
    let io = |e: io::Error| ArcError::Io {
        path: name.to_string(),
        source: e,
    };
    let mut file = File::open(path).map_err(io)?;
    let mut lead = [0u8; 96];
    file.read_exact(&mut lead)
        .map_err(|_| broken("it is too short to be an rpm"))?;
    if lead[..4] != [0xed, 0xab, 0xee, 0xdb] {
        return Err(broken("it does not start with an rpm lead"));
    }
    let mut at: u64 = 96;
    for (which, padded) in [("signature", true), ("package", false)] {
        file.seek(SeekFrom::Start(at)).map_err(io)?;
        let mut intro = [0u8; 16];
        file.read_exact(&mut intro)
            .map_err(|_| broken(&format!("it ends in its {which} header")))?;
        if intro[..3] != [0x8e, 0xad, 0xe8] {
            return Err(broken(&format!("its {which} header has no header magic")));
        }
        let count = u32::from_be_bytes(intro[8..12].try_into().expect("four bytes")) as u64;
        let data = u32::from_be_bytes(intro[12..16].try_into().expect("four bytes")) as u64;
        at += 16 + count * 16 + data;
        if padded {
            at += (8 - at % 8) % 8;
        }
    }
    file.seek(SeekFrom::Start(at)).map_err(io)?;
    let mut head = [0u8; 6];
    let read = read_full(&mut file, &mut head).map_err(io)?;
    let head = &head[..read];
    let codec = if head.starts_with(&[0x1f, 0x8b]) {
        Codec::Gz
    } else if head.starts_with(b"BZh") {
        Codec::Bz2
    } else if head.starts_with(&[0xfd, b'7', b'z', b'X', b'Z', 0x00]) {
        Codec::Xz
    } else if head.starts_with(&[0x28, 0xb5, 0x2f, 0xfd]) {
        Codec::Zst
    } else if head.starts_with(b"0707") {
        Codec::Plain
    } else if head.first() == Some(&0x5d) {
        // The legacy `.lzma` stream, which has no magic: its first byte is the properties
        // byte, and the one every encoder writes by default is 0x5d.
        Codec::Lzma
    } else {
        return Err(broken(
            "its payload is compressed in a way this does not read",
        ));
    };
    Ok((at, codec))
}

/// The rpm's payload, decompressed: a newc cpio, read from its first byte.
pub(crate) fn rpm_stream(path: &Path) -> Result<Box<dyn Read>> {
    let name = path.display().to_string();
    let (at, codec) = payload(path, &name)?;
    let mut file = File::open(path).map_err(|e| ArcError::Io {
        path: name.clone(),
        source: e,
    })?;
    file.seek(SeekFrom::Start(at)).map_err(|e| ArcError::Io {
        path: name.clone(),
        source: e,
    })?;
    let file = BufReader::with_capacity(64 * 1024, file);
    let broken = |e: &dyn std::fmt::Display| ArcError::Broken {
        path: name.clone(),
        reason: e.to_string(),
    };
    Ok(match codec {
        Codec::Plain => Box::new(file),
        Codec::Gz => Box::new(flate2::read::MultiGzDecoder::new(file)),
        Codec::Bz2 => Box::new(bzip2::read::MultiBzDecoder::new(file)),
        Codec::Xz => Box::new(lzma_rust2::XzReader::new(file, true)),
        Codec::Zst => {
            Box::new(ruzstd::decoding::StreamingDecoder::new(file).map_err(|e| broken(&e))?)
        }
        // A dictionary of up to 1.5 GiB is what the format allows; a package asks for a few
        // megabytes, and a header asking for more is refused rather than obeyed.
        Codec::Lzma => Box::new(
            lzma_rust2::LzmaReader::new_mem_limit(file, 256 * 1024, None)
                .map_err(|e| broken(&e))?,
        ),
    })
}

/// The index of an rpm: the files its payload would install.
pub(crate) fn index_rpm(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    index(rpm_stream(path)?, name, limits, cancel, false)
}

/// The bytes of one rpm entry, which means walking the payload up to it. `ino` is what the
/// index kept in the entry's `at`: a hard link with no bytes of its own is answered from the
/// member of its group that has them.
pub(crate) fn rpm_entry(
    path: &Path,
    inner: &str,
    ino: Option<u64>,
    limits: &Limits,
    cancel: &CancelToken,
    mut read: impl FnMut(&mut dyn Read) -> Result<u64>,
) -> Result<Option<u64>> {
    let name = path.display().to_string();
    let mut found = None;
    walk(rpm_stream(path)?, &name, limits, cancel, |member, bytes| {
        let same_name = normalise(&member.name) == inner;
        let same_file = member.nlink > 1 && Some(member.ino) == ino;
        if (member.size > 0 && (same_name || same_file)) || (same_name && member.nlink <= 1) {
            found = Some(read(bytes)?);
            return Ok(false);
        }
        Ok(true)
    })?;
    Ok(found)
}

/// Is this the format the bytes say?
pub(crate) fn of_bytes(head: &[u8]) -> Option<Format> {
    if head.starts_with(&[0xed, 0xab, 0xee, 0xdb]) {
        return Some(Format::Rpm);
    }
    if head.starts_with(b"070701") || head.starts_with(b"070702") || head.starts_with(b"070707") {
        return Some(Format::Cpio);
    }
    None
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A newc member, written the way `cpio -H newc` writes one.
    pub(crate) fn newc_member(
        out: &mut Vec<u8>,
        name: &str,
        mode: u32,
        ino: u64,
        nlink: u32,
        body: &[u8],
    ) {
        let namesize = name.len() + 1;
        let fields = [
            ino,
            mode as u64,
            0,
            0,
            nlink as u64,
            1_700_000_000,
            body.len() as u64,
            8,
            1,
            0,
            0,
            namesize as u64,
            0,
        ];
        out.extend_from_slice(b"070701");
        for f in fields {
            out.extend_from_slice(format!("{f:08x}").as_bytes());
        }
        out.extend_from_slice(name.as_bytes());
        out.push(0);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
        out.extend_from_slice(body);
        while !out.len().is_multiple_of(4) {
            out.push(0);
        }
    }

    pub(crate) fn newc(members: &[(&str, u32, &[u8])]) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, (name, mode, body)) in members.iter().enumerate() {
            newc_member(&mut out, name, *mode, i as u64 + 1, 1, body);
        }
        newc_member(&mut out, "TRAILER!!!", 0, 0, 1, b"");
        out
    }

    /// An rpm around a cpio: a lead, a signature header of one index entry and five bytes of
    /// data — an odd length, so the padding to eight has something to do — an empty package
    /// header, and the payload compressed with xz, which is what Fedora ships.
    pub(crate) fn rpm(cpio: &[u8]) -> Vec<u8> {
        let mut out = vec![0u8; 96];
        out[..4].copy_from_slice(&[0xed, 0xab, 0xee, 0xdb]);
        let header = |count: u32, data: &[u8]| {
            let mut h = vec![0x8e, 0xad, 0xe8, 0x01, 0, 0, 0, 0];
            h.extend_from_slice(&count.to_be_bytes());
            h.extend_from_slice(&(data.len() as u32).to_be_bytes());
            h.extend(std::iter::repeat_n(0u8, count as usize * 16));
            h.extend_from_slice(data);
            h
        };
        out.extend(header(1, b"12345"));
        while !out.len().is_multiple_of(8) {
            out.push(0);
        }
        out.extend(header(0, b""));
        let mut enc = lzma_rust2::XzWriter::new(Vec::new(), lzma_rust2::XzOptions::with_preset(1))
            .expect("an encoder");
        std::io::Write::write_all(&mut enc, cpio).expect("written");
        out.extend(enc.finish().expect("finished"));
        out
    }

    fn entries(bytes: &[u8]) -> Vec<ArcEntry> {
        index(
            bytes,
            "test.cpio",
            &Limits::default(),
            &CancelToken::new(),
            true,
        )
        .unwrap()
    }

    #[test]
    fn a_newc_lists_its_files_directories_and_links() {
        let bytes = newc(&[
            (".", 0o040755, b""),
            ("./etc", 0o040755, b""),
            ("./etc/hostname", 0o100644, b"box\n"),
            ("./etc/localtime", 0o120777, b"/usr/share/zoneinfo/UTC"),
            ("./dev/null", 0o020666, b""),
        ]);
        let got = entries(&bytes);
        let by = |p: &str| {
            got.iter()
                .find(|e| e.path == p)
                .unwrap_or_else(|| panic!("{p}"))
        };
        assert_eq!(by("/etc").kind, EntryKind::Dir);
        let host = by("/etc/hostname");
        assert_eq!(
            (host.kind, host.size, host.mode),
            (EntryKind::File, 4, Some(0o644))
        );
        let at = host.at.unwrap() as usize;
        assert_eq!(&bytes[at..at + 4], b"box\n");
        let link = by("/etc/localtime");
        assert_eq!(link.kind, EntryKind::Symlink);
        assert_eq!(link.link_target.as_deref(), Some("/usr/share/zoneinfo/UTC"));
        assert_eq!(by("/dev/null").kind, EntryKind::Other);
        // `.` is the root and not an entry of its own.
        assert!(got.iter().all(|e| e.path != "/"));
    }

    #[test]
    fn a_hard_link_shows_the_bytes_of_the_name_that_holds_them() {
        // newc stores the bytes once, with the last name of the group.
        let mut bytes = Vec::new();
        newc_member(&mut bytes, "bin/a", 0o100755, 7, 2, b"");
        newc_member(&mut bytes, "bin/b", 0o100755, 7, 2, b"#!/bin/sh\n");
        newc_member(&mut bytes, "TRAILER!!!", 0, 0, 1, b"");
        let got = entries(&bytes);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].size, got[1].size);
        assert_eq!(got[0].at, got[1].at);
    }

    #[test]
    fn an_odc_reads_the_same() {
        let mut bytes = Vec::new();
        for (name, mode, body) in [
            ("notes.txt", 0o100644u32, &b"hello"[..]),
            ("TRAILER!!!", 0, b""),
        ] {
            bytes.extend_from_slice(
                format!(
                    "070707{:06o}{:06o}{:06o}{:06o}{:06o}{:06o}{:06o}{:011o}{:06o}{:011o}",
                    0,
                    1,
                    mode,
                    0,
                    0,
                    1,
                    0,
                    1_700_000_000u64,
                    name.len() + 1,
                    body.len()
                )
                .as_bytes(),
            );
            bytes.extend_from_slice(name.as_bytes());
            bytes.push(0);
            bytes.extend_from_slice(body);
        }
        let got = entries(&bytes);
        assert_eq!(got.len(), 1);
        assert_eq!((got[0].path.as_str(), got[0].size), ("/notes.txt", 5));
        let at = got[0].at.unwrap() as usize;
        assert_eq!(&bytes[at..at + 5], b"hello");
    }

    #[test]
    fn what_is_not_a_cpio_is_refused_with_a_reason() {
        let short = &newc(&[("a", 0o100644, b"0123456789")])[..120];
        let e = index(short, "x", &Limits::default(), &CancelToken::new(), true).unwrap_err();
        assert!(e.to_string().contains("ends in the middle"), "{e}");

        let e = index(
            &[0xc7, 0x71, 0, 0, 0, 0][..],
            "x",
            &Limits::default(),
            &CancelToken::new(),
            true,
        )
        .unwrap_err();
        assert!(e.to_string().contains("old binary cpio"), "{e}");

        let e = index(
            &b"PK\x03\x04 nope"[..],
            "x",
            &Limits::default(),
            &CancelToken::new(),
            true,
        )
        .unwrap_err();
        assert!(e.to_string().contains("cpio header"), "{e}");
    }

    #[test]
    fn a_cpio_with_more_members_than_the_limit_is_refused() {
        let many: Vec<(String, u32, &[u8])> = (0..5)
            .map(|i| (format!("f{i}"), 0o100644, &b""[..]))
            .collect();
        let named: Vec<(&str, u32, &[u8])> =
            many.iter().map(|(n, m, b)| (n.as_str(), *m, *b)).collect();
        let bytes = newc(&named);
        let limits = Limits {
            entries: 3,
            ..Limits::default()
        };
        let e = index(&bytes[..], "x", &limits, &CancelToken::new(), true).unwrap_err();
        assert!(e.to_string().contains("more than 3"), "{e}");
    }
}
