//! Undoing what a file operation did (SPEC 03 §14, 0.39.0).
//!
//! The engine copies, moves and deletes; it does not say afterwards which names it made. So
//! what an operation will do is worked out when it is queued, from the same rule the planner
//! follows — a target that is a folder, or several sources, means *into* it; one source and a
//! target that is not a folder means *as* it — and kept only if the job ends with nothing
//! cancelled and no error. Undoing is then the opposite operation, sent through the same
//! queue as any other, so it asks the same questions on a conflict and can be watched and
//! stopped:
//!
//! * a **copy** is undone by sending what it made to the wastebasket — never by deleting it,
//!   because a copy that wrote over an existing file cannot give that file back, and those
//!   destinations are left out of the undo rather than thrown away;
//! * a **move** or a rename by moving each name back where it was;
//! * a **delete to the wastebasket** by putting the names back from it, where the system lets
//!   a program do that;
//! * a **new folder** by removing it, if it is still empty.
//!
//! Local files only: a server, a bucket or an archive has no wastebasket to fall back on, and
//! an undo that might lose data there is worse than none.

use std::path::{Path, PathBuf};

use fcmd_core::VPath;
use fcmd_jobs::{JobKind, JobSpec};

/// One thing that can be undone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Undo {
    /// The files and folders a copy made, to be sent to the wastebasket.
    Copy { made: Vec<PathBuf> },
    /// `(where it was, where it is now)`, for each name a move took.
    Move { pairs: Vec<(PathBuf, PathBuf)> },
    /// What was sent to the wastebasket, by the path it had.
    Trash { paths: Vec<PathBuf> },
    /// A folder `F7` made.
    Mkdir { dir: PathBuf },
    /// Pictures whose metadata was taken out: `(the picture, the copy of it as it was)`.
    Restore { pairs: Vec<(PathBuf, PathBuf)> },
}

/// How many operations are remembered. An undo is for what one just did, not for last week.
pub const KEEP: usize = 20;

impl Undo {
    /// What an operation will do, worked out before it runs — `None` for one that cannot be
    /// undone from here (another kind, anything not on this disk, an extraction).
    pub fn plan(spec: &JobSpec) -> Option<Undo> {
        let local = |p: &VPath| p.is_local() && !p.in_container();
        // `F7` names the folder it makes as the target and has no sources.
        if spec.kind == JobKind::Mkdir {
            let dir = spec
                .target
                .as_ref()
                .filter(|t| local(t))?
                .as_path()
                .to_path_buf();
            return (!dir.exists()).then_some(Undo::Mkdir { dir });
        }
        if spec.sources.is_empty() || !spec.sources.iter().all(local) {
            return None;
        }
        match spec.kind {
            JobKind::Copy | JobKind::Move => {
                let target = spec.target.as_ref().filter(|t| local(t))?;
                if spec.options.contents || spec.options.relative_to.is_some() {
                    return None;
                }
                let target = target.as_path().to_path_buf();
                let into = target.is_dir() || spec.sources.len() > 1;
                let pairs: Vec<(PathBuf, PathBuf)> = spec
                    .sources
                    .iter()
                    .filter_map(|s| {
                        let src = s.as_path().to_path_buf();
                        let dst = if into {
                            target.join(src.file_name()?)
                        } else {
                            target.clone()
                        };
                        Some((src, dst))
                    })
                    .collect();
                if spec.kind == JobKind::Copy {
                    // A destination that is there already is one the copy may write over;
                    // sending it to the wastebasket would throw away what was there before.
                    let made: Vec<PathBuf> = pairs
                        .into_iter()
                        .filter(|(_, dst)| !dst.exists())
                        .map(|(_, dst)| dst)
                        .collect();
                    (!made.is_empty()).then_some(Undo::Copy { made })
                } else {
                    let pairs: Vec<_> = pairs.into_iter().filter(|(_, d)| !d.exists()).collect();
                    (!pairs.is_empty()).then_some(Undo::Move { pairs })
                }
            }
            JobKind::Trash => Some(Undo::Trash {
                paths: spec
                    .sources
                    .iter()
                    .map(|s| s.as_path().to_path_buf())
                    .collect(),
            }),
            _ => None,
        }
    }

    /// How many names it touches, for the dialog that asks first.
    pub fn count(&self) -> usize {
        match self {
            Undo::Copy { made } => made.len(),
            Undo::Move { pairs } => pairs.len(),
            Undo::Trash { paths } => paths.len(),
            Undo::Mkdir { .. } => 1,
            Undo::Restore { pairs } => pairs.len(),
        }
    }

    /// The names, for the dialog: what will move, or be thrown away, or come back.
    pub fn names(&self) -> Vec<String> {
        match self {
            Undo::Copy { made } => made.iter().map(|p| p.display().to_string()).collect(),
            Undo::Move { pairs } => pairs
                .iter()
                .map(|(from, to)| format!("{} → {}", to.display(), from.display()))
                .collect(),
            Undo::Trash { paths } => paths.iter().map(|p| p.display().to_string()).collect(),
            Undo::Mkdir { dir } => vec![dir.display().to_string()],
            Undo::Restore { pairs } => pairs.iter().map(|(p, _)| p.display().to_string()).collect(),
        }
    }

