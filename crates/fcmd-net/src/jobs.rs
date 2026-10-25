//! The job engine's filesystem, routed per path.
//!
//! [`fcmd_jobs`] is synchronous by design — one job, one worker thread, no executor in the
//! middle — while a remote provider is asynchronous. This is where the two meet: the remote
//! calls are driven with [`tokio::runtime::Handle::block_on`], which is why a job must run on
//! a blocking thread (`spawn_blocking`) and never inside an async task.

use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fcmd_core::VPath;
use fcmd_jobs::fs::{Facts, JobFs, LocalFs};
use fcmd_jobs::Preserve;
use tokio::runtime::Handle;

use crate::error::NetError;
use crate::pool::Pool;
use crate::remote::{copy_between, Remote};

/// Local paths go to [`LocalFs`]; remote paths go to their connection.
#[derive(Debug, Clone)]
pub struct RoutedFs {
    local: LocalFs,
    pool: Arc<Pool>,
    handle: Handle,
}

impl RoutedFs {
    /// `handle` is the runtime the connections live on.
    pub fn new(pool: Arc<Pool>, handle: Handle) -> RoutedFs {
        RoutedFs {
            local: LocalFs,
            pool,
            handle,
        }
    }

    fn remote(&self, path: &VPath) -> io::Result<Remote> {
        self.pool.require(path).map_err(NetError::into_io)
    }

    /// Run a remote call to completion from this blocking thread.
    fn run<T>(&self, future: impl std::future::Future<Output = crate::Result<T>>) -> io::Result<T> {
        self.handle.block_on(future).map_err(NetError::into_io)
    }
}

impl JobFs for RoutedFs {
    fn facts(&self, path: &VPath) -> io::Result<Facts> {
        if path.is_local() {
            return self.local.facts(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.facts(path, false).await })
    }

    fn facts_following(&self, path: &VPath) -> io::Result<Facts> {
        if path.is_local() {
            return self.local.facts_following(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.facts(path, true).await })
    }

    fn exists(&self, path: &VPath) -> bool {
        if path.is_local() {
            return self.local.exists(path);
        }
        match self.remote(path) {
            Ok(remote) => self.handle.block_on(remote.exists(path)),
            Err(_) => false,
        }
    }

    fn read_dir(&self, path: &VPath) -> io::Result<Vec<(OsString, Facts)>> {
        if path.is_local() {
            return self.local.read_dir(path);
        }
        let remote = self.remote(path)?;
        let entries = self.run(async move { remote.read_dir_facts(path).await })?;
        Ok(entries
            .into_iter()
            .map(|(name, facts)| (OsString::from(name), facts))
            .collect())
    }

    fn create_dir(&self, path: &VPath) -> io::Result<()> {
        if path.is_local() {
            return self.local.create_dir(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.create_dir(path).await })
    }

    fn create_dir_all(&self, path: &VPath) -> io::Result<()> {
        if path.is_local() {
            return self.local.create_dir_all(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.create_dir_all(path).await })
    }

    fn rename(&self, from: &VPath, to: &VPath) -> io::Result<()> {
        if from.is_local() && to.is_local() {
            return self.local.rename(from, to);
        }
        if !from.same_provider(to) {
            // The engine falls back to copy-then-delete, which is what a cross-host move is.
            return Err(NetError::Unsupported {
                path: to.to_string(),
                detail: format!("a rename cannot cross from {from} to another filesystem"),
            }
            .into_io());
        }
        let remote = self.remote(from)?;
        self.run(async move { remote.rename(from, to).await })
    }

    fn remove_file(&self, path: &VPath) -> io::Result<()> {
        if path.is_local() {
            return self.local.remove_file(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.remove_file(path).await })
    }

    fn remove_dir(&self, path: &VPath) -> io::Result<()> {
        if path.is_local() {
            return self.local.remove_dir(path);
        }
        let remote = self.remote(path)?;
        self.run(async move { remote.remove_dir(path).await })
    }

    fn read_link(&self, path: &VPath) -> io::Result<PathBuf> {
        if path.is_local() {
            return self.local.read_link(path);
        }
        let remote = self.remote(path)?;
        let target = self.run(async move { remote.read_link(path).await })?;
        Ok(PathBuf::from(target))
    }

    fn symlink(&self, target: &Path, link: &VPath) -> io::Result<()> {
        if link.is_local() {
            return self.local.symlink(target, link);
        }
        let remote = self.remote(link)?;
        let target = target.to_string_lossy().replace('\\', "/");
        self.run(async move { remote.symlink(&target, link).await })
    }

    fn same_file(&self, a: &VPath, b: &VPath) -> bool {
        if a.is_local() && b.is_local() {
            return self.local.same_file(a, b);
        }
        // Without an inode, "the same file" is the same connection and the same path.
        a.same_provider(b) && a.normalised() == b.normalised()
    }

