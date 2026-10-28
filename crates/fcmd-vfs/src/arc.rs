use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use fcmd_arc::{ArcError, Cache, Format, Limits};
use fcmd_core::{CancelToken, Entry, Error, Result, VPath};

use crate::{Caps, Vfs};

/// The archive provider: a panel inside a `.zip` or a `.tar.gz`.
///
/// It owns the container cache, so stepping in and out of an archive reads its index once
/// (SPEC 04 §4). Paths are the `!` kind — `/home/mb/src.zip!/doc` — and everything else is
/// somebody else's: the router sends a path here when, and only when, it is in a container.
#[derive(Debug)]
pub struct ArcVfs {
    cache: Arc<Cache>,
}

impl ArcVfs {
    pub fn new(limits: Limits) -> ArcVfs {
        ArcVfs {
            cache: Arc::new(Cache::new(limits)),
        }
    }

    /// The cache, to share with the job engine: one index, read once, used by both.
    pub fn cache(&self) -> Arc<Cache> {
        self.cache.clone()
    }

    /// Could `path` be an archive we can open? By name, which is what deciding whether `Enter`
    /// goes into a file has to be: cheap, and wrong only for a file someone has renamed.
    pub fn looks_like_archive(path: &Path) -> bool {
        Format::of_path(path).is_some()
    }

    /// The archive `path` is inside, and where `path` sits in it.
    ///
    /// A container that is not a file on this disk — one inside another archive, one on a
    /// server — is copied here first by the cache and read from the copy (SPEC 04 §4). That is
    /// below us: from here an archive is an archive, wherever its bytes were.
    fn open_in(
        &self,
        path: &VPath,
        cancel: &CancelToken,
    ) -> Result<(std::sync::Arc<fcmd_arc::Archive>, String)> {
        if path.leave().is_none() {
            return Err(Error::NotADirectory { path: path.clone() });
        }
        self.cache
            .open_in(path, cancel)
            .map_err(|e| convert(path, e))
    }

    /// The synchronous core of [`ArcVfs::list`], kept public for the same reasons the local
    /// provider's is: tests, and the callers that are already on a blocking thread.
    pub fn list_blocking(&self, path: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>> {
        let (archive, inner) = self.open_in(path, cancel)?;
        let inner = inner.as_str();
        if !archive.is_dir(inner) {
            return Err(Error::NotADirectory { path: path.clone() });
        }
        let mut out = Vec::new();
        // An archive's root has a parent — the directory the archive sits in — so `..` is
        // always there, and it is how a user gets out.
        out.push(Entry::parent_entry());
        out.extend(archive.list(inner));
        Ok(out)
    }

    fn blocking_stat(&self, path: &VPath) -> Result<Entry> {
        let (archive, inner) = self.open_in(path, &CancelToken::new())?;
        if inner == "/" {
            // The root of the archive stands for the archive itself, under its own name.
            let mut entry = Entry::parent_entry();
            entry.name = path
                .leave()
                .map(|host| host.display_name())
                .unwrap_or_default()
                .into();
            entry.kind = fcmd_core::EntryKind::Dir;
            return Ok(entry);
        }
        archive
            .row(&inner)
            .ok_or_else(|| Error::Other(format!("{path}: no such entry")))
    }

    /// The bytes of one entry, for the viewer and the editor.
    pub fn read_blocking(&self, path: &VPath, cancel: &CancelToken) -> Result<Vec<u8>> {
        let (archive, inner) = self.open_in(path, cancel)?;
        archive.read(&inner, cancel).map_err(|e| convert(path, e))
    }
}

/// An archive error in the core's currency, keeping the message — which already names the
/// archive and, for a refusal, the limit that fired.
fn convert(path: &VPath, error: ArcError) -> Error {
    match error {
        ArcError::Cancelled => Error::Cancelled,
        ArcError::Io { source, .. } => Error::io(path, source),
        other => Error::Other(other.to_string()),
    }
}

#[async_trait]
impl Vfs for ArcVfs {
    fn scheme(&self) -> &'static str {
        // Not a scheme of its own: an archive is read through the provider that holds the
        // file, and the `!` in the path is what says to come here.
        "arc"
    }

    fn caps(&self) -> Caps {
        // Reading, and one entry at a time. Writing into a container is SPEC 04 §4's rebuild
        // and is not here yet, so nothing offers it.
        Caps::READ | Caps::SEEK
    }

    async fn list(&self, path: &VPath, cancel: CancelToken) -> Result<Vec<Entry>> {
        let owned = path.clone();
        let cache = self.cache.clone();
        tokio::task::spawn_blocking(move || ArcVfs { cache }.list_blocking(&owned, &cancel))
            .await
            .map_err(|e| Error::Other(format!("listing task failed: {e}")))?
    }