    /// The jobs that undo a copy or a move, for the queue. A move back whose names did not
    /// change goes as one job into the folder they came from; one that renamed goes as a job
    /// per name. Empty for the two kinds that are not jobs.
    pub fn jobs(&self) -> Vec<JobSpec> {
        match self {
            Undo::Copy { made } => vec![JobSpec::new(
                JobKind::Trash,
                made.iter().map(VPath::local).collect(),
                None,
            )],
            Undo::Move { pairs } => {
                let mut out: Vec<JobSpec> = Vec::new();
                let mut by_dir: Vec<(PathBuf, Vec<VPath>)> = Vec::new();
                for (from, to) in pairs {
                    let same_name = from.file_name() == to.file_name();
                    match (same_name, from.parent()) {
                        (true, Some(dir)) => match by_dir.iter_mut().find(|(d, _)| d == dir) {
                            Some((_, list)) => list.push(VPath::local(to)),
                            None => by_dir.push((dir.to_path_buf(), vec![VPath::local(to)])),
                        },
                        _ => out.push(JobSpec::new(
                            JobKind::Move,
                            vec![VPath::local(to)],
                            Some(VPath::local(from)),
                        )),
                    }
                }
                for (dir, sources) in by_dir {
                    out.push(JobSpec::new(
                        JobKind::Move,
                        sources,
                        Some(VPath::local(&dir)),
                    ));
                }
                out
            }
            Undo::Trash { .. } | Undo::Mkdir { .. } | Undo::Restore { .. } => Vec::new(),
        }
    }
}

/// Undo the two kinds that are not jobs, on a worker: the wastebasket gives its names back,
/// and an empty folder goes. Returns how many came back or went.
pub fn run(undo: &Undo) -> Result<usize, String> {
    match undo {
        Undo::Trash { paths } => fcmd_jobs::fs::restore_from_trash(paths),
        Undo::Mkdir { dir } => remove_if_empty(dir).map(|()| 1),
        Undo::Restore { pairs } => restore(pairs),
        Undo::Copy { .. } | Undo::Move { .. } => Ok(0),
    }
}

/// Each picture put back as it was from the copy made before its metadata went, and the copy
/// thrown away once it is back.
fn restore(pairs: &[(PathBuf, PathBuf)]) -> Result<usize, String> {
    let mut done = 0;
    for (file, copy) in pairs {
        std::fs::copy(copy, file).map_err(|e| format!("{}: {e}", file.display()))?;
        let _ = std::fs::remove_file(copy);
        if let Some(dir) = copy.parent() {
            let _ = std::fs::remove_dir(dir);
        }
        done += 1;
    }
    Ok(done)
}

/// Take the metadata out of each picture, copying it aside first. A picture with nothing to
/// take out is left alone, its file not even rewritten.
pub fn strip_files(
    files: &[PathBuf],
    how: fcmd_view::strip::Strip,
    backup: &Path,
) -> (Vec<(PathBuf, PathBuf)>, usize, usize, Vec<String>) {
    let mut done = Vec::new();
    let (mut removed, mut untouched) = (0, 0);
    let mut errors = Vec::new();
    for (n, file) in files.iter().enumerate() {
        let one = || -> Result<Option<(PathBuf, usize)>, String> {
            let say = |e: std::io::Error| format!("{}: {e}", file.display());
            let bytes = std::fs::read(file).map_err(say)?;
            let out = fcmd_view::strip::strip(&bytes, how)
                .map_err(|e| format!("{}: {e}", file.display()))?;
            if out.removed.is_empty() {
                return Ok(None);
            }
            std::fs::create_dir_all(backup).map_err(say)?;
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let copy = backup.join(format!("{n}-{name}"));
            std::fs::copy(file, &copy).map_err(say)?;
            // Written beside it and renamed over it, so a failure halfway leaves the picture
            // as it was rather than half of it.
            let tmp = file.with_extension("fcmd-strip");
            std::fs::write(&tmp, &out.bytes).map_err(say)?;
            if let Ok(meta) = std::fs::metadata(file) {
                let _ = std::fs::set_permissions(&tmp, meta.permissions());
            }
            std::fs::rename(&tmp, file).map_err(|e| {
                let _ = std::fs::remove_file(&tmp);
                say(e)
            })?;
            Ok(Some((copy, out.removed.len())))
        };
        match one() {
            Ok(Some((copy, gone))) => {
                done.push((file.clone(), copy));
                removed += gone;
            }
            Ok(None) => untouched += 1,
            Err(e) => errors.push(e),
        }
    }
    (done, removed, untouched, errors)
}

