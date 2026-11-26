//! One commit of a Git repository as a container: a branch, a tag or a commit read as a tree
//! of files, without checking it out (SPEC 06 §15).
//!
//! It is a container like the others, which is the whole design. What says *which* tree is a
//! small text file — a **descriptor**, `<repository>@<ref>.gitref` — holding the repository
//! and the commit the ref named when the descriptor was written. Stepping into it is stepping
//! into an archive, so the panel, the viewer, the quick view, the copy out, the search and the
//! comparison of two folders need nothing new: they already know how to read a container and
//! already refuse to write into one that cannot be written.
//!
//! The index is `git ls-tree -r -t -z --long <commit>`, one run for the whole tree, and one
//! entry's bytes are `git cat-file blob <commit>:<path>`. Both name the commit by its hash:
//! the ref is resolved once, when the descriptor is written, so the tree a panel shows does
//! not change under it when the branch moves.
//!
//! `git` is run as a program, never through a shell, with every path and name an argument of
//! its own. A ref is refused before git is asked when it starts with `-`, which git would
//! read as an option; the commit in a descriptor must be a hash and nothing else, because a
//! descriptor is a file and a file can be written by anybody.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use fcmd_core::{CancelToken, EntryKind};

use crate::archive::ArcEntry;
use crate::{ArcError, Limits, Result};

/// The suffix of a descriptor, without the dot.
pub const EXTENSION: &str = "gitref";

/// The first line of a descriptor: what it is, and the version of its layout.
const MAGIC: &str = "fcmd-gitref 1";

/// Why git could not be asked, or what it answered when the answer was no. Kept apart from
/// [`ArcError`] because the program says each of these in its own words, in the reader's
/// language, and needs to know which one it was.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum GitError {
    /// There is no `git` to run.
    #[error("git is not installed, or is not on the PATH")]
    NoGit,
    #[error("{}: not inside a Git repository", .0.display())]
    NotARepository(PathBuf),
    /// Git knows no branch, tag or commit by this name.
    #[error("{0}: no such branch, tag or commit")]
    UnknownRef(String),
    /// Not a name git may be given: empty, starting with `-`, or holding a control character.
    #[error("{0}: not a name a branch, a tag or a commit can have")]
    BadRef(String),
    /// Git ran and failed, and this is what it said.
    #[error("{0}")]
    Failed(String),
    #[error("cancelled")]
    Cancelled,
}

/// Which tree: the repository, and the commit a ref named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRef {
    /// The top of the work tree, or the repository itself when it has none.
    pub repo: PathBuf,
    /// The commit's full hash. The only thing git is asked about from here on.
    pub commit: String,
    /// What was asked for — `main`, `v1.2`, `a1b2c3d` — kept for the reader.
    pub name: String,
}

impl GitRef {
    /// The commit as a title has room for it.
    pub fn short(&self) -> &str {
        &self.commit[..self.commit.len().min(8)]
    }

    /// The descriptor's text.
    pub fn to_text(&self) -> String {
        format!(
            "{MAGIC}\nrepo={}\ncommit={}\nref={}\n",
            self.repo.display(),
            self.commit,
            self.name
        )
    }

    /// Read a descriptor's text back. `None` for anything that is not one — including one
    /// whose commit is not a hash, which is the line git is handed as an argument.
    pub fn parse(text: &str) -> Option<GitRef> {
        let mut lines = text.lines();
        if lines.next()? != MAGIC {
            return None;
        }
        let (mut repo, mut commit, mut name) = (None, None, None);
        for line in lines {
            match line.split_once('=')? {
                ("repo", value) => repo = Some(PathBuf::from(value)),
                ("commit", value) => commit = Some(value.to_string()),
                ("ref", value) => name = Some(value.to_string()),
                _ => {}
            }
        }
        let commit = commit.filter(|c| is_hash(c))?;
        let repo = repo.filter(|r| r.is_absolute())?;
        Some(GitRef {
            repo,
            name: name.unwrap_or_else(|| commit.clone()),
            commit,
        })
    }

    /// The descriptor at `path`, if it is one.
    pub fn read(path: &Path) -> Option<GitRef> {
        if !named_like(path) {
            return None;
        }
        // A descriptor is four short lines; anything long under this name is something else.
        let meta = std::fs::metadata(path).ok()?;
        if meta.len() > 64 * 1024 {
            return None;
        }
        GitRef::parse(&std::fs::read_to_string(path).ok()?)
    }

