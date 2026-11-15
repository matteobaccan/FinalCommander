//! The OLE compound file (MS-CFB), read only: the container an Outlook `.msg` and the old
//! `.doc`, `.xls` and `.ppt` are written in.
//!
//! A compound file is a small filesystem in one file. It is cut into sectors of 512 bytes
//! (version 3) or 4096 (version 4); a table, the FAT, says for each sector which one follows
//! it, so a stream is a chain of sectors; the FAT itself lives in sectors listed by a second
//! table, the DIFAT, whose first 109 entries are in the header. Streams shorter than 4096
//! bytes are packed 64 bytes at a time into one ordinary stream, the mini-stream, with a FAT
//! of its own. The directory is another chain of 128-byte entries, each a storage (a folder)
//! or a stream (a file), linked as a red-black tree per storage — which is read here as the
//! flat list of a storage's children, since nothing is ever inserted.
//!
//! Every number in the file is hostile until checked: a sector past the end, a chain that
//! comes back to where it started, a stream that says it is larger than the file holding it.
//! Each chain is walked with a bound on its length, so a loop is an error and never a hang,
//! and no buffer is sized from a field before the field has been compared with the file.

/// What every compound file starts with.
pub const MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

/// The last sector of a chain.
const END_OF_CHAIN: u32 = 0xFFFF_FFFE;
/// A sector nobody uses, and the "no such entry" of the directory's links.
const FREE: u32 = 0xFFFF_FFFF;
/// Sector numbers from here up are marks, not sectors.
const FIRST_MARK: u32 = 0xFFFF_FFFA;

/// The size of one directory entry.
const ENTRY: usize = 128;

/// How the reader gets bytes: a slice in memory for the archive reader, the viewer's own
/// source for the structure pane — which is why a `.doc` of a gigabyte costs a few sectors.
pub trait ReadAt {
    /// Up to `len` bytes at `offset`; fewer, or none, at the end of the file.
    fn read_at(&mut self, offset: u64, len: usize) -> Vec<u8>;
    fn size(&mut self) -> u64;
}

impl ReadAt for &[u8] {
    fn read_at(&mut self, offset: u64, len: usize) -> Vec<u8> {
        let total = self.len();
        let start = usize::try_from(offset).unwrap_or(usize::MAX).min(total);
        let end = start.saturating_add(len).min(total);
        self[start..end].to_vec()
    }

