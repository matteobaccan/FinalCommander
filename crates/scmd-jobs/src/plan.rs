use std::path::PathBuf;

use scmd_core::{CancelToken, EntryKind, VPath};

use crate::fs::JobFs;
use crate::spec::{JobKind, JobSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnitKind {
    /// Create a directory at `dst` (and carry `src`'s metadata over at the end).
    Dir,
    File {
        size: u64,
    },
    Symlink {
        target: PathBuf,
    },
    RemoveFile,
    RemoveDir,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    pub src: Option<VPath>,
    pub dst: VPath,
    pub kind: UnitKind,
}

/// What a job is going to do, counted before it starts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    pub units: Vec<Unit>,
    /// `(source, destination)` for each thing the user selected — the Move fast path uses it.
    pub top_level: Vec<(VPath, VPath)>,
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
}

impl Plan {
    pub fn is_empty(&self) -> bool {
        self.units.is_empty()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PlanError {
    #[error("nothing selected")]
    NoSources,
    #[error("no destination given")]
    NoTarget,
    #[error("{dst}: the destination is inside {src}")]
    TargetInsideSource { src: VPath, dst: VPath },
    #[error("{path}: source and destination are the same file")]
    SameFile { path: VPath },
    #[error("{path}: {source}")]
    Io {
        path: VPath,
        #[source]
        source: std::io::Error,
    },
    #[error("cancelled")]
    Cancelled,
}

/// Expand a [`JobSpec`] into units, counting files and bytes, and refuse the impossible here
/// rather than halfway through (`docs/spec/03-file-operations.md` §1).
pub fn plan(fs: &dyn JobFs, spec: &JobSpec, cancel: &CancelToken) -> Result<Plan, PlanError> {
    if spec.kind == JobKind::Mkdir {
        let target = spec.target.clone().ok_or(PlanError::NoTarget)?;
        return Ok(Plan {
            units: vec![Unit {
                src: None,
                dst: target.clone(),
                kind: UnitKind::Dir,
            }],
            top_level: vec![(target.clone(), target)],
            files: 0,
            dirs: 1,
            bytes: 0,
        });
    }

    if spec.sources.is_empty() {
        return Err(PlanError::NoSources);
    }

    let mut plan = Plan::default();

    match spec.kind {
        JobKind::Delete => {
            for src in &spec.sources {
                check_cancel(cancel)?;
                walk_delete(fs, src, &mut plan, cancel)?;
            }
        }
        JobKind::Copy | JobKind::Move => {
            let target = spec.target.clone().ok_or(PlanError::NoTarget)?;
            let target_is_dir = fs
                .facts(&target)
                .map(|f| f.kind == EntryKind::Dir)
                .unwrap_or(false);
            let into_dir = target_is_dir || spec.sources.len() > 1;

            for src in &spec.sources {
                check_cancel(cancel)?;
                let dst = if into_dir {
                    match src.file_name() {
                        Some(name) => target.join(name),
                        None => target.clone(),
                    }
                } else {
                    target.clone()
                };

                if fs.same_file(src, &dst) {
                    return Err(PlanError::SameFile { path: dst });
                }
                if is_inside(src, &dst) {
                    return Err(PlanError::TargetInsideSource {
                        src: src.clone(),
                        dst,
                    });
                }
                plan.top_level.push((src.clone(), dst.clone()));
                walk_copy(fs, src, &dst, spec, &mut plan, cancel)?;
            }

            // A move deletes what it moved, children first, after everything is in place.
            // The removal units are appended but not counted: the counters describe the copy.
            if spec.kind == JobKind::Move {
                let mut removals = Plan::default();
                for src in &spec.sources {
                    walk_delete(fs, src, &mut removals, cancel)?;
                }
                plan.units.extend(removals.units);
            }
        }
        JobKind::Mkdir => unreachable!("handled above"),
    }

    Ok(plan)
}

fn walk_copy(
    fs: &dyn JobFs,
    src: &VPath,
    dst: &VPath,
    spec: &JobSpec,
    plan: &mut Plan,
    cancel: &CancelToken,
) -> Result<(), PlanError> {
    check_cancel(cancel)?;
    let facts = if spec.options.follow_symlinks {
        fs.facts_following(src)
    } else {
        fs.facts(src)
    }
    .map_err(|source| PlanError::Io {
        path: src.clone(),
        source,
    })?;

    match facts.kind {
        EntryKind::Dir => {
            plan.units.push(Unit {
                src: Some(src.clone()),
                dst: dst.clone(),
                kind: UnitKind::Dir,
            });
            plan.dirs += 1;
            let children = fs.read_dir(src).map_err(|source| PlanError::Io {
                path: src.clone(),
                source,
            })?;
            for (name, _) in children {
                walk_copy(fs, &src.join(&name), &dst.join(&name), spec, plan, cancel)?;
            }
        }
        EntryKind::Symlink => {
            let target = fs.read_link(src).map_err(|source| PlanError::Io {
                path: src.clone(),
                source,
            })?;
            plan.units.push(Unit {
                src: Some(src.clone()),
                dst: dst.clone(),
                kind: UnitKind::Symlink { target },
            });
            plan.files += 1;
        }
        _ => {
            plan.units.push(Unit {
                src: Some(src.clone()),
                dst: dst.clone(),
                kind: UnitKind::File { size: facts.size },
            });
            plan.files += 1;
            plan.bytes += facts.size;
        }
    }
    Ok(())
}

fn walk_delete(
    fs: &dyn JobFs,
    src: &VPath,
    plan: &mut Plan,
    cancel: &CancelToken,
) -> Result<(), PlanError> {
    check_cancel(cancel)?;
    let facts = fs.facts(src).map_err(|source| PlanError::Io {
        path: src.clone(),
        source,
    })?;

    if facts.kind == EntryKind::Dir {
        let children = fs.read_dir(src).map_err(|source| PlanError::Io {
            path: src.clone(),
            source,
        })?;
        for (name, _) in children {
            walk_delete(fs, &src.join(&name), plan, cancel)?;
        }
        // Post-order: a directory goes after its contents.
        plan.units.push(Unit {
            src: None,
            dst: src.clone(),
            kind: UnitKind::RemoveDir,
        });
        plan.dirs += 1;
    } else {
        plan.units.push(Unit {
            src: None,
            dst: src.clone(),
            kind: UnitKind::RemoveFile,
        });
        plan.files += 1;
        plan.bytes += facts.size;
    }
    Ok(())
}

/// Is `inner` inside `outer`? Lexical, which is what a copy guard needs.
pub fn is_inside(outer: &VPath, inner: &VPath) -> bool {
    let (a, b) = (outer.normalised(), inner.normalised());
    if a == b {
        return false;
    }
    b.as_path().starts_with(a.as_path())
}

fn check_cancel(cancel: &CancelToken) -> Result<(), PlanError> {
    if cancel.is_cancelled() {
        Err(PlanError::Cancelled)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;
    use crate::spec::{JobOptions, JobSpec};
    use std::fs as stdfs;

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir_all(dir.path().join("src/sub")).unwrap();
        stdfs::write(dir.path().join("src/a.txt"), b"aaaa").unwrap();
        stdfs::write(dir.path().join("src/sub/b.txt"), b"bb").unwrap();
        stdfs::create_dir(dir.path().join("dst")).unwrap();
        dir
    }

    #[test]
    fn copying_a_tree_plans_directories_before_their_contents() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        assert_eq!((p.files, p.dirs, p.bytes), (2, 2, 6));
        assert!(matches!(p.units[0].kind, UnitKind::Dir));
        assert_eq!(
            p.units[0].dst,
            VPath::local(dir.path().join("dst/src")),
            "a directory target means `into`, not `rename to`"
        );
        // every file's destination lives under dst/src
        for u in &p.units {
            assert!(u.dst.to_string().contains("dst/src"), "{:?}", u.dst);
        }
    }

    #[test]
    fn a_single_source_and_a_nonexistent_target_is_a_rename() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst/renamed.txt"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        assert_eq!(p.units.len(), 1);
        assert_eq!(
            p.units[0].dst,
            VPath::local(dir.path().join("dst/renamed.txt"))
        );
    }

