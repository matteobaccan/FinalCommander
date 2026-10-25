//! The job engine's filesystem for object storage.
//!
//! [`fcmd_jobs`] is synchronous — one job, one worker thread — while this provider is
//! asynchronous, so the calls are driven with [`tokio::runtime::Handle::block_on`]. A job must
//! therefore run on a blocking thread (`spawn_blocking`) and never inside an async task, which
//! is the same rule the SFTP route already works under.
//!
//! The shape is [`fcmd_jobs::ArcFs`]'s: wrap another [`JobFs`], take the paths that are ours,
//! and hand everything else straight through. That is what lets `F5`, `F6`, `F7` and `F8` work
//! on a bucket without the engine knowing object storage exists — the copy dialog, the
//! progress bar, the overwrite question and `Esc` are the ones that were already there.
//!
//! **What object storage is not.** There are no directories: `a/b/c.txt` is one key, and the
//! `a/` a panel shows is read out of the keys. There are no timestamps to set, no permissions,
//! no links, and no partial writes — an object either exists with the bytes it was given or
//! does not exist. A rename is a copy and a delete, which the service does itself; it is not
//! atomic, and a failure between the two leaves both keys, which is the honest behaviour to
//! report rather than to hide.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fcmd_core::{CancelToken, EntryKind, Scheme, VPath};
use fcmd_jobs::fs::{Facts, JobFs};
use fcmd_jobs::Preserve;
use tokio::runtime::Handle;

use crate::error::S3Error;
use crate::vfs::S3Vfs;

/// `s3://` paths go to the object store; everything else goes to `inner`.
pub struct ObjectFs<F> {
    objects: Arc<S3Vfs>,
    inner: F,
    handle: Handle,
}

impl<F> std::fmt::Debug for ObjectFs<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObjectFs").finish_non_exhaustive()
    }
}

impl<F: JobFs> ObjectFs<F> {
    /// `handle` is the runtime the requests run on.
    pub fn new(objects: Arc<S3Vfs>, inner: F, handle: Handle) -> ObjectFs<F> {
        ObjectFs {
            objects,
            inner,
            handle,
        }
    }

    pub fn inner(&self) -> &F {
        &self.inner
    }

    /// Run a request to completion from this blocking thread.
    fn run<T>(
        &self,
        future: impl std::future::Future<Output = crate::error::Result<T>>,
    ) -> io::Result<T> {
        self.handle.block_on(future).map_err(into_io)
    }
}

/// Is this a path this filesystem owns? A path inside a container is the archive layer's
/// whatever holds the archive, so it is not ours even when an `s3://` URL is in it.
fn mine(path: &VPath) -> bool {
    path.scheme() == Scheme::S3 && !path.in_container()
}

fn into_io(error: S3Error) -> io::Error {
    let kind = match &error {
        S3Error::NotFound(_) => io::ErrorKind::NotFound,
        S3Error::Cancelled => io::ErrorKind::Interrupted,
        S3Error::NoCredentials(_) => io::ErrorKind::PermissionDenied,
        S3Error::Service { status, .. } if *status == 403 => io::ErrorKind::PermissionDenied,
        S3Error::Service { status, .. } if *status == 404 => io::ErrorKind::NotFound,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, error.to_string())
}

fn unsupported(detail: String) -> io::Error {
    io::Error::new(io::ErrorKind::Unsupported, detail)
}

/// A temporary file for the bytes of a copy that has to pass through here, named after the
/// object so a failure leaves something a reader can recognise.
fn staging(path: &VPath) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("fcmd-{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let name = path.display_name();
    let name = if name.is_empty() { "object" } else { &name };
    dir.join(format!("{name}.staged"))
}

