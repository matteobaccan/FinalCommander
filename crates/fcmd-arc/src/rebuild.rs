//! Changing an archive, which means writing a new one.
//!
//! None of these formats can be edited in place. A zip could have an entry appended and the
//! old one orphaned, and a tar could have its tail rewritten, but a `.tar.gz` is one
//! compressed stream from end to end and nothing in it can be touched without rewriting all
//! of it. So there is one mechanism for all five, and it is the honest one: stream the old
//! archive into a new one beside it, applying the changes as it passes, check what came out,
//! and move it over the original (SPEC 04 §4).
//!
//! That is also what makes it safe. The original keeps its bytes until the new archive is
//! complete and has been read back, so an interrupted rebuild — a full disk, a cancelled job,
//! a crash — leaves the archive exactly as it was and a `.part` file to delete.
//!
//! What is copied is copied without being re-encoded where the format allows it: a zip entry
//! nobody asked about moves across as the compressed bytes it already is, which makes removing
//! one file from a hundred-megabyte archive a copy rather than a recompression.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use fcmd_core::CancelToken;

use crate::archive::{decoder, normalise, tar_path, Archive};
use crate::write::{Level, Written};
use crate::{ArcError, Format, Limits, Result};

/// One thing to do to the archive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    /// The bytes of `source` go in at `inner`, replacing whatever was there or adding it.
    Put { inner: String, source: PathBuf },
    /// `inner` comes out, and everything under it if it is a directory.
    Remove { inner: String },
    /// `from` keeps its bytes under the name `to`.
    Rename { from: String, to: String },
    /// `inner` keeps its bytes and its name, and is written with the permissions or the
    /// modification time given — whichever is `Some`. The archive is rebuilt for it like any
    /// other change, because these live in the entry's header and a header cannot be patched
    /// in a compressed stream.
    Touch {
        inner: String,
        /// The Unix mode, as a file's own is: `0o644`. A format that cannot hold one ignores
        /// it rather than failing, and the dialog says which.
        mode: Option<u32>,
        mtime: Option<SystemTime>,
    },
}

impl Change {
    /// Put a file in under the name it has on disk, in `dir` inside the archive.
    pub fn add(source: &Path, dir: &str) -> Change {
        let name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        let dir = normalise(dir);
        let inner = if dir == "/" {
            format!("/{name}")
        } else {
            format!("{dir}/{name}")
        };
        Change::Put {
            inner,
            source: source.to_path_buf(),
        }
    }
}

/// What the changes add up to, with the names normalised once so the loops below can compare.
#[derive(Debug, Default)]
struct Plan {
    /// Paths that go, and prefixes that take their trees with them.
    removed: Vec<String>,
    /// Old name to new name.
    renamed: BTreeMap<String, String>,
    /// New bytes by the name they take inside, in the order they were asked for.
    put: BTreeMap<String, PathBuf>,
    /// What to write in the header of an entry that keeps its bytes.
    touched: BTreeMap<String, (Option<u32>, Option<SystemTime>)>,
}

impl Plan {
    fn of(changes: &[Change]) -> Plan {
        let mut plan = Plan::default();
        for change in changes {
            match change {
                Change::Put { inner, source } => {
                    plan.put.insert(normalise(inner), source.clone());
                }
                Change::Remove { inner } => plan.removed.push(normalise(inner)),
                Change::Rename { from, to } => {
                    plan.renamed.insert(normalise(from), normalise(to));
                }
                Change::Touch { inner, mode, mtime } => {
                    plan.touched.insert(normalise(inner), (*mode, *mtime));
                }
            }
        }
        plan
    }

    /// Is this entry on its way out? A directory takes its tree.
    fn is_removed(&self, inner: &str) -> bool {
        self.removed
            .iter()
            .any(|gone| inner == gone || inner.starts_with(&format!("{gone}/")))
    }