    fn apply_meta(&self, path: &VPath, facts: &Facts, preserve: Preserve) -> io::Result<()> {
        if path.is_local() {
            return self.local.apply_meta(path, facts, preserve);
        }
        if !preserve.mtime && !preserve.mode {
            return Ok(());
        }
        let remote = self.remote(path)?;
        let facts = facts.clone();
        self.run(async move { remote.apply_meta(path, &facts).await })
    }

    /// A file on a server has a mode and a modification time, and SFTP can set both. There
    /// is nothing there for Windows' four bits, so those rows are not offered for a remote
    /// path; read-only is, because it is the write bits.
    fn set_attrs(&self, path: &VPath, change: &fcmd_jobs::attrs::Change) -> io::Result<()> {
        if path.is_local() {
            return self.local.set_attrs(path, change);
        }
        let remote = self.remote(path)?;
        let mut mode = change.mode;
        if let Some(readonly) = change.readonly {
            let current = match mode {
                Some(mode) => mode,
                None => self.facts(path)?.mode.unwrap_or(0o644),
            };
            mode = Some(if readonly {
                current & !0o222
            } else {
                current | 0o200
            });
        }
        if mode.is_none() && change.mtime.is_none() {
            // Nothing SFTP can carry: the dialog asked for something a server does not have.
            return Ok(());
        }
        let facts = Facts {
            kind: fcmd_core::EntryKind::File,
            size: 0,
            mtime: change.mtime,
            mode,
            readonly: false,
        };
        self.run(async move { remote.apply_meta(path, &facts).await })
    }

