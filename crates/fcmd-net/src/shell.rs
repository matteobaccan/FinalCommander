//! A server reached over SSH that has no SFTP: its files through the shell.
//!
//! Routers, NAS boxes and embedded systems often run an SSH server — dropbear, most of the
//! time — with no `sftp-server` beside it, so the subsystem the netbox asks for is refused and
//! there is nothing to browse. What they do have is a shell. This backend asks it: `ls -la` to
//! list, `cat` to read and to write, `mkdir`, `rm`, `rmdir`, `mv`, `ln -s`, `chmod`, `touch`
//! for the rest — what `scp` itself relies on, and what Midnight Commander's shell link does.
//! The SSH client falls back to it on its own when the server refuses `sftp`.
//!
//! The commands go through [`Exec`], so the parsing and the quoting are tested against a real
//! local shell, and the SSH side is the thin part that opens a channel per command.

use async_trait::async_trait;
use fcmd_core::EntryKind;

use crate::error::{NetError, Result};
use crate::ops::{Reader, RemoteEntry, RemoteFs, Writer};

/// What one command printed, and how it ended.
#[derive(Debug, Clone, Default)]
pub struct Output {
    pub status: i32,
    pub stdout: Vec<u8>,
    pub stderr: String,
}

/// Runs one shell command on the far side.
#[async_trait]
pub trait Exec: Send + Sync {
    async fn run(&self, command: &str) -> Result<Output>;
    /// A command whose standard input is the returned writer; it runs until the writer is
    /// shut down.
    async fn write(&self, command: &str) -> Result<Writer>;
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// A word for `sh`, quoted so nothing in it is read as anything but itself.
pub fn sh_quote(word: &str) -> String {
    format!("'{}'", word.replace('\'', "'\\''"))
}

/// One line of `ls -la --time-style=+%s` (GNU) or of a plain `ls -la` (busybox, BSD),
/// where the time is then not a number and is left out.
pub fn parse_ls(line: &str) -> Option<RemoteEntry> {
    let line = line.trim_end();
    let kind = match line.chars().next()? {
        'd' => EntryKind::Dir,
        '-' => EntryKind::File,
        'l' => EntryKind::Symlink,
        'b' | 'c' | 'p' | 's' => EntryKind::Other,
        _ => return None,
    };
    let perms = line.get(1..10)?;
    let mode = perms
        .chars()
        .zip([
            0o400, 0o200, 0o100, 0o040, 0o020, 0o010, 0o004, 0o002, 0o001,
        ])
        .filter(|(c, _)| !matches!(c, '-' | 'S' | 'T'))
        .fold(0u32, |m, (_, bit)| m | bit);
    // perms, links, owner, group, size, then the time: one field (`+%s`) or three (`Nov 5
    // 10:00`), then the name, which may hold spaces.
    let mut rest = line;
    let mut fields = Vec::new();
    for _ in 0..5 {
        rest = rest.trim_start();
        let end = rest.find(' ')?;
        fields.push(&rest[..end]);
        rest = &rest[end..];
    }
    let size: u64 = fields[4].parse().unwrap_or(0);
    rest = rest.trim_start();
    let (mtime, name) = {
        let end = rest.find(' ')?;
        match rest[..end].parse::<u32>() {
            Ok(secs) if !rest[..end].contains(':') && rest[..end].len() >= 9 => {
                (Some(secs), rest[end..].trim_start())
            }
            _ => {
                // Three fields of a human date.
                let mut r = rest;
                for _ in 0..3 {
                    r = r.trim_start();
                    let e = r.find(' ')?;
                    r = &r[e..];
                }
                (None, r.trim_start())
            }
        }
    };
    let (name, target) = match (kind, name.split_once(" -> ")) {
        (EntryKind::Symlink, Some((n, t))) => (n, Some(t.to_string())),
        _ => (name, None),
    };
    if name.is_empty() {
        return None;
    }
    Some(RemoteEntry {
        name: name.to_string(),
        kind,
        size,
        mtime,
        mode: Some(mode),
        link_target: target,
    })
}

pub struct ShellFs<E: Exec> {
    exec: E,
}

impl<E: Exec> ShellFs<E> {
    pub fn new(exec: E) -> Self {
        ShellFs { exec }
    }

