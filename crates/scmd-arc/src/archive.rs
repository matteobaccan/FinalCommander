use std::collections::HashMap;
use std::ffi::OsString;
use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use scmd_core::{CancelToken, Entry, EntryFlags, EntryId, EntryKind};

use crate::{ArcError, Format, Limits, Result};

/// One entry of an archive, as the index holds it.
#[derive(Debug, Clone)]
pub struct ArcEntry {
    /// Where it sits inside the archive: `/`-separated, always starting with `/`, never ending
    /// with one. The root itself is `/`.
    pub path: String,
    pub size: u64,
    /// What it takes up in the archive. `None` for a format that does not say per entry.
    pub packed: Option<u64>,
    pub mtime: Option<SystemTime>,
    /// Unix permission bits, when the archive carries them.
    pub mode: Option<u32>,
    pub kind: EntryKind,
    pub link_target: Option<String>,
    /// Where to find it again: the index for a zip, the byte offset for a plain tar. Neither
    /// is meaningful for a compressed tar, which is read from the start every time.
    at: Option<u64>,
}

impl ArcEntry {
    /// The entry's own name, without the directories above it.
    pub fn name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or("")
    }

    /// The directory it lives in: `/` for an entry at the root.
    pub fn parent(&self) -> &str {
        match self.path.rfind('/') {
            Some(0) | None => "/",
            Some(cut) => &self.path[..cut],
        }
    }

    fn to_entry(&self) -> Entry {
        let mut flags = EntryFlags::empty();
        if self.name().starts_with('.') {
            flags.insert(EntryFlags::HIDDEN);
        }
        if let Some(mode) = self.mode {
            if mode & 0o111 != 0 && self.kind == EntryKind::File {
                flags.insert(EntryFlags::EXECUTABLE);
            }
        }
        Entry {
            name: OsString::from(self.name()),
            kind: self.kind,
            size: self.size,
            mtime: self.mtime,
            // An archive carries one timestamp per entry, and it is the modification time.
            btime: None,
            atime: None,
            flags,
            // An archive has no inodes, so the path inside it is the identity — which is
            // stable across a re-read and across sorting, which is what the panel needs.
            id: EntryId::Name(hash(&self.path)),
            link_target: self.link_target.as_deref().map(OsString::from),
        }
    }
}

fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut h);
    h.finish()
}

/// An archive, read once and then answered from the index.
///
/// Opening builds the whole index: a zip has one at the end, and a tar has to be walked. That
/// walk is the price of the first `Enter`, and a [`Cache`] keeps it so going in and out of an
/// archive does not pay it again.
#[derive(Debug)]
pub struct Archive {
    format: Format,
    source: PathBuf,
    /// Size and modification time when it was read, so the cache can tell a stale index.
    stamp: (u64, Option<SystemTime>),
    entries: Vec<ArcEntry>,
    /// Entry index by path, and the children of each directory, both built once.
    by_path: HashMap<String, usize>,
    children: HashMap<String, Vec<usize>>,
}