    /// Write the descriptor into `dir`, under a name that says what it is, and say where.
    pub fn write_in(&self, dir: &Path) -> std::io::Result<PathBuf> {
        std::fs::create_dir_all(dir)?;
        let repo = self
            .repo
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "repository".to_string());
        // A ref may hold `/` (`feature/x`, `origin/main`), which a file name may not.
        let name: String = format!("{repo}@{}", self.name)
            .chars()
            .map(|c| match c {
                '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
                c if c.is_control() => '-',
                c => c,
            })
            .collect();
        let path = dir.join(format!("{name}.{EXTENSION}"));
        std::fs::write(&path, self.to_text())?;
        Ok(path)
    }
}

/// Is this file named as a descriptor is? By name alone, which is what deciding whether
/// `Enter` goes into a file has to be.
pub fn named_like(path: &Path) -> bool {
    path.extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(EXTENSION))
}

/// A full object name: forty hex digits, or sixty-four in a SHA-256 repository.
fn is_hash(text: &str) -> bool {
    matches!(text.len(), 40 | 64) && text.bytes().all(|b| b.is_ascii_hexdigit())
}

/// `git -C <dir> <args>`, as every question here is asked: no shell, nothing on the standard
/// input unless `input` is given, and never a prompt on the terminal the program draws on.
fn command(dir: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(dir)
        .args(args)
        // `ssh` asks on the terminal whatever stdin is, so it is told not to ask at all —
        // unless whoever runs this has set their own ssh command, which is theirs to keep.
        .env("GIT_TERMINAL_PROMPT", "0")
        // Reading must not write: no index refreshed behind the reader's back.
        .env("GIT_OPTIONAL_LOCKS", "0");
    if std::env::var_os("GIT_SSH_COMMAND").is_none() {
        command.env("GIT_SSH_COMMAND", "ssh -o BatchMode=yes");
    }
    command
}

fn spawn_failed(e: &std::io::Error) -> GitError {
    if e.kind() == std::io::ErrorKind::NotFound {
        GitError::NoGit
    } else {
        GitError::Failed(format!("git: {e}"))
    }
}

/// What git said on its error stream, or the exit status when it said nothing.
fn said(args: &[&str], status: std::process::ExitStatus, err: &[u8]) -> GitError {
    let err = String::from_utf8_lossy(err).trim().to_string();
    GitError::Failed(if err.is_empty() {
        format!("git {}: {status}", args.join(" "))
    } else {
        err
    })
}

/// Run git in `dir` and hand back what it printed, as bytes: a path is not always text.
///
/// `input` goes to its standard input, from a thread of its own, and both of its output
/// pipes are drained on threads too — a program that fills one pipe while the other is being
/// read stops, and so would this. A cancel kills it.
pub fn run(
    dir: &Path,
    args: &[&str],
    input: Option<Vec<u8>>,
    cancel: &CancelToken,
) -> std::result::Result<Vec<u8>, GitError> {
    let mut child = command(dir, args)
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| spawn_failed(&e))?;
    let stdin = child.stdin.take();
    let feeder = std::thread::spawn(move || {
        if let (Some(mut pipe), Some(bytes)) = (stdin, input) {
            let _ = pipe.write_all(&bytes);
        }
    });
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = stdout.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = stderr.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let mut killed = false;
    let status = loop {
        if cancel.is_cancelled() && !killed {
            let _ = child.kill();
            killed = true;
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => {
                let _ = child.kill();
                return Err(GitError::Failed(format!("git: {e}")));
            }
        }
    };
    let _ = feeder.join();
    let out = out_thread.join().unwrap_or_default();
    let err = err_thread.join().unwrap_or_default();
    if killed {
        return Err(GitError::Cancelled);
    }
    if !status.success() {
        return Err(said(args, status, &err));
    }
    Ok(out)
}