    fn size(&mut self) -> u64 {
        self.len() as u64
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// The root storage: entry 0, whose stream is the mini-stream.
    Root,
    Storage,
    Stream,
}

/// One entry of the directory.
#[derive(Debug, Clone)]
pub struct Entry {
    pub name: String,
    pub kind: Kind,
    /// The length of a stream; 0 for a storage.
    pub size: u64,
    /// When it was created and last written, as FILETIMEs; 0 when the writer left them out.
    pub created: u64,
    pub modified: u64,
    start: u32,
    left: u32,
    right: u32,
    child: u32,
    /// The storage this entry is in; `None` for the root and for an entry no storage reaches.
    parent: Option<usize>,
}

/// An open compound file: its tables, read once, and nothing of its streams.
#[derive(Debug)]
pub struct Cfb {
    shift: u32,
    mini_shift: u32,
    cutoff: u64,
    len: u64,
    fat: Vec<u32>,
    minifat: Vec<u32>,
    /// The sectors of the mini-stream, in order.
    mini_chain: Vec<u32>,
    mini_len: u64,
    entries: Vec<Option<Entry>>,
}

fn le16(b: &[u8], at: usize) -> u16 {
    b.get(at..at + 2)
        .map_or(0, |s| u16::from_le_bytes([s[0], s[1]]))
}

fn le32(b: &[u8], at: usize) -> u32 {
    b.get(at..at + 4)
        .map_or(0, |s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn le64(b: &[u8], at: usize) -> u64 {
    u64::from(le32(b, at)) | (u64::from(le32(b, at + 4)) << 32)
}

impl Cfb {
    /// Read the header, the FAT, the directory and the mini-FAT. `max_entries` is how many
    /// directory entries are accepted before the file is refused as a bomb.
    pub fn open(src: &mut dyn ReadAt, max_entries: usize) -> Result<Cfb, String> {
        let len = src.size();
        let head = src.read_at(0, 512);
        if head.len() < 512 || head[..8] != MAGIC {
            return Err("not an OLE compound file".into());
        }
        let major = le16(&head, 26);
        let shift = u32::from(le16(&head, 30));
        let mini_shift = u32::from(le16(&head, 32));
        // The two sector sizes the format has; anything else is not a version of it.
        if !matches!((major, shift), (3, 9) | (4, 12)) {
            return Err(format!(
                "version {major} with sectors of 2^{shift} bytes is not a compound file"
            ));
        }
        if mini_shift != 6 {
            return Err(format!("mini-sectors of 2^{mini_shift} bytes"));
        }
        let cutoff = u64::from(le32(&head, 56));
        if cutoff != 4096 {
            return Err(format!("a mini-stream cutoff of {cutoff}"));
        }
        let sector = 1usize << shift;
        // Sector n is at (n + 1) × size, so this is one more than the largest number a
        // sector with any byte in the file can have.
        let sectors = usize::try_from(len.div_ceil(sector as u64).saturating_sub(1))
            .map_err(|_| "too large".to_string())?;
        let fat_sectors = le32(&head, 44) as usize;
        if fat_sectors > sectors {
            return Err(format!(
                "{fat_sectors} FAT sectors in a file of {sectors} sectors"
            ));
        }

        // The DIFAT: 109 entries in the header, the rest in a chain of sectors whose last
        // four bytes name the next one.
        let mut difat: Vec<u32> = (0..109)
            .map(|i| le32(&head, 76 + i * 4))
            .filter(|&s| s < FIRST_MARK)
            .collect();
        let mut next = le32(&head, 68);
        let mut walked = 0usize;
        while next < FIRST_MARK && difat.len() < fat_sectors {
            if next as usize >= sectors || walked >= sectors {
                return Err("the DIFAT chain leaves the file or loops".into());
            }
            walked += 1;
            let bytes = src.read_at((u64::from(next) + 1) << shift, sector);
            if bytes.len() < sector {
                return Err("the DIFAT is cut short".into());
            }
            let per = sector / 4 - 1;
            difat.extend(
                (0..per)
                    .map(|i| le32(&bytes, i * 4))
                    .filter(|&s| s < FIRST_MARK),
            );
            next = le32(&bytes, per * 4);
        }
        difat.truncate(fat_sectors);

        let mut fat: Vec<u32> = Vec::new();
        for &s in &difat {
            if s as usize >= sectors {
                return Err(format!("FAT sector {s} is past the end of the file"));
            }
            let bytes = src.read_at((u64::from(s) + 1) << shift, sector);
            // A FAT sector cut short by the end of the file describes sectors that are not
            // there either: what is missing reads as free.
            fat.extend((0..sector / 4).map(|i| {
                if i * 4 + 4 <= bytes.len() {
                    le32(&bytes, i * 4)
                } else {
                    FREE
                }
            }));
        }
        // Entries for sectors beyond the file say nothing a reader can use.
        fat.truncate(sectors);

        let mut cfb = Cfb {
            shift,
            mini_shift,
            cutoff,
            len,
            fat,
            minifat: Vec::new(),
            mini_chain: Vec::new(),
            mini_len: 0,
            entries: Vec::new(),
        };

        // The directory.
        let dir = chain(&cfb.fat, le32(&head, 48), None)?;
        let per = sector / ENTRY;
        if dir.len().saturating_mul(per) > max_entries {
            return Err(format!("a directory of more than {max_entries} entries"));
        }
        for &s in &dir {
            let bytes = src.read_at((u64::from(s) + 1) << shift, sector);
            for i in 0..per {
                let Some(raw) = bytes.get(i * ENTRY..(i + 1) * ENTRY) else {
                    break;
                };
                cfb.entries.push(entry(raw, major));
            }
        }
        if cfb.entries.first().and_then(|e| e.as_ref()).map(|e| e.kind) != Some(Kind::Root) {
            return Err("the directory does not start with a root entry".into());
        }

        // The mini-FAT, and the stream it cuts up, which is the root entry's.
        let mini = chain(&cfb.fat, le32(&head, 60), None)?;
        for &s in &mini {
            let bytes = src.read_at((u64::from(s) + 1) << shift, sector);
            cfb.minifat.extend((0..sector / 4).map(|i| {
                if i * 4 + 4 <= bytes.len() {
                    le32(&bytes, i * 4)
                } else {
                    FREE
                }
            }));
        }
        let (root_start, root_size) = cfb.entries[0]
            .as_ref()
            .map(|e| (e.start, e.size))
            .unwrap_or((END_OF_CHAIN, 0));
        if root_size > len {
            return Err("a mini-stream larger than the file".into());
        }
        cfb.mini_len = root_size;
        let want = usize::try_from(root_size.div_ceil(sector as u64)).unwrap_or(usize::MAX);
        cfb.mini_chain = chain(&cfb.fat, root_start, Some(want))?;
        // A mini-FAT cannot describe more of the mini-stream than there is.
        let minis = usize::try_from(root_size >> mini_shift).unwrap_or(usize::MAX);
        cfb.minifat.truncate(minis.saturating_add(1));

        cfb.link();
        Ok(cfb)
    }

    /// Give every entry its storage. The children of a storage are a binary tree hung from
    /// its `child` link; walking it with a mark on each entry visited makes a tree that
    /// loops, or two storages that claim the same child, harmless: an entry is given to the
    /// first storage that reaches it and never visited again.
    fn link(&mut self) {
        let n = self.entries.len();
        let mut seen = vec![false; n];
        seen[0] = true;
        let mut stack: Vec<(u32, usize)> = Vec::new();
        if let Some(Some(root)) = self.entries.first() {
            stack.push((root.child, 0));
        }
        while let Some((at, parent)) = stack.pop() {
            let i = at as usize;
            if at == FREE || i >= n || seen[i] {
                continue;
            }
            seen[i] = true;
            let Some(e) = self.entries[i].as_mut() else {
                continue;
            };
            e.parent = Some(parent);
            stack.push((e.left, parent));
            stack.push((e.right, parent));
            if e.kind == Kind::Storage {
                stack.push((e.child, i));
            }
        }
    }

    /// The root storage.
    pub fn root(&self) -> usize {
        0
    }

    pub fn entry(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index).and_then(|e| e.as_ref())
    }

    /// What a storage holds, in directory order.
    pub fn children(&self, storage: usize) -> Vec<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| e.as_ref().is_some_and(|e| e.parent == Some(storage)))
            .map(|(i, _)| i)
            .collect()
    }

