//! Splitting a file into pieces and putting the pieces back together (SPEC 03 §9).
//!
//! The pieces are the ones Total Commander writes and reads, so a file split here can be
//! combined there and the other way round: `name.001`, `name.002`, … beside a `name.crc` that
//! says what they make —
//!
//! ```text
//! filename=holiday.mkv
//! size=4700372992
//! crc32=1A2B3C4D
//! ```
//!
//! Combining checks the result against that file when it is there, and refuses to leave a
//! file that does not match: a missing or a damaged piece is the whole reason the `.crc` is
//! written. Local files only, like the checksums, because the bytes are read and written
//! here.

use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use fcmd_core::CancelToken;

/// The pieces a split wrote, and the list beside them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pieces {
    pub parts: Vec<PathBuf>,
    pub crc_file: PathBuf,
}

/// What combining made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Joined {
    pub path: PathBuf,
    pub bytes: u64,
    pub parts: usize,
    /// Whether a `.crc` was there to check against. When there was, the size and the CRC
    /// matched it, or the call would have failed.
    pub checked: bool,
}

/// What a `.crc` file says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Description {
    pub filename: String,
    pub size: Option<u64>,
    pub crc32: Option<u32>,
}

/// The name of the `n`th piece of `name`, from 1: three digits, as Total Commander writes
/// them, and more only past 999.
pub fn part_name(name: &str, n: usize) -> String {
    format!("{name}.{n:03}")
}

/// The piece size that cuts `total` bytes into exactly `count` pieces, the way the format
/// cuts: every piece the same size and the last one what is left. Not every count can be
/// reached like that — ten bytes are four pieces of three or five of two, never six — and a
/// count that cannot is refused naming the nearest that can, rather than quietly making a
/// different number of pieces from the one asked for.
pub fn size_for_count(total: u64, count: u64) -> io::Result<u64> {
    let refuse = |why: String| io::Error::new(io::ErrorKind::InvalidInput, why);
    if count < 2 {
        return Err(refuse("a split makes two pieces or more".to_string()));
    }
    if total < count {
        return Err(refuse(format!(
            "a file of {total} bytes cannot be cut into {count} pieces"
        )));
    }
    let size = total.div_ceil(count);
    let made = total.div_ceil(size);
    if made != count {
        return Err(refuse(format!(
            "{total} bytes cannot be cut into {count} pieces of one size; {made} pieces of {size} bytes is the nearest"
        )));
    }
    Ok(size)
}

/// Split `src` into pieces of `size` bytes in `out_dir`, the last one shorter, and write the
/// `.crc` beside them. Nothing is written over: a piece or a list that is already there stops
/// it before anything is written, and a failure or a cancel takes back what was written.
pub fn split(
    src: &Path,
    out_dir: &Path,
    size: u64,
    cancel: &CancelToken,
    on_bytes: &mut dyn FnMut(u64),
) -> io::Result<Pieces> {
    if size == 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "a piece has to be at least one byte",
        ));
    }
    let name = src
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))?;
    let total = fs::metadata(src)?.len();
    if total <= size {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "the file is no bigger than one piece",
        ));
    }
    let count = total.div_ceil(size) as usize;
    let parts: Vec<PathBuf> = (1..=count)
        .map(|n| out_dir.join(part_name(&name, n)))
        .collect();
    let crc_file = out_dir.join(format!("{name}.crc"));
    for path in parts.iter().chain(std::iter::once(&crc_file)) {
        if path.exists() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                format!("{} is already there", path.display()),
            ));
        }
    }

    let result = (|| -> io::Result<u32> {
        let mut reader = File::open(src)?;
        let mut crc = crc32fast::Hasher::new();
        let mut buf = vec![0u8; 256 * 1024];
        for path in &parts {
            let mut writer = File::create(path)?;
            let mut left = size;
            while left > 0 {
                if cancel.is_cancelled() {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
                }
                let want = buf.len().min(left as usize);
                let read = reader.read(&mut buf[..want])?;
                if read == 0 {
                    break;
                }
                writer.write_all(&buf[..read])?;
                crc.update(&buf[..read]);
                left -= read as u64;
                on_bytes(read as u64);
            }
            writer.flush()?;
        }
        Ok(crc.finalize())
    })();
    match result {
        Ok(crc) => {
            let text = format!("filename={name}\r\nsize={total}\r\ncrc32={crc:08X}\r\n");
            if let Err(e) = fs::write(&crc_file, text) {
                remove_all(&parts);
                return Err(e);
            }
            Ok(Pieces { parts, crc_file })
        }
        Err(e) => {
            remove_all(&parts);
            Err(e)
        }
    }
}

