//! The tar format, read and written here.
//!
//! A tar is a run of 512-byte blocks: a header, then the member's bytes padded to a whole
//! block, then the next header, until two blocks of zeros. Everything about a member is in its
//! header, in ASCII fields of fixed width, and three dialects grew around the original:
//!
//! * **v7**, the original: a name of 100 bytes and no magic. A directory is a name that ends
//!   in `/`.
//! * **ustar** (POSIX 1988, magic `ustar\0` `00`): a type byte, user and group names, and a
//!   155-byte prefix that the name is joined to with a `/`, which is what lets a path of 255
//!   bytes fit.
//! * **GNU** (magic `ustar  \0`): the prefix field means something else there, and a name or a
//!   link target too long for its field travels in an entry of its own just before the member
//!   (`L` and `K`), whose bytes are the name.
//! * **PAX** (POSIX 2001): an entry of type `x` before the member holds `key=value` records
//!   that override the header — `path`, `linkpath`, `size`, `mtime`, the ids and the names —
//!   and one of type `g` holds the same for every member after it. This is what both GNU tar
//!   and bsdtar write today when a field does not fit.
//!
//! Numbers are octal ASCII ended by a space or a NUL; one too large for its field is stored in
//! base 256 with the top bit of the first byte set, which GNU tar does for a member over 8 GiB.
//!
//! What is written is ustar, with a PAX entry in front of a member only when something about
//! it does not fit the header: the form every tar since 2001 reads, and the one that turns a
//! long name into a long name everywhere rather than into a `././@LongLink` somewhere.

use std::fs::Metadata;
use std::io::{self, Read, Write};
use std::path::Path;
use std::time::UNIX_EPOCH;

/// A header block's size, and the unit everything in a tar is padded to.
const BLOCK: usize = 512;
/// The longest name, link target or PAX block read into memory. Real ones are a few hundred
/// bytes; this is what keeps a broken archive from asking for a gigabyte of name.
const MAX_META: u64 = 1 << 20;

/// What a member is, from its type byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    File,
    /// A hard link to a member stored earlier, named by `link`.
    HardLink,
    Symlink,
    Dir,
    /// Devices, fifos, GNU's sparse and volume entries: shown, never read as a file.
    Other,
}

/// One member's header, with the GNU and PAX extensions already applied.
#[derive(Debug, Clone)]
pub(crate) struct Header {
    /// The name as stored: relative, perhaps with a `./` in front or a `/` at the end.
    pub name: String,
    pub link: String,
    /// The type byte itself, kept so a rebuild writes back what it read.
    pub typeflag: u8,
    pub size: u64,
    pub mode: u32,
    pub mtime: u64,
    pub uid: u64,
    pub gid: u64,
    pub uname: String,
    pub gname: String,
    pub devmajor: u32,
    pub devminor: u32,
    /// Where the member's bytes start, counted from the first byte of the tar.
    pub data_at: u64,
}

impl Header {
    pub fn kind(&self) -> Kind {
        match self.typeflag {
            b'0' | 0 | b'7' if self.name.ends_with('/') => Kind::Dir,
            b'0' | 0 | b'7' => Kind::File,
            b'1' => Kind::HardLink,
            b'2' => Kind::Symlink,
            b'5' => Kind::Dir,
            _ => Kind::Other,
        }
    }

    /// A header for something about to be written, with nothing in it but the name and kind.
    pub fn new(name: &str, kind: Kind) -> Header {
        Header {
            name: name.to_string(),
            link: String::new(),
            typeflag: match kind {
                Kind::File => b'0',
                Kind::HardLink => b'1',
                Kind::Symlink => b'2',
                Kind::Dir => b'5',
                Kind::Other => b'0',
            },
            size: 0,
            mode: if kind == Kind::Dir { 0o755 } else { 0o644 },
            mtime: 0,
            uid: 0,
            gid: 0,
            uname: String::new(),
            gname: String::new(),
            devmajor: 0,
            devminor: 0,
            data_at: 0,
        }
    }

