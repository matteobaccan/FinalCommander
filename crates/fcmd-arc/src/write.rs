use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use fcmd_core::CancelToken;

use crate::{ArcError, Format, Result};

/// What a new archive gets: how far the compressor should work for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Level {
    /// As fast as the format allows — for a one-off archive of something already compressed.
    Fast,
    #[default]
    Normal,
    /// As small as the format allows, which on a large tree is noticeably slower.
    Best,
}

/// What was written, for the line the job reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Written {
    pub files: u64,
    pub dirs: u64,
    /// Bytes read from the sources, which is what the progress bar counts.
    pub bytes: u64,
    /// The size of the archive itself.
    pub packed: u64,
}

/// Build an archive at `path` out of `sources`, which are real paths on disk.
///
/// Each source keeps its own name inside the archive, and a directory brings its tree with it:
/// packing `/home/mb/src` and `/home/mb/notes.txt` gives an archive of `src/…` and
/// `notes.txt`, which is what a user who selected two rows and pressed the key meant.
///
/// The archive is written to a sibling temporary file and renamed into place at the end, so a
/// cancelled or failed pack leaves no half-built archive where the user will later trust one.
///
/// `solid` means what 7-Zip means by it, and only a 7z can honour it: the files share one
/// compressed stream, which is smaller and makes reading one of them decode everything in
/// front of it. A tar is solid whatever anybody asks, a zip cannot be, and both ignore it.
pub fn create(
    path: &Path,
    format: Format,
    sources: &[PathBuf],
    level: Level,
    solid: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let name = path.display().to_string();
    // A format this crate only reads is refused here, by name, rather than further in where
    // the only honest thing left would be a panic.
    if !format.writable() {
        return Err(ArcError::Refused {
            path: name,
            reason: format!("{} is read here, not written", format.label()),
        });
    }
    let part = path.with_extension(format!("{}.part", format.extension()));
    let result = write_all(&part, format, sources, level, solid, cancel, progress);
    match result {
        Ok(mut written) => {
            written.packed = std::fs::metadata(&part).map(|m| m.len()).unwrap_or(0);
            std::fs::rename(&part, path).map_err(|e| ArcError::Io {
                path: name,
                source: e,
            })?;
            Ok(written)
        }
        Err(e) => {
            let _ = std::fs::remove_file(&part);
            Err(e)
        }
    }
}

fn write_all(
    part: &Path,
    format: Format,
    sources: &[PathBuf],
    level: Level,
    solid: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let items = collect(sources, cancel)?;
    if format.is_single() {
        // One file in, one file out: gzip's own contract. A folder or a second file has
        // nowhere to go, and the format that holds them is the tar of the same codec.
        let [item] = items.as_slice() else {
            return Err(one_file_only(part, format));
        };
        if item.is_dir {
            return Err(one_file_only(part, format));
        }
        let mtime = std::fs::metadata(&item.source)
            .and_then(|m| m.modified())
            .ok();
        return write_single(
            part,
            format,
            &item.source,
            &item.inner,
            mtime,
            level,
            cancel,
            progress,
        );
    }
    if format == Format::Zip {
        return write_zip(part, &items, level, cancel, progress);
    }
    if format == Format::SevenZ {
        return write_sevenz(part, &items, level, solid, cancel, progress);
    }
    write_tar(part, format, &items, level, cancel, progress)
}

/// Why a single stream refuses a folder or a second file, naming the format that takes them.
pub(crate) fn one_file_only(path: &Path, format: Format) -> ArcError {
    let tar = format.with_tar_inside().map(Format::label).unwrap_or("tar");
    ArcError::Refused {
        path: path.display().to_string(),
        reason: format!(
            "a {} holds one file; a folder or several files go in a {tar}",
            format.label()
        ),
    }
}