    /// The child of `storage` called `name`. Names compare without case, as the format says.
    pub fn child(&self, storage: usize, name: &str) -> Option<usize> {
        self.entries.iter().position(|e| {
            e.as_ref()
                .is_some_and(|e| e.parent == Some(storage) && same_name(&e.name, name))
        })
    }

    /// The bytes of a stream, or a refusal if it is over `cap`.
    pub fn read(&self, src: &mut dyn ReadAt, index: usize, cap: u64) -> Result<Vec<u8>, String> {
        let e = self
            .entry(index)
            .filter(|e| e.kind == Kind::Stream)
            .ok_or_else(|| "not a stream".to_string())?;
        if e.size > cap {
            return Err(format!("the stream {} is over the size limit", e.name));
        }
        let size = usize::try_from(e.size).map_err(|_| "too large".to_string())?;
        let mut out = Vec::new();
        if e.size < self.cutoff {
            if e.size > self.mini_len {
                return Err(format!(
                    "the stream {} is larger than the mini-stream",
                    e.name
                ));
            }
            let mini = 1usize << self.mini_shift;
            let want = size.div_ceil(mini);
            let sectors = chain(&self.minifat, e.start, Some(want))?;
            if sectors.len() < want {
                return Err(format!("the stream {} is cut short", e.name));
            }
            out.reserve(size);
            for &m in &sectors {
                let at = u64::from(m) << self.mini_shift;
                let holder = usize::try_from(at >> self.shift)
                    .ok()
                    .and_then(|i| self.mini_chain.get(i))
                    .ok_or_else(|| "a mini-sector outside the mini-stream".to_string())?;
                let offset =
                    ((u64::from(*holder) + 1) << self.shift) + (at & ((1 << self.shift) - 1));
                let take = mini.min(size - out.len());
                let bytes = src.read_at(offset, take);
                if bytes.len() < take {
                    return Err(format!("the stream {} is cut short", e.name));
                }
                out.extend_from_slice(&bytes);
            }
        } else {
            if e.size > self.len {
                return Err(format!("the stream {} is larger than the file", e.name));
            }
            let sector = 1usize << self.shift;
            let want = size.div_ceil(sector);
            let sectors = chain(&self.fat, e.start, Some(want))?;
            if sectors.len() < want {
                return Err(format!("the stream {} is cut short", e.name));
            }
            out.reserve(size);
            for &s in &sectors {
                let take = sector.min(size - out.len());
                let bytes = src.read_at((u64::from(s) + 1) << self.shift, take);
                if bytes.len() < take {
                    return Err(format!("the stream {} is cut short", e.name));
                }
                out.extend_from_slice(&bytes);
            }
        }
        Ok(out)
    }
}

