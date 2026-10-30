//! What a directory's git repository is doing, read from its files (SPEC 02 §9).
//!
//! A commander is where one *looks* at a working copy, and the question somebody has standing
//! in one is always the same three: which branch, at which commit, and is there anything
//! uncommitted. The information panel is where that belongs, next to the free space and the
//! mount.
//!
//! **Nothing is run.** `git` need not be installed, and a panel that starts a process every
//! time a directory changes is a panel that makes a slow machine slower. Everything here is
//! small files: `HEAD`, a ref, `packed-refs`, the marker a merge or a rebase leaves behind,
//! and the index — which is the one that costs anything, and is capped.
//!
//! What this deliberately does **not** do, because each would cost far more than the answer is
//! worth in a panel:
//!
//! - **hash a file.** The index keeps each file's size and modification time, and comparing
//!   those is one `stat` per tracked file. So a file whose bytes were rewritten identically,
//!   or merely touched, counts here as differing — which is why the panel says *differs from
//!   the index* and not *modified*. Git itself resolves exactly this case by hashing, after
//!   the same comparison has told it which files are worth hashing.
//! - **look for untracked files.** That means walking the tree and reading every
//!   `.gitignore` on the way down, which is a search and not a glance.
//! - **compare the index with `HEAD`**, which means reading commits and trees out of the
//!   object store, half of them inside pack files.

use std::path::{Path, PathBuf};

/// The most tracked files we are willing to `stat` for one glance at a panel. A repository
/// larger than this still gets its branch and its commit, and says that it did not count.
const MAX_INDEX: usize = 20_000;

/// Something the repository is in the middle of, which is worth saying because it changes what
/// the next keystroke ought to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    Merge,
    Rebase,
    CherryPick,
    Revert,
    Bisect,
}

/// What the index says, when it could be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Index {
    /// How many files git is keeping track of.
    pub tracked: usize,
    /// How many of them differ from what the index recorded, by size or by modification
    /// time — see the module's note on what that means and does not mean.
    pub differing: usize,
}

/// A repository, as far as its own files will say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// The working tree's top directory — the one holding `.git`.
    pub root: PathBuf,
    /// Where the repository's own files are, which is not under `root` for a worktree or a
    /// submodule.
    pub git_dir: PathBuf,
    /// The branch `HEAD` is on. `None` when the head is detached.
    pub branch: Option<String>,
    /// The commit it is at, the seven characters everything else shows.
    pub head: Option<String>,
    pub state: Option<State>,
    /// `None` when the index could not be read, or holds more files than one glance pays for.
    pub index: Option<Index>,
}

impl Repo {
    /// `main`, or the commit for a detached head, or nothing at all for a repository with no
    /// commit in it yet.
    pub fn name(&self) -> String {
        match (&self.branch, &self.head) {
            (Some(branch), _) => branch.clone(),
            (None, Some(head)) => head.clone(),
            (None, None) => String::new(),
        }
    }

    /// Is the working tree one `git commit` away from being the same as the index?
    pub fn is_clean(&self) -> bool {
        self.index.is_some_and(|i| i.differing == 0)
    }
}

/// Describe the repository the directory is in, or `None` when it is not in one.
///
/// The walk upwards stops at the first `.git`, which may be a file rather than a directory —
/// as it is in a worktree and in a submodule — and then it says where the real one is.
pub fn describe(dir: &Path) -> Option<Repo> {
    let (root, git_dir) = find(dir)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim().to_string();
    let (branch, commit) = match head.strip_prefix("ref:") {
        Some(reference) => {
            let reference = reference.trim();
            let branch = reference
                .strip_prefix("refs/heads/")
                .unwrap_or(reference)
                .to_string();
            (Some(branch), resolve(&git_dir, reference))
        }
        // Detached: the file holds the commit itself.
        None if is_hex(&head) => (None, Some(head.clone())),
        None => (None, None),
    };
    Some(Repo {
        index: read_index(&git_dir, &root),
        root,
        state: state_of(&git_dir),
        branch,
        head: commit.map(|c| c.chars().take(7).collect()),
        git_dir,
    })
}