    fn is_empty(&self) -> bool {
        self.removed.is_empty()
            && self.renamed.is_empty()
            && self.put.is_empty()
            && self.touched.is_empty()
    }
}

/// Apply `changes` to the archive at `path`, atomically.
///
/// The archive is read, a new one is written beside it, the new one is opened and checked, and
/// only then does it replace the original. What comes back counts what the new archive holds.
pub fn rebuild(
    path: &Path,
    changes: &[Change],
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> Result<Written> {
    let plan = Plan::of(changes);
    if plan.is_empty() {
        return Ok(Written::default());
    }
    let format =
        Format::of_path(path).ok_or_else(|| ArcError::Unknown(path.display().to_string()))?;
    let part = path.with_extension(format!("{}.part", format.extension()));
    let result = write_new(path, &part, format, &plan, level, cancel, progress)
        .and_then(|written| verify(&part, &plan, cancel).map(|_| written));
    match result {
        Ok(mut written) => {
            written.packed = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            std::fs::rename(&part, path).map_err(|e| ArcError::Io {
                path: path.display().to_string(),
                source: e,
            })?;
            Ok(written)
        }
        Err(e) => {
            // The original is untouched; what is left is the half-written copy.
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

fn write_new(
    path: &Path,
    part: &Path,
    format: Format,
    plan: &Plan,
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> Result<Written> {
    if format == Format::Zip {
        return rebuild_zip(path, part, plan, level, cancel, progress);
    }
    if format == Format::SevenZ {
        return rebuild_sevenz(path, part, plan, level, cancel, progress);
    }
    rebuild_tar(path, part, format, plan, level, cancel, progress)
}

fn rebuild_zip(
    path: &Path,
    part: &Path,
    plan: &Plan,
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> Result<Written> {
    let source = File::open(path).map_err(|e| io_error(path, e))?;
    let mut old = zip::ZipArchive::new(BufReader::new(source)).map_err(|e| broken(path, e))?;
    let target = File::create(part).map_err(|e| io_error(part, e))?;
    let mut new = zip::ZipWriter::new(BufWriter::new(target));
    let mut written = Written::default();

    for i in 0..old.len() {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        // What this entry is, read and then let go of: a header rewritten below needs the
        // archive borrowed again, this time for the bytes rather than for them compressed.
        let (stored, is_dir, bytes) = {
            let entry = old.by_index_raw(i).map_err(|e| broken(path, e))?;
            // A name a zip stores in an encoding we cannot read is left exactly as it is: it
            // is copied across untouched, and no change can name it.
            let stored = entry
                .name()
                .map(|n| n.into_owned())
                .map_err(|e| broken(path, e))?;
            (stored, entry.is_dir(), entry.size())
        };
        let inner = normalise(&stored);
        if plan.is_removed(&inner) {
            continue;
        }
        if plan.put.contains_key(&inner) {
            continue; // replaced from disk; the new bytes go in below
        }
        if let Some((mode, mtime)) = plan.touched.get(&inner) {
            // The mode and the time live in the entry's header, and a zip's header cannot be
            // patched where it lies, so this one entry is decompressed and written again
            // while every other is still copied across as the bytes it already is.
            let name = match plan.renamed.get(&inner) {
                None => stored.clone(),
                Some(to) => {
                    let mut name = to.trim_start_matches('/').to_string();
                    if is_dir && !name.ends_with('/') {
                        name.push('/');
                    }
                    name
                }
            };
            let mut options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            if let Some(mode) = mode {
                // And the entry is written as a Unix one: a zip made on Windows is a DOS
                // entry, whose mode a reader is entitled to invent from the attribute byte,
                // so the mode we were asked for would be read back as somebody else's.
                options = options.unix_permissions(*mode).system(zip::System::Unix);
            }
            if let Some(when) = mtime {
                if let Some(stamp) = zip_datetime(*when) {
                    options = options.last_modified_time(stamp);
                }
            }
            if is_dir {
                new.add_directory(name.trim_end_matches('/'), options)
                    .map_err(|e| broken(part, e))?;
                written.dirs += 1;
            } else {
                let mut entry = old.by_index(i).map_err(|e| broken(path, e))?;
                new.start_file(&name, options)
                    .map_err(|e| broken(part, e))?;
                let mut data = Vec::with_capacity(bytes.min(8 * 1024 * 1024) as usize);
                entry
                    .read_to_end(&mut data)
                    .map_err(|e| io_error(path, e))?;
                new.write_all(&data).map_err(|e| io_error(part, e))?;
                written.files += 1;
                written.bytes += bytes;
            }
            progress(&name, written.bytes);
            continue;
        }
        let entry = old.by_index_raw(i).map_err(|e| broken(path, e))?;
        // An entry nobody renamed keeps its stored name exactly — not the normalised one.
        // The difference matters: a zip marks a directory by the `/` its name ends with, and
        // normalising takes that away, which would turn every directory into an empty file.
        let name = match plan.renamed.get(&inner) {
            None => {
                let name = stored.clone();
                new.raw_copy_file(entry).map_err(|e| broken(part, e))?;
                name
            }
            Some(to) => {
                let mut name = to.trim_start_matches('/').to_string();
                if is_dir && !name.ends_with('/') {
                    name.push('/');
                }
                new.raw_copy_file_rename(entry, &name)
                    .map_err(|e| broken(part, e))?;
                name
            }
        };
        if is_dir {
            written.dirs += 1;
        } else {
            written.files += 1;
            written.bytes += bytes;
        }
        progress(&name, written.bytes);
    }

    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(match level {
            Level::Fast => 1,
            Level::Normal => 6,
            Level::Best => 9,
        }));
    for (inner, source) in &plan.put {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let name = inner.trim_start_matches('/');
        new.start_file(name, options).map_err(|e| broken(part, e))?;
        written.bytes += pour(source, &mut new, cancel)?;
        written.files += 1;
        progress(name, written.bytes);
    }
    new.finish().map_err(|e| broken(part, e))?;
    Ok(written)
}

/// A moment as a zip stores one: MS-DOS date and time, to a two-second tick, in whatever the
/// local clock calls now. Out of range — a zip cannot hold a date before 1980 — is `None`, and
/// the entry then keeps the time the writer gives it rather than a wrong one.
fn zip_datetime(when: SystemTime) -> Option<zip::DateTime> {
    let secs = when.duration_since(SystemTime::UNIX_EPOCH).ok()?.as_secs() as i64;
    let (days, rest) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let (year, month, day) = civil_from_days(days);
    zip::DateTime::from_date_and_time(
        u16::try_from(year).ok()?,
        u8::try_from(month).ok()?,
        u8::try_from(day).ok()?,
        (rest / 3_600) as u8,
        ((rest % 3_600) / 60) as u8,
        (rest % 60) as u8,
    )
    .ok()
}

/// Days from the epoch to a civil date, by Howard Hinnant's algorithm — the inverse of the
/// one the index reader uses, and here for the same reason: one conversion does not want a
/// calendar crate behind it.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn rebuild_tar(
    path: &Path,
    part: &Path,
    format: Format,
    plan: &Plan,
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> Result<Written> {
    // As in `create`: zstd has no streaming writer here, so that format builds the tar beside
    // the archive and compresses it in a second pass.
    let plain = if format == Format::TarZst {
        part.with_extension("tar.part")
    } else {
        part.to_path_buf()
    };
    let mut written = Written::default();
    {
        let sink = crate::write::Sink::new(&plain, format, level)?;
        let mut new = tar::Builder::new(sink);
        let mut old = tar::Archive::new(decoder(path, format)?);
        let entries = old.entries().map_err(|e| broken(path, e))?;
        for entry in entries {
            if cancel.is_cancelled() {
                let _ = std::fs::remove_file(&plain);
                return Err(ArcError::Cancelled);
            }
            let mut entry = entry.map_err(|e| broken(path, e))?;
            // Two forms of the same name: the one the tar stores, which is relative and may
            // end in a slash, and the normalised one the changes are written against.
            let stored = entry.path().map(|p| p.into_owned()).unwrap_or_default();
            let inner = normalise(&tar_path(&stored));
            if plan.is_removed(&inner) {
                continue;
            }
            if plan.put.contains_key(&inner) {
                continue;
            }
            // As in the zip: the stored name is kept unless something renamed it, so a
            // trailing slash or a `./` prefix survives the rebuild untouched.
            let name = match plan.renamed.get(&inner) {
                None => stored.clone(),
                Some(to) => PathBuf::from(to.trim_start_matches('/')),
            };
            let mut header = entry.header().clone();
            let is_dir = header.entry_type().is_dir();
            let size = header.size().unwrap_or(0);
            // A tar's header is written afresh for every entry anyway, so what was asked for
            // simply goes in it.
            if let Some((mode, mtime)) = plan.touched.get(&inner) {
                if let Some(mode) = mode {
                    header.set_mode(*mode);
                }
                if let Some(when) = mtime {
                    if let Ok(since) = when.duration_since(std::time::UNIX_EPOCH) {
                        header.set_mtime(since.as_secs());
                    }
                }
            }
            // The data has to be read out of the stream whatever happens to it, because a tar
            // is read in order; holding it is bounded by the entry's own size.
            let mut data = Vec::with_capacity(size.min(8 * 1024 * 1024) as usize);
            entry
                .read_to_end(&mut data)
                .map_err(|e| io_error(path, e))?;
            header.set_size(data.len() as u64);
            header.set_cksum();
            new.append_data(&mut header, &name, data.as_slice())
                .map_err(|e| broken(part, e))?;
            if is_dir {
                written.dirs += 1;
            } else {
                written.files += 1;
                written.bytes += size;
            }
            progress(&name.to_string_lossy(), written.bytes);
        }
        for (inner, source) in &plan.put {
            if cancel.is_cancelled() {
                let _ = std::fs::remove_file(&plain);
                return Err(ArcError::Cancelled);
            }
            let name = inner.trim_start_matches('/');
            let mut file = File::open(source).map_err(|e| io_error(source, e))?;
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            new.append_file(name, &mut file)
                .map_err(|e| broken(part, e))?;
            written.files += 1;
            written.bytes += size;
            progress(name, written.bytes);
        }
        let sink = new.into_inner().map_err(|e| broken(part, e))?;
        sink.finish(part)?;
    }
    if format == Format::TarZst {
        let source = File::open(&plain).map_err(|e| io_error(&plain, e))?;
        let target = File::create(part).map_err(|e| io_error(part, e))?;
        ruzstd::encoding::compress(
            BufReader::new(source),
            BufWriter::new(target),
            match level {
                Level::Fast => ruzstd::encoding::CompressionLevel::Fastest,
                Level::Normal => ruzstd::encoding::CompressionLevel::Default,
                Level::Best => ruzstd::encoding::CompressionLevel::Better,
            },
        );
        let _ = std::fs::remove_file(&plain);
    }
    Ok(written)
}

fn rebuild_sevenz(
    path: &Path,
    part: &Path,
    plan: &Plan,
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&str, u64),
) -> Result<Written> {
    // Nothing is copied across as the bytes it already is, unlike the zip above: a 7z block
    // holds several files in one compressed stream, so even the entries nobody asked about
    // have to be decoded and packed again. That is the format, and it is why changing one
    // file in a large 7z costs what it costs.
    // A rebuild is of an archive on this disk that nobody has locked: an encrypted one is
    // refused when it is opened, long before a change is planned for it.
    let mut old = crate::archive::sevenz_reader(path, None)?;
    let target = File::create(part).map_err(|e| io_error(part, e))?;
    let mut new =
        sevenz_rust2::ArchiveWriter::new(BufWriter::new(target)).map_err(|e| sevenz(part, e))?;
    new.set_content_methods(vec![crate::write::sevenz_method(level)]);
    let mut written = Written::default();
    let mut stopped: Option<ArcError> = None;

    let walk = old.for_each_entries(|entry, bytes| {
        if cancel.is_cancelled() {
            stopped = Some(ArcError::Cancelled);
            return Ok(false);
        }
        let inner = normalise(&entry.name);
        if plan.is_removed(&inner) || plan.put.contains_key(&inner) {
            // Even an entry on its way out has to be read past: in a solid block the files
            // are one stream between them, and leaving one unread would leave the stream in
            // the middle of it and make rubble of the next.
            std::io::copy(bytes, &mut std::io::sink())?;
            return Ok(true);
        }
        let touch = plan.touched.get(&inner).copied();
        let name = plan.renamed.get(&inner).cloned().unwrap_or(inner);
        let mut copy = entry.clone();
        copy.name = name.trim_start_matches('/').to_string();
        if let Some((mode, mtime)) = touch {
            // A 7z keeps the Unix mode in the top half of the Windows attribute word with bit
            // 15 set, which is the convention `p7zip` and `7-Zip` both follow and the only
            // place the mode of a file in a 7z is ever recorded.
            if let Some(mode) = mode {
                copy.windows_attributes =
                    (mode << 16) | 0x8000 | (copy.windows_attributes & 0xffff);
                copy.has_windows_attributes = true;
            }
            if let Some(when) = mtime {
                if let Ok(stamp) = sevenz_rust2::NtTime::try_from(when) {
                    copy.last_modified_date = stamp;
                    copy.has_last_modified_date = true;
                }
            }
        }
        if entry.is_directory {
            new.push_archive_entry::<&[u8]>(copy, None)?;
            written.dirs += 1;
            return Ok(true);
        }
        // The bytes have to come out of the block whatever happens to them, so holding one
        // entry at a time is the floor: the same bound the tar rebuild works under.
        let mut data = Vec::with_capacity(entry.size.min(8 * 1024 * 1024) as usize);
        bytes.read_to_end(&mut data)?;
        written.files += 1;
        written.bytes += data.len() as u64;
        new.push_archive_entry(copy, Some(data.as_slice()))?;
        progress(&name, written.bytes);
        Ok(true)
    });
    if let Some(e) = stopped {
        let _ = std::fs::remove_file(part);
        return Err(e);
    }
    walk.map_err(|e| sevenz(path, e))?;

    for (inner, source) in &plan.put {
        if cancel.is_cancelled() {
            let _ = std::fs::remove_file(part);
            return Err(ArcError::Cancelled);
        }
        let name = inner.trim_start_matches('/');
        let file = File::open(source).map_err(|e| io_error(source, e))?;
        let size = file.metadata().map(|m| m.len()).unwrap_or(0);
        let entry = crate::write::sevenz_entry(source, name, false);
        new.push_archive_entry(entry, Some(BufReader::new(file)))
            .map_err(|e| sevenz(part, e))?;
        written.files += 1;
        written.bytes += size;
        progress(name, written.bytes);
    }
    new.finish().map_err(|e| io_error(part, e))?;
    Ok(written)
}

fn sevenz(path: &Path, e: sevenz_rust2::Error) -> ArcError {
    crate::archive::sevenz_error(path, e)
}

/// Read the new archive back before it replaces the old one.
///
/// It is not a checksum of everything — that would mean decompressing the whole archive a
/// second time — but it is the check that catches the failures this operation really has: an
/// archive that cannot be opened at all, an entry that was asked for and is not there, and one
/// that was removed and still is.
fn verify(part: &Path, plan: &Plan, cancel: &CancelToken) -> Result<()> {
    let archive = Archive::open(part, &Limits::unlimited(), cancel)?;
    for inner in plan.put.keys() {
        if archive.entry(inner).is_none() {
            return Err(ArcError::Broken {
                path: part.display().to_string(),
                reason: format!("{inner} is not in the archive that was just written"),
            });
        }
    }
    for gone in &plan.removed {
        if archive.entry(gone).is_some() {
            return Err(ArcError::Broken {
                path: part.display().to_string(),
                reason: format!("{gone} is still in the archive that was just written"),
            });
        }
    }
    for to in plan.renamed.values() {
        if archive.entry(to).is_none() {
            return Err(ArcError::Broken {
                path: part.display().to_string(),
                reason: format!("{to} is not in the archive that was just written"),
            });
        }
    }
    Ok(())
}

fn pour(source: &Path, to: &mut dyn Write, cancel: &CancelToken) -> Result<u64> {
    let mut file = File::open(source).map_err(|e| io_error(source, e))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let read = file.read(&mut buf).map_err(|e| io_error(source, e))?;
        if read == 0 {
            return Ok(total);
        }
        to.write_all(&buf[..read])
            .map_err(|e| io_error(source, e))?;
        total += read as u64;
    }
}

fn io_error(path: &Path, source: std::io::Error) -> ArcError {
    ArcError::Io {
        path: path.display().to_string(),
        source,
    }
}

fn broken(path: &Path, e: impl std::fmt::Display) -> ArcError {
    ArcError::Broken {
        path: path.display().to_string(),
        reason: e.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::write::create;

    const TREE: [(&str, &[u8]); 3] = [
        ("readme.md", b"# the readme\n"),
        ("src/main.rs", b"fn main() {}\n"),
        ("src/deep/notes.txt", b"notes, two directories down\n"),
    ];

    /// An archive of the usual three files, in the given format, plus the directory the
    /// sources were made in.
    fn archive(dir: &Path, format: Format) -> PathBuf {
        let tree = dir.join("tree");
        for (name, body) in TREE {
            let at = tree.join(name);
            std::fs::create_dir_all(at.parent().unwrap()).unwrap();
            std::fs::write(at, body).unwrap();
        }
        let path = dir.join(format!("tree.{}", format.extension()));
        create(
            &path,
            format,
            std::slice::from_ref(&tree),
            Level::Fast,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .expect("the archive");
        path
    }

    fn names(path: &Path) -> Vec<String> {
        let archive = Archive::open(path, &Limits::default(), &CancelToken::new()).unwrap();
        let mut out: Vec<String> = archive
            .entries()
            .iter()
            .filter(|e| e.kind != fcmd_core::EntryKind::Dir)
            .map(|e| e.path.clone())
            .collect();
        out.sort();
        out
    }

    fn read(path: &Path, inner: &str) -> Vec<u8> {
        Archive::open(path, &Limits::default(), &CancelToken::new())
            .unwrap()
            .read(inner, &CancelToken::new())
            .unwrap()
    }

    /// What the panel's `Ctrl+A` asks for inside an archive: the mode and the time of one
    /// entry, with its bytes and everybody else's untouched.
    #[test]
    fn a_touch_writes_the_mode_and_the_time_and_nothing_else() {
        for format in [Format::Zip, Format::Tar, Format::TarGz, Format::SevenZ] {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            let when = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_722_505_260);
            rebuild(
                &path,
                &[Change::Touch {
                    inner: "/tree/readme.md".into(),
                    mode: Some(0o640),
                    mtime: Some(when),
                }],
                Level::Fast,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));

            let read = Archive::open(&path, &Limits::default(), &CancelToken::new())
                .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            let entry = read
                .entry("/tree/readme.md")
                .unwrap_or_else(|| panic!("{}: the entry is gone", format.label()));
            // The bytes are the ones that went in.
            assert_eq!(
                read.read("/tree/readme.md", &CancelToken::new()).unwrap(),
                b"# the readme\n",
                "{}",
                format.label()
            );
            // A zip and a tar hold a mode; a 7z holds one only where it was written on a
            // Unix, which is what we have just done.
            assert_eq!(
                entry.mode.map(|m| m & 0o777),
                Some(0o640),
                "{}",
                format.label()
            );
            let stamp = entry
                .mtime
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // A zip's clock ticks every two seconds, so the answer is near rather than exact.
            assert!(
                stamp.abs_diff(1_722_505_260) <= 2,
                "{}: {stamp}",
                format.label()
            );
            // And the other entry is where it was.
            assert!(
                read.entry("/tree/src/main.rs").is_some(),
                "{}",
                format.label()
            );
        }
    }

    #[test]
    fn an_entry_is_replaced_in_every_format() {
        for format in Format::ALL {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            let new = dir.path().join("replacement");
            std::fs::write(&new, b"the new bytes\n").unwrap();

            let written = rebuild(
                &path,
                &[Change::Put {
                    inner: "/tree/readme.md".into(),
                    source: new,
                }],
                Level::Fast,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));

            assert_eq!(
                read(&path, "/tree/readme.md"),
                b"the new bytes\n",
                "{}",
                format.label()
            );
            // And nothing else moved.
            assert_eq!(names(&path).len(), 3, "{}", format.label());
            assert_eq!(
                read(&path, "/tree/src/main.rs"),
                TREE[1].1,
                "{}: the other entries did not survive",
                format.label()
            );
            assert!(written.packed > 0, "{}", format.label());
        }
    }

    #[test]
    fn an_entry_and_a_whole_directory_can_be_taken_out() {
        for format in Format::ALL {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            rebuild(
                &path,
                &[Change::Remove {
                    inner: "/tree/src".into(),
                }],
                Level::Fast,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert_eq!(names(&path), ["/tree/readme.md"], "{}", format.label());
        }
    }

    #[test]
    fn an_entry_keeps_its_bytes_under_a_new_name() {
        for format in Format::ALL {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            rebuild(
                &path,
                &[Change::Rename {
                    from: "/tree/readme.md".into(),
                    to: "/tree/README.txt".into(),
                }],
                Level::Fast,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert_eq!(
                read(&path, "/tree/README.txt"),
                TREE[0].1,
                "{}",
                format.label()
            );
            assert!(
                !names(&path).contains(&"/tree/readme.md".to_string()),
                "{}: the old name is still there",
                format.label()
            );
        }
    }

    #[test]
    fn a_file_is_added_where_it_was_asked_for() {
        for format in Format::ALL {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            let new = dir.path().join("extra.txt");
            std::fs::write(&new, b"added\n").unwrap();
            rebuild(
                &path,
                &[Change::add(&new, "/tree/src")],
                Level::Fast,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert_eq!(
                read(&path, "/tree/src/extra.txt"),
                b"added\n",
                "{}",
                format.label()
            );
            assert_eq!(names(&path).len(), 4, "{}", format.label());
        }
    }

    /// The point of writing beside and renaming: a rebuild that does not finish must leave the
    /// archive exactly as it was.
    #[test]
    fn a_cancelled_rebuild_leaves_the_archive_alone() {
        for format in Format::ALL {
            let dir = tempfile::tempdir().unwrap();
            let path = archive(dir.path(), format);
            let before = std::fs::read(&path).unwrap();

            let cancel = CancelToken::new();
            cancel.cancel();
            let err = rebuild(
                &path,
                &[Change::Remove {
                    inner: "/tree/readme.md".into(),
                }],
                Level::Fast,
                &cancel,
                &mut |_, _| {},
            )
            .unwrap_err();
            assert!(
                matches!(err, ArcError::Cancelled),
                "{}: {err}",
                format.label()
            );
            assert_eq!(
                std::fs::read(&path).unwrap(),
                before,
                "{}: the archive was changed by a cancelled rebuild",
                format.label()
            );
            // And no litter beside it.
            let left: Vec<String> = std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n.contains(".part"))
                .collect();
            assert!(left.is_empty(), "{}: {left:?}", format.label());
        }
    }

    #[test]
    fn asking_for_nothing_does_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(dir.path(), Format::Zip);
        let before = std::fs::read(&path).unwrap();
        let written =
            rebuild(&path, &[], Level::Fast, &CancelToken::new(), &mut |_, _| {}).unwrap();
        assert_eq!(written, Written::default());
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }

    /// Several changes at once, which is what a panel full of marked rows asks for.
    #[test]
    fn a_rebuild_takes_every_change_in_one_pass() {
        let dir = tempfile::tempdir().unwrap();
        let path = archive(dir.path(), Format::TarGz);
        let new = dir.path().join("extra.txt");
        std::fs::write(&new, b"added\n").unwrap();
        let replacement = dir.path().join("replacement");
        std::fs::write(&replacement, b"replaced\n").unwrap();

        rebuild(
            &path,
            &[
                Change::Remove {
                    inner: "/tree/src/deep".into(),
                },
                Change::Rename {
                    from: "/tree/src/main.rs".into(),
                    to: "/tree/src/lib.rs".into(),
                },
                Change::Put {
                    inner: "/tree/readme.md".into(),
                    source: replacement,
                },
                Change::add(&new, "/tree"),
            ],
            Level::Fast,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap();

        assert_eq!(
            names(&path),
            ["/tree/extra.txt", "/tree/readme.md", "/tree/src/lib.rs"]
        );
        assert_eq!(read(&path, "/tree/readme.md"), b"replaced\n");
        assert_eq!(read(&path, "/tree/src/lib.rs"), TREE[1].1);
    }

    /// The verification is what stands between a bad rebuild and the original: an archive
    /// whose changes did not take must not replace anything.
    #[test]
    fn a_rebuild_that_did_not_take_does_not_replace_the_original() {
        let dir = tempfile::tempdir().unwrap();
        let mut plan = Plan::default();
        plan.put.insert("/tree/absent.txt".into(), PathBuf::new());
        let part = dir.path().join("empty.zip");
        // An archive with nothing in it, standing in for a rebuild that wrote nothing.
        create(
            &part,
            Format::Zip,
            &[],
            Level::Fast,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap();
        let err = verify(&part, &plan, &CancelToken::new()).unwrap_err();
        assert!(err.to_string().contains("absent.txt"), "{err}");
    }
    #[test]
    fn a_solid_7z_survives_having_one_entry_taken_out() {
        // A 7-Zip archive is solid by default: several files in one compressed stream. The
        // rebuild has to read past the entry it is dropping rather than ignoring it, or the
        // stream stays in the middle of that file and the next one comes out as rubble.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("solid.7z");
        let file = File::create(&path).unwrap();
        let mut writer = sevenz_rust2::ArchiveWriter::new(file).unwrap();
        let bodies: Vec<&[u8]> = TREE.iter().map(|(_, body)| *body).collect();
        let entries = TREE
            .iter()
            .map(|(name, _)| {
                let mut entry = sevenz_rust2::ArchiveEntry::new();
                entry.name = (*name).to_string();
                entry.has_stream = true;
                entry
            })
            .collect();
        writer
            .push_archive_entries(
                entries,
                bodies.iter().map(|b| (*b).into()).collect::<Vec<_>>(),
            )
            .unwrap();
        writer.finish().unwrap();

        rebuild(
            &path,
            &[Change::Remove {
                inner: "/readme.md".into(),
            }],
            Level::Fast,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .expect("the rebuild");

        assert_eq!(names(&path), ["/src/deep/notes.txt", "/src/main.rs"]);
        assert_eq!(read(&path, "/src/main.rs"), TREE[1].1);
        assert_eq!(read(&path, "/src/deep/notes.txt"), TREE[2].1);
    }
}