/// Entry names compare case-insensitively (MS-CFB §2.6.4, simplified to what the names in
/// real files need: they are ASCII, or compared against themselves).
fn same_name(a: &str, b: &str) -> bool {
    a.len() == b.len()
        && a.chars()
            .zip(b.chars())
            .all(|(x, y)| x == y || x.to_uppercase().eq(y.to_uppercase()))
}

/// The sectors of a chain starting at `start`. With `want`, the walk stops once it has that
/// many, which is all a stream of a known length needs; without it, the chain is followed to
/// its end. Either way no chain can be longer than its table, so a walk that gets there has
/// met a loop and says so.
fn chain(table: &[u32], start: u32, want: Option<usize>) -> Result<Vec<u32>, String> {
    let mut out = Vec::new();
    let mut at = start;
    while at < FIRST_MARK {
        if want.is_some_and(|w| out.len() >= w) {
            break;
        }
        if at as usize >= table.len() {
            return Err(format!("sector {at} is past the end of the file"));
        }
        if out.len() >= table.len() {
            return Err("a sector chain loops".into());
        }
        out.push(at);
        at = table[at as usize];
    }
    Ok(out)
}

/// One 128-byte directory entry; `None` for a slot nothing is in.
fn entry(raw: &[u8], major: u16) -> Option<Entry> {
    let kind = match raw[66] {
        1 => Kind::Storage,
        2 => Kind::Stream,
        5 => Kind::Root,
        _ => return None,
    };
    // The length counts the terminating zero, in bytes; a length the field cannot hold is
    // read as the whole field.
    let name_len = (le16(raw, 64) as usize).clamp(2, 64) - 2;
    let units: Vec<u16> = raw[..name_len]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| u16::from_le_bytes(*c))
        .collect();
    let mut size = le64(raw, 120);
    if major == 3 {
        // Version 3 writers were allowed to leave rubbish in the high half.
        size &= 0xFFFF_FFFF;
    }
    Some(Entry {
        name: String::from_utf16_lossy(&units),
        kind,
        size: if kind == Kind::Storage { 0 } else { size },
        created: le64(raw, 100),
        modified: le64(raw, 108),
        start: le32(raw, 116),
        left: le32(raw, 68),
        right: le32(raw, 72),
        child: le32(raw, 76),
        parent: None,
    })
}

/// A FILETIME — hundreds of nanoseconds since 1601 — as seconds since 1970, or `None` for
/// the zero a writer leaves when it has no time to give.
pub fn filetime_unix(ft: u64) -> Option<i64> {
    if ft == 0 {
        return None;
    }
    Some((ft / 10_000_000) as i64 - 11_644_473_600)
}

/// `2026-11-06 14:20:00 UTC` for a FILETIME.
pub fn filetime_text(ft: u64) -> Option<String> {
    let secs = filetime_unix(ft)?;
    let days = secs.div_euclid(86_400);
    let rest = secs.rem_euclid(86_400);
    // Howard Hinnant's days-to-civil, the same arithmetic the rest of the program uses.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    Some(format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02} UTC",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    ))
}

/// A compound file written for the tests: this crate only reads them, and no fixture exists,
/// so the tests build what they read. Public to the crate's tests and to the viewer's.
#[doc(hidden)]
pub mod build {
    use super::{END_OF_CHAIN, ENTRY, FREE, MAGIC};

