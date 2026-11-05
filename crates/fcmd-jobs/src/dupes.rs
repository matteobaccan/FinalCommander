//! Files with the same content (FAR's FindDup, SPEC 06 §13).
//!
//! Three passes, each one only over what the one before left standing, so a folder of a
//! hundred thousand files costs a hundred thousand `stat`s and very few reads:
//!
//! 1. **Size.** A file is a candidate only if another file has exactly its size.
//! 2. **The first 64 KiB.** Hashed, which splits almost every group of same-sized files that
//!    are not copies — two photos, two builds — after reading a sliver of each.
//! 3. **The whole file**, SHA-256, for what is still together.
//!
//! The walk is the job's own filesystem, as the branch view's is, so it works wherever a copy
//! does; links are never followed, and empty files are left out — they are all "the same",
//! and a list of every empty file on a disk is not what anyone was looking for.

use std::collections::HashMap;

use fcmd_core::{CancelToken, VPath};

use crate::fs::{Facts, JobFs};

/// How much of each file the second pass reads.
pub const HEAD: usize = 64 * 1024;

/// What to look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Files smaller than this are not looked at. `1` leaves out the empty ones only.
    pub min_size: u64,
    /// Into subfolders, or only the folders named.
    pub recursive: bool,
    /// Stop collecting files after this many, and say so.
    pub limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            min_size: 1,
            recursive: true,
            limit: 1_000_000,
        }
    }
}

/// One file of a group.
#[derive(Debug, Clone)]
pub struct Copy {
    pub path: VPath,
    pub facts: Facts,
}

/// Files with the same content: at least two, the oldest first, so "keep the first" keeps
/// what was there before the copies were made.
#[derive(Debug, Clone)]
pub struct Group {
    pub size: u64,
    /// The SHA-256 every file of the group has, in hex.
    pub hash: String,
    pub copies: Vec<Copy>,
}

impl Group {
    /// What deleting all but one of them would give back.
    pub fn wasted(&self) -> u64 {
        self.size * (self.copies.len() as u64 - 1)
    }
}

/// What the search found.
#[derive(Debug, Clone, Default)]
pub struct Found {
    /// The groups, the most space wasted first.
    pub groups: Vec<Group>,
    /// How many files were looked at.
    pub scanned: usize,
    /// The walk stopped at the limit, or at a cancel.
    pub truncated: bool,
    /// Folders that could not be listed and files that could not be read, left out.
    pub unreadable: usize,
}

/// Where the search has got to, for a progress line: the pass (1 to 3), and how far into it.
pub type Progress<'a> = &'a mut dyn FnMut(u8, usize, usize);

/// Find the files under `roots` that have the same content.
pub fn find(
    fs: &dyn JobFs,
    roots: &[VPath],
    opts: &Options,
    cancel: &CancelToken,
    progress: Progress,
) -> Found {
    let mut out = Found::default();
    let mut files: Vec<Copy> = Vec::new();
    // A root given twice, or a root inside another, must not make a file its own duplicate.
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stack: Vec<(VPath, u32)> = roots.iter().map(|r| (r.clone(), 0)).collect();
    // A root that is a file is a file to compare, not a folder to list.
    stack.retain(|(root, _)| match fs.facts(root) {
        Ok(facts) if !facts.is_dir() => {
            if facts.size >= opts.min_size && seen.insert(root.to_string()) {
                files.push(Copy {
                    path: root.clone(),
                    facts,
                });
            }
            false
        }
        _ => true,
    });
    'walk: while let Some((dir, depth)) = stack.pop() {
        if cancel.is_cancelled() {
            out.truncated = true;
            break;
        }
        let Ok(entries) = fs.read_dir(&dir) else {
            out.unreadable += 1;
            continue;
        };
        for (name, facts) in entries {
            match facts.kind {
                fcmd_core::EntryKind::Parent => {}
                fcmd_core::EntryKind::Dir => {
                    if opts.recursive && depth < 64 {
                        stack.push((dir.join(&name), depth + 1));
                    }
                }
                fcmd_core::EntryKind::File => {
                    if facts.size < opts.min_size {
                        continue;
                    }
                    let path = dir.join(&name);
                    if !seen.insert(path.to_string()) {
                        continue;
                    }
                    if files.len() >= opts.limit {
                        out.truncated = true;
                        break 'walk;
                    }
                    files.push(Copy { path, facts });
                    if files.len().is_multiple_of(512) {
                        progress(1, files.len(), 0);
                    }
                }
                // Links, devices, sockets: never followed, never compared.
                _ => {}
            }
        }
    }
    out.scanned = files.len();

    // Pass 1: by size.
    let mut by_size: HashMap<u64, Vec<Copy>> = HashMap::new();
    for file in files {
        by_size.entry(file.facts.size).or_default().push(file);
    }
    let candidates: Vec<Vec<Copy>> = by_size.into_values().filter(|g| g.len() > 1).collect();

    // Pass 2: by the first 64 KiB.
    let total: usize = candidates.iter().map(Vec::len).sum();
    let mut done = 0usize;
    let mut by_head: Vec<Vec<Copy>> = Vec::new();
    for group in candidates {
        let mut split: HashMap<String, Vec<Copy>> = HashMap::new();
        for file in group {
            if cancel.is_cancelled() {
                out.truncated = true;
                return out;
            }
            done += 1;
            progress(2, done, total);
            match hash(fs, &file.path, Some(HEAD), cancel) {
                Ok(h) => split.entry(h).or_default().push(file),
                Err(_) if cancel.is_cancelled() => {
                    out.truncated = true;
                    return out;
                }
                Err(_) => out.unreadable += 1,
            }
        }
        by_head.extend(split.into_values().filter(|g| g.len() > 1));
    }

    // Pass 3: the whole file — unless the whole file was the first 64 KiB, which pass 2 has
    // already read in full.
    let total: usize = by_head.iter().map(Vec::len).sum();
    let mut done = 0usize;
    for group in by_head {
        let size = group[0].facts.size;
        let mut split: HashMap<String, Vec<Copy>> = HashMap::new();
        for file in group {
            if cancel.is_cancelled() {
                out.truncated = true;
                break;
            }
            done += 1;
            progress(3, done, total);
            match hash(fs, &file.path, None, cancel) {
                Ok(h) => split.entry(h).or_default().push(file),
                Err(_) if cancel.is_cancelled() => {
                    out.truncated = true;
                    break;
                }
                Err(_) => out.unreadable += 1,
            }
        }
        for (hash, mut copies) in split {
            if copies.len() < 2 {
                continue;
            }
            copies.sort_by(|a, b| {
                a.facts
                    .mtime
                    .cmp(&b.facts.mtime)
                    .then_with(|| a.path.to_string().cmp(&b.path.to_string()))
            });
            out.groups.push(Group { size, hash, copies });
        }
        if out.truncated {
            break;
        }
    }
    out.groups.sort_by(|a, b| {
        b.wasted().cmp(&a.wasted()).then_with(|| {
            a.copies[0]
                .path
                .to_string()
                .cmp(&b.copies[0].path.to_string())
        })
    });
    out
}