    #[test]
    fn two_sources_always_go_into_the_target() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![
                VPath::local(dir.path().join("src/a.txt")),
                VPath::local(dir.path().join("src/sub/b.txt")),
            ],
            Some(VPath::local(dir.path().join("newdir"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        assert_eq!(p.files, 2);
        assert!(p
            .units
            .iter()
            .all(|u| u.dst.to_string().contains("newdir/")));
    }

    #[test]
    fn deleting_plans_children_before_their_directory() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Delete,
            vec![VPath::local(dir.path().join("src"))],
            None,
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        let last = p.units.last().unwrap();
        assert_eq!(last.kind, UnitKind::RemoveDir);
        assert_eq!(last.dst, VPath::local(dir.path().join("src")));
        let first_dir = p
            .units
            .iter()
            .position(|u| u.kind == UnitKind::RemoveDir)
            .unwrap();
        let file_positions: Vec<usize> = p
            .units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.kind == UnitKind::RemoveFile)
            .map(|(i, _)| i)
            .collect();
        assert!(
            file_positions.iter().any(|i| *i < first_dir),
            "files must be removed before their directory"
        );
    }

    #[test]
    fn moving_plans_the_copy_then_the_removal() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        let first_removal = p
            .units
            .iter()
            .position(|u| matches!(u.kind, UnitKind::RemoveFile | UnitKind::RemoveDir))
            .unwrap();
        assert!(p.units[..first_removal]
            .iter()
            .all(|u| !matches!(u.kind, UnitKind::RemoveFile | UnitKind::RemoveDir)));
        assert_eq!(p.top_level.len(), 1);
        // The counts describe the copy, not the cleanup.
        assert_eq!((p.files, p.bytes), (2, 6));
    }

    #[test]
    fn copying_a_directory_into_itself_is_refused_before_anything_happens() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("src/sub"))),
        );
        let err = plan(&LocalFs, &spec, &CancelToken::new()).unwrap_err();
        assert!(matches!(err, PlanError::TargetInsideSource { .. }), "{err}");
    }

    #[test]
    fn copying_a_file_onto_itself_is_refused() {
        let dir = tree();
        let file = VPath::local(dir.path().join("src/a.txt"));
        let spec = JobSpec::new(JobKind::Copy, vec![file.clone()], Some(file));
        let err = plan(&LocalFs, &spec, &CancelToken::new()).unwrap_err();
        assert!(matches!(err, PlanError::SameFile { .. }), "{err}");
    }

    #[test]
    fn symlinks_are_planned_as_links_unless_followed() {
        let dir = tree();
        let link = VPath::local(dir.path().join("src/link"));
        LocalFs
            .symlink(std::path::Path::new("a.txt"), &link)
            .unwrap();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![link.clone()],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        assert!(matches!(p.units[0].kind, UnitKind::Symlink { .. }));

        let followed = spec.clone().with_options(JobOptions {
            follow_symlinks: true,
            ..JobOptions::default()
        });
        let p = plan(&LocalFs, &followed, &CancelToken::new()).unwrap();
        assert!(matches!(p.units[0].kind, UnitKind::File { size: 4 }));
    }

    #[test]
    fn missing_sources_and_targets_are_reported_not_guessed() {
        let dir = tree();
        assert!(matches!(
            plan(
                &LocalFs,
                &JobSpec::new(JobKind::Copy, vec![], Some(VPath::local(dir.path()))),
                &CancelToken::new()
            ),
            Err(PlanError::NoSources)
        ));
        assert!(matches!(
            plan(
                &LocalFs,
                &JobSpec::new(
                    JobKind::Copy,
                    vec![VPath::local(dir.path().join("src"))],
                    None
                ),
                &CancelToken::new()
            ),
            Err(PlanError::NoTarget)
        ));
        let err = plan(
            &LocalFs,
            &JobSpec::new(
                JobKind::Delete,
                vec![VPath::local(dir.path().join("ghost"))],
                None,
            ),
            &CancelToken::new(),
        )
        .unwrap_err();
        assert!(matches!(err, PlanError::Io { .. }), "{err}");
    }

    #[test]
    fn planning_is_cancellable() {
        let dir = tree();
        let token = CancelToken::new();
        token.cancel();
        let spec = JobSpec::new(
            JobKind::Delete,
            vec![VPath::local(dir.path().join("src"))],
            None,
        );
        assert!(matches!(
            plan(&LocalFs, &spec, &token),
            Err(PlanError::Cancelled)
        ));
    }

    #[test]
    fn mkdir_needs_no_sources() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Mkdir,
            vec![],
            Some(VPath::local(dir.path().join("a/b/c"))),
        );
        let p = plan(&LocalFs, &spec, &CancelToken::new()).unwrap();
        assert_eq!(p.units.len(), 1);
        assert_eq!(p.dirs, 1);
    }

    #[test]
    fn inside_check_is_lexical_and_excludes_equality() {
        assert!(is_inside(&VPath::local("/a"), &VPath::local("/a/b")));
        assert!(!is_inside(&VPath::local("/a"), &VPath::local("/a")));
        assert!(!is_inside(&VPath::local("/a/b"), &VPath::local("/a")));
        assert!(!is_inside(&VPath::local("/ab"), &VPath::local("/a")));
    }
}
