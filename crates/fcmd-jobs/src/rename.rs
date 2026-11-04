//! Doing a multi-rename (SPEC 03 §8): all of it, or none of it.
//!
//! `fcmd_core::rename` works out the names; this does the renames, in one folder, through
//! whatever [`JobFs`] the panel is on. Two things make it more than a loop.
//!
//! **Names that trade places.** `1.txt → 2.txt` and `2.txt → 1.txt` cannot be done in either
//! order, because each one would land on a file that is still there. Every file whose current
//! name is the target of another rename is moved to a temporary name first, so that by the
//! time the real names are written every target is free. Only those files take the detour:
//! a plan with no crossing names is one rename per file, as it would be by hand.
//!
//! **A failure halfway.** A full disk, a permission, a cancelled job: a folder left half in the
//! old names and half in the new ones is the one outcome nobody wanted, and an unknown number
//! of files stranded under temporary names is worse. So every rename done is remembered, and on
//! a failure they are undone in reverse order. What comes back says which rename failed, why,
//! and whether putting the others back worked too.

use std::collections::HashSet;
use std::io;

use fcmd_core::{CancelToken, VPath};

use crate::JobFs;

/// Why a multi-rename did not happen.
#[derive(Debug)]
pub struct RenameError {
    /// The rename that failed, as old and new name.
    pub at: (String, String),
    pub error: io::Error,
    /// Every rename done before it was undone again, so the folder is as it was.
    pub restored: bool,
}

impl std::fmt::Display for RenameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} → {}: {}", self.at.0, self.at.1, self.error)
    }
}