/// One file compressed into one stream: what `gzip notes.txt` leaves behind.
///
/// gzip's header has room for the name the file had and the time it was last written, and
/// both are filled in: it is what `gzip` itself does, and it is what lets the entry keep its
/// name when the `.gz` is renamed. The other three keep nothing of the kind, and the reader
/// names the entry by taking the suffix off.
#[allow(clippy::too_many_arguments)]
pub(crate) fn write_single(
    part: &Path,
    format: Format,
    source: &Path,
    name: &str,
    mtime: Option<std::time::SystemTime>,
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let file = BufWriter::new(File::create(part).map_err(|e| io_error(part, e))?);
    let leaf = name.rsplit('/').next().unwrap_or(name);
    let bytes = match format {
        Format::Gz => {
            let mut header = flate2::GzBuilder::new().filename(leaf.as_bytes());
            if let Some(secs) = mtime
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .and_then(|d| u32::try_from(d.as_secs()).ok())
            {
                header = header.mtime(secs);
            }
            let mut enc = header.write(
                file,
                match level {
                    Level::Fast => flate2::Compression::fast(),
                    Level::Normal => flate2::Compression::default(),
                    Level::Best => flate2::Compression::best(),
                },
            );
            let bytes = pour_watched(source, &mut enc, cancel, progress)?;
            enc.finish().map_err(|e| io_error(part, e))?;
            bytes
        }
        Format::Bz2 | Format::Xz => {
            let mut sink = Sink::new(part, format.with_tar_inside().expect("a codec"), level)?;
            let bytes = pour_watched(source, &mut sink, cancel, progress)?;
            sink.finish(part)?;
            bytes
        }
        Format::Zst => {
            // zstd's encoder here pulls from a reader and unwraps what the reader says, so a
            // cancelled or failed read cannot be an error without taking the program down. It
            // is an end of input instead, remembered, and looked at once the encoder is done.
            let input = File::open(source).map_err(|e| io_error(source, e))?;
            let mut watched = Watched {
                inner: BufReader::new(input),
                cancel,
                progress,
                source,
                total: 0,
                stopped: None,
            };
            ruzstd::encoding::compress(&mut watched, file, zstd_level(level));
            match watched.stopped.take() {
                Some(Stopped::Cancelled) => return Err(ArcError::Cancelled),
                Some(Stopped::Failed(e)) => return Err(io_error(source, e)),
                None => {}
            }
            watched.total
        }
        other => unreachable!("{} is not a single stream", other.label()),
    };
    Ok(Written {
        files: 1,
        dirs: 0,
        bytes,
        packed: 0,
    })
}

/// The zstd level a [`Level`] means — which is one level whatever is asked.
///
/// The encoder here implements `Fastest` and `Uncompressed` and nothing else: `Default`,
/// `Better` and `Best` are `unimplemented!()` inside it. Until 0.30.0 `Normal` and `Best` were
/// passed through, and a release build aborts on a panic, so packing a `.tar.zst` at either
/// level ended the program. Every level is `Fastest` now, which is still zstd and still
/// smaller than the tar, and the pack dialog's level is for the other codecs.
pub(crate) fn zstd_level(_level: Level) -> ruzstd::encoding::CompressionLevel {
    ruzstd::encoding::CompressionLevel::Fastest
}

/// One file into an encoder, a block at a time, reporting each block and stopping at a
/// cancellation — so a large file shows its progress rather than arriving all at once.
fn pour_watched(
    source: &Path,
    to: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<u64> {
    let input = File::open(source).map_err(|e| io_error(source, e))?;
    let mut watched = Watched {
        inner: BufReader::new(input),
        cancel,
        progress,
        source,
        total: 0,
        stopped: None,
    };
    io::copy(&mut watched, to).map_err(|e| ArcError::Io {
        path: "the archive".to_string(),
        source: e,
    })?;
    match watched.stopped.take() {
        Some(Stopped::Cancelled) => Err(ArcError::Cancelled),
        Some(Stopped::Failed(e)) => Err(io_error(source, e)),
        None => Ok(watched.total),
    }
}

/// Why a [`Watched`] read stopped early.
enum Stopped {
    Cancelled,
    Failed(io::Error),
}

/// A reader that reports progress, and turns a cancellation or a failure into an end of
/// input it remembers — for an encoder that cannot be handed an error.
struct Watched<'a, R> {
    inner: R,
    cancel: &'a CancelToken,
    progress: &'a mut dyn FnMut(&Path, u64),
    source: &'a Path,
    total: u64,
    stopped: Option<Stopped>,
}