fn remove_all(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

/// Read a `.crc` the way Total Commander writes it: `key=value` lines, in any order, any of
/// them missing but the name.
pub fn parse_description(text: &str) -> Option<Description> {
    let mut filename = None;
    let mut size = None;
    let mut crc32 = None;
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim();
        match key.trim().to_ascii_lowercase().as_str() {
            "filename" => filename = Some(value.to_string()),
            "size" => size = value.parse().ok(),
            "crc32" => crc32 = u32::from_str_radix(value, 16).ok(),
            _ => {}
        }
    }
    Some(Description {
        filename: filename.filter(|f| !f.is_empty())?,
        size,
        crc32,
    })
}

/// The pieces that go with `any`, which may be one of them (`name.001`, `name.007`) or the
/// `.crc`: the base name, and every piece from `.001` up for as long as they follow on.
pub fn pieces_of(any: &Path) -> Option<(PathBuf, Vec<PathBuf>)> {
    let dir = any.parent()?;
    let file = any.file_name()?.to_string_lossy().into_owned();
    let (base, ext) = file.rsplit_once('.')?;
    let numbered = ext.len() >= 3 && ext.bytes().all(|b| b.is_ascii_digit());
    if !numbered && !ext.eq_ignore_ascii_case("crc") {
        return None;
    }
    let base = dir.join(base);
    let base_name = base.file_name()?.to_string_lossy().into_owned();
    let mut parts = Vec::new();
    for n in 1.. {
        let path = dir.join(part_name(&base_name, n));
        if !path.is_file() {
            break;
        }
        parts.push(path);
    }
    (!parts.is_empty()).then_some((base, parts))
}

/// Put the pieces that go with `any` back together in `out_dir`. The file takes the name the
/// `.crc` gives it, or the pieces' own base name when there is no `.crc`; it is written under
/// a temporary name and appears only when it is whole and, where there is a `.crc`, matches it.
pub fn combine(
    any: &Path,
    out_dir: &Path,
    cancel: &CancelToken,
    on_bytes: &mut dyn FnMut(u64),
) -> io::Result<Joined> {
    let (base, parts) = pieces_of(any).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no pieces here: a split file's pieces end in .001, .002, …",
        )
    })?;
    let crc_path = base.with_file_name(format!(
        "{}.crc",
        base.file_name().unwrap_or_default().to_string_lossy()
    ));
    let description = fs::read_to_string(&crc_path)
        .ok()
        .and_then(|t| parse_description(&t));
    let name = description
        .as_ref()
        .map(|d| d.filename.clone())
        .unwrap_or_else(|| {
            base.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into()
        });
    // The name comes out of a file, so it is kept to a name: no directory in it.
    let name = Path::new(&name)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or(name);
    let target = out_dir.join(&name);
    if target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} is already there", target.display()),
        ));
    }
    let temp = out_dir.join(format!("{name}{}", crate::fs::PART_SUFFIX));

    let result = (|| -> io::Result<(u64, u32)> {
        let mut writer = File::create(&temp)?;
        let mut crc = crc32fast::Hasher::new();
        let mut buf = vec![0u8; 256 * 1024];
        let mut bytes = 0u64;
        for part in &parts {
            let mut reader = File::open(part)?;
            loop {
                if cancel.is_cancelled() {
                    return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
                }
                let read = reader.read(&mut buf)?;
                if read == 0 {
                    break;
                }
                writer.write_all(&buf[..read])?;
                crc.update(&buf[..read]);
                bytes += read as u64;
                on_bytes(read as u64);
            }
        }
        writer.flush()?;
        Ok((bytes, crc.finalize()))
    })();
    let (bytes, crc) = match result {
        Ok(done) => done,
        Err(e) => {
            let _ = fs::remove_file(&temp);
            return Err(e);
        }
    };
    if let Some(d) = &description {
        let size_ok = d.size.is_none_or(|s| s == bytes);
        let crc_ok = d.crc32.is_none_or(|c| c == crc);
        if !size_ok || !crc_ok {
            let _ = fs::remove_file(&temp);
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "the pieces do not make the file {} describes: a piece is missing or damaged",
                    crc_path.display()
                ),
            ));
        }
    }
    fs::rename(&temp, &target)?;
    Ok(Joined {
        path: target,
        bytes,
        parts: parts.len(),
        checked: description.is_some(),
    })
}

#[cfg(test)]
mod tests {

