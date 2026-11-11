use std::collections::HashMap;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use fcmd_arc::{ArcError, Archive, Cache};
use fcmd_core::{CancelToken, EntryKind, VPath};

use crate::fs::{part_path, CopyHow, Facts, JobFs};
use crate::spec::Preserve;

/// A filesystem that can also read inside archives.
///
/// It wraps another [`JobFs`] — the local one, or the netbox's router — and takes only the
/// paths that point inside a container (`x.zip!/doc/readme.md`), handing everything else
/// straight on. That is what lets one copy job have its source in an archive and its
/// destination on the disk: the engine never asks which is which.
///
/// Reading only, for now. Every call that would change what is inside an archive refuses with
/// a message that says so, rather than half-writing a container.
///
/// One of these serves one job and is dropped with it, which is what ends anything it left
/// running: see [`Stage`].
pub struct ArcFs<F: JobFs> {
    cache: Arc<Cache>,
    inner: F,
    stages: Mutex<Stages>,
}

/// What is being unpacked through a program on the machine, an archive at a time.
#[derive(Default)]
struct Stages {
    /// What the job said it would ask for ([`JobFs::foresee`]), by the archive's own file
    /// and in the archive's own order — which is the order a program writes them in.
    foreseen: HashMap<PathBuf, Vec<String>>,
    running: HashMap<PathBuf, Stage>,
}

/// One run of an archiver, writing a set of entries into a folder of ours beside where they
/// are going, from which each is moved into place as the job asks for it.
///
/// The folder is on the destination's own filesystem, so the move is a rename and an entry
/// appears under its final name whole or not at all — the promise a copy keeps with its
/// `.fcmd-part` file. It is also why nothing here goes through the temporary folder: a
/// selection out of a `.rar` can be the size of the disk it is going to, and `/tmp` is
/// often memory.
///
/// Dropping it stops the program if it is still running and removes the folder with
/// whatever is left in it, which is what `Esc` and every failure come down to.
struct Stage {
    dir: PathBuf,
    /// The entries the program was asked for, in the archive's order.
    names: Vec<String>,
    /// How many of them have landed, as far as anybody has looked: `names[..landed]` exist.
    landed: usize,
    /// How many the job has not taken yet.
    left: usize,
    cancel: CancelToken,
    /// How the program ended, once it has.
    ended: Arc<Mutex<Option<Ended>>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

/// How a program's run ended: well, or with the kind and the words of its failure — kept
/// apart because an `io::Error` cannot be handed to every file that was waiting on it.
type Ended = Result<(), (io::ErrorKind, String)>;

impl Stage {
    fn start(archive: Arc<Archive>, names: Vec<String>, beside: &Path) -> io::Result<Stage> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let dir = beside.join(format!(
            ".fcmd-unpack-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ));
        std::fs::create_dir_all(&dir)?;
        let cancel = CancelToken::new();
        let ended = Arc::new(Mutex::new(None));
        let worker = {
            let (dir, names, cancel, ended) =
                (dir.clone(), names.clone(), cancel.clone(), ended.clone());
            std::thread::spawn(move || {
                let result = archive
                    .unpack_through_tool(&names, &dir, &cancel)
                    .map_err(into_io)
                    .map_err(|e| (e.kind(), e.to_string()));
                if let Ok(mut ended) = ended.lock() {
                    *ended = Some(result);
                }
            })
        };
        Ok(Stage {
            dir,
            left: names.len(),
            names,
            landed: 0,
            cancel,
            ended,
            worker: Some(worker),
        })
    }

    fn file_of(&self, inner: &str) -> PathBuf {
        self.dir.join(inner.trim_start_matches('/'))
    }

    fn ended(&self) -> Option<Ended> {
        self.ended.lock().ok().and_then(|ended| ended.clone())
    }

    /// Has the program started on an entry after the `at`th?
    ///
    /// These programs write one file at a time and in the archive's order, so a later file
    /// on the disk says every earlier one is finished and closed. Where a program does not
    /// keep to that order nothing is taken early: the answer stays no until it has ended,
    /// and the job waits for that instead.
    fn past(&mut self, at: usize) -> bool {
        while self.landed < self.names.len() && self.file_of(&self.names[self.landed]).exists() {
            self.landed += 1;
        }
        self.landed > at + 1
    }
}

impl Drop for Stage {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

impl<F: JobFs> ArcFs<F> {
    pub fn new(cache: Arc<Cache>, inner: F) -> ArcFs<F> {
        ArcFs {
            cache,
            inner,
            stages: Mutex::new(Stages::default()),
        }
    }