fn remove_if_empty(dir: &Path) -> Result<(), String> {
    let mut entries = std::fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    if entries.next().is_some() {
        return Err(format!(
            "{} is not empty any more, so it stays",
            dir.display()
        ));
    }
    std::fs::remove_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_copy_into_a_folder_is_undone_by_throwing_away_only_what_it_made() {
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("src"), dir.path().join("dst"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        std::fs::write(src.join("a.txt"), "a").unwrap();
        std::fs::write(src.join("b.txt"), "b").unwrap();
        // `b.txt` is already there: the copy may write over it, and the undo leaves it be.
        std::fs::write(dst.join("b.txt"), "old").unwrap();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![
                VPath::local(src.join("a.txt")),
                VPath::local(src.join("b.txt")),
            ],
            Some(VPath::local(&dst)),
        );
        let undo = Undo::plan(&spec).expect("a copy can be undone");
        assert_eq!(
            undo,
            Undo::Copy {
                made: vec![dst.join("a.txt")]
            }
        );
        let jobs = undo.jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].kind, JobKind::Trash);
    }

    #[test]
    fn a_move_goes_back_in_one_job_and_a_rename_in_one_of_its_own() {
        let dir = tempfile::tempdir().unwrap();
        let (src, dst) = (dir.path().join("src"), dir.path().join("dst"));
        std::fs::create_dir_all(&src).unwrap();
        std::fs::create_dir_all(&dst).unwrap();
        for n in ["a", "b"] {
            std::fs::write(src.join(n), n).unwrap();
        }
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(src.join("a")), VPath::local(src.join("b"))],
            Some(VPath::local(&dst)),
        );
        let undo = Undo::plan(&spec).unwrap();
        let jobs = undo.jobs();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].sources.len(), 2);
        assert_eq!(jobs[0].target, Some(VPath::local(&src)));

        // One file moved under another name is put back under its own.
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(src.join("a"))],
            Some(VPath::local(dir.path().join("renamed"))),
        );
        let undo = Undo::plan(&spec).unwrap();
        assert_eq!(undo.count(), 1);
        let jobs = undo.jobs();
        assert_eq!(jobs[0].target, Some(VPath::local(src.join("a"))));
    }

    /// A picture's metadata out, its copy aside, and the undo putting it back byte for byte;
    /// a picture with nothing to take out is not even rewritten.
    #[test]
    fn stripping_a_picture_can_be_undone() {
        fn chunk(kind: &[u8], body: &[u8]) -> Vec<u8> {
            let mut c = (body.len() as u32).to_be_bytes().to_vec();
            c.extend(kind);
            c.extend(body);
            c.extend([0, 0, 0, 0]);
            c
        }
        let dir = tempfile::tempdir().unwrap();
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        png.extend(chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]));
        png.extend(chunk(b"tEXt", b"parameters\0a cat\nSteps: 20, Seed: 1"));
        png.extend(chunk(b"IDAT", b"pixels"));
        png.extend(chunk(b"IEND", b""));
        let made = dir.path().join("made.png");
        std::fs::write(&made, &png).unwrap();
        let mut clean = b"\x89PNG\r\n\x1a\n".to_vec();
        clean.extend(chunk(b"IEND", b""));
        let plain = dir.path().join("plain.png");
        std::fs::write(&plain, &clean).unwrap();
        let backup = dir.path().join("backup");
        let (done, removed, untouched, errors) = strip_files(
            &[made.clone(), plain.clone()],
            fcmd_view::strip::Strip::Ai,
            &backup,
        );
        assert!(errors.is_empty(), "{errors:?}");
        assert_eq!((done.len(), removed, untouched), (1, 1, 1));
        let after = std::fs::read(&made).unwrap();
        assert!(after.len() < png.len());
        assert!(!after.windows(6).any(|w| w == b"Steps:"));
        let undo = Undo::Restore { pairs: done };
        assert_eq!(run(&undo), Ok(1));
        assert_eq!(std::fs::read(&made).unwrap(), png);
        assert!(!backup.exists(), "the copies go once they are back");
    }

    #[test]
    fn a_new_folder_goes_only_while_it_is_empty() {
        let dir = tempfile::tempdir().unwrap();
        let made = dir.path().join("new");
        let spec = JobSpec::new(JobKind::Mkdir, vec![], Some(VPath::local(&made)));
        let undo = Undo::plan(&spec).unwrap();
        std::fs::create_dir(&made).unwrap();
        std::fs::write(made.join("x"), "x").unwrap();
        assert!(run(&undo).is_err());
        assert!(made.exists());
        std::fs::remove_file(made.join("x")).unwrap();
        assert_eq!(run(&undo), Ok(1));
        assert!(!made.exists());
    }

    #[test]
    fn what_is_not_on_this_disk_is_not_undone() {
        let spec = JobSpec::new(
            JobKind::Delete,
            vec![VPath::local(std::env::temp_dir().join("x"))],
            None,
        );
        assert_eq!(
            Undo::plan(&spec),
            None,
            "a real delete has nothing to come back from"
        );
    }
}