    /// The mode, owner and time of a file on disk, the way `tar` itself records them.
    pub fn set_metadata(&mut self, meta: &Metadata) {
        self.mtime = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            self.mode = meta.mode() & 0o7777;
            self.uid = u64::from(meta.uid());
            self.gid = u64::from(meta.gid());
        }
        #[cfg(not(unix))]
        {
            // No mode bits to read: what a Unix tar would give a plain file and a directory,
            // and a read-only file without its write bits.
            let base = if meta.is_dir() { 0o755 } else { 0o644 };
            self.mode = if meta.permissions().readonly() {
                base & !0o222
            } else {
                base
            };
        }
    }
}

/// A tar being read, one member at a time.
///
/// `next` gives the next member's header; the reader itself then reads that member's bytes,
/// and no further. Whatever was not read is skipped by the next `next`, so a walk that only
/// wants the names never holds a byte of data.
pub(crate) struct Reader<R: Read> {
    inner: R,
    /// Bytes consumed from `inner` so far.
    pos: u64,
    /// What is left of the current member's bytes, and of the padding after them.
    left: u64,
    pad: u64,
    /// The PAX records that apply to every member from here on (`g` entries).
    global: Vec<(String, String)>,
    done: bool,
}

impl<R: Read> Reader<R> {
    pub fn new(inner: R) -> Reader<R> {
        Reader {
            inner,
            pos: 0,
            left: 0,
            pad: 0,
            global: Vec::new(),
            done: false,
        }
    }

    /// The next member, or `None` at the end of the archive.
    ///
    /// The end is two blocks of zeros, but a tar cut short after a whole member — which
    /// several writers produce, and which every reader accepts — ends where the stream does.
    pub fn next(&mut self) -> io::Result<Option<Header>> {
        if self.done {
            return Ok(None);
        }
        self.skip_rest()?;
        let mut long_name: Option<String> = None;
        let mut long_link: Option<String> = None;
        let mut pax: Vec<(String, String)> = Vec::new();
        loop {
            let mut block = [0u8; BLOCK];
            let got = self.read_block(&mut block)?;
            if got == 0 {
                self.done = true;
                return Ok(None);
            }
            if got < BLOCK {
                return Err(broken("the archive ends in the middle of a header"));
            }
            if block.iter().all(|&b| b == 0) {
                // The first of the two closing blocks. The second is not waited for: a tar
                // that stops after one is still over.
                self.done = true;
                return Ok(None);
            }
            if !checksum_ok(&block) {
                return Err(broken(
                    "a header's checksum is wrong — this is not a tar, or it is damaged",
                ));
            }
            let mut header = parse(&block)?;
            let data_at = self.pos;
            header.data_at = data_at;
            match header.typeflag {
                // GNU's long name and long link: the next member's, in this entry's bytes.
                b'L' | b'K' => {
                    let text = self.read_meta(header.size)?;
                    let text =
                        String::from_utf8_lossy(text.split(|&b| b == 0).next().unwrap_or(&[]))
                            .into_owned();
                    if header.typeflag == b'L' {
                        long_name = Some(text);
                    } else {
                        long_link = Some(text);
                    }
                }
                b'x' => {
                    let text = self.read_meta(header.size)?;
                    pax.extend(pax_records(&text));
                }
                b'g' => {
                    let text = self.read_meta(header.size)?;
                    self.global.extend(pax_records(&text));
                }
                _ => {
                    for (key, value) in self.global.iter().chain(pax.iter()) {
                        apply_pax(&mut header, key, value);
                    }
                    if let Some(name) = long_name {
                        header.name = name;
                    }
                    if let Some(link) = long_link {
                        header.link = link;
                    }
                    // A hard link, a directory and a device carry no bytes whatever the size
                    // field says; the rest carry `size` of them, padded to a block.
                    let carries =
                        !matches!(header.typeflag, b'1' | b'2' | b'3' | b'4' | b'5' | b'6');
                    self.left = if carries { header.size } else { 0 };
                    self.pad = padding(self.left);
                    return Ok(Some(header));
                }
            }
        }
    }