    /// One entry out of an archive a program reads, through that archive's [`Stage`].
    ///
    /// The first entry asked for starts the program, for everything the job foresaw out of
    /// this archive; this one and each after it then waits for its own file to be finished,
    /// counting its bytes as they land, and moves it into place.
    fn copy_staged(
        &self,
        archive: &Arc<Archive>,
        inner: &str,
        dst: &VPath,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let cancelled = || io::Error::new(io::ErrorKind::Interrupted, "cancelled".to_string());
        let key = archive.source().to_path_buf();
        let size = archive.entry(inner).map(|e| e.size).unwrap_or(0);
        let mut stages = self
            .stages
            .lock()
            .map_err(|_| io::Error::other("the unpacking state is poisoned"))?;
        let mut stage = match stages.running.remove(&key) {
            Some(stage) if stage.names.iter().any(|name| name == inner) => stage,
            other => {
                // An entry nobody foresaw, while a run for the others is in hand: that run
                // is left alone, and this entry gets a short one of its own.
                if let Some(stage) = other {
                    stages.running.insert(key.clone(), stage);
                }
                let names = match stages.foreseen.get(&key) {
                    Some(names) if names.iter().any(|name| name == inner) => {
                        stages.foreseen.remove(&key).unwrap_or_default()
                    }
                    _ => vec![inner.to_string()],
                };
                let beside = dst.as_path().parent().unwrap_or(Path::new("."));
                Stage::start(archive.clone(), names, beside)?
            }
        };
        let at = stage
            .names
            .iter()
            .position(|name| name == inner)
            .unwrap_or(0);
        let landed = stage.file_of(inner);
        let mut counted = 0u64;
        loop {
            // Asked before the file is measured, so that a program found ended is one whose
            // last byte is already in what is measured next.
            let ended = stage.ended();
            let now = std::fs::metadata(&landed).map(|m| m.len()).unwrap_or(0);
            let grown = now.saturating_sub(counted);
            counted += grown;
            // Asked even when nothing landed: it is also how `Esc` is heard, and a program
            // thinking about a large solid block may write nothing for a long while.
            if !on_bytes(grown) {
                // Dropping the stage kills the program and takes the folder away.
                return Err(cancelled());
            }
            match ended {
                Some(Ok(())) => break,
                Some(Err((kind, reason))) => return Err(io::Error::new(kind, reason)),
                None => {}
            }
            // All of its bytes, and the program already on to a later entry: finished.
            if now >= size && stage.past(at) {
                let now = std::fs::metadata(&landed).map(|m| m.len()).unwrap_or(now);
                if !on_bytes(now.saturating_sub(counted)) {
                    return Err(cancelled());
                }
                counted = now;
                break;
            }
            std::thread::sleep(Duration::from_millis(15));
        }
        if !landed.is_file() {
            return Err(io::Error::other(format!(
                "{}: `{}` wrote nothing for {inner}",
                archive.source().display(),
                archive.label()
            )));
        }
        // Into place in one step. Across filesystems — a destination that is a mount point
        // of its own — it is a copy through the part file instead, like any other.
        if std::fs::rename(&landed, dst.as_path()).is_err() {
            let part = part_path(dst);
            let moved = std::fs::copy(&landed, part.as_path())
                .and_then(|_| std::fs::rename(part.as_path(), dst.as_path()));
            let _ = std::fs::remove_file(&landed);
            if let Err(e) = moved {
                let _ = std::fs::remove_file(part.as_path());
                return Err(e);
            }
        }
        stage.left = stage.left.saturating_sub(1);
        if stage.left > 0 {
            stages.running.insert(key, stage);
        }
        Ok(counted)
    }

    /// The filesystem underneath, for the paths that are not in a container.
    pub fn inner(&self) -> &F {
        &self.inner
    }

    /// The archive a container path lives in, and the path inside it.
    ///
    /// A container that is not a file on this disk — one inside another archive, one on a
    /// server — is copied here by the cache and read from the copy (SPEC 04 §4), so a copy job
    /// whose source is three archives down is still one source and one destination here.
    fn archive(&self, path: &VPath) -> io::Result<(Arc<Archive>, String)> {
        if path.leave().is_none() {
            return Err(not_in_archive(path));
        }
        self.cache
            .open_in(path, &CancelToken::new())
            .map_err(into_io)
    }

    fn facts_of(&self, path: &VPath) -> io::Result<Facts> {
        let (archive, inner) = self.archive(path)?;
        if inner == "/" {
            return Ok(Facts {
                kind: EntryKind::Dir,
                size: 0,
                mtime: None,
                mode: None,
                readonly: true,
            });
        }
        let entry = archive.entry(&inner).ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, format!("{path}: no such entry"))
        })?;
        Ok(Facts {
            kind: entry.kind,
            size: entry.size,
            mtime: entry.mtime,
            mode: entry.mode,
            // Nothing inside an archive can be written through this filesystem.
            readonly: true,
        })
    }
}

fn not_in_archive(path: &VPath) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        format!("{path}: not a path inside an archive"),
    )
}

