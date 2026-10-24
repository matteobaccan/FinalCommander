use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use scmd_core::CancelToken;

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
pub fn create(
    path: &Path,
    format: Format,
    sources: &[PathBuf],
    level: Level,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let name = path.display().to_string();
    let part = path.with_extension(format!("{}.part", format.extension()));
    let result = write_all(&part, format, sources, level, cancel, progress);
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
    cancel: &CancelToken,
    progress: &mut dyn FnMut(&Path, u64),
) -> Result<Written> {
    let items = collect(sources, cancel)?;
    if format == Format::Zip {
        return write_zip(part, &items, level, cancel, progress);
    }
    write_tar(part, format, &items, level, cancel, progress)
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
    let mut tar = tar::Builder::new(sink);
    tar.follow_symlinks(true);
    let mut written = Written::default();
    for item in items {
        if cancel.is_cancelled() {
            let _ = std::fs::remove_file(&plain);
            return Err(ArcError::Cancelled);
        }
        if item.is_dir {
            tar.append_dir(&item.inner, &item.source)
                .map_err(|e| broken(part, e))?;
            written.dirs += 1;
            continue;
        }
        let mut file = File::open(&item.source).map_err(|e| io_error(&item.source, e))?;
        tar.append_file(&item.inner, &mut file)
            .map_err(|e| broken(part, e))?;
        written.bytes += item.size;
        written.files += 1;
        progress(&item.source, written.bytes);
    }
    let sink = tar.into_inner().map_err(|e| broken(part, e))?;
    sink.finish(part)?;
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

/// The compressor a tar is written through, as one `Write`.
enum Sink {
    Plain(BufWriter<File>),
    Gz(flate2::write::GzEncoder<BufWriter<File>>),
    Bz(bzip2::write::BzEncoder<BufWriter<File>>),
    Xz(lzma_rust2::XzWriter<BufWriter<File>>),
}

impl Sink {
    fn new(path: &Path, format: Format, level: Level) -> Result<Sink> {
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
            Format::Zip => unreachable!("a zip is not written through a tar"),
        })
    }

    /// Close the compressor, which is what writes its trailer.
    fn finish(self, path: &Path) -> Result<()> {
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

    #[test]
    fn what_goes_in_comes_out_again_in_every_format() {
        let dir = tree();
        for format in Format::ALL {
            let archive = dir.path().join(format!("out.{}", format.extension()));
            let sources = vec![dir.path().join("src"), dir.path().join("readme.md")];
            let mut seen = Vec::new();
            let written = create(
                &archive,
                format,
                &sources,
                Level::Fast,
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
            assert_eq!(read.format(), format);
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
            &CancelToken::new(),
            &mut |_, _| {},
        )
        .unwrap_err();
        assert!(matches!(error, ArcError::Io { .. }), "{error}");
        assert!(!archive.exists());
    }
}