    /// Skip what the caller did not read of the current member.
    fn skip_rest(&mut self) -> io::Result<()> {
        let mut rest = self.left + self.pad;
        self.left = 0;
        self.pad = 0;
        let mut sink = [0u8; 8192];
        while rest > 0 {
            let want = rest.min(sink.len() as u64) as usize;
            let n = self.inner.read(&mut sink[..want])?;
            if n == 0 {
                return Err(broken("the archive ends in the middle of a member"));
            }
            self.pos += n as u64;
            rest -= n as u64;
        }
        Ok(())
    }

    /// A whole block, or as much of one as the stream still has.
    fn read_block(&mut self, block: &mut [u8; BLOCK]) -> io::Result<usize> {
        let mut filled = 0;
        while filled < BLOCK {
            match self.inner.read(&mut block[filled..])? {
                0 => break,
                n => filled += n,
            }
        }
        self.pos += filled as u64;
        Ok(filled)
    }

    /// The bytes of a GNU or PAX entry, with their padding consumed.
    fn read_meta(&mut self, size: u64) -> io::Result<Vec<u8>> {
        if size > MAX_META {
            return Err(broken("a header extension is too large to be one"));
        }
        let mut text = vec![0u8; size as usize];
        self.inner.read_exact(&mut text)?;
        self.pos += size;
        self.left = 0;
        self.pad = padding(size);
        self.skip_rest()?;
        Ok(text)
    }
}

impl<R: Read> Read for Reader<R> {
    /// The current member's bytes, and nothing after them.
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.left == 0 {
            return Ok(0);
        }
        let want = (buf.len() as u64).min(self.left) as usize;
        let n = self.inner.read(&mut buf[..want])?;
        if n == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "the archive ends in the middle of a member",
            ));
        }
        self.pos += n as u64;
        self.left -= n as u64;
        Ok(n)
    }
}

/// A tar being written.
pub(crate) struct Builder<W: Write> {
    out: W,
}

impl<W: Write> Builder<W> {
    pub fn new(out: W) -> Builder<W> {
        Builder { out }
    }