impl<R: Read> Read for Watched<'_, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.stopped.is_some() {
            return Ok(0);
        }
        if self.cancel.is_cancelled() {
            self.stopped = Some(Stopped::Cancelled);
            return Ok(0);
        }
        match self.inner.read(buf) {
            Ok(read) => {
                self.total += read as u64;
                (self.progress)(self.source, self.total);
                Ok(read)
            }
            Err(e) => {
                self.stopped = Some(Stopped::Failed(e));
                Ok(0)
            }
        }
    }
}

/// One thing to put in the archive: where it is now, and the name it takes inside.
#[derive(Debug, Clone)]
struct Item {
    source: PathBuf,
    inner: String,
    is_dir: bool,
    size: u64,
}

/// Walk the sources, depth first, with each source's own name as the top of its tree.
///
/// A symlink is followed and packed as the file it points at: a zip of a link to somewhere
/// that will not exist on the machine the archive is unpacked on is a trap, and the formats
/// that can hold a link cannot all hold one safely.
fn collect(sources: &[PathBuf], cancel: &CancelToken) -> Result<Vec<Item>> {
    let mut out = Vec::new();
    for source in sources {
        let base = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "file".to_string());
        walk(source, &base, &mut out, cancel)?;
    }
    Ok(out)
}

fn walk(path: &Path, inner: &str, out: &mut Vec<Item>, cancel: &CancelToken) -> Result<()> {
    if cancel.is_cancelled() {
        return Err(ArcError::Cancelled);
    }
    let meta = std::fs::metadata(path).map_err(|e| ArcError::Io {
        path: path.display().to_string(),
        source: e,
    })?;
    if meta.is_dir() {
        out.push(Item {
            source: path.to_path_buf(),
            inner: inner.to_string(),
            is_dir: true,
            size: 0,
        });
        let read = std::fs::read_dir(path).map_err(|e| ArcError::Io {
            path: path.display().to_string(),
            source: e,
        })?;
        let mut children: Vec<_> = read.filter_map(|e| e.ok()).collect();
        children.sort_by_key(|e| e.file_name());
        for child in children {
            let name = child.file_name().to_string_lossy().into_owned();
            walk(&child.path(), &format!("{inner}/{name}"), out, cancel)?;
        }
        return Ok(());
    }
    out.push(Item {
        source: path.to_path_buf(),
        inner: inner.to_string(),
        is_dir: false,
        size: meta.len(),
    });
    Ok(())
}

fn write_zip(
    part: &Path,
    items: &[Item],
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let file = File::create(part).map_err(|e| io_error(part, e))?;
    let mut zip = zip::ZipWriter::new(BufWriter::new(file));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .compression_level(Some(match level {
            Level::Fast => 1,
            Level::Normal => 6,
            Level::Best => 9,
        }))
        // Not every file fits in 4 GB, and finding that out at the end would waste the work.
        .large_file(items.iter().any(|i| i.size >= u32::MAX as u64));
    let mut written = Written::default();
    for item in items {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        if item.is_dir {
            zip.add_directory(&item.inner, options)
                .map_err(|e| broken(part, e))?;
            written.dirs += 1;
            continue;
        }
        zip.start_file(&item.inner, options)
            .map_err(|e| broken(part, e))?;
        written.bytes += pour(&item.source, &mut zip, cancel, progress)?;
        written.files += 1;
    }
    zip.finish().map_err(|e| broken(part, e))?;
    Ok(written)
}