impl Archive {
    /// Read the index of the archive at `path`.
    pub fn open(path: &Path, limits: &Limits, cancel: &CancelToken) -> Result<Archive> {
        let name = path.display().to_string();
        let meta = std::fs::metadata(path).map_err(|e| ArcError::Io {
            path: name.clone(),
            source: e,
        })?;
        let mut head = [0u8; 264];
        let read = {
            let mut file = File::open(path).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?;
            read_up_to(&mut file, &mut head).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?
        };
        let leaf = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let format =
            Format::of_bytes(&head[..read], &leaf).ok_or(ArcError::Unknown(name.clone()))?;

        let entries = match format {
            Format::Zip => index_zip(path, &name, limits, cancel)?,
            _ => index_tar(path, &name, format, limits, cancel)?,
        };

        let packed = meta.len();
        let total: u64 = entries.iter().map(|e| e.size).sum();
        if total > limits.total_bytes {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it unpacks to {total} bytes, over the limit of {}",
                    limits.total_bytes
                ),
            });
        }
        // The ratio guard only means anything once the archive is big enough for the division
        // to say something: a 200-byte zip of a 1 MiB file of zeroes is not an attack.
        if packed > 4096 && limits.ratio != u64::MAX && total / packed.max(1) > limits.ratio {
            return Err(ArcError::Refused {
                path: name,
                reason: format!(
                    "it unpacks to {}× its own size, over the limit of {}×",
                    total / packed.max(1),
                    limits.ratio
                ),
            });
        }

        let mut archive = Archive {
            format,
            source: path.to_path_buf(),
            stamp: (meta.len(), meta.modified().ok()),
            entries,
            by_path: HashMap::new(),
            children: HashMap::new(),
        };
        archive.build_tree();
        Ok(archive)
    }

    /// The directories an archive does not bother to list are still directories: a tar of
    /// `a/b/c.txt` with no entry for `a/b` must still show `a` and then `b`.
    fn build_tree(&mut self) {
        let mut synthetic: Vec<String> = Vec::new();
        {
            let known: std::collections::HashSet<&str> =
                self.entries.iter().map(|e| e.path.as_str()).collect();
            for entry in &self.entries {
                let mut dir = entry.parent();
                while dir != "/" {
                    if !known.contains(dir) && !synthetic.iter().any(|s| s == dir) {
                        synthetic.push(dir.to_string());
                    }
                    dir = match dir.rfind('/') {
                        Some(0) | None => "/",
                        Some(cut) => &dir[..cut],
                    };
                }
            }
        }
        for path in synthetic {
            self.entries.push(ArcEntry {
                path,
                size: 0,
                packed: None,
                mtime: None,
                mode: None,
                kind: EntryKind::Dir,
                link_target: None,
                at: None,
            });
        }
        for (i, entry) in self.entries.iter().enumerate() {
            self.by_path.insert(entry.path.clone(), i);
            self.children
                .entry(entry.parent().to_string())
                .or_default()
                .push(i);
        }
    }

    pub fn format(&self) -> Format {
        self.format
    }

    pub fn source(&self) -> &Path {
        &self.source
    }

    pub fn entries(&self) -> &[ArcEntry] {
        &self.entries
    }

    /// Files, directories and the total unpacked size, for the panel's footer.
    pub fn totals(&self) -> (u64, u64, u64) {
        let mut files = 0;
        let mut dirs = 0;
        let mut bytes = 0;
        for entry in &self.entries {
            if entry.kind == EntryKind::Dir {
                dirs += 1;
            } else {
                files += 1;
                bytes += entry.size;
            }
        }
        (files, dirs, bytes)
    }

    /// Is the index still the one for the file on disk?
    pub fn is_current(&self) -> bool {
        match std::fs::metadata(&self.source) {
            Ok(meta) => self.stamp == (meta.len(), meta.modified().ok()),
            Err(_) => false,
        }
    }

    pub fn entry(&self, inner: &str) -> Option<&ArcEntry> {
        let inner = normalise(inner);
        self.by_path.get(&inner).map(|i| &self.entries[*i])
    }

    /// What is immediately inside `dir`, as panel rows. The `..` row is the panel's business,
    /// not ours.
    pub fn list(&self, dir: &str) -> Vec<Entry> {
        let dir = normalise(dir);
        let mut out: Vec<Entry> = self
            .children
            .get(&dir)
            .map(|kids| kids.iter().map(|i| self.entries[*i].to_entry()).collect())
            .unwrap_or_default();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Is this a directory inside the archive (so `Enter` goes into it)?
    pub fn is_dir(&self, inner: &str) -> bool {
        let inner = normalise(inner);
        inner == "/"
            || self
                .entry(&inner)
                .map(|e| e.kind == EntryKind::Dir)
                .unwrap_or(false)
    }

    /// Every file at or under `inner`, in archive order — what an extraction of a directory,
    /// or of the whole archive, has to write.
    pub fn under(&self, inner: &str) -> Vec<&ArcEntry> {
        let inner = normalise(inner);
        self.entries
            .iter()
            .filter(|e| {
                if inner == "/" {
                    return true;
                }
                e.path == inner
                    || (e.path.starts_with(&inner)
                        && e.path.as_bytes().get(inner.len()) == Some(&b'/'))
            })
            .collect()
    }

    /// The bytes of one entry. The viewer and the editor want them in memory; an extraction
    /// goes through [`Archive::extract`] instead and never holds a whole file.
    pub fn read(&self, inner: &str, cancel: &CancelToken) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        self.copy_out(inner, &mut out, cancel, &mut |_| {})?;
        Ok(out)
    }

    /// Write one entry to `dst`, reporting the bytes written as they go.
    pub fn extract(
        &self,
        inner: &str,
        dst: &Path,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64> {
        let name = dst.display().to_string();
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ArcError::Io {
                path: name.clone(),
                source: e,
            })?;
        }
        let file = File::create(dst).map_err(|e| ArcError::Io {
            path: name.clone(),
            source: e,
        })?;
        let mut file = std::io::BufWriter::new(file);
        let written = self.copy_out(inner, &mut file, cancel, progress)?;
        file.flush().map_err(|e| ArcError::Io {
            path: name,
            source: e,
        })?;
        Ok(written)
    }

    fn copy_out(
        &self,
        inner: &str,
        out: &mut dyn Write,
        cancel: &CancelToken,
        progress: &mut dyn FnMut(u64),
    ) -> Result<u64> {
        let inner = normalise(inner);
        let entry = self.entry(&inner).ok_or_else(|| ArcError::NoEntry {
            path: self.name(),
            entry: inner.clone(),
        })?;
        if entry.kind == EntryKind::Dir {
            return Err(ArcError::NoEntry {
                path: self.name(),
                entry: format!("{inner} is a directory"),
            });
        }
        match self.format {
            Format::Zip => {
                let file = File::open(&self.source).map_err(|e| self.io(e))?;
                let mut zip =
                    zip::ZipArchive::new(BufReader::new(file)).map_err(|e| self.broken(e))?;
                let index = entry.at.unwrap_or(0) as usize;
                let mut item = zip.by_index(index).map_err(|e| self.broken(e))?;
                copy(&mut item, out, cancel, progress)
            }
            Format::Tar => {
                // A plain tar says where each entry's bytes start, so one seek is enough.
                let mut file = File::open(&self.source).map_err(|e| self.io(e))?;
                let at = entry.at.ok_or_else(|| ArcError::Broken {
                    path: self.name(),
                    reason: "the index does not say where this entry is".to_string(),
                })?;
                file.seek(SeekFrom::Start(at)).map_err(|e| self.io(e))?;
                let mut window = file.take(entry.size);
                copy(&mut window, out, cancel, progress)
            }
            // A compressed tar is one stream: the only way to the last entry is through all
            // of it. That is the format's doing, not ours, and it is why `seekable` says no.
            _ => {
                let mut stream = tar::Archive::new(decoder(&self.source, self.format)?);
                let items = stream.entries().map_err(|e| self.broken(e))?;
                for item in items {
                    let mut item = item.map_err(|e| self.broken(e))?;
                    let path = item
                        .path()
                        .map(|p| tar_path(&p))
                        .unwrap_or_else(|_| String::new());
                    if path == inner {
                        return copy(&mut item, out, cancel, progress);
                    }
                    if cancel.is_cancelled() {
                        return Err(ArcError::Cancelled);
                    }
                }
                Err(ArcError::NoEntry {
                    path: self.name(),
                    entry: inner,
                })
            }
        }
    }

    fn name(&self) -> String {
        self.source.display().to_string()
    }

    fn io(&self, source: io::Error) -> ArcError {
        ArcError::Io {
            path: self.name(),
            source,
        }
    }

    fn broken(&self, reason: impl std::fmt::Display) -> ArcError {
        ArcError::Broken {
            path: self.name(),
            reason: reason.to_string(),
        }
    }
}

