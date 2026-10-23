use scmd_core::{CancelToken, VPath};

use crate::fs::{free_name, JobFs};
use crate::plan::{Plan, Unit, UnitKind};
use crate::spec::{ConflictPolicy, Decision, JobKind, JobSpec, Phase, Question};

/// What the UI is told while a job runs.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Progress {
    pub phase: Phase,
    pub files_done: u64,
    pub files_total: u64,
    pub bytes_done: u64,
    pub bytes_total: u64,
    /// Bytes of the file being copied right now, for the per-file bar.
    pub file_bytes_done: u64,
    pub file_bytes_total: u64,
    pub current: Option<VPath>,
    pub skipped: u64,
    pub errors: u64,
}

impl Progress {
    /// Percentage of the whole job: bytes when we know them, files otherwise.
    pub fn percent(&self) -> u16 {
        percent_of(self.bytes_done, self.bytes_total)
            .or_else(|| percent_of(self.files_done, self.files_total))
            .unwrap_or(0)
    }

    /// Percentage of the file being copied right now.
    pub fn file_percent(&self) -> u16 {
        percent_of(self.file_bytes_done, self.file_bytes_total).unwrap_or(0)
    }
}

/// `None` when there is no total to measure against.
fn percent_of(done: u64, total: u64) -> Option<u16> {
    let total = (total > 0).then_some(total)?;
    Some(((done.min(total) * 100) / total) as u16)
}

/// How the engine talks to whoever started the job.
///
/// `ask` blocks: in the application it posts a dialog and waits for the answer; in tests it is
/// a scripted closure. That is what keeps the engine itself free of UI concepts.
pub trait Reporter {
    fn progress(&mut self, progress: &Progress);
    fn ask(&mut self, question: Question) -> Decision;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Outcome {
    pub files: u64,
    pub bytes: u64,
    pub dirs: u64,
    pub skipped: u64,
    pub errors: Vec<(VPath, String)>,
    pub cancelled: bool,
    /// Sources a Move finished with a plain rename, so no byte was copied.
    pub renamed: u64,
}

impl Outcome {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && !self.cancelled
    }
}

/// Run a plan. Never panics: every failure becomes a decision point or an entry in
/// [`Outcome::errors`].
pub fn execute(
    fs: &dyn JobFs,
    spec: &JobSpec,
    plan: Plan,
    reporter: &mut dyn Reporter,
    cancel: &CancelToken,
) -> Outcome {
    let mut state = State {
        progress: Progress {
            phase: Phase::Running,
            files_total: plan.files,
            bytes_total: plan.bytes,
            ..Progress::default()
        },
        overwrite_all: spec.options.on_conflict == ConflictPolicy::Overwrite,
        skip_all: spec.options.on_conflict == ConflictPolicy::Skip,
        skip_errors: false,
        outcome: Outcome::default(),
        dir_meta: Vec::new(),
        bytes_since_report: 0,
    };

    // A move on the same filesystem is a rename: no bytes, no risk of a half-copied tree.
    let mut renamed_sources: Vec<VPath> = Vec::new();
    if spec.kind == JobKind::Move && !spec.options.dry_run {
        for (src, dst) in &plan.top_level {
            if cancel.is_cancelled() {
                break;
            }
            if fs.exists(dst) {
                continue; // a conflict: let the unit path deal with it
            }
            if let Some(parent) = dst.parent() {
                let _ = fs.create_dir_all(&parent);
            }
            if fs.rename(src, dst).is_ok() {
                renamed_sources.push(src.clone());
                state.outcome.renamed += 1;
            }
        }
    }

    for unit in &plan.units {
        if cancel.is_cancelled() {
            state.outcome.cancelled = true;
            break;
        }
        // Anything under a source we already renamed is done.
        if renamed_sources.iter().any(|src| unit_belongs_to(unit, src)) {
            if let UnitKind::File { size } = unit.kind {
                state.outcome.files += 1;
                state.outcome.bytes += size;
                state.progress.files_done += 1;
                state.progress.bytes_done += size;
            }
            continue;
        }

        state.progress.current = Some(unit.dst.clone());
        run_unit(fs, spec, unit, &mut state, reporter, cancel);
        reporter.progress(&state.progress);
    }

    // Settle: directory metadata last, deepest first, so creating children cannot touch it.
    state.progress.phase = Phase::Settling;
    if !spec.options.dry_run {
        for (dst, src) in state.dir_meta.iter().rev() {
            if let Ok(facts) = fs.facts(src) {
                let _ = fs.apply_meta(dst, &facts, spec.options.preserve);
            }
        }
    }

    state.progress.phase = Phase::Done;
    state.progress.current = None;
    reporter.progress(&state.progress);
    state.outcome.skipped = state.progress.skipped;
    state.outcome
}