fn write_tar(
    part: &Path,
    format: Format,
    items: &[Item],
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    // zstd is the one compressor here with no streaming writer of its own, so that format
    // builds the tar beside the archive and compresses it in a second pass.
    let plain = if format == Format::TarZst {
        part.with_extension("tar.part")
    } else {
        part.to_path_buf()
    };
    let sink = Sink::new(&plain, format, level)?;
    // Links are followed, as they always were here: what goes in is the file a link points
    // to, which is what the person packing a folder of links almost always means.
    let mut tar = crate::tar::Builder::new(sink);
    let mut written = Written::default();
    for item in items {
        if cancel.is_cancelled() {
            let _ = std::fs::remove_file(&plain);
            return Err(ArcError::Cancelled);
        }
        if item.is_dir {
            tar.append_dir(&item.inner, &item.source)
                .map_err(|e| io_error(&item.source, e))?;
            written.dirs += 1;
            continue;
        }
        let size = tar
            .append_file(&item.inner, &item.source)
            .map_err(|e| io_error(&item.source, e))?;
        written.bytes += size;
        written.files += 1;
        progress(&item.source, written.bytes);
    }
    let sink = tar.finish().map_err(|e| broken(part, e))?;
    sink.finish(part)?;
    if format == Format::TarZst {
        let source = File::open(&plain).map_err(|e| io_error(&plain, e))?;
        let target = File::create(part).map_err(|e| io_error(part, e))?;
        ruzstd::encoding::compress(
            BufReader::new(source),
            BufWriter::new(target),
            zstd_level(level),
        );
        let _ = std::fs::remove_file(&plain);
    }
    Ok(written)
}

/// The compressor a tar is written through, as one `Write`.
pub(crate) enum Sink {
    Plain(BufWriter<File>),
    Gz(flate2::write::GzEncoder<BufWriter<File>>),
    Bz(bzip2::write::BzEncoder<BufWriter<File>>),
    Xz(lzma_rust2::XzWriter<BufWriter<File>>),
}

impl Sink {
    pub(crate) fn new(path: &Path, format: Format, level: Level) -> Result<Sink> {
        let file = BufWriter::new(File::create(path).map_err(|e| io_error(path, e))?);
        Ok(match format {
            // A `.tar.zst` is built as a plain tar first and compressed afterwards.
            Format::Tar | Format::TarZst => Sink::Plain(file),
            Format::TarGz => Sink::Gz(flate2::write::GzEncoder::new(
                file,
                match level {
                    Level::Fast => flate2::Compression::fast(),
                    Level::Normal => flate2::Compression::default(),
                    Level::Best => flate2::Compression::best(),
                },
            )),
            Format::TarBz2 => Sink::Bz(bzip2::write::BzEncoder::new(
                file,
                match level {
                    Level::Fast => bzip2::Compression::fast(),
                    Level::Normal => bzip2::Compression::default(),
                    Level::Best => bzip2::Compression::best(),
                },
            )),
            Format::TarXz => Sink::Xz(
                lzma_rust2::XzWriter::new(
                    file,
                    lzma_rust2::XzOptions::with_preset(match level {
                        Level::Fast => 1,
                        Level::Normal => 6,
                        Level::Best => 9,
                    }),
                )
                .map_err(|e| broken(path, e))?,
            ),
            Format::Zip | Format::SevenZ | Format::Ar | Format::Cpio | Format::Rpm => {
                unreachable!("a {} is not written through a tar", format.label())
            }
            // A single compressed stream shares these codecs and is not written here: it has
            // no tar to wrap, so it asks for the sink of the codec it shares.
            f if f.is_single() => {
                unreachable!("a {} holds one file and no tar", f.label())
            }
            _ => unreachable!("a {} is not written through a tar", format.label()),
        })
    }

    /// Close the compressor, which is what writes its trailer.
    pub(crate) fn finish(self, path: &Path) -> Result<()> {
        match self {
            Sink::Plain(mut w) => w.flush().map_err(|e| io_error(path, e)),
            Sink::Gz(w) => w.finish().map(|_| ()).map_err(|e| io_error(path, e)),
            Sink::Bz(w) => w.finish().map(|_| ()).map_err(|e| io_error(path, e)),
            Sink::Xz(w) => w.finish().map(|_| ()).map_err(|e| io_error(path, e)),
        }
    }
}