fn copy(
    from: &mut dyn Read,
    to: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64),
) -> Result<u64> {
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let read = from.read(&mut buf).map_err(|e| ArcError::Io {
            path: "the archive".to_string(),
            source: e,
        })?;
        if read == 0 {
            return Ok(total);
        }
        to.write_all(&buf[..read]).map_err(|e| ArcError::Io {
            path: "the destination".to_string(),
            source: e,
        })?;
        total += read as u64;
        progress(total);
    }
}

/// Read as much as fits, returning how much that was. `read` is allowed to stop early and a
/// short archive is shorter than the buffer, so neither is an error.
fn read_up_to(file: &mut File, buf: &mut [u8]) -> io::Result<usize> {
    let mut filled = 0;
    while filled < buf.len() {
        match file.read(&mut buf[filled..])? {
            0 => break,
            n => filled += n,
        }
    }
    Ok(filled)
}

/// The decompressor for a compressed tar, as a plain reader.
fn decoder(path: &Path, format: Format) -> Result<Box<dyn Read>> {
    let name = path.display().to_string();
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: name.clone(),
        source: e,
    })?;
    let file = BufReader::with_capacity(64 * 1024, file);
    Ok(match format {
        Format::Tar => Box::new(file),
        Format::TarGz => Box::new(flate2::read::GzDecoder::new(file)),
        Format::TarBz2 => Box::new(bzip2::read::BzDecoder::new(file)),
        Format::TarXz => Box::new(lzma_rust2::XzReader::new(file, true)),
        Format::TarZst => Box::new(ruzstd::decoding::StreamingDecoder::new(file).map_err(|e| {
            ArcError::Broken {
                path: name,
                reason: e.to_string(),
            }
        })?),
        Format::Zip => {
            return Err(ArcError::Broken {
                path: name,
                reason: "a zip is not a stream".to_string(),
            })
        }
    })
}

