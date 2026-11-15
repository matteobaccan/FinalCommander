//! Encoding files as text and decoding them again, on the disk (SPEC 03 §9a).
//!
//! The formats themselves are `fcmd_core::codec`; this is the part that reads and writes.
//! Encoding puts the encoded file beside the one it was made from; decoding puts what comes
//! out beside the encoded file, under the name that file carries. Nothing is ever written
//! over: a name that is taken stops that file before a byte of it is written.
//!
//! A file in parts is one piece of work. Its parts are written all or none, and decoding one
//! of them finds the others beside it, so marking `big.iso.part03.uue` alone joins all
//! twelve. Local files only, like the checksums and the split, because the bytes are read
//! and written here; and whole files in memory, because every one of these formats is read
//! whole — which is what [`LIMIT`] is for.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use fcmd_core::codec::{self, EncodeOptions, Encoder, Input};
use fcmd_core::CancelToken;

/// The largest file that is encoded, and the most that one decode reads. A quarter of a
/// gigabyte is every attachment and every posting there has been; past it the answer is an
/// archiver, which splits and checks and does not grow the file by a third.
pub const LIMIT: u64 = 256 * 1024 * 1024;

/// What a run made, and what it could not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    /// Every file written, in the order they were: what an undo takes back.
    pub written: Vec<PathBuf>,
    /// The files that were not done, each with why. A file in parts is named by the part
    /// that was asked for.
    pub errors: Vec<(PathBuf, String)>,
    pub cancelled: bool,
    /// Decoding only: how many of the files that came out were joined from several parts.
    pub joined: usize,
    /// Decoding only: how many were held against a checksum the format carried.
    pub checked: usize,
}

fn too_large(bytes: u64) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!(
            "{} is more than is encoded or decoded in one go ({})",
            fcmd_core::format_size(bytes),
            fcmd_core::format_size(LIMIT)
        ),
    )
}

fn already_there(path: &Path) -> io::Error {
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} is already there", path.display()),
    )
}

/// Write a file that must not be there yet. `create_new` and not a look first: between the
/// look and the write somebody else can make the file, and this never writes over one.
fn write_new(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(bytes).inspect_err(|_| {
        let _ = fs::remove_file(path);
    })
}

fn remove_all(paths: &[PathBuf]) {
    for path in paths {
        let _ = fs::remove_file(path);
    }
}

fn name_of(path: &Path) -> io::Result<String> {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "no file name"))
}

/// One file, encoded beside itself. `Ok(None)` is "stopped": what was written of it is
/// taken back, because half the parts of a file are no use to anybody.
fn encode_one(
    file: &Path,
    options: &EncodeOptions,
    cancel: &CancelToken,
    on_bytes: &mut dyn FnMut(u64),
) -> io::Result<Option<Vec<PathBuf>>> {
    let name = name_of(file)?;
    let dir = file.parent().unwrap_or(Path::new("."));
    let size = fs::metadata(file)?.len();
    if size > LIMIT {
        return Err(too_large(size));
    }
    let data = fs::read(file)?;
    let encoder = Encoder::new(&name, &data, options)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;
    let count = encoder.count();
    let each = data.len() as u64 / count as u64;
    let mut written: Vec<PathBuf> = Vec::with_capacity(count);
    for index in 0..count {
        if cancel.is_cancelled() {
            remove_all(&written);
            return Ok(None);
        }
        let part = encoder.part(index);
        let path = dir.join(&part.name);
        if let Err(e) = write_new(&path, &part.bytes) {
            remove_all(&written);
            return Err(if e.kind() == io::ErrorKind::AlreadyExists {
                already_there(&path)
            } else {
                e
            });
        }
        written.push(path);
        // The last part is told what is left, so the bytes add up to the file.
        on_bytes(if index + 1 == count {
            data.len() as u64 - each * (count as u64 - 1)
        } else {
            each
        });
    }
    Ok(Some(written))
}