    /// One member, its bytes read from `data` — exactly `header.size` of them.
    pub fn append(&mut self, header: &Header, data: &mut dyn Read) -> io::Result<()> {
        self.write_header(header)?;
        let carries = !matches!(header.typeflag, b'1' | b'2' | b'3' | b'4' | b'5' | b'6');
        if !carries {
            return Ok(());
        }
        let copied = io::copy(&mut data.take(header.size), &mut self.out)?;
        if copied != header.size {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                format!(
                    "{}: {copied} bytes where the header promised {}",
                    header.name, header.size
                ),
            ));
        }
        self.write_padding(header.size)
    }

    /// A file on disk, under `name`, with its mode, owner and time.
    pub fn append_file(&mut self, name: &str, path: &Path) -> io::Result<u64> {
        let mut file = std::fs::File::open(path)?;
        let meta = file.metadata()?;
        let mut header = Header::new(name, Kind::File);
        header.set_metadata(&meta);
        header.size = meta.len();
        self.append(&header, &mut file)?;
        Ok(meta.len())
    }

    /// A directory on disk, under `name`, with a `/` at the end as tar has it.
    pub fn append_dir(&mut self, name: &str, path: &Path) -> io::Result<()> {
        let meta = std::fs::metadata(path)?;
        let name = if name.ends_with('/') {
            name.to_string()
        } else {
            format!("{name}/")
        };
        let mut header = Header::new(&name, Kind::Dir);
        header.set_metadata(&meta);
        self.append(&header, &mut io::empty())
    }

    /// The two blocks of zeros that end a tar, and the writer back.
    pub fn finish(mut self) -> io::Result<W> {
        self.out.write_all(&[0u8; BLOCK * 2])?;
        self.out.flush()?;
        Ok(self.out)
    }

    fn write_header(&mut self, header: &Header) -> io::Result<()> {
        let mut block = [0u8; BLOCK];
        let mut pax: Vec<(&str, String)> = Vec::new();
        // The name: in the name field if it fits, split at a `/` across the prefix and the
        // name if that fits, and otherwise in a PAX record, with as much as fits left in the
        // field for a reader that knows nothing of PAX.
        let name = header.name.as_bytes();
        if name.len() <= 100 {
            block[..name.len()].copy_from_slice(name);
        } else if let Some((prefix, rest)) = split_ustar(&header.name) {
            block[..rest.len()].copy_from_slice(rest.as_bytes());
            block[345..345 + prefix.len()].copy_from_slice(prefix.as_bytes());
        } else {
            pax.push(("path", header.name.clone()));
            let cut = floor_char(&header.name, 100);
            block[..cut].copy_from_slice(&name[..cut]);
        }
        let link = header.link.as_bytes();
        if link.len() <= 100 {
            block[157..157 + link.len()].copy_from_slice(link);
        } else {
            pax.push(("linkpath", header.link.clone()));
            let cut = floor_char(&header.link, 100);
            block[157..157 + cut].copy_from_slice(&link[..cut]);
        }
        octal(&mut block[100..108], u64::from(header.mode & 0o7777777));
        number(&mut block[108..116], header.uid, "uid", &mut pax);
        number(&mut block[116..124], header.gid, "gid", &mut pax);
        number(&mut block[124..136], header.size, "size", &mut pax);
        number(&mut block[136..148], header.mtime, "mtime", &mut pax);
        block[156] = header.typeflag;
        block[257..263].copy_from_slice(b"ustar\0");
        block[263..265].copy_from_slice(b"00");
        for (field, text, key) in [
            (265..297, &header.uname, "uname"),
            (297..329, &header.gname, "gname"),
        ] {
            let bytes = text.as_bytes();
            if bytes.len() < 32 {
                block[field.start..field.start + bytes.len()].copy_from_slice(bytes);
            } else {
                pax.push((key, text.clone()));
            }
        }
        octal(&mut block[329..337], u64::from(header.devmajor));
        octal(&mut block[337..345], u64::from(header.devminor));
        seal(&mut block);

        if !pax.is_empty() {
            let mut records = Vec::new();
            for (key, value) in &pax {
                records.extend_from_slice(&pax_record(key, value));
            }
            let base = header
                .name
                .rsplit('/')
                .find(|s| !s.is_empty())
                .unwrap_or("member");
            let mut ext = Header::new(
                &format!("PaxHeader/{}", &base[..floor_char(base, 80)]),
                Kind::File,
            );
            ext.typeflag = b'x';
            ext.size = records.len() as u64;
            ext.mtime = header.mtime.min(0o77777777777);
            self.write_header(&ext)?;
            self.out.write_all(&records)?;
            self.write_padding(records.len() as u64)?;
        }
        self.out.write_all(&block)
    }

    fn write_padding(&mut self, size: u64) -> io::Result<()> {
        let pad = padding(size) as usize;
        if pad > 0 {
            self.out.write_all(&[0u8; BLOCK][..pad])?;
        }
        Ok(())
    }
}

/// What a header block says, before any extension is applied.
fn parse(block: &[u8; BLOCK]) -> io::Result<Header> {
    let magic = &block[257..265];
    let ustar = &magic[..6] == b"ustar\0";
    let mut name = text(&block[0..100]);
    // Only POSIX ustar has a prefix there; GNU keeps times in the same bytes.
    if ustar {
        let prefix = text(&block[345..500]);
        if !prefix.is_empty() {
            name = format!("{prefix}/{name}");
        }
    }
    Ok(Header {
        name,
        link: text(&block[157..257]),
        typeflag: block[156],
        size: numeric(&block[124..136])?,
        mode: numeric(&block[100..108])? as u32,
        mtime: numeric(&block[136..148])?,
        uid: numeric(&block[108..116])?,
        gid: numeric(&block[116..124])?,
        uname: text(&block[265..297]),
        gname: text(&block[297..329]),
        devmajor: numeric(&block[329..337]).unwrap_or(0) as u32,
        devminor: numeric(&block[337..345]).unwrap_or(0) as u32,
        data_at: 0,
    })
}