    /// A storage or a stream to be written.
    pub enum Item {
        Stream(String, Vec<u8>),
        Storage(String, Vec<Item>),
    }

    pub fn stream(name: &str, bytes: &[u8]) -> Item {
        Item::Stream(name.to_string(), bytes.to_vec())
    }

    pub fn storage(name: &str, items: Vec<Item>) -> Item {
        Item::Storage(name.to_string(), items)
    }

    struct Flat {
        name: String,
        kind: u8,
        bytes: Vec<u8>,
        left: u32,
        right: u32,
        child: u32,
    }

    /// Lay a storage's children out as directory entries. The sibling tree is written as a
    /// list hanging off each entry's right link, which is a valid (if unbalanced) tree for a
    /// reader that only walks it.
    fn flatten(items: Vec<Item>, out: &mut Vec<Flat>) -> u32 {
        let mut first = FREE;
        let mut prev: Option<usize> = None;
        for item in items {
            let at = out.len();
            match item {
                Item::Stream(name, bytes) => out.push(Flat {
                    name,
                    kind: 2,
                    bytes,
                    left: FREE,
                    right: FREE,
                    child: FREE,
                }),
                Item::Storage(name, inner) => {
                    out.push(Flat {
                        name,
                        kind: 1,
                        bytes: Vec::new(),
                        left: FREE,
                        right: FREE,
                        child: FREE,
                    });
                    let child = flatten(inner, out);
                    out[at].child = child;
                }
            }
            match prev {
                Some(p) => out[p].right = at as u32,
                None => first = at as u32,
            }
            prev = Some(at);
        }
        first
    }