    async fn ok(&self, path: &str, command: String) -> Result<Output> {
        let out = self.exec.run(&command).await?;
        if out.status != 0 {
            return Err(NetError::Remote {
                path: path.to_string(),
                detail: if out.stderr.trim().is_empty() {
                    format!("the command ended with status {}", out.status)
                } else {
                    out.stderr.trim().to_string()
                },
            });
        }
        Ok(out)
    }

    async fn ls(&self, path: &str, flags: &str) -> Result<Vec<RemoteEntry>> {
        let q = sh_quote(path);
        // GNU first, for times as numbers; anything else understands the plain form.
        let command = format!(
            "LC_ALL=C ls {flags} --time-style=+%s -- {q} 2>/dev/null || LC_ALL=C ls {flags} -- {q}"
        );
        let out = self.ok(path, command).await?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .lines()
            .filter_map(parse_ls)
            .collect())
    }
}

impl<E: Exec> std::fmt::Debug for ShellFs<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ShellFs")
    }
}

#[async_trait]
impl<E: Exec + 'static> RemoteFs for ShellFs<E> {
    async fn read_dir(&self, path: &str) -> Result<Vec<RemoteEntry>> {
        Ok(self
            .ls(path, "-la")
            .await?
            .into_iter()
            .filter(|e| e.name != "." && e.name != "..")
            .collect())
    }

    async fn stat(&self, path: &str, follow: bool) -> Result<RemoteEntry> {
        let flags = if follow { "-ldL" } else { "-ld" };
        let mut found = self
            .ls(path, flags)
            .await?
            .into_iter()
            .next()
            .ok_or_else(|| NetError::Remote {
                path: path.to_string(),
                detail: "no such file".into(),
            })?;
        found.name = path
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or(path)
            .to_string();
        Ok(found)
    }

    async fn create_dir(&self, path: &str) -> Result<()> {
        self.ok(path, format!("mkdir -- {}", sh_quote(path)))
            .await
            .map(|_| ())
    }
    async fn remove_file(&self, path: &str) -> Result<()> {
        self.ok(path, format!("rm -f -- {}", sh_quote(path)))
            .await
            .map(|_| ())
    }
    async fn remove_dir(&self, path: &str) -> Result<()> {
        self.ok(path, format!("rmdir -- {}", sh_quote(path)))
            .await
            .map(|_| ())
    }
    async fn rename(&self, from: &str, to: &str) -> Result<()> {
        self.ok(from, format!("mv -- {} {}", sh_quote(from), sh_quote(to)))
            .await
            .map(|_| ())
    }
    async fn read_link(&self, path: &str) -> Result<String> {
        let out = self
            .ok(path, format!("readlink -- {}", sh_quote(path)))
            .await?;
        Ok(String::from_utf8_lossy(&out.stdout)
            .trim_end_matches('\n')
            .to_string())
    }
    async fn symlink(&self, target: &str, link: &str) -> Result<()> {
        self.ok(
            link,
            format!("ln -s -- {} {}", sh_quote(target), sh_quote(link)),
        )
        .await
        .map(|_| ())
    }
    async fn set_meta(&self, path: &str, mode: Option<u32>, mtime: Option<u32>) -> Result<()> {
        let q = sh_quote(path);
        if let Some(mode) = mode {
            self.ok(path, format!("chmod {:o} -- {q}", mode & 0o7777))
                .await?;
        }
        if let Some(secs) = mtime {
            self.ok(path, format!("touch -d @{secs} -- {q}")).await?;
        }
        Ok(())
    }
    async fn open_read(&self, path: &str) -> Result<Reader> {
        let out = self.ok(path, format!("cat -- {}", sh_quote(path))).await?;
        Ok(Box::new(std::io::Cursor::new(out.stdout)))
    }
    async fn create(&self, path: &str) -> Result<Writer> {
        self.exec.write(&format!("cat > {}", sh_quote(path))).await
    }
    async fn canonicalize(&self, path: &str) -> Result<String> {
        let target = if path.is_empty() { "." } else { path };
        let out = self
            .ok(path, format!("cd -- {} && pwd", sh_quote(target)))
            .await?;
        Ok(String::from_utf8_lossy(&out.stdout).trim_end().to_string())
    }
    async fn close(&self) -> Result<()> {
        self.exec.close().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    /// The far side is this machine's own `sh`.
    struct LocalSh;

    struct Pending {
        command: String,
        bytes: Vec<u8>,
    }

    impl tokio::io::AsyncWrite for Pending {
        fn poll_write(
            mut self: Pin<&mut Self>,
            _: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<std::io::Result<usize>> {
            self.bytes.extend_from_slice(buf);
            Poll::Ready(Ok(buf.len()))
        }
        fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            Poll::Ready(Ok(()))
        }
        fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
            use std::io::Write;
            let mut child = std::process::Command::new("sh")
                .arg("-c")
                .arg(&self.command)
                .stdin(std::process::Stdio::piped())
                .spawn()?;
            child.stdin.take().expect("stdin").write_all(&self.bytes)?;
            child.wait()?;
            Poll::Ready(Ok(()))
        }
    }

    #[async_trait]
    impl Exec for LocalSh {
        async fn run(&self, command: &str) -> Result<Output> {
            let out = std::process::Command::new("sh")
                .arg("-c")
                .arg(command)
                .output()
                .map_err(|e| NetError::remote("sh", e))?;
            Ok(Output {
                status: out.status.code().unwrap_or(-1),
                stdout: out.stdout,
                stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
            })
        }
        async fn write(&self, command: &str) -> Result<Writer> {
            Ok(Box::new(Pending {
                command: command.to_string(),
                bytes: Vec::new(),
            }))
        }
    }

    #[test]
    fn ls_lines_of_both_kinds_are_read() {
        let gnu = parse_ls("-rw-r--r-- 1 mb staff 1234 1730800000 a name with spaces.txt").unwrap();
        assert_eq!(gnu.name, "a name with spaces.txt");
        assert_eq!(gnu.size, 1234);
        assert_eq!(gnu.mtime, Some(1_730_800_000));
        assert_eq!(gnu.mode, Some(0o644));
        let plain =
            parse_ls("drwxr-xr-x    2 root     root          4096 Nov  5 10:00 etc").unwrap();
        assert_eq!(plain.name, "etc");
        assert_eq!(plain.kind, EntryKind::Dir);
        assert_eq!(plain.mtime, None);
        let link = parse_ls("lrwxrwxrwx 1 mb mb 7 1730800000 latest -> v1.2.3").unwrap();
        assert_eq!(link.name, "latest");
        assert_eq!(link.link_target.as_deref(), Some("v1.2.3"));
        assert!(parse_ls("total 12").is_none());
        assert_eq!(sh_quote("it's"), "'it'\\''s'");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_shell_serves_a_tree_as_sftp_would() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_string_lossy().to_string();
        let fs = ShellFs::new(LocalSh);
        let odd = format!("{root}/it's a dir");
        fs.create_dir(&odd).await.unwrap();
        let file = format!("{odd}/notes.txt");
        let mut w = fs.create(&file).await.unwrap();
        w.write_all(b"hello over the shell\n").await.unwrap();
        w.shutdown().await.unwrap();

        let rows = fs.read_dir(&odd).await.unwrap();
        assert_eq!(rows.len(), 1, "{rows:?}");
        assert_eq!(rows[0].name, "notes.txt");
        assert_eq!(rows[0].size, 21);
        let mut text = String::new();
        fs.open_read(&file)
            .await
            .unwrap()
            .read_to_string(&mut text)
            .await
            .unwrap();
        assert_eq!(text, "hello over the shell\n");
        assert_eq!(fs.stat(&file, false).await.unwrap().name, "notes.txt");

        let moved = format!("{odd}/renamed.txt");
        fs.rename(&file, &moved).await.unwrap();
        fs.set_meta(&moved, Some(0o600), Some(1_000_000_000))
            .await
            .unwrap();
        let st = fs.stat(&moved, false).await.unwrap();
        assert_eq!(st.mode, Some(0o600));
        fs.remove_file(&moved).await.unwrap();
        fs.remove_dir(&odd).await.unwrap();
        assert!(fs.read_dir(&odd).await.is_err(), "and a failure says so");
        assert!(fs
            .canonicalize(&root)
            .await
            .unwrap()
            .ends_with(dir.path().file_name().unwrap().to_str().unwrap()));
    }
}