/// Encode each of `files` beside itself. A file that fails is named with its reason and the
/// rest go on; `Esc` stops before the next part, and the file it stopped in is taken back.
pub fn encode(
    files: &[PathBuf],
    options: &EncodeOptions,
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Outcome {
    let total: u64 = files
        .iter()
        .filter_map(|f| fs::metadata(f).ok())
        .map(|m| m.len())
        .sum();
    let mut done = 0u64;
    let mut outcome = Outcome::default();
    for file in files {
        let before = done;
        let result = encode_one(file, options, cancel, &mut |n| {
            done += n;
            on_progress(done, total);
        });
        match result {
            Ok(Some(written)) => outcome.written.extend(written),
            Ok(None) => {
                outcome.cancelled = true;
                break;
            }
            Err(e) => {
                // What it would have counted is counted anyway, or the total is never met.
                done = before + fs::metadata(file).map(|m| m.len()).unwrap_or(0);
                on_progress(done, total);
                outcome.errors.push((file.clone(), e.to_string()));
            }
        }
    }
    outcome
}

/// The files to read together: each file on its own, except one that is a part of several
/// (`name.partN.ext`), which brings the other parts in its folder with it. A part that was
/// asked for twice — marked, or brought in by another — is read once.
pub fn clusters(files: &[PathBuf]) -> Vec<Vec<PathBuf>> {
    let mut out: Vec<Vec<PathBuf>> = Vec::new();
    for file in files {
        if out.iter().any(|cluster| cluster.contains(file)) {
            continue;
        }
        let parts = file
            .file_name()
            .and_then(|n| codec::part_of(&n.to_string_lossy()))
            .and_then(|(stem, _, ext)| {
                let dir = file.parent()?;
                let mut found: Vec<(usize, PathBuf)> = fs::read_dir(dir)
                    .ok()?
                    .filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let name = entry.file_name();
                        let (other, n, other_ext) = codec::part_of(&name.to_string_lossy())?;
                        (other == stem && other_ext == ext).then(|| (n, entry.path()))
                    })
                    .collect();
                found.sort();
                Some(found.into_iter().map(|(_, path)| path).collect::<Vec<_>>())
            })
            .filter(|parts| !parts.is_empty());
        out.push(parts.unwrap_or_else(|| vec![file.clone()]));
    }
    out
}

/// One file, or the parts of one, decoded beside the first of them. Everything is checked
/// before anything is written, and what is written is written all or none.
fn decode_one(cluster: &[PathBuf], outcome: &mut Outcome) -> io::Result<()> {
    let refuse = |why: String| io::Error::new(io::ErrorKind::InvalidData, why);
    let first = cluster
        .first()
        .ok_or_else(|| refuse("nothing to decode".to_string()))?;
    let dir = first.parent().unwrap_or(Path::new("."));
    let size: u64 = cluster
        .iter()
        .map(|p| fs::metadata(p).map(|m| m.len()))
        .sum::<io::Result<u64>>()?;
    if size > LIMIT {
        return Err(too_large(size));
    }
    let mut read: Vec<(String, Vec<u8>)> = Vec::with_capacity(cluster.len());
    for path in cluster {
        read.push((name_of(path)?, fs::read(path)?));
    }
    let inputs: Vec<Input<'_>> = read
        .iter()
        .map(|(name, bytes)| Input { name, bytes })
        .collect();
    let decoded = codec::decode_files(&inputs).map_err(|e| refuse(e.to_string()))?;
    let targets: Vec<PathBuf> = decoded.iter().map(|d| dir.join(&d.name)).collect();
    if let Some(taken) = targets.iter().find(|t| t.exists()) {
        return Err(already_there(taken));
    }
    let mut written = Vec::with_capacity(targets.len());
    for (file, path) in decoded.iter().zip(&targets) {
        if let Err(e) = write_new(path, &file.bytes) {
            remove_all(&written);
            return Err(if e.kind() == io::ErrorKind::AlreadyExists {
                already_there(path)
            } else {
                e
            });
        }
        written.push(path.clone());
    }
    outcome.joined += decoded.iter().filter(|d| d.parts > 1).count();
    outcome.checked += decoded.iter().filter(|d| d.checked).count();
    outcome.written.extend(written);
    Ok(())
}

