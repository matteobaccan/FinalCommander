//! The branch view: every file under a folder, as one flat list (SPEC 02 §3d).
//!
//! Total Commander's `Ctrl+B`. The walk is the job's own filesystem, so it works wherever a
//! copy does — a server, a bucket, inside an archive — and it is bounded: a branch of a whole
//! disk is a list nobody could read, so it stops at a count and says it stopped.

use fcmd_core::{CancelToken, VPath};

use crate::fs::{Facts, JobFs};

/// One file of the branch: where it is, what it is called from the root down, and its facts.
#[derive(Debug, Clone)]
pub struct Leaf {
    pub path: VPath,
    /// `sub/dir/name`, from the folder the branch was taken of.
    pub relative: String,
    pub facts: Facts,
}

/// What the walk found, and whether it stopped short of the end.
#[derive(Debug, Clone, Default)]
pub struct Branch {
    pub leaves: Vec<Leaf>,
    /// The walk stopped at `limit` files, or at a cancel.
    pub truncated: bool,
    /// Folders that could not be read, which are left out rather than ending the walk.
    pub unreadable: usize,
}

/// Every file under `root`, depth first, folders before their contents' turn comes, names in
/// the order the filesystem gives them (the panel sorts them afterwards). Directories are
/// walked and not listed — the view is of files — and links are listed and never followed,
/// so a link back up the tree cannot make the walk go round for ever.
pub fn walk(fs: &dyn JobFs, root: &VPath, limit: usize, cancel: &CancelToken) -> Branch {
    let mut out = Branch::default();
    let mut stack: Vec<(VPath, String, u32)> = vec![(root.clone(), String::new(), 0)];
    while let Some((dir, prefix, depth)) = stack.pop() {
        if cancel.is_cancelled() {
            out.truncated = true;
            break;
        }
        let Ok(mut entries) = fs.read_dir(&dir) else {
            out.unreadable += 1;
            continue;
        };
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        let mut subdirs = Vec::new();
        for (name, facts) in entries {
            let shown = name.to_string_lossy().into_owned();
            let relative = if prefix.is_empty() {
                shown
            } else {
                format!("{prefix}/{shown}")
            };
            match facts.kind {
                fcmd_core::EntryKind::Parent => {}
                fcmd_core::EntryKind::Dir if depth < 64 => {
                    subdirs.push((dir.join(&name), relative, depth + 1));
                }
                fcmd_core::EntryKind::Dir => {}
                _ => {
                    if out.leaves.len() >= limit {
                        out.truncated = true;
                        return out;
                    }
                    out.leaves.push(Leaf {
                        path: dir.join(&name),
                        relative,
                        facts,
                    });
                }
            }
        }
        // Reversed onto the stack, so they come off in name order.
        stack.extend(subdirs.into_iter().rev());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFs;

    #[test]
    fn every_file_under_the_folder_is_listed_by_its_path_from_there() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/deep")).unwrap();
        std::fs::create_dir(dir.path().join("empty")).unwrap();
        std::fs::write(dir.path().join("README"), "r").unwrap();
        std::fs::write(dir.path().join("src/main.rs"), "m").unwrap();
        std::fs::write(dir.path().join("src/deep/x.rs"), "xx").unwrap();
        let branch = walk(
            &LocalFs,
            &VPath::local(dir.path()),
            100,
            &CancelToken::new(),
        );
        let names: Vec<&str> = branch.leaves.iter().map(|l| l.relative.as_str()).collect();
        assert_eq!(names, ["README", "src/main.rs", "src/deep/x.rs"]);
        assert!(!branch.truncated);
        assert_eq!(branch.leaves[2].facts.size, 2);
        assert_eq!(
            branch.leaves[2].path,
            VPath::local(dir.path().join("src/deep/x.rs"))
        );
    }

    #[test]
    fn a_branch_too_big_to_read_stops_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        for i in 0..10 {
            std::fs::write(dir.path().join(format!("f{i}")), "").unwrap();
        }
        let branch = walk(&LocalFs, &VPath::local(dir.path()), 4, &CancelToken::new());
        assert_eq!(branch.leaves.len(), 4);
        assert!(branch.truncated);
        let cancel = CancelToken::new();
        cancel.cancel();
        assert!(walk(&LocalFs, &VPath::local(dir.path()), 100, &cancel).truncated);
    }
}