struct State {
    progress: Progress,
    overwrite_all: bool,
    skip_all: bool,
    skip_errors: bool,
    outcome: Outcome,
    dir_meta: Vec<(VPath, VPath)>,
    bytes_since_report: u64,
}

fn unit_belongs_to(unit: &Unit, src: &VPath) -> bool {
    match &unit.src {
        Some(unit_src) => unit_src == src || unit_src.starts_with(src),
        None => unit.dst == *src || unit.dst.starts_with(src),
    }
}

fn run_unit(
    fs: &dyn JobFs,
    spec: &JobSpec,
    unit: &Unit,
    state: &mut State,
    reporter: &mut dyn Reporter,
    cancel: &CancelToken,
) {
    loop {
        let result = attempt(fs, spec, unit, state, reporter, cancel);
        match result {
            Attempt::Done => return,
            Attempt::Skipped => {
                state.progress.skipped += 1;
                return;
            }
            Attempt::Cancelled => {
                state.outcome.cancelled = true;
                return;
            }
            Attempt::Failed(error) => {
                if state.skip_errors {
                    state.progress.errors += 1;
                    state.outcome.errors.push((unit.dst.clone(), error));
                    return;
                }
                match reporter.ask(Question::Error {
                    path: unit.dst.clone(),
                    error: error.clone(),
                }) {
                    Decision::Retry => continue,
                    Decision::SkipAll => {
                        state.skip_errors = true;
                        state.progress.errors += 1;
                        state.outcome.errors.push((unit.dst.clone(), error));
                        return;
                    }
                    Decision::Cancel => {
                        state.outcome.cancelled = true;
                        state.outcome.errors.push((unit.dst.clone(), error));
                        return;
                    }
                    _ => {
                        state.progress.errors += 1;
                        state.outcome.errors.push((unit.dst.clone(), error));
                        return;
                    }
                }
            }
        }
    }
}

enum Attempt {
    Done,
    Skipped,
    Cancelled,
    Failed(String),
}

fn attempt(
    fs: &dyn JobFs,
    spec: &JobSpec,
    unit: &Unit,
    state: &mut State,
    reporter: &mut dyn Reporter,
    cancel: &CancelToken,
) -> Attempt {
    let dry = spec.options.dry_run;

    match &unit.kind {
        UnitKind::Dir => {
            if !dry {
                if let Err(e) = fs.create_dir_all(&unit.dst) {
                    return Attempt::Failed(e.to_string());
                }
            }
            if let Some(src) = &unit.src {
                state.dir_meta.push((unit.dst.clone(), src.clone()));
            }
            state.outcome.dirs += 1;
            Attempt::Done
        }

        UnitKind::File { size } => {
            let Some(src) = unit.src.clone() else {
                return Attempt::Failed("no source".into());
            };
            let dst = match resolve_conflict(fs, spec, unit, state, reporter) {
                Conflict::Proceed(dst) => dst,
                Conflict::Skip => return Attempt::Skipped,
                Conflict::Cancel => return Attempt::Cancelled,
            };

            if dry {
                state.outcome.files += 1;
                state.outcome.bytes += size;
                state.progress.files_done += 1;
                state.progress.bytes_done += size;
                return Attempt::Done;
            }

            state.progress.file_bytes_total = *size;
            state.progress.file_bytes_done = 0;
            let mut tick = |n: u64| {
                state.progress.bytes_done += n;
                state.progress.file_bytes_done += n;
                state.bytes_since_report += n;
                !cancel.is_cancelled()
            };
            let copied = fs.copy_file(&src, &dst, spec.options.buffer, &mut tick);
            match copied {
                Ok(bytes) => {
                    if let Ok(facts) = fs.facts(&src) {
                        let _ = fs.apply_meta(&dst, &facts, spec.options.preserve);
                    }
                    if spec.kind == JobKind::Move {
                        // The source goes only after the destination is complete.
                        if let Err(e) = fs.remove_file(&src) {
                            return Attempt::Failed(e.to_string());
                        }
                    }
                    state.outcome.files += 1;
                    state.outcome.bytes += bytes;
                    state.progress.files_done += 1;
                    if state.bytes_since_report > 1024 * 1024 {
                        state.bytes_since_report = 0;
                        reporter.progress(&state.progress);
                    }
                    Attempt::Done
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Attempt::Cancelled,
                Err(e) => Attempt::Failed(e.to_string()),
            }
        }

        UnitKind::Symlink { target } => {
            let dst = match resolve_conflict(fs, spec, unit, state, reporter) {
                Conflict::Proceed(dst) => dst,
                Conflict::Skip => return Attempt::Skipped,
                Conflict::Cancel => return Attempt::Cancelled,
            };
            if !dry {
                if fs.exists(&dst) {
                    if let Err(e) = fs.remove_file(&dst) {
                        return Attempt::Failed(e.to_string());
                    }
                }
                if let Err(e) = fs.symlink(target, &dst) {
                    return Attempt::Failed(e.to_string());
                }
                if spec.kind == JobKind::Move {
                    if let Some(src) = &unit.src {
                        if let Err(e) = fs.remove_file(src) {
                            return Attempt::Failed(e.to_string());
                        }
                    }
                }
            }
            state.outcome.files += 1;
            state.progress.files_done += 1;
            Attempt::Done
        }

        UnitKind::RemoveFile => {
            if !dry {
                match fs.remove_file(&unit.dst) {
                    Ok(()) => {}
                    // A move already took it away: not an error.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Attempt::Failed(e.to_string()),
                }
            }
            if spec.kind == JobKind::Delete {
                state.outcome.files += 1;
                state.progress.files_done += 1;
            }
            Attempt::Done
        }

        UnitKind::RemoveDir => {
            if !dry {
                match fs.remove_dir(&unit.dst) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Attempt::Failed(e.to_string()),
                }
            }
            if spec.kind == JobKind::Delete {
                state.outcome.dirs += 1;
            }
            Attempt::Done
        }
    }
}