/// Which commit `reference` names in the repository `dir` is inside — asked once, so that
/// what is browsed afterwards is that commit and not wherever the branch has moved to.
pub fn resolve(dir: &Path, reference: &str) -> std::result::Result<GitRef, GitError> {
    let name = reference.trim();
    // A name starting with `-` is an option to git, whatever follows it; refused here rather
    // than escaped, because no branch, tag or hash is spelled that way.
    if name.is_empty() || name.starts_with('-') || name.chars().any(|c| c.is_control()) {
        return Err(GitError::BadRef(name.to_string()));
    }
    let cancel = CancelToken::new();
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).trim().to_string();
    // The top of the work tree; a repository without one (a bare one) is named by itself.
    let top = match run(dir, &["rev-parse", "--show-toplevel"], None, &cancel) {
        Ok(top) => text(top),
        Err(GitError::NoGit) => return Err(GitError::NoGit),
        Err(_) => match run(dir, &["rev-parse", "--absolute-git-dir"], None, &cancel) {
            Ok(top) => text(top),
            Err(GitError::NoGit) => return Err(GitError::NoGit),
            Err(_) => return Err(GitError::NotARepository(dir.to_path_buf())),
        },
    };
    // git names the top with `/` on Windows as well (`D:/repo`).
    let repo = PathBuf::from(top.replace('/', std::path::MAIN_SEPARATOR_STR));
    // `^{commit}` peels a tag to the commit it marks and refuses a name that is a file or a
    // tree; `--verify` wants exactly one object and `--quiet` keeps the refusal short.
    let spec = format!("{name}^{{commit}}");
    let commit = match run(
        &repo,
        &["rev-parse", "--verify", "--quiet", &spec],
        None,
        &cancel,
    ) {
        Ok(out) => text(out),
        Err(GitError::NoGit) => return Err(GitError::NoGit),
        Err(_) => return Err(GitError::UnknownRef(name.to_string())),
    };
    if !is_hash(&commit) {
        return Err(GitError::UnknownRef(name.to_string()));
    }
    Ok(GitRef {
        repo,
        commit,
        name: name.to_string(),
    })
}

/// A tree read out of git: its entries, and which of them are other repositories.
#[derive(Debug)]
pub(crate) struct Tree {
    pub entries: Vec<ArcEntry>,
    /// The submodules, by their path inside the tree, each with the commit it is pinned at.
    pub submodules: Vec<(String, String)>,
}

fn external(reference: &GitRef, error: GitError) -> ArcError {
    match error {
        GitError::Cancelled => ArcError::Cancelled,
        other => ArcError::External {
            path: format!("{}@{}", reference.repo.display(), reference.name),
            reason: other.to_string(),
        },
    }
}

/// Every entry of the commit's tree, in one run of `git ls-tree`.
///
/// Git keeps no time per file, so every entry carries the commit's: it is when this version
/// of the tree was made, and it is what a copy out keeps. The guard on the total size is not
/// asked here — nothing is unpacked by listing a tree, and a repository is not a bomb — but
/// the one on the number of entries is, because the index is held in memory.
pub(crate) fn index(reference: &GitRef, limits: &Limits, cancel: &CancelToken) -> Result<Tree> {
    let fail = |e| external(reference, e);
    let listing = run(
        &reference.repo,
        &["ls-tree", "-r", "-t", "-z", "--long", &reference.commit],
        None,
        cancel,
    )
    .map_err(fail)?;
    let when = run(
        &reference.repo,
        &["log", "-1", "--format=%ct", &reference.commit],
        None,
        cancel,
    )
    .ok()
    .and_then(|out| String::from_utf8_lossy(&out).trim().parse::<u64>().ok())
    .map(|secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs));

    let mut entries = Vec::new();
    let mut submodules = Vec::new();
    // The links, to be asked about together: where each is in `entries`, and its object.
    let mut links: Vec<(usize, String)> = Vec::new();
    for record in listing.split(|b| *b == 0).filter(|r| !r.is_empty()) {
        // `<mode> <type> <object> <size>\t<path>`: the path is everything after the first
        // tab, and may hold anything but a NUL.
        let Some(tab) = record.iter().position(|b| *b == b'\t') else {
            continue;
        };
        let meta = String::from_utf8_lossy(&record[..tab]);
        let mut fields = meta.split_ascii_whitespace();
        let (Some(mode), Some(kind), Some(object)) = (fields.next(), fields.next(), fields.next())
        else {
            continue;
        };
        let size = fields
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        let path = crate::archive::normalise(&String::from_utf8_lossy(&record[tab + 1..]));
        if entries.len() >= limits.entries {
            return Err(ArcError::Refused {
                path: reference.repo.display().to_string(),
                reason: format!(
                    "the tree holds more than {} entries, the limit",
                    limits.entries
                ),
            });
        }
        let (kind, mode, size) = match (kind, mode) {
            ("tree", _) => (EntryKind::Dir, None, 0),
            // Another repository, pinned at a commit: a folder here, with nothing in it,
            // because its files are not in this repository's objects.
            ("commit", _) => {
                submodules.push((path.clone(), object.to_string()));
                (EntryKind::Dir, None, 0)
            }
            (_, "120000") => {
                links.push((entries.len(), object.to_string()));
                (EntryKind::Symlink, None, size)
            }
            // Git records one bit about a file: whether it may be run.
            (_, "100755") => (EntryKind::File, Some(0o755), size),
            _ => (EntryKind::File, Some(0o644), size),
        };
        entries.push(ArcEntry {
            path,
            size,
            packed: None,
            mtime: when,
            mode,
            kind,
            link_target: None,
            at: None,
        });
    }
    if !links.is_empty() {
        for (at, target) in link_targets(reference, &links, cancel) {
            if let Some(entry) = entries.get_mut(at) {
                entry.link_target = Some(target);
            }
        }
    }
    Ok(Tree {
        entries,
        submodules,
    })
}