/// `/`-separated, leading slash, no trailing slash, `.` and `..` resolved. An entry whose name
/// climbs out of the archive is pinned at the root instead: a zip that says `../../etc/passwd`
/// is a zip-slip attack, and the answer is to keep it inside, not to trust it.
fn normalise(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for part in path.split(['/', '\\']) {
        match part {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return "/".to_string();
    }
    format!("/{}", out.join("/"))
}

fn tar_path(path: &Path) -> String {
    normalise(&path.to_string_lossy())
}

fn index_zip(
    path: &Path,
    name: &str,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let file = File::open(path).map_err(|e| ArcError::Io {
        path: name.to_string(),
        source: e,
    })?;
    let mut zip = zip::ZipArchive::new(BufReader::new(file)).map_err(|e| ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    })?;
    if zip.len() > limits.entries {
        return Err(ArcError::Refused {
            path: name.to_string(),
            reason: format!(
                "{} entries, over the limit of {}",
                zip.len(),
                limits.entries
            ),
        });
    }
    let mut out = Vec::with_capacity(zip.len());
    for i in 0..zip.len() {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let item = zip.by_index(i).map_err(|e| ArcError::Broken {
            path: name.to_string(),
            reason: e.to_string(),
        })?;
        // An entry whose name we cannot read is skipped rather than losing the archive.
        let Ok(raw) = item.name() else { continue };
        let inner = normalise(&raw);
        if inner == "/" {
            continue;
        }
        let mode = item.unix_mode();
        let kind = if item.is_dir() {
            EntryKind::Dir
        } else if mode.map(|m| m & 0o170000 == 0o120000).unwrap_or(false) {
            EntryKind::Symlink
        } else {
            EntryKind::File
        };
        out.push(ArcEntry {
            path: inner,
            size: item.size(),
            packed: Some(item.compressed_size()),
            mtime: item.last_modified().and_then(system_time),
            mode,
            kind,
            link_target: None,
            at: Some(i as u64),
        });
    }
    Ok(out)
}

fn index_tar(
    path: &Path,
    name: &str,
    format: Format,
    limits: &Limits,
    cancel: &CancelToken,
) -> Result<Vec<ArcEntry>> {
    let mut archive = tar::Archive::new(decoder(path, format)?);
    let items = archive.entries().map_err(|e| ArcError::Broken {
        path: name.to_string(),
        reason: e.to_string(),
    })?;
    let mut out = Vec::new();
    for item in items {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        if out.len() >= limits.entries {
            return Err(ArcError::Refused {
                path: name.to_string(),
                reason: format!("more than {} entries", limits.entries),
            });
        }
        let item = item.map_err(|e| ArcError::Broken {
            path: name.to_string(),
            reason: e.to_string(),
        })?;
        let header = item.header();
        let Ok(inner) = item.path().map(|p| tar_path(&p)) else {
            continue;
        };
        if inner == "/" {
            continue;
        }
        let kind = match header.entry_type() {
            tar::EntryType::Directory => EntryKind::Dir,
            tar::EntryType::Regular | tar::EntryType::Continuous => EntryKind::File,
            tar::EntryType::Symlink | tar::EntryType::Link => EntryKind::Symlink,
            // Devices, fifos and the GNU bookkeeping entries: shown, never read.
            _ => EntryKind::Other,
        };
        out.push(ArcEntry {
            path: inner,
            size: header.size().unwrap_or(0),
            packed: None,
            mtime: header
                .mtime()
                .ok()
                .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs)),
            mode: header.mode().ok(),
            kind,
            link_target: header
                .link_name()
                .ok()
                .flatten()
                .map(|p| p.to_string_lossy().into_owned()),
            // Only a plain tar can be seeked back to; the compressed ones re-read the stream.
            at: format.seekable().then(|| item.raw_file_position()),
        });
    }
    Ok(out)
}