impl<F: JobFs> JobFs for ObjectFs<F> {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        if !mine(path) {
            return self.inner.facts(path);
        }
        let objects = self.objects.clone();
        let writable = self.objects.writable(path);
        let here = path.clone();
        let entry =
            self.run(async move { objects.stat_object(&here, &CancelToken::new()).await })?;
        Ok(Facts {
            kind: entry.kind,
            size: entry.size,
            mtime: entry.mtime,
            // No mode to read and none to set: a key's access is the bucket's policy, which
            // is not a number of permission bits and is not this program's to change.
            mode: None,
            readonly: !writable,
        })
    }

    /// There are no links here, so following one is the same as not following one.
    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        if !mine(path) {
            return self.inner.facts_following(path);
        }
        self.facts(path)
    }

    fn exists(&self, path: &VPath) -> bool {
        if !mine(path) {
            return self.inner.exists(path);
        }
        self.facts(path).is_ok()
    }

    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
        if !mine(path) {
            return self.inner.read_dir(path);
        }
        let objects = self.objects.clone();
        let here = path.clone();
        let writable = self.objects.writable(path);
        let entries = self
            .handle
            .block_on(async move { objects.list_objects(&here, &CancelToken::new()).await })
            .map_err(into_io)?;
        Ok(entries
            .into_iter()
            .filter(|e| e.kind != EntryKind::Parent)
            .map(|e| {
                let facts = Facts {
                    kind: e.kind,
                    size: e.size,
                    mtime: e.mtime,
                    mode: None,
                    readonly: !writable,
                };
                (e.name.clone(), facts)
            })
            .collect())
    }

    /// `F7`. There is no directory to create, so what is written is the empty object named
    /// with a trailing slash that every console uses to show a prefix with nothing in it.
    fn create_dir(&self, path: &VPath) -> io::Result<()> {
        if !mine(path) {
            return self.inner.create_dir(path);
        }
        let objects = self.objects.clone();
        let here = path.clone();
        self.run(async move { objects.make_prefix(&here, &CancelToken::new()).await })
    }

    /// The same thing: a key names its whole path, so there are no intermediate directories
    /// to make on the way to it.
    fn create_dir_all(&self, path: &VPath) -> io::Result<()> {
        if !mine(path) {
            return self.inner.create_dir_all(path);
        }
        self.create_dir(path)
    }

    /// A copy inside the service and then a delete, which is what a rename is here.
    ///
    /// It is not atomic and cannot be: a failure between the two leaves both keys. The engine
    /// is told so by [`fcmd_vfs::Caps`] — `RENAME` without `ATOMIC_RENAME` — and a caller
    /// that needs the guarantee has to do without.
    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()> {
        if !mine(from) && !mine(to) {
            return self.inner.rename(from, to);
        }
        if !mine(from) || !mine(to) {
            // The engine falls back to copy-then-delete, which is what crossing from object
            // storage to a filesystem is.
            return Err(unsupported(format!(
                "a rename cannot cross from {from} to {to}"
            )));
        }
        let objects = self.objects.clone();
        let (from, to) = (from.clone(), to.clone());
        self.run(async move {
            let cancel = CancelToken::new();
            objects.copy_object(&from, &to, &cancel).await?;
            objects.delete_object(&from, &cancel).await
        })
    }

    fn remove_file(&self, path: &VPath) -> io::Result<()> {
        if !mine(path) {
            return self.inner.remove_file(path);
        }
        let objects = self.objects.clone();
        let here = path.clone();
        self.run(async move { objects.delete_object(&here, &CancelToken::new()).await })
    }

    /// A prefix is not a thing, so there is nothing to remove but the marker object that made
    /// it visible — and the engine has already deleted the keys under it by the time it asks.
    /// A marker that was never written is not an error: the prefix is gone either way.
    fn remove_dir(&self, path: &VPath) -> io::Result<()> {
        if !mine(path) {
            return self.inner.remove_dir(path);
        }
        let objects = self.objects.clone();
        // The marker's name is the prefix with its slash: `notes` and `notes/` are the same
        // place to a panel and two different keys to the service.
        let marker = VPath::parse(&format!("{}/", path.to_string().trim_end_matches('/')));
        self.run(async move { objects.delete_object(&marker, &CancelToken::new()).await })
    }

    fn read_link(&self, path: &VPath) -> io::Result<PathBuf> {
        if !mine(path) {
            return self.inner.read_link(path);
        }
        Err(unsupported(format!("{path}: object storage has no links")))
    }

    fn symlink(&self, target: &Path, link: &VPath) -> io::Result<()> {
        if !mine(link) {
            return self.inner.symlink(target, link);
        }
        Err(unsupported(format!("{link}: object storage has no links")))
    }

    fn same_file(&self, a: &VPath, b: &VPath) -> bool {
        if !mine(a) && !mine(b) {
            return self.inner.same_file(a, b);
        }
        // Without an inode, the same object is the same service, bucket and key.
        mine(a) && mine(b) && a.normalised() == b.normalised()
    }

    /// Nothing to apply. An object carries the time the service wrote it and no other, there
    /// are no permission bits, and a copy that refused on that account would refuse every
    /// copy — so the times a filesystem would have kept are simply not kept, which is what
    /// every other client of these services also does.
    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()> {
        if !mine(path) {
            return self.inner.apply_meta(path, facts, preserve);
        }
        Ok(())
    }

    /// There is nothing here to set. An object carries the time the service wrote it, and a
    /// bucket's access is its policy — not a mode, and not this program's to change — so a
    /// request to change either is refused rather than silently doing nothing.
    fn set_attrs(&self, path: &VPath, change: &fcmd_jobs::attrs::Change) -> io::Result<()> {
        if !mine(path) {
            return self.inner.set_attrs(path, change);
        }
        Err(unsupported(format!(
            "{path}: an object has no permissions or times to set — its access is the bucket's policy"
        )))
    }

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        match (mine(src), mine(dst)) {
            (false, false) => self.inner.copy_file(src, dst, buffer, on_bytes),

            // Upload. One `PUT` is one operation to the service: it arrives whole or leaves
            // nothing, so the progress it reports is nothing and then all of it.
            (false, true) if src.is_local() => {
                let objects = self.objects.clone();
                let (source, to) = (src.as_path().to_path_buf(), dst.clone());
                let written = self.run(async move {
                    objects
                        .write_object(&to, &source, &CancelToken::new())
                        .await
                })?;
                on_bytes(written);
                Ok(written)
            }

            // Download, a chunk at a time, so a large object is not held here.
            (true, false) if dst.is_local() => {
                let objects = self.objects.clone();
                let (from, target) = (src.clone(), dst.as_path().to_path_buf());
                self.handle
                    .block_on(async move {
                        objects
                            .fetch_object(&from, &target, on_bytes, &CancelToken::new())
                            .await
                    })
                    .map_err(into_io)
            }

            // Inside one service, the service does it and the bytes stay there.
            (true, true) => {
                let objects = self.objects.clone();
                let (from, to) = (src.clone(), dst.clone());
                let size = self.facts(src)?.size;
                let same_service = self
                    .run(async move { objects.copy_object(&from, &to, &CancelToken::new()).await });
                match same_service {
                    Ok(()) => {
                        on_bytes(size);
                        Ok(size)
                    }
                    // Two services: the bytes have to come through here after all.
                    Err(_) => self.through_staging(src, dst, buffer, on_bytes),
                }
            }

            // One end is object storage and the other is a server: neither can talk to the
            // other, so the bytes land here on the way past.
            _ => self.through_staging(src, dst, buffer, on_bytes),
        }
    }
}