/// A field's text, up to its first NUL.
fn text(field: &[u8]) -> String {
    let end = field.iter().position(|&b| b == 0).unwrap_or(field.len());
    String::from_utf8_lossy(&field[..end]).into_owned()
}

/// A numeric field: octal ASCII, or base 256 when the top bit of the first byte is set.
fn numeric(field: &[u8]) -> io::Result<u64> {
    if field.first().is_some_and(|&b| b & 0x80 != 0) {
        // Base 256, big-endian, the first byte's top bit the marker. A negative number (the
        // next bit also set) is a time before 1970 at most; nothing here wants one.
        if field[0] & 0x40 != 0 {
            return Ok(0);
        }
        let mut value: u64 = u64::from(field[0] & 0x3f);
        for &b in &field[1..] {
            value = value
                .checked_mul(256)
                .and_then(|v| v.checked_add(u64::from(b)))
                .ok_or_else(|| broken("a number in a header is too large"))?;
        }
        return Ok(value);
    }
    let digits: Vec<u8> = field
        .iter()
        .copied()
        .skip_while(|&b| b == b' ')
        .take_while(|&b| (b'0'..=b'7').contains(&b))
        .collect();
    if digits.is_empty() {
        return Ok(0);
    }
    let mut value: u64 = 0;
    for d in digits {
        value = value
            .checked_mul(8)
            .and_then(|v| v.checked_add(u64::from(d - b'0')))
            .ok_or_else(|| broken("a number in a header is too large"))?;
    }
    Ok(value)
}

/// The header's checksum: every byte summed with the checksum field read as spaces. Some old
/// writers summed signed bytes, so either sum is accepted.
fn checksum_ok(block: &[u8; BLOCK]) -> bool {
    let Ok(stored) = numeric(&block[148..156]) else {
        return false;
    };
    let (mut unsigned, mut signed) = (0u64, 0i64);
    for (i, &b) in block.iter().enumerate() {
        let b = if (148..156).contains(&i) { b' ' } else { b };
        unsigned += u64::from(b);
        signed += i64::from(b as i8);
    }
    stored == unsigned || i64::try_from(stored).is_ok_and(|s| s == signed)
}

/// Write the checksum into a block whose other fields are final.
fn seal(block: &mut [u8; BLOCK]) {
    block[148..156].fill(b' ');
    let sum: u64 = block.iter().map(|&b| u64::from(b)).sum();
    // Six octal digits, a NUL and a space: the form every tar writes.
    let text = format!("{sum:06o}\0 ");
    block[148..156].copy_from_slice(text.as_bytes());
}

/// Octal digits filling the field but its last byte, which is a NUL.
fn octal(field: &mut [u8], value: u64) {
    let width = field.len() - 1;
    let text = format!("{value:0width$o}");
    let text = &text.as_bytes()[text.len().saturating_sub(width)..];
    field[..width].copy_from_slice(text);
    field[width] = 0;
}

/// A number in its field when it fits, and in a PAX record otherwise — with base 256 in the
/// field as well, which is what GNU tar writes and what a reader without PAX understands.
fn number(field: &mut [u8], value: u64, key: &'static str, pax: &mut Vec<(&'static str, String)>) {
    let width = field.len() - 1;
    let max = (1u64 << (3 * width as u32)) - 1;
    if value <= max {
        octal(field, value);
        return;
    }
    pax.push((key, value.to_string()));
    field.fill(0);
    let bytes = value.to_be_bytes();
    let n = field.len();
    field[n - 8..].copy_from_slice(&bytes);
    field[0] |= 0x80;
}

