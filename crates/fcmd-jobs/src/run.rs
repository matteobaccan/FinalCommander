use std::time::{Duration, Instant};

use fcmd_core::{CancelToken, VPath};

use crate::fs::{free_name, JobFs};
use crate::plan::{Plan, Unit, UnitKind};
use crate::replace::{Matcher, ReplacePass};
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
    /// How long the job has been running. The engine refreshes it on every frame it reports,
    /// so the three derived numbers need no clock of their own and a test can state the time
    /// outright instead of sleeping.
    pub elapsed: Duration,
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

    /// Average speed over the whole job so far, in bytes per second. `None` until there is
    /// something to divide: no byte has moved, or no time has passed yet.
    pub fn bytes_per_sec(&self) -> Option<f64> {
        let secs = self.elapsed.as_secs_f64();
        if self.bytes_done == 0 || secs <= 0.0 {
            return None;
        }
        Some(self.bytes_done as f64 / secs)
    }

    /// What is left of the job at the average speed so far, or `None` when any answer would
    /// be invention: nothing moved yet to average over, or no byte total to count down to
    /// (a job of empty files, or a provider that could not size the sources).
    ///
    /// The average and not the last tick, deliberately: tick speed swings by an order of
    /// magnitude between a cached file and a cold one, and a remaining time that jumps
    /// around is worse than no remaining time at all.
    pub fn eta(&self) -> Option<Duration> {
        let total = (self.bytes_total > 0).then_some(self.bytes_total)?;
        let speed = self.bytes_per_sec()?;
        let left = total.saturating_sub(self.bytes_done) as f64;
        // A speed near zero can ask for more seconds than a `Duration` holds; saturating is
        // the only sane reading of "at this rate, never".
        Some(Duration::try_from_secs_f64(left / speed).unwrap_or(Duration::MAX))
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
    /// For a [`JobKind::Replace`]: how many occurrences each file had, in the order the files
    /// were visited.
    ///
    /// Only the files whose bytes really differ are listed, which is exactly what a dry run
    /// lists and the whole reason the UI can show one before asking: a file where the needle
    /// was found but the replacement spelled the same bytes had nothing to do, and a file
    /// where it was not found at all has nothing to say.
    pub replacements: Vec<(VPath, u64)>,
}

impl Outcome {
    pub fn is_clean(&self) -> bool {
        self.errors.is_empty() && !self.cancelled
    }

    /// Every occurrence replaced across every file, for the one line the job reports.
    pub fn occurrences(&self) -> u64 {
        self.replacements.iter().map(|(_, n)| n).sum()
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
        append_all: spec.options.on_conflict == ConflictPolicy::Append,
        skip_all: spec.options.on_conflict == ConflictPolicy::Skip,
        skip_errors: false,
        outcome: Outcome::default(),
        dir_meta: Vec::new(),
        kept: Vec::new(),
        bytes_since_report: 0,
        started: Instant::now(),
    };

    // Compiled once for the whole job. A needle that does not compile was refused while
    // planning, so the only way to be here without one is a caller that went around the
    // planner — which every unit then reports, rather than guessing at what to look for.
    let matcher = spec
        .replace
        .as_ref()
        .and_then(|options| Matcher::compile(options).ok());

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
        run_unit(
            fs,
            spec,
            unit,
            matcher.as_ref(),
            &mut state,
            reporter,
            cancel,
        );
        state.progress.elapsed = state.started.elapsed();
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
    state.progress.elapsed = state.started.elapsed();
    reporter.progress(&state.progress);
    state.outcome.skipped = state.progress.skipped;
    state.outcome
}