/// Rename the files of `dir` as `plan` says, each `(old, new)` a name in that folder.
///
/// Returns how many files were renamed. On an error nothing is left renamed, unless putting a
/// file back failed as well, which [`RenameError::restored`] says.
pub fn rename_all(
    fs: &dyn JobFs,
    dir: &VPath,
    plan: &[(String, String)],
    cancel: &CancelToken,
) -> Result<usize, RenameError> {
    let olds: HashSet<&str> = plan.iter().map(|(o, _)| o.as_str()).collect();
    let targets: HashSet<&str> = plan.iter().map(|(_, n)| n.as_str()).collect();

    // The preview checked the folder as the panel last read it. Something may have appeared
    // since; finding out now costs a stat per file and nothing has been touched yet.
    for (old, new) in plan {
        if !olds.contains(new.as_str()) && fs.exists(&dir.join(new)) {
            return Err(RenameError {
                at: (old.clone(), new.clone()),
                error: io::Error::new(io::ErrorKind::AlreadyExists, "a file of that name is there"),
                restored: true,
            });
        }
    }

    let mut done: Vec<(VPath, VPath)> = Vec::new();
    let undo = |done: &mut Vec<(VPath, VPath)>| -> bool {
        let mut whole = true;
        while let Some((from, to)) = done.pop() {
            if fs.rename(&to, &from).is_err() {
                whole = false;
            }
        }
        whole
    };
    let step = |from: VPath, to: VPath, done: &mut Vec<(VPath, VPath)>| -> io::Result<()> {
        if cancel.is_cancelled() {
            return Err(io::Error::new(io::ErrorKind::Interrupted, "cancelled"));
        }
        fs.rename(&from, &to)?;
        done.push((from, to));
        Ok(())
    };

    // First the files whose names another rename needs, out of the way.
    let detour = format!(".fcmd-rename-{}", std::process::id());
    let mut at: Vec<VPath> = Vec::with_capacity(plan.len());
    for (i, (old, new)) in plan.iter().enumerate() {
        let from = dir.join(old);
        if targets.contains(old.as_str()) {
            let parked = dir.join(format!("{detour}-{i}"));
            if let Err(error) = step(from, parked.clone(), &mut done) {
                return Err(RenameError {
                    at: (old.clone(), new.clone()),
                    error,
                    restored: undo(&mut done),
                });
            }
            at.push(parked);
        } else {
            at.push(from);
        }
    }
    // Then every file to its own name.
    for ((old, new), from) in plan.iter().zip(at) {
        if let Err(error) = step(from, dir.join(new), &mut done) {
            return Err(RenameError {
                at: (old.clone(), new.clone()),
                error,
                restored: undo(&mut done),
            });
        }
    }
    Ok(plan.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalFs;

    fn folder(names: &[(&str, &str)]) -> (tempfile::TempDir, VPath) {
        let dir = tempfile::tempdir().unwrap();
        for (name, body) in names {
            std::fs::write(dir.path().join(name), body).unwrap();
        }
        let path = VPath::local(dir.path());
        (dir, path)
    }

    fn read(dir: &tempfile::TempDir) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = std::fs::read_dir(dir.path())
            .unwrap()
            .map(|e| {
                let e = e.unwrap();
                (
                    e.file_name().to_string_lossy().into_owned(),
                    std::fs::read_to_string(e.path()).unwrap(),
                )
            })
            .collect();
        out.sort();
        out
    }

    fn pairs(p: &[(&str, &str)]) -> Vec<(String, String)> {
        p.iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect()
    }

    #[test]
    fn every_file_gets_its_new_name() {
        let (dir, path) = folder(&[("a.txt", "A"), ("b.txt", "B")]);
        let n = rename_all(
            &LocalFs,
            &path,
            &pairs(&[("a.txt", "one.txt"), ("b.txt", "two.txt")]),
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(n, 2);
        assert_eq!(read(&dir), pairs(&[("one.txt", "A"), ("two.txt", "B")]));
    }

    #[test]
    fn two_names_that_trade_places_trade_them() {
        let (dir, path) = folder(&[("1.txt", "first"), ("2.txt", "second"), ("3.txt", "third")]);
        // A cycle of three, which no order of plain renames can do.
        rename_all(
            &LocalFs,
            &path,
            &pairs(&[("1.txt", "2.txt"), ("2.txt", "3.txt"), ("3.txt", "1.txt")]),
            &CancelToken::new(),
        )
        .unwrap();
        assert_eq!(
            read(&dir),
            pairs(&[("1.txt", "third"), ("2.txt", "first"), ("3.txt", "second")])
        );
    }

    #[test]
    fn a_name_that_appeared_since_the_preview_stops_it_before_anything_moves() {
        let (dir, path) = folder(&[("a.txt", "A"), ("b.txt", "B"), ("taken.txt", "T")]);
        let e = rename_all(
            &LocalFs,
            &path,
            &pairs(&[("a.txt", "x.txt"), ("b.txt", "taken.txt")]),
            &CancelToken::new(),
        )
        .unwrap_err();
        assert_eq!(e.at.1, "taken.txt");
        assert!(e.restored);
        assert_eq!(
            read(&dir),
            pairs(&[("a.txt", "A"), ("b.txt", "B"), ("taken.txt", "T")])
        );
    }

    /// The second rename fails because its file is not there; the first is put back, and no
    /// file is left under a temporary name.
    #[test]
    fn a_failure_halfway_puts_everything_back() {
        let (dir, path) = folder(&[("1.txt", "first"), ("2.txt", "second")]);
        let e = rename_all(
            &LocalFs,
            &path,
            &pairs(&[
                ("1.txt", "2.txt"),
                ("2.txt", "1.txt"),
                ("gone.txt", "x.txt"),
            ]),
            &CancelToken::new(),
        )
        .unwrap_err();
        assert_eq!(e.at.0, "gone.txt");
        assert!(e.restored);
        assert_eq!(
            read(&dir),
            pairs(&[("1.txt", "first"), ("2.txt", "second")])
        );
    }

    #[test]
    fn a_cancelled_rename_leaves_the_folder_as_it_was() {
        let (dir, path) = folder(&[("a.txt", "A")]);
        let cancel = CancelToken::new();
        cancel.cancel();
        let e = rename_all(&LocalFs, &path, &pairs(&[("a.txt", "b.txt")]), &cancel).unwrap_err();
        assert_eq!(e.error.kind(), io::ErrorKind::Interrupted);
        assert_eq!(read(&dir), pairs(&[("a.txt", "A")]));
    }
}