/// An archive's own errors, in the engine's currency. The message is kept whole: it already
/// names the archive and, for a refusal, the limit that fired.
fn into_io(error: ArcError) -> io::Error {
    let kind = match &error {
        ArcError::Cancelled => io::ErrorKind::Interrupted,
        ArcError::NoEntry { .. } => io::ErrorKind::NotFound,
        ArcError::Io { source, .. } => source.kind(),
        ArcError::Unknown(_) | ArcError::Broken { .. } => io::ErrorKind::InvalidData,
        // A program on the machine said no, or is not there at all. Its own words travel
        // with it, because they are the only ones that know why.
        ArcError::External { .. } => io::ErrorKind::Other,
        // A password this cannot ask for is a door that is shut, not a file that is missing.
        ArcError::Refused { .. } | ArcError::NeedsPassword { .. } => {
            io::ErrorKind::PermissionDenied
        }
    };
    io::Error::new(kind, error.to_string())
}

fn read_only(path: &VPath) -> io::Error {
    io::Error::new(
        io::ErrorKind::Unsupported,
        format!("{path}: what is inside an archive cannot be changed yet"),
    )
}

impl<F: JobFs> JobFs for ArcFs<F> {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        if path.in_container() {
            return self.facts_of(path);
        }
        self.inner.facts(path)
    }

    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        if path.in_container() {
            // A symlink inside an archive points at a name we would have to resolve inside
            // the archive; until that is worth doing, the link itself is the answer.
            return self.facts_of(path);
        }
        self.inner.facts_following(path)
    }

    fn exists(&self, path: &VPath) -> bool {
        if path.in_container() {
            return self.facts_of(path).is_ok();
        }
        self.inner.exists(path)
    }

    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
        if !path.in_container() {
            return self.inner.read_dir(path);
        }
        let (archive, inner) = self.archive(path)?;
        Ok(archive
            .list(&inner)
            .into_iter()
            .map(|entry| {
                let facts = Facts {
                    kind: entry.kind,
                    size: entry.size,
                    mtime: entry.mtime,
                    mode: None,
                    readonly: true,
                };
                (entry.name, facts)
            })
            .collect())
    }

    fn create_dir(&self, path: &VPath) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.create_dir(path)
    }

    fn create_dir_all(&self, path: &VPath) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.create_dir_all(path)
    }

    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()> {
        if from.in_container() || to.in_container() {
            return Err(read_only(if from.in_container() { from } else { to }));
        }
        self.inner.rename(from, to)
    }

    fn remove_file(&self, path: &VPath) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.remove_file(path)
    }

    /// Outside an archive the wastebasket is the one under this layer's: a method this
    /// wrapper did not pass on would be the trait's refusal for every file in the program.
    fn trash(&self, path: &VPath) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.trash(path)
    }

    fn remove_dir(&self, path: &VPath) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.remove_dir(path)
    }

    fn read_link(&self, path: &VPath) -> io::Result<PathBuf> {
        if !path.in_container() {
            return self.inner.read_link(path);
        }
        let (archive, inner) = self.archive(path)?;
        archive
            .entry(&inner)
            .and_then(|e| e.link_target.clone())
            .map(PathBuf::from)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{path}: not a link, or the archive does not say where it points"),
                )
            })
    }

    fn symlink(&self, target: &std::path::Path, link: &VPath) -> io::Result<()> {
        if link.in_container() {
            return Err(read_only(link));
        }
        self.inner.symlink(target, link)
    }

    fn same_file(&self, a: &VPath, b: &VPath) -> bool {
        if a.in_container() || b.in_container() {
            // Inside an archive the path *is* the identity, and no path in an archive is the
            // same file as one on the disk.
            return a == b;
        }
        self.inner.same_file(a, b)
    }

    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.apply_meta(path, facts, preserve)
    }

    /// An archive's entries carry no extended attributes this could read or write, so only a
    /// copy with both ends outside one has any to take along.
    fn copy_xattrs(&self, src: &VPath, dst: &VPath) -> io::Result<()> {
        if src.in_container() || dst.in_container() {
            return Ok(());
        }
        self.inner.copy_xattrs(src, dst)
    }

    /// An entry's permissions and times are the archive's, and writing them means writing the
    /// archive again for nothing: `fcmd_arc::Change` has no variant for metadata, and a dialog
    /// that offered it would have nothing to do. Outside a container this is the inner
    /// filesystem's business as everything else is.
    fn set_attrs(&self, path: &VPath, change: &crate::attrs::Change) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.set_attrs(path, change)
    }

    /// The entries about to come out of an archive a program reads are kept, by archive, so
    /// the first of them asked for can start that program once for all of them. Everything
    /// else needs no preparing: this crate's own readers seek, or are asked a file at a time.
    fn foresee(&self, files: &mut dyn Iterator<Item = &VPath>) {
        let mut outside = Vec::new();
        let mut by_host: HashMap<VPath, Vec<&VPath>> = HashMap::new();
        for path in files {
            match path.leave() {
                Some(host) => by_host.entry(host).or_default().push(path),
                None => outside.push(path),
            }
        }
        self.inner.foresee(&mut outside.into_iter());
        for (host, paths) in by_host {
            let Ok(archive) = self.cache.open_host(&host, &CancelToken::new()) else {
                continue;
            };
            if archive.tool().is_none() {
                continue;
            }
            let order: HashMap<&str, usize> = archive
                .entries()
                .iter()
                .enumerate()
                .map(|(at, entry)| (entry.path.as_str(), at))
                .collect();
            let mut names: Vec<(usize, String)> = paths
                .iter()
                .filter_map(|path| {
                    let inner = path.inner_path();
                    order.get(inner).map(|at| (*at, inner.to_string()))
                })
                .collect();
            names.sort();
            names.dedup();
            if let Ok(mut stages) = self.stages.lock() {
                stages.foreseen.insert(
                    archive.source().to_path_buf(),
                    names.into_iter().map(|(_, name)| name).collect(),
                );
            }
        }
    }

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        how: CopyHow,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        if dst.in_container() {
            return Err(read_only(dst));
        }
        if !src.in_container() {
            return self.inner.copy_file(src, dst, how, on_bytes);
        }
        if !dst.is_local() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{dst}: unpacking straight onto a remote filesystem is not supported yet"),
            ));
        }
        let (archive, inner) = self.archive(src)?;
        if archive.tool().is_some() {
            return self.copy_staged(&archive, &inner, dst, on_bytes);
        }
        // The cancellation the engine signals by returning `false` becomes the token the
        // archive understands, so a stopped copy stops inside the decompressor too.
        let cancel = CancelToken::new();
        // Under the name a copy uses for a file it has not finished, and renamed when it
        // is whole: `Esc`, a damaged entry or a full disk leave nothing called `dst`.
        let part = part_path(dst);
        let mut counted = 0;
        let result = archive.extract(&inner, part.as_path(), &cancel, &mut |bytes| {
            // The archive counts from the start of the entry and the engine wants what is
            // new since it last heard.
            let grown = bytes.saturating_sub(counted);
            counted = bytes;
            if !on_bytes(grown) {
                cancel.cancel();
            }
        });
        let result = result.map_err(|e| match e {
            ArcError::Cancelled => {
                io::Error::new(io::ErrorKind::Interrupted, "cancelled".to_string())
            }
            other => into_io(other),
        });
        match result
            .and_then(|bytes| std::fs::rename(part.as_path(), dst.as_path()).map(|()| bytes))
        {
            Ok(bytes) => Ok(bytes),
            Err(e) => {
                // A half-written file is worse than none: the engine's own copy does the same.
                let _ = std::fs::remove_file(part.as_path());
                Err(e)
            }
        }
    }

    fn append_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        if src.in_container() || dst.in_container() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "an archive entry cannot be appended to a file yet".to_string(),
            ));
        }
        self.inner.append_file(src, dst, buffer, on_bytes)
    }

    fn hard_link(&self, target: &VPath, link: &VPath) -> io::Result<()> {
        if target.in_container() || link.in_container() {
            return Err(read_only(if link.in_container() { link } else { target }));
        }
        self.inner.hard_link(target, link)
    }

    fn wipe_file(
        &self,
        path: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<()> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner.wipe_file(path, buffer, on_bytes)
    }

    /// Outside an archive the layer below reads it; an entry inside one is not read a chunk
    /// at a time yet, and says so rather than handing over the archive's own bytes.
    fn read_chunks(
        &self,
        path: &VPath,
        buffer: usize,
        on_chunk: &mut dyn FnMut(&[u8]) -> bool,
    ) -> io::Result<u64> {
        if path.in_container() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{path}: an entry inside an archive cannot be read a chunk at a time"),
            ));
        }
        self.inner.read_chunks(path, buffer, on_chunk)
    }

    fn replace_in_file(
        &self,
        path: &VPath,
        matcher: &crate::replace::Matcher,
        pass: crate::replace::ReplacePass,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<crate::replace::Replaced> {
        if path.in_container() {
            return Err(read_only(path));
        }
        self.inner
            .replace_in_file(path, matcher, pass, buffer, on_bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;
    use crate::plan::plan;
    use crate::run::{execute, Progress, Reporter};
    use crate::spec::{Decision, JobKind, JobSpec, Question};
    use fcmd_arc::Limits;
    use std::io::Write;

    struct Quiet;

    impl Reporter for Quiet {
        fn progress(&mut self, _p: &Progress) {}

        fn ask(&mut self, _question: Question) -> Decision {
            Decision::Cancel
        }
    }

    /// A zip of two files, one of them in a directory.
    fn zip_of(path: &std::path::Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, body) in [("readme.md", &b"# readme"[..]), ("src/a.txt", &b"aaaa"[..])] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    fn fs() -> ArcFs<LocalFs> {
        ArcFs::new(Arc::new(Cache::new(Limits::default())), LocalFs)
    }

    /// The program's jobs all go through this layer, so a file outside an archive must reach
    /// the system's wastebasket under it; for a while it got the trait's refusal instead,
    /// "there is no wastebasket here", on every `F8`.
    #[test]
    fn the_wastebasket_is_reached_through_the_archive_layer() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("rubbish.txt");
        std::fs::write(&file, b"out").unwrap();
        fs().trash(&VPath::local(&file))
            .expect("the wastebasket took it");
        assert!(!file.exists());
        let zip = dir.path().join("a.zip");
        zip_of(&zip);
        let inside = VPath::local(&zip).enter("/readme.md");
        assert!(
            fs().trash(&inside).is_err(),
            "an archive has no wastebasket"
        );
    }

    #[test]
    fn an_archive_looks_like_a_filesystem_from_the_outside() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let fs = fs();
        let root = VPath::local(&zip).enter("/");

        let names: Vec<String> = fs
            .read_dir(&root)
            .unwrap()
            .into_iter()
            .map(|(name, _)| name.to_string_lossy().into_owned())
            .collect();
        assert_eq!(names, ["readme.md", "src"]);

        let facts = fs.facts(&root.join("readme.md")).unwrap();
        assert_eq!((facts.kind, facts.size), (EntryKind::File, 8));
        assert!(facts.readonly, "nothing in an archive is writable yet");
        assert!(fs.facts(&root.join("src")).unwrap().is_dir());
        assert!(fs.exists(&root.join("src/a.txt")));
        assert!(!fs.exists(&root.join("nothing.txt")));

        // And a path that is not in an archive is simply handed on.
        assert!(fs.facts(&VPath::local(&zip)).unwrap().size > 0);
        assert!(!fs.facts(&VPath::local(&zip)).unwrap().readonly);
    }

    #[test]
    fn the_engine_unpacks_an_archive_without_knowing_it_is_one() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        // An ordinary copy job: the source happens to be inside an archive.
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(&zip).enter("/src")],
            Some(VPath::local(&out)),
        );
        let fs = fs();
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let outcome = execute(&fs, &spec, plan, &mut Quiet, &cancel);
        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!((outcome.files, outcome.bytes), (1, 4));
        assert_eq!(std::fs::read(out.join("src/a.txt")).unwrap(), b"aaaa");
    }

    #[test]
    fn writing_into_an_archive_is_refused_rather_than_half_done() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let fs = fs();
        let inside = VPath::local(&zip).enter("/readme.md");
        let plain = VPath::local(dir.path().join("x.txt"));
        std::fs::write(plain.as_path(), b"x").unwrap();

        for error in [
            fs.remove_file(&inside).unwrap_err(),
            fs.create_dir(&VPath::local(&zip).enter("/new"))
                .unwrap_err(),
            fs.rename(&inside, &plain).unwrap_err(),
            fs.copy_file(&plain, &inside, CopyHow::buffered(4096), &mut |_| true)
                .unwrap_err(),
        ] {
            assert_eq!(error.kind(), io::ErrorKind::Unsupported, "{error}");
            assert!(error.to_string().contains("archive"), "{error}");
        }
    }

    /// An archive inside an archive is read through the copy the cache makes of it, so the
    /// engine copies out of one without knowing there was a container in the way.
    #[test]
    fn an_archive_inside_an_archive_is_read_like_any_other() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner.zip");
        zip_of(&inner);
        let outer = dir.path().join("outer.zip");
        let file = std::fs::File::create(&outer).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "inner.zip",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(&std::fs::read(&inner).unwrap()).unwrap();
        zip.finish().unwrap();

        let fs = fs();
        let deep = VPath::local(&outer).enter("/inner.zip").enter("/src/a.txt");
        let facts = fs.facts(&deep).unwrap();
        assert_eq!((facts.kind, facts.size), (EntryKind::File, 4));
        assert!(facts.readonly, "a copy is never written to");

        let out = dir.path().join("a.txt");
        fs.copy_file(
            &deep,
            &VPath::local(&out),
            CopyHow::buffered(4096),
            &mut |_| true,
        )
        .unwrap();
        assert_eq!(std::fs::read(&out).unwrap(), b"aaaa");
    }

    /// Deeper than the limit allows is refused by the limit rather than read.
    #[test]
    fn a_container_deeper_than_the_limit_allows_says_which_limit() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let shallow = ArcFs::new(
            Arc::new(Cache::new(Limits {
                depth: 0,
                ..Limits::default()
            })),
            LocalFs,
        );
        let nested = VPath::local(&zip).enter("/inner.zip").enter("/a.txt");
        let error = shallow.facts(&nested).unwrap_err();
        assert!(error.to_string().contains("max_depth"), "{error}");
    }

    #[test]
    fn a_broken_or_missing_archive_reports_why_rather_than_an_empty_listing() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("notes.txt");
        std::fs::write(&text, b"not an archive at all").unwrap();
        let fs = fs();

        let error = fs.read_dir(&VPath::local(&text).enter("/")).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("not an archive"), "{error}");

        let missing = VPath::local(dir.path().join("nothing.zip")).enter("/");
        assert!(fs.read_dir(&missing).is_err());
        assert!(!fs.exists(&missing));
    }

    /// The largest figure a job reported, and the token to pull when asked to.
    struct Watching {
        bytes_seen: u64,
        bytes_total: u64,
        /// Cancel once this many files are done.
        stop_after: Option<(u64, CancelToken)>,
    }

    impl Watching {
        fn new() -> Watching {
            Watching {
                bytes_seen: 0,
                bytes_total: 0,
                stop_after: None,
            }
        }
    }

    impl Reporter for Watching {
        fn progress(&mut self, p: &Progress) {
            self.bytes_seen = self.bytes_seen.max(p.bytes_done);
            self.bytes_total = p.bytes_total;
            if let Some((files, cancel)) = &self.stop_after {
                if p.files_done >= *files {
                    cancel.cancel();
                }
            }
        }

        fn ask(&mut self, _question: Question) -> Decision {
            Decision::Cancel
        }
    }

    /// Everything under `dir`, as paths relative to it, sorted: what a job left behind.
    fn left_in(dir: &std::path::Path) -> Vec<String> {
        fn walk(at: &std::path::Path, root: &std::path::Path, out: &mut Vec<String>) {
            for entry in std::fs::read_dir(at).unwrap().flatten() {
                let path = entry.path();
                let rel = path.strip_prefix(root).unwrap();
                out.push(
                    rel.components()
                        .map(|c| c.as_os_str().to_string_lossy().into_owned())
                        .collect::<Vec<_>>()
                        .join("/"),
                );
                if path.is_dir() {
                    walk(&path, root, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(dir, dir, &mut out);
        out.sort();
        out
    }

    /// A megabyte that does not compress to nothing, so the decoder hands it over in many
    /// pieces and the engine reports more than once.
    fn noise(len: usize) -> Vec<u8> {
        let mut state = 0x2545_f491_4f6c_dd1du64;
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }

    /// The bytes of a file were once counted from its start at every report, so anything
    /// longer than one buffer ran the bar past the end several times over.
    #[test]
    fn unpacking_counts_each_byte_once() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.zip");
        let body = noise(3 * 1024 * 1024);
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for name in ["one.bin", "deep/two.bin"] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(&body).unwrap();
        }
        zip.finish().unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        let options = crate::spec::JobOptions {
            contents: true,
            ..Default::default()
        };
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(&path).enter("/")],
            Some(VPath::local(&out)),
        )
        .with_options(options);
        let fs = fs();
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let mut watching = Watching::new();
        let outcome = execute(&fs, &spec, plan, &mut watching, &cancel);
        assert!(outcome.is_clean(), "{outcome:?}");
        let total = 2 * body.len() as u64;
        assert_eq!((outcome.files, outcome.bytes), (2, total));
        assert_eq!(watching.bytes_total, total);
        assert_eq!(
            watching.bytes_seen, total,
            "the bar ends where the files do"
        );
        assert_eq!(std::fs::read(out.join("deep/two.bin")).unwrap(), body);
        assert_eq!(left_in(&out), ["deep", "deep/two.bin", "one.bin"]);
    }

    /// `Esc` in the middle of an entry: what was finished stays, and of the one that was
    /// not there is neither a file under its name nor the part it was being written as.
    #[test]
    fn a_cancelled_unpacking_leaves_no_half_written_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.zip");
        let body = noise(3 * 1024 * 1024);
        let file = std::fs::File::create(&path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file("big.bin", zip::write::SimpleFileOptions::default())
            .unwrap();
        zip.write_all(&body).unwrap();
        zip.finish().unwrap();
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        // Straight at the layer, to stop at a known byte: the engine's own `Esc` arrives
        // here as this `false`.
        let mut seen = 0u64;
        let error = fs()
            .copy_file(
                &VPath::local(&path).enter("/big.bin"),
                &VPath::local(out.join("big.bin")),
                CopyHow::buffered(64 * 1024),
                &mut |n| {
                    seen += n;
                    seen < 1024 * 1024
                },
            )
            .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Interrupted, "{error}");
        assert!(seen >= 1024 * 1024 && seen < body.len() as u64, "{seen}");
        assert_eq!(left_in(&out), Vec::<String>::new());

        // And an entry that is damaged ends the same way: an error, and nothing on the disk.
        let mut bytes = std::fs::read(&path).unwrap();
        let middle = bytes.len() / 2;
        for byte in &mut bytes[middle..middle + 64] {
            *byte ^= 0x55;
        }
        let broken = dir.path().join("broken.zip");
        std::fs::write(&broken, bytes).unwrap();
        let error = fs()
            .copy_file(
                &VPath::local(&broken).enter("/big.bin"),
                &VPath::local(out.join("big.bin")),
                CopyHow::buffered(64 * 1024),
                &mut |_| true,
            )
            .unwrap_err();
        assert_ne!(error.kind(), io::ErrorKind::Interrupted, "{error}");
        assert_eq!(left_in(&out), Vec::<String>::new());
    }

    /// A stand-in for an archiver: it lists three files, and asked to extract it writes the
    /// ones named in the list file, one at a time and in pieces, the way a real one does.
    #[derive(Debug)]
    struct Archiver {
        files: Vec<(&'static str, Vec<u8>)>,
        /// How many times it was started to extract.
        runs: Mutex<usize>,
        /// Write this many files whole, then half of the next, and wait to be killed.
        hang_after: Option<usize>,
        /// Say this instead of extracting anything.
        refuse: Option<&'static str>,
    }

    impl Archiver {
        fn new() -> Archiver {
            Archiver {
                files: vec![
                    ("readme.md", b"leggimi\n".to_vec()),
                    ("src/main.rs", noise(300_000)),
                    ("src/deep/x.bin", noise(200_000)),
                ],
                runs: Mutex::new(0),
                hang_after: None,
                refuse: None,
            }
        }

        fn listing(&self) -> String {
            let mut out = String::from(concat!(
                "   Date      Time    Attr         Size   Compressed  Name\n",
                "------------------- ----- ------------ ------------  ------------------------\n",
            ));
            for (name, body) in &self.files {
                out.push_str(&format!(
                    "2026-11-04 09:57:13 ....A {:>12} {:>12}  {name}\n",
                    body.len(),
                    body.len()
                ));
            }
            out.push_str(
                "------------------- ----- ------------ ------------  ------------------------\n",
            );
            out
        }
    }

    impl fcmd_arc::Run for Archiver {
        fn run(
            &self,
            _program: &Path,
            args: &[OsString],
            cwd: Option<&Path>,
            cancel: &CancelToken,
        ) -> Result<fcmd_arc::Ran, fcmd_arc::ExtError> {
            let Some(cwd) = cwd else {
                assert_eq!(args[0], "l");
                return Ok(fcmd_arc::Ran {
                    code: Some(0),
                    out: self.listing(),
                    err: String::new(),
                });
            };
            assert_eq!(args[0], "x");
            *self.runs.lock().unwrap() += 1;
            if let Some(reason) = self.refuse {
                return Ok(fcmd_arc::Ran {
                    code: Some(2),
                    out: String::new(),
                    err: reason.to_string(),
                });
            }
            let wanted = std::fs::read_to_string(cwd.join(".fcmd-names")).expect("the names");
            let mut written = 0;
            for (name, body) in &self.files {
                if !wanted.lines().any(|line| line == *name) {
                    continue;
                }
                let at = cwd.join(name);
                std::fs::create_dir_all(at.parent().unwrap()).unwrap();
                let mut file = std::fs::File::create(&at).unwrap();
                let hang = self.hang_after == Some(written);
                let pieces: Vec<&[u8]> = body.chunks(body.len().div_ceil(4).max(1)).collect();
                for (n, piece) in pieces.iter().enumerate() {
                    if hang && n == 2 {
                        file.flush().unwrap();
                        while !cancel.is_cancelled() {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        return Err(fcmd_arc::ExtError::Cancelled);
                    }
                    file.write_all(piece).unwrap();
                    file.flush().unwrap();
                    std::thread::sleep(Duration::from_millis(10));
                }
                written += 1;
            }
            Ok(fcmd_arc::Ran {
                code: Some(0),
                out: String::new(),
                err: String::new(),
            })
        }
    }

    /// An archive only a program can read, that program being `archiver`: the shipped `7z`
    /// table, pointed at a program that is certainly here so `Tool::found` says yes. Nothing
    /// is ever spawned.
    fn through(archiver: &Arc<Archiver>, dir: &Path) -> (ArcFs<LocalFs>, VPath) {
        let mut tool = fcmd_arc::builtin_tools().remove(0);
        tool.program = std::env::current_exe().unwrap().display().to_string();
        let cache = Cache::new(Limits::default())
            .with_tools(vec![Arc::new(tool)])
            .with_runner(archiver.clone());
        let path = dir.join("prova.wim");
        std::fs::write(&path, b"MSWIM\x00\x00\x00").unwrap();
        (ArcFs::new(Arc::new(cache), LocalFs), VPath::local(&path))
    }

    fn unpack_all(archive: &VPath, to: &Path) -> JobSpec {
        let options = crate::spec::JobOptions {
            contents: true,
            ..Default::default()
        };
        JobSpec::new(
            JobKind::Copy,
            vec![archive.enter("/")],
            Some(VPath::local(to)),
        )
        .with_options(options)
    }

    /// The whole of what is asked for comes out in one run of the program, each file is
    /// moved into place as it is finished, the bar counts what lands, and the folder the
    /// program wrote in is gone at the end.
    #[test]
    fn a_selection_out_of_a_program_s_archive_is_one_run_of_the_program() {
        let dir = tempfile::tempdir().unwrap();
        let archiver = Arc::new(Archiver::new());
        let (fs, archive) = through(&archiver, dir.path());
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        let spec = unpack_all(&archive, &out);
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let mut watching = Watching::new();
        let outcome = execute(&fs, &spec, plan, &mut watching, &cancel);
        drop(fs);

        assert!(outcome.is_clean(), "{outcome:?}");
        let total: u64 = archiver.files.iter().map(|(_, b)| b.len() as u64).sum();
        assert_eq!((outcome.files, outcome.bytes), (3, total));
        assert_eq!(watching.bytes_seen, total);
        assert_eq!(*archiver.runs.lock().unwrap(), 1, "one run for all three");
        for (name, body) in &archiver.files {
            assert_eq!(&std::fs::read(out.join(name)).unwrap(), body, "{name}");
        }
        assert_eq!(
            left_in(&out),
            [
                "readme.md",
                "src",
                "src/deep",
                "src/deep/x.bin",
                "src/main.rs"
            ],
            "and nothing of the program's own"
        );
    }

    /// `Esc` while the program is in the middle of a file: the program is stopped, the file
    /// it had finished and the job had taken stays, and nothing else does.
    #[test]
    fn stopping_a_program_s_extraction_keeps_only_what_was_finished() {
        let dir = tempfile::tempdir().unwrap();
        let archiver = Arc::new(Archiver {
            hang_after: Some(1),
            ..Archiver::new()
        });
        let (fs, archive) = through(&archiver, dir.path());
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        let spec = unpack_all(&archive, &out);
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let mut watching = Watching::new();
        watching.stop_after = Some((1, cancel.clone()));
        let outcome = execute(&fs, &spec, plan, &mut watching, &cancel);
        drop(fs);

        assert!(outcome.cancelled, "{outcome:?}");
        assert_eq!(outcome.files, 1);
        let left = left_in(&out);
        assert!(left.contains(&"readme.md".to_string()), "{left:?}");
        assert!(
            left.iter()
                .all(|name| !name.contains(".fcmd") && !name.ends_with(".rs")),
            "{left:?}"
        );
    }

    /// A program that says no is an error against each file it was asked for, in its own
    /// words, and leaves nothing behind either.
    #[test]
    fn a_program_that_refuses_to_extract_is_an_error_per_file() {
        let dir = tempfile::tempdir().unwrap();
        let archiver = Arc::new(Archiver {
            refuse: Some("ERROR: Unsupported Method"),
            ..Archiver::new()
        });
        let (fs, archive) = through(&archiver, dir.path());
        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();

        let options = crate::spec::JobOptions {
            on_error: crate::spec::ErrorPolicy::Skip,
            ..Default::default()
        };
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![archive.enter("/readme.md")],
            Some(VPath::local(&out)),
        )
        .with_options(options);
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let outcome = execute(&fs, &spec, plan, &mut Quiet, &cancel);
        drop(fs);
        assert_eq!(outcome.errors.len(), 1, "{outcome:?}");
        assert!(
            outcome.errors[0].1.contains("Unsupported Method"),
            "{outcome:?}"
        );
        assert_eq!(left_in(&out), Vec::<String>::new());
    }

    /// The real thing, when the machine has it: `7z` makes a `.wim`, which nothing here
    /// parses, and a job unpacks all of it through the shipped table. Skipped where there is
    /// no `7z` rather than failed.
    #[test]
    fn a_wim_is_unpacked_through_7z_when_7z_is_here() {
        let Some(program) = fcmd_arc::which("7z") else {
            eprintln!("no 7z on this machine: skipped");
            return;
        };
        let dir = tempfile::tempdir().unwrap();
        let made = dir.path().join("made");
        std::fs::create_dir_all(made.join("src/deep")).unwrap();
        let big = noise(700_000);
        std::fs::write(made.join("src/main.rs"), b"ciao\n").unwrap();
        std::fs::write(made.join("src/deep/big.bin"), &big).unwrap();
        std::fs::write(made.join("readme.md"), b"leggimi\n").unwrap();
        let ran = std::process::Command::new(&program)
            .args(["a", "-bd", "-twim", "prova.wim", "src", "readme.md"])
            .current_dir(&made)
            .output()
            .expect("7z runs");
        assert!(ran.status.success(), "{ran:?}");

        let out = dir.path().join("out");
        std::fs::create_dir(&out).unwrap();
        let fs = fs();
        let spec = unpack_all(&VPath::local(made.join("prova.wim")), &out);
        let cancel = CancelToken::new();
        let plan = plan(&fs, &spec, &cancel).expect("the plan");
        let mut watching = Watching::new();
        let outcome = execute(&fs, &spec, plan, &mut watching, &cancel);
        drop(fs);
        assert!(outcome.is_clean(), "{outcome:?}");
        assert_eq!(outcome.files, 3);
        assert_eq!(outcome.bytes, big.len() as u64 + 5 + 8);
        assert_eq!(watching.bytes_seen, outcome.bytes);
        assert_eq!(std::fs::read(out.join("src/deep/big.bin")).unwrap(), big);
        assert_eq!(std::fs::read(out.join("src/main.rs")).unwrap(), b"ciao\n");
        assert_eq!(
            left_in(&out),
            [
                "readme.md",
                "src",
                "src/deep",
                "src/deep/big.bin",
                "src/main.rs"
            ]
        );
    }
}