/// The branch a directory is on — and nothing else, for the caller that wants a word and not
/// a report. The command line's prompt is redrawn on every keystroke, so it takes this road:
/// two small reads and no `stat` of anything.
pub fn branch_of(dir: &Path) -> Option<String> {
    let (_, git_dir) = find(dir)?;
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    match head.strip_prefix("ref:") {
        Some(reference) => {
            let reference = reference.trim();
            Some(
                reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_string(),
            )
        }
        // Detached: the file holds the commit itself, and seven characters of it is what
        // everything else that shows a commit shows.
        None if is_hex(head) => Some(head.chars().take(7).collect()),
        None => None,
    }
}

/// The working tree's root and the repository's own directory.
fn find(dir: &Path) -> Option<(PathBuf, PathBuf)> {
    let mut at = Some(dir);
    while let Some(here) = at {
        let dot = here.join(".git");
        if dot.is_dir() {
            return Some((here.to_path_buf(), dot));
        }
        if dot.is_file() {
            // `gitdir: /path/to/.git/worktrees/name`
            let text = std::fs::read_to_string(&dot).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let target = Path::new(target);
            let target = if target.is_absolute() {
                target.to_path_buf()
            } else {
                here.join(target)
            };
            return Some((here.to_path_buf(), target));
        }
        at = here.parent();
    }
    None
}

/// A reference's commit: its own file, else the packed list every repository that has been
/// garbage-collected keeps instead.
fn resolve(git_dir: &Path, reference: &str) -> Option<String> {
    if let Ok(text) = std::fs::read_to_string(git_dir.join(reference)) {
        let text = text.trim();
        if is_hex(text) {
            return Some(text.to_string());
        }
        // A symbolic ref pointing at another, which is rare and still legal.
        if let Some(next) = text.strip_prefix("ref:") {
            return resolve(git_dir, next.trim());
        }
    }
    let packed = std::fs::read_to_string(git_dir.join("packed-refs")).ok()?;
    for line in packed.lines() {
        if line.starts_with('#') || line.starts_with('^') {
            continue;
        }
        let (commit, name) = line.split_once(' ')?;
        if name.trim() == reference && is_hex(commit) {
            return Some(commit.to_string());
        }
    }
    None
}

fn is_hex(text: &str) -> bool {
    text.len() >= 7 && text.chars().all(|c| c.is_ascii_hexdigit())
}

/// What the repository is in the middle of, which it says by leaving a file or a directory
/// where the next command will look for it.
fn state_of(git_dir: &Path) -> Option<State> {
    // Rebase first: a rebase that stops on a conflict also leaves `MERGE_MSG` behind, and
    // calling that a merge would be the wrong word for what is happening.
    for (name, state) in [
        ("rebase-merge", State::Rebase),
        ("rebase-apply", State::Rebase),
    ] {
        if git_dir.join(name).is_dir() {
            return Some(state);
        }
    }
    for (name, state) in [
        ("MERGE_HEAD", State::Merge),
        ("CHERRY_PICK_HEAD", State::CherryPick),
        ("REVERT_HEAD", State::Revert),
        ("BISECT_LOG", State::Bisect),
    ] {
        if git_dir.join(name).exists() {
            return Some(state);
        }
    }
    None
}

