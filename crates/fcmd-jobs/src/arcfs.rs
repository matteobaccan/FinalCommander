use std::ffi::OsString;
use std::io;
use std::path::PathBuf;
use std::sync::Arc;

use fcmd_arc::{ArcError, Archive, Cache};
use fcmd_core::{CancelToken, EntryKind, VPath};

use crate::fs::{Facts, JobFs};
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
pub struct ArcFs<F: JobFs> {
    cache: Arc<Cache>,
    inner: F,
}

impl<F: JobFs> ArcFs<F> {
    pub fn new(cache: Arc<Cache>, inner: F) -> ArcFs<F> {
        ArcFs { cache, inner }
    }

    /// The filesystem underneath, for the paths that are not in a container.
    pub fn inner(&self) -> &F {
        &self.inner
    }

    /// The archive a container path lives in, and the path inside it.
    fn archive(&self, path: &VPath) -> io::Result<(Arc<Archive>, String)> {
        let host = path.leave().ok_or_else(|| not_in_archive(path))?;
        if host.in_container() {
            // Nesting is SPEC 04 §4's and is not here yet: an archive inside an archive has
            // to be unpacked to a temporary file first, and nothing does that today.
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{host}: an archive inside an archive cannot be read yet"),
            ));
        }
        if !host.is_local() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{host}: an archive can only be read from a local disk"),
            ));
        }
        let archive = self
            .cache
            .open(host.as_path(), &CancelToken::new())
            .map_err(into_io)?;
        Ok((archive, path.inner_path().to_string()))
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

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        if dst.in_container() {
            return Err(read_only(dst));
        }
        if !src.in_container() {
            return self.inner.copy_file(src, dst, buffer, on_bytes);
        }
        if !dst.is_local() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                format!("{dst}: unpacking straight onto a remote filesystem is not supported yet"),
            ));
        }
        let (archive, inner) = self.archive(src)?;
        // The cancellation the engine signals by returning `false` becomes the token the
        // archive understands, so a stopped copy stops inside the decompressor too.
        let cancel = CancelToken::new();
        let mut written = 0;
        let result = archive.extract(&inner, dst.as_path(), &cancel, &mut |bytes| {
            written = bytes;
            if !on_bytes(bytes) {
                cancel.cancel();
            }
        });
        match result {
            Ok(bytes) => Ok(bytes),
            Err(ArcError::Cancelled) => {
                // A half-written file is worse than none: the engine's own copy does the same.
                let _ = std::fs::remove_file(dst.as_path());
                Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "cancelled".to_string(),
                ))
            }
            Err(other) => {
                let _ = std::fs::remove_file(dst.as_path());
                Err(into_io(other))
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
            fs.copy_file(&plain, &inside, 4096, &mut |_| true)
                .unwrap_err(),
        ] {
            assert_eq!(error.kind(), io::ErrorKind::Unsupported, "{error}");
            assert!(error.to_string().contains("archive"), "{error}");
        }
    }

    #[test]
    fn an_archive_inside_an_archive_says_it_cannot_yet() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let nested = VPath::local(&zip).enter("/inner.zip").enter("/a.txt");
        let error = fs().facts(&nested).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::Unsupported);
        assert!(error.to_string().contains("inside an archive"), "{error}");
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
}
