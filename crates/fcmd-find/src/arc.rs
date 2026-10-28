//! Searching inside archives (SPEC 06 §1b).
//!
//! The walk next door knows nothing about containers: it asks a [`FindFs`] for a listing, for
//! the bytes of a file, and — when the search is told to look inside archives — whether a file
//! is something it may step into. This is the `impl` that says yes to a `.zip`.
//!
//! What it is not is a second walker. `x.zip` becomes `x.zip!/`, which is a directory to
//! everything above, so the depth limit, the loop guard, the mask, the size condition and the
//! content scanner all work in there exactly as they do on the disk — and a hit comes back as
//! `/home/mb/src.zip!/doc/readme.md`, which `Enter`, `F3` and *panelize* already understand.
//!
//! **The bytes of an entry are read whole.** A container is not a file one can seek about in
//! per entry: a tar is a stream and a 7z block holds several files in one. So a content search
//! inside an archive holds one entry in memory at a time, and refuses an entry larger than
//! [`BIGGEST`] rather than filling the machine — the archive itself is still searched, and the
//! entry is reported as a problem, which is the honest answer to "I could not look in there".

use std::io::{self, Cursor, Read};
use std::sync::Arc;

use fcmd_arc::{Cache, Format};
use fcmd_core::{CancelToken, Entry, Error, Result, VPath};

use crate::fs::{FindFs, VisitId};

/// The largest entry a content search will read out of an archive. Four files of this size
/// never exist at once — the walk reads one at a time — and a file bigger than this inside an
/// archive is a disk image or a database, which a search of a whole drive meets twice a year.
pub const BIGGEST: u64 = 64 << 20;

/// A search filesystem that can also step into archives.
///
/// It wraps another one — the local filesystem, today — and takes only the paths that point
/// inside a container, handing everything else straight on, the way the job engine's `ArcFs`
/// does. The cache is the program's own, so an archive a panel has already read is not read
/// again to search it.
pub struct ArcFind<F: FindFs> {
    cache: Arc<Cache>,
    inner: F,
}

impl<F: FindFs> ArcFind<F> {
    pub fn new(cache: Arc<Cache>, inner: F) -> ArcFind<F> {
        ArcFind { cache, inner }
    }

    /// The filesystem underneath, for the paths that are not in a container.
    pub fn inner(&self) -> &F {
        &self.inner
    }
}

impl<F: FindFs> FindFs for ArcFind<F> {
    fn list(&self, dir: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>> {
        if !dir.in_container() {
            return self.inner.list(dir, cancel);
        }
        let (archive, inner) = self
            .cache
            .open_in(dir, cancel)
            .map_err(|e| Error::Other(e.to_string()))?;
        if !archive.is_dir(&inner) {
            return Err(Error::NotADirectory { path: dir.clone() });
        }
        Ok(archive.list(&inner))
    }

    fn open(&self, path: &VPath) -> io::Result<Box<dyn Read + Send>> {
        if !path.in_container() {
            return self.inner.open(path);
        }
        let cancel = CancelToken::new();
        let (archive, inner) = self
            .cache
            .open_in(path, &cancel)
            .map_err(|e| io::Error::other(e.to_string()))?;
        if let Some(entry) = archive.entry(&inner) {
            if entry.size > BIGGEST {
                return Err(io::Error::other(format!(
                    "{} is {} MB, too large to search inside an archive",
                    path,
                    entry.size >> 20
                )));
            }
        }
        let bytes = archive
            .read(&inner, &cancel)
            .map_err(|e| io::Error::other(e.to_string()))?;
        Ok(Box::new(Cursor::new(bytes)))
    }

    fn visit_id(&self, dir: &VPath) -> Option<VisitId> {
        if dir.in_container() {
            // Inside a container the path is the identity: there are no links to go round in
            // a circle with, and the archive's own name is already unique.
            return None;
        }
        self.inner.visit_id(dir)
    }

    fn is_dir_following(&self, path: &VPath) -> bool {
        !path.in_container() && self.inner.is_dir_following(path)
    }

    fn opens_as_dir(&self, path: &VPath) -> bool {
        // By name, as `Enter` decides it: opening every file of a tree to look at its first
        // bytes is not something a search of a whole disk may do.
        Format::of_name(&path.display_name()).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{find, Content, Event, Needle, Query};
    use fcmd_core::{Filter, MaskSet};
    use fcmd_vfs::LocalVfs;
    use std::io::Write;

    fn zip_at(path: &std::path::Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, body) in [
            ("doc/readme.md", &b"the needle is in here\n"[..]),
            ("src/main.rs", &b"fn main() {}\n"[..]),
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    fn hits(query: &Query, fs: &dyn FindFs) -> Vec<String> {
        let mut found = Vec::new();
        find(fs, query, &CancelToken::new(), &mut |event| {
            if let Event::Hit(hit) = event {
                found.push(hit.path.to_string());
            }
        })
        .unwrap();
        found.sort();
        found
    }

    #[test]
    fn a_search_steps_into_an_archive_when_it_is_asked_to() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("outside.md"), b"nothing here\n").unwrap();
        zip_at(&dir.path().join("tree.zip"));
        let fs = ArcFind::new(Arc::new(Cache::new(Default::default())), LocalVfs);

        let query = Query::under(VPath::local(dir.path()))
            .with_filter(Filter::from_masks(MaskSet::parse("*.md").unwrap()));
        // Without the box, the archive is one file and nothing is looked at inside it.
        assert_eq!(
            hits(&query, &fs),
            ["outside.md"].map(|n| format!("{}", VPath::local(dir.path().join(n))))
        );

        let deep = Query {
            in_archives: true,
            ..query.clone()
        };
        let found = hits(&deep, &fs);
        assert_eq!(found.len(), 2, "{found:?}");
        assert!(
            found.iter().any(|p| p.ends_with("tree.zip!/doc/readme.md")),
            "{found:?}"
        );
    }

    #[test]
    fn what_is_inside_an_archive_is_searched_for_its_text_too() {
        let dir = tempfile::tempdir().unwrap();
        zip_at(&dir.path().join("tree.zip"));
        let fs = ArcFind::new(Arc::new(Cache::new(Default::default())), LocalVfs);

        let query = Query {
            in_archives: true,
            ..Query::under(VPath::local(dir.path()))
                .containing(Content::containing(Needle::text("the needle")))
        };
        let found = hits(&query, &fs);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].ends_with("tree.zip!/doc/readme.md"), "{found:?}");
    }

    /// A file that is not an archive is not stepped into, whatever the box says, and a
    /// container that cannot be read is a problem rather than the end of the search.
    #[test]
    fn a_file_that_only_looks_like_an_archive_is_a_problem_and_not_a_stop() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("lying.zip"), b"not a zip at all").unwrap();
        std::fs::write(dir.path().join("plain.txt"), b"text\n").unwrap();
        let fs = ArcFind::new(Arc::new(Cache::new(Default::default())), LocalVfs);
        assert!(!fs.opens_as_dir(&VPath::local(dir.path().join("plain.txt"))));

        let query = Query {
            in_archives: true,
            ..Query::under(VPath::local(dir.path()))
        };
        let mut problems = 0;
        let stats = find(&fs, &query, &CancelToken::new(), &mut |event| {
            if let Event::Problem(_) = event {
                problems += 1;
            }
        })
        .unwrap();
        assert_eq!(problems, 1, "the archive that is not one says so");
        assert_eq!(stats.hits, 2, "both files are still results");
    }
}