    async fn stat(&self, path: &VPath) -> Result<Entry> {
        let owned = path.clone();
        let cache = self.cache.clone();
        tokio::task::spawn_blocking(move || ArcVfs { cache }.blocking_stat(&owned))
            .await
            .map_err(|e| Error::Other(format!("stat task failed: {e}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_of(path: &Path) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        for (name, body) in [("readme.md", &b"# readme"[..]), ("src/a.txt", &b"aaaa"[..])] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(body).unwrap();
        }
        zip.finish().unwrap();
    }

    #[tokio::test]
    async fn a_panel_can_look_inside_an_archive() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let vfs = ArcVfs::new(Limits::default());
        let root = VPath::local(&zip).enter("/");

        let rows = vfs
            .list(&root, CancelToken::new())
            .await
            .expect("a listing");
        let names: Vec<String> = rows.iter().map(|e| e.name_str().into_owned()).collect();
        // `..` first, because the way out of an archive is the way out of any directory.
        assert_eq!(names, ["..", "readme.md", "src"]);
        assert_eq!(rows[0].kind, fcmd_core::EntryKind::Parent);

        let inside = vfs
            .list(&root.join("src"), CancelToken::new())
            .await
            .expect("a listing");
        assert_eq!(inside.len(), 2);
        assert_eq!(inside[1].name_str(), "a.txt");
        assert_eq!(inside[1].size, 4);

        // The root of an archive stats as the archive's own name.
        let stat = vfs.stat(&root).await.unwrap();
        assert_eq!(stat.name_str(), "tree.zip");
        assert!(stat.kind.is_dirlike());
        assert_eq!(vfs.stat(&root.join("readme.md")).await.unwrap().size, 8);

        // And the bytes come out for the viewer.
        assert_eq!(
            vfs.read_blocking(&root.join("src/a.txt"), &CancelToken::new())
                .unwrap(),
            b"aaaa"
        );
    }

    #[tokio::test]
    async fn a_file_inside_an_archive_is_not_a_directory_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let zip = dir.path().join("tree.zip");
        zip_of(&zip);
        let vfs = ArcVfs::new(Limits::default());
        let file = VPath::local(&zip).enter("/readme.md");
        assert!(matches!(
            vfs.list(&file, CancelToken::new()).await,
            Err(Error::NotADirectory { .. })
        ));

        // A refusal keeps the archive's own words, limit and all.
        let tight = ArcVfs::new(Limits {
            total_bytes: 1,
            ..Limits::default()
        });
        let error = tight
            .list(&VPath::local(&zip).enter("/"), CancelToken::new())
            .await
            .unwrap_err();
        assert!(error.to_string().contains("unpacks to"), "{error}");
    }

    #[tokio::test]
    async fn an_archive_inside_an_archive_opens_through_a_copy() {
        let dir = tempfile::tempdir().unwrap();
        let inner = dir.path().join("inner.zip");
        zip_of(&inner);
        // The outer one holds the inner archive, stored rather than deflated — which is what a
        // real one looks like, since a zip inside a zip does not compress.
        let outer = dir.path().join("outer.zip");
        let file = std::fs::File::create(&outer).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        zip.start_file(
            "nested/inner.zip",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )
        .unwrap();
        zip.write_all(&std::fs::read(&inner).unwrap()).unwrap();
        zip.finish().unwrap();

        let vfs = ArcVfs::new(Limits::default());
        let deep = VPath::local(&outer).enter("/nested/inner.zip").enter("/");
        let rows = vfs
            .list(&deep, CancelToken::new())
            .await
            .expect("a listing");
        let names: Vec<String> = rows.iter().map(|e| e.name_str().into_owned()).collect();
        assert_eq!(names, ["..", "readme.md", "src"]);
        // And the bytes of a file two containers down come out.
        assert_eq!(
            vfs.read_blocking(&deep.join("src/a.txt"), &CancelToken::new())
                .unwrap(),
            b"aaaa"
        );
        // The copy is made once: a second listing answers from the same file.
        assert!(vfs.list(&deep, CancelToken::new()).await.is_ok());

        // Deeper than the limit allows is refused by the limit, not by a failure to read.
        let shallow = ArcVfs::new(Limits {
            depth: 0,
            ..Limits::default()
        });
        let error = shallow.list(&deep, CancelToken::new()).await.unwrap_err();
        assert!(error.to_string().contains("max_depth"), "{error}");
    }

    #[tokio::test]
    async fn an_archive_nothing_can_bring_here_says_so() {
        let vfs = ArcVfs::new(Limits::default());
        let remote = VPath::remote(fcmd_core::Scheme::Sftp, "host", "/srv/x.zip").enter("/");
        let error = vfs.list(&remote, CancelToken::new()).await.unwrap_err();
        assert!(error.to_string().contains("copied here first"), "{error}");
    }

    #[test]
    fn what_looks_like_an_archive_is_decided_by_name() {
        assert!(ArcVfs::looks_like_archive(Path::new("/a/src.tar.gz")));
        assert!(ArcVfs::looks_like_archive(Path::new("/a/app.jar")));
        assert!(!ArcVfs::looks_like_archive(Path::new("/a/notes.txt")));
    }
}