    #[test]
    fn a_count_of_pieces_is_a_size_when_one_size_can_make_it() {
        // A 4 GB DVD image in five: four of 860 MiB-ish and what is left.
        let total = 4_294_967_296u64;
        let size = size_for_count(total, 5).unwrap();
        assert_eq!(total.div_ceil(size), 5);
        // Ten bytes are three pieces of four, four of three, five of two, ten of one.
        assert_eq!(size_for_count(10, 4).unwrap(), 3);
        assert_eq!(size_for_count(10, 5).unwrap(), 2);
        let six = size_for_count(10, 6).unwrap_err().to_string();
        assert!(six.contains("5 pieces of 2 bytes"), "{six}");
        assert!(size_for_count(10, 1).is_err(), "one piece is not a split");
        assert!(size_for_count(3, 4).is_err(), "more pieces than bytes");
    }

    use super::*;

    fn body(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 7 % 256) as u8).collect()
    }

    #[test]
    fn a_file_splits_into_numbered_pieces_and_comes_back_whole() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("video.mkv");
        let data = body(250_000);
        fs::write(&src, &data).unwrap();
        let pieces_dir = dir.path().join("pieces");
        fs::create_dir(&pieces_dir).unwrap();

        let mut moved = 0;
        let pieces = split(&src, &pieces_dir, 100_000, &CancelToken::new(), &mut |n| {
            moved += n
        })
        .unwrap();
        assert_eq!(moved, 250_000);
        let names: Vec<String> = pieces
            .parts
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["video.mkv.001", "video.mkv.002", "video.mkv.003"]);
        assert_eq!(fs::metadata(&pieces.parts[2]).unwrap().len(), 50_000);
        // The list Total Commander reads.
        let crc = fs::read_to_string(&pieces.crc_file).unwrap();
        let expected = format!("{:08X}", crc32fast::hash(&data));
        assert!(crc.contains("filename=video.mkv"), "{crc}");
        assert!(crc.contains("size=250000"), "{crc}");
        assert!(crc.contains(&format!("crc32={expected}")), "{crc}");

        // Back together, from any piece, into another folder.
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let joined = combine(&pieces.parts[1], &out, &CancelToken::new(), &mut |_| {}).unwrap();
        assert_eq!(joined.path, out.join("video.mkv"));
        assert_eq!(
            (joined.bytes, joined.parts, joined.checked),
            (250_000, 3, true)
        );
        assert_eq!(fs::read(&joined.path).unwrap(), data);
    }

    #[test]
    fn a_damaged_piece_is_caught_and_leaves_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.bin");
        fs::write(&src, body(30_000)).unwrap();
        let pieces = split(&src, dir.path(), 10_000, &CancelToken::new(), &mut |_| {}).unwrap();
        let mut second = fs::read(&pieces.parts[1]).unwrap();
        second[5] ^= 0xFF;
        fs::write(&pieces.parts[1], second).unwrap();
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let err = combine(&pieces.crc_file, &out, &CancelToken::new(), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
        assert_eq!(fs::read_dir(&out).unwrap().count(), 0, "no file, no part");

        // A missing last piece is a wrong size, caught the same way.
        fs::remove_file(&pieces.parts[2]).unwrap();
        fs::write(&pieces.parts[1], &body(30_000)[10_000..20_000]).unwrap();
        let err = combine(&pieces.parts[0], &out, &CancelToken::new(), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn nothing_is_written_over_and_pieces_without_a_list_still_join() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("b.bin");
        fs::write(&src, body(20_000)).unwrap();
        let pieces = split(&src, dir.path(), 10_000, &CancelToken::new(), &mut |_| {}).unwrap();
        // A second split finds its pieces there and stops before writing anything.
        let err = split(&src, dir.path(), 10_000, &CancelToken::new(), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        // Without the list the pieces are joined under their own base name, unchecked.
        fs::remove_file(&pieces.crc_file).unwrap();
        let out = dir.path().join("out");
        fs::create_dir(&out).unwrap();
        let joined = combine(&pieces.parts[0], &out, &CancelToken::new(), &mut |_| {}).unwrap();
        assert_eq!(joined.path, out.join("b.bin"));
        assert!(!joined.checked);
        // A file no bigger than one piece is not split.
        let err = split(&src, &out, 50_000, &CancelToken::new(), &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn a_cancelled_split_takes_its_pieces_back() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("c.bin");
        fs::write(&src, body(40_000)).unwrap();
        let cancel = CancelToken::new();
        cancel.cancel();
        let err = split(&src, dir.path(), 10_000, &cancel, &mut |_| {}).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::Interrupted);
        assert!(!dir.path().join("c.bin.001").exists());
    }

    #[test]
    fn a_list_is_read_whatever_its_order_and_line_ends() {
        let d = parse_description("crc32=0000ABCD\nfilename=x.iso\r\nsize=12\n").unwrap();
        assert_eq!(d.filename, "x.iso");
        assert_eq!((d.size, d.crc32), (Some(12), Some(0xABCD)));
        assert!(parse_description("size=1\n").is_none(), "no name, no list");
    }
}