/// Where each link points: the link's blob *is* its target, and one `git cat-file --batch`
/// reads all of them. A link whose answer cannot be read keeps no target, which shows as a
/// link that does not say where it goes rather than as a tree that failed to open.
fn link_targets(
    reference: &GitRef,
    links: &[(usize, String)],
    cancel: &CancelToken,
) -> Vec<(usize, String)> {
    let mut asked = Vec::new();
    for (_, object) in links {
        asked.extend_from_slice(object.as_bytes());
        asked.push(b'\n');
    }
    let Ok(out) = run(
        &reference.repo,
        &["cat-file", "--batch"],
        Some(asked),
        cancel,
    ) else {
        return Vec::new();
    };
    // Each answer is `<object> <type> <size>\n`, then that many bytes, then a newline.
    let mut found = Vec::new();
    let mut rest = &out[..];
    for (at, _) in links {
        let Some(end) = rest.iter().position(|b| *b == b'\n') else {
            break;
        };
        let header = String::from_utf8_lossy(&rest[..end]).into_owned();
        rest = &rest[end + 1..];
        let mut fields = header.split_ascii_whitespace();
        let (_, kind, size) = (fields.next(), fields.next(), fields.next());
        let Some(size) = size.and_then(|s| s.parse::<usize>().ok()) else {
            // `<object> missing`: no bytes follow.
            continue;
        };
        if rest.len() < size {
            break;
        }
        if kind == Some("blob") {
            found.push((*at, String::from_utf8_lossy(&rest[..size]).into_owned()));
        }
        rest = rest.get(size + 1..).unwrap_or(&[]);
    }
    found
}