impl<F: JobFs> ObjectFs<F> {
    /// Copy by way of a local file: download, hand the file to whoever owns the destination,
    /// and take the staging copy away afterwards however it went.
    fn through_staging(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        let staged = staging(src);
        let local = VPath::local(&staged);
        let result = self
            .copy_file(src, &local, buffer, on_bytes)
            .and_then(|_| self.copy_file(&local, dst, buffer, on_bytes));
        let _ = std::fs::remove_file(&staged);
        result
    }
}

#[cfg(all(test, feature = "fake"))]
mod tests {
    use super::*;
    use crate::fake::FakeS3;
    use crate::vfs::Service;
    use fcmd_jobs::fs::LocalFs;
    use std::collections::BTreeMap;

    /// A runtime, a fake service with one bucket, and the job filesystem over both.
    async fn setup() -> (FakeS3, ObjectFs<LocalFs>) {
        let mut buckets = BTreeMap::new();
        let mut logs = BTreeMap::new();
        logs.insert("notes/one.txt".to_string(), b"first".to_vec());
        logs.insert("notes/two.txt".to_string(), b"second".to_vec());
        buckets.insert("logs".to_string(), logs);
        buckets.insert("spare".to_string(), BTreeMap::new());
        let fake = FakeS3::start(buckets).await;
        let vfs = S3Vfs::new(vec![
            Service::new("", fake.endpoint()).with_credentials(Some(fake.credentials.clone()))
        ]);
        let fs = ObjectFs::new(Arc::new(vfs), LocalFs, Handle::current());
        (fake, fs)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn a_bucket_looks_like_a_filesystem_to_the_engine() {
        let (_fake, fs) = setup().await;
        let fs = Arc::new(fs);
        let worker = fs.clone();
        tokio::task::spawn_blocking(move || {
            let fs = &*worker;
            let prefix = VPath::parse("s3://logs/notes/");
            let one = VPath::parse("s3://logs/notes/one.txt");

            // A key is a file with a size, and a prefix is a directory.
            let facts = fs.facts(&one).expect("the object is there");
            assert_eq!(facts.kind, EntryKind::File);
            assert_eq!(facts.size, 5);
            assert!(!facts.readonly, "a service with credentials can be written");
            assert!(fs.facts(&prefix).expect("the prefix is there").is_dir());
            assert!(fs.exists(&one));
            assert!(!fs.exists(&VPath::parse("s3://logs/notes/nope.txt")));

            // And the directory lists, without the `..` row the panel wants and a job does not.
            let mut names: Vec<String> = fs
                .read_dir(&prefix)
                .expect("it lists")
                .into_iter()
                .map(|(name, _)| name.to_string_lossy().into_owned())
                .collect();
            names.sort();
            assert_eq!(names, vec!["one.txt", "two.txt"]);
        })
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn the_engine_can_write_delete_and_rename_objects() {
        let (_fake, fs) = setup().await;
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("upload.txt");
        std::fs::write(&file, b"from disk").unwrap();
        let fs = Arc::new(fs);
        let worker = fs.clone();
        let dir_path = dir.path().to_path_buf();
        tokio::task::spawn_blocking(move || {
            let fs = &*worker;
            let seen = std::cell::Cell::new(0u64);
            let mut on_bytes = |n: u64| {
                seen.set(n);
                true
            };

            // Upload: a local file becomes a key.
            let up = VPath::parse("s3://logs/notes/up.txt");
            let written = fs
                .copy_file(&VPath::local(&file), &up, 64 * 1024, &mut on_bytes)
                .expect("the upload works");
            assert_eq!(written, 9);
            assert_eq!(seen.get(), 9, "the progress reported what went up");
            assert_eq!(fs.facts(&up).expect("it is there").size, 9);

            // Download: the key becomes a local file again, and the bytes match.
            let back = dir_path.join("back.txt");
            let read = fs
                .copy_file(&up, &VPath::local(&back), 64 * 1024, &mut on_bytes)
                .expect("the download works");
            assert_eq!(read, 9);
            assert_eq!(std::fs::read(&back).unwrap(), b"from disk");

            // Rename inside the service: the new key is there and the old one is not.
            let moved = VPath::parse("s3://logs/notes/moved.txt");
            fs.rename(&up, &moved).expect("the rename works");
            assert!(fs.exists(&moved));
            assert!(!fs.exists(&up));

            // Copy inside the service: the bytes never come through here.
            let beside = VPath::parse("s3://logs/notes/beside.txt");
            fs.copy_file(&moved, &beside, 64 * 1024, &mut on_bytes)
                .expect("the copy works");
            assert!(fs.exists(&beside));

            // Delete.
            fs.remove_file(&beside).expect("the delete works");
            assert!(!fs.exists(&beside));

            // `F7`: a prefix with nothing in it, which lists as a directory.
            let made = VPath::parse("s3://logs/fresh");
            fs.create_dir(&made).expect("the prefix is made");
            assert!(fs.facts(&made).expect("it is there").is_dir());
            fs.remove_dir(&made).expect("and it goes away again");
        })
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn what_object_storage_cannot_do_it_says_rather_than_pretends() {
        let (_fake, fs) = setup().await;
        let fs = Arc::new(fs);
        let worker = fs.clone();
        tokio::task::spawn_blocking(move || {
            let fs = &*worker;
            let key = VPath::parse("s3://logs/notes/one.txt");

            // No links, either way round.
            assert_eq!(
                fs.read_link(&key).unwrap_err().kind(),
                io::ErrorKind::Unsupported
            );
            assert_eq!(
                fs.symlink(Path::new("/tmp/x"), &key).unwrap_err().kind(),
                io::ErrorKind::Unsupported
            );

            // A rename that crosses out of object storage is refused, so the engine falls
            // back to a copy and a delete — which is what such a move is.
            let local = VPath::local("/tmp/fcmd-not-written");
            assert_eq!(
                fs.rename(&key, &local).unwrap_err().kind(),
                io::ErrorKind::Unsupported
            );

            // And a key that is not there reads as not found rather than as some other error,
            // because the engine's overwrite question depends on telling those apart.
            assert_eq!(
                fs.facts(&VPath::parse("s3://logs/nope"))
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::NotFound
            );
        })
        .await
        .unwrap();
    }
}