impl Write for Sink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self {
            Sink::Plain(w) => w.write(buf),
            Sink::Gz(w) => w.write(buf),
            Sink::Bz(w) => w.write(buf),
            Sink::Xz(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self {
            Sink::Plain(w) => w.flush(),
            Sink::Gz(w) => w.flush(),
            Sink::Bz(w) => w.flush(),
            Sink::Xz(w) => w.flush(),
        }
    }
}

/// Copy one file into the archive, reporting what has gone in so far.
fn pour(
    source: &Path,
    to: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<u64> {
    let mut file = File::open(source).map_err(|e| io_error(source, e))?;
    let mut buf = vec![0u8; 64 * 1024];
    let mut total = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let read = file.read(&mut buf).map_err(|e| io_error(source, e))?;
        if read == 0 {
            progress(source, total);
            return Ok(total);
        }
        to.write_all(&buf[..read]).map_err(|e| ArcError::Io {
            path: "the archive".to_string(),
            source: e,
        })?;
        total += read as u64;
    }
}

fn io_error(path: &Path, source: io::Error) -> ArcError {
    ArcError::Io {
        path: path.display().to_string(),
        source,
    }
}

fn broken(path: &Path, reason: impl std::fmt::Display) -> ArcError {
    ArcError::Broken {
        path: path.display().to_string(),
        reason: reason.to_string(),
    }
}

/// What LZMA2 setting a level means, and the one encoder every 7z we write is made with.
///
/// LZMA2 is 7-Zip's own default and the method every tool that reads a 7z supports, so an
/// archive written here opens in 7-Zip, in p7zip and in the file manager of whoever is handed
/// it. The dictionary sizes are the levels `7z -mx1`, `-mx5` and `-mx9` use.
pub(crate) fn sevenz_method(level: Level) -> sevenz_rust2::EncoderConfiguration {
    let preset = match level {
        Level::Fast => 1,
        Level::Normal => 6,
        Level::Best => 9,
    };
    sevenz_rust2::EncoderConfiguration::new(sevenz_rust2::EncoderMethod::LZMA2).with_options(
        sevenz_rust2::encoder_options::EncoderOptions::Lzma2(
            sevenz_rust2::encoder_options::Lzma2Options::from_level(preset),
        ),
    )
}

/// The entry a path takes inside a 7z: the name it is given, and the times and attributes the
/// format can hold for it.
pub(crate) fn sevenz_entry(source: &Path, inner: &str, is_dir: bool) -> sevenz_rust2::ArchiveEntry {
    let mut entry = sevenz_rust2::ArchiveEntry::new();
    entry.name = inner.to_string();
    entry.is_directory = is_dir;
    if let Ok(meta) = std::fs::metadata(source) {
        if let Ok(mtime) = meta.modified() {
            if let Ok(stamp) = sevenz_rust2::NtTime::try_from(mtime) {
                entry.last_modified_date = stamp;
                entry.has_last_modified_date = true;
            }
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            // Bit 15 set says "the Unix mode is in the top half", which is how p7zip and
            // 7-Zip both carry a mode through a 7z and the only way one survives at all.
            entry.windows_attributes = (meta.mode() << 16) | 0x8000;
            entry.has_windows_attributes = true;
        }
    }
    entry
}

/// How much goes into one solid block: enough for the saving to show, little enough that one
/// pack does not hold ten thousand file handles open or make reading one entry decode a
/// gigabyte. 7-Zip has the same two dials under *Solid Block size*.
const SOLID_FILES: usize = 64;
const SOLID_BYTES: u64 = 128 << 20;