/// A path too long for the name field, split at a `/` into a prefix of at most 155 bytes and a
/// name of at most 100, as ustar allows; `None` when no `/` makes both fit.
fn split_ustar(path: &str) -> Option<(&str, &str)> {
    if path.len() > 256 {
        return None;
    }
    path.match_indices('/').find_map(|(at, _)| {
        let (prefix, rest) = (&path[..at], &path[at + 1..]);
        (prefix.len() <= 155 && rest.len() <= 100 && !rest.is_empty()).then_some((prefix, rest))
    })
}

/// The longest prefix of `s` no longer than `max` bytes that ends on a character boundary.
fn floor_char(s: &str, max: usize) -> usize {
    let mut cut = s.len().min(max);
    while !s.is_char_boundary(cut) {
        cut -= 1;
    }
    cut
}

fn padding(size: u64) -> u64 {
    (BLOCK as u64 - size % BLOCK as u64) % BLOCK as u64
}

/// One PAX record: `<length> <key>=<value>\n`, where the length counts itself.
fn pax_record(key: &str, value: &str) -> Vec<u8> {
    let body = format!(" {key}={value}\n");
    let mut len = body.len() + 1;
    while len != body.len() + len.to_string().len() {
        len = body.len() + len.to_string().len();
    }
    format!("{len}{body}").into_bytes()
}

/// The records of a PAX block. A malformed record ends the reading of the block, not of the
/// archive: what came before it still applies.
fn pax_records(text: &[u8]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        let Some(space) = rest.iter().position(|&b| b == b' ') else {
            break;
        };
        let Some(len) = std::str::from_utf8(&rest[..space])
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
        else {
            break;
        };
        if len <= space || len > rest.len() {
            break;
        }
        let record = &rest[space + 1..len];
        let record = record.strip_suffix(b"\n").unwrap_or(record);
        if let Some(eq) = record.iter().position(|&b| b == b'=') {
            let key = String::from_utf8_lossy(&record[..eq]).into_owned();
            let value = String::from_utf8_lossy(&record[eq + 1..]).into_owned();
            out.push((key, value));
        }
        rest = &rest[len..];
    }
    out
}

fn apply_pax(header: &mut Header, key: &str, value: &str) {
    // Times may carry a fraction (`1700000000.123456789`); the seconds are what is kept.
    let whole = |v: &str| v.split('.').next().and_then(|s| s.parse::<u64>().ok());
    match key {
        "path" => header.name = value.to_string(),
        "linkpath" => header.link = value.to_string(),
        "size" => {
            if let Some(n) = whole(value) {
                header.size = n;
            }
        }
        "mtime" => {
            if let Some(n) = whole(value) {
                header.mtime = n;
            }
        }
        "uid" => {
            if let Some(n) = whole(value) {
                header.uid = n;
            }
        }
        "gid" => {
            if let Some(n) = whole(value) {
                header.gid = n;
            }
        }
        "uname" => header.uname = value.to_string(),
        "gname" => header.gname = value.to_string(),
        _ => {}
    }
}