/// Read the index and compare each entry with the file it stands for.
///
/// The format is `DIRC`, a version, a count, and then one record per file: the `stat` data git
/// saw when the file was added, the blob it hashed to, and the path. Versions 2 and 3 are
/// read; version 4 compresses the paths against each other, which nothing writes unless it is
/// asked to, and is reported as "not counted" rather than guessed at.
fn read_index(git_dir: &Path, root: &Path) -> Option<Index> {
    let bytes = std::fs::read(git_dir.join("index")).ok()?;
    if bytes.len() < 12 || &bytes[..4] != b"DIRC" {
        return None;
    }
    let version = u32::from_be_bytes(bytes[4..8].try_into().ok()?);
    if !(2..=3).contains(&version) {
        return None;
    }
    let count = u32::from_be_bytes(bytes[8..12].try_into().ok()?) as usize;
    if count > MAX_INDEX {
        return None;
    }
    let mut at = 12usize;
    let mut tracked = 0usize;
    let mut differing = 0usize;
    for _ in 0..count {
        // 62 bytes of fixed fields: two timestamps, the device and inode, the mode, the owner,
        // the size, the hash, and the flags.
        if at + 62 > bytes.len() {
            break;
        }
        let mtime = u32::from_be_bytes(bytes[at + 8..at + 12].try_into().ok()?);
        let size = u32::from_be_bytes(bytes[at + 36..at + 40].try_into().ok()?);
        let flags = u16::from_be_bytes(bytes[at + 60..at + 62].try_into().ok()?);
        let extended = flags & 0x4000 != 0;
        let stage = (flags >> 12) & 0x3;
        let name_len = (flags & 0x0FFF) as usize;
        let mut p = at + 62;
        if extended {
            p += 2;
        }
        // A name of 0xFFF or longer is not counted in the flags and runs to the NUL.
        let name_end = if name_len < 0x0FFF {
            (p + name_len).min(bytes.len())
        } else {
            bytes[p..]
                .iter()
                .position(|b| *b == 0)
                .map(|n| p + n)
                .unwrap_or(bytes.len())
        };
        let name = &bytes[p..name_end];
        // Every record is padded with NULs to a multiple of eight bytes.
        let unpadded = name_end + 1 - at;
        at += unpadded.div_ceil(8) * 8;

        // A path in a conflict is in the index three times over; count the file once.
        if stage != 0 {
            continue;
        }
        tracked += 1;
        let path = root.join(String::from_utf8_lossy(name).as_ref());
        match std::fs::symlink_metadata(&path) {
            Err(_) => differing += 1,
            Ok(meta) => {
                let same_size = meta.len() == u64::from(size);
                let same_time = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as u32 == mtime)
                    .unwrap_or(false);
                if !same_size || !same_time {
                    differing += 1;
                }
            }
        }
    }
    Some(Index { tracked, differing })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    /// A repository's files, written by hand: this tests the reader, and a reader tested
    /// against `git` would be a test of whichever `git` the machine happens to have.
    struct Fake {
        dir: tempfile::TempDir,
    }

    impl Fake {
        fn new() -> Fake {
            let fake = Fake {
                dir: tempfile::tempdir().unwrap(),
            };
            std::fs::create_dir_all(fake.git().join("refs/heads")).unwrap();
            fake.write(".git/HEAD", b"ref: refs/heads/main\n");
            fake.write(
                ".git/refs/heads/main",
                b"3a7f2e1c9d4b5a6f8e2d1c0b9a8f7e6d5c4b3a2f\n",
            );
            fake
        }

        fn root(&self) -> PathBuf {
            self.dir.path().to_path_buf()
        }

        fn git(&self) -> PathBuf {
            self.root().join(".git")
        }

        fn write(&self, rel: &str, bytes: &[u8]) {
            let path = self.root().join(rel);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let mut f = std::fs::File::create(path).unwrap();
            f.write_all(bytes).unwrap();
        }

        /// An index holding the files named, each recorded with the size and time the file
        /// has right now — which is what `git add` would have written.
        fn index(&self, names: &[&str]) {
            let mut out = b"DIRC".to_vec();
            out.extend(2u32.to_be_bytes());
            out.extend((names.len() as u32).to_be_bytes());
            for name in names {
                let meta = std::fs::metadata(self.root().join(name)).unwrap();
                let mtime = meta
                    .modified()
                    .unwrap()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as u32;
                let start = out.len();
                out.extend(mtime.to_be_bytes()); // ctime seconds
                out.extend(0u32.to_be_bytes()); // ctime nanoseconds
                out.extend(mtime.to_be_bytes()); // mtime seconds
                out.extend(0u32.to_be_bytes()); // mtime nanoseconds
                out.extend(0u32.to_be_bytes()); // device
                out.extend(0u32.to_be_bytes()); // inode
                out.extend(0o100_644u32.to_be_bytes()); // mode
                out.extend(0u32.to_be_bytes()); // uid
                out.extend(0u32.to_be_bytes()); // gid
                out.extend((meta.len() as u32).to_be_bytes());
                out.extend([0u8; 20]); // the blob's hash, which this reader never uses
                out.extend((name.len() as u16).to_be_bytes()); // flags: stage 0, no extension
                out.extend(name.as_bytes());
                out.push(0);
                while !(out.len() - start).is_multiple_of(8) {
                    out.push(0);
                }
            }
            self.write(".git/index", &out);
        }
    }

    #[test]
    fn a_repository_says_its_branch_and_its_commit() {
        let fake = Fake::new();
        let repo = describe(&fake.root()).expect("this is a repository");
        assert_eq!(repo.branch.as_deref(), Some("main"));
        assert_eq!(repo.head.as_deref(), Some("3a7f2e1"));
        assert_eq!(repo.name(), "main");
        assert_eq!(repo.state, None);
        assert_eq!(repo.root, fake.root());
    }

    #[test]
    fn the_walk_upwards_finds_the_repository_from_inside_it() {
        let fake = Fake::new();
        std::fs::create_dir_all(fake.root().join("src/deep/deeper")).unwrap();
        let repo = describe(&fake.root().join("src/deep/deeper")).unwrap();
        assert_eq!(repo.root, fake.root());
        // And a directory in no repository is not one.
        let elsewhere = tempfile::tempdir().unwrap();
        assert_eq!(describe(elsewhere.path()), None);
    }

    #[test]
    fn a_detached_head_is_the_commit_itself() {
        let fake = Fake::new();
        fake.write(".git/HEAD", b"9f8e7d6c5b4a39281716253443526170f8e9d0c1\n");
        let repo = describe(&fake.root()).unwrap();
        assert_eq!(repo.branch, None);
        assert_eq!(repo.head.as_deref(), Some("9f8e7d6"));
        assert_eq!(repo.name(), "9f8e7d6");
    }

    #[test]
    fn a_commit_that_is_only_in_the_packed_list_is_still_found() {
        let fake = Fake::new();
        std::fs::remove_file(fake.git().join("refs/heads/main")).unwrap();
        fake.write(
            ".git/packed-refs",
            b"# pack-refs with: peeled fully-peeled sorted \n\
              1122334455667788990011223344556677889900 refs/heads/main\n\
              aabbccddeeff00112233445566778899aabbccdd refs/tags/v1\n\
              ^1122334455667788990011223344556677889900\n",
        );
        let repo = describe(&fake.root()).unwrap();
        assert_eq!(repo.head.as_deref(), Some("1122334"));
    }

    #[test]
    fn a_worktree_points_at_the_repository_it_belongs_to() {
        let fake = Fake::new();
        let tree = tempfile::tempdir().unwrap();
        let git_dir = fake.git().join("worktrees/side");
        std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
        std::fs::write(git_dir.join("HEAD"), b"ref: refs/heads/side\n").unwrap();
        std::fs::write(
            git_dir.join("refs/heads/side"),
            b"5566778899001122334455667788990011223344\n",
        )
        .unwrap();
        std::fs::write(
            tree.path().join(".git"),
            format!("gitdir: {}\n", git_dir.display()),
        )
        .unwrap();
        let repo = describe(tree.path()).unwrap();
        assert_eq!(repo.root, tree.path());
        assert_eq!(repo.git_dir, git_dir);
        assert_eq!(repo.branch.as_deref(), Some("side"));
        assert_eq!(repo.head.as_deref(), Some("5566778"));
    }

    #[test]
    fn what_the_repository_is_in_the_middle_of_is_said() {
        let fake = Fake::new();
        fake.write(
            ".git/MERGE_HEAD",
            b"1111111111111111111111111111111111111111\n",
        );
        assert_eq!(describe(&fake.root()).unwrap().state, Some(State::Merge));

        // A rebase that stopped leaves a merge message behind as well, and it is a rebase.
        std::fs::create_dir_all(fake.git().join("rebase-merge")).unwrap();
        assert_eq!(describe(&fake.root()).unwrap().state, Some(State::Rebase));

        std::fs::remove_dir_all(fake.git().join("rebase-merge")).unwrap();
        std::fs::remove_file(fake.git().join("MERGE_HEAD")).unwrap();
        fake.write(".git/BISECT_LOG", b"git bisect start\n");
        assert_eq!(describe(&fake.root()).unwrap().state, Some(State::Bisect));
    }

    #[test]
    fn the_index_counts_the_files_that_differ_from_what_it_recorded() {
        let fake = Fake::new();
        fake.write("one.txt", b"hello");
        fake.write("two.txt", b"world");
        fake.write("dir/three.txt", b"again");
        fake.index(&["one.txt", "two.txt", "dir/three.txt"]);

        let repo = describe(&fake.root()).unwrap();
        let index = repo.index.expect("the index was read");
        assert_eq!(index.tracked, 3);
        assert_eq!(index.differing, 0, "nothing has been touched yet");
        assert!(repo.is_clean());

        // A file that grew differs by size, whatever its timestamp says.
        fake.write("two.txt", b"world, and then some");
        let repo = describe(&fake.root()).unwrap();
        assert_eq!(repo.index.unwrap().differing, 1);
        assert!(!repo.is_clean());

        // And one that is gone differs too.
        std::fs::remove_file(fake.root().join("dir/three.txt")).unwrap();
        let repo = describe(&fake.root()).unwrap();
        assert_eq!(repo.index.unwrap().differing, 2);
        assert_eq!(repo.index.unwrap().tracked, 3);
    }

    #[test]
    fn a_file_in_a_conflict_is_counted_once() {
        let fake = Fake::new();
        fake.write("clash.txt", b"ours");
        // The same path at stages 1, 2 and 3, which is what a conflict looks like in here.
        let mut out = b"DIRC".to_vec();
        out.extend(2u32.to_be_bytes());
        out.extend(3u32.to_be_bytes());
        for stage in 1..=3u16 {
            let start = out.len();
            out.extend([0u8; 36]);
            out.extend(4u32.to_be_bytes()); // size
            out.extend([0u8; 20]);
            out.extend((stage << 12 | 9).to_be_bytes());
            out.extend(b"clash.txt");
            out.push(0);
            while !(out.len() - start).is_multiple_of(8) {
                out.push(0);
            }
        }
        fake.write(".git/index", &out);
        let index = describe(&fake.root()).unwrap().index.unwrap();
        assert_eq!(index.tracked, 0, "every stage of a conflict is above zero");
        assert_eq!(index.differing, 0);
    }

    #[test]
    fn an_index_this_reader_does_not_know_says_so_rather_than_guessing() {
        let fake = Fake::new();
        // Version 4 compresses the paths, which nothing writes unless it is asked to.
        let mut out = b"DIRC".to_vec();
        out.extend(4u32.to_be_bytes());
        out.extend(1u32.to_be_bytes());
        out.extend([0u8; 62]);
        fake.write(".git/index", &out);
        assert_eq!(describe(&fake.root()).unwrap().index, None);

        // And a file that is not an index at all.
        fake.write(".git/index", b"not an index");
        assert_eq!(describe(&fake.root()).unwrap().index, None);

        // A repository with nothing staged has no index file, which is not an error.
        std::fs::remove_file(fake.git().join("index")).unwrap();
        let repo = describe(&fake.root()).unwrap();
        assert_eq!(repo.index, None);
        assert!(!repo.is_clean(), "nothing counted is not the same as clean");
    }
}