fn write_sevenz(
    part: &Path,
    items: &[Item],
    level: Level,
    solid: bool,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let file = File::create(part).map_err(|e| io_error(part, e))?;
    let mut writer =
        sevenz_rust2::ArchiveWriter::new(BufWriter::new(file)).map_err(|e| sevenz(part, e))?;
    writer.set_content_methods(vec![sevenz_method(level)]);
    if solid {
        let written = write_sevenz_solid(part, items, &mut writer, cancel, progress)?;
        writer.finish().map_err(|e| io_error(part, e))?;
        return Ok(written);
    }
    let mut written = Written::default();
    for item in items {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        let entry = sevenz_entry(&item.source, &item.inner, item.is_dir);
        if item.is_dir {
            writer
                .push_archive_entry::<File>(entry, None)
                .map_err(|e| sevenz(part, e))?;
            written.dirs += 1;
            continue;
        }
        let source = File::open(&item.source).map_err(|e| io_error(&item.source, e))?;
        writer
            .push_archive_entry(entry, Some(BufReader::new(source)))
            .map_err(|e| sevenz(part, e))?;
        written.files += 1;
        written.bytes += item.size;
        progress(&item.source, written.bytes);
    }
    writer.finish().map_err(|e| io_error(part, e))?;
    Ok(written)
}

/// The same archive, with the files packed together rather than one by one.
///
/// Together in blocks, not all in one: a block is the unit that has to be decoded to read any
/// file in it, so one block for a whole tree would make opening the first file read the lot.
/// The directories go in on their own, because a directory has no bytes and the writer wants
/// one reader per entry.
fn write_sevenz_solid<W: Write + std::io::Seek>(
    part: &Path,
    items: &[Item],
    writer: &mut sevenz_rust2::ArchiveWriter<W>,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let mut written = Written::default();
    let mut block: Vec<&Item> = Vec::new();
    let mut block_bytes = 0u64;

    let flush = |block: &mut Vec<&Item>,
                 block_bytes: &mut u64,
                 written: &mut Written,
                 writer: &mut sevenz_rust2::ArchiveWriter<W>,
                 progress: &mut dyn FnMut(&Path, u64)|
     -> Result<()> {
        if block.is_empty() {
            return Ok(());
        }
        let mut entries = Vec::with_capacity(block.len());
        let mut readers = Vec::with_capacity(block.len());
        for item in block.iter() {
            let mut entry = sevenz_entry(&item.source, &item.inner, false);
            // The writer fills in the size and the CRC after it has read each one, but the
            // flag that says there are bytes at all is the caller's to set: without it the
            // entry lands in the archive as a name with nothing behind it.
            entry.has_stream = true;
            entries.push(entry);
            let file = File::open(&item.source).map_err(|e| io_error(&item.source, e))?;
            readers.push(sevenz_rust2::SourceReader::new(BufReader::new(file)));
        }
        writer
            .push_archive_entries(entries, readers)
            .map_err(|e| sevenz(part, e))?;
        for item in block.iter() {
            written.files += 1;
            written.bytes += item.size;
            progress(&item.source, written.bytes);
        }
        block.clear();
        *block_bytes = 0;
        Ok(())
    };

    for item in items {
        if cancel.is_cancelled() {
            return Err(ArcError::Cancelled);
        }
        if item.is_dir {
            let entry = sevenz_entry(&item.source, &item.inner, true);
            writer
                .push_archive_entry::<File>(entry, None)
                .map_err(|e| sevenz(part, e))?;
            written.dirs += 1;
            continue;
        }
        block.push(item);
        block_bytes += item.size;
        if block.len() >= SOLID_FILES || block_bytes >= SOLID_BYTES {
            flush(&mut block, &mut block_bytes, &mut written, writer, progress)?;
        }
    }
    flush(&mut block, &mut block_bytes, &mut written, writer, progress)?;
    Ok(written)
}

