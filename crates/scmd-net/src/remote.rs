use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use scmd_core::{CancelToken, Entry, EntryKind, VPath};
use scmd_jobs::Facts;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::error::{NetError, Result};
use crate::ops::{RemoteEntry, Sftp};
use crate::site::Site;

/// Bytes moved per read/write round trip. SFTP likes large reads; 256 KiB keeps the window
/// full without holding much memory per job.
pub const CHUNK: usize = 256 * 1024;

/// What an incomplete upload is called on the server, matching the job engine's local rule.
pub const PART_SUFFIX: &str = scmd_jobs::PART_SUFFIX;

/// One live connection, addressed with [`VPath`]s instead of wire strings.
///
/// Every operation checks that the path belongs to this connection, so a `file://` path can
/// never reach a server by accident.
#[derive(Debug, Clone)]
pub struct Remote {
    site: Site,
    sftp: Arc<dyn Sftp>,
}

impl Remote {
    pub fn new(site: Site, sftp: Arc<dyn Sftp>) -> Remote {
        Remote { site, sftp }
    }

    pub fn site(&self) -> &Site {
        &self.site
    }

    pub fn authority(&self) -> String {
        self.site.authority()
    }

    pub fn sftp(&self) -> &Arc<dyn Sftp> {
        &self.sftp
    }

    pub fn owns(&self, path: &VPath) -> bool {
        self.site.owns(path)
    }

    /// The wire form of a path on this connection.
    pub fn wire(&self, path: &VPath) -> Result<String> {
        if !self.owns(path) {
            return Err(NetError::NotRemote(path.to_string()));
        }
        Ok(path.remote_path())
    }

    pub async fn close(&self) -> Result<()> {
        self.sftp.close().await
    }

    pub async fn canonicalize(&self, path: &VPath) -> Result<VPath> {
        let wire = self.wire(path)?;
        let resolved = self.sftp.canonicalize(&wire).await?;
        Ok(VPath::remote(self.site.scheme, self.authority(), &resolved))
    }

    /// A panel listing, with the `..` entry the panel expects.
    pub async fn list(&self, path: &VPath, cancel: &CancelToken) -> Result<Vec<Entry>> {
        let wire = self.wire(path)?;
        let entries = self.sftp.read_dir(&wire).await?;
        if cancel.is_cancelled() {
            return Err(NetError::Cancelled);
        }
        let mut out = Vec::with_capacity(entries.len() + 1);
        if !path.is_root() {
            out.push(Entry::parent_entry());
        }
        for entry in entries {
            if entry.name == "." || entry.name == ".." {
                continue;
            }
            out.push(entry.to_entry());
        }
        Ok(out)
    }

    pub async fn stat(&self, path: &VPath) -> Result<Entry> {
        let wire = self.wire(path)?;
        Ok(self.sftp.stat(&wire, false).await?.to_entry())
    }

    pub async fn facts(&self, path: &VPath, follow: bool) -> Result<Facts> {
        let wire = self.wire(path)?;
        Ok(self.sftp.stat(&wire, follow).await?.to_facts())
    }

    pub async fn exists(&self, path: &VPath) -> bool {
        match self.wire(path) {
            Ok(wire) => self.sftp.exists(&wire).await,
            Err(_) => false,
        }
    }

    pub async fn read_dir_facts(&self, path: &VPath) -> Result<Vec<(String, Facts)>> {
        let wire = self.wire(path)?;
        Ok(self
            .sftp
            .read_dir(&wire)
            .await?
            .into_iter()
            .filter(|e| e.name != "." && e.name != "..")
            .map(|e| (e.name.clone(), e.to_facts()))
            .collect())
    }

    pub async fn create_dir(&self, path: &VPath) -> Result<()> {
        let wire = self.wire(path)?;
        self.sftp.create_dir(&wire).await
    }