    fn copy_file(
        &self,
        src: &VPath,
        dst: &VPath,
        buffer: usize,
        on_bytes: &mut dyn FnMut(u64) -> bool,
    ) -> io::Result<u64> {
        match (src.is_local(), dst.is_local()) {
            (true, true) => self.local.copy_file(src, dst, buffer, on_bytes),
            // Upload.
            (true, false) => {
                let remote = self.remote(dst)?;
                self.handle
                    .block_on(remote.upload(src.as_path(), dst, on_bytes))
                    .map_err(NetError::into_io)
            }
            // Download.
            (false, true) => {
                let remote = self.remote(src)?;
                self.handle
                    .block_on(remote.download(src, dst.as_path(), on_bytes))
                    .map_err(NetError::into_io)
            }
            // Server to server: one connection, or two.
            (false, false) => {
                let from = self.remote(src)?;
                if src.same_provider(dst) {
                    return self
                        .handle
                        .block_on(from.copy_within(src, dst, on_bytes))
                        .map_err(NetError::into_io);
                }
                let to = self.remote(dst)?;
                self.handle
                    .block_on(copy_between(&from, src, &to, dst, on_bytes))
                    .map_err(NetError::into_io)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryFs;
    use crate::site::Site;
    use fcmd_core::{CancelToken, EntryKind};
    use fcmd_jobs::{ConflictPolicy, JobKind, JobOptions, JobSpec, Reporter};
    use tokio::runtime::Runtime;

    struct Silent;
    impl Reporter for Silent {
        fn progress(&mut self, _p: &fcmd_jobs::Progress) {}
        fn ask(&mut self, _q: fcmd_jobs::Question) -> fcmd_jobs::Decision {
            fcmd_jobs::Decision::Overwrite
        }
    }

    /// A runtime, a pool with one fake server, and the routed filesystem over both.
    fn setup() -> (Runtime, Arc<Pool>, Arc<MemoryFs>, RoutedFs, Site) {
        let runtime = Runtime::new().unwrap();
        let pool = Pool::new();
        let site = Site::parse_url("sftp://mb@host:22/srv").unwrap();
        let fs = MemoryFs::new().with_dir("/srv");
        pool.insert(Remote::new(site.clone(), fs.clone()));
        let routed = RoutedFs::new(pool.clone(), runtime.handle().clone());
        (runtime, pool, fs, routed, site)
    }

    #[test]
    fn an_upload_and_a_download_go_through_the_job_engine() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/a.txt"), b"alpha").unwrap();
        std::fs::write(dir.path().join("src/b.bin"), vec![3u8; 100_000]).unwrap();
        let (_rt, _pool, server, routed, site) = setup();

        // Copy a local directory up to the server.
        let spec = JobSpec {
            kind: JobKind::Copy,
            sources: vec![VPath::local(dir.path().join("src"))],
            target: Some(site.start()),
            options: JobOptions::default(),
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(server.file("/srv/src/a.txt").unwrap(), b"alpha");
        assert_eq!(server.file("/srv/src/b.bin").unwrap().len(), 100_000);
        assert_eq!(outcome.files, 2);

        // And back down into another local directory.
        let down = dir.path().join("down");
        std::fs::create_dir(&down).unwrap();
        let spec = JobSpec {
            kind: JobKind::Copy,
            sources: vec![site.start().join("src")],
            target: Some(VPath::local(&down)),
            options: JobOptions::default(),
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(
            std::fs::read_to_string(down.join("src/a.txt")).unwrap(),
            "alpha"
        );
    }

    #[test]
    fn a_remote_move_is_a_rename_and_moves_no_bytes() {
        let (_rt, _pool, server, routed, site) = setup();
        server
            .with_file("/srv/one/a.txt", b"x")
            .with_dir("/srv/two");
        let spec = JobSpec {
            kind: JobKind::Move,
            sources: vec![site.start().join("one")],
            target: Some(site.start().join("two")),
            options: JobOptions::default(),
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.renamed, 1, "a same-host move must not copy bytes");
        assert!(server.paths().contains(&"/srv/two/one/a.txt".to_string()));
        assert!(!server.paths().contains(&"/srv/one".to_string()));
    }

    #[test]
    fn a_cross_filesystem_move_copies_then_deletes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gone.txt"), b"bye").unwrap();
        let (_rt, _pool, server, routed, site) = setup();
        let spec = JobSpec {
            kind: JobKind::Move,
            sources: vec![VPath::local(dir.path().join("gone.txt"))],
            target: Some(site.start()),
            options: JobOptions::default(),
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert_eq!(outcome.renamed, 0, "a rename cannot cross filesystems");
        assert_eq!(server.file("/srv/gone.txt").unwrap(), b"bye");
        assert!(
            !dir.path().join("gone.txt").exists(),
            "the source should be gone after a move"
        );
    }

    #[test]
    fn deleting_a_remote_tree_walks_it_in_post_order() {
        let (_rt, _pool, server, routed, site) = setup();
        server
            .with_file("/srv/tree/deep/a.txt", b"a")
            .with_file("/srv/tree/b.txt", b"b");
        let spec = JobSpec {
            kind: JobKind::Delete,
            sources: vec![site.start().join("tree")],
            target: None,
            options: JobOptions::default(),
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert!(outcome.errors.is_empty(), "{:?}", outcome.errors);
        assert!(!server.paths().iter().any(|p| p.starts_with("/srv/tree")));
    }

    #[test]
    fn facts_and_listings_cross_the_bridge_unchanged() {
        let (_rt, _pool, server, routed, site) = setup();
        server
            .with_file("/srv/a.txt", b"12345")
            .with_dir("/srv/sub");
        let facts = routed.facts(&site.start().join("a.txt")).unwrap();
        assert_eq!(facts.kind, EntryKind::File);
        assert_eq!(facts.size, 5);
        assert!(routed.exists(&site.start().join("sub")));
        assert!(!routed.exists(&site.start().join("nope")));
        let mut names: Vec<String> = routed
            .read_dir(&site.start())
            .unwrap()
            .into_iter()
            .map(|(n, _)| n.to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, ["a.txt", "sub"]);
    }

    #[test]
    fn the_server_s_refusal_reaches_the_job_engine_as_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("x.txt"), b"x").unwrap();
        let (_rt, _pool, server, routed, site) = setup();
        server.break_path("/srv/x.txt", "permission denied");
        let spec = JobSpec {
            kind: JobKind::Copy,
            sources: vec![VPath::local(dir.path().join("x.txt"))],
            target: Some(site.start()),
            options: JobOptions {
                on_conflict: ConflictPolicy::Overwrite,
                ..JobOptions::default()
            },
            replace: None,
        };
        let plan = fcmd_jobs::plan(&routed, &spec, &CancelToken::new()).unwrap();
        let outcome = execute_now(&routed, &spec, plan);
        assert_eq!(outcome.errors.len(), 1, "{outcome:?}");
        let (path, message) = &outcome.errors[0];
        assert!(path.to_string().contains("x.txt"), "{path}");
        assert!(message.contains("permission denied"), "{message}");
    }

    #[test]
    fn a_rename_across_providers_is_refused_with_both_paths() {
        let (_rt, _pool, _server, routed, site) = setup();
        let err = routed
            .rename(&VPath::local("/tmp/a"), &site.start().join("a"))
            .unwrap_err();
        let text = err.to_string();
        assert!(text.contains("/tmp/a"), "{text}");
        assert!(text.contains("mb@host:22"), "{text}");
    }

    /// `execute` is synchronous; the remote calls inside it block on the runtime handle, which
    /// is exactly how the application runs a job (on a blocking thread).
    fn execute_now(fs: &RoutedFs, spec: &JobSpec, plan: fcmd_jobs::Plan) -> fcmd_jobs::Outcome {
        fcmd_jobs::execute(fs, spec, plan, &mut Silent, &CancelToken::new())
    }
}