/// A zip timestamp is MS-DOS local time with no zone at all. We read it as UTC, which is what
/// every other tool does, and keep `None` for the dates a zip cannot hold (before 1980).
fn system_time(at: zip::DateTime) -> Option<SystemTime> {
    if !at.is_valid() {
        return None;
    }
    let days = days_from_civil(at.year() as i64, at.month() as u32, at.day() as u32)?;
    let secs = days * 86_400
        + i64::from(at.hour()) * 3_600
        + i64::from(at.minute()) * 60
        + i64::from(at.second());
    if secs < 0 {
        return None;
    }
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs as u64))
}

/// Days from the epoch to a civil date, by Howard Hinnant's algorithm — no calendar crate
/// for one conversion.
fn days_from_civil(year: i64, month: u32, day: u32) -> Option<i64> {
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let y = if month <= 2 { year - 1 } else { year };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (month as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// The container cache of SPEC 04 §4: an open index per archive, so stepping in and out of one
/// does not read it again. Keyed by the file and its stamp, so an archive that changed on disk
/// is read afresh rather than answered from a stale index.
#[derive(Debug, Default)]
pub struct Cache {
    limits: Limits,
    /// Most recently used last; small by design, because an index can be large.
    open: Mutex<Vec<(PathBuf, Arc<Archive>)>>,
    capacity: usize,
}

impl Cache {
    pub fn new(limits: Limits) -> Cache {
        Cache {
            limits,
            open: Mutex::new(Vec::new()),
            capacity: 4,
        }
    }

    pub fn limits(&self) -> &Limits {
        &self.limits
    }

    /// The archive at `path`, from the cache when the file has not changed under us.
    pub fn open(&self, path: &Path, cancel: &CancelToken) -> Result<Arc<Archive>> {
        if let Ok(mut open) = self.open.lock() {
            if let Some(i) = open.iter().position(|(p, _)| p == path) {
                let (_, archive) = open[i].clone();
                if archive.is_current() {
                    let hit = open.remove(i);
                    open.push(hit);
                    return Ok(archive);
                }
                open.remove(i);
            }
        }
        let archive = Arc::new(Archive::open(path, &self.limits, cancel)?);
        if let Ok(mut open) = self.open.lock() {
            open.push((path.to_path_buf(), archive.clone()));
            while open.len() > self.capacity {
                open.remove(0);
            }
        }
        Ok(archive)
    }

    /// Forget an archive, for when something has written to it.
    pub fn forget(&self, path: &Path) {
        if let Ok(mut open) = self.open.lock() {
            open.retain(|(p, _)| p != path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same three files in every format, one of them two directories down.
    const TREE: [(&str, &[u8]); 3] = [
        ("readme.md", b"# the readme\n"),
        ("src/main.rs", b"fn main() {}\n"),
        ("src/deep/notes.txt", b"notes, two directories down\n"),
    ];

    fn write_zip(path: &Path, names: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, body) in names {
            zip.start_file(*name, options).unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    fn tar_bytes(names: &[(&str, &[u8])]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (name, body) in names {
            let mut header = tar::Header::new_gnu();
            header.set_size(body.len() as u64);
            header.set_mode(0o644);
            header.set_mtime(1_700_000_000);
            header.set_cksum();
            builder.append_data(&mut header, name, &body[..]).unwrap();
        }
        builder.into_inner().unwrap()
    }

    fn write_archive(path: &Path, format: Format, names: &[(&str, &[u8])]) {
        if format == Format::Zip {
            return write_zip(path, names);
        }
        let tar = tar_bytes(names);
        let out = File::create(path).unwrap();
        match format {
            Format::Tar => {
                let mut out = out;
                out.write_all(&tar).unwrap();
            }
            Format::TarGz => {
                let mut enc = flate2::write::GzEncoder::new(out, flate2::Compression::fast());
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarBz2 => {
                let mut enc = bzip2::write::BzEncoder::new(out, bzip2::Compression::fast());
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarXz => {
                let mut enc =
                    lzma_rust2::XzWriter::new(out, lzma_rust2::XzOptions::with_preset(1)).unwrap();
                enc.write_all(&tar).unwrap();
                enc.finish().unwrap();
            }
            Format::TarZst => {
                let mut out = out;
                let packed = ruzstd::encoding::compress_to_vec(
                    &tar[..],
                    ruzstd::encoding::CompressionLevel::Fastest,
                );
                out.write_all(&packed).unwrap();
            }
            Format::Zip => unreachable!("handled above"),
        }
    }

    fn open(path: &Path) -> Archive {
        Archive::open(path, &Limits::default(), &CancelToken::new()).expect("the archive")
    }

    #[test]
    fn every_format_lists_the_same_tree_and_reads_the_same_bytes() {
        let dir = tempfile::tempdir().unwrap();
        for format in Format::ALL {
            let path = dir.path().join(format!("tree.{}", format.extension()));
            write_archive(&path, format, &TREE);
            let archive = open(&path);
            assert_eq!(archive.format(), format, "{}", format.label());

            let root: Vec<String> = archive
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(root, ["readme.md", "src"], "{}", format.label());

            let src: Vec<String> = archive
                .list("/src")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(src, ["deep", "main.rs"], "{}", format.label());

            assert!(archive.is_dir("/src") && archive.is_dir("/src/deep"));
            assert!(!archive.is_dir("/readme.md"));
            // The root is a directory even in an archive that never says so.
            assert!(archive.is_dir("/"));

            let bytes = archive
                .read("/src/deep/notes.txt", &CancelToken::new())
                .expect("the entry");
            assert_eq!(bytes, TREE[2].1, "{}", format.label());
            // And the first entry too, so a seekable format is not read off by one.
            assert_eq!(
                archive.read("/readme.md", &CancelToken::new()).unwrap(),
                TREE[0].1,
                "{}",
                format.label()
            );

            let (files, dirs, bytes) = archive.totals();
            assert_eq!((files, dirs), (3, 2), "{}", format.label());
            assert_eq!(bytes, TREE.iter().map(|(_, b)| b.len() as u64).sum::<u64>());
        }
    }

    #[test]
    fn a_directory_nobody_wrote_down_is_still_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("deep.zip");
        // One entry, three levels down, and no entries for the levels themselves.
        write_zip(&path, &[("a/b/c.txt", b"x")]);
        let archive = open(&path);
        assert_eq!(archive.list("/").len(), 1);
        assert_eq!(archive.list("/")[0].name_str(), "a");
        assert_eq!(archive.list("/a")[0].name_str(), "b");
        assert_eq!(archive.list("/a/b")[0].name_str(), "c.txt");
        assert!(archive.is_dir("/a") && archive.is_dir("/a/b"));
    }

    #[test]
    fn an_entry_that_climbs_out_of_the_archive_stays_inside_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("slip.zip");
        write_zip(
            &path,
            &[("../../etc/passwd", b"root:x:0:0\n"), ("./ok.txt", b"ok")],
        );
        let archive = open(&path);
        // The name is pinned at the root rather than trusted, so no extraction of this
        // archive can write outside where it was told to.
        assert!(archive.entry("/etc/passwd").is_some());
        assert_eq!(
            archive
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect::<Vec<_>>(),
            ["etc", "ok.txt"]
        );
    }

    #[test]
    fn extraction_writes_the_file_where_it_is_told_and_counts_the_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.tar.gz");
        write_archive(&path, Format::TarGz, &TREE);
        let archive = open(&path);

        let out = dir.path().join("out/src/deep/notes.txt");
        let mut seen = 0;
        let written = archive
            .extract("/src/deep/notes.txt", &out, &CancelToken::new(), &mut |n| {
                seen = n
            })
            .expect("extraction");
        assert_eq!(written, TREE[2].1.len() as u64);
        assert_eq!(seen, written, "the progress ends where the file does");
        assert_eq!(std::fs::read(&out).unwrap(), TREE[2].1);

        // A directory is not a file: say so rather than writing an empty one.
        assert!(matches!(
            archive.extract(
                "/src",
                &dir.path().join("x"),
                &CancelToken::new(),
                &mut |_| {}
            ),
            Err(ArcError::NoEntry { .. })
        ));
    }

    #[test]
    fn under_gives_a_directory_and_everything_in_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);
        let archive = open(&path);
        let mut under: Vec<&str> = archive
            .under("/src")
            .iter()
            .map(|e| e.path.as_str())
            .collect();
        under.sort();
        assert_eq!(
            under,
            ["/src", "/src/deep", "/src/deep/notes.txt", "/src/main.rs"]
        );
        assert_eq!(archive.under("/").len(), archive.entries().len());
        // `/src` is not `/srcery`: a name prefix is not a path prefix.
        assert!(archive.under("/sr").is_empty());
    }

    #[test]
    fn the_guards_refuse_a_bomb_and_say_which_limit_fired() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);

        let few = Limits {
            entries: 2,
            ..Limits::default()
        };
        let err = Archive::open(&path, &few, &CancelToken::new()).unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains("limit of 2")),
            "{err}"
        );

        let small = Limits {
            total_bytes: 8,
            ..Limits::default()
        };
        let err = Archive::open(&path, &small, &CancelToken::new()).unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains("unpacks to")),
            "{err}"
        );

        // A tar of a megabyte of zeroes compresses far past the ratio limit, and that is
        // exactly the shape of a bomb.
        let zeroes = vec![0u8; 1 << 20];
        let bomb = dir.path().join("bomb.tar.gz");
        write_archive(&bomb, Format::TarGz, &[("zeroes", &zeroes)]);
        let err = Archive::open(
            &bomb,
            &Limits {
                ratio: 100,
                ..Limits::default()
            },
            &CancelToken::new(),
        )
        .unwrap_err();
        assert!(
            matches!(&err, ArcError::Refused { reason, .. } if reason.contains('×')),
            "{err}"
        );
        // And with the guard raised it reads fine: the limit is a decision, not a verdict.
        assert!(Archive::open(&bomb, &Limits::unlimited(), &CancelToken::new()).is_ok());
    }

    #[test]
    fn a_file_that_is_not_an_archive_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.txt");
        std::fs::write(&path, b"just text").unwrap();
        assert!(matches!(
            Archive::open(&path, &Limits::default(), &CancelToken::new()),
            Err(ArcError::Unknown(_))
        ));
    }

    #[test]
    fn the_cache_reads_an_archive_once_and_notices_when_it_changes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tree.zip");
        write_archive(&path, Format::Zip, &TREE);
        let cache = Cache::new(Limits::default());

        let first = cache.open(&path, &CancelToken::new()).unwrap();
        let again = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(Arc::ptr_eq(&first, &again), "the index was read twice");

        // Rewritten under us: the stamp no longer matches, so the index is read afresh.
        std::thread::sleep(std::time::Duration::from_millis(10));
        write_zip(&path, &[("only.txt", b"one")]);
        let third = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(!Arc::ptr_eq(&first, &third), "a stale index was handed out");
        assert_eq!(third.list("/").len(), 1);

        cache.forget(&path);
        let fourth = cache.open(&path, &CancelToken::new()).unwrap();
        assert!(!Arc::ptr_eq(&third, &fourth));
    }

    #[test]
    fn a_zip_timestamp_becomes_a_real_time() {
        // The epoch, and a date a zip can hold.
        assert_eq!(days_from_civil(1970, 1, 1), Some(0));
        assert_eq!(days_from_civil(2000, 1, 1), Some(10_957));
        assert_eq!(days_from_civil(1980, 1, 1), Some(3_652));
        assert_eq!(days_from_civil(2026, 10, 24), Some(20_750));
        assert_eq!(days_from_civil(2026, 13, 1), None);

        let dir = tempfile::tempdir().unwrap();
        // A zip written with a stamp of its own comes back with that stamp, read as UTC.
        let stamped = dir.path().join("stamped.zip");
        let file = File::create(&stamped).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "readme.md",
            zip::write::SimpleFileOptions::default().last_modified_time(
                zip::DateTime::from_date_and_time(2026, 10, 24, 9, 30, 0).unwrap(),
            ),
        )
        .unwrap();
        zip.write_all(b"x").unwrap();
        zip.finish().unwrap();

        let archive = open(&stamped);
        let entry = archive.entry("/readme.md").unwrap();
        let when = entry.mtime.expect("a zip carries a modification time");
        assert_eq!(
            when,
            SystemTime::UNIX_EPOCH + Duration::from_secs(20_750 * 86_400 + 9 * 3_600 + 30 * 60)
        );
    }
}