    /// `mkdir -p`, which SFTP has no single call for.
    pub async fn create_dir_all(&self, path: &VPath) -> Result<()> {
        let wire = self.wire(path)?;
        let mut so_far = String::new();
        for part in wire.split('/').filter(|p| !p.is_empty()) {
            so_far.push('/');
            so_far.push_str(part);
            if self.sftp.exists(&so_far).await {
                continue;
            }
            // A racing creation by someone else is not our problem to report.
            if let Err(e) = self.sftp.create_dir(&so_far).await {
                if !self.sftp.exists(&so_far).await {
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    pub async fn remove_file(&self, path: &VPath) -> Result<()> {
        let wire = self.wire(path)?;
        self.sftp.remove_file(&wire).await
    }

    pub async fn remove_dir(&self, path: &VPath) -> Result<()> {
        let wire = self.wire(path)?;
        self.sftp.remove_dir(&wire).await
    }

    pub async fn rename(&self, from: &VPath, to: &VPath) -> Result<()> {
        let (from, to) = (self.wire(from)?, self.wire(to)?);
        self.sftp.rename(&from, &to).await
    }

    pub async fn read_link(&self, path: &VPath) -> Result<String> {
        let wire = self.wire(path)?;
        self.sftp.read_link(&wire).await
    }

    pub async fn symlink(&self, target: &str, link: &VPath) -> Result<()> {
        let wire = self.wire(link)?;
        self.sftp.symlink(target, &wire).await
    }

    /// Carry over what SFTP can carry: the mode and the modification time.
    pub async fn apply_meta(&self, path: &VPath, facts: &Facts) -> Result<()> {
        let wire = self.wire(path)?;
        let mtime = facts.mtime.and_then(|t| {
            t.duration_since(UNIX_EPOCH)
                .ok()
                .map(|d| d.as_secs() as u32)
        });
        self.sftp.set_meta(&wire, facts.mode, mtime).await
    }

    /// Remote → local.
    pub async fn download<F: FnMut(u64) -> bool + ?Sized>(
        &self,
        src: &VPath,
        dst: &Path,
        on_bytes: &mut F,
    ) -> Result<u64> {
        let wire = self.wire(src)?;
        let mut reader = self.sftp.open_read(&wire).await?;
        let mut writer = tokio::fs::File::create(dst)
            .await
            .map_err(|e| NetError::remote(dst.display().to_string(), e))?;
        let moved = pump(&mut reader, &mut writer, on_bytes, &wire).await;
        // A failed or cancelled download leaves nothing behind.
        if moved.is_err() {
            let _ = tokio::fs::remove_file(dst).await;
        }
        let total = moved?;
        writer
            .flush()
            .await
            .map_err(|e| NetError::remote(dst.display().to_string(), e))?;
        Ok(total)
    }

    /// Local → remote.
    pub async fn upload<F: FnMut(u64) -> bool + ?Sized>(
        &self,
        src: &Path,
        dst: &VPath,
        on_bytes: &mut F,
    ) -> Result<u64> {
        let wire = self.wire(dst)?;
        let mut reader = tokio::fs::File::open(src)
            .await
            .map_err(|e| NetError::remote(src.display().to_string(), e))?;
        let mut writer = self.sftp.create(&wire).await?;
        let moved = pump(&mut reader, &mut writer, on_bytes, &wire).await;
        if moved.is_err() {
            let _ = self.sftp.remove_file(&wire).await;
        }
        let total = moved?;
        writer
            .shutdown()
            .await
            .map_err(|e| NetError::remote(&wire, e))?;
        Ok(total)
    }

    /// Upload, then put the file in place with a rename: the destination keeps its old
    /// bytes until the new ones are completely on the server.
    ///
    /// SFTP version 3's rename fails when the destination exists and OpenSSH's atomic
    /// `posix-rename` extension is not in `russh-sftp` yet, so the old file is removed
    /// between the upload and the rename. If the rename then fails, the uploaded bytes are
    /// still on the server under the `.scmd-part` name, and the error says so — nothing is
    /// lost, but it has to be moved by hand.
    pub async fn upload_in_place<F: FnMut(u64) -> bool + ?Sized>(
        &self,
        src: &Path,
        dst: &VPath,
        on_bytes: &mut F,
    ) -> Result<u64> {
        let wire = self.wire(dst)?;
        let part = format!("{wire}{PART_SUFFIX}");
        let part_path = VPath::remote(self.site.scheme, self.authority(), &part);
        let written = self.upload(src, &part_path, on_bytes).await?;
        if self.sftp.exists(&wire).await {
            self.sftp.remove_file(&wire).await?;
        }
        match self.sftp.rename(&part, &wire).await {
            Ok(()) => Ok(written),
            Err(e) => Err(NetError::Remote {
                path: wire,
                detail: format!("{e}: the new bytes are on the server as {part}"),
            }),
        }
    }

    /// Remote → remote on the same connection. SFTP has no server-side copy, so the bytes
    /// travel through us; the caller sees the same progress either way.
    pub async fn copy_within<F: FnMut(u64) -> bool + ?Sized>(
        &self,
        src: &VPath,
        dst: &VPath,
        on_bytes: &mut F,
    ) -> Result<u64> {
        let (src_wire, dst_wire) = (self.wire(src)?, self.wire(dst)?);
        let mut reader = self.sftp.open_read(&src_wire).await?;
        let mut writer = self.sftp.create(&dst_wire).await?;
        let moved = pump(&mut reader, &mut writer, on_bytes, &dst_wire).await;
        if moved.is_err() {
            let _ = self.sftp.remove_file(&dst_wire).await;
        }
        let total = moved?;
        writer
            .shutdown()
            .await
            .map_err(|e| NetError::remote(&dst_wire, e))?;
        Ok(total)
    }

    /// Fetch a remote file into a local directory, so the viewer and the editor — which read
    /// real files — can open it. Returns the local path.
    pub async fn fetch_to(
        &self,
        src: &VPath,
        dir: &Path,
        cancel: &CancelToken,
    ) -> Result<std::path::PathBuf> {
        let name = src.display_name();
        let local = dir.join(if name.is_empty() { "file".into() } else { name });
        let mut cancelled = false;
        let mut on_bytes = |_: u64| {
            if cancel.is_cancelled() {
                cancelled = true;
                return false;
            }
            true
        };
        self.download(src, &local, &mut on_bytes).await?;
        if cancelled {
            return Err(NetError::Cancelled);
        }
        Ok(local)
    }
}

/// Remote → remote across two connections: the bytes come down from one and go up to the
/// other without touching the local disk.
pub async fn copy_between<F: FnMut(u64) -> bool + ?Sized>(
    from: &Remote,
    src: &VPath,
    to: &Remote,
    dst: &VPath,
    on_bytes: &mut F,
) -> Result<u64> {
    let src_wire = from.wire(src)?;
    let dst_wire = to.wire(dst)?;
    let mut reader = from.sftp().open_read(&src_wire).await?;
    let mut writer = to.sftp().create(&dst_wire).await?;
    let moved = pump(&mut reader, &mut writer, on_bytes, &dst_wire).await;
    if moved.is_err() {
        let _ = to.sftp().remove_file(&dst_wire).await;
    }
    let total = moved?;
    writer
        .shutdown()
        .await
        .map_err(|e| NetError::remote(&dst_wire, e))?;
    Ok(total)
}

/// Move bytes, reporting progress and stopping when the caller says so.
async fn pump<F: FnMut(u64) -> bool + ?Sized>(
    reader: &mut (impl AsyncReadExt + Unpin + ?Sized),
    writer: &mut (impl AsyncWriteExt + Unpin + ?Sized),
    on_bytes: &mut F,
    path: &str,
) -> Result<u64> {
    let mut buffer = vec![0u8; CHUNK];
    let mut total = 0u64;
    loop {
        let read = reader
            .read(&mut buffer)
            .await
            .map_err(|e| NetError::remote(path, e))?;
        if read == 0 {
            return Ok(total);
        }
        writer
            .write_all(&buffer[..read])
            .await
            .map_err(|e| NetError::remote(path, e))?;
        total += read as u64;
        if !on_bytes(total) {
            return Err(NetError::Cancelled);
        }
    }
}

/// `RemoteEntry` for a local file, so one code path can describe both sides of a copy.
pub fn facts_to_entry(name: &str, facts: &Facts) -> RemoteEntry {
    RemoteEntry {
        name: name.to_string(),
        kind: facts.kind,
        size: facts.size,
        mtime: facts
            .mtime
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as u32),
        mode: facts.mode,
        link_target: None,
    }
}

/// Did this entry come back as a directory?
pub fn is_dir(entry: &Entry) -> bool {
    entry.kind == EntryKind::Dir
}

/// The epoch seconds of a [`SystemTime`], for SFTP's 32-bit times.
pub fn epoch_secs(time: SystemTime) -> Option<u32> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemorySftp;
    use crate::site::Site;

    fn remote(fs: Arc<MemorySftp>) -> Remote {
        let site = Site::parse_url("sftp://mb@host:22/").unwrap();
        Remote::new(site, fs)
    }

    fn path(remote: &Remote, path: &str) -> VPath {
        VPath::remote(scmd_core::Scheme::Sftp, remote.authority(), path)
    }

    #[tokio::test]
    async fn a_listing_has_the_parent_entry_and_real_metadata() {
        let fs = MemorySftp::new()
            .with_file("/srv/www/index.html", b"<html>")
            .with_dir("/srv/www/assets")
            .with_file("/srv/www/.hidden", b"x");
        let r = remote(fs);
        let entries = r
            .list(&path(&r, "/srv/www"), &CancelToken::new())
            .await
            .unwrap();
        let names: Vec<String> = entries.iter().map(|e| e.name_str().into_owned()).collect();
        assert_eq!(names, ["..", ".hidden", "assets", "index.html"]);
        let index = entries
            .iter()
            .find(|e| e.name_str() == "index.html")
            .unwrap();
        assert_eq!(index.size, 6);
        assert!(index.mtime.is_some());
        let hidden = entries.iter().find(|e| e.name_str() == ".hidden").unwrap();
        assert!(hidden.flags.contains(scmd_core::EntryFlags::HIDDEN));
        // The root has no `..`.
        let root = r.list(&path(&r, "/"), &CancelToken::new()).await.unwrap();
        assert_eq!(root.iter().filter(|e| e.name_str() == "..").count(), 0);
    }

    #[tokio::test]
    async fn a_path_from_another_connection_is_refused() {
        let r = remote(MemorySftp::new());
        let err = r.list(&VPath::local("/srv"), &CancelToken::new()).await;
        assert!(matches!(err, Err(NetError::NotRemote(_))), "{err:?}");
        let other = VPath::remote(scmd_core::Scheme::Sftp, "mb@elsewhere:22", "/srv");
        assert!(r.stat(&other).await.is_err());
    }

    #[tokio::test]
    async fn mkdir_all_makes_what_is_missing_and_keeps_what_is_there() {
        let fs = MemorySftp::new().with_dir("/srv");
        let r = remote(fs.clone());
        r.create_dir_all(&path(&r, "/srv/a/b/c")).await.unwrap();
        assert!(fs.paths().contains(&"/srv/a/b/c".to_string()));
        // Again: nothing breaks, nothing is lost.
        r.create_dir_all(&path(&r, "/srv/a/b/c")).await.unwrap();
    }

    #[tokio::test]
    async fn download_upload_and_remote_copy_move_every_byte() {
        let dir = tempfile::tempdir().unwrap();
        let payload: Vec<u8> = (0..300_000u32).map(|i| (i % 251) as u8).collect();
        let fs = MemorySftp::new().with_file("/srv/big.bin", &payload);
        let r = remote(fs.clone());

        // Remote → local, with progress.
        let local = dir.path().join("big.bin");
        let mut seen = Vec::new();
        let mut on_bytes = |done: u64| {
            seen.push(done);
            true
        };
        let total = r
            .download(&path(&r, "/srv/big.bin"), &local, &mut on_bytes)
            .await
            .unwrap();
        assert_eq!(total, payload.len() as u64);
        assert_eq!(std::fs::read(&local).unwrap(), payload);
        assert!(seen.len() > 1, "progress was reported once: {seen:?}");
        assert_eq!(seen.last(), Some(&total));

        // Local → remote.
        let mut ignore = |_: u64| true;
        r.upload(&local, &path(&r, "/srv/copy.bin"), &mut ignore)
            .await
            .unwrap();
        assert_eq!(fs.file("/srv/copy.bin").unwrap(), payload);

        // Remote → remote.
        r.copy_within(
            &path(&r, "/srv/copy.bin"),
            &path(&r, "/srv/third.bin"),
            &mut ignore,
        )
        .await
        .unwrap();
        assert_eq!(fs.file("/srv/third.bin").unwrap(), payload);
    }

    #[tokio::test]
    async fn a_cancelled_transfer_leaves_nothing_behind() {
        let dir = tempfile::tempdir().unwrap();
        let payload = vec![7u8; CHUNK * 3];
        let fs = MemorySftp::new().with_file("/srv/big.bin", &payload);
        let r = remote(fs.clone());

        let local = dir.path().join("big.bin");
        let mut stop = |_: u64| false;
        let err = r
            .download(&path(&r, "/srv/big.bin"), &local, &mut stop)
            .await;
        assert!(matches!(err, Err(NetError::Cancelled)), "{err:?}");
        assert!(!local.exists(), "the partial download was left on disk");

        std::fs::write(&local, &payload).unwrap();
        let err = r.upload(&local, &path(&r, "/srv/up.bin"), &mut stop).await;
        assert!(matches!(err, Err(NetError::Cancelled)), "{err:?}");
        assert_eq!(
            fs.file("/srv/up.bin"),
            None,
            "a partial file was left on the server"
        );
    }

    #[tokio::test]
    async fn the_servers_own_message_reaches_the_caller() {
        let fs = MemorySftp::new()
            .with_dir("/srv")
            .break_path("/srv/locked", "permission denied");
        let r = remote(fs);
        let err = r.create_dir(&path(&r, "/srv/locked")).await.unwrap_err();
        let text = err.to_string();
        assert!(text.contains("/srv/locked"), "{text}");
        assert!(text.contains("permission denied"), "{text}");
        // And it maps onto the io error kind the job engine reads.
        assert_eq!(err.into_io().kind(), std::io::ErrorKind::PermissionDenied);
    }

    #[tokio::test]
    async fn an_upload_in_place_keeps_the_old_bytes_until_the_new_ones_are_there() {
        let dir = tempfile::tempdir().unwrap();
        let local = dir.path().join("notes.txt");
        std::fs::write(&local, b"second version\n").unwrap();
        let fs = MemorySftp::new().with_file("/srv/notes.txt", b"first version\n");
        let r = remote(fs.clone());
        let mut ignore = |_: u64| true;

        let written = r
            .upload_in_place(&local, &path(&r, "/srv/notes.txt"), &mut ignore)
            .await
            .unwrap();
        assert_eq!(written, 15);
        assert_eq!(fs.file("/srv/notes.txt").unwrap(), b"second version\n");
        assert!(
            !fs.paths().iter().any(|p| p.ends_with(".scmd-part")),
            "the part file is still there: {:?}",
            fs.paths()
        );

        // A write to a name that is not there yet is just a write.
        r.upload_in_place(&local, &path(&r, "/srv/new.txt"), &mut ignore)
            .await
            .unwrap();
        assert_eq!(fs.file("/srv/new.txt").unwrap(), b"second version\n");

        // When the upload itself fails, the destination is untouched.
        let fs = MemorySftp::new()
            .with_file("/srv/notes.txt", b"first version\n")
            .break_path("/srv/notes.txt.scmd-part", "quota exceeded");
        let r = remote(fs.clone());
        let err = r
            .upload_in_place(&local, &path(&r, "/srv/notes.txt"), &mut ignore)
            .await
            .unwrap_err();
        assert!(err.to_string().contains("quota exceeded"), "{err}");
        assert_eq!(fs.file("/srv/notes.txt").unwrap(), b"first version\n");
    }

    #[tokio::test]
    async fn fetch_to_brings_a_file_down_for_the_viewer() {
        let dir = tempfile::tempdir().unwrap();
        let fs = MemorySftp::new().with_file("/srv/notes.txt", b"hello\n");
        let r = remote(fs);
        let local = r
            .fetch_to(&path(&r, "/srv/notes.txt"), dir.path(), &CancelToken::new())
            .await
            .unwrap();
        assert_eq!(local.file_name().unwrap(), "notes.txt");
        assert_eq!(std::fs::read_to_string(&local).unwrap(), "hello\n");
    }

    #[tokio::test]
    async fn rename_moves_a_whole_subtree_and_meta_survives() {
        let fs = MemorySftp::new()
            .with_file("/srv/a/x.txt", b"x")
            .with_dir("/srv/b");
        let r = remote(fs.clone());
        r.rename(&path(&r, "/srv/a"), &path(&r, "/srv/b/a"))
            .await
            .unwrap();
        assert!(fs.paths().contains(&"/srv/b/a/x.txt".to_string()));
        assert!(!fs.paths().contains(&"/srv/a".to_string()));

        let facts = Facts {
            kind: EntryKind::File,
            size: 1,
            mtime: Some(UNIX_EPOCH + std::time::Duration::from_secs(1_600_000_000)),
            mode: Some(0o600),
            readonly: false,
        };
        r.apply_meta(&path(&r, "/srv/b/a/x.txt"), &facts)
            .await
            .unwrap();
        assert_eq!(fs.mode("/srv/b/a/x.txt"), Some(0o600));
        assert_eq!(fs.mtime("/srv/b/a/x.txt"), Some(1_600_000_000));
    }
}