enum Conflict {
    Proceed(VPath),
    Skip,
    Cancel,
}

fn resolve_conflict(
    fs: &dyn JobFs,
    spec: &JobSpec,
    unit: &Unit,
    state: &mut State,
    reporter: &mut dyn Reporter,
) -> Conflict {
    if !fs.exists(&unit.dst) {
        return Conflict::Proceed(unit.dst.clone());
    }
    if state.skip_all {
        return Conflict::Skip;
    }
    if state.overwrite_all {
        return Conflict::Proceed(unit.dst.clone());
    }

    let src = unit.src.clone().unwrap_or_else(|| unit.dst.clone());
    let src_facts = fs.facts(&src).ok();
    let dst_facts = fs.facts(&unit.dst).ok();
    let src_newer = match (
        src_facts.as_ref().and_then(|f| f.mtime),
        dst_facts.as_ref().and_then(|f| f.mtime),
    ) {
        (Some(a), Some(b)) => a > b,
        _ => true,
    };

    match spec.options.on_conflict {
        ConflictPolicy::Overwrite => return Conflict::Proceed(unit.dst.clone()),
        ConflictPolicy::Skip => return Conflict::Skip,
        ConflictPolicy::Rename => return Conflict::Proceed(free_name(fs, &unit.dst)),
        ConflictPolicy::OverwriteIfNewer => {
            return if src_newer {
                Conflict::Proceed(unit.dst.clone())
            } else {
                Conflict::Skip
            }
        }
        ConflictPolicy::Ask => {}
    }

    let question = Question::Conflict {
        src: src.clone(),
        dst: unit.dst.clone(),
        src_size: src_facts.map(|f| f.size).unwrap_or(0),
        dst_size: dst_facts.map(|f| f.size).unwrap_or(0),
        src_newer,
    };
    match reporter.ask(question) {
        Decision::Overwrite => Conflict::Proceed(unit.dst.clone()),
        Decision::OverwriteAll => {
            state.overwrite_all = true;
            Conflict::Proceed(unit.dst.clone())
        }
        Decision::Skip => Conflict::Skip,
        Decision::SkipAll => {
            state.skip_all = true;
            Conflict::Skip
        }
        Decision::Rename(name) => {
            let dst = match unit.dst.parent() {
                Some(parent) => parent.join(name),
                None => unit.dst.clone(),
            };
            Conflict::Proceed(dst)
        }
        Decision::Retry => Conflict::Proceed(unit.dst.clone()),
        Decision::Cancel => Conflict::Cancel,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;
    use crate::plan::plan;
    use crate::spec::{JobOptions, Preserve};
    use std::fs as stdfs;

    /// A reporter that answers from a script and records what it was asked.
    struct Scripted {
        answers: Vec<Decision>,
        asked: Vec<Question>,
        frames: usize,
    }

    impl Scripted {
        fn new(answers: Vec<Decision>) -> Self {
            Scripted {
                answers,
                asked: Vec::new(),
                frames: 0,
            }
        }
    }

    impl Reporter for Scripted {
        fn progress(&mut self, _p: &Progress) {
            self.frames += 1;
        }

        fn ask(&mut self, question: Question) -> Decision {
            self.asked.push(question);
            if self.answers.is_empty() {
                Decision::Cancel
            } else {
                self.answers.remove(0)
            }
        }
    }

    fn tree() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        stdfs::create_dir_all(dir.path().join("src/sub")).unwrap();
        stdfs::write(dir.path().join("src/a.txt"), b"aaaa").unwrap();
        stdfs::write(dir.path().join("src/sub/b.txt"), b"bb").unwrap();
        stdfs::create_dir(dir.path().join("dst")).unwrap();
        dir
    }

    fn run(spec: &JobSpec, reporter: &mut Scripted) -> Outcome {
        let cancel = CancelToken::new();
        let p = plan(&LocalFs, spec, &cancel).expect("plan");
        execute(&LocalFs, spec, p, reporter, &cancel)
    }

    #[test]
    fn copy_reproduces_the_tree_and_counts_what_it_did() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let mut r = Scripted::new(vec![]);
        let out = run(&spec, &mut r);
        assert!(out.is_clean(), "{out:?}");
        assert_eq!((out.files, out.dirs, out.bytes), (2, 2, 6));
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/sub/b.txt")).unwrap(),
            b"bb"
        );
        assert!(dir.path().join("src/a.txt").exists(), "the source survived");
        assert!(r.frames > 0, "no progress was reported");
    }

    #[test]
    fn a_conflict_is_asked_about_once_and_all_answers_stick() {
        let dir = tree();
        stdfs::create_dir_all(dir.path().join("dst/src/sub")).unwrap();
        stdfs::write(dir.path().join("dst/src/a.txt"), b"old").unwrap();
        stdfs::write(dir.path().join("dst/src/sub/b.txt"), b"old").unwrap();

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let mut r = Scripted::new(vec![Decision::OverwriteAll]);
        let out = run(&spec, &mut r);
        assert!(out.is_clean(), "{out:?}");
        assert_eq!(r.asked.len(), 1, "`All` should not ask again");
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/a.txt")).unwrap(),
            b"aaaa"
        );
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/sub/b.txt")).unwrap(),
            b"bb"
        );
    }

    #[test]
    fn skip_leaves_the_destination_untouched() {
        let dir = tree();
        stdfs::write(dir.path().join("dst/a.txt"), b"keep me").unwrap();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let mut r = Scripted::new(vec![Decision::Skip]);
        let out = run(&spec, &mut r);
        assert_eq!(out.skipped, 1);
        assert_eq!(out.files, 0);
        assert_eq!(
            stdfs::read(dir.path().join("dst/a.txt")).unwrap(),
            b"keep me"
        );
    }

    #[test]
    fn rename_on_conflict_writes_beside_the_original() {
        let dir = tree();
        stdfs::write(dir.path().join("dst/a.txt"), b"old").unwrap();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            on_conflict: ConflictPolicy::Rename,
            ..JobOptions::default()
        });
        let mut r = Scripted::new(vec![]);
        let out = run(&spec, &mut r);
        assert!(out.is_clean(), "{out:?}");
        assert_eq!(stdfs::read(dir.path().join("dst/a.txt")).unwrap(), b"old");
        assert_eq!(
            stdfs::read(dir.path().join("dst/a (2).txt")).unwrap(),
            b"aaaa"
        );
        assert!(r.asked.is_empty(), "a policy answer must not ask");
    }

    #[test]
    fn overwrite_if_newer_skips_the_older_source() {
        let dir = tree();
        let dst = dir.path().join("dst/a.txt");
        stdfs::write(&dst, b"newer").unwrap();
        let future = std::time::SystemTime::now() + std::time::Duration::from_secs(3600);
        stdfs::File::options()
            .write(true)
            .open(&dst)
            .unwrap()
            .set_modified(future)
            .unwrap();

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            on_conflict: ConflictPolicy::OverwriteIfNewer,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert_eq!(out.skipped, 1);
        assert_eq!(stdfs::read(&dst).unwrap(), b"newer");
    }

    #[test]
    fn move_on_the_same_filesystem_renames_instead_of_copying() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert!(out.is_clean(), "{out:?}");
        assert_eq!(out.renamed, 1, "a same-filesystem move must not copy bytes");
        assert!(!dir.path().join("src").exists());
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/sub/b.txt")).unwrap(),
            b"bb"
        );
    }

    #[test]
    fn move_falls_back_to_copy_and_delete_when_the_target_exists() {
        let dir = tree();
        stdfs::create_dir_all(dir.path().join("dst/src")).unwrap();
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let out = run(&spec, &mut Scripted::new(vec![Decision::OverwriteAll]));
        assert_eq!(out.renamed, 0);
        assert!(out.is_clean(), "{out:?}");
        assert!(
            !dir.path().join("src").exists(),
            "the source should be gone"
        );
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/a.txt")).unwrap(),
            b"aaaa"
        );
    }

    #[test]
    fn delete_removes_a_whole_tree() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Delete,
            vec![VPath::local(dir.path().join("src"))],
            None,
        );
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert!(out.is_clean(), "{out:?}");
        assert_eq!((out.files, out.dirs), (2, 2));
        assert!(!dir.path().join("src").exists());
    }

    #[test]
    fn mkdir_creates_the_whole_path() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Mkdir,
            vec![],
            Some(VPath::local(dir.path().join("a/b/c"))),
        );
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert!(out.is_clean(), "{out:?}");
        assert!(dir.path().join("a/b/c").is_dir());
    }

    #[test]
    fn an_error_offers_retry_then_records_it() {
        let dir = tree();
        // A file where a directory has to go: creating it must fail.
        stdfs::write(dir.path().join("dst/src"), b"in the way").unwrap();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let mut r = Scripted::new(vec![Decision::Retry, Decision::SkipAll]);
        let out = run(&spec, &mut r);
        assert!(!out.is_clean());
        assert_eq!(r.asked.len(), 2, "retry asked once, then the second answer");
        assert!(matches!(r.asked[0], Question::Error { .. }));
        assert!(!out.errors.is_empty());
    }

    #[test]
    fn a_cancelled_job_stops_and_says_so() {
        let dir = tree();
        let cancel = CancelToken::new();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let p = plan(&LocalFs, &spec, &cancel).unwrap();
        cancel.cancel();
        let out = execute(&LocalFs, &spec, p, &mut Scripted::new(vec![]), &cancel);
        assert!(out.cancelled);
        assert_eq!(out.files, 0);
        assert!(!dir.path().join("dst/src/a.txt").exists());
    }

    #[test]
    fn a_dry_run_changes_nothing_but_counts_everything() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            dry_run: true,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert_eq!((out.files, out.bytes), (2, 6));
        assert!(
            !dir.path().join("dst/src").exists(),
            "a dry run wrote something"
        );
    }

    #[test]
    fn metadata_survives_a_copy_but_only_when_asked() {
        let dir = tree();
        let when =
            std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_500_000_000);
        stdfs::File::options()
            .write(true)
            .open(dir.path().join("src/a.txt"))
            .unwrap()
            .set_modified(when)
            .unwrap();

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst/kept.txt"))),
        );
        run(&spec, &mut Scripted::new(vec![]));
        assert_eq!(
            LocalFs
                .facts(&VPath::local(dir.path().join("dst/kept.txt")))
                .unwrap()
                .mtime,
            Some(when)
        );

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst/fresh.txt"))),
        )
        .with_options(JobOptions {
            preserve: Preserve {
                mtime: false,
                mode: false,
            },
            ..JobOptions::default()
        });
        run(&spec, &mut Scripted::new(vec![]));
        assert_ne!(
            LocalFs
                .facts(&VPath::local(dir.path().join("dst/fresh.txt")))
                .unwrap()
                .mtime,
            Some(when)
        );
    }

    #[test]
    fn the_per_file_bar_tracks_the_file_being_copied() {
        let p = Progress {
            file_bytes_done: 3,
            file_bytes_total: 4,
            ..Progress::default()
        };
        assert_eq!(p.file_percent(), 75);
        assert_eq!(Progress::default().file_percent(), 0);
    }

    #[test]
    fn progress_percentage_uses_bytes_then_files() {
        let p = Progress {
            bytes_done: 50,
            bytes_total: 200,
            ..Progress::default()
        };
        assert_eq!(p.percent(), 25);
        let p = Progress {
            files_done: 3,
            files_total: 4,
            ..Progress::default()
        };
        assert_eq!(p.percent(), 75);
        assert_eq!(Progress::default().percent(), 0);
    }
}