    /// The file: `major` 3 for 512-byte sectors, 4 for 4096. Streams under 4096 bytes go to
    /// the mini-stream, as a real writer puts them.
    pub fn file(major: u16, items: Vec<Item>) -> Vec<u8> {
        let shift = if major == 4 { 12 } else { 9 };
        let sector = 1usize << shift;
        let mut flat = vec![Flat {
            name: "Root Entry".into(),
            kind: 5,
            bytes: Vec::new(),
            left: FREE,
            right: FREE,
            child: FREE,
        }];
        let child = flatten(items, &mut flat);
        flat[0].child = child;

        // Data sectors, numbered from 0 in the order they are laid down; the FAT grows
        // beside them and is placed last.
        let mut data: Vec<u8> = Vec::new();
        let mut fat: Vec<u32> = Vec::new();
        let put = |bytes: &[u8], data: &mut Vec<u8>, fat: &mut Vec<u32>| -> u32 {
            if bytes.is_empty() {
                return END_OF_CHAIN;
            }
            let first = fat.len() as u32;
            let count = bytes.len().div_ceil(sector);
            for i in 0..count {
                fat.push(if i + 1 == count {
                    END_OF_CHAIN
                } else {
                    first + i as u32 + 1
                });
            }
            data.extend_from_slice(bytes);
            data.resize(data.len().div_ceil(sector) * sector, 0);
            first
        };

        // The mini-stream and its FAT.
        let mut mini: Vec<u8> = Vec::new();
        let mut minifat: Vec<u32> = Vec::new();
        let mut starts: Vec<u32> = vec![END_OF_CHAIN; flat.len()];
        for (i, f) in flat.iter().enumerate() {
            if f.kind != 2 || f.bytes.is_empty() {
                continue;
            }
            if f.bytes.len() < 4096 {
                let first = minifat.len() as u32;
                let count = f.bytes.len().div_ceil(64);
                for k in 0..count {
                    minifat.push(if k + 1 == count {
                        END_OF_CHAIN
                    } else {
                        first + k as u32 + 1
                    });
                }
                mini.extend_from_slice(&f.bytes);
                mini.resize(mini.len().div_ceil(64) * 64, 0);
                starts[i] = first;
            } else {
                starts[i] = put(&f.bytes, &mut data, &mut fat);
            }
        }
        let mini_len = mini.len();
        starts[0] = put(&mini, &mut data, &mut fat);
        let minifat_bytes: Vec<u8> = minifat.iter().flat_map(|v| v.to_le_bytes()).collect();
        let minifat_start = put(&minifat_bytes, &mut data, &mut fat);
        let minifat_sectors = minifat_bytes.len().div_ceil(sector);

        // The directory.
        let mut dir: Vec<u8> = Vec::new();
        for (i, f) in flat.iter().enumerate() {
            let mut e = [0u8; ENTRY];
            let units: Vec<u16> = f.name.encode_utf16().take(31).collect();
            for (k, u) in units.iter().enumerate() {
                e[k * 2..k * 2 + 2].copy_from_slice(&u.to_le_bytes());
            }
            e[64..66].copy_from_slice(&((units.len() as u16 + 1) * 2).to_le_bytes());
            e[66] = f.kind;
            e[67] = 1;
            e[68..72].copy_from_slice(&f.left.to_le_bytes());
            e[72..76].copy_from_slice(&f.right.to_le_bytes());
            e[76..80].copy_from_slice(&f.child.to_le_bytes());
            e[116..120].copy_from_slice(&starts[i].to_le_bytes());
            let size = if i == 0 { mini_len } else { f.bytes.len() } as u64;
            e[120..128].copy_from_slice(&size.to_le_bytes());
            dir.extend_from_slice(&e);
        }
        let dir_start = put(&dir, &mut data, &mut fat);
        let dir_sectors = dir.len().div_ceil(sector);

        // The FAT describes itself too, and the DIFAT sectors that list it once it has
        // outgrown the 109 places in the header: find how many of each that takes.
        let per = sector / 4;
        let mut fat_sectors = 0usize;
        let mut difat_sectors = 0usize;
        loop {
            let f = (fat.len() + fat_sectors + difat_sectors).div_ceil(per);
            let d = f.saturating_sub(109).div_ceil(per - 1);
            if f == fat_sectors && d == difat_sectors {
                break;
            }
            fat_sectors = f;
            difat_sectors = d;
        }
        // 0xFFFFFFFC marks a sector that holds DIFAT, 0xFFFFFFFD one that holds FAT.
        let difat_first = fat.len() as u32;
        fat.extend(std::iter::repeat_n(0xFFFF_FFFC, difat_sectors));
        let fat_first = fat.len() as u32;
        fat.extend(std::iter::repeat_n(0xFFFF_FFFD, fat_sectors));
        fat.resize(fat_sectors * per, FREE);
        let mut difat: Vec<u8> = Vec::new();
        for d in 0..difat_sectors {
            for k in 0..per - 1 {
                let i = 109 + d * (per - 1) + k;
                let v = if i < fat_sectors {
                    fat_first + i as u32
                } else {
                    FREE
                };
                difat.extend_from_slice(&v.to_le_bytes());
            }
            let next = if d + 1 < difat_sectors {
                difat_first + d as u32 + 1
            } else {
                END_OF_CHAIN
            };
            difat.extend_from_slice(&next.to_le_bytes());
        }

        let mut out = vec![0u8; sector];
        out[..8].copy_from_slice(&MAGIC);
        out[24..26].copy_from_slice(&0x3Eu16.to_le_bytes());
        out[26..28].copy_from_slice(&major.to_le_bytes());
        out[28..30].copy_from_slice(&0xFFFEu16.to_le_bytes());
        out[30..32].copy_from_slice(&(shift as u16).to_le_bytes());
        out[32..34].copy_from_slice(&6u16.to_le_bytes());
        if major == 4 {
            out[40..44].copy_from_slice(&(dir_sectors as u32).to_le_bytes());
        }
        out[44..48].copy_from_slice(&(fat_sectors as u32).to_le_bytes());
        out[48..52].copy_from_slice(&dir_start.to_le_bytes());
        out[56..60].copy_from_slice(&4096u32.to_le_bytes());
        out[60..64].copy_from_slice(&minifat_start.to_le_bytes());
        out[64..68].copy_from_slice(&(minifat_sectors as u32).to_le_bytes());
        let difat_start = if difat_sectors > 0 {
            difat_first
        } else {
            END_OF_CHAIN
        };
        out[68..72].copy_from_slice(&difat_start.to_le_bytes());
        out[72..76].copy_from_slice(&(difat_sectors as u32).to_le_bytes());
        for i in 0..109 {
            let v = if i < fat_sectors {
                fat_first + i as u32
            } else {
                FREE
            };
            out[76 + i * 4..80 + i * 4].copy_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&data);
        out.extend_from_slice(&difat);
        for v in &fat {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::build::{file, storage, stream};
    use super::*;

    fn open(bytes: &[u8]) -> Result<Cfb, String> {
        let mut src = bytes;
        Cfb::open(&mut src, 10_000)
    }

    fn sample(major: u16) -> Vec<u8> {
        let big: Vec<u8> = (0..10_000u32).map(|i| (i % 251) as u8).collect();
        file(
            major,
            vec![
                stream(
                    "small",
                    b"sixty-four bytes and a few more, to cross one mini-sector: 0123456789",
                ),
                stream("big", &big),
                storage(
                    "folder",
                    vec![stream("inner", b"inside"), stream("empty", b"")],
                ),
            ],
        )
    }

    #[test]
    fn both_sector_sizes_list_the_same_tree_and_read_the_same_bytes() {
        for major in [3, 4] {
            let bytes = sample(major);
            let cfb = open(&bytes).unwrap();
            let mut src = bytes.as_slice();
            let names: Vec<String> = cfb
                .children(cfb.root())
                .iter()
                .map(|&i| cfb.entry(i).unwrap().name.clone())
                .collect();
            assert_eq!(names, ["small", "big", "folder"], "v{major}");
            // A short stream comes out of the mini-stream…
            let small = cfb.child(0, "SMALL").unwrap();
            assert_eq!(
                cfb.read(&mut src, small, u64::MAX).unwrap(),
                b"sixty-four bytes and a few more, to cross one mini-sector: 0123456789"
            );
            // …and a long one out of sectors of its own.
            let big = cfb.child(0, "big").unwrap();
            let read = cfb.read(&mut src, big, u64::MAX).unwrap();
            assert_eq!(read.len(), 10_000);
            assert!(read.iter().enumerate().all(|(i, &b)| b == (i % 251) as u8));
            let folder = cfb.child(0, "folder").unwrap();
            assert_eq!(cfb.entry(folder).unwrap().kind, Kind::Storage);
            let inner = cfb.child(folder, "inner").unwrap();
            assert_eq!(cfb.read(&mut src, inner, u64::MAX).unwrap(), b"inside");
            let empty = cfb.child(folder, "empty").unwrap();
            assert_eq!(cfb.read(&mut src, empty, u64::MAX).unwrap(), b"");
            // A storage is not a stream, and a child is looked up in its own storage only.
            assert!(cfb.read(&mut src, folder, u64::MAX).is_err());
            assert!(cfb.child(0, "inner").is_none());
            // The cap refuses before anything is read.
            assert!(cfb.read(&mut src, big, 100).is_err());
        }
    }

    #[test]
    fn a_fat_too_long_for_the_header_is_followed_through_the_difat() {
        // Eight megabytes in 512-byte sectors is more FAT than the header's 109 places hold.
        let big: Vec<u8> = (0..8_400_000u32).map(|i| (i % 249) as u8).collect();
        let bytes = file(
            3,
            vec![stream("big", &big), stream("after", b"still found")],
        );
        assert!(le32(&bytes, 44) > 109);
        assert_eq!(le32(&bytes, 72), 1);
        let cfb = open(&bytes).unwrap();
        let mut src = bytes.as_slice();
        let read = cfb
            .read(&mut src, cfb.child(0, "big").unwrap(), u64::MAX)
            .unwrap();
        assert!(read == big);
        let after = cfb.child(0, "after").unwrap();
        assert_eq!(cfb.read(&mut src, after, u64::MAX).unwrap(), b"still found");
        // A DIFAT sector that names itself as the next one is not followed for ever.
        let at = (le32(&bytes, 68) as usize + 1) * 512;
        let mut bad = bytes.clone();
        let own = le32(&bytes, 68).to_le_bytes();
        bad[at + 508..at + 512].copy_from_slice(&own);
        bad[at..at + 508].fill(0xFF);
        let _ = open(&bad);
    }

    #[test]
    fn a_file_cut_anywhere_is_an_error_or_a_shorter_file_and_never_a_panic() {
        for major in [3, 4] {
            let bytes = sample(major);
            for cut in (0..bytes.len()).step_by(37) {
                let part = &bytes[..cut];
                if let Ok(cfb) = open(part) {
                    let mut src = part;
                    for i in 0..cfb.entries.len() {
                        let _ = cfb.read(&mut src, i, u64::MAX);
                        let _ = cfb.children(i);
                    }
                }
            }
        }
    }

    #[test]
    fn rubbish_in_any_byte_is_survived() {
        // Not a fuzzer, but the same idea run over every table: each byte of the header, the
        // FAT and the directory is replaced in turn with values that make loops and wild
        // offsets, and the reader has to come back with an answer either way.
        for major in [3, 4] {
            let bytes = sample(major);
            let sector = if major == 4 { 4096 } else { 512 };
            let tables: Vec<usize> = (0..sector.min(512))
                .chain(bytes.len() - 3 * sector..bytes.len())
                .collect();
            for &at in &tables {
                for v in [0x00, 0x01, 0x7F, 0xFE, 0xFF] {
                    let mut bad = bytes.clone();
                    bad[at] = v;
                    if let Ok(cfb) = open(&bad) {
                        let mut src = bad.as_slice();
                        for i in 0..cfb.entries.len() {
                            let _ = cfb.read(&mut src, i, u64::MAX);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn a_chain_that_comes_back_to_itself_is_an_error_and_not_a_loop() {
        assert_eq!(
            chain(&[1, 2, 0], 0, None),
            Err("a sector chain loops".into())
        );
        assert_eq!(chain(&[1, 2, END_OF_CHAIN], 0, None), Ok(vec![0, 1, 2]));
        // A reader that knows how many sectors it wants never walks further than that.
        assert_eq!(chain(&[1, 2, 0], 0, Some(2)), Ok(vec![0, 1]));
        assert!(chain(&[7], 0, None).is_err());

        // The same in a whole file: the directory's chain pointed back at itself.
        let bytes = sample(3);
        let cfb = open(&bytes).unwrap();
        let dir_start = le32(&bytes, 48) as usize;
        drop(cfb);
        let fat_at = bytes.len() - le32(&bytes, 44) as usize * 512;
        let mut bad = bytes.clone();
        bad[fat_at + dir_start * 4..fat_at + dir_start * 4 + 4]
            .copy_from_slice(&(dir_start as u32).to_le_bytes());
        assert!(open(&bad).unwrap_err().contains("loops"));
    }

    #[test]
    fn a_directory_tree_that_loops_gives_each_entry_one_parent() {
        let mut bytes = sample(3);
        // Entry 1's right sibling is made entry 1 itself, and the folder its own child.
        let cfb = open(&bytes).unwrap();
        let dir = ((le32(&bytes, 48) as usize) + 1) * 512;
        bytes[dir + ENTRY + 68..dir + ENTRY + 72].copy_from_slice(&1u32.to_le_bytes());
        let folder = cfb.child(0, "folder").unwrap();
        bytes[dir + folder * ENTRY + 76..dir + folder * ENTRY + 80]
            .copy_from_slice(&(folder as u32).to_le_bytes());
        let cfb = open(&bytes).unwrap();
        let total: usize = (0..cfb.entries.len()).map(|i| cfb.children(i).len()).sum();
        assert!(total < cfb.entries.len());
    }

    #[test]
    fn what_is_not_a_compound_file_says_so() {
        assert!(open(b"").is_err());
        assert!(open(&[0u8; 600]).is_err());
        let mut bytes = sample(3);
        bytes[30] = 20;
        assert!(open(&bytes).is_err());
        // More entries than the caller allows is a refusal.
        let bytes = sample(3);
        let mut src = bytes.as_slice();
        assert!(Cfb::open(&mut src, 2).unwrap_err().contains("entries"));
    }

    #[test]
    fn a_filetime_is_a_date() {
        // 2024-03-05 14:30:00 UTC.
        let ft = (1_709_649_000u64 + 11_644_473_600) * 10_000_000;
        assert_eq!(
            filetime_text(ft).as_deref(),
            Some("2024-03-05 14:30:00 UTC")
        );
        assert_eq!(filetime_text(0), None);
    }
}