fn broken(reason: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build(members: &[(Header, &[u8])]) -> Vec<u8> {
        let mut tar = Builder::new(Vec::new());
        for (header, body) in members {
            let mut h = header.clone();
            h.size = body.len() as u64;
            tar.append(&h, &mut &body[..]).unwrap();
        }
        tar.finish().unwrap()
    }

    fn read_all(bytes: &[u8]) -> Vec<(Header, Vec<u8>)> {
        let mut reader = Reader::new(bytes);
        let mut out = Vec::new();
        while let Some(h) = reader.next().unwrap() {
            let mut body = Vec::new();
            reader.read_to_end(&mut body).unwrap();
            out.push((h, body));
        }
        out
    }

    fn file(name: &str) -> Header {
        let mut h = Header::new(name, Kind::File);
        h.mode = 0o640;
        h.mtime = 1_700_000_000;
        h
    }

    #[test]
    fn what_is_written_is_read_back_name_by_name_and_byte_by_byte() {
        let mut dir = Header::new("src/", Kind::Dir);
        dir.mtime = 1_600_000_000;
        let mut link = Header::new("src/latest", Kind::Symlink);
        link.link = "main.rs".into();
        let bytes = build(&[
            (dir, b""),
            (file("src/main.rs"), b"fn main() {}\n"),
            (file("empty"), b""),
            (link, b""),
            (file("big.bin"), &[7u8; 1500]),
        ]);
        assert_eq!(bytes.len() % BLOCK, 0);
        let back = read_all(&bytes);
        let names: Vec<_> = back.iter().map(|(h, _)| h.name.as_str()).collect();
        assert_eq!(
            names,
            ["src/", "src/main.rs", "empty", "src/latest", "big.bin"]
        );
        assert_eq!(back[0].0.kind(), Kind::Dir);
        assert_eq!(back[1].1, b"fn main() {}\n");
        assert_eq!(back[1].0.mode, 0o640);
        assert_eq!(back[1].0.mtime, 1_700_000_000);
        assert_eq!(back[3].0.kind(), Kind::Symlink);
        assert_eq!(back[3].0.link, "main.rs");
        assert_eq!(back[4].1, vec![7u8; 1500]);
    }

    /// A name over 100 bytes goes in the ustar prefix when a `/` can split it, and in a PAX
    /// record when none can; either way it comes back whole, and so does a long link.
    #[test]
    fn long_names_survive_the_header_they_do_not_fit() {
        let split = format!("{}/{}", "d".repeat(120), "f".repeat(90));
        let unsplittable = "n".repeat(300);
        let accented = format!("{}/é-{}", "à".repeat(60), "ü".repeat(70));
        let mut link = Header::new("l", Kind::Symlink);
        link.link = "t".repeat(180);
        let bytes = build(&[
            (file(&split), b"one"),
            (file(&unsplittable), b"two"),
            (file(&accented), b"three"),
            (link, b""),
        ]);
        let back = read_all(&bytes);
        assert_eq!(back[0].0.name, split);
        assert_eq!(back[1].0.name, unsplittable);
        assert_eq!(back[1].1, b"two");
        assert_eq!(back[2].0.name, accented);
        assert_eq!(back[3].0.link, "t".repeat(180));
    }

    /// GNU tar's own long-name entries (`././@LongLink`, type `L`) are understood.
    #[test]
    fn a_gnu_long_name_entry_names_the_member_after_it() {
        let long = "g".repeat(150);
        let mut ext = Header::new("././@LongLink", Kind::File);
        ext.typeflag = b'L';
        let mut name = long.clone().into_bytes();
        name.push(0);
        let bytes = build(&[(ext, &name), (file("short"), b"body")]);
        let back = read_all(&bytes);
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].0.name, long);
        assert_eq!(back[0].1, b"body");
    }

    /// Numbers too large for their field: a size over 8 GiB is written in base 256 with a PAX
    /// record, and read back from either.
    #[test]
    fn a_number_too_large_for_its_field_is_kept() {
        let mut block = [0u8; 12];
        let mut pax = Vec::new();
        number(&mut block, 20 * (1 << 30), "size", &mut pax);
        assert_eq!(block[0] & 0x80, 0x80);
        assert_eq!(numeric(&block).unwrap(), 20 * (1 << 30));
        assert_eq!(pax, [("size", (20u64 * (1 << 30)).to_string())]);
        let mut small = [0u8; 12];
        number(&mut small, 0o644, "size", &mut pax);
        assert_eq!(&small, b"00000000644\0");
        assert_eq!(numeric(b"   644 \0").unwrap(), 0o644);
    }

    #[test]
    fn a_pax_record_counts_its_own_length() {
        let r = pax_record("path", "x");
        assert_eq!(r, b"9 path=x\n");
        // Where the length's own digits push it over a power of ten.
        let v = "y".repeat(91);
        let r = pax_record("path", &v);
        assert_eq!(
            r.len().to_string(),
            String::from_utf8_lossy(&r).split(' ').next().unwrap()
        );
        let parsed = pax_records(&r);
        assert_eq!(parsed, [("path".to_string(), v)]);
    }

    /// The bytes nobody read are skipped by the next header, so a walk over names alone works.
    #[test]
    fn a_member_left_unread_is_skipped() {
        let bytes = build(&[(file("a"), &[1u8; 2000]), (file("b"), b"bee")]);
        let mut reader = Reader::new(&bytes[..]);
        let a = reader.next().unwrap().unwrap();
        assert_eq!(a.data_at, 512);
        let mut two = [0u8; 2];
        reader.read_exact(&mut two).unwrap();
        let b = reader.next().unwrap().unwrap();
        assert_eq!(b.name, "b");
        // Two blocks of header and four of data before it.
        assert_eq!(b.data_at, 512 * 6);
        let mut body = String::new();
        reader.read_to_string(&mut body).unwrap();
        assert_eq!(body, "bee");
        assert!(reader.next().unwrap().is_none());
    }

    /// The failures: a damaged header is said to be one, a stream cut in a member is an error
    /// and not a short file, and a tar without its closing blocks still ends.
    #[test]
    fn a_damaged_or_cut_archive_says_so() {
        let mut bytes = build(&[(file("a"), b"hello"), (file("b"), &[2u8; 900])]);
        // The closing zeros taken off: still read to the end.
        let open = &bytes[..bytes.len() - 1024];
        assert_eq!(read_all(open).len(), 2);
        // Cut in the middle of b's bytes.
        let cut = &bytes[..512 * 3 + 100];
        let mut reader = Reader::new(cut);
        reader.next().unwrap();
        reader.next().unwrap();
        let mut sink = Vec::new();
        assert!(reader.read_to_end(&mut sink).is_err());
        // A flipped byte in a header.
        bytes[10] ^= 0x55;
        assert!(Reader::new(&bytes[..]).next().is_err());
        // Not a tar at all.
        assert!(Reader::new(&[b'x'; 1024][..]).next().is_err());
    }

    /// A v7 tar has no type byte for a directory; its name says it.
    #[test]
    fn an_old_tar_directory_is_known_by_its_slash() {
        let mut h = Header::new("old/", Kind::File);
        h.typeflag = 0;
        let back = read_all(&build(&[(h, b"")]));
        assert_eq!(back[0].0.kind(), Kind::Dir);
    }

    /// The archive is written in the form the system's own `tar` reads, names, sizes and bytes
    /// alike — checked against it where it is installed, as it is on every CI machine.
    #[test]
    fn the_systems_tar_reads_what_is_written() {
        let Ok(probe) = std::process::Command::new("tar").arg("--version").output() else {
            return;
        };
        if !probe.status.success() {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let long = format!("{}/{}", "deep".repeat(30), "name.txt");
        let bytes = build(&[(file("plain.txt"), b"plain\n"), (file(&long), b"long\n")]);
        let path = dir.path().join("t.tar");
        std::fs::write(&path, &bytes).unwrap();
        let out = std::process::Command::new("tar")
            .arg("-tf")
            .arg(&path)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let listed = String::from_utf8_lossy(&out.stdout);
        assert!(listed.lines().any(|l| l == "plain.txt"), "{listed}");
        assert!(listed.lines().any(|l| l == long), "{listed}");
        // And the other way: what it writes, read here.
        let src = dir.path().join("src");
        std::fs::create_dir_all(src.join("sub/".repeat(40))).unwrap();
        std::fs::write(src.join("a.txt"), "A").unwrap();
        let theirs = dir.path().join("theirs.tar");
        let made = std::process::Command::new("tar")
            .arg("-cf")
            .arg(&theirs)
            .arg("-C")
            .arg(&src)
            .arg(".")
            .status()
            .unwrap();
        assert!(made.success());
        let back = read_all(&std::fs::read(&theirs).unwrap());
        assert!(back
            .iter()
            .any(|(h, body)| h.name.ends_with("a.txt") && body == b"A"));
        assert!(back
            .iter()
            .any(|(h, _)| h.name.len() > 150 && h.kind() == Kind::Dir));
    }
}