/// Decode each of `files` beside itself, the parts of a split one together. A file that is
/// not encoded, or is damaged, is named with its reason and the rest go on; `Esc` stops
/// before the next file.
pub fn decode(
    files: &[PathBuf],
    cancel: &CancelToken,
    on_progress: &mut dyn FnMut(u64, u64),
) -> Outcome {
    let clusters = clusters(files);
    let size_of = |cluster: &[PathBuf]| -> u64 {
        cluster
            .iter()
            .filter_map(|p| fs::metadata(p).ok())
            .map(|m| m.len())
            .sum()
    };
    let total: u64 = clusters.iter().map(|c| size_of(c)).sum();
    let mut done = 0u64;
    let mut outcome = Outcome::default();
    for cluster in &clusters {
        if cancel.is_cancelled() {
            outcome.cancelled = true;
            break;
        }
        let size = size_of(cluster);
        if let Err(e) = decode_one(cluster, &mut outcome) {
            // Named by the file that was asked for, which is the one the reader knows.
            let asked = cluster
                .iter()
                .find(|p| files.contains(p))
                .or(cluster.first())
                .cloned()
                .unwrap_or_default();
            outcome.errors.push((asked, e.to_string()));
        }
        done += size;
        on_progress(done, total);
    }
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;
    use fcmd_core::codec::Format;

    fn body(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i * 31 + i / 251) as u8).collect()
    }

    fn options(format: Format) -> EncodeOptions {
        EncodeOptions {
            format,
            line: None,
            part_size: None,
        }
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_file_is_encoded_beside_itself_and_decoded_back_under_its_own_name() {
        for format in Format::FILES {
            let dir = tempfile::tempdir().unwrap();
            let src = dir.path().join("photo.jpg");
            let data = body(5_000);
            fs::write(&src, &data).unwrap();
            let mut seen = Vec::new();
            let made = encode(
                std::slice::from_ref(&src),
                &options(format),
                &CancelToken::new(),
                &mut |d, t| seen.push((d, t)),
            );
            let coded = dir
                .path()
                .join(format!("photo.jpg.{}", format.extension().unwrap()));
            assert_eq!(made.written, vec![coded.clone()], "{format:?}");
            assert!(made.errors.is_empty() && !made.cancelled);
            assert_eq!(
                seen.last(),
                Some(&(5_000, 5_000)),
                "the progress reaches the end"
            );

            // The original out of the way, so that what comes back can take its name.
            fs::remove_file(&src).unwrap();
            let back = decode(&[coded], &CancelToken::new(), &mut |_, _| {});
            assert_eq!(back.written, vec![src.clone()], "{format:?}");
            assert!(back.errors.is_empty(), "{:?}", back.errors);
            assert_eq!(fs::read(&src).unwrap(), data);
            assert_eq!(back.checked, usize::from(format == Format::Yenc));
            assert_eq!(back.joined, 0);
        }
    }

    #[test]
    fn a_file_in_parts_is_joined_from_any_one_of_them() {
        for format in [Format::Base64, Format::Uu, Format::Yenc] {
            let dir = tempfile::tempdir().unwrap();
            let src = dir.path().join("big.iso");
            let data = body(10_000);
            fs::write(&src, &data).unwrap();
            let mut opts = options(format);
            opts.part_size = Some(3_000);
            let made = encode(
                std::slice::from_ref(&src),
                &opts,
                &CancelToken::new(),
                &mut |_, _| {},
            );
            assert_eq!(made.written.len(), 4);
            let ext = format.extension().unwrap();
            assert_eq!(
                names(dir.path()),
                [
                    "big.iso".to_string(),
                    format!("big.iso.part1.{ext}"),
                    format!("big.iso.part2.{ext}"),
                    format!("big.iso.part3.{ext}"),
                    format!("big.iso.part4.{ext}"),
                ]
            );

            // Only the third is asked for; the others are found beside it.
            fs::remove_file(&src).unwrap();
            let third = made.written[2].clone();
            assert_eq!(
                clusters(std::slice::from_ref(&third)),
                vec![made.written.clone()]
            );
            let back = decode(
                std::slice::from_ref(&third),
                &CancelToken::new(),
                &mut |_, _| {},
            );
            assert!(back.errors.is_empty(), "{:?}", back.errors);
            assert_eq!(back.written, vec![src.clone()]);
            assert_eq!(back.joined, 1);
            assert_eq!(fs::read(&src).unwrap(), data);

            // All four marked is the same file once, not four times.
            fs::remove_file(&src).unwrap();
            let back = decode(&made.written, &CancelToken::new(), &mut |_, _| {});
            assert_eq!(back.written, vec![src.clone()]);

            // A part gone: nothing is written, and the reason names what was asked for.
            fs::remove_file(&src).unwrap();
            fs::remove_file(&made.written[1]).unwrap();
            let back = decode(
                std::slice::from_ref(&third),
                &CancelToken::new(),
                &mut |_, _| {},
            );
            assert!(back.written.is_empty());
            assert_eq!(back.errors.len(), 1);
            assert_eq!(back.errors[0].0, third);
            assert!(back.errors[0].1.contains("piece 2"), "{}", back.errors[0].1);
            assert!(!src.exists());
        }
    }

    #[test]
    fn nothing_is_written_over_and_one_refusal_does_not_stop_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        fs::write(&a, "alpha").unwrap();
        fs::write(&b, "beta").unwrap();
        // `a.txt.b64` is taken by something else.
        fs::write(dir.path().join("a.txt.b64"), "mine").unwrap();
        let made = encode(
            &[a.clone(), b.clone()],
            &options(Format::Base64),
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(made.written, vec![dir.path().join("b.txt.b64")]);
        assert_eq!(made.errors.len(), 1);
        assert_eq!(made.errors[0].0, a);
        assert!(made.errors[0].1.contains("already there"));
        assert_eq!(fs::read(dir.path().join("a.txt.b64")).unwrap(), b"mine");

        // Decoding beside the original is refused for the same reason; and a file that is
        // not encoded says so and leaves the next one to be done.
        let prose = dir.path().join("notes.txt");
        fs::write(&prose, "just some prose\n").unwrap();
        fs::remove_file(&b).unwrap();
        fs::write(dir.path().join("a.txt.hex"), "616c706861\n").unwrap();
        let back = decode(
            &[
                dir.path().join("a.txt.hex"),
                prose.clone(),
                dir.path().join("b.txt.b64"),
            ],
            &CancelToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(back.written, vec![b.clone()]);
        assert_eq!(fs::read(&b).unwrap(), b"beta");
        assert_eq!(back.errors.len(), 2);
        assert!(back.errors[0].1.contains("already there"));
        assert_eq!(back.errors[1].0, prose);
        assert_eq!(fs::read(&a).unwrap(), b"alpha", "the original is as it was");
    }

    #[test]
    fn a_stop_takes_back_the_file_it_stopped_in_and_keeps_what_was_finished() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.bin");
        let b = dir.path().join("b.bin");
        fs::write(&a, body(100)).unwrap();
        fs::write(&b, body(4_000)).unwrap();
        let cancel = CancelToken::new();
        let mut opts = options(Format::Hex);
        opts.part_size = Some(1_000);
        // Stopped once the second file's second part is down.
        let stopper = cancel.clone();
        let made = encode(&[a.clone(), b.clone()], &opts, &cancel, &mut |done, _| {
            if done >= 2_100 {
                stopper.cancel();
            }
        });
        assert!(made.cancelled);
        assert_eq!(made.written, vec![dir.path().join("a.bin.hex")]);
        assert_eq!(names(dir.path()), ["a.bin", "a.bin.hex", "b.bin"]);

        // And a decode that is stopped before it starts writes nothing.
        let back = decode(&made.written, &cancel, &mut |_, _| {});
        assert!(back.cancelled && back.written.is_empty());
    }

    #[test]
    fn a_damaged_yenc_file_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("a.bin");
        fs::write(&src, body(2_000)).unwrap();
        let made = encode(
            std::slice::from_ref(&src),
            &options(Format::Yenc),
            &CancelToken::new(),
            &mut |_, _| {},
        );
        fs::remove_file(&src).unwrap();
        let mut bytes = fs::read(&made.written[0]).unwrap();
        bytes[100] = bytes[100].wrapping_add(1);
        fs::write(&made.written[0], &bytes).unwrap();
        let back = decode(&made.written, &CancelToken::new(), &mut |_, _| {});
        assert!(back.written.is_empty());
        assert!(back.errors[0].1.contains("CRC32"), "{}", back.errors[0].1);
        assert!(!src.exists());
    }
}