/// SHA-256 of a file, or of its first `head` bytes.
fn hash(
    fs: &dyn JobFs,
    path: &VPath,
    head: Option<usize>,
    cancel: &CancelToken,
) -> std::io::Result<String> {
    use sha2::Digest;
    let mut sha = sha2::Sha256::new();
    let mut read = 0usize;
    let mut enough = false;
    let result = fs.read_chunks(path, HEAD, &mut |chunk| {
        let take = match head {
            Some(limit) => chunk.len().min(limit - read),
            None => chunk.len(),
        };
        sha.update(&chunk[..take]);
        read += take;
        if head.is_some_and(|limit| read >= limit) {
            enough = true;
            return false;
        }
        !cancel.is_cancelled()
    });
    match result {
        Ok(_) => {}
        // Stopped on purpose at the head: the read said "interrupted", which here is done.
        Err(_) if enough => {}
        Err(e) => return Err(e),
    }
    Ok(crate::sum::hex(sha.finalize().as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFs;

    fn none(_: u8, _: usize, _: usize) {}

    #[test]
    fn copies_are_found_by_content_and_the_oldest_comes_first() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        let big: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
        let mut other = big.clone();
        // Same size, same first 64 KiB, different end: only the third pass can tell.
        *other.last_mut().unwrap() ^= 1;
        std::fs::write(dir.path().join("b.bin"), &big).unwrap();
        std::fs::write(dir.path().join("sub/a.bin"), &big).unwrap();
        std::fs::write(dir.path().join("c.bin"), &other).unwrap();
        std::fs::write(dir.path().join("x.txt"), "same").unwrap();
        std::fs::write(dir.path().join("sub/y.txt"), "same").unwrap();
        std::fs::write(dir.path().join("z.txt"), "diff").unwrap();
        std::fs::write(dir.path().join("e1"), "").unwrap();
        std::fs::write(dir.path().join("e2"), "").unwrap();
        let old = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000);
        let f = std::fs::File::options()
            .write(true)
            .open(dir.path().join("sub/a.bin"))
            .unwrap();
        f.set_modified(old).unwrap();
        drop(f);

        let found = find(
            &LocalFs,
            &[VPath::local(dir.path())],
            &Options::default(),
            &CancelToken::new(),
            &mut none,
        );
        assert_eq!(found.scanned, 6, "the empty files are left out");
        assert_eq!(found.groups.len(), 2, "{:?}", found.groups);
        let first = &found.groups[0];
        assert_eq!(first.size, 200_000, "the most space wasted first");
        let names: Vec<String> = first.copies.iter().map(|c| c.path.to_string()).collect();
        assert!(names[0].ends_with("a.bin"), "the oldest first: {names:?}");
        assert!(names[1].ends_with("b.bin"));
        assert_eq!(first.wasted(), 200_000);
        assert_eq!(found.groups[1].copies.len(), 2);
    }

    #[test]
    fn the_same_folder_twice_is_not_its_own_duplicate_and_the_flat_search_stays_flat() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("sub")).unwrap();
        std::fs::write(dir.path().join("a"), "one").unwrap();
        std::fs::write(dir.path().join("sub/b"), "one").unwrap();
        let root = VPath::local(dir.path());
        let found = find(
            &LocalFs,
            &[root.clone(), root.clone()],
            &Options {
                recursive: false,
                ..Options::default()
            },
            &CancelToken::new(),
            &mut none,
        );
        assert!(found.groups.is_empty(), "{:?}", found.groups);
        let found = find(
            &LocalFs,
            &[root.clone(), root],
            &Options::default(),
            &CancelToken::new(),
            &mut none,
        );
        assert_eq!(found.groups.len(), 1);
        assert_eq!(found.groups[0].copies.len(), 2);
    }

    #[test]
    fn small_files_can_be_left_out_and_a_cancel_stops_it() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a"), "one").unwrap();
        std::fs::write(dir.path().join("b"), "one").unwrap();
        let root = [VPath::local(dir.path())];
        let opts = Options {
            min_size: 4,
            ..Options::default()
        };
        assert!(find(&LocalFs, &root, &opts, &CancelToken::new(), &mut none)
            .groups
            .is_empty());
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(find(&LocalFs, &root, &Options::default(), &cancel, &mut none).truncated);
    }
}