fn sevenz(path: &Path, e: sevenz_rust2::Error) -> ArcError {
    crate::archive::sevenz_error(path, e)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archive, Limits};

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/deep")).unwrap();
        std::fs::write(dir.path().join("src/main.rs"), b"fn main() {}\n").unwrap();
        std::fs::write(dir.path().join("src/deep/notes.txt"), b"two down\n").unwrap();
        std::fs::write(dir.path().join("readme.md"), b"# readme\n").unwrap();
        dir
    }

    const SINGLES: [Format; 4] = [Format::Gz, Format::Bz2, Format::Xz, Format::Zst];

    fn quiet(_: &Path, _: u64) {}

    #[test]
    fn one_file_goes_in_and_comes_out_again_in_each_single_stream() {
        let dir = tree();
        let source = dir.path().join("readme.md");
        for format in SINGLES {
            let path = dir.path().join(format!("readme.md.{}", format.extension()));
            let written = create(
                &path,
                format,
                std::slice::from_ref(&source),
                Level::Normal,
                false,
                &CancelToken::new(),
                &mut quiet,
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert_eq!((written.files, written.bytes), (1, 9), "{}", format.label());
            let archive = Archive::open(&path, &Limits::default(), &CancelToken::new()).unwrap();
            assert_eq!(archive.format(), Some(format));
            assert_eq!(archive.entries().len(), 1, "{}", format.label());
            assert_eq!(
                archive.entries()[0].path,
                "/readme.md",
                "{}",
                format.label()
            );
            assert_eq!(
                archive.read("/readme.md", &CancelToken::new()).unwrap(),
                b"# readme\n",
                "{}",
                format.label()
            );
        }
    }

    /// gzip keeps the name in its header, as `gzip` itself does, so the entry is still called
    /// what the file was called after the `.gz` has been renamed.
    #[test]
    fn a_gz_keeps_the_name_of_the_file_it_holds() {
        let dir = tree();
        let path = dir.path().join("packed.gz");
        create(
            &path,
            Format::Gz,
            &[dir.path().join("src/main.rs")],
            Level::Fast,
            false,
            &CancelToken::new(),
            &mut quiet,
        )
        .unwrap();
        let archive = Archive::open(&path, &Limits::default(), &CancelToken::new()).unwrap();
        assert_eq!(archive.entries()[0].path, "/main.rs");
        assert!(archive.entries()[0].mtime.is_some());
    }

    #[test]
    fn a_folder_or_a_second_file_is_refused_naming_the_tar_that_takes_them() {
        let dir = tree();
        for format in SINGLES {
            for sources in [
                vec![dir.path().join("src")],
                vec![dir.path().join("readme.md"), dir.path().join("src/main.rs")],
            ] {
                let path = dir.path().join(format!("out.{}", format.extension()));
                let e = create(
                    &path,
                    format,
                    &sources,
                    Level::Fast,
                    false,
                    &CancelToken::new(),
                    &mut quiet,
                )
                .unwrap_err();
                let tar = format.with_tar_inside().unwrap().label();
                assert!(e.to_string().contains(tar), "{}: {e}", format.label());
                assert!(!path.exists(), "{}", format.label());
                let leftovers: Vec<_> = std::fs::read_dir(dir.path())
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_name().to_string_lossy().ends_with(".part"))
                    .collect();
                assert!(leftovers.is_empty(), "{}", format.label());
            }
        }
    }

    /// zstd's encoder unwraps what its reader says, so a cancellation has to reach it as an
    /// end of input. In a build that aborts on a panic, the other way would end the program.
    #[test]
    fn a_cancelled_single_stream_leaves_nothing_and_does_not_panic() {
        let dir = tree();
        let big = dir.path().join("big.bin");
        std::fs::write(&big, vec![7u8; 4 * 1024 * 1024]).unwrap();
        for format in SINGLES {
            let path = dir.path().join(format!("big.bin.{}", format.extension()));
            let cancel = CancelToken::new();
            let mut seen = 0u64;
            let e = create(
                &path,
                format,
                std::slice::from_ref(&big),
                Level::Fast,
                false,
                &cancel,
                &mut |_, bytes| {
                    seen = bytes;
                    cancel.cancel();
                },
            )
            .unwrap_err();
            assert!(matches!(e, ArcError::Cancelled), "{}: {e}", format.label());
            assert!(seen < 4 * 1024 * 1024, "{}", format.label());
            assert!(!path.exists(), "{}", format.label());
        }
    }

    /// Solid means the files share a compressed stream, which is smaller; what matters here
    /// is that everything still comes out, because a block read wrongly gives rubble rather
    /// than an error.

    #[test]
    fn a_solid_7z_holds_what_a_plain_one_does() {
        let dir = tree();
        let sources = vec![dir.path().join("src"), dir.path().join("readme.md")];
        let mut sizes = Vec::new();
        for (name, solid) in [("plain.7z", false), ("solid.7z", true)] {
            let archive = dir.path().join(name);
            let written = create(
                &archive,
                Format::SevenZ,
                &sources,
                Level::Best,
                solid,
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .expect("the archive was written");
            assert_eq!((written.files, written.dirs), (3, 2), "{name}");

            let read = Archive::open(&archive, &Limits::default(), &CancelToken::new())
                .expect("it reads back");
            assert_eq!(
                read.read("/src/deep/notes.txt", &CancelToken::new())
                    .expect("the entry comes out"),
                b"two down\n",
                "{name}"
            );
            assert_eq!(
                read.read("/readme.md", &CancelToken::new()).unwrap(),
                b"# readme\n",
                "{name}"
            );
            sizes.push(written.packed);
        }
        assert!(sizes.iter().all(|&n| n > 0), "{sizes:?}");
    }

    #[test]
    fn what_goes_in_comes_out_again_in_every_format() {
        let dir = tree();
        for format in Format::writable_formats()
            .into_iter()
            .filter(|f| f.holds_many())
        {
            let archive = dir.path().join(format!("out.{}", format.extension()));
            let sources = vec![dir.path().join("src"), dir.path().join("readme.md")];
            let mut seen = Vec::new();
            let written = create(
                &archive,
                format,
                &sources,
                Level::Fast,
                false,
                &CancelToken::new(),
                &mut |path, _| seen.push(path.to_path_buf()),
            )
            .unwrap_or_else(|e| panic!("{}: {e}", format.label()));

            assert_eq!((written.files, written.dirs), (3, 2), "{}", format.label());
            assert_eq!(written.bytes, 31, "{}", format.label());
            assert!(written.packed > 0, "{}", format.label());
            assert!(!seen.is_empty(), "{}", format.label());
            // Nothing is left beside it: the half-built archive and the plain tar are gone.
            let leftovers: Vec<_> = std::fs::read_dir(dir.path())
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().contains(".part"))
                .collect();
            assert!(leftovers.is_empty(), "{}", format.label());

            // And the archive reads back as the tree that went in.
            let read = Archive::open(&archive, &Limits::default(), &CancelToken::new())
                .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert_eq!(read.format(), Some(format));
            let names: Vec<String> = read
                .list("/")
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect();
            assert_eq!(names, ["readme.md", "src"], "{}", format.label());
            assert_eq!(
                read.read("/src/deep/notes.txt", &CancelToken::new())
                    .unwrap(),
                b"two down\n",
                "{}",
                format.label()
            );
            assert_eq!(read.totals().0, 3, "{}", format.label());
        }
    }

    #[test]
    fn a_cancelled_pack_leaves_nothing_behind() {
        let dir = tree();
        let archive = dir.path().join("out.zip");
        let cancel = CancelToken::new();
        cancel.cancel();
        let error = create(
            &archive,
            Format::Zip,
            &[dir.path().join("src")],
            Level::Normal,
            false,
            &cancel,
            &mut |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(error, ArcError::Cancelled), "{error}");
        assert!(!archive.exists(), "a half-built archive was left in place");
        assert!(!dir.path().join("out.zip.part").exists());
    }

    #[test]
    fn a_source_that_is_not_there_is_reported_and_nothing_is_written() {
        let dir = tree();
        let archive = dir.path().join("out.tar.gz");
        let error = create(
            &archive,
            Format::TarGz,
            &[dir.path().join("nothing")],
            Level::Normal,
            false,
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(error, ArcError::Io { .. }), "{error}");
        assert!(!archive.exists());
    }
}