struct State {
    progress: Progress,
    overwrite_all: bool,
    append_all: bool,
    skip_all: bool,
    skip_errors: bool,
    outcome: Outcome,
    dir_meta: Vec<(VPath, VPath)>,
    /// Sources whose copy did not happen, so a move must leave them where they are.
    kept: Vec<VPath>,
    bytes_since_report: u64,
    started: Instant,
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
    matcher: Option<&Matcher>,
    state: &mut State,
    reporter: &mut dyn Reporter,
    cancel: &CancelToken,
) {
    loop {
        let result = attempt(fs, spec, unit, matcher, state, reporter, cancel);
        match result {
            Attempt::Done => return,
            Attempt::Skipped => {
                state.progress.skipped += 1;
                keep_source(state, unit);
                return;
            }
            Attempt::Cancelled => {
                state.outcome.cancelled = true;
                keep_source(state, unit);
                return;
            }
            Attempt::Failed(error) => {
                keep_source(state, unit);
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

/// A move removes a source only once its copy is really in place. Anything skipped, failed or
/// cut short keeps its source — and, through [`State::kept`], the directories above it — so
/// the one thing a move must never do, lose the file it was moving, cannot happen.
fn keep_source(state: &mut State, unit: &Unit) {
    if let Some(src) = &unit.src {
        state.kept.push(src.clone());
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
    matcher: Option<&Matcher>,
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
            let (dst, append) = match resolve_conflict(fs, spec, unit, state, reporter) {
                Conflict::Proceed(dst) => (dst, false),
                Conflict::Append(dst) => (dst, true),
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
            let copied = if append {
                fs.append_file(&src, &dst, spec.options.buffer, &mut tick)
            } else {
                fs.copy_file(&src, &dst, spec.options.buffer, &mut tick)
            };
            match copied {
                Ok(bytes) => {
                    // A concatenation inherits nothing: the destination is no longer a copy of
                    // the source, so its own times and permissions are the right ones to keep.
                    if !append {
                        if let Ok(facts) = fs.facts(&src) {
                            let _ = fs.apply_meta(&dst, &facts, spec.options.preserve);
                        }
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
                        state.progress.elapsed = state.started.elapsed();
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
                // A link holds a path, not bytes, so there is nothing to append to. Skipping
                // leaves the destination as it is rather than replacing it behind an answer
                // that meant something else.
                Conflict::Append(_) => return Attempt::Skipped,
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
            if state.kept.iter().any(|kept| kept == &unit.dst) {
                return Attempt::Done;
            }
            if !dry {
                match fs.remove_file(&unit.dst) {
                    Ok(()) => {}
                    // A move already took it away: not an error.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Attempt::Failed(e.to_string()),
                }
            }
            if destroys(spec.kind) {
                state.outcome.files += 1;
                state.progress.files_done += 1;
            }
            Attempt::Done
        }

        UnitKind::Trash => {
            if !dry {
                match fs.trash(&unit.dst) {
                    Ok(()) => {}
                    // Something else already took it away: not an error, as in a delete.
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Attempt::Failed(e.to_string()),
                }
            }
            // One unit is one thing the user chose, whether it was a file or a tree: the
            // wastebasket took it whole, and counting what was inside it would mean walking
            // a directory that is no longer there.
            state.outcome.files += 1;
            state.progress.files_done += 1;
            Attempt::Done
        }

        UnitKind::WipeFile { size } => {
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
            match fs.wipe_file(&unit.dst, spec.options.buffer, &mut tick) {
                Ok(()) => {
                    state.outcome.files += 1;
                    state.outcome.bytes += size;
                    state.progress.files_done += 1;
                    Attempt::Done
                }
                // A file already gone is not an error, here or in a plain delete.
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Attempt::Done,
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Attempt::Cancelled,
                Err(e) => Attempt::Failed(e.to_string()),
            }
        }

        UnitKind::ReplaceInFile { size } => {
            let Some(matcher) = matcher else {
                return Attempt::Failed("there is nothing to search for".into());
            };
            // Read before the rewrite, because a file being rewritten is its own source of
            // permissions and times, the way a copy's source is: one `Preserve` then decides
            // what a replacement keeps and what a copy keeps alike.
            let facts = fs.facts(&unit.dst).ok();
            // A dry run differs in this one value and in nothing else, which is what makes the
            // number it shows the number the write then produces.
            let pass = if dry {
                ReplacePass::Count
            } else {
                ReplacePass::Rewrite
            };

            state.progress.file_bytes_total = *size;
            state.progress.file_bytes_done = 0;
            let mut tick = |n: u64| {
                state.progress.bytes_done += n;
                state.progress.file_bytes_done += n;
                state.bytes_since_report += n;
                !cancel.is_cancelled()
            };
            match fs.replace_in_file(&unit.dst, matcher, pass, spec.options.buffer, &mut tick) {
                Ok(report) => {
                    // Every byte read is work this job did, whether it changed anything or not.
                    state.outcome.bytes += size;
                    if !report.changed {
                        // Nothing to do: either the needle was not there, or putting the
                        // replacement in its place spells the bytes already on the disk. The
                        // file is left as it is, and that is a skip rather than a change.
                        return Attempt::Skipped;
                    }
                    if report.rewritten {
                        if let Some(facts) = &facts {
                            let _ = fs.apply_meta(&unit.dst, facts, spec.options.preserve);
                        }
                    }
                    state
                        .outcome
                        .replacements
                        .push((unit.dst.clone(), report.occurrences));
                    state.outcome.files += 1;
                    state.progress.files_done += 1;
                    if state.bytes_since_report > 1024 * 1024 {
                        state.bytes_since_report = 0;
                        state.progress.elapsed = state.started.elapsed();
                        reporter.progress(&state.progress);
                    }
                    Attempt::Done
                }
                Err(e) if e.kind() == std::io::ErrorKind::Interrupted => Attempt::Cancelled,
                Err(e) => Attempt::Failed(e.to_string()),
            }
        }

        UnitKind::RemoveDir => {
            // Something below this directory stayed, so the directory stays too. Removing it
            // would fail anyway; not trying is the difference between a quiet result and a
            // dialog about every parent of the one file the user chose to keep.
            if state.kept.iter().any(|kept| kept.starts_with(&unit.dst)) {
                return Attempt::Done;
            }
            if !dry {
                match fs.remove_dir(&unit.dst) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    Err(e) => return Attempt::Failed(e.to_string()),
                }
            }
            if destroys(spec.kind) {
                state.outcome.dirs += 1;
            }
            Attempt::Done
        }
    }
}

/// Is the removal the point of this job, or the cleanup half of a move? A move's counters
/// describe the copy, so its removals are not counted again.
fn destroys(kind: JobKind) -> bool {
    matches!(kind, JobKind::Delete | JobKind::Wipe | JobKind::Trash)
}

enum Conflict {
    Proceed(VPath),
    Append(VPath),
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

    let src = unit.src.clone().unwrap_or_else(|| unit.dst.clone());
    // "Only newer" is a gate, not a policy: it is checked before an `…All` answer is honoured,
    // so "overwrite all" plus "only newer" still leaves an older source where it is.
    if spec.options.only_newer && !src_is_newer(fs, &src, &unit.dst) {
        return Conflict::Skip;
    }
    if state.skip_all {
        return Conflict::Skip;
    }
    if state.append_all {
        return Conflict::Append(unit.dst.clone());
    }
    if state.overwrite_all {
        return Conflict::Proceed(unit.dst.clone());
    }

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
        ConflictPolicy::Append => return Conflict::Append(unit.dst.clone()),
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
        Decision::Append => Conflict::Append(unit.dst.clone()),
        Decision::AppendAll => {
            state.append_all = true;
            Conflict::Append(unit.dst.clone())
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

/// Is the source newer than the destination? A timestamp either side does not have counts as
/// "newer", because a provider that cannot date a file must not turn "only newer" into
/// "nothing at all".
fn src_is_newer(fs: &dyn JobFs, src: &VPath, dst: &VPath) -> bool {
    match (
        fs.facts(src).ok().and_then(|f| f.mtime),
        fs.facts(dst).ok().and_then(|f| f.mtime),
    ) {
        (Some(a), Some(b)) => a > b,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::LocalFs;
    use crate::plan::plan;
    use crate::replace::ReplaceOptions;
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

    /// Set a file's modification time, which is how a test says "older" or "newer".
    fn touch(path: &std::path::Path, when: std::time::SystemTime) {
        stdfs::File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(when)
            .unwrap();
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
    fn a_move_that_skips_a_file_leaves_the_source_alone() {
        let dir = tree();
        stdfs::write(dir.path().join("dst/a.txt"), b"keep me").unwrap();
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            on_conflict: ConflictPolicy::Skip,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));

        assert!(out.is_clean(), "{out:?}");
        assert_eq!(out.skipped, 1);
        assert_eq!(
            stdfs::read(dir.path().join("dst/a.txt")).unwrap(),
            b"keep me"
        );
        assert_eq!(
            stdfs::read(dir.path().join("src/a.txt")).unwrap(),
            b"aaaa",
            "a move that copied nothing must not remove the source"
        );
    }

    #[test]
    fn a_move_keeps_the_directories_above_what_it_skipped() {
        let dir = tree();
        // Only the deeper file is in the way, so one half of the tree moves and one stays.
        stdfs::create_dir_all(dir.path().join("dst/src/sub")).unwrap();
        stdfs::write(dir.path().join("dst/src/sub/b.txt"), b"keep me").unwrap();
        let spec = JobSpec::new(
            JobKind::Move,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            on_conflict: ConflictPolicy::Skip,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));

        assert!(out.is_clean(), "{out:?}");
        assert_eq!(
            stdfs::read(dir.path().join("src/sub/b.txt")).unwrap(),
            b"bb",
            "the file that was not copied had to stay"
        );
        assert!(
            !dir.path().join("src/a.txt").exists(),
            "the file that did move should be gone"
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
    fn wipe_empties_every_file_then_takes_the_tree_with_it() {
        let dir = tree();
        // A second name for one of the files, so there is something left to read afterwards.
        let witness = VPath::local(dir.path().join("witness"));
        LocalFs
            .hard_link(&VPath::local(dir.path().join("src/a.txt")), &witness)
            .unwrap();

        let spec = JobSpec::new(
            JobKind::Wipe,
            vec![VPath::local(dir.path().join("src"))],
            None,
        );
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert!(out.is_clean(), "{out:?}");
        assert_eq!((out.files, out.dirs, out.bytes), (2, 2, 6));
        assert!(!dir.path().join("src").exists());
        assert_eq!(
            stdfs::read(witness.as_path()).unwrap(),
            b"\0\0\0\0",
            "the bytes should have been written over, not just unlinked"
        );
    }

    #[test]
    fn a_dry_run_wipe_destroys_nothing() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Wipe,
            vec![VPath::local(dir.path().join("src"))],
            None,
        )
        .with_options(JobOptions {
            dry_run: true,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert_eq!((out.files, out.bytes), (2, 6));
        assert_eq!(stdfs::read(dir.path().join("src/a.txt")).unwrap(), b"aaaa");
    }

    #[test]
    fn append_puts_the_source_after_what_the_destination_already_holds() {
        let dir = tree();
        let dst = dir.path().join("dst/a.txt");
        stdfs::write(&dst, b"before ").unwrap();
        let before = stdfs::metadata(&dst).unwrap().permissions();

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            on_conflict: ConflictPolicy::Append,
            ..JobOptions::default()
        });
        let mut r = Scripted::new(vec![]);
        let out = run(&spec, &mut r);

        assert!(out.is_clean(), "{out:?}");
        assert!(r.asked.is_empty(), "a policy answer must not ask");
        assert_eq!(stdfs::read(&dst).unwrap(), b"before aaaa");
        assert_eq!(out.bytes, 4, "only the appended bytes are this job's work");
        assert_eq!(
            stdfs::metadata(&dst).unwrap().permissions(),
            before,
            "an append must leave the destination's permissions alone"
        );
    }

    #[test]
    fn append_can_be_answered_once_for_the_whole_job() {
        let dir = tree();
        stdfs::create_dir_all(dir.path().join("dst/src/sub")).unwrap();
        stdfs::write(dir.path().join("dst/src/a.txt"), b"1:").unwrap();
        stdfs::write(dir.path().join("dst/src/sub/b.txt"), b"2:").unwrap();

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        let mut r = Scripted::new(vec![Decision::AppendAll]);
        let out = run(&spec, &mut r);
        assert!(out.is_clean(), "{out:?}");
        assert_eq!(r.asked.len(), 1, "`All` should not ask again");
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/a.txt")).unwrap(),
            b"1:aaaa"
        );
        assert_eq!(
            stdfs::read(dir.path().join("dst/src/sub/b.txt")).unwrap(),
            b"2:bb"
        );
    }

    #[test]
    fn only_newer_skips_an_older_source_whatever_the_policy_says() {
        let dir = tree();
        // One destination newer than its source, one older. Both are in the way.
        let newer_dst = dir.path().join("dst/a.txt");
        let older_dst = dir.path().join("dst/b.txt");
        stdfs::write(&newer_dst, b"newer destination").unwrap();
        stdfs::write(&older_dst, b"older destination").unwrap();
        touch(
            &newer_dst,
            std::time::SystemTime::now() + Duration::from_secs(3600),
        );
        touch(
            &older_dst,
            std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1),
        );

        let spec = JobSpec::new(
            JobKind::Copy,
            vec![
                VPath::local(dir.path().join("src/a.txt")),
                VPath::local(dir.path().join("src/sub/b.txt")),
            ],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            // The gate holds even here, where the policy would have overwritten everything
            // without asking: that is what makes it a gate and not another policy.
            on_conflict: ConflictPolicy::Overwrite,
            only_newer: true,
            ..JobOptions::default()
        });
        let mut r = Scripted::new(vec![]);
        let out = run(&spec, &mut r);

        assert!(out.is_clean(), "{out:?}");
        assert!(r.asked.is_empty());
        assert_eq!(out.skipped, 1, "a file that is not newer is skipped");
        assert_eq!(out.files, 1);
        assert_eq!(
            stdfs::read(&newer_dst).unwrap(),
            b"newer destination",
            "the newer destination survived"
        );
        assert_eq!(
            stdfs::read(&older_dst).unwrap(),
            b"bb",
            "the older destination was replaced"
        );
    }

    #[test]
    fn only_newer_leaves_a_destination_that_is_not_there_to_the_normal_path() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src/a.txt"))],
            Some(VPath::local(dir.path().join("dst"))),
        )
        .with_options(JobOptions {
            only_newer: true,
            ..JobOptions::default()
        });
        let out = run(&spec, &mut Scripted::new(vec![]));
        assert_eq!((out.files, out.skipped), (1, 0));
        assert_eq!(stdfs::read(dir.path().join("dst/a.txt")).unwrap(), b"aaaa");
    }

    /// Three files with the same needle in them, and one without.
    fn texts() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        stdfs::write(dir.path().join("one.txt"), b"alpha beta alpha").unwrap();
        stdfs::write(dir.path().join("two.txt"), b"beta alpha gamma").unwrap();
        stdfs::write(dir.path().join("three.txt"), b"alpha").unwrap();
        stdfs::write(dir.path().join("none.txt"), b"nothing of the sort").unwrap();
        dir
    }

    fn all_texts(dir: &tempfile::TempDir) -> Vec<VPath> {
        ["one.txt", "two.txt", "three.txt", "none.txt"]
            .iter()
            .map(|name| VPath::local(dir.path().join(name)))
            .collect()
    }

    fn replace_spec(dir: &tempfile::TempDir, dry_run: bool) -> JobSpec {
        JobSpec::new(JobKind::Replace, all_texts(dir), None)
            .with_options(JobOptions {
                dry_run,
                ..JobOptions::default()
            })
            .with_replace(ReplaceOptions::new("alpha", "OMEGA"))
    }

    #[test]
    fn a_dry_run_counts_exactly_what_the_write_then_changes() {
        let dir = texts();
        let dry = run(&replace_spec(&dir, true), &mut Scripted::new(vec![]));
        assert!(dry.is_clean(), "{dry:?}");
        assert_eq!(
            stdfs::read(dir.path().join("one.txt")).unwrap(),
            b"alpha beta alpha",
            "a dry run wrote something"
        );

        let wet = run(&replace_spec(&dir, false), &mut Scripted::new(vec![]));
        assert!(wet.is_clean(), "{wet:?}");
        assert_eq!(
            dry.replacements, wet.replacements,
            "the dry run and the write disagreed about the files"
        );
        assert_eq!(dry.occurrences(), 4);
        assert_eq!((dry.files, dry.skipped), (wet.files, wet.skipped));
        assert_eq!((wet.files, wet.skipped), (3, 1));
        assert_eq!(
            stdfs::read(dir.path().join("one.txt")).unwrap(),
            b"OMEGA beta OMEGA"
        );
        assert_eq!(
            stdfs::read(dir.path().join("two.txt")).unwrap(),
            b"beta OMEGA gamma"
        );
        assert_eq!(
            stdfs::read(dir.path().join("none.txt")).unwrap(),
            b"nothing of the sort",
            "a file with no match must not be rewritten"
        );
    }

    #[test]
    fn a_replacement_keeps_the_file_it_rewrote_as_the_copy_rules_say() {
        let dir = texts();
        let path = dir.path().join("one.txt");
        let when = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000);
        touch(&path, when);
        #[cfg(unix)]
        let mode = {
            use std::os::unix::fs::PermissionsExt;
            stdfs::set_permissions(&path, stdfs::Permissions::from_mode(0o640)).unwrap();
            0o640
        };

        let out = run(&replace_spec(&dir, false), &mut Scripted::new(vec![]));
        assert!(out.is_clean(), "{out:?}");
        let facts = LocalFs.facts(&VPath::local(&path)).unwrap();
        assert_eq!(
            facts.mtime,
            Some(when),
            "the default `Preserve` keeps the file's own modification time"
        );
        #[cfg(unix)]
        assert_eq!(facts.mode.map(|m| m & 0o777), Some(mode));

        // And without `Preserve`, the rewrite is as recent as it really is.
        stdfs::write(&path, b"alpha again").unwrap();
        touch(&path, when);
        let spec = replace_spec(&dir, false).with_options(JobOptions {
            preserve: Preserve {
                mtime: false,
                mode: false,
            },
            ..JobOptions::default()
        });
        run(&spec, &mut Scripted::new(vec![]));
        assert_ne!(
            LocalFs.facts(&VPath::local(&path)).unwrap().mtime,
            Some(when)
        );
    }

    #[test]
    fn a_cancelled_replacement_leaves_every_file_it_had_not_reached_untouched() {
        /// A reporter that pulls the plug as soon as it sees the first file go by.
        struct CancelAtOnce<'a>(&'a CancelToken);
        impl Reporter for CancelAtOnce<'_> {
            fn progress(&mut self, _p: &Progress) {
                self.0.cancel();
            }

            fn ask(&mut self, _q: Question) -> Decision {
                Decision::Cancel
            }
        }

        let dir = texts();
        let spec = replace_spec(&dir, false);
        let cancel = CancelToken::new();
        let p = plan(&LocalFs, &spec, &cancel).unwrap();
        let out = execute(&LocalFs, &spec, p, &mut CancelAtOnce(&cancel), &cancel);

        assert!(out.cancelled);
        assert_eq!(out.files, 1, "the first file was already written");
        assert_eq!(
            stdfs::read(dir.path().join("one.txt")).unwrap(),
            b"OMEGA beta OMEGA"
        );
        for name in ["two.txt", "three.txt", "none.txt"] {
            assert!(
                !stdfs::read(dir.path().join(name))
                    .unwrap()
                    .starts_with(b"OMEGA"),
                "{name} was changed after the job was cancelled"
            );
        }
    }

    #[test]
    fn a_directory_in_the_list_is_a_problem_and_the_other_files_still_change() {
        let dir = texts();
        stdfs::create_dir(dir.path().join("sub")).unwrap();
        let mut sources = vec![VPath::local(dir.path().join("sub"))];
        sources.extend(all_texts(&dir));
        let spec = JobSpec::new(JobKind::Replace, sources, None)
            .with_replace(ReplaceOptions::new("alpha", "OMEGA"));

        let mut r = Scripted::new(vec![Decision::SkipAll]);
        let out = run(&spec, &mut r);

        assert_eq!(r.asked.len(), 1, "the directory is the one problem");
        assert!(matches!(r.asked[0], Question::Error { .. }));
        assert_eq!(out.errors.len(), 1);
        assert_eq!(out.errors[0].0, VPath::local(dir.path().join("sub")));
        assert_eq!(out.files, 3, "the rest of the list was still done");
        assert_eq!(
            stdfs::read(dir.path().join("two.txt")).unwrap(),
            b"beta OMEGA gamma"
        );
    }

    #[test]
    fn a_replacement_that_spells_the_same_bytes_is_nothing_to_do() {
        let dir = texts();
        let spec = JobSpec::new(JobKind::Replace, all_texts(&dir), None)
            .with_replace(ReplaceOptions::new("alpha", "alpha"));
        let out = run(&spec, &mut Scripted::new(vec![]));

        assert!(out.is_clean(), "{out:?}");
        assert_eq!(out.files, 0, "no file changed");
        assert_eq!(out.skipped, 4, "every one of them had nothing to do");
        assert!(out.replacements.is_empty());
        assert_eq!(out.occurrences(), 0);
        assert_eq!(
            stdfs::read(dir.path().join("one.txt")).unwrap(),
            b"alpha beta alpha"
        );
    }

    #[test]
    fn a_replacement_reports_the_file_it_is_working_on_and_the_bytes_it_has_read() {
        /// A reporter that keeps the last frame of every phase it saw.
        struct Frames {
            current: Vec<VPath>,
            last: Progress,
        }
        impl Reporter for Frames {
            fn progress(&mut self, p: &Progress) {
                if let Some(path) = &p.current {
                    self.current.push(path.clone());
                }
                self.last = p.clone();
            }

            fn ask(&mut self, _q: Question) -> Decision {
                Decision::Cancel
            }
        }

        let dir = texts();
        let spec = replace_spec(&dir, false);
        let cancel = CancelToken::new();
        let p = plan(&LocalFs, &spec, &cancel).unwrap();
        let total = p.bytes;
        let mut frames = Frames {
            current: Vec::new(),
            last: Progress::default(),
        };
        let out = execute(&LocalFs, &spec, p, &mut frames, &cancel);

        assert_eq!(frames.current.len(), 4, "one frame per file");
        assert_eq!(frames.current[0], VPath::local(dir.path().join("one.txt")));
        assert_eq!(frames.last.phase, Phase::Done);
        assert_eq!(frames.last.current, None);
        assert_eq!(
            (frames.last.bytes_done, out.bytes),
            (total, total),
            "every byte of every file was read, matched or not"
        );
        assert_eq!(frames.last.percent(), 100);
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
    fn average_speed_is_the_bytes_over_the_elapsed_time() {
        let p = Progress {
            bytes_done: 3_000_000,
            bytes_total: 12_000_000,
            elapsed: Duration::from_secs(2),
            ..Progress::default()
        };
        assert_eq!(p.bytes_per_sec(), Some(1_500_000.0));
    }

    #[test]
    fn speed_and_remaining_time_say_nothing_until_they_know_something() {
        // No clock reading yet: dividing by zero seconds is the obvious trap.
        let no_time = Progress {
            bytes_done: 1_000,
            bytes_total: 2_000,
            ..Progress::default()
        };
        assert_eq!(no_time.bytes_per_sec(), None);
        assert_eq!(no_time.eta(), None);

        // Time but no bytes: the job has not shown how fast it is.
        let no_bytes = Progress {
            bytes_total: 2_000,
            elapsed: Duration::from_secs(5),
            ..Progress::default()
        };
        assert_eq!(no_bytes.bytes_per_sec(), None);
        assert_eq!(no_bytes.eta(), None);

        // Moving along, but nobody counted the sources: a speed, and no estimate.
        let unknown_total = Progress {
            bytes_done: 4_000,
            elapsed: Duration::from_secs(2),
            ..Progress::default()
        };
        assert_eq!(unknown_total.bytes_per_sec(), Some(2_000.0));
        assert_eq!(
            unknown_total.eta(),
            None,
            "an unknown total cannot be timed"
        );
    }

    #[test]
    fn remaining_time_comes_from_the_average_and_reaches_zero() {
        // A quarter done in one second: three more seconds at that average.
        let p = Progress {
            bytes_done: 1_000,
            bytes_total: 4_000,
            elapsed: Duration::from_secs(1),
            ..Progress::default()
        };
        assert_eq!(p.eta(), Some(Duration::from_secs(3)));

        // The same job after a slow patch: the average, not the last tick, decides.
        let slower = Progress {
            elapsed: Duration::from_secs(4),
            ..p.clone()
        };
        assert_eq!(slower.eta(), Some(Duration::from_secs(12)));

        // Finished, and the odd case of a total the plan undercounted.
        let done = Progress {
            bytes_done: 4_000,
            ..p.clone()
        };
        assert_eq!(done.eta(), Some(Duration::ZERO));
        let over = Progress {
            bytes_done: 5_000,
            ..p
        };
        assert_eq!(over.eta(), Some(Duration::ZERO));
    }

    #[test]
    fn a_speed_too_slow_to_measure_saturates_instead_of_panicking() {
        let crawling = Progress {
            bytes_done: 1,
            bytes_total: u64::MAX,
            elapsed: Duration::from_secs(1_000_000),
            ..Progress::default()
        };
        assert_eq!(crawling.eta(), Some(Duration::MAX));
    }

    #[test]
    fn a_finished_job_reports_the_time_it_took() {
        let dir = tree();
        let spec = JobSpec::new(
            JobKind::Copy,
            vec![VPath::local(dir.path().join("src"))],
            Some(VPath::local(dir.path().join("dst"))),
        );
        struct Last(Duration);
        impl Reporter for Last {
            fn progress(&mut self, p: &Progress) {
                if p.phase == Phase::Done {
                    self.0 = p.elapsed;
                }
            }

            fn ask(&mut self, _q: Question) -> Decision {
                Decision::Cancel
            }
        }
        let mut last = Last(Duration::MAX);
        let cancel = CancelToken::new();
        let p = plan(&LocalFs, &spec, &cancel).unwrap();
        execute(&LocalFs, &spec, p, &mut last, &cancel);
        assert!(last.0 < Duration::from_secs(60), "{:?}", last.0);
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