/// The bytes of one file of the tree, poured into `out` as git prints them.
///
/// Exactly the blob: no line-ending conversion and no filter is applied, so what is read is
/// what was committed — a file kept in Git LFS is the pointer that was committed for it.
pub(crate) fn copy_out(
    reference: &GitRef,
    inner: &str,
    out: &mut dyn Write,
    cancel: &CancelToken,
    progress: &mut dyn FnMut(u64),
) -> Result<u64> {
    let spec = format!("{}:{}", reference.commit, inner.trim_start_matches('/'));
    let args = ["cat-file", "blob", spec.as_str()];
    let mut child = command(&reference.repo, &args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| external(reference, spawn_failed(&e)))?;
    let mut stderr = child.stderr.take();
    let err_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(pipe) = stderr.as_mut() {
            let _ = pipe.read_to_end(&mut buf);
        }
        buf
    });
    let mut written = 0u64;
    let mut failure = None;
    if let Some(mut pipe) = child.stdout.take() {
        let mut buf = vec![0u8; 64 * 1024];
        loop {
            if cancel.is_cancelled() {
                failure = Some(ArcError::Cancelled);
                break;
            }
            match pipe.read(&mut buf) {
                Ok(0) => break,
                Ok(read) => {
                    if let Err(e) = out.write_all(&buf[..read]) {
                        failure = Some(ArcError::Io {
                            path: "the destination".to_string(),
                            source: e,
                        });
                        break;
                    }
                    written += read as u64;
                    progress(written);
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
                Err(e) => {
                    failure = Some(ArcError::Io {
                        path: reference.repo.display().to_string(),
                        source: e,
                    });
                    break;
                }
            }
        }
    }
    if failure.is_some() {
        // Stopped half way: git is still writing to a pipe nobody reads.
        let _ = child.kill();
    }
    let status = child.wait();
    let err = err_thread.join().unwrap_or_default();
    if let Some(failure) = failure {
        return Err(failure);
    }
    match status {
        Ok(status) if status.success() => Ok(written),
        Ok(status) => Err(external(reference, said(&args, status, &err))),
        Err(e) => Err(external(reference, GitError::Failed(format!("git: {e}")))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Archive, Cache};

    /// A small real repository: `main` with a folder, a file that may be run and a link, and
    /// `other`, one commit on, where a file differs and one is new. `None` without `git` on
    /// the machine — the feature needs it, and so does its test.
    pub(crate) fn repository() -> Option<tempfile::TempDir> {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().to_path_buf();
        let git = |args: &[&str]| Command::new("git").arg("-C").arg(&root).args(args).output();
        if git(&["init", "-q", "-b", "main"]).is_err() {
            return None;
        }
        git(&["config", "user.email", "t@example.com"]).unwrap();
        git(&["config", "user.name", "T"]).unwrap();
        git(&["config", "core.autocrlf", "false"]).unwrap();
        git(&["config", "commit.gpgsign", "false"]).unwrap();
        std::fs::write(root.join("readme.md"), "# on main\n").unwrap();
        std::fs::create_dir(root.join("src")).unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn one() {}\n").unwrap();
        std::fs::write(root.join("run.sh"), "#!/bin/sh\necho hi\n").unwrap();
        git(&["add", "."]).unwrap();
        // The two things a work tree cannot be trusted to hold on every system — a file that
        // may be run, a link — are written into the index as git itself records them.
        git(&["update-index", "--chmod=+x", "run.sh"]).unwrap();
        let blob = {
            use std::io::Write as _;
            let mut child = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(["hash-object", "-w", "--stdin"])
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(b"readme.md").unwrap();
            let out = child.wait_with_output().unwrap();
            String::from_utf8_lossy(&out.stdout).trim().to_string()
        };
        git(&[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{blob},latest"),
        ])
        .unwrap();
        git(&["commit", "-q", "-m", "first"]).unwrap();
        git(&["tag", "v1"]).unwrap();
        git(&["switch", "-q", "-c", "other"]).unwrap();
        std::fs::write(root.join("readme.md"), "# on the other branch\n").unwrap();
        std::fs::write(root.join("src/new.rs"), "pub fn two() {}\n").unwrap();
        git(&["add", "readme.md", "src/new.rs"]).unwrap();
        git(&["commit", "-q", "-m", "second"]).unwrap();
        // Back on `main`, with the link left out of the work tree's business: the tests
        // read commits, and only the index of `main` matters to what is checked out.
        git(&["switch", "-q", "main"]).unwrap();
        Some(dir)
    }

    fn open(reference: &GitRef, scratch: &Path) -> Archive {
        let descriptor = reference.write_in(scratch).unwrap();
        Archive::open(&descriptor, &Limits::default(), &CancelToken::new()).unwrap()
    }

    #[test]
    fn a_branch_is_listed_and_read_without_being_checked_out() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let other = resolve(repo.path(), "other").unwrap();
        assert_eq!(other.name, "other");
        assert_eq!(other.commit.len(), 40);
        let tree = open(&other, scratch.path());
        assert_eq!(tree.label(), "git");

        let names = |dir: &str| -> Vec<String> {
            tree.list(dir)
                .iter()
                .map(|e| e.name_str().into_owned())
                .collect()
        };
        assert_eq!(names("/"), ["latest", "readme.md", "run.sh", "src"]);
        assert_eq!(names("/src"), ["lib.rs", "new.rs"]);
        assert!(tree.is_dir("/src"));

        // The bytes are the other branch's, while the work tree still holds `main`'s.
        let cancel = CancelToken::new();
        assert_eq!(
            tree.read("/readme.md", &cancel).unwrap(),
            b"# on the other branch\n"
        );
        assert_eq!(
            std::fs::read(repo.path().join("readme.md")).unwrap(),
            b"# on main\n"
        );
        assert_eq!(
            tree.read("/src/new.rs", &cancel).unwrap(),
            b"pub fn two() {}\n"
        );
        assert!(!repo.path().join("src/new.rs").exists());

        // `main` is a different tree of the same repository.
        let main = open(&resolve(repo.path(), "main").unwrap(), scratch.path());
        assert_eq!(main.read("/readme.md", &cancel).unwrap(), b"# on main\n");
        assert!(main.entry("/src/new.rs").is_none());
    }

    #[test]
    fn the_executable_bit_a_link_and_the_size_are_what_git_recorded() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let tree = open(&resolve(repo.path(), "main").unwrap(), scratch.path());

        let run = tree.row("/run.sh").unwrap();
        assert!(run.flags.contains(fcmd_core::EntryFlags::EXECUTABLE));
        assert_eq!(run.size, 18);
        assert!(
            run.mtime.is_some(),
            "the commit's time stands for the file's"
        );
        let readme = tree.row("/readme.md").unwrap();
        assert!(!readme.flags.contains(fcmd_core::EntryFlags::EXECUTABLE));

        let link = tree.entry("/latest").unwrap();
        assert_eq!(link.kind, EntryKind::Symlink);
        assert_eq!(link.link_target.as_deref(), Some("readme.md"));
    }

    #[test]
    fn a_file_is_copied_out_to_the_disk() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let tree = open(&resolve(repo.path(), "other").unwrap(), scratch.path());
        let out = scratch.path().join("out/new.rs");
        let mut seen = 0;
        let written = tree
            .extract("/src/new.rs", &out, &CancelToken::new(), &mut |n| seen = n)
            .unwrap();
        assert_eq!(written, 16);
        assert_eq!(seen, 16);
        assert_eq!(std::fs::read(&out).unwrap(), b"pub fn two() {}\n");

        // A name the tree does not hold is refused before git is asked.
        assert!(tree
            .extract("/nothing", &out, &CancelToken::new(), &mut |_| {})
            .is_err());
    }

    #[test]
    fn the_commit_is_fixed_when_the_ref_is_resolved() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let before = resolve(repo.path(), "main").unwrap();
        let descriptor = before.write_in(scratch.path()).unwrap();
        // `main` moves on; the descriptor still names the commit it was written for.
        std::fs::write(repo.path().join("readme.md"), "# main, later\n").unwrap();
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .args(args)
                .output()
                .unwrap()
        };
        git(&["commit", "-q", "-a", "-m", "third"]);
        assert_ne!(resolve(repo.path(), "main").unwrap().commit, before.commit);

        let cache = Cache::new(Limits::default());
        let tree = cache.open(&descriptor, &CancelToken::new()).unwrap();
        assert_eq!(
            tree.read("/readme.md", &CancelToken::new()).unwrap(),
            b"# on main\n"
        );
        assert_eq!(GitRef::read(&descriptor), Some(before));
    }

    #[test]
    fn a_tag_and_a_hash_are_refs_too() {
        let Some(repo) = repository() else { return };
        let main = resolve(repo.path(), "main").unwrap();
        assert_eq!(resolve(repo.path(), "v1").unwrap().commit, main.commit);
        let short = &main.commit[..10];
        assert_eq!(resolve(repo.path(), short).unwrap().commit, main.commit);
        // From a folder inside the repository the answer is the same, and names its top.
        let inside = resolve(&repo.path().join("src"), "main").unwrap();
        assert_eq!(inside.commit, main.commit);
        assert_eq!(inside.repo.file_name(), main.repo.file_name());
    }

    #[test]
    fn a_ref_git_does_not_know_and_a_folder_that_is_no_repository_say_so() {
        let Some(repo) = repository() else { return };
        assert_eq!(
            resolve(repo.path(), "no-such-branch"),
            Err(GitError::UnknownRef("no-such-branch".to_string()))
        );
        // A file's name is not a commit, even where the file exists.
        assert!(matches!(
            resolve(repo.path(), "main:readme.md"),
            Err(GitError::UnknownRef(_))
        ));
        // A folder with no repository around it. Where the temporary directory is itself
        // inside somebody's repository there is no such folder to make, and nothing to say.
        let plain = tempfile::tempdir().unwrap();
        let cancel = CancelToken::new();
        if run(plain.path(), &["rev-parse", "--git-dir"], None, &cancel).is_ok() {
            return;
        }
        assert_eq!(
            resolve(plain.path(), "main"),
            Err(GitError::NotARepository(plain.path().to_path_buf()))
        );
    }

    #[test]
    fn a_ref_that_starts_with_a_dash_is_refused_before_git_is_asked() {
        // No repository is needed, and none is touched: the refusal is this program's.
        let nowhere = tempfile::tempdir().unwrap();
        for bad in ["-h", "--upload-pack=touch x", "", "  ", "a\nb"] {
            assert_eq!(
                resolve(nowhere.path(), bad),
                Err(GitError::BadRef(bad.trim().to_string())),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn a_descriptor_is_only_what_this_program_wrote() {
        let dir = tempfile::tempdir().unwrap();
        let repo = dir.path().to_path_buf();
        let good = GitRef {
            repo: repo.clone(),
            commit: "a".repeat(40),
            name: "feature/x".to_string(),
        };
        let path = good.write_in(dir.path()).unwrap();
        // The `/` of the ref is not a folder in the descriptor's name.
        assert_eq!(path.parent(), Some(dir.path()));
        assert!(named_like(&path));
        assert_eq!(GitRef::read(&path), Some(good.clone()));
        assert_eq!(good.short(), "aaaaaaaa");

        // A commit that is not a hash is the argument git would be handed: refused.
        for commit in ["--help", "main", "abc", &"g".repeat(40)] {
            let text = format!("{MAGIC}\nrepo={}\ncommit={commit}\nref=x\n", repo.display());
            assert_eq!(GitRef::parse(&text), None, "{commit}");
        }
        assert_eq!(GitRef::parse("something else\n"), None);
        assert_eq!(
            GitRef::parse(&format!(
                "{MAGIC}\nrepo=relative\ncommit={}\n",
                "a".repeat(40)
            )),
            None
        );
        // Under another name the same text is an ordinary file.
        let plain = dir.path().join("notes.txt");
        std::fs::write(&plain, good.to_text()).unwrap();
        assert_eq!(GitRef::read(&plain), None);
        // And a file with the name and other contents is not an archive of any kind.
        let fake = dir.path().join("fake.gitref");
        std::fs::write(&fake, "hello").unwrap();
        assert!(matches!(
            Archive::open(&fake, &Limits::default(), &CancelToken::new()),
            Err(ArcError::Unknown(_))
        ));
    }

    #[test]
    fn nothing_is_written_into_a_tree() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let descriptor = resolve(repo.path(), "main")
            .unwrap()
            .write_in(scratch.path())
            .unwrap();
        let file = scratch.path().join("extra.txt");
        std::fs::write(&file, "x").unwrap();
        for change in [
            crate::Change::add(&file, "/"),
            crate::Change::Remove {
                inner: "/readme.md".to_string(),
            },
        ] {
            let error = crate::rebuild(
                &descriptor,
                &[change],
                crate::Level::default(),
                &CancelToken::new(),
                &mut |_, _| {},
            )
            .unwrap_err();
            assert!(
                matches!(&error, ArcError::Refused { reason, .. } if reason.contains("read only")),
                "{error}"
            );
        }
        // The descriptor is as it was, and so is the repository.
        assert!(GitRef::read(&descriptor).is_some());
    }

    #[test]
    fn a_submodule_is_a_folder_whose_files_are_elsewhere() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        // A gitlink is an entry of mode 160000 naming a commit of another repository; any
        // hash will do for one, since nothing here ever follows it.
        let pinned = "1234567890123456789012345678901234567890";
        let git = |args: &[&str]| {
            Command::new("git")
                .arg("-C")
                .arg(repo.path())
                .args(args)
                .output()
                .unwrap()
        };
        git(&[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{pinned},vendor/lib"),
        ]);
        git(&["commit", "-q", "-m", "a submodule"]);
        let tree = open(&resolve(repo.path(), "main").unwrap(), scratch.path());
        assert!(tree.is_dir("/vendor/lib"));
        assert!(tree.list("/vendor/lib").is_empty());
        assert_eq!(tree.submodule("/vendor/lib"), Some(pinned));
        assert_eq!(tree.submodule("/src"), None);
        // It is a folder, so there are no bytes to hand over.
        assert!(tree.read("/vendor/lib", &CancelToken::new()).is_err());
    }

    #[test]
    fn a_tree_over_the_limit_of_entries_is_refused() {
        let Some(repo) = repository() else { return };
        let scratch = tempfile::tempdir().unwrap();
        let descriptor = resolve(repo.path(), "main")
            .unwrap()
            .write_in(scratch.path())
            .unwrap();
        let tight = Limits {
            entries: 2,
            ..Limits::default()
        };
        let error = Archive::open(&descriptor, &tight, &CancelToken::new()).unwrap_err();
        assert!(matches!(error, ArcError::Refused { .. }), "{error}");
    }
}
